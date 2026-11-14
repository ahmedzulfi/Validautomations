use super::common::{capture, mesh_alpha, shape_mark, tessellator, to_meshes};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Color32, Id, InnerResponse, Rect, Shape, Ui, UiBuilder};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Where [`AnimatedContent`] slides in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SlideFrom {
    /// Starts below its slot and rises up.
    #[default]
    Below,
    /// Starts above its slot and drops down.
    Above,
    /// Starts left of its slot.
    Left,
    /// Starts right of its slot.
    Right,
}

impl SlideFrom {
    fn vector(self) -> egui::Vec2 {
        match self {
            SlideFrom::Below => vec2(0.0, 1.0),
            SlideFrom::Above => vec2(0.0, -1.0),
            SlideFrom::Left => vec2(-1.0, 0.0),
            SlideFrom::Right => vec2(1.0, 0.0),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct RevealState {
    start: Option<f64>,
    rect: Option<Rect>,
}

/// Shared "reveal when scrolled into view" timing.
#[derive(Clone, Copy)]
struct RevealTiming {
    duration: f32,
    delay: f32,
    threshold: f32,
    once: bool,
    easing: Easing,
    progress: Option<f32>,
}

impl RevealTiming {
    /// Returns eased progress 0..1 for this frame, based on last frame's rect.
    fn progress(&self, ui: &Ui, id: Id) -> f32 {
        if let Some(p) = self.progress {
            return ease(self.easing, p.clamp(0.0, 1.0));
        }
        let now = ui.input(|i| i.time);
        let clip = ui.clip_rect();
        let st: RevealState = glowkit_core::state(ui, id, RevealState::default);
        let visible = st.rect.map(|r| {
            let inter = r.intersect(clip);
            if inter.is_negative() || r.height() <= 0.0 {
                0.0
            } else {
                (inter.height() / r.height().min(clip.height()).max(1.0)).clamp(0.0, 1.0)
            }
        });
        let mut start = st.start;
        match visible {
            Some(v) if v >= self.threshold.max(0.001) => {
                if start.is_none() {
                    start = Some(now);
                }
            }
            Some(v) if v <= 0.0 && !self.once => start = None,
            _ => {}
        }
        glowkit_core::set_state(ui, id, RevealState { start, ..st });
        let raw = match start {
            Some(s) => ((now - s) as f32 - self.delay) / self.duration.max(1e-3),
            None => 0.0,
        };
        if start.is_none() || raw < 1.05 {
            glowkit_core::animate(ui);
        }
        ease(self.easing, raw.clamp(0.0, 1.0))
    }

    fn store_rect(ui: &Ui, id: Id, rect: Rect) {
        let mut st: RevealState = glowkit_core::state(ui, id, RevealState::default);
        st.rect = Some(rect);
        glowkit_core::set_state(ui, id, st);
    }
}

/// Wraps any content so it slides, scales and fades into place the first time it scrolls into
/// view (or every time, with [`Self::once`]`(false)`). Put it inside a `ScrollArea` and each
/// section animates in as you reach it, or drive it yourself with [`Self::progress`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{AnimatedContent, SlideFrom};
/// AnimatedContent::new().from(SlideFrom::Below).distance(80.0).scale(0.9).show(ui, |ui| {
///     ui.heading("Hello there");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct AnimatedContent {
    from: SlideFrom,
    distance: f32,
    scale: f32,
    opacity: f32,
    timing: RevealTiming,
    salt: u64,
}

impl Default for AnimatedContent {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimatedContent {
    /// Slides up 100 points while fading in, over 0.8 s, once.
    pub fn new() -> Self {
        Self {
            from: SlideFrom::Below,
            distance: 100.0,
            scale: 1.0,
            opacity: 0.0,
            timing: RevealTiming {
                duration: 0.8,
                delay: 0.0,
                threshold: 0.1,
                once: true,
                easing: Easing::OutCubic,
                progress: None,
            },
            salt: 0,
        }
    }
    /// Direction the content enters from.
    pub fn from(mut self, from: SlideFrom) -> Self {
        self.from = from;
        self
    }
    /// Travel distance in points.
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Starting scale (1 = no scaling).
    pub fn scale(mut self, s: f32) -> Self {
        self.scale = s.max(0.0);
        self
    }
    /// Starting opacity (0..1).
    pub fn initial_opacity(mut self, o: f32) -> Self {
        self.opacity = o.clamp(0.0, 1.0);
        self
    }
    /// Animation length in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.timing.duration = s.max(0.01);
        self
    }
    /// Delay after entering view, in seconds.
    pub fn delay(mut self, s: f32) -> Self {
        self.timing.delay = s.max(0.0);
        self
    }
    /// Fraction of the content that must be visible before it animates (0..1).
    pub fn threshold(mut self, t: f32) -> Self {
        self.timing.threshold = t.clamp(0.0, 1.0);
        self
    }
    /// Play only the first time (default), or replay whenever it re-enters the view.
    pub fn once(mut self, once: bool) -> Self {
        self.timing.once = once;
        self
    }
    /// Easing curve (default `OutCubic`; try `OutBack` for a little overshoot).
    pub fn easing(mut self, e: Easing) -> Self {
        self.timing.easing = e;
        self
    }
    /// Drive the animation manually (0 = hidden, 1 = in place), e.g. from scroll position.
    pub fn progress(mut self, p: f32) -> Self {
        self.timing.progress = Some(p);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show `content` with the reveal animation.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let id = ui.next_auto_id().with(("glowkit_animated_content", self.salt));
        let e = self.timing.progress(ui, id);
        let alpha = self.opacity + (1.0 - self.opacity) * e.clamp(0.0, 1.0);
        let start = shape_mark(ui);
        let r = ui.scope_builder(UiBuilder::new(), |ui| {
            ui.multiply_opacity(alpha);
            content(ui)
        });
        let rect = r.response.rect;
        RevealTiming::store_rect(ui, id, rect);
        if e < 0.9999 {
            let offset = self.from.vector() * self.distance * (1.0 - e);
            let s = self.scale + (1.0 - self.scale) * e;
            let c = rect.center();
            let tf = TSTransform::from_translation(c.to_vec2() + offset)
                * TSTransform::from_scaling(s)
                * TSTransform::from_translation(-c.to_vec2());
            let layer = ui.layer_id();
            ui.ctx().graphics_mut(|g| {
                let list = g.entry(layer);
                let end = list.next_idx();
                list.transform_range(start, end, tf);
            });
        }
        r
    }
}

/// Wraps content so it fades in from a blur the first time it scrolls into view: it starts
/// soft, ghosted and transparent, then sharpens into focus.
///
/// The blur is faked by re-painting the content's meshes at small offsets with low alpha, so it
/// works on any egui content (text, widgets, images).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::FadeContent::new().blur(8.0).duration(1.0).show(ui, |ui| {
///     ui.label("Soft focus, then crisp.");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct FadeContent {
    blur: f32,
    opacity: f32,
    rise: f32,
    timing: RevealTiming,
    salt: u64,
}

impl Default for FadeContent {
    fn default() -> Self {
        Self::new()
    }
}

impl FadeContent {
    /// A 1 s fade from 6 px blur and zero opacity, once.
    pub fn new() -> Self {
        Self {
            blur: 6.0,
            opacity: 0.0,
            rise: 0.0,
            timing: RevealTiming {
                duration: 1.0,
                delay: 0.0,
                threshold: 0.1,
                once: true,
                easing: Easing::OutCubic,
                progress: None,
            },
            salt: 0,
        }
    }
    /// Starting blur radius in points (0 = plain fade).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
    /// Starting opacity (0..1).
    pub fn initial_opacity(mut self, o: f32) -> Self {
        self.opacity = o.clamp(0.0, 1.0);
        self
    }
    /// Optional upward drift in points while fading in.
    pub fn rise(mut self, d: f32) -> Self {
        self.rise = d;
        self
    }
    /// Animation length in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.timing.duration = s.max(0.01);
        self
    }
    /// Delay after entering view, in seconds.
    pub fn delay(mut self, s: f32) -> Self {
        self.timing.delay = s.max(0.0);
        self
    }
    /// Fraction of the content that must be visible before it animates (0..1).
    pub fn threshold(mut self, t: f32) -> Self {
        self.timing.threshold = t.clamp(0.0, 1.0);
        self
    }
    /// Play only the first time (default), or replay whenever it re-enters the view.
    pub fn once(mut self, once: bool) -> Self {
        self.timing.once = once;
        self
    }
    /// Easing curve.
    pub fn easing(mut self, e: Easing) -> Self {
        self.timing.easing = e;
        self
    }
    /// Drive the animation manually (0 = hidden, 1 = sharp).
    pub fn progress(mut self, p: f32) -> Self {
        self.timing.progress = Some(p);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show `content` with the blur-fade animation.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let id = ui.next_auto_id().with(("glowkit_fade_content", self.salt));
        let e = self.timing.progress(ui, id);
        let alpha = self.opacity + (1.0 - self.opacity) * e;
        let start = shape_mark(ui);
        let r = ui.scope_builder(UiBuilder::new(), |ui| {
            ui.multiply_opacity(alpha);
            content(ui)
        });
        RevealTiming::store_rect(ui, id, r.response.rect);
        let radius = self.blur * (1.0 - e);
        let lift = vec2(0.0, self.rise * (1.0 - e));
        if radius > 0.25 || lift.y.abs() > 0.1 {
            let cap = capture(ui, start);
            let ctx = ui.ctx().clone();
            let mut tess = tessellator(&ctx);
            cap.restore(&ctx, |_, cs| {
                let mut meshes = Vec::new();
                to_meshes(&mut tess, cs.shape, &mut meshes);
                (cs.clip_rect, blur_meshes(meshes, radius, lift))
            });
        }
        r
    }
}

