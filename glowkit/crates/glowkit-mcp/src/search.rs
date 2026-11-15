//! Ranked keyword search over the catalog.
//!
//! Each query word is matched against several fields with different weights (name and id
//! weigh most, then the type name, category, the web components a demo covers, and finally the
//! description). Whole-word hits beat prefix hits, which beat substring hits. Components that
//! match every query word get a bonus, and the curated "showcase" set breaks ties.

use crate::data::Component;

/// One scored hit.
#[derive(Debug, Clone, Copy)]
pub struct Hit<'a> {
    pub score: f32,
    pub component: &'a Component,
}

/// Lowercase alphanumeric words, with a light plural/verb-suffix stemming.
pub fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| stem(&w.to_lowercase()))
        .collect()
}

fn stem(w: &str) -> String {
    for suf in ["ies", "es", "s"] {
        if let Some(base) = w.strip_suffix(suf) {
            if base.len() >= 3 && !w.ends_with("ss") {
                return if suf == "ies" { format!("{base}y") } else { base.to_string() };
            }
        }
    }
    w.to_string()
}

/// How well `term` matches the words of `field`: 1.0 whole word, 0.6 prefix, 0.3 substring.
fn field_match(term: &str, field_words: &[String], field_lower: &str) -> f32 {
    if field_words.iter().any(|w| w == term) {
        1.0
    } else if term.len() >= 2 && field_words.iter().any(|w| w.starts_with(term)) {
        0.6
    } else if term.len() >= 3 && field_lower.contains(term) {
        0.3
    } else {
        0.0
    }
}

/// Does `category` pass the filter? Matches case-insensitively on the label or a slug of it
/// ("Charts", "charts", "cards-and-borders", "Cards & Borders", "3d").
pub fn category_matches(category: &str, filter: &str) -> bool {
    let norm = |s: &str| words(&s.replace('&', " and ")).join("-");
    let (c, f) = (norm(category), norm(filter));
    !f.is_empty() && (c == f || c.contains(&f))
}

struct Field {
    weight: f32,
    words: Vec<String>,
    lower: String,
}

impl Field {
    fn new(weight: f32, text: &str) -> Self {
        Self { weight, words: words(text), lower: text.to_lowercase() }
    }
}

/// Score one component against the query terms (0 = no match).
pub fn score(c: &Component, terms: &[String], query_lower: &str) -> f32 {
    if terms.is_empty() {
        return 0.0;
    }
    let type_name = c.type_key.as_deref().and_then(|k| k.rsplit("::").next()).unwrap_or("");
    // Split CamelCase so "ShinyText" also matches "shiny".
    let mut camel = String::new();
    for (i, ch) in type_name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            camel.push(' ');
        }
        camel.push(ch);
    }
    let fields = [
        Field::new(10.0, &c.name),
        Field::new(8.0, &c.id),
        Field::new(6.0, &format!("{type_name} {camel}")),
        Field::new(4.0, &c.category),
        Field::new(3.5, &c.covers.join(" | ")),
        Field::new(2.0, &c.description),
    ];
    let mut total = 0.0;
    let mut matched = 0;
    for term in terms {
        let best = fields.iter().map(|f| f.weight * field_match(term, &f.words, &f.lower)).fold(0.0, f32::max);
        if best > 0.0 {
            matched += 1;
            total += best;
        }
    }
    if matched == 0 {
        return 0.0;
    }
    // Exact phrase hits.
    let q = query_lower.trim();
    if c.id == q.replace(' ', "-") || c.name.to_lowercase() == q || type_name.to_lowercase() == q.replace(' ', "") {
        total += 40.0;
    } else if c.name.to_lowercase().contains(q) {
        total += 12.0;
    } else if c.covers.iter().any(|cv| cv.to_lowercase().contains(q)) {
        total += 8.0;
    }
    if matched == terms.len() {
        total += 5.0 * terms.len() as f32;
    } else {
        // Partial matches rank below every full match of the same strength.
        total *= matched as f32 / terms.len() as f32;
    }
    if c.showcase {
        total += 0.5;
    }
    total
}

