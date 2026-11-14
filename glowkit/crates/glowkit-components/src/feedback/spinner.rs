use egui::{pos2, vec2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::paint;
use glowkit_core::Theme;
use std::f32::consts::{PI, TAU};

/// The loader styles available in [`Spinner`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerKind {
    /// A quarter arc chasing around a faint track.
    #[default]
    Ring,
    /// Material-style arc that grows and shrinks while rotating.
    Arc,
    /// A comet-tailed arc in the theme gradient.
    Gradient,
    /// Two concentric arcs spinning in opposite directions.
    DualRing,
    /// Eight dots around a circle with a fading tail.
    Dots,
    /// Twelve radial bars (the classic iOS activity indicator).
    Bars,
    /// Two discs that grow and fade out, staggered.
    Pulse,
    /// Two satellites orbiting a core.
    Orbit,
    /// Three dots bouncing in sequence.
    Bounce,
    /// Five vertical bars rising like an equalizer.
    Wave,
    /// A 3×3 grid of cells scaling in a diagonal wave.
    Grid,
    /// A comet racing around an infinity sign.
    Infinity,
    /// Concentric rings rippling outward.
    Ripple,
    /// A clock face with fast-spinning hands.
    Clock,
    /// Chat "typing" dots that lift and brighten.
    Typing,
    /// A square flipping in 3D on alternating axes.
    SquareFlip,
    /// Two strands of dots twisting like a DNA helix.
    Helix,
    /// Dots spiralling into the center.
    Spiral,
}

impl SpinnerKind {
    /// Every kind, handy for galleries.
    pub const ALL: [SpinnerKind; 18] = [
        SpinnerKind::Ring,
        SpinnerKind::Arc,
        SpinnerKind::Gradient,
        SpinnerKind::DualRing,
        SpinnerKind::Dots,
        SpinnerKind::Bars,
        SpinnerKind::Pulse,
        SpinnerKind::Orbit,
        SpinnerKind::Bounce,
        SpinnerKind::Wave,
        SpinnerKind::Grid,
        SpinnerKind::Infinity,
        SpinnerKind::Ripple,
        SpinnerKind::Clock,
        SpinnerKind::Typing,
        SpinnerKind::SquareFlip,
        SpinnerKind::Helix,
        SpinnerKind::Spiral,
    ];

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            SpinnerKind::Ring => "Ring",
            SpinnerKind::Arc => "Arc",
            SpinnerKind::Gradient => "Gradient",
            SpinnerKind::DualRing => "Dual ring",
            SpinnerKind::Dots => "Dots",
            SpinnerKind::Bars => "Bars",
            SpinnerKind::Pulse => "Pulse",
            SpinnerKind::Orbit => "Orbit",
            SpinnerKind::Bounce => "Bounce",
            SpinnerKind::Wave => "Wave",
            SpinnerKind::Grid => "Grid",
            SpinnerKind::Infinity => "Infinity",
            SpinnerKind::Ripple => "Ripple",
            SpinnerKind::Clock => "Clock",
            SpinnerKind::Typing => "Typing",
            SpinnerKind::SquareFlip => "Square flip",
            SpinnerKind::Helix => "Helix",
            SpinnerKind::Spiral => "Spiral",
        }
    }
}

/// An animated loading indicator. Pick a style with [`SpinnerKind`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Spinner, SpinnerKind};
/// ui.add(Spinner::new(SpinnerKind::Orbit).size(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Spinner {
    kind: SpinnerKind,
    size: f32,
    color: Option<Color32>,
    speed: f32,
}

impl Spinner {
    /// A spinner of the given kind (24 pt, theme foreground).
    pub fn new(kind: SpinnerKind) -> Self {
        Self { kind, size: 24.0, color: None, speed: 1.0 }
    }
    /// Size of the square it occupies, in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Color (default theme foreground; gradient kinds use the theme gradient).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Speed multiplier (1 = default).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.max(0.05);
        self
    }

    /// Paint the spinner into `rect` (no layout).
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let t = Theme::of(ui);
        let col = self.color.unwrap_or(t.foreground);
        let time = glowkit_core::time(ui) * self.speed;
        paint_kind(ui.painter(), self.kind, rect, col, time, &t, self.color.is_some());
        glowkit_core::animate(ui);
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect);
        }
        resp
    }
}

