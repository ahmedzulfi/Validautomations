use super::common::{bold_text, capture, clip_mesh, desaturate, paint_avatar, pointer_or_auto, subdivide, Persp, VIVID};
use crate::buttons::util::{self, cr, outline};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, InnerResponse, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{follow, smoothstep, spring2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};

/// One card of a [`MagicBento`] grid.
#[derive(Clone, Debug, PartialEq)]
pub struct BentoItem {
    /// Small label in the top-left corner.
    pub label: String,
    /// Card title.
    pub title: String,
    /// One or two lines under the title.
    pub description: String,
}

impl BentoItem {
    /// A card with a corner label, a title and a description.
    pub fn new(label: impl Into<String>, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self { label: label.into(), title: title.into(), description: description.into() }
    }
}

#[derive(Clone, Copy, Debug)]
struct Ripple {
    at: f64,
    pos: Pos2,
    card: usize,
}

/// A bento grid of dark cards brought to life by the pointer: a soft spotlight follows it
/// across the whole grid and lights up nearby card borders, the hovered card tilts and leans
/// toward the pointer, sparkling particles drift inside it, and clicks send out a ripple.
///
/// With six items it uses the classic bento layout (two tall feature cards), otherwise an even
/// grid.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{BentoItem, MagicBento};
/// let r = MagicBento::new(vec![BentoItem::new("Insights", "Analytics", "Track user behavior")]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MagicBento {
    items: Vec<BentoItem>,
    height: f32,
    glow: Option<Color32>,
    background: Option<Color32>,
    spotlight_radius: f32,
    particles: usize,
    tilt: bool,
    autoplay: bool,
}

