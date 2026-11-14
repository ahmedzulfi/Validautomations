//! Shared building blocks for the marketing sections and pages: the section frame (centered
//! container, background, entrance timing), typography, cards, responsive grids and small
//! painted illustrations.
//!
//! The public items here ([`SectionResponse`], [`SectionBackground`], [`SectionAlign`],
//! [`SectionCtaStyle`]) are shared by every section builder.

use egui::epaint::{RectShape, Shadow};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Layout, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};
use glowkit_components::{Button, ButtonVariant, ControlSize, UiIcon};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Easing, Theme};
use std::sync::Arc;

/// Widest a section's content gets; wider screens get centered content with side margins.
pub const SECTION_MAX_WIDTH: f32 = 1100.0;
/// Below this content width, multi-column layouts stack into one column.
pub const SECTION_STACK_BELOW: f32 = 720.0;

// ---------------------------------------------------------------------------------------------
// Public shared types

/// What a section (or page) reports back after it was shown.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::HeroSection;
/// let r = HeroSection::new("Ship faster").primary_cta("Get started").show(ui);
/// if r.clicked("Get started") { /* navigate */ }
/// ```
#[derive(Clone, Debug)]
pub struct SectionResponse {
    /// The response covering the whole section.
    pub response: Response,
    /// Label of the call-to-action, link or item that was clicked this frame, if any.
    pub clicked: Option<String>,
    /// Value submitted by a form in the section this frame (an email address, a message...).
    pub submitted: Option<String>,
}

impl SectionResponse {
    /// True if the CTA / link with this label was clicked this frame.
    pub fn clicked(&self, label: &str) -> bool {
        self.clicked.as_deref() == Some(label)
    }

    /// The label of whatever was clicked this frame.
    pub fn clicked_label(&self) -> Option<&str> {
        self.clicked.as_deref()
    }
}

/// Background painted behind a section. Animated variants come from `glowkit-backgrounds`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SectionBackground {
    /// The theme's page background color.
    #[default]
    Plain,
    /// Paint nothing: the section sits on whatever is behind it.
    Transparent,
    /// A slightly tinted band, good for alternating sections.
    Muted,
    /// Soft colored glows from the theme gradient at the top of the section.
    Glow,
    /// Animated aurora curtains.
    Aurora,
    /// A faded line grid with a few highlighted cells.
    Grid,
    /// A faded dot grid.
    Dots,
    /// Drifting, linked particles.
    Particles,
    /// A perspective synthwave floor.
    RetroGrid,
    /// Falling meteors.
    Meteors,
    /// Light beams sweeping through curved lines.
    Beams,
    /// A diagonal wash of the theme gradient (for banners and CTAs).
    Gradient,
}

impl SectionBackground {
    /// All variants, for pickers and demos.
    pub const ALL: [SectionBackground; 12] = [
        Self::Plain,
        Self::Transparent,
        Self::Muted,
        Self::Glow,
        Self::Aurora,
        Self::Grid,
        Self::Dots,
        Self::Particles,
        Self::RetroGrid,
        Self::Meteors,
        Self::Beams,
        Self::Gradient,
    ];
}

/// Horizontal alignment of a section's heading and copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SectionAlign {
    /// Left-aligned copy.
    Left,
    /// Centered copy (the default for most sections).
    #[default]
    Center,
}

impl SectionAlign {
    pub(crate) fn align(self) -> Align {
        match self {
            SectionAlign::Left => Align::Min,
            SectionAlign::Center => Align::Center,
        }
    }
}

/// Look of a call-to-action button inside a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SectionCtaStyle {
    /// Solid primary button.
    #[default]
    Primary,
    /// Muted secondary button.
    Secondary,
    /// Bordered button.
    Outline,
    /// Text-only button.
    Ghost,
    /// The animated shimmer button from `glowkit-effects`.
    Shimmer,
}

// ---------------------------------------------------------------------------------------------
// Typography

/// A block of text with the extras egui's labels lack: fake-bold weight, gradient fill, line
/// height, letter spacing and alignment inside the available width.
#[derive(Clone)]
pub(crate) struct Tx {
    text: String,
    size: f32,
    color: Option<Color32>,
    weight: f32,
    align: Align,
    max_w: Option<f32>,
    gradient: Option<Vec<Color32>>,
    line: Option<f32>,
    tracking: f32,
    mono: bool,
    sense: bool,
}

