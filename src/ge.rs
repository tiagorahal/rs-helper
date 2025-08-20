use axum::{
    extract::{Path, Query},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::{
    cache::{CACHE, CacheKey, CachedValue},
    config::CONFIG,
    error::{AppError, AppResult},
    metrics::{record_cache_hit, record_cache_miss, record_upstream_request, RequestTimer},
    types::*,
};

// Router configuration
pub fn router() -> Router {
    Router::new()
        .route("/info", get(ge_info))
        .route("/categories", get(ge_categories))
        .route("/items", get(ge_items))
        .route("/items/:id", get(ge_item_detail))
        .route("/graph/:id", get(ge_graph))
}

// HTTP client singleton
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent(&CONFIG.upstream.user_agent)
        .timeout(Duration::from_secs(CONFIG.upstream.timeout_seconds))
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to create HTTP client")
});

// Info endpoint with caching
pub async fn ge_info(Query(q): Query<InfoQuery>) -> AppResult<Json<GeInfo>> {
    let timer = RequestTimer::new("/v1/ge/info");
    let cache_key = CacheKey::info(&q.game.to_string());
    
    // Check cache
    if let Some(CachedValue::Info(cached)) = CACHE.get(&cache_key).await {
        record_cache_hit("info");
        timer.record(200);
        
        let mut info: GeInfo = serde_json::from_value(cached)?;
        info.cached = Some(true);
        return Ok(Json(info));
    }
    
    record_cache_miss("info");
    
    // Fetch from upstream
    let (base, _) = base_urls(q.game);
    let url = format!("{base}/api/info.json");
    
    let start = Instant::now();
    let response = fetch_with_retry(&url).await?;
    let duration = start.elapsed().as_secs_f64();
    record_upstream_request("runescape_api", true, duration);
    
    let runedate = response
        .get("runedate")
        .and_then(|v| v.as_i64())
        .or_else(|| response.get("lastConfigUpdateRuneday").and_then(|v| v.as_i64()))
        .unwrap_or_default();
    
    let info = GeInfo {
        runedate,
        last_updated_at: Some(Utc::now()),
        cached: Some(false),
    };
    
    // Cache the result
    CACHE.set(
        cache_key,
        CachedValue::Info(serde_json::to_value(&info)?),
        Some(CONFIG.cache_ttl()),
    ).await;
    
    timer.record(200);
    Ok(Json(info))
}

// Categories endpoint (static but can be cached)
pub async fn ge_categories() -> AppResult<Json<GeCategories>> {
    let timer = RequestTimer::new("/v1/ge/categories");
    
    let categories = GeCategories {
        categories: CATEGORIES.iter().copied().collect(),
        cached: Some(false),
    };
    
    timer.record(200);
    Ok(Json(categories))
}

// Items search with caching
pub async fn ge_items(Query(q): Query<ItemsQuery>) -> AppResult<Json<GeItemList>> {
    let timer = RequestTimer::new("/v1/ge/items");
    let alpha = q.alpha.as_deref().unwrap_or("a");
    let page = q.page.unwrap_or(1);
    
    validate_alpha(alpha)?;
    
    let cache_key = CacheKey::items_list(q.category, alpha, page, &q.game.to_string());
    
    // Check cache
    if let Some(CachedValue::ItemsList(mut cached)) = CACHE.get(&cache_key).await {
        record_cache_hit("items_list");
        cached.cached = Some(true);
        timer.record(200);
        return Ok(Json(cached));
    }
    
    record_cache_miss("items_list");
    
    // Fetch from upstream
    let mut items = fetch_items_internal(q.category, alpha, page, q.game).await?;
    items.page = Some(page);
    items.cached = Some(false);
    
    // Cache with shorter TTL for item lists
    CACHE.set(
        cache_key,
        CachedValue::ItemsList(items.clone()),
        Some(CONFIG.items_cache_ttl()),
    ).await;
    
    timer.record(200);
    Ok(Json(items))
}

