use gtk::{prelude::*, subclass::prelude::ObjectSubclassExt};

use crate::widgets::validated_entry::ValidatorResult;
mod imp {
    use crate::widgets::validated_entry::ValidatorResult;

    use super::*;
    use gtk::{subclass::prelude::*, CompositeTemplate};
    use adw::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/preferences_window.ui")]
    pub struct PreferencesWindow {
        #[template_child(id = "dark_theme_switch")]
        pub dark_theme: TemplateChild<gtk::Switch>,
        #[template_child]
        pub endpoint: TemplateChild<crate::widgets::ValidatedEntry>,

        pub settings: gio::Settings,
    }

    impl Default for PreferencesWindow {
        fn default() -> Self {
            Self {
                dark_theme: Default::default(),
                endpoint: Default::default(),
                settings: crate::config::settings(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "BleWsPreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.settings
                .bind("dark-theme", &*self.dark_theme, "state")
                .build();
            let endpoint = self.settings.get::<String>("endpoint");
            if let ValidatorResult::Error(_) = validate_endpoint(&endpoint) {
                self.settings.set("endpoint", &"").unwrap();
                self.endpoint.entry().set_text(&"");
            } else {
                self.endpoint.entry().set_text(&endpoint);
            }

            self.endpoint
                .connect_activate(glib::clone!(@weak obj => move |text| {
                    let self_ = PreferencesWindow::from_instance(&obj);
                    tracing::info!("Set endpoint setting to {}", text);
                    self_.settings.set("endpoint", &text).unwrap();
                }));
            self.endpoint.set_validator(validate_endpoint);
        }
    }

    impl WidgetImpl for PreferencesWindow {}
    impl WindowImpl for PreferencesWindow {}
    impl AdwWindowImpl for PreferencesWindow {}
    impl PreferencesWindowImpl for PreferencesWindow {}
}

glib::wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow;
}

impl PreferencesWindow {
    pub fn new() -> Self {
        let ret: Self = glib::Object::new(&[]).unwrap();
        ret
    }
}

fn validate_endpoint(s: &str) -> ValidatorResult {
    let url = match url::Url::parse(s) {
        Ok(url) => url,
        Err(_) => return ValidatorResult::Error("Invalid url"),
    };

    match url.scheme() {
        "http" => ValidatorResult::Warning("http is insecure"),
        "https" => ValidatorResult::Ok,
        _ => ValidatorResult::Error("scheme needs to be either http or https"),
    }
}
