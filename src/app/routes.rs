use super::file_stream::FileStream;
use super::task::ThreadPool;
use crate::motion_detect::gpio::{monitor_loop_record, monitor_loop_stream, MotionDetector};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{offset::Utc, DateTime};
use ffmpeg_next::{ffi::AV_TIME_BASE, format::input};
use glob::glob;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::{
    env::var,
    fmt::Display,
    path::Path,
    result::Result,
    sync::Arc,
    thread::spawn,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

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

#[derive(Deserialize)]
pub struct FileName {
    filename: String,
}

#[derive(Deserialize, Serialize)]
pub struct VideoData {
    video_created: String,
    video_duration: f64,
}

impl VideoData {
    pub fn new(created: SystemTime, video_path: &Path) -> Self {
        let duration = input(video_path).unwrap().duration() as f64 / AV_TIME_BASE as f64;
        let datetime: DateTime<Utc> = created.into();
        let formated_date = datetime.format("%d/%m/%Y %T").to_string();

        return VideoData {
            video_created: formated_date,
            video_duration: duration,
        };
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

pub async fn stream(file_name: Query<FileName>) -> Response {
    let file_name = &file_name.filename;
    let file_dir = var("VIDEO_SAVE_PATH").unwrap_or("/home".to_string());
    let formated_path = format!("{file_dir}/{file_name}");
    FileStream::<ReaderStream<File>>::from_path(formated_path)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("File not found: {e}")))
        .into_response()
}

pub async fn get_all_videos_data(videos_since: Query<Option<u64>>) -> Response {
    let file_dir = var("VIDEO_SAVE_PATH").unwrap_or("/home".to_string());
    let pattern = format!("{file_dir}/*.mp4");
    let videos_delta = match videos_since.0 {
        Some(time) => UNIX_EPOCH + Duration::from_secs(time),
        None => UNIX_EPOCH + Duration::from_secs(0),
    };

    match ffmpeg_next::init() {
        Ok(()) => {}
        Err(err) => {
            println!("Encountered error: {err} trying to init ffmpeg");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let paths = match glob(&pattern) {
        Ok(paths) => paths,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error encountered trying to find videos, {err}"),
            )
                .into_response();
        }
    };

    let mut video_names = Vec::new();
    for file in paths.filter_map(Result::ok) {
        let file_created = file.metadata().unwrap().created().unwrap();
        println!("found file {}", &file.display());
        if file_created >= videos_delta {
            let video_data = VideoData::new(file_created, file.as_path());
            video_names.push(video_data);
        }
    }
    return (StatusCode::OK, to_string(&video_names).unwrap()).into_response();
}

pub async fn start_download(
    thread_pool: State<Arc<ThreadPool>>,
    last_download: Query<u64>,
) -> Response {
    let file_dir = var("VIDEO_SAVE_PATH").unwrap_or("/home".to_string());
    let pattern = format!("{file_dir}/*.mp4");
    let time_delta = UNIX_EPOCH + Duration::from_secs(last_download.0);
    // handle possible errors here return bad resp
    let paths = match glob(&pattern) {
        Ok(paths) => paths,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error encountered trying to find videos, {err}"),
            )
                .into_response();
        }
    };
    for file in paths.filter_map(Result::ok) {
        let file_created = file.metadata().unwrap().created().unwrap();
        println!("found file {}", &file.display());
        if file_created >= time_delta {
            println!("adding file to queue");
            thread_pool.queue_file(file).await;
        }
    }
    StatusCode::OK.into_response()
}

pub async fn download(thread_pool: State<Arc<ThreadPool>>) -> Response {
    let (task_running, stream) = thread_pool.get_result().await;
    if !task_running {
        return (
            StatusCode::BAD_REQUEST,
            "No task is currently running and no results are still pending download.",
        )
            .into_response();
    }
    stream.unwrap().into_response()
}
