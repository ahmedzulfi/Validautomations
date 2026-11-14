use super::common::{cover_uv, image_rect, paint_clipped, PaintedScene};
use egui::emath::TSTransform;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, TextureId, Ui, UiBuilder,
};
use glowkit_core::anim::{ease, smoothstep, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};

/// Smoothly wandering idle position inside `rect` (a slow Lissajous curve).
fn wander(rect: Rect, t: f32, salt: f32) -> Pos2 {
    let u = 0.5 + 0.36 * (t * 0.53 + salt).sin() * (t * 0.21).cos().abs().max(0.6);
    let v = 0.5 + 0.3 * (t * 0.77 + salt * 2.0).sin();
    pos2(rect.min.x + rect.width() * u, rect.min.y + rect.height() * v)
}

/// Tracks the pointer, or wanders when idle (with autoplay). Returns (position, active 0..1).
fn pointer_or_wander(ui: &Ui, id: Id, rect: Rect, response: &Response, autoplay: bool) -> (Pos2, bool, f32) {
    let hover = response.hover_pos();
    let dt = glowkit_core::dt(ui);
    let idle = glowkit_core::with_state(ui, id.with("idle"), || 10.0f32, |s| {
        if hover.is_some() {
            *s = 0.0;
        } else {
            *s += dt;
        }
        *s
    });
    let time = glowkit_core::time(ui);
    let (target, active) = match hover {
        Some(p) => (p, true),
        None if autoplay && idle > 0.8 => (wander(rect, time, (id.value() % 97) as f32 * 0.1), true),
        None => (glowkit_core::state(ui, id.with("last"), || rect.center()), false),
    };
    glowkit_core::set_state(ui, id.with("last"), target);
    let p = glowkit_core::anim::spring2(ui, id.with("pos"), target.to_vec2(), 170.0, 22.0).to_pos2();
    if autoplay {
        glowkit_core::animate(ui);
    }
    (p, active, idle)
}

// ---------------------------------------------------------------------------------------------

/// A magnifying glass over any painted scene: inside the circle the scene is re-painted
/// `zoom` times larger (crisp, not upscaled pixels), clipped to a glassy ring. It follows the
/// pointer, and drifts around on its own when [`Self::autoplay`] is on.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::Lens::new().zoom(2.2).radius(70.0).show(ui, egui::vec2(480.0, 300.0), |painter, rect| {
///     painter.rect_filled(rect, 12.0, egui::Color32::DARK_BLUE);
///     painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Tiny print", egui::FontId::proportional(12.0), egui::Color32::WHITE);
/// });
/// ```
#[must_use = "call `.show(ui, size, |painter, rect| ...)`"]
pub struct Lens {
    zoom: f32,
    radius: f32,
    autoplay: bool,
    fixed: Option<egui::Vec2>,
    ring: Option<Color32>,
    salt: u64,
}

impl Default for Lens {
    fn default() -> Self {
        Self::new()
    }
}