impl Tx {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 15.0,
            color: None,
            weight: 0.0,
            align: Align::Min,
            max_w: None,
            gradient: None,
            line: None,
            tracking: 0.0,
            mono: false,
            sense: false,
        }
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fake bold (overdraw) scaled to the font size.
    pub fn bold(mut self) -> Self {
        self.weight = (self.size * 0.035).clamp(0.6, 2.4);
        self
    }
    /// Semi-bold (lighter overdraw).
    pub fn semi(mut self) -> Self {
        self.weight = (self.size * 0.04).clamp(0.6, 1.0);
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_w = Some(w);
        self
    }
    pub fn gradient(mut self, c: &[Color32]) -> Self {
        self.gradient = Some(c.to_vec());
        self
    }
    /// Line height as a multiple of the font size.
    pub fn line(mut self, k: f32) -> Self {
        self.line = Some(k);
        self
    }
    pub fn tracking(mut self, px: f32) -> Self {
        self.tracking = px;
        self
    }
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    pub fn galley(&self, ui: &Ui, wrap: f32) -> Arc<Galley> {
        let font = if self.mono { FontId::monospace(self.size) } else { FontId::proportional(self.size) };
        let fmt = TextFormat {
            font_id: font,
            color: Color32::PLACEHOLDER,
            line_height: self.line.map(|k| (self.size * k).round()),
            extra_letter_spacing: self.tracking,
            ..Default::default()
        };
        let mut job = LayoutJob::single_section(self.text.clone(), fmt);
        job.wrap.max_width = wrap.max(10.0);
        job.halign = self.align;
        ui.painter().layout_job(job)
    }

    /// Paint a galley made by [`Tx::galley`] so its box starts at `min` and is `w` wide.
    pub fn paint(&self, ui: &Ui, galley: &Arc<Galley>, min: Pos2, w: f32, fallback: Color32) {
        let painter = ui.painter();
        let anchor_x = match self.align {
            Align::Min => min.x,
            Align::Center => min.x + w / 2.0,
            Align::Max => min.x + w,
        };
        let pos = pos2(anchor_x, min.y);
        let color = self.color.unwrap_or(fallback);
        let ppp = ui.ctx().pixels_per_point();
        let px = (self.weight * ppp).round();
        let copies = px as usize;
        for i in 0..=copies {
            // whole physical pixels, so the weight is identical wherever the text lands
            let dx = i as f32 / ppp;
            let p = pos + vec2(dx, 0.0);
            if let Some(g) = &self.gradient {
                let b = galley.mesh_bounds.translate(pos.to_vec2());
                let stops = g.clone();
                paint::galley_with(painter, p, galley, move |q| {
                    let u = ((q.x - b.min.x) / b.width().max(1.0)) * 0.85 + ((q.y - b.min.y) / b.height().max(1.0)) * 0.15;
                    gradient(&stops, u)
                });
            } else {
                let c = color;
                painter.galley(p, galley.clone(), c);
            }
        }
    }

    /// Lay out in the available width (or `max_width`) and paint, allocating a full-width row.
    pub fn show(self, ui: &mut Ui) -> Response {
        let avail = ui.available_width();
        let w = self.max_w.map_or(avail, |m| m.min(avail));
        let galley = self.galley(ui, w);
        let h = galley.size().y;
        let (row, _) = ui.allocate_exact_size(vec2(avail, h), Sense::hover());
        let x0 = match self.align {
            Align::Min => row.min.x,
            Align::Center => row.center().x - w / 2.0,
            Align::Max => row.max.x - w,
        };
        let text_rect = {
            let gw = galley.size().x.min(w);
            let gx = match self.align {
                Align::Min => x0,
                Align::Center => x0 + (w - gw) / 2.0,
                Align::Max => x0 + w - gw,
            };
            Rect::from_min_size(pos2(gx, row.min.y), vec2(gw, h))
        };
        let resp = ui.interact(text_rect, ui.scope_id().with(("tx", row.min.y as i32, row.min.x as i32)), if self.sense { Sense::click() } else { Sense::hover() });
        if ui.is_rect_visible(row) {
            let t = Theme::of(ui);
            self.paint(ui, &galley, pos2(x0, row.min.y), w, t.foreground);
        }
        resp
    }

    /// Allocate exactly the text's size (for horizontal layouts) and paint it.
    pub fn show_inline(self, ui: &mut Ui) -> Response {
        let wrap = self.max_w.unwrap_or(f32::INFINITY);
        let me = Self { align: Align::Min, ..self };
        let galley = me.galley(ui, wrap);
        let (rect, resp) = ui.allocate_exact_size(galley.size(), if me.sense { Sense::click() } else { Sense::hover() });
        if ui.is_rect_visible(rect) {
            let t = Theme::of(ui);
            me.paint(ui, &galley, rect.min, rect.width(), t.foreground);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Section frame

/// Per-section context handed to the content closure.
pub(crate) struct Sec {
    pub t: Theme,
    pub id: Id,
    /// Content width.
    pub w: f32,
    /// Content is narrower than [`SECTION_STACK_BELOW`]: stack columns.
    pub narrow: bool,
    pub clicked: Option<String>,
    pub submitted: Option<String>,
    /// Seconds since the section first became visible.
    pub elapsed: f32,
    /// The section has been on screen at least once.
    pub seen: bool,
    /// Horizontal extent of the whole (full-bleed) section.
    pub full_x: egui::Rangef,
}

impl Sec {
    /// Entrance progress (0..1, eased) of the `i`-th staggered element.
    pub fn reveal(&self, i: usize) -> f32 {
        glowkit_core::ease(Easing::OutCubic, glowkit_core::anim::stagger(self.elapsed - 0.05, i, 0.08, 0.7))
    }

    /// Run `add` faded and lifted by the entrance animation of element `i`.
    pub fn fade<R>(&self, ui: &mut Ui, i: usize, add: impl FnOnce(&mut Ui) -> R) -> R {
        fade_up(ui, self.reveal(i), 14.0, add)
    }

    /// Record a click.
    pub fn click(&mut self, label: impl Into<String>) {
        self.clicked = Some(label.into());
    }

    /// The column count for a grid of items at least `min_w` wide.
    pub fn cols(&self, min_w: f32, max: usize) -> usize {
        cols_for(self.w, min_w, 24.0, max)
    }
}

/// A CTA button in the given style.
pub(crate) fn cta_button(ui: &mut Ui, label: &str, style: SectionCtaStyle, large: bool) -> Response {
    let size = if large { ControlSize::Lg } else { ControlSize::Md };
    match style {
        SectionCtaStyle::Shimmer => ui.add(glowkit_effects::ShimmerButton::new(label).size(if large { 14.5 } else { 13.5 })),
        _ => {
            let v = match style {
                SectionCtaStyle::Primary => ButtonVariant::Default,
                SectionCtaStyle::Secondary => ButtonVariant::Secondary,
                SectionCtaStyle::Outline => ButtonVariant::Outline,
                _ => ButtonVariant::Ghost,
            };
            let mut b = Button::new(label).variant(v).size(size);
            if style == SectionCtaStyle::Primary {
                b = b.trailing_icon(UiIcon::ArrowRight);
            }
            ui.add(b)
        }
    }
}

/// Number of columns that fit `w` with at least `min_w` per column.
pub(crate) fn cols_for(w: f32, min_w: f32, gap: f32, max: usize) -> usize {
    (((w + gap) / (min_w + gap)).floor() as usize).clamp(1, max.max(1))
}

/// Options for [`section`].
pub(crate) struct Frame {
    pub id: Id,
    pub bg: SectionBackground,
    pub pad_y: f32,
    pub max_w: f32,
    pub min_h: f32,
    pub divider: bool,
}

impl Frame {
    pub fn new(id: Id, bg: SectionBackground) -> Self {
        Self { id, bg, pad_y: 88.0, max_w: SECTION_MAX_WIDTH, min_h: 0.0, divider: false }
    }
    pub fn pad(mut self, y: f32) -> Self {
        self.pad_y = y;
        self
    }
    pub fn max_w(mut self, w: f32) -> Self {
        self.max_w = w;
        self
    }
    pub fn min_h(mut self, h: f32) -> Self {
        self.min_h = h;
        self
    }
    pub fn divider(mut self, d: bool) -> Self {
        self.divider = d;
        self
    }
}

/// Lay out a full-width section: background (sized from last frame's height), centered content
/// column, entrance clock, and the [`SectionResponse`].
pub(crate) fn section(ui: &mut Ui, f: Frame, add: impl FnOnce(&mut Ui, &mut Sec)) -> SectionResponse {
    let t = Theme::of(ui);
    let full_w = ui.available_width();
    let origin = ui.cursor().min;
    let narrow_screen = full_w < 640.0;
    let gutter = if narrow_screen { 16.0 } else { 32.0 };
    let pad_y = if narrow_screen { (f.pad_y * 0.7).round() } else { f.pad_y };
    let prev_h: f32 = glowkit_core::state(ui, f.id.with("h"), || f.min_h.max(320.0));
    let bg_rect = Rect::from_min_size(origin, vec2(full_w, prev_h));
    paint_background(ui, f.bg, bg_rect, f.id, &t);
    if f.divider {
        ui.painter().hline(bg_rect.x_range(), bg_rect.min.y + 0.5, Stroke::new(1.0, t.border));
    }

    // entrance clock starts when the section is first on screen
    let visible = ui.is_rect_visible(bg_rect.shrink2(vec2(0.0, (bg_rect.height() * 0.2).min(80.0))));
    let now = ui.input(|i| i.time);
    let start: Option<f64> = glowkit_core::state(ui, f.id.with("seen"), || None);
    let seen = start.is_some() || visible;
    let start = match start {
        Some(s) => s,
        None if visible => {
            glowkit_core::set_state(ui, f.id.with("seen"), Some(now));
            now
        }
        None => now,
    };
    let elapsed = (now - start) as f32;
    if elapsed < 2.0 {
        ui.ctx().request_repaint();
    }

    let cw = (full_w - gutter * 2.0).min(f.max_w).max(10.0);
    let cx = origin.x + full_w / 2.0;
    let content_rect = Rect::from_min_size(pos2(cx - cw / 2.0, origin.y + pad_y), vec2(cw, f32::INFINITY));
    let mut sec = Sec { t: t.clone(), id: f.id, w: cw, narrow: cw < SECTION_STACK_BELOW, clicked: None, submitted: None, elapsed, seen, full_x: bg_rect.x_range() };
    let mut child = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(Layout::top_down(Align::Min)).id_salt(f.id));
    // pages stack sections with zero spacing; content always gets the 8pt scale back
    child.spacing_mut().item_spacing = vec2(8.0, 8.0);
    add(&mut child, &mut sec);
    let used = child.min_rect();
    let mut h = (used.max.y - origin.y + pad_y).max(f.min_h).max(1.0);
    if !h.is_finite() {
        h = prev_h;
    }
    if (h - prev_h).abs() > 0.5 {
        glowkit_core::set_state(ui, f.id.with("h"), h);
        ui.ctx().request_repaint();
    }
    let rect = Rect::from_min_size(origin, vec2(full_w, h));
    let response = ui.allocate_rect(rect, Sense::hover());
    SectionResponse { response, clicked: sec.clicked, submitted: sec.submitted }
}

/// Paint one of the [`SectionBackground`]s into `rect`.
pub(crate) fn paint_background(ui: &Ui, bg: SectionBackground, rect: Rect, id: Id, t: &Theme) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    let base = t.background;
    let fade = |ui: &Ui| paint::fade_edges_vertical(&ui.painter_at(rect), rect, (rect.height() * 0.18).clamp(24.0, 120.0), base);
    match bg {
        SectionBackground::Transparent => {}
        SectionBackground::Plain => {
            painter.rect_filled(rect, 0.0, base);
        }
        SectionBackground::Muted => {
            painter.rect_filled(rect, 0.0, muted_band(t));
        }
        SectionBackground::Glow => {
            painter.rect_filled(rect, 0.0, base);
            let k = if t.dark { 0.22 } else { 0.12 };
            let c = rect.center_top();
            let r = vec2(rect.width().min(900.0) * 0.55, rect.height().min(520.0) * 0.55);
            paint::glow_ellipse(&painter, c + vec2(-r.x * 0.35, 0.0), r, with_alpha(t.gradient[0], k));
            paint::glow_ellipse(&painter, c + vec2(r.x * 0.4, -10.0), r * 0.8, with_alpha(t.gradient[1], k * 0.8));
        }
        SectionBackground::Aurora => {
            glowkit_backgrounds::Aurora::new().id_salt(id).background(base).blend(0.6).paint(ui, rect);
            // a soft scrim keeps copy readable over the brightest curtains
            paint::glow_ellipse(&painter, rect.center(), vec2(rect.width() * 0.42, rect.height() * 0.42), with_alpha(base, if t.dark { 0.7 } else { 0.75 }));
            fade(ui);
        }
        SectionBackground::Grid => {
            glowkit_backgrounds::GridPattern::new().id_salt(id).background(base).line_color(with_alpha(t.foreground, if t.dark { 0.07 } else { 0.06 })).cell_size(44.0).squares(14).mask(true).paint(ui, rect);
        }
        SectionBackground::Dots => {
            glowkit_backgrounds::DotGrid::new().id_salt(id).background(base).gap(22.0).dot_size(1.6).auto_hover(false).paint(ui, rect);
            fade(ui);
        }
        SectionBackground::Particles => {
            glowkit_backgrounds::Particles::new().id_salt(id).background(base).count(70).links(true).paint(ui, rect);
            fade(ui);
        }
        SectionBackground::RetroGrid => {
            glowkit_backgrounds::RetroGrid::new().id_salt(id).background(base).horizon(0.55).paint(ui, rect);
            paint::fade_edges_vertical(&painter, rect, (rect.height() * 0.12).clamp(20.0, 80.0), base);
        }
        SectionBackground::Meteors => {
            painter.rect_filled(rect, 0.0, base);
            paint::glow_ellipse(&painter, rect.center_top(), vec2(rect.width() * 0.4, rect.height() * 0.5), with_alpha(t.gradient[0], if t.dark { 0.14 } else { 0.07 }));
            glowkit_backgrounds::Meteors::new().id_salt(id).transparent().count(14).paint(ui, rect);
        }
        SectionBackground::Beams => {
            glowkit_backgrounds::BackgroundBeams::new().id_salt(id).background(base).paint(ui, rect);
            fade(ui);
        }
        SectionBackground::Gradient => {
            painter.rect_filled(rect, 0.0, base);
            let a = if t.dark { 0.28 } else { 0.16 };
            let stops = [with_alpha(t.gradient[0], a), with_alpha(t.gradient[1], a * 0.7), with_alpha(t.gradient[2], a * 0.5)];
            paint::gradient_rect(&painter, rect, 0.0, 20.0, &stops);
            fade(ui);
        }
    }
}

