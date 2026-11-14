//! Material headlines: molten metal, liquid chrome, film-grain gradient and ordered-dither ink.
//!
//! Each one rasterises its text once into a cached coverage mask, then paints that mask as a
//! fine mesh whose vertices are shaded every frame, so the fill is a real per-pixel-ish material
//! (with bevel normals derived from the glyph shapes) rather than a flat text color.

use super::common::{
    bayer4, display_font, lerp_c, lighten, mask_glow, paint_mask_cells, paint_mask_smooth, ramp, scale_rgb, text_mask,
};
use egui::{vec2, Color32, FontId, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::noise::{fbm3, noise3};
use glowkit_core::Theme;

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    glowkit_core::anim::smoothstep(e0, e1, x)
}

/// Shared builder state of the material headlines.
#[derive(Clone, Debug)]
struct Head {
    text: String,
    size: f32,
    font: Option<FontId>,
    speed: f32,
}

impl Head {
    fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 104.0, font: None, speed: 1.0 }
    }
    fn font(&self, ui: &Ui) -> FontId {
        self.font.clone().unwrap_or_else(|| display_font(ui, self.size))
    }
}

macro_rules! head_setters {
    () => {
        /// Font size in points (ignored when [`Self::font`] is set).
        pub fn size(mut self, size: f32) -> Self {
            self.head.size = size.clamp(12.0, 400.0);
            self
        }
        /// Use a specific font (defaults to the heaviest installed display face).
        pub fn font(mut self, font: FontId) -> Self {
            self.head.font = Some(font);
            self
        }
        /// Animation speed multiplier (0 freezes it).
        pub fn speed(mut self, speed: f32) -> Self {
            self.head.speed = speed.max(0.0);
            self
        }
    };
}

fn place(ui: &mut Ui, size: Vec2) -> (egui::Rect, Response) {
    ui.allocate_exact_size(size, Sense::hover())
}

// ---------------------------------------------------------------------------------------------

/// A headline filled with flowing molten metal: domain-warped noise churns through a lava ramp
/// (dark crust, cherry red, orange, white-hot), the letter edges are bevelled and catch light,
/// and a warm heat haze glows around the type.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::MoltenText::new("Molten.").size(110.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MoltenText {
    head: Head,
    heat: f32,
    bevel: f32,
    palette: Option<[Color32; 5]>,
}

impl MoltenText {
    /// Molten headline showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { head: Head::new(text), heat: 0.6, bevel: 1.0, palette: None }
    }
    head_setters!();
    /// Heat: how bright and white-hot the flow runs, `0..=1`.
    pub fn heat(mut self, heat: f32) -> Self {
        self.heat = heat.clamp(0.0, 1.0);
        self
    }
    /// Bevel strength of the letter edges, `0..=2`.
    pub fn bevel(mut self, bevel: f32) -> Self {
        self.bevel = bevel.clamp(0.0, 2.0);
        self
    }
    /// Five-stop ramp from coolest crust to white-hot.
    pub fn palette(mut self, ramp: [Color32; 5]) -> Self {
        self.palette = Some(ramp);
        self
    }
}

impl Widget for MoltenText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = self.head.font(ui);
        let mask = text_mask(ui.ctx(), &self.head.text, &font, false);
        let (rect, response) = place(ui, mask.size + vec2(24.0, 24.0));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) * self.head.speed;
        let painter = ui.painter_at(rect.expand(40.0));
        let pal = self.palette.unwrap_or([
            hex("#1E0400"),
            hex("#7A1000"),
            hex("#D93A00"),
            hex("#FF8A1E"),
            hex("#FFE7A8"),
        ]);
        let origin = rect.center() - mask.size / 2.0;
        let glow_a = if t.dark { 0.38 } else { 0.14 };
        mask_glow(&painter, &mask, rect.center(), font.size * 0.12, with_alpha(pal[3], glow_a));
        let k = 2.4 / font.size;
        let sz = mask.size;
        let heat = self.heat;
        let bevel = self.bevel;
        let cell = (font.size / 44.0).clamp(1.4, 3.0);
        paint_mask_smooth(&painter, &mask, origin, cell, |uv, _| {
            let (x, y) = (uv.x * sz.x * k, uv.y * sz.y * k);
            let tt = time * 0.22;
            let qx = fbm3(x + 0.0, y + 0.0, tt, 3);
            let qy = fbm3(x + 5.2, y + 1.3, tt + 3.1, 3);
            let n = fbm3(x + 1.9 * qx + tt * 0.6, y + 1.9 * qy - tt * 0.9, tt * 0.5, 4);
            let mut h = (0.5 + 0.85 * n + 0.18 * qx).clamp(0.0, 1.0);
            h = (h * (0.75 + 0.45 * heat)).clamp(0.0, 1.0);
            let base = ramp(&pal, h);
            let nrm = mask.normal(uv.x, uv.y) * bevel;
            let lit = nrm.dot(vec2(-0.55, -0.83));
            let soft = mask.height(uv.x, uv.y);
            let rim = smooth(0.95, 0.45, soft);
            let mut c = if lit > 0.0 { lighten(base, (lit * 0.9).min(0.8)) } else { scale_rgb(base, 1.0 + lit * 1.1) };
            c = lerp_c(c, lighten(pal[3], 0.35), rim * 0.35 * bevel.min(1.0));
            c
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A headline in liquid chrome: an environment of sky, a hard dark horizon and warm ground is
/// reflected in bevelled letters, the reflection bends as the surface ripples, and a specular
/// glint sweeps across every few seconds.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::ChromeText::new("Chrome."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ChromeText {
    head: Head,
    ripple: f32,
    glint_every: f32,
    tint: Option<Color32>,
}

impl ChromeText {
    /// Chrome headline showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { head: Head::new(text), ripple: 1.0, glint_every: 3.2, tint: None }
    }
    head_setters!();
    /// How strongly the liquid surface bends the reflection, `0..=3`.
    pub fn ripple(mut self, ripple: f32) -> Self {
        self.ripple = ripple.clamp(0.0, 3.0);
        self
    }
    /// Seconds between glint sweeps.
    pub fn glint_every(mut self, secs: f32) -> Self {
        self.glint_every = secs.max(0.8);
        self
    }
    /// Tint the metal (e.g. gold or rose); default is neutral steel.
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
}

