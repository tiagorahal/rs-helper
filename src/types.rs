use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

// GE Info
#[derive(Debug, Serialize, Deserialize)]
pub struct GeInfo {
    pub runedate: i64,
    pub last_updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

// Categories
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct GeCategory {
    pub id: i32,
    pub name: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeCategories {
    pub categories: Vec<GeCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

// Item List
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeItemList {
    pub total: i64,
    pub items: Vec<GeItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

// Item
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeItem {
    pub id: i64,
    pub name: String,
    pub category_id: i32,
    pub category_name: String,
    pub members: bool,
    pub icons: GeIcons,
    pub price: GePrice,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeIcons {
    pub small: String,
    pub large: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GePrice {
    /// "neutral" | "positive" | "negative"
    pub trend: String,
    /// Current price in units
    pub current: Option<i64>,
    /// Today's change in units (can be negative)
    pub today_change: Option<i64>,
    /// Percentage change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_percentage: Option<f64>,
}

// Graph data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeGraph {
    pub id: i64,
    pub daily: Vec<TsPrice>,
    pub average: Vec<TsPrice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TsPrice {
    pub ts: DateTime<Utc>,
    pub price: i64,
}

// Query parameters
#[derive(Debug, Deserialize)]
pub struct ItemsQuery {
    pub category: i32,
    pub alpha: Option<String>,
    pub page: Option<u32>,
    #[serde(default)]
    pub game: Game,
}

#[derive(Debug, Deserialize)]
pub struct ItemDetailQuery {
    #[serde(default)]
    pub game: Game,
}

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    #[serde(default)]
    pub game: Game,
}

#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    #[serde(default)]
    pub game: Game,
}

// Game enum
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Game::Rs3 => write!(f, "rs3"),
            Game::Osrs => write!(f, "osrs"),
        }
    }
}

// Helper functions
pub fn parse_price_to_i64(s: &serde_json::Value) -> Option<i64> {
    match s {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(txt) => parse_abbrev(txt),
        _ => None,
    }
}

fn parse_abbrev(txt: &str) -> Option<i64> {
    let t = txt.trim().to_lowercase();
    if t.is_empty() { 
        return None; 
    }

    // Handle sign
    let (sign, body) = if let Some(stripped) = t.strip_prefix('-') {
        (-1_i64, stripped)
    } else if let Some(stripped) = t.strip_prefix('+') {
        (1_i64, stripped)
    } else {
        (1_i64, t.as_str())
    };

    // Handle suffixes (k/m/b)
    let (num_part, multiplier) = if body.ends_with('k') {
        (&body[..body.len()-1], 1_000_f64)
    } else if body.ends_with('m') {
        (&body[..body.len()-1], 1_000_000_f64)
    } else if body.ends_with('b') {
        (&body[..body.len()-1], 1_000_000_000_f64)
    } else {
        (body, 1.0_f64)
    };

    // Parse the number
    let parsed = num_part.replace(',', "");
    let val = parsed.parse::<f64>().ok()?;
    Some((val * multiplier).round() as i64 * sign)
}

// Format helpers
pub fn format_price_compact(v: i64) -> String {
    let abs = (v as f64).abs();
    let (num, suffix) = if abs >= 1_000_000_000.0 {
        (abs / 1_000_000_000.0, "b")
    } else if abs >= 1_000_000.0 {
        (abs / 1_000_000.0, "m")
    } else if abs >= 1_000.0 {
        (abs / 1_000.0, "k")
    } else {
        (abs, "")
    };
    
    let sign = if v < 0 { "-" } else { "" };
    if suffix.is_empty() {
        format!("{sign}{}", v.abs())
    } else if num >= 100.0 {
        format!("{sign}{:.0}{}", num, suffix)
    } else if num >= 10.0 {
        format!("{sign}{:.1}{}", num, suffix)
    } else {
        format!("{sign}{:.2}{}", num, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_price() {
        assert_eq!(parse_abbrev("1.5m"), Some(1_500_000));
        assert_eq!(parse_abbrev("-2.3k"), Some(-2_300));
        assert_eq!(parse_abbrev("100"), Some(100));
        assert_eq!(parse_abbrev("5.5b"), Some(5_500_000_000));
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price_compact(1_500_000), "1.5m");
        assert_eq!(format_price_compact(-2_300), "-2.3k");
        assert_eq!(format_price_compact(100), "100");
        assert_eq!(format_price_compact(5_500_000_000), "5.5b");
    }
}
