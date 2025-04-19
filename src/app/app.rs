use crate::app::{routes, task::ThreadPool, web_routes};
use crate::motion_detect::gpio::MotionDetector;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub async fn create_app(motion_detector: MotionDetector) -> Router {
    let thread_pool = ThreadPool::new(20).await;
    let app = Router::new()
        .route("/start_cam", post(routes::init_camera))
        .route("/shutdown", post(routes::shutdown_device))
        .route("/cam_status", get(routes::get_current_cam_status))
        .route("/stream.m3u8", get(routes::stream_handler))
        .with_state(Arc::new(motion_detector))
        .route("/start_download", post(routes::start_download))
        .route("/download", get(routes::download_from_task))
        .with_state(Arc::new(thread_pool))
        .route("/file", get(routes::stream))
        .route("/video_data", get(routes::get_all_videos_data))
        .route("/", get(web_routes::index))
        .route("/play_videos", get(web_routes::play_videos))
        .route("/start_motion_detector", get(web_routes::start_motion_detector));
        // .nest_service("/static", ServeDir::new("/home/jamie/coding/rust/motion_camera_server/frontend/static"));
    app
}
