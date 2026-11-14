use egui::{pos2, vec2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::paint;
use glowkit_core::Theme;
use std::f32::consts::{PI, TAU};

/// A linear progress bar: determinate (value eases to its target), indeterminate (sliding bar),
/// optionally striped or gradient-filled, with an optional label row.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Progress;
/// ui.add(Progress::new(0.66).label("Uploading").show_percent(true));
/// ui.add(Progress::indeterminate());
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Progress {
    value: Option<f32>,
    height: f32,
    width: Option<f32>,
    color: Option<Color32>,
    gradient: bool,
    striped: bool,
    label: Option<String>,
    percent: bool,
}

impl Progress {
    /// Determinate progress, `value` in `0..=1`.
    pub fn new(value: f32) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            height: 8.0,
            width: None,
            color: None,
            gradient: false,
            striped: false,
            label: None,
            percent: false,
        }
    }
    /// Unknown duration: a bar slides back and forth.
    pub fn indeterminate() -> Self {
        Self { value: None, ..Self::new(0.0) }
    }
    /// Bar thickness in points (default 8).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Width (default: available, at most 420).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Fill color (default theme primary).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fill with the theme gradient.
    pub fn gradient(mut self, g: bool) -> Self {
        self.gradient = g;
        self
    }
    /// Animated diagonal stripes on the fill.
    pub fn striped(mut self, s: bool) -> Self {
        self.striped = s;
        self
    }
    /// Label above the bar.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Show the percentage above the bar on the right.
    pub fn show_percent(mut self, p: bool) -> Self {
        self.percent = p;
        self
    }
}

impl Widget for Progress {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(420.0));
        let head = if self.label.is_some() || self.percent { 22.0 } else { 0.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, head + self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let track = Rect::from_min_size(pos2(rect.min.x, rect.min.y + head), vec2(w, self.height));
        let r = CornerRadius::same((self.height / 2.0).min(127.0) as u8);
        let track_col = if t.dark { t.secondary } else { mix(t.muted, t.input, 0.5) };
        painter.rect_filled(track, r, track_col);
        let col = self.color.unwrap_or(t.primary);
        let fill_painter = |painter: &egui::Painter, fr: Rect| {
            if fr.width() <= 0.5 {
                return;
            }
            if self.gradient {
                paint::gradient_rect(painter, fr, self.height / 2.0, 0.0, &t.gradient);
            } else {
                painter.rect_filled(fr, r, col);
            }
            if self.striped {
                let p = painter.with_clip_rect(fr.intersect(painter.clip_rect()));
                let off = (glowkit_core::time(ui) * 18.0) % 16.0;
                let mut x = fr.min.x - 16.0 - self.height + off;
                let stripe = if t.dark && !self.gradient && self.color.is_none() {
                    Color32::from_black_alpha(40)
                } else {
                    Color32::from_white_alpha(45)
                };
                while x < fr.max.x + self.height {
                    p.add(egui::Shape::convex_polygon(
                        vec![
                            pos2(x, fr.max.y),
                            pos2(x + 8.0, fr.max.y),
                            pos2(x + 8.0 + self.height, fr.min.y),
                            pos2(x + self.height, fr.min.y),
                        ],
                        stripe,
                        Stroke::NONE,
                    ));
                    x += 16.0;
                }
                glowkit_core::animate(ui);
            }
        };
        match self.value {
            Some(v) => {
                let shown = glowkit_core::anim::follow(ui, resp.id.with("v"), v, 6.0);
                let fr = Rect::from_min_size(
                    track.min,
                    vec2((w * shown).max(if shown > 0.0 { self.height } else { 0.0 }), self.height),
                );
                fill_painter(painter, fr);
                if self.percent {
                    painter.text(
                        pos2(rect.max.x, rect.min.y + 8.0),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.0}%", shown * 100.0),
                        FontId::proportional(13.0),
                        t.muted_foreground,
                    );
                }
            }
            None => {
                let time = glowkit_core::time(ui);
                let ph = (time / 1.5).fract();
                let e = glowkit_core::ease(glowkit_core::Easing::InOutSine, ph);
                let len = 0.3 + 0.12 * (ph * std::f32::consts::PI).sin();
                let x0 = track.min.x + (e * (1.0 + len) - len) * w;
                let x1 = x0 + len * w;
                let fr =
                    Rect::from_min_max(pos2(x0.max(track.min.x), track.min.y), pos2(x1.min(track.max.x), track.max.y));
                if fr.width() > 0.0 {
                    fill_painter(&painter.with_clip_rect(track), fr);
                }
                glowkit_core::animate(ui);
            }
        }
        if let Some(l) = &self.label {
            painter.text(
                pos2(rect.min.x, rect.min.y + 8.0),
                egui::Align2::LEFT_CENTER,
                l,
                FontId::proportional(13.5),
                t.foreground,
            );
        }
        resp
    }
}

