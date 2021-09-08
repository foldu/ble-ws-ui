use crate::config::APP_ID;
use gtk::prelude::*;

mod imp {
    use crate::event_loop::Event;

    use super::*;
    use glib::WeakRef;
    use gtk::subclass::prelude::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, Default)]
    pub struct BleWsGtk {
        window: OnceCell<WeakRef<crate::widgets::Window>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BleWsGtk {
        const NAME: &'static str = "BleWsGtkApp";
        type Type = super::BleWsGtk;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for BleWsGtk {}
    impl ApplicationImpl for BleWsGtk {
        fn startup(&self, app: &Self::Type) {
            self.parent_startup(app);
            adw::init();
        }

        fn activate(&self, app: &Self::Type) {
            // GtkApplication can connect to an already running app and send the activate signal
            // the below block just raises the already open instead of creating a new window
            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            if let Some(ref display) = gtk::gdk::Display::default() {
                let p = gtk::CssProvider::new();
                gtk::CssProvider::load_from_resource(&p, "/li/_5kw/BleWsGtk/style.css");
                gtk::StyleContext::add_provider_for_display(display, &p, 500);
                let theme = gtk::IconTheme::for_display(display).unwrap();
                theme.add_resource_path("/li/_5kw/BleWsGtk/icons/");
            }

            let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            gtk_macros::action!(
                app,
                "preferences",
                glib::clone!(@weak app  => move |_, _| {
                    let active_window = app.active_window().unwrap();
                    let preferences = crate::widgets::PreferencesWindow::new();
                    preferences.set_transient_for(Some(&active_window));
                    preferences.show();
                })
            );

            gtk_macros::action!(
                app,
                "search",
                glib::clone!(@weak app, @strong tx => move |_,_| {
                    tx.send(Event::OpenSearch).unwrap();
                })
            );

            app.set_accels_for_action("app.preferences", &["<primary>p"]);
            app.set_accels_for_action("app.search", &["<primary>f"]);
            app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);

            let window = crate::widgets::Window::connect(app, tx.clone());
            self.window.set(window.downgrade()).unwrap();

            crate::event_loop::attach(tx, rx, window.clone());

            window.show();
        }
    }
    impl GtkApplicationImpl for BleWsGtk {}
}

glib::wrapper! {
    pub struct BleWsGtk(ObjectSubclass<imp::BleWsGtk>) @extends gio::Application, gtk::Application, gio::ActionMap;
}

impl Default for BleWsGtk {
    fn default() -> Self {
        Self::new()
    }
}

impl BleWsGtk {
    pub fn new() -> Self {
        let app: Self = glib::Object::new(&[
            ("application-id", &Some(APP_ID)),
            ("flags", &gio::ApplicationFlags::empty()),
            ("resource-base-path", &Some("/li/_5kw/BleWsGtk")),
        ])
        .expect("Failed to create application instance");

        app
    }
}
