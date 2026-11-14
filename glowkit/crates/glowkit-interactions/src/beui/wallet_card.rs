use super::common::{card, cr, hover_fill, icon, idle_for, money, orb, text_width, well, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{Rng, Theme};

/// Paint `text` as an odometer: every digit rolls vertically to its value on its own spring
/// (digits further right are a touch quicker). Returns the painted width.
pub(crate) fn odometer(ui: &Ui, painter: &Painter, id: egui::Id, text: &str, center: Pos2, size: f32, color: Color32) -> f32 {
    let font = FontId::proportional(size);
    let dw = text_width(painter, "0", size);
    let widths: Vec<f32> = text.chars().map(|c| if c.is_ascii_digit() { dw } else { text_width(painter, &c.to_string(), size) }).collect();
    let target_w: f32 = widths.iter().sum();
    let w = spring(ui, id.with("odo_w"), target_w, 300.0, 26.0);
    let mut x = center.x + w / 2.0;
    let h = size * 1.25;
    let clip = painter.with_clip_rect(Rect::from_center_size(center, vec2(w + size, h)).intersect(painter.clip_rect()));
    // walk right to left so positions are anchored at the decimal end
    for (k, (ch, cw)) in text.chars().rev().zip(widths.iter().rev()).enumerate() {
        x -= cw;
        let cx = x + cw / 2.0;
        if let Some(d) = ch.to_digit(10) {
            let v = spring(ui, id.with(("odo", k)), d as f32, 260.0 - k.min(10) as f32 * 14.0, 22.0);
            let base = v.floor();
            let frac = v - base;
            for (j, dy) in [(0.0f32, 0.0f32), (1.0, 1.0)] {
                let digit = ((base + j) as i32).rem_euclid(10);
                let y = center.y + (dy - frac) * h;
                let a = 1.0 - ((y - center.y).abs() / h).clamp(0.0, 1.0);
                if a > 0.01 {
                    clip.text(pos2(cx, y), Align2::CENTER_CENTER, digit.to_string(), font.clone(), with_alpha(color, a));
                }
            }
        } else {
            clip.text(pos2(cx, center.y), Align2::CENTER_CENTER, ch.to_string(), font.clone(), color);
        }
    }
    w
}

/// A crypto wallet card: account switcher, a balance that rolls like an odometer when it
/// changes (each digit spinning to its new value), an eye toggle that folds the figure into
/// dots, a gain / loss pill, and four round actions that squash when pressed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut balance = 12_480.32;
/// ui.add(glowkit_interactions::WalletCard::new(&mut balance).name("Main Wallet"));
/// ```
pub struct WalletCard<'a> {
    balance: &'a mut f64,
    name: String,
    width: f32,
    autoplay: bool,
}

#[derive(Clone, Copy, Debug)]
struct WalletState {
    prev: f64,
    delta: f64,
    hidden: bool,
    last_auto: f64,
    seed: u64,
}

