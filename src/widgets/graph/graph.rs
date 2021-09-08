use super::graph_painter::GraphPainter;
use gtk::{prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/li/_5kw/BleWsGtk/graph.ui")]
    pub struct Graph {
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        pub painter: RefCell<crate::widgets::graph::GraphPainter>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "BleWsGraph";
        type Type = super::Graph;
        type ParentType = adw::Bin;
        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Graph {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.picture.set_paintable(Some(&*self.painter.borrow()));
            obj.add_css_class("ble-ws-graph");
        }
    }

    impl WidgetImpl for Graph {}

    impl BinImpl for Graph {}
}

glib::wrapper! {
    pub struct Graph(ObjectSubclass<imp::Graph>) @ extends adw::Bin, gtk::Widget;
}

fn to_plotters_color(color: gtk::gdk::RGBA) -> plotters::style::RGBColor {
    plotters::style::RGBColor(
        (color.red * 255.) as u8,
        (color.green * 255.) as u8,
        (color.blue * 255.) as u8,
    )
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn painter(&self) -> GraphPainter {
        let self_ = imp::Graph::from_instance(&self);
        self_.painter.borrow().clone()
    }

    pub fn set_painter(&self, painter: &GraphPainter) {
        let self_ = imp::Graph::from_instance(&self);
        let style = self.style_context();
        let grid = to_plotters_color(style.color());
        painter.set_grid_color(grid);
        self_.picture.set_paintable(Some(painter));
        self_.painter.replace(painter.clone());
    }
}

impl Default for Graph {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Can't create Graph object")
    }
}