impl MagicBento {
    /// A bento grid of these items.
    pub fn new(items: Vec<BentoItem>) -> Self {
        Self {
            items,
            height: 380.0,
            glow: None,
            background: None,
            spotlight_radius: 260.0,
            particles: 12,
            tilt: true,
            autoplay: false,
        }
    }
    /// Height of the grid.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Glow / spotlight / particle color. Default: theme accent.
    pub fn glow_color(mut self, c: Color32) -> Self {
        self.glow = Some(c);
        self
    }
    /// Card fill. Default: a deep tint of the background.
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Radius of the pointer spotlight in points.
    pub fn spotlight_radius(mut self, r: f32) -> Self {
        self.spotlight_radius = r.max(20.0);
        self
    }
    /// Particles drifting in the hovered card (0 disables).
    pub fn particles(mut self, n: usize) -> Self {
        self.particles = n.min(64);
        self
    }
    /// Tilt the hovered card toward the pointer.
    pub fn tilt(mut self, on: bool) -> Self {
        self.tilt = on;
        self
    }
    /// Wander the spotlight over the grid while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    fn layout(&self, rect: Rect) -> Vec<Rect> {
        let gap = 10.0;
        let n = self.items.len();
        if n == 6 && rect.width() > 420.0 {
            let cw = (rect.width() - gap * 3.0) / 4.0;
            let ch = (rect.height() - gap * 2.0) / 3.0;
            let cell = |c: usize, r: usize, w: usize, h: usize| {
                Rect::from_min_size(
                    rect.min + vec2(c as f32 * (cw + gap), r as f32 * (ch + gap)),
                    vec2(w as f32 * cw + (w - 1) as f32 * gap, h as f32 * ch + (h - 1) as f32 * gap),
                )
            };
            return vec![cell(0, 0, 1, 1), cell(1, 0, 1, 1), cell(2, 0, 2, 2), cell(0, 1, 2, 2), cell(2, 2, 1, 1), cell(3, 2, 1, 1)];
        }
        let cols = n.clamp(1, 3);
        let rows = n.div_ceil(cols).max(1);
        let cw = (rect.width() - gap * (cols - 1) as f32) / cols as f32;
        let ch = (rect.height() - gap * (rows - 1) as f32) / rows as f32;
        (0..n)
            .map(|i| {
                Rect::from_min_size(
                    rect.min + vec2((i % cols) as f32 * (cw + gap), (i / cols) as f32 * (ch + gap)),
                    vec2(cw, ch),
                )
            })
            .collect()
    }

    /// Show the grid. `inner` is the card clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click());
        let id = resp.id;
        if !ui.is_rect_visible(rect) {
            return InnerResponse::new(None, resp);
        }
        let glow = self.glow.unwrap_or(t.accent);
        let bg = self.background.unwrap_or(if t.dark { mix(t.background, glow, 0.05) } else { Color32::WHITE });
        let fg = glowkit_core::color::contrast_text(bg);
        let muted = mix(fg, bg, 0.42);
        let time = glowkit_core::time(ui);
        let now = ui.input(|i| i.time);
        let ptr = pointer_or_auto(ui, rect.expand(24.0), self.autoplay, 0.7);
        let last: Vec2 = glowkit_core::state(ui, id.with("last"), || rect.center() - rect.min);
        let target = ptr.map(|p| p.0 - rect.min).unwrap_or(last);
        glowkit_core::set_state(ui, id.with("last"), target);
        let sp = rect.min + spring2(ui, id.with("ptr"), target, 170.0, 24.0);
        let k = follow(ui, id.with("k"), if ptr.is_some() { 1.0 } else { 0.0 }, 6.0);
        let cards = self.layout(rect);
        let painter = ui.painter_at(rect.expand(20.0));
        let spot_r = self.spotlight_radius;
        let real_click = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        let mut clicked = None;
        if let Some(cp) = real_click {
            if let Some(i) = cards.iter().position(|c| c.contains(cp)) {
                clicked = Some(i);
                glowkit_core::set_state(ui, id.with("ripple"), Some(Ripple { at: now, pos: cp, card: i }));
            }
        }
        let ripple: Option<Ripple> = glowkit_core::state(ui, id.with("ripple"), || None);
        if resp.hovered() && cards.iter().any(|c| ptr.is_some_and(|p| p.1 && c.contains(p.0))) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        for (i, (card, item)) in cards.iter().zip(&self.items).enumerate() {
            let on = ptr.is_some_and(|p| card.contains(p.0));
            let h = follow(ui, id.with(("h", i)), if on { 1.0 } else { 0.0 }, 8.0);
            let radius = 18.0;
            let rel = ((sp - card.center()) / (card.size() / 2.0)).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
            let mut pr = Persp::new(card.center() + (sp - card.center()) * 0.025 * h);
            if self.tilt {
                pr.rx = rel.y * 0.12 * h;
                pr.ry = rel.x * 0.12 * h;
                pr.distance = 700.0;
            }
            pr.scale = 1.0 + 0.012 * h;
            let local = outline(Rect::from_center_size(Pos2::ZERO, card.size()), radius, 10.0);
            let pts: Vec<Pos2> = local.iter().map(|p| pr.apply(p.to_vec2())).collect();
            let center = pr.apply(Vec2::ZERO);
            if t.dark {
                crate::motion::common::polygon_shadow(&painter, &pts, center, vec2(0.0, 6.0), 6.0 + 8.0 * h, Color32::BLACK.gamma_multiply(0.6));
            } else {
                crate::motion::common::polygon_shadow(
                    &painter,
                    &pts,
                    center,
                    vec2(0.0, 4.0 + 4.0 * h),
                    8.0 + 8.0 * h,
                    Color32::from_rgb(40, 30, 90).gamma_multiply(0.18 + 0.2 * h),
                );
            }
            paint::fill_convex_fn(&painter, &pts, 6, |q| {
                let d = q.distance(sp) / (spot_r * 0.9);
                let f = (1.0 - d).clamp(0.0, 1.0);
                let c = bg.lerp_to_gamma(mix(bg, glow, if t.dark { 0.14 } else { 0.06 }), f * f * k);
                shade(c, -0.02 * (q.y - card.min.y) / card.height())
            });
            // border: hairline + spotlight glow
            let base_border = if t.dark { mix(bg, glow, 0.22) } else { t.border };
            util::ribbon(&painter, &pts, 1.0, |q, _| {
                let d = q.distance(sp) / spot_r;
                let f = ((1.0 - d).clamp(0.0, 1.0)).powf(1.6) * k;
                mix(base_border, glow, f)
            });
            util::ribbon(&painter, &pts, 7.0, |q, _| {
                let d = q.distance(sp) / spot_r;
                let f = ((1.0 - d).clamp(0.0, 1.0)).powf(2.0) * k;
                with_alpha(glow, 0.22 * f)
            });

            // ripple
            if let Some(rp) = ripple.filter(|r| r.card == i) {
                let e = ((now - rp.at) as f32 / 0.9).clamp(0.0, 1.0);
                if e < 1.0 {
                    util::circle_in_rounded(
                        &painter,
                        *card,
                        radius,
                        rp.pos,
                        e * card.size().length(),
                        with_alpha(glow, 0.3 * (1.0 - e).powi(2)),
                    );
                    glowkit_core::animate(ui);
                }
            }

            // particles
            if self.particles > 0 && h > 0.01 {
                for j in 0..self.particles {
                    let seed = (i * 97 + j * 13) as u64;
                    let u = hash01(seed);
                    let v = hash01(seed + 5);
                    let sp_ = 0.4 + hash01(seed + 9) * 0.6;
                    let ph = hash01(seed + 11) * std::f32::consts::TAU;
                    let base = pos2(card.min.x + 12.0 + u * (card.width() - 24.0), card.min.y + 12.0 + v * (card.height() - 24.0));
                    let p = base + vec2((time * sp_ + ph).sin() * 22.0, (time * sp_ * 0.8 + ph * 1.7).cos() * 18.0);
                    let p = pr.apply(p - card.center());
                    let tw = 0.5 + 0.5 * (time * 2.4 + ph * 3.0).sin();
                    let a = h * (0.35 + 0.65 * tw);
                    paint::glow(&painter, p, 9.0, with_alpha(glow, 0.35 * a));
                    painter.circle_filled(p, 1.6, with_alpha(mix(glow, Color32::WHITE, 0.5), a));
                }
                glowkit_core::animate(ui);
            }

            // text
            let half = card.size() / 2.0;
            let pad = 18.0;
            let lp = pr.apply(vec2(-half.x + pad, -half.y + pad));
            painter.text(lp, Align2::LEFT_TOP, &item.label, FontId::proportional(13.0), muted);
            let wrap = card.width() - pad * 2.0;
            let dg = painter.layout(item.description.clone(), FontId::proportional(13.0), muted, wrap);
            let dp = pr.apply(vec2(-half.x + pad, half.y - pad - dg.size().y));
            let title_size = if card.width() > 300.0 && card.height() > 200.0 { 22.0 } else { 17.0 };
            bold_text(
                &painter,
                dp - vec2(0.0, 6.0),
                Align2::LEFT_BOTTOM,
                &item.title,
                FontId::proportional(title_size),
                fg,
            );
            painter.galley(dp, dg, muted);
        }

        // grid-wide spotlight
        if k > 0.01 {
            paint::glow(&painter, sp, spot_r * 0.55, with_alpha(glow, (if t.dark { 0.08 } else { 0.05 }) * k));
        }
        if ptr.is_some() && self.autoplay {
            glowkit_core::animate(ui);
        }
        InnerResponse::new(clicked, resp)
    }
}

