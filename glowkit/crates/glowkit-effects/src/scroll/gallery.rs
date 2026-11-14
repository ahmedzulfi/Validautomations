use super::common::{cover_uv, image_rect, paint_clipped, PaintedScene};
use egui::emath::TSTransform;
use egui::epaint::{Mesh, Vertex};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Key, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, TextureId, Ui, UiBuilder,
};
use glowkit_core::anim::{ease, smoothstep, Clock, Easing, Trigger};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};

fn rounded(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// `cover_uv` zoomed by `zoom` about its center, panned by `pan` (fractions of the free space).
fn cover_zoom_uv(tex_aspect: f32, target_aspect: f32, zoom: f32, pan: egui::Vec2) -> Rect {
    let base = cover_uv(tex_aspect, target_aspect);
    let size = base.size() / zoom.max(1.0);
    let free = base.size() - size;
    let c = base.center() + vec2(free.x * 0.5 * pan.x.clamp(-1.0, 1.0), free.y * 0.5 * pan.y.clamp(-1.0, 1.0));
    Rect::from_center_size(c, size)
}

fn shadow_color(t: &Theme) -> Color32 {
    if t.dark {
        Color32::from_black_alpha(160)
    } else {
        Color32::from_rgba_unmultiplied(30, 30, 70, 46)
    }
}

// ---------------------------------------------------------------------------------------------

/// One card of an [`AppleCardsCarousel`].
#[derive(Clone, Debug, PartialEq)]
pub struct AppleCard {
    /// Small label above the title.
    pub category: String,
    /// Card headline.
    pub title: String,
    /// Longer text shown in the expanded dialog.
    pub body: String,
    /// Cover image; `None` uses a [`PaintedScene`].
    pub texture: Option<TextureId>,
}

impl AppleCard {
    /// A card with a category and a title.
    pub fn new(category: impl Into<String>, title: impl Into<String>) -> Self {
        Self { category: category.into(), title: title.into(), body: String::new(), texture: None }
    }
    /// Detail text for the expanded view.
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = s.into();
        self
    }
    /// Cover image.
    pub fn texture(mut self, t: TextureId) -> Self {
        self.texture = Some(t);
        self
    }
}

#[derive(Clone, Copy, Default)]
struct AppleState {
    target: f32,
    open: Option<usize>,
    last: usize,
    idle: f32,
    tick: f32,
    phase: u32,
}

/// A horizontal row of tall, full-bleed image cards (category and headline over a soft scrim)
/// that scrolls with arrows, wheel or drag. Clicking a card morphs it into a large detail dialog;
/// click outside or the close button to shrink it back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{AppleCard, AppleCardsCarousel};
/// let cards = [AppleCard::new("Productivity", "Enhance your productivity.").body("…")];
/// ui.add(AppleCardsCarousel::new(&cards).autoplay(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AppleCardsCarousel<'a> {
    cards: &'a [AppleCard],
    card: egui::Vec2,
    gap: f32,
    autoplay: bool,
    salt: u64,
}

