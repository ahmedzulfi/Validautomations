//! Render streamed JSX-like markup into real widgets.

use crate::common;
use crate::media::{self, PillStyle};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A node of parsed markup.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkupNode {
    /// An element such as `<Button variant="outline">Save</Button>`.
    Element {
        /// Tag name as written (`Card`, `Button`, ...).
        tag: String,
        /// Attributes in source order. Bare attributes get the value `"true"`.
        attrs: Vec<(String, String)>,
        /// Child nodes.
        children: Vec<MarkupNode>,
        /// `false` when the input ended before the closing tag (still streaming).
        closed: bool,
    },
    /// Text content (whitespace collapsed, entities decoded).
    Text(String),
}

impl MarkupNode {
    /// Attribute value by name.
    pub fn attr(&self, name: &str) -> Option<&str> {
        match self {
            MarkupNode::Element { attrs, .. } => attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()),
            MarkupNode::Text(_) => None,
        }
    }
    /// All text inside this node, concatenated.
    pub fn text(&self) -> String {
        match self {
            MarkupNode::Text(s) => s.clone(),
            MarkupNode::Element { children, .. } => children.iter().map(|c| c.text()).collect::<Vec<_>>().join(" "),
        }
    }
}

type Attrs = Vec<(String, String)>;
/// An element still open while parsing: tag, attributes, children so far.
type Open = (String, Attrs, Vec<MarkupNode>);

fn decode(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            ws = true;
        } else {
            if ws && !out.is_empty() {
                out.push(' ');
            }
            ws = false;
            out.push(c);
        }
    }
    if ws && !out.is_empty() {
        out.push(' ');
    }
    out
}

/// Parse a tag body (between `<` and `>`, without them). Returns `(name, attrs, self_closing)`.
fn parse_tag(body: &str) -> Option<(String, Attrs, bool)> {
    let body = body.trim();
    let (body, self_closing) = match body.strip_suffix('/') {
        Some(b) => (b.trim_end(), true),
        None => (body, false),
    };
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    let name: String = chars.iter().take_while(|c| c.is_alphanumeric() || **c == '.' || **c == '-' || **c == '_').collect();
    if name.is_empty() {
        return None;
    }
    i += name.chars().count();
    let mut attrs = Vec::new();
    loop {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' {
            i += 1;
        }
        let key: String = chars[start..i].iter().collect();
        if key.is_empty() {
            i += 1;
            continue;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            let value = if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let q = chars[i];
                i += 1;
                let s = i;
                while i < chars.len() && chars[i] != q {
                    i += 1;
                }
                let v: String = chars[s..i.min(chars.len())].iter().collect();
                i += 1;
                v
            } else if i < chars.len() && chars[i] == '{' {
                let mut depth = 0;
                let s = i + 1;
                while i < chars.len() {
                    match chars[i] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let v: String = chars[s..i.min(chars.len())].iter().collect();
                i += 1;
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'').to_owned();
                v
            } else {
                let s = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                chars[s..i].iter().collect()
            };
            attrs.push((key, decode(&value)));
        } else {
            attrs.push((key, "true".to_owned()));
        }
    }
    Some((name, attrs, self_closing))
}

/// Find the `>` that ends a tag starting at byte `from`, skipping quoted / braced attribute values.
fn tag_end(src: &str, from: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut depth = 0i32;
    for (k, c) in src[from..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => depth -= 1,
            (None, '>') if depth <= 0 => return Some(from + k),
            _ => {}
        }
    }
    None
}

