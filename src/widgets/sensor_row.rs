use std::convert::TryFrom;

use crate::{
    event_loop::{Event, Label},
    sensor_obj::SensorObj,
};
use glib::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

mod imp {
    use std::cell::RefCell;

    use super::*;
    use adw::subclass::prelude::BinImpl;
    use glib::SignalHandlerId;
    use gtk::subclass::prelude::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/sensor_row.ui")]
    pub struct SensorRow {
        #[template_child]
        pub temperature: TemplateChild<gtk::Label>,
        #[template_child]
        pub humidity: TemplateChild<gtk::Label>,
        #[template_child]
        pub pressure: TemplateChild<gtk::Label>,
        #[template_child]
        pub sensor_label: TemplateChild<gtk::EditableLabel>,
        #[template_child]
        pub sensor_id: TemplateChild<gtk::Label>,
        #[template_child]
        pub info_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub detail_button: TemplateChild<gtk::Button>,

        pub sensor_obj: RefCell<Option<SensorObj>>,
        pub bindings: RefCell<Vec<glib::Binding>>,
        pub edit_handler_id: RefCell<Option<SignalHandlerId>>,

        pub tx: RefCell<Option<glib::Sender<Event>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorRow {
        const NAME: &'static str = "BleWsSensorRow";
        type Type = super::SensorRow;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SensorRow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl BinImpl for SensorRow {}
    impl WidgetImpl for SensorRow {}
}

glib::wrapper! {
    pub struct SensorRow(ObjectSubclass<imp::SensorRow>) @extends gtk::Widget, gtk::Box;
}

impl SensorRow {
    pub fn connect(tx: glib::Sender<Event>) -> Self {
        let ret: Self = glib::Object::new(&[]).unwrap();
        let self_ = imp::SensorRow::from_instance(&ret);
        self_.tx.replace(Some(tx.clone()));
        self_
            .detail_button
            .connect_clicked(glib::clone!(@weak ret, @strong tx => move |_| {
                let self_ = imp::SensorRow::from_instance(&ret);
                let obj = self_.sensor_obj.borrow();
                if let Some(obj) = &*obj {
                    tx.send(Event::OpenDetail(obj.id())).unwrap();
                }
            }));
        ret
    }

    pub fn connect_sensor_obj(&self, sensor_obj: &SensorObj) {
        let self_ = imp::SensorRow::from_instance(self);
        let mut bindings = self_.bindings.borrow_mut();
        // FIXME: trying to do this generically is a major pain, think carefully
        // before trying to refactor this
        let binding = sensor_obj
            .bind_property("temperature", &*self_.temperature, "label")
            .transform_to(|_, val| {
                let val = val.get::<i32>().unwrap();
                let converted = ble_ws_api::data::Celsius::try_from(val as i16)
                    .unwrap()
                    .to_string();
                Some(converted.to_value())
            })
            .build();
        bindings.push(binding.unwrap());

        let binding = sensor_obj
            .bind_property("pressure", &*self_.pressure, "label")
            .transform_to(|_, val| {
                let val = val.get::<u32>().unwrap();
                let converted = ble_ws_api::data::Pascal::from(val).to_string();
                Some(converted.to_value())
            })
            .build();
        bindings.push(binding.unwrap());

        let binding = sensor_obj
            .bind_property("humidity", &*self_.humidity, "label")
            .transform_to(|_, val| {
                let val = val.get::<u32>().unwrap();
                let converted = ble_ws_api::data::RelativeHumidity::try_from(val as u16)
                    .unwrap()
                    .to_string();
                Some(converted.to_value())
            })
            .build();

        bindings.push(binding.unwrap());

        let binding = sensor_obj
            .bind_property("label", &*self_.sensor_label, "text")
            .transform_to(|_, val| {
                let val = val.get::<Option<String>>().unwrap();
                Some(val.unwrap_or_default().to_value())
            })
            .build();
        bindings.push(binding.unwrap());

        let binding = sensor_obj
            .bind_property("id", &*self_.sensor_id, "label")
            .build();
        bindings.push(binding.unwrap());

        let binding = sensor_obj
            .bind_property("connected", &*self_.info_stack, "visible-child-name")
            .transform_to(|_, val| {
                let val = val.get::<bool>().unwrap();
                Some(if val { "connected" } else { "disconnected" }.to_value())
            })
            .build();
        bindings.push(binding.unwrap());

        // just pretend the properties changed so we don't have to do the
        // conversion dance all over again
        sensor_obj.notify("connected");
        sensor_obj.notify("temperature");
        sensor_obj.notify("pressure");
        sensor_obj.notify("humidity");
        sensor_obj.notify("id");
        sensor_obj.notify("label");

        let tx = self_.tx.borrow().as_ref().unwrap().clone();
        let id = sensor_obj.id();
        let edit_handler_id = self_.sensor_label.connect_editing_notify(move |label| {
            if !label.is_editing() {
                tx.send(Event::ChangeLabel {
                    id,
                    label: Label::from(label.text().to_string()),
                })
                .unwrap();
            }
        });
        self_.edit_handler_id.replace(Some(edit_handler_id));

        *self_.sensor_obj.borrow_mut() = Some(sensor_obj.clone());
    }

    pub fn disconnect_sensor_obj(&self, sensor_obj: &SensorObj) {
        let self_ = imp::SensorRow::from_instance(self);
        let mut bindings = self_.bindings.borrow_mut();
        for binding in bindings.drain(0..) {
            if binding.source().as_ref() != Some(sensor_obj.as_ref()) {
                panic!("Tried to unbind wrong SensorObj");
            }
            binding.unbind();
        }

        if let Some(edit_handler_id) = self_.edit_handler_id.borrow_mut().take() {
            self_.sensor_label.disconnect(edit_handler_id);
        }
        self_.edit_handler_id.replace(None);

        *self_.sensor_obj.borrow_mut() = None;
    }
}