fn chrome_env(r: f32) -> Color32 {
    // sky -> bright horizon haze -> hard dark horizon -> warm ground -> dark ground
    const STOPS: [(f32, [u8; 3]); 9] = [
        (0.00, [92, 104, 124]),
        (0.18, [176, 190, 210]),
        (0.36, [236, 242, 250]),
        (0.45, [255, 255, 255]),
        (0.52, [34, 36, 42]),
        (0.58, [70, 64, 60]),
        (0.70, [150, 140, 130]),
        (0.86, [62, 58, 56]),
        (1.00, [22, 22, 26]),
    ];
    let r = r.clamp(0.0, 1.0);
    let mut i = 0;
    while i + 2 < STOPS.len() && r > STOPS[i + 1].0 {
        i += 1;
    }
    let (a, ca) = STOPS[i];
    let (b, cb) = STOPS[i + 1];
    let f = ((r - a) / (b - a).max(1e-4)).clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f) as u8;
    Color32::from_rgb(l(ca[0], cb[0]), l(ca[1], cb[1]), l(ca[2], cb[2]))
}

impl Widget for ChromeText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = self.head.font(ui);
        let mask = text_mask(ui.ctx(), &self.head.text, &font, false);
        let (rect, response) = place(ui, mask.size + vec2(24.0, 24.0));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) * self.head.speed;
        let painter = ui.painter_at(rect.expand(40.0));
        let origin = rect.center() - mask.size / 2.0;
        // a faint cool reflection halo / contact shadow
        let halo = if t.dark { with_alpha(hex("#C8D4E8"), 0.10) } else { with_alpha(hex("#1A1D26"), 0.16) };
        mask_glow(&painter, &mask, rect.center() + vec2(0.0, font.size * 0.05), font.size * 0.07, halo);
        let k = 1.8 / font.size;
        let sz = mask.size;
        let ripple = self.ripple;
        let period = self.glint_every;
        let glint_pos = (time % period) / period * 1.8 - 0.4;
        let tint = self.tint;
        let cell = (font.size / 72.0).clamp(1.0, 2.0);
        paint_mask_smooth(&painter, &mask, origin, cell, |uv, _| {
            let (x, y) = (uv.x * sz.x * k, uv.y * sz.y * k);
            let nrm = mask.normal(uv.x, uv.y);
            let wob = noise3(x * 1.3, y * 1.3, time * 0.35) * 0.07 * ripple
                + noise3(x * 3.1 + 4.0, y * 2.7, time * 0.5) * 0.025 * ripple;
            let r = 0.12 + uv.y * 0.72 + nrm.y * 0.42 + nrm.x * 0.12 + wob + (time * 0.4).sin() * 0.025;
            let mut c = chrome_env(r);
            // fine brushed banding
            let band = (r * 38.0).sin() * 0.04;
            c = scale_rgb(c, 1.0 + band);
            // bevel: lit edges toward the top-left, darker toward bottom-right
            let lit = nrm.dot(vec2(-0.6, -0.8));
            if lit > 0.0 {
                c = lighten(c, (lit * 0.7).min(0.75));
            } else {
                c = scale_rgb(c, 1.0 + lit * 0.6);
            }
            // sweeping glint
            let g = uv.x + (uv.y - 0.5) * 0.45 - glint_pos;
            let spec = (-(g / 0.035).powi(2)).exp() * (0.4 + 0.6 * mask.height(uv.x, uv.y));
            c = lighten(c, spec * 0.9);
            if let Some(tc) = tint {
                let m = Color32::from_rgb(
                    ((c.r() as u16 * tc.r() as u16) / 255) as u8,
                    ((c.g() as u16 * tc.g() as u16) / 255) as u8,
                    ((c.b() as u16 * tc.b() as u16) / 255) as u8,
                );
                c = lerp_c(c, m, 0.75);
            }
            c
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A headline filled with a slow, soft gradient of light under real film grain: the colors drift
/// through the letters like light through frosted glass, and every pixel carries static grain.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GrainText::new("Grainy."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GrainText {
    head: Head,
    colors: Option<Vec<Color32>>,
    drift: f32,
}

impl GrainText {
    /// Grain headline showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { head: Head::new(text), colors: None, drift: 1.0 }
    }
    head_setters!();
    /// Gradient stops (defaults to a warm orange, rose and violet dusk).
    pub fn colors(mut self, stops: impl Into<Vec<Color32>>) -> Self {
        let v: Vec<Color32> = stops.into();
        if !v.is_empty() {
            self.colors = Some(v);
        }
        self
    }
    /// How far the light wanders, `0..=2`.
    pub fn drift(mut self, drift: f32) -> Self {
        self.drift = drift.clamp(0.0, 2.0);
        self
    }
}

