use super::common::{
    cover_uv, drive_loop, drive_scroll, in_convex, paint_warped, scroll_rail, view_progress, warp_card, PaintedScene, Persp,
};
use egui::emath::TSTransform;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind,
    TextureId, Ui, UiBuilder,
};
use glowkit_core::anim::{ease, smoothstep, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

// ---------------------------------------------------------------------------------------------

/// A scrolling column of cards that pin near the top and pile up: each new card slides over the
/// previous ones, which shrink back into a neat stack as you keep scrolling.
///
/// The widget owns its viewport: scroll with the wheel or by dragging; with
/// [`Self::autoplay`] it glides through the stack on its own while the pointer is away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::ScrollStack::new(5).show(ui, |ui, i| {
///     ui.heading(format!("Card {}", i + 1));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct ScrollStack {
    count: usize,
    height: f32,
    card_height: f32,
    card_width: f32,
    gap: f32,
    stack_top: f32,
    stack_distance: f32,
    base_scale: f32,
    item_scale: f32,
    dim: f32,
    speed: f32,
    colors: Option<Vec<[Color32; 2]>>,
    salt: u64,
}

impl ScrollStack {
    /// A stack of `count` cards.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            height: 400.0,
            card_height: 230.0,
            card_width: 620.0,
            gap: 110.0,
            stack_top: 0.1,
            stack_distance: 16.0,
            base_scale: 0.86,
            item_scale: 0.03,
            dim: 0.4,
            speed: 0.0,
            colors: None,
            salt: 0,
        }
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Size of every card.
    pub fn card_size(mut self, size: egui::Vec2) -> Self {
        self.card_width = size.x.max(60.0);
        self.card_height = size.y.max(40.0);
        self
    }
    /// Scroll distance between consecutive cards before they pin.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Where the stack pins, as a fraction of the viewport height from the top.
    pub fn stack_position(mut self, f: f32) -> Self {
        self.stack_top = f.clamp(0.0, 0.8);
        self
    }
    /// Vertical offset between pinned cards.
    pub fn stack_distance(mut self, d: f32) -> Self {
        self.stack_distance = d.max(0.0);
        self
    }
    /// Scale the first card shrinks to; later cards shrink a bit less (see [`Self::item_scale`]).
    pub fn base_scale(mut self, s: f32) -> Self {
        self.base_scale = s.clamp(0.3, 1.0);
        self
    }
    /// Extra scale per card index on top of [`Self::base_scale`].
    pub fn item_scale(mut self, s: f32) -> Self {
        self.item_scale = s.clamp(0.0, 0.2);
        self
    }
    /// How much covered cards darken (0..1).
    pub fn dim(mut self, d: f32) -> Self {
        self.dim = d.clamp(0.0, 1.0);
        self
    }
    /// Glide through the stack on its own at `speed` points / second when idle (0 = off).
    pub fn autoplay(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }
    /// Card gradients (cycled). Defaults to plain theme cards.
    pub fn colors(mut self, colors: Vec<[Color32; 2]>) -> Self {
        self.colors = Some(colors);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the stack; `card(ui, index)` fills one card (its `ui` spans the card's inner area).
    pub fn show(self, ui: &mut Ui, mut card: impl FnMut(&mut Ui, usize)) -> Response {
        let t = Theme::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_scroll_stack", self.salt)), Sense::drag());
        let id = response.id;
        let n = self.count;
        if n == 0 || !ui.is_rect_visible(rect) {
            return response;
        }
        let cw = self.card_width.min(rect.width() - 24.0);
        let ch = self.card_height.min(self.height * 0.85);
        let pin0 = self.height * self.stack_top;
        let first = pin0 + 70.0;
        let y_of = |i: usize| first + i as f32 * (ch + self.gap);
        let pin_of = |i: usize| pin0 + i as f32 * self.stack_distance;
        let s_pin = |i: usize| y_of(i) - pin_of(i);
        let max = s_pin(n - 1) + 50.0;
        let s = drive_scroll(ui, id, &response, max, false, self.speed);
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        let clip = rect.intersect(ui.clip_rect());
        let radius = (t.radius * 2.2).round();
        let last_pin = s_pin(n - 1);
        for i in 0..n {
            let y = (y_of(i) - s).max(pin_of(i));
            let r = Rect::from_min_size(pos2(rect.center().x - cw / 2.0, rect.min.y + y), vec2(cw, ch));
            if r.min.y > rect.max.y + 10.0 {
                continue;
            }
            let k = if i + 1 < n { ((s - s_pin(i)) / (last_pin - s_pin(i)).max(1.0)).clamp(0.0, 1.0) } else { 0.0 };
            let target = (self.base_scale + i as f32 * self.item_scale).min(1.0);
            let scale = 1.0 - k * (1.0 - target);
            let start = super::common::shape_mark(ui);
            {
                let mut child = ui.new_child(UiBuilder::new().max_rect(r.shrink(26.0)).id_salt(("stack_card", i)));
                child.set_clip_rect(clip);
                let p = ui.painter().with_clip_rect(clip);
                let sc = if t.dark { Color32::from_black_alpha(150) } else { Color32::from_rgba_unmultiplied(30, 30, 70, 40) };
                paint::soft_shadow(&p, r, radius, 30.0, vec2(0.0, 12.0), sc);
                match &self.colors {
                    Some(cols) if !cols.is_empty() => {
                        let c = cols[i % cols.len()];
                        paint::gradient_rect(&p, r, radius, 35.0, &c);
                        p.rect_stroke(
                            r,
                            CornerRadius::same(radius as u8),
                            Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18)),
                            StrokeKind::Inside,
                        );
                    }
                    _ => {
                        p.rect(r, CornerRadius::same(radius as u8), t.popover, Stroke::new(1.0, t.border), StrokeKind::Inside);
                    }
                }
                card(&mut child, i);
                if k > 0.0 && self.dim > 0.0 {
                    let d = if t.dark { Color32::BLACK } else { Color32::from_rgb(20, 20, 40) };
                    p.rect_filled(r, CornerRadius::same(radius as u8), with_alpha(d, k * self.dim * if t.dark { 1.0 } else { 0.5 }));
                }
            }
            if scale < 0.9999 {
                let pivot = pos2(r.center().x, r.min.y);
                let tf = TSTransform::from_translation(pivot.to_vec2())
                    * TSTransform::from_scaling(scale)
                    * TSTransform::from_translation(-pivot.to_vec2());
                let layer = ui.layer_id();
                ui.ctx().graphics_mut(|g| {
                    let list = g.entry(layer);
                    let end = list.next_idx();
                    list.transform_range(start, end, tf);
                });
            }
        }
        let track = Rect::from_min_size(pos2(rect.max.x - 6.0, rect.min.y + 16.0), vec2(3.0, rect.height() - 32.0));
        scroll_rail(&ui.painter_at(rect), &t, track, self.height / (max + self.height), s / max.max(1.0));
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// Posters riding a curved, drum-like 3D surface: they rise from the bottom, swell as they pass
/// the middle, then shrink and tilt away over the top, in three staggered columns. Scroll or drag
/// to fly through them; they drift on their own when idle.
///
/// Pass your own textures with [`Self::textures`]; by default it shows [`PaintedScene`]s.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::FlyingPosters::new().columns(3).speed(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FlyingPosters<'a> {
    textures: Option<&'a [TextureId]>,
    height: f32,
    poster: egui::Vec2,
    columns: usize,
    gap: f32,
    curvature: f32,
    speed: f32,
    salt: u64,
}

