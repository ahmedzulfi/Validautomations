//! Testimonial decks: a fanned quote deck and a story deck of throwable portrait cards.

use super::common::{
    cover_uv, display_font, drive_snap, galley_mapped, idle_time, lerp_c, poly_shadow, semibold_font, surface_grid,
    v3, wall_dt, Cam,
};
use egui::{pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Response, Sense, Stroke, TextureId, Ui, Vec2, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Spring, Theme};
use glowkit_effects::PaintedScene;
use std::f32::consts::TAU;

fn initials(name: &str) -> String {
    name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect()
}

fn round_btn(painter: &egui::Painter, c: Pos2, r: f32, dark: bool, dir: f32, t: &Theme, hover: bool) {
    let (bg, fg) = if dark { (t.primary, t.primary_foreground) } else { (t.card, t.foreground) };
    painter.circle_filled(c, r + if hover { 1.5 } else { 0.0 }, bg);
    if !dark {
        painter.circle_stroke(c, r, Stroke::new(1.0, t.border));
    }
    let s = r * 0.32;
    painter.line(vec![c + vec2(-s * 0.45 * dir, -s), c + vec2(s * 0.45 * dir, 0.0), c + vec2(-s * 0.45 * dir, s)], Stroke::new(1.8, fg));
}

// ---------------------------------------------------------------------------------------------

/// One testimonial of a [`QuoteDeck`].
#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    /// The quote itself.
    pub text: String,
    /// Who said it.
    pub name: String,
    /// Their role.
    pub role: String,
    /// Company or project, shown small at the top of the card.
    pub company: String,
    /// Star rating out of five.
    pub stars: u8,
    /// Card tint (defaults cycle through soft pastels).
    pub tint: Option<Color32>,
}

impl Quote {
    /// A five-star quote.
    pub fn new(text: impl Into<String>, name: impl Into<String>, role: impl Into<String>, company: impl Into<String>) -> Self {
        Self { text: text.into(), name: name.into(), role: role.into(), company: company.into(), stars: 5, tint: None }
    }
    /// Star rating out of five.
    pub fn stars(mut self, s: u8) -> Self {
        self.stars = s.min(5);
        self
    }
    /// Card tint.
    pub fn tint(mut self, c: Color32) -> Self {
        self.tint = Some(c);
        self
    }
}

/// A testimonial deck fanned across the width: the focused quote stands upright and the rest
/// lean away in 3D, shrinking and dimming. Autoplays and loops; drag, wheel, the arrow buttons
/// and the arrow keys step through it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{Quote, QuoteDeck};
/// let quotes = vec![Quote::new("Every detail was argued for.", "Mira Okafor", "Design lead", "Northwind")];
/// ui.add(QuoteDeck::new(&quotes).title("Five teams, five honest reviews."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct QuoteDeck<'a> {
    quotes: &'a [Quote],
    eyebrow: String,
    title: String,
    height: f32,
    card: Vec2,
    lean: f32,
    autoplay: f32,
}

impl<'a> QuoteDeck<'a> {
    /// A deck of `quotes`.
    pub fn new(quotes: &'a [Quote]) -> Self {
        Self {
            quotes,
            eyebrow: "FIELD NOTES".into(),
            title: "Five teams, five honest reviews.".into(),
            height: 440.0,
            card: vec2(300.0, 250.0),
            lean: 1.0,
            autoplay: 3.6,
        }
    }
    /// Small caps line above the title.
    pub fn eyebrow(mut self, s: impl Into<String>) -> Self {
        self.eyebrow = s.into();
        self
    }
    /// Heading.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(300.0);
        self
    }
    /// Card size.
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(160.0, 140.0));
        self
    }
    /// How far the side cards lean away, `0..=2`.
    pub fn lean(mut self, k: f32) -> Self {
        self.lean = k.clamp(0.0, 2.0);
        self
    }
    /// Seconds between autoplay steps (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
}

