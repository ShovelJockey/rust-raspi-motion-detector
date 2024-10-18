use crate::app::routes;
use crate::motion_detect::gpio::MotionDetector;
use axum::{routing::post, serve, Router};
use std::sync::Arc;

pub async fn create_app(motion_detector: MotionDetector) -> Router {
    let app = Router::new()
        .route("/start_cam", post(routes::init_camera))
        .route("/shutdown", post(routes::shutdown_device))
        .with_state(Arc::new(motion_detector));
    app
}