/// A circular progress ring with a centered label; supports gradients and an indeterminate spin.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::ProgressRing;
/// ui.add(ProgressRing::new(0.72).size(96.0).gradient(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ProgressRing {
    value: Option<f32>,
    size: f32,
    thickness: f32,
    color: Option<Color32>,
    gradient: bool,
    label: Option<String>,
    sublabel: Option<String>,
    show_label: bool,
}

/// Alias matching shadcn naming.
pub type CircularProgress = ProgressRing;

impl ProgressRing {
    /// Determinate ring, `value` in `0..=1`.
    pub fn new(value: f32) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            size: 72.0,
            thickness: 7.0,
            color: None,
            gradient: false,
            label: None,
            sublabel: None,
            show_label: true,
        }
    }
    /// Spinning ring for unknown durations.
    pub fn indeterminate() -> Self {
        Self { value: None, show_label: false, ..Self::new(0.0) }
    }
    /// Diameter in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Stroke thickness.
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t;
        self
    }
    /// Arc color (default theme primary).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Use the theme gradient along the arc.
    pub fn gradient(mut self, g: bool) -> Self {
        self.gradient = g;
        self
    }
    /// Replace the percentage with custom center text.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Small muted line under the center text.
    pub fn sublabel(mut self, l: impl Into<String>) -> Self {
        self.sublabel = Some(l.into());
        self
    }
    /// Show the center text (default on).
    pub fn show_label(mut self, s: bool) -> Self {
        self.show_label = s;
        self
    }
}

impl Widget for ProgressRing {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let c = rect.center();
        let w = self.thickness;
        let r = self.size / 2.0 - w / 2.0;
        let track = if t.dark { t.secondary } else { mix(t.muted, t.input, 0.5) };
        painter.circle_stroke(c, r, Stroke::new(w, track));
        let col = self.color.unwrap_or(t.primary);
        let start = -PI / 2.0;
        match self.value {
            Some(v) => {
                let shown = glowkit_core::anim::follow(ui, resp.id.with("v"), v, 5.0);
                if shown > 0.002 {
                    let a1 = start + TAU * shown;
                    if self.gradient {
                        let g = t.gradient;
                        paint::arc_gradient(painter, c, r, start, a1, w, |u| {
                            glowkit_core::color::gradient(&g, u * shown)
                        });
                        painter.circle_filled(paint_pt(c, r, start), w / 2.0, g[0]);
                        painter.circle_filled(paint_pt(c, r, a1), w / 2.0, glowkit_core::color::gradient(&g, shown));
                    } else {
                        paint::arc(painter, c, r, start, a1, w, col);
                    }
                }
                if self.show_label {
                    let main = self.label.clone().unwrap_or_else(|| format!("{:.0}%", shown * 100.0));
                    let fs = (self.size * 0.22).clamp(11.0, 30.0);
                    let dy = if self.sublabel.is_some() { -fs * 0.3 } else { 0.0 };
                    painter.text(
                        c + vec2(0.0, dy),
                        egui::Align2::CENTER_CENTER,
                        main,
                        FontId::proportional(fs),
                        t.foreground,
                    );
                    if let Some(s) = &self.sublabel {
                        painter.text(
                            c + vec2(0.0, fs * 0.65),
                            egui::Align2::CENTER_CENTER,
                            s,
                            FontId::proportional((fs * 0.48).max(10.0)),
                            t.muted_foreground,
                        );
                    }
                }
            }
            None => {
                let time = glowkit_core::time(ui);
                let a0 = start + time * TAU;
                let len = 0.15 + 0.2 * (0.5 - 0.5 * (time * 2.5).cos());
                if self.gradient {
                    let g = t.gradient;
                    paint::arc_gradient(painter, c, r, a0, a0 + TAU * len, w, |u| {
                        with_alpha(glowkit_core::color::gradient(&g, u), u.max(0.2))
                    });
                } else {
                    paint::arc(painter, c, r, a0, a0 + TAU * len, w, col);
                }
                glowkit_core::animate(ui);
            }
        }
        resp
    }
}

