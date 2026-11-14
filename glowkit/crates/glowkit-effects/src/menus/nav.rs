//! Navigation menus: bubble menu, card nav, pill nav and the staggered side menu.

use super::common::{bold_text, burger, toggle};
use crate::buttons::util::{clip_convex, cr, fill_poly, outline};
use crate::motion::common::polygon_shadow;
use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, InnerResponse, Layout, Pos2, Rect, Response, Sense, Stroke,
    Ui, UiBuilder, Vec2, Widget,
};
use glowkit_core::anim::{ease, follow, spring, stagger, Easing};
use glowkit_core::color::{contrast_text, mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.card, 0.05)
    } else {
        Color32::WHITE
    }
}

fn shadow_color(t: &Theme) -> Color32 {
    if t.dark {
        Color32::BLACK
    } else {
        Color32::from_rgb(40, 40, 90).gamma_multiply(0.5)
    }
}

/// Paint a rounded rect (pill) rotated by `angle` and scaled by `scale` around `center`.
fn rotated_pill(size: Vec2, radius: f32, center: Pos2, angle: f32, scale: f32) -> Vec<Pos2> {
    let (s, c) = angle.sin_cos();
    outline(Rect::from_center_size(Pos2::ZERO, size), radius, 8.0)
        .into_iter()
        .map(|p| {
            let p = p.to_vec2() * scale;
            center + vec2(p.x * c - p.y * s, p.x * s + p.y * c)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------

/// A floating header with a logo bubble and a round menu button. Clicking the button bursts the
/// menu open: big pill-shaped links pop in one after another on bouncy springs, each slightly
/// tilted, and straighten up and flood with their own color when hovered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = glowkit_effects::BubbleMenu::new(&["Home", "About", "Projects", "Blog", "Contact"]).show(ui);
/// if let Some(i) = r.inner { /* item i clicked */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct BubbleMenu {
    items: Vec<String>,
    colors: Vec<Color32>,
    logo: String,
    height: f32,
    autoplay: bool,
}

impl BubbleMenu {
    /// A bubble menu with these link labels.
    pub fn new<S: AsRef<str>>(items: &[S]) -> Self {
        Self {
            items: items.iter().map(|s| s.as_ref().to_owned()).collect(),
            colors: Vec::new(),
            logo: "◉ glowkit".into(),
            height: 340.0,
            autoplay: false,
        }
    }
    /// Hover colors of the pills (cycled). Default: the theme chart palette.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Text in the logo bubble.
    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.logo = logo.into();
        self
    }
    /// Total height of the widget (header plus the area the pills burst into).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(160.0);
        self
    }
    /// Open, hover and close on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the menu. `inner` is the item clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return InnerResponse::new(None, resp);
        }
        let n = self.items.len();
        let pad = 14.0;
        let bar = 54.0;
        let painter = ui.painter_at(rect.expand(30.0));
        let hovered = ui.rect_contains_pointer(rect);
        let btn_rect =
            Rect::from_center_size(pos2(rect.max.x - pad - bar / 2.0, rect.min.y + pad + bar / 2.0), Vec2::splat(bar));
        let btn = ui.interact(btn_rect, id.with("btn"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let (open, since) = toggle(ui, id, btn.clicked(), None, self.autoplay, hovered, 6.0, 1.8);
        let p_open = spring(ui, id.with("open"), if open { 1.0 } else { 0.0 }, 240.0, 22.0);
        let time = glowkit_core::time(ui);
        let surf = surface(&t);
        let sc = shadow_color(&t);

        // pill grid
        let per_row = if n <= 4 { 2 } else { 3 };
        let rows = n.div_ceil(per_row).max(1);
        let gap = 26.0;
        let pill_h = 62.0;
        let pill_w = ((rect.width() - 2.0 * pad - gap * (per_row as f32 - 1.0)) / per_row as f32).min(220.0);
        let area_top = rect.min.y + pad + bar + 12.0;
        let grid_h = rows as f32 * pill_h + (rows as f32 - 1.0) * gap;
        let top = area_top + ((rect.max.y - area_top - grid_h) / 2.0).max(0.0);
        let centers: Vec<Pos2> = (0..n)
            .map(|i| {
                let row = i / per_row;
                let in_row = if row == rows - 1 { n - row * per_row } else { per_row };
                let col = i % per_row;
                let row_w = in_row as f32 * pill_w + (in_row as f32 - 1.0) * gap;
                let x = rect.center().x - row_w / 2.0 + pill_w / 2.0 + col as f32 * (pill_w + gap);
                pos2(x, top + pill_h / 2.0 + row as f32 * (pill_h + gap))
            })
            .collect();

        // backdrop glow
        if p_open > 0.01 {
            let c = pos2(rect.center().x, top + grid_h / 2.0);
            let a = p_open.clamp(0.0, 1.0);
            paint::glow_ellipse(
                &painter,
                c,
                vec2(rect.width() * 0.45, grid_h * 0.9 + 40.0),
                with_alpha(t.accent, 0.12 * a),
            );
        }

        let mut clicked = None;
        let auto_hover = if self.autoplay && !hovered && open && since > 0.9 && n > 0 {
            let k = ((since - 0.9) / 0.9) as usize;
            if k % (n + 1) < n {
                Some(k % (n + 1))
            } else {
                None
            }
        } else {
            None
        };
        let angles = [-8.0f32, 6.0, -5.0, 9.0, -7.0, 5.0, -4.0, 8.0];
        for i in 0..n {
            let delay = if open { i as f32 * 0.06 } else { (n - 1 - i) as f32 * 0.035 };
            let target = if open == (since > delay) { 1.0 } else { 0.0 };
            let s = spring(ui, id.with(("s", i)), target, 300.0, 15.0).max(0.0);
            if s < 0.01 {
                continue;
            }
            let c = centers[i];
            let pill_rect = Rect::from_center_size(c, vec2(pill_w, pill_h));
            let r = if open && s > 0.6 {
                Some(
                    ui.interact(pill_rect, id.with(("pill", i)), Sense::click())
                        .on_hover_cursor(CursorIcon::PointingHand),
                )
            } else {
                None
            };
            let hov = r.as_ref().is_some_and(|r| r.hovered()) || auto_hover == Some(i);
            if r.as_ref().is_some_and(|r| r.clicked()) {
                clicked = Some(i);
            }
            let h = follow(ui, id.with(("h", i)), if hov { 1.0 } else { 0.0 }, 14.0);
            let col =
                if self.colors.is_empty() { t.chart[i % t.chart.len()] } else { self.colors[i % self.colors.len()] };
            let angle = angles[i % angles.len()].to_radians() * (1.0 - h);
            let scale = s * (1.0 + 0.06 * h);
            let float = (time * 1.6 + i as f32 * 1.3).sin() * 2.0 * (1.0 - h);
            let c = c + vec2(0.0, float);
            let pts = rotated_pill(vec2(pill_w, pill_h), pill_h / 2.0, c, angle, scale);
            polygon_shadow(&painter, &pts, c, vec2(0.0, 8.0 * scale), 12.0, sc.gamma_multiply(0.8 * s.min(1.0)));
            let fill = mix(surf, col, h);
            paint::fill_convex_fn(&painter, &pts, 2, |_| fill);
            let mut rim = pts.clone();
            rim.push(rim[0]);
            painter.add(egui::Shape::line(rim, Stroke::new(1.0, with_alpha(t.border, (1.0 - h) * s.min(1.0)))));
            let txt = mix(t.foreground, contrast_text(col), h);
            glowkit_core::text::paint_rotated(
                &painter,
                &self.items[i],
                c,
                FontId::proportional((25.0 * scale).max(1.0)),
                txt,
                angle,
            );
            glowkit_core::text::paint_rotated(
                &painter,
                &self.items[i],
                c + vec2(0.6 * scale, 0.0),
                FontId::proportional((25.0 * scale).max(1.0)),
                txt,
                angle,
            );
        }
        if clicked.is_some() {
            toggle(ui, id, false, Some(false), self.autoplay, hovered, 6.0, 1.8);
        }

        // header bubbles
        let logo_g = painter.layout_no_wrap(self.logo.clone(), FontId::proportional(19.0), t.foreground);
        let logo_rect = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(logo_g.size().x + 44.0, bar));
        paint::soft_shadow(
            &painter,
            logo_rect,
            bar / 2.0,
            18.0,
            vec2(0.0, 6.0),
            sc.gamma_multiply(if t.dark { 0.5 } else { 0.25 }),
        );
        painter.rect(logo_rect, cr(bar / 2.0), surf, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        bold_text(
            &painter,
            logo_rect.center(),
            Align2::CENTER_CENTER,
            &self.logo,
            FontId::proportional(19.0),
            t.foreground,
        );

        let bh = follow(ui, id.with("bh"), if btn.hovered() { 1.0 } else { 0.0 }, 14.0);
        let bs = 1.0 + 0.06 * bh;
        let btn_draw = Rect::from_center_size(btn_rect.center(), btn_rect.size() * bs);
        paint::soft_shadow(
            &painter,
            btn_draw,
            bar / 2.0,
            18.0,
            vec2(0.0, 6.0),
            sc.gamma_multiply(if t.dark { 0.5 } else { 0.25 }),
        );
        let bfill = mix(surf, t.foreground, p_open.clamp(0.0, 1.0));
        painter.circle(btn_draw.center(), bar / 2.0 * bs, bfill, Stroke::new(1.0, t.border));
        let lines = mix(t.foreground, t.background, p_open.clamp(0.0, 1.0));
        burger(&painter, btn_draw.center(), 20.0, p_open, 2, lines);

        InnerResponse::new(clicked, resp)
    }
}