impl Lens {
    /// A 2x lens of radius 70.
    pub fn new() -> Self {
        Self { zoom: 2.0, radius: 70.0, autoplay: false, fixed: None, ring: None, salt: 0 }
    }
    /// Magnification factor.
    pub fn zoom(mut self, z: f32) -> Self {
        self.zoom = z.clamp(1.0, 8.0);
        self
    }
    /// Lens radius in points.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(8.0);
        self
    }
    /// Wander around the scene when the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Keep the lens at a fixed spot (fractions of the size, `0..1`), ignoring the pointer.
    pub fn fixed(mut self, at: egui::Vec2) -> Self {
        self.fixed = Some(at);
        self
    }
    /// Ring color (defaults to translucent white).
    pub fn ring_color(mut self, c: Color32) -> Self {
        self.ring = Some(c);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show `scene(painter, rect)` at `size` with the lens over it. The scene is called twice
    /// per frame (once normally, once magnified), so keep it reasonably cheap.
    pub fn show(self, ui: &mut Ui, size: egui::Vec2, scene: impl Fn(&Painter, Rect)) -> Response {
        let size = size.min(vec2(ui.available_width(), f32::INFINITY));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_lens", self.salt)), Sense::hover());
        let id = response.id;
        let (pos, active) = match self.fixed {
            Some(f) => (rect.min + rect.size() * f, true),
            None => {
                let (p, a, _) = pointer_or_wander(ui, id, rect, &response, self.autoplay);
                (p, a)
            }
        };
        let vis = glowkit_core::anim::spring(ui, id.with("vis"), if active { 1.0 } else { 0.0 }, 220.0, 20.0);
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::None);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let clip = rect.intersect(ui.clip_rect());
        scene(&ui.painter().with_clip_rect(clip), rect);
        if vis < 0.01 {
            return response;
        }
        let r = self.radius * vis.max(0.0);
        let t = Theme::of(ui);
        let painter = ui.painter().with_clip_rect(clip.expand(self.radius));
        // drop shadow under the glass
        paint::glow(&painter, pos + vec2(0.0, 8.0), r * 1.35, Color32::from_black_alpha(if t.dark { 120 } else { 60 }));
        let outline = paint::circle_points(pos, r, 72);
        let safe = Rect::from_center_size(pos, vec2(r, r) * 1.38);
        let zoom = 1.0 + (self.zoom - 1.0) * vis;
        let zoomed = Rect::from_min_max(pos + (rect.min - pos) * zoom, pos + (rect.max - pos) * zoom);
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
        child.set_clip_rect(clip.expand(self.radius));
        paint_clipped(&mut child, &outline, safe, TSTransform::IDENTITY, 1.0, |ui| {
            scene(&ui.painter().with_clip_rect(Rect::EVERYTHING), zoomed);
        });
        // glass: a soft top-left highlight and a crisp rim
        let ring = self.ring.unwrap_or(with_alpha(Color32::WHITE, 0.75));
        let hl_c = pos + vec2(-r * 0.35, -r * 0.4);
        paint::glow_ellipse(&painter, hl_c, vec2(r * 0.55, r * 0.35), with_alpha(Color32::WHITE, 0.16 * vis));
        painter.circle_stroke(pos, r, Stroke::new(3.0, with_alpha(Color32::BLACK, 0.18)));
        painter.circle_stroke(pos, r - 0.5, Stroke::new(1.6, ring.gamma_multiply(vis)));
        painter.circle_stroke(pos, r - 3.0, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.14 * vis)));
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// An inline link that pops a preview card above itself on hover: the card springs up from
/// the text, leans toward the pointer and shows a screenshot of the destination.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.horizontal_wrapped(|ui| {
///     ui.label("Built with");
///     ui.add(glowkit_effects::LinkPreview::new("egui", "https://egui.rs"));
/// });
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LinkPreview {
    text: String,
    url: String,
    texture: Option<(TextureId, f32)>,
    scene: usize,
    card: egui::Vec2,
    font_size: f32,
    color: Option<Color32>,
    autoplay: Option<f32>,
}

impl LinkPreview {
    /// A link showing `text`, pointing at `url`.
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
            texture: None,
            scene: 0,
            card: vec2(220.0, 136.0),
            font_size: 16.0,
            color: None,
            autoplay: None,
        }
    }
    /// Preview screenshot (`aspect` = width / height).
    pub fn texture(mut self, t: TextureId, aspect: f32) -> Self {
        self.texture = Some((t, aspect.max(0.01)));
        self
    }
    /// Built-in [`PaintedScene`] to use when no texture is set.
    pub fn scene(mut self, index: usize) -> Self {
        self.scene = index;
        self
    }
    /// Preview card size.
    pub fn card_size(mut self, s: egui::Vec2) -> Self {
        self.card = s.max(vec2(40.0, 30.0));
        self
    }
    /// Link text size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.max(6.0);
        self
    }
    /// Link color (defaults to the theme foreground with a gradient underline).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Pop the preview on its own every 6 s while idle, starting `offset` seconds into the cycle
    /// (give each link a different offset).
    pub fn autoplay(mut self, offset: f32) -> Self {
        self.autoplay = Some(offset);
        self
    }
}