fn paint_pt(c: egui::Pos2, r: f32, a: f32) -> egui::Pos2 {
    c + vec2(a.cos(), a.sin()) * r
}

/// A horizontal stepper: numbered circles joined by connectors that fill as steps complete.
/// Completed steps show a check; the current one pulses softly.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::StepProgress;
/// ui.add(StepProgress::new(&["Account", "Profile", "Billing", "Done"], 1));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StepProgress {
    steps: Vec<String>,
    current: usize,
    width: Option<f32>,
    descriptions: Vec<String>,
}

impl StepProgress {
    /// Steps by label; `current` is the index of the active step (== len means all done).
    pub fn new(steps: &[&str], current: usize) -> Self {
        Self { steps: steps.iter().map(|s| s.to_string()).collect(), current, width: None, descriptions: Vec::new() }
    }
    /// Muted second line for each step.
    pub fn descriptions(mut self, d: &[&str]) -> Self {
        self.descriptions = d.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Total width (default: available, at most 640).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for StepProgress {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.steps.len().max(1);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(640.0));
        let h = 32.0 + 26.0 + if self.descriptions.is_empty() { 0.0 } else { 16.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let seg = w / n as f32;
        let cy = rect.min.y + 16.0;
        let prog = glowkit_core::anim::follow(ui, resp.id.with("p"), self.current as f32, 5.0);
        let painter = ui.painter();
        let track = if t.dark { t.secondary } else { mix(t.muted, t.input, 0.6) };
        // connectors
        for i in 0..n.saturating_sub(1) {
            let x0 = rect.min.x + seg * (i as f32 + 0.5) + 20.0;
            let x1 = rect.min.x + seg * (i as f32 + 1.5) - 20.0;
            painter.line_segment([pos2(x0, cy), pos2(x1, cy)], Stroke::new(2.0, track));
            let f = (prog - i as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                painter.line_segment([pos2(x0, cy), pos2(x0 + (x1 - x0) * f, cy)], Stroke::new(2.0, t.primary));
            }
        }
        let time = glowkit_core::time(ui);
        let mut pulse = false;
        for i in 0..n {
            let c = pos2(rect.min.x + seg * (i as f32 + 0.5), cy);
            let is_current = i == self.current;
            if is_current {
                let ph = (time * 0.8).fract();
                painter.circle_filled(c, 15.0 + 8.0 * ph, with_alpha(t.ring, 0.25 * (1.0 - ph)));
                pulse = true;
            }
            if i < self.current {
                painter.circle_filled(c, 15.0, t.primary);
                let k = (prog - i as f32 - 0.0).clamp(0.0, 1.0);
                crate::inputs::kit::check_path(
                    painter,
                    Rect::from_center_size(c, vec2(16.0, 16.0)),
                    t.primary_foreground,
                    2.0,
                    k,
                );
            } else if is_current {
                painter.circle(c, 15.0, t.background, Stroke::new(2.0, t.primary));
                painter.text(
                    c,
                    egui::Align2::CENTER_CENTER,
                    (i + 1).to_string(),
                    FontId::proportional(13.0),
                    t.foreground,
                );
            } else {
                painter.circle(c, 15.0, t.background, Stroke::new(1.5, track));
                painter.text(
                    c,
                    egui::Align2::CENTER_CENTER,
                    (i + 1).to_string(),
                    FontId::proportional(13.0),
                    t.muted_foreground,
                );
            }
            let lc = if i <= self.current { t.foreground } else { t.muted_foreground };
            painter.text(
                pos2(c.x, cy + 30.0),
                egui::Align2::CENTER_CENTER,
                &self.steps[i],
                FontId::proportional(13.5),
                lc,
            );
            if let Some(d) = self.descriptions.get(i) {
                painter.text(
                    pos2(c.x, cy + 47.0),
                    egui::Align2::CENTER_CENTER,
                    d,
                    FontId::proportional(12.0),
                    t.muted_foreground,
                );
            }
        }
        if pulse {
            glowkit_core::animate(ui);
        }
        resp
    }
}
