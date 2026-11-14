use super::common::{card, cr, hairline, hover_fill, icon, idle_for, morph, surface, well, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Closed,
    Open,
    Sent,
}

#[derive(Clone, Copy, Debug)]
struct FbState {
    phase: Phase,
    since: f64,
}

/// A floating feedback button that becomes the form. The round chat button in the corner of a
/// page springs open into a panel (position, size and corners all morph from the button), with a
/// message box and Cancel / Submit; submitting shrinks the panel into a "thanks" pill with a
/// drawn check, which then folds back into the button.
///
/// [`FeedbackWidget::show`] returns the message on the frame it is submitted.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut draft = String::new();
/// if let Some(msg) = glowkit_interactions::FeedbackWidget::new(&mut draft).show(ui) {
///     println!("feedback: {msg}");
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FeedbackWidget<'a> {
    draft: &'a mut String,
    title: String,
    size: Option<Vec2>,
    page: bool,
    autoplay: bool,
}

impl<'a> FeedbackWidget<'a> {
    /// A widget editing the message in `draft`.
    pub fn new(draft: &'a mut String) -> Self {
        Self { draft, title: "Help us improve".into(), size: None, page: true, autoplay: true }
    }
    /// Panel title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Size of the area the widget lives in (default 460 x 320).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Paint a placeholder page behind the button (default on).
    pub fn page(mut self, on: bool) -> Self {
        self.page = on;
        self
    }
    /// Open, type, send and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the widget. Returns the message when it is submitted.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(vec2(460.0, 320.0));
        let (area, resp) = ui.allocate_exact_size(size, Sense::click());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let m = 16.0;
        let btn = Rect::from_center_size(area.max - vec2(m + 22.0, m + 22.0), vec2(44.0, 44.0));
        let panel = Rect::from_min_max(pos2(area.max.x - m - 300.0, area.max.y - m - 196.0), area.max - vec2(m, m));
        let pill = Rect::from_min_max(pos2(area.max.x - m - 236.0, area.max.y - m - 44.0), area.max - vec2(m, m));
        let st0: FbState = glowkit_core::state(ui, id, || FbState { phase: Phase::Closed, since: now - 10.0 });
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let idle = idle_for(ui, id, ptr.is_some_and(|p| area.contains(p)) || !self.draft.is_empty());
        let auto = self.autoplay && idle > 2.0;
        let close_c = pos2(panel.max.x - 22.0, panel.min.y + 22.0);
        let cancel = Rect::from_min_size(pos2(panel.min.x + 12.0, panel.max.y - 48.0), vec2(134.0, 36.0));
        let submit = Rect::from_min_size(pos2(panel.max.x - 146.0, panel.max.y - 48.0), vec2(134.0, 36.0));
        let text_r = Rect::from_min_max(pos2(panel.min.x + 12.0, panel.min.y + 44.0), pos2(panel.max.x - 12.0, panel.max.y - 58.0));
        let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        let mut sent = None;
        let mut next = st0;
        if let Some(p) = click {
            match st0.phase {
                Phase::Closed if btn.contains(p) => {
                    next = FbState { phase: Phase::Open, since: now };
                }
                Phase::Open => {
                    if close_c.distance(p) < 12.0 || cancel.contains(p) {
                        next = FbState { phase: Phase::Closed, since: now };
                    } else if submit.contains(p) && !self.draft.trim().is_empty() {
                        sent = Some(std::mem::take(self.draft));
                        next = FbState { phase: Phase::Sent, since: now };
                    }
                }
                _ => {}
            }
        }
        let held = (now - next.since) as f32;
        // autoplay script: open, type, send
        let demo_text = "Love the new motion! Could the sheet close on swipe too?";
        let mut ghost_text = None;
        if auto {
            glowkit_core::animate(ui);
            match next.phase {
                Phase::Closed if held > 1.3 => next = FbState { phase: Phase::Open, since: now },
                Phase::Open => {
                    let n = (((held - 0.5) / 0.045).max(0.0) as usize).min(demo_text.len());
                    ghost_text = Some(&demo_text[..n]);
                    if held > 0.5 + demo_text.len() as f32 * 0.045 + 0.8 {
                        next = FbState { phase: Phase::Sent, since: now };
                    }
                }
                _ => {}
            }
        }
        if next.phase == Phase::Sent && (now - next.since) > 1.8 {
            next = FbState { phase: Phase::Closed, since: now };
        }
        glowkit_core::set_state(ui, id, next);
        let st = next;
        if st.phase != Phase::Closed || (now - st.since) < 1.0 {
            glowkit_core::animate(ui);
        }
        let (target, rad) = match st.phase {
            Phase::Closed => (btn, 22.0),
            Phase::Open => (panel, 22.0),
            Phase::Sent => (pill, 22.0),
        };
        let (rect, r) = morph(ui, id.with("m"), target, rad, 340.0, 25.0);
        if !ui.is_rect_visible(area) {
            return sent;
        }
        // placeholder page
        if self.page {
            let pg = area;
            painter.rect(pg, cr(22.0), surface(&t), hairline(&t), egui::StrokeKind::Inside);
            let bar = |x: f32, y: f32, w: f32, h: f32| {
                painter.rect_filled(Rect::from_min_size(pg.min + vec2(x, y), vec2(w, h)), cr(h / 2.0), well(&t));
            };
            bar(20.0, 18.0, 96.0, 10.0);
            painter.line_segment([pos2(pg.min.x, pg.min.y + 46.0), pos2(pg.max.x, pg.min.y + 46.0)], hairline(&t));
            bar(20.0, 66.0, 300.0, 10.0);
            bar(20.0, 88.0, 200.0, 10.0);
            painter.rect_filled(Rect::from_min_size(pg.min + vec2(20.0, 112.0), vec2(pg.width() - 40.0, 92.0)), cr(14.0), well(&t));
            bar(20.0, 222.0, 270.0, 10.0);
        }
        let k_open = ((rect.width() - btn.width()) / (panel.width() - btn.width())).clamp(0.0, 1.0);
        card(&painter, &t, rect, r.min(rect.height() / 2.0), 0.6 + 0.6 * k_open);
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        match st.phase {
            Phase::Closed => {
                let a = (1.0 - k_open * 2.0).clamp(0.0, 1.0);
                let hov = ptr.is_some_and(|p| btn.contains(p));
                if hov {
                    clip.circle_filled(rect.center(), rect.height() / 2.0 - 2.0, hover_fill(&t));
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                icon(&clip, BeIcon::Chat, rect.center(), 17.0, with_alpha(t.foreground, a), 1.5);
            }
            Phase::Open => {
                let a = ((k_open - 0.5) / 0.5).clamp(0.0, 1.0);
                let o = rect.min - panel.min;
                clip.text(pos2(panel.min.x + 16.0, panel.min.y + 22.0) + o, Align2::LEFT_CENTER, &self.title, FontId::proportional(14.0), with_alpha(t.foreground, a));
                let cc = close_c + o;
                clip.circle_filled(cc, 11.0, with_alpha(hover_fill(&t), a));
                icon(&clip, BeIcon::Close, cc, 8.0, with_alpha(t.muted_foreground, a), 1.3);
                clip.rect_filled(text_r.translate(o), cr(14.0), with_alpha(well(&t), a));
                let has_text;
                if let Some(g) = ghost_text {
                    has_text = !g.is_empty();
                    let shown = if g.is_empty() { "Share an idea or report a bug" } else { g };
                    let col = if g.is_empty() { t.muted_foreground } else { t.foreground };
                    let gal = clip.layout(shown.to_string(), FontId::proportional(13.0), with_alpha(col, a), text_r.width() - 20.0);
                    clip.galley(text_r.min + o + vec2(10.0, 10.0), gal, col);
                } else if a > 0.9 {
                    let te = egui::TextEdit::multiline(self.draft)
                        .frame(egui::Frame::NONE)
                        .hint_text(egui::RichText::new("Share an idea or report a bug").color(t.muted_foreground))
                        .text_color(t.foreground)
                        .font(FontId::proportional(13.0))
                        .desired_rows(4);
                    ui.put(text_r.shrink(8.0), te);
                    has_text = !self.draft.trim().is_empty();
                } else {
                    has_text = !self.draft.trim().is_empty();
                }
                let hov_c = ptr.is_some_and(|p| cancel.contains(p));
                clip.rect(cancel.translate(o), cr(18.0), with_alpha(if hov_c { hover_fill(&t) } else { well(&t) }, a), Stroke::NONE, egui::StrokeKind::Inside);
                clip.text(cancel.center() + o, Align2::CENTER_CENTER, "Cancel", FontId::proportional(13.0), with_alpha(t.foreground, a));
                let ready = if has_text { 1.0 } else { 0.45 };
                let hov_s = ptr.is_some_and(|p| submit.contains(p)) && has_text;
                clip.rect_filled(submit.translate(o), cr(18.0), with_alpha(if hov_s { mix(t.primary, t.primary_foreground, 0.12) } else { t.primary }, a * ready));
                clip.text(submit.center() + o, Align2::CENTER_CENTER, "Submit", FontId::proportional(13.0), with_alpha(t.primary_foreground, a));
                if ptr.is_some_and(|p| cancel.contains(p) || (submit.contains(p) && has_text) || close_c.distance(p) < 12.0) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
            Phase::Sent => {
                let since = (now - st.since) as f32;
                let a = ((since - 0.15) / 0.2).clamp(0.0, 1.0);
                let c = pos2(rect.min.x + 24.0, rect.center().y);
                clip.circle_filled(c, 10.0, with_alpha(t.success, a));
                // the check draws itself
                let p = ((since - 0.25) / 0.35).clamp(0.0, 1.0);
                let pts = [c + vec2(-4.5, 0.2), c + vec2(-1.2, 3.4), c + vec2(4.6, -3.2)];
                let l1 = (p * 2.0).min(1.0);
                let l2 = (p * 2.0 - 1.0).clamp(0.0, 1.0);
                let stc = Stroke::new(1.8, egui::Color32::WHITE);
                if l1 > 0.0 {
                    clip.line_segment([pts[0], pts[0] + (pts[1] - pts[0]) * l1], stc);
                }
                if l2 > 0.0 {
                    clip.line_segment([pts[1], pts[1] + (pts[2] - pts[1]) * l2], stc);
                }
                clip.text(pos2(rect.min.x + 44.0, rect.center().y), Align2::LEFT_CENTER, "Thanks for the feedback!", FontId::proportional(13.0), with_alpha(t.foreground, a));
            }
        }
        sent
    }
}
