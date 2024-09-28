use axum::{Router, serve, routing::get};
use std::sync::Arc;
use crate::motion_detect::gpio::MotionDetector;

async fn basic_route() -> String {
    return "test".to_string();
}

pub async fn create_app(motion_dector: MotionDetector) -> Router {
    let app = Router::new().route("/", get(basic_route)).with_state(Arc::new(motion_dector));
    app
}