use crate::motion_detect::gpio::MotionDetector;
use crate::streaming::turn;
use axum_server::tls_rustls::RustlsConfig;
use dotenvy::dotenv;
use std::{io::stdout, net::SocketAddr, path::PathBuf};
use tokio;
use tokio_rustls::rustls;
use tracing::info;
use tracing_subscriber::{fmt::layer, prelude::*, registry, filter::LevelFilter};

pub mod app;
mod camera;
pub mod motion_detect;
mod streaming;

#[tokio::main]
async fn main() {
    camera::camera::test_initialise_camera().expect("Camera initialised successfully");
    dotenv().ok();
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let trace_layer = layer().pretty().with_writer(stdout).with_filter(LevelFilter::DEBUG);
    registry().with(trace_layer).init();

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

    let _turn_server = turn::create_turn_server()
        .await
        .expect("turn server starts successfully");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("started");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
