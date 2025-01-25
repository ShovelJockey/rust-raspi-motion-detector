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
    let motion_detector = motion_detect::gpio::MotionDetector::new(4);
    let app = app::app::create_app(motion_detector).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("started");
    serve(listener, app).await.unwrap();
}