/// The tinted band color used by [`SectionBackground::Muted`].
pub(crate) fn muted_band(t: &Theme) -> Color32 {
    if t.dark {
        mix(t.background, t.muted, 0.45)
    } else {
        mix(t.background, t.muted, 0.7)
    }
}

/// Run `add` with opacity `p` and shifted down by `(1 - p) * dist` (shapes only; layout is not
/// affected, so nothing jumps).
pub(crate) fn fade_up<R>(ui: &mut Ui, p: f32, dist: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layer = ui.layer_id();
    let start = ui.ctx().graphics_mut(|g| g.entry(layer).next_idx());
    let r = ui
        .scope(|ui| {
            if p < 1.0 {
                ui.multiply_opacity(p.clamp(0.0, 1.0));
            }
            add(ui)
        })
        .inner;
    if p < 1.0 {
        let end = ui.ctx().graphics_mut(|g| g.entry(layer).next_idx());
        let dy = (1.0 - p) * dist;
        ui.ctx().graphics_mut(|g| g.entry(layer).transform_range(start, end, egui::emath::TSTransform::from_translation(vec2(0.0, dy))));
    }
    r
}

// ---------------------------------------------------------------------------------------------
// Layout helpers

/// Lay out `add` as a horizontal row aligned within the available width. The row's width is
/// measured on the previous frame (so centering converges after one frame).
pub(crate) fn row<R>(ui: &mut Ui, id: Id, align: Align, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_width();
    let w: f32 = glowkit_core::state(ui, id.with("roww"), || 0.0);
    let x0 = ui.cursor().min.x;
    let x = match align {
        Align::Min => x0,
        Align::Center => x0 + ((avail - w) / 2.0).max(0.0),
        Align::Max => x0 + (avail - w).max(0.0),
    };
    let rect = Rect::from_min_size(pos2(x, ui.cursor().min.y), vec2(avail - (x - x0), f32::INFINITY));
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).id_salt(id));
    let r = child
        .horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            add(ui)
        })
        .inner;
    let used = child.min_rect();
    if (used.width() - w).abs() > 0.5 {
        glowkit_core::set_state(ui, id.with("roww"), used.width());
        ui.ctx().request_repaint();
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(pos2(x0, used.min.y), pos2(x0 + avail, used.max.y)));
    r
}

/// Lay out items of known widths in lines that wrap, each line aligned on its own (so a
/// centered row of logos stays centered when it wraps). `add(ui, i)` must use `widths[i]`.
pub(crate) fn wrap_rows(ui: &mut Ui, widths: &[f32], h: f32, gap: f32, line_gap: f32, align: Align, mut add: impl FnMut(&mut Ui, usize)) {
    let avail = ui.available_width();
    let x0 = ui.cursor().min.x;
    let mut y = ui.cursor().min.y;
    let mut i = 0;
    let n = widths.len();
    while i < n {
        let mut j = i;
        let mut w = 0.0;
        while j < n && (j == i || w + gap + widths[j] <= avail) {
            w += if j == i { widths[j] } else { gap + widths[j] };
            j += 1;
        }
        let x = match align {
            Align::Min => x0,
            Align::Center => x0 + ((avail - w) / 2.0).max(0.0),
            Align::Max => x0 + (avail - w).max(0.0),
        };
        let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(pos2(x, y), vec2(w + 1.0, h))).layout(Layout::left_to_right(Align::Center)).id_salt(("wrap", i)));
        child.spacing_mut().item_spacing.x = gap;
        for k in i..j {
            add(&mut child, k);
        }
        y += h + if j < n { line_gap } else { 0.0 };
        i = j;
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(pos2(x0, ui.cursor().min.y), pos2(x0 + avail, y)));
}

