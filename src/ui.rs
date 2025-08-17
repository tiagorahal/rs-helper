use axum::{Router, routing::get, extract::Query, response::Html};
use askama::Template;
use serde::Deserialize;

use crate::types::{GeCategory, GeItem};
use crate::ge::{fetch_items_for_ui, CATEGORIES, Game};

pub fn router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/ui/ge/items", get(items_partial))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexT<'a> {
    categories: &'a [GeCategory],
}

#[derive(Template)]
#[template(path = "partials/items.html")]
struct ItemsT<'a> {
    items: &'a [UiItem],
    total: i64,
    page: u32,
}

pub async fn index() -> Html<String> {
    Html(IndexT { categories: CATEGORIES }.render().unwrap())
}

#[derive(Debug, Deserialize)]
pub struct UiItemsQuery {
    pub q: Option<String>,
    pub page: Option<u32>,
    #[allow(dead_code)]
    pub category: Option<i32>,
    #[allow(dead_code)]
    pub alpha: Option<String>,
}

/// Busca global por nome (sem escolher categoria)
async fn search_all_categories_by_term(term: &str, page: u32) -> Result<(i64, Vec<GeItem>), String> {
    // Heurística: usa a primeira letra do termo para reduzir chamadas
    // (o endpoint de itens exige alpha). Dígitos viram "#".
    let alpha = term
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c.is_ascii_alphabetic() { c.to_string() } else { "#".to_string() })
        .unwrap_or_else(|| "a".to_string());

    let mut all: Vec<GeItem> = Vec::new();
    let needle = term.to_lowercase();

    for cat in CATEGORIES {
        match fetch_items_for_ui(cat.id, &alpha, 1, Game::Rs3).await {
            Ok(list) => {
                for it in list.items {
                    if it.name.to_lowercase().contains(&needle) {
                        all.push(it);
                    }
                }
            }
            Err(e) => {
                eprintln!("fetch_items_for_ui error on cat {}: {}", cat.id, e);
            }
        }
    }

    // Ordena por nome e pagina localmente (50 por página), sem exigir Clone
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let total = all.len() as i64;
    let per_page = 50usize;
    let start = per_page.saturating_mul(page.saturating_sub(1) as usize);
    let end = (start + per_page).min(all.len());

    let page_vec: Vec<GeItem> = all
        .into_iter()
        .enumerate()
        .filter_map(|(i, it)| if i >= start && i < end { Some(it) } else { None })
        .collect();

    Ok((total, page_vec))
}

pub async fn items_partial(Query(q): Query<UiItemsQuery>) -> Html<String> {
    let page = q.page.unwrap_or(1);

    // Requer um termo de busca
    let term = match q.q.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(t) => t.to_string(),
        None => {
            return Html(
                "<div class='muted'>Type an item name and press <strong>Search</strong>.</div>"
                    .to_string(),
            );
        }
    };

    let (total, page_items) = match search_all_categories_by_term(&term, page).await {
        Ok(res) => res,
        Err(e) => {
            let msg = format!("Error: {}", askama_escape(&e));
            return Html(format!("<div class='error'>{}</div>", msg));
        }
    };

    let mut items: Vec<UiItem> = page_items.into_iter().map(UiItem::from).collect();
    if items.len() > 50 { items.truncate(50); }

    Html(ItemsT {
        items: &items,
        total,
        page,
    }.render().unwrap())
}

/// Modelo para renderização
#[derive(Debug)]
struct UiItem {
    id: i64,
    name: String,
    category_name: String,
    members: bool,
    price_current: String,
    price_trend: String, 
    icon_small: String,
}

impl From<GeItem> for UiItem {
    fn from(g: GeItem) -> Self {
        UiItem {
            id: g.id,
            name: g.name,
            category_name: g.category_name,
            members: g.members,
            price_current: g.price.current.map(format_compact).unwrap_or_else(|| "-".to_string()),
            price_trend: g.price.trend,
            icon_small: g.icons.small,
        }
    }
}

// -------- utils ----------
fn format_compact(v: i64) -> String {
    let abs = (v as f64).abs();
    let (num, suf) = if abs >= 1_000_000_000.0 {
        (abs / 1_000_000_000.0, "b")
    } else if abs >= 1_000_000.0 {
        (abs / 1_000_000.0, "m")
    } else if abs >= 1_000.0 {
        (abs / 1_000.0, "k")
    } else {
        (abs, "")
    };
    let sign = if v < 0 { "-" } else { "" };
    if suf.is_empty() { format!("{sign}{}", v.abs()) } else { format!("{sign}{:.2}{}", num, suf) }
}

fn askama_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
