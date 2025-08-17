use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GeInfo {
    pub runedate: i64,
    pub last_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub struct GeCategory {
    pub id: i32,
    pub name: &'static str,
}

#[derive(Debug, Serialize)]
pub struct GeCategories {
    pub categories: Vec<GeCategory>,
}

#[derive(Debug, Serialize)]
pub struct GeItemList {
    pub total: i64,
    pub items: Vec<GeItem>,
}

#[derive(Debug, Serialize)]
pub struct GeItem {
    pub id: i64,
    pub name: String,
    pub category_id: i32,
    pub category_name: String,
    pub members: bool,
    pub icons: GeIcons,
    pub price: GePrice,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GeIcons {
    pub small: String,
    pub large: String,
}

#[derive(Debug, Serialize)]
pub struct GePrice {
    pub trend: String,
    pub current: Option<i64>,
    pub today_change: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct GeGraph {
    pub id: i64,
    pub daily: Vec<TsPrice>,
    pub average: Vec<TsPrice>,
}

#[derive(Debug, Serialize)]
pub struct TsPrice {
    pub ts: DateTime<Utc>,
    pub price: i64,
}

pub fn parse_price_to_i64(s: &serde_json::Value) -> Option<i64> {
    match s {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(txt) => parse_abbrev(txt),
        _ => None,
    }
}

fn parse_abbrev(txt: &str) -> Option<i64> {
    let t = txt.trim().to_lowercase();
    if t.is_empty() { return None; }

    // sinal
    let (sign, body) = if let Some(stripped) = t.strip_prefix('-') {
        (-1_i64, stripped)
    } else if let Some(stripped) = t.strip_prefix('+') {
        (1_i64, stripped)
    } else {
        (1_i64, t.as_str())
    };

    // sufixos k/m/b
    let (num_part, mul) = if body.ends_with('k') {
        (&body[..body.len()-1], 1_000_f64)
    } else if body.ends_with('m') {
        (&body[..body.len()-1], 1_000_000_f64)
    } else if body.ends_with('b') {
        (&body[..body.len()-1], 1_000_000_000_f64)
    } else {
        (body, 1.0_f64)
    };

    let parsed = num_part.replace(',', "");
    let val = parsed.parse::<f64>().ok()?;
    Some((val * mul).round() as i64 * sign)
}
