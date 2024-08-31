use axum::{Router, serve, routing::get};
use tokio;


async fn basic_route() -> String {
    return "test".to_string();
}
async fn create_app() -> Router {
    let app = Router::new().route("/", get(basic_route));
    app
}


#[tokio::main]
async fn main() {
    let app = create_app().await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let server = serve(listener, app);
}


// look at why initial pause isnt working, see if it is compatible with streaming, 
// also consider that stream seems to crash on client disconnect with listen - could require rethinking? though if pause works with signal thats fine,
// how to test? maybe with rust?
// always on camera mode for app,
// 