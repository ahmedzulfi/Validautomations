use super::util::{bold_text, idle_time, mesh_dot};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::{color::with_alpha, mix, noise::fbm3, paint, Theme};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

/// A 3D vector helper (tiny, private).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl V3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn from_lat_lon(lat: f32, lon: f32) -> Self {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        Self::new(la.cos() * lo.sin(), la.sin(), la.cos() * lo.cos())
    }
    /// Rotate around Y by `yaw`, then around X by `pitch`.
    pub fn rotate(self, yaw: f32, pitch: f32) -> Self {
        let (sy, cy) = yaw.sin_cos();
        let x = self.x * cy + self.z * sy;
        let z = -self.x * sy + self.z * cy;
        let (sp, cp) = pitch.sin_cos();
        let y = self.y * cp - z * sp;
        let z2 = self.y * sp + z * cp;
        Self::new(x, y, z2)
    }
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
    pub fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn norm(self) -> Self {
        let l = self.dot(self).sqrt().max(1e-6);
        self.scale(1.0 / l)
    }
}

/// `n` evenly spread points on the unit sphere (Fibonacci lattice).
pub(crate) fn fibonacci(n: usize) -> Vec<V3> {
    let golden = PI * (3.0 - 5f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let phi = i as f32 * golden;
            V3::new(phi.cos() * r, y, phi.sin() * r)
        })
        .collect()
}

/// Shared drag-to-spin state with inertia.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spin {
    pub yaw: f32,
    pub pitch: f32,
    pub vel: Vec2,
}

pub(crate) fn update_spin(ui: &Ui, resp: &Response, id: egui::Id, auto: Vec2, pitch0: f32, sensitivity: f32) -> Spin {
    let dt = glowkit_core::dt(ui);
    let drag = resp.drag_delta();
    let dragging = resp.dragged();
    glowkit_core::with_state(
        ui,
        id.with("spin"),
        || Spin { yaw: 0.0, pitch: pitch0, vel: Vec2::ZERO },
        |s| {
            if dragging {
                let v = drag * sensitivity / dt.max(1e-3);
                s.vel = s.vel * 0.5 + v * 0.5;
                s.yaw += drag.x * sensitivity;
                s.pitch = (s.pitch + drag.y * sensitivity).clamp(-1.2, 1.2);
            } else {
                s.vel *= (-2.5 * dt).exp();
                s.yaw += (s.vel.x + auto.x) * dt;
                s.pitch = (s.pitch + s.vel.y * dt).clamp(-1.2, 1.2);
                // drift back toward the resting tilt
                s.pitch +=
                    (pitch0 - s.pitch) * (1.0 - (-0.8 * dt).exp()) * if s.vel.length() < 0.3 { 1.0 } else { 0.0 };
            }
            *s
        },
    )
}

/// A marker on the [`Globe`], at a latitude / longitude in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeMarker {
    /// Latitude in degrees (north positive).
    pub lat: f32,
    /// Longitude in degrees (east positive).
    pub lon: f32,
}

impl GlobeMarker {
    /// A marker at `lat`, `lon` (degrees).
    pub const fn new(lat: f32, lon: f32) -> Self {
        Self { lat, lon }
    }
}

/// A rotating dotted globe: a Fibonacci lattice of "land" dots (back face dimmed), a soft rim
/// glow, pulsing markers and animated arcs that fly between them. Drag to spin it; it keeps
/// turning with inertia.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{Globe, GlobeMarker};
/// let sf = GlobeMarker::new(37.7, -122.4);
/// let tokyo = GlobeMarker::new(35.7, 139.7);
/// ui.add(Globe::new().radius(160.0).markers(&[sf, tokyo]).arcs(&[(0, 1)]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Globe {
    radius: f32,
    dots: usize,
    markers: Vec<GlobeMarker>,
    arcs: Vec<(usize, usize)>,
    dot_color: Option<Color32>,
    marker_color: Option<Color32>,
    arc_colors: Option<[Color32; 2]>,
    glow: Option<Color32>,
    speed: f32,
    tilt: f32,
    land_only: bool,
}

impl Default for Globe {
    fn default() -> Self {
        Self::new()
    }
}

