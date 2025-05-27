use crate::motion_detect::gpio::MotionDetector;
use dotenvy::dotenv;
use tokio;
use axum_server::tls_rustls::RustlsConfig;
use std::{path::PathBuf, net::SocketAddr};

pub mod app;
mod camera;
pub mod motion_detect;

#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    dotenv().ok();
    let motion_detector = MotionDetector::new(4);
    let app = app::app::create_app(motion_detector).await;

    tokio::spawn(app::app::redirect_http_to_https());

    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .expect("Valid https certs");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("started");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
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
