//! [`CanvasView`] (pan + zoom transform), the [`InfiniteCanvas`] widget, its [`Background`]
//! patterns, the [`MiniMap`] and the [`Controls`] panel.

use crate::common::{icon_button, panel, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Key, Painter, PointerButton, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{anim::approach, color::with_alpha, Theme};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ViewAnim {
    None,
    /// Smooth zoom keeping the world point under `anchor` (relative to the canvas min) fixed.
    ZoomAt { anchor: Vec2, target: f32 },
    /// Fly to a pan/zoom pair.
    To { pan: Vec2, zoom: f32 },
}

/// Pan / zoom state of an infinite canvas. Owned by the caller so it survives across frames
/// and can be saved, reset, or driven programmatically.
///
/// World coordinates map to screen as `screen = canvas.min + pan + world * zoom`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut view = glowkit_canvas::CanvasView::default();
/// glowkit_canvas::InfiniteCanvas::new(&mut view).show(ui, |cv| {
///     let p = cv.to_screen(egui::pos2(0.0, 0.0));
///     cv.painter.circle_filled(p, 8.0 * cv.zoom(), egui::Color32::RED);
/// });
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasView {
    /// Screen-space offset of the world origin relative to the canvas' top-left corner.
    pub pan: Vec2,
    /// Current zoom factor (1 = 100%).
    pub zoom: f32,
    /// Smallest allowed zoom.
    pub min_zoom: f32,
    /// Largest allowed zoom.
    pub max_zoom: f32,
    /// When locked, the user cannot pan or zoom (programmatic changes still work).
    pub locked: bool,
    /// Set until the owning component had a chance to fit its content on first show.
    pub needs_fit: bool,
    anim: ViewAnim,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0, min_zoom: 0.15, max_zoom: 4.0, locked: false, needs_fit: true, anim: ViewAnim::None }
    }
}

/// Compute `(pan, zoom)` so `content` fills a viewport of `size` with `padding` points around it.
pub fn fit_view(size: Vec2, content: Rect, padding: f32, min_zoom: f32, max_zoom: f32) -> (Vec2, f32) {
    if !content.is_positive() && content.width() < 0.0 {
        return (size / 2.0, 1.0);
    }
    let avail = (size - Vec2::splat(padding * 2.0)).max(Vec2::splat(1.0));
    let z = (avail.x / content.width().max(1.0)).min(avail.y / content.height().max(1.0)).clamp(min_zoom, max_zoom);
    let pan = size / 2.0 - content.center().to_vec2() * z;
    (pan, z)
}

impl CanvasView {
    /// A view with the given zoom limits.
    pub fn with_limits(min_zoom: f32, max_zoom: f32) -> Self {
        Self { min_zoom, max_zoom, ..Default::default() }
    }

    /// World → screen for a canvas occupying `rect`.
    pub fn to_screen(&self, rect: Rect, world: Pos2) -> Pos2 {
        rect.min + self.pan + world.to_vec2() * self.zoom
    }

    /// Screen → world for a canvas occupying `rect`.
    pub fn to_world(&self, rect: Rect, screen: Pos2) -> Pos2 {
        ((screen - rect.min - self.pan) / self.zoom).to_pos2()
    }

