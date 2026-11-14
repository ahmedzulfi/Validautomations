//! Materials (how a surface responds to light) and lights.

use crate::math::{vec3, Vec3};
use egui::Color32;

/// How a surface is shaded.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Shading {
    /// Base color only, no lighting.
    Unlit,
    /// One color per triangle (crisp low-poly facets).
    Flat,
    /// Per-vertex lighting interpolated across triangles (Gouraud).
    #[default]
    Smooth,
    /// Cel shading: diffuse light quantized into `bands` steps.
    Toon { bands: u8 },
    /// Debug-style rainbow: the surface normal mapped to RGB.
    Normal,
    /// A glossy "studio" look: the material's gradient is sampled by the view-space normal,
    /// plus a soft highlight. Lighting-independent and very pretty.
    Matcap,
    /// Edges only (no faces), fading with depth.
    Wireframe,
}

/// Wireframe overlay settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wire {
    pub color: Color32,
    /// Line width in points.
    pub width: f32,
    /// How much far edges fade (0 = no fade, 1 = far edges vanish).
    pub depth_fade: f32,
    /// Draw a soft additive halo around each edge (dark backgrounds).
    pub glow: f32,
}

impl Wire {
    /// A thin wire of the given color.
    pub fn new(color: Color32) -> Self {
        Self { color, width: 1.0, depth_fade: 0.7, glow: 0.0 }
    }
    /// Line width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Far-edge fade amount, 0..1.
    pub fn depth_fade(mut self, f: f32) -> Self {
        self.depth_fade = f.clamp(0.0, 1.0);
        self
    }
    /// Additive glow strength (0 = off).
    pub fn glow(mut self, g: f32) -> Self {
        self.glow = g.max(0.0);
        self
    }
}

/// Surface appearance: base color, shading model, highlights, rim light, emission and
/// an optional wireframe overlay.
///
/// ```no_run
/// use glowkit_3d::{Material, Shading};
/// use egui::Color32;
/// let glossy = Material::smooth(Color32::from_rgb(139, 92, 246)).specular(0.6).rim(0.5);
/// let toon = Material::toon(Color32::LIGHT_BLUE, 3);
/// let wire = Material::wireframe(Color32::WHITE);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub color: Color32,
    pub shading: Shading,
    /// Specular highlight strength (0..1+).
    pub specular: f32,
    /// Specular exponent (higher = smaller, sharper highlight).
    pub shininess: f32,
    /// Fresnel rim light strength.
    pub rim: f32,
    /// Rim light color (defaults to white-ish when `None`).
    pub rim_color: Option<Color32>,
    /// Self-illumination added on top of lighting (0..1 of the base color).
    pub emissive: f32,
    /// Overall opacity 0..1.
    pub opacity: f32,
    /// Render back faces too (and light them from behind).
    pub double_sided: bool,
    /// Blend additively (glowy, best on dark backgrounds).
    pub additive: bool,
    /// Wireframe overlay (or the wire style for [`Shading::Wireframe`]).
    pub wire: Option<Wire>,
    /// Gradient used by [`Shading::Matcap`] (shadow → mid → highlight).
    pub gradient: [Color32; 3],
    /// Whether scene fog affects this material.
    pub fog: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self::smooth(Color32::from_rgb(200, 200, 210))
    }
}

