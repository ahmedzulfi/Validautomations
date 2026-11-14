use super::common::{card, cr, hover_fill, icon, idle_for, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct CenterState {
    open: bool,
    since: f64,
}

/// A modal that unfolds from the middle of the screen. The trigger collapses into a thin bright
/// seam that stretches sideways, then the seam opens vertically into the dialog, as if a folded
/// card were being opened along its crease; the content rises in line by line. Closing folds
/// it back: height first, then width.
///
/// [`CenterMorphModal::show`] returns `true` on the frame the call-to-action is pressed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::CenterMorphModal;
/// let go = CenterMorphModal::new("Open modal")
///     .eyebrow("glowkit Pro")
///     .title("Ship the whole experience.")
///     .body("Animated sections and complete templates.")
///     .bullet("Premium sections")
///     .action("Explore")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CenterMorphModal {
    trigger: String,
    eyebrow: String,
    title: String,
    body: String,
    bullets: Vec<String>,
    action: String,
    width: f32,
    size: Option<Vec2>,
    seam: f32,
    autoplay: bool,
}

impl CenterMorphModal {
    /// A trigger button labelled `trigger`.
    pub fn new(trigger: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            eyebrow: String::new(),
            title: String::new(),
            body: String::new(),
            bullets: Vec::new(),
            action: "Continue".into(),
            width: 360.0,
            size: None,
            seam: 0.12,
            autoplay: true,
        }
    }
    /// Small line above the title.
    pub fn eyebrow(mut self, s: impl Into<String>) -> Self {
        self.eyebrow = s.into();
        self
    }
    /// Dialog title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Paragraph under the title.
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = s.into();
        self
    }
    /// Add a checked bullet line.
    pub fn bullet(mut self, s: impl Into<String>) -> Self {
        self.bullets.push(s.into());
        self
    }
    /// Label of the full-width call-to-action.
    pub fn action(mut self, s: impl Into<String>) -> Self {
        self.action = s.into();
        self
    }
    /// Width of the dialog.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Stage size (default: available width x 340).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// How long the seam holds before opening, in seconds.
    pub fn seam_delay(mut self, s: f32) -> Self {
        self.seam = s.clamp(0.0, 0.6);
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the trigger / dialog. Returns `true` when the action button is pressed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 340.0);
        let (area, resp) = ui.allocate_exact_size(size, Sense::click());
        let id = resp.id;
        let painter = ui.painter_at(area);
        let now = ui.input(|i| i.time);
        let pad = 20.0;
        let w = self.width.min(area.width() - 24.0);
        let inner_w = w - pad * 2.0;
        let body_g = painter.layout(self.body.clone(), FontId::proportional(12.5), t.muted_foreground, inner_w);
        let title_g = painter.layout(self.title.clone(), FontId::proportional(18.0), t.foreground, inner_w - 30.0);
        let bullets_h = self.bullets.len() as f32 * 26.0;
        let h = pad + 18.0 + title_g.size().y + 10.0 + body_g.size().y + 18.0 + bullets_h + 16.0 + 40.0 + pad;
        let dialog = Rect::from_center_size(area.center(), vec2(w, h.min(area.height() - 10.0)));
        let trig_w = text_width(&painter, &self.trigger, 14.0) + 40.0;
        let trig = Rect::from_center_size(area.center(), vec2(trig_w, 40.0));
        let st0: CenterState = glowkit_core::state(ui, id, || CenterState { open: false, since: now - 10.0 });
        let close_c = pos2(dialog.max.x - 26.0, dialog.min.y + 26.0);
        let cta = Rect::from_min_size(pos2(dialog.min.x + pad, dialog.max.y - pad - 40.0), vec2(inner_w, 40.0));
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| area.contains(*p));
        let idle = idle_for(ui, id, ptr.is_some());
        let auto = self.autoplay && idle > 2.5;
        let mut fired = false;
        let mut toggle = false;
        if let Some(p) = if resp.clicked() { resp.interact_pointer_pos() } else { None } {
            if !st0.open && trig.contains(p) {
                toggle = true;
            } else if st0.open {
                if cta.contains(p) {
                    fired = true;
                    toggle = true;
                } else if close_c.distance(p) < 14.0 || !dialog.contains(p) {
                    toggle = true;
                }
            }
        }
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if toggle || (auto && now - s.since > if s.open { 3.0 } else { 1.5 }) {
                    s.open = !s.open;
                    s.since = now;
                }
                *s
            },
        );
        if auto {
            glowkit_core::animate(ui);
        }
        let since = (now - st.since) as f32;
        // phase 1: the box flattens into a seam and stretches; phase 2: the seam opens.
        let seam_h = 3.0;
        let (tw, th) = if st.open {
            (w, if since > self.seam { dialog.height() } else { seam_h })
        } else {
            (if since > self.seam * 0.8 { trig_w } else { w }, if since > self.seam * 0.8 { 40.0 } else { seam_h })
        };
        let cw = spring(ui, id.with("w"), tw, if st.open { 520.0 } else { 420.0 }, 34.0);
        let ch = spring(ui, id.with("h"), th, if st.open { 300.0 } else { 560.0 }, if st.open { 24.0 } else { 38.0 });
        let r = spring(ui, id.with("r"), if st.open { 24.0 } else { 20.0 }, 300.0, 30.0).min(ch / 2.0);
        let rect = Rect::from_center_size(area.center(), vec2(cw, ch.max(1.0)));
        let k = ((ch - seam_h) / (dialog.height() - seam_h)).clamp(0.0, 1.0);
        let opening = st.open || ch < 30.0;
        if ptr.is_some() && (st.open || trig.contains(ptr.unwrap_or_default())) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(area) {
            return fired;
        }
        // the trigger is a filled pill; the dialog a raised card. Blend between them.
        let is_trigger = !opening || (!st.open && ch >= 30.0);
        if is_trigger && (cw - trig_w).abs() < 30.0 {
            let hov = ptr.is_some_and(|p| trig.contains(p));
            let fill = if hov { mix(t.primary, t.primary_foreground, 0.1) } else { t.primary };
            painter.rect_filled(rect, cr(r), fill);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                &self.trigger,
                FontId::proportional(14.0),
                t.primary_foreground,
            );
            return fired;
        }
        if ch < 8.0 {
            // the bright seam
            painter.rect_filled(rect.expand2(vec2(0.0, 1.0)), cr(2.0), with_alpha(t.foreground, 0.85));
            glowkit_core::paint::glow_ellipse(
                &painter,
                rect.center(),
                vec2(cw * 0.55, 16.0),
                with_alpha(t.foreground, 0.1),
            );
            return fired;
        }
        card(&painter, &t, rect, r, 0.5 + 0.8 * k);
        // the crease fades as it opens
        if k < 0.6 {
            let a = (1.0 - k / 0.6) * 0.8;
            painter.line_segment(
                [pos2(rect.min.x + 8.0, rect.center().y), pos2(rect.max.x - 8.0, rect.center().y)],
                Stroke::new(1.0, with_alpha(t.foreground, a)),
            );
        }
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        // content rises in line by line, measured from the dialog's final layout
        let off = dialog.center().y - rect.center().y;
        let line = |i: usize| -> (f32, f32) {
            let e = ((k - 0.45 - i as f32 * 0.06) / 0.35).clamp(0.0, 1.0);
            let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, e);
            (e, (1.0 - e) * 10.0 - off)
        };
        let x0 = dialog.min.x + pad;
        let mut y = dialog.min.y + pad;
        let (a, dy) = line(0);
        clip.text(
            pos2(x0, y + 8.0 + dy),
            Align2::LEFT_CENTER,
            &self.eyebrow,
            FontId::proportional(12.0),
            with_alpha(t.muted_foreground, a),
        );
        let hov_close = ptr.is_some_and(|p| close_c.distance(p) < 14.0);
        clip.circle_filled(
            pos2(close_c.x, close_c.y + dy),
            13.0,
            with_alpha(hover_fill(&t), a * if hov_close { 1.0 } else { 0.7 }),
        );
        icon(&clip, BeIcon::Close, pos2(close_c.x, close_c.y + dy), 9.0, with_alpha(t.muted_foreground, a), 1.4);
        y += 18.0;
        let (a, dy) = line(1);
        let tg =
            clip.layout(self.title.clone(), FontId::proportional(18.0), with_alpha(t.foreground, a), inner_w - 30.0);
        clip.galley(pos2(x0, y + 4.0 + dy), tg, t.foreground);
        y += title_g.size().y + 10.0;
        let (a, dy) = line(2);
        let bg = clip.layout(self.body.clone(), FontId::proportional(12.5), with_alpha(t.muted_foreground, a), inner_w);
        clip.galley(pos2(x0, y + dy), bg, t.muted_foreground);
        y += body_g.size().y + 10.0;
        let (a, dy) = line(3);
        clip.line_segment([pos2(x0, y + dy), pos2(x0 + inner_w, y + dy)], Stroke::new(1.0, with_alpha(t.border, a)));
        y += 8.0;
        for (i, b) in self.bullets.iter().enumerate() {
            let (a, dy) = line(4 + i);
            let cy = y + 13.0 + dy;
            icon(&clip, BeIcon::Check, pos2(x0 + 6.0, cy), 10.0, with_alpha(t.muted_foreground, a), 1.5);
            clip.text(
                pos2(x0 + 22.0, cy),
                Align2::LEFT_CENTER,
                b,
                FontId::proportional(12.5),
                with_alpha(t.foreground, a),
            );
            y += 26.0;
        }
        let (a, dy) = line(4 + self.bullets.len());
        let c = cta.translate(vec2(0.0, dy));
        let hov = ptr.is_some_and(|p| cta.contains(p));
        clip.rect_filled(
            c,
            cr(20.0),
            with_alpha(if hov { mix(t.primary, t.primary_foreground, 0.1) } else { t.primary }, a),
        );
        let lw = text_width(&clip, &self.action, 13.5);
        clip.text(
            c.center() - vec2(8.0, 0.0),
            Align2::CENTER_CENTER,
            &self.action,
            FontId::proportional(13.5),
            with_alpha(t.primary_foreground, a),
        );
        icon(
            &clip,
            BeIcon::ArrowUpRight,
            pos2(c.center().x + lw / 2.0 + 4.0, c.center().y),
            9.0,
            with_alpha(t.primary_foreground, a),
            1.4,
        );
        fired
    }
}
