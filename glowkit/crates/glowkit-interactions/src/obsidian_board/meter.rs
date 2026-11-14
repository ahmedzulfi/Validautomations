//! A usage meter that warns as it fills up.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui};
use glowkit_core::color::{fade, mix};
use glowkit_core::{paint, Theme};

/// A quota meter: a label with "7.2 of 10 GB" (or a percentage) on the right, a slim rounded
/// bar that fills with a spring, and an optional hint line. The fill shifts from blue to amber
/// past the warning threshold and to red past the critical one, with a soft breathing glow at
/// the head while critical.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardMeter;
/// ui.add(BoardMeter::new("Storage", 7.2, 10.0).unit("GB"));
/// ui.add(BoardMeter::new("Seats", 19.0, 20.0).integer().hint("Add seats before inviting more teammates."));
/// ui.add(BoardMeter::new("API requests this month", 96.0, 100.0).percent());
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardMeter {
    label: String,
    value: f32,
    max: f32,
    unit: String,
    percent: bool,
    integer: bool,
    hint: Option<String>,
    warn: f32,
    critical: f32,
    width: f32,
    thickness: f32,
    colors: Option<[Color32; 3]>,
}

impl BoardMeter {
    /// A meter labelled `label` showing `value` of `max`.
    pub fn new(label: impl Into<String>, value: f32, max: f32) -> Self {
        Self {
            label: label.into(),
            value,
            max: max.max(1e-6),
            unit: String::new(),
            percent: false,
            integer: false,
            hint: None,
            warn: 0.8,
            critical: 0.96,
            width: 400.0,
            thickness: 5.0,
            colors: None,
        }
    }
    /// Unit appended to the max, e.g. `"GB"`.
    pub fn unit(mut self, u: impl Into<String>) -> Self {
        self.unit = u.into();
        self
    }
    /// Show the reading as a percentage instead of "x of y".
    pub fn percent(mut self) -> Self {
        self.percent = true;
        self
    }
    /// Show whole numbers ("19 of 20").
    pub fn integer(mut self) -> Self {
        self.integer = true;
        self
    }
    /// A small hint line under the bar.
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = Some(h.into());
        self
    }
    /// Fractions at which the meter turns amber and red (default 0.8 and 0.96).
    pub fn thresholds(mut self, warn: f32, critical: f32) -> Self {
        self.warn = warn.clamp(0.0, 1.0);
        self.critical = critical.clamp(self.warn, 1.0);
        self
    }
    /// Width in points (default 400).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(160.0);
        self
    }
    /// Bar thickness in points (default 5).
    pub fn thickness(mut self, h: f32) -> Self {
        self.thickness = h.clamp(2.0, 16.0);
        self
    }
    /// Normal, warning and critical colours.
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
}

impl egui::Widget for BoardMeter {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let h = 30.0 + self.thickness + if self.hint.is_some() { 22.0 } else { 6.0 };
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(12.0));
        let el = entry(ui, id);
        let frac_t = (self.value / self.max).clamp(0.0, 1.0);
        let frac = tween(ui, id.with("v"), frac_t, 0.0, 0.9) * ease_out(entry_k(ui, el, 0.1, 0.9));
        let [c_ok, c_warn, c_crit] = self.colors.unwrap_or([p[BLUE], p[YELLOW], p[RED]]);
        let lvl_t = if frac_t >= self.critical {
            2.0
        } else if frac_t >= self.warn {
            1.0
        } else {
            0.0
        };
        let lvl = glowkit_core::anim::follow(ui, id.with("lvl"), lvl_t, 6.0);
        let col = if lvl <= 1.0 { mix(c_ok, c_warn, lvl) } else { mix(c_warn, c_crit, lvl - 1.0) };
        text(&painter, pos2(rect.min.x, rect.min.y + 11.0), Align2::LEFT_CENTER, &self.label, f(14.0), t.foreground);
        let shown_v = frac / frac_t.max(1e-6) * self.value;
        let reading = if self.percent {
            format!("{:.0}%", frac * 100.0)
        } else if self.integer {
            format!(
                "{} of {}{}",
                shown_v.round(),
                self.max.round(),
                if self.unit.is_empty() { String::new() } else { format!(" {}", self.unit) }
            )
        } else {
            format!(
                "{:.1} of {}{}",
                shown_v,
                trim_num(self.max),
                if self.unit.is_empty() { String::new() } else { format!(" {}", self.unit) }
            )
        };
        text(&painter, pos2(rect.max.x, rect.min.y + 11.0), Align2::RIGHT_CENTER, reading, f(13.5), dim(&t));
        let bar = Rect::from_min_size(pos2(rect.min.x, rect.min.y + 26.0), vec2(rect.width(), self.thickness));
        let r = self.thickness / 2.0;
        painter.rect_filled(bar, cr(r), track(&t));
        if frac > 0.002 {
            let fr = Rect::from_min_size(bar.min, vec2((bar.width() * frac).max(self.thickness), self.thickness));
            if lvl > 1.4 {
                let breathe = 0.5 + 0.5 * (glowkit_core::time(ui) * 2.4).sin();
                paint::glow(
                    &painter,
                    pos2(fr.max.x - r, fr.center().y),
                    14.0 + 4.0 * breathe,
                    fade(col, 0.22 * (lvl - 1.4) / 0.6 * (0.6 + 0.4 * breathe)),
                );
                glowkit_core::animate(ui);
            }
            painter.rect_filled(fr, cr(r), col);
        }
        if let Some(hint) = &self.hint {
            text(&painter, pos2(rect.min.x, bar.max.y + 13.0), Align2::LEFT_CENTER, hint, f(12.0), faint(&t));
        }
        response
    }
}

fn trim_num(v: f32) -> String {
    if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}