    /// World rect → screen rect.
    pub fn rect_to_screen(&self, rect: Rect, world: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(rect, world.min), self.to_screen(rect, world.max))
    }

    /// The part of the world currently visible.
    pub fn visible_world(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.to_world(rect, rect.min), self.to_world(rect, rect.max))
    }

    /// Zoom instantly by `factor`, keeping the world point under `anchor` (screen) fixed.
    pub fn zoom_at(&mut self, rect: Rect, anchor: Pos2, factor: f32) {
        let w = self.to_world(rect, anchor);
        self.zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);
        self.pan = anchor - rect.min - w.to_vec2() * self.zoom;
        self.anim = ViewAnim::None;
    }

    /// Animated zoom by `factor` around `anchor` (screen). Repeated calls accumulate.
    pub fn zoom_smooth(&mut self, rect: Rect, anchor: Pos2, factor: f32) {
        let base = match self.anim {
            ViewAnim::ZoomAt { target, .. } => target,
            ViewAnim::To { zoom, .. } => zoom,
            ViewAnim::None => self.zoom,
        };
        let target = (base * factor).clamp(self.min_zoom, self.max_zoom);
        self.anim = ViewAnim::ZoomAt { anchor: anchor - rect.min, target };
    }

    /// Animated zoom in around the canvas center.
    pub fn zoom_in(&mut self, rect: Rect) {
        self.zoom_smooth(rect, rect.center(), 1.25);
    }

    /// Animated zoom out around the canvas center.
    pub fn zoom_out(&mut self, rect: Rect) {
        self.zoom_smooth(rect, rect.center(), 0.8);
    }

    /// Pan by a screen-space delta (cancels animations).
    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
        if let ViewAnim::ZoomAt { anchor, target } = self.anim {
            self.anim = ViewAnim::ZoomAt { anchor: anchor + delta, target };
        } else {
            self.anim = ViewAnim::None;
        }
    }

    /// Animate so `content` (world) fits the canvas with `padding` points of margin.
    pub fn fit(&mut self, rect: Rect, content: Rect, padding: f32) {
        let (pan, zoom) = fit_view(rect.size(), content, padding, self.min_zoom, self.max_zoom.min(1.25));
        self.anim = ViewAnim::To { pan, zoom };
        self.needs_fit = false;
    }

    /// Fit instantly (no animation).
    pub fn fit_now(&mut self, rect: Rect, content: Rect, padding: f32) {
        let (pan, zoom) = fit_view(rect.size(), content, padding, self.min_zoom, self.max_zoom.min(1.25));
        self.pan = pan;
        self.zoom = zoom;
        self.anim = ViewAnim::None;
        self.needs_fit = false;
    }

    /// Animate so the world point `world` ends up in the canvas center (zoom unchanged).
    pub fn center_on(&mut self, rect: Rect, world: Pos2) {
        let zoom = match self.anim {
            ViewAnim::To { zoom, .. } | ViewAnim::ZoomAt { target: zoom, .. } => zoom,
            ViewAnim::None => self.zoom,
        };
        self.anim = ViewAnim::To { pan: rect.size() / 2.0 - world.to_vec2() * zoom, zoom };
    }

    /// Center instantly on `world`.
    pub fn center_now(&mut self, rect: Rect, world: Pos2) {
        self.pan = rect.size() / 2.0 - world.to_vec2() * self.zoom;
        self.anim = ViewAnim::None;
    }

    /// Is a pan/zoom animation running?
    pub fn is_animating(&self) -> bool {
        self.anim != ViewAnim::None
    }

    /// The zoom level the view is heading to (equals `zoom` when idle).
    pub fn target_zoom(&self) -> f32 {
        match self.anim {
            ViewAnim::To { zoom, .. } | ViewAnim::ZoomAt { target: zoom, .. } => zoom,
            ViewAnim::None => self.zoom,
        }
    }

    /// Advance animations by `dt` seconds. Returns `true` while still animating.
    pub fn step(&mut self, rect: Rect, dt: f32) -> bool {
        match self.anim {
            ViewAnim::None => false,
            ViewAnim::ZoomAt { anchor, target } => {
                let a = rect.min + anchor;
                let w = self.to_world(rect, a);
                let z = approach(self.zoom, target, 16.0, dt);
                let done = (z - target).abs() < target * 1e-3;
                self.zoom = if done { target } else { z };
                self.pan = anchor - w.to_vec2() * self.zoom;
                if done {
                    self.anim = ViewAnim::None;
                }
                !done
            }
            ViewAnim::To { pan, zoom } => {
                // interpolate the visible center in world space so the move feels like a camera
                let k = 1.0 - (-9.0 * dt).exp();
                let c_now = self.to_world(rect, rect.center());
                let c_tgt = ((rect.size() / 2.0 - pan) / zoom).to_pos2();
                let z = (self.zoom.ln() + (zoom.ln() - self.zoom.ln()) * k).exp();
                let c = c_now + (c_tgt - c_now) * k;
                self.zoom = z;
                self.pan = rect.size() / 2.0 - c.to_vec2() * z;
                let done = (self.pan - pan).length() < 0.3 && (self.zoom - zoom).abs() < 1e-3;
                if done {
                    self.pan = pan;
                    self.zoom = zoom;
                    self.anim = ViewAnim::None;
                }
                !done
            }
        }
    }
}

