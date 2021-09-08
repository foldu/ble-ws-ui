use gio::Settings;

pub const APP_ID: &str = "li._5kw.BleWsGtk";

pub fn settings() -> Settings {
    Settings::new(APP_ID)
}
