use super::common::{lerp_c, sstep, sticky};
use egui::epaint::{Mesh, Vertex};
use egui::{
    pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, TextureId, Ui, Vec2, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::paint::rounded_rect_points;
use glowkit_core::Theme;
use glowkit_effects::PaintedScene;

/// A rounded rect filled with a texture, where `uv_of` maps screen points to texture coords.
fn textured_round_rect(
    painter: &egui::Painter,
    r: Rect,
    radius: f32,
    tex: TextureId,
    uv_of: impl Fn(Pos2) -> Pos2,
    tint: Color32,
) {
    if r.width() < 1.0 || r.height() < 1.0 {
        return;
    }
    let outline = rounded_rect_points(r, radius.min(r.width() * 0.5).min(r.height() * 0.5), 10);
    let c = r.center();
    let mut m = Mesh::with_texture(tex);
    m.vertices.push(Vertex { pos: c, uv: uv_of(c), color: tint });
    for p in &outline {
        m.vertices.push(Vertex { pos: *p, uv: uv_of(*p), color: tint });
    }
    let n = outline.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
    // anti-aliased rim
    painter.add(Shape::closed_line(outline, Stroke::new(1.0, with_alpha(tint, 0.25))));
}

/// Allocate either a stage (explicit progress) or a tall sticky track inside a scroll area.
fn stage(ui: &mut Ui, height: f32, track: f32, progress: Option<f32>) -> (Response, Rect, f32) {
    let w = ui.available_width();
    match progress {
        Some(p) => {
            let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
            (resp, rect, p)
        }
        None => {
            let (rect, resp) = ui.allocate_exact_size(vec2(w, height * track), Sense::hover());
            let (p, st) = sticky(ui, rect, height);
            (resp, st, p)
        }
    }
}

/// A hero that starts as a small rounded window onto a picture. As the page scrolls the window
/// opens like a camera aperture until it fills the stage, the picture eases back from a slight
/// zoom, and the two words of the headline slide apart to make room (the parts of the letters
/// over the picture take on its light). A caption settles in once it is fully open.
///
/// Inside a `ScrollArea` it is a tall sticky track driven by the scroll position; set
/// [`Self::progress`] to drive it yourself instead.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// egui::ScrollArea::vertical().show(ui, |ui| {
///     ui.add(glowkit_interactions::ApertureZoom::new("Open", "wide."));
/// });
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ApertureZoom {
    left: String,
    right: String,
    caption: (String, String),
    image: Option<TextureId>,
    scene: usize,
    height: f32,
    track: f32,
    zoom: f32,
    progress: Option<f32>,
}

impl ApertureZoom {
    /// A headline split into a `left` and `right` word.
    pub fn new(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            caption: ("CHAPTER ONE".into(), "The long way round".into()),
            image: None,
            scene: 0,
            height: 380.0,
            track: 2.6,
            zoom: 1.3,
            progress: None,
        }
    }
    /// Caption pill: small label and title.
    pub fn caption(mut self, label: impl Into<String>, title: impl Into<String>) -> Self {
        self.caption = (label.into(), title.into());
        self
    }
    /// Your own picture (cover-fit, assumed 16:10).
    pub fn image(mut self, tex: TextureId) -> Self {
        self.image = Some(tex);
        self
    }
    /// Built-in painted scene to show when no image is set.
    pub fn scene(mut self, i: usize) -> Self {
        self.scene = i;
        self
    }
    /// Stage height (default 380).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Scroll distance as a multiple of the stage height (default 2.6).
    pub fn track(mut self, k: f32) -> Self {
        self.track = k.clamp(1.2, 8.0);
        self
    }
    /// Starting zoom of the picture (default 1.3).
    pub fn zoom(mut self, k: f32) -> Self {
        self.zoom = k.clamp(1.0, 3.0);
        self
    }
    /// Drive it directly (`0` closed, `1` open) instead of from the scroll position.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
}

