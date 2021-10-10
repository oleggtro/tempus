use actix::Message;
use actix::{Actor, Context, Handler};
use prometheus_exporter::prometheus::core::AtomicF64;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::{self, read_to_string};
use std::io;
use std::path::PathBuf;

use crate::GenericGauge;

pub struct TempActor {
    pub sensors: Vec<PathBuf>,
    pub metrics: HashMap<OsString, GenericGauge<AtomicF64>>,
}

impl TempActor {
    pub fn new() -> Self {
        let mut entries = fs::read_dir("/sys/bus/w1/devices")
            .expect("couldnt read dir")
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .expect("failed to collect");

        //  dbg!(&entries)

        let mut sensors: Vec<PathBuf> = Vec::new();
        for i in entries {
            if i != PathBuf::from("/sys/bus/w1/devices/w1_bus_master1") {
                sensors.push(i);
            }
        }

        Self {
            metrics: HashMap::new(),
            sensors: sensors,
        }
    }
}

impl Actor for TempActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "")]
pub struct InitMeasurement;

#[derive(Message)]
#[rtype(result = "")]
pub struct SingleMeasurement {
    pub sensor: PathBuf,
}

impl Handler<InitMeasurement> for TempActor {
    type Result = ();

    fn handle(&mut self, msg: InitMeasurement, ctx: &mut Self::Context) -> Self::Result {}
}

impl Handler<SingleMeasurement> for TempActor {
    type Result = ();

    fn handle(&mut self, msg: SingleMeasurement, ctx: &mut Self::Context) -> Self::Result {
        let mut sensor = msg.sensor;
        let xy = sensor.clone();
        let id = xy.file_name().unwrap();
        sensor.push("w1_slave");
        let output = read_to_string(sensor).unwrap();
        let temp = &output[69..74];
        let x = temp.parse::<f32>().unwrap();
        let temp = x / 1000 as f32;
        self.metrics
            .get_mut(id.clone())
            .expect("please restart app")
            .set(temp.into());
    }
}
