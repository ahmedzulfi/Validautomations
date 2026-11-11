use super::{glow_c, salt_or};
use crate::{vec3, Background, Material, Mesh, OrbitControls, Scene, Transform, Vec3, Viewport3D};
use egui::{Color32, Response, Ui, Widget};
use glowkit_core::{color, Theme};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// An arc between two places on a [`Globe3D`], given as `(latitude, longitude)` in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeArc {
    pub from: (f32, f32),
    pub to: (f32, f32),
    /// Arc color (defaults to the theme gradient).
    pub color: Option<Color32>,
}

impl GlobeArc {
    /// An arc from `(lat, lon)` to `(lat, lon)` in degrees.
    pub fn new(from: (f32, f32), to: (f32, f32)) -> Self {
        Self { from, to, color: None }
    }
    /// Arc color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// A set of arcs between major world cities.
    pub fn samples() -> Vec<GlobeArc> {
        let sf = (37.7, -122.4);
        let ny = (40.7, -74.0);
        let lon = (51.5, -0.1);
        let tok = (35.7, 139.7);
        let syd = (-33.9, 151.2);
        let sao = (-23.5, -46.6);
        let sin = (1.35, 103.8);
        let cpt = (-33.9, 18.4);
        let dxb = (25.2, 55.3);
        let bom = (19.0, 72.8);
        vec![
            GlobeArc::new(sf, tok),
            GlobeArc::new(ny, lon),
            GlobeArc::new(lon, dxb),
            GlobeArc::new(dxb, sin),
            GlobeArc::new(sin, syd),
            GlobeArc::new(sao, cpt),
            GlobeArc::new(ny, sao),
            GlobeArc::new(bom, tok),
            GlobeArc::new(sf, ny),
            GlobeArc::new(cpt, dxb),
        ]
    }
}

/// `(lat, lon)` in degrees to a point on the unit sphere.
pub(crate) fn latlon(lat: f32, lon: f32) -> Vec3 {
    Vec3::from_spherical(1.0, lon.to_radians(), lat.to_radians())
}