impl Widget for ApertureZoom {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (response, st, p) = stage(ui, self.height, self.track, self.progress);
        if !ui.is_rect_visible(st) {
            return response;
        }
        let painter = ui.painter_at(st.intersect(ui.clip_rect()));
        painter.rect_filled(st, 0.0, t.background);
        let k = glowkit_core::ease(glowkit_core::Easing::InOutCubic, sstep(0.0, 0.85, p));
        let small = Rect::from_center_size(st.center(), vec2(st.width() * 0.3, st.height() * 0.62));
        let win = Rect::from_min_max(small.min.lerp(st.min, k), small.max.lerp(st.max, k));
        let radius = 26.0 * (1.0 - k);
        let tex = self.image.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), self.scene));
        let zoom = self.zoom + (1.0 - self.zoom) * k;
        let aspect = PaintedScene::ASPECT;
        let sa = st.width() / st.height();
        let (uw, vh) = if sa > aspect { (1.0, aspect / sa) } else { (sa / aspect, 1.0) };
        let uv_of = |q: Pos2| {
            let fx = (q.x - st.center().x) / st.width() / zoom;
            let fy = (q.y - st.center().y) / st.height() / zoom;
            pos2(0.5 + fx * uw, 0.5 + fy * vh)
        };
        textured_round_rect(&painter, win, radius, tex, uv_of, Color32::WHITE);
        // headline: the two words part as the aperture opens
        let font = FontId::proportional((st.height() * 0.2).clamp(36.0, 96.0));
        let gl = painter.layout_no_wrap(self.left.clone(), font.clone(), t.foreground);
        let gr = painter.layout_no_wrap(self.right.clone(), font.clone(), t.foreground);
        let gap = 18.0 + k * st.width() * 0.62;
        let y = st.center().y - gl.size().y * 0.5;
        let lpos = pos2(st.center().x - gap * 0.5 - gl.size().x, y);
        let rpos = pos2(st.center().x + gap * 0.5, y);
        let fade = 1.0 - sstep(0.7, 0.95, p);
        let base = with_alpha(t.foreground, fade);
        painter.galley_with_override_text_color(lpos, gl.clone(), base);
        painter.galley_with_override_text_color(rpos, gr.clone(), base);
        // over the picture the letters take on its light
        let lit = painter.with_clip_rect(win.intersect(painter.clip_rect()));
        let glow = with_alpha(lerp_c(Color32::from_rgb(196, 240, 214), Color32::from_rgb(255, 214, 170), 0.4), fade);
        lit.galley_with_override_text_color(lpos, gl, glow);
        lit.galley_with_override_text_color(rpos, gr, with_alpha(Color32::from_rgb(40, 70, 120), fade));
        // caption
        let c = sstep(0.82, 1.0, p);
        if c > 0.0 {
            let g1 = painter.layout_no_wrap(
                self.caption.0.clone(),
                FontId::monospace(8.5),
                with_alpha(Color32::WHITE, 0.6 * c),
            );
            let g2 = painter.layout_no_wrap(
                self.caption.1.clone(),
                FontId::proportional(12.0),
                with_alpha(Color32::WHITE, c),
            );
            let w = g1.size().x + g2.size().x + 34.0;
            let pill = Rect::from_center_size(pos2(st.center().x, st.max.y - 46.0 + (1.0 - c) * 12.0), vec2(w, 28.0));
            painter.rect_filled(pill, 14.0, Color32::from_black_alpha((150.0 * c) as u8));
            painter.galley(pill.min + vec2(14.0, (28.0 - g1.size().y) * 0.5), g1.clone(), Color32::WHITE);
            painter.galley(pill.min + vec2(20.0 + g1.size().x, (28.0 - g2.size().y) * 0.5), g2, Color32::WHITE);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A small pill of "footage" sitting between two words that opens into a full-bleed frame as
/// the page scrolls. The words ride the frame's edges out of the screen, the footage keeps
/// drifting (a slow pan with a sweeping light), and a play label surfaces at the end.
///
/// Inside a `ScrollArea` it is a tall sticky track driven by the scroll position; set
/// [`Self::progress`] to drive it yourself instead.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// egui::ScrollArea::vertical().show(ui, |ui| {
///     ui.add(glowkit_interactions::ZoomReveal::new("North", "Light").play_label("Begin the film"));
/// });
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ZoomReveal {
    left: String,
    right: String,
    play: String,
    image: Option<TextureId>,
    scene: usize,
    height: f32,
    track: f32,
    font_size: f32,
    progress: Option<f32>,
}

impl ZoomReveal {
    /// Two words with a pill of footage between them.
    pub fn new(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            play: "Begin the film".into(),
            image: None,
            scene: 6,
            height: 380.0,
            track: 2.6,
            font_size: 28.0,
            progress: None,
        }
    }
    /// The label that appears once the frame is open.
    pub fn play_label(mut self, s: impl Into<String>) -> Self {
        self.play = s.into();
        self
    }
    /// Your own footage still (cover-fit, assumed 16:10).
    pub fn image(mut self, tex: TextureId) -> Self {
        self.image = Some(tex);
        self
    }
    /// Built-in painted scene to use when no image is set.
    pub fn scene(mut self, i: usize) -> Self {
        self.scene = i;
        self
    }
    /// Stage height (default 380).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Scroll distance as a multiple of the stage height (default 2.6).
    pub fn track(mut self, k: f32) -> Self {
        self.track = k.clamp(1.2, 8.0);
        self
    }
    /// Word size (default 28).
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.clamp(12.0, 80.0);
        self
    }
    /// Drive it directly (`0` pill, `1` full-bleed) instead of from the scroll position.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
}

