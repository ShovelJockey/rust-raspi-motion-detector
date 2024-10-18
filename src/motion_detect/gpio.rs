use crate::camera;
use chrono::prelude::*;
use rppal::gpio::Mode::Output;
use rppal::gpio::{Gpio, IoPin};
use std::{
    sync::RwLock,
    thread::{self},
    time,
};

pub struct SensorConfig {
    pub sensor_pin: IoPin,
}

impl Clone for SensorConfig {
    fn clone(&self) -> Self {
        let pin_num = self.sensor_pin.pin();
        SensorConfig::new(pin_num)
    }
}

impl SensorConfig {
    pub fn new(pin_num: u8) -> SensorConfig {
        let gpio = Gpio::new().unwrap();
        let pin = gpio
            .get(pin_num)
            .expect(format!("Pin found with number: {pin_num}").as_str());
        let output_pin = pin.into_io(Output);

        return SensorConfig {
            sensor_pin: output_pin,
        };
    }
}

pub struct MotionDetector {
    pub sensor_config: SensorConfig,
    pub is_active: RwLock<bool>,
    pub is_shutdown: RwLock<bool>,
    is_recording: RwLock<bool>,
}

impl MotionDetector {
    pub fn new(pin_num: u8) -> MotionDetector {
        return MotionDetector {
            sensor_config: SensorConfig::new(pin_num),
            is_active: RwLock::new(false), // make sure is false on shutdown
            is_shutdown: RwLock::new(false), // make sure set false after shutdown, check thread non existant?
            is_recording: RwLock::new(false),
        };
    }

    pub fn is_high(&self) -> bool {
        self.sensor_config.sensor_pin.is_high()
    }

    pub fn is_motion(&self) -> bool {
        let mut count = 5;
        let mut is_high = false;
        while count > 0 {
            if self.sensor_config.sensor_pin.is_high() {
                is_high = true;
                break;
            } else {
                thread::sleep(time::Duration::from_secs(1));
                count -= 1;
            }
        }
        is_high
    }

    pub fn monitor_loop_record(&self) {
        println!("Starting motion sensor camera in monitor mode.");
        let mut is_motion: bool;
        let mut camera_process_id: Option<u32> = None;
        *self.is_active.write().unwrap() = true;
        loop {
            if *self.is_shutdown.read().unwrap() {
                if *self.is_recording.read().unwrap() {
                    camera::camera::shutdown_process(&camera_process_id.unwrap());
                    *self.is_recording.write().unwrap() = false;
                }
                *self.is_active.write().unwrap() = false;
                break;
            }
            is_motion = self.is_motion();
            let is_recording = *self.is_recording.read().unwrap();
            if is_motion && !is_recording {
                let current_time = Utc::now().to_string();
                println!("Motion detected at {current_time} starting camera");
                camera_process_id = Some(camera::camera::start_recording(current_time));
                *self.is_recording.write().unwrap() = true;
            } else if is_motion && is_recording {
                let current_time = Utc::now().to_string();
                println!("Motion detected at {current_time} camera already recording");
            } else if !is_motion && is_recording {
                if camera_process_id.is_none() {
                    panic!("Error is_recording evaluates to true but camera process id is none");
                }
                camera::camera::shutdown_process(&camera_process_id.unwrap());
                *self.is_recording.write().unwrap() = false;
            }
        }
    }

    pub fn monitor_loop_stream(&self) {
        println!("Starting camera in streaming mode.");
        let mut is_motion: bool;
        let camera_process_id = camera::camera::start_stream();
        *self.is_active.write().unwrap() = true;
        loop {
            if *self.is_shutdown.read().unwrap() {
                camera::camera::shutdown_process(&camera_process_id);
                *self.is_active.write().unwrap() = false;
            }
            is_motion = self.is_motion();
            if is_motion {
                let current_time = Utc::now().to_string();
                println!("Motion detected at {current_time}");
            }
        }
    }
}