/// A responsive grid: `n` cells in `cols` columns, rows as tall as their tallest cell.
/// `add(ui, index, row_height)` draws one cell; cards can paint themselves `row_height` tall
/// (measured last frame) so every card in a row lines up.
pub(crate) fn grid(ui: &mut Ui, id: Id, n: usize, cols: usize, gap: f32, mut add: impl FnMut(&mut Ui, usize, f32)) {
    let cols = cols.max(1);
    let w = ui.available_width();
    let cw = ((w - gap * (cols as f32 - 1.0)) / cols as f32).max(10.0);
    let x0 = ui.cursor().min.x;
    let mut y = ui.cursor().min.y;
    let rows = n.div_ceil(cols);
    for r in 0..rows {
        let key = id.with(("gridrow", r, cols));
        let prev: f32 = glowkit_core::state(ui, key, || 0.0);
        let mut max_h: f32 = 0.0;
        for c in 0..cols {
            let i = r * cols + c;
            if i >= n {
                break;
            }
            let rect = Rect::from_min_size(pos2(x0 + c as f32 * (cw + gap), y), vec2(cw, f32::INFINITY));
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).id_salt(("cell", i)));
            add(&mut child, i, prev);
            max_h = max_h.max(child.min_rect().height());
        }
        if (max_h - prev).abs() > 0.5 {
            glowkit_core::set_state(ui, key, max_h);
            ui.ctx().request_repaint();
        }
        let h = max_h.max(prev.min(max_h + 400.0));
        let h = if prev > 0.0 { prev.max(max_h) } else { h };
        y += h + if r + 1 < rows { gap } else { 0.0 };
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(pos2(x0, ui.cursor().min.y), pos2(x0 + w, y)));
}

/// Two columns side by side (`ratio` = left share), stacked when `stack` is set. When `vcenter`,
/// the shorter column is vertically centered against the taller one (measured last frame).
pub(crate) fn two_col(
    ui: &mut Ui,
    id: Id,
    ratio: f32,
    gap: f32,
    stack: bool,
    vcenter: bool,
    left: impl FnOnce(&mut Ui),
    right: impl FnOnce(&mut Ui),
) {
    if stack {
        ui.scope(left);
        ui.add_space(gap * 0.75);
        ui.scope(right);
        return;
    }
    let w = ui.available_width();
    let lw = ((w - gap) * ratio).round();
    let rw = w - gap - lw;
    let top = ui.cursor().min;
    let (hl, hr): (f32, f32) = glowkit_core::state(ui, id.with("2col"), || (0.0, 0.0));
    let tall = hl.max(hr);
    let off = |h: f32| if vcenter && h > 0.0 { ((tall - h) / 2.0).max(0.0) } else { 0.0 };
    let mut l = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(top + vec2(0.0, off(hl)), vec2(lw, f32::INFINITY))).layout(Layout::top_down(Align::Min)).id_salt("l"));
    left(&mut l);
    let mut r = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(top + vec2(lw + gap, off(hr)), vec2(rw, f32::INFINITY))).layout(Layout::top_down(Align::Min)).id_salt("r"));
    right(&mut r);
    let nh = (l.min_rect().height(), r.min_rect().height());
    if (nh.0 - hl).abs() > 0.5 || (nh.1 - hr).abs() > 0.5 {
        glowkit_core::set_state(ui, id.with("2col"), nh);
        ui.ctx().request_repaint();
    }
    let bottom = l.min_rect().max.y.max(r.min_rect().max.y);
    ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + w, bottom)));
}

// ---------------------------------------------------------------------------------------------
// Cards

/// Look of a [`card`].
#[derive(Clone, Copy)]
pub(crate) struct CardLook {
    pub pad: f32,
    pub radius: f32,
    pub fill: Color32,
    pub stroke: Color32,
    pub hover: bool,
    pub min_h: f32,
    pub shadow: bool,
    pub clickable: bool,
}

impl CardLook {
    pub fn of(t: &Theme) -> Self {
        Self {
            pad: 24.0,
            radius: (t.radius * 1.4).round(),
            fill: t.card,
            stroke: t.border,
            hover: true,
            min_h: 0.0,
            shadow: true,
            clickable: false,
        }
    }
    pub fn pad(mut self, p: f32) -> Self {
        self.pad = p;
        self
    }
    pub fn min_h(mut self, h: f32) -> Self {
        self.min_h = h;
        self
    }
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = c;
        self
    }
    pub fn stroke(mut self, c: Color32) -> Self {
        self.stroke = c;
        self
    }
    pub fn hover(mut self, h: bool) -> Self {
        self.hover = h;
        self
    }
    pub fn clickable(mut self, c: bool) -> Self {
        self.clickable = c;
        self
    }
    pub fn shadow(mut self, s: bool) -> Self {
        self.shadow = s;
        self
    }
}

/// Result of [`card`].
pub(crate) struct CardOut<R> {
    pub inner: R,
    pub rect: Rect,
    pub response: Response,
    pub hover: f32,
}

/// A themed card with hover lift: allocates only its natural height but paints at least
/// `look.min_h` tall (so grid rows can equalize).
pub(crate) fn card<R>(ui: &mut Ui, id: Id, look: CardLook, add: impl FnOnce(&mut Ui) -> R) -> CardOut<R> {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let origin = ui.cursor().min;
    let layer = ui.layer_id();
    let start = ui.ctx().graphics_mut(|g| g.entry(layer).next_idx());
    let painter = ui.painter().clone();
    let i_shadow = painter.add(Shape::Noop);
    let i_bg = painter.add(Shape::Noop);
    let inner_rect = Rect::from_min_size(origin + Vec2::splat(look.pad), vec2(w - look.pad * 2.0, f32::INFINITY));
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner_rect).layout(Layout::top_down(Align::Min)).id_salt(id));
    let inner = add(&mut child);
    let natural = child.min_rect().height() + look.pad * 2.0;
    let rect = Rect::from_min_size(origin, vec2(w, natural.max(look.min_h)));
    let alloc = Rect::from_min_size(origin, vec2(w, natural));
    ui.advance_cursor_after_rect(alloc);
    let sense = if look.clickable { Sense::click() } else { Sense::hover() };
    let mut response = ui.interact(rect, id.with("card"), sense);
    if look.clickable {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }
    let hover = if look.hover { ui.ctx().animate_bool_with_time(id.with("hov"), response.hovered(), 0.18) } else { 0.0 };
    let cr = CornerRadius::same(look.radius.round().clamp(0.0, 255.0) as u8);
    if look.shadow {
        let a = if t.dark { 0.35 + 0.25 * hover } else { 0.06 + 0.08 * hover };
        let s = Shadow { offset: [0, (2.0 + 6.0 * hover) as i8], blur: (10.0 + 14.0 * hover) as u8, spread: 0, color: with_alpha(Color32::BLACK, a) };
        painter.set(i_shadow, s.as_shape(rect, cr));
    }
    let stroke = mix(look.stroke, t.accent, 0.45 * hover);
    let fill = if t.dark { mix(look.fill, t.foreground, 0.012 * hover) } else { look.fill };
    painter.set(i_bg, RectShape::new(rect, cr, fill, Stroke::new(1.0, stroke), StrokeKind::Inside));
    if hover > 0.0 {
        let end = ui.ctx().graphics_mut(|g| g.entry(layer).next_idx());
        ui.ctx().graphics_mut(|g| g.entry(layer).transform_range(start, end, egui::emath::TSTransform::from_translation(vec2(0.0, -2.0 * hover))));
    }
    CardOut { inner, rect, response, hover }
}

