use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, StrokeKind, TextureId, Ui, Vec2, Widget};
use glowkit_ai::Icon;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{paint, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::TAU;

fn tex_or_scene(ui: &Ui, tex: Option<TextureId>, scene: usize) -> TextureId {
    tex.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), scene))
}

/// Paint `tex` covering `rect` (center-cropped to the rect's aspect) with rounded corners.
pub(crate) fn image_cover(ui: &Ui, rect: Rect, tex: TextureId, aspect: f32, zoom: f32, corner: egui::CornerRadius, alpha: f32) {
    let ra = rect.width() / rect.height().max(1.0);
    let (mut uw, mut vh) = if ra > aspect { (1.0, aspect / ra) } else { (ra / aspect, 1.0) };
    uw /= zoom;
    vh /= zoom;
    let uv = Rect::from_center_size(pos2(0.5, 0.5), vec2(uw, vh));
    egui::Image::from_texture((tex, rect.size()))
        .uv(uv)
        .corner_radius(corner)
        .tint(Color32::WHITE.gamma_multiply(alpha.clamp(0.0, 1.0)))
        .paint_at(ui, rect);
}

/// A tool result that unfurls a URL: a cover image, the site's favicon and domain, the page title
/// and a two-line description. The card lifts on hover and the image eases into a slow zoom; an
/// arrow slides in to invite opening the link.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = ui.add(glowkit_agents::LinkPreviewResult::new("fieldnotes.dev", "Designing calm agent UIs", "Why quiet interfaces earn trust."));
/// if r.clicked() { /* open the link */ }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LinkPreviewResult<'a> {
    domain: &'a str,
    title: &'a str,
    description: &'a str,
    image: Option<TextureId>,
    scene: usize,
    accent: Option<Color32>,
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> LinkPreviewResult<'a> {
    /// A preview of a page on `domain`.
    pub fn new(domain: &'a str, title: &'a str, description: &'a str) -> Self {
        Self { domain, title, description, image: None, scene: 0, accent: None, width: None, salt: None }
    }
    /// Cover image texture (default: a painted scene).
    pub fn image(mut self, tex: TextureId) -> Self {
        self.image = Some(tex);
        self
    }
    /// Which painted scene to use when no image is set.
    pub fn scene(mut self, i: usize) -> Self {
        self.scene = i;
        self
    }
    /// Favicon color (default: the scene accent).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Card width (default: available, at most 380).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for LinkPreviewResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_link");
        let w = self.width.unwrap_or_else(|| ui.available_width().min(380.0));
        let img_h = (w / 1.9).round();
        let desc = galley_wrap(ui, self.description, 12.0, t.muted_foreground, w - 28.0);
        let title = galley_wrap(ui, self.title, 14.5, t.foreground, w - 28.0);
        let h = img_h + 14.0 + 18.0 + title.size().y + 6.0 + desc.size().y.min(34.0) + 16.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let e = since_change(ui, id, hash_str(self.title));
        keep_alive(ui, e, 1.0);
        let a = appear_curve(e).min(1.0);
        let hov = toggle(ui, id.with("h"), resp.hovered());
        let zoom = glowkit_core::anim::follow(ui, id.with("z"), if resp.hovered() { 1.06 } else { 1.0 }, 4.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let rect = rect.translate(vec2(0.0, (1.0 - a) * 12.0 - hov * 2.0));
        let p = ui.painter();
        paint::soft_shadow(p, rect, 14.0, 14.0 + 10.0 * hov, vec2(0.0, 4.0 + 4.0 * hov), shadow(&t, (0.7 + 0.6 * hov) * a));
        p.rect_filled(rect, cr(14.0), with_alpha(surface(&t), a));
        let img = Rect::from_min_size(rect.min, vec2(w, img_h));
        let tex = tex_or_scene(ui, self.image, self.scene);
        image_cover(ui, img, tex, PaintedScene::ASPECT, zoom, egui::CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, a);
        let p = ui.painter();
        p.rect_stroke(rect, cr(14.0), hairline(&t), StrokeKind::Inside);
        let accent = self.accent.unwrap_or_else(|| PaintedScene::accent(self.scene));
        let y = img.max.y + 14.0;
        let fav = pos2(rect.min.x + 14.0 + 7.0, y + 7.0);
        p.circle_filled(fav, 7.0, accent);
        text_bold(p, fav, Align2::CENTER_CENTER, &self.domain.chars().next().unwrap_or('•').to_uppercase().to_string(), 8.5, Color32::WHITE);
        text(p, pos2(fav.x + 12.0, fav.y), Align2::LEFT_CENTER, self.domain, 11.5, t.muted_foreground);
        let tp = pos2(rect.min.x + 14.0, y + 22.0);
        p.galley(tp, title.clone(), t.foreground);
        p.galley(tp + vec2(0.45, 0.0), title.clone(), t.foreground);
        let dp = tp + vec2(0.0, title.size().y + 6.0);
        let clip = Rect::from_min_size(dp, vec2(w, 34.0));
        p.with_clip_rect(clip).galley(dp, desc, t.muted_foreground);
        // open arrow slides in
        let ac = pos2(rect.max.x - 22.0 - (1.0 - hov) * 6.0, fav.y);
        icon(p, Icon::External, ac, 13.0, with_alpha(t.foreground, 0.25 + 0.75 * hov));
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// Image search / generation results as a tidy gallery: one hero image and a grid of thumbnails
/// that fade up one after another, a "+N" tile for the rest, hover zoom, and click-to-expand: the
/// chosen image grows from its tile into a lightbox inside the card with a caption and arrows.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let captions = ["Alpenglow", "Lagoon", "Aurora", "Dunes", "Forest"];
/// ui.add(glowkit_agents::ImageGalleryResult::new("image_search", &captions));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ImageGalleryResult<'a> {
    tool: &'a str,
    captions: &'a [&'a str],
    textures: Vec<TextureId>,
    width: Option<f32>,
    open: Option<Option<usize>>,
    salt: Option<egui::IdSalt>,
}