// Item detail with caching
pub async fn ge_item_detail(
    Path(id): Path<i64>,
    Query(q): Query<ItemDetailQuery>,
) -> AppResult<Json<GeItem>> {
    let timer = RequestTimer::new("/v1/ge/items/:id");
    let cache_key = CacheKey::item_detail(id, &q.game.to_string());
    
    // Check cache
    if let Some(CachedValue::ItemDetail(mut cached)) = CACHE.get(&cache_key).await {
        record_cache_hit("item_detail");
        cached.cached = Some(true);
        timer.record(200);
        return Ok(Json(cached));
    }
    
    record_cache_miss("item_detail");
    
    // Fetch from upstream
    let (base, cat_name_map) = base_urls(q.game);
    let url = format!("{base}/api/catalogue/detail.json?item={id}");
    
    let start = Instant::now();
    let raw = fetch_with_retry(&url).await?;
    let duration = start.elapsed().as_secs_f64();
    record_upstream_request("runescape_api", true, duration);
    
    let item = raw
        .get("item")
        .ok_or_else(|| AppError::Upstream("Missing 'item' field in response".to_string()))?;
    
    let mut parsed_item = parse_item(item, &cat_name_map)?;
    parsed_item.cached = Some(false);
    
    // Cache the result
    CACHE.set(
        cache_key,
        CachedValue::ItemDetail(parsed_item.clone()),
        Some(CONFIG.cache_ttl()),
    ).await;
    
    timer.record(200);
    Ok(Json(parsed_item))
}

// Graph endpoint with caching
pub async fn ge_graph(
    Path(id): Path<i64>,
    Query(q): Query<GraphQuery>,
) -> AppResult<Json<GeGraph>> {
    let timer = RequestTimer::new("/v1/ge/graph/:id");
    let cache_key = CacheKey::graph(id, &q.game.to_string());
    
    // Check cache
    if let Some(CachedValue::Graph(mut cached)) = CACHE.get(&cache_key).await {
        record_cache_hit("graph");
        cached.cached = Some(true);
        timer.record(200);
        return Ok(Json(cached));
    }
    
    record_cache_miss("graph");
    
    // Fetch from upstream
    let (base, _) = base_urls(q.game);
    let url = format!("{base}/api/graph/{id}.json");
    
    let start = Instant::now();
    let raw = fetch_with_retry(&url).await?;
    let duration = start.elapsed().as_secs_f64();
    record_upstream_request("runescape_api", true, duration);
    
    let mut graph = parse_graph(id, &raw)?;
    graph.cached = Some(false);
    
    // Cache the result
    CACHE.set(
        cache_key,
        CachedValue::Graph(graph.clone()),
        Some(CONFIG.cache_ttl()),
    ).await;
    
    timer.record(200);
    Ok(Json(graph))
}

// Internal function for fetching items
async fn fetch_items_internal(
    category: i32,
    alpha: &str,
    page: u32,
    game: Game,
) -> AppResult<GeItemList> {
    let (base, cat_name_map) = base_urls(game);
    let url = format!("{base}/api/catalogue/items.json?category={}&alpha={}&page={}", 
                      category, alpha, page);
    
    let start = Instant::now();
    let raw = fetch_with_retry(&url).await?;
    let duration = start.elapsed().as_secs_f64();
    record_upstream_request("runescape_api", true, duration);
    
    let total = raw.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut items = Vec::new();
    
    if let Some(arr) = raw.get("items").and_then(|v| v.as_array()) {
        for item_json in arr {
            if let Ok(item) = parse_item(item_json, &cat_name_map) {
                items.push(item);
            }
        }
    }
    
    Ok(GeItemList { total, items, page: Some(page), cached: Some(false) })
}

