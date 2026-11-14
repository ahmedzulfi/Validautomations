//! Photo-based blocks: a fanning photo deck, an image accordion and a "generate" card that
//! resolves a painting under frosted glass.

use super::common::{cr, idle_for, soft_card, surface, track};
use crate::motion::common::polygon_shadow;
use crate::scroll::common::{cover_uv, image_rect, warp_card};
use crate::PaintedScene;
use egui::{
    pos2, vec2, Align2, Color32, ColorImage, Context, CursorIcon, FontId, Id, Pos2, Rect, Response, Sense, TextureId,
    Ui, Vec2,
};
use glowkit_core::anim::{ease, lerp, Easing, Spring};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::noise::noise2;
use glowkit_core::{Rng, Theme};
use std::f32::consts::{PI, TAU};

fn damping_from_bounce(bounce: f32, stiffness: f32) -> f32 {
    // bounce 0 → critically damped, 100 → very springy
    let crit = 2.0 * stiffness.sqrt();
    crit * lerp(1.0, 0.18, (bounce / 100.0).clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------------------------
// PhotoFan

#[derive(Clone, Copy)]
struct CardSpring {
    x: Spring,
    y: Spring,
    angle: Spring,
    scale: Spring,
    flip: Spring,
}

impl CardSpring {
    fn new() -> Self {
        Self {
            x: Spring::new(0.0),
            y: Spring::new(0.0),
            angle: Spring::new(0.0),
            scale: Spring::new(1.0),
            flip: Spring::new(0.0),
        }
    }
    fn springs(&mut self) -> [&mut Spring; 5] {
        [&mut self.x, &mut self.y, &mut self.angle, &mut self.scale, &mut self.flip]
    }
}

#[derive(Clone, Default)]
struct FanState {
    cards: Vec<CardSpring>,
    open: f32,
    selected: Option<usize>,
    /// Index of the card the selected one was before it flipped (to return it to its slot).
    loop_count: u32,
    last_phase: i32,
}

/// A small deck of photos that fans out as the pointer reaches it. Hover draws a card up out of
/// the fan, a click flips it (a full 3D turn) into the middle, and clicking again or clicking
/// away tucks it back into the deck. Everything moves on springs.
///
/// Cards show [`PaintedScene`] photos unless you pass your own textures.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::PhotoFan;
/// ui.add(PhotoFan::new().cards(5).spread(85.0).bounce(5.0).corner(14.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PhotoFan {
    cards: usize,
    textures: Vec<TextureId>,
    card_size: Vec2,
    spread: f32,
    scatter: f32,
    bounce: f32,
    corner: f32,
    autoplay: bool,
}

impl Default for PhotoFan {
    fn default() -> Self {
        Self::new()
    }
}

impl PhotoFan {
    /// A deck of five painted photos.
    pub fn new() -> Self {
        Self {
            cards: 5,
            textures: Vec::new(),
            card_size: vec2(150.0, 205.0),
            spread: 85.0,
            scatter: 0.0,
            bounce: 5.0,
            corner: 14.0,
            autoplay: false,
        }
    }
    /// Number of cards (1..=9).
    pub fn cards(mut self, n: usize) -> Self {
        self.cards = n.clamp(1, 9);
        self
    }
    /// Use your own photos (cycled if fewer than `cards`).
    pub fn textures(mut self, tex: Vec<TextureId>) -> Self {
        self.textures = tex;
        self
    }
    /// Card size in points.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card_size = s;
        self
    }
    /// How far the fan opens, 0..100.
    pub fn spread(mut self, s: f32) -> Self {
        self.spread = s.clamp(0.0, 100.0);
        self
    }
    /// Random tilt and offset per card, 0..100.
    pub fn scatter(mut self, s: f32) -> Self {
        self.scatter = s.clamp(0.0, 100.0);
        self
    }
    /// Spring overshoot, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Card corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Fan, lift and flip cards on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

struct FanPose {
    center: Pos2,
    angle: f32,
    scale: f32,
    flip: f32,
    size: Vec2,
}

impl FanPose {
    fn map(&self, local: Vec2) -> Pos2 {
        // local in 0..size; flip squashes x and adds a touch of perspective
        let c = local - self.size / 2.0;
        let fa = self.flip * TAU;
        let sx = fa.cos();
        let persp = 1.0 + 0.12 * fa.sin() * (c.x / (self.size.x / 2.0));
        let l = vec2(c.x * sx, c.y * persp) * self.scale;
        let (s, co) = self.angle.sin_cos();
        self.center + vec2(l.x * co - l.y * s, l.x * s + l.y * co)
    }
    fn contains(&self, p: Pos2) -> bool {
        let d = p - self.center;
        let (s, co) = (-self.angle).sin_cos();
        let l = vec2(d.x * co - d.y * s, d.x * s + d.y * co) / self.scale.max(0.01);
        l.x.abs() <= self.size.x / 2.0 && l.y.abs() <= self.size.y / 2.0
    }
}

impl egui::Widget for PhotoFan {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.cards;
        let size = vec2(ui.available_width(), self.card_size.y * 1.25 + 90.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let id = response.id;
        let center = rect.center() + vec2(0.0, 6.0);
        let dt = glowkit_core::dt(ui);
        let hover = response.hover_pos();
        let clicked = response.clicked();
        let interacting = hover.is_some();
        let idle = idle_for(ui, id, interacting);
        let s = self.spread / 100.0;
        let stiff = 240.0;
        let damp = damping_from_bounce(self.bounce, stiff);
        let card_size = self.card_size;
        let scatter = self.scatter / 100.0;

        // read state, compute poses (no ui calls inside with_state)
        let mut st: FanState = glowkit_core::state(ui, id.with("fan"), FanState::default);
        if st.cards.len() != n {
            st.cards = (0..n).map(|_| CardSpring::new()).collect();
            st.selected = None;
        }
        let pose_of = |st: &FanState, i: usize| -> FanPose {
            let c = st.cards[i];
            FanPose {
                center: center + vec2(c.x.value, c.y.value),
                angle: c.angle.value,
                scale: c.scale.value,
                flip: c.flip.value,
                size: card_size,
            }
        };
        let order = |st: &FanState| -> Vec<usize> {
            let mut o: Vec<usize> = (0..n).collect();
            if let Some(sel) = st.selected {
                o.retain(|&k| k != sel);
                o.push(sel);
            }
            o
        };
        // hit-test, topmost first
        let mut hovered = None;
        if let Some(p) = hover {
            for &i in order(&st).iter().rev() {
                if pose_of(&st, i).contains(p) {
                    hovered = Some(i);
                    break;
                }
            }
        }
        let near = hover.map(|p| (p - center).length() < card_size.y * 1.1).unwrap_or(false);
        let mut open_target: f32 = if near { 1.0 } else { 0.0 };
        if clicked {
            st.selected = match (hovered, st.selected) {
                (Some(h), Some(sel)) if h == sel => None,
                (Some(h), _) => Some(h),
                (None, _) => None,
            };
        }
        // autoplay timeline
        if self.autoplay && !interacting && idle > 1.2 {
            let cycle = 7.0;
            let p = (idle - 1.2) % cycle;
            let lap = ((idle - 1.2) / cycle) as u32;
            let phase = if p < 2.6 {
                0
            } else if p < 5.4 {
                1
            } else {
                2
            };
            match phase {
                0 => {
                    open_target = 1.0;
                    let k = ((p / 2.6) * n as f32).floor() as usize;
                    hovered = Some(k.min(n - 1));
                    st.selected = None;
                }
                1 => {
                    open_target = 1.0;
                    if st.last_phase != 1 {
                        st.loop_count = lap;
                    }
                    st.selected = Some((st.loop_count as usize * 2 + 1) % n);
                }
                _ => {
                    st.selected = None;
                }
            }
            st.last_phase = phase;
        } else {
            st.last_phase = -1;
        }
        if st.selected.is_some() {
            open_target = open_target.max(0.6);
        }
        st.open = glowkit_core::anim::approach(st.open, open_target, 7.0, dt);
        let open = st.open;
        let mut settled = true;
        for i in 0..n {
            let tt = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
            let mut r = Rng::new(i as u64 * 7919 + 3);
            let jitter_a = r.range(-1.0, 1.0) * 0.22 * scatter;
            let jitter = vec2(r.range(-1.0, 1.0), r.range(-1.0, 1.0)) * 26.0 * scatter;
            let fan = lerp(0.28, 1.0, open) * s;
            let mut x = tt * 175.0 * fan + jitter.x;
            let mut y = tt * tt * 70.0 * fan + jitter.y + 8.0;
            let mut a = tt * 0.62 * fan + jitter_a + 0.04;
            let mut sc = 1.0;
            let mut flip = 0.0;
            if st.selected == Some(i) {
                x = 0.0;
                y = -14.0;
                a = 0.0;
                sc = 1.22;
                flip = 1.0;
            } else if hovered == Some(i) && st.selected.is_none() {
                y -= 30.0;
                sc = 1.04;
                a *= 0.7;
            } else if st.selected.is_some() {
                // others drop back a little
                sc = 0.94;
                y += 6.0;
            }
            let c = &mut st.cards[i];
            c.x.target = x;
            c.y.target = y;
            c.angle.target = a;
            c.scale.target = sc;
            c.flip.target = flip;
            for sp in c.springs() {
                sp.stiffness = stiff;
                sp.damping = damp;
                sp.step(dt);
                settled &= sp.is_settled();
            }
            // flip uses a firmer, less bouncy spring so the turn reads cleanly
            c.flip.stiffness = 140.0;
            c.flip.damping = damping_from_bounce(self.bounce * 0.5, 140.0);
        }
        // flip returns to 0 by wrapping: once a deselected card is flipped >0.5 it spins forward
        for c in st.cards.iter_mut() {
            if c.flip.target == 0.0 && c.flip.value > 0.5 {
                c.flip.value -= 1.0;
            }
        }
        glowkit_core::set_state(ui, id.with("fan"), st.clone());
        if !settled || (open - open_target).abs() > 0.001 || self.autoplay {
            glowkit_core::animate(ui);
        }
        if hovered.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(40.0));
        let shadow_col = if t.dark { Color32::BLACK } else { Color32::from_rgb(40, 40, 70).gamma_multiply(0.5) };
        for &i in order(&st).iter() {
            let pose = pose_of(&st, i);
            let corner = self.corner.min(card_size.min_elem() * 0.3);
            let outline: Vec<Pos2> =
                glowkit_core::paint::rounded_rect_points(Rect::from_min_size(Pos2::ZERO, card_size), corner, 5)
                    .into_iter()
                    .map(|p| pose.map(p.to_vec2()))
                    .collect();
            let lift = (st.cards[i].scale.value - 1.0).max(0.0) * 4.0 + 0.6;
            polygon_shadow(
                &painter,
                &outline,
                pose.center,
                vec2(0.0, 7.0 * lift),
                7.0 * lift,
                shadow_col.gamma_multiply(0.55),
            );
            let front = (pose.flip * TAU).cos() >= 0.0;
            let shade = 0.82 + 0.18 * (pose.flip * TAU).cos().abs();
            let tint = Color32::from_gray((255.0 * shade) as u8);
            if front {
                let tex = if self.textures.is_empty() {
                    PaintedScene::texture(ui.ctx(), (i * 3 + 1) % PaintedScene::COUNT)
                } else {
                    self.textures[i % self.textures.len()]
                };
                let uv = cover_uv(PaintedScene::ASPECT, card_size.x / card_size.y);
                warp_card(&painter, card_size, corner, 3, Some(tex), uv, |l| pose.map(l), |_| tint);
            } else {
                let acc = PaintedScene::accent(i * 3 + 1);
                let back = mix(acc, Color32::from_rgb(20, 20, 30), 0.35);
                warp_card(
                    &painter,
                    card_size,
                    corner,
                    3,
                    None,
                    Rect::ZERO,
                    |l| pose.map(l),
                    |l| {
                        let u = (l.x + l.y) / 14.0;
                        let stripe = ((u * PI).sin() * 0.5 + 0.5).powi(6) * 0.25;
                        mix(back, Color32::WHITE, stripe).gamma_multiply(shade)
                    },
                );
            }
            // rim
            let mut rim = outline.clone();
            rim.push(rim[0]);
            painter.add(egui::Shape::line(rim, egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.35))));
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// ImageAccordion

/// A row of tall photo strips. Hover one and it opens wide while its neighbours give way,
/// sharing the space with a springy, slightly elastic feel; the open strip shows its caption.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::ImageAccordion;
/// ui.add(ImageAccordion::new().open(50.0).reach(35.0).gap(8.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ImageAccordion {
    items: Vec<(TextureId, String)>,
    count: usize,
    size: Vec2,
    open: f32,
    reach: f32,
    bounce: f32,
    gap: f32,
    corner: Option<f32>,
    autoplay: bool,
}