#[derive(Clone, Copy, Default)]
struct GalleryState {
    open: Option<usize>,
    from: Option<usize>,
    at: f64,
}

impl<'a> ImageGalleryResult<'a> {
    /// A gallery with one image per caption (painted scenes unless [`Self::textures`] is set).
    pub fn new(tool: &'a str, captions: &'a [&'a str]) -> Self {
        Self { tool, captions, textures: Vec::new(), width: None, open: None, salt: None }
    }
    /// Real image textures (indexed like the captions).
    pub fn textures(mut self, t: Vec<TextureId>) -> Self {
        self.textures = t;
        self
    }
    /// Card width (default: available, at most 520).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Force the lightbox open on image `i` (or closed with `None`) this frame.
    pub fn open(mut self, i: Option<usize>) -> Self {
        self.open = Some(i);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl Widget for ImageGalleryResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_gallery");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(520.0));
        let now = ui.input(|i| i.time);
        let e = since_change(ui, id, self.captions.len() as u64);
        keep_alive(ui, e, 1.6);
        let mut st: GalleryState = glowkit_core::state(ui, id.with("st"), GalleryState::default);
        if let Some(o) = self.open {
            if o != st.open {
                st = GalleryState { open: o, from: o.or(st.open), at: now };
            }
        }
        let n = self.captions.len();
        let head = ToolHead { icon: Icon::Image, title: self.tool, subtitle: None, status: Some((&format!("{n} images"), t.info)) };
        let status = format!("{n} images");
        let head = ToolHead { status: Some((&status, t.info)), ..head };
        let captions = self.captions;
        let textures = self.textures.clone();
        let mut clicked: Option<Option<usize>> = None;
        let tex = |ui: &Ui, i: usize| textures.get(i).copied().unwrap_or_else(|| PaintedScene::texture(ui.ctx(), i));
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            let gap = 6.0;
            let h = w * 0.56;
            let (area, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            let big = Rect::from_min_size(area.min, vec2(w * 0.58 - gap / 2.0, h));
            let cw = w - big.width() - gap;
            let small_w = (cw - gap) / 2.0;
            let small_h = (h - gap) / 2.0;
            let mut tiles = vec![big];
            for k in 0..4 {
                let (cx, cy) = (k % 2, k / 2);
                tiles.push(Rect::from_min_size(
                    pos2(big.max.x + gap + cx as f32 * (small_w + gap), area.min.y + cy as f32 * (small_h + gap)),
                    vec2(small_w, small_h),
                ));
            }
            let shown = n.min(5);
            let open_k = toggle(ui, id.with("lb"), st.open.is_some());
            for (i, tile) in tiles.iter().enumerate().take(shown) {
                let r = ui.interact(*tile, id.with(("tile", i)), Sense::click());
                let hk = toggle(ui, id.with(("th", i)), r.hovered());
                let k = ((e - 0.2 - i as f32 * 0.09) * 4.0).clamp(0.0, 1.0);
                let p = ui.painter().with_clip_rect(tile.intersect(ui.clip_rect()));
                // shimmer placeholder until the image "loads"
                p.rect_filled(*tile, cr(10.0), well(&t));
                if k < 1.0 {
                    let sx = tile.min.x + ((e * 1.3 + i as f32 * 0.2).fract()) * (tile.width() + 80.0) - 40.0;
                    let sh = Rect::from_min_max(pos2(sx - 30.0, tile.min.y), pos2(sx + 30.0, tile.max.y));
                    paint::gradient_rect(&p, sh, 0.0, 0.0, &[with_alpha(t.background, 0.0), with_alpha(t.background, 0.5), with_alpha(t.background, 0.0)]);
                }
                if k > 0.0 {
                    let z = 1.0 + 0.06 * hk + (1.0 - k) * 0.08;
                    image_cover(ui, *tile, tex(ui, i), PaintedScene::ASPECT, z, cr(10.0), k);
                }
                let more = i == 4 && n > 5;
                if more {
                    p.rect_filled(*tile, cr(10.0), Color32::from_black_alpha(110));
                    text_bold(&ui.painter().clone(), tile.center(), Align2::CENTER_CENTER, &format!("+{}", n - 4), 18.0, Color32::WHITE);
                }
                if r.clicked() && open_k < 0.5 {
                    clicked = Some(Some(i));
                }
                if i == 0 && hk > 0.01 {
                    let p = ui.painter();
                    let g = galley(ui, captions[0], 11.5, Color32::WHITE);
                    let cr_ = Rect::from_min_size(pos2(tile.min.x + 10.0, tile.max.y - 30.0), g.size() + vec2(16.0, 8.0));
                    p.rect_filled(cr_, cr(8.0), Color32::from_black_alpha((120.0 * hk) as u8));
                    p.galley(cr_.min + vec2(8.0, 4.0), g, Color32::WHITE);
                }
            }
            // lightbox grows from the tile it was opened from
            let src = st.open.or(st.from);
            if open_k > 0.005 {
                if let Some(i) = src {
                    let from = tiles[i.min(4)];
                    let r = Rect::from_min_max(
                        from.min.lerp(area.min, open_k),
                        from.max.lerp(area.max, open_k),
                    );
                    let p = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
                    p.rect_filled(area, cr(10.0), with_alpha(surface(&t), open_k * 0.9));
                    image_cover(ui, r, tex(ui, i), PaintedScene::ASPECT, 1.0, cr(10.0), 1.0);
                    if open_k > 0.6 {
                        let a = (open_k - 0.6) / 0.4;
                        let p = ui.painter();
                        let g = galley(ui, format!("{}  ·  {} / {}", captions.get(i).unwrap_or(&""), i + 1, n), 12.0, Color32::WHITE);
                        let cap = Rect::from_min_size(pos2(r.min.x + 12.0, r.max.y - 36.0), g.size() + vec2(18.0, 12.0));
                        p.rect_filled(cap, cr(9.0), Color32::from_black_alpha((130.0 * a) as u8));
                        p.galley(cap.min + vec2(9.0, 6.0), g, Color32::WHITE);
                        for (k, (ic, c)) in [(Icon::X, pos2(r.max.x - 22.0, r.min.y + 22.0)), (Icon::ChevronLeft, pos2(r.min.x + 22.0, r.center().y)), (Icon::ChevronRight, pos2(r.max.x - 22.0, r.center().y))].iter().enumerate() {
                            let br = Rect::from_center_size(*c, Vec2::splat(30.0));
                            let b = ui.interact(br, id.with(("lbb", k)), Sense::click());
                            let hk = toggle(ui, id.with(("lbh", k)), b.hovered());
                            let p = ui.painter();
                            p.circle_filled(*c, 15.0, Color32::from_black_alpha((90.0 + 60.0 * hk) as u8));
                            icon(p, *ic, *c, 14.0, with_alpha(Color32::WHITE, a));
                            if b.clicked() {
                                clicked = Some(match k {
                                    0 => None,
                                    1 => Some((i + n - 1) % n),
                                    _ => Some((i + 1) % n),
                                });
                            }
                        }
                    }
                }
            }
        });
        if let Some(o) = clicked {
            st = GalleryState { open: o, from: o.or(st.open), at: now };
        }
        let _ = st.at;
        glowkit_core::set_state(ui, id.with("st"), st);
        ui.interact(rect, id.with("card"), Sense::hover())
    }
}

