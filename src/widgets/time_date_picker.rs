use gtk::{prelude::*, subclass::prelude::ObjectSubclassExt};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

#[derive(glib::GBoxed, Clone)]
#[gboxed(type_name = "BleWsBoxedDateTime")]
pub struct BoxedDateTime(Box<OffsetDateTime>);

impl BoxedDateTime {
    pub fn new(dt: OffsetDateTime) -> Self {
        Self(Box::new(dt))
    }

    pub fn inner(&self) -> OffsetDateTime {
        *(&*self.0)
    }
}

mod imp {
    use super::*;
    use adw::subclass::prelude::*;
    use glib::ParamFlags;
    use gtk::{subclass::prelude::*, CompositeTemplate, Inhibit};
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/time_date_picker.ui")]
    pub struct TimeDatePicker {
        #[template_child]
        pub hour: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub minute: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub calendar_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub calendar: TemplateChild<gtk::Calendar>,

        pub datetime: Cell<OffsetDateTime>,
    }

    impl Default for TimeDatePicker {
        fn default() -> Self {
            Self {
                hour: Default::default(),
                minute: Default::default(),
                calendar_label: Default::default(),
                calendar: Default::default(),
                datetime: Cell::new(crate::util::now_local()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimeDatePicker {
        const NAME: &'static str = "BleWsTimeDatePicker";
        type Type = super::TimeDatePicker;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TimeDatePicker {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.refresh_date();

            // FIXME: bad way to prevent label from annoying resizing
            self.calendar_label
                .set_width_chars((self.calendar_label.label().as_str().len() + 2) as i32);

            // FIXME: terrible!
            self.calendar
                .connect_day_notify(glib::clone!(@weak obj => move |_| obj.refresh_date()));
            self.calendar
                .connect_month_notify(glib::clone!(@weak obj => move |_| obj.refresh_date()));
            self.calendar
                .connect_year_notify(glib::clone!(@weak obj => move |_| obj.refresh_date()));
            self.minute
                .connect_value_changed(glib::clone!(@weak obj => move |_| obj.refresh_date()));
            self.hour
                .connect_value_changed(glib::clone!(@weak obj => move |_| obj.refresh_date()));

            let fmt_time = |button: &gtk::SpinButton| {
                let adjustment = button.adjustment();
                let v = adjustment.value() as i32;
                button.set_text(&format!("{:02}", v));
                Inhibit(true)
            };

            self.minute.connect_output(fmt_time);
            self.hour.connect_output(fmt_time);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpec::new_boxed(
                    "datetime",
                    "datetime",
                    "datetime",
                    BoxedDateTime::static_type(),
                    ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "datetime" => BoxedDateTime(Box::new(self.datetime.get())).to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "datetime" => {
                    let dt: BoxedDateTime = value.get().unwrap();
                    let dt = dt.inner();
                    self.datetime.replace(dt.clone());
                    // did noboy ever use GtkCalendar?
                    self.calendar.set_year(dt.year());
                    self.calendar.set_month(i32::from(dt.month() as u8 - 1));
                    self.calendar.set_day(i32::from(dt.day()) - 1);
                    self.hour.set_value(dt.hour() as f64);
                    self.minute.set_value(dt.minute() as f64);
                }
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for TimeDatePicker {}
    impl BinImpl for TimeDatePicker {}
}

glib::wrapper! {
    pub struct TimeDatePicker(ObjectSubclass<imp::TimeDatePicker>) @extends gtk::Widget;
}

impl TimeDatePicker {
    pub fn datetime(&self) -> OffsetDateTime {
        let self_ = imp::TimeDatePicker::from_instance(self);
        self_.datetime.get()
    }

    pub fn set_datetime(&self, dt: OffsetDateTime) {
        self.set_property("datetime", &BoxedDateTime::new(dt))
            .unwrap();
    }

    fn refresh_date(&self) {
        let self_ = imp::TimeDatePicker::from_instance(self);
        let date = self_.calendar.date();
        let dt = PrimitiveDateTime::new(
            time::Date::from_ordinal_date(date.year(), date.day_of_year() as u16).unwrap(),
            time::Time::from_hms(
                self_.hour.value_as_int() as u8,
                self_.minute.value_as_int() as u8,
                0,
            )
            .unwrap(),
        )
        .assume_offset(UtcOffset::current_local_offset().unwrap());

        let s = dt
            .format(&time::macros::format_description!(
                "[year]-[month]-[day] [hour]:[minute]"
            ))
            .unwrap();
        self_.calendar_label.set_label(s.as_str());
        self_.datetime.set(dt);
        self.notify("datetime");
    }
}
