use crate::camera;
use chrono::prelude::*;
use rppal::gpio::Mode::Input;
use rppal::gpio::{Gpio, IoPin};
use serde::Deserialize;
use std::{
    sync::RwLock,
    thread::{self},
    time,
};

pub struct SensorConfig {
    pub sensor_pin: IoPin,
}

#[derive(Deserialize, Debug)]
pub enum CameraType {
    Stream,
    Record,
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
        let output_pin = pin.into_io(Input);

        return SensorConfig {
            sensor_pin: output_pin,
        };
    }
}

pub struct MotionDetector {
    pub sensor_config: SensorConfig,
    pub cam_type: RwLock<Option<CameraType>>,
    pub is_shutdown: RwLock<bool>,
}

impl MotionDetector {
    pub fn new(pin_num: u8) -> MotionDetector {
        return MotionDetector {
            sensor_config: SensorConfig::new(pin_num),
            cam_type: RwLock::new(None),
            is_shutdown: RwLock::new(false),
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
}

pub fn monitor_loop_record(motion_detector: &MotionDetector) {
    println!("Starting motion sensor camera in monitor mode.");
    let mut is_motion: bool;
    let mut is_recording = false;
    let mut camera_process_id: Option<u32> = None;
    loop {
        if *motion_detector.is_shutdown.read().unwrap() {
            println!("shutdown ordered");
            if is_recording {
                println!("ending current recording");
                camera::camera::shutdown_cam_process(camera_process_id.unwrap());
            }
            *motion_detector.cam_type.write().unwrap() = None;
            *motion_detector.is_shutdown.write().unwrap() = false;
            break;
        }
        is_motion = motion_detector.is_motion();
        if is_motion && !is_recording {
            println!("Motion detected starting camera");
            camera_process_id = Some(camera::camera::start_recording());
            is_recording = true;
            thread::sleep(time::Duration::from_secs(5));
        } else if is_motion && is_recording {
            println!("Motion detected camera already recording");
            thread::sleep(time::Duration::from_secs(1));
        } else if !is_motion && is_recording {
            println!("No motion detected stopping recording");
            if camera_process_id.is_none() {
                panic!("Error is_recording evaluates to true but camera process id is none");
            }
            camera::camera::shutdown_cam_process(camera_process_id.unwrap());
            is_recording = false;
            thread::sleep(time::Duration::from_secs_f32(0.5));
        }
    }
}

pub fn monitor_loop_stream(motion_detector: &MotionDetector) {
    println!("Starting camera in streaming mode.");
    let mut is_motion: bool;
    let stream_process_id = camera::camera::start_stream_rtp();
    loop {
        if *motion_detector.is_shutdown.read().unwrap() {
            camera::camera::shutdown_cam_process(stream_process_id);
            *motion_detector.cam_type.write().unwrap() = None;
            *motion_detector.is_shutdown.write().unwrap() = false;
            break;
        }
        is_motion = motion_detector.is_motion();
        if is_motion {
            let current_time = Utc::now().to_string();
            println!("Motion detected at {current_time}");
        }
    }
}