// ---------------------------------------------------------------------------------------------
// Headings

/// Eyebrow, headline and subheading for a section, staggered in. Returns the next stagger index.
pub(crate) struct Heading<'a> {
    pub eyebrow: Option<&'a str>,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub align: Align,
    pub size: f32,
    pub max_w: f32,
}

impl<'a> Heading<'a> {
    pub fn new(title: &'a str) -> Self {
        Self { eyebrow: None, title, subtitle: None, align: Align::Center, size: 36.0, max_w: 680.0 }
    }
    pub fn eyebrow(mut self, e: Option<&'a str>) -> Self {
        self.eyebrow = e.filter(|s| !s.is_empty());
        self
    }
    pub fn subtitle(mut self, s: Option<&'a str>) -> Self {
        self.subtitle = s.filter(|s| !s.is_empty());
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }

    pub fn show(self, ui: &mut Ui, sec: &Sec, first: usize) -> usize {
        let t = &sec.t;
        let mut i = first;
        let size = if sec.narrow { (self.size * 0.78).round() } else { self.size };
        if let Some(e) = self.eyebrow {
            sec.fade(ui, i, |ui| eyebrow(ui, e, self.align, t));
            ui.add_space(14.0);
            i += 1;
        }
        sec.fade(ui, i, |ui| {
            Tx::new(self.title).size(size).bold().line(1.15).align(self.align).max_width(self.max_w + 120.0).color(t.foreground).show(ui);
        });
        i += 1;
        if let Some(s) = self.subtitle {
            ui.add_space(14.0);
            sec.fade(ui, i, |ui| {
                Tx::new(s).size(if sec.narrow { 15.0 } else { 17.0 }).line(1.55).align(self.align).max_width(self.max_w).color(t.muted_foreground).show(ui);
            });
            i += 1;
        }
        i
    }
}

/// Small uppercase accent label above a headline.
pub(crate) fn eyebrow(ui: &mut Ui, text: &str, align: Align, t: &Theme) {
    let tx = Tx::new(text.to_uppercase()).size(11.5).tracking(1.6).semi().color(t.accent);
    let galley = tx.galley(ui, f32::INFINITY);
    let pad = vec2(12.0, 5.0);
    let size = galley.size() + pad * 2.0;
    let avail = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(avail, size.y), Sense::hover());
    let x = match align {
        Align::Min => row.min.x,
        Align::Center => row.center().x - size.x / 2.0,
        Align::Max => row.max.x - size.x,
    };
    let r = Rect::from_min_size(pos2(x, row.min.y), size);
    ui.painter().rect(r, CornerRadius::same(99), with_alpha(t.accent, if t.dark { 0.12 } else { 0.08 }), Stroke::new(1.0, with_alpha(t.accent, 0.3)), StrokeKind::Inside);
    tx.paint(ui, &galley, r.min + pad, galley.size().x, t.accent);
}

// ---------------------------------------------------------------------------------------------
// Small painted pieces

/// A rounded tile with a tinted fill and an icon (feature cards, steps, contact info).
pub(crate) fn icon_tile(painter: &egui::Painter, rect: Rect, icon: UiIcon, color: Color32, t: &Theme) {
    let r = CornerRadius::same((rect.width() * 0.26) as u8);
    painter.rect(rect, r, with_alpha(color, if t.dark { 0.14 } else { 0.1 }), Stroke::new(1.0, with_alpha(color, 0.28)), StrokeKind::Inside);
    icon.paint(painter, rect.shrink(rect.width() * 0.27), color);
}

/// A check mark in a small tinted circle.
pub(crate) fn check_dot(painter: &egui::Painter, center: Pos2, r: f32, color: Color32, t: &Theme) {
    painter.circle_filled(center, r, with_alpha(color, if t.dark { 0.18 } else { 0.12 }));
    UiIcon::Check.paint(painter, Rect::from_center_size(center, Vec2::splat(r * 1.3)), color);
}

