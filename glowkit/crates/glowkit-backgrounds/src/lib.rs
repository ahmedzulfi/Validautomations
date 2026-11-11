//! # glowkit-backgrounds
//!
//! Animated backgrounds for egui: aurora, particles, grids, waves, beams, meteors, starfields,
//! silk, plasma and more.
//!
//! Every background is a builder with two ways to use it:
//!
//! * as a widget, `ui.add(Aurora::new())`, which allocates the full available width and 320
//!   points of height (change it with `.size(..)`), or
//! * behind other content with `.paint(ui, rect)`, which paints into any rect without
//!   allocating space, so you can put a headline or a whole page section on top.
//!
//! ```no_run
//! # let ui: &mut egui::Ui = todo!();
//! use glowkit_backgrounds::Aurora;
//! ui.add(Aurora::new());
//! let rect = ui.max_rect();
//! Aurora::new().speed(0.6).paint(ui, rect);
//! ```
//!
//! Backgrounds fill their rect with the theme background first (override with
//! `.background(color)`, or `.transparent()` to paint only the effect).
//! Interactive backgrounds read the pointer directly, so they react even when other widgets sit
//! on top of them. When several backgrounds of the same type are painted with `paint` inside one
//! `Ui`, give each a distinct `.id_salt(..)` so their animation state stays separate.

use egui::{pos2, vec2, Color32, Id, Painter, Pos2, Rect, Ui};
use glowkit_core::Theme;

mod aurora;
mod glyphs;
mod grain;
mod grids;
mod light;
mod lines;
mod particles;
mod shaders;
mod sky;

pub use aurora::{Aurora, GradientBlobs, MeshGradient};
pub use glyphs::{LetterGlitch, MatrixRain};
pub use grain::GrainNoise;
pub use grids::{BackgroundBoxes, DotGrid, FlickeringGrid, GridPattern, HexGrid, RetroGrid, SquaresGrid};
pub use light::{Lamp, Lightning, Orb, Ripple, Spotlight, SpotlightOrigin};
pub use lines::{BackgroundBeams, Beams, Threads, Waves, WavyBackground};
pub use particles::{Galaxy, Particles, Snowfall, Vortex, WarpSpeed};
pub use shaders::{Iridescence, LiquidMetal, Plasma, Silk, Swirl};
pub use sky::{Meteors, ShootingStars, StarField};
pub use grids::Direction;

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this crate.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

// ---------------------------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------------------------

/// How a background fills its rect before painting the effect.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum Fill {
    #[default]
    Theme,
    Color(Color32),
    None,
}

/// Settings every background shares (size, id, base fill).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Base {
    pub size: Option<egui::Vec2>,
    pub salt: Option<Id>,
    pub auto: Option<Id>,
    pub fill: Fill,
}

/// Everything a background needs while painting one frame.
pub(crate) struct Frame {
    pub painter: Painter,
    pub id: Id,
    pub theme: Theme,
    pub time: f32,
    pub dt: f32,
    pub rect: Rect,
    /// The color under the effect (for masks and fades).
    pub bg: Color32,
}

impl Frame {
    pub fn dark(&self) -> bool {
        self.theme.dark
    }
}

impl Base {
    pub fn with_fill(fill: Fill) -> Self {
        Self { fill, ..Default::default() }
    }

    /// Prepare painting: returns `None` when the rect is not visible.
    pub fn begin(&self, ui: &Ui, rect: Rect, name: &'static str) -> Option<Frame> {
        if !ui.is_rect_visible(rect) || rect.width() < 1.0 || rect.height() < 1.0 {
            return None;
        }
        let theme = Theme::of(ui);
        let painter = ui.painter_at(rect);
        let bg = match self.fill {
            Fill::Theme => theme.background,
            Fill::Color(c) => c,
            Fill::None => theme.background,
        };
        if !matches!(self.fill, Fill::None) {
            painter.rect_filled(rect, 0.0, bg);
        }
        glowkit_core::animate(ui);
        let id = match (self.salt, self.auto) {
            (Some(s), _) => ui.scope_id().with((name, s)),
            (None, Some(a)) => a,
            _ => ui.scope_id().with(name),
        };
        Some(Frame { painter, id, theme, time: glowkit_core::time(ui), dt: glowkit_core::dt(ui), rect, bg })
    }
}

/// Generates the setters and the `Widget` impl every background shares.
macro_rules! background {
    ($T:ident) => {
        impl $T {
            /// Size when added as a widget (default: available width x 320).
            pub fn size(mut self, size: egui::Vec2) -> Self {
                self.base.size = Some(size);
                self
            }
            /// Salt for the animation state id; needed when several of these share one `Ui`.
            pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
                self.base.salt = Some(egui::Id::NULL.with(salt));
                self
            }
            /// Fill the rect with this color before painting (default: theme background).
            pub fn background(mut self, color: egui::Color32) -> Self {
                self.base.fill = crate::Fill::Color(color);
                self
            }
            /// Do not fill the rect first: only the effect is painted, over whatever is below.
            pub fn transparent(mut self) -> Self {
                self.base.fill = crate::Fill::None;
                self
            }
        }

        impl egui::Widget for $T {
            fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
                let size = glowkit_core::desired_size(ui, self.base.size, 320.0);
                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
                if self.base.salt.is_none() {
                    self.base.auto = Some(resp.id);
                }
                self.paint(ui, rect);
                resp
            }
        }
    };
}
pub(crate) use background;