// ---------------------------------------------------------------------------------------------

/// One colored card of a [`CardNav`]: a heading and a list of links.
#[derive(Clone, Debug, PartialEq)]
pub struct CardNavItem {
    /// Card heading.
    pub label: String,
    /// Link labels listed at the bottom of the card.
    pub links: Vec<String>,
    /// Card background and text colors; `None` derives them from the theme.
    pub colors: Option<[Color32; 2]>,
}

impl CardNavItem {
    /// A card with a heading and links.
    pub fn new<S: AsRef<str>>(label: impl Into<String>, links: &[S]) -> Self {
        Self { label: label.into(), links: links.iter().map(|s| s.as_ref().to_owned()).collect(), colors: None }
    }
    /// Card background and text color.
    pub fn colors(mut self, background: Color32, text: Color32) -> Self {
        self.colors = Some([background, text]);
        self
    }
}

/// A rounded navigation bar (hamburger, centered logo, call-to-action) that smoothly grows
/// downward when opened, revealing a row of colored link cards that slide up one by one.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{CardNav, CardNavItem};
/// let r = CardNav::new(vec![
///     CardNavItem::new("About", &["Company", "Careers"]),
///     CardNavItem::new("Projects", &["Featured", "Case studies"]),
/// ]).show(ui);
/// if let Some((card, link)) = r.inner { /* ... */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CardNav {
    items: Vec<CardNavItem>,
    logo: String,
    cta: String,
    width: Option<f32>,
    body_height: f32,
    autoplay: bool,
}

