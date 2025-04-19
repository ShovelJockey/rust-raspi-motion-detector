use axum::{
    http::StatusCode,
    response::Html,
};
use tokio::fs::read_to_string;


pub async fn index() -> Result<Html<String>, (StatusCode, String)> {
    let html_content = match read_to_string("/home/jamie/coding/rust/motion_camera_server/frontend/index.html").await {
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
    let html_content = match read_to_string("/home/jamie/coding/rust/motion_camera_server/frontend/play_videos.html").await {
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

pub async fn start_motion_detector() -> Result<Html<String>, (StatusCode, String)> {
    let html_content = match read_to_string("/home/jamie/coding/rust/motion_camera_server/frontend/start_motion_detector.html").await {
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
