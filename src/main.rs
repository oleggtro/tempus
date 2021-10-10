use chrono::{DateTime, Utc};
use ds18b20::{Ds18b20, Resolution};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use one_wire_bus::{Address, OneWire, OneWireResult};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericGauge};
use prometheus_exporter::prometheus::{register_gauge, register_gauge_vec};
use rppal::gpio::Gpio;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::fs::{self, read_dir, read_to_string};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::thread::{self, sleep};
use std::time::Duration;
use tokio::sync::oneshot;

mod temp_actor;

#[tokio::main]
async fn main() {
    //let exporter = prometheus_exporter::start("0.0.0.0:9184".parse().unwrap()).unwrap();

    let addr: SocketAddr = "0.0.0.0:9184".parse().expect("cannot listen on addr");
    let exporter = prometheus_exporter::start(addr).expect("can not start exporter");

    let mut metrics = HashMap::new();

    for id in get_device_ids() {
        let metric = register_gauge!(
            format!("sensor_{}", id.to_str().unwrap().replace("-", "_")),
            "the id"
        )
        .expect("can not create gauge");
        metrics.insert(id, metric);
    }

    //dbg!(get_temperature(&mut Delay::new(), &mut OneWire::new(x.into_output()).unwrap()));

    loop {
        dbg!(&metrics);
        sleep(Duration::from_secs(7));
    }
}

/*pub async fn read_temp(
    mut metrics: HashMap<OsString, GenericGauge<AtomicF64>>,
) -> HashMap<OsString, GenericGauge<AtomicF64>> {
    // read_dir(Path::new("/sys/bus/w1/devices"));

    let mut entries = fs::read_dir("/sys/bus/w1/devices")
        .expect("couldnt read dir")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .expect("failed to collect");

    //  dbg!(&entries);

    let mut sensors: Vec<PathBuf> = Vec::new();
    for i in entries {
        if i != PathBuf::from("/sys/bus/w1/devices/w1_bus_master1") {
            sensors.push(i);
        }
    }

    println!("");
    println!("");
    println!("");

    //let mut temps = HashMap::new();

    for mut sensor in sensors {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let xy = sensor.clone();
            let id = xy.file_name().unwrap();
            sensor.push("w1_slave");
            let output = read_to_string(sensor).unwrap();
            let temp = &output[69..74];
            let x = temp.parse::<f32>().unwrap();
            let temp = x / 1000 as f32;
            tx.send(TempMsg(id, temp.into()))
        });
        metrics
            .get_mut(id.clone())
            .expect("please restart app")
            .set(temp.into());
    }

    metrics
}*/

fn get_device_ids() -> Vec<OsString> {
    let mut entries = fs::read_dir("/sys/bus/w1/devices")
        .expect("couldnt read dir")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .expect("failed to collect");

    //  dbg!(&entries);

    let mut sensors: Vec<PathBuf> = Vec::new();
    for i in entries {
        if i != PathBuf::from("/sys/bus/w1/devices/w1_bus_master1") {
            sensors.push(i);
        }
    }

    println!("");
    println!("");
    println!("");

    let mut ids = Vec::new();

    for mut sensor in sensors {
        let id = sensor.file_name().unwrap();
        ids.push(id.to_owned());
    }
    ids
}

// Bad stuff
// no work

fn get_temperature<P, E>(
    delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    one_wire_bus: &mut OneWire<P>,
) -> OneWireResult<(), E>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    println!("1");
    // initiate a temperature measurement for all connected devices
    ds18b20::start_simultaneous_temp_measurement(one_wire_bus, delay)?;
    println!("2");
    // wait until the measurement is done. This depends on the resolution you specified
    // If you don't know the resolution, you can obtain it from reading the sensor data,
    // or just wait the longest time, which is the 12-bit resolution (750ms)
    Resolution::Bits12.delay_for_measurement_time(delay);
    println!("3");
    // iterate over all the devices, and report their temperature
    let mut search_state = None;

    loop {
        if let Some((device_address, state)) =
            one_wire_bus.device_search(search_state.as_ref(), false, delay)?
        {
            search_state = Some(state);
            println!("5");
            if device_address.family_code() != ds18b20::FAMILY_CODE {
                // skip other devices
                println!("continue");
                continue;
            }
            // You will generally create the sensor once, and save it for later
            let sensor = Ds18b20::new(device_address)?;
            println!("6");
            // contains the read temperature, as well as config info such as the resolution used
            let sensor_data = sensor.read_data(one_wire_bus, delay)?;
            println!(
                "Device at {:?} is {}°C",
                device_address, sensor_data.temperature
            );
        } else {
            println!("else");
            break;
        }
        println!("4");
    }
    Ok(())
}

fn test_config<P, E>(
    delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    one_wire_bus: &mut OneWire<P>,
) -> OneWireResult<(), E>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    // Find the first device on the bus (assuming they are all Ds18b20's)
    if let Some(device_address) = one_wire_bus.devices(false, delay).next() {
        println!("iter");
        let device_address = device_address?;
        let device = Ds18b20::new(device_address)?;

        // read the initial config values (read from EEPROM by the device when it was first powered)
        let initial_data = device.read_data(one_wire_bus, delay)?;
        println!("Initial data: {:?}", initial_data);

        let resolution = initial_data.resolution;

        // set new alarm values, but keep the resolution the same
        device.set_config(18, 24, resolution, one_wire_bus, delay)?;

        // confirm the new config is now in the scratchpad memory
        let new_data = device.read_data(one_wire_bus, delay)?;
        println!("New data: {:?}", new_data);

        // save the config to EEPROM to save it permanently
        device.save_to_eeprom(one_wire_bus, delay)?;

        // read the values from EEPROM back to the scratchpad to verify it was saved correctly
        device.recall_from_eeprom(one_wire_bus, delay)?;
        let eeprom_data = device.read_data(one_wire_bus, delay)?;
        println!("EEPROM data: {:?}", eeprom_data);
    }
    Ok(())
}