/// Background pattern of an [`InfiniteCanvas`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Background {
    /// A dot at every grid intersection.
    #[default]
    Dots,
    /// Thin grid lines with stronger major lines every 5 cells.
    Lines,
    /// Small crosses at every intersection.
    Cross,
    /// Plain fill.
    None,
}

/// Paint a background pattern for a canvas occupying `rect`.
pub fn paint_background(painter: &Painter, rect: Rect, view: &CanvasView, kind: Background, gap: f32, color: Color32) {
    if kind == Background::None {
        return;
    }
    let mut step = gap.max(1.0);
    while step * view.zoom < 10.0 {
        step *= 2.0;
    }
    let s = step * view.zoom;
    let origin = rect.min + view.pan;
    let x0 = origin.x + ((rect.left() - origin.x) / s).floor() * s;
    let y0 = origin.y + ((rect.top() - origin.y) / s).floor() * s;
    // fade the pattern as cells get small
    let fade = ((s - 10.0) / 14.0).clamp(0.25, 1.0);
    let col = color.gamma_multiply(fade);
    let nx = ((rect.right() - x0) / s).ceil() as i64 + 1;
    let ny = ((rect.bottom() - y0) / s).ceil() as i64 + 1;
    match kind {
        Background::Dots => {
            let r = (1.1 * view.zoom.sqrt()).clamp(0.7, 1.8);
            let mut mesh = egui::Mesh::default();
            for iy in 0..ny {
                for ix in 0..nx {
                    let p = pos2(x0 + ix as f32 * s, y0 + iy as f32 * s);
                    if rect.contains(p) {
                        let base = mesh.vertices.len() as u32;
                        mesh.colored_vertex(p, col);
                        for k in 0..6 {
                            let a = k as f32 / 6.0 * std::f32::consts::TAU;
                            mesh.colored_vertex(p + vec2(a.cos(), a.sin()) * r, col);
                        }
                        for k in 0..6u32 {
                            mesh.add_triangle(base, base + 1 + k, base + 1 + (k + 1) % 6);
                        }
                    }
                }
            }
            painter.add(mesh);
        }
        Background::Lines => {
            let major = step * 5.0 * view.zoom;
            for ix in 0..nx {
                let x = x0 + ix as f32 * s;
                let is_major = (((x - origin.x) / major).round() * major - (x - origin.x)).abs() < 0.5;
                let c = if is_major { color } else { col.gamma_multiply(0.55) };
                painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(if is_major { 1.0 } else { 0.6 }, c));
            }
            for iy in 0..ny {
                let y = y0 + iy as f32 * s;
                let is_major = (((y - origin.y) / major).round() * major - (y - origin.y)).abs() < 0.5;
                let c = if is_major { color } else { col.gamma_multiply(0.55) };
                painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(if is_major { 1.0 } else { 0.6 }, c));
            }
        }
        Background::Cross => {
            let arm = (s * 0.12).clamp(2.0, 5.0);
            let st = Stroke::new(1.0, col);
            for iy in 0..ny {
                for ix in 0..nx {
                    let p = pos2(x0 + ix as f32 * s, y0 + iy as f32 * s);
                    if rect.contains(p) {
                        painter.line_segment([p - vec2(arm, 0.0), p + vec2(arm, 0.0)], st);
                        painter.line_segment([p - vec2(0.0, arm), p + vec2(0.0, arm)], st);
                    }
                }
            }
        }
        Background::None => {}
    }
}

