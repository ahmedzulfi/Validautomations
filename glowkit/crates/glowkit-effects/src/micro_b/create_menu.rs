use super::common::{cr, hairline, icon, idle_for, ink, shadow_color, surface, surface_hover, text_width, MicroIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::{spring, stagger};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug)]
struct MenuState {
    open: bool,
    since: f64,
}

/// A "+ Create" pill that spreads into the menu it was all along. Press it and the pill swells
/// into a card (width and height ride separate springs, so it pours open rather than scaling),
/// the label melts away and the items rise in one after another. Pick an item or press outside
/// and it pulls back into the pill.
///
/// [`CreateMenu::show`] returns the index of the item picked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CreateMenu, MicroIcon};
/// let picked = CreateMenu::new("Create")
///     .item(MicroIcon::Doc, "Document")
///     .item(MicroIcon::Folder, "Folder")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CreateMenu {
    label: String,
    items: Vec<(MicroIcon, String)>,
    width: f32,
    corner: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl CreateMenu {
    /// A pill labelled `label` with no items yet.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), items: Vec::new(), width: 200.0, corner: 20.0, dark: false, stroke: true, autoplay: true }
    }
    /// Add a menu item.
    pub fn item(mut self, icon: MicroIcon, label: impl Into<String>) -> Self {
        self.items.push((icon, label.into()));
        self
    }
    /// Width of the open menu.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Corner radius of the open menu (the pill is always round).
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Dark (ink) surface.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the pill / menu. Returns the index of the item picked this frame, if any.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let row_h = 38.0;
        let pad = 7.0;
        let n = self.items.len();
        let open_size = vec2(self.width, pad * 2.0 + row_h * n as f32);
        let painter = ui.painter().clone();
        let pill_size = vec2(text_width(&painter, &self.label, 14.0) + 52.0, 38.0);
        let area = open_size.max(pill_size);
        let (outer, resp) = ui.allocate_exact_size(area, Sense::click());
        let id = resp.id;
        let center = outer.center();
        let now = ui.input(|i| i.time);
        let st0: MenuState = glowkit_core::state(ui, id, || MenuState { open: false, since: now });
        let card_now = Rect::from_center_size(center, if st0.open { open_size } else { pill_size });
        let hovering = ui.rect_contains_pointer(card_now);
        let idle = idle_for(ui, id, ui.rect_contains_pointer(outer));
        let pill_click = !st0.open && resp.clicked() && card_now.contains(resp.interact_pointer_pos().unwrap_or(center));
        // which row is under the pointer
        let rows_top = center.y - open_size.y / 2.0 + pad;
        let row_at = |y: f32| -> Option<usize> {
            let i = ((y - rows_top) / row_h).floor();
            (i >= 0.0 && (i as usize) < n).then_some(i as usize)
        };
        let ptr_row = ui.input(|i| i.pointer.hover_pos()).filter(|p| card_now.contains(*p)).and_then(|p| row_at(p.y));
        let mut picked = None;
        if st0.open && resp.clicked() {
            picked = resp.interact_pointer_pos().filter(|p| card_now.contains(*p)).and_then(|p| row_at(p.y));
        }
        let outside_click = st0.open && ui.input(|i| i.pointer.any_click()) && !hovering && !resp.clicked();
        let auto = self.autoplay && idle > 2.0;
        let time = glowkit_core::time(ui);
        let st = glowkit_core::with_state(ui, id, || st0, |s| {
            let close = picked.is_some() || outside_click || (st0.open && resp.clicked() && picked.is_none() && !hovering);
            if pill_click {
                s.open = true;
                s.since = now;
            } else if close && s.open {
                s.open = false;
                s.since = now;
            }
            if auto {
                let held = now - s.since;
                if s.open && held > 2.8 {
                    s.open = false;
                    s.since = now;
                } else if !s.open && held > 1.6 {
                    s.open = true;
                    s.since = now;
                }
            }
            *s
        });
        if auto {
            glowkit_core::animate(ui);
        }
        let open = st.open;
        // width pours open first, height follows; closing reverses the order
        let w = spring(ui, id.with("w"), if open { open_size.x } else { pill_size.x }, if open { 340.0 } else { 260.0 }, if open { 19.0 } else { 24.0 });
        let h = spring(ui, id.with("h"), if open { open_size.y } else { pill_size.y }, if open { 200.0 } else { 380.0 }, if open { 17.0 } else { 26.0 });
        let r = spring(ui, id.with("r"), if open { self.corner } else { pill_size.y / 2.0 }, 260.0, 24.0);
        let k = ((h - pill_size.y) / (open_size.y - pill_size.y).max(1.0)).clamp(0.0, 1.2);
        let rect = Rect::from_center_size(center, vec2(w.max(10.0), h.max(10.0)));
        if !ui.is_rect_visible(outer) {
            return picked;
        }
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, fg, sub) =
            if self.dark { (ink_bg, ink_fg, with_alpha(ink_fg, 0.6)) } else { (surface(&t), t.foreground, t.muted_foreground) };
        let elev = 0.5 + 0.5 * k.min(1.0);
        paint::soft_shadow(&painter, rect, r.min(rect.height() / 2.0), 12.0 + 16.0 * elev, vec2(0.0, 3.0 + 6.0 * elev), shadow_color(&t, elev));
        painter.rect(rect, cr(r.min(rect.height() / 2.0)), bg, if self.stroke { hairline(&t) } else { Stroke::NONE }, egui::StrokeKind::Inside);
        let clip = painter.with_clip_rect(rect.shrink(1.0));
        // pill label melts away as the card grows
        let pa = (1.0 - k * 3.0).clamp(0.0, 1.0);
        if pa > 0.01 {
            let lw = text_width(&painter, &self.label, 14.0);
            let x0 = center.x - (lw + 22.0) / 2.0;
            let rot = k * 4.0;
            let ic = pos2(x0 + 7.0, center.y);
            let s = 6.0;
            let (sn, cs) = rot.sin_cos();
            for d in [vec2(1.0, 0.0), vec2(0.0, 1.0)] {
                let v = vec2(d.x * cs - d.y * sn, d.x * sn + d.y * cs) * s;
                clip.line_segment([ic - v, ic + v], Stroke::new(1.6, with_alpha(fg, pa)));
            }
            clip.text(pos2(x0 + 22.0, center.y), Align2::LEFT_CENTER, &self.label, FontId::proportional(14.0), with_alpha(fg, pa));
        }
        // items stagger in from slightly below
        let since = (now - st.since) as f32;
        let hover_target = if open { ptr_row.map(|i| i as f32).unwrap_or(-1.0) } else { -1.0 };
        let hover_target = if auto && open && ptr_row.is_none() {
            ((time * 1.3) as usize % n.max(1)) as f32
        } else {
            hover_target
        };
        let hy = spring(ui, id.with("hy"), hover_target.max(0.0), 420.0, 30.0);
        let ha = spring(ui, id.with("ha"), if hover_target >= 0.0 { 1.0 } else { 0.0 }, 300.0, 26.0);
        if open && ha > 0.01 {
            let hr = Rect::from_min_size(pos2(rect.min.x + pad, rows_top + hy * row_h), vec2(rect.width() - pad * 2.0, row_h));
            clip.rect_filled(hr, cr((self.corner - pad).max(6.0)), with_alpha(if self.dark { ink_fg } else { surface_hover(&t) }, if self.dark { 0.1 * ha } else { ha }));
        }
        for (i, (ic, label)) in self.items.iter().enumerate() {
            let e = if open { stagger(since - 0.06, i, 0.045, 0.28) } else { 1.0 - stagger(since, n - 1 - i, 0.02, 0.12) };
            let a = e * k.min(1.0);
            if a <= 0.01 {
                continue;
            }
            let y = rows_top + row_h * (i as f32 + 0.5) + (1.0 - e) * 10.0;
            let x = rect.min.x + pad + 14.0;
            icon(&clip, *ic, pos2(x + 7.0, y), 14.0, with_alpha(sub, a), 1.4);
            clip.text(pos2(x + 26.0, y), Align2::LEFT_CENTER, label, FontId::proportional(14.0), with_alpha(fg, a));
        }
        if ptr_row.is_some() && open || (!open && hovering) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        picked
    }
}
