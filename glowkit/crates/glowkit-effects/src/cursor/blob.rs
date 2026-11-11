use super::area_input;
use super::trail::{area_setters, area_widget};
use crate::motion::common::{fill_field, smin};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Theme};

/// 2D spring stored in widget state.
#[derive(Clone, Copy, Default)]
pub(crate) struct Spring2 {
    pub x: Spring,
    pub y: Spring,
}

impl Spring2 {
    pub fn at(p: Pos2, stiffness: f32, damping: f32) -> Self {
        Self { x: Spring::new(p.x).with(stiffness, damping), y: Spring::new(p.y).with(stiffness, damping) }
    }
    pub fn step(&mut self, target: Pos2, dt: f32) -> Pos2 {
        self.x.target = target.x;
        self.y.target = target.y;
        pos2(self.x.step(dt), self.y.step(dt))
    }
    pub fn pos(&self) -> Pos2 {
        pos2(self.x.value, self.y.value)
    }
    pub fn vel(&self) -> Vec2 {
        vec2(self.x.velocity, self.y.velocity)
    }
    pub fn settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }
}

// ---------------------------------------------------------------------------------------------

/// Gooey blobs that chase the pointer with increasing lag and melt into one another
/// (a metaball effect drawn with marching squares).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::BlobCursor::new().sizes(&[60.0, 90.0, 44.0]).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct BlobCursor {
    colors: Option<Vec<Color32>>,
    sizes: Vec<f32>,
    goo: f32,
    inner_dot: bool,
    hide_cursor: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for BlobCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobCursor {
    /// Three blobs in the theme gradient.
    pub fn new() -> Self {
        Self { colors: None, sizes: vec![56.0, 84.0, 44.0], goo: 26.0, inner_dot: true, hide_cursor: true, autoplay: false, area: None, salt: 0 }
    }
    /// Gradient across the goo. Defaults to the theme gradient.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Diameter of each blob, leader first; later blobs lag further behind.
    pub fn sizes(mut self, sizes: &[f32]) -> Self {
        if !sizes.is_empty() {
            self.sizes = sizes.iter().map(|s| s.max(4.0)).collect();
        }
        self
    }
    /// How far blobs reach out to merge (points). 0 = no goo.
    pub fn goo(mut self, k: f32) -> Self {
        self.goo = k.max(0.1);
        self
    }
    /// Paint a small highlight dot inside each blob.
    pub fn inner_dot(mut self, on: bool) -> Self {
        self.inner_dot = on;
        self
    }
    /// Hide the system cursor while it is over the area (default on).
    pub fn hide_cursor(mut self, on: bool) -> Self {
        self.hide_cursor = on;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("blob_cursor", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        if input.real && self.hide_cursor {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        let dt = glowkit_core::dt(ui);
        let n = self.sizes.len();
        let target = input.pos;
        let (blobs, settled) = glowkit_core::with_state(
            ui,
            id.with("blobs"),
            || (Vec::<Spring2>::new(), None::<Pos2>),
            |(springs, last)| {
                if target.is_some() {
                    *last = target;
                }
                let goal = last.unwrap_or(rect.center());
                if springs.len() != n {
                    *springs = (0..n)
                        .map(|i| {
                            let k = 1.0 - i as f32 / n as f32 * 0.8;
                            Spring2::at(goal, 420.0 * k * k, 32.0 * k)
                        })
                        .collect();
                }
                let mut settled = true;
                let pts: Vec<Pos2> = springs
                    .iter_mut()
                    .map(|s| {
                        let p = s.step(goal, dt);
                        settled &= s.settled();
                        p
                    })
                    .collect();
                (pts, settled)
            },
        );
        if !settled {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) || (target.is_none() && !self.autoplay && blobs.is_empty()) {
            return;
        }
        let t = Theme::of(ui);
        let colors = self.colors.clone().unwrap_or_else(|| t.gradient.to_vec());
        let painter = ui.painter_at(rect);
        let radii: Vec<f32> = self.sizes.iter().map(|s| s / 2.0).collect();
        let mut bounds = Rect::NOTHING;
        for (p, r) in blobs.iter().zip(&radii) {
            bounds = bounds.union(Rect::from_center_size(*p, Vec2::splat(2.0 * (r + self.goo) + 8.0)));
        }
        let bounds = bounds.intersect(rect.expand(4.0));
        let k = self.goo;
        let field = |p: Pos2| {
            let mut d = f32::MAX;
            for (c, r) in blobs.iter().zip(&radii) {
                let di = p.distance(*c) - r;
                d = if d == f32::MAX { di } else { smin(d, di, k) };
            }
            d
        };
        let span = bounds;
        let cols = colors.clone();
        // soft glow under the goo
        paint::glow(&painter, blobs[0], radii[0] * 2.4, with_alpha(colors[0], if t.dark { 0.35 } else { 0.22 }));
        fill_field(&painter, bounds, 3.0, field, move |p| {
            let u = ((p.x - span.min.x) / span.width().max(1.0) * 0.6 + (p.y - span.min.y) / span.height().max(1.0) * 0.4).clamp(0.0, 1.0);
            gradient(&cols, u)
        });
        if self.inner_dot {
            for (p, r) in blobs.iter().zip(&radii) {
                painter.circle_filled(*p - vec2(r * 0.18, r * 0.18), r * 0.26, with_alpha(Color32::WHITE, 0.85));
            }
        }
    }
}
area_widget!(BlobCursor);

// ---------------------------------------------------------------------------------------------

fn arrow_points(center: Pos2, angle: f32, size: f32) -> [Pos2; 4] {
    let (s, c) = angle.sin_cos();
    let tf = |x: f32, y: f32| center + vec2(x * c - y * s, x * s + y * c) * size;
    // pointing along +x
    [tf(1.0, 0.0), tf(-0.75, 0.72), tf(-0.35, 0.0), tf(-0.75, -0.72)]
}

fn paint_arrow(painter: &Painter, pts: [Pos2; 4], fill: Color32, stroke: Stroke) {
    let mut m = Mesh::default();
    for p in pts {
        m.colored_vertex(p, fill);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
    painter.add(Shape::closed_line(pts.to_vec(), stroke));
}

/// A custom cursor that glides after the pointer on a spring, turns to face where it is going
/// and stretches slightly with speed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::SmoothCursor::new().size(14.0).show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct SmoothCursor {
    color: Option<Color32>,
    size: f32,
    stiffness: f32,
    damping: f32,
    rotate: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl Default for SmoothCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl SmoothCursor {
    /// A foreground-colored arrow with a snappy spring.
    pub fn new() -> Self {
        Self { color: None, size: 14.0, stiffness: 400.0, damping: 34.0, rotate: true, autoplay: false, area: None, salt: 0 }
    }
    /// Arrow fill color. Defaults to the theme foreground.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Arrow size (half-length) in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(4.0);
        self
    }
    /// Spring stiffness and damping of the glide.
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness.max(1.0);
        self.damping = damping.max(0.1);
        self
    }
    /// Turn the arrow toward the direction of travel (default on).
    pub fn rotate(mut self, on: bool) -> Self {
        self.rotate = on;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("smooth_cursor", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        let Some(target) = input.pos else {
            glowkit_core::set_state(ui, id.with("spring"), None::<(Spring2, f32)>);
            return;
        };
        if input.real {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        let dt = glowkit_core::dt(ui);
        let (st, dm) = (self.stiffness, self.damping);
        let rotate = self.rotate;
        let (pos, vel, angle, settled) = glowkit_core::with_state(
            ui,
            id.with("spring"),
            || None::<(Spring2, f32)>,
            |s| {
                let (sp, ang) = s.get_or_insert_with(|| (Spring2::at(target, st, dm), -2.35));
                let p = sp.step(target, dt);
                let v = sp.vel();
                if rotate && v.length() > 40.0 {
                    let want = v.y.atan2(v.x);
                    let mut d = want - *ang;
                    d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    *ang += d * (1.0 - (-14.0 * dt).exp());
                }
                (p, v, *ang, sp.settled())
            },
        );
        if !settled || !input.real {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = Theme::of(ui);
        let fill = self.color.unwrap_or(t.foreground);
        let outline = if t.dark { t.background } else { Color32::WHITE };
        let painter = ui.painter_at(rect);
        let speed = vel.length();
        let stretch = (speed / 2500.0).min(0.35);
        let size = self.size * (1.0 + stretch * 0.6);
        paint::glow(&painter, pos + vec2(0.0, 3.0), self.size * 2.4, with_alpha(Color32::BLACK, if t.dark { 0.45 } else { 0.18 }));
        let mut pts = arrow_points(pos, angle, size);
        // squash sideways with speed
        let (s, c) = angle.sin_cos();
        let side = vec2(-s, c);
        for p in pts.iter_mut().skip(1) {
            let d = (*p - pos).dot(side);
            *p -= side * d * stretch;
        }
        paint_arrow(&painter, pts, fill, Stroke::new(1.6, outline));
    }
}
area_widget!(SmoothCursor);

// ---------------------------------------------------------------------------------------------

/// A pointer with a name tag that follows it around, like a collaborator's cursor in a
/// multiplayer app.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// glowkit_effects::FollowingPointer::new("Ada Lovelace").show(ui, rect);
/// ```
#[must_use = "call `.show(ui, rect)` or add it with `ui.add(...)`"]
pub struct FollowingPointer {
    label: String,
    color: Option<Color32>,
    hide_cursor: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: u64,
}

impl FollowingPointer {
    /// A pointer tagged with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), color: None, hide_cursor: true, autoplay: false, area: None, salt: 0 }
    }
    /// Pointer and tag color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Hide the system cursor while it is over the area (default on).
    pub fn hide_cursor(mut self, on: bool) -> Self {
        self.hide_cursor = on;
        self
    }
    area_setters!();

    /// Paint the effect over `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) {
        let id = ui.scope_id().with(("following_pointer", self.salt));
        let input = area_input(ui, rect, id, self.autoplay, 0.0);
        if input.real && self.hide_cursor {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        let visible = glowkit_core::anim::spring(ui, id.with("vis"), if input.pos.is_some() { 1.0 } else { 0.0 }, 300.0, 20.0);
        let dt = glowkit_core::dt(ui);
        let target = input.pos;
        let (tip, tag) = glowkit_core::with_state(
            ui,
            id.with("pos"),
            || None::<(Pos2, Spring2)>,
            |s| {
                if let Some(p) = target {
                    let (last, sp) = s.get_or_insert_with(|| (p, Spring2::at(p, 260.0, 22.0)));
                    *last = p;
                    let q = sp.step(p, dt);
                    Some((p, q))
                } else {
                    s.as_mut().map(|(last, sp)| (*last, sp.step(*last, dt)))
                }
            },
        )
        .unwrap_or((rect.center(), rect.center()));
        if visible <= 0.01 || !ui.is_rect_visible(rect) {
            return;
        }
        if !input.real {
            glowkit_core::animate(ui);
        } else if tip.distance(tag) > 0.3 {
            ui.ctx().request_repaint();
        }
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.accent);
        // the tag may hang past the area's edge
        let painter = ui.painter().with_clip_rect(rect.expand(80.0).intersect(ui.clip_rect()));
        let v = visible.clamp(0.0, 1.2);
        // pointer: classic arrow tilted up-left, tip at the pointer
        let size = 9.0 * v;
        let ang = -2.25f32;
        let center = tip - vec2(ang.cos(), ang.sin()) * size;
        let pts = arrow_points(center, ang, size);
        paint_arrow(&painter, pts, color, Stroke::new(1.4, mix(color, Color32::WHITE, 0.55)));
        // name tag, lagging slightly behind on a spring
        let font = FontId::proportional(13.0);
        let galley = painter.layout_no_wrap(self.label.clone(), font, Color32::WHITE);
        let pad = vec2(10.0, 5.0);
        let tag_size = (galley.size() + pad * 2.0) * v.max(0.0);
        let tag_min = tag + vec2(10.0, 14.0);
        let tag_rect = Rect::from_min_size(tag_min, tag_size);
        paint::soft_shadow(&painter, tag_rect, 99.0, 12.0, vec2(0.0, 4.0), with_alpha(Color32::BLACK, 0.3 * v.min(1.0)));
        painter.rect_filled(tag_rect, egui::CornerRadius::same(99), color);
        if v > 0.6 {
            let text_col = glowkit_core::color::contrast_text(color);
            painter.text(tag_rect.center(), Align2::CENTER_CENTER, &self.label, FontId::proportional(13.0), with_alpha(text_col, ((v - 0.6) / 0.4).min(1.0)));
        }
    }
}
area_widget!(FollowingPointer);
