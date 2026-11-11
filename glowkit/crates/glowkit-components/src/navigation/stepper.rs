use crate::overlays::kit;
use crate::overlays::LineIcon;
use egui::{pos2, vec2, Align2, CursorIcon, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// A wizard header: numbered circles joined by a line that fills as you advance. Completed
/// steps turn into check marks, the current step gets a soft pulsing ring, and clicking a
/// completed step goes back to it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut step = 1;
/// use glowkit_components::Stepper;
/// ui.add(Stepper::new(&mut step, &[("Account", "Create your login"), ("Profile", "Tell us about you"), ("Done", "")]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Stepper<'a> {
    current: &'a mut usize,
    steps: Vec<(String, String)>,
    vertical: bool,
    clickable: bool,
}

impl<'a> Stepper<'a> {
    /// Steps as `(title, description)`; `current` is the active index (`steps.len()` = all done).
    pub fn new(current: &'a mut usize, steps: &[(&str, &str)]) -> Self {
        Self { current, steps: steps.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), vertical: false, clickable: true }
    }
    /// Lay the steps out top-to-bottom.
    pub fn vertical(mut self, yes: bool) -> Self {
        self.vertical = yes;
        self
    }
    /// Allow clicking completed steps to go back (default true).
    pub fn clickable(mut self, yes: bool) -> Self {
        self.clickable = yes;
        self
    }
}

impl Widget for Stepper<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.steps.len().max(1);
        let id = ui.scope_id().with(("glowkit_stepper", &self.steps));
        let size = if self.vertical { vec2(ui.available_width().min(360.0), n as f32 * 74.0) } else { vec2(ui.available_width(), 86.0) };
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::hover());
        let cur = (*self.current).min(n);
        let prog = glowkit_core::anim::spring(ui, id.with("prog"), cur as f32, 160.0, 24.0);
        let r = 15.0;
        let centers: Vec<egui::Pos2> = (0..n)
            .map(|i| {
                if self.vertical {
                    pos2(rect.min.x + r + 2.0, rect.min.y + r + i as f32 * 74.0)
                } else {
                    let seg = rect.width() / n as f32;
                    pos2(rect.min.x + seg * (i as f32 + 0.5), rect.min.y + r + 4.0)
                }
            })
            .collect();
        let p = ui.painter();
        let time = glowkit_core::time(ui);
        // connectors
        for i in 0..n.saturating_sub(1) {
            let (a, b) = (centers[i], centers[i + 1]);
            let (a, b) = if self.vertical { (a + vec2(0.0, r + 6.0), b - vec2(0.0, r + 6.0)) } else { (a + vec2(r + 8.0, 0.0), b - vec2(r + 8.0, 0.0)) };
            p.line_segment([a, b], Stroke::new(2.0, t.border));
            let f = (prog - i as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                p.line_segment([a, a + (b - a) * f], Stroke::new(2.0, t.accent));
            }
        }
        let mut new_cur = None;
        for (i, c) in centers.iter().enumerate() {
            let done = (i as f32) < prog - 0.5;
            let active = i == cur;
            let hit = Rect::from_center_size(*c, vec2(r * 2.0 + 8.0, r * 2.0 + 8.0));
            let sense = if self.clickable && i < cur { Sense::click() } else { Sense::hover() };
            let sresp = ui.interact(hit, id.with(i), sense);
            let hv = kit::hover_anim(ui, id.with(("h", i)), sresp.hovered() && i < cur);
            if sresp.clicked() {
                new_cur = Some(i);
            }
            if i < cur && self.clickable {
                sresp.on_hover_cursor(CursorIcon::PointingHand);
            }
            let fill_k = (prog - i as f32).clamp(0.0, 1.0);
            if active {
                let pulse = 0.5 + 0.5 * (time * 2.5).sin();
                p.circle_filled(*c, r + 4.0 + 2.0 * pulse, with_alpha(t.accent, 0.12 + 0.08 * pulse));
                glowkit_core::animate(ui);
            }
            let bg = mix(t.background, t.accent, fill_k);
            p.circle_filled(*c, r, bg);
            let stroke_col = if active || done { t.accent } else { t.border };
            p.circle_stroke(*c, r, Stroke::new(if active { 2.0 } else { 1.0 }, stroke_col));
            if done {
                let s = 13.0 + hv * 1.5;
                LineIcon::Check.paint_with(p, Rect::from_center_size(*c, vec2(s, s)), t.accent_foreground, 2.0);
            } else {
                let col = if active { t.accent } else { t.muted_foreground };
                p.text(*c, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(13.0), col);
            }
            let (title, desc) = &self.steps[i];
            let tcol = if active || done { t.foreground } else { t.muted_foreground };
            if self.vertical {
                p.text(pos2(c.x + r + 14.0, c.y - 8.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), tcol);
                p.text(pos2(c.x + r + 14.0, c.y + 10.0), Align2::LEFT_CENTER, desc, FontId::proportional(12.5), t.muted_foreground);
            } else {
                p.text(pos2(c.x, c.y + r + 16.0), Align2::CENTER_CENTER, title, FontId::proportional(13.5), tcol);
                p.text(pos2(c.x, c.y + r + 34.0), Align2::CENTER_CENTER, desc, FontId::proportional(12.0), t.muted_foreground);
            }
        }
        if let Some(c) = new_cur {
            *self.current = c;
            resp.mark_changed();
        }
        resp
    }
}
