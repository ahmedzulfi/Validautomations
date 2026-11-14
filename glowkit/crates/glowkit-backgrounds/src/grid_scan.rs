//! Grid Scan: a perspective grid tunnel swept by a glowing scan line.

use crate::{background, ghost, light, pointer, Base};
use egui::{pos2, vec2, Color32, Pos2, Rect, Ui};
use glowkit_core::{
    color::with_alpha,
    paint::{glow_ellipse, gradient_polyline},
};

/// A wireframe tunnel (floor, ceiling and walls ruled with grid lines) receding into the
/// distance, with a glowing scan frame sweeping through it and lighting up the lines it passes.
/// The view leans toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::GridScan;
/// ui.add(GridScan::new().scan_color(glowkit_core::hex("#FF9FFC")));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct GridScan {
    base: Base,
    line_color: Option<Color32>,
    scan_color: Option<Color32>,
    cells: usize,
    depth: usize,
    speed: f32,
    tilt: f32,
}

impl Default for GridScan {
    fn default() -> Self {
        Self::new()
    }
}

impl GridScan {
    /// A grid tunnel with an accent-colored scan.
    pub fn new() -> Self {
        Self { base: Base::default(), line_color: None, scan_color: None, cells: 10, depth: 18, speed: 1.0, tilt: 1.0 }
    }
    /// Grid line color (default: a muted tint of the theme foreground).
    pub fn line_color(mut self, c: Color32) -> Self {
        self.line_color = Some(c);
        self
    }
    /// Scan glow color (default: the theme accent).
    pub fn scan_color(mut self, c: Color32) -> Self {
        self.scan_color = Some(c);
        self
    }
    /// Grid cells across the tunnel's height (default 10).
    pub fn cells(mut self, n: usize) -> Self {
        self.cells = n.clamp(2, 40);
        self
    }
    /// Number of cross-sections drawn into the distance (default 18).
    pub fn depth(mut self, n: usize) -> Self {
        self.depth = n.clamp(4, 60);
        self
    }
    /// Scan speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// How strongly the view leans toward the pointer, 0 = fixed (default 1).
    pub fn tilt(mut self, k: f32) -> Self {
        self.tilt = k.max(0.0);
        self
    }

