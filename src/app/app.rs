use crate::app::{middleware, routes, task::ThreadPool, web_routes};
use crate::motion_detect::gpio::MotionDetector;
use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::{env::var, fs::File, sync::Arc};
use tower_http::services::ServeDir;
use tracing_subscriber::{fmt::layer, prelude::*, registry};

pub async fn create_app(motion_detector: MotionDetector) -> Router {
    let thread_pool = ThreadPool::new(20).await;

    let file_dir = var("LOG_PATH").unwrap_or("/home".to_string());
    let file = match File::create_new(&file_dir) {
        Ok(file) => file,
        Err(_) => File::open(&file_dir).expect("Open already existing file"),
    };
    let trace_layer = layer().pretty().with_writer(file);
    registry().with(trace_layer).init();

    let session_store = middleware::build_session_layer().await;

    let app = Router::new()
        .route("/start_cam", post(routes::init_camera))
        .route("/shutdown", post(routes::shutdown_device))
        .route("/cam_status", get(routes::get_current_cam_status))
        .with_state(Arc::new(motion_detector))
        .route("/start_download", post(routes::start_download))
        .route("/download", get(routes::download_from_task))
        .with_state(Arc::new(thread_pool))
        .route("/file", get(routes::stream))
        .route("/video_data", get(routes::get_all_videos_data))
        .route("/dashboard", get(web_routes::index))
        .route("/play_videos", get(web_routes::play_videos))
        .route("/watch_stream", get(web_routes::watch_stream))
        .route(
            "/motion_detector_controls",
            get(web_routes::motion_detector_controls),
        )
        .layer(from_fn(middleware::auth_session))
        .route("/auth_login", post(middleware::auth_login))
        .route("/login", get(web_routes::login))
        .layer(session_store)
        .nest_service(
            "/static",
            ServeDir::new("/home/jamie/coding/rust-raspi-motion-detector/frontend/static"),
        );
    app
}