/// Re-paint meshes as a ring of faint offset copies: a cheap, convincing blur.
fn blur_meshes(meshes: Vec<egui::epaint::Mesh>, radius: f32, lift: egui::Vec2) -> Shape {
    if radius < 0.25 {
        return Shape::Vec(
            meshes
                .into_iter()
                .map(|mut m| {
                    m.translate(lift);
                    Shape::mesh(m)
                })
                .collect(),
        );
    }
    use std::f32::consts::TAU;
    let taps: [(f32, f32, f32); 7] = [
        (0.0, 0.0, 0.34),
        (0.0, 1.0, 0.2),
        (TAU / 3.0, 1.0, 0.2),
        (TAU * 2.0 / 3.0, 1.0, 0.2),
        (TAU / 6.0, 0.55, 0.22),
        (TAU / 2.0, 0.55, 0.22),
        (TAU * 5.0 / 6.0, 0.55, 0.22),
    ];
    let mut out = Vec::with_capacity(meshes.len() * taps.len());
    for m in meshes {
        for (ang, r, a) in taps {
            let mut c = m.clone();
            c.translate(lift + vec2(ang.cos(), ang.sin()) * radius * r);
            mesh_alpha(&mut c, a * (1.0 - (radius / 30.0)).clamp(0.4, 1.0));
            out.push(Shape::mesh(c));
        }
    }
    Shape::Vec(out)
}