// Very rough continent outlines as (lon, lat) polygons: enough to read as Earth.
const LAND: &[&[(f32, f32)]] = &[
    // North America
    &[
        (-168.0, 65.0), (-160.0, 71.0), (-140.0, 70.0), (-125.0, 72.0), (-95.0, 73.0), (-80.0, 70.0), (-75.0, 62.0), (-62.0, 58.0), (-55.0, 50.0), (-66.0, 44.0),
        (-71.0, 41.0), (-76.0, 35.0), (-81.0, 31.0), (-80.0, 25.0), (-84.0, 30.0), (-90.0, 29.0), (-97.0, 27.0), (-97.0, 21.0), (-92.0, 18.0), (-87.0, 15.0),
        (-83.0, 9.0), (-78.0, 8.0), (-86.0, 13.0), (-92.0, 15.0), (-105.0, 20.0), (-110.0, 24.0), (-115.0, 30.0), (-117.0, 33.0), (-121.0, 35.0), (-124.0, 41.0),
        (-124.0, 48.0), (-131.0, 55.0), (-140.0, 59.0), (-152.0, 58.0), (-158.0, 56.0), (-165.0, 60.0),
    ],
    // Greenland
    &[(-55.0, 60.0), (-44.0, 60.0), (-35.0, 66.0), (-20.0, 70.0), (-18.0, 77.0), (-30.0, 83.0), (-55.0, 82.0), (-70.0, 78.0), (-58.0, 72.0), (-52.0, 66.0)],
    // South America
    &[
        (-80.0, 10.0), (-72.0, 12.0), (-62.0, 10.0), (-52.0, 5.0), (-35.0, -6.0), (-38.0, -14.0), (-40.0, -22.0), (-48.0, -26.0), (-53.0, -34.0), (-58.0, -38.0),
        (-65.0, -42.0), (-66.0, -48.0), (-69.0, -52.0), (-74.0, -53.0), (-73.0, -45.0), (-72.0, -35.0), (-71.0, -25.0), (-70.0, -18.0), (-76.0, -14.0), (-81.0, -6.0),
        (-80.0, 0.0), (-78.0, 6.0),
    ],
    // Europe
    &[
        (-10.0, 36.0), (-9.0, 43.0), (-1.0, 46.0), (-4.0, 48.0), (2.0, 51.0), (8.0, 54.0), (8.0, 57.0), (5.0, 62.0), (12.0, 66.0), (18.0, 70.0), (28.0, 71.0), (40.0, 68.0),
        (42.0, 60.0), (40.0, 48.0), (30.0, 45.0), (28.0, 41.0), (26.0, 38.0), (20.0, 40.0), (16.0, 38.0), (12.0, 44.0), (8.0, 44.0), (3.0, 42.0), (0.0, 38.0), (-6.0, 36.0),
    ],
    // Britain
    &[(-6.0, 50.0), (1.0, 51.0), (2.0, 53.0), (-2.0, 56.0), (-3.0, 58.5), (-6.5, 58.0), (-5.0, 54.0)],
    // Africa
    &[
        (-17.0, 21.0), (-16.0, 28.0), (-10.0, 30.0), (-6.0, 35.0), (10.0, 37.0), (11.0, 33.0), (20.0, 31.0), (32.0, 31.0), (35.0, 28.0), (43.0, 12.0), (51.0, 12.0),
        (51.0, 10.0), (42.0, -1.0), (40.0, -10.0), (40.0, -16.0), (35.0, -24.0), (32.0, -29.0), (27.0, -34.0), (19.0, -35.0), (17.0, -29.0), (12.0, -17.0), (13.0, -6.0),
        (9.0, -1.0), (9.0, 4.0), (5.0, 5.0), (-4.0, 5.0), (-8.0, 4.0), (-13.0, 8.0), (-17.0, 14.0),
    ],
    // Madagascar
    &[(44.0, -25.0), (47.0, -25.0), (50.0, -16.0), (49.0, -12.0), (44.0, -17.0)],
    // Asia (with Arabia and India)
    &[
        (40.0, 68.0), (60.0, 70.0), (70.0, 73.0), (80.0, 73.0), (100.0, 77.0), (115.0, 74.0), (130.0, 72.0), (150.0, 71.0), (170.0, 70.0), (179.0, 68.0), (179.0, 64.0),
        (170.0, 60.0), (160.0, 58.0), (156.0, 51.0), (142.0, 47.0), (141.0, 53.0), (135.0, 44.0), (129.0, 41.0), (126.0, 35.0), (122.0, 40.0), (121.0, 31.0),
        (118.0, 24.0), (110.0, 20.0), (107.0, 16.0), (108.0, 11.0), (104.0, 9.0), (103.0, 1.0), (100.0, 6.0), (98.0, 12.0), (94.0, 17.0), (91.0, 22.0), (87.0, 21.0),
        (80.0, 15.0), (77.0, 8.0), (73.0, 20.0), (67.0, 25.0), (57.0, 25.0), (56.0, 27.0), (52.0, 24.0), (59.0, 22.0), (52.0, 15.0), (43.0, 13.0), (35.0, 28.0),
        (35.0, 36.0), (28.0, 41.0), (30.0, 45.0), (40.0, 48.0), (42.0, 60.0),
    ],
    // Japan
    &[(130.0, 31.0), (135.0, 34.0), (140.0, 35.0), (142.0, 40.0), (141.5, 45.0), (139.5, 42.0), (136.0, 36.5), (131.0, 34.5)],
    // Sumatra, Borneo, New Guinea
    &[(95.0, 5.0), (98.0, 4.0), (106.0, -6.0), (103.0, -5.0), (100.0, -1.0)],
    &[(109.0, 1.0), (117.0, 7.0), (119.0, 1.0), (116.0, -4.0), (110.0, -3.0)],
    &[(131.0, -1.0), (141.0, -3.0), (150.0, -10.0), (142.0, -9.0), (135.0, -4.0)],
    // Australia
    &[
        (114.0, -22.0), (122.0, -18.0), (129.0, -15.0), (136.0, -12.0), (142.0, -11.0), (146.0, -19.0), (153.0, -26.0), (151.0, -34.0), (146.0, -39.0), (140.0, -38.0),
        (135.0, -34.0), (129.0, -32.0), (117.0, -35.0), (115.0, -30.0),
    ],
    // New Zealand
    &[(172.0, -35.0), (178.0, -38.0), (174.0, -41.5), (167.0, -46.0), (170.5, -43.0)],
];

