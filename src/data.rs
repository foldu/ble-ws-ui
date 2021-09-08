use indexmap::IndexMap;
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct SensorValues {
    pub temperature: i32,
    pub humidity: u32,
    pub pressure: u32,
}

pub struct Data {
    pub sensors: IndexMap<Uuid, crate::sensor_obj::SensorObj>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            sensors: IndexMap::new(),
        }
    }
}

#[derive(Default)]
pub struct TimeseriesBuilder {
    timeseries: Option<Timeseries>,
}

impl TimeseriesBuilder {
    fn series(&mut self) -> &mut Timeseries {
        if let Some(ref mut tm) = self.timeseries {
            return tm;
        }
        self.timeseries = Some(Default::default());
        self.timeseries.as_mut().unwrap()
    }

    pub fn temperature(&mut self, temperature: Vec<i16>) -> &mut Self {
        self.series().temperature = temperature;
        self
    }

    pub fn time(&mut self, time: Vec<u32>) -> &mut Self {
        self.series().time = time;
        self
    }

    pub fn humidity(&mut self, humidity: Vec<u32>) -> &mut Self {
        self.series().humidity = humidity;
        self
    }

    pub fn pressure(&mut self, pressure: Vec<u32>) -> &mut Self {
        self.series().pressure = pressure;
        self
    }

    pub fn build(&mut self) -> Result<Timeseries, anyhow::Error> {
        match self.timeseries.take() {
            Some(series) => {
                if [
                    series.temperature.len(),
                    series.humidity.len(),
                    series.pressure.len(),
                ]
                .iter()
                .all(|&n| n == series.time.len())
                {
                    Ok(series)
                } else {
                    Err(anyhow::format_err!("Timeseries columns not same length"))
                }
            }
            None => Ok(Default::default()),
        }
    }
}

#[derive(Default, Debug)]
pub struct Timeseries {
    time: Vec<u32>,
    temperature: Vec<i16>,
    humidity: Vec<u32>,
    pressure: Vec<u32>,
}

impl Timeseries {
    pub fn is_empty(&self) -> bool {
        self.time.is_empty()
    }

    /// Get a reference to the timeseries's time.
    pub fn time(&self) -> &[u32] {
        self.time.as_slice()
    }

    /// Get a reference to the timeseries's temperature.
    pub fn temperature(&self) -> &[i16] {
        self.temperature.as_slice()
    }

    /// Get a reference to the timeseries's humidity.
    pub fn humidity(&self) -> &[u32] {
        self.humidity.as_slice()
    }

    /// Get a reference to the timeseries's pressure.
    pub fn pressure(&self) -> &[u32] {
        self.pressure.as_slice()
    }

    pub fn push_row(&mut self, row: TimeseriesRow) -> Result<(), PushError> {
        match self.time.last() {
            Some(&time) if row.time <= time => Err(PushError {
                theirs: row.time,
                ours: time,
            }),
            _ => {
                self.time.push(row.time);
                self.temperature.push(row.temperature);
                self.humidity.push(row.humidity);
                self.pressure.push(row.pressure);

                Ok(())
            }
        }
    }
}

#[derive(Clone, glib::GSharedBoxed, Default)]
#[gshared_boxed(type_name = "SharedTimeseries")]
pub struct SharedTimeseries(pub Rc<Option<RefCell<Timeseries>>>);

impl SharedTimeseries {
    pub fn new(timeseries: Timeseries) -> Self {
        Self(Rc::new(Some(RefCell::new(timeseries))))
    }

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub fn empty() -> Self {
        Default::default()
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid push, tried to push row with timestamp {theirs} while last is {ours}")]
pub struct PushError {
    theirs: u32,
    ours: u32,
}

#[derive(Copy, Clone)]
pub struct TimeseriesRow {
    pub time: u32,
    pub temperature: i16,
    pub humidity: u32,
    pub pressure: u32,
}
