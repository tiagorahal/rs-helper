// src/ge.rs
use axum::{
    extract::{Path, Query},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::types::*;

// ---------------- Router ----------------

pub fn router() -> Router {
    Router::new()
        .route("/info", get(ge_info))
        .route("/categories", get(ge_categories))
        .route("/items", get(ge_items))
        .route("/items/:id", get(ge_item_detail))
        .route("/graph/:id", get(ge_graph))
}

// ---------------- Tipos compartilhados ----------------

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Rs3,
    Osrs,
}
impl Default for Game {
    fn default() -> Self {
        Game::Rs3
    }
}

// usado pela rota JSON /v1/ge/items
#[derive(Debug, Deserialize)]
struct ItemsQuery {
    category: i32,
    alpha: Option<String>,
    page: Option<u32>,
    #[serde(default)]
    game: Game,
}

// usado por /v1/ge/info
#[derive(Debug, Deserialize)]
struct InfoQuery {
    #[serde(default)]
    game: Game,
}

// usado por /v1/ge/items/:id e /v1/ge/graph/:id
#[derive(Debug, Deserialize)]
struct ItemDetailQuery {
    #[serde(default)]
    game: Game,
}
#[derive(Debug, Deserialize)]
struct GraphQuery {
    #[serde(default)]
    game: Game,
}

// ---------------- Handlers JSON (API pública) ----------------

pub async fn ge_info(Query(q): Query<InfoQuery>) -> Result<Json<GeInfo>, (axum::http::StatusCode, Json<ErrorBody>)> {
    let (base, _) = base_urls(q.game);
    let url = format!("{base}/api/info.json");

    let client = http();
    let raw: Value = client
        .get(url)
        .send()
        .await
        .map_err(up_err)?
        .json()
        .await
        .map_err(up_err)?;

    // Tenta ler "runedate" ou chaves similares
    let runedate = raw
        .get("runedate")
        .and_then(|v| v.as_i64())
        .or_else(|| raw.get("lastConfigUpdateRuneday").and_then(|v| v.as_i64()))
        .unwrap_or_default();

    Ok(Json(GeInfo {
        runedate,
        last_updated_at: None,
    }))
}

pub async fn ge_categories() -> Result<Json<GeCategories>, (axum::http::StatusCode, Json<ErrorBody>)> {
    Ok(Json(GeCategories {
        categories: CATEGORIES.iter().copied().collect(),
    }))
}

pub async fn ge_items(
    Query(q): Query<ItemsQuery>,
) -> Result<Json<GeItemList>, (axum::http::StatusCode, Json<ErrorBody>)> {
    let alpha = q.alpha.as_deref().unwrap_or("all");
    let page = q.page.unwrap_or(1);

    if alpha == "all" {
        let res = fetch_items_all_for_ui(q.category, page, q.game)
            .await
            .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, Json(ErrorBody {
                code: "upstream_error".into(),
                message: e,
            })))?;
        return Ok(Json(res));
    }

    let res = fetch_items_for_ui(q.category, alpha, page, q.game)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, Json(ErrorBody {
            code: "upstream_error".into(),
            message: e,
        })))?;

    Ok(Json(res))
}

pub async fn ge_item_detail(
    Path(id): Path<i64>,
    Query(q): Query<ItemDetailQuery>,
) -> Result<Json<GeItem>, (axum::http::StatusCode, Json<ErrorBody>)> {
    let (base, cat_name_map) = base_urls(q.game);
    let url = format!("{base}/api/catalogue/detail.json?item={id}");

    let client = http();
    let raw: Value = client
        .get(url)
        .send()
        .await
        .map_err(up_err)?
        .json()
        .await
        .map_err(up_err)?;

    let item = raw
        .get("item")
        .ok_or_else(|| bad_gateway("upstream_format", "missing field 'item'"))?;

    let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let category_name = item.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let category_id = category_id_from_name(&category_name);
    let members = item
        .get("members")
        .and_then(|v| v.as_str())
        .map(|s| s == "true")
        .or_else(|| item.get("members").and_then(|v| v.as_bool()))
        .unwrap_or(false);

    let icons = GeIcons {
        small: item.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        large: item.get("icon_large").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    };

    let price = GePrice {
        trend: item
            .get("current")
            .and_then(|c| c.get("trend"))
            .and_then(|v| v.as_str())
            .unwrap_or("neutral")
            .to_string(),
        current: item.get("current").and_then(|c| c.get("price")).and_then(parse_price_to_i64),
        today_change: item.get("today").and_then(|c| c.get("price")).and_then(parse_price_to_i64),
    };

    let description = item.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

    Ok(Json(GeItem {
        id,
        name,
        category_id,
        category_name: if category_id >= 0 {
            cat_name_map.get(&(category_id as i32)).map(|s| s.to_string()).unwrap_or(category_name)
        } else {
            category_name
        },
        members,
        icons,
        price,
        description,
    }))
}

