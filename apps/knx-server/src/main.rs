use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("KNX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let static_dir = std::env::var("KNX_STATIC_DIR").ok().map(PathBuf::from);

    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, static_dir);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind 0.0.0.0:{port}: {e}"));
    println!("knx-server listening on :{port}");
    axum::serve(listener, app).await.expect("server error");
}
