//! Crate-private helpers shared by the audio / voice / approval components.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Format seconds as `m:ss` (or `h:mm:ss`).
pub(crate) fn fmt_clock(secs: f32) -> String {
    let s = secs.max(0.0).floor() as u32;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// Deterministic, speech-like waveform peaks in `0..=1` (`n` bars, shaped by `seed`).
pub(crate) fn waveform_peaks(seed: u32, n: usize) -> Vec<f32> {
    let s = seed as f32 * 7.31;
    (0..n)
        .map(|i| {
            let x = i as f32 / n.max(1) as f32;
            let words = glowkit_core::noise::noise2(x * 38.0, s) * 0.5 + 0.5;
            let phrase = glowkit_core::noise::noise2(x * 6.0, s + 11.0) * 0.5 + 0.5;
            let fine = glowkit_core::noise::noise2(x * 140.0, s + 3.0) * 0.5 + 0.5;
            let v = (words * 0.55 + fine * 0.25) * (0.35 + phrase * 0.9);
            let gap = if phrase < 0.28 { 0.25 } else { 1.0 };
            (v * gap).clamp(0.06, 1.0)
        })
        .collect()
}

/// Visual style of [`pill_button`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PillStyle {
    /// Solid primary fill.
    Primary,
    /// Hairline outline.
    Outline,
    /// No chrome until hovered.
    Ghost,
    /// Solid fill in a custom color.
    Solid(Color32),
}

/// A compact rounded button with optional leading icon. Returns the click response.
pub(crate) fn pill_button(ui: &mut Ui, label: &str, icon: Option<Icon>, style: PillStyle, height: f32) -> Response {
    let t = Theme::of(ui);
    let g = common::galley(ui, label, 13.0, t.foreground);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let w = g.size().x + 26.0 + icon_w;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let h = common::anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let press = if resp.is_pointer_button_down_on() { 0.97 } else { 1.0 };
        let rect = Rect::from_center_size(rect.center(), rect.size() * press);
        let cr = CornerRadius::same((t.radius * 0.8) as u8);
        let p = ui.painter();
        let (fill, fg, stroke) = match style {
            PillStyle::Primary => (mix(t.primary, t.primary_foreground, 0.1 * h), t.primary_foreground, Stroke::NONE),
            PillStyle::Solid(c) => {
                (mix(c, Color32::WHITE, 0.1 * h), glowkit_core::color::contrast_text(c), Stroke::NONE)
            }
            PillStyle::Outline => (
                mix(t.card, t.foreground, if t.dark { 0.06 } else { 0.04 } * h),
                t.foreground,
                Stroke::new(1.0, mix(t.border, t.muted_foreground, 0.3 * h)),
            ),
            PillStyle::Ghost => (common::hover_fill(&t).gamma_multiply(h), t.foreground, Stroke::NONE),
        };
        if !t.dark && matches!(style, PillStyle::Outline) {
            glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 2.0, vec2(0.0, 1.0), Color32::from_black_alpha(10));
        }
        p.rect(rect, cr, fill, stroke, StrokeKind::Inside);
        let mut x = rect.min.x + 13.0;
        if let Some(i) = icon {
            paint_icon(p, i, Rect::from_center_size(pos2(x + 7.0, rect.center().y), vec2(14.0, 14.0)), fg);
            x += icon_w;
        }
        p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(13.0), fg);
    }
    resp
}

/// A round play / pause button. Returns the click response.
pub(crate) fn play_button(ui: &mut Ui, playing: bool, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let h = common::anim_bool(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let press = if resp.is_pointer_button_down_on() { 0.92 } else { 1.0 + 0.04 * h };
        let r = size / 2.0 * press;
        let p = ui.painter();
        glowkit_core::paint::glow(p, rect.center(), r * 1.6, with_alpha(color, 0.18 + 0.12 * h));
        p.circle_filled(rect.center(), r, color);
        let fg = glowkit_core::color::contrast_text(color);
        let ir = Rect::from_center_size(rect.center(), vec2(r, r));
        if playing {
            let bw = r * 0.22;
            for dx in [-r * 0.2, r * 0.2] {
                let br = Rect::from_center_size(ir.center() + vec2(dx, 0.0), vec2(bw, r * 0.78));
                p.rect_filled(br, CornerRadius::same(2), fg);
            }
        } else {
            let c = ir.center() + vec2(r * 0.08, 0.0);
            let s = r * 0.42;
            p.add(egui::Shape::convex_polygon(
                vec![c + vec2(-s * 0.8, -s), c + vec2(s, 0.0), c + vec2(-s * 0.8, s)],
                fg,
                Stroke::NONE,
            ));
        }
    }
    resp
}

/// A floating popover frame with the house shadow.
pub(crate) fn popover_frame(t: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(t.popover)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner_scaled(1.1))
        .shadow(egui::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(if t.dark { 140 } else { 34 }),
        })
        .inner_margin(egui::Margin::same(6))
}

/// Paint a "mic off" slash across an icon rect.
pub(crate) fn slash(painter: &egui::Painter, rect: Rect, color: Color32, bg: Color32) {
    let a = rect.left_top() + vec2(rect.width() * 0.12, rect.height() * 0.1);
    let b = rect.right_bottom() - vec2(rect.width() * 0.12, rect.height() * 0.06);
    let w = (rect.width() / 24.0 * 1.9).clamp(1.1, 3.0);
    painter.line_segment([a, b], Stroke::new(w * 2.6, bg));
    painter.line_segment([a, b], Stroke::new(w, color));
}

/// Close a popup when clicking outside `inside` rects or pressing Escape. Returns the new open flag.
pub(crate) fn close_on_outside(ui: &Ui, open: bool, just_toggled: bool, inside: &[Rect]) -> bool {
    if !open {
        return false;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        return false;
    }
    if !just_toggled && ui.input(|i| i.pointer.any_pressed()) {
        if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
            if !inside.iter().any(|r| r.contains(p)) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_format() {
        assert_eq!(fmt_clock(0.0), "0:00");
        assert_eq!(fmt_clock(65.4), "1:05");
        assert_eq!(fmt_clock(3725.0), "1:02:05");
    }

    #[test]
    fn peaks_in_range() {
        let p = waveform_peaks(3, 120);
        assert_eq!(p.len(), 120);
        assert!(p.iter().all(|v| (0.0..=1.0).contains(v)));
    }
}