impl egui::Widget for LinkPreview {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.foreground);
        let galley = ui.painter().layout_no_wrap(self.text.clone(), FontId::proportional(self.font_size), color);
        let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(0.0, 3.0), Sense::click());
        let id = response.id;
        if response.clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&self.url));
        }
        let time = glowkit_core::time(ui);
        let hovered = response.hovered();
        let auto = self.autoplay.is_some_and(|off| (time - off).rem_euclid(6.0) < 2.4);
        let open = hovered || auto;
        if self.autoplay.is_some() {
            glowkit_core::animate(ui);
        }
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let p = glowkit_core::anim::spring(ui, id.with("open"), if open { 1.0 } else { 0.0 }, 260.0, 17.0);
        let lean_target = match response.hover_pos() {
            Some(q) => ((q.x - rect.center().x) / rect.width().max(1.0)).clamp(-0.5, 0.5) * 2.0,
            None => (time * 1.3).sin() * 0.4 * if auto { 1.0 } else { 0.0 },
        };
        let lean = glowkit_core::anim::spring(ui, id.with("lean"), lean_target, 160.0, 16.0);
        if !ui.is_rect_visible(rect) && p < 0.01 {
            return response;
        }
        let painter = ui.painter();
        // text with an animated gradient underline
        painter.galley(rect.min, galley.clone(), color);
        let base = rect.max.y - 1.5;
        let ul = [pos2(rect.min.x, base), pos2(rect.max.x, base)];
        let stops = [t.gradient[0], t.gradient[1], t.gradient[2]];
        let k = 0.35 + 0.65 * p.clamp(0.0, 1.0);
        paint::gradient_polyline(painter, &ul, false, 1.6, |u| with_alpha(glowkit_core::color::gradient(&stops, u), k));
        if p > 0.005 {
            let layer = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, id.with("card")));
            let q = p.clamp(0.0, 1.2);
            let s = 0.6 + 0.4 * p;
            let card = self.card * s;
            let max_shift = self.card.x * 0.25;
            let center = pos2(rect.center().x + lean * max_shift, rect.min.y - 14.0 - card.y / 2.0 - 16.0 * (1.0 - p));
            let r = Rect::from_center_size(center, card);
            let a = q.min(1.0);
            paint::soft_shadow(&layer, r, 14.0, 26.0, vec2(0.0, 12.0), Color32::from_black_alpha((110.0 * a) as u8));
            let frame_c = if t.dark { t.popover } else { Color32::WHITE };
            layer.rect(r, CornerRadius::same(14), with_alpha(frame_c, a), Stroke::new(1.0, with_alpha(t.border, a)), StrokeKind::Inside);
            let img = r.shrink(5.0 * s);
            let (tex, aspect) = self.texture.unwrap_or((PaintedScene::texture(ui.ctx(), self.scene), PaintedScene::ASPECT));
            let uv = cover_uv(aspect, img.width() / img.height().max(1.0));
            image_rect(&layer, img, 10.0, tex, uv, Color32::WHITE.gamma_multiply(a));
            // fake browser chrome + url chip
            let bar = Rect::from_min_size(img.min, vec2(img.width(), 18.0 * s));
            let pts = paint::rounded_rect_points(bar, 9.0 * s, 4);
            paint::fill_convex_fn(&layer, &pts, 2, |_| with_alpha(Color32::BLACK, 0.32 * a));
            for (i, c) in ["#FF5F57", "#FEBC2E", "#28C840"].iter().enumerate() {
                layer.circle_filled(bar.left_center() + vec2(10.0 * s + i as f32 * 9.0 * s, 0.0), 2.6 * s, with_alpha(glowkit_core::hex(c), a));
            }
            let domain = self.url.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
            layer.text(
                bar.center() + vec2(10.0 * s, 0.0),
                Align2::CENTER_CENTER,
                domain,
                FontId::proportional(10.0 * s),
                with_alpha(Color32::WHITE, 0.9 * a),
            );
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A spotlight mask: a circle follows the pointer and reveals an alternate version of the
/// content underneath (different colors, a secret message...). The circle is small while idle
/// and swells when you hover; with [`Self::autoplay`] it roams and pulses on its own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::SvgMaskEffect::new().show(
///     ui,
///     |ui, rect| { ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Hello", egui::FontId::proportional(40.0), egui::Color32::GRAY); },
///     |ui, rect| {
///         ui.painter().rect_filled(rect, 0.0, egui::Color32::GOLD);
///         ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "Secret", egui::FontId::proportional(40.0), egui::Color32::BLACK);
///     },
/// );
/// ```
#[must_use = "call `.show(ui, base, reveal)`"]
pub struct SvgMaskEffect {
    height: f32,
    idle_radius: f32,
    hover_radius: f32,
    autoplay: bool,
    edge: Option<Color32>,
    salt: u64,
}