fn polar(c: Pos2, r: f32, a: f32) -> Pos2 {
    c + vec2(a.cos(), a.sin()) * r
}

fn paint_kind(p: &egui::Painter, kind: SpinnerKind, rect: Rect, col: Color32, time: f32, t: &Theme, custom: bool) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let r = s / 2.0;
    match kind {
        SpinnerKind::Ring => {
            let w = (s * 0.1).max(2.0);
            p.circle_stroke(c, r - w / 2.0, Stroke::new(w, with_alpha(col, 0.18)));
            let a0 = time * TAU * 1.2;
            paint::arc(p, c, r - w / 2.0, a0, a0 + TAU * 0.27, w, col);
        }
        SpinnerKind::Arc => {
            let w = (s * 0.1).max(2.0);
            let cycle = (time / 1.4).fract();
            let grow = glowkit_core::ease(glowkit_core::Easing::InOutCubic, (cycle * 2.0).min(1.0));
            let shrink = glowkit_core::ease(glowkit_core::Easing::InOutCubic, (cycle * 2.0 - 1.0).max(0.0));
            let head = grow * 0.75;
            let tail = shrink * 0.75;
            let base = time * TAU * 0.8 + (time / 1.4).floor() * TAU * 0.75;
            let a0 = base + tail * TAU;
            let a1 = base + (head + 0.1) * TAU;
            paint::arc(p, c, r - w / 2.0, a0, a1, w, col);
        }
        SpinnerKind::Gradient => {
            let w = (s * 0.12).max(2.5);
            let stops = if custom { vec![col, col] } else { t.gradient.to_vec() };
            let a0 = time * TAU * 1.1;
            paint::arc_gradient(p, c, r - w / 2.0, a0, a0 + TAU * 0.8, w, |u| {
                with_alpha(glowkit_core::color::gradient(&stops, u), u.powf(1.3))
            });
            p.circle_filled(polar(c, r - w / 2.0, a0 + TAU * 0.8), w / 2.0, *stops.last().unwrap());
        }
        SpinnerKind::DualRing => {
            let w = (s * 0.08).max(1.8);
            let a = time * TAU * 1.1;
            paint::arc(p, c, r - w / 2.0, a, a + TAU * 0.3, w, col);
            paint::arc(p, c, r - w / 2.0, a + PI, a + PI + TAU * 0.3, w, col);
            let b = -time * TAU * 1.6;
            let ri = r * 0.55;
            paint::arc(p, c, ri, b, b + TAU * 0.25, w, with_alpha(col, 0.6));
            paint::arc(p, c, ri, b + PI, b + PI + TAU * 0.25, w, with_alpha(col, 0.6));
        }
        SpinnerKind::Dots => {
            let n = 8;
            let head = (time * 1.2).fract() * n as f32;
            for i in 0..n {
                let a = i as f32 / n as f32 * TAU - PI / 2.0;
                let d = (head - i as f32).rem_euclid(n as f32) / n as f32;
                let k = 1.0 - d;
                p.circle_filled(polar(c, r * 0.76, a), r * (0.09 + 0.07 * k), with_alpha(col, 0.15 + 0.85 * k * k));
            }
        }
        SpinnerKind::Bars => {
            let n = 12;
            let head = (time * 1.1).fract() * n as f32;
            let w = (s * 0.075).max(1.5);
            for i in 0..n {
                let a = i as f32 / n as f32 * TAU - PI / 2.0;
                let d = (head - i as f32).rem_euclid(n as f32) / n as f32;
                let k = 1.0 - d;
                let a0 = polar(c, r * 0.48, a);
                let a1 = polar(c, r * 0.92, a);
                p.line_segment([a0, a1], Stroke::new(w, with_alpha(col, 0.12 + 0.88 * k)));
                p.circle_filled(a0, w / 2.0, with_alpha(col, 0.12 + 0.88 * k));
                p.circle_filled(a1, w / 2.0, with_alpha(col, 0.12 + 0.88 * k));
            }
        }
        SpinnerKind::Pulse => {
            for i in 0..2 {
                let ph = (time * 0.9 + i as f32 * 0.5).fract();
                let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, ph);
                p.circle_filled(c, r * k, with_alpha(col, 0.75 * (1.0 - ph)));
            }
        }
        SpinnerKind::Orbit => {
            p.circle_stroke(c, r * 0.72, Stroke::new(1.0, with_alpha(col, 0.18)));
            p.circle_filled(c, r * 0.2, col);
            let a = time * TAU * 0.9;
            p.circle_filled(polar(c, r * 0.72, a), r * 0.14, col);
            p.circle_filled(polar(c, r * 0.72, a + PI), r * 0.1, with_alpha(col, 0.6));
            let b = -time * TAU * 0.55;
            p.circle_filled(polar(c, r * 0.95, b), r * 0.07, with_alpha(col, 0.5));
        }
        SpinnerKind::Bounce => {
            let dr = r * 0.2;
            for i in 0..3 {
                let ph = (time * 1.3 - i as f32 * 0.14).rem_euclid(1.0);
                let up = (ph * PI * 2.0).sin().max(0.0);
                let x = c.x + (i as f32 - 1.0) * r * 0.62;
                let y = c.y + r * 0.3 - up * r * 0.6;
                let squash = if up < 0.08 { 1.15 } else { 1.0 };
                p.add(Shape::ellipse_filled(pos2(x, y), vec2(dr * squash, dr / squash), col));
            }
        }
        SpinnerKind::Wave => {
            let n = 5;
            let bw = s / (n as f32 * 1.8);
            for i in 0..n {
                let ph = (time * 1.4 - i as f32 * 0.11).rem_euclid(1.0);
                let k = 0.35 + 0.65 * (0.5 - 0.5 * (ph * TAU).cos());
                let x = c.x + (i as f32 - (n - 1) as f32 / 2.0) * bw * 1.8;
                let h = s * 0.85 * k;
                p.rect_filled(
                    Rect::from_center_size(pos2(x, c.y), vec2(bw, h)),
                    CornerRadius::same((bw / 2.0) as u8),
                    col,
                );
            }
        }
        SpinnerKind::Grid => {
            let cell = s / 3.0;
            for y in 0..3 {
                for x in 0..3 {
                    let delay = (x + 2 - y) as f32 * 0.1;
                    let ph = (time * 0.85 - delay).rem_euclid(1.0);
                    let k = if ph < 0.35 {
                        1.0 - glowkit_core::ease(glowkit_core::Easing::InOutQuad, ph / 0.35)
                    } else if ph < 0.7 {
                        glowkit_core::ease(glowkit_core::Easing::InOutQuad, (ph - 0.35) / 0.35)
                    } else {
                        1.0
                    };
                    let center = pos2(rect.min.x + cell * (x as f32 + 0.5), rect.min.y + cell * (y as f32 + 0.5));
                    let sz = cell * 0.82 * k;
                    if sz > 0.5 {
                        p.rect_filled(
                            Rect::from_center_size(center, vec2(sz, sz)),
                            CornerRadius::same((sz * 0.2) as u8),
                            with_alpha(col, 0.35 + 0.65 * k),
                        );
                    }
                }
            }
        }
        SpinnerKind::Infinity => {
            let n = 96;
            let pts: Vec<Pos2> = (0..n)
                .map(|i| {
                    let u = i as f32 / n as f32 * TAU;
                    let d = 1.0 + u.sin().powi(2);
                    pos2(c.x + r * 0.95 * u.cos() / d, c.y + r * 0.95 * u.sin() * u.cos() / d)
                })
                .collect();
            let w = (s * 0.09).max(1.8);
            paint::gradient_polyline(p, &pts, true, w, |_| with_alpha(col, 0.15));
            let head = (time * 0.55).fract();
            let colr = if custom { col } else { t.gradient[0] };
            paint::gradient_polyline(p, &pts, true, w, paint::comet_fn(head, 0.4, colr));
        }
        SpinnerKind::Ripple => {
            let w = (s * 0.06).max(1.5);
            for i in 0..3 {
                let ph = (time * 0.7 + i as f32 / 3.0).fract();
                let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, ph);
                p.circle_stroke(c, (r - w) * k, Stroke::new(w, with_alpha(col, 1.0 - ph)));
            }
        }
        SpinnerKind::Clock => {
            let w = (s * 0.08).max(1.6);
            p.circle_stroke(c, r - w / 2.0, Stroke::new(w, col));
            let m = time * TAU * 1.2 - PI / 2.0;
            let h = time * TAU * 0.1 - PI / 2.0;
            p.line_segment([c, polar(c, r * 0.62, m)], Stroke::new(w, col));
            p.line_segment([c, polar(c, r * 0.42, h)], Stroke::new(w, col));
            p.circle_filled(c, w * 0.8, col);
        }
        SpinnerKind::Typing => {
            let dr = r * 0.17;
            for i in 0..3 {
                let ph = (time * 1.1 - i as f32 * 0.16).rem_euclid(1.0);
                let k = (ph * TAU).sin().max(0.0);
                let x = c.x + (i as f32 - 1.0) * r * 0.55;
                p.circle_filled(pos2(x, c.y - k * r * 0.22), dr, with_alpha(col, 0.35 + 0.65 * k));
            }
        }
        SpinnerKind::SquareFlip => {
            let ph = (time * 0.8).fract();
            let (sx, sy) = if ph < 0.5 { (1.0, (ph * 2.0 * PI).cos()) } else { (((ph - 0.5) * 2.0 * PI).cos(), 1.0) };
            let side = s * 0.68;
            let shade = 0.6 + 0.4 * (sx * sy).abs();
            let fill = col.lerp_to_gamma(if t.dark { Color32::BLACK } else { Color32::WHITE }, 1.0 - shade);
            p.rect_filled(
                Rect::from_center_size(c, vec2(side * sx.abs().max(0.04), side * sy.abs().max(0.04))),
                CornerRadius::same((side * 0.12) as u8),
                fill,
            );
        }
        SpinnerKind::Helix => {
            let n = 8;
            for i in 0..n {
                let u = i as f32 / (n - 1) as f32;
                let x = rect.min.x + s * 0.1 + u * s * 0.8;
                let ph = time * TAU * 0.8 + u * PI * 1.6;
                for strand in 0..2 {
                    let a = ph + strand as f32 * PI;
                    let y = c.y + a.sin() * r * 0.55;
                    let depth = 0.5 + 0.5 * a.cos();
                    let colr = if strand == 0 || custom { col } else { t.gradient[1] };
                    p.circle_filled(pos2(x, y), r * (0.06 + 0.06 * depth), with_alpha(colr, 0.3 + 0.7 * depth));
                }
            }
        }
        SpinnerKind::Spiral => {
            let n = 14;
            for i in 0..n {
                let u = i as f32 / n as f32;
                let ph = (time * 0.6 + u).fract();
                let rr = r * (1.0 - ph) * 0.92;
                let a = ph * TAU * 1.5 + u * TAU;
                p.circle_filled(
                    polar(c, rr, a),
                    r * 0.05 + r * 0.07 * (1.0 - ph),
                    with_alpha(col, (1.0 - ph) * 0.9 + 0.1),
                );
            }
        }
    }
}

