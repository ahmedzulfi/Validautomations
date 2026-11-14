use super::common::{cr, hairline, hover_fill, icon, idle_for, morph, shadow_color, surface, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// A control that grows to explain itself. With an icon it is a round button that springs open
/// into a pill with its label beside the icon; without one it is a chip that grows a little
/// remove cross when selected. The box's width and center ride springs, so the content seems
/// to push the edge outward rather than fade in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, ExpandableControl};
/// let mut on = false;
/// ui.add(ExpandableControl::new(&mut on, "Notifications").icon(BeIcon::Bell));
/// let mut picked = true;
/// ui.add(ExpandableControl::new(&mut picked, "Rust"));
/// ```
pub struct ExpandableControl<'a> {
    open: &'a mut bool,
    label: String,
    icon: Option<BeIcon>,
    height: f32,
    tint: bool,
    autoplay: bool,
}

impl<'a> ExpandableControl<'a> {
    /// A control toggling `open`, showing `label` when expanded (or always, for a chip).
    pub fn new(open: &'a mut bool, label: impl Into<String>) -> Self {
        Self { open, label: label.into(), icon: None, height: 40.0, tint: false, autoplay: false }
    }
    /// Make it a round icon button that expands into icon + label.
    pub fn icon(mut self, icon: BeIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Height in points (default 40).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(24.0, 72.0);
        self
    }
    /// Fill with the accent color when open.
    pub fn tint(mut self, on: bool) -> Self {
        self.tint = on;
        self
    }
    /// Toggle on its own while idle (default off).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for ExpandableControl<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let h = self.height;
        let fs = h * 0.34;
        let lw = text_width(&painter, &self.label, fs);
        let closed_w = if self.icon.is_some() { h } else { lw + h * 0.8 };
        let open_w = if self.icon.is_some() { h * 0.95 + lw + h * 0.45 } else { lw + h * 0.8 + h * 0.5 };
        let (outer, mut resp) = ui.allocate_exact_size(vec2(open_w, h), Sense::click());
        let id = resp.id;
        let idle = idle_for(ui, id, resp.hovered());
        if self.autoplay && idle > 2.0 {
            let now = glowkit_core::time(ui);
            let phase = ((now + id.value() as f32 % 3.0) / 1.9) as i64;
            let want = phase % 2 == 0;
            if want != *self.open {
                *self.open = want;
            }
            glowkit_core::animate(ui);
        }
        if resp.clicked() {
            *self.open = !*self.open;
            resp.mark_changed();
        }
        let open = *self.open;
        let target = Rect::from_center_size(outer.center(), vec2(if open { open_w } else { closed_w }, h));
        let (rect, r) = morph(ui, id.with("m"), target, h / 2.0, 420.0, 24.0);
        let k = ((rect.width() - closed_w) / (open_w - closed_w)).clamp(0.0, 1.1);
        let press = spring(ui, id.with("press"), if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 600.0, 30.0);
        let rect = rect.shrink(press * 1.5);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let tinted = self.tint && open;
        let fill = if tinted {
            mix(surface(&t), t.accent, k.min(1.0))
        } else if resp.hovered() {
            hover_fill(&t)
        } else {
            surface(&t)
        };
        let fg = if tinted { mix(t.foreground, t.accent_foreground, k.min(1.0)) } else { t.foreground };
        paint::soft_shadow(&painter, rect, r, 8.0, vec2(0.0, 2.0), shadow_color(&t, 0.25));
        painter.rect(rect, cr(r), fill, hairline(&t), egui::StrokeKind::Inside);
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        if let Some(ic) = self.icon {
            // icon slides from the middle of the circle to the left of the pill
            let cx = rect.min.x + h / 2.0 + k.min(1.0) * h * 0.05;
            icon(&clip, ic, pos2(cx, rect.center().y), h * 0.38, fg, 1.4);
            clip.text(
                pos2(rect.min.x + h * 0.95 - (1.0 - k) * 6.0, rect.center().y),
                Align2::LEFT_CENTER,
                &self.label,
                FontId::proportional(fs),
                with_alpha(fg, (k * 1.4 - 0.25).clamp(0.0, 1.0)),
            );
        } else {
            clip.text(
                pos2(rect.min.x + h * 0.4, rect.center().y),
                Align2::LEFT_CENTER,
                &self.label,
                FontId::proportional(fs),
                fg,
            );
            let xc = pos2(rect.max.x - h * 0.42, rect.center().y);
            let a = (k * 1.5 - 0.3).clamp(0.0, 1.0);
            let s = h * 0.1 * (0.5 + 0.5 * a);
            let st = egui::Stroke::new(1.3, with_alpha(mix(fg, t.muted_foreground, 0.4), a));
            clip.line_segment([xc - vec2(s, s), xc + vec2(s, s)], st);
            clip.line_segment([xc - vec2(s, -s), xc + vec2(s, -s)], st);
        }
        resp
    }
}
