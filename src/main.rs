use axum::serve;
use dotenvy::dotenv;
use tokio;
use tempfile::TempDir;
use crate::motion_detect::gpio::MotionDetector;

pub mod app;
mod camera;
pub mod motion_detect;

#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    dotenv().ok();
    let temp_dir = TempDir::new().unwrap();
    let motion_detector = MotionDetector::new(4, temp_dir);
    let app = app::app::create_app(motion_detector).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("started");
    serve(listener, app).await.unwrap();
}
