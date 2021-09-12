use crate::{
    data::{SharedTimeseries, Timeseries},
    sensor_obj::SensorObj,
};
use ble_ws_api::data::{Celsius, RelativeHumidity};
use gtk::{gdk, prelude::*, subclass::prelude::*};
use plotters::{
    coord::ranged1d::{AsRangedCoord, DefaultFormatting, Ranged},
    prelude::DrawingBackend,
    style::RGBColor,
};
use std::{cell::RefCell, convert::TryFrom, ops::Range};
use time::{macros::format_description, UtcOffset};

#[derive(Debug, Eq, PartialEq, Clone, Copy, glib::GEnum)]
#[repr(u8)]
#[genum(type_name = "BleWsGraphUnit")]
pub enum Unit {
    Temperature = 0,
    Humidity = 1,
    Pressure = 2,
}

impl Default for Unit {
    fn default() -> Self {
        Self::Temperature
    }
}

impl Unit {
    pub fn as_usize(self) -> usize {
        usize::from(self as u8)
    }

    pub fn label(self) -> &'static str {
        match self {
            Unit::Temperature => "Temperature",
            Unit::Humidity => "Humidity",
            Unit::Pressure => "Pressure",
        }
    }
}

mod imp {
    use super::*;
    use glib::SignalHandlerId;
    use std::cell::Cell;

    #[derive(Default)]
    pub struct GraphPainter {
        pub timeseries: RefCell<SharedTimeseries>,
        pub unit: Cell<Unit>,
        pub grid_color: Cell<plotters::style::RGBColor>,
        pub obj: RefCell<Option<SensorObj>>,
        pub live_binding: Cell<Option<SignalHandlerId>>,
        pub bounds: Cell<Option<(u32, u32)>>,
        pub time_format: Cell<TimeFormat>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphPainter {
        const NAME: &'static str = "BleWsGraphPainter";
        type Type = super::GraphPainter;
        type ParentType = glib::Object;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for GraphPainter {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl PaintableImpl for GraphPainter {
        fn snapshot(
            &self,
            _paintable: &Self::Type,
            snapshot: &gdk::Snapshot,
            width: f64,
            height: f64,
        ) {
            let timeseries = self.timeseries.borrow();
            match &*timeseries.0 {
                Some(timeseries) => {
                    let timeseries = timeseries.borrow();
                    if timeseries.is_empty() {
                        return;
                    }
                    let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
                    let bounding_box = (approx_to(width), approx_to(height));

                    let ctx = snapshot
                        .append_cairo(&gtk::graphene::Rect::new(
                            0.,
                            0.,
                            width as f32,
                            height as f32,
                        ))
                        .expect("Failed acquiring cairo context");

                    if let Err(e) = plot(
                        &ctx,
                        self.grid_color.get(),
                        &timeseries,
                        self.unit.get(),
                        self.bounds.get(),
                        bounding_box,
                        self.time_format.get(),
                    ) {
                        tracing::error!("{}", e);
                    }
                }
                _ => (),
            }
        }
    }
}

fn plot(
    ctx: &gtk::cairo::Context,
    grid_color: RGBColor,
    timeseries: &Timeseries,
    unit: Unit,
    bounds: Option<(u32, u32)>,
    bounding_box: (u32, u32),
    time_format: TimeFormat,
) -> Result<(), anyhow::Error> {
    use plotters::prelude::*;
    let root = plotters_cairo::CairoBackend::new(ctx, (bounding_box.0, bounding_box.1))
        .into_drawing_area();

    let (time, (start, end)) = match bounds {
        None => (timeseries.time(), (0, timeseries.time().len())),
        Some((lower, upper)) => {
            let time = timeseries.time();
            let start = match time.iter().position(|t| *t >= lower) {
                Some(i) => i,
                // empty, nothing to draw
                None => return Ok(()),
            };
            let sliced = &time[start..];
            let end = sliced
                .iter()
                .position(|t| *t >= upper)
                .unwrap_or(sliced.len());

            (&sliced[..end], (start, start + end))
        }
    };

    match unit {
        Unit::Temperature => {
            // FIXME: fork plotters and impl Rangedi16
            let hoopla = timeseries
                .temperature()
                .iter()
                .copied()
                .map(i32::from)
                .collect::<Vec<_>>();
            draw_graph(
                &root,
                &time,
                &hoopla[start..end],
                grid_color,
                FormatSpec {
                    color: RGBColor(178, 34, 34),
                    formatter: &|temp| Celsius::try_from(*temp as i16).unwrap().to_string(),
                    margin_px: 75,
                    time_format,
                },
            )?;
        }

        Unit::Humidity => {
            // FIXME: fork plotters and impl Rangedu16
            draw_graph(
                &root,
                &time,
                &timeseries.humidity()[start..end],
                grid_color,
                FormatSpec {
                    margin_px: 75,
                    formatter: &|humidity| {
                        RelativeHumidity::try_from(*humidity as u16)
                            .unwrap()
                            .to_string()
                    },
                    color: RGBColor(106, 90, 205),
                    time_format,
                },
            )?;
        }

        Unit::Pressure => {
            draw_graph(
                &root,
                &time,
                &timeseries.pressure()[start..end],
                grid_color,
                FormatSpec {
                    margin_px: 75,
                    formatter: &|pressure| {
                        let floating = pressure % 1000;
                        let other = pressure / 1000;
                        format!("{}.{:<2}hPa", floating, other)
                    },
                    color: RGBColor(0, 128, 0),
                    time_format,
                },
            )?;
        }
    }

    Ok(())
}

#[derive(Copy, Clone)]
pub enum TimeFormat {
    TimeOnly,
    DateTime,
}

impl Default for TimeFormat {
    fn default() -> Self {
        TimeFormat::DateTime
    }
}

struct FormatSpec<'a, N, C> {
    margin_px: u32,
    formatter: &'a dyn Fn(&N) -> String,
    color: C,
    time_format: TimeFormat,
}

fn draw_graph<DB, N, GridColor, GraphColor>(
    area: &plotters::prelude::DrawingArea<DB, plotters::coord::Shift>,
    timestamps: &[u32],
    nums: &[N],
    grid_color: GridColor,
    spec: FormatSpec<'_, N, GraphColor>,
) -> Result<(), anyhow::Error>
where
    DB: DrawingBackend,
    <DB as DrawingBackend>::ErrorType: 'static,
    N: Ord + Copy + std::fmt::Debug + 'static,
    Range<N>: AsRangedCoord<Value = N>,
    <Range<N> as AsRangedCoord>::CoordDescType:
        Ranged<ValueType = N, FormatOption = DefaultFormatting>,
    GridColor: plotters::prelude::Color,
    GraphColor: plotters::prelude::Color,
{
    use plotters::prelude::*;
    tracing::trace!("draw_graph called");

    let first = *timestamps.first().unwrap();
    let last = *timestamps.last().unwrap();

    let (min, max) = minmax(&nums).unwrap();

    let mut chart = ChartBuilder::on(area)
        .margin(5)
        .x_label_area_size(30)
        // TODO: calculate via font size instead of guessing
        .y_label_area_size(spec.margin_px)
        .build_cartesian_2d(first..last, min..max)?;

    let today = crate::util::now_local().date();
    let local_offset = UtcOffset::current_local_offset().unwrap();

    chart
        .configure_mesh()
        .label_style(&grid_color)
        .axis_style(&grid_color)
        .light_line_style(&grid_color.mix(0.1))
        .x_label_formatter(&|time| {
            let dt = time::OffsetDateTime::from_unix_timestamp(i64::from(*time))
                .unwrap()
                .to_offset(local_offset);
            match spec.time_format {
                TimeFormat::TimeOnly => dt.format(&format_description!("[hour]:[minute]")).unwrap(),
                TimeFormat::DateTime => {
                    if dt.date() != today {
                        dt.format(&format_description!("[year]-[month]-[day] [hour]:[minute]"))
                            .unwrap()
                    } else {
                        dt.format(&format_description!("[hour]:[minute]")).unwrap()
                    }
                }
            }
        })
        .y_label_formatter(spec.formatter)
        .draw()?;

    chart.draw_series(LineSeries::new(
        timestamps.iter().copied().zip(nums.iter().copied()),
        &spec.color,
    ))?;

    chart.configure_series_labels().draw()?;

    Ok(())
}

fn approx_to(n: f64) -> u32 {
    if n.is_finite() {
        if n <= f64::from(u32::MIN) {
            u32::MIN
        } else if n >= f64::from(u32::MAX) {
            u32::MAX
        } else {
            n.round() as u32
        }
    } else {
        tracing::error!("Got garbage f64 in approx_to: {}", n);
        0
    }
}

pub fn minmax<T>(a: &[T]) -> Option<(T, T)>
where
    T: Ord + Copy,
{
    let mut min = *a.get(0)?;
    let mut max = min;
    for n in a.iter().skip(1).copied() {
        if n < min {
            min = n;
        } else if n > max {
            max = n;
        }
    }

    Some((min, max))
}

glib::wrapper! {
    pub struct GraphPainter(ObjectSubclass<imp::GraphPainter>) @implements gdk::Paintable, gtk::Widget;
}

impl Default for GraphPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphPainter {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a Graph")
    }
}

#[derive(Debug)]
pub enum Data {
    Live(SensorObj),
    Static(Timeseries),
}

impl GraphPainter {
    pub fn displayed_unit(&self) -> Unit {
        let self_ = imp::GraphPainter::from_instance(&self);
        self_.unit.get()
    }

