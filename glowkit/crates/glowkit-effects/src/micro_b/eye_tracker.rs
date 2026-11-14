use super::common::{cr, hairline, idle_for, ink, lissajous, shadow_color, surface};
use egui::{vec2, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::spring2;
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::{paint, Theme};

/// Eye style for an [`EyeTracker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EyeShape {
    /// Tall rounded pills (default).
    #[default]
    Eyes,
    /// Pills tilted inward, a little mischievous.
    Slant,
    /// Round dots.
    Dots,
    /// Rounded squares.
    Squares,
}

/// A soft rounded cube with two eyes that turn to follow the cursor anywhere on the stage. The
/// face slides toward the pointer on a spring (overshooting a little with more bounce), the cube
/// leans after it, and it blinks now and then. While the pointer is away it watches an invisible
/// fly drift around.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{EyeShape, EyeTracker};
/// ui.add(EyeTracker::new().shape(EyeShape::Dots).size(100.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct EyeTracker {
    size: f32,
    height: f32,
    shape: EyeShape,
    follow: f32,
    bounce: f32,
    dark: bool,
    autoplay: bool,
}

impl Default for EyeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EyeTracker {
    /// An eye-tracking cube.
    pub fn new() -> Self {
        Self { size: 100.0, height: 300.0, shape: EyeShape::Eyes, follow: 0.6, bounce: 0.3, dark: false, autoplay: true }
    }
    /// Cube size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(30.0, 400.0);
        self
    }
    /// Height of the area it watches over (width is all available).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(self.size + 20.0);
        self
    }
    /// Eye style.
    pub fn shape(mut self, s: EyeShape) -> Self {
        self.shape = s;
        self
    }
    /// How far the face turns toward the pointer (0..1).
    pub fn follow(mut self, k: f32) -> Self {
        self.follow = k.clamp(0.0, 1.0);
        self
    }
    /// Overshoot when the gaze moves (0..1).
    pub fn bounce(mut self, k: f32) -> Self {
        self.bounce = k.clamp(0.0, 1.0);
        self
    }
    /// Dark (ink) cube.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Watch a drifting point while the pointer is away (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for EyeTracker {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (stage, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let center = stage.center();
        let s = self.size;
        let real = ui.input(|i| i.pointer.hover_pos()).filter(|_| ui.rect_contains_pointer(stage));
        let idle = idle_for(ui, id, real.is_some());
        let time = glowkit_core::time(ui);
        let target: Option<Pos2> = match real {
            Some(p) => Some(p),
            None if self.autoplay && idle > 0.8 => {
                glowkit_core::animate(ui);
                // a fly that darts between slow drifts
                let jump = (time / 1.7).floor();
                let base = lissajous(stage.shrink(20.0), time * 0.5, 1.3);
                let dart = vec2((jump * 12.9898).sin(), (jump * 78.233).cos()) * 70.0;
                Some(base + dart)
            }
            _ => None,
        };
        // gaze offset: direction to the target, saturating with distance
        let look = target.map_or(Vec2::ZERO, |p| {
            let d = p - center;
            let reach = stage.width().max(stage.height()) * 0.5;
            let m = (d.length() / reach).min(1.0);
            if d.length() > 0.1 { d.normalized() * m } else { Vec2::ZERO }
        });
        let damping = 26.0 - 18.0 * self.bounce;
        let face = spring2(ui, id.with("face"), look * self.follow, 160.0, damping);
        let body = spring2(ui, id.with("body"), look * self.follow, 70.0, 14.0);
        if !ui.is_rect_visible(stage) {
            return response;
        }
        let painter = ui.painter_at(stage);
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, eye) = if self.dark { (ink_bg, ink_fg) } else { (surface(&t), if t.dark { t.foreground } else { egui::Color32::from_rgb(20, 20, 24) }) };
        // the cube leans toward the gaze a little
        let lean = body * s * 0.06;
        let cube = Rect::from_center_size(center + lean, Vec2::splat(s));
        let radius = s * 0.26;
        paint::soft_shadow(&painter, cube, radius, s * 0.22, vec2(-body.x * s * 0.05, s * 0.07 - body.y * s * 0.03), shadow_color(&t, 0.9));
        painter.rect(cube, cr(radius), bg, hairline(&t), egui::StrokeKind::Inside);
        // soft shading on the side facing away from the gaze
        let shade_c = cube.center() - body * s * 0.45;
        paint::glow(&painter.with_clip_rect(cube.shrink(2.0)), shade_c, s * 0.7, with_alpha(shade(bg, -0.12), 0.25));
        // blinks
        let cycle = 3.6;
        let ph = (time + (id.value() % 100) as f32 * 0.01) % cycle;
        let blink = if ph < 0.14 { 1.0 - (ph / 0.07 - 1.0).abs() } else { 0.0 };
        let fc = cube.center() + face * s * 0.24;
        let sep = s * 0.15;
        for side in [-1.0f32, 1.0] {
            let c = fc + vec2(side * sep, -s * 0.02) + face * s * 0.02 * side.abs();
            let open = 1.0 - 0.9 * blink;
            match self.shape {
                EyeShape::Eyes | EyeShape::Slant => {
                    let w = s * 0.085;
                    let h = (s * 0.17 * open).max(w * 0.4);
                    let tilt = if self.shape == EyeShape::Slant { side * -0.35 } else { face.x * 0.15 };
                    let d = Vec2::angled(std::f32::consts::FRAC_PI_2 + tilt);
                    let half = (h - w).max(0.0) / 2.0;
                    painter.line_segment([c - d * half, c + d * half], Stroke::new(w, eye));
                    painter.circle_filled(c - d * half, w / 2.0, eye);
                    painter.circle_filled(c + d * half, w / 2.0, eye);
                }
                EyeShape::Dots => {
                    let r = s * 0.06;
                    if open > 0.5 {
                        painter.circle_filled(c, r, eye);
                    } else {
                        painter.add(Shape::ellipse_filled(c, vec2(r * 1.1, r * open.max(0.15)), eye));
                    }
                }
                EyeShape::Squares => {
                    let w = s * 0.12;
                    let r = Rect::from_center_size(c, vec2(w, w * open.max(0.12)));
                    painter.rect_filled(r, cr(w * 0.25), eye);
                }
            }
        }
        response
    }
}
