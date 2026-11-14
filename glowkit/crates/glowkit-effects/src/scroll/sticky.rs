use super::common::{drive_scroll, scroll_rail};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder,
};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// One section of a [`StickyScrollReveal`].
#[derive(Clone, Debug, PartialEq)]
pub struct StickySection {
    /// Section heading.
    pub title: String,
    /// Paragraph under the heading.
    pub body: String,
    /// Glyph shown on the default sticky panel.
    pub icon: String,
    /// Gradient of the default panel; `None` uses the theme chart palette.
    pub colors: Option<[Color32; 2]>,
}

impl StickySection {
    /// A section with a title and body text.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into(), icon: String::new(), colors: None }
    }
    /// Glyph for the default panel.
    pub fn icon(mut self, s: impl Into<String>) -> Self {
        self.icon = s.into();
        self
    }
    /// Gradient for the default panel.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
}

/// Two columns: a list of text sections scrolls on the left while a sticky panel on the right
/// stays put and swaps its content (and the whole backdrop shifts color) as each section
/// becomes active. Inactive sections are dimmed.
///
/// Owns its viewport (wheel / drag), glides through the sections when idle. Use it as a widget
/// for the default gradient panel, or [`Self::show`] to draw your own panel per section.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{StickyScrollReveal, StickySection};
/// let sections = [
///     StickySection::new("Collaborative editing", "Work together in real time…").icon("✍"),
///     StickySection::new("Version control", "Never lose a change…").icon("⑂"),
/// ];
/// ui.add(StickyScrollReveal::new(&sections));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.show(ui, |ui, index, rect| ...)`"]
pub struct StickyScrollReveal<'a> {
    sections: &'a [StickySection],
    height: f32,
    panel: egui::Vec2,
    speed: f32,
    tint: bool,
    salt: u64,
}

