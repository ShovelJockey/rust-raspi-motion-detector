use axum::{http::StatusCode, response::Html};
use tokio::fs::read_to_string;

pub async fn index() -> Result<Html<String>, (StatusCode, String)> {
    let html_content =
        match read_to_string("/home/jamie/coding/rust-raspi-motion-detector/frontend/index.html")
            .await
        {
            Ok(content) => content,
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read HTML file: {}", err),
                ));
            }
        };
    Ok(Html(html_content))
}

pub async fn play_videos() -> Result<Html<String>, (StatusCode, String)> {
    let html_content = match read_to_string(
        "/home/jamie/coding/rust-raspi-motion-detector/frontend/play_videos.html",
    )
    .await
    {
        Ok(content) => content,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read HTML file: {}", err),
            ));
        }
    };
    Ok(Html(html_content))
}

pub async fn motion_detector_controls() -> Result<Html<String>, (StatusCode, String)> {
    let html_content = match read_to_string(
        "/home/jamie/coding/rust-raspi-motion-detector/frontend/motion_detector_controls.html",
    )
    .await
    {
        Ok(content) => content,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read HTML file: {}", err),
            ));
        }
    };
    Ok(Html(html_content))
}

pub async fn watch_stream() -> Result<Html<String>, (StatusCode, String)> {
    let html_content =
        match read_to_string("/home/jamie/coding/rust-raspi-motion-detector/frontend/watch_stream.html")
            .await
        {
            Ok(content) => content,
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read HTML file: {}", err),
                ));
            }
        };
    Ok(Html(html_content))
}

pub async fn login() -> Result<Html<String>, (StatusCode, String)> {
    let html_content =
        match read_to_string("/home/jamie/coding/rust-raspi-motion-detector/frontend/login.html")
            .await
        {
            Ok(content) => content,
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read HTML file: {}", err),
                ));
            }
        };
    Ok(Html(html_content))
}
