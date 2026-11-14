use crate::overlays::kit;
use crate::overlays::LineIcon;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Hover effect of an [`UnderlineLink`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LinkStyle {
    /// An underline draws in from the left on hover and leaves to the right.
    #[default]
    Underline,
    /// A soft highlighter band sweeps in behind the text.
    Highlight,
    /// Always underlined (muted); the line turns accent-colored on hover and an arrow nudges.
    Arrow,
}

/// A text link with an animated hover treatment.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{LinkStyle, UnderlineLink};
/// if ui.add(UnderlineLink::new("Read the docs").style(LinkStyle::Arrow)).clicked() { /* … */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct UnderlineLink {
    text: String,
    style: LinkStyle,
    size: f32,
    color: Option<Color32>,
}

impl UnderlineLink {
    /// A link with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), style: LinkStyle::Underline, size: 15.0, color: None }
    }
    /// Hover effect.
    pub fn style(mut self, style: LinkStyle) -> Self {
        self.style = style;
        self
    }
    /// Font size (default 15).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Text color (default foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for UnderlineLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let g = ui.painter().layout_no_wrap(self.text.clone(), font, self.color.unwrap_or(t.foreground));
        let arrow_w = if self.style == LinkStyle::Arrow { self.size + 2.0 } else { 0.0 };
        let (rect, resp) = ui.allocate_exact_size(g.size() + vec2(arrow_w, 4.0), Sense::click());
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id.with("glowkit_link");
        let hovered = resp.hovered();
        let h = kit::follow_ctx(ui.ctx(), id.with("h"), if hovered { 1.0 } else { 0.0 }, 12.0);
        // direction memory: grow from the left, retract toward the right
        let growing: bool = glowkit_core::state(ui, id.with("dir"), || true);
        if hovered != growing && !(0.01..=0.99).contains(&h) {
            glowkit_core::set_state(ui, id.with("dir"), hovered);
        }
        let growing: bool = glowkit_core::state(ui, id.with("dir"), || true);
        let col = self.color.unwrap_or(t.foreground);
        let p = ui.painter();
        let tx = rect.min.x;
        let ty = rect.min.y + 1.0;
        let w = g.size().x;
        let base_y = ty + g.size().y + 1.0;
        let e = glowkit_core::ease(glowkit_core::Easing::InOutCubic, h);
        match self.style {
            LinkStyle::Underline => {
                let (a, b) = if growing || hovered { (tx, tx + w * e) } else { (tx + w * (1.0 - e), tx + w) };
                if e > 0.01 {
                    p.line_segment([pos2(a, base_y), pos2(b, base_y)], Stroke::new(1.5, t.accent));
                }
                p.galley(pos2(tx, ty), g, col);
            }
            LinkStyle::Highlight => {
                let band = Rect::from_min_max(
                    pos2(tx - 2.0, ty + g.size().y * 0.45),
                    pos2(tx - 2.0 + (w + 4.0) * e, base_y + 1.0),
                );
                if e > 0.01 {
                    p.rect_filled(band, 3.0, with_alpha(t.accent, 0.3));
                }
                p.galley(pos2(tx, ty), g, col);
            }
            LinkStyle::Arrow => {
                p.line_segment(
                    [pos2(tx, base_y), pos2(tx + w, base_y)],
                    Stroke::new(1.0, with_alpha(t.muted_foreground, 0.5)),
                );
                if e > 0.01 {
                    p.line_segment([pos2(tx, base_y), pos2(tx + w * e, base_y)], Stroke::new(1.0, t.accent));
                }
                p.galley(pos2(tx, ty), g.clone(), col);
                let s = self.size * 0.85;
                let ax = tx + w + 4.0 + 3.0 * e;
                // arrow rotates from ↗ toward → on hover
                let ar = Rect::from_center_size(pos2(ax + s / 2.0, ty + g.size().y / 2.0), vec2(s, s));
                let ang = -0.785 * (1.0 - e);
                let c = ar.center();
                let rot =
                    |v: egui::Vec2| c + vec2(v.x * ang.cos() - v.y * ang.sin(), v.x * ang.sin() + v.y * ang.cos());
                let k = s / 2.0 * 0.8;
                let acol = t.muted_foreground.lerp_to_gamma(t.accent, e);
                p.line_segment([rot(vec2(-k, 0.0)), rot(vec2(k, 0.0))], Stroke::new(1.4, acol));
                p.add(egui::Shape::line(
                    vec![rot(vec2(k * 0.2, -k * 0.8)), rot(vec2(k, 0.0)), rot(vec2(k * 0.2, k * 0.8))],
                    Stroke::new(1.4, acol),
                ));
            }
        }
        resp
    }
}

/// An "On this page" table of contents with a vertical rail and an accent indicator that slides
/// to the active entry (pair with your scroll position for scroll-spy behavior).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut active = 0;
/// use glowkit_components::TableOfContents;
/// if let Some(i) = TableOfContents::new(&[(0, "Introduction"), (1, "Install"), (0, "Usage")], active).show(ui) {
///     active = i; // and scroll to section i
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct TableOfContents {
    entries: Vec<(u8, String)>,
    active: usize,
    title: Option<String>,
}

impl TableOfContents {
    /// Entries as `(indent level, title)`; `active` is the highlighted entry.
    pub fn new(entries: &[(u8, &str)], active: usize) -> Self {
        Self {
            entries: entries.iter().map(|(l, s)| (*l, s.to_string())).collect(),
            active,
            title: Some("On this page".into()),
        }
    }
    /// Heading above the list (`None` for no heading).
    pub fn title(mut self, title: Option<&str>) -> Self {
        self.title = title.map(|s| s.to_string());
        self
    }
    /// Show the list. Returns a clicked entry index.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_toc");
        let row = 30.0;
        let head = if self.title.is_some() { 30.0 } else { 0.0 };
        let w = ui.available_width().min(260.0);
        let (rect, _) = ui.allocate_exact_size(vec2(w, head + row * self.entries.len() as f32), Sense::hover());
        let p = ui.painter();
        if let Some(title) = &self.title {
            p.text(rect.min + vec2(0.0, 10.0), Align2::LEFT_CENTER, title, FontId::proportional(13.0), t.foreground);
        }
        let top = rect.min.y + head;
        let rail_x = rect.min.x + 1.0;
        p.line_segment([pos2(rail_x, top), pos2(rail_x, rect.max.y)], Stroke::new(1.0, t.border));
        let a = self.active.min(self.entries.len().saturating_sub(1));
        let ay = glowkit_core::anim::spring(ui, id.with("ay"), top + a as f32 * row, 300.0, 28.0);
        p.rect_filled(Rect::from_min_size(pos2(rail_x - 1.0, ay + 4.0), vec2(2.0, row - 8.0)), 1.0, t.accent);
        let mut clicked = None;
        for (i, (lvl, title)) in self.entries.iter().enumerate() {
            let r = Rect::from_min_size(pos2(rect.min.x, top + i as f32 * row), vec2(w, row));
            let resp = ui.interact(r, id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with(("h", i)), resp.hovered());
            let on = kit::follow_ctx(ui.ctx(), id.with(("on", i)), if i == a { 1.0 } else { 0.0 }, 14.0);
            let col = t.muted_foreground.lerp_to_gamma(t.foreground, (on + hv * 0.6).min(1.0));
            let x = r.min.x + 14.0 + *lvl as f32 * 14.0 + 2.0 * on;
            p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, title, FontId::proportional(13.5), col);
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }
}