impl Default for ImageAccordion {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageAccordion {
    /// Five painted photos.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            count: 5,
            size: vec2(360.0, 300.0),
            open: 50.0,
            reach: 35.0,
            bounce: 10.0,
            gap: 8.0,
            corner: None,
            autoplay: false,
        }
    }
    /// Your own images with captions.
    pub fn items(mut self, items: Vec<(TextureId, String)>) -> Self {
        self.items = items;
        self
    }
    /// Number of painted placeholder strips when no items are given (2..=8).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.clamp(2, 8);
        self
    }
    /// Overall size in points.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// How wide the hovered strip opens, 0..100.
    pub fn open(mut self, v: f32) -> Self {
        self.open = v.clamp(0.0, 100.0);
        self
    }
    /// How much the neighbours of the hovered strip open too, 0..100.
    pub fn reach(mut self, v: f32) -> Self {
        self.reach = v.clamp(0.0, 100.0);
        self
    }
    /// Spring overshoot, 0..100.
    pub fn bounce(mut self, v: f32) -> Self {
        self.bounce = v.clamp(0.0, 100.0);
        self
    }
    /// Gap between strips in points.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Strip corner radius (default from the theme).
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = Some(r.max(0.0));
        self
    }
    /// Open strips one after another while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for ImageAccordion {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let items: Vec<(TextureId, String)> = if self.items.is_empty() {
            (0..self.count)
                .map(|i| {
                    let k = (i * 5 + 2) % PaintedScene::COUNT;
                    (PaintedScene::texture(ui.ctx(), k), PaintedScene::name(k).to_owned())
                })
                .collect()
        } else {
            self.items.clone()
        };
        let n = items.len().max(1);
        let w = self.size.x.min(ui.available_width());
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.size.y), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), vec2(w, self.size.y));
        let id = response.id;
        let hover = response.hover_pos().filter(|p| rect.contains(*p));
        let idle = idle_for(ui, id, hover.is_some());
        let total = rect.width() - self.gap * (n - 1) as f32;
        let widths_now: Vec<f32> = glowkit_core::state(ui, id.with("w"), || vec![total / n as f32; n]);
        let widths_now = if widths_now.len() == n { widths_now } else { vec![total / n as f32; n] };
        let mut hovered = None;
        if let Some(p) = hover {
            let mut x = rect.min.x;
            for (i, w) in widths_now.iter().enumerate() {
                if p.x >= x - self.gap / 2.0 && p.x < x + w + self.gap / 2.0 {
                    hovered = Some(i);
                }
                x += w + self.gap;
            }
        }
        if self.autoplay && hover.is_none() && idle > 1.0 {
            let k = ((idle - 1.0) / 1.6) as usize;
            let seq = [2usize, 3, 1, 4, 0];
            hovered = Some(seq[k % seq.len()] % n);
        }
        let open = 1.0 + self.open / 100.0 * 5.0;
        let reach = self.reach / 100.0;
        let weights: Vec<f32> = (0..n)
            .map(|i| match hovered {
                Some(h) => {
                    let d = (i as i32 - h as i32).unsigned_abs();
                    if d == 0 {
                        open
                    } else {
                        1.0 + (open - 1.0) * 0.45 * reach.powi(d as i32)
                    }
                }
                None => 1.0,
            })
            .collect();
        let sum: f32 = weights.iter().sum();
        let stiff = 220.0;
        let damp = damping_from_bounce(self.bounce, stiff);
        let mut widths = Vec::with_capacity(n);
        for (i, wgt) in weights.iter().enumerate() {
            let target = total * wgt / sum;
            widths.push(glowkit_core::anim::spring(ui, id.with(("strip", i)), target, stiff, damp).max(2.0));
        }
        // normalize so the row never jitters wider than its box
        let ws: f32 = widths.iter().sum();
        for w in widths.iter_mut() {
            *w *= total / ws.max(1.0);
        }
        glowkit_core::set_state(ui, id.with("w"), widths.clone());
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        if hovered.is_some() && hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let radius = self.corner.unwrap_or(t.radius * 1.1);
        let mut x = rect.min.x;
        let max_w = total * open / (open + (n - 1) as f32);
        for (i, w) in widths.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(*w, rect.height()));
            // window into the photo: image height fills the strip, centered horizontally
            let img_w = rect.height() * PaintedScene::ASPECT;
            let frac = (w / img_w).min(1.0);
            let uv = Rect::from_min_size(pos2(0.5 - frac / 2.0, 0.0), vec2(frac, 1.0));
            image_rect(painter, r, radius.min(w / 2.0), items[i].0, uv, Color32::WHITE);
            let show = ((w - total / n as f32) / (max_w - total / n as f32).max(1.0)).clamp(0.0, 1.0);
            if show > 0.02 {
                let shade_r = Rect::from_min_max(pos2(r.min.x, r.max.y - 64.0), r.max);
                glowkit_core::paint::fill_convex_fn(
                    painter,
                    &glowkit_core::paint::rounded_rect_points(shade_r, radius.min(w / 2.0).min(20.0), 4),
                    3,
                    glowkit_core::paint::linear_fn(
                        shade_r,
                        90.0,
                        &[Color32::TRANSPARENT, Color32::from_black_alpha((120.0 * show) as u8)],
                    ),
                );
                let a = ease(Easing::OutCubic, show);
                painter.text(
                    pos2(r.min.x + 14.0, r.max.y - 16.0 + (1.0 - a) * 8.0),
                    Align2::LEFT_BOTTOM,
                    &items[i].1,
                    FontId::proportional(14.0),
                    with_alpha(Color32::WHITE, a),
                );
            }
            x += w + self.gap;
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------
// GenerateCard

