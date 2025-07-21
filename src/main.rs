use crate::motion_detect::gpio::MotionDetector;
use axum_server::tls_rustls::RustlsConfig;
use dotenvy::dotenv;
use std::{env::var, fs::File, io::stdout, net::SocketAddr, path::PathBuf};
use tokio;
use tokio_rustls::rustls;
use tracing::info;
use tracing_subscriber::{fmt::layer, prelude::*, registry};

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

    // let file_dir = var("LOG_PATH").unwrap_or("/log.logfile".to_string());
    // let log_file = match File::create_new(&file_dir) {
    //     Ok(file) => file,
    //     Err(_) => File::open(&file_dir).expect("Open already existing file"),
    // };
    let trace_layer = layer().pretty().with_writer(stdout);
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

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("started");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
