use motion_detector::{camera, motion_detect::{self, gpio::MotionDetector}};
use std::{thread::{self, JoinHandle}, time};
use chrono::prelude::*;
use log::info;


fn monitor_loop(motion_detector: &MotionDetector) {
    let sleep_duration = time::Duration::from_secs(1);
    let mut camera_thread: JoinHandle<String>;
    loop {
        if motion_detector.sensor_config.sensor_pin.is_high() {
            let current_time = Utc::now().to_string();
            info!("Motion detected at {current_time} starting camera");
            camera_thread = thread::spawn(|| {camera::camera::start_recording(current_time)});
            let thread_result = camera_thread.join().unwrap_or_default();
            info!("Camera thread result: {thread_result}");
        }
        else {
            thread::sleep(sleep_duration)
        }
    }
}


fn main() {
    let detector_obj = motion_detect::gpio::MotionDetector::new(20);
    if camera::camera::initialise_camera() {
        monitor_loop(&detector_obj)
    }
    else {
        info!("Camera failed to initialise");
        println!("Camera failed to initialise.");
    }
}
