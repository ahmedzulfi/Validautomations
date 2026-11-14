use super::common::{cr, hairline, idle_for, surface, well};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Ui};
use glowkit_core::anim::Spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Rng, Theme};

#[derive(Clone, Copy, Debug)]
struct ColState {
    pos: Spring,
    vel: f32,
}

#[derive(Clone, Copy, Debug)]
struct AutoState {
    last: f64,
    seed: u64,
}

/// An iOS-style wheel picker: each column is a drum of values that rolls under your finger, the
/// rows tipping away and fading toward the rim while the middle row sits in a raised band. Fling
/// a column and it coasts, then clicks into the nearest value.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::WheelPicker;
/// let mut sel = [5usize, 8, 24];
/// let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
/// let days: Vec<String> = (1..=31).map(|d| d.to_string()).collect();
/// let years: Vec<String> = (1980..=2030).map(|y| y.to_string()).collect();
/// ui.add(WheelPicker::new(&mut sel).column(months, 120.0).column(days, 56.0).column(years, 80.0));
/// ```
pub struct WheelPicker<'a> {
    selected: &'a mut [usize],
    columns: Vec<(Vec<String>, f32)>,
    row_h: f32,
    visible: usize,
    font: f32,
    autoplay: bool,
}

impl<'a> WheelPicker<'a> {
    /// A picker editing one index per column in `selected`.
    pub fn new(selected: &'a mut [usize]) -> Self {
        Self { selected, columns: Vec::new(), row_h: 38.0, visible: 7, font: 17.0, autoplay: true }
    }
    /// Add a column of `items`, `width` points wide.
    pub fn column(mut self, items: impl IntoIterator<Item = impl Into<String>>, width: f32) -> Self {
        self.columns.push((items.into_iter().map(Into::into).collect(), width.max(30.0)));
        self
    }
    /// Height of one row (default 38).
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_h = h.max(20.0);
        self
    }
    /// Number of rows visible (odd, default 7).
    pub fn visible_rows(mut self, n: usize) -> Self {
        self.visible = (n.max(3)) | 1;
        self
    }
    /// Font size of the middle row.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font = s.max(8.0);
        self
    }
    /// Spin a column now and then while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for WheelPicker<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let pad = 8.0;
        let gap = 4.0;
        let ncol = self.columns.len().min(self.selected.len());
        let inner_w: f32 = self.columns.iter().take(ncol).map(|c| c.1).sum::<f32>() + gap * (ncol.max(1) - 1) as f32;
        let h = self.row_h * self.visible as f32 * 0.82 + pad * 2.0;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(inner_w + pad * 2.0, h), Sense::hover());
        let id = resp.id;
        let dt = glowkit_core::dt(ui);
        let now = ui.input(|i| i.time);
        let any_hover = ui.rect_contains_pointer(rect);
        let idle = idle_for(ui, id, any_hover);
        let auto = self.autoplay && idle > 2.5;
        // autoplay: pick a column and spin it to a new value
        let mut auto_kick: Option<(usize, f32)> = None;
        if auto && ncol > 0 {
            glowkit_core::animate(ui);
            glowkit_core::with_state(
                ui,
                id.with("auto"),
                || AutoState { last: now, seed: 7 },
                |a| {
                    if now - a.last > 1.9 {
                        a.last = now;
                        a.seed += 1;
                        let mut rng = Rng::new(a.seed * 31);
                        let c = rng.below(ncol);
                        let jump = rng.range(-4.0, 4.0).round();
                        auto_kick = Some((c, if jump == 0.0 { 2.0 } else { jump }));
                    }
                },
            );
        }
        let painter = ui.painter_at(rect.expand(2.0));
        if ui.is_rect_visible(rect) {
            paint::soft_shadow(&painter, rect, 22.0, 18.0, vec2(0.0, 6.0), super::common::shadow_color(&t, 0.5));
            painter.rect(rect, cr(22.0), surface(&t), hairline(&t), egui::StrokeKind::Inside);
        }
        let mid_y = rect.center().y;
        let radius = self.row_h * 2.6;
        let mut x = rect.min.x + pad;
        let mut changed = false;
        for (ci, (items, w)) in self.columns.iter().take(ncol).enumerate() {
            let n = items.len();
            let col = Rect::from_min_size(pos2(x, rect.min.y + pad), vec2(*w, rect.height() - pad * 2.0));
            x += w + gap;
            if n == 0 {
                continue;
            }
            let r = ui.interact(col, id.with(("col", ci)), Sense::drag());
            let drag = r.drag_delta().y;
            let dragging = r.dragged();
            let released = r.drag_stopped();
            let scroll = if r.hovered() { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
            if r.hovered() || dragging {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            let max = (n - 1) as f32;
            let cur = self.selected[ci].min(n - 1) as f32;
            let kick = auto_kick.filter(|k| k.0 == ci).map(|k| k.1);
            let row_h = self.row_h;
            let (pos, settled, target) = glowkit_core::with_state(
                ui,
                id.with(("cs", ci)),
                || ColState { pos: Spring::new(cur).with(150.0, 22.0), vel: 0.0 },
                |s| {
                    if dragging {
                        let d = -drag / row_h;
                        s.pos.value = (s.pos.value + d).clamp(-0.5, max + 0.5);
                        s.pos.velocity = 0.0;
                        s.vel = s.vel * 0.6 + 0.4 * if dt > 0.0 { d / dt } else { 0.0 };
                        s.pos.target = s.pos.value.round().clamp(0.0, max);
                    } else {
                        if released {
                            s.pos.target = (s.pos.value + s.vel * 0.22).round().clamp(0.0, max);
                            s.vel = 0.0;
                        }
                        if scroll != 0.0 {
                            s.pos.target = (s.pos.target - scroll / row_h).clamp(0.0, max);
                        } else {
                            s.pos.target = s.pos.target.round();
                        }
                        if let Some(k) = kick {
                            let mut nt = s.pos.target + k;
                            if nt < 0.0 || nt > max {
                                nt = s.pos.target - k;
                            }
                            s.pos.target = nt.clamp(0.0, max);
                        }
                        s.pos.step(dt);
                    }
                    (s.pos.value, s.pos.is_settled() && !dragging, s.pos.target.round().clamp(0.0, max) as usize)
                },
            );
            if !settled {
                glowkit_core::animate(ui);
            }
            if target != self.selected[ci] {
                self.selected[ci] = target;
                changed = true;
            }
            if !ui.is_rect_visible(rect) {
                continue;
            }
            // selection band
            let band = Rect::from_center_size(pos2(col.center().x, mid_y), vec2(*w, self.row_h + 2.0));
            painter.rect_filled(band, cr(12.0), well(&t));
            let clip = painter.with_clip_rect(col);
            let half = self.visible as f32 / 2.0 + 1.0;
            for (i, item) in items.iter().enumerate() {
                let d = i as f32 - pos;
                if d.abs() > half {
                    continue;
                }
                let ang = (d * self.row_h / radius).clamp(-1.5, 1.5);
                let y = mid_y + ang.sin() * radius;
                let c = ang.cos().max(0.0);
                let near = (1.0 - d.abs()).clamp(0.0, 1.0);
                let size = self.font * (0.72 + 0.28 * c) * (0.92 + 0.08 * near);
                let col_c = mix(t.muted_foreground, t.foreground, near);
                let a = c.powf(1.6);
                clip.text(
                    pos2(col.center().x, y),
                    Align2::CENTER_CENTER,
                    item,
                    FontId::proportional(size),
                    with_alpha(col_c, a),
                );
            }
        }
        if changed {
            resp.mark_changed();
        }
        resp
    }
}