/// What a plain scroll wheel / two-finger swipe does on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WheelMode {
    /// Scroll pans; ctrl/cmd + scroll and pinch zoom (Figma / tldraw).
    #[default]
    Pan,
    /// Scroll zooms (React Flow default).
    Zoom,
}

/// Context handed to the [`InfiniteCanvas`] content closure.
pub struct CanvasUi<'a> {
    /// The ui the canvas lives in (use it for `interact`, `put`, ...).
    pub ui: &'a mut Ui,
    /// Screen rect of the canvas.
    pub rect: Rect,
    /// Painter clipped to the canvas.
    pub painter: Painter,
    /// Background response (click / drag on empty space).
    pub response: Response,
    /// The view transform (mutable: content may animate it).
    pub view: &'a mut CanvasView,
    /// True while space is held (space+drag pans; content should ignore drags).
    pub space_pan: bool,
}

impl CanvasUi<'_> {
    /// World → screen.
    pub fn to_screen(&self, p: Pos2) -> Pos2 {
        self.view.to_screen(self.rect, p)
    }
    /// Screen → world.
    pub fn to_world(&self, p: Pos2) -> Pos2 {
        self.view.to_world(self.rect, p)
    }
    /// World rect → screen rect.
    pub fn rect_to_screen(&self, r: Rect) -> Rect {
        self.view.rect_to_screen(self.rect, r)
    }
    /// Current zoom.
    pub fn zoom(&self) -> f32 {
        self.view.zoom
    }
    /// Pointer position in world coordinates (if over the canvas).
    pub fn pointer_world(&self) -> Option<Pos2> {
        self.ui.input(|i| i.pointer.hover_pos()).filter(|p| self.rect.contains(*p)).map(|p| self.to_world(p))
    }
}

/// An infinite, pannable, zoomable surface (the base of every editor in this crate).
///
/// * Pan: drag the background, middle-mouse drag anywhere, space + drag, or two-finger scroll.
/// * Zoom: ctrl/cmd + wheel or pinch, smoothly animated around the cursor.
/// * Background: dots, lines or crosses that scale with the zoom (see [`Background`]).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut view = glowkit_canvas::CanvasView::default();
/// use glowkit_canvas::{Background, InfiniteCanvas};
/// InfiniteCanvas::new(&mut view).background(Background::Lines).height(480.0).show(ui, |cv| {
///     let r = cv.rect_to_screen(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(120.0, 80.0)));
///     cv.painter.rect_filled(r, 8.0, egui::Color32::DARK_GRAY);
/// });
/// ```
#[must_use = "call `.show(ui, |cv| ...)`"]
pub struct InfiniteCanvas<'a> {
    view: &'a mut CanvasView,
    background: Background,
    gap: f32,
    size: Option<Vec2>,
    pan_on_drag: bool,
    wheel: WheelMode,
    fill: Option<Color32>,
    pattern: Option<Color32>,
    id_salt: Option<Id>,
    rounding: f32,
    border: bool,
}

/// Result of [`InfiniteCanvas::show`].
pub struct CanvasResponse<R> {
    /// Background response.
    pub response: Response,
    /// Screen rect of the canvas.
    pub rect: Rect,
    /// Whatever the content closure returned.
    pub inner: R,
}

