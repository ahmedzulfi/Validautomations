//! Gray blobs that melt into a glowing heat map under the pointer.

use super::common::idle_for;
use egui::{pos2, vec2, Color32, CursorIcon, Pos2, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::anim::{lerp, smoothstep, Spring};
use glowkit_core::color::{gradient, hex, mix, with_alpha};
use glowkit_core::{paint, Theme};

const BLOBS: usize = 32;

/// The arrangement a [`HeatBlobs`] forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeatShape {
    /// A loose 7 x 3 grid of dots.
    #[default]
    Dots,
    /// An eight-armed asterisk.
    Asterisk,
    /// The word "HOT" in chunky dots.
    Word,
}

impl HeatShape {
    fn next(self) -> Self {
        match self {
            HeatShape::Dots => HeatShape::Asterisk,
            HeatShape::Asterisk => HeatShape::Word,
            HeatShape::Word => HeatShape::Dots,
        }
    }
    /// Blob centers (in a ~300 x 170 box around 0) and radii.
    fn layout(self) -> Vec<(Vec2, f32)> {
        let mut out = Vec::with_capacity(BLOBS);
        match self {
            HeatShape::Dots => {
                for j in 0..3 {
                    for i in 0..7 {
                        let h = glowkit_core::rng::hash01((i * 31 + j * 17) as u64);
                        let jit = vec2(glowkit_core::rng::hash01((i * 7 + j * 3 + 5) as u64) - 0.5, h - 0.5) * 8.0;
                        out.push((vec2((i as f32 - 3.0) * 40.0, (j as f32 - 1.0) * 38.0) + jit, 14.0 + 7.0 * h));
                    }
                }
            }
            HeatShape::Asterisk => {
                out.push((Vec2::ZERO, 22.0));
                for arm in 0..8 {
                    let a = std::f32::consts::TAU * arm as f32 / 8.0 - std::f32::consts::FRAC_PI_2;
                    for k in 1..=3 {
                        let d = 22.0 + k as f32 * 19.0;
                        out.push((vec2(a.cos(), a.sin()) * d, 15.0 - k as f32 * 2.2));
                    }
                }
            }
            HeatShape::Word => {
                const GLYPHS: [[&str; 5]; 3] = [
                    ["X.X", "X.X", "XXX", "X.X", "X.X"],
                    ["XXX", "X.X", "X.X", "X.X", "XXX"],
                    ["XXX", ".X.", ".X.", ".X.", ".X."],
                ];
                for (g, rows) in GLYPHS.iter().enumerate() {
                    for (j, row) in rows.iter().enumerate() {
                        for (i, ch) in row.chars().enumerate() {
                            if ch == 'X' {
                                let x = (g as f32 - 1.0) * 100.0 + (i as f32 - 1.0) * 28.0;
                                let y = (j as f32 - 2.0) * 28.0;
                                out.push((vec2(x, y), 15.5));
                            }
                        }
                    }
                }
            }
        }
        // spare blobs hide inside existing ones with zero radius
        let n = out.len();
        while out.len() < BLOBS {
            let (p, _) = out[out.len() % n];
            out.push((p, 0.0));
        }
        out.truncate(BLOBS);
        out
    }
}

/// Color ramp of a [`HeatBlobs`] from rim (cool) to core (hot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeatPalette {
    /// Pink rim, blue, cyan, yellow, red core.
    #[default]
    Neon,
    /// Ember reds to white-hot.
    Fire,
    /// Deep blue to aqua foam.
    Sea,
    /// Candy violet, pink, orange, lime.
    Pop,
}

impl HeatPalette {
    fn stops(self) -> [Color32; 5] {
        match self {
            HeatPalette::Neon => [hex("#FF5AD9"), hex("#3B4BFF"), hex("#22D3EE"), hex("#FDE047"), hex("#FF3B1F")],
            HeatPalette::Fire => [hex("#7F1D1D"), hex("#DC2626"), hex("#F97316"), hex("#FACC15"), hex("#FFF7D6")],
            HeatPalette::Sea => [hex("#1E1B4B"), hex("#1D4ED8"), hex("#06B6D4"), hex("#5EEAD4"), hex("#F0FDFA")],
            HeatPalette::Pop => [hex("#8B5CF6"), hex("#EC4899"), hex("#F97316"), hex("#FACC15"), hex("#A3E635")],
        }
    }
}

#[derive(Clone)]
struct Blob {
    x: Spring,
    y: Spring,
    r: Spring,
    heat: f32,
}

#[derive(Clone)]
struct HeatState {
    shape: HeatShape,
    blobs: Vec<Blob>,
    auto_shape: i32,
}

/// Soft gray dots that melt together into a heat map where the pointer lingers: nearby blobs
/// warm up, swell and merge (metaball style) and glow through a color ramp, then cool back to
/// gray. Press to morph into the next shape (dots, asterisk, word).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{HeatBlobs, HeatPalette, HeatShape};
/// ui.add(HeatBlobs::new().shape(HeatShape::Dots).palette(HeatPalette::Neon).heat(60.0).area(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HeatBlobs {
    shape: HeatShape,
    palette: HeatPalette,
    heat: f32,
    area: f32,
    softness: f32,
    cold: Option<Color32>,
    autoplay: bool,
}