impl<'a> AppleCardsCarousel<'a> {
    /// A carousel over `cards`.
    pub fn new(cards: &'a [AppleCard]) -> Self {
        Self { cards, card: vec2(210.0, 320.0), gap: 16.0, autoplay: false, salt: 0 }
    }
    /// Card size.
    pub fn card_size(mut self, s: egui::Vec2) -> Self {
        self.card = s.max(vec2(60.0, 80.0));
        self
    }
    /// Gap between cards.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Scroll and open cards on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl egui::Widget for AppleCardsCarousel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.cards.len();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.card.y + 64.0), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_apple_cards", self.salt)), Sense::click_and_drag());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let dt = glowkit_core::dt(ui);
        let step = self.card.x + self.gap;
        let row_w = n as f32 * step - self.gap;
        let max = (row_w - (rect.width() - 8.0)).max(0.0);
        let row = Rect::from_min_size(rect.min + vec2(4.0, 6.0), vec2(rect.width() - 8.0, self.card.y));
        let arrow_y = row.max.y + 30.0;
        let left_btn = Rect::from_center_size(pos2(rect.max.x - 70.0, arrow_y), vec2(36.0, 36.0));
        let right_btn = Rect::from_center_size(pos2(rect.max.x - 26.0, arrow_y), vec2(36.0, 36.0));
        let modal = Rect::from_center_size(rect.center(), vec2((rect.width() - 24.0).min(720.0), rect.height() - 12.0));
        let close_btn = Rect::from_center_size(pos2(modal.max.x - 26.0, modal.min.y + 26.0), vec2(30.0, 30.0));
        let mut st: AppleState = glowkit_core::state(ui, id.with("st"), AppleState::default);
        let offset_now = glowkit_core::state(ui, id.with("off"), || 0.0f32);
        let hovered = response.hovered();
        let wheel = if hovered && st.open.is_none() { ui.input(|i| i.smooth_scroll_delta()) } else { egui::Vec2::ZERO };
        st.target -= wheel.x + wheel.y;
        if response.dragged() && st.open.is_none() {
            st.target -= response.drag_delta().x;
        }
        let card_rect = |i: usize, off: f32| {
            Rect::from_min_size(pos2(row.min.x + i as f32 * step - off, row.min.y), self.card)
        };
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                if st.open.is_some() {
                    if !modal.contains(p) || close_btn.contains(p) {
                        st.open = None;
                    }
                } else if left_btn.contains(p) {
                    st.target -= step * 2.0;
                } else if right_btn.contains(p) {
                    st.target += step * 2.0;
                } else if let Some(i) = (0..n).find(|i| card_rect(*i, offset_now).contains(p) && row.contains(p)) {
                    st.open = Some(i);
                    st.last = i;
                }
            }
        }
        if hovered || response.dragged() {
            st.idle = 0.0;
        } else {
            st.idle += dt;
        }
        if self.autoplay && st.idle > 1.5 {
            st.tick += dt;
            if st.tick > 2.2 {
                st.tick = 0.0;
                st.phase += 1;
                if st.open.is_some() {
                    st.open = None;
                } else if st.phase % 3 == 0 {
                    let first = ((st.target / step).round() as usize).min(n - 1);
                    let i = (first + 1).min(n - 1);
                    st.open = Some(i);
                    st.last = i;
                } else {
                    st.target = if st.target >= max - 1.0 { 0.0 } else { st.target + step };
                }
            }
            glowkit_core::animate(ui);
        }
        st.target = st.target.clamp(0.0, max);
        glowkit_core::set_state(ui, id.with("st"), st);
        let off = glowkit_core::anim::spring(ui, id.with("offs"), st.target, 120.0, 22.0);
        glowkit_core::set_state(ui, id.with("off"), off);
        let p = glowkit_core::anim::spring(ui, id.with("open"), if st.open.is_some() { 1.0 } else { 0.0 }, 170.0, 22.0);
        let pointer = response.hover_pos();
        if pointer.is_some_and(|q| row.contains(q) || left_btn.contains(q) || right_btn.contains(q)) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let appear = Clock::elapsed(ui, id.with("appear"));
        if appear < 1.6 {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter_at(rect);
        let builtin: Vec<TextureId> = (0..PaintedScene::COUNT).map(|i| PaintedScene::texture(ui.ctx(), i + 3)).collect();
        let radius = 22.0;
        let row_clip = ui.painter_at(Rect::from_min_max(pos2(rect.min.x, rect.min.y), pos2(rect.max.x, row.max.y + 20.0)));
        for (i, c) in self.cards.iter().enumerate() {
            let r = card_rect(i, off);
            if r.max.x < rect.min.x - 10.0 || r.min.x > rect.max.x + 10.0 {
                continue;
            }
            if p > 0.001 && i == st.last {
                row_clip.rect_filled(r, rounded(radius), with_alpha(t.muted, 0.6));
                continue;
            }
            let a = ease(Easing::OutCubic, ((appear - 0.08 * i as f32) / 0.6).clamp(0.0, 1.0));
            let hov = pointer.is_some_and(|q| r.contains(q) && row.contains(q)) && st.open.is_none();
            let h = glowkit_core::anim::follow(ui, id.with(("h", i)), if hov { 1.0 } else { 0.0 }, 12.0);
            let r = Rect::from_center_size(r.center() + vec2(0.0, 30.0 * (1.0 - a)), r.size() * (1.0 + 0.02 * h));
            draw_apple_card(&row_clip, &t, c, c.texture.unwrap_or(builtin[i % builtin.len()]), c.texture.is_none(), r, radius, a, 0.0);
        }
        // arrows
        for (b, glyph, enabled) in [(left_btn, "‹", off > 1.0), (right_btn, "›", off < max - 1.0)] {
            let hv = pointer.is_some_and(|q| b.contains(q));
            let fill = if enabled { if hv { t.foreground } else { mix(t.muted, t.foreground, 0.12) } } else { with_alpha(t.muted, 0.6) };
            painter.circle_filled(b.center(), 18.0, fill);
            let fg = if hv && enabled { t.background } else if enabled { t.foreground } else { t.muted_foreground };
            painter.text(b.center() - vec2(0.0, 2.0), Align2::CENTER_CENTER, glyph, FontId::proportional(26.0), fg);
        }
        // expanded dialog
        if p > 0.001 {
            let i = st.last;
            let c = &self.cards[i];
            let back = if t.dark { Color32::BLACK } else { Color32::from_gray(245) };
            painter.rect_filled(rect, rounded(t.radius), with_alpha(back, 0.7 * p.clamp(0.0, 1.0)));
            let from = card_rect(i, off);
            let r = Rect::from_min_max(from.min.lerp(modal.min, p), from.max.lerp(modal.max, p));
            paint::soft_shadow(&painter, r, radius, 40.0, vec2(0.0, 18.0), shadow_color(&t));
            let face = if t.dark { mix(t.card, t.popover, 0.6) } else { Color32::WHITE };
            painter.rect(r, rounded(radius), with_alpha(face, p.clamp(0.0, 1.0)), Stroke::new(1.0, with_alpha(t.border, p)), StrokeKind::Inside);
            // the cover shrinks from the whole card into the dialog's left panel
            let img_target = Rect::from_min_size(modal.min + vec2(14.0, 14.0), vec2(modal.width() * 0.42, modal.height() - 28.0));
            let img = Rect::from_min_max(from.min.lerp(img_target.min, p), from.max.lerp(img_target.max, p));
            let q = p.clamp(0.0, 1.0);
            draw_apple_card(&painter, &t, c, c.texture.unwrap_or(builtin[i % builtin.len()]), c.texture.is_none(), img, radius - 6.0 * q, 1.0, q);
            let ta = smoothstep(0.55, 1.0, p);
            if ta > 0.0 {
                let x = img_target.max.x + 22.0;
                let wrap = modal.max.x - x - 30.0;
                let y0 = modal.min.y + 30.0 + 10.0 * (1.0 - ta);
                painter.text(pos2(x, y0), Align2::LEFT_TOP, &c.category, FontId::proportional(14.0), with_alpha(t.muted_foreground, ta));
                let g = painter.layout(c.title.clone(), FontId::proportional(26.0), with_alpha(t.foreground, ta), wrap - 20.0);
                let gh = g.size().y;
                painter.galley(pos2(x, y0 + 24.0), g, t.foreground);
                let b = painter.layout(c.body.clone(), FontId::proportional(14.5), with_alpha(t.muted_foreground, ta), wrap);
                painter.with_clip_rect(modal.shrink(10.0)).galley(pos2(x, y0 + 40.0 + gh), b, t.muted_foreground);
                let hv = pointer.is_some_and(|q| close_btn.contains(q));
                painter.circle_filled(close_btn.center(), 15.0, with_alpha(if hv { t.foreground } else { mix(t.muted, t.foreground, 0.15) }, ta));
                painter.text(
                    close_btn.center(),
                    Align2::CENTER_CENTER,
                    "✕",
                    FontId::proportional(13.0),
                    with_alpha(if hv { t.background } else { t.foreground }, ta),
                );
            }
        }
        response
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_apple_card(painter: &Painter, t: &Theme, c: &AppleCard, tex: TextureId, builtin: bool, r: Rect, radius: f32, alpha: f32, text_fade: f32) {
    if alpha <= 0.0 {
        return;
    }
    paint::soft_shadow(painter, r, radius, 22.0, vec2(0.0, 10.0), shadow_color(t).gamma_multiply(alpha));
    let uv = if builtin { cover_uv(PaintedScene::ASPECT, r.width() / r.height().max(1.0)) } else { Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)) };
    image_rect(painter, r, radius, tex, uv, Color32::WHITE.gamma_multiply(alpha));
    // scrim from the top so the white text reads on any image
    let top = Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.55));
    let pts = paint::rounded_rect_points(top, radius.min(top.height() / 2.0), 5);
    paint::fill_convex_fn(painter, &pts, 4, |q| {
        let k = ((q.y - top.min.y) / top.height()).clamp(0.0, 1.0);
        with_alpha(Color32::BLACK, 0.55 * (1.0 - k).powf(1.6) * alpha)
    });
    let a = alpha * (1.0 - text_fade);
    if a > 0.01 {
        painter.text(
            r.min + vec2(18.0, 18.0),
            Align2::LEFT_TOP,
            &c.category,
            FontId::proportional(13.0),
            with_alpha(Color32::WHITE, 0.85 * a),
        );
        let g = painter.layout(c.title.clone(), FontId::proportional(21.0), with_alpha(Color32::WHITE, a), r.width() - 36.0);
        painter.galley(r.min + vec2(18.0, 40.0), g, Color32::WHITE);
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct SliderState {
    index: usize,
    prev: usize,
    changed: f64,
    dir: f32,
}

/// A full-bleed image slideshow: slides cross-fade with a zoom-and-rise entrance and slowly
/// push in while shown (Ken Burns), under a dimming scrim with headline text and a call to
/// action. Click either half, use the arrow keys while hovered, or let it autoplay.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::ImagesSlider::new().interval(4.0).title("The hero section slideshow\nnobody asked for"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ImagesSlider<'a> {
    textures: Option<&'a [TextureId]>,
    height: f32,
    interval: f32,
    title: String,
    action: String,
    overlay: f32,
    radius: Option<f32>,
    salt: u64,
}