/// Sky conditions for a [`WeatherResult`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Sky {
    /// Clear and sunny.
    #[default]
    Sunny,
    /// Sun with drifting clouds.
    PartlyCloudy,
    /// Rain falling from a cloud.
    Rain,
    /// A clear night with a moon.
    Night,
}

/// A generative weather card in the style of agent "generative UI": a softly tinted sky that
/// matches the conditions, an animated painted glyph (rotating sun rays, drifting clouds, falling
/// rain or a glowing moon), a big temperature that counts up, and an hourly strip with a smooth
/// temperature curve whose dots pop in left to right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{Sky, WeatherResult};
/// let hourly = [18.0, 19.0, 21.0, 23.0, 24.0, 22.0];
/// ui.add(WeatherResult::new("Lisbon", 24.0, Sky::Sunny).hourly(&hourly).range(16.0, 25.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct WeatherResult<'a> {
    place: &'a str,
    temp: f32,
    sky: Sky,
    summary: Option<&'a str>,
    hourly: &'a [f32],
    range: Option<(f32, f32)>,
    unit: &'a str,
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> WeatherResult<'a> {
    /// Weather for `place` at `temp` degrees.
    pub fn new(place: &'a str, temp: f32, sky: Sky) -> Self {
        Self { place, temp, sky, summary: None, hourly: &[], range: None, unit: "°", width: None, salt: None }
    }
    /// One-line summary (default derived from the sky).
    pub fn summary(mut self, s: &'a str) -> Self {
        self.summary = Some(s);
        self
    }
    /// Hourly temperatures for the strip (up to 8 shown).
    pub fn hourly(mut self, h: &'a [f32]) -> Self {
        self.hourly = h;
        self
    }
    /// Today's low and high.
    pub fn range(mut self, lo: f32, hi: f32) -> Self {
        self.range = Some((lo, hi));
        self
    }
    /// Unit suffix (default "°").
    pub fn unit(mut self, u: &'a str) -> Self {
        self.unit = u;
        self
    }
    /// Card width (default: available, at most 360).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

