mod types;
mod ge;
mod ui;
mod config;
mod cache;
mod rate_limit;
mod error;
mod metrics;

use axum::{
    routing::get,
    Router,
    middleware,
};
use std::net::SocketAddr;
use tower_http::{
    cors::{CorsLayer, Any},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::CONFIG;
use crate::metrics::metrics_app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    init_tracing();
    
    // Log startup info
    tracing::info!(
        "Starting RS Helper v{} on {}:{}",
        env!("CARGO_PKG_VERSION"),
        CONFIG.server.host,
        CONFIG.server.port
    );
    
    // Build main application
    let app = build_app().await?;
    
    // Start metrics server if enabled
    if CONFIG.metrics.enabled {
        tokio::spawn(async move {
            let metrics_addr: SocketAddr = 
                format!("{}:{}", CONFIG.server.host, CONFIG.metrics.port)
                    .parse()
                    .expect("Invalid metrics address");
            
            let metrics_app = metrics_app();
            
            tracing::info!("Metrics server listening on {}", metrics_addr);
            
            let listener = tokio::net::TcpListener::bind(metrics_addr)
                .await
                .expect("Failed to bind metrics port");
                
            axum::serve(listener, metrics_app)
                .await
                .expect("Metrics server failed");
        });
    }
    
    // Start main server
    let addr: SocketAddr = 
        format!("{}:{}", CONFIG.server.host, CONFIG.server.port)
            .parse()?;
    
    tracing::info!("Server listening on {}", addr);
    tracing::info!("UI available at http://{}", addr);
    tracing::info!("API available at http://{}/v1/ge", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn build_app() -> anyhow::Result<Router> {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Build router with all routes
    let app = Router::new()
        // UI routes (HTML pages)
        .nest("/", ui::router())
        // API routes (JSON)
        .nest("/v1/ge", ge::router())
        // Health check
        .route("/healthz", get(health_check))
        // Cache stats (admin endpoint)
        .route("/admin/cache/stats", get(cache_stats))
        .route("/admin/cache/clear", post(clear_cache))
        // Apply middlewares
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http());
    
    Ok(app)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn cache_stats() -> axum::Json<cache::CacheStats> {
    axum::Json(cache::CACHE.stats().await)
}

async fn clear_cache() -> &'static str {
    cache::CACHE.clear().await;
    "Cache cleared"
}

use axum::routing::post;

fn init_tracing() {
    let log_level = CONFIG.server.log_level.parse()
        .unwrap_or(tracing::Level::INFO);
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("rs_helper={},tower_http=debug", log_level).into()
                }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
