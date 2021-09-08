use crate::{
    data::Data,
    sensor_obj::SensorObj,
    services::{
        self,
        central::{TimeseriesRequest, TimeseriesResponse},
    },
    widgets::graph::Unit,
};
use ble_ws_api::data::Timestamp;
use gio::prelude::*;
use url::Url;
use uuid::Uuid;

pub enum Event {
    OpenSearch,
    OpenOverview,
    OpenDetail(Uuid),
    DetailRangeChanged {
        id: Uuid,
        from: time::OffsetDateTime,
        to: time::OffsetDateTime,
    },
    ChangeLabel {
        id: Uuid,
        label: Label,
    },
    OverviewUpdate(Vec<(Uuid, ble_ws_api::proto::SensorOverview)>),
    FetchedTimeseries {
        timeseries: TimeseriesResponse,
        id: Uuid,
    },
    SensorSelected(Uuid),
}

pub fn attach(tx: glib::Sender<Event>, rx: glib::Receiver<Event>, window: crate::widgets::Window) {
    let mut state = State::default();
    let svcs = services::ServiceManager::new(tx.clone()).unwrap();

    let central = svcs.create_service::<services::central::Central>().unwrap();
    let settings = crate::config::settings();

    // FIXME: get token from secret service instead
    let token_file = std::env::var_os("TOKEN_FILE").expect("Missing TOKEN_FILE env var");
    let token = std::fs::read_to_string(&token_file)
        .ok()
        .and_then(|s| tonic::metadata::MetadataValue::from_str(&s).ok())
        .expect("Could not read TOKEN_FILE");

    if let Ok(url) = Url::parse(&settings.get::<String>("endpoint")) {
        central.set_endpoint(url, token.clone());
    }

    settings.connect_changed(Some("endpoint"), {
        let central = central.clone();
        move |settings, key| {
            let endpoint = settings.get::<String>(key);
            if let Ok(url) = Url::parse(&endpoint) {
                central.set_endpoint(url, token.clone());
            }
        }
    });

    rx.attach(None, {
        move |evt| {
            match evt {
                Event::DetailRangeChanged { id, from, to } => {
                    let from = Timestamp::from(from.unix_timestamp() as u32);
                    let to = Timestamp::from(to.unix_timestamp() as u32);
                    central.fetch_timeseries(TimeseriesRequest::Range {
                        id,
                        range: from..=to,
                    });
                }
                Event::OpenSearch => {
                    if let View::Overview {
                        ref mut search_active,
                        ..
                    } = state.display
                    {
                        *search_active = !*search_active;
                        if *search_active {
                            window.activate_search();
                        } else {
                            window.deactivate_search();
                        }
                    };
                }

                Event::SensorSelected(id) => {
                    if let Some(sensor) = state.data.sensors.get(&id) {
                        if sensor.timeseries().is_none() {
                            central.fetch_timeseries(TimeseriesRequest::Live(id));
                        }

                        if let View::Overview {
                            ref mut selected_sensor,
                            ..
                        } = state.display
                        {
                            *selected_sensor = Some(sensor.clone());
                            window.switch_view(&state.display);
                        }
                    }
                }

                Event::OpenOverview => {
                    state.display = View::Overview {
                        selected_sensor: None,
                        search_active: false,
                    };
                    window.switch_view(&state.display);
                }

                Event::FetchedTimeseries { id, timeseries } => {
                    if let Some(obj) = state.data.sensors.get(&id) {
                        match timeseries {
                            TimeseriesResponse::Live(timeseries) => {
                                obj.set_timeseries(timeseries);
                            }
                            TimeseriesResponse::Range(timeseries) => match &state.display {
                                View::Detail { sensor, .. } if sensor.id() == id => {
                                    window.set_static_timeseries(timeseries);
                                }
                                _ => (),
                            },
                        }
                    }
                }

                Event::OverviewUpdate(update) => {
                    for (addr, data) in update {
                        match state.data.sensors.get(&addr) {
                            Some(sensor) => {
                                sensor.update_values(data);
                            }
                            None => {
                                let sensor = SensorObj::new(addr);
                                sensor.update_values(data);
                                window.add_sensor(&sensor);
                                state.data.sensors.insert(addr, sensor);
                            }
                        }
                    }
                }

                Event::OpenDetail(addr) => {
                    if let Some(sensor) = state.data.sensors.get(&addr) {
                        if sensor.timeseries().is_none() {
                            central.fetch_timeseries(TimeseriesRequest::Live(addr));
                        }
                        state.display = View::Detail {
                            sensor: sensor.clone(),
                            unit: Unit::Temperature,
                        };

                        window.switch_view(&state.display);
                    }
                }

                Event::ChangeLabel { label, id } => {
                    central.set_label(id, label);
                }
            };
            glib::Continue(true)
        }
    });
}

#[derive(Default)]
struct State {
    data: Data,
    display: View,
}

pub enum View {
    Overview {
        selected_sensor: Option<SensorObj>,
        search_active: bool,
    },
    Detail {
        sensor: SensorObj,
        unit: Unit,
    },
}

impl Default for View {
    fn default() -> Self {
        View::Overview {
            selected_sensor: None,
            search_active: false,
        }
    }
}

#[derive(Debug)]
pub struct Label(Option<String>);

impl From<String> for Label {
    fn from(s: String) -> Self {
        if s.is_empty() {
            Self(None)
        } else {
            Self(Some(s))
        }
    }
}

impl Label {
    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}