impl Material {
    fn base(color: Color32, shading: Shading) -> Self {
        Self {
            color,
            shading,
            specular: 0.35,
            shininess: 32.0,
            rim: 0.0,
            rim_color: None,
            emissive: 0.0,
            opacity: 1.0,
            double_sided: false,
            additive: false,
            wire: None,
            gradient: [glowkit_core::color::shade(color, -0.3), color, glowkit_core::color::shade(color, 0.25)],
            fog: true,
        }
    }
    /// Unlit base color.
    pub fn unlit(color: Color32) -> Self {
        Self::base(color, Shading::Unlit)
    }
    /// Faceted (one color per triangle).
    pub fn flat(color: Color32) -> Self {
        Self::base(color, Shading::Flat)
    }
    /// Smooth Gouraud shading with a specular highlight.
    pub fn smooth(color: Color32) -> Self {
        Self::base(color, Shading::Smooth)
    }
    /// Cel shading with `bands` light levels.
    pub fn toon(color: Color32, bands: u8) -> Self {
        Self { specular: 0.0, rim: 0.0, ..Self::base(color, Shading::Toon { bands: bands.max(2) }) }
    }
    /// Normal-colored rainbow.
    pub fn normal() -> Self {
        Self::base(Color32::WHITE, Shading::Normal)
    }
    /// Glossy gradient "matcap" look using three stops (shadow, mid, highlight).
    pub fn matcap(stops: [Color32; 3]) -> Self {
        Self { gradient: stops, rim: 0.35, specular: 0.7, ..Self::base(stops[1], Shading::Matcap) }
    }
    /// Edges only.
    pub fn wireframe(color: Color32) -> Self {
        Self { wire: Some(Wire::new(color)), ..Self::base(color, Shading::Wireframe) }
    }
    /// Emissive, additive glow material (neon). Best on dark backgrounds.
    pub fn glow(color: Color32) -> Self {
        Self { emissive: 1.0, additive: true, rim: 0.8, rim_color: Some(color), ..Self::base(color, Shading::Unlit) }
    }
    /// Set the base color (also re-derives the matcap gradient unless it's a matcap material).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        if self.shading != Shading::Matcap {
            self.gradient = [glowkit_core::color::shade(c, -0.3), c, glowkit_core::color::shade(c, 0.25)];
        }
        self
    }
    /// Set the shading model.
    pub fn shading(mut self, s: Shading) -> Self {
        self.shading = s;
        self
    }
    /// Specular strength.
    pub fn specular(mut self, s: f32) -> Self {
        self.specular = s.max(0.0);
        self
    }
    /// Specular exponent.
    pub fn shininess(mut self, s: f32) -> Self {
        self.shininess = s.max(1.0);
        self
    }
    /// Fresnel rim light strength.
    pub fn rim(mut self, r: f32) -> Self {
        self.rim = r.max(0.0);
        self
    }
    /// Rim light color.
    pub fn rim_color(mut self, c: Color32) -> Self {
        self.rim_color = Some(c);
        self
    }
    /// Emission amount 0..1.
    pub fn emissive(mut self, e: f32) -> Self {
        self.emissive = e.max(0.0);
        self
    }
    /// Opacity 0..1.
    pub fn opacity(mut self, o: f32) -> Self {
        self.opacity = o.clamp(0.0, 1.0);
        self
    }
    /// Render both sides of every triangle.
    pub fn double_sided(mut self, d: bool) -> Self {
        self.double_sided = d;
        self
    }
    /// Additive blending.
    pub fn additive(mut self, a: bool) -> Self {
        self.additive = a;
        self
    }
    /// Add a wireframe overlay.
    pub fn wire(mut self, w: Wire) -> Self {
        self.wire = Some(w);
        self
    }
    /// Gradient for matcap shading.
    pub fn gradient(mut self, stops: [Color32; 3]) -> Self {
        self.gradient = stops;
        self
    }
    /// Whether fog applies.
    pub fn fog(mut self, f: bool) -> Self {
        self.fog = f;
        self
    }
}

/// An infinitely far light (the sun).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// Direction *towards* the light.
    pub direction: Vec3,
    pub color: Color32,
    pub intensity: f32,
}

/// A light at a position, fading with distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Color32,
    pub intensity: f32,
    /// Distance at which the light has faded to roughly a quarter.
    pub range: f32,
}

/// The lights in a scene.
#[derive(Clone, Debug, PartialEq)]
pub struct Lights {
    pub ambient: Color32,
    pub ambient_intensity: f32,
    /// Sky/ground hemisphere tint: adds `sky` light to up-facing normals.
    pub sky: Color32,
    pub sky_intensity: f32,
    pub directional: Vec<DirectionalLight>,
    pub point: Vec<PointLight>,
}

impl Default for Lights {
    /// A soft three-point studio rig: warm key from the upper right, cool fill from the left,
    /// ambient plus sky light.
    fn default() -> Self {
        Self {
            ambient: Color32::WHITE,
            ambient_intensity: 0.28,
            sky: Color32::from_rgb(200, 215, 255),
            sky_intensity: 0.12,
            directional: vec![
                DirectionalLight {
                    direction: vec3(0.55, 0.8, 0.6).normalized(),
                    color: Color32::from_rgb(255, 250, 240),
                    intensity: 0.85,
                },
                DirectionalLight {
                    direction: vec3(-0.8, 0.1, 0.3).normalized(),
                    color: Color32::from_rgb(190, 205, 255),
                    intensity: 0.28,
                },
            ],
            point: Vec::new(),
        }
    }
}

