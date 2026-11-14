//! Surface transitions: a card that resizes (its content reflowing live), a two-page card that
//! slides side by side, a skeleton that pulses and reveals, and a 4 x 4 dot-matrix loader.

use super::common::{avatar, bar, cr, layer, material, phase, skeleton, surface, ui_layer, MotionCurve, Tween, AVATAR_TONES};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Color32, FontId, Rect, Response, Sense, Ui, UiBuilder, Vec2, Widget};
use glowkit_core::color::mix;
use glowkit_core::Theme;

/// A card that animates between two sizes on a long smooth curve. Its content is laid out
/// again every frame at the in-between size, so skeleton bars stretch and real text reflows
/// while it moves.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let wide = true;
/// ui.add(glowkit_interactions::ResizeCard::new(wide).text("Release notes", "Faster sync, fewer conflicts..."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ResizeCard {
    expanded: bool,
    a: Vec2,
    b: Vec2,
    dur: f32,
    curve: MotionCurve,
    text: Option<(String, String)>,
}

impl ResizeCard {
    /// A card at its first size, or its second when `expanded`.
    pub fn new(expanded: bool) -> Self {
        Self { expanded, a: vec2(220.0, 112.0), b: vec2(154.0, 128.0), dur: 0.3, curve: MotionCurve::SMOOTH_OUT, text: None }
    }
    /// The two sizes (default 220 x 112 and 154 x 128).
    pub fn sizes(mut self, first: Vec2, second: Vec2) -> Self {
        self.a = first;
        self.b = second;
        self
    }
    /// Duration in seconds (default 0.3).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.01);
        self
    }
    /// Timing curve (default [`MotionCurve::SMOOTH_OUT`]).
    pub fn curve(mut self, c: MotionCurve) -> Self {
        self.curve = c;
        self
    }
    /// Show a title and wrapped body instead of skeleton bars.
    pub fn text(mut self, title: impl Into<String>, body: impl Into<String>) -> Self {
        self.text = Some((title.into(), body.into()));
        self
    }
}