// Helper: Fetch with retry logic
async fn fetch_with_retry(url: &str) -> AppResult<Value> {
    let mut retries = 0;
    let max_retries = CONFIG.upstream.max_retries;
    
    loop {
        match HTTP_CLIENT.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return response.json().await
                        .map_err(|e| AppError::Upstream(format!("Failed to parse JSON: {}", e)));
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    
                    if retries < max_retries {
                        retries += 1;
                        tracing::warn!("Request failed with status {}, retry {}/{}", 
                                     status, retries, max_retries);
                        tokio::time::sleep(Duration::from_millis(100 * (2_u64.pow(retries)))).await;
                        continue;
                    }
                    
                    return Err(AppError::Upstream(
                        format!("HTTP {}: {}", status, body)
                    ));
                }
            }
            Err(e) => {
                if retries < max_retries {
                    retries += 1;
                    tracing::warn!("Request error: {}, retry {}/{}", e, retries, max_retries);
                    tokio::time::sleep(Duration::from_millis(100 * (2_u64.pow(retries)))).await;
                    continue;
                }
                
                record_upstream_request("runescape_api", false, 0.0);
                return Err(AppError::Network(e));
            }
        }
    }
}

// Helper: Parse item from JSON
fn parse_item(json: &Value, cat_map: &HashMap<i32, &'static str>) -> AppResult<GeItem> {
    let id = json.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let type_name = json.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let category_id = category_id_from_name(&type_name);
    
    let members = json
        .get("members")
        .and_then(|v| v.as_str())
        .map(|s| s == "true")
        .or_else(|| json.get("members").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    
    let icons = GeIcons {
        small: json.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        large: json.get("icon_large").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    };
    
    let current_price = json.get("current").and_then(|c| c.get("price")).and_then(parse_price_to_i64);
    let today_change = json.get("today").and_then(|c| c.get("price")).and_then(parse_price_to_i64);
    
    let change_percentage = if let (Some(current), Some(change)) = (current_price, today_change) {
        if current > 0 {
            Some((change as f64 / current as f64) * 100.0)
        } else {
            None
        }
    } else {
        None
    };
    
    let price = GePrice {
        trend: json
            .get("current")
            .and_then(|c| c.get("trend"))
            .and_then(|v| v.as_str())
            .unwrap_or("neutral")
            .to_string(),
        current: current_price,
        today_change,
        change_percentage,
    };
    
    let description = json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    Ok(GeItem {
        id,
        name,
        category_id,
        category_name: if category_id >= 0 {
            cat_map.get(&(category_id as i32))
                .map(|s| s.to_string())
                .unwrap_or(type_name)
        } else {
            type_name
        },
        members,
        icons,
        price,
        description,
        cached: None,
    })
}

// Helper: Parse graph data
fn parse_graph(id: i64, json: &Value) -> AppResult<GeGraph> {
    let mut daily = Vec::new();
    let mut average = Vec::new();
    
    if let Some(d) = json.get("daily").and_then(|v| v.as_object()) {
        for (k, v) in d {
            if let (Ok(ms), Some(price)) = (k.parse::<i64>(), v.as_i64()) {
                daily.push(TsPrice { 
                    ts: DateTime::from_timestamp_millis(ms)
                        .unwrap_or_else(|| Utc::now()),
                    price 
                });
            }
        }
        daily.sort_by_key(|p| p.ts);
    }
    
    if let Some(a) = json.get("average").and_then(|v| v.as_object()) {
        for (k, v) in a {
            if let (Ok(ms), Some(price)) = (k.parse::<i64>(), v.as_i64()) {
                average.push(TsPrice { 
                    ts: DateTime::from_timestamp_millis(ms)
                        .unwrap_or_else(|| Utc::now()),
                    price 
                });
            }
        }
        average.sort_by_key(|p| p.ts);
    }
    
    Ok(GeGraph { id, daily, average, cached: None })
}

// Validation helpers
fn validate_alpha(alpha: &str) -> AppResult<()> {
    if alpha == "#" {
        return Ok(());
    }
    if alpha.len() == 1 && alpha.chars().all(|c| c.is_ascii_lowercase()) {
        return Ok(());
    }
    Err(AppError::BadRequest(
        "Alpha must be a single lowercase letter a-z or '#'".to_string()
    ))
}

// Categories data
pub const CATEGORIES: &[GeCategory] = &[
    GeCategory { id: 0, name: "Miscellaneous" },
    GeCategory { id: 1, name: "Ammo" },
    GeCategory { id: 2, name: "Arrows" },
    GeCategory { id: 3, name: "Bolts" },
    GeCategory { id: 4, name: "Construction materials" },
    GeCategory { id: 5, name: "Construction products" },
    GeCategory { id: 6, name: "Cooking ingredients" },
    GeCategory { id: 7, name: "Costumes" },
    GeCategory { id: 8, name: "Crafting materials" },
    GeCategory { id: 9, name: "Familiars" },
    GeCategory { id: 10, name: "Farming produce" },
    GeCategory { id: 11, name: "Fletching materials" },
    GeCategory { id: 12, name: "Food and Drink" },
    GeCategory { id: 13, name: "Herblore materials" },
    GeCategory { id: 14, name: "Hunting equipment" },
    GeCategory { id: 15, name: "Hunting Produce" },
    GeCategory { id: 16, name: "Jewellery" },
    GeCategory { id: 17, name: "Mage armour" },
    GeCategory { id: 18, name: "Mage weapons" },
    GeCategory { id: 19, name: "Melee armour - low level" },
    GeCategory { id: 20, name: "Melee armour - mid level" },
    GeCategory { id: 21, name: "Melee armour - high level" },
    GeCategory { id: 22, name: "Melee weapons - low level" },
    GeCategory { id: 23, name: "Melee weapons - mid level" },
    GeCategory { id: 24, name: "Melee weapons - high level" },
    GeCategory { id: 25, name: "Mining and Smithing" },
    GeCategory { id: 26, name: "Potions" },
    GeCategory { id: 27, name: "Prayer armour" },
    GeCategory { id: 28, name: "Prayer materials" },
    GeCategory { id: 29, name: "Range armour" },
    GeCategory { id: 30, name: "Range weapons" },
    GeCategory { id: 31, name: "Runecrafting" },
    GeCategory { id: 32, name: "Runes, Spells and Teleports" },
    GeCategory { id: 33, name: "Seeds" },
    GeCategory { id: 34, name: "Summoning scrolls" },
    GeCategory { id: 35, name: "Tools and containers" },
    GeCategory { id: 36, name: "Woodcutting product" },
    GeCategory { id: 37, name: "Pocket items" },
    GeCategory { id: 38, name: "Stone spirits" },
    GeCategory { id: 39, name: "Salvage" },
    GeCategory { id: 40, name: "Firemaking products" },
    GeCategory { id: 41, name: "Archaeology materials" },
    GeCategory { id: 42, name: "Wood spirits" },
    GeCategory { id: 43, name: "Necromancy armour" },
];

fn categories_map() -> HashMap<i32, &'static str> {
    CATEGORIES.iter().map(|c| (c.id, c.name)).collect()
}

fn category_id_from_name(name: &str) -> i32 {
    for c in CATEGORIES {
        if c.name.eq_ignore_ascii_case(name) {
            return c.id;
        }
    }
    -1
}

fn base_urls(game: Game) -> (&'static str, HashMap<i32, &'static str>) {
    static RS3_CAT: Lazy<HashMap<i32, &'static str>> = Lazy::new(categories_map);
    static OSRS_CAT: Lazy<HashMap<i32, &'static str>> = Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert(1, "All items (OSRS)");
        m
    });
    
    match game {
        Game::Rs3 => ("https://secure.runescape.com/m=itemdb_rs", RS3_CAT.clone()),
        Game::Osrs => ("https://secure.runescape.com/m=itemdb_oldschool", OSRS_CAT.clone()),
    }
}

// Public function for UI use
pub async fn fetch_items_for_ui(
    category: i32,
    alpha: &str,
    page: u32,
    game: Game,
) -> Result<GeItemList, String> {
    fetch_items_internal(category, alpha, page, game)
        .await
        .map_err(|e| e.to_string())
}
