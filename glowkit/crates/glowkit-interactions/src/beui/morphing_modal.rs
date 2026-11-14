use super::common::{card, cr, hairline, hover_fill, icon, idle_for, morph, text_width, well, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct ModalState {
    open: bool,
    /// `None` = the option list, `Some(i)` = the detail page of option `i`.
    page: Option<usize>,
    since: f64,
    page_since: f64,
}

struct ModalOption {
    icon: BeIcon,
    label: String,
    title: String,
    body: String,
    confirm: String,
    danger: bool,
}

/// A button that becomes its own modal. Press it and its box springs into a floating sheet (the
/// position, size and corner radius all morph, the label crossfading into the sheet's content);
/// pick a row and the sheet re-measures, its height easing to fit the new page while the old
/// content slides out and the new one slides in. Close it and it shrinks back into the button.
///
/// [`MorphingModal::show`] returns the option confirmed this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, MorphingModal};
/// let done = MorphingModal::new("Open wallet options")
///     .option(BeIcon::Eye, "View private key", "Private key", "Never share it.", "Reveal", false)
///     .option(BeIcon::Trash, "Remove wallet", "Remove wallet?", "You can re-import it later.", "Remove", true)
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MorphingModal {
    trigger: String,
    title: String,
    options: Vec<ModalOption>,
    size: Option<Vec2>,
    width: f32,
    autoplay: bool,
}

impl MorphingModal {
    /// A trigger button labelled `trigger`.
    pub fn new(trigger: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            title: "Options".into(),
            options: Vec::new(),
            size: None,
            width: 340.0,
            autoplay: true,
        }
    }
    /// Heading of the option list (default "Options").
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Add a row; picking it opens a page with `title`, `body` and a `confirm` button
    /// (red when `danger`).
    pub fn option(
        mut self,
        icon: BeIcon,
        label: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        confirm: impl Into<String>,
        danger: bool,
    ) -> Self {
        self.options.push(ModalOption {
            icon,
            label: label.into(),
            title: title.into(),
            body: body.into(),
            confirm: confirm.into(),
            danger,
        });
        self
    }
    /// Size of the stage the sheet floats in (default: available width x 340).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Width of the open sheet.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Walk through itself while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the trigger / sheet. Returns the option confirmed this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 340.0);
        let (area, resp) = ui.allocate_exact_size(size, Sense::click());
        let id = resp.id;
        let painter = ui.painter_at(area);
        let now = ui.input(|i| i.time);
        let st0: ModalState =
            glowkit_core::state(ui, id, || ModalState { open: false, page: None, since: now, page_since: now });
        let n = self.options.len();
        let pad = 18.0;
        let row_h = 44.0;
        let w = self.width.min(area.width() - 24.0);
        // page heights
        let list_h = 56.0 + n as f32 * (row_h + 4.0) + pad - 4.0;
        let body_w = w - pad * 2.0;
        let detail_h = |i: usize| -> f32 {
            let g = painter.layout(self.options[i].body.clone(), FontId::proportional(13.0), t.foreground, body_w);
            pad + 52.0 + 30.0 + g.size().y + 22.0 + 40.0 + pad
        };
        let page_h = match st0.page {
            Some(i) if i < n => detail_h(i),
            _ => list_h,
        };
        let trig_w = text_width(&painter, &self.trigger, 14.0) + 40.0;
        let trig = Rect::from_center_size(area.center() - vec2(0.0, 10.0), vec2(trig_w, 40.0));
        let sheet = Rect::from_min_size(pos2(area.center().x - w / 2.0, area.max.y - 14.0 - page_h), vec2(w, page_h));
        // interaction
        let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| area.contains(*p));
        let idle = idle_for(ui, id, ptr.is_some());
        let auto = self.autoplay && idle > 2.5;
        let close_btn = Rect::from_center_size(pos2(sheet.max.x - 24.0, sheet.min.y + 26.0), vec2(26.0, 26.0));
        let row_rect = |i: usize| {
            Rect::from_min_size(
                pos2(sheet.min.x + 10.0, sheet.min.y + 52.0 + i as f32 * (row_h + 4.0)),
                vec2(w - 20.0, row_h),
            )
        };
        let btn_y = sheet.max.y - pad - 40.0;
        let half = (w - pad * 2.0 - 10.0) / 2.0;
        let cancel_r = Rect::from_min_size(pos2(sheet.min.x + pad, btn_y), vec2(half, 40.0));
        let confirm_r = Rect::from_min_size(pos2(sheet.min.x + pad + half + 10.0, btn_y), vec2(half, 40.0));
        let mut confirmed = None;
        let mut next = st0;
        if let Some(p) = click {
            if !st0.open {
                if trig.contains(p) {
                    next.open = true;
                    next.page = None;
                    next.since = now;
                }
            } else if !sheet.contains(p) || close_btn.contains(p) {
                next.open = false;
                next.since = now;
            } else if let Some(pg) = st0.page {
                if cancel_r.contains(p) {
                    next.page = None;
                    next.page_since = now;
                } else if confirm_r.contains(p) {
                    confirmed = Some(pg);
                    next.open = false;
                    next.since = now;
                }
            } else if let Some(i) = (0..n).find(|&i| row_rect(i).contains(p)) {
                next.page = Some(i);
                next.page_since = now;
            }
        }
        if auto {
            glowkit_core::animate(ui);
            let held = now - next.since.max(next.page_since);
            if !next.open && held > 1.4 {
                next.open = true;
                next.page = None;
                next.since = now;
                next.page_since = now;
            } else if next.open && next.page.is_none() && held > 1.5 {
                next.page = Some(((now / 9.0) as usize) % n.max(1));
                next.page_since = now;
            } else if next.open && next.page.is_some() && held > 2.2 {
                next.open = false;
                next.page = None;
                next.since = now;
                next.page_since = now;
            }
        }
        glowkit_core::set_state(ui, id, next);
        let st = next;
        let ghost_row = if auto && st.open && st.page.is_none() && now - st.page_since > 0.8 {
            Some(((now / 9.0) as usize) % n.max(1))
        } else {
            None
        };
        let (rect, radius) =
            morph(ui, id.with("m"), if st.open { sheet } else { trig }, if st.open { 26.0 } else { 20.0 }, 300.0, 25.0);
        let k = ((rect.height() - 40.0) / (sheet.height() - 40.0).max(1.0)).clamp(0.0, 1.0);
        if ptr.is_some_and(|p| (st.open && sheet.contains(p)) || (!st.open && trig.contains(p))) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(area) {
            return confirmed;
        }
        // stage: hint text and a dimming veil
        painter.text(
            trig.center() + vec2(0.0, 40.0),
            Align2::CENTER_CENTER,
            "Pick a row: the sheet morphs its height to fit.",
            FontId::proportional(12.0),
            with_alpha(t.muted_foreground, 1.0 - k),
        );
        card(&painter, &t, rect, radius, 0.4 + 0.8 * k);
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        // trigger label fades as the box opens
        let la = (1.0 - k * 3.0).clamp(0.0, 1.0);
        if la > 0.0 {
            clip.text(
                rect.center(),
                Align2::CENTER_CENTER,
                &self.trigger,
                FontId::proportional(14.0),
                with_alpha(t.foreground, la),
            );
        }
        let ca = ((k - 0.35) / 0.65).clamp(0.0, 1.0);
        if ca <= 0.0 {
            return confirmed;
        }
        // page transition: 0 = list, 1 = detail
        let pk = spring(ui, id.with("page"), if st.page.is_some() { 1.0 } else { 0.0 }, 260.0, 26.0);
        let shown_page = glowkit_core::with_state(
            ui,
            id.with("shown"),
            || 0usize,
            |v| {
                if let Some(p) = st.page {
                    *v = p;
                }
                *v
            },
        );
        let top = rect.min.y;
        let x0 = rect.min.x;
        // close button
        let ccol = with_alpha(t.muted_foreground, ca);
        let cc = pos2(rect.max.x - 24.0, top + 26.0);
        if ptr.is_some_and(|p| close_btn.contains(p)) {
            clip.circle_filled(cc, 13.0, hover_fill(&t));
        }
        icon(&clip, BeIcon::Close, cc, 10.0, ccol, 1.4);
        // list page slides left as the detail page arrives
        let la = ca * (1.0 - pk * 2.0).clamp(0.0, 1.0);
        if la > 0.01 {
            let dx = -pk * 24.0;
            clip.text(
                pos2(x0 + pad + dx, top + 26.0),
                Align2::LEFT_CENTER,
                &self.title,
                FontId::proportional(15.0),
                with_alpha(t.foreground, la),
            );
            let hov = ptr.and_then(|p| (0..n).find(|&i| row_rect(i).contains(p))).or(ghost_row);
            for (i, o) in self.options.iter().enumerate() {
                let r = row_rect(i).translate(vec2(dx, rect.min.y - sheet.min.y));
                let fill = if hov == Some(i) { hover_fill(&t) } else { well(&t) };
                clip.rect_filled(r, cr(14.0), with_alpha(fill, la));
                let col = if o.danger { t.destructive } else { t.foreground };
                icon(&clip, o.icon, pos2(r.min.x + 22.0, r.center().y), 14.0, with_alpha(col, la), 1.4);
                clip.text(
                    pos2(r.min.x + 42.0, r.center().y),
                    Align2::LEFT_CENTER,
                    &o.label,
                    FontId::proportional(13.5),
                    with_alpha(col, la),
                );
                icon(
                    &clip,
                    BeIcon::ChevronRight,
                    pos2(r.max.x - 18.0, r.center().y),
                    10.0,
                    with_alpha(t.muted_foreground, la),
                    1.3,
                );
            }
        }
        let da = ca * (pk * 2.0 - 1.0).clamp(0.0, 1.0);
        if da > 0.01 && shown_page < n {
            let o = &self.options[shown_page];
            let dx = (1.0 - pk) * 24.0;
            let accent = if o.danger { t.destructive } else { t.accent };
            let ic = pos2(x0 + pad + 22.0 + dx, top + pad + 22.0);
            clip.circle_filled(ic, 22.0, with_alpha(accent, 0.14 * da));
            icon(&clip, o.icon, ic, 18.0, with_alpha(accent, da), 1.6);
            clip.text(
                pos2(x0 + pad + dx, top + pad + 66.0),
                Align2::LEFT_CENTER,
                &o.title,
                FontId::proportional(16.0),
                with_alpha(t.foreground, da),
            );
            let g = clip.layout(o.body.clone(), FontId::proportional(13.0), with_alpha(t.muted_foreground, da), body_w);
            clip.galley(pos2(x0 + pad + dx, top + pad + 82.0), g, t.muted_foreground);
            let by = rect.max.y - pad - 40.0;
            let cr1 = Rect::from_min_size(pos2(x0 + pad + dx, by), vec2(half, 40.0));
            let cr2 = Rect::from_min_size(pos2(x0 + pad + half + 10.0 + dx, by), vec2(half, 40.0));
            let hov_c = ptr.is_some_and(|p| cancel_r.contains(p));
            clip.rect(
                cr1,
                cr(20.0),
                with_alpha(if hov_c { hover_fill(&t) } else { well(&t) }, da),
                Stroke::new(1.0, with_alpha(hairline(&t).color, da)),
                egui::StrokeKind::Inside,
            );
            clip.text(
                cr1.center(),
                Align2::CENTER_CENTER,
                "Back",
                FontId::proportional(13.5),
                with_alpha(t.foreground, da),
            );
            let (bf, bt) = if o.danger { (t.destructive, Color32::WHITE) } else { (t.primary, t.primary_foreground) };
            let hov_k = ptr.is_some_and(|p| confirm_r.contains(p));
            clip.rect_filled(cr2, cr(20.0), with_alpha(if hov_k { mix(bf, bt, 0.1) } else { bf }, da));
            clip.text(cr2.center(), Align2::CENTER_CENTER, &o.confirm, FontId::proportional(13.5), with_alpha(bt, da));
        }
        confirmed
    }
}
