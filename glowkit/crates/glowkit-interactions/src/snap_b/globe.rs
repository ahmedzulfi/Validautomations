//! A particle globe: continents drawn as thousands of points on a rotating sphere.

use super::common::{idle_time, mesh_rect, wall_dt};
use egui::epaint::Mesh;
use egui::{vec2, Color32, CursorIcon, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// Rough continent outlines as (longitude, latitude) polygons. Coarse on purpose: the points
/// only need to read as Earth from a distance.
const LAND: &[&[(f32, f32)]] = &[
    // North America
    &[
        (-168.0, 66.0), (-156.0, 71.5), (-130.0, 70.0), (-110.0, 73.0), (-90.0, 74.0), (-75.0, 72.0), (-64.0, 60.0),
        (-56.0, 52.0), (-66.0, 44.5), (-70.0, 41.5), (-76.0, 35.0), (-81.0, 31.0), (-80.0, 25.5), (-82.5, 28.0),
        (-84.0, 30.0), (-90.0, 29.5), (-97.0, 27.5), (-97.5, 22.0), (-94.0, 18.5), (-90.5, 21.0), (-87.0, 21.5),
        (-88.0, 16.0), (-83.5, 11.0), (-79.5, 9.0), (-77.5, 7.5), (-81.0, 7.5), (-86.0, 11.5), (-92.0, 14.5),
        (-105.0, 20.0), (-110.0, 24.0), (-112.5, 30.0), (-117.0, 32.5), (-121.0, 35.0), (-124.0, 41.0),
        (-124.5, 48.0), (-130.0, 54.5), (-137.0, 58.5), (-148.0, 60.5), (-158.0, 57.0), (-164.0, 60.0),
    ],
    // Greenland
    &[(-73.0, 78.0), (-60.0, 82.0), (-32.0, 83.5), (-18.0, 79.0), (-21.0, 70.0), (-42.0, 60.0), (-50.0, 64.0), (-54.0, 70.0), (-62.0, 76.0)],
    // South America
    &[
        (-80.0, 9.0), (-72.0, 12.0), (-62.0, 10.5), (-52.0, 5.0), (-50.0, 0.0), (-44.0, -2.5), (-35.0, -5.5),
        (-35.0, -9.0), (-39.0, -15.0), (-41.0, -22.0), (-48.0, -26.0), (-53.0, -34.0), (-58.0, -38.0), (-62.0, -40.0),
        (-65.0, -45.0), (-68.0, -50.0), (-70.0, -54.5), (-74.0, -52.0), (-74.5, -45.0), (-72.5, -35.0),
        (-71.0, -25.0), (-70.0, -18.0), (-76.0, -14.0), (-81.0, -5.0), (-80.0, 0.0), (-78.0, 5.0),
    ],
    // Africa
    &[
        (-17.0, 21.0), (-13.0, 28.0), (-6.0, 35.5), (10.0, 37.0), (11.0, 33.0), (20.0, 31.0), (32.0, 31.0), (34.0, 28.0),
        (38.0, 18.0), (43.0, 12.0), (51.0, 12.0), (51.0, 10.0), (45.0, 2.0), (40.0, -5.0), (40.0, -15.0), (35.0, -24.0),
        (32.0, -29.0), (27.0, -34.0), (20.0, -35.0), (18.0, -30.0), (14.0, -23.0), (12.0, -17.0), (13.0, -8.0),
        (9.0, -1.0), (9.5, 4.0), (5.0, 6.0), (-4.0, 5.0), (-8.0, 4.5), (-13.0, 8.0), (-17.0, 14.0),
    ],
    // Madagascar
    &[(44.0, -25.0), (47.0, -25.5), (50.5, -15.0), (49.0, -12.0), (44.0, -17.0)],
    // Eurasia
    &[
        (-9.5, 36.5), (-9.0, 43.0), (-2.0, 43.5), (-4.5, 48.0), (2.0, 51.0), (8.0, 54.0), (10.0, 57.5), (5.0, 62.0),
        (14.0, 67.5), (25.0, 71.0), (40.0, 68.0), (60.0, 70.0), (70.0, 73.0), (80.0, 73.0), (100.0, 77.0),
        (115.0, 74.0), (130.0, 72.0), (140.0, 72.5), (160.0, 70.0), (178.0, 69.0), (179.0, 65.0), (170.0, 60.0),
        (162.0, 56.0), (156.0, 51.0), (155.0, 58.0), (142.0, 59.0), (137.0, 54.0), (141.0, 48.0), (135.0, 43.0),
        (129.0, 40.0), (126.5, 35.0), (122.0, 37.0), (121.0, 32.0), (122.0, 30.0), (117.0, 24.0), (110.0, 21.0),
        (108.0, 16.0), (109.0, 12.0), (105.0, 9.0), (103.0, 13.0), (100.0, 13.0), (99.0, 8.0), (103.5, 1.5),
        (100.0, 3.5), (98.0, 8.0), (98.0, 16.0), (94.0, 17.0), (91.0, 22.0), (86.0, 20.0), (80.0, 15.0), (77.0, 8.0),
        (73.0, 17.0), (72.0, 21.0), (67.0, 25.0), (62.0, 25.0), (57.0, 26.0), (56.0, 24.0), (59.5, 22.0),
        (55.0, 17.0), (48.0, 14.0), (43.0, 13.0), (39.0, 21.0), (35.0, 28.0), (34.0, 31.0), (36.0, 36.0),
        (27.0, 37.0), (24.0, 40.0), (20.0, 38.5), (16.0, 38.0), (18.5, 40.5), (13.0, 45.5), (12.0, 44.0), (8.0, 44.0),
        (3.0, 43.0), (-1.0, 37.0), (-6.0, 36.0),
    ],
    // Great Britain and Ireland
    &[(-5.5, 50.0), (1.5, 51.0), (1.8, 53.0), (-2.0, 56.0), (-2.0, 58.5), (-6.0, 58.5), (-5.0, 55.0), (-3.0, 54.0), (-5.0, 52.0)],
    &[(-10.0, 52.0), (-6.0, 52.0), (-6.0, 55.3), (-9.0, 54.5)],
    // Iceland
    &[(-24.0, 64.0), (-14.0, 64.0), (-14.0, 66.5), (-22.0, 66.5)],
    // Japan
    &[(130.0, 31.0), (132.0, 34.0), (136.0, 34.5), (140.0, 35.5), (142.0, 40.0), (141.5, 45.5), (145.5, 44.0), (141.0, 41.0), (140.0, 38.0), (136.0, 36.5), (131.0, 34.5)],
    // Islands of South-East Asia
    &[(95.0, 5.5), (98.0, 4.0), (106.0, -6.0), (102.0, -4.0)],
    &[(109.0, 1.0), (117.0, 7.0), (119.0, 1.0), (116.0, -4.0), (110.0, -3.0)],
    &[(105.0, -6.0), (114.0, -7.0), (114.5, -8.5), (106.0, -7.5)],
    &[(131.0, -1.0), (141.0, -2.5), (150.0, -10.0), (141.0, -9.0), (137.0, -5.0)],
    &[(120.0, 18.5), (122.5, 18.5), (126.0, 7.0), (122.0, 7.0)],
    // Australia
    &[
        (114.0, -22.0), (114.5, -34.0), (118.0, -35.0), (124.0, -34.0), (132.0, -32.0), (138.0, -35.0), (140.0, -38.0),
        (146.0, -39.0), (150.0, -37.0), (153.5, -32.0), (153.0, -25.0), (146.0, -19.0), (142.5, -10.5),
        (141.0, -17.0), (136.0, -12.0), (131.0, -11.0), (126.0, -14.0), (122.0, -18.0),
    ],
    // New Zealand
    &[(172.5, -34.5), (178.5, -37.5), (175.0, -41.5), (171.0, -46.0), (166.5, -46.0), (173.0, -41.0)],
    // Antarctica
    &[(-180.0, -70.0), (180.0, -70.0), (180.0, -90.0), (-180.0, -90.0)],
];

/// Water cut out of the polygons above.
const WATER: &[&[(f32, f32)]] = &[
    // Hudson Bay
    &[(-95.0, 58.5), (-80.0, 55.0), (-77.0, 62.0), (-88.0, 64.5), (-94.5, 62.0)],
    // Black Sea and Caspian
    &[(28.0, 41.0), (41.0, 41.0), (40.0, 46.0), (31.0, 46.5)],
    &[(47.0, 37.0), (54.0, 37.0), (54.0, 46.0), (48.0, 46.5)],
    // Baltic
    &[(10.5, 54.0), (24.0, 54.5), (29.0, 60.0), (22.0, 65.5), (17.0, 61.0)],
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

fn is_land(lon: f32, lat: f32) -> bool {
    LAND.iter().any(|p| in_poly(p, lon, lat)) && !WATER.iter().any(|p| in_poly(p, lon, lat))
}

/// Cached unit-sphere points: (x, y, z, land?, jitter seed).
fn globe_points(ctx: &egui::Context, count: usize) -> Arc<Vec<[f32; 5]>> {
    let key = egui::Id::unique(("snap_b_globe_pts", count));
    if let Some(p) = ctx.data(|d| d.get_temp::<Arc<Vec<[f32; 5]>>>(key)) {
        return p;
    }
    let golden = PI * (3.0 - 5f32.sqrt());
    let mut out = Vec::with_capacity(count / 3);
    for i in 0..count {
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
        let r = (1.0 - y * y).sqrt();
        let th = golden * i as f32;
        let (x, z) = (th.cos() * r, th.sin() * r);
        let lat = (-y).asin().to_degrees();
        let lon = z.atan2(x).to_degrees();
        let land = is_land(lon, lat);
        if land || super::common::hash3(i as i32, 7, 2) < 0.07 {
            out.push([x, y, z, if land { 1.0 } else { 0.0 }, super::common::hash3(i as i32, 3, 1)]);
        }
    }
    let p = Arc::new(out);
    ctx.data_mut(|d| d.insert_temp(key, p.clone()));
    p
}

#[derive(Clone, Copy, Default)]
struct SpinState {
    yaw: f32,
    vel: f32,
    pitch: f32,
}

/// Continents drawn as thousands of individual points on a slowly rotating sphere, the points
/// shimmering and drifting a little as it turns, with a bright limb and a soft atmosphere.
/// Drag to spin it; let go and it keeps its momentum before easing back to a gentle drift.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ParticleGlobe::new().radius(160.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ParticleGlobe {
    radius: f32,
    density: usize,
    speed: f32,
    color: Option<Color32>,
    dot: f32,
}

impl Default for ParticleGlobe {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleGlobe {
    /// A globe of about 12k sample points.
    pub fn new() -> Self {
        Self { radius: 160.0, density: 12_000, speed: 0.18, color: None, dot: 1.6 }
    }
    /// Sphere radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.clamp(40.0, 600.0);
        self
    }
    /// Number of sample points over the whole sphere (about a third land on continents).
    pub fn density(mut self, n: usize) -> Self {
        self.density = n.clamp(1000, 60_000);
        self
    }
    /// Idle spin in radians per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Point color (defaults to the foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Point size in points.
    pub fn dot(mut self, d: f32) -> Self {
        self.dot = d.clamp(0.5, 5.0);
        self
    }
}

impl Widget for ParticleGlobe {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = vec2(ui.available_width().min(self.radius * 2.0 + 160.0), self.radius * 2.0 + 40.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
        let id = response.id;
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        let dt = wall_dt(ui);
        let drag = if response.dragged() { response.drag_delta() } else { Vec2::ZERO };
        let dragging = response.dragged();
        let _ = idle_time(ui, id, response.hovered());
        let r = self.radius;
        let idle_spin = self.speed;
        let st = glowkit_core::with_state(ui, id.with("spin"), || SpinState { yaw: 1.9, vel: 0.0, pitch: 0.38 }, |s| {
            if dragging {
                let dy = drag.x / r;
                s.yaw += dy;
                s.vel = if dt > 0.0 { dy / dt } else { 0.0 };
                s.pitch = (s.pitch + drag.y / r * 0.6).clamp(-0.9, 0.9);
            } else {
                s.vel = glowkit_core::anim::approach(s.vel, idle_spin, 0.8, dt);
                s.yaw += s.vel * dt;
                s.pitch = glowkit_core::anim::approach(s.pitch, 0.38, 0.5, dt);
            }
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let time = glowkit_core::time(ui);
        let painter = ui.painter_at(rect);
        let c = rect.center();
        let ink = self.color.unwrap_or(if t.dark { hex("#EEF2FF") } else { hex("#1A1B22") });
        // atmosphere and a faint body
        paint::glow(&painter, c, r * 1.25, with_alpha(if t.dark { hex("#6A7BFF") } else { hex("#8A96C8") }, 0.10));
        painter.circle_filled(c, r, with_alpha(t.background, 0.9));
        painter.circle_stroke(c, r, egui::Stroke::new(1.0, with_alpha(ink, 0.08)));
        let pts = globe_points(ui.ctx(), self.density);
        let (sy, cy) = st.yaw.sin_cos();
        let (sp, cp) = st.pitch.sin_cos();
        let mut mesh = Mesh::default();
        let d = self.dot;
        for p in pts.iter() {
            // a little drift so the field never looks frozen
            let j = p[4] * TAU;
            let wob = 0.004 * (time * 0.7 + j).sin();
            let (x, y, z) = (p[0] + wob * j.cos(), p[1], p[2] + wob * j.sin());
            // yaw around Y, then pitch around X
            let x1 = x * cy + z * sy;
            let z1 = -x * sy + z * cy;
            let y2 = y * cp - z1 * sp;
            let z2 = y * sp + z1 * cp;
            if z2 > 0.05 {
                continue; // back side
            }
            let facing = (-z2).clamp(0.0, 1.0);
            let limb = (1.0 - facing).powi(3);
            let land = p[3] > 0.5;
            let tw = 0.75 + 0.25 * (time * 2.0 + j * 7.0).sin();
            let a = if land { (0.35 + 0.65 * facing.sqrt()) * tw + limb * 0.5 } else { 0.10 + limb * 0.2 };
            let s = if land { d * (0.7 + 0.5 * facing) } else { d * 0.7 };
            let q = c + vec2(x1, y2) * r;
            mesh_rect(&mut mesh, Rect::from_center_size(q, Vec2::splat(s)), with_alpha(ink, a.min(1.0)));
        }
        painter.add(Shape::mesh(mesh));
        // limb highlight
        paint::arc(&painter, c, r, PI * 1.05, PI * 1.75, 1.6, with_alpha(ink, 0.25));
        response
    }
}
