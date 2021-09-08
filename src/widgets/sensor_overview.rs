use super::SensorRow;
use crate::{event_loop::Event, sensor_obj::SensorObj};
use glib::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};
use std::{cell::RefCell, collections::BTreeMap};
use uuid::Uuid;

mod imp {
    use std::cell::Cell;

    use crate::sensor_name_filter::SensorNameFilter;

    use super::*;
    use adw::subclass::prelude::BinImpl;
    use gio::ListStore;
    use glib::{ParamSpec, Value};
    use gtk::{subclass::prelude::*, FilterListModel, NoSelection, SingleSelection};

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/sensor_overview.ui")]
    pub struct SensorOverview {
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub pane_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub overview_detail: TemplateChild<crate::widgets::SensorOverviewDetail>,

        pub sensor_name_filter: SensorNameFilter,
        pub selection_enabled: Cell<bool>,
        pub single_selection: gtk::SingleSelection,
        pub no_selection: gtk::NoSelection,
        pub filtered_model: gtk::FilterListModel,
        pub model: gio::ListStore,
        pub rows: RefCell<BTreeMap<Uuid, SensorRow>>,
    }

    impl Default for SensorOverview {
        fn default() -> Self {
            let model = ListStore::new(crate::sensor_obj::SensorObj::static_type());
            let sensor_name_filter = SensorNameFilter::new();
            let filtered_model = FilterListModel::new(Some(&model), Some(&sensor_name_filter));
            Self {
                list_view: Default::default(),
                single_selection: SingleSelection::new(Some(&filtered_model)),
                no_selection: NoSelection::new(Some(&filtered_model)),
                selection_enabled: Default::default(),
                model,
                sensor_name_filter,
                filtered_model,
                rows: Default::default(),
                leaflet: Default::default(),
                scrolled_window: Default::default(),
                pane_stack: Default::default(),
                status_page: Default::default(),
                overview_detail: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorOverview {
        const NAME: &'static str = "BleWsSensorOverview";
        type Type = super::SensorOverview;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SensorOverview {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.leaflet
                .connect_folded_notify(glib::clone!(@weak obj => move |leaflet| {
                    let self_ = Self::from_instance(&obj);
                    self_.scrolled_window.set_hexpand(leaflet.is_folded());
                    // NOTE: this is programmed weirdly because of generics in set_model
                    if leaflet.is_folded() {
                        self_.list_view.set_model(Some(&self_.no_selection));
                    } else {
                        self_.list_view.set_model(Some(&self_.single_selection));
                    }
                }));
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpec::new_boolean(
                    "selection-enabled",
                    "selection_enabled",
                    "Selection enabled",
                    false,
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "selection-enabled" => self.selection_enabled.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "selection-enabled" => {
                    let selection_enabled = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.selection_enabled.replace(selection_enabled);
                }
                _ => unimplemented!(),
            }
        }
    }

    impl BoxImpl for SensorOverview {}
    impl BinImpl for SensorOverview {}
    impl WidgetImpl for SensorOverview {}
}

glib::wrapper! {
    pub struct SensorOverview(ObjectSubclass<imp::SensorOverview>) @extends gtk::Box, gtk::Widget, adw::Bin;
}

impl SensorOverview {
    pub fn connect(&self, tx: glib::Sender<Event>) {
        self.setup_list(tx.clone());
    }

    pub fn set_displayed_sensor(&self, sensor: Option<SensorObj>) {
        let self_ = imp::SensorOverview::from_instance(self);
        if let None = sensor {
            self_.single_selection.set_selected(u32::MAX);
        }
        self_.overview_detail.set_displayed_sensor(sensor);
    }

    pub fn set_filter(&self, filter: &str) {
        let self_ = imp::SensorOverview::from_instance(self);
        self_.sensor_name_filter.set_filter(filter);
    }

    pub fn setup_list(&self, tx: glib::Sender<Event>) {
        let self_ = imp::SensorOverview::from_instance(self);
        self_.single_selection.set_autoselect(false);
        self_.single_selection.connect_selected_notify(
            glib::clone!(@weak self as this @strong tx => move |selection| {
                let self_ = imp::SensorOverview::from_instance(&this);
                match self_.model.item(selection.selected()) {
                    Some(obj) => {
                        let sensor = obj.downcast::<SensorObj>().unwrap();
                        tx.send(Event::SensorSelected(sensor.id())).unwrap();
                        tracing::trace!("Selected {}", sensor.id());
                        self_.pane_stack.set_visible_child(&*self_.overview_detail);
                    }
                    None => {
                        self_.pane_stack.set_visible_child(&*self_.status_page);
                    }
                }
            }),
        );
        // TODO: listen to selection_enabled and set model based on that
        self_.list_view.set_model(Some(&self_.single_selection));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            item.set_child(Some(&SensorRow::connect(tx.clone())));
        });

        factory.connect_bind(move |_factory, item| {
            let data = item.item().unwrap().downcast::<SensorObj>().unwrap();
            let row = item.child().unwrap().downcast::<SensorRow>().unwrap();

            row.connect_sensor_obj(&data);
        });

        factory.connect_unbind(move |_facotyr, item| {
            let data = item.item().unwrap().downcast::<SensorObj>().unwrap();
            let row = item.child().unwrap().downcast::<SensorRow>().unwrap();

            row.disconnect_sensor_obj(&data);
        });

        self_.list_view.set_factory(Some(&factory));
    }

    pub fn add_sensor(&self, sensor: &SensorObj) {
        let self_ = imp::SensorOverview::from_instance(self);
        self_.model.append(sensor);
    }
}
