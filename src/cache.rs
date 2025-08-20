use moka::future::Cache;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::config::CONFIG;
use crate::types::{GeItemList, GeItem, GeGraph};

// Cache keys structure
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CacheKey {
    ItemsList { 
        category: i32, 
        alpha: String, 
        page: u32, 
        game: String 
    },
    ItemDetail { 
        id: i64, 
        game: String 
    },
    Graph { 
        id: i64, 
        game: String 
    },
    Info { 
        game: String 
    },
}

// Cached value wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CachedValue {
    ItemsList(GeItemList),
    ItemDetail(GeItem),
    Graph(GeGraph),
    Info(serde_json::Value),
}

// Global cache instance
pub static CACHE: Lazy<CacheManager> = Lazy::new(|| {
    CacheManager::new(&CONFIG)
});

pub struct CacheManager {
    cache: Arc<Cache<CacheKey, CachedValue>>,
    enabled: bool,
}

impl CacheManager {
    pub fn new(config: &crate::config::Config) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.cache.max_capacity)
            .time_to_live(Duration::from_secs(config.cache.ttl_seconds))
            .build();
        
        Self {
            cache: Arc::new(cache),
            enabled: config.cache.enabled,
        }
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CachedValue> {
        if !self.enabled {
            return None;
        }
        
        self.cache.get(key).await
    }
    
    pub async fn set(&self, key: CacheKey, value: CachedValue, ttl: Option<Duration>) {
        if !self.enabled {
            return;
        }
        
        if let Some(ttl) = ttl {
            // Set with custom TTL
            self.cache.insert(key, value).await;
            // Note: moka doesn't support per-key TTL easily, 
            // so we'd need a more complex solution for production
        } else {
            self.cache.insert(key, value).await;
        }
    }
    
    pub async fn invalidate(&self, key: &CacheKey) {
        self.cache.invalidate(key).await;
    }
    
    pub async fn clear(&self) {
        self.cache.invalidate_all().await;
    }
    
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            enabled: self.enabled,
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub enabled: bool,
    pub entry_count: u64,
    pub weighted_size: u64,
}

// Helper functions for cache key generation
impl CacheKey {
    pub fn items_list(category: i32, alpha: &str, page: u32, game: &str) -> Self {
        Self::ItemsList {
            category,
            alpha: alpha.to_string(),
            page,
            game: game.to_string(),
        }
    }
    
    pub fn item_detail(id: i64, game: &str) -> Self {
        Self::ItemDetail {
            id,
            game: game.to_string(),
        }
    }
    
    pub fn graph(id: i64, game: &str) -> Self {
        Self::Graph {
            id,
            game: game.to_string(),
        }
    }
    
    pub fn info(game: &str) -> Self {
        Self::Info {
            game: game.to_string(),
        }
    }
}
