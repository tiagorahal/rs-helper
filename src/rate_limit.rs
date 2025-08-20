use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

use crate::config::CONFIG;
use crate::types::ErrorBody;

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

pub static RATE_LIMITER: Lazy<SharedRateLimiter> = Lazy::new(|| {
    let quota = Quota::per_second(
        std::num::NonZeroU32::new(CONFIG.rate_limit.requests_per_second)
            .unwrap_or_else(|| std::num::NonZeroU32::new(50).unwrap())
    )
    .allow_burst(
        std::num::NonZeroU32::new(CONFIG.rate_limit.burst_size)
            .unwrap_or_else(|| std::num::NonZeroU32::new(100).unwrap())
    );
    
    Arc::new(RateLimiter::direct(quota))
});

pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
    if !CONFIG.rate_limit.enabled {
        return Ok(next.run(req).await);
    }
    
    // Check rate limit
    match RATE_LIMITER.check() {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => {
            // Rate limit exceeded
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorBody {
                    code: "rate_limit_exceeded".to_string(),
                    message: "Too many requests. Please slow down.".to_string(),
                }),
            ))
        }
    }
}

// Per-IP rate limiting (more advanced)
pub mod ip_based {
    use super::*;
    use axum::extract::ConnectInfo;
    use governor::state::keyed::{DashMapStateStore, KeyedRateLimiter};
    use std::net::SocketAddr;
    
    pub type IpRateLimiter = Arc<KeyedRateLimiter<String, DashMapStateStore<String>, DefaultClock>>;
    
    pub static IP_RATE_LIMITER: Lazy<IpRateLimiter> = Lazy::new(|| {
        let quota = Quota::per_second(
            std::num::NonZeroU32::new(CONFIG.rate_limit.requests_per_second / 2)
                .unwrap_or_else(|| std::num::NonZeroU32::new(25).unwrap())
        );
        
        Arc::new(KeyedRateLimiter::dashmap(quota))
    });
    
    pub async fn ip_rate_limit_middleware(
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        req: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
        if !CONFIG.rate_limit.enabled {
            return Ok(next.run(req).await);
        }
        
        let ip = addr.ip().to_string();
        
        match IP_RATE_LIMITER.check_key(&ip) {
            Ok(_) => Ok(next.run(req).await),
            Err(_) => {
                Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorBody {
                        code: "ip_rate_limit_exceeded".to_string(),
                        message: format!("Too many requests from IP {}. Please slow down.", ip),
                    }),
                ))
            }
        }
    }
}
