use super::area_input;
use super::blob::Spring2;
use super::trail::{area_setters, area_widget};
use egui::{pos2, vec2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Full-width and full-height hairlines that track the pointer, with a ring at the
/// intersection and a live coordinate readout.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::Crosshair::new().coordinates(true).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct Crosshair {
    color: Option<Color32>,
    width: f32,
    coordinates: bool,
    smoothing: f32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for Crosshair {
    fn default() -> Self {
        Self::new()
    }
}

impl Crosshair {
    /// Accent-colored crosshair with a coordinate readout.
    pub fn new() -> Self {
        Self { color: None, width: 1.0, coordinates: true, smoothing: 22.0, autoplay: false, area: None, salt: 0 }
    }
    /// Line color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Line width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(0.5);
        self
    }
    /// Show the `x / y` readout next to the intersection.
    pub fn coordinates(mut self, on: bool) -> Self {
        self.coordinates = on;
        self
    }
    /// Follow speed (higher = tighter; `f32::INFINITY` = no lag).
    pub fn smoothing(mut self, speed: f32) -> Self {
        self.smoothing = speed.max(1.0);
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("crosshair", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let vis = glowkit_core::anim::follow(ui, id.with("vis"), if input.pos.is_some() { 1.0 } else { 0.0 }, 10.0);
        let target = input.pos.unwrap_or_else(|| glowkit_core::state(ui, id.with("last"), || rect.center()));
        glowkit_core::set_state(ui, id.with("last"), target);
        let p = pos2(
            glowkit_core::anim::follow(ui, id.with("x"), target.x, self.smoothing),
            glowkit_core::anim::follow(ui, id.with("y"), target.y, self.smoothing),
        );
        if vis < 0.01 || !ui.is_rect_visible(rect) {
            return;
        }
        if input.real {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        let t = Theme::of(ui);
        let c = self.color.unwrap_or(t.accent);
        let painter = ui.painter_at(rect);
        let w = self.width;
        // lines brightest near the pointer, fading toward the edges
        let fade = |d: f32, span: f32| (1.0 - (d / span).min(1.0)).powf(0.6) * 0.85 + 0.15;
        let hx: Vec<Pos2> = (0..=48).map(|i| pos2(rect.min.x + rect.width() * i as f32 / 48.0, p.y)).collect();
        let span_x = rect.width() * 0.6;
        paint::gradient_polyline(&painter, &hx, false, w, |u| {
            let x = rect.min.x + u * rect.width();
            with_alpha(c, vis * fade((x - p.x).abs(), span_x))
        });
        let vy: Vec<Pos2> = (0..=32).map(|i| pos2(p.x, rect.min.y + rect.height() * i as f32 / 32.0)).collect();
        let span_y = rect.height() * 0.7;
        paint::gradient_polyline(&painter, &vy, false, w, |u| {
            let y = rect.min.y + u * rect.height();
            with_alpha(c, vis * fade((y - p.y).abs(), span_y))
        });
        // edge ticks
        for (a, b) in [
            (pos2(p.x, rect.min.y), pos2(p.x, rect.min.y + 8.0)),
            (pos2(p.x, rect.max.y), pos2(p.x, rect.max.y - 8.0)),
            (pos2(rect.min.x, p.y), pos2(rect.min.x + 8.0, p.y)),
            (pos2(rect.max.x, p.y), pos2(rect.max.x - 8.0, p.y)),
        ] {
            painter.line_segment([a, b], Stroke::new(w + 1.5, with_alpha(c, vis)));
        }
        paint::glow(&painter, p, 26.0, with_alpha(c, 0.25 * vis));
        painter.circle_stroke(p, 9.0, Stroke::new(1.5, with_alpha(c, vis)));
        painter.circle_filled(p, 2.0, with_alpha(t.foreground, vis));
        if self.coordinates {
            let local = p - rect.min;
            let txt = format!("x {:>4.0}   y {:>4.0}", local.x, local.y);
            let font = FontId::monospace(11.0);
            let galley = painter.layout_no_wrap(txt, font, t.foreground);
            let mut pos = p + vec2(14.0, 12.0);
            if pos.x + galley.size().x + 12.0 > rect.max.x {
                pos.x = p.x - 14.0 - galley.size().x - 12.0;
            }
            if pos.y + galley.size().y + 8.0 > rect.max.y {
                pos.y = p.y - 12.0 - galley.size().y - 8.0;
            }
            let bg = Rect::from_min_size(pos, galley.size() + vec2(12.0, 8.0));
            painter.rect(bg, egui::CornerRadius::same(5), with_alpha(t.popover, 0.92 * vis), Stroke::new(1.0, with_alpha(t.border, vis)), egui::StrokeKind::Inside);
            painter.galley(bg.min + vec2(6.0, 4.0), galley, with_alpha(t.foreground, vis));
        }
    }
}
area_widget!(Crosshair);

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct TargetState {
    corners: [Spring2; 4],
    angle: f32,
    init: bool,
    locked: f32,
}

const QUADS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

/// A cursor of four corner brackets that spins slowly, and snaps around whatever target the
/// pointer hovers, framing it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (area, button_rect) = (ui.max_rect(), ui.max_rect());
/// glowkit_effects::TargetCursor::new().targets([button_rect]).show(ui, area);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct TargetCursor {
    targets: Vec<Rect>,
    color: Option<Color32>,
    size: f32,
    spin: f32,
    padding: f32,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for TargetCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetCursor {
    /// A spinning bracket cursor with no targets yet.
    pub fn new() -> Self {
        Self { targets: Vec::new(), color: None, size: 16.0, spin: 1.6, padding: 6.0, autoplay: false, area: None, salt: 0 }
    }
    /// Rects the cursor snaps to when hovered (e.g. the `Response::rect` of buttons).
    pub fn targets(mut self, rects: impl IntoIterator<Item = Rect>) -> Self {
        self.targets = rects.into_iter().collect();
        self
    }
    /// Bracket color. Defaults to the theme foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Half-size of the idle bracket square.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(4.0);
        self
    }
    /// Idle spin speed in radians per second.
    pub fn spin(mut self, s: f32) -> Self {
        self.spin = s;
        self
    }
    /// Gap between a locked target and the brackets.
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("target_cursor", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let Some(mut p) = input.pos else {
            glowkit_core::set_state(ui, id.with("st"), TargetState::default());
            return;
        };
        if input.real {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        let mut target = self.targets.iter().copied().find(|r| r.contains(p));
        if !input.real && target.is_none() {
            // autoplay: hop between targets so the snapping is visible
            let now = glowkit_core::time(ui);
            if !self.targets.is_empty() {
                let slot = (now / 1.6).floor() as usize;
                if slot % 2 == 0 {
                    let r = self.targets[(slot / 2) % self.targets.len()];
                    target = Some(r);
                    p = r.center() + vec2((now * 2.0).sin() * r.width() * 0.2, (now * 1.7).cos() * r.height() * 0.15);
                }
            }
        }
        let dt = glowkit_core::dt(ui);
        let s = self.size;
        let pad = self.padding;
        let spin = self.spin;
        let st = glowkit_core::with_state(ui, id.with("st"), TargetState::default, |st| {
            if !st.init {
                for (k, c) in st.corners.iter_mut().enumerate() {
                    *c = Spring2::at(p + vec2(QUADS[k].0, QUADS[k].1) * s, 600.0, 36.0);
                }
                st.init = true;
            }
            let lock = target.is_some();
            st.locked += ((if lock { 1.0 } else { 0.0 }) - st.locked) * (1.0 - (-12.0 * dt).exp());
            if lock {
                let snap = (st.angle / FRAC_PI_2).round() * FRAC_PI_2;
                st.angle += (snap - st.angle) * (1.0 - (-14.0 * dt).exp());
            } else {
                st.angle = (st.angle + spin * dt).rem_euclid(TAU);
            }
            let (sn, cs) = st.angle.sin_cos();
            for (k, &(qx, qy)) in QUADS.iter().enumerate() {
                let rq = vec2(qx * cs - qy * sn, qx * sn + qy * cs);
                let goal = match target {
                    Some(r) => {
                        let r = r.expand(pad);
                        pos2(if rq.x < 0.0 { r.min.x } else { r.max.x }, if rq.y < 0.0 { r.min.y } else { r.max.y })
                    }
                    None => p + rq * s,
                };
                st.corners[k].step(goal, dt);
            }
            *st
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = Theme::of(ui);
        let c = self.color.unwrap_or(t.foreground);
        let painter = ui.painter_at(rect);
        let (sn, cs) = st.angle.sin_cos();
        let arm = 9.0 + 3.0 * st.locked;
        let stroke = Stroke::new(2.4, c);
        for (k, &(qx, qy)) in QUADS.iter().enumerate() {
            let corner = st.corners[k].pos();
            // arms point back toward the inside of the square
            let ax = vec2(-qx * cs, -qx * sn) * arm;
            let ay = vec2(qy * sn, -qy * cs) * arm;
            painter.line_segment([corner, corner + ax], stroke);
            painter.line_segment([corner, corner + ay], stroke);
            painter.circle_filled(corner, 1.2, c);
        }
        painter.circle_filled(p, 2.6 * (1.0 - st.locked * 0.4), c);
    }
}
area_widget!(TargetCursor);
