use super::common::idle_for;
use egui::{pos2, vec2, FontId, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::{spring, Spring};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::text::paint_rotated;
use glowkit_core::Theme;

/// Which way the arc of an [`ArcPicker`] bulges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ArcSide {
    /// Items run down a curve that bulges to the right (drag vertically).
    #[default]
    Right,
    /// Items run down a curve that bulges to the left (drag vertically).
    Left,
    /// Items run across a curve that bulges upward (drag horizontally).
    Top,
    /// Items run across a curve that bulges downward (drag horizontally).
    Bottom,
}

#[derive(Clone, Copy, Debug)]
struct ArcState {
    pos: Spring,
    vel: f32,
    last_auto: f64,
    dir: f32,
}

/// A radial scroll picker: the options sit on the rim of a large wheel, each one turned to follow
/// the curve, and fade and shrink as they roll away from the middle. Drag, scroll or use the
/// arrow keys to roll it; the chosen item glows in the accent color between a pair of brackets
/// that spring to its width.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{ArcPicker, ArcSide};
/// let mut sel = 3usize;
/// ui.add(ArcPicker::new(&mut sel, ["Dawn", "Morning", "Noon", "Dusk"]).side(ArcSide::Left));
/// ```
pub struct ArcPicker<'a> {
    selected: &'a mut usize,
    items: Vec<String>,
    side: ArcSide,
    radius: f32,
    step_deg: f32,
    font: f32,
    size: Option<Vec2>,
    accent: Option<egui::Color32>,
    autoplay: bool,
}

