use super::kit::{self, control_radius};
use egui::{
    pos2, vec2, Align, Color32, CursorIcon, FontId, Layout, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui,
    UiBuilder, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A one-time-code input: a row of boxes that fill as you type, auto-advancing with a blinking
/// caret, accepting paste, with optional group separator.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut code = String::new();
/// use glowkit_components::OtpInput;
/// if ui.add(OtpInput::new(&mut code).length(6).separator_after(3)).changed() && code.len() == 6 {
///     // verify
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct OtpInput<'a> {
    code: &'a mut String,
    length: usize,
    separator: Option<usize>,
    digits_only: bool,
    masked: bool,
    error: bool,
    box_size: egui::Vec2,
    joined: bool,
}

impl<'a> OtpInput<'a> {
    /// An OTP field editing `code`.
    pub fn new(code: &'a mut String) -> Self {
        Self {
            code,
            length: 6,
            separator: None,
            digits_only: true,
            masked: false,
            error: false,
            box_size: vec2(40.0, 44.0),
            joined: false,
        }
    }
    /// Number of characters (default 6).
    pub fn length(mut self, n: usize) -> Self {
        self.length = n.max(1);
        self
    }
    /// Draw a dash separator after this many boxes.
    pub fn separator_after(mut self, n: usize) -> Self {
        self.separator = Some(n);
        self
    }
    /// Accept only digits (default) or any alphanumeric character.
    pub fn digits_only(mut self, d: bool) -> Self {
        self.digits_only = d;
        self
    }
    /// Show dots instead of characters.
    pub fn masked(mut self, m: bool) -> Self {
        self.masked = m;
        self
    }
    /// Red error state (e.g. wrong code).
    pub fn error(mut self, e: bool) -> Self {
        self.error = e;
        self
    }
    /// Size of each box.
    pub fn box_size(mut self, s: egui::Vec2) -> Self {
        self.box_size = s;
        self
    }
    /// Join boxes into one segmented field (shared borders) instead of separate boxes.
    pub fn joined(mut self, j: bool) -> Self {
        self.joined = j;
        self
    }
}

impl Widget for OtpInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.length;
        let gap = if self.joined { -1.0 } else { 8.0 };
        let sep_w = if self.separator.is_some() { 28.0 } else { 0.0 };
        let total = self.box_size.x * n as f32 + gap * (n - 1) as f32 + sep_w;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(total, self.box_size.y), Sense::click());
        resp = resp.on_hover_cursor(CursorIcon::Text);
        let te_id = resp.id.with("otp_te");
        // a clipped-away TextEdit captures typing, paste and backspace
        let before = self.code.clone();
        let te_resp = {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(rect.min, vec2(1.0, 1.0)))
                    .layout(Layout::left_to_right(Align::Min)),
            );
            child.set_clip_rect(Rect::NOTHING);
            child.add(TextEdit::singleline(&mut *self.code).id(te_id).desired_width(1.0).char_limit(n * 2))
        };
        if resp.clicked() {
            te_resp.request_focus();
        }
        let filtered: String = self
            .code
            .chars()
            .filter(|c| if self.digits_only { c.is_ascii_digit() } else { c.is_alphanumeric() })
            .map(|c| c.to_ascii_uppercase())
            .take(n)
            .collect();
        if filtered != *self.code {
            *self.code = filtered;
            // keep the cursor at the end after filtering
            if let Some(mut s) = TextEdit::load_state(ui.ctx(), te_id) {
                let end = egui::text::CCursor::new(self.code.chars().count());
                s.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                TextEdit::store_state(ui.ctx(), te_id, s);
            }
        }
        if *self.code != before {
            resp.mark_changed();
        }
        let focused = te_resp.has_focus();
        let chars: Vec<char> = self.code.chars().collect();
        let active = chars.len().min(n - 1);
        let time = glowkit_core::time(ui);
        let r = control_radius(&t);
        let painter = ui.painter();
        let mut x = rect.min.x;
        for i in 0..n {
            if Some(i) == self.separator {
                let c = pos2(x + sep_w / 2.0 - gap / 2.0, rect.center().y);
                painter.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], Stroke::new(1.6, t.muted_foreground));
                x += sep_w;
            }
            let br = Rect::from_min_size(pos2(x, rect.min.y), self.box_size);
            let filled = i < chars.len();
            let pop = kit::anim(ui, resp.id.with(("pop", i)), filled, 0.18);
            let is_active = focused && i == active;
            let act = kit::anim(ui, resp.id.with(("act", i)), is_active, 0.14);
            let corner = if self.joined {
                let rr = r.nw;
                egui::CornerRadius {
                    nw: if i == 0 { rr } else { 0 },
                    sw: if i == 0 { rr } else { 0 },
                    ne: if i + 1 == n { rr } else { 0 },
                    se: if i + 1 == n { rr } else { 0 },
                }
            } else {
                r
            };
            let accent = if self.error { t.destructive } else { t.ring };
            let base = if self.error { with_alpha(t.destructive, 0.7) } else { t.input };
            if !t.dark && !self.joined {
                glowkit_core::paint::soft_shadow(
                    painter,
                    br,
                    r.nw as f32,
                    2.0,
                    vec2(0.0, 1.0),
                    Color32::from_black_alpha(10),
                );
            }
            painter.rect(br, corner, kit::field_fill(&t), Stroke::new(1.0, base), StrokeKind::Inside);
            if act > 0.0 {
                painter.rect_stroke(br, corner, Stroke::new(1.0, with_alpha(accent, act)), StrokeKind::Inside);
                painter.rect_stroke(
                    br,
                    corner,
                    Stroke::new(1.0 + 2.0 * act, with_alpha(accent, 0.3 * act)),
                    StrokeKind::Outside,
                );
            }
            if filled {
                let s = 1.0 + 0.18 * (pop * std::f32::consts::PI).sin();
                let ch = chars[i];
                if self.masked {
                    painter.circle_filled(br.center(), 4.5 * s, t.foreground);
                } else {
                    painter.text(
                        br.center(),
                        egui::Align2::CENTER_CENTER,
                        ch,
                        FontId::proportional(18.0 * s),
                        t.foreground,
                    );
                }
            } else if is_active && (time * 2.0).fract() < 0.55 {
                let c = br.center();
                painter.line_segment([c - vec2(0.0, 9.0), c + vec2(0.0, 9.0)], Stroke::new(1.5, t.foreground));
            }
            x += self.box_size.x + gap;
        }
        if focused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        resp
    }
}