/// Text followed by three animated dots ("Loading…"), for inline busy states.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_components::LoadingDots::new("Generating response"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LoadingDots {
    text: String,
    size: f32,
    color: Option<Color32>,
}

impl LoadingDots {
    /// Busy text (the dots are appended).
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 14.0, color: None }
    }
    /// Font size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Text color (default muted foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

impl Widget for LoadingDots {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let col = self.color.unwrap_or(t.muted_foreground);
        let g = ui.painter().layout_no_wrap(self.text, FontId::proportional(self.size), col);
        let dot_r = (self.size * 0.1).max(1.8);
        let w = g.size().x + self.size * 1.2;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
        let base_y = rect.min.y + g.size().y * 0.72;
        ui.painter().galley(rect.min, g.clone(), col);
        let time = glowkit_core::time(ui);
        for i in 0..3 {
            let ph = (time * 1.2 - i as f32 * 0.15).rem_euclid(1.0);
            let k = (ph * TAU).sin().max(0.0);
            let x = rect.min.x + g.size().x + self.size * 0.3 + i as f32 * self.size * 0.36;
            ui.painter().circle_filled(pos2(x, base_y - k * self.size * 0.18), dot_r, with_alpha(col, 0.35 + 0.65 * k));
        }
        glowkit_core::animate(ui);
        resp
    }
}