impl CardNav {
    /// A card nav with these cards.
    pub fn new(items: Vec<CardNavItem>) -> Self {
        Self {
            items,
            logo: "◆ glowkit".into(),
            cta: "Get Started".into(),
            width: None,
            body_height: 200.0,
            autoplay: false,
        }
    }
    /// Logo text in the middle of the bar.
    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.logo = logo.into();
        self
    }
    /// Label of the call-to-action button (empty hides it).
    pub fn cta(mut self, cta: impl Into<String>) -> Self {
        self.cta = cta.into();
        self
    }
    /// Width of the bar (default: up to 820 points, centered).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Height of the card area revealed when open.
    pub fn body_height(mut self, h: f32) -> Self {
        self.body_height = h.max(80.0);
        self
    }
    /// Open and close on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    fn card_colors(&self, t: &Theme, i: usize) -> [Color32; 2] {
        if let Some(c) = self.items[i].colors {
            return c;
        }
        let k = [0.12, 0.24, 0.38, 0.52][i % 4];
        let bg =
            if t.dark { mix(shade(t.background, 0.04), t.accent, k) } else { mix(t.accent, Color32::WHITE, 0.86 - k) };
        [bg, contrast_text(bg)]
    }

    /// Show the nav. `inner` is `(card, link)` when a link was clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<(usize, usize)>> {
        let t = Theme::of(ui);
        let bar_h = 60.0;
        let total_h = bar_h + self.body_height + 24.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), total_h), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return InnerResponse::new(None, resp);
        }
        let w = self.width.unwrap_or(820.0).min(rect.width() - 8.0);
        let x0 = rect.center().x - w / 2.0;
        let y0 = rect.min.y + 6.0;
        let bar = Rect::from_min_size(pos2(x0, y0), vec2(w, bar_h));
        let burger_rect = Rect::from_center_size(pos2(bar.min.x + 34.0, bar.center().y), vec2(44.0, 44.0));
        let bt = ui.interact(burger_rect, id.with("burger"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hovered = ui.rect_contains_pointer(rect);
        let (open, since) = toggle(ui, id, bt.clicked(), None, self.autoplay, hovered, 6.0, 1.8);
        let p = spring(ui, id.with("p"), if open { 1.0 } else { 0.0 }, 190.0, 21.0).max(0.0);
        let nav = Rect::from_min_size(bar.min, vec2(w, bar_h + self.body_height * p));
        let painter = ui.painter_at(rect.expand(24.0));
        let surf = surface(&t);
        let radius = 16.0;
        paint::soft_shadow(
            &painter,
            nav,
            radius,
            28.0,
            vec2(0.0, 10.0),
            shadow_color(&t).gamma_multiply(if t.dark { 0.55 } else { 0.22 }),
        );
        painter.rect(nav, cr(radius), surf, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);

        // top row
        let bh = follow(ui, id.with("bh"), if bt.hovered() { 1.0 } else { 0.0 }, 14.0);
        if bh > 0.01 {
            painter.circle_filled(burger_rect.center(), 20.0, with_alpha(t.foreground, 0.06 * bh));
        }
        burger(&painter, burger_rect.center(), 22.0, p.min(1.0), 2, t.foreground);
        bold_text(&painter, bar.center(), Align2::CENTER_CENTER, &self.logo, FontId::proportional(20.0), t.foreground);
        if !self.cta.is_empty() {
            let g = painter.layout_no_wrap(self.cta.clone(), FontId::proportional(14.5), t.background);
            let cta = Rect::from_min_size(
                pos2(bar.max.x - 10.0 - g.size().x - 36.0, bar.min.y + 10.0),
                vec2(g.size().x + 36.0, bar_h - 20.0),
            );
            let cr_ = ui.interact(cta, id.with("cta"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let ch = follow(ui, id.with("ch"), if cr_.hovered() { 1.0 } else { 0.0 }, 14.0);
            painter.rect_filled(cta, cr(10.0), mix(t.foreground, t.accent, ch));
            painter.galley(cta.center() - g.size() / 2.0, g, t.background);
        }

        // cards
        let mut clicked = None;
        if p > 0.01 {
            let clip = painter.with_clip_rect(nav.shrink(1.0));
            let n = self.items.len().max(1);
            let gap = 10.0;
            let pad = 10.0;
            let cw = (w - 2.0 * pad - gap * (n as f32 - 1.0)) / n as f32;
            let ch = self.body_height - pad;
            for (i, item) in self.items.iter().enumerate() {
                let delay = 0.1 + i as f32 * 0.07;
                let target = if open && since > delay { 1.0 } else { 0.0 };
                let q = spring(ui, id.with(("q", i)), target, 230.0, 24.0);
                let card = Rect::from_min_size(
                    pos2(x0 + pad + i as f32 * (cw + gap), bar.max.y + (1.0 - q) * 50.0),
                    vec2(cw, ch),
                );
                let a = q.clamp(0.0, 1.0);
                let [bg, fg] = self.card_colors(&t, i);
                clip.rect_filled(card, cr(12.0), bg.gamma_multiply(a));
                bold_text(
                    &clip,
                    card.min + vec2(18.0, 16.0),
                    Align2::LEFT_TOP,
                    &item.label,
                    FontId::proportional(22.0),
                    with_alpha(fg, a),
                );
                let lh = 24.0;
                let ly0 = card.max.y - 16.0 - lh * item.links.len() as f32;
                for (j, link) in item.links.iter().enumerate() {
                    let lr = Rect::from_min_size(pos2(card.min.x + 14.0, ly0 + j as f32 * lh), vec2(cw - 28.0, lh));
                    let hov = if open && q > 0.8 {
                        let r = ui
                            .interact(lr, id.with(("l", i, j)), Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if r.clicked() {
                            clicked = Some((i, j));
                        }
                        r.hovered()
                    } else {
                        false
                    };
                    let lhv = follow(ui, id.with(("lh", i, j)), if hov { 1.0 } else { 0.0 }, 16.0);
                    let col = with_alpha(fg, a * (0.78 + 0.22 * lhv));
                    let ax = lr.min.x + 4.0 + 3.0 * lhv;
                    let ay = lr.center().y;
                    // arrow ↗
                    let s = 4.5;
                    let st = Stroke::new(1.6, col);
                    clip.line_segment([pos2(ax, ay + s), pos2(ax + 2.0 * s, ay - s)], st);
                    clip.line_segment([pos2(ax + 0.6 * s, ay - s), pos2(ax + 2.0 * s, ay - s)], st);
                    clip.line_segment([pos2(ax + 2.0 * s, ay - s), pos2(ax + 2.0 * s, ay + 0.4 * s)], st);
                    let g = clip.text(
                        pos2(ax + 2.0 * s + 9.0, ay),
                        Align2::LEFT_CENTER,
                        link,
                        FontId::proportional(15.0),
                        col,
                    );
                    if lhv > 0.01 {
                        clip.line_segment(
                            [pos2(g.min.x, g.max.y + 1.0), pos2(g.min.x + g.width() * lhv, g.max.y + 1.0)],
                            Stroke::new(1.0, col),
                        );
                    }
                }
            }
        }
        if clicked.is_some() {
            toggle(ui, id, false, Some(false), self.autoplay, hovered, 6.0, 1.8);
        }
        InnerResponse::new(clicked, resp)
    }
}

// ---------------------------------------------------------------------------------------------

/// A pill-shaped navigation bar: a round logo next to a dark capsule of light pills. Hovering a
/// pill raises a circle of the base color from below that floods the pill while the label rolls
/// up and is replaced by an inverted copy. The active item is marked with a small dot.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut active = 0usize;
/// ui.add(glowkit_effects::PillNav::new(&mut active, &["Home", "About", "Services", "Contact"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PillNav<'a> {
    active: &'a mut usize,
    items: Vec<String>,
    logo: String,
    base: Option<Color32>,
    pill: Option<Color32>,
    pill_text: Option<Color32>,
    hover_text: Option<Color32>,
    height: f32,
    font_size: f32,
    autoplay: bool,
}

impl<'a> PillNav<'a> {
    /// A pill nav selecting into `active`.
    pub fn new<S: AsRef<str>>(active: &'a mut usize, items: &[S]) -> Self {
        Self {
            active,
            items: items.iter().map(|s| s.as_ref().to_owned()).collect(),
            logo: "✦".into(),
            base: None,
            pill: None,
            pill_text: None,
            hover_text: None,
            height: 44.0,
            font_size: 15.0,
            autoplay: false,
        }
    }
    /// Glyph in the round logo (empty hides it).
    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.logo = logo.into();
        self
    }
    /// Base color: the capsule, the logo and the hover fill. Default: theme foreground.
    pub fn base_color(mut self, c: Color32) -> Self {
        self.base = Some(c);
        self
    }
    /// Pill color. Default: theme background.
    pub fn pill_color(mut self, c: Color32) -> Self {
        self.pill = Some(c);
        self
    }
    /// Label color on a resting pill. Default: the base color.
    pub fn pill_text_color(mut self, c: Color32) -> Self {
        self.pill_text = Some(c);
        self
    }
    /// Label color on a hovered pill. Default: the pill color.
    pub fn hover_text_color(mut self, c: Color32) -> Self {
        self.hover_text = Some(c);
        self
    }
    /// Bar height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(24.0);
        self
    }
    /// Label font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.max(8.0);
        self
    }
    /// Sweep the hover effect across the pills while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for PillNav<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let base = self.base.unwrap_or(t.foreground);
        let pill = self.pill.unwrap_or(t.background);
        let pill_text = self.pill_text.unwrap_or(base);
        let hover_text = self.hover_text.unwrap_or(pill);
        let h = self.height;
        let font = FontId::proportional(self.font_size);
        let pad = 4.0;
        let gap = 3.0;
        let widths: Vec<f32> = self
            .items
            .iter()
            .map(|s| ui.painter().layout_no_wrap(s.clone(), font.clone(), pill_text).size().x + 36.0)
            .collect();
        let logo_w = if self.logo.is_empty() { 0.0 } else { h + 8.0 };
        let cap_w = widths.iter().sum::<f32>() + gap * (widths.len().max(1) - 1) as f32 + pad * 2.0;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(logo_w + cap_w, h + 12.0), Sense::hover());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        let time = glowkit_core::time(ui);
        // logo
        if !self.logo.is_empty() {
            let c = pos2(rect.min.x + h / 2.0, rect.min.y + h / 2.0);
            let lr = ui.interact(Rect::from_center_size(c, Vec2::splat(h)), id.with("logo"), Sense::hover());
            let spin = spring(ui, id.with("spin"), if lr.hovered() { 1.0 } else { 0.0 }, 120.0, 16.0);
            painter.circle_filled(c, h / 2.0, base);
            glowkit_core::text::paint_rotated(
                painter,
                &self.logo,
                c,
                FontId::proportional(h * 0.45),
                pill,
                spin * std::f32::consts::TAU,
            );
        }
        let cap = Rect::from_min_size(pos2(rect.min.x + logo_w, rect.min.y), vec2(cap_w, h));
        painter.rect_filled(cap, cr(h / 2.0), base);
        let hovered_any = ui.rect_contains_pointer(rect);
        let n = self.items.len();
        let auto = if self.autoplay && !hovered_any && n > 0 {
            glowkit_core::animate(ui);
            let k = (time / 1.1) as usize;
            let slot = k % (n + 1);
            (slot < n).then_some(slot)
        } else {
            None
        };
        let mut x = cap.min.x + pad;
        let ph = h - pad * 2.0;
        for (i, label) in self.items.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, cap.min.y + pad), vec2(widths[i], ph));
            x += widths[i] + gap;
            let pr = ui.interact(r, id.with(("pill", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if pr.clicked() && *self.active != i {
                *self.active = i;
                resp.mark_changed();
            }
            let on = pr.hovered() || auto == Some(i);
            let hv = spring(ui, id.with(("h", i)), if on { 1.0 } else { 0.0 }, 170.0, 22.0);
            painter.rect_filled(r, cr(ph / 2.0), pill);
            let e = hv.clamp(0.0, 1.05);
            if e > 0.002 {
                // a circle grows from just below the pill until it floods it
                let origin = pos2(r.center().x, r.max.y + ph * 0.35);
                let cover = (r.width() * r.width() / 4.0 + (ph * 1.35) * (ph * 1.35)).sqrt();
                let circle = paint::circle_points(origin, cover * e, 72);
                let poly = clip_convex(&circle, &outline(r, ph / 2.0, 6.0));
                if poly.len() >= 3 {
                    fill_poly(painter, &poly, base);
                }
            }
            let clip = painter.with_clip_rect(r.shrink(1.0));
            let off = e.min(1.0) * ph;
            let g = clip.layout_no_wrap(label.clone(), font.clone(), pill_text);
            let tl = r.center() - g.size() / 2.0;
            if off < ph - 0.5 {
                clip.galley_with_override_text_color(tl - vec2(0.0, off), g.clone(), pill_text);
            }
            if off > 0.5 {
                clip.galley_with_override_text_color(tl + vec2(0.0, ph - off), g, hover_text);
            }
            if *self.active == i {
                let a = follow(ui, id.with(("dot", i)), 1.0, 8.0);
                painter.circle_filled(pos2(r.center().x, cap.max.y + 6.0), 3.0 * a, base);
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------

/// A page header with a "Menu +" toggle that slides in a full-height side panel: a couple of
/// colored layers sweep in first, staggered, then the panel itself arrives with huge numbered
/// links rising into place one by one and a row of social links underneath.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = glowkit_effects::StaggeredMenu::new(&["Home", "About", "Services", "Contact"])
///     .show(ui, |ui| { ui.heading("Page content"); });
/// if let Some(i) = r.inner { /* item i clicked */ }
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct StaggeredMenu {
    items: Vec<String>,
    socials: Vec<String>,
    logo: String,
    layers: Vec<Color32>,
    accent: Option<Color32>,
    panel: Option<Color32>,
    height: f32,
    panel_width: Option<f32>,
    numbered: bool,
    autoplay: bool,
}

impl StaggeredMenu {
    /// A staggered menu with these items.
    pub fn new<S: AsRef<str>>(items: &[S]) -> Self {
        Self {
            items: items.iter().map(|s| s.as_ref().to_owned()).collect(),
            socials: vec!["Twitter".into(), "GitHub".into(), "LinkedIn".into()],
            logo: "glowkit".into(),
            layers: Vec::new(),
            accent: None,
            panel: None,
            height: 420.0,
            panel_width: None,
            numbered: true,
            autoplay: false,
        }
    }
    /// Social links shown at the bottom of the panel (empty hides the row).
    pub fn socials<S: AsRef<str>>(mut self, socials: &[S]) -> Self {
        self.socials = socials.iter().map(|s| s.as_ref().to_owned()).collect();
        self
    }
    /// Logo text in the header.
    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.logo = logo.into();
        self
    }
    /// Colors of the layers that sweep in before the panel. Default: two accent tints.
    pub fn layer_colors(mut self, colors: &[Color32]) -> Self {
        self.layers = colors.to_vec();
        self
    }
    /// Accent for numbers, hover and the socials title. Default: theme accent.
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Panel color. Default: a card surface.
    pub fn panel_color(mut self, c: Color32) -> Self {
        self.panel = Some(c);
        self
    }
    /// Height of the whole widget.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Width of the side panel (default: about half the widget, max 440).
    pub fn panel_width(mut self, w: f32) -> Self {
        self.panel_width = Some(w);
        self
    }
    /// Show small "01", "02"... numbers next to the items.
    pub fn numbered(mut self, on: bool) -> Self {
        self.numbered = on;
        self
    }
    /// Open and close on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the menu over a page laid out by `page`. `inner` is the item clicked this frame.
    pub fn show<R>(self, ui: &mut Ui, page: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = resp.id;
        let radius = 18.0;
        let frame = outline(rect, radius, 12.0);
        let painter = ui.painter().clone();
        painter.rect(rect, cr(radius), t.background, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        // page content
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("page"))
                .max_rect(rect.shrink2(vec2(28.0, 0.0)).with_min_y(rect.min.y + 74.0))
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        page(&mut child);

        let accent = self.accent.unwrap_or(t.accent);
        let panel_col = self.panel.unwrap_or(if t.dark { shade(t.card, 0.06) } else { Color32::WHITE });
        let panel_text = contrast_text(panel_col);
        let layers: Vec<Color32> = if self.layers.is_empty() {
            vec![mix(accent, t.gradient[1], 0.25), shade(accent, if t.dark { -0.25 } else { 0.15 })]
        } else {
            self.layers.clone()
        };
        let pw = self.panel_width.unwrap_or((rect.width() * 0.5).clamp(240.0, 440.0)).min(rect.width());
        let hovered = ui.rect_contains_pointer(rect);

        // header toggle
        let toggle_rect = Rect::from_min_size(pos2(rect.max.x - 130.0, rect.min.y + 14.0), vec2(112.0, 40.0));
        let tr = ui.interact(toggle_rect, id.with("toggle"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let (open, since) = toggle(ui, id, tr.clicked(), None, self.autoplay, hovered, 6.0, 1.8);
        let p = follow(ui, id.with("p"), if open { 1.0 } else { 0.0 }, 7.0);

        let nl = layers.len() + 1;
        let mut panel_x = rect.max.x;
        let es: Vec<f32> = (0..nl)
            .map(|k| {
                if open {
                    ease(Easing::OutQuart, stagger(since, k, 0.08, 0.6))
                } else {
                    1.0 - ease(Easing::InOutCubic, stagger(since, nl - 1 - k, 0.05, 0.4))
                }
            })
            .collect();
        let covered = es[nl - 1] >= 0.999;
        for (k, &e) in es.iter().enumerate() {
            if e <= 0.001 || (covered && k < nl - 1) {
                continue;
            }
            let x = rect.max.x - pw * e;
            let col = if k < layers.len() { layers[k] } else { panel_col };
            let r = Rect::from_min_max(pos2(x, rect.min.y), rect.max);
            let poly = clip_convex(&outline(r, 0.0, 400.0), &frame);
            if poly.len() >= 3 {
                if k == nl - 1 {
                    let sh = Rect::from_min_max(pos2(x, rect.min.y + 2.0), pos2(x + 8.0, rect.max.y - 2.0));
                    paint::soft_shadow(&painter, sh, 0.0, 24.0, vec2(-6.0, 0.0), shadow_color(&t).gamma_multiply(0.35));
                }
                fill_poly(&painter, &poly, col);
            }
            if k == nl - 1 {
                panel_x = x;
            }
        }

        // header (drawn over the layers)
        bold_text(
            &painter,
            pos2(rect.min.x + 28.0, rect.min.y + 34.0),
            Align2::LEFT_CENTER,
            &self.logo,
            FontId::proportional(21.0),
            t.foreground,
        );
        let tcol = mix(t.foreground, panel_text, p);
        let th = follow(ui, id.with("th"), if tr.hovered() { 1.0 } else { 0.0 }, 14.0);
        let tclip = painter.with_clip_rect(toggle_rect);
        let roll = p * toggle_rect.height();
        let tx = toggle_rect.max.x - 30.0;
        tclip.text(
            pos2(tx, toggle_rect.center().y - roll),
            Align2::RIGHT_CENTER,
            "Menu",
            FontId::proportional(17.0),
            tcol,
        );
        tclip.text(
            pos2(tx, toggle_rect.center().y + toggle_rect.height() - roll),
            Align2::RIGHT_CENTER,
            "Close",
            FontId::proportional(17.0),
            tcol,
        );
        let ic = pos2(toggle_rect.max.x - 14.0, toggle_rect.center().y);
        let ang = p * std::f32::consts::FRAC_PI_4 + th * 0.2;
        let st = Stroke::new(2.0, mix(tcol, accent, th));
        for a in [ang, ang + std::f32::consts::FRAC_PI_2] {
            let d = vec2(a.cos(), a.sin()) * 7.0;
            painter.line_segment([ic - d, ic + d], st);
        }

        // items
        let mut clicked = None;
        if panel_x < rect.max.x - 1.0 {
            let clip = painter.with_clip_rect(Rect::from_min_max(pos2(panel_x, rect.min.y), rect.max).intersect(rect));
            let size = (self.height * 0.105).clamp(26.0, 46.0);
            let line = size * 1.22;
            let x = panel_x + 34.0;
            let y0 = rect.min.y + 80.0;
            for (i, item) in self.items.iter().enumerate() {
                let e = if open {
                    ease(Easing::OutCubic, stagger(since - 0.28, i, 0.07, 0.6))
                } else {
                    1.0 - (since * 4.0).min(1.0)
                };
                let row = Rect::from_min_size(pos2(panel_x, y0 + i as f32 * line), vec2(pw, line));
                let hov = if open && e > 0.9 {
                    let r = ui
                        .interact(row, id.with(("item", i)), Sense::click())
                        .on_hover_cursor(CursorIcon::PointingHand);
                    if r.clicked() {
                        clicked = Some(i);
                    }
                    r.hovered()
                } else {
                    false
                };
                let hv = follow(ui, id.with(("ih", i)), if hov { 1.0 } else { 0.0 }, 14.0);
                let rclip = clip.with_clip_rect(row.intersect(clip.clip_rect()));
                let y = row.min.y + (1.0 - e) * line * 1.1;
                let col = mix(panel_text, accent, hv);
                let text = item.to_uppercase();
                let g =
                    bold_text(&rclip, pos2(x + 6.0 * hv, y), Align2::LEFT_TOP, &text, FontId::proportional(size), col);
                if self.numbered {
                    rclip.text(
                        pos2(g.max.x + 8.0, y + size * 0.12),
                        Align2::LEFT_TOP,
                        format!("{:02}", i + 1),
                        FontId::proportional(13.0),
                        with_alpha(accent, e),
                    );
                }
            }
            if !self.socials.is_empty() {
                let e = if open {
                    ease(Easing::OutCubic, stagger(since - 0.55, 0, 0.0, 0.5))
                } else {
                    1.0 - (since * 4.0).min(1.0)
                };
                let sy = rect.max.y - 62.0 + (1.0 - e) * 14.0;
                clip.text(pos2(x, sy), Align2::LEFT_TOP, "Socials", FontId::proportional(14.0), with_alpha(accent, e));
                let mut sx = x;
                for (j, s) in self.socials.iter().enumerate() {
                    let ej = if open { ease(Easing::OutCubic, stagger(since - 0.6, j, 0.06, 0.5)) } else { e };
                    let g = clip.text(
                        pos2(sx, sy + 24.0 + (1.0 - ej) * 10.0),
                        Align2::LEFT_TOP,
                        s,
                        FontId::proportional(15.0),
                        with_alpha(panel_text, ej * 0.9),
                    );
                    sx = g.max.x + 18.0;
                }
            }
        }
        if clicked.is_some() {
            toggle(ui, id, false, Some(false), self.autoplay, hovered, 6.0, 1.8);
        }
        InnerResponse::new(clicked, resp)
    }
}
