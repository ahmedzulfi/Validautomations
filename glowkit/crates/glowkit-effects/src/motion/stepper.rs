use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy)]
struct StepAnim {
    from: usize,
    to: usize,
    t0: f32,
}

/// A multi-step flow: numbered indicators with animated connectors and check marks, content
/// that slides between steps while the panel height springs to fit, and Back / Continue buttons.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut step = 0usize;
/// glowkit_effects::Stepper::new(&mut step, 4).show(ui, |ui, s| {
///     ui.label(format!("Content of step {}", s + 1));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, step| ...)`"]
pub struct Stepper<'a> {
    step: &'a mut usize,
    count: usize,
    width: f32,
    buttons: bool,
    finish_label: String,
    duration: f32,
}

impl<'a> Stepper<'a> {
    /// A stepper over `count` steps editing the current `step` (0-based; `count` means done).
    pub fn new(step: &'a mut usize, count: usize) -> Self {
        Self { step, count: count.max(1), width: 460.0, buttons: true, finish_label: "Complete".into(), duration: 0.45 }
    }
    /// Panel width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(200.0);
        self
    }
    /// Show the Back / Continue footer (default on).
    pub fn buttons(mut self, on: bool) -> Self {
        self.buttons = on;
        self
    }
    /// Label of the button on the last step.
    pub fn finish_label(mut self, s: impl Into<String>) -> Self {
        self.finish_label = s.into();
        self
    }
    /// Slide duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }

    /// Show the stepper; `content(ui, step)` lays out one step's body.
    pub fn show(self, ui: &mut Ui, mut content: impl FnMut(&mut Ui, usize)) -> Response {
        let t = Theme::of(ui);
        let n = self.count;
        let id = ui.scope_id().with("glowkit_stepper");
        let cur = (*self.step).min(n);
        let now = glowkit_core::time(ui);
        let mut anim: StepAnim = glowkit_core::state(ui, id.with("anim"), || StepAnim { from: cur, to: cur, t0: -10.0 });
        if anim.to != cur {
            anim = StepAnim { from: anim.to, to: cur, t0: now };
            glowkit_core::set_state(ui, id.with("anim"), anim);
        }
        let p = ((now - anim.t0) / self.duration).clamp(0.0, 1.0);
        if p < 1.0 {
            ui.ctx().request_repaint();
        }
        let content_h: f32 = glowkit_core::state(ui, id.with(("h", cur)), || 80.0);
        let panel_body_h = glowkit_core::anim::spring(ui, id.with("body_h"), content_h, 240.0, 26.0);
        let header_h = 88.0;
        let footer_h = if self.buttons { 64.0 } else { 0.0 };
        let pad = 24.0;
        let w = self.width.min(ui.available_width());
        let total_h = header_h + panel_body_h + footer_h + pad;
        let x0 = ui.max_rect().center().x - w / 2.0;
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), total_h), Sense::hover());
        let panel = Rect::from_min_size(pos2(x0.max(outer.min.x), outer.min.y), vec2(w, total_h));
        let painter = ui.painter().clone();
        super::common::theme_card(&painter, &t, panel, (t.radius * 2.0).round(), 1.0);

        // indicators
        let ind_y = panel.min.y + pad + 16.0;
        let left = panel.min.x + pad + 16.0;
        let right = panel.max.x - pad - 16.0;
        let gapx = if n > 1 { (right - left) / (n as f32 - 1.0) } else { 0.0 };
        let mut clicked_step = None;
        for i in 0..n {
            let cx = if n > 1 { left + gapx * i as f32 } else { panel.center().x };
            if i + 1 < n {
                let a = pos2(cx + 22.0, ind_y);
                let b = pos2(cx + gapx - 22.0, ind_y);
                painter.line_segment([a, b], Stroke::new(2.0, t.border));
                let fillp = glowkit_core::anim::spring(ui, id.with(("conn", i)), if cur > i { 1.0 } else { 0.0 }, 140.0, 22.0);
                if fillp > 0.001 {
                    painter.line_segment([a, a.lerp(b, fillp.min(1.0))], Stroke::new(2.0, t.accent));
                }
            }
            let state = if cur > i { 2.0 } else if cur == i { 1.0 } else { 0.0 };
            let s = glowkit_core::anim::spring(ui, id.with(("ind", i)), state, 260.0, 20.0);
            let c = pos2(cx, ind_y);
            let active = s.clamp(0.0, 1.0);
            let complete = (s - 1.0).clamp(0.0, 1.0);
            let fill = mix(t.muted, t.accent, active.max(complete));
            let r = 15.0 + (1.0 - (s - 1.0).abs()).max(0.0) * 2.0;
            if cur == i {
                painter.circle_filled(c, r + 5.0, with_alpha(t.accent, 0.18));
            }
            painter.circle(c, r, fill, Stroke::new(1.0, mix(t.border, t.accent, active)));
            if complete > 0.05 {
                // check mark draws itself in
                let a = c + vec2(-6.0, 0.5);
                let m = c + vec2(-1.5, 5.0);
                let b = c + vec2(6.5, -4.5);
                let k = complete;
                let col = t.accent_foreground;
                let first = (k * 2.0).min(1.0);
                painter.line_segment([a, a.lerp(m, first)], Stroke::new(2.4, col));
                if k > 0.5 {
                    painter.line_segment([m, m.lerp(b, (k - 0.5) * 2.0)], Stroke::new(2.4, col));
                }
            } else {
                let tc = mix(t.muted_foreground, t.accent_foreground, active);
                painter.text(c, Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(13.0), tc);
            }
            let hit = ui.interact(Rect::from_center_size(c, egui::Vec2::splat(34.0)), id.with(("ind_hit", i)), Sense::click());
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                clicked_step = Some(i);
            }
        }

        // sliding content
        let body = Rect::from_min_size(pos2(panel.min.x + pad, panel.min.y + header_h), vec2(w - pad * 2.0, panel_body_h));
        let clip = body.expand2(vec2(pad, 4.0)).intersect(ui.clip_rect());
        let dir = if anim.to >= anim.from { 1.0 } else { -1.0 };
        let e = ease(Easing::OutCubic, p);
        let slide = body.width() * 0.6;
        let mut measured = content_h;
        let mut render = |ui: &mut Ui, step: usize, offset: f32, alpha: f32, measure: bool| {
            let r = Rect::from_min_size(body.min + vec2(offset, 0.0), vec2(body.width(), 4000.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(r).id_salt(("step_body", step)));
            child.set_clip_rect(clip);
            child.multiply_opacity(alpha);
            if step >= n {
                child.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("🎉").size(30.0));
                    ui.label(egui::RichText::new("All done!").size(18.0).color(t.foreground));
                });
            } else {
                content(&mut child, step);
            }
            if measure {
                measured = child.min_rect().height();
            }
        };
        if p < 1.0 && anim.from != anim.to {
            render(ui, anim.from, -dir * slide * e, 1.0 - e, false);
        }
        render(ui, cur, dir * slide * (1.0 - e), if p < 1.0 { e } else { 1.0 }, true);
        if (measured - content_h).abs() > 0.5 {
            glowkit_core::set_state(ui, id.with(("h", cur)), measured);
            ui.ctx().request_repaint();
        }

        // footer
        if self.buttons {
            let fy = panel.max.y - pad / 2.0 - footer_h / 2.0;
            if cur > 0 && cur < n {
                let br = Rect::from_center_size(pos2(panel.min.x + pad + 40.0, fy), vec2(80.0, 36.0));
                let b = ui.interact(br, id.with("back"), Sense::click());
                let col = if b.hovered() { t.foreground } else { t.muted_foreground };
                painter.text(br.center(), Align2::CENTER_CENTER, "Back", FontId::proportional(14.0), col);
                if b.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if b.clicked() {
                    clicked_step = Some(cur - 1);
                }
            }
            if cur < n {
                let label = if cur + 1 == n { self.finish_label.as_str() } else { "Continue" };
                let br = Rect::from_center_size(pos2(panel.max.x - pad - 56.0, fy), vec2(112.0, 38.0));
                let b = ui.interact(br, id.with("next"), Sense::click());
                let hv = glowkit_core::anim::follow(ui, id.with("next_hv"), if b.hovered() { 1.0 } else { 0.0 }, 14.0);
                let fill = mix(t.accent, glowkit_core::color::shade(t.accent, 0.08), hv);
                painter.rect(br, CornerRadius::same(99), fill, Stroke::NONE, StrokeKind::Inside);
                painter.text(br.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), t.accent_foreground);
                if b.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if b.clicked() {
                    clicked_step = Some(cur + 1);
                }
            } else {
                let br = Rect::from_center_size(pos2(panel.center().x, fy), vec2(120.0, 36.0));
                let b = ui.interact(br, id.with("restart"), Sense::click());
                painter.rect(br, CornerRadius::same(99), Color32::TRANSPARENT, Stroke::new(1.0, t.border), StrokeKind::Inside);
                painter.text(br.center(), Align2::CENTER_CENTER, "Start over", FontId::proportional(14.0), t.foreground);
                if b.clicked() {
                    clicked_step = Some(0);
                }
            }
        }
        if let Some(s) = clicked_step {
            *self.step = s.min(n);
        }
        response
    }
}
