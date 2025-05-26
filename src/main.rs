use crate::motion_detect::gpio::MotionDetector;
use axum::serve;
use dotenvy::dotenv;
use tokio;

pub mod app;
mod camera;
pub mod motion_detect;

#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    dotenv().ok();
    let motion_detector = MotionDetector::new(4);
    let app = app::app::create_app(motion_detector).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("started");
    serve(listener, app).await.unwrap();
}

// add frontend shutdown of camera -- added needs testing
// add button for stream view if stream currently running - not just after being started -- added needs testing SORT

// finish login setup
// add login redirect
// move to https
// add redirect to https

// add css styling
// adjust env/path usage to be less specific to current pi system
// look into doing webrtc myself in rust
