use crate::camera;
use chrono::prelude::*;
use rppal::gpio::Mode::Output;
use rppal::gpio::{Gpio, IoPin};
use std::{
    process::Child,
    thread::{self},
    time,
};

pub struct SensorConfig {
    pub sensor_pin: IoPin,
}

impl SensorConfig {
    pub fn new(gpio: Gpio, pin_num: u8) -> SensorConfig {
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
}

impl MotionDetector {
    pub fn new(pin_num: u8) -> MotionDetector {
        let gpio_interface = Gpio::new().unwrap();
        let sensor_config = SensorConfig::new(gpio_interface, pin_num);

        return MotionDetector { sensor_config };
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

    pub fn monitor_loop(&self) {
        println!("Starting camera");
        let camera_thread: Child = camera::camera::initialise_camera();
        let mut camera_recording = false;
        let mut is_motion: bool;
        loop {
            is_motion = self.is_motion();
            if is_motion && !camera_recording {
                let current_time = Utc::now().to_string();
                println!("Motion detected at {current_time} starting camera");
                camera::camera::start_stop_recording(&camera_thread.id());
                camera_recording = true;
            } else if is_motion && camera_recording {
                let current_time = Utc::now().to_string();
                println!("Motion detected at {current_time} camera already recording");
            } else if !is_motion && camera_recording {
                let current_time = Utc::now().to_string();
                println!("No motion detected at {current_time} stopping camera");
                camera::camera::start_stop_recording(&camera_thread.id());
                camera_recording = false;
            }
        }
    }
}
