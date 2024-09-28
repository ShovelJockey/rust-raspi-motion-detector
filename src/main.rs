use axum::{Router, serve, routing::get};
use tokio;

mod camera;
pub mod motion_detect;
pub mod app;


#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    let motion_detector = motion_detect::gpio::MotionDetector::new(8);
    let mut app = app::app::create_app(motion_detector).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let server = serve(listener, app);
    
}


// look at why initial pause isnt working, see if it is compatible with streaming, 
// also consider that stream seems to crash on client disconnect with listen - could require rethinking? though if pause works with signal thats fine,
// how to test? maybe with rust?
// always on camera mode for app,
// 