const PASTELS: [&str; 6] = ["#DCE8D2", "#DDD8F3", "#F3DDD3", "#F5EDC6", "#D6E6F2", "#EED9E8"];

impl Widget for QuoteDeck<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let n = self.quotes.len().max(1);
        let prev_b = pos2(rect.max.x - 76.0, rect.min.y + 40.0);
        let next_b = pos2(rect.max.x - 30.0, rect.min.y + 40.0);
        let hp = response.hover_pos();
        let over = |c: Pos2| hp.is_some_and(|p| p.distance(c) < 19.0);
        let mut nudge = 0.0;
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                if p.distance(prev_b) < 19.0 {
                    nudge = -1.0;
                } else if p.distance(next_b) < 19.0 {
                    nudge = 1.0;
                }
            }
        }
        if over(prev_b) || over(next_b) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let pos = drive_snap(ui, id, &response, 200.0, self.autoplay, nudge);
        if !ui.is_rect_visible(rect) || self.quotes.is_empty() {
            return response;
        }
        let painter = ui.painter_at(rect);
        // header
        let hx = rect.min.x + 28.0;
        painter.text(pos2(hx, rect.min.y + 20.0), Align2::LEFT_TOP, &self.eyebrow, FontId::monospace(11.0), t.muted_foreground);
        let title_g = painter.layout(self.title.clone(), display_font(ui, 30.0), t.foreground, (rect.width() * 0.5).max(240.0));
        painter.galley(pos2(hx, rect.min.y + 40.0), title_g.clone(), t.foreground);
        round_btn(&painter, prev_b, 18.0, true, -1.0, &t, over(prev_b));
        round_btn(&painter, next_b, 18.0, true, 1.0, &t, over(next_b));
        // cards
        let cy = rect.min.y + 52.0 + title_g.size().y + (self.height - 110.0 - title_g.size().y) / 2.0 + 8.0;
        let cam = Cam { center: pos2(rect.center().x, cy), focal: 1000.0 };
        let size = self.card;
        let lo = (pos - 3.0).floor() as i32;
        let hi = (pos + 3.0).ceil() as i32;
        let mut order: Vec<i32> = (lo..=hi).collect();
        order.sort_by(|a, b| {
            let da = (*a as f32 - pos).abs();
            let db = (*b as f32 - pos).abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let quote_f = FontId::proportional(18.0);
        let small = FontId::proportional(11.5);
        let name_f = semibold_font(ui, 12.5);
        for i in order {
            let d = i as f32 - pos;
            if d.abs() > 3.2 {
                continue;
            }
            let q = &self.quotes[i.rem_euclid(n as i32) as usize];
            let ad = d.abs();
            let x = d.signum() * (ad.min(1.0) * (size.x * 0.86) + (ad - 1.0).max(0.0) * size.x * 0.52);
            let z = ad.min(2.5) * 110.0;
            let yaw = -d.clamp(-1.6, 1.6) * 0.42 * self.lean;
            let roll = d.clamp(-2.0, 2.0) * 0.035 * self.lean;
            let map3 = |l: Vec2| v3(l.x - size.x / 2.0, l.y - size.y / 2.0, 0.0).rot_z(roll).rot_y(yaw).add(v3(x, ad.min(2.0) * 8.0, z));
            let map = |l: Vec2| cam.pt(map3(l));
            let tint = q.tint.unwrap_or_else(|| glowkit_core::hex(PASTELS[i.rem_euclid(PASTELS.len() as i32) as usize]));
            let base = if t.dark { lerp_c(hex("#1C1C21"), tint, 0.18) } else { tint };
            let dim = (1.0 - ad * 0.16).clamp(0.45, 1.0);
            let fade = (1.0 - (ad - 2.2).max(0.0) / 1.0).clamp(0.0, 1.0);
            let outline: Vec<Pos2> =
                [vec2(0.0, 0.0), vec2(size.x, 0.0), vec2(size.x, size.y), vec2(0.0, size.y)].iter().map(|l| map(*l)).collect();
            let sc = if t.dark { Color32::from_black_alpha(130) } else { Color32::from_rgba_unmultiplied(40, 40, 30, 36) };
            poly_shadow(&painter, &outline, vec2(0.0, 14.0), 18.0, sc.gamma_multiply(fade));
            let bg = t.background;
            surface_grid(&painter, None, Rect::ZERO, 4, 4, size, 18.0, |u, v| map(vec2(u * size.x, v * size.y)), |_, _| {
                lerp_c(bg, base, dim * fade)
            });
            let ink = if t.dark { hex("#F2F1EC") } else { hex("#1D1D1F") };
            let ink = lerp_c(bg, ink, dim * fade);
            let muted = lerp_c(bg, with_alpha(ink, 0.6), 0.9);
            let g = painter.layout_no_wrap(q.company.clone(), small.clone(), ink);
            galley_mapped(&painter, &g, with_alpha(ink, 0.75), |l| map(vec2(20.0, 18.0) + l));
            let stars: String = (0..5).map(|k| if k < q.stars { '★' } else { '☆' }).collect();
            let gs = painter.layout_no_wrap(stars, FontId::proportional(11.0), ink);
            let sw = gs.size().x;
            galley_mapped(&painter, &gs, with_alpha(ink, 0.8), |l| map(vec2(size.x - 20.0 - sw, 18.0) + l));
            let mut job = egui::text::LayoutJob::simple(format!("“{}”", q.text), quote_f.clone(), ink, size.x - 44.0);
            job.halign = Align::Center;
            let gq = painter.layout_job(job);
            galley_mapped(&painter, &gq, ink, |l| map(vec2(size.x / 2.0, 48.0) + l));
            // divider + person
            let a = map(vec2(20.0, size.y - 62.0));
            let b = map(vec2(size.x - 20.0, size.y - 62.0));
            painter.line_segment([a, b], Stroke::new(1.0, with_alpha(ink, 0.55)));
            let ac = map(vec2(36.0, size.y - 32.0));
            let s = (map(vec2(1.0, 0.0)) - map(vec2(0.0, 0.0))).length();
            painter.circle_filled(ac, 15.0 * s, lerp_c(base, ink, 0.25));
            painter.text(ac, Align2::CENTER_CENTER, initials(&q.name), FontId::proportional(10.5 * s), with_alpha(ink, 0.85));
            let gn = painter.layout_no_wrap(q.name.clone(), name_f.clone(), ink);
            galley_mapped(&painter, &gn, ink, |l| map(vec2(60.0, size.y - 46.0) + l));
            let gr = painter.layout_no_wrap(q.role.clone(), small.clone(), ink);
            galley_mapped(&painter, &gr, muted, |l| map(vec2(60.0, size.y - 28.0) + l));
        }
        // counter + dots
        let idx = pos.round().rem_euclid(n as f32) as usize;
        let by = rect.max.y - 26.0;
        painter.text(pos2(hx, by), Align2::LEFT_CENTER, format!("{:02} / {:02}", idx + 1, n), FontId::monospace(11.0), t.muted_foreground);
        for k in 0..n {
            let on = glowkit_core::anim::spring(ui, id.with(("dot", k)), if k == idx { 1.0 } else { 0.0 }, 300.0, 24.0);
            let w = 5.0 + 11.0 * on;
            let x = hx + 64.0 + k as f32 * 12.0 + if k > idx { 11.0 } else { 0.0 } * 1.0;
            painter.rect_filled(
                Rect::from_min_size(pos2(x, by - 2.5), vec2(w, 5.0)),
                CornerRadius::same(3),
                with_alpha(t.foreground, 0.25 + 0.6 * on),
            );
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// One customer story of a [`StoryDeck`].
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    /// The quote.
    pub quote: String,
    /// Who said it.
    pub name: String,
    /// Role and company.
    pub role: String,
    /// A headline metric, e.g. "+38%".
    pub metric: String,
    /// What the metric measures.
    pub metric_label: String,
    /// Portrait; `None` uses a painted scene.
    pub texture: Option<TextureId>,
}

impl Story {
    /// A story with a quote, a person and a metric.
    pub fn new(
        quote: impl Into<String>,
        name: impl Into<String>,
        role: impl Into<String>,
        metric: impl Into<String>,
        metric_label: impl Into<String>,
    ) -> Self {
        Self {
            quote: quote.into(),
            name: name.into(),
            role: role.into(),
            metric: metric.into(),
            metric_label: metric_label.into(),
            texture: None,
        }
    }
    /// Portrait texture.
    pub fn texture(mut self, t: TextureId) -> Self {
        self.texture = Some(t);
        self
    }
}

#[derive(Clone, Copy)]
struct DeckState {
    index: usize,
    drag: Vec2,
    fly: Option<(Vec2, Vec2, f32)>,
    back_x: Spring,
    back_y: Spring,
    timer: f32,
    text_t: f32,
    prev: usize,
}

/// Customer stories beside a deck of portrait cards you can throw. Fling the top card and it
/// flies off while the next story slides in; autoplay runs with a progress ring around the
/// next button.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{Story, StoryDeck};
/// let stories = vec![Story::new("We shipped in half the time.", "Ana Ruiz", "CTO, Brightline", "2x", "faster releases")];
/// ui.add(StoryDeck::new(&stories));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StoryDeck<'a> {
    stories: &'a [Story],
    height: f32,
    autoplay: f32,
    accent: Option<Color32>,
}

impl<'a> StoryDeck<'a> {
    /// A deck of `stories`.
    pub fn new(stories: &'a [Story]) -> Self {
        Self { stories, height: 400.0, autoplay: 5.0, accent: None }
    }
    /// Widget height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(280.0);
        self
    }
    /// Seconds per story in autoplay (0 = off).
    pub fn autoplay(mut self, secs: f32) -> Self {
        self.autoplay = secs.max(0.0);
        self
    }
    /// Accent for the metric and progress ring (default: theme accent).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl Widget for StoryDeck<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let n = self.stories.len().max(1);
        let wide = rect.width() > 620.0;
        let deck_c = if wide {
            pos2(rect.max.x - rect.width() * 0.27, rect.center().y)
        } else {
            pos2(rect.center().x, rect.min.y + self.height * 0.36)
        };
        let card = vec2(200.0, 260.0).min(vec2(rect.width() * 0.4, self.height * 0.72));
        let top_rect = Rect::from_center_size(deck_c, card);
        let btn_y = rect.max.y - 44.0;
        let prev_b = pos2(rect.min.x + 52.0, btn_y);
        let next_b = pos2(rect.min.x + 106.0, btn_y);
        let hp = response.hover_pos();
        let over = |c: Pos2| hp.is_some_and(|p| p.distance(c) < 21.0);
        let press = response.interact_pointer_pos();
        let on_card = press.is_some_and(|p| top_rect.expand(20.0).contains(p));
        let dragging = response.dragged() && on_card;
        let drag_d = if dragging { response.drag_delta() } else { Vec2::ZERO };
        let released = response.drag_stopped();
        let vel = ui.input(|i| i.pointer.velocity());
        let mut click_dir = 0i32;
        if response.clicked() {
            if let Some(p) = press {
                if p.distance(prev_b) < 21.0 {
                    click_dir = -1;
                } else if p.distance(next_b) < 21.0 {
                    click_dir = 1;
                }
            }
        }
        if over(prev_b) || over(next_b) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        } else if hp.is_some_and(|p| top_rect.contains(p)) {
            ui.ctx().set_cursor_icon(if dragging { CursorIcon::Grabbing } else { CursorIcon::Grab });
        }
        let idle = idle_time(ui, id, response.hovered());
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let auto = self.autoplay;
        let st = glowkit_core::with_state(
            ui,
            id.with("story_deck"),
            || DeckState {
                index: 0,
                drag: Vec2::ZERO,
                fly: None,
                back_x: Spring::new(0.0).with(260.0, 20.0),
                back_y: Spring::new(0.0).with(260.0, 20.0),
                timer: 0.0,
                text_t: 1.0,
                prev: 0,
            },
            |s| {
                s.text_t += wdt;
                let throw = |s: &mut DeckState, v: Vec2| {
                    s.fly = Some((s.drag, v, 0.0));
                    s.drag = Vec2::ZERO;
                    s.back_x.value = 0.0;
                    s.back_y.value = 0.0;
                    s.prev = s.index;
                    s.index = (s.index + 1) % n;
                    s.text_t = 0.0;
                    s.timer = 0.0;
                };
                if dragging {
                    s.drag += drag_d;
                    s.timer = 0.0;
                } else if released && s.drag != Vec2::ZERO {
                    if s.drag.length() > 110.0 || vel.length() > 700.0 {
                        let v = if vel.length() > 300.0 { vel } else { s.drag.normalized() * 1600.0 };
                        throw(s, v);
                    } else {
                        s.back_x.value = s.drag.x;
                        s.back_y.value = s.drag.y;
                        s.drag = Vec2::ZERO;
                    }
                }
                if click_dir == 1 {
                    throw(s, vec2(-1700.0, -200.0));
                } else if click_dir == -1 {
                    s.prev = s.index;
                    s.index = (s.index + n - 1) % n;
                    s.text_t = 0.0;
                    s.timer = 0.0;
                }
                if auto > 0.0 && idle > 1.0 && s.fly.is_none() && !dragging {
                    s.timer += wdt;
                    if s.timer > auto {
                        throw(s, vec2(-1800.0, -260.0));
                    }
                }
                if let Some((p, v, age)) = s.fly.as_mut() {
                    *p += *v * dt;
                    *age += wdt;
                    if *age > 0.7 {
                        s.fly = None;
                    }
                }
                s.back_x.target = 0.0;
                s.back_y.target = 0.0;
                s.back_x.step(dt);
                s.back_y.step(dt);
                *s
            },
        );
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) || self.stories.is_empty() {
            return response;
        }
        let painter = ui.painter_at(rect);
        let accent = self.accent.unwrap_or(t.accent);
        // ----- story copy
        let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, (st.text_t / 0.6).min(1.0));
        let text_w = if wide { rect.width() * 0.46 } else { rect.width() - 48.0 };
        let tx = rect.min.x + 32.0;
        let ty = if wide { rect.min.y + 40.0 } else { rect.min.y + self.height * 0.72 };
        let draw_story = |s: &Story, a: f32, dx: f32| {
            if a <= 0.01 {
                return;
            }
            let x = tx + dx;
            painter.text(pos2(x, ty), Align2::LEFT_TOP, &s.metric, display_font(ui, 34.0), with_alpha(accent, a));
            painter.text(pos2(x, ty + 42.0), Align2::LEFT_TOP, &s.metric_label, FontId::proportional(12.5), with_alpha(t.muted_foreground, a));
            let g = painter.layout(format!("“{}”", s.quote), display_font(ui, 21.0), with_alpha(t.foreground, a), text_w);
            let gh = g.size().y;
            painter.galley(pos2(x, ty + 76.0), g, with_alpha(t.foreground, a));
            painter.text(pos2(x, ty + 92.0 + gh), Align2::LEFT_TOP, &s.name, semibold_font(ui, 14.0), with_alpha(t.foreground, a));
            painter.text(pos2(x, ty + 112.0 + gh), Align2::LEFT_TOP, &s.role, FontId::proportional(12.5), with_alpha(t.muted_foreground, a));
        };
        if wide {
            if e < 1.0 {
                draw_story(&self.stories[st.prev % n], 1.0 - e, -24.0 * e);
            }
            draw_story(&self.stories[st.index % n], e, 30.0 * (1.0 - e));
        }
        // ----- deck
        let tex_of = |k: usize| -> (TextureId, Rect) {
            match self.stories[k % n].texture {
                Some(tex) => (tex, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0))),
                None => (PaintedScene::texture(ui.ctx(), [5, 0, 4, 1, 7, 3, 6, 2][k % 8]), cover_uv(PaintedScene::ASPECT, card.x / card.y)),
            }
        };
        let frame = if t.dark { hex("#EDEBE6") } else { Color32::WHITE };
        let sh = if t.dark { Color32::from_black_alpha(150) } else { Color32::from_rgba_unmultiplied(40, 30, 20, 50) };
        let draw_card = |k: usize, offset: Vec2, rot: f32, scale: f32, alpha: f32| {
            let (tex, uv) = tex_of(k);
            let (s, c) = rot.sin_cos();
            let ctr = deck_c + offset;
            let map = |l: Vec2| {
                let d = (l - card / 2.0) * scale;
                ctr + vec2(d.x * c - d.y * s, d.x * s + d.y * c)
            };
            let outline: Vec<Pos2> = [vec2(0.0, 0.0), vec2(card.x, 0.0), vec2(card.x, card.y), vec2(0.0, card.y)].iter().map(|l| map(*l)).collect();
            poly_shadow(&painter, &outline, vec2(0.0, 16.0), 20.0, sh.gamma_multiply(alpha));
            surface_grid(&painter, None, Rect::ZERO, 2, 2, card, 18.0, |u, v| map(vec2(u * card.x, v * card.y)), |_, _| frame.gamma_multiply(alpha));
            let inner = Rect::from_min_max(pos2(7.0, 7.0), (card - vec2(7.0, 7.0)).to_pos2());
            surface_grid(
                &painter,
                Some(tex),
                uv,
                2,
                2,
                inner.size(),
                13.0,
                |u, v| map(inner.min.to_vec2() + vec2(u * inner.width(), v * inner.height())),
                |_, _| Color32::WHITE.gamma_multiply(alpha),
            );
        };
        // stacked cards underneath (furthest first)
        for depth in (1..4usize).rev() {
            let k = st.index + depth;
            let fan = [0.0f32, 0.055, -0.045, 0.07][depth];
            let off = vec2(depth as f32 * 10.0, depth as f32 * 2.0);
            draw_card(k, off, fan, 1.0 - depth as f32 * 0.04, 1.0);
        }
        // top card (dragged or springing back)
        let drag = st.drag + vec2(st.back_x.value, st.back_y.value);
        draw_card(st.index, drag, drag.x / 900.0, 1.0, 1.0);
        // the thrown card
        if let Some((p, v, age)) = st.fly {
            let a = (1.0 - age / 0.7).clamp(0.0, 1.0);
            draw_card(st.prev, p, p.x / 600.0 + v.x.signum() * age * 0.8, 1.0 + age * 0.05, a);
        }
        // hint
        painter.text(
            pos2(deck_c.x, top_rect.max.y + 26.0),
            Align2::CENTER_CENTER,
            "↔  drag to throw",
            FontId::proportional(11.5),
            with_alpha(t.muted_foreground, 0.8),
        );
        // ----- buttons with autoplay ring
        round_btn(&painter, prev_b, 19.0, false, -1.0, &t, over(prev_b));
        round_btn(&painter, next_b, 19.0, true, 1.0, &t, over(next_b));
        if self.autoplay > 0.0 {
            let k = (st.timer / self.autoplay).clamp(0.0, 1.0);
            paint::arc(&painter, next_b, 24.0, -TAU / 4.0, -TAU / 4.0 + TAU * k, 2.2, accent);
        }
        response
    }
}