/// Search the components, best first.
pub fn search<'a>(components: &'a [Component], query: &str, category: Option<&str>, limit: usize) -> Vec<Hit<'a>> {
    let terms = words(query);
    let q = query.to_lowercase();
    let mut hits: Vec<Hit<'a>> = components
        .iter()
        .filter(|c| category.is_none_or(|f| category_matches(&c.category, f)))
        .filter_map(|c| {
            let s = if terms.is_empty() { if c.showcase { 1.0 } else { 0.5 } } else { score(c, &terms, &q) };
            (s > 0.0).then_some(Hit { score: s, component: c })
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.component.id.cmp(&b.component.id)));
    hits.truncate(limit);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::catalog;

    fn comp(id: &str, name: &str, category: &str, description: &str, covers: &[&str]) -> Component {
        Component {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            description: description.into(),
            covers: covers.iter().map(|s| s.to_string()).collect(),
            code: String::new(),
            showcase: true,
            crate_name: "glowkit-x".into(),
            feature: "x".into(),
            module: "glowkit::x".into(),
            type_key: None,
            use_paths: vec![],
            template: false,
            related: vec![],
            state: vec![],
        }
    }

    fn fixture() -> Vec<Component> {
        vec![
            comp("shiny-text", "Shiny Text", "Text Animations", "A band of light sweeps across text.", &["Magic UI: Animated Shiny Text"]),
            comp("shimmer-button", "Shimmer Button", "Buttons", "A button with a shimmer and a shiny edge.", &[]),
            comp("aurora", "Aurora", "Backgrounds", "Soft northern lights behind text.", &["React Bits: Aurora"]),
            comp("line-chart", "Line Chart", "Charts", "Animated lines.", &["Recharts: Line chart"]),
            comp("bar-chart", "Bar Chart", "Charts", "Animated bars with a text label.", &[]),
        ]
    }

    #[test]
    fn stemming_and_words() {
        assert_eq!(words("Charts & Buttons"), vec!["chart", "button"]);
        assert_eq!(words("galleries"), vec!["gallery"]);
        assert_eq!(words("glass"), vec!["glass"]);
    }

    #[test]
    fn name_beats_description() {
        let f = fixture();
        let hits = search(&f, "shiny", None, 10);
        assert_eq!(hits[0].component.id, "shiny-text");
        assert_eq!(hits[1].component.id, "shimmer-button", "description match ranks lower");
    }

    #[test]
    fn exact_name_wins_and_all_terms_beat_some() {
        let f = fixture();
        let hits = search(&f, "bar chart", None, 10);
        assert_eq!(hits[0].component.id, "bar-chart");
        assert_eq!(hits[1].component.id, "line-chart");
    }

    #[test]
    fn covers_are_searchable() {
        let f = fixture();
        assert_eq!(search(&f, "recharts", None, 10)[0].component.id, "line-chart");
        assert_eq!(search(&f, "React Bits: Aurora", None, 10)[0].component.id, "aurora");
    }

    #[test]
    fn category_filter_and_limit() {
        let f = fixture();
        let hits = search(&f, "animated", Some("charts"), 10);
        assert!(hits.iter().all(|h| h.component.category == "Charts"));
        assert_eq!(hits.len(), 2);
        assert_eq!(search(&f, "", Some("Charts"), 1).len(), 1);
        assert!(category_matches("Cards & Borders", "cards-and-borders"));
        assert!(category_matches("Cards & Borders", "cards"));
        assert!(!category_matches("Charts", "cards"));
    }

    #[test]
    fn no_match_is_empty() {
        assert!(search(&fixture(), "zzzz", None, 10).is_empty());
    }

    #[test]
    fn real_catalog_ranking() {
        let c = &catalog().components;
        let top = |q: &str| search(c, q, None, 5).into_iter().map(|h| h.component.id.as_str()).collect::<Vec<_>>();
        assert_eq!(top("shiny text")[0], "shiny-text");
        assert_eq!(top("aurora")[0], "aurora");
        assert!(top("candlestick").iter().any(|id| id.contains("candle")), "{:?}", top("candlestick"));
        assert!(top("chat").len() == 5);
        let cat = search(c, "button", Some("Buttons"), 50);
        assert!(!cat.is_empty() && cat.iter().all(|h| h.component.category == "Buttons"));
    }
}