pub async fn ge_graph(
    Path(id): Path<i64>,
    Query(q): Query<GraphQuery>,
) -> Result<Json<GeGraph>, (axum::http::StatusCode, Json<ErrorBody>)> {
    let (base, _) = base_urls(q.game);
    let url = format!("{base}/api/graph/{id}.json");

    let client = http();
    let raw: Value = client
        .get(url)
        .send()
        .await
        .map_err(up_err)?
        .json()
        .await
        .map_err(up_err)?;

    let mut daily = Vec::new();
    let mut average = Vec::new();

    if let Some(d) = raw.get("daily").and_then(|v| v.as_object()) {
        for (k, v) in d {
            if let (Ok(ms), Some(price)) = (k.parse::<i64>(), v.as_i64()) {
                daily.push(TsPrice { ts: ms_to_iso(ms), price });
            }
        }
        daily.sort_by_key(|p| p.ts);
    }
    if let Some(a) = raw.get("average").and_then(|v| v.as_object()) {
        for (k, v) in a {
            if let (Ok(ms), Some(price)) = (k.parse::<i64>(), v.as_i64()) {
                average.push(TsPrice { ts: ms_to_iso(ms), price });
            }
        }
        average.sort_by_key(|p| p.ts);
    }

    Ok(Json(GeGraph { id, daily, average }))
}

// ---------------- Funções utilitárias usadas pela UI ----------------

