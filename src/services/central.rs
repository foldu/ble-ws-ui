use crate::{
    data::{Timeseries, TimeseriesBuilder},
    event_loop::{Event, Label},
};
use ble_ws_api::{
    data::Timestamp,
    proto::{
        self,
        ble_weatherstation_service_client::BleWeatherstationServiceClient,
        OverviewRequest,
        OverviewResponse,
        SensorOverview,
        SubscribeToChangesRequest,
    },
};
use futures_util::StreamExt;
use tokio::runtime::Handle;
use tonic::{codegen::InterceptedService, service::Interceptor, transport::Channel, Request};
use url::Url;
use uuid::Uuid;

pub type Token = tonic::metadata::MetadataValue<tonic::metadata::Ascii>;

#[derive(Clone)]
pub struct Central {
    tx: tokio::sync::mpsc::Sender<Command>,
}

#[derive(Debug)]
enum Command {
    FetchTimeseries(TimeseriesRequest),
    SetEndpoint { url: Url, token: Token },
    SetLabel { label: Label, id: Uuid },
}

impl super::Service for Central {
    fn create(
        handle: &Handle,
        mut glib_sender: glib::Sender<Event>,
    ) -> Result<Self, anyhow::Error> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        handle.spawn(async move {
            let mut client = None;
            while let Some(cmd) = rx.recv().await {
                if let Err(e) = handle_cmd(&mut client, &mut glib_sender, cmd).await {
                    tracing::error!("{}", e);
                }
            }
        });

        Ok(Self { tx })
    }
}

struct ClientHandle {
    client: BleWeatherstationServiceClient<InterceptedService<Channel, AuthInterceptor>>,
    subscribe_task: tokio::task::JoinHandle<()>,
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.subscribe_task.abort();
        drop(&mut self.client);
    }
}

struct AuthInterceptor {
    token: Token,
}

impl Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        request
            .metadata_mut()
            .insert("authorization", self.token.clone());
        Ok(request)
    }
}

async fn handle_cmd(
    client_handle: &mut Option<ClientHandle>,
    sender: &mut glib::Sender<Event>,
    cmd: Command,
) -> Result<(), anyhow::Error> {
    match cmd {
        Command::SetLabel { id, label } => {
            if let Some(client_handle) = client_handle {
                client_handle
                    .client
                    .change_label(Request::new(ble_ws_api::proto::ChangeLabelRequest {
                        id: Some(ble_ws_api::proto::Uuid::from(id)),
                        label: label
                            .into_inner()
                            .map(|label| ble_ws_api::proto::Label { name: label }),
                    }))
                    .await
                    .unwrap();
            }
        }

        Command::FetchTimeseries(kind) => {
            if let Some(client_handle) = client_handle {
                let (&id, req) = match &kind {
                    TimeseriesRequest::Live(id) => (
                        id,
                        ble_ws_api::proto::SensorDataRequest {
                            id: Some(proto::Uuid::from(*id)),
                            start: Timestamp::now().bottoming_sub(Timestamp::ONE_DAY).as_u32(),
                            end: u32::MAX,
                        },
                    ),
                    TimeseriesRequest::Range { id, range } => (
                        id,
                        ble_ws_api::proto::SensorDataRequest {
                            id: Some(proto::Uuid::from(*id)),
                            start: range.start().as_u32(),
                            end: range.end().as_u32(),
                        },
                    ),
                };
                let resp = client_handle
                    .client
                    .get_sensor_data(req)
                    .await?
                    .into_inner();
                let timeseries = TimeseriesBuilder::default()
                    .time(resp.time)
                    .temperature(resp.temperature.into_iter().map(|n| n as i16).collect())
                    .humidity(resp.humidity)
                    .pressure(resp.pressure)
                    .build();
                match timeseries {
                    Ok(timeseries) => {
                        sender
                            .send(Event::FetchedTimeseries {
                                id,
                                timeseries: match kind {
                                    TimeseriesRequest::Live(_) => {
                                        TimeseriesResponse::Live(timeseries)
                                    }
                                    TimeseriesRequest::Range { .. } => {
                                        TimeseriesResponse::Range(timeseries)
                                    }
                                },
                            })
                            .unwrap();
                    }
                    Err(_) => {
                        tracing::error!("Received invalid length timeseries from endpoint");
                    }
                }
            }
        }
        Command::SetEndpoint { url, token } => {
            // TODO: make name less obnoxious
            let channel = tonic::transport::Channel::from_shared(url.to_string())?
                .connect()
                .await?;
            let mut new_client = BleWeatherstationServiceClient::with_interceptor(
                channel,
                AuthInterceptor { token },
            );
            tracing::info!("Connected to {}", url);
            let overview = new_client.overview(OverviewRequest {}).await?.into_inner();
            sender
                .send(Event::OverviewUpdate(overview_transform(overview)))
                .unwrap();
            let stream = new_client
                .subscribe_to_changes(SubscribeToChangesRequest {})
                .await?;
            let subscribe_task = tokio::task::spawn({
                let sender = sender.clone();
                async move {
                    let mut stream = stream.into_inner();
                    while let Some(update) = stream.next().await {
                        match update {
                            Ok(update) => {
                                sender
                                    .send(Event::OverviewUpdate(overview_transform(update)))
                                    .unwrap();
                            }
                            Err(e) => {
                                tracing::error!("Subscription error: {}", e);
                            }
                        }
                    }
                }
            });
            *client_handle = Some(ClientHandle {
                subscribe_task,
                client: new_client,
            });
        }
    }

    Ok(())
}

fn overview_transform(resp: OverviewResponse) -> Vec<(Uuid, SensorOverview)> {
    resp.overview
        .into_iter()
        .map(|field| (Uuid::from(field.id.unwrap()), field.overview.unwrap()))
        .collect()
}

#[derive(Clone, Debug)]
pub enum TimeseriesRequest {
    Live(Uuid),
    Range {
        id: Uuid,
        range: std::ops::RangeInclusive<Timestamp>,
    },
}

pub enum TimeseriesResponse {
    Live(Timeseries),
    Range(Timeseries),
}

impl Central {
    fn send(&self, cmd: Command) {
        self.tx.blocking_send(cmd).expect("Central thread died");
    }

    pub fn fetch_timeseries(&self, req: TimeseriesRequest) {
        self.send(Command::FetchTimeseries(req));
    }

    pub fn set_endpoint(&self, url: url::Url, token: Token) {
        tracing::info!("Connecting to endpoint {}", url);
        self.send(Command::SetEndpoint { url, token });
    }

    pub fn set_label(&self, id: Uuid, label: Label) {
        self.send(Command::SetLabel { id, label });
    }
}
