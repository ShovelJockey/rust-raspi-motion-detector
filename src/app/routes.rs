use axum::extract::{Query, State};
use serde::Deserialize;
use crate::motion_detect::gpio::MotionDetector;

#[derive(Deserialize)]
enum CameraType {
    Stream,
    Record,
}

#[derive(Deserialize)]
struct CameraParam {
    camera_type: CameraType
}

async fn init_camera(motion_detector: State<MotionDetector>, recording_type: Query<CameraParam>) {
    if *motion_detector.is_active.read().unwrap() {
        return ;
    };
    if *motion_detector.is_shutdown.read().unwrap() {
        return ;
    };
    match recording_type.camera_type {
        CameraType::Record => {
            motion_detector.monitor_loop_record();
        },
        CameraType::Stream => {
            motion_detector.monitor_loop_stream();
        }
    }
}

async fn shutdown_device(motion_detector: State<MotionDetector>) {
    if !*motion_detector.is_active.read().unwrap() {
        return ;
    };
    if *motion_detector.is_shutdown.read().unwrap() {
        return ;
    };
}