impl Widget for ResizeCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let max = self.a.max(self.b);
        let (rect, response) = ui.allocate_exact_size(max + vec2(40.0, 40.0), Sense::hover());
        let ph = phase(ui, response.id, self.expanded, self.dur, self.dur, self.curve, self.curve);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let size = self.a + (self.b - self.a) * ph.p;
        let card = Rect::from_center_size(rect.center(), size);
        let painter = ui.painter();
        material(painter, &t, card, 12.0, surface(&t));
        let inner = card.shrink2(vec2(14.0, 14.0));
        let clip = painter.with_clip_rect(card.shrink(1.0).intersect(ui.clip_rect()));
        match &self.text {
            None => {
                let w = inner.width();
                let rows = [(0.0, 0.45, 9.0), (16.0, 1.0, 6.0), (26.0, 1.0, 6.0), (36.0, 0.7, 6.0), (56.0, 1.0, 6.0), (66.0, 1.0, 6.0)];
                for (y, k, h) in rows {
                    let r = Rect::from_min_size(inner.min + vec2(0.0, y), vec2(w * k, h));
                    clip.rect_filled(r, cr(h / 2.0), skeleton(&t));
                }
            }
            Some((title, body)) => {
                let tg = clip.layout(title.clone(), FontId::proportional(14.0), t.foreground, inner.width());
                let th = tg.size().y;
                clip.galley(inner.min, tg, t.foreground);
                let bg = clip.layout(body.clone(), FontId::proportional(12.5), t.muted_foreground, inner.width());
                clip.galley(inner.min + vec2(0.0, th + 6.0), bg, t.muted_foreground);
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct PagerState {
    page: usize,
    prev: usize,
    since: f64,
    height: Tween,
}

/// A card with pages that sit side by side: going forward the current page drifts a few points
/// left and blurs out while the next drifts in from the right (and the other way going back),
/// and the card's height eases to fit the new page. Pages are your own `egui` content.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut page = 0;
/// glowkit_interactions::SidePager::new(page).show(ui, |ui, p| {
///     if p == 0 && ui.button("Next").clicked() { page = 1; }
///     if p == 1 && ui.button("Back").clicked() { page = 0; }
/// });
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct SidePager {
    page: usize,
    width: f32,
    dur: f32,
    distance: f32,
    blur: f32,
    pad: f32,
}

impl SidePager {
    /// Show page `page`.
    pub fn new(page: usize) -> Self {
        Self { page, width: 280.0, dur: 0.25, distance: 8.0, blur: 3.0, pad: 18.0 }
    }
    /// Card width (default 280).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(80.0);
        self
    }
    /// Transition length in seconds (default 0.25).
    pub fn duration(mut self, s: f32) -> Self {
        self.dur = s.max(0.01);
        self
    }
    /// Drift distance in points (default 8).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Blur at the ends of the transition (default 3).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Inner padding (default 18).
    pub fn padding(mut self, p: f32) -> Self {
        self.pad = p.max(0.0);
        self
    }

    /// Show the card; `page_ui` lays out page `n` into the given `Ui`.
    pub fn show(self, ui: &mut Ui, mut page_ui: impl FnMut(&mut Ui, usize)) -> Response {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("side_pager", ui.next_auto_id()));
        let now = ui.input(|i| i.time);
        let page = self.page;
        let measured: Vec<f32> = glowkit_core::state(ui, id.with("heights"), Vec::new);
        let target_h = measured.get(page).copied().unwrap_or(160.0);
        let dur = self.dur;
        let st = glowkit_core::with_state(
            ui,
            id.with("pager"),
            || PagerState { page, prev: page, since: -1e6, height: Tween::new(target_h) },
            |s| {
                if s.page != page {
                    s.prev = s.page;
                    s.page = page;
                    s.since = now;
                }
                if s.height.to == 160.0 && target_h != 160.0 && (now - s.since) > 1e5 {
                    s.height.snap(target_h);
                }
                s.height.retarget(target_h, now, dur, MotionCurve::SMOOTH_OUT);
                *s
            },
        );
        let age = (now - st.since) as f32;
        let h = st.height.value(now, dur, MotionCurve::SMOOTH_OUT);
        let max_h = measured.iter().copied().fold(h, f32::max);
        let (outer, response) = ui.allocate_exact_size(vec2(self.width + 40.0, max_h + 40.0), Sense::hover());
        let card = Rect::from_min_size(pos2(outer.center().x - self.width / 2.0, outer.min.y + 20.0), vec2(self.width, h));
        material(ui.painter(), &t, card, 16.0, surface(&t));
        let k = MotionCurve::SMOOTH_OUT.at(age / dur);
        if k < 1.0 || st.height.moving(now, dur) {
            ui.ctx().request_repaint();
        }
        let dir = if st.page >= st.prev { 1.0 } else { -1.0 };
        let content = Rect::from_min_size(card.min + Vec2::splat(self.pad), vec2(self.width - self.pad * 2.0, 2000.0));
        let clip = card.shrink(1.0).intersect(ui.clip_rect());
        let mut heights = measured.clone();
        let mut run = |ui: &mut Ui, p: usize, xf: TSTransform, a: f32, blur: f32, interactive: bool| {
            let used = ui_layer(ui, xf, a, blur, |ui| {
                let mut child = ui.new_child(UiBuilder::new().max_rect(content).id_salt(("page", p)).layout(egui::Layout::top_down(egui::Align::Min)));
                child.set_clip_rect(clip);
                if !interactive {
                    child.disable();
                }
                page_ui(&mut child, p);
                child.min_rect().height()
            });
            if heights.len() <= p {
                heights.resize(p + 1, 160.0);
            }
            heights[p] = used + self.pad * 2.0;
        };
        if k < 1.0 && st.prev != st.page {
            run(ui, st.prev, TSTransform::from_translation(vec2(-dir * self.distance * k, 0.0)), 1.0 - k, self.blur * k, false);
            run(ui, st.page, TSTransform::from_translation(vec2(dir * self.distance * (1.0 - k), 0.0)), k, self.blur * (1.0 - k), true);
        } else {
            run(ui, st.page, TSTransform::IDENTITY, 1.0, 0.0, true);
        }
        if heights != measured {
            glowkit_core::set_state(ui, id.with("heights"), heights);
            ui.ctx().request_repaint();
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A profile row that loads: while waiting the skeleton (a disc and two bars) breathes, and
/// once loaded the real avatar and text fade up out of a slight blur as the skeleton melts away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let loaded = true;
/// ui.add(glowkit_interactions::SkeletonReveal::new(loaded).name("Jane Cooper").detail("jane@example.com"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SkeletonReveal {
    loaded: bool,
    name: String,
    detail: String,
    tone: usize,
    width: f32,
    pulse: f32,
    pulse_min: f32,
    reveal: f32,
    blur: f32,
}

impl SkeletonReveal {
    /// A row showing its content once `loaded`.
    pub fn new(loaded: bool) -> Self {
        Self {
            loaded,
            name: "Jane Cooper".into(),
            detail: "jane.cooper@example.com".into(),
            tone: 0,
            width: 260.0,
            pulse: 1.0,
            pulse_min: 0.5,
            reveal: 0.4,
            blur: 2.0,
        }
    }
    /// Name line.
    pub fn name(mut self, s: impl Into<String>) -> Self {
        self.name = s.into();
        self
    }
    /// Detail line.
    pub fn detail(mut self, s: impl Into<String>) -> Self {
        self.detail = s.into();
        self
    }
    /// Avatar color index.
    pub fn tone(mut self, i: usize) -> Self {
        self.tone = i;
        self
    }
    /// Width (default 260).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Seconds per breath of the skeleton (default 1).
    pub fn pulse(mut self, s: f32) -> Self {
        self.pulse = s.max(0.1);
        self
    }
    /// Opacity at the bottom of a breath (default 0.5).
    pub fn pulse_min(mut self, a: f32) -> Self {
        self.pulse_min = a.clamp(0.0, 1.0);
        self
    }
    /// Reveal length in seconds (default 0.4).
    pub fn reveal(mut self, s: f32) -> Self {
        self.reveal = s.max(0.01);
        self
    }
}

impl Widget for SkeletonReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = vec2(self.width, 60.0);
        let (rect, response) = ui.allocate_exact_size(size + vec2(20.0, 20.0), Sense::hover());
        let ph = phase(ui, response.id, self.loaded, self.reveal, self.reveal, MotionCurve::EASE_IN_OUT, MotionCurve::EASE_IN_OUT);
        if !self.loaded {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let card = Rect::from_center_size(rect.center(), size);
        let painter = ui.painter();
        material(painter, &t, card, 14.0, surface(&t));
        let p = ph.p.clamp(0.0, 1.0);
        let now = glowkit_core::time(ui);
        let breath = {
            let u = (now / self.pulse).fract();
            let w = 0.5 - 0.5 * (u * std::f32::consts::TAU).cos();
            1.0 - (1.0 - self.pulse_min) * w
        };
        let ac = pos2(card.min.x + 30.0, card.center().y);
        let x = card.min.x + 56.0;
        if p < 1.0 {
            layer(ui, TSTransform::IDENTITY, (1.0 - p) * breath, self.blur * p, |pt| {
                pt.circle_filled(ac, 17.0, skeleton(&t));
                bar(pt, &t, Rect::from_min_size(pos2(x, card.center().y - 10.0), vec2(90.0, 8.0)));
                bar(pt, &t, Rect::from_min_size(pos2(x, card.center().y + 3.0), vec2(150.0, 8.0)));
            });
        }
        if p > 0.0 {
            layer(ui, TSTransform::IDENTITY, p, self.blur * (1.0 - p), |pt| {
                avatar(pt, ac, 17.0, AVATAR_TONES[self.tone % AVATAR_TONES.len()], None);
                super::common::two_lines(pt, &t, x, card.center().y, &self.name, &self.detail, card.max.x - x - 12.0);
            });
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// The light pattern a [`DotMatrixLoader`] plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DotPattern {
    /// Columns light up left to right like a scanner.
    #[default]
    Scan,
    /// Dots twinkle in a scattered order.
    Twinkle,
    /// A light runs around the outer ring (corners off).
    Orbit,
    /// The middle lights, then the ring around it, like a heartbeat.
    Pulse,
}

const RING: [usize; 8] = [1, 2, 7, 11, 14, 13, 8, 4];
const INNER: [usize; 4] = [5, 6, 9, 10];
const CORNERS: [usize; 4] = [0, 3, 12, 15];
const TWINKLE: [usize; 16] = [7, 2, 11, 5, 14, 9, 0, 12, 3, 15, 6, 10, 13, 1, 8, 4];

/// A tiny 4 x 4 dot matrix loader: each dot flashes from a quiet base to an active tone and
/// eases back, offset in time by its pattern (scan, twinkle, orbit or pulse).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::DotMatrixLoader::new(glowkit_interactions::DotPattern::Orbit));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DotMatrixLoader {
    pattern: DotPattern,
    dot: f32,
    gap: f32,
    cycle: f32,
    round: Option<bool>,
    base: Option<Color32>,
    active: Option<Color32>,
}

impl DotMatrixLoader {
    /// A loader playing `pattern`.
    pub fn new(pattern: DotPattern) -> Self {
        Self { pattern, dot: 4.0, gap: 2.0, cycle: 1.2, round: None, base: None, active: None }
    }
    /// Dot size in points (default 4).
    pub fn dot(mut self, s: f32) -> Self {
        self.dot = s.max(1.0);
        self
    }
    /// Gap between dots (default 2).
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Seconds per cycle (default 1.2).
    pub fn cycle(mut self, s: f32) -> Self {
        self.cycle = s.max(0.1);
        self
    }
    /// Round dots instead of square (default: round for orbit and pulse).
    pub fn round(mut self, on: bool) -> Self {
        self.round = Some(on);
        self
    }
    /// Resting and lit colors (default: quiet greys from the theme).
    pub fn colors(mut self, base: Color32, active: Color32) -> Self {
        self.base = Some(base);
        self.active = Some(active);
        self
    }
}

/// Brightness (0..1) of a dot `f` (0..1) of the way through its cycle: lit at 15%, out by 45%.
fn dot_wave(f: f32) -> f32 {
    if f < 0.15 {
        MotionCurve::EASE_IN_OUT.at(f / 0.15)
    } else if f < 0.45 {
        1.0 - MotionCurve::EASE_IN_OUT.at((f - 0.15) / 0.3)
    } else {
        0.0
    }
}

impl Widget for DotMatrixLoader {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let side = self.dot * 4.0 + self.gap * 3.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let base = self.base.unwrap_or(if t.dark { Color32::from_rgb(0x3A, 0x3A, 0x3E) } else { Color32::from_rgb(0xD9, 0xD9, 0xD9) });
        let active = self.active.unwrap_or(if t.dark { Color32::from_rgb(0xB8, 0xB8, 0xC2) } else { Color32::from_rgb(0x85, 0x85, 0x8F) });
        let round = self.round.unwrap_or(matches!(self.pattern, DotPattern::Orbit | DotPattern::Pulse));
        let now = glowkit_core::time(ui);
        let c = self.cycle;
        let painter = ui.painter();
        for (idx, tw) in TWINKLE.iter().enumerate() {
            let (row, col) = (idx / 4, idx % 4);
            let delay = match self.pattern {
                DotPattern::Scan => Some(col as f32 * c / 10.0),
                DotPattern::Twinkle => Some(*tw as f32 * c / 16.0),
                DotPattern::Orbit => {
                    if CORNERS.contains(&idx) {
                        continue;
                    }
                    RING.iter().position(|&r| r == idx).map(|k| k as f32 * c / 8.0)
                }
                DotPattern::Pulse => {
                    if CORNERS.contains(&idx) {
                        continue;
                    }
                    Some(if INNER.contains(&idx) { 0.0 } else { c * 0.16 })
                }
            };
            let k = delay.map_or(0.0, |d| dot_wave(((now - d) / c).rem_euclid(1.0)));
            let col_c = mix(base, active, k);
            let p = rect.min + vec2(col as f32 * (self.dot + self.gap), row as f32 * (self.dot + self.gap));
            let r = Rect::from_min_size(p, Vec2::splat(self.dot));
            if round {
                painter.circle_filled(r.center(), self.dot / 2.0, col_c);
            } else {
                painter.rect_filled(r, cr(self.dot * 0.18), col_c);
            }
        }
        response
    }
}
