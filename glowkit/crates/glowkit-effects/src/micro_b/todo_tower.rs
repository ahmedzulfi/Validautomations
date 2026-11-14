use super::common::{hairline, icon, idle_for, ink, shadow_color, surface, text_width, MicroIcon};
use egui::{pos2, vec2, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};

#[derive(Clone, Debug)]
struct Brick {
    item: usize,
    y: f32,
    vy: f32,
    x: f32,
    angle: f32,
    spin: f32,
    checked: Option<f64>,
    leave: Option<(f64, f32)>,
}

#[derive(Clone, Debug, Default)]
struct TowerState {
    bricks: Vec<Brick>,
    empty_since: Option<f64>,
    last_auto: f64,
    serial: u64,
}

/// A to-do list stacked like a tower of bricks. Tick one and it slides out of the stack while
/// everything above drops into the gap under gravity, landing with a little bounce and a wobble.
/// When the last brick is gone a fresh stack tumbles in from above.
///
/// While idle it checks items off on its own so the tower keeps moving.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::TodoTower::new(["Water the plants", "Reply to Sam", "Book flights"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TodoTower {
    items: Vec<String>,
    count: usize,
    height: f32,
    gravity: f32,
    slip: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl TodoTower {
    /// A tower of `items`.
    pub fn new<S: Into<String>>(items: impl IntoIterator<Item = S>) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let count = items.len();
        Self { items, count, height: 340.0, gravity: 0.5, slip: 0.45, dark: false, stroke: true, autoplay: true }
    }
    /// How many bricks to stack (cycles through the items).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(1, 12);
        self
    }
    /// Stage height (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// Strength of gravity (0..1).
    pub fn gravity(mut self, k: f32) -> Self {
        self.gravity = k.clamp(0.0, 1.0);
        self
    }
    /// How loosely the bricks are stacked: sideways offsets and landing wobble (0..1).
    pub fn slip(mut self, k: f32) -> Self {
        self.slip = k.clamp(0.0, 1.0);
        self
    }
    /// Dark (ink) bricks.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outlines (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Check items off on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const BRICK_H: f32 = 34.0;
const GAP: f32 = 3.0;

fn rotated(c: Pos2, v: Vec2, a: f32) -> Pos2 {
    let (s, co) = a.sin_cos();
    c + vec2(v.x * co - v.y * s, v.x * s + v.y * co)
}