impl Globe {
    /// A globe with default size and no markers.
    pub fn new() -> Self {
        Self {
            radius: 150.0,
            dots: 2600,
            markers: Vec::new(),
            arcs: Vec::new(),
            dot_color: None,
            marker_color: None,
            arc_colors: None,
            glow: None,
            speed: 0.25,
            tilt: 0.3,
            land_only: true,
        }
    }
    /// Sphere radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Number of lattice points sampled (before the land mask).
    pub fn dots(mut self, n: usize) -> Self {
        self.dots = n.clamp(100, 8000);
        self
    }
    /// Markers to pin on the globe.
    pub fn markers(mut self, m: &[GlobeMarker]) -> Self {
        self.markers = m.to_vec();
        self
    }
    /// Arcs between markers, as pairs of marker indices.
    pub fn arcs(mut self, a: &[(usize, usize)]) -> Self {
        self.arcs = a.to_vec();
        self
    }
    /// Dot color (defaults to the foreground).
    pub fn dot_color(mut self, c: Color32) -> Self {
        self.dot_color = Some(c);
        self
    }
    /// Marker color (defaults to the accent).
    pub fn marker_color(mut self, c: Color32) -> Self {
        self.marker_color = Some(c);
        self
    }
    /// Arc gradient colors (defaults to the theme gradient).
    pub fn arc_colors(mut self, a: Color32, b: Color32) -> Self {
        self.arc_colors = Some([a, b]);
        self
    }
    /// Rim glow color (defaults to the accent).
    pub fn glow(mut self, c: Color32) -> Self {
        self.glow = Some(c);
        self
    }
    /// Idle spin speed in radians per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Resting tilt toward the viewer, in radians.
    pub fn tilt(mut self, t: f32) -> Self {
        self.tilt = t;
        self
    }
    /// Only draw dots over (procedural) land; `false` draws the whole lattice.
    pub fn land_only(mut self, b: bool) -> Self {
        self.land_only = b;
        self
    }
}

type Lattice = Arc<Vec<(V3, f32)>>;

