use super::common::icon_tile;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::PI;

/// One icon in a [`Dock`].
#[derive(Clone, Debug, PartialEq)]
pub struct DockItem {
    /// Glyph shown on the icon (emoji or a short string).
    pub icon: String,
    /// Tooltip label.
    pub label: String,
    /// Tile gradient; `None` uses the theme chart palette.
    pub colors: Option<[Color32; 2]>,
    /// Show a "running" dot under the icon.
    pub active: bool,
}

impl DockItem {
    /// An icon with a tooltip label.
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self { icon: icon.into(), label: label.into(), colors: None, active: false }
    }
    /// Tile gradient colors.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
    /// Show the "running" indicator dot.
    pub fn active(mut self, on: bool) -> Self {
        self.active = on;
        self
    }
}

/// Result of [`Dock::show`].
pub struct DockResponse {
    /// Response for the whole dock area.
    pub response: Response,
    /// Index of the item clicked this frame.
    pub clicked: Option<usize>,
    /// Index of the item under the pointer.
    pub hovered: Option<usize>,
}

/// Visual style of dock icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockStyle {
    /// Colorful gradient app tiles (macOS-like).
    #[default]
    Tiles,
    /// Muted circles with a foreground glyph (minimal / floating dock look).
    Minimal,
}

/// A macOS-style dock: icons magnify smoothly around the pointer, show a tooltip label and
/// bounce when clicked.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{Dock, DockItem};
/// let items = [DockItem::new("🏠", "Home"), DockItem::new("✉", "Mail").active(true)];
/// let r = Dock::new(&items).magnification(1.8).show(ui);
/// if let Some(i) = r.clicked { /* open items[i] */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Dock<'a> {
    items: &'a [DockItem],
    size: f32,
    magnification: f32,
    distance: f32,
    gap: f32,
    style: DockStyle,
    labels: bool,
    autoplay: bool,
}

