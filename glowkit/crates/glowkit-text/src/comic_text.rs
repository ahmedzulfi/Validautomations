use crate::raster::{self, Coverage};
use crate::util;
use egui::{vec2, Color32, Pos2, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::{color::hex, ease, Clock, Easing, Theme, Trigger};
use std::f32::consts::TAU;
use std::sync::Arc;

#[derive(Clone)]
struct ComicCache {
    key: u64,
    cov: Arc<Coverage>,
}

/// Bold comic-book lettering: chunky text with a thick ink outline, a hard offset drop shadow,
/// a halftone dot pattern printed over the fill and a forward slant. It pops in with a springy
/// scale-and-twist.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::ComicText::new("BOOM!").size(110.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ComicText {
    text: String,
    size: f32,
    fill: Color32,
    dots: Color32,
    outline: Color32,
    shadow: Option<Color32>,
    stroke: Option<f32>,
    slant: f32,
    dot_gap: Option<f32>,
    trigger: Trigger,
    duration: f32,
}

impl ComicText {
    /// Comic lettering for `text` (short and loud works best).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 96.0,
            fill: hex("#FACC15"),
            dots: hex("#EF4444"),
            outline: hex("#0B0B0F"),
            shadow: None,
            stroke: None,
            slant: 10.0,
            dot_gap: None,
            trigger: Trigger::OnAppear,
            duration: 0.9,
        }
    }
    /// Font size in points (default 96).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }
    /// Fill color of the letters (default comic yellow).
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = c;
        self
    }
    /// Color of the halftone dots (default red; transparent hides them).
    pub fn dots(mut self, c: Color32) -> Self {
        self.dots = c;
        self
    }
    /// Outline ink color (default near-black).
    pub fn outline(mut self, c: Color32) -> Self {
        self.outline = c;
        self
    }
    /// Drop shadow color (default: the theme accent on dark themes, ink on light ones).
    pub fn shadow(mut self, c: Color32) -> Self {
        self.shadow = Some(c);
        self
    }
    /// Outline thickness in points (default: 5% of the font size).
    pub fn stroke(mut self, pts: f32) -> Self {
        self.stroke = Some(pts.max(0.0));
        self
    }
    /// Forward slant in degrees (default 10).
    pub fn slant(mut self, degrees: f32) -> Self {
        self.slant = degrees;
        self
    }
    /// Distance between halftone dots in points (default: 7% of the font size).
    pub fn dot_gap(mut self, pts: f32) -> Self {
        self.dot_gap = Some(pts.max(2.0));
        self
    }
    /// When the pop-in plays (default: on appear).
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Seconds for the pop-in (default 0.9).
    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds.max(0.05);
        self
    }
}

impl Widget for ComicText {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = util::font(self.size, false);
        let galley = util::layout(ui.painter(), self.text.as_str(), &font);
        let bold = util::bold_px(self.size) * 1.8;
        let stroke = self.stroke.unwrap_or(self.size * 0.05);
        let shadow_off = vec2(1.0, 1.0) * (self.size * 0.06).max(2.0);
        let shear = self.slant.to_radians().tan();
        let gs = galley.size() + vec2(bold, 0.0);
        let pad = vec2(stroke * 2.0 + shadow_off.x + gs.y * shear.abs() * 0.5 + 8.0, stroke * 2.0 + shadow_off.y + 8.0);
        let (rect, resp) = ui.allocate_exact_size(gs + pad * 2.0, Sense::click());
        let e = Clock::triggered(ui, resp.id, self.trigger, self.duration, resp.hovered(), resp.clicked());
        let k = (e / self.duration).clamp(0.0, 1.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let gap = self.dot_gap.unwrap_or(self.size * 0.07).max(2.0);
        let key = raster::key(&self.text, &[self.size, gap, bold]);
        let id = resp.id.with("comic");
        let cached: Option<ComicCache> = ui.data(|d| d.get_temp(id));
        let cov = match cached {
            Some(c) if c.key == key => c.cov,
            _ => {
                let raw = raster::coverage(ui.painter(), &galley, gs, vec2(gap, gap) * 0.4);
                let cov = Arc::new(raw);
                ui.data_mut(|d| d.insert_temp(id, ComicCache { key, cov: cov.clone() }));
                cov
            }
        };
        let scale = 0.35 + 0.65 * ease(Easing::OutBack, k);
        let angle = (-14.0 + 12.0 * ease(Easing::OutElastic, k)).to_radians();
        let alpha = ease(Easing::OutCubic, (k * 3.0).min(1.0));
        if alpha <= 0.0 {
            return resp;
        }
        let center = rect.center();
        let half = gs / 2.0;
        let (sa, ca) = angle.sin_cos();
        // galley-local point -> screen: slant, scale, rotate around the center
        let xf = move |v: Vec2| -> Pos2 {
            let mut p = v - half;
            p.x -= p.y * shear;
            p *= scale;
            center + vec2(p.x * ca - p.y * sa, p.x * sa + p.y * ca)
        };
        let painter = ui.painter_at(rect.expand(stroke * 2.0));
        let ink = |off: Vec2, color: Color32| {
            let c = color.gamma_multiply(alpha);
            let steps = (bold / 0.8).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let dx = bold * s as f32 / steps as f32;
                util::paint_galley_mapped(&painter, &galley, |v| xf(v + vec2(dx, 0.0)) + off, |_| c);
            }
        };
        let shadow = self.shadow.unwrap_or(if t.dark { t.accent } else { self.outline });
        let around = 14;
        // hard drop shadow (outline-thick, offset)
        for i in 0..around {
            let a = i as f32 / around as f32 * TAU;
            ink(vec2(a.cos(), a.sin()) * stroke * scale + shadow_off * scale, shadow);
        }
        // ink outline
        for i in 0..around {
            let a = i as f32 / around as f32 * TAU;
            ink(vec2(a.cos(), a.sin()) * stroke * scale, self.outline);
        }
        // fill
        ink(Vec2::ZERO, self.fill);
        // halftone dots printed over the fill, growing toward the bottom-right
        if self.dots.a() > 0 {
            let dc = self.dots.gamma_multiply(alpha * 0.9);
            let r_max = gap * 0.38;
            let cols = (gs.x / gap).ceil() as i32;
            let rows = (gs.y / gap).ceil() as i32;
            for cy in 0..rows {
                for cx in 0..cols {
                    let local = vec2((cx as f32 + 0.5) * gap, (cy as f32 + 0.5) * gap);
                    let u = (local.x / gs.x * 0.6 + local.y / gs.y * 0.4).clamp(0.0, 1.0);
                    let r = r_max * (0.35 + 0.65 * u);
                    // only where the whole dot sits inside the letter, so the outline stays crisp
                    let probe = r + 1.0;
                    let inside =
                        [vec2(0.0, 0.0), vec2(probe, 0.0), vec2(-probe, 0.0), vec2(0.0, probe), vec2(0.0, -probe)]
                            .iter()
                            .all(|d| cov.sample(local + *d) > 0.55);
                    if inside {
                        painter.circle_filled(xf(local + vec2(bold * 0.5, 0.0)), r * scale, dc);
                    }
                }
            }
        }
        resp
    }
}