fn sky_colors(t: &Theme, sky: Sky) -> [Color32; 2] {
    let (a, b) = match sky {
        Sky::Sunny => (hex("#FFE6B8"), hex("#FFF7EA")),
        Sky::PartlyCloudy => (hex("#D9E9FB"), hex("#F3F8FE")),
        Sky::Rain => (hex("#CBD5E1"), hex("#EEF2F7")),
        Sky::Night => (hex("#1E1B4B"), hex("#312E81")),
    };
    if t.dark && sky != Sky::Night {
        [mix(a, t.muted, 0.82), mix(b, t.muted, 0.9)]
    } else {
        [a, b]
    }
}

pub(crate) fn paint_sky_glyph(p: &egui::Painter, c: egui::Pos2, s: f32, sky: Sky, time: f32) {
    let sun = hex("#F59E0B");
    let cloud = |p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32| {
        p.circle_filled(c + vec2(-0.32, 0.08) * s, 0.26 * s, col);
        p.circle_filled(c + vec2(0.0, -0.08) * s, 0.36 * s, col);
        p.circle_filled(c + vec2(0.34, 0.1) * s, 0.24 * s, col);
        p.rect_filled(Rect::from_min_max(c + vec2(-0.32, 0.06) * s, c + vec2(0.34, 0.34) * s), cr(0.0), col);
    };
    match sky {
        Sky::Sunny | Sky::PartlyCloudy => {
            let sc = if sky == Sky::Sunny { c } else { c + vec2(-0.2, -0.18) * s };
            paint::glow(p, sc, s * 0.75, with_alpha(sun, 0.35));
            for i in 0..10 {
                let a = time * 0.4 + i as f32 / 10.0 * TAU;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([sc + d * s * 0.36, sc + d * s * (0.48 + 0.04 * (time * 2.0 + i as f32).sin())], Stroke::new(s * 0.05, sun));
            }
            p.circle_filled(sc, s * 0.27, sun);
            p.circle_filled(sc + vec2(-0.07, -0.07) * s, s * 0.12, with_alpha(Color32::WHITE, 0.35));
            if sky == Sky::PartlyCloudy {
                let drift = (time * 0.5).sin() * s * 0.06;
                cloud(p, c + vec2(0.12 * s + drift, 0.16 * s), s * 0.9, Color32::from_gray(250));
            }
        }
        Sky::Rain => {
            for i in 0..7 {
                let x = c.x + (i as f32 - 3.0) * s * 0.12;
                let ph = (time * 1.6 + i as f32 * 0.37).fract();
                let y0 = c.y + s * 0.2 + ph * s * 0.45;
                p.line_segment([pos2(x, y0), pos2(x - s * 0.04, y0 + s * 0.12)], Stroke::new(s * 0.035, with_alpha(hex("#3B82F6"), 1.0 - ph)));
            }
            cloud(p, c + vec2(0.0, -0.08) * s, s, hex("#94A3B8"));
        }
        Sky::Night => {
            paint::glow(p, c, s * 0.7, with_alpha(hex("#FDE68A"), 0.25));
            p.circle_filled(c, s * 0.3, hex("#FDE68A"));
            p.circle_filled(c + vec2(0.14, -0.1) * s, s * 0.27, hex("#312E81"));
            for (i, (dx, dy)) in [(-0.45, -0.3), (0.42, 0.25), (0.3, -0.42), (-0.3, 0.38)].iter().enumerate() {
                let tw = ((time * 2.0 + i as f32 * 1.7).sin() * 0.5 + 0.5) * 0.7 + 0.3;
                paint::sparkle(p, c + vec2(*dx, *dy) * s, s * 0.08, 0.0, with_alpha(Color32::WHITE, tw));
            }
        }
    }
}

