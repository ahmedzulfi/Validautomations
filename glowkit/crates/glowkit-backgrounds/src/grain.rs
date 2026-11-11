//! Film grain overlay.

use crate::{background, Base, Fill};
use egui::{pos2, Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui};
use glowkit_core::Rng;

const TEX: usize = 256;

#[derive(Clone, Default)]
struct GrainTex {
    tex: Option<TextureHandle>,
}

/// Animated film grain: a fine, flickering noise overlay that gives flat colors and gradients
/// an analog, printed texture. Transparent by default, so paint it on top of something.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let rect = ui.max_rect();
/// use glowkit_backgrounds::{GrainNoise, MeshGradient};
/// MeshGradient::new().paint(ui, rect);
/// GrainNoise::new().opacity(0.12).paint(ui, rect);
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct GrainNoise {
    base: Base,
    opacity: f32,
    fps: f32,
    scale: f32,
}

impl Default for GrainNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl GrainNoise {
    /// Subtle grain at 12 frames per second.
    pub fn new() -> Self {
        Self { base: Base::with_fill(Fill::None), opacity: 0.1, fps: 12.0, scale: 1.0 }
    }
    /// Grain strength, `0..=1` (default 0.1).
    pub fn opacity(mut self, a: f32) -> Self {
        self.opacity = a.clamp(0.0, 1.0);
        self
    }
    /// How many times per second the grain changes; 0 for static grain (default 12).
    pub fn fps(mut self, fps: f32) -> Self {
        self.fps = fps.max(0.0);
        self
    }
    /// Grain size in points (default 1).
    pub fn grain_size(mut self, s: f32) -> Self {
        self.scale = s.clamp(0.25, 8.0);
        self
    }

    /// Paint the grain into `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_grain_noise") else { return };
        let tex = glowkit_core::with_state(ui, f.id, GrainTex::default, |s| {
            s.tex
                .get_or_insert_with(|| {
                    let mut rng = Rng::new(2024);
                    let pixels = (0..TEX * TEX)
                        .map(|_| {
                            let v = rng.f32();
                            // half the specks lighten, half darken, most are near-neutral
                            let a = ((v - 0.5).abs() * 2.0).powf(1.5);
                            let a8 = (a * 255.0) as u8;
                            if v > 0.5 {
                                Color32::from_rgba_premultiplied(a8, a8, a8, a8)
                            } else {
                                Color32::from_rgba_premultiplied(0, 0, 0, a8)
                            }
                        })
                        .collect();
                    ui.ctx().load_texture("glowkit_grain", ColorImage::new([TEX, TEX], pixels), TextureOptions::NEAREST_REPEAT)
                })
                .clone()
        });
        let frame = if self.fps > 0.0 { (f.time * self.fps).floor() as u64 } else { 0 };
        let mut rng = Rng::new(frame.wrapping_mul(0x9E37) + 1);
        let ppp = ui.ctx().pixels_per_point();
        let texel = self.scale / ppp; // grain size in points
        let span = rect.size() / (TEX as f32 * texel);
        let (u0, v0) = (rng.f32(), rng.f32());
        let uv = Rect::from_min_max(pos2(u0, v0), pos2(u0 + span.x, v0 + span.y));
        f.painter.image(tex.id(), rect, uv, Color32::WHITE.gamma_multiply(self.opacity));
    }
}

background!(GrainNoise);