pub async fn fetch_items_for_ui(
    category: i32,
    alpha: &str,
    page: u32,
    game: Game,
) -> Result<GeItemList, String> {
    validate_alpha(alpha).map_err(|e| e.to_string())?;

    let (base, cat_name_map) = base_urls(game);

    let alpha_param = if alpha == "#" { "%23" } else { alpha };

    let url = format!(
        "{base}/api/catalogue/items.json?category={}&alpha={}&page={}",
        category, alpha_param, page
    );

    let client = http();
    let raw: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let total = raw.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut items_out = Vec::new();

    if let Some(arr) = raw.get("items").and_then(|v| v.as_array()) {
        for it in arr {
            let id = it.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let category_name = it.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let category_id = category_id_from_name(&category_name);
            let members = it
                .get("members")
                .and_then(|v| v.as_str())
                .map(|s| s == "true")
                .or_else(|| it.get("members").and_then(|v| v.as_bool()))
                .unwrap_or(false);

            let icons = GeIcons {
                small: it.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                large: it.get("icon_large").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            };

            let price = GePrice {
                trend: it
                    .get("current")
                    .and_then(|c| c.get("trend"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("neutral")
                    .to_string(),
                current: it.get("current").and_then(|c| c.get("price")).and_then(parse_price_to_i64),
                today_change: it.get("today").and_then(|c| c.get("price")).and_then(parse_price_to_i64),
            };

            let description = it.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

            items_out.push(GeItem {
                id,
                name,
                category_id,
                category_name: if category_id >= 0 {
                    cat_name_map.get(&(category_id as i32)).map(|s| s.to_string()).unwrap_or(category_name)
                } else {
                    category_name
                },
                members,
                icons,
                price,
                description,
            });
        }
    }

    Ok(GeItemList { total, items: items_out })
}

async fn fetch_alpha_counts(category: i32, game: Game) -> Result<Vec<(String, i64)>, String> {
    let (base, _cat_name_map) = base_urls(game);
    let url = format!("{base}/api/catalogue/category.json?category={category}");
    let client = http();
    let raw: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    if let Some(alpha_arr) = raw.get("alpha").and_then(|v| v.as_array()) {
        for ent in alpha_arr {
            if let (Some(letter), Some(items)) = (
                ent.get("letter").and_then(|v| v.as_str()),
                ent.get("items").and_then(|v| v.as_i64()),
            ) {
                map.insert(letter.to_string(), items);
            }
        }
    }

    let mut out = Vec::new();
    for ch in ('a'..='z').map(|c| c.to_string()) {
        out.push((ch.clone(), *map.get(&ch).unwrap_or(&0)));
    }
    out.push(("#".to_string(), *map.get("#").unwrap_or(&0)));
    Ok(out)
}

pub async fn fetch_items_all_for_ui(
    category: i32,
    page: u32,
    game: Game,
) -> Result<GeItemList, String> {
    const UI_PAGE: usize = 50;
    const UPSTREAM_PAGE_SIZE: usize = 12;

    let alpha_counts = fetch_alpha_counts(category, game).await?;
    let total_all: i64 = alpha_counts.iter().map(|(_, n)| *n).sum();

    // offset global a partir da página da UI
    let mut remaining_offset: usize = ((page.saturating_sub(1)) as usize) * UI_PAGE;
    let mut collected: Vec<GeItem> = Vec::with_capacity(UI_PAGE);

    for (letter, count_i64) in alpha_counts {
        let count = usize::try_from(count_i64.max(0)).unwrap_or(0);

        if remaining_offset >= count {
            remaining_offset -= count;
            continue;
        }

        // páginas locais dentro da letra
        let mut lpage: usize = (remaining_offset / UPSTREAM_PAGE_SIZE) + 1;
        let mut skip_first: usize = remaining_offset % UPSTREAM_PAGE_SIZE;
        let lp_end: usize = if count == 0 {
            0
        } else {
            (count + UPSTREAM_PAGE_SIZE - 1) / UPSTREAM_PAGE_SIZE
        };
        remaining_offset = 0; // offset já aplicado

        while collected.len() < UI_PAGE && lpage != 0 && lpage <= lp_end {
            let batch = fetch_items_for_ui(category, &letter, lpage as u32, game).await?;

            if batch.items.is_empty() {
                break; // acabou essa letra
            }

            let mut iter = batch.items.into_iter();

            if skip_first > 0 {
                for _ in 0..skip_first {
                    if iter.next().is_none() {
                        break;
                    }
                }
                skip_first = 0;
            }

            for it in iter {
                collected.push(it);
                if collected.len() >= UI_PAGE {
                    break;
                }
            }

            lpage += 1;
        }

        if collected.len() >= UI_PAGE {
            break;
        }
    }

    Ok(GeItemList {
        total: total_all,
        items: collected,
    })
}

// ---------------- Helpers ----------------

fn validate_alpha(alpha: &str) -> Result<(), &'static str> {
    if alpha == "#" {
        return Ok(());
    }
    if alpha.len() == 1 && alpha.chars().all(|c| c.is_ascii_lowercase()) {
        return Ok(());
    }
    Err("alpha must be a single lowercase letter a-z or '#'")
}

fn http() -> Client {
    Client::builder()
        .user_agent("rs-helper/0.1 (unofficial; contact: your-email)")
        .build()
        .unwrap()
}

fn ms_to_iso(ms: i64) -> chrono::DateTime<chrono::Utc> {
    let secs = ms / 1000;
    let nsec = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nsec).unwrap()
}

fn up_err<E: std::fmt::Display>(e: E) -> (axum::http::StatusCode, Json<ErrorBody>) {
    bad_gateway("upstream_error", &format!("Upstream request failed: {e}"))
}

fn bad_gateway(code: &str, message: &str) -> (axum::http::StatusCode, Json<ErrorBody>) {
    (axum::http::StatusCode::BAD_GATEWAY, Json(ErrorBody { code: code.into(), message: message.into() }))
}

// Mapa de categorias por jogo + constante pública RS3
use once_cell::sync::Lazy;
use std::collections::HashMap;

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

fn base_urls(game: Game) -> (&'static str, &'static HashMap<i32, &'static str>) {
    static RS3_CAT: Lazy<HashMap<i32, &'static str>> = Lazy::new(categories_map);
    static OSRS_CAT: Lazy<HashMap<i32, &'static str>> = Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert(1, "All items (OSRS)");
        m
    });

    match game {
        Game::Rs3 => ("https://secure.runescape.com/m=itemdb_rs", &RS3_CAT),
        Game::Osrs => ("https://secure.runescape.com/m=itemdb_oldschool", &OSRS_CAT),
    }
}