impl Lights {
    /// No lights at all.
    pub fn none() -> Self {
        Self {
            ambient: Color32::WHITE,
            ambient_intensity: 0.0,
            sky: Color32::WHITE,
            sky_intensity: 0.0,
            directional: vec![],
            point: vec![],
        }
    }
    /// Only ambient light.
    pub fn ambient(intensity: f32) -> Self {
        Self { ambient_intensity: intensity, ..Self::none() }
    }
    /// Add a directional light (direction points towards the light).
    pub fn with_directional(mut self, direction: Vec3, color: Color32, intensity: f32) -> Self {
        self.directional.push(DirectionalLight { direction: direction.normalized(), color, intensity });
        self
    }
    /// Add a point light.
    pub fn with_point(mut self, position: Vec3, color: Color32, intensity: f32, range: f32) -> Self {
        self.point.push(PointLight { position, color, intensity, range: range.max(1e-3) });
        self
    }
}

/// Linear distance fog: geometry blends towards `color` between `near` and `far` (view distance).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub color: Color32,
    pub near: f32,
    pub far: f32,
    /// Fade alpha instead of blending to `color` (lets the background show through).
    pub fade: bool,
}

impl Fog {
    /// Fog blending to `color` between `near` and `far`.
    pub fn new(color: Color32, near: f32, far: f32) -> Self {
        Self { color, near, far, fade: false }
    }
    /// Fog that fades geometry out (transparent) instead of tinting it.
    pub fn fade(near: f32, far: f32) -> Self {
        Self { color: Color32::TRANSPARENT, near, far, fade: true }
    }
    /// Fog amount at a view distance (0 = clear, 1 = fully fogged).
    pub fn amount(&self, dist: f32) -> f32 {
        ((dist - self.near) / (self.far - self.near).max(1e-4)).clamp(0.0, 1.0)
    }
}

/// Float RGB helper used by the shader code.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    #[inline]
    pub fn from(c: Color32) -> Rgb {
        let [r, g, b, _] = c.to_srgba_unmultiplied();
        Rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }
    #[inline]
    pub fn scale(self, k: f32) -> Rgb {
        Rgb(self.0 * k, self.1 * k, self.2 * k)
    }
    #[inline]
    pub fn mul(self, o: Rgb) -> Rgb {
        Rgb(self.0 * o.0, self.1 * o.1, self.2 * o.2)
    }
    #[inline]
    pub fn add(self, o: Rgb) -> Rgb {
        Rgb(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    #[inline]
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        Rgb(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t, self.2 + (o.2 - self.2) * t)
    }
    /// To a premultiplied color with `alpha`; `additive` zeroes alpha for additive blending.
    #[inline]
    pub fn to_color(self, alpha: f32, additive: bool) -> Color32 {
        let a = alpha.clamp(0.0, 1.0);
        // soft tone-map so bright highlights roll off instead of clipping hard
        let tm = |x: f32| {
            let x = x.max(0.0);
            if x < 0.8 {
                x
            } else {
                0.8 + 0.2 * (1.0 - (-(x - 0.8) * 5.0).exp())
            }
        };
        let q = |x: f32| (tm(x) * a * 255.0).round().clamp(0.0, 255.0) as u8;
        Color32::from_rgba_premultiplied(
            q(self.0),
            q(self.1),
            q(self.2),
            if additive { 0 } else { (a * 255.0).round() as u8 },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_amount() {
        let f = Fog::new(Color32::BLACK, 2.0, 4.0);
        assert_eq!(f.amount(1.0), 0.0);
        assert_eq!(f.amount(3.0), 0.5);
        assert_eq!(f.amount(10.0), 1.0);
    }

    #[test]
    fn rgb_roundtrip_and_additive() {
        let c = Rgb::from(Color32::from_rgb(10, 128, 200)).to_color(1.0, false);
        assert_eq!(c, Color32::from_rgb(10, 128, 200));
        assert_eq!(Rgb(0.5, 0.5, 0.5).to_color(1.0, true).a(), 0);
    }
}