const ART_W: usize = 192;
const ART_H: usize = 256;
const ART_LEVELS: usize = 5;

fn art_pixel(seed: u32, u: f32, v: f32) -> [f32; 3] {
    // a flowing "ink bloom": a petal shape with domain-warped edges over grainy paper
    let palettes: [[[f32; 3]; 3]; 4] = [
        [[0.12, 0.12, 0.14], [0.36, 0.36, 0.40], [0.93, 0.92, 0.90]],
        [[0.93, 0.33, 0.30], [0.99, 0.72, 0.42], [0.98, 0.95, 0.91]],
        [[0.26, 0.30, 0.86], [0.55, 0.80, 0.98], [0.94, 0.95, 0.99]],
        [[0.10, 0.45, 0.33], [0.85, 0.80, 0.35], [0.96, 0.96, 0.92]],
    ];
    let pal = palettes[seed as usize % palettes.len()];
    let sf = seed as f32 * 3.17;
    let x = (u - 0.5) * 2.0;
    let y = (v - 0.5) * 2.0 * (ART_H as f32 / ART_W as f32);
    let wx = x + 0.28 * noise2(x * 1.6 + sf, y * 1.6);
    let wy = y + 0.28 * noise2(x * 1.6 - 7.0, y * 1.6 + sf);
    let r = (wx * wx + wy * wy).sqrt();
    let a = wy.atan2(wx);
    let petals = 5.0 + (seed % 3) as f32;
    let arm = (a * petals + sf + 0.6 * noise2(r * 2.0, sf)).sin() * 0.5 + 0.5;
    let reach = 0.32 + 0.55 * arm.powf(2.2);
    let edge = ((reach - r) / 0.06).clamp(-1.0, 1.0) * 0.5 + 0.5;
    let core = (1.0 - r / reach.max(0.01)).clamp(0.0, 1.0);
    let grain = noise2(u * 90.0, v * 90.0) * 0.035 + noise2(u * 9.0 + sf, v * 9.0) * 0.03;
    let ink = [
        pal[0][0] + (pal[1][0] - pal[0][0]) * (1.0 - core).powi(2),
        pal[0][1] + (pal[1][1] - pal[0][1]) * (1.0 - core).powi(2),
        pal[0][2] + (pal[1][2] - pal[0][2]) * (1.0 - core).powi(2),
    ];
    let paper = pal[2];
    let mut c = [0.0; 3];
    for k in 0..3 {
        c[k] = (paper[k] + (ink[k] - paper[k]) * edge + grain).clamp(0.0, 1.0);
    }
    c
}

