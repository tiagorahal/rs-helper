mod types;
mod ge;
mod ui; // <— novo

use axum::{routing::get, Router};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/", ui::router())
        .nest("/v1/ge", ge::router())
        .route("/healthz", get(|| async { "ok" }))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    let _ = axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await;
}