impl Default for FlyingPosters<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FlyingPosters<'a> {
    /// Three columns of built-in posters.
    pub fn new() -> Self {
        Self {
            textures: None,
            height: 420.0,
            poster: vec2(150.0, 200.0),
            columns: 3,
            gap: 26.0,
            curvature: 1.0,
            speed: 45.0,
            salt: 0,
        }
    }
    /// Poster images (cropped to fill each poster).
    pub fn textures(mut self, t: &'a [TextureId]) -> Self {
        self.textures = Some(t);
        self
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Poster size at the closest point.
    pub fn poster_size(mut self, s: egui::Vec2) -> Self {
        self.poster = s.max(vec2(20.0, 20.0));
        self
    }
    /// Number of columns (1..=7).
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = n.clamp(1, 7);
        self
    }
    /// Space between posters.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// How strongly the surface bends (0 = flat, 1 = default drum).
    pub fn curvature(mut self, c: f32) -> Self {
        self.curvature = c.clamp(0.0, 2.0);
        self
    }
    /// Idle drift in points per second (negative flies downward).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl egui::Widget for FlyingPosters<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_flying_posters", self.salt)), Sense::drag());
        let off = drive_loop(ui, response.id, &response, false, self.speed);
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let builtin: Vec<TextureId> = (0..PaintedScene::COUNT).map(|i| PaintedScene::texture(ui.ctx(), i)).collect();
        let (texs, tex_aspect) = match self.textures {
            Some(t) if !t.is_empty() => (t, 1.0),
            _ => (&builtin[..], PaintedScene::ASPECT),
        };
        let painter = ui.painter_at(rect);
        let center = rect.center();
        let step = self.poster.y + self.gap;
        let ry = self.height * 0.62 / self.curvature.max(0.05);
        let rx = rect.width() * 1.1 / self.curvature.max(0.05);
        let focal = 700.0;
        let rows = ((std::f32::consts::PI * ry) / step).ceil() as i32 + 2;
        let period = rows as f32 * step;
        let uv = cover_uv(tex_aspect, self.poster.x / self.poster.y);
        let project = |w: egui::Vec2| {
            let (ty, tx) = (w.y / ry, w.x / rx);
            let x3 = rx * tx.sin();
            let y3 = ry * ty.clamp(-1.57, 1.57).sin();
            let z = rx * (1.0 - tx.cos()) + ry * (1.0 - ty.clamp(-1.57, 1.57).cos());
            let s = focal / (focal + z);
            (center + vec2(x3, y3) * s, z)
        };
        let mut items: Vec<(f32, egui::Vec2, usize, f32)> = Vec::new();
        let cols = self.columns as i32;
        for c in 0..cols {
            let cx = (c as f32 - (cols - 1) as f32 / 2.0) * (self.poster.x + self.gap);
            let col_shift = if c % 2 == 0 { 0.0 } else { step * 0.5 };
            let col_speed = 1.0 + 0.12 * ((c as f32 - (cols - 1) as f32 / 2.0).abs());
            for j in 0..rows {
                let y = (j as f32 * step + col_shift - off * col_speed).rem_euclid(period) - period / 2.0;
                let world = vec2(cx - self.poster.x / 2.0, y - self.poster.y / 2.0);
                let theta = (y / ry).abs();
                if theta > 1.65 {
                    continue;
                }
                let z = project(world + self.poster / 2.0).1;
                let idx = ((j + 1000) as usize * cols as usize + c as usize) % texs.len();
                let alpha = 1.0 - smoothstep(1.15, 1.6, theta);
                items.push((z, world, idx, alpha));
            }
        }
        items.sort_by(|a, b| b.0.total_cmp(&a.0));
        let zmax = rx * 0.4 + ry * 0.9;
        for (z, world, idx, alpha) in items {
            if alpha <= 0.01 {
                continue;
            }
            let depth = (z / zmax).clamp(0.0, 1.0);
            let shade_k = 1.0 - 0.55 * depth;
            let tint = Color32::from_gray((255.0 * shade_k) as u8).gamma_multiply(alpha);
            let map = |l: egui::Vec2| project(world + l).0;
            warp_card(&painter, self.poster, 12.0, 4, Some(texs[idx]), uv, map, |l| {
                // a little top sheen
                let k = 1.0 - (l.y / self.poster.y);
                mix(tint, Color32::WHITE.gamma_multiply(alpha), 0.12 * k * k)
            });
        }
        paint::fade_edges_vertical(&painter, rect, 46.0, t.card);
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A device mockup that starts tilted back (like a laptop lid seen from the front) and rotates
/// flat, slightly shrinking into place, as it scrolls into view; a title above it lifts away.
///
/// The screen holds any egui content. Progress comes from the widget's position inside the
/// enclosing `ScrollArea` viewport, or from [`Self::progress`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::ContainerScroll::new()
///     .title("Unleash the power of scroll")
///     .show(ui, |ui| { ui.heading("Dashboard"); });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct ContainerScroll {
    title: String,
    subtitle: String,
    width: f32,
    height: f32,
    tilt: f32,
    start_scale: f32,
    progress: Option<f32>,
    frame: Option<Color32>,
}