impl Widget for Globe {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = Vec2::splat(self.radius * 2.0 + 40.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::drag());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let id = resp.id;
        let spin = update_spin(ui, &resp, id, vec2(self.speed, 0.0), self.tilt, 0.008);
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let land_only = self.land_only;
        let n = self.dots;
        let lattice: Lattice = glowkit_core::with_state(
            ui,
            id.with(("lattice", n, land_only)),
            || {
                Arc::new(
                    fibonacci(n)
                        .into_iter()
                        .filter_map(|p| {
                            let v = fbm3(p.x * 1.7 + 3.1, p.y * 1.7 - 1.3, p.z * 1.7 + 0.7, 4);
                            let polar = p.y.abs() > 0.88;
                            let land = v > 0.02 && !polar;
                            if land || !land_only {
                                Some((p, ((v - 0.02) * 6.0).clamp(0.0, 1.0)))
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            },
            |l: &mut Lattice| l.clone(),
        );

        let time = glowkit_core::time(ui);
        let c = rect.center();
        let r = self.radius;
        let p = ui.painter_at(rect.expand(30.0));
        let accent = self.glow.unwrap_or(t.accent);
        let dot = self.dot_color.unwrap_or(if t.dark {
            mix(t.foreground, accent, 0.15)
        } else {
            mix(t.foreground, accent, 0.25)
        });
        let marker = self.marker_color.unwrap_or(t.accent);
        let arc_cols = self.arc_colors.unwrap_or([t.gradient[0], t.gradient[1]]);

        // halo + body
        paint::glow(&p, c, r * 1.45, with_alpha(accent, if t.dark { 0.20 } else { 0.12 }));
        let body_hi = if t.dark { mix(t.background, accent, 0.10) } else { mix(Color32::WHITE, accent, 0.05) };
        let body_lo =
            if t.dark { mix(t.background, Color32::BLACK, 0.3) } else { mix(t.background, t.muted_foreground, 0.12) };
        let light = c + vec2(-r * 0.35, -r * 0.45);
        let pts = paint::circle_points(c, r, 96);
        paint::fill_convex_fn(&p, &pts, 8, |q| {
            let k = (q.distance(light) / (r * 1.7)).clamp(0.0, 1.0);
            body_hi.lerp_to_gamma(body_lo, k)
        });
        // rim light
        paint::gradient_polyline(&p, &pts, true, 1.5, |u| {
            let a = u * TAU;
            let k = 0.5 + 0.5 * (a + 2.4).cos();
            with_alpha(accent, 0.15 + 0.55 * k)
        });

        // dots
        let mut m = Mesh::default();
        let proj = |v: V3| pos2(c.x + v.x * r, c.y - v.y * r);
        let dr = (r / 150.0).clamp(0.6, 1.6);
        for (v, strength) in lattice.iter() {
            let q = v.rotate(spin.yaw, spin.pitch);
            if q.z < 0.0 {
                if q.z > -0.95 {
                    m_dot(&mut m, proj(q), 0.8 * dr, with_alpha(dot, 0.07));
                }
                continue;
            }
            let face = q.z; // 0 at rim, 1 facing us
            let a = (0.25 + 0.75 * face.powf(0.6)) * (0.55 + 0.45 * strength);
            m_dot(&mut m, proj(q), (0.9 + 0.7 * face) * dr, with_alpha(dot, a));
        }
        p.add(Shape::mesh(m));

        let visible = |v: V3| {
            v.z >= 0.0 || {
                let d = vec2(v.x, v.y).length();
                d > 1.0
            }
        };

        // arcs
        for (k, &(a, b)) in self.arcs.iter().enumerate() {
            let (Some(ma), Some(mb)) = (self.markers.get(a), self.markers.get(b)) else { continue };
            let va = V3::from_lat_lon(ma.lat, ma.lon);
            let vb = V3::from_lat_lon(mb.lat, mb.lon);
            let ang = va.dot(vb).clamp(-1.0, 1.0).acos();
            let lift = 0.04 + ang * 0.09;
            let steps = 48;
            let period = 3.2;
            let phase = ((time + k as f32 * 0.9) / period).fract();
            let head = phase * 1.5;
            let pts3: Vec<V3> = (0..=steps)
                .map(|i| {
                    let u = i as f32 / steps as f32;
                    let s = slerp(va, vb, ang, u).scale(1.0 + lift * (PI * u).sin());
                    s.rotate(spin.yaw, spin.pitch)
                })
                .collect();
            // split into visible runs
            let mut run: Vec<(Pos2, f32)> = Vec::new();
            let mut runs = Vec::new();
            for (i, q) in pts3.iter().enumerate() {
                if visible(*q) {
                    run.push((proj(*q), i as f32 / steps as f32));
                } else if run.len() > 1 {
                    runs.push(std::mem::take(&mut run));
                } else {
                    run.clear();
                }
            }
            if run.len() > 1 {
                runs.push(run);
            }
            for run in runs {
                let pts: Vec<Pos2> = run.iter().map(|x| x.0).collect();
                let u0 = run[0].1;
                let u1 = run[run.len() - 1].1;
                // faint full path
                paint::gradient_polyline(&p, &pts, false, 1.0, |_| with_alpha(arc_cols[0], 0.18));
                // comet
                paint::gradient_polyline(&p, &pts, false, 2.2, |s| {
                    let u = u0 + (u1 - u0) * s;
                    let d = head - u;
                    if (0.0..0.45).contains(&d) {
                        let k = 1.0 - d / 0.45;
                        with_alpha(arc_cols[0].lerp_to_gamma(arc_cols[1], u), k * k)
                    } else {
                        Color32::TRANSPARENT
                    }
                });
                if (u0..=u1).contains(&head) {
                    let i = ((head - u0) / (u1 - u0).max(1e-4) * (pts.len() - 1) as f32).round() as usize;
                    let hp = pts[i.min(pts.len() - 1)];
                    paint::glow(&p, hp, 10.0, with_alpha(arc_cols[1], 0.6));
                    p.circle_filled(hp, 1.8, Color32::WHITE);
                }
            }
        }

        // markers
        for (i, mk) in self.markers.iter().enumerate() {
            let q = V3::from_lat_lon(mk.lat, mk.lon).rotate(spin.yaw, spin.pitch);
            if q.z < 0.0 {
                continue;
            }
            let pp = proj(q);
            let f = q.z.powf(0.5);
            let pulse = ((time * 0.8 + i as f32 * 0.37).fract()) as f32;
            p.circle_stroke(pp, 3.0 + pulse * 12.0, Stroke::new(1.2, with_alpha(marker, (1.0 - pulse) * 0.7 * f)));
            paint::glow(&p, pp, 9.0, with_alpha(marker, 0.5 * f));
            p.circle_filled(pp, 3.2, with_alpha(marker, f));
            p.circle_filled(pp, 1.3, with_alpha(Color32::WHITE, f));
        }
        resp
    }
}

fn m_dot(m: &mut Mesh, c: Pos2, r: f32, col: Color32) {
    mesh_dot(m, c, r, col);
}

fn slerp(a: V3, b: V3, ang: f32, u: f32) -> V3 {
    if ang < 1e-4 {
        return a;
    }
    let s = ang.sin();
    a.scale(((1.0 - u) * ang).sin() / s).add(b.scale((u * ang).sin() / s)).norm()
}

/// One item in an [`IconCloud`]: a short glyph (1-3 characters or an emoji) on a colored tile.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudIcon {
    /// Text drawn on the tile.
    pub glyph: String,
    /// Tile color.
    pub color: Color32,
}

impl CloudIcon {
    /// A tile showing `glyph` on `color`.
    pub fn new(glyph: impl Into<String>, color: Color32) -> Self {
        Self { glyph: glyph.into(), color }
    }
}

/// A sphere of icon tiles rotating in 3D: tiles are scaled and faded by depth, the cloud
/// drifts toward the pointer and can be flicked with a drag.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::{CloudIcon, IconCloud};
/// use glowkit_core::hex;
/// ui.add(IconCloud::new(vec![CloudIcon::new("Rs", hex("#F74C00")), CloudIcon::new("Go", hex("#00ADD8"))]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct IconCloud {
    icons: Vec<CloudIcon>,
    radius: f32,
    tile: f32,
    speed: f32,
}

impl IconCloud {
    /// A cloud made of `icons` (they are spread evenly over the sphere).
    pub fn new(icons: Vec<CloudIcon>) -> Self {
        Self { icons, radius: 150.0, tile: 40.0, speed: 0.35 }
    }
    /// Sphere radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Tile size (at the front of the sphere) in points.
    pub fn tile(mut self, s: f32) -> Self {
        self.tile = s;
        self
    }
    /// Idle spin speed in radians per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
}

impl Widget for IconCloud {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = Vec2::splat(self.radius * 2.0 + self.tile * 1.5);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::drag());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let c = rect.center();
        // drift toward the pointer, like the original: the further from center, the faster
        let auto = match resp.hover_pos() {
            Some(hp) if !resp.dragged() => {
                let d = (hp - c) / (self.radius);
                vec2(self.speed + d.x * 0.6, 0.0)
            }
            _ => vec2(self.speed, 0.0),
        };
        let idle = idle_time(ui, resp.id, resp.hovered());
        let wobble = if idle > 1.0 { (glowkit_core::time(ui) * 0.4).sin() * 0.25 } else { 0.0 };
        let spin = update_spin(ui, &resp, resp.id, auto, 0.25 + wobble, 0.01);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let p = ui.painter_at(rect.expand(20.0));
        paint::glow(&p, c, self.radius * 1.2, with_alpha(t.accent, if t.dark { 0.10 } else { 0.06 }));
        let pts = fibonacci(self.icons.len().max(1));
        let mut items: Vec<(f32, Pos2, usize)> = pts
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let q = v.rotate(spin.yaw, spin.pitch);
                (q.z, pos2(c.x + q.x * self.radius, c.y - q.y * self.radius), i)
            })
            .collect();
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (z, pos, i) in items {
            let ic = &self.icons[i];
            let depth = (z + 1.0) / 2.0; // 0 back .. 1 front
            let s = self.tile * (0.45 + 0.55 * depth);
            let alpha = 0.18 + 0.82 * depth.powf(1.4);
            let r = Rect::from_center_size(pos, Vec2::splat(s));
            let cr = s * 0.24;
            let top = mix(ic.color, Color32::WHITE, 0.18);
            let bot = mix(ic.color, Color32::BLACK, 0.18);
            if depth > 0.6 {
                paint::soft_shadow(
                    &p,
                    r,
                    cr,
                    10.0,
                    vec2(0.0, 3.0),
                    Color32::from_black_alpha((60.0 * (depth - 0.6) * 2.5) as u8),
                );
            }
            let outline = paint::rounded_rect_points(r, cr, 5);
            paint::fill_convex_fn(&p, &outline, 1, |q| {
                let k = ((q.y - r.min.y) / r.height()).clamp(0.0, 1.0);
                // fade toward the page background (looks like depth-of-field haze)
                with_alpha(top.lerp_to_gamma(bot, k), alpha)
            });
            let fg = glowkit_core::color::contrast_text(ic.color);
            let fs = s * if ic.glyph.chars().count() > 2 { 0.34 } else { 0.44 };
            bold_text(
                &p,
                r.center(),
                Align2::CENTER_CENTER,
                &ic.glyph,
                FontId::proportional(fs),
                with_alpha(fg, alpha),
            );
        }
        resp
    }
}
