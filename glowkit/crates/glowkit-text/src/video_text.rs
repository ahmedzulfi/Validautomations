use crate::raster;
use crate::util;
use egui::{vec2, Color32, Pos2, Rect, Response, Sense, Ui, Widget};
use glowkit_core::{
    color::{gradient, gradient_cyclic, hex},
    noise::{fbm3, noise3},
};

/// The moving picture shown inside [`VideoText`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VideoScene {
    /// Green and violet curtains of northern light drifting over a night sky (default).
    #[default]
    Aurora,
    /// Rolling blue swell with bright foam crests sliding past.
    Ocean,
    /// Molten rock: glowing orange veins churning upward.
    Lava,
    /// Classic looping plasma in the given (or a neon) palette.
    Plasma,
}

/// Huge text that acts as a window onto an animated scene, the way a video can play through
/// the letters of a headline. The scene (aurora, ocean, lava or plasma) is evaluated inside each
/// glyph, so it flows continuously across the whole word.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_text::{VideoText, VideoScene};
/// ui.add(VideoText::new("OCEAN").scene(VideoScene::Ocean).size(180.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VideoText {
    text: String,
    size: f32,
    scene: VideoScene,
    colors: Vec<Color32>,
    speed: f32,
    weight: f32,
}

impl VideoText {
    /// Create the effect for `text` (one big word works best).
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 170.0, scene: VideoScene::Aurora, colors: Vec::new(), speed: 1.0, weight: 1.4 }
    }
    /// Font size in points (default 170).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }
    /// The scene playing inside the letters (default aurora).
    pub fn scene(mut self, scene: VideoScene) -> Self {
        self.scene = scene;
        self
    }
    /// Override the scene's palette (dark to bright for aurora / ocean / lava, any loop for plasma).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Playback speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Extra letter weight, 0..2 (default 1.4): thicker letters show more of the scene.
    pub fn weight(mut self, w: f32) -> Self {
        self.weight = w.clamp(0.0, 2.0);
        self
    }
}

fn pal(custom: &[Color32], default: &[Color32]) -> Vec<Color32> {
    if custom.len() >= 2 {
        custom.to_vec()
    } else {
        default.to_vec()
    }
}

/// Evaluate a scene at normalized `(u, v)` (v down) and aspect-correct `x`.
fn scene_color(scene: VideoScene, stops: &[Color32], u: f32, v: f32, aspect: f32, t: f32) -> Color32 {
    let x = u * aspect;
    match scene {
        VideoScene::Aurora => {
            // night sky gradient, then luminous curtains: bright where folded noise peaks
            let sky = gradient(&stops[..2], v);
            let warp = noise3(x * 0.9, t * 0.12, 3.1) * 1.3;
            let band = (x * 2.2 + warp + t * 0.25).sin() * 0.5 + 0.5;
            let rays = noise3(x * 9.0 + warp * 2.0, t * 0.6, 0.0) * 0.5 + 0.5;
            let curtain = band.powf(1.4) * (0.6 + 0.4 * rays) * (1.0 - v * 0.45);
            let hue = (x * 0.25 + noise3(x * 0.6, v * 1.5, t * 0.1) * 0.6 + 0.5).clamp(0.0, 1.0);
            let glow = gradient(&stops[2..], hue);
            glowkit_core::mix(sky, glow, (curtain * 1.6 + 0.15).clamp(0.0, 1.0))
        }
        VideoScene::Ocean => {
            let swell = fbm3(x * 1.4 - t * 0.35, v * 3.2, t * 0.15, 3);
            let crest = ((x * 5.0 + v * 9.0 + swell * 4.5 - t * 1.6).sin() * 0.5 + 0.5).powf(6.0);
            let depth = (v * 0.8 + swell * 0.35 + 0.1).clamp(0.0, 1.0);
            let water = gradient(&stops[..stops.len() - 1], 1.0 - depth);
            glowkit_core::mix(
                water,
                stops[stops.len() - 1],
                (crest * 0.9 + (swell * 0.5 + 0.5).powf(5.0) * 0.6).min(1.0),
            )
        }
        VideoScene::Lava => {
            let flow = fbm3(x * 1.6, v * 2.2 + t * 0.35, t * 0.18, 4);
            let veins = 1.0 - ((flow * 9.0 + t * 0.5).sin().abs());
            let heat = (veins.powf(3.0) * 0.85 + (1.0 - v) * 0.2 + flow * 0.25).clamp(0.0, 1.0);
            gradient(stops, heat)
        }
        VideoScene::Plasma => {
            let a = (x * 3.0 + t).sin() + (v * 4.0 - t * 1.3).sin() + ((x + v) * 3.0 + t * 0.7).sin();
            let r = ((x - 1.0).powi(2) + (v - 0.5).powi(2)).sqrt();
            let b = (r * 7.0 - t * 1.8).sin();
            gradient_cyclic(stops, (a + b) * 0.125 + t * 0.05)
        }
    }
}

impl Widget for VideoText {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = util::font(self.size, false);
        let galley = util::layout(ui.painter(), self.text.as_str(), &font);
        let bold = util::bold_px(self.size) * (1.0 + self.weight);
        let size = galley.size() + vec2(bold, 0.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let time = glowkit_core::time(ui) * self.speed;
        let stops = match self.scene {
            VideoScene::Aurora => pal(
                &self.colors,
                &[hex("#1E1B4B"), hex("#3B2F7A"), hex("#22D3EE"), hex("#34D399"), hex("#A78BFA"), hex("#F472B6")],
            ),
            VideoScene::Ocean => {
                pal(&self.colors, &[hex("#082F49"), hex("#0369A1"), hex("#0EA5E9"), hex("#67E8F9"), hex("#F0FDFF")])
            }
            VideoScene::Lava => pal(
                &self.colors,
                &[hex("#1C0A05"), hex("#7F1D1D"), hex("#DC2626"), hex("#F97316"), hex("#FDE047"), hex("#FFFBEB")],
            ),
            VideoScene::Plasma => pal(&self.colors, &glowkit_core::color::palettes::NEON[..3]),
        };
        let stops = if self.scene == VideoScene::Aurora && stops.len() < 3 {
            vec![stops[0], stops[0], stops[1]]
        } else {
            stops
        };
        let painter = ui.painter_at(rect.expand(2.0));
        let aspect = rect.width() / rect.height().max(1.0);
        let field_rect: Rect = rect;
        let scene = self.scene;
        let color = |p: Pos2| {
            let u = (p.x - field_rect.min.x) / field_rect.width();
            let v = (p.y - field_rect.min.y) / field_rect.height();
            scene_color(scene, &stops, u, v, aspect, time)
        };
        let step = (self.size / 24.0).max(4.0);
        let n = (bold / 1.2).ceil().max(1.0) as usize;
        let copies: Vec<egui::Vec2> = (0..=n).map(|c| vec2(bold * c as f32 / n as f32, 0.0)).collect();
        raster::paint_galley_field(&painter, &galley, step, |v| rect.min + v, color, &copies);
        resp
    }
}
