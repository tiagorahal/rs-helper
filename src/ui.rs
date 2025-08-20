use axum::{
    Router, 
    routing::get, 
    extract::Query, 
    response::Html,
    http::StatusCode,
};
use askama::Template;
use serde::Deserialize;

use crate::{
    types::{GeCategory, GeItem, Game},
    ge::{fetch_items_for_ui, CATEGORIES},
    metrics::RequestTimer,
};

pub fn router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/ui/ge/items", get(items_partial))
        .route("/ui/ge/item/:id", get(item_modal))
        .route("/about", get(about_page))
}

// Templates
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
    page: u32,
    has_more: bool,
}

#[derive(Template)]
#[template(path = "partials/item_modal.html")]
struct ItemModalT<'a> {
    item: &'a UiItemDetail,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutT {
    version: &'static str,
    rust_version: &'static str,
}

// Main page handler
pub async fn index() -> Html<String> {
    let timer = RequestTimer::new("/");
    let html = IndexT { categories: CATEGORIES }.render().unwrap();
    timer.record(200);
    Html(html)
}

// About page
pub async fn about_page() -> Html<String> {
    let timer = RequestTimer::new("/about");
    let html = AboutT {
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
    }.render().unwrap_or_else(|_| {
        AboutT {
            version: "0.2.0",
            rust_version: "1.82",
        }.render().unwrap()
    });
    timer.record(200);
    Html(html)
}

// Query parameters for items
#[derive(Debug, Deserialize)]
pub struct UiItemsQuery {
    pub category: i32,
    pub alpha: Option<String>,
    pub q: Option<String>,      // search term
    #[serde(default)]
    pub game: Game,
    pub page: Option<u32>,
}

// Items partial handler with search
pub async fn items_partial(Query(q): Query<UiItemsQuery>) -> Result<Html<String>, (StatusCode, Html<String>)> {
    let timer = RequestTimer::new("/ui/ge/items");
    let page = q.page.unwrap_or(1);
    let alpha = q.alpha.clone().unwrap_or_else(|| "a".to_string());
    
    // Fetch items
    let list = match fetch_items_for_ui(q.category, &alpha, page, q.game).await {
        Ok(v) => v,
        Err(e) => {
            timer.record(500);
            let error_html = format!(
                r#"<div class="error">
                    <svg style="width: 24px; height: 24px;" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" 
                              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <span>{}</span>
                </div>"#,
                html_escape(&e)
            );
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Html(error_html)));
        }
    };
    
    // Filter by search term if provided
    let mut items: Vec<UiItem> = list.items.into_iter()
        .filter(|item| {
            if let Some(ref search_term) = q.q {
                let term = search_term.trim().to_lowercase();
                if term.is_empty() { 
                    return true; 
                }
                item.name.to_lowercase().contains(&term) ||
                item.description.as_ref()
                    .map(|d| d.to_lowercase().contains(&term))
                    .unwrap_or(false)
            } else { 
                true 
            }
        })
        .map(UiItem::from)
        .collect();
    
    // Pagination info
    let total_items = items.len();
    let items_per_page = 50;
    let has_more = total_items > items_per_page || page > 1;
    
    // Truncate to page size
    if items.len() > items_per_page {
        items.truncate(items_per_page);
    }
    
    let html = ItemsT { 
        items: &items, 
        total: list.total,
        page,
        has_more,
    }.render().unwrap_or_else(|e| {
        format!("<div class='error'>Template error: {}</div>", e)
    });
    
    timer.record(200);
    Ok(Html(html))
}

// Item modal/detail view
pub async fn item_modal(axum::extract::Path(id): axum::extract::Path<i64>) -> Html<String> {
    let timer = RequestTimer::new("/ui/ge/item/:id");
    
    // For now, return a placeholder
    // In production, fetch full item details including graph data
    let item = UiItemDetail {
        id,
        name: "Item Details".to_string(),
        description: "Loading...".to_string(),
        price_current: "-".to_string(),
        price_change: "-".to_string(),
        members: false,
        icon_large: "".to_string(),
    };
    
    let html = ItemModalT { item: &item }.render().unwrap_or_else(|_| {
        "<div>Error loading item</div>".to_string()
    });
    
    timer.record(200);
    Html(html)
}

// UI Models
#[derive(Debug)]
struct UiItem {
    id: i64,
    name: String,
    category_name: String,
    members: bool,
    price_current: String,
    price_trend: String,
    price_change_percent: String,
    icon_small: String,
}

#[derive(Debug)]
struct UiItemDetail {
    id: i64,
    name: String,
    description: String,
    price_current: String,
    price_change: String,
    members: bool,
    icon_large: String,
}

impl From<GeItem> for UiItem {
    fn from(item: GeItem) -> Self {
        use crate::types::format_price_compact;
        
        let price_change_percent = item.price.change_percentage
            .map(|p| {
                if p > 0.0 {
                    format!("+{:.1}%", p)
                } else {
                    format!("{:.1}%", p)
                }
            })
            .unwrap_or_else(|| "—".to_string());
        
        UiItem {
            id: item.id,
            name: item.name,
            category_name: item.category_name,
            members: item.members,
            price_current: item.price.current
                .map(format_price_compact)
                .unwrap_or_else(|| "—".to_string()),
            price_trend: item.price.trend,
            price_change_percent,
            icon_small: item.icons.small,
        }
    }
}

// Utility functions
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
