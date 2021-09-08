use glib::subclass::prelude::*;
use gtk::prelude::*;

mod imp {
    use gtk::subclass::prelude::FilterImpl;
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct SensorNameFilter {
        pub filter: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SensorNameFilter {
        const NAME: &'static str = "BleWsSensorNameFilter";
        type Type = super::SensorNameFilter;
        type ParentType = gtk::Filter;
        type Interfaces = ();
    }

    impl ObjectImpl for SensorNameFilter {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl FilterImpl for SensorNameFilter {
        fn match_(&self, _filter: &Self::Type, item: &glib::Object) -> bool {
            let obj = item.downcast_ref::<crate::sensor_obj::SensorObj>().unwrap();
            let filter = self.filter.borrow();
            if filter.is_empty() {
                return true;
            }

            let label = obj.data().label.borrow();
            if let Some(label) = &*label {
                label.starts_with(&*filter)
            } else {
                false
            }
        }
    }
}

glib::wrapper! {
    pub struct SensorNameFilter(ObjectSubclass<imp::SensorNameFilter>) @ extends gtk::Filter;
}

impl SensorNameFilter {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Can't create SensorNameFilter")
    }

    pub fn set_filter(&self, filter: &str) {
        let self_ = imp::SensorNameFilter::from_instance(self);
        let mut my_filter = self_.filter.borrow_mut();
        if filter != &*my_filter {
            my_filter.clear();
            my_filter.push_str(filter);
            drop(my_filter);
            self.changed(gtk::FilterChange::Different);
        }
    }
}
