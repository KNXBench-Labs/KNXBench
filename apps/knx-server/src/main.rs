//! The standalone `knx-server` binary: HTTP API and built frontend on one port.

use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if matches!(std::env::args().nth(1).as_deref(), Some("--version" | "-V")) {
        println!("{}", knx_server::version_line());
        return;
    }
    let port: u16 = std::env::var("KNX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let static_dir = std::env::var("KNX_STATIC_DIR").ok().map(PathBuf::from);
    let data_dir = std::env::var("KNX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    // Created up front rather than lazily: every path under `data_dir` is
    // resolved by canonicalizing it, which fails outright if the root
    // itself does not exist — a `KNX_DATA_DIR` pointing at a volume that
    // has not been created yet would otherwise turn every `/api/fs/*`
    // call into an opaque "data dir unreadable" 500. Same call the Tauri
    // shell already makes for its own data dir.
    std::fs::create_dir_all(&data_dir)
        .unwrap_or_else(|e| panic!("failed to create data dir {}: {e}", data_dir.display()));

    let state = Arc::new(knx_server::AppState::new(data_dir));
    let app = knx_server::app(state, static_dir);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind 0.0.0.0:{port}: {e}"));
    println!("knx-server listening on :{port}");
    axum::serve(listener, app).await.expect("server error");
}