impl Widget for TodoTower {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (stage, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click());
        let id = response.id;
        if self.items.is_empty() {
            return response;
        }
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let painter = ui.painter_at(stage);
        let floor = stage.max.y - 10.0;
        let widths: Vec<f32> = self.items.iter().map(|s| text_width(&painter, s, 13.5) + 52.0).collect();
        let idle = idle_for(ui, id, response.hovered());
        let click = if response.clicked() { response.interact_pointer_pos() } else { None };
        let hover = response.hover_pos();
        let (count, n_items, slip) = (self.count, self.items.len(), self.slip);
        let g = 900.0 + 2600.0 * self.gravity;
        let auto = self.autoplay && idle > 2.5;
        let cx = stage.center().x;
        let brick_rect = |b: &Brick| Rect::from_center_size(pos2(cx + b.x, b.y), vec2(widths[b.item % n_items], BRICK_H));
        let mut hovered_brick: Option<usize> = None;
        let st = glowkit_core::with_state(ui, id, TowerState::default, |s| {
            // (re)fill: bricks tumble in from above, one after another
            let empty = s.bricks.iter().all(|b| b.leave.is_some());
            if s.bricks.is_empty() || (empty && s.empty_since.is_some_and(|e| now - e > 0.7)) {
                s.bricks.clear();
                s.serial += 1;
                for i in 0..count {
                    let h = hash01(s.serial * 97 + i as u64);
                    s.bricks.push(Brick {
                        item: i % n_items,
                        y: stage.min.y - 40.0 - i as f32 * 70.0,
                        vy: 0.0,
                        x: ((if i % 2 == 0 { -1.0 } else { 1.0 }) * (2.0 + 6.0 * h) + (h - 0.5) * 6.0) * (0.4 + slip * 1.6),
                        angle: 0.0,
                        spin: 0.0,
                        checked: None,
                        leave: None,
                    });
                }
                s.empty_since = None;
                s.last_auto = now;
            }
            if empty && s.empty_since.is_none() && !s.bricks.is_empty() {
                s.empty_since = Some(now);
            }
            // clicks and hover on standing bricks (topmost first)
            let standing: Vec<usize> = (0..s.bricks.len()).filter(|i| s.bricks[*i].checked.is_none()).collect();
            for &i in standing.iter().rev() {
                let r = brick_rect(&s.bricks[i]);
                if hover.is_some_and(|p| r.contains(p)) && hovered_brick.is_none() {
                    hovered_brick = Some(i);
                }
                if click.is_some_and(|p| r.contains(p)) {
                    s.bricks[i].checked = Some(now);
                    break;
                }
            }
            if auto && now - s.last_auto > 1.5 && !standing.is_empty() {
                s.last_auto = now;
                let pick = standing[((now * 7.31) as usize) % standing.len()];
                s.bricks[pick].checked = Some(now);
            }
            // checked bricks slide out after the tick animates
            for b in s.bricks.iter_mut() {
                if let (Some(at), None) = (b.checked, b.leave) {
                    if now - at > 0.32 {
                        let dir = if b.x >= 0.0 { 1.0 } else { -1.0 };
                        b.leave = Some((now, dir));
                    }
                }
            }
            // stack physics: each standing brick rests on the one below
            let mut rest = floor;
            let mut order: Vec<usize> = (0..s.bricks.len()).filter(|i| s.bricks[*i].leave.is_none()).collect();
            order.sort_by(|a, b| s.bricks[*b].y.total_cmp(&s.bricks[*a].y));
            for i in order {
                let b = &mut s.bricks[i];
                let target = rest - BRICK_H / 2.0;
                b.vy += g * dt;
                b.y += b.vy * dt;
                if b.y >= target {
                    b.y = target;
                    if b.vy > 120.0 {
                        // landing: a small bounce and a wobble
                        b.spin += (if b.x >= 0.0 { 1.0 } else { -1.0 }) * b.vy * 0.0009 * (0.4 + slip);
                        b.vy = -b.vy * 0.18;
                    } else {
                        b.vy = 0.0;
                    }
                }
                rest = b.y - BRICK_H / 2.0 - GAP;
                // wobble spring
                let acc = -b.angle * 300.0 - b.spin * 14.0;
                b.spin += acc * dt;
                b.angle += b.spin * dt;
            }
            for b in s.bricks.iter_mut() {
                if let Some((_, dir)) = b.leave {
                    b.x += dir * 900.0 * dt * 0.5;
                    b.angle += dir * dt * 0.6;
                    b.vy += g * dt * 0.25;
                    b.y += b.vy * dt;
                }
            }
            s.bricks.retain(|b| b.leave.is_none_or(|(at, _)| now - at < 0.8));
            s.clone()
        });
        glowkit_core::animate(ui);
        if hovered_brick.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(stage) {
            return response;
        }
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, fg, sub) = if self.dark {
            (ink_bg, ink_fg, with_alpha(ink_fg, 0.5))
        } else {
            (surface(&t), t.foreground, with_alpha(t.muted_foreground, 0.7))
        };
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        let mut draw: Vec<&Brick> = st.bricks.iter().collect();
        draw.sort_by(|a, b| b.y.total_cmp(&a.y));
        for b in draw {
            let w = widths[b.item % n_items];
            let c = pos2(cx + b.x, b.y);
            let alpha = b.leave.map_or(1.0, |(at, _)| (1.0 - (now - at) as f32 / 0.6).clamp(0.0, 1.0));
            let a = b.angle;
            let rect = Rect::from_center_size(c, vec2(w, BRICK_H));
            let pts: Vec<Pos2> = paint::rounded_rect_points(rect, 11.0, 5).into_iter().map(|p| rotated(c, p - c, a)).collect();
            paint::soft_shadow(&painter, rect, 11.0, 10.0, vec2(0.0, 2.0), shadow_color(&t, 0.4 * alpha));
            painter.add(Shape::convex_polygon(pts.clone(), with_alpha(bg, alpha), Stroke::NONE));
            if line.width > 0.0 {
                painter.add(Shape::closed_line(pts, Stroke::new(1.0, line.color.gamma_multiply(alpha))));
            }
            // checkbox
            let tick = b.checked.map_or(0.0, |at| ((now - at) as f32 / 0.22).clamp(0.0, 1.0));
            let box_c = rotated(c, vec2(-w / 2.0 + 18.0, 0.0), a);
            let bs = 13.0;
            let box_pts: Vec<Pos2> = paint::rounded_rect_points(Rect::from_center_size(box_c, Vec2::splat(bs)), 3.5, 3)
                .into_iter()
                .map(|p| rotated(box_c, p - box_c, a))
                .collect();
            if tick > 0.0 {
                painter.add(Shape::convex_polygon(box_pts, with_alpha(fg, alpha * tick), Stroke::NONE));
                let col = with_alpha(if self.dark { ink_bg } else { t.background }, alpha);
                icon(&painter, MicroIcon::Check, box_c, 8.0 * (0.5 + 0.5 * tick), col, 1.6);
            } else {
                painter.add(Shape::closed_line(box_pts, Stroke::new(1.2, with_alpha(sub, alpha))));
            }
            let text_c = rotated(c, vec2(-w / 2.0 + 34.0 + (w - 52.0) / 2.0, 0.0), a);
            let label = &self.items[b.item % n_items];
            let tc = if tick > 0.0 { with_alpha(fg, alpha * (1.0 - 0.5 * tick)) } else { with_alpha(fg, alpha) };
            glowkit_core::text::paint_rotated(&painter, label, text_c, FontId::proportional(13.5), tc, a);
            if tick > 0.0 {
                let half = (w - 52.0) / 2.0 * tick;
                painter.line_segment(
                    [rotated(text_c, vec2(-(w - 52.0) / 2.0, 0.5), a), rotated(text_c, vec2(-(w - 52.0) / 2.0 + half * 2.0, 0.5), a)],
                    Stroke::new(1.2, with_alpha(fg, alpha * 0.6)),
                );
            }
        }
        response
    }
}