impl Widget for WeatherResult<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_weather");
        let w = self.width.unwrap_or_else(|| ui.available_width().min(360.0));
        let has_strip = !self.hourly.is_empty();
        let h = if has_strip { 236.0 } else { 140.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let e = since_change(ui, id, hash_str(self.place) ^ self.sky as u64);
        let time = glowkit_core::time(ui);
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let a = appear_curve(e).min(1.0);
        let rect = Rect::from_center_size(rect.center() + vec2(0.0, (1.0 - a) * 12.0), rect.size() * (0.97 + 0.03 * a));
        let p = ui.painter();
        let night = self.sky == Sky::Night;
        let [c0, c1] = sky_colors(&t, self.sky);
        paint::soft_shadow(p, rect, 18.0, 18.0, vec2(0.0, 6.0), shadow(&t, 0.9 * a));
        paint::gradient_rect(p, rect, 18.0, 160.0, &[with_alpha(c0, a), with_alpha(c1, a)]);
        p.rect_stroke(rect, cr(18.0), Stroke::new(1.0, with_alpha(if night { Color32::WHITE } else { t.border }, 0.25 * a)), StrokeKind::Inside);
        let fg = if night || t.dark { Color32::WHITE } else { hex("#0F172A") };
        let sub = with_alpha(fg, 0.62);
        let x0 = rect.min.x + 18.0;
        icon(p, Icon::Globe, pos2(x0 + 6.0, rect.min.y + 22.0), 12.0, sub);
        text(p, pos2(x0 + 18.0, rect.min.y + 22.0), Align2::LEFT_CENTER, self.place, 12.5, sub);
        let shown = self.temp * egui::emath::easing::cubic_out((e / 0.9).clamp(0.0, 1.0));
        let tr = text(p, pos2(x0 - 2.0, rect.min.y + 66.0), Align2::LEFT_CENTER, &format!("{:.0}", shown), 52.0, fg);
        text(p, pos2(tr.max.x + 1.0, tr.min.y + 14.0), Align2::LEFT_CENTER, self.unit, 22.0, fg);
        let summary = self.summary.unwrap_or(match self.sky {
            Sky::Sunny => "Sunny and clear",
            Sky::PartlyCloudy => "Partly cloudy",
            Sky::Rain => "Light rain",
            Sky::Night => "Clear night",
        });
        text(p, pos2(x0, rect.min.y + 106.0), Align2::LEFT_CENTER, summary, 13.0, fg);
        if let Some((lo, hi)) = self.range {
            text(p, pos2(x0, rect.min.y + 125.0), Align2::LEFT_CENTER, &format!("H {:.0}{u}  ·  L {:.0}{u}", hi, lo, u = self.unit), 11.5, sub);
        }
        paint_sky_glyph(p, pos2(rect.max.x - 62.0, rect.min.y + 64.0), 78.0, self.sky, time);
        if has_strip {
            let strip = Rect::from_min_max(pos2(rect.min.x + 12.0, rect.min.y + 146.0), pos2(rect.max.x - 12.0, rect.max.y - 12.0));
            p.rect_filled(strip, cr(12.0), with_alpha(Color32::WHITE, if night || t.dark { 0.06 } else { 0.55 }));
            let hrs = &self.hourly[..self.hourly.len().min(8)];
            let n = hrs.len();
            let (lo, hi) = hrs.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
            let span = (hi - lo).max(1.0);
            let step = strip.width() / n as f32;
            let pts: Vec<egui::Pos2> = hrs
                .iter()
                .enumerate()
                .map(|(i, v)| pos2(strip.min.x + step * (i as f32 + 0.5), strip.min.y + 44.0 - (v - lo) / span * 22.0))
                .collect();
            let draw = ((e - 0.3) / 0.9).clamp(0.0, 1.0);
            let smooth = paint::monotone(&pts, 10);
            let cut = ((smooth.len() as f32) * draw) as usize;
            if cut > 1 {
                paint::gradient_polyline(p, &smooth[..cut], false, 2.0, |u| mix(hex("#F59E0B"), hex("#F97316"), u).gamma_multiply(if night { 0.9 } else { 1.0 }));
            }
            for (i, q) in pts.iter().enumerate() {
                let k = ((e - 0.35 - i as f32 * 0.08) * 6.0).clamp(0.0, 1.0);
                if k <= 0.0 {
                    continue;
                }
                p.circle_filled(*q, 3.2 * k, if night || t.dark { hex("#FDE68A") } else { Color32::WHITE });
                p.circle_stroke(*q, 3.2 * k, Stroke::new(1.5, hex("#F59E0B")));
                text(p, *q - vec2(0.0, 11.0), Align2::CENTER_CENTER, &format!("{:.0}°", hrs[i]), 10.5, with_alpha(fg, k));
                let hour = (9 + i * 2) % 24;
                text(p, pos2(q.x, strip.max.y - 12.0), Align2::CENTER_CENTER, &format!("{hour}:00"), 10.0, with_alpha(sub, k));
            }
        }
        resp
    }
}