impl<'a> WalletCard<'a> {
    /// A card showing (and, in autoplay, nudging) `balance`.
    pub fn new(balance: &'a mut f64) -> Self {
        Self { balance, name: "Main Wallet".into(), width: 320.0, autoplay: true }
    }
    /// Account name in the header.
    pub fn name(mut self, s: impl Into<String>) -> Self {
        self.name = s.into();
        self
    }
    /// Card width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(260.0, 480.0);
        self
    }
    /// Change the balance now and then while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for WalletCard<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let w = self.width;
        let h = 316.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let idle = idle_for(ui, id, resp.hovered());
        let auto = self.autoplay && idle > 2.0;
        let cur = *self.balance;
        let eye_c = pos2(rect.center().x + 26.0, rect.min.y + 100.0);
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        let eye_clicked = click.is_some_and(|p| p.distance(eye_c) < 12.0);
        let mut new_balance = None;
        let st = glowkit_core::with_state(
            ui,
            id,
            || WalletState { prev: cur, delta: 124.50, hidden: false, last_auto: now, seed: 3 },
            |s| {
                if (cur - s.prev).abs() > 0.004 {
                    s.delta = cur - s.prev;
                    s.prev = cur;
                }
                if eye_clicked {
                    s.hidden = !s.hidden;
                }
                if auto && now - s.last_auto > 2.2 {
                    s.last_auto = now;
                    s.seed += 1;
                    let mut rng = Rng::new(s.seed * 7717);
                    let d = (rng.range(-900.0, 1400.0) * 100.0).round() / 100.0;
                    new_balance = Some((cur + d as f64).max(0.0));
                }
                *s
            },
        );
        if let Some(b) = new_balance {
            *self.balance = b;
        }
        if auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        card(&painter, &t, rect, 28.0, 0.7);
        let pad = 22.0;
        // header
        let hy = rect.min.y + 40.0;
        orb(&painter, pos2(rect.min.x + pad + 13.0, hy), 13.0, [hex("#5EEAD4"), hex("#A7F3D0")]);
        let nr = painter.text(pos2(rect.min.x + pad + 34.0, hy), Align2::LEFT_CENTER, &self.name, FontId::proportional(14.5), t.foreground);
        icon(&painter, BeIcon::ChevronDown, pos2(nr.max.x + 12.0, hy), 9.0, t.muted_foreground, 1.4);
        icon(&painter, BeIcon::Search, pos2(rect.max.x - pad - 44.0, hy), 14.0, t.muted_foreground, 1.4);
        let bell = pos2(rect.max.x - pad - 8.0, hy);
        icon(&painter, BeIcon::Bell, bell, 14.0, t.muted_foreground, 1.4);
        let pulse = ((now * 2.0).sin() * 0.5 + 0.5) as f32;
        painter.circle_filled(bell + vec2(5.0, -6.0), 3.5 + pulse * 0.6, t.info);
        // balance label + eye
        let lr = painter.text(pos2(rect.center().x - 6.0, rect.min.y + 100.0), Align2::CENTER_CENTER, "Balance", FontId::proportional(12.0), t.muted_foreground);
        let eye_c = pos2(lr.max.x + 12.0, lr.center().y);
        let hov_eye = ptr.is_some_and(|p| p.distance(eye_c) < 12.0);
        if hov_eye {
            painter.circle_filled(eye_c, 11.0, hover_fill(&t));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        icon(&painter, if st.hidden { BeIcon::EyeOff } else { BeIcon::Eye }, eye_c, 12.0, t.muted_foreground, 1.2);
        // the figure: odometer, or dots when hidden
        let hide = spring(ui, id.with("hide"), if st.hidden { 1.0 } else { 0.0 }, 300.0, 24.0);
        let fig_c = pos2(rect.center().x, rect.min.y + 134.0);
        let text = format!("${}", money(*self.balance));
        if hide < 0.98 {
            let a = 1.0 - hide;
            odometer(ui, &painter, id.with("odo"), &text, fig_c + vec2(0.0, -hide * 8.0), 34.0, with_alpha(t.foreground, a));
        }
        if hide > 0.02 {
            let n = 6;
            for i in 0..n {
                let x = fig_c.x + (i as f32 - (n - 1) as f32 / 2.0) * 22.0 * hide;
                painter.circle_filled(pos2(x, fig_c.y + (1.0 - hide) * 8.0), 6.0 * hide, with_alpha(t.foreground, hide));
            }
        }
        // delta pill
        let up = st.delta >= 0.0;
        let col = if up { t.success } else { t.destructive };
        let dtext = format!("{}${}", if up { "+" } else { "-" }, money(st.delta.abs()));
        let pw = text_width(&painter, &dtext, 12.5) + 38.0;
        let pw = spring(ui, id.with("pw"), pw, 300.0, 24.0);
        let pill = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 176.0), vec2(pw, 26.0));
        painter.rect_filled(pill, cr(13.0), with_alpha(col, 0.14));
        let tilt = spring(ui, id.with("tilt"), if up { 0.0 } else { 1.0 }, 260.0, 18.0);
        let ic = pos2(pill.min.x + 16.0, pill.center().y);
        // trend arrow flips with a spring
        let s = 5.0;
        let dy = 1.0 - 2.0 * tilt;
        let a = ic + vec2(-s, s * dy);
        let b = ic + vec2(s, -s * dy);
        painter.line_segment([a, b], egui::Stroke::new(1.5, col));
        painter.line_segment([b, b + vec2(-4.0, 0.0)], egui::Stroke::new(1.5, col));
        painter.line_segment([b, b + vec2(0.0, 4.0 * dy)], egui::Stroke::new(1.5, col));
        painter.text(pos2(pill.min.x + 28.0, pill.center().y), Align2::LEFT_CENTER, dtext, FontId::proportional(12.5), col);
        // actions
        let acts = [(BeIcon::ArrowUp, "Send"), (BeIcon::Download, "Deposit"), (BeIcon::Swap, "Swap"), (BeIcon::Card, "Buy")];
        let step = (w - pad * 2.0) / 4.0;
        for (i, (ic, label)) in acts.iter().enumerate() {
            let c = pos2(rect.min.x + pad + step * (i as f32 + 0.5), rect.max.y - 66.0);
            let r = Rect::from_center_size(c, vec2(48.0, 48.0));
            let ar = ui.interact(r, id.with(("act", i)), Sense::click());
            let press = spring(ui, id.with(("press", i)), if ar.is_pointer_button_down_on() { 1.0 } else if ar.hovered() { 0.4 } else { 0.0 }, 500.0, 18.0);
            if ar.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let rad = 24.0 - press * 2.5;
            painter.circle_filled(c, rad, mix(well(&t), hover_fill(&t), press.clamp(0.0, 1.0)));
            icon(&painter, *ic, c, 16.0, t.foreground, 1.5);
            painter.text(c + vec2(0.0, 38.0), Align2::CENTER_CENTER, *label, FontId::proportional(12.0), t.muted_foreground);
        }
        resp
    }
}
