use crate::motion_detect::gpio::MotionDetector;
use axum_server::tls_rustls::RustlsConfig;
use dotenvy::dotenv;
use std::{net::SocketAddr, path::PathBuf};
use tokio;
use tokio_rustls::rustls;

pub mod app;
mod camera;
pub mod motion_detect;

#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    dotenv().ok();
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();
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
    camera::camera::start_stream_rtp();
    println!("started");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// double check security measures are ok
// improve redirect to login
// improve https redirect

// implement manual udp to webrtc stream

// add --low-latency to recording?

// add css styling
// adjust env/path usage to be less specific to current pi system
// look into doing webrtc myself in rust
