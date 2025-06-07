use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{Redirect, Response},
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use tower_sessions::{cookie::time::Duration, Expiry, MemoryStore, Session, SessionManagerLayer};

pub async fn build_session_layer() -> SessionManagerLayer<MemoryStore> {
    let session_store = MemoryStore::default();
    SessionManagerLayer::new(session_store)
        .with_http_only(true)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(10)))
        .with_name("site.sid")
}

pub async fn auth_session(request: Request<Body>, next: Next) -> Result<Response, Response> {
    use axum::response::IntoResponse;
    let session = request
        .extensions()
        .get::<Session>()
        .ok_or_else(|| (axum::http::StatusCode::UNAUTHORIZED, "Login required").into_response())?;
    let id = session.id().unwrap_or_default();
    println!("{id}");
    if session.is_empty().await {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Login required").into_response());
    }

    Ok(next.run(request).await)
}

#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
}

pub async fn auth_login(session: Session, Form(login_form): Form<LoginForm>) -> Response {
    use axum::response::IntoResponse;
    let pwd = std::env::var("PASSWORD").expect("a Password has been set");
    let print = &login_form.password;
    println!("{print}");
    println!("{pwd}");
    if pwd != login_form.password.to_lowercase() {
        return (axum::http::StatusCode::UNAUTHORIZED, "Bad password").into_response();
    }
    let id = session.id().unwrap_or_default();
    println!("Id: {id}");
    let time = Utc::now().timestamp();
    match session.insert("session_started", time).await {
        Ok(_) => {
            return Redirect::to("/dashboard").into_response();
        }
        Err(err) => {
            println!("Hit error trying to store session, error: {err}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Bad password",
            )
                .into_response();
        }
    };
}