/// Painted artwork for `seed` at several blur levels (index 0 = sharp), generated once.
fn art_levels(ctx: &Context, seed: u32) -> [TextureId; ART_LEVELS] {
    let key = Id::unique(("glowkit_micro_art", seed));
    if let Some(h) = ctx.data(|d| d.get_temp::<Vec<egui::TextureHandle>>(key)) {
        let mut out = [TextureId::default(); ART_LEVELS];
        for (o, t) in out.iter_mut().zip(h.iter()) {
            *o = t.id();
        }
        return out;
    }
    let mut base = vec![[0.0f32; 3]; ART_W * ART_H];
    for y in 0..ART_H {
        for x in 0..ART_W {
            base[y * ART_W + x] = art_pixel(seed, (x as f32 + 0.5) / ART_W as f32, (y as f32 + 0.5) / ART_H as f32);
        }
    }
    let to_img = |px: &[[f32; 3]], w: usize, h: usize| {
        let bytes: Vec<u8> =
            px.iter().flat_map(|c| [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]).collect();
        ColorImage::from_rgba_unmultiplied([w, h], &bytes)
    };
    let mut handles = Vec::with_capacity(ART_LEVELS);
    let (mut cur, mut w, mut h) = (base, ART_W, ART_H);
    for lvl in 0..ART_LEVELS {
        handles.push(ctx.load_texture(
            format!("glowkit-micro-art-{seed}-{lvl}"),
            to_img(&cur, w, h),
            egui::TextureOptions::LINEAR,
        ));
        if lvl + 1 < ART_LEVELS {
            let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
            let mut next = vec![[0.0f32; 3]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    let mut s = [0.0; 3];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = cur[(y * 2 + dy).min(h - 1) * w + (x * 2 + dx).min(w - 1)];
                        for k in 0..3 {
                            s[k] += p[k] * 0.25;
                        }
                    }
                    next[y * nw + x] = s;
                }
            }
            cur = next;
            w = nw;
            h = nh;
        }
    }
    let mut out = [TextureId::default(); ART_LEVELS];
    for (o, t) in out.iter_mut().zip(handles.iter()) {
        *o = t.id();
    }
    ctx.data_mut(|d| d.insert_temp(key, handles));
    out
}

