mod application;
mod config;
mod data;
mod event_loop;
mod sensor_name_filter;
mod sensor_obj;
mod services;
mod util;
mod widgets;

use gtk::prelude::*;
use tracing_subscriber::EnvFilter;

fn main() {
    if let None = std::env::var_os("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    glib::set_application_name("BleWsGtk");
    glib::set_prgname(Some("BleWsGtk"));

    gtk::init().expect("Unable to start GTK4");

    gio::resources_register_include!("compiled.gresource").unwrap();

    let application = application::BleWsGtk::new();
    std::process::exit(application.run())
}
