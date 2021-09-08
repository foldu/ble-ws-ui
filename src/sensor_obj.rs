use ble_ws_api::data::Timestamp;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use uuid::Uuid;

use crate::data::{SharedTimeseries, Timeseries, TimeseriesRow};

pub mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub struct SensorObj {
        pub label: RefCell<Option<String>>,
        pub id: RefCell<String>,
        pub uuid: Cell<Uuid>,
        pub connected: Cell<bool>,
        pub temperature: Cell<i32>,
        pub pressure: Cell<u32>,
        pub humidity: Cell<u32>,
        pub live_timeseries: RefCell<SharedTimeseries>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorObj {
        const NAME: &'static str = "BleWsSensor";
        type Type = super::SensorObj;
        type ParentType = glib::Object;
        type Interfaces = ();
    }

    impl ObjectImpl for SensorObj {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "label",
                        "Label",
                        "Label",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "id",
                        "Id",
                        "Id",
                        Some(""),
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boolean(
                        "connected",
                        "Connected",
                        "Connnected",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_int(
                        "temperature",
                        "Temperature",
                        "Temperature",
                        i32::MIN,
                        i32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_uint(
                        "humidity",
                        "Humidity",
                        "Humidity",
                        u32::MIN,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_uint(
                        "pressure",
                        "Pressure",
                        "Pressure",
                        u32::MIN,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boxed(
                        "live-timeseries",
                        "Live timeseries",
                        "Live timeseries",
                        crate::data::SharedTimeseries::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "label" => {
                    let label = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.label.replace(label);
                }
                "id" => {
                    let id = value
                        .get::<String>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.uuid.replace(Uuid::parse_str(&id).unwrap());
                    self.id.replace(id);
                }
                "connected" => {
                    let connected = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.connected.replace(connected);
                }
                "temperature" => {
                    let temperature = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.temperature.replace(temperature);
                }
                "humidity" => {
                    let humidity = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.humidity.replace(humidity);
                }
                "pressure" => {
                    let pressure = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.pressure.replace(pressure);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "connected" => self.connected.get().to_value(),
                "temperature" => self.temperature.get().to_value(),
                "humidity" => self.humidity.get().to_value(),
                "pressure" => self.pressure.get().to_value(),
                "live-timeseries" => self.live_timeseries.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }
}

glib::wrapper! {
    pub struct SensorObj(ObjectSubclass<imp::SensorObj>);
}

impl SensorObj {
    pub fn new(id: Uuid) -> Self {
        let ret: Self =
            glib::Object::new(&[("id", &id.to_string())]).expect("Can't create SensorObj");
        ret
    }

    pub fn id(&self) -> Uuid {
        let self_ = imp::SensorObj::from_instance(self);
        self_.uuid.get()
    }

    pub fn timeseries(&self) -> SharedTimeseries {
        let self_ = imp::SensorObj::from_instance(self);
        self_.live_timeseries.borrow().clone()
    }

    pub fn set_timeseries(&self, timeseries: Timeseries) {
        let self_ = imp::SensorObj::from_instance(self);
        self_
            .live_timeseries
            .replace(SharedTimeseries::new(timeseries));
        self.notify("live-timeseries");
    }

    // kind of defeats the purpose of encapsulation but I don't want to clone strings
    // all the time when accessing label
    pub fn data(&self) -> &imp::SensorObj {
        imp::SensorObj::from_instance(self)
    }

    pub fn update_values(&self, sensor_values: ble_ws_api::proto::SensorOverview) {
        let self_ = imp::SensorObj::from_instance(self);
        let label = sensor_values.label.map(|label| label.name);
        if &*self_.label.borrow() != &label {
            self.set_property("label", &label).unwrap();
        }
        match sensor_values.values {
            Some(values) => {
                self.set_properties(&[
                    ("connected", &true),
                    ("temperature", &values.temperature),
                    ("humidity", &values.humidity),
                    ("pressure", &values.pressure),
                ])
                .unwrap();

                let live_timeseries = self_.live_timeseries.borrow_mut();
                if let Some(timeseries) = &*live_timeseries.0 {
                    let mut timeseries = timeseries.borrow_mut();
                    let res = timeseries.push_row(TimeseriesRow {
                        // BUG: FIXME: fuckkkkkkkkkkkkkkk
                        time: Timestamp::now().as_u32(),
                        temperature: values.temperature as i16,
                        humidity: values.humidity,
                        pressure: values.pressure,
                    });
                    drop(timeseries);
                    drop(live_timeseries);
                    if res.is_ok() {
                        self.notify("live-timeseries");
                    }
                }
            }
            None => self.set_property("connected", &false).unwrap(),
        };
    }
}