#[derive(Clone, Copy, Default)]
struct GenState {
    start: Option<f32>,
    seed: u32,
    done: bool,
}

/// A blank card with a **Generate** button. Press it and an image resolves under frosted glass,
/// broad shapes first: the picture sharpens through blur levels while a ripple of light runs
/// over the glass and a percentage ticks up. When it is done the button floats over the result
/// to generate the next one.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::GenerateCard;
/// ui.add(GenerateCard::new().pace(5.0).ripple(50.0).corner(20.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GenerateCard {
    size: Vec2,
    pace: f32,
    ripple: f32,
    corner: f32,
    stroke: bool,
    label: String,
    autoplay: bool,
}

impl Default for GenerateCard {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerateCard {
    /// A 270 x 330 card.
    pub fn new() -> Self {
        Self {
            size: vec2(250.0, 310.0),
            pace: 5.0,
            ripple: 50.0,
            corner: 20.0,
            stroke: false,
            label: "Generate".into(),
            autoplay: false,
        }
    }
    /// Card size in points.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Seconds a generation takes.
    pub fn pace(mut self, s: f32) -> Self {
        self.pace = s.max(0.3);
        self
    }
    /// Strength of the glass ripple while resolving, 0..100.
    pub fn ripple(mut self, r: f32) -> Self {
        self.ripple = r.clamp(0.0, 100.0);
        self
    }
    /// Corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Draw a hairline border.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Button label.
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// Press the button on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for GenerateCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, outer_resp) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.size.y + 10.0), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), self.size);
        let id = outer_resp.id;
        let now = glowkit_core::time(ui);
        let font = FontId::proportional(14.5);
        let galley = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), t.foreground);
        let btn = Rect::from_center_size(rect.center(), galley.size() + vec2(36.0, 20.0));
        let st: GenState = glowkit_core::state(ui, id, GenState::default);
        let running = st.start.is_some() && !st.done;
        let resp = ui.interact(btn, id.with("btn"), if running { Sense::hover() } else { Sense::click() });
        let card_resp = ui.interact(rect, id.with("card"), Sense::hover());
        let idle = idle_for(ui, id, card_resp.hovered() || running);
        let mut st = st;
        let auto_go = self.autoplay && !running && idle > if st.done { 2.2 } else { 0.8 };
        if (resp.clicked() || auto_go) && !running {
            if st.start.is_some() {
                st.seed = st.seed.wrapping_add(1);
            }
            st.start = Some(now);
            st.done = false;
        }
        let progress = st.start.map(|s| ((now - s) / self.pace).clamp(0.0, 1.0)).unwrap_or(0.0);
        if st.start.is_some() && progress >= 1.0 {
            st.done = true;
        }
        glowkit_core::set_state(ui, id, st);
        let running = st.start.is_some() && !st.done;
        if running || self.autoplay {
            glowkit_core::animate(ui);
        }
        let press = glowkit_core::anim::spring(
            ui,
            id.with("press"),
            if resp.is_pointer_button_down_on() {
                0.92
            } else if resp.hovered() {
                1.04
            } else {
                1.0
            },
            400.0,
            18.0,
        );
        let btn_alpha = glowkit_core::anim::follow(ui, id.with("btn_a"), if running { 0.0 } else { 1.0 }, 10.0);
        if resp.hovered() && !running {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter();
        soft_card(painter, &t, rect, self.corner, 1.0, self.stroke);
        if st.start.is_some() {
            let levels = art_levels(ui.ctx(), st.seed % 4);
            // broad shapes first: walk from the blurriest level to the sharp one
            let p = ease(Easing::InOutSine, progress);
            let lv = (1.0 - p) * (ART_LEVELS - 1) as f32;
            let lo = lv.floor() as usize;
            let frac = lv - lo as f32;
            let reveal = (progress / 0.15).clamp(0.0, 1.0);
            let amp = self.ripple / 100.0 * 7.0 * (1.0 - p);
            let size = rect.size();
            let center = rect.center();
            let map = |l: Vec2| {
                let c = l - size / 2.0;
                let d = c.length();
                let w = (d / 22.0 - now * 5.0).sin() * amp;
                let dir = if d > 0.1 { c / d } else { Vec2::ZERO };
                center + c + dir * w * (d / size.max_elem()).min(1.0)
            };
            let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            let alpha_lo = (reveal * 255.0) as u8;
            let tint_lo = Color32::from_white_alpha(alpha_lo);
            warp_card(painter, size, self.corner, 4, Some(levels[(lo + 1).min(ART_LEVELS - 1)]), full, map, |_| {
                tint_lo
            });
            let tint_hi = Color32::from_white_alpha(((1.0 - frac) * reveal * 255.0) as u8);
            warp_card(painter, size, self.corner, 4, Some(levels[lo]), full, map, |_| tint_hi);
            // frosted glass + ripple of light
            let frost = (1.0 - p).powf(1.5) * 0.55;
            if frost > 0.01 {
                let glass = if t.dark { Color32::from_gray(40) } else { Color32::WHITE };
                painter.rect_filled(rect, cr(self.corner), with_alpha(glass, frost));
                let band = (now * 0.6).fract() * 1.6 - 0.3;
                warp_card(
                    painter,
                    size,
                    self.corner,
                    1,
                    None,
                    Rect::ZERO,
                    |l| rect.min + l,
                    |l| {
                        let u = (l.x / size.x + l.y / size.y) * 0.5;
                        let k = (1.0 - ((u - band) / 0.12).abs()).clamp(0.0, 1.0);
                        with_alpha(Color32::WHITE, k * k * 0.35 * frost / 0.55 * (self.ripple / 100.0 + 0.3))
                    },
                );
            }
            if running {
                let pct = format!("{}%", (progress * 100.0) as u32);
                let g = painter.layout_no_wrap(pct, FontId::proportional(11.5), t.foreground);
                let br = Rect::from_min_size(
                    pos2(rect.min.x + 12.0, rect.max.y - 12.0 - 22.0),
                    vec2(g.size().x + 16.0, 22.0),
                );
                painter.rect_filled(br, cr(11.0), with_alpha(surface(&t), 0.92));
                painter.galley(br.center() - g.size() / 2.0, g, t.foreground);
            }
        }
        if btn_alpha > 0.01 {
            let b = Rect::from_center_size(btn.center(), btn.size() * press);
            let fill = if st.start.is_some() { surface(&t) } else { track(&t) };
            if st.start.is_some() {
                glowkit_core::paint::soft_shadow(
                    painter,
                    b,
                    b.height() / 2.0,
                    12.0,
                    vec2(0.0, 3.0),
                    Color32::from_black_alpha((40.0 * btn_alpha) as u8),
                );
            }
            painter.rect_filled(b, cr(b.height() / 2.0), with_alpha(fill, btn_alpha));
            painter.text(
                b.center(),
                Align2::CENTER_CENTER,
                &self.label,
                FontId::proportional(14.5 * press),
                with_alpha(t.foreground, btn_alpha),
            );
        }
        resp
    }
}