fn in_poly(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Whether `(lat, lon)` (degrees) is on land, using built-in rough continent outlines.
pub fn is_land(lat: f32, lon: f32) -> bool {
    lat < -70.0 || LAND.iter().any(|p| in_poly(p, lon, lat))
}

fn land_points(samples: usize) -> Vec<Vec3> {
    let mut out = Vec::new();
    for i in 0..samples {
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / samples as f32;
        let r = (1.0 - y * y).max(0.0).sqrt();
        let a = i as f32 * PI * (3.0 - 5f32.sqrt());
        let p = vec3(a.cos() * r, y, a.sin() * r);
        let lat = y.asin().to_degrees();
        let lon = p.x.atan2(p.z).to_degrees();
        if is_land(lat, lon) {
            out.push(p);
        }
    }
    out
}

/// Whether a point is hidden behind a sphere of radius `r` at the origin, seen from `cam`.
fn occluded(cam: Vec3, p: Vec3, r: f32) -> bool {
    let to = p - cam;
    let len = to.length();
    let d = to / len.max(1e-6);
    let b = cam.dot(d);
    let c = cam.length_sq() - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return false;
    }
    let t1 = -b - disc.sqrt();
    t1 > 0.0 && t1 < len - 1e-3
}

/// A true-3D globe: continents drawn as thousands of lit dots on a shaded sphere, glowing
/// arcs with travelling comets between cities, pulsing markers, a graticule and an
/// atmosphere halo. Drag to spin it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_3d::{Globe3D, GlobeArc};
/// ui.add(Globe3D::new().arcs(GlobeArc::samples()).height(380.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Globe3D {
    arcs: Vec<GlobeArc>,
    dot_color: Option<Color32>,
    ocean: Option<Color32>,
    glow: Option<Color32>,
    density: usize,
    graticule: bool,
    speed: f32,
    height: f32,
    id_salt: Option<String>,
}

impl Default for Globe3D {
    fn default() -> Self {
        Self::new()
    }
}

impl Globe3D {
    /// A globe with no arcs.
    pub fn new() -> Self {
        Self { arcs: Vec::new(), dot_color: None, ocean: None, glow: None, density: 6000, graticule: true, speed: 0.18, height: 380.0, id_salt: None }
    }
    /// Arcs between places.
    pub fn arcs(mut self, arcs: Vec<GlobeArc>) -> Self {
        self.arcs = arcs;
        self
    }
    /// Land dot color.
    pub fn dot_color(mut self, c: Color32) -> Self {
        self.dot_color = Some(c);
        self
    }
    /// Sphere (ocean) color.
    pub fn ocean_color(mut self, c: Color32) -> Self {
        self.ocean = Some(c);
        self
    }
    /// Atmosphere glow color.
    pub fn glow_color(mut self, c: Color32) -> Self {
        self.glow = Some(c);
        self
    }
    /// Dot sampling density (points over the whole sphere; ~30% land).
    pub fn density(mut self, n: usize) -> Self {
        self.density = n.clamp(500, 40_000);
        self
    }
    /// Latitude/longitude grid lines.
    pub fn graticule(mut self, on: bool) -> Self {
        self.graticule = on;
        self
    }
    /// Auto-rotation speed (radians/second).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Id salt for several instances.
    pub fn id_salt(mut self, s: impl Into<String>) -> Self {
        self.id_salt = Some(s.into());
        self
    }
}