impl<'a> InfiniteCanvas<'a> {
    /// A canvas driven by the caller-owned `view`.
    pub fn new(view: &'a mut CanvasView) -> Self {
        Self { view, background: Background::Dots, gap: 20.0, size: None, pan_on_drag: true, wheel: WheelMode::Pan, fill: None, pattern: None, id_salt: None, rounding: 10.0, border: true }
    }
    /// Background pattern.
    pub fn background(mut self, b: Background) -> Self {
        self.background = b;
        self
    }
    /// Grid spacing in world units.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    /// Size of the canvas (defaults to available width x 480).
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    /// Height of the canvas (width = available).
    pub fn height(mut self, h: f32) -> Self {
        self.size = None;
        self.rounding = self.rounding.max(0.0);
        self.gap = self.gap.max(1.0);
        self.size = Some(vec2(f32::NAN, h));
        self
    }
    /// Whether dragging the empty background with the primary button pans (default `true`).
    pub fn pan_on_drag(mut self, on: bool) -> Self {
        self.pan_on_drag = on;
        self
    }
    /// What the plain scroll wheel does.
    pub fn wheel(mut self, mode: WheelMode) -> Self {
        self.wheel = mode;
        self
    }
    /// Background fill (defaults to the theme background).
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Pattern color (defaults to a faint foreground).
    pub fn pattern_color(mut self, c: Color32) -> Self {
        self.pattern = Some(c);
        self
    }
    /// Salt for the widget id when several canvases share a parent.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
    /// Corner radius of the canvas frame.
    pub fn rounding(mut self, r: f32) -> Self {
        self.rounding = r;
        self
    }
    /// Draw a hairline border around the canvas.
    pub fn border(mut self, on: bool) -> Self {
        self.border = on;
        self
    }

    /// Show the canvas and run `add` to paint / interact with its content.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut CanvasUi) -> R) -> CanvasResponse<R> {
        let t = Theme::of(ui);
        let size = match self.size {
            Some(s) if s.x.is_nan() => vec2(ui.available_width(), s.y),
            Some(s) => s,
            None => vec2(ui.available_width(), 480.0),
        };
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let id = match self.id_salt {
            Some(s) => ui.scope_id().with(s),
            None => ui.scope_id().with("glowkit_infinite_canvas"),
        };
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let view = self.view;
        let dt = glowkit_core::dt(ui);
        let hovered = ui.rect_contains_pointer(rect);
        let typing = ui.ctx().text_edit_focused();
        let space = hovered && !typing && ui.input(|i| i.key_down(Key::Space));