/// A row: check dot + text. Returns its response.
pub(crate) fn check_row(ui: &mut Ui, text: &str, color: Color32, muted: bool) -> Response {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let tx = Tx::new(text).size(14.0).color(if muted { t.muted_foreground } else { t.foreground });
    let galley = tx.galley(ui, w - 28.0);
    let h = galley.size().y.max(20.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    check_dot(ui.painter(), pos2(rect.min.x + 9.0, rect.min.y + 10.0), 9.0, color, &t);
    tx.paint(ui, &galley, pos2(rect.min.x + 28.0, rect.min.y + (20.0 - galley.rows.first().map_or(16.0, |r| r.height())) / 2.0), w - 28.0, t.foreground);
    resp
}

/// A text link with an animated underline on hover. Returns its response.
pub(crate) fn link(ui: &mut Ui, text: &str, size: f32, color: Color32, hover_color: Color32) -> Response {
    let tx = Tx::new(text).size(size);
    let galley = tx.galley(ui, f32::INFINITY);
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let h = ui.ctx().animate_bool_with_time(resp.id.with("lnk"), resp.hovered(), 0.15);
    let c = color.lerp_to_gamma(hover_color, h);
    tx.paint(ui, &galley, rect.min, rect.width(), c);
    if h > 0.0 {
        let y = rect.max.y + 1.0;
        ui.painter().line_segment([pos2(rect.min.x, y), pos2(rect.min.x + rect.width() * h, y)], Stroke::new(1.0, with_alpha(hover_color, 0.6)));
    }
    resp
}

/// A brand mark for a fictional company: a little geometric glyph plus a wordmark.
/// Returns the painted width.
pub(crate) fn paint_logo(painter: &egui::Painter, left_center: Pos2, name: &str, kind: usize, color: Color32, h: f32) -> f32 {
    let s = h;
    let c = left_center + vec2(s / 2.0, 0.0);
    let st = Stroke::new((s * 0.11).max(1.2), color);
    match kind % 8 {
        0 => {
            painter.circle_stroke(c, s * 0.42, st);
            painter.circle_filled(c, s * 0.16, color);
        }
        1 => {
            let pts = vec![c + vec2(0.0, -s * 0.45), c + vec2(s * 0.45, s * 0.38), c + vec2(-s * 0.45, s * 0.38)];
            painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        2 => {
            for k in 0..3 {
                let x = c.x - s * 0.4 + k as f32 * s * 0.3;
                painter.rect_filled(Rect::from_min_size(pos2(x, c.y - s * 0.4 + k as f32 * s * 0.12), vec2(s * 0.2, s * 0.8 - k as f32 * s * 0.12)), 1.5, color);
            }
        }
        3 => {
            painter.rect_stroke(Rect::from_center_size(c, Vec2::splat(s * 0.78)), CornerRadius::same((s * 0.2) as u8), st, StrokeKind::Middle);
            painter.circle_filled(c, s * 0.14, color);
        }
        4 => {
            paint::star(painter, c, s * 0.48, s * 0.2, 4, 0.0, color);
        }
        5 => {
            painter.circle_filled(c + vec2(-s * 0.16, 0.0), s * 0.3, color);
            painter.circle_stroke(c + vec2(s * 0.16, 0.0), s * 0.3, st);
        }
        6 => {
            let pts: Vec<Pos2> = (0..6)
                .map(|i| {
                    let a = i as f32 / 6.0 * std::f32::consts::TAU + 0.5;
                    c + vec2(a.cos(), a.sin()) * s * 0.46
                })
                .collect();
            painter.add(Shape::closed_line(pts, st));
        }
        _ => {
            painter.line_segment([c + vec2(-s * 0.4, s * 0.35), c + vec2(0.0, -s * 0.4)], st);
            painter.line_segment([c + vec2(0.0, -s * 0.4), c + vec2(s * 0.4, s * 0.35)], st);
            painter.line_segment([c + vec2(-s * 0.2, s * 0.05), c + vec2(s * 0.2, s * 0.05)], st);
        }
    }
    let g = painter.layout_no_wrap(name.to_owned(), FontId::proportional(s * 0.92), color);
    let pos = pos2(left_center.x + s + s * 0.35, left_center.y - g.size().y / 2.0);
    painter.galley(pos, g.clone(), color);
    painter.galley(pos + vec2(0.6, 0.0), g.clone(), color.gamma_multiply(0.7));
    s * 1.35 + g.size().x + 0.6
}

/// Width [`paint_logo`] will take.
pub(crate) fn logo_width(ui: &Ui, name: &str, h: f32) -> f32 {
    let g = ui.painter().layout_no_wrap(name.to_owned(), FontId::proportional(h * 0.92), Color32::WHITE);
    h * 1.35 + g.size().x + 0.6
}

/// Social network glyphs for footers and profiles (simple, brand-neutral drawings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionSocial {
    /// A crossed "X" mark.
    X,
    /// A code-hosting cat-ish mark.
    GitHub,
    /// "in" in a rounded square.
    LinkedIn,
    /// A play button.
    YouTube,
    /// A chat bubble.
    Discord,
    /// A camera outline.
    Instagram,
}

impl SectionSocial {
    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::GitHub => "GitHub",
            Self::LinkedIn => "LinkedIn",
            Self::YouTube => "YouTube",
            Self::Discord => "Discord",
            Self::Instagram => "Instagram",
        }
    }

    /// Paint the glyph centered in `rect`.
    pub fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32) {
        let s = rect.width().min(rect.height());
        let c = rect.center();
        let k = s / 24.0;
        let st = Stroke::new((1.8 * k).max(1.1), color);
        let p = |x: f32, y: f32| pos2(c.x + (x - 12.0) * k, c.y + (y - 12.0) * k);
        match self {
            Self::X => {
                painter.line_segment([p(5.0, 4.5), p(19.0, 19.5)], Stroke::new(2.4 * k, color));
                painter.line_segment([p(19.0, 4.5), p(5.0, 19.5)], st);
            }
            Self::GitHub => {
                painter.circle_stroke(p(12.0, 12.5), 9.0 * k, st);
                painter.add(Shape::convex_polygon(vec![p(7.0, 8.0), p(8.0, 3.8), p(10.5, 6.6)], color, Stroke::NONE));
                painter.add(Shape::convex_polygon(vec![p(13.5, 6.6), p(16.0, 3.8), p(17.0, 8.0)], color, Stroke::NONE));
                painter.circle_filled(p(9.3, 12.0), 1.2 * k, color);
                painter.circle_filled(p(14.7, 12.0), 1.2 * k, color);
            }
            Self::LinkedIn => {
                painter.rect_stroke(Rect::from_min_max(p(3.5, 3.5), p(20.5, 20.5)), CornerRadius::same((4.0 * k) as u8), st, StrokeKind::Middle);
                painter.line_segment([p(8.0, 10.5), p(8.0, 16.5)], st);
                painter.circle_filled(p(8.0, 7.5), 1.2 * k, color);
                painter.line_segment([p(11.5, 10.5), p(11.5, 16.5)], st);
                let arc: Vec<Pos2> = (0..=8)
                    .map(|i| {
                        let a = std::f32::consts::PI * (1.0 + i as f32 / 8.0);
                        p(14.0 + 2.5 * a.cos(), 13.0 + 2.5 * a.sin())
                    })
                    .collect();
                painter.add(Shape::line(arc, st));
                painter.line_segment([p(16.5, 13.0), p(16.5, 16.5)], st);
            }
            Self::YouTube => {
                painter.rect_stroke(Rect::from_min_max(p(2.5, 5.5), p(21.5, 18.5)), CornerRadius::same((5.0 * k) as u8), st, StrokeKind::Middle);
                painter.add(Shape::convex_polygon(vec![p(10.0, 9.0), p(15.5, 12.0), p(10.0, 15.0)], color, Stroke::NONE));
            }
            Self::Discord => {
                painter.rect_stroke(Rect::from_min_max(p(3.0, 5.0), p(21.0, 17.5)), CornerRadius::same((6.0 * k) as u8), st, StrokeKind::Middle);
                painter.add(Shape::line(vec![p(8.0, 17.5), p(7.0, 21.0), p(11.0, 17.5)], st));
                painter.circle_filled(p(9.0, 11.2), 1.4 * k, color);
                painter.circle_filled(p(15.0, 11.2), 1.4 * k, color);
            }
            Self::Instagram => {
                painter.rect_stroke(Rect::from_min_max(p(3.5, 3.5), p(20.5, 20.5)), CornerRadius::same((5.5 * k) as u8), st, StrokeKind::Middle);
                painter.circle_stroke(p(12.0, 12.0), 4.0 * k, st);
                painter.circle_filled(p(16.8, 7.2), 1.1 * k, color);
            }
        }
    }
}

/// Round icon button showing a social glyph; returns its response.
pub(crate) fn social_button(ui: &mut Ui, s: SectionSocial, size: f32) -> Response {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(s.label());
    let h = ui.ctx().animate_bool_with_time(resp.id.with("soc"), resp.hovered(), 0.15);
    let painter = ui.painter();
    painter.rect(rect, CornerRadius::same((size * 0.3) as u8), mix(t.card, t.secondary, 0.5 + 0.5 * h), Stroke::new(1.0, mix(t.border, t.accent, 0.5 * h)), StrokeKind::Inside);
    s.paint(painter, rect.shrink(size * 0.27), mix(t.muted_foreground, t.foreground, h));
    resp
}

/// Abstract "cover image" art: soft blobs from the theme palette on a tinted ground, unique
/// per `seed`. Used where a real product would show photos.
pub(crate) fn paint_art(painter: &egui::Painter, rect: Rect, seed: usize, t: &Theme, radius: f32, time: f32) {
    let pal = t.chart;
    let a = pal[seed % 6];
    let b = pal[(seed + 2) % 6];
    let c = pal[(seed + 4) % 6];
    let ground = if t.dark { mix(t.card, a, 0.16) } else { mix(t.card, a, 0.12) };
    let cr = CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);
    painter.rect_filled(rect, cr, ground);
    let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let mut rng = glowkit_core::Rng::new(seed as u64 * 7919 + 13);
    let alpha = if t.dark { 0.55 } else { 0.42 };
    for (k, col) in [a, b, c].iter().enumerate() {
        let u = rng.f32();
        let v = rng.f32();
        let drift = vec2((time * 0.3 + k as f32 * 2.0).sin(), (time * 0.23 + k as f32).cos()) * 6.0;
        let center = pos2(rect.min.x + rect.width() * (0.15 + 0.7 * u), rect.min.y + rect.height() * (0.2 + 0.6 * v)) + drift;
        let r = rect.width().max(rect.height()) * (0.35 + 0.2 * rng.f32());
        paint::glow(&clip, center, r, with_alpha(*col, alpha));
    }
    // a light geometric overlay so covers don't all look like the same blur
    let line = with_alpha(if t.dark { Color32::WHITE } else { Color32::BLACK }, 0.06);
    match seed % 3 {
        0 => {
            let step = 22.0;
            let mut x = rect.min.x + step;
            while x < rect.max.x {
                clip.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], Stroke::new(1.0, line));
                x += step;
            }
        }
        1 => {
            let c = rect.center() + vec2(rect.width() * 0.18, 0.0);
            for k in 1..6 {
                clip.circle_stroke(c, k as f32 * rect.height() * 0.14, Stroke::new(1.0, line));
            }
        }
        _ => {
            let step = 18.0;
            let mut y = rect.min.y + step;
            while y < rect.max.y {
                let mut x = rect.min.x + step;
                while x < rect.max.x {
                    clip.circle_filled(pos2(x, y), 1.0, line);
                    x += step;
                }
                y += step;
            }
        }
    }
    painter.rect_stroke(rect, cr, Stroke::new(1.0, with_alpha(t.border, 0.6)), StrokeKind::Inside);
}