    /// Paint the tunnel into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_grid_scan") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let line = self.line_color.unwrap_or_else(|| {
            if dark {
                glowkit_core::mix(t.muted_foreground, t.accent, 0.25).gamma_multiply(0.55)
            } else {
                glowkit_core::mix(t.muted_foreground, t.accent, 0.2).gamma_multiply(0.55)
            }
        });
        let scan = self.scan_color.unwrap_or(t.accent);
        // camera leans toward the pointer (springy)
        let target = pointer(ui, rect).unwrap_or_else(|| ghost(rect, f.time * 0.5));
        let aim = (target - rect.center()) / (rect.size() * 0.5);
        let lean = glowkit_core::anim::spring2(ui, f.id.with("lean"), aim * self.tilt, 40.0, 12.0);
        let p = &f.painter;
        let half = rect.size() * 0.5;
        // world: tunnel cross-section spans [-aspect, aspect] x [-1, 1]; camera at z = 0 looking
        // down +z; the opening (z = 1) exactly fills the rect
        let aspect = half.x / half.y.max(1.0);
        let cam = vec2(-lean.x * 0.35 * aspect, -lean.y * 0.3);
        let proj = |x: f32, y: f32, z: f32| -> Pos2 {
            pos2(rect.center().x + (x - cam.x) / z * half.y, rect.center().y + (y - cam.y) / z * half.y)
        };
        let z_far = 1.0 + self.depth as f32 * 0.6;
        let fog = |z: f32| (1.0 - (z - 1.0) / (z_far - 1.0)).clamp(0.0, 1.0).powf(1.4);
        // scan position ping-pongs through depth, uniform in screen space (1/z)
        let period = 4.2 / self.speed.max(0.05);
        let ph = (f.time / period).rem_euclid(2.0);
        let s = if ph < 1.0 { ph } else { 2.0 - ph };
        let s = s * s * (3.0 - 2.0 * s);
        let inv = 1.0 - s * (1.0 - 1.0 / z_far);
        let zs = 1.0 / inv;
        let near_scan = |z: f32| {
            let d = (1.0 / z - 1.0 / zs) * 8.0;
            (-d * d).exp()
        };
        // soft glow of the scan frame in the distance
        let ring_center = proj(0.0, 0.0, zs);
        let ring_half = vec2(aspect, 1.0) * half.y / zs;
        glow_ellipse(p, ring_center, ring_half * 1.25, light(scan, 0.18 * fog(zs).max(0.3), dark));
        // longitudinal lines (along depth) on the four faces
        let n = self.cells as i32;
        let cell = 2.0 / n as f32;
        let nx = (aspect / cell).ceil() as i32;
        let seg = 14;
        let long = |a: (f32, f32), w: f32| {
            let pts: Vec<Pos2> = (0..=seg)
                .map(|i| {
                    let u = i as f32 / seg as f32;
                    let z = 1.0 / (1.0 - u * (1.0 - 1.0 / z_far));
                    proj(a.0, a.1, z)
                })
                .collect();
            gradient_polyline(p, &pts, false, w, |u| {
                let z = 1.0 / (1.0 - u * (1.0 - 1.0 / z_far));
                let k = fog(z);
                let hot = near_scan(z);
                glowkit_core::mix(line.gamma_multiply(k), scan, hot * 0.85).gamma_multiply((k + hot * 0.6).min(1.0))
            });
        };
        for i in -nx..=nx {
            let x = (i as f32 * cell).clamp(-aspect, aspect);
            long((x, 1.0), 1.0);
            long((x, -1.0), 1.0);
        }
        for j in -n / 2..=n / 2 {
            let y = j as f32 * cell;
            long((-aspect, y), 1.0);
            long((aspect, y), 1.0);
        }
        // cross-sections (rectangles at fixed depths)
        let rect_at = |z: f32| {
            let a = proj(-aspect, -1.0, z);
            let b = proj(aspect, 1.0, z);
            Rect::from_min_max(a, b)
        };
        for k in 0..=self.depth {
            let z = 1.0 + k as f32 * 0.6;
            let a = fog(z);
            if a < 0.02 {
                continue;
            }
            let r = rect_at(z);
            let hot = near_scan(z);
            let c = glowkit_core::mix(line, scan, hot * 0.8).gamma_multiply((a + hot * 0.5).min(1.0));
            p.rect_stroke(r, 0.0, egui::Stroke::new(1.0, c), egui::StrokeKind::Middle);
        }
        // the scan frame itself: bright core and a wide soft halo
        let r = rect_at(zs);
        let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        let pulse = 0.85 + 0.15 * (f.time * 7.0).sin();
        for (w, a) in [(18.0, 0.07), (9.0, 0.14), (4.0, 0.35)] {
            p.add(egui::Shape::line(corners.to_vec(), egui::Stroke::new(w / zs.sqrt(), light(scan, a * pulse, dark))));
        }
        let core = if dark { glowkit_core::mix(scan, Color32::WHITE, 0.55) } else { scan };
        p.add(egui::Shape::line(corners.to_vec(), egui::Stroke::new(1.6, core)));
        // a light sheet trailing the scan on the floor
        let floor_a = proj(-aspect, 1.0, zs);
        let floor_b = proj(aspect, 1.0, zs);
        glow_ellipse(
            p,
            floor_a.lerp(floor_b, 0.5),
            vec2((floor_b.x - floor_a.x) * 0.5, 14.0 / zs.sqrt()),
            light(scan, 0.35, dark),
        );
        // vignette into the background
        let clear = with_alpha(f.bg, 0.0);
        let edge = rect.height() * 0.18;
        glowkit_core::paint::quad(
            p,
            [
                rect.left_top(),
                rect.right_top(),
                pos2(rect.max.x, rect.min.y + edge),
                pos2(rect.min.x, rect.min.y + edge),
            ],
            [f.bg, f.bg, clear, clear],
        );
        glowkit_core::paint::quad(
            p,
            [
                pos2(rect.min.x, rect.max.y - edge),
                pos2(rect.max.x, rect.max.y - edge),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            [clear, clear, f.bg, f.bg],
        );
    }
}

background!(GridScan);