impl<'a> StickyScrollReveal<'a> {
    /// A reveal over `sections`.
    pub fn new(sections: &'a [StickySection]) -> Self {
        Self { sections, height: 380.0, panel: vec2(300.0, 200.0), speed: 70.0, tint: true, salt: 0 }
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Size of the sticky panel.
    pub fn panel_size(mut self, s: egui::Vec2) -> Self {
        self.panel = s.max(vec2(40.0, 40.0));
        self
    }
    /// Autoplay speed in points per second (0 = off).
    pub fn autoplay(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }
    /// Shift the backdrop color with the active section (default on).
    pub fn tint_background(mut self, on: bool) -> Self {
        self.tint = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    fn colors(&self, t: &Theme, i: usize) -> [Color32; 2] {
        self.sections[i].colors.unwrap_or_else(|| {
            let a = t.chart[i % t.chart.len()];
            let b = t.chart[(i + 2) % t.chart.len()];
            [a, b]
        })
    }

    /// Show the reveal; `panel(ui, index, rect)` draws section `index`'s sticky panel (it is
    /// called for every section that is currently visible, with opacity already applied).
    pub fn show(self, ui: &mut Ui, mut panel: impl FnMut(&mut Ui, usize, Rect)) -> Response {
        let t = Theme::of(ui);
        let n = self.sections.len();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_sticky_reveal", self.salt)), Sense::drag());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let pad = 34.0;
        let panel_w = self.panel.x.min(rect.width() * 0.42);
        let panel_h = self.panel.y.min(rect.height() - 2.0 * pad);
        let text_w = (rect.width() - panel_w - pad * 3.0).max(80.0);
        let text_x = rect.min.x + pad;
        // measure sections
        let title_font = FontId::proportional(24.0);
        let body_font = FontId::proportional(15.0);
        let painter0 = ui.painter();
        let galleys: Vec<_> = self
            .sections
            .iter()
            .map(|s| {
                (
                    painter0.layout(s.title.clone(), title_font.clone(), t.foreground, text_w),
                    painter0.layout(s.body.clone(), body_font.clone(), t.muted_foreground, text_w),
                )
            })
            .collect();
        let gap = 90.0;
        let top_pad = rect.height() * 0.22;
        let mut tops = Vec::with_capacity(n);
        let mut y = top_pad;
        for (g, b) in &galleys {
            tops.push(y);
            y += g.size().y + 12.0 + b.size().y + gap;
        }
        let content_h = y - gap + rect.height() * 0.55;
        let max = (content_h - rect.height()).max(0.0);
        let s = drive_scroll(ui, id, &response, max, false, self.speed);
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        // active section: the last one whose top passed 35% of the viewport
        let line = rect.height() * 0.38;
        let active = (0..n).rev().find(|i| tops[*i] - s <= line).unwrap_or(0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let weights: Vec<f32> = (0..n)
            .map(|i| glowkit_core::anim::follow(ui, id.with(("w", i)), if i == active { 1.0 } else { 0.0 }, 7.0))
            .collect();
        let painter = ui.painter_at(rect);
        // backdrop tinted by the active section
        let base_bg = if t.dark { shade(t.card, 0.02) } else { t.card };
        let mut bg = base_bg;
        if self.tint {
            let mut acc = [0.0f32; 3];
            let mut wsum = 0.0;
            for (i, w) in weights.iter().enumerate() {
                let c = self.colors(&t, i)[0];
                acc[0] += c.r() as f32 * w;
                acc[1] += c.g() as f32 * w;
                acc[2] += c.b() as f32 * w;
                wsum += w;
            }
            if wsum > 0.0 {
                let c = Color32::from_rgb((acc[0] / wsum) as u8, (acc[1] / wsum) as u8, (acc[2] / wsum) as u8);
                bg = mix(base_bg, c, if t.dark { 0.13 } else { 0.08 });
            }
        }
        let radius = (t.radius * 1.6).round();
        painter.rect(rect, CornerRadius::same(radius as u8), bg, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // text column
        let text_clip = Rect::from_min_max(
            pos2(rect.min.x + radius, rect.min.y + 1.0),
            pos2(text_x + text_w + 10.0, rect.max.y - 1.0),
        );
        let tp = painter.with_clip_rect(text_clip.intersect(ui.clip_rect()));
        for (i, (g, b)) in galleys.iter().enumerate() {
            let y = rect.min.y + tops[i] - s;
            if y > rect.max.y || y + g.size().y + b.size().y + 12.0 < rect.min.y {
                continue;
            }
            let a = 0.28 + 0.72 * weights[i];
            tp.galley(pos2(text_x, y), g.clone(), with_alpha(t.foreground, a));
            tp.galley_with_override_text_color(
                pos2(text_x, y + g.size().y + 12.0),
                b.clone(),
                with_alpha(t.muted_foreground, a),
            );
            // accent tick next to the active title
            let [c0, _] = self.colors(&t, i);
            let tick = Rect::from_min_size(pos2(text_x - 16.0, y + 4.0), vec2(3.0, g.size().y - 8.0));
            tp.rect_filled(tick, CornerRadius::same(2), with_alpha(c0, weights[i]));
        }
        paint::fade_edges_vertical(&tp, text_clip.shrink2(vec2(0.0, 1.0)), 40.0, bg);
        // sticky panel
        let panel_rect =
            Rect::from_center_size(pos2(rect.max.x - pad - panel_w / 2.0, rect.center().y), vec2(panel_w, panel_h));
        let sc = if t.dark { Color32::from_black_alpha(150) } else { Color32::from_rgba_unmultiplied(30, 30, 70, 40) };
        paint::soft_shadow(&painter, panel_rect, 16.0, 26.0, vec2(0.0, 10.0), sc);
        for (i, w) in weights.iter().enumerate() {
            if *w <= 0.01 {
                continue;
            }
            let mut child = ui.new_child(UiBuilder::new().max_rect(panel_rect).id_salt(("sticky_panel", i)));
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            child.multiply_opacity(*w);
            panel(&mut child, i, panel_rect);
        }
        let track = Rect::from_min_size(pos2(rect.max.x - 9.0, rect.min.y + 18.0), vec2(3.0, rect.height() - 36.0));
        scroll_rail(&painter, &t, track, rect.height() / content_h, s / max.max(1.0));
        response
    }
}

impl egui::Widget for StickyScrollReveal<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let sections = self.sections;
        let cols: Vec<[Color32; 2]> = (0..sections.len()).map(|i| self.colors(&t, i)).collect();
        self.show(ui, |ui, i, r| {
            let p = ui.painter();
            let a = ui.opacity();
            let stops = [cols[i][0], cols[i][1]];
            paint::gradient_rect(p, r, 16.0, 45.0, &stops);
            p.rect_stroke(
                r,
                CornerRadius::same(16),
                Stroke::new(1.0, with_alpha(Color32::WHITE, 0.2 * a)),
                StrokeKind::Inside,
            );
            // faint grid texture
            let grid = with_alpha(Color32::WHITE, 0.07);
            let mut x = r.min.x + 20.0;
            while x < r.max.x {
                p.line_segment([pos2(x, r.min.y + 1.0), pos2(x, r.max.y - 1.0)], Stroke::new(1.0, grid));
                x += 20.0;
            }
            let mut y = r.min.y + 20.0;
            while y < r.max.y {
                p.line_segment([pos2(r.min.x + 1.0, y), pos2(r.max.x - 1.0, y)], Stroke::new(1.0, grid));
                y += 20.0;
            }
            let s = &sections[i];
            if !s.icon.is_empty() {
                p.text(
                    r.center() - vec2(0.0, 14.0),
                    Align2::CENTER_CENTER,
                    &s.icon,
                    FontId::proportional(r.height() * 0.3),
                    Color32::WHITE,
                );
            }
            p.text(
                pos2(r.center().x, r.max.y - 30.0),
                Align2::CENTER_CENTER,
                &s.title,
                FontId::proportional(16.0),
                with_alpha(Color32::WHITE, 0.95),
            );
        })
    }
}