impl Default for ContainerScroll {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerScroll {
    /// A device with no title.
    pub fn new() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            width: 760.0,
            height: 360.0,
            tilt: 24.0,
            start_scale: 1.04,
            progress: None,
            frame: None,
        }
    }
    /// Large heading above the device.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Small line above the heading.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Device size (outer frame).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.width = s.x.max(80.0);
        self.height = s.y.max(60.0);
        self
    }
    /// Starting tilt in degrees.
    pub fn tilt(mut self, deg: f32) -> Self {
        self.tilt = deg.clamp(0.0, 70.0);
        self
    }
    /// Starting scale (it settles to 1).
    pub fn start_scale(mut self, s: f32) -> Self {
        self.start_scale = s.clamp(0.5, 1.5);
        self
    }
    /// Drive the animation manually (0 = tilted, 1 = flat).
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p);
        self
    }
    /// Device frame color (defaults to a dark graphite bezel).
    pub fn frame_color(mut self, c: Color32) -> Self {
        self.frame = Some(c);
        self
    }

    /// Show the device; `screen(ui)` lays out what appears on it.
    pub fn show(self, ui: &mut Ui, screen: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let head_h = if self.title.is_empty() { 0.0 } else { 74.0 + if self.subtitle.is_empty() { 0.0 } else { 22.0 } };
        let w = self.width.min(ui.available_width() - 8.0);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), head_h + self.height + 30.0), Sense::hover());
        let raw = self.progress.unwrap_or_else(|| view_progress(ui, rect, 0.12));
        let e = ease(Easing::OutCubic, raw.clamp(0.0, 1.0));
        if self.progress.is_none() {
            // keep tracking while the parent scrolls
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(40.0));
        // title lifts and fades in as the device settles
        if !self.title.is_empty() {
            let ty = rect.min.y + 8.0 - 26.0 * e + 20.0;
            let a = 0.55 + 0.45 * e;
            if !self.subtitle.is_empty() {
                painter.text(
                    pos2(rect.center().x, ty),
                    Align2::CENTER_TOP,
                    &self.subtitle,
                    FontId::proportional(16.0),
                    with_alpha(t.muted_foreground, a),
                );
            }
            let y2 = ty + if self.subtitle.is_empty() { 0.0 } else { 24.0 };
            let g = painter.layout_no_wrap(self.title.clone(), FontId::proportional(38.0), t.foreground);
            let pos = pos2(rect.center().x - g.size().x / 2.0, y2);
            let stops = [t.foreground, mix(t.foreground, t.gradient[1], 0.6)];
            paint::galley_with(&painter, pos, &g, |p| {
                with_alpha(paint::linear_fn(Rect::from_min_size(pos, g.size()), 0.0, &stops)(p), a)
            });
        }
        let dev = Rect::from_center_size(pos2(rect.center().x, rect.min.y + head_h + 12.0 + self.height / 2.0), vec2(w, self.height));
        let mut persp = Persp::new(pos2(dev.center().x, dev.max.y));
        persp.rx = self.tilt.to_radians() * (1.0 - e);
        persp.scale = self.start_scale + (1.0 - self.start_scale) * e;
        persp.focal = 1100.0;
        persp.translate = vec2(0.0, -10.0 * e);
        let frame = self.frame.unwrap_or(if t.dark { Color32::from_rgb(34, 34, 38) } else { Color32::from_rgb(38, 38, 44) });
        let screen_r = dev.shrink(12.0);
        let inner_clip = screen_r;
        paint_warped(ui, |p| persp.apply(p), |ui| {
            let p = ui.painter().with_clip_rect(dev.expand(60.0));
            let sc = if t.dark { Color32::from_black_alpha(170) } else { Color32::from_rgba_unmultiplied(20, 20, 60, 60) };
            paint::soft_shadow(&p, dev, 30.0, 40.0, vec2(0.0, 22.0), sc);
            p.rect(dev, CornerRadius::same(30), frame, Stroke::new(3.0, Color32::from_gray(108)), StrokeKind::Inside);
            p.rect_filled(screen_r, CornerRadius::same(18), t.background);
            let mut child = ui.new_child(UiBuilder::new().max_rect(screen_r.shrink(14.0)).id_salt("container_scroll_screen"));
            child.set_clip_rect(inner_clip);
            screen(&mut child);
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// One product tile of a [`HeroParallax`].
#[derive(Clone, Debug, PartialEq)]
pub struct ParallaxProduct {
    /// Name shown when the tile is hovered.
    pub title: String,
    /// Screenshot / artwork; `None` uses a [`PaintedScene`].
    pub texture: Option<TextureId>,
}

impl ParallaxProduct {
    /// A product with a title and built-in artwork.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), texture: None }
    }
    /// Artwork texture.
    pub fn texture(mut self, t: TextureId) -> Self {
        self.texture = Some(t);
        self
    }
}

/// A landing-page hero: a big heading over three rows of product tiles that start tilted and
/// faded in 3D; as you scroll they rotate flat while the rows slide in opposite directions.
/// Hovering a tile lifts it and shows its name.
///
/// Owns its viewport (wheel / drag / autoplay), or follow an external value with [`Self::progress`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{HeroParallax, ParallaxProduct};
/// let products: Vec<_> = (0..15).map(|i| ParallaxProduct::new(format!("Product {i}"))).collect();
/// ui.add(HeroParallax::new(&products).title("The Ultimate\ndevelopment studio"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct HeroParallax<'a> {
    products: &'a [ParallaxProduct],
    title: String,
    body: String,
    height: f32,
    card: egui::Vec2,
    progress: Option<f32>,
    speed: f32,
    salt: u64,
}

