use super::common::{hairline, hover_fill, icon, idle_for, morph, surface, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Debug)]
struct StepState {
    prev: String,
    shown: String,
    changed_at: f64,
    dir: f32,
    held_since: Option<f64>,
    last_repeat: f64,
    last_auto: f64,
    auto_dir: i64,
}

/// A number stepper that adapts its shape. The value sits in a pill between minus and plus
/// buttons; changed digits roll out and the new ones roll in (up for more, down for less), the
/// pill's width springs to fit the number of digits, and at either end of the range the dead
/// button folds away while the pill swells into its space. Hold a button to repeat.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut qty = 2i64;
/// ui.add(glowkit_interactions::AdaptiveStepper::new(&mut qty).range(0, 1200));
/// ```
pub struct AdaptiveStepper<'a> {
    value: &'a mut i64,
    min: i64,
    max: i64,
    step: i64,
    height: f32,
    autoplay: bool,
}

impl<'a> AdaptiveStepper<'a> {
    /// A stepper editing `value` (range 0..=99 by default).
    pub fn new(value: &'a mut i64) -> Self {
        Self { value, min: 0, max: 99, step: 1, height: 48.0, autoplay: true }
    }
    /// Allowed range (inclusive).
    pub fn range(mut self, min: i64, max: i64) -> Self {
        self.min = min.min(max);
        self.max = max.max(min);
        self
    }
    /// Amount per press.
    pub fn step(mut self, s: i64) -> Self {
        self.step = s.max(1);
        self
    }
    /// Height of the control.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(28.0, 80.0);
        self
    }
    /// Count up and down on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for AdaptiveStepper<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let h = self.height;
        let fs = h * 0.36;
        let painter = ui.painter().clone();
        let max_digits = self.max.abs().max(self.min.abs()).to_string().len() + usize::from(self.min < 0);
        let dw = text_width(&painter, "0", fs);
        let pill_max = dw * max_digits as f32 + h * 1.0;
        let gap = 8.0;
        let total = h * 2.0 + gap * 2.0 + pill_max;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let v = (*self.value).clamp(self.min, self.max);
        let at_min = v <= self.min;
        let at_max = v >= self.max;
        // live layout: buttons fold away at the ends, the pill grows into the gap
        let lw = spring(ui, id.with("lw"), if at_min { 0.0 } else { h }, 380.0, 24.0);
        let rw = spring(ui, id.with("rw"), if at_max { 0.0 } else { h }, 380.0, 24.0);
        let digits = v.to_string().len() as f32;
        let pill_w = dw * digits + h * 0.9;
        let lg = gap * (lw / h);
        let rg = gap * (rw / h);
        let pill_target_w = pill_w + (h - lw) + (h - rw);
        let row_w = lw + lg + pill_target_w + rg + rw;
        let x0 = rect.center().x - row_w / 2.0;
        let minus = Rect::from_min_size(pos2(x0, rect.min.y), vec2(lw, h));
        let pill_t = Rect::from_min_size(pos2(minus.max.x + lg, rect.min.y), vec2(pill_target_w, h));
        let plus = Rect::from_min_size(pos2(pill_t.max.x + rg, rect.min.y), vec2(rw, h));
        let mr = ui.interact(minus, id.with("minus"), Sense::click_and_drag());
        let pr = ui.interact(plus, id.with("plus"), Sense::click_and_drag());
        for r in [&mr, &pr] {
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        let idle = idle_for(ui, id, mr.hovered() || pr.hovered());
        let auto = self.autoplay && idle > 2.0;
        let press_dir = if mr.is_pointer_button_down_on() && !at_min {
            -1
        } else if pr.is_pointer_button_down_on() && !at_max {
            1
        } else {
            0
        };
        let clicked_dir = if mr.clicked() || mr.drag_started() {
            -1
        } else if pr.clicked() || pr.drag_started() {
            1
        } else {
            0
        };
        let text_now = v.to_string();
        let (min, max, step) = (self.min, self.max, self.step);
        let mut delta = 0i64;
        glowkit_core::with_state(
            ui,
            id,
            || initial_state(&text_now, now),
            |s| {
                if clicked_dir != 0 && s.held_since.is_none() {
                    delta = clicked_dir;
                    s.held_since = Some(now);
                    s.last_repeat = now;
                }
                if press_dir != 0 {
                    if let Some(h0) = s.held_since {
                        if now - h0 > 0.45 && now - s.last_repeat > 0.07 {
                            s.last_repeat = now;
                            delta = press_dir;
                        }
                    }
                } else if clicked_dir == 0 {
                    s.held_since = None;
                }
                if auto && now - s.last_auto > 0.55 {
                    s.last_auto = now;
                    if v + s.auto_dir * step > max || v + s.auto_dir * step < min {
                        s.auto_dir = -s.auto_dir;
                    }
                    // jump a little now and then so the width change shows
                    delta = s.auto_dir * if (now as i64) % 5 == 0 { 7 } else { 1 };
                }
            },
        );
        if delta != 0 {
            let nv = (v + delta * step).clamp(min, max);
            if nv != v {
                *self.value = nv;
                resp.mark_changed();
                let txt = nv.to_string();
                glowkit_core::with_state(
                    ui,
                    id,
                    || initial_state(&txt, now),
                    |s| {
                        s.prev = s.shown.clone();
                        s.shown = txt.clone();
                        s.changed_at = now;
                        s.dir = if delta > 0 { 1.0 } else { -1.0 };
                    },
                );
            }
        }
        if auto || press_dir != 0 {
            glowkit_core::animate(ui);
        }
        let st: StepState = glowkit_core::state(ui, id, || initial_state(&text_now, now));
        let (pill, pr_) = morph(ui, id.with("pill"), pill_t, h / 2.0, 360.0, 22.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        // buttons
        for (r, resp_b, ic, k) in [(minus, &mr, BeIcon::Minus, lw / h), (plus, &pr, BeIcon::Plus, rw / h)] {
            if k < 0.03 {
                continue;
            }
            let c = r.center();
            let press = if resp_b.is_pointer_button_down_on() { 1.5 } else { 0.0 };
            let rad = (h / 2.0) * k - press;
            painter.circle(c, rad, if resp_b.hovered() { hover_fill(&t) } else { surface(&t) }, hairline(&t));
            icon(&painter, ic, c, h * 0.32 * k, with_alpha(t.foreground, k), 1.6);
        }
        painter.rect(
            pill,
            egui::CornerRadius::same(pr_.round().min(255.0) as u8),
            surface(&t),
            hairline(&t),
            egui::StrokeKind::Inside,
        );
        // rolling digits, right aligned on the pill centre
        let since = (now - st.changed_at) as f32;
        let p = (since / 0.28).clamp(0.0, 1.0);
        let e = glowkit_core::ease(glowkit_core::Easing::OutBack, p);
        if p < 1.0 {
            glowkit_core::animate(ui);
        }
        let cur: Vec<char> = st.shown.chars().collect();
        let old: Vec<char> = st.prev.chars().collect();
        let tw = dw * cur.len() as f32;
        let clip = painter.with_clip_rect(pill.shrink(2.0));
        let font = FontId::monospace(fs);
        let roll = h * 0.6;
        for (k, ch) in cur.iter().enumerate() {
            // align from the right so the units digit stays put
            let from_right = cur.len() - 1 - k;
            let ok = old.len() as i64 - 1 - from_right as i64;
            let old_ch = if ok >= 0 { old.get(ok as usize).copied() } else { None };
            let x = pill.center().x - tw / 2.0 + dw * (k as f32 + 0.5);
            let y = pill.center().y;
            if old_ch == Some(*ch) || p >= 1.0 {
                clip.text(pos2(x, y), Align2::CENTER_CENTER, ch.to_string(), font.clone(), t.foreground);
            } else {
                let d = st.dir;
                if let Some(oc) = old_ch {
                    clip.text(
                        pos2(x, y - d * roll * e),
                        Align2::CENTER_CENTER,
                        oc.to_string(),
                        font.clone(),
                        with_alpha(t.foreground, 1.0 - p),
                    );
                }
                clip.text(
                    pos2(x, y + d * roll * (1.0 - e)),
                    Align2::CENTER_CENTER,
                    ch.to_string(),
                    font.clone(),
                    with_alpha(mix(t.foreground, t.accent, 1.0 - p), p.max(0.2)),
                );
            }
        }
        resp
    }
}

fn initial_state(txt: &str, now: f64) -> StepState {
    StepState {
        prev: txt.to_string(),
        shown: txt.to_string(),
        changed_at: now - 10.0,
        dir: 1.0,
        held_since: None,
        last_repeat: now,
        last_auto: now,
        auto_dir: 1,
    }
}
