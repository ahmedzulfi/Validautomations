//! Shared card plumbing: layout of a container with an effect painted behind (and optionally
//! in front of) caller content, pointer tracking and a few painting helpers.

use egui::epaint::Mesh;
use egui::{pos2, vec2, Align, Color32, Id, InnerResponse, Layout, Pos2, Rect, Sense, Shape, Ui, UiBuilder, Vec2};
use glowkit_core::Theme;

/// Common sizing / styling options every card has.
#[derive(Clone, Debug)]
pub(crate) struct Base {
    pub width: Option<f32>,
    pub min_height: f32,
    pub padding: f32,
    pub radius: Option<f32>,
    pub background: Option<Color32>,
}

impl Default for Base {
    fn default() -> Self {
        Self { width: None, min_height: 0.0, padding: 24.0, radius: None, background: None }
    }
}

impl Base {
    pub fn radius(&self, t: &Theme) -> f32 {
        self.radius.unwrap_or(t.radius * 1.4)
    }
    pub fn background(&self, t: &Theme) -> Color32 {
        self.background.unwrap_or(t.card)
    }
}

/// Builder setters shared by every card type (expects a `base: Base` field).
macro_rules! base_setters {
    () => {
        /// Fixed width in points (default: all available width).
        pub fn width(mut self, width: f32) -> Self {
            self.base.width = Some(width);
            self
        }
        /// Minimum height in points (the card grows to fit its content).
        pub fn min_height(mut self, height: f32) -> Self {
            self.base.min_height = height;
            self
        }
        /// Width and minimum height in one call.
        pub fn size(mut self, size: egui::Vec2) -> Self {
            self.base.width = Some(size.x);
            self.base.min_height = size.y;
            self
        }
        /// Inner padding around the content, in points.
        pub fn padding(mut self, padding: f32) -> Self {
            self.base.padding = padding;
            self
        }
        /// Corner radius. Defaults to 1.4x the theme radius.
        pub fn radius(mut self, radius: f32) -> Self {
            self.base.radius = Some(radius);
            self
        }
        /// Card fill color. Defaults to the theme's card color.
        pub fn background(mut self, color: egui::Color32) -> Self {
            self.base.background = Some(color);
            self
        }
    };
}
pub(crate) use base_setters;

/// What the effect knows about the card while painting.
pub(crate) struct ShellCtx {
    pub rect: Rect,
    pub id: Id,
    /// Pointer position if it is over the card (and this layer).
    pub pointer: Option<Pos2>,
}

/// How the effect wants the content placed.
#[derive(Clone, Copy)]
pub(crate) struct Content {
    pub offset: Vec2,
    pub opacity: f32,
}

impl Default for Content {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, opacity: 1.0 }
    }
}

/// Lay out a card: `bg` paints the effect behind and returns content placement, `add` fills the
/// card, `fg` paints on top of the content.
pub(crate) fn show_shell<R>(
    ui: &mut Ui,
    base: &Base,
    bg: impl FnOnce(&Ui, &ShellCtx) -> Content,
    add: impl FnOnce(&mut Ui) -> R,
    fg: impl FnOnce(&Ui, &ShellCtx),
) -> InnerResponse<R> {
    let id = ui.next_auto_id();
    let width = base.width.unwrap_or_else(|| ui.available_width()).max(base.padding * 2.0 + 1.0);
    let h_key = id.with("glowkit_card_h");
    let height: f32 = glowkit_core::state(ui, h_key, || base.min_height.max(base.padding * 2.0));
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height.max(base.min_height)), Sense::hover());
    let pointer = if ui.rect_contains_pointer(rect) { ui.input(|i| i.pointer.hover_pos()) } else { None };
    let ctx = ShellCtx { rect, id, pointer };
    let content = bg(ui, &ctx);
    let inner_rect = rect.shrink(base.padding).translate(content.offset);
    let mut child = ui.new_child(UiBuilder::new().id_salt(id.with("content")).max_rect(inner_rect).layout(Layout::top_down(Align::Min)));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    if content.opacity < 1.0 {
        child.multiply_opacity(content.opacity);
    }
    let r = add(&mut child);
    let measured = (child.min_rect().height() + base.padding * 2.0).max(base.min_height);
    if (measured - height).abs() > 0.5 {
        glowkit_core::set_state(ui, h_key, measured);
        ui.ctx().request_repaint();
    }
    fg(ui, &ctx);
    InnerResponse::new(r, resp)
}