    pub fn set_displayed_unit(&self, unit: Unit) {
        let self_ = imp::GraphPainter::from_instance(&self);
        if self_.unit.replace(unit) != unit {
            self.invalidate_contents();
        }
    }

    pub fn set_displayed_data(&self, data: Option<Data>) {
        let self_ = imp::GraphPainter::from_instance(&self);
        if let Some(obj) = self_.obj.take() {
            if let Some(bind) = self_.live_binding.take() {
                obj.disconnect(bind);
            }
        }
        self_.bounds.set(None);
        match data {
            Some(Data::Live(obj)) => {
                // TODO: use binding instead
                self_.timeseries.replace(obj.timeseries());
                let binding = obj.connect_notify_local(
                    Some("live-timeseries"),
                    glib::clone!(@weak self as this => move |obj, _param_spec| {
                        let self_ = imp::GraphPainter::from_instance(&this);
                        self_.timeseries.replace(obj.timeseries());
                        this.invalidate_contents();
                    }),
                );
                self_.live_binding.replace(Some(binding));
                self_.obj.replace(Some(obj));
                self_.time_format.set(TimeFormat::TimeOnly);
            }
            Some(Data::Static(timeseries)) => {
                self_.timeseries.replace(SharedTimeseries::new(timeseries));
                self_.time_format.set(TimeFormat::DateTime);
            }
            None => {
                self_.timeseries.replace(SharedTimeseries::empty());
            }
        }
        self.invalidate_contents();
    }

    pub fn set_grid_color(&self, color: plotters::style::RGBColor) {
        let self_ = imp::GraphPainter::from_instance(&self);
        self_.grid_color.set(color);
        self.invalidate_contents();
    }

    pub fn set_bounds(&self, bounds: Option<(u32, u32)>) {
        let self_ = imp::GraphPainter::from_instance(&self);
        self_.bounds.set(bounds);
        self.invalidate_contents();
    }
}
