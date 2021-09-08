use crate::sensor_obj::SensorObj;
use glib::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

use super::graph;

mod imp {
    use super::*;
    use crate::widgets::{
        graph::{GraphPainter, Unit},
        Graph,
    };
    use glib::{ParamFlags, ParamSpec, Value};
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/sensor_overview_detail.ui")]
    pub struct SensorOverviewDetail {
        #[template_child]
        sensor_label: TemplateChild<gtk::Label>,
        #[template_child]
        temperature_graph: TemplateChild<Graph>,
        #[template_child]
        humidity_graph: TemplateChild<Graph>,
        #[template_child]
        pressure_graph: TemplateChild<Graph>,

        pub painters: [GraphPainter; 3],
        displayed_sensor: RefCell<Option<SensorObj>>,
        sensor_binding: RefCell<Option<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorOverviewDetail {
        const NAME: &'static str = "BleWsSensorOverviewDetail";
        type Type = super::SensorOverviewDetail;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SensorOverviewDetail {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::new_object(
                    "sensor",
                    "sensor",
                    "Sensor",
                    SensorObj::static_type(),
                    ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "sensor" => {
                    let sensor = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.displayed_sensor.replace(sensor);
                    let sensor = self.displayed_sensor.borrow();
                    if let Some(sensor) = &*sensor {
                        let binding = sensor
                            .bind_property("label", &*self.sensor_label, "label")
                            .transform_to(|_, val| {
                                let val = val.get::<Option<String>>().unwrap();
                                Some(val.unwrap_or_default().to_value())
                            })
                            .build();
                        sensor.notify("label");

                        if let Some(old_binding) = self.sensor_binding.replace(binding) {
                            old_binding.unbind()
                        }
                    } else {
                        if let Some(binding) = self.sensor_binding.take() {
                            binding.unbind();
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "sensor" => self.displayed_sensor.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            for unit in &[Unit::Temperature, Unit::Humidity, Unit::Pressure] {
                let painter = &self.painters[unit.as_usize()];
                painter.set_displayed_unit(*unit);
            }
            self.temperature_graph
                .set_painter(&self.painters[Unit::Temperature.as_usize()]);
            self.humidity_graph
                .set_painter(&self.painters[Unit::Humidity.as_usize()]);
            self.pressure_graph
                .set_painter(&self.painters[Unit::Pressure.as_usize()]);
            self.parent_constructed(obj);
        }
    }

    impl BoxImpl for SensorOverviewDetail {}
    impl WidgetImpl for SensorOverviewDetail {}
}

glib::wrapper! {
    pub struct SensorOverviewDetail(ObjectSubclass<imp::SensorOverviewDetail>) @extends gtk::Box, gtk::Widget;
}

impl SensorOverviewDetail {
    pub fn set_displayed_sensor(&self, sensor: Option<SensorObj>) {
        let self_ = imp::SensorOverviewDetail::from_instance(self);
        self.set_property("sensor", &sensor).unwrap();
        for painter in &self_.painters {
            painter.set_displayed_data(sensor.clone().map(graph::Data::Live));
        }
    }
}