impl Default for SvgMaskEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgMaskEffect {
    /// A 360-point tall reveal area.
    pub fn new() -> Self {
        Self { height: 360.0, idle_radius: 22.0, hover_radius: 130.0, autoplay: true, edge: None, salt: 0 }
    }
    /// Height of the area.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(40.0);
        self
    }
    /// Mask radius while not hovering.
    pub fn idle_radius(mut self, r: f32) -> Self {
        self.idle_radius = r.max(0.0);
        self
    }
    /// Mask radius while hovering.
    pub fn hover_radius(mut self, r: f32) -> Self {
        self.hover_radius = r.max(1.0);
        self
    }
    /// Roam and pulse when the pointer is away (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Color of the mask's soft edge ring (defaults to the theme accent gradient).
    pub fn edge_color(mut self, c: Color32) -> Self {
        self.edge = Some(c);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Paint `base(ui, rect)` and, inside the moving circle, `reveal(ui, rect)` (which should
    /// fill its own background).
    pub fn show(self, ui: &mut Ui, base: impl FnOnce(&mut Ui, Rect), reveal: impl FnOnce(&mut Ui, Rect)) -> Response {
        let t = Theme::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_svg_mask", self.salt)), Sense::hover());
        let id = response.id;
        let (pos, _, idle) = pointer_or_wander(ui, id, rect, &response, self.autoplay);
        let time = glowkit_core::time(ui);
        let target = if response.hovered() {
            self.hover_radius
        } else if self.autoplay && idle > 0.8 {
            // breathe between small and large
            let k = smoothstep(-0.3, 0.6, (time * 0.7).sin());
            self.idle_radius + (self.hover_radius - self.idle_radius) * k
        } else {
            self.idle_radius
        };
        let r = glowkit_core::anim::spring(ui, id.with("r"), target, 120.0, 14.0).max(0.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let clip = rect.intersect(ui.clip_rect());
        {
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt("mask_base"));
            child.set_clip_rect(clip);
            base(&mut child, rect);
        }
        if r > 0.5 {
            let outline = paint::circle_points(pos, r, 80);
            let safe = Rect::from_center_size(pos, vec2(r, r) * 1.38);
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt("mask_reveal"));
            child.set_clip_rect(clip);
            paint_clipped(&mut child, &outline, safe, TSTransform::IDENTITY, 1.0, |ui| reveal(ui, rect));
            let painter = ui.painter().with_clip_rect(clip);
            let edge = self.edge.unwrap_or(t.gradient[1]);
            painter.circle_stroke(pos, r, Stroke::new(1.5, with_alpha(edge, 0.85)));
            painter.circle_stroke(pos, r + 3.0, Stroke::new(4.0, with_alpha(edge, 0.12)));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A card with a line of text that hides a second line: drag (or hover) across it and
/// everything left of a glowing divider shows the hidden text, sprinkled with twinkling stars.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(
///     glowkit_effects::TextRevealCard::new("You know the business", "I know the chemistry")
///         .title("Sometimes, you just need to see it."),
/// );
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TextRevealCard {
    text: String,
    reveal: String,
    title: String,
    description: String,
    size: egui::Vec2,
    font_size: f32,
    autoplay: bool,
}

impl TextRevealCard {
    /// A card showing `text`, hiding `reveal`.
    pub fn new(text: impl Into<String>, reveal: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reveal: reveal.into(),
            title: String::new(),
            description: String::new(),
            size: vec2(620.0, 300.0),
            font_size: 46.0,
            autoplay: true,
        }
    }
    /// Small heading at the top of the card.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Muted line under the title.
    pub fn description(mut self, s: impl Into<String>) -> Self {
        self.description = s.into();
        self
    }
    /// Card size.
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s.max(vec2(120.0, 80.0));
        self
    }
    /// Size of the big text.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.max(8.0);
        self
    }
    /// Sweep the divider back and forth when idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for TextRevealCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.min(vec2(ui.available_width(), f32::INFINITY));
        let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), size.y), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), size);
        let response = ui.interact(rect, ui.next_auto_id().with("glowkit_text_reveal"), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let pad = 30.0;
        let text_area = Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y + 96.0), pos2(rect.max.x - pad, rect.max.y - 26.0));
        let hover = response.hover_pos();
        let idle = glowkit_core::with_state(ui, id.with("idle"), || 10.0f32, |s| {
            if hover.is_some() {
                *s = 0.0;
            } else {
                *s += dt;
            }
            *s
        });
        let target = match hover {
            Some(p) => ((p.x - text_area.min.x) / text_area.width()).clamp(0.0, 1.0),
            None if self.autoplay && idle > 0.6 => {
                let ph = (time * 0.35).rem_euclid(1.0);
                // reveal sweep: in, hold, out, hold
                if ph < 0.4 {
                    ease(Easing::InOutCubic, ph / 0.4)
                } else if ph < 0.55 {
                    1.0
                } else if ph < 0.85 {
                    1.0 - ease(Easing::InOutCubic, (ph - 0.55) / 0.3)
                } else {
                    0.0
                }
            }
            None => 0.0,
        };
        let x = glowkit_core::anim::follow(ui, id.with("x"), target, 9.0);
        glowkit_core::animate(ui);
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(30.0));
        let face = if t.dark { mix(t.card, Color32::from_rgb(29, 28, 32), 0.7) } else { Color32::WHITE };
        let sc = if t.dark { Color32::from_black_alpha(150) } else { Color32::from_rgba_unmultiplied(30, 30, 70, 40) };
        paint::soft_shadow(&painter, rect, 18.0, 30.0, vec2(0.0, 12.0), sc);
        painter.rect(rect, CornerRadius::same(18), face, Stroke::new(1.0, t.border), StrokeKind::Inside);
        if !self.title.is_empty() {
            painter.text(rect.min + vec2(pad, 30.0), Align2::LEFT_TOP, &self.title, FontId::proportional(17.0), t.foreground);
        }
        if !self.description.is_empty() {
            painter.text(rect.min + vec2(pad, 56.0), Align2::LEFT_TOP, &self.description, FontId::proportional(13.5), t.muted_foreground);
        }
        let font = FontId::proportional(self.font_size.min(text_area.height() * 0.8));
        let base_c = if t.dark { Color32::from_gray(64) } else { Color32::from_gray(205) };
        let gb = painter.layout(self.text.clone(), font.clone(), base_c, text_area.width());
        let gr = painter.layout(self.reveal.clone(), font, Color32::WHITE, text_area.width());
        let pos_b = pos2(text_area.min.x, text_area.center().y - gb.size().y / 2.0);
        let pos_r = pos2(text_area.min.x, text_area.center().y - gr.size().y / 2.0);
        let split = text_area.min.x + text_area.width() * x;
        let left = Rect::from_min_max(pos2(rect.min.x, text_area.min.y - 30.0), pos2(split, text_area.max.y + 20.0));
        let right = Rect::from_min_max(pos2(split, text_area.min.y - 30.0), pos2(rect.max.x, text_area.max.y + 20.0));
        painter.with_clip_rect(right.intersect(ui.clip_rect())).galley(pos_b, gb, base_c);
        let lp = painter.with_clip_rect(left.intersect(ui.clip_rect()));
        // the hidden text: a soft white-to-silver gradient with a faint glow behind it
        let stops = if t.dark {
            [Color32::WHITE, Color32::from_gray(170)]
        } else {
            [t.foreground, mix(t.foreground, t.gradient[1], 0.7)]
        };
        let rr = Rect::from_min_size(pos_r, gr.size());
        paint::glow_ellipse(&lp, rr.center(), rr.size() * vec2(0.6, 0.9), with_alpha(t.gradient[1], if t.dark { 0.16 } else { 0.1 }));
        paint::galley_with(&lp, pos_r, &gr, |q| paint::linear_fn(rr, 90.0, &stops)(q));
        // stars over the revealed part
        for i in 0..44u64 {
            let sx = hash01(i * 3 + 1);
            let sy = hash01(i * 7 + 2);
            let sp = 0.6 + hash01(i * 11 + 3);
            let px = text_area.min.x + text_area.width() * ((sx + time * 0.012 * sp).rem_euclid(1.0));
            let py = text_area.min.y - 10.0 + (text_area.height() + 20.0) * ((sy + (time * 0.4 * sp + i as f32).sin() * 0.03).rem_euclid(1.0));
            if px > split {
                continue;
            }
            let tw = 0.5 + 0.5 * (time * 2.5 * sp + i as f32 * 1.7).sin();
            let col = if t.dark { Color32::WHITE } else { t.gradient[1] };
            lp.circle_filled(pos2(px, py), 0.8 + 0.7 * hash01(i * 13), with_alpha(col, 0.25 + 0.6 * tw));
        }
        // glowing divider
        if x > 0.002 && x < 0.998 {
            let top = text_area.min.y - 18.0;
            let bot = text_area.max.y + 10.0;
            let g = [t.gradient[0], t.gradient[1], t.gradient[2]];
            paint::glow_ellipse(&painter, pos2(split, (top + bot) / 2.0), vec2(14.0, (bot - top) * 0.55), with_alpha(t.gradient[1], 0.35));
            paint::gradient_polyline(&painter, &[pos2(split, top), pos2(split, bot)], false, 2.0, |u| {
                let a = (1.0 - (u * 2.0 - 1.0).abs()).powf(0.6);
                with_alpha(glowkit_core::color::gradient(&g, u), a)
            });
        }
        response
    }
}