impl Widget for ZoomReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (response, st, p) = stage(ui, self.height, self.track, self.progress);
        let time = glowkit_core::time(ui);
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(st) {
            return response;
        }
        let painter = ui.painter_at(st.intersect(ui.clip_rect()));
        painter.rect_filled(st, 0.0, t.background);
        let k = glowkit_core::ease(glowkit_core::Easing::InOutCubic, sstep(0.0, 0.8, p));
        let pill_h = self.font_size * 0.85;
        let small = Rect::from_center_size(st.center(), vec2(pill_h * 2.9, pill_h));
        let full = st.shrink2(vec2(-2.0, -2.0));
        let win = Rect::from_min_max(small.min.lerp(full.min, k), small.max.lerp(full.max, k));
        let radius = (pill_h * 0.5) * (1.0 - k) + 14.0 * (k * (1.0 - k)) * 2.0;
        let tex = self.image.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), self.scene));
        // footage: a slow pan across the still
        let pan = (time * 0.05).sin() * 0.08;
        let zoom = 1.15 - 0.1 * k;
        let uv_of = |q: Pos2| {
            let fx = (q.x - st.center().x) / st.width() / zoom;
            let fy = (q.y - st.center().y) / st.height() / zoom;
            let aspect = PaintedScene::ASPECT;
            let sa = st.width() / st.height();
            let (uw, vh) = if sa > aspect { (1.0, aspect / sa) } else { (sa / aspect, 1.0) };
            pos2(0.5 + fx * uw + pan, 0.5 + fy * vh)
        };
        textured_round_rect(&painter, win, radius, tex, uv_of, Color32::WHITE);
        // a sweeping band of light across the footage
        let lp = painter.with_clip_rect(win.shrink(1.0).intersect(painter.clip_rect()));
        let sweep = (time * 0.35).fract();
        let cx = win.min.x - win.width() * 0.3 + sweep * win.width() * 1.6;
        let mut m = Mesh::default();
        for (dx, a) in [(-60.0, 0.0), (-10.0, 0.16), (10.0, 0.16), (60.0, 0.0)] {
            m.colored_vertex(pos2(cx + dx - 80.0, win.max.y), Color32::from_white_alpha((a * 255.0) as u8));
            m.colored_vertex(pos2(cx + dx + 80.0, win.min.y), Color32::from_white_alpha((a * 255.0) as u8));
        }
        for i in 0..3u32 {
            let a = i * 2;
            m.add_triangle(a, a + 1, a + 3);
            m.add_triangle(a, a + 3, a + 2);
        }
        lp.add(Shape::mesh(m));
        // the words ride the frame's edges
        let font = FontId::proportional(self.font_size);
        let gap = 10.0;
        painter.text(
            pos2(win.min.x - gap, st.center().y),
            Align2::RIGHT_CENTER,
            &self.left,
            font.clone(),
            t.foreground,
        );
        painter.text(pos2(win.max.x + gap, st.center().y), Align2::LEFT_CENTER, &self.right, font, t.foreground);
        // play label
        let a = sstep(0.82, 1.0, p);
        if a > 0.0 {
            let g = painter.layout_no_wrap(self.play.clone(), FontId::proportional(13.0), Color32::WHITE);
            let pill = Rect::from_center_size(st.center() + vec2(0.0, (1.0 - a) * 10.0), vec2(g.size().x + 54.0, 34.0));
            painter.rect_filled(pill, 17.0, Color32::from_black_alpha((110.0 * a) as u8));
            painter.galley(pill.min + vec2(16.0, (34.0 - g.size().y) * 0.5), g, with_alpha(Color32::WHITE, a));
            let c = pill.right_center() - vec2(20.0, 0.0);
            painter.circle_filled(c, 9.0, with_alpha(Color32::WHITE, a));
            painter.add(Shape::convex_polygon(
                vec![c + vec2(-2.5, -4.0), c + vec2(4.5, 0.0), c + vec2(-2.5, 4.0)],
                with_alpha(Color32::BLACK, a),
                Stroke::NONE,
            ));
        }
        let _: Vec2 = Vec2::ZERO;
        response
    }
}