        // --- wheel / pinch / trackpad
        if hovered && !view.locked {
            let (zoom_delta, touch, scroll, pointer) = ui.input(|i| (i.zoom_delta(), i.multi_touch(), i.smooth_scroll_delta, i.pointer.hover_pos()));
            let anchor = pointer.unwrap_or(rect.center());
            if let Some(touch) = touch {
                view.zoom_at(rect, touch.center_pos, touch.zoom_delta);
                view.pan_by(touch.translation_delta);
            } else {
                if (zoom_delta - 1.0).abs() > 1e-4 {
                    view.zoom_smooth(rect, anchor, zoom_delta);
                }
                if scroll != Vec2::ZERO {
                    match self.wheel {
                        WheelMode::Pan => view.pan_by(scroll),
                        WheelMode::Zoom => view.zoom_smooth(rect, anchor, (scroll.y / 300.0).exp()),
                    }
                }
            }
            if scroll != Vec2::ZERO || (zoom_delta - 1.0).abs() > 1e-4 {
                ui.input_mut(|i| {
                    i.smooth_scroll_delta = Vec2::ZERO;
                });
            }
        }
        // --- drag to pan (background primary, or middle anywhere)
        if !view.locked {
            if self.pan_on_drag && response.dragged_by(PointerButton::Primary) && !space {
                view.pan_by(response.drag_delta());
            }
            let (middle, delta, press) = ui.input(|i| (i.pointer.button_down(PointerButton::Middle), i.pointer.delta(), i.pointer.press_origin()));
            if middle && press.is_some_and(|p| rect.contains(p)) {
                view.pan_by(delta);
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
        }
        if view.step(rect, dt) {
            ui.ctx().request_repaint();
        }

        // --- paint background
        let painter = ui.painter_at(rect);
        let fill = self.fill.unwrap_or(t.background);
        painter.rect_filled(rect, CornerRadius::same(self.rounding as u8), fill);
        let pattern = self.pattern.unwrap_or(with_alpha(t.muted_foreground, if t.dark { 0.28 } else { 0.38 }));
        paint_background(&painter, rect, view, self.background, self.gap, pattern);

        let mut cv = CanvasUi { ui, rect, painter, response: response.clone(), view, space_pan: space };
        let inner = add(&mut cv);
        let CanvasUi { ui, view, .. } = cv;

        // --- space + drag overlay sits on top of content
        if space && !view.locked {
            let ov = ui.interact(rect, id.with("space_pan"), Sense::drag());
            if ov.dragged() {
                view.pan_by(ov.drag_delta());
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            } else {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        } else if self.pan_on_drag && !view.locked {
            if response.dragged_by(PointerButton::Primary) {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            } else if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        if self.border {
            ui.painter().rect_stroke(rect, CornerRadius::same(self.rounding as u8), Stroke::new(1.0, t.border), StrokeKind::Inside);
        }
        CanvasResponse { response, rect, inner }
    }
}

/// A miniature overview of the canvas. Shows item rects and the visible viewport; click or drag
/// inside it to navigate.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (mut view, canvas_rect) = (glowkit_canvas::CanvasView::default(), egui::Rect::ZERO);
/// let items = [(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 60.0)), egui::Color32::GRAY)];
/// glowkit_canvas::MiniMap::new(&mut view, canvas_rect).items(&items).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MiniMap<'a> {
    view: &'a mut CanvasView,
    canvas: Rect,
    items: &'a [(Rect, Color32)],
    size: Vec2,
    anchor: Align2,
    margin: f32,
}

impl<'a> MiniMap<'a> {
    /// A minimap for the canvas occupying `canvas` (screen).
    pub fn new(view: &'a mut CanvasView, canvas: Rect) -> Self {
        Self { view, canvas, items: &[], size: vec2(184.0, 124.0), anchor: Align2::RIGHT_BOTTOM, margin: 12.0 }
    }
    /// World-space rects (with colors) to draw.
    pub fn items(mut self, items: &'a [(Rect, Color32)]) -> Self {
        self.items = items;
        self
    }
    /// Panel size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
    /// Which corner of the canvas to sit in.
    pub fn anchor(mut self, a: Align2) -> Self {
        self.anchor = a;
        self
    }

    /// Paint and handle navigation. Returns the panel response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let rect = self.anchor.align_size_within_rect(self.size, self.canvas.shrink(self.margin));
        let id = ui.scope_id().with("glowkit_minimap");
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        let painter = ui.painter_at(rect.expand(20.0));
        panel(&painter, &t, rect, 10.0);
        let inner = rect.shrink(8.0);
        let visible = self.view.visible_world(self.canvas);
        let mut world = visible;
        for (r, _) in self.items {
            world = world.union(*r);
        }
        let world = world.expand(world.size().max_elem() * 0.05);
        let s = (inner.width() / world.width()).min(inner.height() / world.height());
        let off = inner.center() - world.center().to_vec2() * s;
        let map = |p: Pos2| off + p.to_vec2() * s;
        let unmap = |p: Pos2| ((p - off) / s).to_pos2();
        let mp = painter.with_clip_rect(inner);
        for (r, c) in self.items {
            let rr = Rect::from_min_max(map(r.min), map(r.max));
            mp.rect_filled(rr, CornerRadius::same(2), with_alpha(*c, 0.85));
        }
        let vr = Rect::from_min_max(map(visible.min), map(visible.max));
        // mask outside the viewport
        let mask = with_alpha(t.foreground, if t.dark { 0.06 } else { 0.05 });
        for m in [
            Rect::from_min_max(inner.min, pos2(inner.max.x, vr.min.y)),
            Rect::from_min_max(pos2(inner.min.x, vr.max.y), inner.max),
            Rect::from_min_max(pos2(inner.min.x, vr.min.y), pos2(vr.min.x, vr.max.y)),
            Rect::from_min_max(pos2(vr.max.x, vr.min.y), pos2(inner.max.x, vr.max.y)),
        ] {
            if m.is_positive() {
                mp.rect_filled(m.intersect(inner), CornerRadius::ZERO, mask);
            }
        }
        let hov = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered() || resp.dragged(), 0.15);
        mp.rect_stroke(vr, CornerRadius::same(3), Stroke::new(1.0 + hov * 0.5, t.accent.gamma_multiply(0.6 + 0.4 * hov)), StrokeKind::Middle);
        mp.rect_filled(vr, CornerRadius::same(3), with_alpha(t.accent, 0.06 + 0.05 * hov));
        if let Some(p) = resp.interact_pointer_pos() {
            if resp.is_pointer_button_down_on() {
                let w = unmap(p);
                if resp.drag_started() || resp.clicked() || !resp.dragged() {
                    self.view.center_on(self.canvas, w);
                } else {
                    self.view.center_now(self.canvas, w);
                }
            }
        }
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::PointingHand });
        }
        resp
    }
}