impl Default for ImagesSlider<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ImagesSlider<'a> {
    /// A slider over the built-in scenes.
    pub fn new() -> Self {
        Self {
            textures: None,
            height: 380.0,
            interval: 4.0,
            title: String::new(),
            action: String::new(),
            overlay: 0.55,
            radius: None,
            salt: 0,
        }
    }
    /// Slide images (cropped to fill).
    pub fn textures(mut self, t: &'a [TextureId]) -> Self {
        self.textures = Some(t);
        self
    }
    /// Height of the slider.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(80.0);
        self
    }
    /// Seconds per slide when autoplaying (0 = manual only).
    pub fn interval(mut self, s: f32) -> Self {
        self.interval = s.max(0.0);
        self
    }
    /// Headline over the slides (use `\n` for line breaks).
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Label of the call-to-action pill (empty = none).
    pub fn action(mut self, s: impl Into<String>) -> Self {
        self.action = s.into();
        self
    }
    /// Darkness of the overlay scrim (0..1).
    pub fn overlay(mut self, a: f32) -> Self {
        self.overlay = a.clamp(0.0, 1.0);
        self
    }
    /// Corner radius (defaults to the theme radius x 2).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(0.0));
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl egui::Widget for ImagesSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_images_slider", self.salt)), Sense::click());
        let id = response.id;
        let builtin: Vec<TextureId>;
        let (texs, aspect) = match self.textures {
            Some(x) if !x.is_empty() => (x, 1.0),
            _ => {
                builtin = (0..PaintedScene::COUNT).map(|i| PaintedScene::texture(ui.ctx(), i)).collect();
                (&builtin[..], PaintedScene::ASPECT)
            }
        };
        let n = texs.len();
        let now = ui.input(|i| i.time);
        let mut st: SliderState = glowkit_core::state(ui, id.with("st"), || SliderState { index: 0, prev: 0, changed: now - 5.0, dir: 1.0 });
        let go = |st: &mut SliderState, d: i32| {
            st.prev = st.index;
            st.index = (st.index as i32 + d).rem_euclid(n as i32) as usize;
            st.changed = now;
            st.dir = d.signum() as f32;
        };
        if response.hovered() {
            let (l, r) = ui.input(|i| (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight)));
            if l {
                go(&mut st, -1);
            }
            if r {
                go(&mut st, 1);
            }
        }
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                go(&mut st, if p.x < rect.center().x { -1 } else { 1 });
            }
        }
        if self.interval > 0.0 && !response.hovered() && (now - st.changed) as f32 > self.interval {
            go(&mut st, 1);
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(30.0));
        let radius = self.radius.unwrap_or((t.radius * 2.0).min(24.0));
        let since = (now - st.changed) as f32;
        let tr = ease(Easing::OutCubic, (since / 0.9).clamp(0.0, 1.0));
        let target_aspect = rect.width() / rect.height();
        paint::soft_shadow(&painter, rect, radius, 26.0, vec2(0.0, 10.0), shadow_color(&t));
        let draw = |idx: usize, age: f32, alpha: f32, extra_zoom: f32| {
            let zoom = 1.0 + 0.08 * (age / (self.interval.max(2.0) + 1.0)).clamp(0.0, 1.0) + extra_zoom;
            let pan = vec2((idx as f32 * 1.7).sin() * 0.6, (idx as f32 * 2.3).cos() * 0.4);
            let uv = cover_zoom_uv(aspect, target_aspect, zoom, pan);
            image_rect(&painter, rect, radius, texs[idx], uv, Color32::WHITE.gamma_multiply(alpha));
        };
        if tr < 1.0 {
            draw(st.prev, since + self.interval, 1.0, 0.0);
        }
        draw(st.index, since, tr, 0.22 * (1.0 - tr));
        // scrim + vignette
        let pts = paint::rounded_rect_points(rect, radius, 6);
        let c = rect.center();
        let ov = self.overlay;
        paint::fill_convex_fn(&painter, &pts, 6, |q| {
            let d = ((q - c) / (rect.size() / 2.0)).length().min(1.4);
            with_alpha(Color32::BLACK, ov * (0.55 + 0.45 * d * d * 0.6))
        });
        // text
        if !self.title.is_empty() {
            let ta = 1.0;
            let g = painter.layout(
                self.title.clone(),
                FontId::proportional((rect.width() / 18.0).clamp(22.0, 46.0)),
                Color32::WHITE,
                rect.width() - 60.0,
            );
            let has_btn = !self.action.is_empty();
            let total_h = g.size().y + if has_btn { 64.0 } else { 0.0 };
            let pos = pos2(rect.center().x - g.size().x / 2.0, rect.center().y - total_h / 2.0);
            let stops = [Color32::from_gray(255), Color32::from_gray(200)];
            let gr = Rect::from_min_size(pos, g.size());
            paint::galley_with(&painter, pos, &g, |q| with_alpha(paint::linear_fn(gr, 90.0, &stops)(q), ta));
            if has_btn {
                let bw = 30.0 + painter.layout_no_wrap(self.action.clone(), FontId::proportional(15.0), Color32::WHITE).size().x;
                let b = Rect::from_center_size(pos2(rect.center().x, gr.max.y + 36.0), vec2(bw + 16.0, 40.0));
                painter.rect(
                    b,
                    CornerRadius::same(99),
                    with_alpha(PaintedScene::accent(st.index).gamma_multiply(0.35), 0.9),
                    Stroke::new(1.0, with_alpha(Color32::WHITE, 0.45)),
                    StrokeKind::Inside,
                );
                painter.text(b.center(), Align2::CENTER_CENTER, format!("{} →", self.action), FontId::proportional(15.0), Color32::WHITE);
                // glow line under the pill
                let gl = with_alpha(PaintedScene::accent(st.index), 0.9);
                paint::gradient_polyline(&painter, &[pos2(b.min.x + 16.0, b.max.y), pos2(b.max.x - 16.0, b.max.y)], false, 1.5, |u| {
                    with_alpha(gl, (1.0 - (u * 2.0 - 1.0).abs()) * 0.9)
                });
            }
        }
        // dots (laid out with their animated widths so the active pill never overlaps)
        let widths: Vec<f32> = (0..n)
            .map(|k| glowkit_core::anim::follow(ui, id.with(("dot", k)), if k == st.index { 22.0 } else { 7.0 }, 10.0))
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + 7.0 * (n as f32 - 1.0);
        let mut x = rect.center().x - total / 2.0;
        for (k, w) in widths.iter().enumerate() {
            let d = Rect::from_min_size(pos2(x, rect.max.y - 24.0), vec2(*w, 7.0));
            painter.rect_filled(d, CornerRadius::same(99), with_alpha(Color32::WHITE, if k == st.index { 0.95 } else { 0.45 }));
            x += w + 7.0;
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// An image that resolves from a few big pixels to full detail: tiles pop in with a random
/// stagger, then the mosaic sharpens step by step (each step doubling the resolution) until the
/// real texture cross-fades in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_core::Trigger;
/// ui.add(glowkit_effects::PixelImage::new().scene(2).grid(6, 4).trigger(Trigger::Loop { pause_ms: 1500 }));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PixelImage {
    texture: Option<TextureId>,
    tex_aspect: f32,
    scene: usize,
    size: egui::Vec2,
    grid: (usize, usize),
    duration: f32,
    trigger: Trigger,
    radius: f32,
    salt: u64,
}

