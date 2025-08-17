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
#[template(path = "base.html")]
struct BaseT;

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
}

pub async fn index() -> Html<String> {
    Html(IndexT { categories: CATEGORIES }.render().unwrap())
}

#[derive(Debug, Deserialize)]
pub struct UiItemsQuery {
    pub category: i32,
    pub alpha: Option<String>,  // a-z ou "#"
    pub q: Option<String>,      // termo de busca (name contains)
    #[serde(default)]
    pub game: Game,             // rs3 | osrs (default rs3)
    pub page: Option<u32>,      // paginação upstream (1..N)
}

pub async fn items_partial(Query(q): Query<UiItemsQuery>) -> Html<String> {
    let page = q.page.unwrap_or(1);
    let alpha = q.alpha.clone().unwrap_or_else(|| "a".to_string());

    let list = match fetch_items_for_ui(q.category, &alpha, page, q.game).await {
        Ok(v) => v,
        Err(e) => {
            let _empty: Vec<UiItem> = vec![];
            let msg = format!("Error: {e}");
            let html = format!("<div class='error'>{}</div>", askama_escape(&msg));
            return Html(html);
        }
    };

    let mut items: Vec<UiItem> = list.items.into_iter()
        .filter(|it| {
            if let Some(ref term) = q.q {
                let t = term.trim().to_lowercase();
                if t.is_empty() { return true; }
                it.name.to_lowercase().contains(&t)
            } else { true }
        })
        .map(UiItem::from)
        .collect();

    if items.len() > 50 {
        items.truncate(50);
    }

    Html(ItemsT { items: &items, total: list.total }.render().unwrap())
}

/// Modelo “sanitizado” para renderização
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
    if suf.is_empty() {
        format!("{sign}{}", v.abs())
    } else {
        format!("{sign}{:.2}{}", num, suf)
    }
}

fn askama_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