/// Parse JSX-like markup into a tree. Tolerant of partial (streaming) input: an unfinished tag
/// at the end is dropped, elements that are still open are returned with `closed: false`,
/// stray closing tags are ignored and mismatched ones close the nearest matching element.
///
/// ```
/// use glowkit_ai::{parse_markup, MarkupNode};
/// let nodes = parse_markup("<Card><Title>Hi</Title><Button variant=\"outline\">Go");
/// assert_eq!(nodes.len(), 1);
/// ```
pub fn parse_markup(src: &str) -> Vec<MarkupNode> {
    // stack of (tag, attrs, children)
    let mut stack: Vec<Open> = vec![(String::new(), Vec::new(), Vec::new())];
    let mut i = 0;
    let push_text = |stack: &mut Vec<Open>, s: &str| {
        let t = collapse_ws(&decode(s));
        if !t.trim().is_empty() {
            if let Some(top) = stack.last_mut() {
                top.2.push(MarkupNode::Text(t));
            }
        }
    };
    while i < src.len() {
        let rest = &src[i..];
        let Some(lt) = rest.find('<') else {
            push_text(&mut stack, rest);
            break;
        };
        if lt > 0 {
            push_text(&mut stack, &rest[..lt]);
        }
        let start = i + lt;
        // comments
        if src[start..].starts_with("<!--") {
            match src[start..].find("-->") {
                Some(e) => {
                    i = start + e + 3;
                    continue;
                }
                None => break,
            }
        }
        let Some(end) = tag_end(src, start + 1) else {
            break; // partial tag at the end of a stream
        };
        let body = &src[start + 1..end];
        i = end + 1;
        if let Some(name) = body.trim().strip_prefix('/') {
            let name = name.trim();
            if name.is_empty() {
                // fragment close `</>`
                if stack.len() > 1 && stack.last().is_some_and(|s| s.0.is_empty()) {
                    close_top(&mut stack, true);
                }
                continue;
            }
            if let Some(pos) = stack.iter().rposition(|(t, _, _)| t == name) {
                if pos > 0 {
                    while stack.len() > pos + 1 {
                        close_top(&mut stack, false);
                    }
                    close_top(&mut stack, true);
                }
            }
            continue;
        }
        if body.trim().is_empty() {
            // fragment open `<>`: treat as a transparent group
            stack.push((String::new(), Vec::new(), Vec::new()));
            continue;
        }
        if let Some((name, attrs, self_closing)) = parse_tag(body) {
            if self_closing {
                if let Some(top) = stack.last_mut() {
                    top.2.push(MarkupNode::Element { tag: name, attrs, children: Vec::new(), closed: true });
                }
            } else {
                stack.push((name, attrs, Vec::new()));
            }
        }
    }
    while stack.len() > 1 {
        close_top(&mut stack, false);
    }
    stack.pop().map(|s| s.2).unwrap_or_default()
}

fn close_top(stack: &mut Vec<Open>, closed: bool) {
    if let Some((tag, attrs, children)) = stack.pop() {
        let Some(parent) = stack.last_mut() else { return };
        if tag.is_empty() {
            parent.2.extend(children); // fragment
        } else {
            parent.2.push(MarkupNode::Element { tag, attrs, children, closed });
        }
    }
}

/// Renders a small JSX-like markup language into real, theme-aware widgets, the way generative-UI
/// chat answers stream a component tree. Feed it the markup received so far: it re-parses every
/// frame, tolerates half-written tags, and fades new elements in as they arrive.
///
/// Supported tags: `Card`, `Title` / `H1`..`H3`, `Text` / `P` / `Description`, `Button`
/// (`variant="outline|secondary|ghost|destructive"`), `Badge` (`variant`), `Row` / `HStack`,
/// `Stack` / `VStack` / `Col`, `Input` (`placeholder`), `Separator`, `Progress` (`value` 0-100),
/// `Avatar` (`name`), `Switch` (`checked`), `Image` (`ratio`), `Stat` (`label`, `value`, `delta`),
/// `List` / `Item`. Unknown tags render their children inside a dashed outline.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::JsxPreview;
/// ui.add(JsxPreview::new(r#"<Card><Title>Upgrade</Title><Text>Get more tokens.</Text><Button>Upgrade</Button></Card>"#));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct JsxPreview<'a> {
    source: &'a str,
    streaming_caret: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> JsxPreview<'a> {
    /// Render `source` (complete or partial).
    pub fn new(source: &'a str) -> Self {
        Self { source, streaming_caret: true, id_salt: None }
    }
    /// Show a pulsing caret after the last still-open element (default on).
    pub fn streaming_caret(mut self, on: bool) -> Self {
        self.streaming_caret = on;
        self
    }
    /// Stable id for the appear animations.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for JsxPreview<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let nodes = parse_markup(self.source);
        let id = common::widget_id(ui, self.id_salt).with("jsx_preview");
        let open_tail = nodes.last().is_some_and(|n| matches!(n, MarkupNode::Element { closed: false, .. }));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
            let mut r = Renderer { caret: self.streaming_caret && open_tail };
            for (i, n) in nodes.iter().enumerate() {
                r.node(ui, n, id.with(i), Ctx::default());
            }
        })
        .response
    }
}

