use crate::motion_detect::gpio::{monitor_loop_record, monitor_loop_stream, MotionDetector};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{fmt::Display, sync::Arc, thread::spawn};

#[derive(Deserialize, Debug)]
enum CameraType {
    Stream,
    Record,
}

impl Display for CameraType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize)]
pub struct CameraParam {
    camera_type: CameraType,
}

struct CameraResponse {
    status: StatusCode,
    message: String,
}

impl IntoResponse for CameraResponse {
    fn into_response(self) -> Response {
        return (self.status, self.message).into_response();
    }
}

pub async fn init_camera(
    motion_detector: State<Arc<MotionDetector>>,
    recording_type: Query<CameraParam>,
) -> Response {
    if *motion_detector.is_active.read().unwrap() {
        let message = "Cannot start camera as it is already active";
        return CameraResponse {
            status: StatusCode::CONFLICT,
            message: message.to_string(),
        }
        .into_response();
    };
    if *motion_detector.is_shutdown.read().unwrap() {
        let message = "Camera is shutting down cannot activate yet";
        return CameraResponse {
            status: StatusCode::CONFLICT,
            message: message.to_string(),
        }
        .into_response();
    };
    match recording_type.camera_type {
        CameraType::Record => {
            spawn(move || monitor_loop_record(&motion_detector));
        }
        CameraType::Stream => {
            spawn(move || monitor_loop_stream(&motion_detector));
        }
    }
    let message = format!("Camera started in {}", recording_type.camera_type);
    return CameraResponse {
        status: StatusCode::OK,
        message: message.to_string(),
    }
    .into_response();
}

pub async fn shutdown_device(motion_detector: State<Arc<MotionDetector>>) -> Response {
    if !*motion_detector.is_active.read().unwrap() {
        let message = "Cannot shutdown motion detector as it is not active";
        return CameraResponse {
            status: StatusCode::CONFLICT,
            message: message.to_string(),
        }
        .into_response();
    } else if *motion_detector.is_shutdown.read().unwrap() {
        let message = "Cannot shutdown motion detector as it is already shutting down";
        return CameraResponse {
            status: StatusCode::CONFLICT,
            message: message.to_string(),
        }
        .into_response();
    };
    *motion_detector.is_shutdown.write().unwrap() = true;
    let message = "Started shutdown process for motion detector";
    return CameraResponse {
        status: StatusCode::OK,
        message: message.to_string(),
    }
    .into_response();
}
