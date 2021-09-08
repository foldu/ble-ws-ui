use gtk::{prelude::*, subclass::prelude::ObjectSubclassExt};

mod imp {
    use super::*;
    use glib::SignalHandlerId;
    use gtk::{subclass::prelude::*, CompositeTemplate};
    use adw::subclass::prelude::BinImpl;
    use std::cell::{Cell, RefCell};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/li/_5kw/BleWsGtk/validated_entry.ui")]
    pub struct ValidatedEntry {
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub msg: TemplateChild<gtk::Label>,
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub msg_revealer: TemplateChild<gtk::Revealer>,

        pub blocked: Cell<bool>,
        pub validator: RefCell<Option<Box<dyn Fn(&str) -> ValidatorResult>>>,
        pub activate_handler: RefCell<Option<SignalHandlerId>>,
        pub activate_cb: RefCell<Option<Box<(dyn Fn(&str))>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ValidatedEntry {
        const NAME: &'static str = "BleWsValidatedEntry";
        type Type = super::ValidatedEntry;
        // Bin until CenterBox can be subclassed
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ValidatedEntry {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let signal = self
                .entry
                .connect_activate(glib::clone!(@weak obj => move |entry| {
                    let self_ = Self::from_instance(&obj);
                    let text = entry.buffer().text();
                    let activate_cb = self_.activate_cb.borrow();
                    if let Some(cb) = &*activate_cb {
                        cb(&text)
                    }
                }));
            *self.activate_handler.borrow_mut() = Some(signal);

            self.entry
                .connect_changed(glib::clone!(@weak obj => move |entry| {
                    let self_ = Self::from_instance(&obj);
                    let validator = self_.validator.borrow() ;
                    if let Some(validator) = &*validator{
                        let text = entry.buffer().text();
                        let activate_handler = self_.activate_handler.borrow();
                        let activate_handler = activate_handler.as_ref().unwrap();
                        match validator(&text) {
                            ValidatorResult::Ok => {
                                if self_.blocked.replace(false) {
                                    entry.unblock_signal(&*activate_handler);
                                }
                                self_.msg_revealer.set_reveal_child(false);
                            }
                            ValidatorResult::Warning(s) => {
                                if !self_.blocked.replace(true) {
                                    entry.unblock_signal(&*activate_handler);
                                }
                                self_.msg_revealer.set_reveal_child(true);
                                self_.icon.set_from_icon_name(Some("dialog-warning-symbolic"));
                                self_.msg.set_text(s);
                            }
                            ValidatorResult::Error(s) => {
                                if !self_.blocked.replace(true) {
                                    entry.block_signal(&*activate_handler);
                                }
                                self_.msg_revealer.set_reveal_child(true);
                                self_.icon.set_from_icon_name(Some("dialog-error-symbolic"));
                                self_.msg.set_label(s);
                            }
                        }
                    }
                }));
        }
    }

    impl WidgetImpl for ValidatedEntry {}

    impl BinImpl for ValidatedEntry {}

    impl BoxImpl for ValidatedEntry {}
}

glib::wrapper! {
    pub struct ValidatedEntry(ObjectSubclass<imp::ValidatedEntry>)
        @extends gtk::Widget, gtk::Box;
}

pub enum ValidatorResult {
    Ok,
    Warning(&'static str),
    Error(&'static str),
}

impl ValidatedEntry {
    pub fn new() -> Self {
        let ret: Self = glib::Object::new(&[]).unwrap();
        ret
    }

    pub fn set_validator<F>(&self, f: F)
    where
        F: Fn(&str) -> ValidatorResult + 'static,
    {
        let self_ = imp::ValidatedEntry::from_instance(self);
        *self_.validator.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_activate<F>(&self, cb: F)
    where
        F: Fn(&str) + 'static,
    {
        let self_ = imp::ValidatedEntry::from_instance(self);
        *self_.activate_cb.borrow_mut() = Some(Box::new(cb));
    }

    pub fn entry(&self) -> &gtk::Entry {
        let self_ = imp::ValidatedEntry::from_instance(self);
        &self_.entry
    }
}