/// Which edge a [`GradualBlur`] veil sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlurEdge {
    /// Top edge.
    Top,
    /// Bottom edge (default).
    #[default]
    Bottom,
    /// Left edge.
    Left,
    /// Right edge.
    Right,
}

/// A progressive blur at the edge of a scrolling region: content gets softer and fainter in
/// steps as it approaches the edge, the way frosted-glass headers and "more below" fades look on
/// the web.
///
/// The blur is built from stacked bands; each band re-paints the content under it as faint
/// offset copies (stronger toward the edge), and a translucent veil fades into the background.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{BlurEdge, GradualBlur};
/// GradualBlur::new().edge(BlurEdge::Bottom).height(90.0).show(ui, |ui| {
///     egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
///         for i in 0..40 { ui.label(format!("Row {i}")); }
///     });
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct GradualBlur {
    edge: BlurEdge,
    height: f32,
    strength: f32,
    bands: usize,
    veil: f32,
    color: Option<Color32>,
    curve: f32,
}

impl Default for GradualBlur {
    fn default() -> Self {
        Self::new()
    }
}

impl GradualBlur {
    /// A 96-point, 6-band blur on the bottom edge.
    pub fn new() -> Self {
        Self { edge: BlurEdge::Bottom, height: 96.0, strength: 5.0, bands: 6, veil: 0.85, color: None, curve: 1.6 }
    }
    /// Which edge gets the blur.
    pub fn edge(mut self, e: BlurEdge) -> Self {
        self.edge = e;
        self
    }
    /// Depth of the blurred zone in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(4.0);
        self
    }
    /// Blur radius at the very edge, in points.
    pub fn strength(mut self, s: f32) -> Self {
        self.strength = s.max(0.0);
        self
    }
    /// Number of bands (more = smoother, costs more).
    pub fn bands(mut self, n: usize) -> Self {
        self.bands = n.clamp(1, 16);
        self
    }
    /// How opaque the background veil gets at the edge (0..1).
    pub fn veil(mut self, v: f32) -> Self {
        self.veil = v.clamp(0.0, 1.0);
        self
    }
    /// Veil color (defaults to the theme card color, the usual panel fill).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Exponent shaping how fast the blur ramps up (1 = linear).
    pub fn curve(mut self, c: f32) -> Self {
        self.curve = c.max(0.2);
        self
    }

    fn zone(&self, rect: Rect) -> Rect {
        let h = self.height.min(if matches!(self.edge, BlurEdge::Left | BlurEdge::Right) {
            rect.width()
        } else {
            rect.height()
        });
        match self.edge {
            BlurEdge::Top => Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + h)),
            BlurEdge::Bottom => Rect::from_min_max(pos2(rect.min.x, rect.max.y - h), rect.max),
            BlurEdge::Left => Rect::from_min_max(rect.min, pos2(rect.min.x + h, rect.max.y)),
            BlurEdge::Right => Rect::from_min_max(pos2(rect.max.x - h, rect.min.y), rect.max),
        }
    }

    /// Band `k` (0 = innermost) of the zone.
    fn band(&self, zone: Rect, k: usize) -> Rect {
        let n = self.bands as f32;
        let (a, b) = (k as f32 / n, (k + 1) as f32 / n);
        match self.edge {
            BlurEdge::Bottom => Rect::from_min_max(
                pos2(zone.min.x, zone.min.y + zone.height() * a),
                pos2(zone.max.x, zone.min.y + zone.height() * b),
            ),
            BlurEdge::Top => Rect::from_min_max(
                pos2(zone.min.x, zone.max.y - zone.height() * b),
                pos2(zone.max.x, zone.max.y - zone.height() * a),
            ),
            BlurEdge::Right => Rect::from_min_max(
                pos2(zone.min.x + zone.width() * a, zone.min.y),
                pos2(zone.min.x + zone.width() * b, zone.max.y),
            ),
            BlurEdge::Left => Rect::from_min_max(
                pos2(zone.max.x - zone.width() * b, zone.min.y),
                pos2(zone.max.x - zone.width() * a, zone.max.y),
            ),
        }
    }

    /// Paint only the translucent veil (no content blur) over `rect`.
    pub fn paint_veil(&self, ui: &Ui, rect: Rect) {
        let bg = self.color.unwrap_or_else(|| Theme::of(ui).card);
        let zone = self.zone(rect);
        let painter = ui.painter_at(rect);
        let clear = with_alpha(bg, 0.0);
        let strong = bg.gamma_multiply(self.veil);
        let soft = bg.gamma_multiply(self.veil * 0.35);
        let (tl, tr, br, bl) = (zone.left_top(), zone.right_top(), zone.right_bottom(), zone.left_bottom());
        let mid = |a: egui::Pos2, b: egui::Pos2| a.lerp(b, 0.55);
        let (c_in, c_mid, c_out) = (clear, soft, strong);
        // two-step gradient so the veil eases in instead of a hard linear ramp
        match self.edge {
            BlurEdge::Bottom => {
                let (ml, mr) = (mid(tl, bl), mid(tr, br));
                glowkit_core::paint::quad(&painter, [tl, tr, mr, ml], [c_in, c_in, c_mid, c_mid]);
                glowkit_core::paint::quad(&painter, [ml, mr, br, bl], [c_mid, c_mid, c_out, c_out]);
            }
            BlurEdge::Top => {
                let (ml, mr) = (mid(bl, tl), mid(br, tr));
                glowkit_core::paint::quad(&painter, [ml, mr, br, bl], [c_mid, c_mid, c_in, c_in]);
                glowkit_core::paint::quad(&painter, [tl, tr, mr, ml], [c_out, c_out, c_mid, c_mid]);
            }
            BlurEdge::Right => {
                let (mt, mb) = (mid(tl, tr), mid(bl, br));
                glowkit_core::paint::quad(&painter, [tl, mt, mb, bl], [c_in, c_mid, c_mid, c_in]);
                glowkit_core::paint::quad(&painter, [mt, tr, br, mb], [c_mid, c_out, c_out, c_mid]);
            }
            BlurEdge::Left => {
                let (mt, mb) = (mid(tr, tl), mid(br, bl));
                glowkit_core::paint::quad(&painter, [mt, tr, br, mb], [c_mid, c_in, c_in, c_mid]);
                glowkit_core::paint::quad(&painter, [tl, mt, mb, bl], [c_out, c_mid, c_mid, c_out]);
            }
        }
    }

    /// Show `content` and blur its edge. The blurred zone is measured against the rect the
    /// content occupies (for a `ScrollArea`, its viewport).
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let start = shape_mark(ui);
        let r = ui.scope_builder(UiBuilder::new(), content);
        let rect = r.response.rect;
        if !ui.is_rect_visible(rect) {
            return r;
        }
        let zone = self.zone(rect);
        // the part of the content that stays sharp
        let sharp = match self.edge {
            BlurEdge::Bottom => Rect::from_min_max(rect.min, pos2(rect.max.x, zone.min.y)),
            BlurEdge::Top => Rect::from_min_max(pos2(rect.min.x, zone.max.y), rect.max),
            BlurEdge::Right => Rect::from_min_max(rect.min, pos2(zone.min.x, rect.max.y)),
            BlurEdge::Left => Rect::from_min_max(pos2(zone.max.x, rect.min.y), rect.max),
        };
        let cap = capture(ui, start);
        let ctx = ui.ctx().clone();
        let mut tess = tessellator(&ctx);
        let bands: Vec<(Rect, f32)> = (0..self.bands)
            .map(|k| {
                let f = ((k as f32 + 1.0) / self.bands as f32).powf(self.curve);
                (self.band(zone, k), (self.strength * f).max(0.6))
            })
            .collect();
        let mut extra: Vec<(Rect, Shape)> = Vec::new();
        cap.restore(&ctx, |_, cs| {
            let bb = cs.shape.visual_bounding_rect();
            if !bb.intersects(zone) {
                return (cs.clip_rect, cs.shape);
            }
            let mut meshes = Vec::new();
            to_meshes(&mut tess, cs.shape.clone(), &mut meshes);
            for (band, radius) in &bands {
                let clip = band.intersect(cs.clip_rect);
                if clip.is_negative() || !bb.intersects(*band) {
                    continue;
                }
                extra.push((clip, blur_meshes(meshes.clone(), *radius, egui::Vec2::ZERO)));
            }
            (cs.clip_rect.intersect(sharp), cs.shape)
        });
        let layer = ui.layer_id();
        ctx.graphics_mut(|g| {
            let list = g.entry(layer);
            for (clip, s) in extra {
                list.add(clip, s);
            }
        });
        self.paint_veil(ui, rect);
        r
    }
}
