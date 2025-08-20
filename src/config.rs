use serde::Deserialize;
use std::time::Duration;
use once_cell::sync::Lazy;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub cache: CacheConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    #[serde(default = "default_cache_max_capacity")]
    pub max_capacity: u64,
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,
    #[serde(default = "default_cache_items_ttl")]
    pub items_ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_requests")]
    pub requests_per_second: u32,
    #[serde(default = "default_rate_limit_burst")]
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamConfig {
    #[serde(default = "default_upstream_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_upstream_retries")]
    pub max_retries: u32,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

// Default values
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 3000 }
fn default_log_level() -> String { "info".to_string() }
fn default_cache_enabled() -> bool { true }
fn default_cache_max_capacity() -> u64 { 10_000 }
fn default_cache_ttl() -> u64 { 300 } // 5 minutes
fn default_cache_items_ttl() -> u64 { 60 } // 1 minute for item lists
fn default_rate_limit_enabled() -> bool { true }
fn default_rate_limit_requests() -> u32 { 50 }
fn default_rate_limit_burst() -> u32 { 100 }
fn default_upstream_timeout() -> u64 { 10 }
fn default_upstream_retries() -> u32 { 3 }
fn default_user_agent() -> String { "rs-helper/0.2 (unofficial)".to_string() }
fn default_metrics_enabled() -> bool { true }
fn default_metrics_port() -> u16 { 9090 }

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
                log_level: default_log_level(),
            },
            cache: CacheConfig {
                enabled: default_cache_enabled(),
                max_capacity: default_cache_max_capacity(),
                ttl_seconds: default_cache_ttl(),
                items_ttl_seconds: default_cache_items_ttl(),
            },
            rate_limit: RateLimitConfig {
                enabled: default_rate_limit_enabled(),
                requests_per_second: default_rate_limit_requests(),
                burst_size: default_rate_limit_burst(),
            },
            upstream: UpstreamConfig {
                timeout_seconds: default_upstream_timeout(),
                max_retries: default_upstream_retries(),
                user_agent: default_user_agent(),
            },
            metrics: MetricsConfig {
                enabled: default_metrics_enabled(),
                port: default_metrics_port(),
            },
        }
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // Try to load from environment variables or config file
    dotenvy::dotenv().ok();
    
    // For now, use defaults. In production, load from env/file
    Config::default()
});

impl Config {
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache.ttl_seconds)
    }
    
    pub fn items_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache.items_ttl_seconds)
    }
    
    pub fn upstream_timeout(&self) -> Duration {
        Duration::from_secs(self.upstream.timeout_seconds)
    }
}
