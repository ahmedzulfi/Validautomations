use super::common::{cr, idle_for, lerp, shadow_color};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// A project folder with its files peeking out of the top. Hover and the pages fan out and rise,
/// the middle one highest, each on its own spring with a little overshoot; press and the folder
/// opens, the pages sliding up into a spread row while the frosted front panel sinks. The front
/// carries the project name, a file count and an "updated" note.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ProjectFolder;
/// ui.add(ProjectFolder::new("Brand direction").files(5).updated("Updated recently"));
/// ```
pub struct ProjectFolder {
    title: String,
    files: usize,
    updated: String,
    colors: Vec<Color32>,
    width: f32,
    fan: f32,
    autoplay: bool,
}

impl ProjectFolder {
    /// A folder named `title` with five pages.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            files: 5,
            updated: "Updated recently".into(),
            colors: ["#86C5A4", "#F6CF9E", "#BAE6FD", "#D8CCF5", "#C9C29A"].iter().map(|h| hex(h)).collect(),
            width: 290.0,
            fan: 1.0,
            autoplay: true,
        }
    }
    /// Number of pages (and the count printed on the front, 1..=7 pages are drawn).
    pub fn files(mut self, n: usize) -> Self {
        self.files = n;
        self
    }
    /// The note on the bottom right.
    pub fn updated(mut self, s: impl Into<String>) -> Self {
        self.updated = s.into();
        self
    }
    /// Page colors (cycled).
    pub fn colors(mut self, c: impl IntoIterator<Item = Color32>) -> Self {
        let v: Vec<Color32> = c.into_iter().collect();
        if !v.is_empty() {
            self.colors = v;
        }
        self
    }
    /// Width of the folder.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(180.0, 480.0);
        self
    }
    /// How far the pages fan out on hover (0..2).
    pub fn fan(mut self, f: f32) -> Self {
        self.fan = f.clamp(0.0, 2.0);
        self
    }
    /// Hover and open on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn page(painter: &Painter, t: &Theme, c: Pos2, size: Vec2, angle: f32, color: Color32) {
    let local = paint::rounded_rect_points(Rect::from_center_size(Pos2::ZERO, size), 8.0, 5);
    let pts: Vec<Pos2> = local.iter().map(|p| c + rot(p.to_vec2(), angle)).collect();
    let sh: Vec<Pos2> = pts.iter().map(|p| *p + vec2(0.0, 3.0)).collect();
    painter.add(Shape::convex_polygon(sh, shadow_color(t, 0.6), Stroke::NONE));
    painter.add(Shape::convex_polygon(pts, color, Stroke::new(1.0, shade(color, -0.12))));
    let at = |x: f32, y: f32| c + rot(vec2(x * size.x, y * size.y), angle);
    let ink = shade(color, -0.22);
    // a title pill and a few text lines
    let a = at(-0.38, -0.4);
    let b = at(-0.05, -0.4);
    painter.line_segment([a, b], Stroke::new(5.0, with_alpha(ink, 0.6)));
    for (k, len) in [(0.0, 0.7), (1.0, 0.55), (2.0, 0.62)] {
        let y = -0.26 + k * 0.08;
        painter.line_segment([at(-0.38, y), at(-0.38 + len, y)], Stroke::new(1.0, with_alpha(ink, 0.35)));
    }
}