impl<'a> HeroParallax<'a> {
    /// A hero over `products` (shown in three rows).
    pub fn new(products: &'a [ParallaxProduct]) -> Self {
        Self {
            products,
            title: "The Ultimate\ndevelopment studio".into(),
            body: "We build beautiful products with the latest technologies and frameworks.".into(),
            height: 440.0,
            card: vec2(230.0, 150.0),
            progress: None,
            speed: 120.0,
            salt: 0,
        }
    }
    /// Heading text (use `\n` for line breaks).
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Paragraph under the heading.
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = s.into();
        self
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Tile size.
    pub fn card_size(mut self, s: egui::Vec2) -> Self {
        self.card = s.max(vec2(40.0, 30.0));
        self
    }
    /// Drive the effect from outside (0 = top of page, 1 = fully scrolled).
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p);
        self
    }
    /// Autoplay speed when idle (points per second of virtual scroll; 0 = off).
    pub fn autoplay(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl egui::Widget for HeroParallax<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let response = ui.interact(rect, ui.scope_id().with(("glowkit_hero_parallax", self.salt)), Sense::drag());
        let id = response.id;
        let range = 520.0;
        let p = match self.progress {
            Some(p) => p.clamp(0.0, 1.0),
            None => drive_scroll(ui, id, &response, range, false, self.speed) / range,
        };
        if !ui.is_rect_visible(rect) || self.products.is_empty() {
            return response;
        }
        let pointer = response.hover_pos();
        let clip = rect.intersect(ui.clip_rect());
        let scroll = p * range;
        // heading
        {
            let painter = ui.painter().with_clip_rect(clip);
            let x = rect.min.x + 30.0;
            let y = rect.min.y + 30.0 - scroll * 0.75;
            let g = painter.layout(self.title.clone(), FontId::proportional(40.0), t.foreground, rect.width() - 60.0);
            let h = g.size().y;
            painter.galley(pos2(x, y), g, t.foreground);
            let b = painter.layout(self.body.clone(), FontId::proportional(15.5), t.muted_foreground, 520.0f32.min(rect.width() - 60.0));
            painter.galley(pos2(x, y + h + 12.0), b, t.muted_foreground);
        }
        let e = smoothstep(0.0, 0.55, p);
        let block_top = rect.min.y + 270.0 - scroll * 0.75;
        let (cw, chh) = (self.card.x, self.card.y);
        let gap = 22.0;
        let block_h = 3.0 * chh + 2.0 * gap;
        let mut persp = Persp::new(pos2(rect.center().x, block_top + block_h / 2.0));
        persp.rx = 15f32.to_radians() * (1.0 - e);
        persp.rz = 16f32.to_radians() * (1.0 - e);
        persp.focal = 900.0;
        persp.translate = vec2(0.0, 30.0 * (1.0 - e));
        let alpha = 0.25 + 0.75 * e;
        let n = self.products.len();
        let per_row = n.div_ceil(3).max(1);
        let builtin: Vec<TextureId> = (0..PaintedScene::COUNT).map(|i| PaintedScene::texture(ui.ctx(), i)).collect();
        let uv = cover_uv(PaintedScene::ASPECT, cw / chh);
        let mut hover_title: Option<(usize, [egui::Pos2; 4], f32)> = None;
        let painter = ui.painter().with_clip_rect(clip);
        for row in 0..3 {
            let dir = if row == 1 { -1.0 } else { 1.0 };
            let row_w = per_row as f32 * (cw + gap) - gap;
            let base_x = rect.center().x - row_w / 2.0 + dir * (p * 360.0 - 180.0) * 0.9;
            let y = block_top + row as f32 * (chh + gap);
            for k in 0..per_row {
                let i = row * per_row + k;
                if i >= n {
                    break;
                }
                let prod = &self.products[i];
                let flat = Rect::from_min_size(pos2(base_x + k as f32 * (cw + gap), y), vec2(cw, chh));
                let corners = [flat.left_top(), flat.right_top(), flat.right_bottom(), flat.left_bottom()].map(|q| persp.apply(q));
                let hov = pointer.is_some_and(|q| in_convex(&corners, q));
                let lift = glowkit_core::anim::spring(ui, id.with(("lift", i)), if hov { 1.0 } else { 0.0 }, 260.0, 20.0);
                let flat = flat.translate(vec2(0.0, -14.0 * lift));
                if !clip.expand(200.0).intersects(Rect::from_points(&corners)) {
                    continue;
                }
                let tex = prod.texture.unwrap_or(builtin[i % builtin.len()]);
                let tuv = if prod.texture.is_some() { Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)) } else { uv };
                let sh = if t.dark { Color32::BLACK } else { Color32::from_rgb(40, 40, 90) };
                warp_card(&painter, flat.size(), 12.0, 1, None, Rect::ZERO, |l| persp.apply(flat.min + l + vec2(0.0, 8.0 + 8.0 * lift)), |_| {
                    with_alpha(sh, (0.22 + 0.1 * lift) * alpha)
                });
                warp_card(&painter, flat.size(), 12.0, 3, Some(tex), tuv, |l| persp.apply(flat.min + l), |_| {
                    Color32::WHITE.gamma_multiply(alpha)
                });
                if lift > 0.02 {
                    let fc = [flat.left_top(), flat.right_top(), flat.right_bottom(), flat.left_bottom()].map(|q| persp.apply(q));
                    hover_title = Some((i, fc, lift));
                    warp_card(&painter, flat.size(), 12.0, 3, None, Rect::ZERO, |l| persp.apply(flat.min + l), |l| {
                        let k = (l.y / flat.height()).clamp(0.0, 1.0);
                        with_alpha(Color32::BLACK, lift * (0.15 + 0.5 * k * k))
                    });
                }
            }
        }
        if let Some((i, fc, a)) = hover_title {
            painter.text(
                fc[3] + (fc[2] - fc[3]) * 0.06 + (fc[0] - fc[3]) * 0.12,
                Align2::LEFT_BOTTOM,
                &self.products[i].title,
                FontId::proportional(16.0),
                with_alpha(Color32::WHITE, a),
            );
        }
        paint::fade_edges_vertical(&painter, rect, 28.0, t.card);
        response
    }
}
