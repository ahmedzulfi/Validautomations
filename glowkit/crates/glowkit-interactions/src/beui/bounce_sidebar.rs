use super::common::{idle_for, lerp};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Shape, Stroke, Ui};
use glowkit_core::anim::{spring, Spring};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
struct HopState {
    y: Spring,
    from: f32,
    last_auto: f64,
}

/// Sidebar navigation with a bouncing indicator. Pick a link and the little dot hops to it: it
/// leaps out sideways in an arc, stretches along its flight, and lands with a squash and a
/// springy settle, while the chosen label brightens and nudges right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BounceSidebar;
/// let mut active = 1usize;
/// ui.add(BounceSidebar::new(&mut active, ["Overview", "Components", "Motion", "Templates"]));
/// ```
pub struct BounceSidebar<'a> {
    active: &'a mut usize,
    items: Vec<String>,
    row_h: f32,
    hop: f32,
    color: Option<Color32>,
    width: f32,
    autoplay: bool,
}

impl<'a> BounceSidebar<'a> {
    /// A sidebar over `items`, editing the active index.
    pub fn new(active: &'a mut usize, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            active,
            items: items.into_iter().map(Into::into).collect(),
            row_h: 42.0,
            hop: 1.0,
            color: None,
            width: 200.0,
            autoplay: true,
        }
    }
    /// Height of one link.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_h = h.clamp(24.0, 80.0);
        self
    }
    /// How far the dot leaps out sideways (0..2).
    pub fn hop(mut self, k: f32) -> Self {
        self.hop = k.clamp(0.0, 2.0);
        self
    }
    /// Dot color (default: the theme's info blue).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Width of the sidebar.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(100.0);
        self
    }
    /// Hop around on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BounceSidebar<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let n = self.items.len();
        let (rect, mut resp) = ui.allocate_exact_size(vec2(self.width, self.row_h * n as f32), Sense::hover());
        let id = resp.id;
        if n == 0 {
            return resp;
        }
        let row = |i: usize| {
            Rect::from_min_size(pos2(rect.min.x, rect.min.y + i as f32 * self.row_h), vec2(self.width, self.row_h))
        };
        let mut picked = None;
        let mut hovered = None;
        for i in 0..n {
            let r = ui.interact(row(i), id.with(i), Sense::click());
            if r.hovered() {
                hovered = Some(i);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                picked = Some(i);
            }
        }
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let idle = idle_for(ui, id, hovered.is_some());
        let auto = self.autoplay && idle > 2.0;
        let mut active = (*self.active).min(n - 1);
        if let Some(i) = picked {
            active = i;
        }
        let cy = |i: usize| rect.min.y + (i as f32 + 0.5) * self.row_h;
        let init_y = cy(active);
        let (y, vel, from, settled) = glowkit_core::with_state(
            ui,
            id,
            move || HopState { y: Spring::new(init_y).with(240.0, 15.0), from: init_y, last_auto: now },
            |s| {
                if auto && now - s.last_auto > 1.3 {
                    s.last_auto = now;
                    let k = ((now * 0.77).sin() as f32 * 0.5 + 0.5) * n as f32;
                    let mut next = (k as usize).min(n - 1);
                    if next == active {
                        next = (active + 2) % n;
                    }
                    active = next;
                }
                let target = cy(active);
                if (s.y.target - target).abs() > 0.5 {
                    s.from = s.y.value;
                    s.y.target = target;
                }
                s.y.step(dt);
                (s.y.value, s.y.velocity, s.from, s.y.is_settled())
            },
        );
        if active != *self.active {
            *self.active = active;
            resp.mark_changed();
        }
        if !settled || auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        let color = self.color.unwrap_or(t.info);
        let target = cy(active);
        // progress of the current hop: 0 at take-off, 1 on landing
        let span = (target - from).abs().max(1e-3);
        let prog = (1.0 - (target - y).abs() / span).clamp(0.0, 1.0);
        let arc = (prog * PI).sin() * (6.0 + span.sqrt() * 2.2) * self.hop;
        let base_x = rect.min.x + 8.0;
        let x = base_x - arc;
        // stretch along the motion, squash on landing (overshoot shows as negative progress)
        let speed = vel.abs();
        let stretch = (speed / 900.0).clamp(0.0, 0.8);
        let (ry, rx) = (3.5 * (1.0 + stretch), 3.5 / (1.0 + stretch * 0.6));
        // faint trail
        if speed > 40.0 {
            for k in 1..5 {
                let back = vel.signum() * k as f32 * 4.0;
                let tp = (y - back - from) / (target - from + 1e-3);
                let ta = (tp.clamp(0.0, 1.0) * PI).sin() * (6.0 + span.sqrt() * 2.2) * self.hop;
                painter.circle_filled(
                    pos2(base_x - ta, y - back),
                    3.0 - k as f32 * 0.5,
                    with_alpha(color, 0.25 - k as f32 * 0.05),
                );
            }
        }
        painter.add(Shape::ellipse_filled(pos2(x, y), vec2(rx, ry), color));
        painter.add(Shape::ellipse_stroke(
            pos2(x, y),
            vec2(rx + 3.0, ry + 3.0),
            Stroke::new(1.0, with_alpha(color, 0.18)),
        ));
        for (i, label) in self.items.iter().enumerate() {
            let on = spring(ui, id.with(("on", i)), if i == active { 1.0 } else { 0.0 }, 300.0, 20.0);
            let hv = spring(ui, id.with(("hv", i)), if hovered == Some(i) { 1.0 } else { 0.0 }, 300.0, 24.0);
            let col = mix(t.muted_foreground, t.foreground, (on + hv * 0.5).clamp(0.0, 1.0));
            let size = lerp(15.5, 16.0, on);
            painter.text(
                pos2(rect.min.x + 26.0 + on * 3.0 + hv * 2.0, cy(i)),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(size),
                col,
            );
        }
        resp
    }
}
