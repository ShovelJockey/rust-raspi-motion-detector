use crate::app::{routes, task::ThreadPool};
use crate::motion_detect::gpio::MotionDetector;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub async fn create_app(motion_detector: MotionDetector) -> Router {
    let thread_pool = ThreadPool::new(20);
    let app = Router::new()
        .route("/start_cam", post(routes::init_camera))
        .route("/shutdown", post(routes::shutdown_device))
        .with_state(Arc::new(motion_detector))
        .route("start_download", post(routes::start_download))
        .route("/download", get(routes::download))
        .with_state(Arc::new(thread_pool.await))
        .route("/file", get(routes::stream));
    app
}
