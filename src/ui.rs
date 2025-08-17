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
    alpha: String,
    letters: &'a [Letter],
}

pub async fn index() -> Html<String> {
    Html(IndexT { categories: CATEGORIES }.render().unwrap())
}

#[derive(Debug, Deserialize)]
pub struct UiItemsQuery {
    pub category: i32,
    pub alpha: Option<String>,
    pub q: Option<String>,
    #[serde(default)]
    pub game: Game,   // rs3 | osrs
    pub page: Option<u32>,
}

pub async fn items_partial(Query(q): Query<UiItemsQuery>) -> Html<String> {
    let page = q.page.unwrap_or(1);
    let alpha = q.alpha.clone().unwrap_or_else(|| "a".to_string());

    let list = match fetch_items_for_ui(q.category, &alpha, page, q.game).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("Error: {}", askama_escape(&e));
            let html = format!("<div class='error'>{}</div>", msg);
            return Html(html);
        }
    };

    let items: Vec<UiItem> = list.items.into_iter().map(UiItem::from).collect();

    // barra de letras pronta para o template (sem comparar tipos lá)
    const LETTERS: [&str; 27] = [
        "a","b","c","d","e","f","g","h","i","j","k","l","m","n",
        "o","p","q","r","s","t","u","v","w","x","y","z","#"
    ];
    let letters_vec: Vec<Letter> = LETTERS
        .iter()
        .map(|&ch| Letter { ch, active: alpha == ch })
        .collect();

    Html(ItemsT {
        items: &items,
        total: list.total,
        page,
        alpha,
        letters: &letters_vec,
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
    price_trend: String, // "positive" | "negative" | "neutral"
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

#[derive(Debug)]
struct Letter {
    ch: &'static str,
    active: bool,
}

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