impl Widget for GrainText {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = self.head.font(ui);
        let mask = text_mask(ui.ctx(), &self.head.text, &font, true);
        let (rect, response) = place(ui, mask.size + vec2(24.0, 24.0));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) * self.head.speed;
        let painter = ui.painter_at(rect.expand(30.0));
        let stops = self.colors.clone().unwrap_or_else(|| {
            vec![hex("#FF6B2C"), hex("#E2566E"), hex("#9B5A8C"), hex("#5B4BA0"), hex("#FF8A3D")]
        });
        let origin = rect.center() - mask.size / 2.0;
        let sz = mask.size;
        let k = 1.6 / font.size;
        let drift = self.drift;
        let cell = (font.size / 18.0).clamp(3.0, 8.0);
        paint_mask_smooth(&painter, &mask, origin, cell, |uv, _| {
            let (x, y) = (uv.x * sz.x * k, uv.y * sz.y * k);
            let n = fbm3(x * 0.7, y * 0.7, time * 0.12, 3);
            let p = uv.x * 0.85 + (uv.y - 0.5) * 0.35 + n * 0.45 * drift + (time * 0.05 * drift).sin() * 0.25;
            let c = glowkit_core::color::gradient_cyclic(&stops, p * 0.6);
            let light = 0.5 + 0.5 * noise3(x * 0.9 - time * 0.2, y * 0.9, time * 0.1);
            lighten(c, light * 0.22)
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A headline printed in four inks through an ordered-dither (Bayer) screen: a slow ink field
/// decides which ink each cell gets, and a band of light sweeps across, washing cells to the
/// lightest ink as it passes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::DitherText::new("DITHER").cell(4.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DitherText {
    head: Head,
    inks: Option<[Color32; 4]>,
    cell: f32,
    sweep_every: f32,
}

impl DitherText {
    /// Dithered headline showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { head: Head::new(text), inks: None, cell: 0.0, sweep_every: 4.5 }
    }
    head_setters!();
    /// The four inks, from the one shown under the light band to the deepest.
    pub fn inks(mut self, inks: [Color32; 4]) -> Self {
        self.inks = Some(inks);
        self
    }
    /// Dither cell size in points (0 = derived from the font size).
    pub fn cell(mut self, cell: f32) -> Self {
        self.cell = cell.max(0.0);
        self
    }
    /// Seconds between light sweeps.
    pub fn sweep_every(mut self, secs: f32) -> Self {
        self.sweep_every = secs.max(1.0);
        self
    }
}

impl Widget for DitherText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = self.head.font(ui);
        let mask = text_mask(ui.ctx(), &self.head.text, &font, false);
        let (rect, response) = place(ui, mask.size + vec2(24.0, 24.0));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui) * self.head.speed;
        let painter = ui.painter_at(rect.expand(10.0));
        let inks = self.inks.unwrap_or(if t.dark {
            [hex("#F1EDE2"), hex("#FF5B1F"), hex("#9E3CF0"), hex("#3539F2")]
        } else {
            [hex("#17171C"), hex("#FF5B1F"), hex("#9E3CF0"), hex("#3539F2")]
        });
        let origin = rect.center() - mask.size / 2.0;
        let cell = if self.cell > 0.0 { self.cell } else { (font.size / 26.0).clamp(2.0, 6.0) };
        let period = self.sweep_every;
        let sweep = (time % period) / period * 1.9 - 0.45;
        paint_mask_cells(&painter, &mask, origin, cell, |uv, (cx, cy)| {
            // ink field: diagonal drift plus slow noise
            let base = 0.5 + 0.42 * noise3(uv.x * 2.2 - time * 0.12, uv.y * 1.6, time * 0.08) + (uv.x - 0.5) * 0.9;
            let d = uv.x - sweep - (uv.y - 0.5) * 0.25;
            let band = (-(d / 0.11).powi(2)).exp();
            let v = (base.clamp(0.0, 1.0) * (1.0 - band) + 0.0 * band).clamp(0.0, 1.0);
            // light band pulls toward ink 0
            let level = v * 3.0 + bayer4(cx, cy) - 0.5 - band * 2.2;
            let i = level.round().clamp(0.0, 3.0) as usize;
            Some(inks[i])
        });
        response
    }
}