/// Pointer position if it hovers `rect`.
pub(crate) fn pointer(ui: &Ui, rect: Rect) -> Option<Pos2> {
    if !ui.rect_contains_pointer(rect) {
        return None;
    }
    ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p))
}

/// A slowly wandering point inside `rect`, used as a stand-in pointer when nobody hovers so
/// interactive backgrounds still show off their reaction.
pub(crate) fn ghost(rect: Rect, t: f32) -> Pos2 {
    let u = 0.5 + 0.36 * (t * 0.37).sin() * (t * 0.11 + 1.0).cos();
    let v = 0.5 + 0.34 * (t * 0.53 + 0.7).sin();
    pos2(rect.min.x + u * rect.width(), rect.min.y + v * rect.height())
}

#[inline]
pub(crate) fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub(crate) fn uv_to_pos(rect: Rect, uv: egui::Vec2) -> Pos2 {
    pos2(rect.min.x + uv.x * rect.width(), rect.min.y + uv.y * rect.height())
}

/// A light-emitting color: additive on dark themes (overlaps get brighter, like real light),
/// normal alpha blending on light themes. `k` scales the intensity.
#[inline]
pub(crate) fn light(c: Color32, k: f32, dark: bool) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    if dark {
        let [r, g, b, a] = c.to_array();
        let s = k * a as f32 / 255.0;
        Color32::from_rgba_premultiplied((r as f32 * s) as u8, (g as f32 * s) as u8, (b as f32 * s) as u8, 0)
    } else {
        c.gamma_multiply(k)
    }
}

/// Fast RGB working color in sRGB-ish `0..1` space (for per-vertex shading).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    pub fn of(c: Color32) -> Self {
        let [r, g, b, _] = c.to_srgba_unmultiplied();
        Self(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t, self.2 + (o.2 - self.2) * t)
    }
    pub fn scale(self, k: f32) -> Self {
        Self(self.0 * k, self.1 * k, self.2 * k)
    }
    pub fn add(self, o: Self) -> Self {
        Self(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    pub fn to_color(self) -> Color32 {
        let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        Color32::from_rgb(f(self.0), f(self.1), f(self.2))
    }
}

/// A precomputed gradient lookup table (perceptual mixing done once, cheap sampling after).
pub(crate) struct Lut {
    table: Vec<Rgb>,
    cyclic: bool,
}

impl Lut {
    pub fn new(stops: &[Color32], cyclic: bool) -> Self {
        let n = 96;
        let table = (0..n)
            .map(|i| {
                let t = i as f32 / if cyclic { n as f32 } else { (n - 1) as f32 };
                let c = if cyclic {
                    glowkit_core::color::gradient_cyclic(stops, t)
                } else {
                    glowkit_core::color::gradient(stops, t)
                };
                Rgb::of(c)
            })
            .collect();
        Self { table, cyclic }
    }
    pub fn sample(&self, t: f32) -> Rgb {
        let n = self.table.len();
        let x = if self.cyclic { t.rem_euclid(1.0) * n as f32 } else { t.clamp(0.0, 1.0) * (n - 1) as f32 };
        let i = x.floor() as usize;
        let f = x - i as f32;
        let a = self.table[i % n];
        let b = if self.cyclic { self.table[(i + 1) % n] } else { self.table[(i + 1).min(n - 1)] };
        a.lerp(b, f)
    }
}

/// A ribbon along `pts` with soft (blurred) edges: solid core of `width`, fading to transparent
/// over `soft` points on each side. `f(t)` colors it along its length.
pub(crate) fn soft_ribbon(painter: &Painter, pts: &[Pos2], width: f32, soft: f32, f: impl Fn(f32) -> Color32) {
    use egui::epaint::{Mesh, Shape};
    let n = pts.len();
    if n < 2 {
        return;
    }
    let hw = (width * 0.5).max(0.0);
    let mut m = Mesh::default();
    m.reserve_vertices(n * 4);
    for i in 0..n {
        let p = pts[i];
        let a = pts[i.saturating_sub(1)];
        let b = pts[(i + 1).min(n - 1)];
        let normal = (b - a).normalized().rot90();
        let c = f(i as f32 / (n - 1) as f32);
        let clear = Color32::TRANSPARENT;
        m.colored_vertex(p + normal * (hw + soft), clear);
        m.colored_vertex(p + normal * hw, c);
        m.colored_vertex(p - normal * hw, c);
        m.colored_vertex(p - normal * (hw + soft), clear);
    }
    for i in 0..n as u32 - 1 {
        let a = i * 4;
        let b = a + 4;
        for s in 0..3 {
            m.add_triangle(a + s, b + s, b + s + 1);
            m.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(m));
}

/// Paint `bg` over the outside of an ellipse inscribed in `rect`: transparent inside `inner`
/// (fraction of the radius), fully `bg` from `outer` on. A soft "mask-image: radial-gradient".
pub(crate) fn radial_mask(painter: &Painter, rect: Rect, bg: Color32, inner: f32, outer: f32) {
    let c = rect.center();
    let r = rect.size() * 0.5;
    glowkit_core::paint::shader_grid(painter, rect, 24, 16, |uv| {
        let p = uv_to_pos(rect, uv);
        let d = vec2((p.x - c.x) / r.x, (p.y - c.y) / r.y).length();
        bg.gamma_multiply(smoothstep(inner, outer, d))
    });
}

/// Cheap hash of integers to `0..1`.
#[inline]
pub(crate) fn h01(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77).rotate_left(13);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}