impl<'a> Dock<'a> {
    /// A dock over `items`.
    pub fn new(items: &'a [DockItem]) -> Self {
        Self {
            items,
            size: 46.0,
            magnification: 1.75,
            distance: 150.0,
            gap: 10.0,
            style: DockStyle::Tiles,
            labels: true,
            autoplay: false,
        }
    }
    /// Resting icon size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(16.0);
        self
    }
    /// Size multiplier of the icon right under the pointer.
    pub fn magnification(mut self, m: f32) -> Self {
        self.magnification = m.clamp(1.0, 3.0);
        self
    }
    /// How far (points) the magnification reaches around the pointer.
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d.max(10.0);
        self
    }
    /// Gap between icons.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Icon style.
    pub fn style(mut self, s: DockStyle) -> Self {
        self.style = s;
        self
    }
    /// Show tooltip labels above hovered icons (default on).
    pub fn labels(mut self, on: bool) -> Self {
        self.labels = on;
        self
    }
    /// When the pointer is away, sweep a simulated pointer across the dock.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the dock centered in the available width.
    pub fn show(self, ui: &mut Ui) -> DockResponse {
        let t = Theme::of(ui);
        let n = self.items.len();
        let pad = 10.0;
        let big = self.size * self.magnification;
        let height = big + pad * 2.0 + 44.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
        let id = response.id;
        if n == 0 {
            return DockResponse { response, clicked: None, hovered: None };
        }
        let panel_h = self.size + pad * 2.0;
        let panel_bottom = rect.max.y - 2.0;
        let base_w = n as f32 * self.size + (n as f32 - 1.0) * self.gap + pad * 2.0;
        let base_left = rect.center().x - base_w / 2.0;
        let base_center = |i: usize| base_left + pad + self.size / 2.0 + i as f32 * (self.size + self.gap);
        let hot_zone = Rect::from_min_max(
            pos2(base_left - 20.0, panel_bottom - big - pad - 8.0),
            pos2(base_left + base_w + 20.0, panel_bottom),
        );

        let real =
            ui.input(|i| i.pointer.hover_pos()).filter(|p| hot_zone.contains(*p) && ui.rect_contains_pointer(hot_zone));
        let now = glowkit_core::time(ui);
        let mut pointer_x = real.map(|p| p.x);
        let mut sim_click = None;
        if pointer_x.is_none() && self.autoplay {
            let phase = now * 0.55;
            let sweep = phase.sin();
            // rest between sweeps so both states show
            if (phase / PI).floor() as i64 % 3 != 2 {
                pointer_x = Some(rect.center().x + sweep * base_w * 0.5);
            }
            let k = (now / 2.6).floor();
            let prev: f32 = glowkit_core::state(ui, id.with("auto_k"), || k);
            if prev != k {
                if let Some(x) = pointer_x {
                    sim_click =
                        (0..n).min_by(|a, b| (base_center(*a) - x).abs().total_cmp(&(base_center(*b) - x).abs()));
                }
            }
            glowkit_core::set_state(ui, id.with("auto_k"), k);
            glowkit_core::animate(ui);
        }

        // target sizes from the resting layout (stable, no feedback)
        let sizes: Vec<f32> = (0..n)
            .map(|i| {
                let target = match pointer_x {
                    Some(x) => {
                        let d = ((base_center(i) - x).abs() / self.distance).min(1.0);
                        self.size + (big - self.size) * ((PI * d).cos() + 1.0) / 2.0
                    }
                    None => self.size,
                };
                glowkit_core::anim::spring(ui, id.with(("sz", i)), target, 320.0, 24.0)
            })
            .collect();
        let total_w = sizes.iter().sum::<f32>() + (n as f32 - 1.0) * self.gap + pad * 2.0;
        let left = rect.center().x - total_w / 2.0;
        // items sit on a shared baseline and grow upward; the bar always spans them plus padding
        let mut x = left + pad;
        let mut item_rects = Vec::with_capacity(n);
        for s in &sizes {
            item_rects.push(Rect::from_min_size(pos2(x, panel_bottom - pad - s), Vec2::splat(*s)));
            x += s + self.gap;
        }
        let items_top = item_rects.iter().map(|r| r.min.y).fold(panel_bottom - pad - self.size, f32::min);
        let panel = Rect::from_min_max(
            pos2(left, (items_top - pad).min(panel_bottom - panel_h)),
            pos2(left + total_w, panel_bottom),
        );
        let hovered = real
            .and_then(|p| item_rects.iter().position(|r| r.expand2(vec2(self.gap / 2.0, pad)).contains(p)))
            .or_else(|| {
                if real.is_none() {
                    pointer_x.and_then(|px| {
                        item_rects.iter().position(|r| px >= r.min.x - self.gap / 2.0 && px <= r.max.x + self.gap / 2.0)
                    })
                } else {
                    None
                }
            });
        let mut clicked = None;
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                clicked = item_rects.iter().position(|r| r.expand2(vec2(self.gap / 2.0, pad)).contains(p));
            }
        }
        if let Some(i) = clicked.or(sim_click) {
            glowkit_core::set_state(ui, id.with(("bounce", i)), now);
        }
        if hovered.is_some() && real.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if !ui.is_rect_visible(rect) {
            return DockResponse { response, clicked, hovered };
        }
        let painter = ui.painter_at(rect.expand(24.0));
        // panel
        let radius = (panel_h * 0.32).min(22.0);
        let sc = if t.dark { Color32::from_black_alpha(150) } else { Color32::from_rgba_unmultiplied(30, 30, 70, 40) };
        paint::soft_shadow(&painter, panel, radius, 24.0, vec2(0.0, 8.0), sc);
        let fill = if t.dark { with_alpha(shade(t.popover, 0.02), 0.92) } else { with_alpha(t.card, 0.95) };
        painter.rect(panel, CornerRadius::same(radius as u8), fill, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // inner top highlight
        let hl = Rect::from_min_max(panel.min + vec2(radius, 1.0), pos2(panel.max.x - radius, panel.min.y + 1.6));
        painter.rect_filled(hl, CornerRadius::ZERO, with_alpha(Color32::WHITE, if t.dark { 0.06 } else { 0.5 }));

        for (i, (item, r)) in self.items.iter().zip(&item_rects).enumerate() {
            let t0: f32 = glowkit_core::state(ui, id.with(("bounce", i)), || -10.0);
            let bt = now - t0;
            let bounce = if bt < 1.4 {
                ui.ctx().request_repaint();
                -((bt * PI * 2.6).sin().abs()) * 20.0 * (-bt * 2.6).exp()
            } else {
                0.0
            };
            let r = r.translate(vec2(0.0, bounce));
            let colors = item.colors.unwrap_or_else(|| {
                let a = t.chart[i % t.chart.len()];
                [shade(a, 0.08), shade(a, -0.12)]
            });
            match self.style {
                DockStyle::Tiles => {
                    paint::soft_shadow(
                        &painter,
                        r,
                        r.width() * 0.24,
                        10.0,
                        vec2(0.0, 3.0),
                        with_alpha(Color32::BLACK, if t.dark { 0.35 } else { 0.15 }),
                    );
                    icon_tile(&painter, r, r.width() * 0.24, colors, &item.icon, 1.0);
                }
                DockStyle::Minimal => {
                    let hov = glowkit_core::anim::follow(
                        ui,
                        id.with(("hv", i)),
                        if hovered == Some(i) { 1.0 } else { 0.0 },
                        14.0,
                    );
                    let bg = mix(t.muted, t.secondary, hov);
                    painter.circle(r.center(), r.width() / 2.0, bg, Stroke::new(1.0, t.border));
                    painter.text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        &item.icon,
                        FontId::proportional(r.height() * 0.42),
                        mix(t.muted_foreground, t.foreground, 0.5 + hov * 0.5),
                    );
                }
            }
            if item.active {
                painter.circle_filled(pos2(r.center().x, panel.max.y - 3.5), 2.0, t.foreground.gamma_multiply(0.75));
            }
            if self.labels {
                let show = glowkit_core::anim::spring(
                    ui,
                    id.with(("tip", i)),
                    if hovered == Some(i) { 1.0 } else { 0.0 },
                    380.0,
                    26.0,
                );
                if show > 0.02 {
                    let a = show.clamp(0.0, 1.0);
                    let font = FontId::proportional(12.5);
                    let g = painter.layout_no_wrap(item.label.clone(), font, t.popover);
                    let size = g.size() + vec2(18.0, 10.0);
                    let c = pos2(r.center().x, r.min.y - 12.0 - size.y / 2.0 + (1.0 - show) * 8.0);
                    let tip = Rect::from_center_size(c, size * (0.85 + 0.15 * show));
                    painter.rect(
                        tip,
                        CornerRadius::same(7),
                        with_alpha(t.foreground, a * 0.95),
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                    painter.text(
                        tip.center(),
                        Align2::CENTER_CENTER,
                        &item.label,
                        FontId::proportional(12.5),
                        with_alpha(t.background, a),
                    );
                }
            }
        }
        DockResponse { response, clicked, hovered }
    }
}