impl<'a> ArcPicker<'a> {
    /// A picker over `items`, editing the index in `selected`.
    pub fn new(selected: &'a mut usize, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            selected,
            items: items.into_iter().map(Into::into).collect(),
            side: ArcSide::Right,
            radius: 260.0,
            step_deg: 11.0,
            font: 22.0,
            size: None,
            accent: None,
            autoplay: true,
        }
    }
    /// Which way the arc bulges.
    pub fn side(mut self, side: ArcSide) -> Self {
        self.side = side;
        self
    }
    /// Radius of the wheel the items sit on (larger = flatter curve).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(60.0);
        self
    }
    /// Angle between neighbouring items, in degrees.
    pub fn spacing(mut self, deg: f32) -> Self {
        self.step_deg = deg.clamp(3.0, 40.0);
        self
    }
    /// Font size of the selected item.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font = s.max(8.0);
        self
    }
    /// Widget size (default: available width x 360).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Highlight color (default: the theme accent).
    pub fn accent(mut self, c: egui::Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Roll back and forth on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for ArcPicker<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = resp.id;
        let n = self.items.len();
        if n == 0 {
            return resp;
        }
        let step = self.step_deg.to_radians();
        let vertical = matches!(self.side, ArcSide::Left | ArcSide::Right);
        let unit = self.radius * step; // points per item along the rim
        let drag = if vertical { resp.drag_delta().y } else { resp.drag_delta().x };
        let hovered = resp.hovered();
        let scroll = if hovered {
            let d = ui.input(|i| i.smooth_scroll_delta);
            if vertical {
                d.y
            } else {
                d.y + d.x
            }
        } else {
            0.0
        };
        let key = if resp.has_focus() || hovered {
            ui.input(|i| {
                let fwd = i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowRight);
                let back = i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowLeft);
                fwd as i32 - back as i32
            })
        } else {
            0
        };
        if resp.clicked() {
            resp.request_focus();
        }
        let dragging = resp.dragged();
        let released = resp.drag_stopped();
        let active = dragging || scroll.abs() > 0.0 || key != 0 || hovered;
        let idle = idle_for(ui, id, active);
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let auto = self.autoplay && idle > 2.5;
        let cur = (*self.selected).min(n - 1);
        let max = (n - 1) as f32;
        let (pos, sel, moving) = glowkit_core::with_state(
            ui,
            id,
            || ArcState { pos: Spring::new(cur as f32).with(210.0, 24.0), vel: 0.0, last_auto: now, dir: 1.0 },
            |s| {
                if dragging {
                    let d = -drag / unit;
                    s.pos.value = (s.pos.value + d).clamp(-0.6, max + 0.6);
                    s.pos.velocity = 0.0;
                    s.vel = if dt > 0.0 { d / dt } else { 0.0 };
                    s.pos.target = s.pos.value.round().clamp(0.0, max);
                } else {
                    if released {
                        s.pos.target = (s.pos.value + s.vel * 0.12).round().clamp(0.0, max);
                    }
                    if scroll.abs() > 0.0 {
                        s.pos.target = (s.pos.target - scroll / unit).clamp(0.0, max);
                    } else {
                        s.pos.target = s.pos.target.round();
                    }
                    if key != 0 {
                        s.pos.target = (s.pos.target.round() + key as f32).clamp(0.0, max);
                    }
                    if auto && now - s.last_auto > 1.4 {
                        s.last_auto = now;
                        let mut nt = s.pos.target.round() + s.dir;
                        if nt > max || nt < 0.0 {
                            s.dir = -s.dir;
                            nt = s.pos.target.round() + s.dir;
                        }
                        s.pos.target = nt.clamp(0.0, max);
                    }
                    s.pos.step(dt);
                }
                if active {
                    s.last_auto = now;
                }
                let sel = s.pos.target.round().clamp(0.0, max) as usize;
                (s.pos.value, sel, dragging || !s.pos.is_settled())
            },
        );
        if moving || auto {
            glowkit_core::animate(ui);
        }
        if sel != *self.selected {
            *self.selected = sel;
            resp.mark_changed();
        }
        if hovered || dragging {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let accent = self.accent.unwrap_or(t.accent);
        let r = self.radius;
        // the rim passes through a point a little inside the bulge side
        let bulge = 0.18;
        let center = match self.side {
            ArcSide::Right => pos2(rect.center().x + rect.width() * bulge - r, rect.center().y),
            ArcSide::Left => pos2(rect.center().x - rect.width() * bulge + r, rect.center().y),
            ArcSide::Top => pos2(rect.center().x, rect.center().y - rect.height() * bulge + r),
            ArcSide::Bottom => pos2(rect.center().x, rect.center().y + rect.height() * bulge - r),
        };
        let side = self.side;
        let dir_fn = |a: f32| -> (Vec2, f32) {
            match side {
                ArcSide::Right => (vec2(a.cos(), a.sin()), a),
                ArcSide::Left => (vec2(-a.cos(), a.sin()), -a),
                ArcSide::Top => (vec2(a.sin(), -a.cos()), a),
                ArcSide::Bottom => (vec2(a.sin(), a.cos()), -a),
            }
        };
        {
            let (dir, _) = dir_fn(0.0);
            let mid = center + dir * r;
            glowkit_core::paint::glow_ellipse(&painter, mid, vec2(120.0, self.font * 1.4), with_alpha(accent, 0.07));
        }
        let fade_span = 5.5;
        for (i, label) in self.items.iter().enumerate() {
            let d = i as f32 - pos;
            if d.abs() > fade_span {
                continue;
            }
            let a = d * step;
            let (dir, rot) = dir_fn(a);
            let p = center + dir * r;
            let k = (1.0 - (d.abs() / fade_span)).clamp(0.0, 1.0);
            let near = (1.0 - d.abs()).clamp(0.0, 1.0);
            let alpha = k.powf(1.3);
            let scale = 0.78 + 0.22 * near;
            let col = mix(t.muted_foreground, accent, near);
            let col = with_alpha(col, alpha * (0.55 + 0.45 * near));
            paint_rotated(&painter, label, p, FontId::proportional(self.font * scale), col, rot);
        }
        // brackets around the middle slot, sized to the selected label
        let tw =
            painter.layout_no_wrap(self.items[sel].clone(), FontId::proportional(self.font), t.foreground).size().x;
        let bw = spring(ui, id.with("bw"), tw + 26.0, 300.0, 18.0);
        let (dir, rot) = dir_fn(0.0);
        let mid = center + dir * r;
        let h = self.font * 1.3;
        let along = vec2(rot.cos(), rot.sin());
        let across = along.rot90();
        let st = Stroke::new(1.6, accent);
        for sgn in [-1.0f32, 1.0] {
            let x = mid + along * (bw / 2.0) * sgn;
            let lip = along * 5.0 * -sgn;
            painter.line_segment([x - across * h / 2.0, x + across * h / 2.0], st);
            painter.line_segment([x - across * h / 2.0, x - across * h / 2.0 + lip], st);
            painter.line_segment([x + across * h / 2.0, x + across * h / 2.0 + lip], st);
        }
        resp
    }
}