#[derive(Clone, Copy, Default)]
struct Ctx {
    horizontal: bool,
}

struct Renderer {
    caret: bool,
}

fn variant_of(n: &MarkupNode) -> &str {
    n.attr("variant").unwrap_or("default")
}

impl Renderer {
    /// Appear progress (0..1) of an element first seen now.
    fn appear(&self, ui: &Ui, id: Id) -> f32 {
        let e = glowkit_core::Clock::elapsed(ui, id.with("appear"));
        if e < 0.35 {
            ui.ctx().request_repaint();
        }
        glowkit_core::ease(glowkit_core::Easing::OutCubic, (e / 0.35).clamp(0.0, 1.0))
    }

    fn children(&mut self, ui: &mut Ui, children: &[MarkupNode], id: Id, ctx: Ctx) {
        for (i, c) in children.iter().enumerate() {
            self.node(ui, c, id.with(i), ctx);
        }
    }

    fn node(&mut self, ui: &mut Ui, n: &MarkupNode, id: Id, ctx: Ctx) {
        let t = Theme::of(ui);
        let MarkupNode::Element { tag, children, closed, .. } = n else {
            if let MarkupNode::Text(s) = n {
                let s = s.trim();
                if !s.is_empty() {
                    ui.add(egui::Label::new(egui::RichText::new(s).size(13.5).color(t.foreground)).wrap());
                }
            }
            return;
        };
        let k = self.appear(ui, id);
        ui.scope(|ui| {
            ui.multiply_opacity(k);
            let lower = tag.to_ascii_lowercase();
            match lower.as_str() {
                "card" => {
                    let w = if ctx.horizontal { 220.0 } else { ui.available_width() };
                    egui::Frame::new()
                        .fill(t.card)
                        .stroke(Stroke::new(1.0, t.border))
                        .corner_radius(t.corner())
                        .shadow(egui::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(if t.dark { 40 } else { 10 }) })
                        .inner_margin(egui::Margin::same(18))
                        .show(ui, |ui| {
                            ui.set_width(w - 38.0);
                            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
                            if let Some(title) = n.attr("title") {
                                ui.label(egui::RichText::new(title).size(16.0).strong().color(t.foreground));
                            }
                            self.children(ui, children, id, Ctx { horizontal: false });
                            self.maybe_caret(ui, *closed);
                        });
                }
                "title" | "h1" | "h2" | "h3" | "heading" => {
                    let size = match lower.as_str() {
                        "h1" => 22.0,
                        "h3" => 15.0,
                        _ => 17.5,
                    };
                    ui.add(egui::Label::new(egui::RichText::new(n.text().trim()).size(size).strong().color(t.foreground)).wrap());
                }
                "text" | "p" | "description" | "span" => {
                    let muted = lower == "description" || n.attr("muted").is_some();
                    let txt = n.text();
                    ui.add(egui::Label::new(egui::RichText::new(txt.trim()).size(13.5).color(if muted { t.muted_foreground } else { t.foreground })).wrap());
                }
                "button" => {
                    let style = match variant_of(n) {
                        "outline" => PillStyle::Outline,
                        "ghost" => PillStyle::Ghost,
                        "secondary" => PillStyle::Solid(t.secondary),
                        "destructive" => PillStyle::Solid(t.destructive),
                        _ => PillStyle::Primary,
                    };
                    let label = n.text();
                    media::pill_button(ui, label.trim(), None, style, 34.0);
                }
                "badge" => {
                    let c = match variant_of(n) {
                        "success" => t.success,
                        "warning" => t.warning,
                        "destructive" => t.destructive,
                        "info" => t.info,
                        "secondary" | "outline" => t.muted_foreground,
                        _ => t.accent,
                    };
                    let label = n.text();
                    let g = common::galley(ui, label.trim(), 11.5, c);
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 14.0, 22.0), Sense::hover());
                    common::badge(ui.painter(), pos2(r.min.x, r.center().y), label.trim(), c, &t);
                }
                "row" | "hstack" | "flex" | "group" => {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        self.children(ui, children, id, Ctx { horizontal: true });
                    });
                }
                "stack" | "vstack" | "col" | "column" | "div" | "list" => {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = if lower == "list" { 2.0 } else { 8.0 };
                        self.children(ui, children, id, Ctx { horizontal: false });
                    });
                }
                "item" | "li" => {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::hover());
                        crate::icons::paint_icon(ui.painter(), crate::Icon::Check, Rect::from_center_size(r.center(), vec2(12.0, 12.0)), t.success);
                        self.children(ui, children, id, Ctx { horizontal: true });
                    });
                }
                "input" | "textfield" => {
                    let w = if ctx.horizontal { 200.0 } else { ui.available_width() };
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
                    let focus = common::anim_bool(ui, resp.id.with("f"), resp.hovered(), 0.12);
                    let p = ui.painter();
                    p.rect(r, t.corner_scaled(0.8), if t.dark { with_alpha(t.input, 0.3) } else { t.background }, Stroke::new(1.0, t.input.lerp_to_gamma(t.ring, focus)), StrokeKind::Inside);
                    let ph = n.attr("placeholder").or(n.attr("value")).unwrap_or("");
                    let c = if n.attr("value").is_some() { t.foreground } else { t.muted_foreground };
                    common::text(p, pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, ph, 13.5, c);
                }
                "separator" | "hr" | "divider" => {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 3.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, t.border));
                }
                "progress" => {
                    let v = n.attr("value").and_then(|v| v.trim_end_matches('%').parse::<f32>().ok()).unwrap_or(50.0) / 100.0;
                    let w = if ctx.horizontal { 160.0 } else { ui.available_width() };
                    let (r, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
                    let shown = v.clamp(0.0, 1.0) * k;
                    ui.painter().rect_filled(r, CornerRadius::same(4), with_alpha(t.muted_foreground, 0.18));
                    if shown > 0.0 {
                        glowkit_core::paint::gradient_rect(ui.painter(), Rect::from_min_size(r.min, vec2(r.width() * shown, r.height())), 4.0, 0.0, &[t.gradient[0], t.gradient[1]]);
                    }
                }
                "avatar" => {
                    let name = n.attr("name").map(str::to_owned).unwrap_or_else(|| n.text());
                    let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                    let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
                    let hue = name.bytes().fold(5u32, |a, b| a.wrapping_mul(33).wrapping_add(b as u32)) % 360;
                    let c = glowkit_core::hsv(hue as f32, 0.5, if t.dark { 0.85 } else { 0.6 });
                    ui.painter().circle_filled(r.center(), 18.0, with_alpha(c, 0.2));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, initials, FontId::proportional(13.0), c);
                }
                "switch" | "toggle" => {
                    let on = n.attr("checked").is_some_and(|v| v != "false");
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(36.0, 20.0), Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(r, CornerRadius::same(10), if on { t.primary } else { with_alpha(t.muted_foreground, 0.3) });
                        p.circle_filled(pos2(if on { r.max.x - 10.0 } else { r.min.x + 10.0 }, r.center().y), 8.0, if on { t.primary_foreground } else { Color32::WHITE });
                        let label = n.text();
                        if !label.trim().is_empty() {
                            ui.label(egui::RichText::new(label.trim()).size(13.5).color(t.foreground));
                        }
                    });
                }
                "image" | "img" => {
                    let ratio = n.attr("ratio").and_then(|r| {
                        let mut it = r.split(['/', ':']);
                        let a: f32 = it.next()?.trim().parse().ok()?;
                        let b: f32 = it.next().map(|b| b.trim().parse().unwrap_or(1.0)).unwrap_or(1.0);
                        Some(a / b.max(0.01))
                    });
                    let w = if ctx.horizontal { 160.0 } else { ui.available_width() };
                    let h = w / ratio.unwrap_or(16.0 / 9.0);
                    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                    glowkit_core::paint::gradient_rect(ui.painter(), r, t.radius * 0.8, 135.0, &[with_alpha(t.gradient[0], 0.8), with_alpha(t.gradient[1], 0.7), with_alpha(t.gradient[2], 0.6)]);
                    if let Some(alt) = n.attr("alt") {
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, alt, FontId::proportional(13.0), Color32::WHITE);
                    }
                }
                "stat" => {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(egui::RichText::new(n.attr("label").unwrap_or("")).size(12.0).color(t.muted_foreground));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(n.attr("value").unwrap_or("–")).size(22.0).strong().color(t.foreground));
                            if let Some(d) = n.attr("delta") {
                                let c = if d.starts_with('-') { t.destructive } else { t.success };
                                ui.label(egui::RichText::new(d).size(12.0).color(c));
                            }
                        });
                    });
                }
                _ => {
                    // unknown component: dashed outline with its name
                    let inner = egui::Frame::new().inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(egui::RichText::new(format!("<{tag}>")).monospace().size(11.0).color(t.muted_foreground));
                        self.children(ui, children, id, ctx);
                    });
                    let r = inner.response.rect;
                    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
                    for w in pts.windows(2) {
                        ui.painter().extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1.0, t.border), 4.0, 3.0));
                    }
                }
            }
        });
    }

    fn maybe_caret(&mut self, ui: &mut Ui, closed: bool) {
        if !closed && self.caret {
            self.caret = false;
            let t = Theme::of(ui);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 14.0), Sense::hover());
            let time = glowkit_core::time(ui);
            for i in 0..3 {
                let a = ((time * 4.0 - i as f32 * 0.6).sin() * 0.5 + 0.5) * 0.8 + 0.2;
                ui.painter().circle_filled(pos2(r.min.x + 4.0 + i as f32 * 10.0, r.center().y), 3.0, with_alpha(t.muted_foreground, a));
            }
            glowkit_core::animate(ui);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(n: &MarkupNode) -> (&str, &[MarkupNode], bool) {
        match n {
            MarkupNode::Element { tag, children, closed, .. } => (tag.as_str(), children.as_slice(), *closed),
            MarkupNode::Text(_) => panic!("expected element"),
        }
    }

    #[test]
    fn complete_tree() {
        let n = parse_markup("<Card>\n  <Title>Hello &amp; welcome</Title>\n  <Button variant=\"outline\">Go</Button>\n</Card>");
        assert_eq!(n.len(), 1);
        let (tag, kids, closed) = el(&n[0]);
        assert_eq!(tag, "Card");
        assert!(closed);
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].text(), "Hello & welcome");
        assert_eq!(kids[1].attr("variant"), Some("outline"));
    }

    #[test]
    fn partial_input_is_tolerated() {
        // unfinished tag at the end is dropped, open elements are returned unclosed
        let n = parse_markup("<Card><Title>Hi</Title><Butt");
        let (_, kids, closed) = el(&n[0]);
        assert!(!closed);
        assert_eq!(kids.len(), 1);
        // half an attribute
        let n = parse_markup("<Card><Button variant=\"outl");
        let (_, kids, _) = el(&n[0]);
        assert!(kids.is_empty());
        // text still streaming
        let n = parse_markup("<Text>Hello wor");
        assert_eq!(n[0].text(), "Hello wor");
        assert!(!el(&n[0]).2);
        assert!(parse_markup("").is_empty());
        assert!(parse_markup("<").is_empty());
    }

    #[test]
    fn self_closing_attrs_and_mismatch() {
        let n = parse_markup("<Row><Input placeholder='Email' disabled /><Progress value={42}/></Row></Stray>");
        assert_eq!(n.len(), 1);
        let (_, kids, closed) = el(&n[0]);
        assert!(closed);
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].attr("placeholder"), Some("Email"));
        assert_eq!(kids[0].attr("disabled"), Some("true"));
        assert_eq!(kids[1].attr("value"), Some("42"));
        // closing an outer tag closes the inner ones
        let n = parse_markup("<Card><Text>a</Card><Badge>b</Badge>");
        assert_eq!(n.len(), 2);
        let (_, kids, closed) = el(&n[0]);
        assert!(closed);
        assert!(!el(&kids[0]).2);
    }

    #[test]
    fn fragments_and_comments() {
        let n = parse_markup("<><!-- hi --><Badge>x</Badge><Badge>y</Badge></>");
        assert_eq!(n.len(), 2);
        let n = parse_markup("<Text>a > b</Text>");
        assert_eq!(n[0].text(), "a > b");
    }
}
