use super::{graph::Data, SensorDetail, SensorOverview};
use crate::{
    data::Timeseries,
    event_loop::{Event, View},
};
use adw::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

mod imp {
    use std::cell::Cell;

    use super::*;
    use glib::signal::Inhibit;
    use gtk::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/window.ui")]
    pub struct Window {
        #[template_child]
        pub previous_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub overview_graph_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub sensor_overview: TemplateChild<SensorOverview>,
        #[template_child]
        pub sensor_detail: TemplateChild<SensorDetail>,
        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub view_switcher_title: TemplateChild<adw::ViewSwitcherTitle>,
        #[template_child]
        pub burger_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "BleWsGtkWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                previous_button: Default::default(),
                overview_graph_button: Default::default(),
                search_button: Default::default(),
                sensor_overview: Default::default(),
                sensor_detail: Default::default(),
                main_stack: Default::default(),
                view_switcher_title: Default::default(),
                burger_button: Default::default(),
                settings: crate::config::settings(),
                search_bar: Default::default(),
                search_entry: Default::default(),
            }
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.view_switcher_title
                .set_stack(Some(&self.sensor_detail.stack()));
            self.burger_button
                .popover()
                .unwrap()
                .set_halign(gtk::Align::End);

            self.search_button
                .connect_clicked(glib::clone!(@weak obj => move |_| {
                    let app = obj.application().unwrap();
                    app.activate_action("search", None);
                }));
            self.search_entry
                .connect_stop_search(glib::clone!(@weak obj => move |_| {
                    let app = obj.application().unwrap();
                    app.activate_action("search", None);
                }));

            self.search_button
                .bind_property("active", &*self.search_bar, "search-mode-enabled")
                .build();

            self.search_entry
                .connect_search_changed(glib::clone!(@weak obj => move |entry| {
                    let self_ = Self::from_instance(&obj);
                    self_.sensor_overview.set_filter(entry.text().as_str());
                }));
            obj.load_window_state();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {
        // save window state on delete event
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            if let Err(err) = obj.save_window_state() {
                tracing::error!("Failed to save window state, {}", &err);
            }
            Inhibit(false)
        }
    }
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl Window {
    pub fn connect<P: glib::IsA<gtk::Application>>(app: &P, tx: glib::Sender<Event>) -> Self {
        let ret: Self =
            glib::Object::new(&[("application", app)]).expect("Failed to create Window");
        let self_ = imp::Window::from_instance(&ret);
        self_.sensor_overview.connect(tx.clone());
        self_
            .previous_button
            .connect_clicked(glib::clone!(@strong tx => move |_| {
                tx.send(Event::OpenOverview).unwrap();
            }));
        self_.sensor_detail.init(tx.clone());

        let builder = gtk::Builder::from_resource("/li/_5kw/BleWsGtk/shortcuts.ui");
        gtk_macros::get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        ret.set_help_overlay(Some(&shortcuts));

        ret
    }

    pub fn deactivate_search(&self) {
        let self_ = imp::Window::from_instance(self);
        self_.sensor_overview.set_filter("");
        self_.search_entry.set_text("");
        self_.search_button.set_active(false);
    }

    pub fn activate_search(&self) {
        let self_ = imp::Window::from_instance(self);
        self_.search_button.set_active(true);
    }

    pub fn set_static_timeseries(&self, timeseries: Timeseries) {
        let self_ = imp::Window::from_instance(self);
        self_.sensor_detail.set_static_timeseries(timeseries);
    }

    pub fn switch_view(&self, view: &View) {
        let self_ = imp::Window::from_instance(self);
        match view {
            View::Overview {
                selected_sensor: None,
                ..
            } => {
                self.deactivate_search();
                self_.view_switcher_title.set_view_switcher_enabled(false);
                self_.main_stack.set_visible_child(&*self_.sensor_overview);
                self_.previous_button.set_visible(false);
                self_.overview_graph_button.set_visible(true);
                self_.search_button.set_visible(true);
                self_.sensor_overview.set_displayed_sensor(None);
            }
            View::Overview {
                selected_sensor: Some(sensor),
                ..
            } => {
                self_
                    .sensor_overview
                    .set_displayed_sensor(Some(sensor.clone()));
            }
            View::Detail { sensor, .. } => {
                self.deactivate_search();
                self_.view_switcher_title.set_view_switcher_enabled(true);
                self_.main_stack.set_visible_child(&*self_.sensor_detail);
                self_.previous_button.set_visible(true);
                self_.overview_graph_button.set_visible(false);
                self_.search_button.set_visible(false);
                self_.sensor_detail.set_sensor(Some(sensor.clone()));
                self_.sensor_detail.set_live(true);
            }
        }
    }

    pub fn add_sensor(&self, obj: &crate::sensor_obj::SensorObj) {
        let self_ = imp::Window::from_instance(self);
        self_.sensor_overview.add_sensor(obj);
    }

    pub fn save_window_state(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::Window::from_instance(self).settings;

        let size = self.default_size();

        settings
            .set::<(i32, i32, bool)>("last-window-state", &(size.0, size.1, self.is_maximized()))?;

        Ok(())
    }

    fn load_window_state(&self) {
        let settings = &imp::Window::from_instance(self).settings;

        let (width, height, is_maximized) = settings.get::<(i32, i32, bool)>("last-window-state");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }
}