impl Default for PixelImage {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelImage {
    /// A pixel reveal of the first built-in scene.
    pub fn new() -> Self {
        Self {
            texture: None,
            tex_aspect: 1.0,
            scene: 0,
            size: vec2(320.0, 220.0),
            grid: (6, 4),
            duration: 3.2,
            trigger: Trigger::OnAppear,
            radius: 16.0,
            salt: 0,
        }
    }
    /// Show your own texture (`aspect` = its width / height, used to crop it to fill).
    pub fn texture(mut self, t: TextureId, aspect: f32) -> Self {
        self.texture = Some(t);
        self.tex_aspect = aspect.max(0.01);
        self
    }
    /// Use built-in [`PaintedScene`] `index` (when no texture is set).
    pub fn scene(mut self, index: usize) -> Self {
        self.scene = index;
        self
    }
    /// Widget size.
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s.max(vec2(16.0, 16.0));
        self
    }
    /// Starting mosaic (columns, rows); it doubles until it is finer than ~6 points per tile.
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.grid = (cols.clamp(1, 64), rows.clamp(1, 64));
        self
    }
    /// Total animation time in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.2);
        self
    }
    /// When the animation (re)plays.
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(0.0);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl egui::Widget for PixelImage {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.min(vec2(ui.available_width(), f32::INFINITY));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id.with(self.salt);
        let e = Clock::triggered(ui, id, self.trigger, self.duration, response.hovered(), false);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let (tex, aspect) = match self.texture {
            Some(x) => (x, self.tex_aspect),
            None => (PaintedScene::texture(ui.ctx(), self.scene), PaintedScene::ASPECT),
        };
        let uv = cover_uv(aspect, rect.width() / rect.height());
        let k = (e / self.duration).clamp(0.0, 1.0);
        let painter = ui.painter_at(rect.expand(30.0));
        paint::soft_shadow(&painter, rect, self.radius, 22.0, vec2(0.0, 8.0), shadow_color(&t).gamma_multiply(smoothstep(0.0, 0.3, k)));
        // levels: base grid, x2, x4 ... until tiles are tiny
        let (c0, r0) = self.grid;
        let mut levels = vec![(c0, r0)];
        while rect.width() / levels.last().map_or(1, |l| l.0) as f32 > 6.0 && levels.len() < 6 {
            let (c, r) = *levels.last().unwrap_or(&(c0, r0));
            levels.push((c * 2, r * 2));
        }
        let pop_end = 0.3;
        let full_start = 0.88;
        let outline = paint::rounded_rect_points(rect, self.radius, 6);
        let safe = rect.shrink(self.radius);
        let mut ui2 = ui.new_child(UiBuilder::new().max_rect(rect));
        ui2.set_clip_rect(ui.clip_rect());
        if k < full_start + 0.12 {
            let lk = if k < pop_end { 0.0 } else { ((k - pop_end) / (full_start - pop_end)).clamp(0.0, 0.999) * levels.len() as f32 };
            let li = (lk.floor() as usize).min(levels.len() - 1);
            let (cols, rows) = levels[li];
            let next = levels.get(li + 1).copied();
            let blend = smoothstep(0.55, 1.0, lk.fract());
            let pop = k < pop_end;
            let tile_alpha = 1.0 - smoothstep(full_start, full_start + 0.12, k);
            let gap_px = if pop { 2.0 } else { 0.0 };
            paint_clipped(&mut ui2, &outline, safe, TSTransform::IDENTITY, 1.0, |ui| {
                let mut mesh = Mesh::with_texture(tex);
                let mut add_grid = |cols: usize, rows: usize, alpha: f32, popping: bool| {
                    let cw = rect.width() / cols as f32;
                    let ch = rect.height() / rows as f32;
                    for y in 0..rows {
                        for x in 0..cols {
                            let mut a = alpha;
                            let mut s = 1.0;
                            if popping {
                                let delay = hash01((y * 131 + x * 7) as u64 + 17) * 0.6;
                                let local = ((k / pop_end - delay) / 0.4).clamp(0.0, 1.0);
                                a *= ease(Easing::OutCubic, local);
                                s = 0.6 + 0.4 * ease(Easing::OutBack, local);
                            }
                            if a <= 0.003 {
                                continue;
                            }
                            let cell = Rect::from_min_size(rect.min + vec2(x as f32 * cw, y as f32 * ch), vec2(cw, ch));
                            let cell = Rect::from_center_size(cell.center(), (cell.size() - vec2(gap_px, gap_px)) * s);
                            let u = pos2(
                                uv.min.x + (x as f32 + 0.5) / cols as f32 * uv.width(),
                                uv.min.y + (y as f32 + 0.5) / rows as f32 * uv.height(),
                            );
                            let col = Color32::WHITE.gamma_multiply(a);
                            let base = mesh.vertices.len() as u32;
                            for q in [cell.left_top(), cell.right_top(), cell.right_bottom(), cell.left_bottom()] {
                                mesh.vertices.push(Vertex { pos: q, uv: u, color: col });
                            }
                            mesh.add_triangle(base, base + 1, base + 2);
                            mesh.add_triangle(base, base + 2, base + 3);
                        }
                    }
                };
                add_grid(cols, rows, tile_alpha, pop);
                if let Some((nc, nr)) = next {
                    if blend > 0.0 && !pop {
                        add_grid(nc, nr, blend * tile_alpha, false);
                    }
                }
                ui.painter().add(Shape::mesh(mesh));
            });
        }
        let fa = smoothstep(full_start - 0.04, 1.0, k);
        if fa > 0.0 {
            image_rect(&painter, rect, self.radius, tex, uv, Color32::WHITE.gamma_multiply(fa));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// How a [`HeroVideoDialog`] enters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VideoDialogAnimation {
    /// Grows from the thumbnail into place (default).
    #[default]
    FromThumbnail,
    /// Scales up from the center.
    FromCenter,
    /// Slides up from below.
    FromBottom,
    /// Drops in from above.
    FromTop,
    /// Plain fade.
    Fade,
}

#[derive(Clone, Copy, Default)]
struct VideoState {
    open: bool,
    opened_at: f64,
    idle: f32,
    tick: f32,
}

/// A hero video thumbnail with a pulsing glass play button; clicking it opens an animated
/// dialog that "plays" the video (any painter closure driven by playback time) with a progress
/// bar and a close button.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::HeroVideoDialog::new().show(ui, |painter, rect, secs| {
///     painter.rect_filled(rect, 0.0, egui::Color32::from_gray((secs * 40.0) as u8 % 255));
/// });
/// ```
#[must_use = "call `.show(ui, |painter, rect, seconds| ...)`"]
pub struct HeroVideoDialog {
    size: egui::Vec2,
    animation: VideoDialogAnimation,
    length: f32,
    autoplay: bool,
    salt: u64,
}

impl Default for HeroVideoDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl HeroVideoDialog {
    /// A 16:9 thumbnail.
    pub fn new() -> Self {
        Self { size: vec2(440.0, 248.0), animation: VideoDialogAnimation::FromThumbnail, length: 92.0, autoplay: false, salt: 0 }
    }
    /// Thumbnail size.
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = s.max(vec2(80.0, 45.0));
        self
    }
    /// Dialog entrance animation.
    pub fn animation(mut self, a: VideoDialogAnimation) -> Self {
        self.animation = a;
        self
    }
    /// Video length in seconds (for the progress bar; playback loops).
    pub fn length(mut self, s: f32) -> Self {
        self.length = s.max(1.0);
        self
    }
    /// Open and close the dialog on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the thumbnail (and the dialog when open). `video(painter, rect, seconds)` paints one
    /// frame of the video; it is called with `seconds = 0` for the thumbnail. The dialog opens
    /// over the area the widget is placed in: give it room (the response covers that area).
    pub fn show(self, ui: &mut Ui, video: impl Fn(&Painter, Rect, f32)) -> Response {
        let t = Theme::of(ui);
        let area_h = (self.size.y + 110.0).max(ui.available_height().min(self.size.y + 160.0));
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), area_h), Sense::hover());
        let response = ui.interact(area, ui.scope_id().with(("glowkit_hero_video", self.salt)), Sense::click());
        let id = response.id;
        let thumb = Rect::from_center_size(area.center(), self.size.min(area.size() - vec2(8.0, 8.0)));
        let dlg_w = (area.width() - 40.0).min((area.height() - 30.0) * 16.0 / 9.0);
        let dialog = Rect::from_center_size(area.center(), vec2(dlg_w, dlg_w * 9.0 / 16.0));
        let close_btn = Rect::from_center_size(pos2(dialog.max.x - 4.0, dialog.min.y - 4.0), vec2(30.0, 30.0));
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let mut st: VideoState = glowkit_core::state(ui, id.with("st"), VideoState::default);
        let pointer = response.hover_pos();
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                if st.open {
                    if close_btn.contains(p) || !dialog.contains(p) {
                        st.open = false;
                    }
                } else if thumb.contains(p) {
                    st.open = true;
                    st.opened_at = now;
                }
            }
        }
        if st.open && ui.input(|i| i.key_pressed(Key::Escape)) {
            st.open = false;
        }
        if response.hovered() {
            st.idle = 0.0;
        } else {
            st.idle += dt;
        }
        if self.autoplay && st.idle > 1.5 {
            st.tick += dt;
            let dur = if st.open { 4.5 } else { 3.0 };
            if st.tick > dur {
                st.tick = 0.0;
                st.open = !st.open;
                if st.open {
                    st.opened_at = now;
                }
            }
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        glowkit_core::animate(ui);
        let p = glowkit_core::anim::spring(ui, id.with("p"), if st.open { 1.0 } else { 0.0 }, 190.0, 23.0);
        if pointer.is_some_and(|q| (!st.open && thumb.contains(q)) || (st.open && close_btn.contains(q))) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(area) {
            return response;
        }
        let painter = ui.painter_at(area.expand(20.0));
        let time = glowkit_core::time(ui);
        // thumbnail
        let hov = !st.open && pointer.is_some_and(|q| thumb.contains(q));
        let h = glowkit_core::anim::follow(ui, id.with("hov"), if hov { 1.0 } else { 0.0 }, 12.0);
        let radius = 18.0;
        paint::soft_shadow(&painter, thumb, radius, 30.0, vec2(0.0, 12.0), shadow_color(&t));
        let outline = paint::rounded_rect_points(thumb, radius, 6);
        let mut ui2 = ui.new_child(UiBuilder::new().max_rect(thumb));
        ui2.set_clip_rect(area.expand(20.0).intersect(ui.clip_rect()));
        paint_clipped(&mut ui2, &outline, thumb.shrink(radius), TSTransform::IDENTITY, 1.0, |ui| {
            video(ui.painter(), thumb, 0.0);
            ui.painter().rect_filled(thumb, 0.0, with_alpha(Color32::BLACK, 0.18 + 0.12 * h));
        });
        painter.rect_stroke(thumb, rounded(radius), Stroke::new(1.0, with_alpha(t.border, 0.8)), StrokeKind::Inside);
        // play button: pulsing glass halo + white disc
        let c = thumb.center();
        let pulse = (time * 2.2).sin() * 0.5 + 0.5;
        let s = 1.0 + 0.08 * h;
        let accent = t.gradient[0];
        paint::glow(&painter, c, 70.0 * s, with_alpha(accent, 0.35 + 0.15 * pulse));
        painter.circle_filled(c, (40.0 + 4.0 * pulse) * s, with_alpha(Color32::WHITE, 0.18));
        painter.circle_stroke(c, (40.0 + 4.0 * pulse) * s, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.35)));
        paint::gradient_circle(&painter, c, 27.0 * s, &[Color32::WHITE, Color32::from_gray(235)]);
        play_triangle(&painter, c + vec2(2.0, 0.0), 10.0 * s, mix(accent, Color32::BLACK, 0.15));
        // dialog
        if p > 0.002 {
            let q = p.clamp(0.0, 1.0);
            painter.rect_filled(area.expand(20.0), rounded(t.radius), with_alpha(Color32::BLACK, 0.6 * q));
            let (r, alpha) = match self.animation {
                VideoDialogAnimation::FromThumbnail => (Rect::from_min_max(thumb.min.lerp(dialog.min, p), thumb.max.lerp(dialog.max, p)), q.sqrt()),
                VideoDialogAnimation::FromCenter => (Rect::from_center_size(dialog.center(), dialog.size() * (0.4 + 0.6 * p)), q),
                VideoDialogAnimation::FromBottom => (dialog.translate(vec2(0.0, (1.0 - p) * area.height() * 0.7)), q),
                VideoDialogAnimation::FromTop => (dialog.translate(vec2(0.0, -(1.0 - p) * area.height() * 0.7)), q),
                VideoDialogAnimation::Fade => (dialog, q),
            };
            paint::soft_shadow(&painter, r, 16.0, 40.0, vec2(0.0, 18.0), Color32::from_black_alpha((180.0 * alpha) as u8));
            let secs = ((now - st.opened_at) as f32).max(0.0);
            let outline = paint::rounded_rect_points(r, 16.0, 6);
            let mut ui3 = ui.new_child(UiBuilder::new().max_rect(r));
            ui3.set_clip_rect(area.expand(20.0).intersect(ui.clip_rect()));
            paint_clipped(&mut ui3, &outline, r.shrink(16.0), TSTransform::IDENTITY, alpha, |ui| {
                video(ui.painter(), r, secs);
                // controls bar
                let bar = Rect::from_min_max(pos2(r.min.x, r.max.y - 44.0), r.max);
                let pts = [bar.left_top(), bar.right_top(), bar.right_bottom(), bar.left_bottom()];
                paint::quad(ui.painter(), pts, [Color32::TRANSPARENT, Color32::TRANSPARENT, with_alpha(Color32::BLACK, 0.6), with_alpha(Color32::BLACK, 0.6)]);
                let frac = (secs % self.length) / self.length;
                let track = Rect::from_min_max(pos2(r.min.x + 46.0, r.max.y - 20.0), pos2(r.max.x - 90.0, r.max.y - 16.0));
                ui.painter().rect_filled(track, CornerRadius::same(99), with_alpha(Color32::WHITE, 0.3));
                let fill = Rect::from_min_max(track.min, pos2(track.min.x + track.width() * frac, track.max.y));
                ui.painter().rect_filled(fill, CornerRadius::same(99), Color32::WHITE);
                ui.painter().circle_filled(pos2(fill.max.x, fill.center().y), 5.5, Color32::WHITE);
                play_pause(ui.painter(), pos2(r.min.x + 24.0, track.center().y));
                let fmt = |s: f32| format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32);
                ui.painter().text(
                    pos2(r.max.x - 16.0, track.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{} / {}", fmt(secs % self.length), fmt(self.length)),
                    FontId::monospace(11.5),
                    with_alpha(Color32::WHITE, 0.9),
                );
            });
            let ca = smoothstep(0.6, 1.0, p);
            if ca > 0.0 {
                let hv = pointer.is_some_and(|q| close_btn.contains(q));
                painter.circle(
                    close_btn.center(),
                    15.0,
                    with_alpha(if hv { Color32::WHITE } else { Color32::from_gray(30) }, ca),
                    Stroke::new(1.0, with_alpha(Color32::from_gray(120), ca)),
                );
                painter.text(
                    close_btn.center(),
                    Align2::CENTER_CENTER,
                    "✕",
                    FontId::proportional(13.0),
                    with_alpha(if hv { Color32::BLACK } else { Color32::WHITE }, ca),
                );
            }
        }
        response
    }
}

fn play_triangle(painter: &Painter, c: Pos2, r: f32, color: Color32) {
    let pts = vec![c + vec2(-r * 0.7, -r), c + vec2(r, 0.0), c + vec2(-r * 0.7, r)];
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

fn play_pause(painter: &Painter, c: Pos2) {
    for dx in [-3.5, 3.5] {
        painter.rect_filled(Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.5, 13.0)), CornerRadius::same(1), Color32::WHITE);
    }
}