impl egui::Widget for ProjectFolder {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let w = self.width;
        let body_h = w * 0.44;
        let page_size = vec2(w * 0.32, w * 0.5);
        let size = vec2(w + 40.0, body_h + page_size.y * 0.95 + 20.0);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
        let id = resp.id;
        let idle = idle_for(ui, id, resp.hovered());
        let open_now: bool = glowkit_core::state(ui, id.with("open"), || false);
        let mut open = open_now;
        if resp.clicked() {
            open = !open;
            resp.mark_changed();
        }
        let mut hover = resp.hovered();
        if self.autoplay && idle > 2.0 {
            glowkit_core::animate(ui);
            let ph = (idle - 2.0) % 6.0;
            hover = (0.6..2.4).contains(&ph) || (3.2..5.2).contains(&ph);
            open = (3.2..5.2).contains(&ph);
        }
        glowkit_core::set_state(ui, id.with("open"), open);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let hv = spring(ui, id.with("hover"), if hover { 1.0 } else { 0.0 }, 260.0, 17.0);
        let op = spring(ui, id.with("opn"), if open { 1.0 } else { 0.0 }, 220.0, 20.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        let body = Rect::from_min_size(pos2(rect.center().x - w / 2.0, rect.max.y - body_h - 10.0 + op * 14.0), vec2(w, body_h));
        // the back of the folder (a tab and a panel)
        let back_col = if t.dark { shade(t.muted, 0.1) } else { mix(t.card, t.muted, 0.6) };
        let back = Rect::from_min_max(pos2(body.min.x, body.min.y - page_size.y * 0.55), body.max);
        painter.rect(back, cr(18.0), back_col, Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.07) } else { t.border }), egui::StrokeKind::Inside);
        // pages
        let n = self.files.clamp(1, 7);
        let mid = (n as f32 - 1.0) / 2.0;
        let mut order: Vec<usize> = (0..n).collect();
        // centre page on top
        order.sort_by(|a, b| {
            let da = (*a as f32 - mid).abs();
            let db = (*b as f32 - mid).abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        for i in order {
            let d = i as f32 - mid;
            // each page rides its own spring, so the fan ripples
            let hv = spring(ui, id.with(("pj", i)), if hover { 1.0 } else { 0.0 }, 200.0 + d.abs() * 70.0, 13.0 + d.abs() * 2.0);
            let spread = w * 0.17 * (1.0 + 0.25 * hv * self.fan);
            let rest_x = rect.center().x + d * spread;
            let lift = (1.0 - d.abs() / (mid + 1.0)) * (8.0 + 16.0 * hv * self.fan);
            let rest_y = body.min.y - page_size.y * 0.18 - lift - hv * 6.0;
            let ang = d * (0.09 + 0.05 * hv * self.fan);
            // opened: pages slide up into a level row
            let row_w = (n as f32) * page_size.x * 0.78;
            let open_x = rect.center().x - row_w / 2.0 + (i as f32 + 0.5) * row_w / n as f32;
            let open_y = rect.min.y + page_size.y * 0.5 + 4.0;
            let x = lerp(rest_x, open_x, op);
            let y = lerp(rest_y, open_y, op);
            let a = lerp(ang, d * 0.02, op);
            page(&painter, &t, pos2(x, y), page_size, a, self.colors[i % self.colors.len()]);
        }
        // frosted front panel
        let front = Rect::from_min_max(pos2(body.min.x, body.min.y + body_h * 0.18 - hv * 3.0), body.max);
        paint::soft_shadow(&painter, front, 18.0, 14.0, vec2(0.0, 2.0), shadow_color(&t, 0.3));
        let glass = if t.dark { with_alpha(shade(t.muted, 0.08), 0.94) } else { with_alpha(Color32::WHITE, 0.9) };
        painter.rect(front, cr(18.0), glass, Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.1) } else { t.border }), egui::StrokeKind::Inside);
        let pad = 18.0;
        painter.text(pos2(front.min.x + pad, front.min.y + 30.0), Align2::LEFT_CENTER, &self.title, FontId::proportional(17.0), t.foreground);
        let ly = front.max.y - 42.0;
        painter.line_segment([pos2(front.min.x, ly), pos2(front.max.x, ly)], Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.07) } else { t.border }));
        painter.text(pos2(front.min.x + pad, front.max.y - 21.0), Align2::LEFT_CENTER, format!("{} files", self.files), FontId::proportional(12.5), t.foreground);
        painter.text(pos2(front.max.x - pad, front.max.y - 21.0), Align2::RIGHT_CENTER, &self.updated, FontId::proportional(12.5), t.muted_foreground);
        resp
    }
}