/// Project a point into a rounded rect (points inside are unchanged).
pub(crate) fn project_into(rect: Rect, radius: f32, p: Pos2) -> Pos2 {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    let q = p.clamp(rect.min, rect.max);
    let cx = q.x.clamp(rect.min.x + r, rect.max.x - r);
    let cy = q.y.clamp(rect.min.y + r, rect.max.y - r);
    let c = pos2(cx, cy);
    let d = q - c;
    let len = d.length();
    if len > r && len > 0.0 {
        c + d * (r / len)
    } else {
        q
    }
}

/// A "fragment shader" over a rounded rect: a grid mesh whose outer vertices are pulled onto
/// the rounded outline, colored by `f(point)`.
pub(crate) fn shade_rounded(painter: &egui::Painter, rect: Rect, radius: f32, cell: f32, f: impl Fn(Pos2) -> Color32) {
    let cols = ((rect.width() / cell).ceil() as usize).clamp(2, 80);
    let rows = ((rect.height() / cell).ceil() as usize).clamp(2, 60);
    let mut m = Mesh::default();
    m.reserve_vertices((cols + 1) * (rows + 1));
    for y in 0..=rows {
        for x in 0..=cols {
            let p = pos2(rect.min.x + rect.width() * x as f32 / cols as f32, rect.min.y + rect.height() * y as f32 / rows as f32);
            let p = project_into(rect, radius, p);
            m.colored_vertex(p, f(p));
        }
    }
    let w = (cols + 1) as u32;
    for y in 0..rows as u32 {
        for x in 0..cols as u32 {
            let i = y * w + x;
            m.add_triangle(i, i + 1, i + w + 1);
            m.add_triangle(i, i + w + 1, i + w);
        }
    }
    painter.add(Shape::mesh(m));
}

/// A slow Lissajous path inside `rect`, used as a virtual pointer when a card is "ambient".
pub(crate) fn ambient_point(rect: Rect, time: f32, seed: f32) -> Pos2 {
    let c = rect.center();
    c + vec2((time * 0.83 + seed).sin() * 0.36 * rect.width(), (time * 1.21 + seed * 2.3).sin() * 0.32 * rect.height())
}

/// Smoothed pointer for card `id`: returns `(position, strength)` where strength fades 0..1 as
/// the pointer (or the ambient path) enters/leaves.
pub(crate) fn tracked_pointer(ui: &Ui, ctx: &ShellCtx, ambient: bool, seed: f32) -> (Pos2, f32) {
    let time = glowkit_core::time(ui);
    let target = match ctx.pointer {
        Some(p) => Some(p),
        None if ambient => {
            glowkit_core::animate(ui);
            Some(ambient_point(ctx.rect, time, seed))
        }
        None => None,
    };
    let key = ctx.id.with("glowkit_ptr");
    let last: Vec2 = glowkit_core::state(ui, key, || ctx.rect.size() / 2.0);
    let local = target.map(|p| p - ctx.rect.min).unwrap_or(last);
    glowkit_core::set_state(ui, key, local);
    let s = glowkit_core::anim::spring2(ui, ctx.id.with("glowkit_ptr_s"), local, 260.0, 30.0);
    let k = glowkit_core::anim::follow(ui, ctx.id.with("glowkit_ptr_k"), if target.is_some() { 1.0 } else { 0.0 }, 7.0);
    (ctx.rect.min + s, k)
}

/// The standard card surface: soft shadow, fill and hairline border.
pub(crate) fn card_surface(painter: &egui::Painter, t: &Theme, rect: Rect, radius: f32, fill: Color32, border: Option<Color32>) {
    glowkit_core::paint::soft_shadow(painter, rect, radius, 20.0, vec2(0.0, 6.0), glowkit_core::with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.07 }));
    painter.rect_filled(rect, crate::buttons::util::cr(radius), fill);
    if let Some(b) = border {
        painter.rect_stroke(rect, crate::buttons::util::cr(radius), egui::Stroke::new(1.0, b), egui::StrokeKind::Inside);
    }
}