/// A product screenshot stand-in: an app window with sidebar, KPI tiles, an animated area
/// chart and a list. `kind` 0 = dashboard, 1 = code editor, 2 = chat.
pub(crate) fn paint_mockup(ui: &Ui, rect: Rect, kind: usize, t: &Theme) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let time = glowkit_core::time(ui);
    let r = (t.radius * 1.4).round();
    paint::soft_shadow(painter, rect, r, 40.0, vec2(0.0, 18.0), with_alpha(Color32::BLACK, if t.dark { 0.55 } else { 0.14 }));
    paint::glow_ellipse(painter, rect.center_top() + vec2(0.0, rect.height() * 0.1), vec2(rect.width() * 0.6, rect.height() * 0.4), with_alpha(t.gradient[0], if t.dark { 0.16 } else { 0.06 }));
    painter.rect(rect, CornerRadius::same(r as u8), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let p = painter.with_clip_rect(rect.shrink(1.0).intersect(painter.clip_rect()));
    // title bar
    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 34.0));
    p.rect_filled(bar, CornerRadius { nw: r as u8, ne: r as u8, sw: 0, se: 0 }, mix(t.card, t.muted, 0.6));
    p.hline(bar.x_range(), bar.max.y, Stroke::new(1.0, t.border));
    for (k, c) in [Color32::from_rgb(255, 95, 87), Color32::from_rgb(254, 188, 46), Color32::from_rgb(40, 200, 64)].iter().enumerate() {
        p.circle_filled(pos2(bar.min.x + 16.0 + k as f32 * 14.0, bar.center().y), 4.0, with_alpha(*c, 0.85));
    }
    let url = Rect::from_center_size(bar.center(), vec2((rect.width() * 0.36).min(220.0), 18.0));
    p.rect_filled(url, CornerRadius::same(6), with_alpha(t.foreground, 0.05));
    let body = Rect::from_min_max(pos2(rect.min.x, bar.max.y), rect.max);
    let faint = with_alpha(t.foreground, if t.dark { 0.08 } else { 0.07 });
    let faint2 = with_alpha(t.foreground, if t.dark { 0.14 } else { 0.12 });
    match kind % 3 {
        0 => {
            // sidebar
            let side = Rect::from_min_size(body.min, vec2((body.width() * 0.2).clamp(60.0, 150.0), body.height()));
            p.vline(side.max.x, side.y_range(), Stroke::new(1.0, t.border));
            for i in 0..6 {
                let y = side.min.y + 20.0 + i as f32 * 26.0;
                let active = i == 1;
                if active {
                    p.rect_filled(Rect::from_min_size(pos2(side.min.x + 8.0, y - 8.0), vec2(side.width() - 16.0, 22.0)), CornerRadius::same(6), with_alpha(t.accent, 0.14));
                }
                p.circle_filled(pos2(side.min.x + 20.0, y + 3.0), 4.0, if active { t.accent } else { faint2 });
                p.rect_filled(Rect::from_min_size(pos2(side.min.x + 32.0, y), vec2(side.width() * 0.45, 6.0)), CornerRadius::same(3), if active { with_alpha(t.accent, 0.6) } else { faint2 });
            }
            let main = Rect::from_min_max(pos2(side.max.x + 16.0, body.min.y + 16.0), body.max - vec2(16.0, 16.0));
            // KPI tiles
            let n = 3;
            let gap = 10.0;
            let tw = (main.width() - gap * (n as f32 - 1.0)) / n as f32;
            for i in 0..n {
                let tr = Rect::from_min_size(pos2(main.min.x + i as f32 * (tw + gap), main.min.y), vec2(tw, 58.0));
                p.rect(tr, CornerRadius::same(8), with_alpha(t.foreground, 0.025), Stroke::new(1.0, t.border), StrokeKind::Inside);
                p.rect_filled(Rect::from_min_size(tr.min + vec2(10.0, 12.0), vec2(tw * 0.35, 5.0)), CornerRadius::same(3), faint2);
                let vals = ["$48.2k", "2,431", "98.4%"];
                p.text(tr.min + vec2(10.0, 24.0), egui::Align2::LEFT_TOP, vals[i], FontId::proportional(15.0), t.foreground);
                let col = t.chart[i];
                let spark: Vec<Pos2> = (0..12)
                    .map(|k| {
                        let x = tr.max.x - 54.0 + k as f32 * 4.0;
                        let y = tr.center().y + 4.0 - ((k as f32 * 0.9 + i as f32 + time * 0.8).sin() * 0.5 + k as f32 * 0.06) * 10.0;
                        pos2(x, y)
                    })
                    .collect();
                p.add(Shape::line(spark, Stroke::new(1.5, col)));
            }
            // chart
            let cr = Rect::from_min_max(pos2(main.min.x, main.min.y + 72.0), pos2(main.max.x, main.min.y + 72.0 + (main.height() - 72.0) * 0.62));
            p.rect(cr, CornerRadius::same(8), with_alpha(t.foreground, 0.02), Stroke::new(1.0, t.border), StrokeKind::Inside);
            let plot = cr.shrink2(vec2(14.0, 18.0));
            for k in 0..4 {
                let y = plot.min.y + k as f32 * plot.height() / 3.0;
                p.hline(plot.x_range(), y, Stroke::new(1.0, faint));
            }
            let n = 24;
            let pts: Vec<Pos2> = (0..n)
                .map(|k| {
                    let u = k as f32 / (n - 1) as f32;
                    let v = 0.35 + 0.25 * (u * 5.0 + time * 0.6).sin() * 0.6 + 0.4 * u + 0.08 * (u * 17.0).sin();
                    pos2(plot.min.x + u * plot.width(), plot.max.y - v.clamp(0.05, 0.95) * plot.height())
                })
                .collect();
            let smooth = paint::monotone(&pts, 4);
            paint::area_fill(&p, &smooth, plot.max.y, with_alpha(t.accent, 0.35), with_alpha(t.accent, 0.0));
            p.add(Shape::line(smooth, Stroke::new(2.0, t.accent)));
            // list
            let lr = Rect::from_min_max(pos2(main.min.x, cr.max.y + 12.0), main.max);
            for i in 0..4 {
                let y = lr.min.y + 6.0 + i as f32 * 24.0;
                if y + 12.0 > lr.max.y {
                    break;
                }
                p.circle_filled(pos2(lr.min.x + 8.0, y + 4.0), 6.0, with_alpha(t.chart[(i + 1) % 6], 0.7));
                p.rect_filled(Rect::from_min_size(pos2(lr.min.x + 22.0, y), vec2(lr.width() * 0.3, 6.0)), CornerRadius::same(3), faint2);
                p.rect_filled(Rect::from_min_size(pos2(lr.max.x - 60.0, y), vec2(60.0, 6.0)), CornerRadius::same(3), faint);
            }
        }
        1 => {
            let gutter = 36.0;
            let lines: [(f32, usize, f32); 12] = [
                (0.0, 0, 0.35),
                (0.0, 1, 0.55),
                (1.0, 2, 0.4),
                (1.0, 3, 0.62),
                (2.0, 1, 0.3),
                (2.0, 4, 0.45),
                (1.0, 0, 0.2),
                (0.0, 0, 0.0),
                (0.0, 5, 0.5),
                (1.0, 2, 0.58),
                (1.0, 3, 0.36),
                (0.0, 0, 0.12),
            ];
            let typed = ((time * 0.9) % 14.0) as usize;
            for (i, (indent, col, len)) in lines.iter().enumerate() {
                let y = body.min.y + 18.0 + i as f32 * 22.0;
                if y + 10.0 > body.max.y {
                    break;
                }
                p.text(pos2(body.min.x + gutter - 10.0, y - 2.0), egui::Align2::RIGHT_TOP, format!("{}", i + 1), FontId::monospace(10.5), with_alpha(t.muted_foreground, 0.6));
                if *len <= 0.0 {
                    continue;
                }
                let x = body.min.x + gutter + 6.0 + indent * 18.0;
                let c = if *col == 0 { faint2 } else { with_alpha(t.chart[col - 1], 0.75) };
                let w = (body.width() - gutter - 30.0) * len;
                let shown = if i < typed { w } else { 0.0 };
                if shown > 0.0 {
                    p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w * 0.35, 8.0)), CornerRadius::same(3), t.accent.gamma_multiply(0.7));
                    p.rect_filled(Rect::from_min_size(pos2(x + w * 0.38, y), vec2(w * 0.62, 8.0)), CornerRadius::same(3), c);
                }
                if i == typed.min(lines.len() - 1) && (time * 2.0).fract() < 0.5 {
                    p.vline(x, (y - 3.0)..=(y + 11.0), Stroke::new(1.5, t.foreground));
                }
            }
        }
        _ => {
            let mut y = body.min.y + 18.0;
            for i in 0..4 {
                let mine = i % 2 == 1;
                let w = body.width() * [0.55, 0.42, 0.62, 0.38][i];
                let h = [44.0, 30.0, 58.0, 30.0][i];
                if y + h > body.max.y - 50.0 {
                    break;
                }
                let x = if mine { body.max.x - 16.0 - w } else { body.min.x + 44.0 };
                if !mine {
                    p.circle_filled(pos2(body.min.x + 24.0, y + 12.0), 10.0, with_alpha(t.accent, 0.8));
                }
                let fill = if mine { t.accent } else { with_alpha(t.foreground, 0.06) };
                p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, h)), CornerRadius::same(10), fill);
                let lc = if mine { with_alpha(glowkit_core::color::contrast_text(t.accent), 0.55) } else { faint2 };
                let mut ly = y + 11.0;
                while ly + 6.0 < y + h - 6.0 {
                    p.rect_filled(Rect::from_min_size(pos2(x + 12.0, ly), vec2(w - 24.0 - (ly - y) * 1.5, 6.0)), CornerRadius::same(3), lc);
                    ly += 14.0;
                }
                y += h + 12.0;
            }
            let input = Rect::from_min_max(pos2(body.min.x + 14.0, body.max.y - 46.0), pos2(body.max.x - 14.0, body.max.y - 12.0));
            p.rect(input, CornerRadius::same(10), with_alpha(t.foreground, 0.03), Stroke::new(1.0, t.border), StrokeKind::Inside);
            let dots = (time * 3.0) as usize % 4;
            for k in 0..3 {
                p.circle_filled(pos2(input.min.x + 18.0 + k as f32 * 9.0, input.center().y), 2.5, if k < dots { t.muted_foreground } else { faint2 });
            }
            p.circle_filled(pos2(input.max.x - 18.0, input.center().y), 11.0, t.accent);
            UiIcon::ArrowUp.paint(&p, Rect::from_center_size(pos2(input.max.x - 18.0, input.center().y), Vec2::splat(12.0)), glowkit_core::color::contrast_text(t.accent));
        }
    }
    glowkit_core::animate(ui);
}

