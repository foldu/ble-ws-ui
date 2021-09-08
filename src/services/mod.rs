pub mod central;

use crate::event_loop::Event;
use tokio::{runtime::Handle, sync::oneshot};

pub struct ServiceManager {
    handle: Handle,
    glib_sender: glib::Sender<Event>,
}

pub trait Service: Sized {
    fn create(handle: &Handle, glib_sender: glib::Sender<Event>) -> Result<Self, anyhow::Error>;
}

impl ServiceManager {
    pub(crate) fn new(glib_sender: glib::Sender<Event>) -> Result<Self, anyhow::Error> {
        let (handle_tx, mut handle_rx) = oneshot::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            handle_tx.send(rt.handle().clone()).unwrap();
            rt.block_on(async move {
                futures_util::future::pending::<()>().await;
            });
        });

        let handle = loop {
            match handle_rx.try_recv() {
                Ok(handle) => break handle,
                Err(oneshot::error::TryRecvError::Empty) => (),
                _ => anyhow::bail!("Service manager thread could not send handle"),
            }
        };

        Ok(Self {
            handle,
            glib_sender,
        })
    }

    pub(crate) fn create_service<S>(&self) -> Result<S, anyhow::Error>
    where
        S: Service,
    {
        S::create(&self.handle, self.glib_sender.clone())
    }
}