fn arc_points(a: Vec3, b: Vec3, n: usize) -> Vec<Vec3> {
    let th = a.dot(b).clamp(-1.0, 1.0).acos();
    let s = th.sin().max(1e-4);
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let d = a * (((1.0 - t) * th).sin() / s) + b * ((t * th).sin() / s);
            d.normalized() * (1.0 + (t * PI).sin() * (0.05 + th * 0.12))
        })
        .collect()
}

impl Widget for Globe3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let dark = t.dark;
        let glow = self.glow.unwrap_or(t.gradient[0]);
        let dots = self.dot_color.unwrap_or(if dark { color::mix(t.gradient[0], Color32::WHITE, 0.45) } else { color::shade(t.accent, -0.05) });
        let ocean = self.ocean.unwrap_or(if dark { color::mix(t.card, glow, 0.14) } else { color::mix(Color32::WHITE, glow, 0.06) });
        Viewport3D::new(salt_or(&self.id_salt, "globe-3d"))
            .height(self.height)
            .background(Background::None)
            .fov(32.0)
            .controls(OrbitControls::new().distance(4.9).pitch(0.3).yaw(-1.2).auto_rotate(self.speed).pitch_range(-1.0, 1.0).zoom(false).damping(2.5))
            .show(ui, |scene, ctx| {
                let land = ctx.memo(("land", self.density), || Arc::new(land_points(self.density)));
                let sphere = ctx.mesh("sphere", || Mesh::icosphere(1.0, 3));
                let cam = scene.camera().position;
                let fwd = scene.camera().forward();
                let right = fwd.cross(crate::Vec3::Y).normalized();
                let upv = right.cross(fwd);
                // the sun sits up-left of the viewer so the visible side is always lit
                let sun = (-fwd * 1.0 + upv * 0.6 - right * 0.5).normalized();
                scene.lights = crate::Lights::ambient(0.35).with_directional(sun, Color32::WHITE, 0.75);
                // halo behind the globe
                scene.underlay(|s| {
                    s.glow_point(Vec3::ZERO, 1.42, glow_c(&t, glow, if dark { 0.55 } else { 0.35 }));
                });
                let body = Material::smooth(ocean).specular(if dark { 0.12 } else { 0.3 }).shininess(28.0).rim(if dark { 0.9 } else { 0.6 }).rim_color(glow);
                scene.draw(&sphere, Transform::IDENTITY.with_scale(0.995), &body);
                let time = ctx.time;
                scene.overlay(|s: &mut Scene| {
                    if self.graticule {
                        let gc = if dark { color::with_alpha(glow, 0.16) } else { color::with_alpha(glow, 0.22) };
                        let mut lines: Vec<Vec<Vec3>> = Vec::new();
                        for lat in [-60.0f32, -30.0, 0.0, 30.0, 60.0] {
                            lines.push((0..=72).map(|i| latlon(lat, i as f32 * 5.0) * 1.002).collect());
                        }
                        for lon in (0..12).map(|i| i as f32 * 30.0) {
                            lines.push((0..=36).map(|i| latlon(-90.0 + i as f32 * 5.0, lon) * 1.002).collect());
                        }
                        for l in &lines {
                            for w in l.windows(2) {
                                if !occluded(cam, w[0], 1.0) && !occluded(cam, w[1], 1.0) {
                                    let k = (w[0].normalized().dot((cam - w[0]).normalized())).clamp(0.0, 1.0);
                                    s.line(w[0], w[1], 1.0, gc.gamma_multiply(0.3 + 0.7 * k));
                                }
                            }
                        }
                    }
                    for p in land.iter() {
                        let q = *p * 1.006;
                        if occluded(cam, q, 1.0) {
                            continue;
                        }
                        let lit = 0.35 + 0.65 * p.dot(sun).max(0.0);
                        let edge = p.dot((cam - q).normalized()).clamp(0.0, 1.0);
                        let a = (0.25 + 0.75 * edge.sqrt()) * lit;
                        let c = if dark { crate::additive(dots.gamma_multiply(a)) } else { color::with_alpha(dots, (0.35 + 0.65 * a).min(1.0)) };
                        s.point(q, 0.0105, c);
                    }
                    for (k, arc) in self.arcs.iter().enumerate() {
                        let (a, b) = (latlon(arc.from.0, arc.from.1), latlon(arc.to.0, arc.to.1));
                        let ac = arc.color.unwrap_or_else(|| color::gradient_cyclic(&t.gradient, k as f32 / self.arcs.len().max(1) as f32));
                        let pts = arc_points(a, b, 48);
                        let head = ((time * 0.35 + k as f32 * 0.37) % 1.6) / 1.2;
                        let tail = 0.35;
                        let n = pts.len() - 1;
                        for i in 0..n {
                            let (p0, p1) = (pts[i], pts[i + 1]);
                            if occluded(cam, p0, 1.0) || occluded(cam, p1, 1.0) {
                                continue;
                            }
                            let tt = i as f32 / n as f32;
                            s.line(p0, p1, 1.2, color::with_alpha(ac, if dark { 0.28 } else { 0.4 }));
                            let d = head - tt;
                            if (0.0..tail).contains(&d) {
                                let k2 = 1.0 - d / tail;
                                s.line(p0, p1, 1.0 + 2.2 * k2, glow_c(&t, color::mix(ac, Color32::WHITE, 0.4 * k2), k2));
                                if i % 3 == 0 && dark {
                                    s.glow_point(p0, 0.05 * k2, glow_c(&t, ac, 0.5 * k2));
                                }
                            }
                        }
                        for (end, delay) in [(a, 0.0), (b, 0.5)] {
                            let q = end * 1.004;
                            if occluded(cam, q, 1.0) {
                                continue;
                            }
                            s.point(q, 0.018, ac);
                            let ph = (time * 0.8 + k as f32 * 0.21 + delay).fract();
                            let u = end.any_orthogonal();
                            let v = end.cross(u);
                            let ring = 0.02 + ph * 0.09;
                            let pts: Vec<Vec3> = (0..24).map(|i| q + (u * (i as f32 / 24.0 * TAU).cos() + v * (i as f32 / 24.0 * TAU).sin()) * ring).collect();
                            let rc = color::with_alpha(ac, (1.0 - ph) * 0.9);
                            s.polyline(&pts, 1.2, true, |_| rc);
                        }
                    }
                });
            })
            .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_mask_reads_as_earth() {
        assert!(is_land(40.0, -100.0)); // USA
        assert!(is_land(0.0, 20.0)); // Congo
        assert!(is_land(-25.0, 134.0)); // Australia
        assert!(is_land(50.0, 90.0)); // Siberia
        assert!(!is_land(30.0, -40.0)); // Atlantic
        assert!(!is_land(0.0, -150.0)); // Pacific
        assert!(!is_land(-20.0, 80.0)); // Indian ocean
        let pts = land_points(4000);
        let frac = pts.len() as f32 / 4000.0;
        assert!(frac > 0.2 && frac < 0.4, "land fraction {frac}");
    }

    #[test]
    fn occlusion() {
        let cam = vec3(0.0, 0.0, 4.0);
        assert!(!occluded(cam, vec3(0.0, 0.0, 1.01), 1.0));
        assert!(occluded(cam, vec3(0.0, 0.0, -1.01), 1.0));
    }
}