/// The id salt helper every section builder uses.
pub(crate) fn section_id(ui: &Ui, kind: &str, salt: &Option<String>) -> Id {
    match salt {
        Some(s) => ui.scope_id().with((kind, s.as_str())),
        None => ui.scope_id().with(kind),
    }
}

/// A small "brand" lockup: gradient tile with the first letter + name.
pub(crate) fn brand(ui: &mut Ui, name: &str, size: f32) -> Response {
    let t = Theme::of(ui);
    let tile = size * 1.45;
    let tx = Tx::new(name).size(size).bold();
    let g = tx.galley(ui, f32::INFINITY);
    let (rect, resp) = ui.allocate_exact_size(vec2(tile + 9.0 + g.size().x + 2.0, tile.max(g.size().y)), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let painter = ui.painter();
    let tr = Rect::from_min_size(pos2(rect.min.x, rect.center().y - tile / 2.0), Vec2::splat(tile));
    paint::gradient_rect(painter, tr, tile * 0.28, 135.0, &[t.gradient[0], t.gradient[1]]);
    let letter = name.chars().next().unwrap_or('G').to_uppercase().to_string();
    let lg = painter.layout_no_wrap(letter, FontId::proportional(size * 0.95), Color32::WHITE);
    let lp = tr.center() - lg.size() / 2.0;
    painter.galley(lp, lg.clone(), Color32::WHITE);
    painter.galley(lp + vec2(0.7, 0.0), lg, Color32::WHITE);
    tx.paint(ui, &g, pos2(tr.max.x + 9.0, rect.center().y - g.size().y / 2.0), g.size().x, t.foreground);
    resp
}

/// A hairline divider with optional centered label ("or continue with").
pub(crate) fn divider(ui: &mut Ui, label: Option<&str>) {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    let y = rect.center().y;
    match label {
        Some(l) => {
            let g = ui.painter().layout_no_wrap(l.to_owned(), FontId::proportional(12.0), t.muted_foreground);
            let gw = g.size().x + 20.0;
            ui.painter().hline(rect.min.x..=rect.center().x - gw / 2.0, y, Stroke::new(1.0, t.border));
            ui.painter().hline(rect.center().x + gw / 2.0..=rect.max.x, y, Stroke::new(1.0, t.border));
            ui.painter().galley(rect.center() - g.size() / 2.0, g, t.muted_foreground);
        }
        None => {
            ui.painter().hline(rect.x_range(), y, Stroke::new(1.0, t.border));
        }
    }
}

/// Little 5-star rating row.
pub(crate) fn stars(ui: &mut Ui, n: usize, size: f32) {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(size * 5.0 + 4.0 * 4.0, size), Sense::hover());
    let gold = if t.dark { glowkit_core::hex("#FBBF24") } else { glowkit_core::hex("#F59E0B") };
    for i in 0..5 {
        let c = pos2(rect.min.x + size / 2.0 + i as f32 * (size + 4.0), rect.center().y);
        paint::star(ui.painter(), c, size * 0.5, size * 0.22, 5, -std::f32::consts::FRAC_PI_2, if i < n { gold } else { with_alpha(t.muted_foreground, 0.3) });
    }
}
