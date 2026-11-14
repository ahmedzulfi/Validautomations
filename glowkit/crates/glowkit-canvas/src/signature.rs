//! [`SignaturePad`]: smooth ink capture for signatures and quick sketches.

use crate::common::{icon_button, qfont, Icon};
use crate::freehand::{paint_ink, InkPoint, InkStyle};
use crate::json;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use glowkit_core::{color::with_alpha, Theme};

/// Captured ink: a list of strokes in pad-local coordinates (0,0 = top-left of the pad).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ink {
    pub strokes: Vec<Vec<InkPoint>>,
}

impl Ink {
    /// No strokes?
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }
    /// Total captured points.
    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(|s| s.len()).sum()
    }
    /// Remove everything.
    pub fn clear(&mut self) {
        self.strokes.clear();
    }
    /// Remove the last stroke.
    pub fn undo(&mut self) {
        self.strokes.pop();
    }
    /// Export as JSON: `[[[x, y, t], ...], ...]` (rounded to 0.01).
    pub fn to_json(&self) -> String {
        let strokes: Vec<String> = self
            .strokes
            .iter()
            .map(|s| {
                let pts: Vec<String> = s
                    .iter()
                    .map(|p| format!("[{},{},{}]", json::num(p.pos.x), json::num(p.pos.y), json::num(p.t)))
                    .collect();
                format!("[{}]", pts.join(","))
            })
            .collect();
        format!("[{}]", strokes.join(","))
    }
    /// Export as an SVG path `d` attribute (polyline per stroke).
    pub fn to_svg_path(&self) -> String {
        let mut out = String::new();
        for s in &self.strokes {
            for (i, p) in s.iter().enumerate() {
                out.push_str(&format!(
                    "{}{} {} ",
                    if i == 0 { "M" } else { "L" },
                    json::num(p.pos.x),
                    json::num(p.pos.y)
                ));
            }
        }
        out.trim_end().to_owned()
    }
    /// Bounding box of all points.
    pub fn bounds(&self) -> Rect {
        crate::common::bounds_of(self.strokes.iter().flatten().map(|p| p.pos))
    }
}

/// A signature / drawing pad: smooth variable-width ink (speed-based thinning, tapering ends),
/// a baseline with a "sign here" hint that fades once you start, and clear / undo buttons.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut ink = glowkit_canvas::Ink::default();
/// ui.add(glowkit_canvas::SignaturePad::new(&mut ink).size(egui::vec2(520.0, 200.0)));
/// let json = ink.to_json();
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SignaturePad<'a> {
    ink: &'a mut Ink,
    size: Vec2,
    color: Option<Color32>,
    style: InkStyle,
    placeholder: String,
    baseline: bool,
    background: Option<Color32>,
}

impl<'a> SignaturePad<'a> {
    /// A pad writing into `ink`.
    pub fn new(ink: &'a mut Ink) -> Self {
        Self {
            ink,
            size: vec2(520.0, 200.0),
            color: None,
            style: InkStyle { size: 3.6, thinning: 0.6, streamline: 0.5, taper_start: 1.5, taper_end: 3.5 },
            placeholder: "Sign here".into(),
            baseline: true,
            background: None,
        }
    }
    /// Pad size.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Ink color (defaults to the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Stroke shape.
    pub fn style(mut self, s: InkStyle) -> Self {
        self.style = s;
        self
    }
    /// Placeholder text shown while empty.
    pub fn placeholder(mut self, s: impl Into<String>) -> Self {
        self.placeholder = s.into();
        self
    }
    /// Show the signature baseline.
    pub fn baseline(mut self, on: bool) -> Self {
        self.baseline = on;
        self
    }
    /// Pad fill.
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
}

impl egui::Widget for SignaturePad<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut resp) = ui.allocate_exact_size(self.size, Sense::drag());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let now = glowkit_core::time(ui);
        let painter = ui.painter_at(rect.expand(2.0));
        let fill = self.background.unwrap_or(if t.dark { t.background } else { t.card });
        let hov = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered() || resp.dragged(), 0.15);
        painter.rect(
            rect,
            CornerRadius::same(12),
            fill,
            Stroke::new(1.0, t.border.lerp_to_gamma(t.accent, 0.5 * hov)),
            StrokeKind::Inside,
        );
        let ink_col = self.color.unwrap_or(t.foreground);
        // capture
        if let Some(p) = resp.interact_pointer_pos() {
            let local = (p - rect.min).to_pos2();
            if resp.drag_started() {
                self.ink.strokes.push(vec![InkPoint::new(local, now)]);
                resp.mark_changed();
            } else if resp.dragged() {
                if let Some(s) = self.ink.strokes.last_mut() {
                    if s.last().is_none_or(|l| l.pos.distance(local) > 0.7) {
                        s.push(InkPoint::new(local, now));
                        resp.mark_changed();
                    }
                }
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        // guide
        let base_y = rect.bottom() - rect.height() * 0.28;
        let empty_k = ui.ctx().animate_bool_with_time(id.with("empty"), self.ink.is_empty(), 0.35);
        if self.baseline {
            painter.line_segment(
                [pos2(rect.left() + 28.0, base_y), pos2(rect.right() - 28.0, base_y)],
                Stroke::new(1.0, with_alpha(t.muted_foreground, 0.45)),
            );
            painter.text(
                pos2(rect.left() + 30.0, base_y - 6.0),
                Align2::LEFT_BOTTOM,
                "×",
                qfont(18.0),
                with_alpha(t.muted_foreground, 0.7),
            );
        }
        if empty_k > 0.01 {
            let mut p = painter.clone();
            p.set_opacity(empty_k);
            p.text(
                pos2(rect.center().x, base_y + 18.0 - 4.0 * (1.0 - empty_k)),
                Align2::CENTER_CENTER,
                &self.placeholder,
                qfont(12.5),
                t.muted_foreground,
            );
        }
        // ink
        for s in &self.ink.strokes {
            paint_ink(&painter, s, &self.style, ink_col, |p| rect.min + p.to_vec2(), 1.0);
        }
        // buttons
        if !self.ink.is_empty() {
            let undo_r = Rect::from_min_size(pos2(rect.right() - 70.0, rect.top() + 8.0), Vec2::splat(28.0));
            let clear_r = Rect::from_min_size(pos2(rect.right() - 38.0, rect.top() + 8.0), Vec2::splat(28.0));
            if icon_button(ui, id.with("undo"), undo_r, Icon::Undo, false, "Undo stroke").clicked() {
                self.ink.undo();
                resp.mark_changed();
            }
            if icon_button(ui, id.with("clear"), clear_r, Icon::Trash, false, "Clear").clicked() {
                self.ink.clear();
                resp.mark_changed();
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_formats() {
        let mut ink = Ink::default();
        ink.strokes.push(vec![InkPoint::new(pos2(1.0, 2.0), 0.0), InkPoint::new(pos2(3.5, 4.0), 0.1)]);
        ink.strokes.push(vec![InkPoint::new(pos2(10.0, 10.0), 0.5)]);
        assert_eq!(ink.point_count(), 3);
        assert_eq!(ink.to_json(), "[[[1,2,0],[3.5,4,0.1]],[[10,10,0.5]]]");
        assert_eq!(ink.to_svg_path(), "M1 2 L3.5 4 M10 10");
        let parsed = crate::json::Json::parse(&ink.to_json()).unwrap();
        assert_eq!(parsed.arr().len(), 2);
        ink.undo();
        assert_eq!(ink.strokes.len(), 1);
    }
}
