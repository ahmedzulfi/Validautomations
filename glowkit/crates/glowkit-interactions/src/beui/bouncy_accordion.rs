use super::common::{hover_fill, icon, idle_for, lerp, surface, BeIcon};
use egui::{pos2, vec2, Align2, CornerRadius, FontId, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct AccState {
    open: Option<usize>,
    last_auto: f64,
    step: usize,
}

/// An accordion whose rows are one rounded block until you open one: the opened row breaks away
/// into its own card, gaps springing open above and below it while every corner around the
/// split re-rounds, and its content drops in with a bounce. Close it and the blocks snap back
/// together.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, BouncyAccordion};
/// ui.add(
///     BouncyAccordion::new()
///         .item(BeIcon::Doc, "Release brief", "Notes, owners and risks in one handoff.")
///         .item(BeIcon::Shield, "Launch checklist", "Everything that must be green first."),
/// );
/// ```
pub struct BouncyAccordion {
    items: Vec<(BeIcon, String, String)>,
    width: f32,
    radius: f32,
    gap: f32,
    bounce: f32,
    autoplay: bool,
}

impl Default for BouncyAccordion {
    fn default() -> Self {
        Self::new()
    }
}

impl BouncyAccordion {
    /// An empty accordion.
    pub fn new() -> Self {
        Self { items: Vec::new(), width: 400.0, radius: 22.0, gap: 10.0, bounce: 0.5, autoplay: true }
    }
    /// Add a row with an icon, a title and the text it reveals.
    pub fn item(mut self, icon: BeIcon, title: impl Into<String>, body: impl Into<String>) -> Self {
        self.items.push((icon, title.into(), body.into()));
        self
    }
    /// Width of the accordion.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Corner radius of the blocks.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.clamp(0.0, 40.0);
        self
    }
    /// Gap that opens around the expanded row.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.clamp(0.0, 40.0);
        self
    }
    /// Springiness, 0 (none) to 1 (very bouncy).
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 1.0);
        self
    }
    /// Open rows one after another while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BouncyAccordion {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let n = self.items.len();
        let head = 52.0;
        let pad = 20.0;
        let id = ui.next_auto_id().with("beui_accordion");
        let now = ui.input(|i| i.time);
        let st0: AccState = glowkit_core::state(ui, id, || AccState {
            open: Some(3.min(n.saturating_sub(1))),
            last_auto: now,
            step: 0,
        });
        let damping = lerp(28.0, 11.0, self.bounce);
        let bodies: Vec<std::sync::Arc<egui::Galley>> = self
            .items
            .iter()
            .map(|it| {
                painter.layout(it.2.clone(), FontId::proportional(13.5), t.muted_foreground, self.width - pad * 2.0)
            })
            .collect();
        // springs: how open each row is, and the gap after each row
        let opens: Vec<f32> = (0..n)
            .map(|i| spring(ui, id.with(("o", i)), if st0.open == Some(i) { 1.0 } else { 0.0 }, 280.0, damping))
            .collect();
        let gaps: Vec<f32> = (0..n.saturating_sub(1))
            .map(|i| {
                let split = st0.open == Some(i) || st0.open == Some(i + 1);
                spring(ui, id.with(("g", i)), if split { self.gap } else { 0.0 }, 340.0, damping + 4.0)
            })
            .collect();
        let heights: Vec<f32> = (0..n).map(|i| head + opens[i].max(0.0) * (bodies[i].size().y + 16.0)).collect();
        let max_total: f32 =
            n as f32 * head + bodies.iter().map(|b| b.size().y + 16.0).fold(0.0, f32::max) + 2.0 * self.gap + 8.0;
        let (outer, resp) = ui.allocate_exact_size(vec2(self.width, max_total), Sense::hover());
        // layout
        let mut y = outer.min.y;
        let mut rows = Vec::new();
        for (i, h) in heights.iter().enumerate() {
            rows.push(Rect::from_min_size(pos2(outer.min.x, y), vec2(self.width, *h)));
            y += h + gaps.get(i).copied().unwrap_or(0.0);
        }
        let mut clicked = None;
        for (i, r) in rows.iter().enumerate() {
            let hr = Rect::from_min_size(r.min, vec2(r.width(), head));
            let rsp = ui.interact(hr, id.with(("h", i)), Sense::click());
            if rsp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if rsp.clicked() {
                clicked = Some(i);
            }
        }
        let idle = idle_for(ui, id, ui.rect_contains_pointer(outer));
        let auto = self.autoplay && idle > 2.0 && n > 0;
        glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if let Some(i) = clicked {
                    s.open = if s.open == Some(i) { None } else { Some(i) };
                }
                if auto && now - s.last_auto > 1.8 {
                    s.last_auto = now;
                    s.step += 1;
                    let seq = [Some(0), None, Some(2), Some(4.min(n - 1)), None, Some(3.min(n - 1))];
                    s.open = seq[s.step % seq.len()].map(|v| v.min(n - 1));
                }
            },
        );
        if auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let hov = ui.input(|i| i.pointer.hover_pos());
        let fill = surface(&t);
        for (i, r) in rows.iter().enumerate() {
            let rad = self.radius;
            let top_k = if i == 0 { 1.0 } else { (gaps[i - 1] / self.gap.max(1e-3)).clamp(0.0, 1.0) };
            let bot_k = if i + 1 == n { 1.0 } else { (gaps[i] / self.gap.max(1e-3)).clamp(0.0, 1.0) };
            let tr = (rad * top_k).round() as u8;
            let br = (rad * bot_k).round() as u8;
            let corners = CornerRadius { nw: tr, ne: tr, sw: br, se: br };
            // overlap neighbours a hair so joined blocks show no seam
            let rr = Rect::from_min_max(
                r.min - vec2(0.0, if top_k < 0.5 && i > 0 { 0.5 } else { 0.0 }),
                r.max + vec2(0.0, if bot_k < 0.5 && i + 1 < n { 0.5 } else { 0.0 }),
            );
            painter.rect_filled(rr, corners, fill);
            let hr = Rect::from_min_size(r.min, vec2(r.width(), head));
            if hov.is_some_and(|p| hr.contains(p)) {
                painter.rect_filled(hr.shrink(5.0), CornerRadius::same(14), with_alpha(hover_fill(&t), 0.7));
            }
            let (ic, title, _) = &self.items[i];
            let cy = r.min.y + head / 2.0;
            icon(&painter, *ic, pos2(r.min.x + 30.0, cy), 14.0, t.muted_foreground, 1.3);
            painter.text(
                pos2(r.min.x + 54.0, cy),
                Align2::LEFT_CENTER,
                title,
                FontId::proportional(14.5),
                t.foreground,
            );
            // chevron flips with the row's spring
            let k = opens[i];
            let cc = pos2(r.max.x - 28.0, cy);
            let s = 4.0;
            let dy = 1.0 - 2.0 * k.clamp(0.0, 1.0);
            painter.add(egui::Shape::line(
                vec![cc + vec2(-s, -s * 0.5 * dy), cc + vec2(0.0, s * 0.6 * dy), cc + vec2(s, -s * 0.5 * dy)],
                egui::Stroke::new(1.4, t.muted_foreground),
            ));
            if k > 0.01 {
                let clip = painter.with_clip_rect(r.shrink2(vec2(0.0, 1.0)));
                let a = ((k - 0.15) / 0.85).clamp(0.0, 1.0);
                let g = painter.layout(
                    self.items[i].2.clone(),
                    FontId::proportional(13.5),
                    with_alpha(mix(t.muted_foreground, t.foreground, 0.15), a),
                    self.width - pad * 2.0,
                );
                clip.galley(pos2(r.min.x + pad, r.min.y + head - 6.0 + (1.0 - k) * -8.0), g, t.muted_foreground);
            }
        }
        resp
    }
}
