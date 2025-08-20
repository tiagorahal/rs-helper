#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use serde_json::Value;
    
    // Helper to create test app
    async fn app() -> axum::Router {
        rs_helper::build_app().await.unwrap()
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let app = app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"OK");
    }
    
    #[tokio::test]
    async fn test_index_page() {
        let app = app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        
        assert!(html.contains("RS Helper"));
        assert!(html.contains("RuneScape Grand Exchange Explorer"));
    }
    
    #[tokio::test]
    async fn test_categories_endpoint() {
        let app = app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/ge/categories")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert!(json["categories"].is_array());
        assert!(!json["categories"].as_array().unwrap().is_empty());
        
        // Check first category
        let first = &json["categories"][0];
        assert_eq!(first["id"], 0);
        assert_eq!(first["name"], "Miscellaneous");
    }
    
    #[tokio::test]
    async fn test_items_endpoint_validation() {
        let app = app().await;
        
        // Test with invalid alpha
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/ge/items?category=0&alpha=invalid")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["code"], "bad_request");
        assert!(json["message"].as_str().unwrap().contains("alpha"));
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        use std::time::Duration;
        use tokio::time::sleep;
        
        let app = app().await;
        
        // Make many requests quickly
        let mut responses = Vec::new();
        for _ in 0..200 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/healthz")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap();
            
            responses.push(response.status());
            
            // Small delay to avoid overwhelming
            sleep(Duration::from_millis(10)).await;
        }
        
        // Check that some requests were rate limited
        let rate_limited = responses.iter()
            .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
            .count();
        
        assert!(rate_limited > 0, "Expected some requests to be rate limited");
    }
    
    #[tokio::test]
    async fn test_cache_stats_endpoint() {
        let app = app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/cache/stats")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert!(json["enabled"].is_boolean());
        assert!(json["entry_count"].is_number());
        assert!(json["weighted_size"].is_number());
    }
    
    #[tokio::test]
    async fn test_cors_headers() {
        let app = app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/ge/categories")
                    .header("Origin", "https://example.com")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        // Check CORS headers
        assert!(response.headers().contains_key("access-control-allow-origin"));
    }
}

#[cfg(test)]
mod unit_tests {
    use rs_helper::types::*;
    
    #[test]
    fn test_price_parsing() {
        assert_eq!(parse_price_to_i64(&serde_json::json!("1.5m")), Some(1_500_000));
        assert_eq!(parse_price_to_i64(&serde_json::json!("-2.3k")), Some(-2_300));
        assert_eq!(parse_price_to_i64(&serde_json::json!(100)), Some(100));
        assert_eq!(parse_price_to_i64(&serde_json::json!("5.5b")), Some(5_500_000_000));
        assert_eq!(parse_price_to_i64(&serde_json::json!("invalid")), None);
    }
    
    #[test]
    fn test_price_formatting() {
        assert_eq!(format_price_compact(1_500_000), "1.5m");
        assert_eq!(format_price_compact(-2_300), "-2.3k");
        assert_eq!(format_price_compact(100), "100");
        assert_eq!(format_price_compact(5_500_000_000), "5.5b");
        assert_eq!(format_price_compact(999), "999");
        assert_eq!(format_price_compact(1_000), "1.0k");
        assert_eq!(format_price_compact(10_500), "10.5k");
        assert_eq!(format_price_compact(100_000), "100k");
    }
    
    #[test]
    fn test_game_display() {
        assert_eq!(Game::Rs3.to_string(), "rs3");
        assert_eq!(Game::Osrs.to_string(), "osrs");
    }
    
    #[test]
    fn test_game_default() {
        let game: Game = Default::default();
        assert_eq!(game, Game::Rs3);
    }
}