/// An action reported by the [`Controls`] panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    ZoomIn,
    ZoomOut,
    Fit,
    ToggleLock,
    /// One of the extra buttons, by index.
    Extra(usize),
}

/// A floating zoom-in / zoom-out / fit / lock control panel (plus optional extra buttons).
///
/// Zoom and lock are applied to the view directly; [`ControlAction::Fit`] and extras are
/// returned for the caller to handle (only it knows the content bounds).
#[must_use = "call `.show(ui)`"]
pub struct Controls<'a> {
    view: &'a mut CanvasView,
    canvas: Rect,
    anchor: Align2,
    extras: Vec<(Icon, &'static str, bool)>,
    horizontal: bool,
}

impl<'a> Controls<'a> {
    /// Controls for the canvas occupying `canvas` (screen).
    pub fn new(view: &'a mut CanvasView, canvas: Rect) -> Self {
        Self { view, canvas, anchor: Align2::LEFT_BOTTOM, extras: Vec::new(), horizontal: false }
    }
    /// Which corner to sit in.
    pub fn anchor(mut self, a: Align2) -> Self {
        self.anchor = a;
        self
    }
    /// Lay the buttons out in a row.
    pub fn horizontal(mut self, on: bool) -> Self {
        self.horizontal = on;
        self
    }
    /// Add an extra button (`active` highlights it).
    pub fn extra(mut self, icon: Icon, tooltip: &'static str, active: bool) -> Self {
        self.extras.push((icon, tooltip, active));
        self
    }

    /// Paint and handle clicks.
    pub fn show(self, ui: &mut Ui) -> Option<ControlAction> {
        let t = Theme::of(ui);
        let n = 4 + self.extras.len();
        let b = 30.0;
        let size = if self.horizontal { vec2(n as f32 * b + 8.0 + 46.0, b + 8.0) } else { vec2(b + 8.0, n as f32 * b + 8.0 + 24.0) };
        let rect = self.anchor.align_size_within_rect(size, self.canvas.shrink(12.0));
        panel(ui.painter(), &t, rect, 10.0);
        let mut out = None;
        let id = ui.scope_id().with("glowkit_controls");
        let mut buttons: Vec<(Icon, &str, bool, ControlAction)> = vec![
            (Icon::Plus, "Zoom in", false, ControlAction::ZoomIn),
            (Icon::Minus, "Zoom out", false, ControlAction::ZoomOut),
            (Icon::Fit, "Fit view", false, ControlAction::Fit),
            (if self.view.locked { Icon::Lock } else { Icon::Unlock }, "Lock pan & zoom", self.view.locked, ControlAction::ToggleLock),
        ];
        for (i, (ic, tip, act)) in self.extras.iter().enumerate() {
            buttons.push((*ic, tip, *act, ControlAction::Extra(i)));
        }
        let mut cursor = rect.min + vec2(4.0, 4.0);
        for (i, (icon, tip, active, action)) in buttons.into_iter().enumerate() {
            let r = Rect::from_min_size(cursor, Vec2::splat(b));
            if icon_button(ui, id.with(i), r, icon, active, tip).clicked() {
                out = Some(action);
            }
            if self.horizontal {
                cursor.x += b;
            } else {
                cursor.y += b;
            }
        }
        // zoom readout
        let pct = format!("{:.0}%", self.view.zoom * 100.0);
        let readout = if self.horizontal { Rect::from_min_max(pos2(cursor.x, rect.top()), rect.max) } else { Rect::from_min_max(pos2(rect.left(), cursor.y), rect.max) };
        ui.painter().text(readout.center(), Align2::CENTER_CENTER, pct, FontId::monospace(9.5), t.muted_foreground);
        match out {
            Some(ControlAction::ZoomIn) => self.view.zoom_in(self.canvas),
            Some(ControlAction::ZoomOut) => self.view.zoom_out(self.canvas),
            Some(ControlAction::ToggleLock) => self.view.locked = !self.view.locked,
            _ => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Rect {
        Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 400.0))
    }

