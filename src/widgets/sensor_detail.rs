use std::time::Duration;

use crate::{
    data::Timeseries,
    event_loop::Event,
    sensor_obj::SensorObj,
    widgets::graph::{Graph, GraphPainter, Unit},
};
use glib::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

use super::graph::Data;

mod imp {

    use std::cell::RefCell;

    use crate::event_loop::Event;

    use super::*;
    use ble_ws_api::data::Timestamp;
    use gtk::subclass::prelude::*;
    use once_cell::unsync::OnceCell;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/sensor_detail.ui")]
    pub struct SensorDetail {
        // FIXME: AIDS
        #[template_child]
        pub graph_temperature: TemplateChild<Graph>,
        #[template_child]
        pub graph_humidity: TemplateChild<Graph>,
        #[template_child]
        pub graph_pressure: TemplateChild<Graph>,
        #[template_child]
        pub from_picker: TemplateChild<crate::widgets::TimeDatePicker>,
        #[template_child]
        pub to_picker: TemplateChild<crate::widgets::TimeDatePicker>,
        #[template_child]
        pub detail_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub live_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub live_slider: TemplateChild<gtk::Scale>,
        #[template_child]
        pub menu_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub advanced_menu: TemplateChild<gtk::Box>,
        #[template_child]
        pub live_slider_box: TemplateChild<gtk::Box>,

        pub evt_tx: OnceCell<glib::Sender<Event>>,
        pub change_tx: OnceCell<tokio::sync::mpsc::Sender<()>>,
        pub painter: GraphPainter,
        pub sensor: RefCell<Option<SensorObj>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorDetail {
        const NAME: &'static str = "BleWsSensorDetail";
        type Type = super::SensorDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SensorDetail {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let now = crate::util::now_local();
            self.from_picker
                .set_datetime(now.replace_date(now.date().previous_day().unwrap()));
            self.to_picker.set_datetime(now);

            self.graph_humidity.set_painter(&self.painter);
            self.graph_temperature.set_painter(&self.painter);
            self.graph_pressure.set_painter(&self.painter);
            let painter = self.painter.downgrade();
            self.detail_stack
                .connect_visible_child_name_notify(move |me| {
                    let name = me
                        .visible_child_name()
                        .expect("Visible child name changed to something without a name");
                    let unit = match name.as_str() {
                        "temperature" => Unit::Temperature,
                        "pressure" => Unit::Pressure,
                        "humidity" => Unit::Humidity,
                        _ => panic!("Invalid child name"),
                    };

                    if let Some(painter) = painter.upgrade() {
                        if painter.displayed_unit() != unit {
                            painter.set_displayed_unit(unit);
                        }
                    } else {
                        panic!("Graph disappeared");
                    }
                });

            self.live_slider
                .connect_value_changed(glib::clone!(@weak obj => move |slider| {
                    let now = Timestamp::now();
                    let v = f64::floor(slider.value() * 60.) as u32;
                    let lower = now.as_u32().checked_sub(v).unwrap();
                    let self_ = imp::SensorDetail::from_instance(&obj);
                    self_.painter.set_bounds(Some((lower, u32::MAX)));
                }));
            self.live_slider.set_format_value_func(|_slider, value| {
                let value = value as u32;
                if value >= 60 {
                    let hours = value / 60;
                    let minutes = value % 60;
                    if minutes == 0 {
                        format!("{} hours", hours)
                    } else {
                        format!("{} hours {} minutes", hours, minutes)
                    }
                } else {
                    format!("{} minutes", value)
                }
            });

            self.live_switch
                .connect_active_notify(glib::clone!(@weak obj => move |_| {
                    let self_ = Self::from_instance(&obj);
                    if self_.live_switch.is_active() {
                        self_.live_slider.set_value(24. * 60.);
                        let sensor = self_.sensor.borrow();
                        self_.painter.set_displayed_data(Some(Data::Live(
                            sensor
                                .as_ref()
                                .expect("Tried to display detailed sensor info without setting it")
                                .clone(),
                        )));
                        self_.menu_stack.set_visible_child(&*self_.live_slider_box);
                    } else {
                        match self_.evt_tx.get() {
                            Some(tx) => {
                                if let Some(evt) = obj.mk_details_range() {
                                    let _ = tx.send(evt);
                                }
                            }
                                None => {}
                        }
                        self_.menu_stack.set_visible_child(&*self_.advanced_menu);
                    }
                }));
        }
    }

    impl BoxImpl for SensorDetail {}
    impl WidgetImpl for SensorDetail {}
}

glib::wrapper! {
    pub struct SensorDetail(ObjectSubclass<imp::SensorDetail>) @extends gtk::Box, gtk::Widget;
}

impl SensorDetail {
    pub fn init(&self, tx: glib::Sender<crate::event_loop::Event>) {
        let self_ = imp::SensorDetail::from_instance(&self);
        let ctx = glib::MainContext::default();
        let (change_tx, mut change_rx) = tokio::sync::mpsc::channel(1);
        self_.from_picker.connect_notify_local(
            Some("datetime"),
            glib::clone!(@strong change_tx => move |_, _| {
                change_tx.blocking_send(()).unwrap();
            }),
        );

        self_.to_picker.connect_notify_local(
            Some("datetime"),
            glib::clone!(@strong change_tx => move |_, _| {
                change_tx.blocking_send(()).unwrap();
            }),
        );

        let this = self.clone();
        ctx.spawn_local({
            let tx = tx.clone();
            async move {
                loop {
                    change_rx.recv().await;
                    let mut timeout = glib::timeout_future(Duration::from_millis(500));
                    loop {
                        tokio::select! {
                            Some(()) = change_rx.recv() => {
                                timeout = glib::timeout_future(Duration::from_millis(500));
                            }
                            () = &mut timeout => {
                                if let Some(evt) = this.mk_details_range() {
                                    tx.send(evt).unwrap();
                                }
                                break;
                            }
                        }
                    }
                }
            }
        });

        self_.change_tx.set(change_tx).unwrap();
        self_.evt_tx.set(tx).unwrap();
    }

    fn mk_details_range(&self) -> Option<Event> {
        let self_ = imp::SensorDetail::from_instance(self);

        let sensor = self_.sensor.borrow();
        let id = sensor
            .as_ref()
            .map(|sensor| sensor.id())
            .expect("SensorDetail missing sensor");
        let from = self_.from_picker.datetime();
        let to = self_.to_picker.datetime();
        if from > to {
            tracing::error!("Specified invalid time range");
            None
        } else {
            Some(Event::DetailRangeChanged {
                id,
                from: self_.from_picker.datetime(),
                to: self_.to_picker.datetime(),
            })
        }
    }

    pub fn stack(&self) -> &gtk::Stack {
        let self_ = imp::SensorDetail::from_instance(&self);
        &self_.detail_stack
    }

    pub fn set_sensor(&self, sensor: Option<SensorObj>) {
        let self_ = imp::SensorDetail::from_instance(&self);
        self_.sensor.replace(sensor);
    }

    pub fn set_static_timeseries(&self, timeseries: Timeseries) {
        let self_ = imp::SensorDetail::from_instance(&self);
        self_
            .painter
            .set_displayed_data(Some(Data::Static(timeseries)));
    }

    pub fn set_live(&self, live: bool) {
        let self_ = imp::SensorDetail::from_instance(&self);
        self_.live_switch.set_active(live);
    }
}