impl Default for HeatBlobs {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatBlobs {
    /// Gray dots with the neon ramp.
    pub fn new() -> Self {
        Self { shape: HeatShape::Dots, palette: HeatPalette::Neon, heat: 60.0, area: 50.0, softness: 40.0, cold: None, autoplay: false }
    }
    /// Starting shape (a press cycles to the next one).
    pub fn shape(mut self, s: HeatShape) -> Self {
        self.shape = s;
        self
    }
    /// Color ramp.
    pub fn palette(mut self, p: HeatPalette) -> Self {
        self.palette = p;
        self
    }
    /// How fast blobs heat up, 0..100.
    pub fn heat(mut self, h: f32) -> Self {
        self.heat = h.clamp(0.0, 100.0);
        self
    }
    /// Radius of the pointer's warmth, 0..100.
    pub fn area(mut self, a: f32) -> Self {
        self.area = a.clamp(0.0, 100.0);
        self
    }
    /// Edge softness, 0..100.
    pub fn softness(mut self, s: f32) -> Self {
        self.softness = s.clamp(0.0, 100.0);
        self
    }
    /// Color of cold blobs (default: a neutral gray from the theme).
    pub fn cold_color(mut self, c: Color32) -> Self {
        self.cold = Some(c);
        self
    }
    /// Wander a warm "pointer" around and change shapes while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for HeatBlobs {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = vec2(340.0, 200.0);
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), size.y), Sense::click());
        let rect = Rect::from_center_size(outer.center(), size);
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let hover = response.hover_pos().filter(|p| rect.expand(30.0).contains(*p));
        let idle = idle_for(ui, id, hover.is_some());
        let c = rect.center();
        let mut st: HeatState = glowkit_core::state(ui, id.with("heat"), || HeatState {
            shape: self.shape,
            blobs: self
                .shape
                .layout()
                .into_iter()
                .map(|(p, r)| Blob { x: Spring::new(p.x), y: Spring::new(p.y), r: Spring::new(r), heat: 0.0 })
                .collect(),
            auto_shape: 0,
        });
        let mut hand = hover;
        if response.clicked() {
            st.shape = st.shape.next();
        }
        if hand.is_none() && self.autoplay && idle > 0.5 {
            let u = idle - 0.5;
            hand = Some(c + vec2((u * 0.9).sin() * 120.0, (u * 1.7).sin() * 42.0));
            let k = (u / 5.0) as i32;
            if k != st.auto_shape {
                st.auto_shape = k;
                st.shape = st.shape.next();
            }
        }
        let layout = st.shape.layout();
        let area = lerp(30.0, 130.0, self.area / 100.0);
        let gain = lerp(0.4, 2.6, self.heat / 100.0);
        let mut settled = true;
        for (b, (p, r)) in st.blobs.iter_mut().zip(layout.iter()) {
            b.x.target = p.x;
            b.y.target = p.y;
            let pos = c + vec2(b.x.value, b.y.value);
            let near = hand.map(|h| (1.0 - (h - pos).length() / area).max(0.0)).unwrap_or(0.0);
            b.heat = (b.heat + near * gain * dt - b.heat * 1.1 * dt).clamp(0.0, 1.0);
            b.r.target = *r * (1.0 + 0.32 * b.heat);
            for s in [&mut b.x, &mut b.y, &mut b.r] {
                s.stiffness = 140.0;
                s.damping = 13.0;
                s.step(dt);
                settled &= s.is_settled();
            }
        }
        let blobs: Vec<(Pos2, f32, f32)> = st.blobs.iter().map(|b| (c + vec2(b.x.value, b.y.value), b.r.value.max(0.0), b.heat)).collect();
        glowkit_core::set_state(ui, id.with("heat"), st);
        if !settled || self.autoplay || blobs.iter().any(|b| b.2 > 0.001) || hand.is_some() {
            glowkit_core::animate(ui);
        }
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let cold = self.cold.unwrap_or_else(|| mix(t.muted, t.foreground, if t.dark { 0.35 } else { 0.42 }));
        let pal = self.palette.stops();
        let soft = lerp(0.08, 0.75, self.softness / 100.0);
        let draw = rect.expand(24.0);
        // only shade the area actually covered by blobs
        let mut bb = Rect::NOTHING;
        for (p, r, _) in &blobs {
            if *r > 0.5 {
                bb = bb.union(Rect::from_center_size(*p, Vec2::splat(r * 2.8)));
            }
        }
        let area_rect = bb.intersect(draw);
        if !area_rect.is_positive() {
            return response;
        }
        let cols = ((area_rect.width() / 2.6) as usize).clamp(20, 140);
        let rows = ((area_rect.height() / 2.6) as usize).clamp(12, 80);
        paint::shader_grid(painter, area_rect, cols, rows, |uv| {
            let p = pos2(area_rect.min.x + uv.x * area_rect.width(), area_rect.min.y + uv.y * area_rect.height());
            let mut field = 0.0;
            let mut hw = 0.0;
            for (bp, r, h) in &blobs {
                if *r < 0.5 {
                    continue;
                }
                let d2 = (p - *bp).length_sq().max(1.0);
                let q = r * r / d2;
                let w = q * q;
                field += w;
                hw += w * h;
            }
            if field < 0.25 {
                return Color32::TRANSPARENT;
            }
            let heat = (hw / field).clamp(0.0, 1.0);
            let a = smoothstep(1.0 - soft, 1.0 + soft * 0.25, field);
            let core = smoothstep(1.0, 10.0, field).sqrt();
            let hot = gradient(&pal, (heat * (0.2 + 0.8 * core)).clamp(0.0, 1.0));
            let k = smoothstep(0.02, 0.35, heat);
            with_alpha(mix(cold, hot, k), a)
        });
        response
    }
}
