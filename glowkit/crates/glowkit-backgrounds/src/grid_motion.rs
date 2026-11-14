//! Grid Motion: a tilted wall of tiles whose rows slide with pointer parallax.

use crate::{background, h01, pointer, Base};
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Ui};
use glowkit_core::{
    color::{mix, with_alpha},
    paint::{fill_convex_fn, linear_fn, rounded_rect_points},
};

/// A tilted wall of rounded tiles in staggered rows. Moving the pointer left and right slides
/// alternate rows in opposite directions, each with its own lag, for a deep parallax feel. With
/// no pointer around the rows drift on their own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::GridMotion;
/// ui.add(GridMotion::new().items(["Rust", "egui", "wasm", "60 fps"]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct GridMotion {
    base: Base,
    items: Vec<String>,
    rows: usize,
    cols: usize,
    angle: f32,
    shift: f32,
    colors: Vec<Color32>,
    autoplay: bool,
}

impl Default for GridMotion {
    fn default() -> Self {
        Self::new()
    }
}

impl GridMotion {
    /// A 5 x 8 wall with placeholder labels.
    pub fn new() -> Self {
        Self {
            base: Base::default(),
            items: ["✦", "Rust", "egui", "◆", "wasm", "60 fps", "glowkit", "★", "GPU", "pure", "native", "●", "UI", "motion"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            rows: 5,
            cols: 8,
            angle: -15.0,
            shift: 1.0,
            colors: Vec::new(),
            autoplay: true,
        }
    }
    /// Tile labels, cycled across the wall.
    pub fn items<S: Into<String>>(mut self, items: impl IntoIterator<Item = S>) -> Self {
        let v: Vec<String> = items.into_iter().map(Into::into).collect();
        if !v.is_empty() {
            self.items = v;
        }
        self
    }
    /// Number of rows (default 5).
    pub fn rows(mut self, n: usize) -> Self {
        self.rows = n.clamp(1, 12);
        self
    }
    /// Tiles per row (default 8).
    pub fn cols(mut self, n: usize) -> Self {
        self.cols = n.clamp(2, 20);
        self
    }
    /// Tilt of the wall in degrees (default -15).
    pub fn angle(mut self, degrees: f32) -> Self {
        self.angle = degrees;
        self
    }
    /// How far rows slide with the pointer, relative (default 1).
    pub fn shift(mut self, k: f32) -> Self {
        self.shift = k.max(0.0);
        self
    }
    /// Accent colors for the highlighted tiles (default: the theme gradient).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Drift on their own when no pointer is around (default true).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Paint the wall into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_grid_motion") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let accents: Vec<Color32> = if self.colors.is_empty() { t.gradient.to_vec() } else { self.colors.clone() };
        // pointer x in -1..1 (or a slow sway)
        let px = match pointer(ui, rect) {
            Some(p) => (p.x - rect.center().x) / (rect.width() * 0.5),
            None if self.autoplay => (f.time * 0.35).sin() * 0.8,
            None => 0.0,
        };
        let center = rect.center();
        let ang = self.angle.to_radians();
        let (sa, ca) = ang.sin_cos();
        let rot = |p: Pos2| -> Pos2 {
            let d = p - center;
            center + vec2(d.x * ca - d.y * sa, d.x * sa + d.y * ca)
        };
        // the unrotated wall is larger than the rect so tilted edges never show
        let wall = Rect::from_center_size(center, rect.size() * vec2(1.35, 1.6));
        let gap = (wall.height() / self.rows as f32 * 0.07).max(6.0);
        let tile_h = (wall.height() - gap * (self.rows as f32 - 1.0)) / self.rows as f32;
        let tile_w = (wall.width() - gap * (self.cols as f32 - 1.0)) / self.cols as f32;
        let max_shift = tile_w * 0.9 * self.shift;
        let p = &f.painter;
        let radius = (tile_h * 0.12).min(18.0);
        for r in 0..self.rows {
            let dir = if r % 2 == 0 { 1.0 } else { -1.0 };
            // each row eases toward its target with its own lag
            let lag = 2.2 + r as f32 * 0.9;
            let target = px * max_shift * dir;
            let shift = glowkit_core::anim::follow(ui, f.id.with(("row", r)), target, lag);
            let y = wall.min.y + r as f32 * (tile_h + gap);
            for c in -1..=self.cols as i32 {
                let x = wall.min.x + c as f32 * (tile_w + gap) + shift;
                let tile = Rect::from_min_size(pos2(x, y), vec2(tile_w, tile_h));
                let outline: Vec<Pos2> = rounded_rect_points(tile, radius, 6).into_iter().map(rot).collect();
                // quick cull
                let bb = Rect::from_points(&outline);
                if !bb.intersects(rect) {
                    continue;
                }
                let idx = (r * (self.cols + 2) + (c + 1) as usize) % self.items.len().max(1);
                let seed = (r * 31 + (c + 7) as usize) as u32;
                let special = h01(seed, 5) < 0.28;
                let accent = accents[(seed as usize) % accents.len()];
                let (top, bottom) = if special {
                    (accent, mix(accent, accents[(seed as usize + 1) % accents.len()], 0.7))
                } else if dark {
                    (mix(t.card, t.foreground, 0.06), t.card)
                } else {
                    (t.muted, mix(t.muted, t.background, 0.5))
                };
                let stops = [top, bottom];
                let lin = linear_fn(tile, 60.0, &stops);
                // gradient evaluated in the unrotated frame
                let inv = |q: Pos2| -> Pos2 {
                    let d = q - center;
                    center + vec2(d.x * ca + d.y * sa, -d.x * sa + d.y * ca)
                };
                fill_convex_fn(p, &outline, 3, |q| lin(inv(q)));
                let border = if special { with_alpha(Color32::WHITE, 0.25) } else { t.border };
                let mut closed = outline.clone();
                closed.push(outline[0]);
                p.add(egui::Shape::line(closed, egui::Stroke::new(1.0, border)));
                let label = &self.items[idx];
                let fg = if special { glowkit_core::color::contrast_text(accent) } else { t.foreground.gamma_multiply(0.85) };
                let size = (tile_h * 0.2).clamp(12.0, 28.0);
                glowkit_core::text::paint_rotated(p, label, rot(tile.center()), FontId::proportional(size), fg, ang);
            }
        }
        // soften the edges into the background
        crate::radial_mask(p, rect, f.bg, 0.75, 1.25);
    }
}

background!(GridMotion);