    #[test]
    fn transform_roundtrip() {
        let mut v = CanvasView::default();
        v.pan = vec2(37.0, -12.0);
        v.zoom = 1.7;
        let w = pos2(123.0, -45.0);
        let back = v.to_world(canvas(), v.to_screen(canvas(), w));
        assert!((back - w).length() < 1e-3);
    }

    #[test]
    fn zoom_keeps_anchor_fixed() {
        let mut v = CanvasView::default();
        let anchor = pos2(300.0, 200.0);
        let before = v.to_world(canvas(), anchor);
        v.zoom_at(canvas(), anchor, 2.0);
        assert!((v.zoom - 2.0).abs() < 1e-5);
        let after = v.to_world(canvas(), anchor);
        assert!((before - after).length() < 1e-3);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut v = CanvasView::with_limits(0.5, 2.0);
        v.zoom_at(canvas(), canvas().center(), 100.0);
        assert_eq!(v.zoom, 2.0);
        v.zoom_at(canvas(), canvas().center(), 0.0001);
        assert_eq!(v.zoom, 0.5);
    }

    #[test]
    fn smooth_zoom_converges_to_target() {
        let mut v = CanvasView::default();
        let anchor = pos2(250.0, 120.0);
        let before = v.to_world(canvas(), anchor);
        v.zoom_smooth(canvas(), anchor, 2.0);
        for _ in 0..200 {
            v.step(canvas(), 1.0 / 60.0);
        }
        assert!(!v.is_animating());
        assert!((v.zoom - 2.0).abs() < 1e-3);
        assert!((v.to_world(canvas(), anchor) - before).length() < 1e-2);
    }

    #[test]
    fn fit_centers_content() {
        let content = Rect::from_min_size(pos2(1000.0, 1000.0), vec2(200.0, 100.0));
        let (pan, zoom) = fit_view(vec2(800.0, 400.0), content, 20.0, 0.1, 10.0);
        let mut v = CanvasView::default();
        v.pan = pan;
        v.zoom = zoom;
        let c = v.to_screen(canvas(), content.center());
        assert!((c - canvas().center()).length() < 1e-2);
        let sr = v.rect_to_screen(canvas(), content);
        assert!(canvas().expand(0.5).contains_rect(sr));
    }

    #[test]
    fn animated_fit_converges() {
        let mut v = CanvasView::default();
        let content = Rect::from_min_size(pos2(-500.0, 300.0), vec2(300.0, 300.0));
        v.fit(canvas(), content, 30.0);
        for _ in 0..400 {
            v.step(canvas(), 1.0 / 60.0);
        }
        assert!(!v.is_animating());
        assert!((v.to_screen(canvas(), content.center()) - canvas().center()).length() < 0.5);
    }
}
