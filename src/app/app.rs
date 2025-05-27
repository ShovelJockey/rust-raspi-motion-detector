use crate::app::{middleware, routes, task::ThreadPool, web_routes};
use crate::motion_detect::gpio::MotionDetector;
use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
    handler::HandlerWithoutStateExt,
    http::{uri::Authority, StatusCode, Uri},
    response::Redirect,
    BoxError
};
use axum_extra::extract::Host;
use std::{env::var, fs::File, sync::Arc, net::SocketAddr};
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

fn make_https(host: &str, uri: Uri, https_port: u16) -> Result<Uri, BoxError> {
    let mut parts = uri.into_parts();

    parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

    if parts.path_and_query.is_none() {
        parts.path_and_query = Some("/".parse().unwrap());
    }

    let authority: Authority = host.parse()?;
    let bare_host = match authority.port() {
        Some(port_struct) => authority
            .as_str()
            .strip_suffix(port_struct.as_str())
            .unwrap()
            .strip_suffix(':')
            .unwrap(), // if authority.port() is Some(port) then we can be sure authority ends with :{port}
        None => authority.as_str(),
    };

    parts.authority = Some(format!("{bare_host}:{https_port}").parse()?);

    Ok(Uri::from_parts(parts)?)
}

pub async fn redirect_http_to_https() {
    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri, 3001) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], 7878));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}