// ---------------------------------------------------------------------------------------------

/// One profile tile of a [`ChromaGrid`].
#[derive(Clone, Debug, PartialEq)]
pub struct ChromaItem {
    /// Display name.
    pub name: String,
    /// Role or job title.
    pub role: String,
    /// Handle shown in the footer.
    pub handle: String,
    /// Gradient colors of the portrait; `None` picks from a vivid palette.
    pub colors: Option<[Color32; 2]>,
}

impl ChromaItem {
    /// A tile for a person.
    pub fn new(name: impl Into<String>, role: impl Into<String>, handle: impl Into<String>) -> Self {
        Self { name: name.into(), role: role.into(), handle: handle.into(), colors: None }
    }
    /// Portrait gradient colors.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }
}

/// A grid of profile tiles shown in grayscale, with a circle of full color that follows the
/// pointer across the grid. The tile under the pointer also lights up its border in its own
/// color and gets a soft inner spotlight.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{ChromaGrid, ChromaItem};
/// ChromaGrid::new(vec![ChromaItem::new("Ada Park", "Frontend", "@ada")]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ChromaGrid {
    items: Vec<ChromaItem>,
    tile: Vec2,
    columns: Option<usize>,
    radius: f32,
    autoplay: bool,
}

impl ChromaGrid {
    /// A grid of these profiles.
    pub fn new(items: Vec<ChromaItem>) -> Self {
        Self { items, tile: vec2(190.0, 250.0), columns: None, radius: 220.0, autoplay: false }
    }
    /// Size of each tile.
    pub fn tile_size(mut self, size: Vec2) -> Self {
        self.tile = size.max(vec2(80.0, 100.0));
        self
    }
    /// Number of columns (default: as many as fit).
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = Some(n.max(1));
        self
    }
    /// Radius of the color spotlight in points.
    pub fn spotlight_radius(mut self, r: f32) -> Self {
        self.radius = r.max(20.0);
        self
    }
    /// Wander the spotlight over the grid while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the grid. `inner` is the tile clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let gap = 14.0;
        let n = self.items.len().max(1);
        let fit = (((ui.available_width() + gap) / (self.tile.x + gap)).floor() as usize).max(1);
        let cols = self.columns.unwrap_or(fit).min(n);
        let rows = n.div_ceil(cols);
        let size = vec2(cols as f32 * (self.tile.x + gap) - gap, rows as f32 * (self.tile.y + gap) - gap);
        let (outer, resp) = ui.allocate_exact_size(vec2(ui.available_width(), size.y), Sense::click());
        let id = resp.id;
        if !ui.is_rect_visible(outer) {
            return InnerResponse::new(None, resp);
        }
        let rect = Rect::from_center_size(outer.center(), size);
        let ptr = pointer_or_auto(ui, rect.expand(30.0), self.autoplay, 1.9);
        let last: Vec2 = glowkit_core::state(ui, id.with("last"), || rect.center() - rect.min);
        let target = ptr.map(|p| p.0 - rect.min).unwrap_or(last);
        glowkit_core::set_state(ui, id.with("last"), target);
        let sp = rect.min + spring2(ui, id.with("ptr"), target, 120.0, 22.0);
        let k = follow(ui, id.with("k"), if ptr.is_some() { 1.0 } else { 0.0 }, 5.0);
        let spot = self.radius;
        let darken = if t.dark { 0.22 } else { 0.0 };
        let mut clicked = None;
        let painter = ui.painter();
        for (i, item) in self.items.iter().enumerate() {
            let tile = Rect::from_min_size(
                rect.min + vec2((i % cols) as f32 * (self.tile.x + gap), (i / cols) as f32 * (self.tile.y + gap)),
                self.tile,
            );
            let on = ptr.is_some_and(|p| tile.contains(p.0));
            let h = follow(ui, id.with(("h", i)), if on { 1.0 } else { 0.0 }, 8.0);
            if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| tile.contains(p)) {
                clicked = Some(i);
            }
            let colors = item.colors.unwrap_or(VIVID[i % VIVID.len()]);
            let radius = 16.0;
            let card_bg = if t.dark { shade(t.card, 0.03) } else { Color32::WHITE };
            let fg = t.foreground;
            let muted = t.muted_foreground;
            let img = Rect::from_min_max(tile.min + vec2(8.0, 8.0), pos2(tile.max.x - 8.0, tile.max.y - 64.0));
            let mesh = capture(ui, |p| {
                paint::fill_convex_fn(p, &outline(tile, radius, 10.0), 4, |q| {
                    let u = ((q.y - tile.min.y) / tile.height()).clamp(0.0, 1.0);
                    mix(mix(card_bg, colors[0], if t.dark { 0.22 } else { 0.12 }), card_bg, u)
                });
                let y = img.max.y + 12.0;
                bold_text(p, pos2(tile.min.x + 14.0, y), Align2::LEFT_TOP, &item.name, FontId::proportional(15.0), fg);
                p.text(pos2(tile.min.x + 14.0, y + 22.0), Align2::LEFT_TOP, &item.role, FontId::proportional(12.5), muted);
                p.text(pos2(tile.max.x - 14.0, y + 2.0), Align2::RIGHT_TOP, &item.handle, FontId::proportional(12.0), muted);
            });
            let mut mesh = subdivide(&mesh, 14.0, tile.expand(2.0));
            let avatar = capture(ui, |p| paint_avatar(p, img, 11.0, colors, i as u32, true));
            let avatar = clip_mesh(&subdivide(&avatar, 14.0, img.expand(2.0)), &outline(img, 11.0, 6.0));
            mesh.append(avatar);
            for v in &mut mesh.vertices {
                let d = v.pos.distance(sp);
                let m = (1.0 - smoothstep(spot * 0.25, spot, d)) * k;
                let m = m.max(h * 0.65);
                v.color = desaturate(v.color, 1.0 - m, darken * (1.0 - m));
            }
            if t.dark {
                paint::soft_shadow(painter, tile, radius, 18.0, vec2(0.0, 6.0), Color32::from_black_alpha(120));
            } else {
                paint::soft_shadow(painter, tile, radius, 18.0, vec2(0.0, 6.0), Color32::from_rgba_unmultiplied(30, 30, 70, 26));
            }
            painter.add(Shape::mesh(mesh));
            // inner spotlight + colored border on the hovered tile
            if h > 0.01 {
                util::inner_glow(painter, tile, radius, sp, tile.width() * 0.9, with_alpha(Color32::WHITE, 0.12 * h));
            }
            let border = mix(if t.dark { mix(t.border, Color32::BLACK, 0.2) } else { t.border }, colors[0], h);
            painter.rect_stroke(tile, cr(radius), Stroke::new(1.0 + h, border), egui::StrokeKind::Inside);
        }
        if ptr.is_some_and(|p| p.1) && clicked.is_none() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        InnerResponse::new(clicked, resp)
    }
}
