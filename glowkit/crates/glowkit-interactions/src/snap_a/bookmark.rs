use super::common::{idle_time, lerp_c, wall_dt};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::{hex, paint, Theme};

/// One card of a [`CardBookmark`] row.
#[derive(Clone, Debug)]
pub struct BookmarkCard {
    /// Big title under the art.
    pub title: String,
    /// Line printed over the art.
    pub tagline: String,
    /// Small brand mark on the tab line.
    pub brand: String,
    /// Card color.
    pub color: Color32,
}

impl BookmarkCard {
    /// A card with a title, tagline, brand and color.
    pub fn new(title: impl Into<String>, tagline: impl Into<String>, brand: impl Into<String>, color: Color32) -> Self {
        Self { title: title.into(), tagline: tagline.into(), brand: brand.into(), color }
    }
}

#[derive(Clone, Debug, Default)]
struct BookmarkState {
    active: Vec<f32>,
    current: usize,
    auto_t: f32,
}

/// A row of cards cut like bookmarks (a raised tab on the top-left edge), each with a field of
/// silky wave ribbons in its own color. The active card fills with its color, lifts and glows,
/// its ribbons speed up and its title lights; the others wait in the dark. Hover or click to
/// switch; idle, it takes turns on its own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CardBookmark::new());
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CardBookmark {
    cards: Vec<BookmarkCard>,
    card: Vec2,
    waves: usize,
    height: f32,
    autoplay: bool,
}

impl Default for CardBookmark {
    fn default() -> Self {
        Self::new()
    }
}

impl CardBookmark {
    /// Three built-in cards.
    pub fn new() -> Self {
        Self {
            cards: vec![
                BookmarkCard::new("Start Here.", "Your brand deserves a second look.", "Nova Collective", hex("#3B4FE0")),
                BookmarkCard::new("+240%", "Growth that compounds every quarter.", "Bright Path", hex("#E8452C")),
                BookmarkCard::new("See It.", "The work speaks before we do.", "North Field", hex("#8B3FE0")),
            ],
            card: vec2(196.0, 260.0),
            waves: 34,
            height: 320.0,
            autoplay: true,
        }
    }
    /// Replace the cards.
    pub fn cards(mut self, cards: Vec<BookmarkCard>) -> Self {
        if !cards.is_empty() {
            self.cards = cards;
        }
        self
    }
    /// Card size (default 196 x 260).
    pub fn card_size(mut self, s: Vec2) -> Self {
        self.card = s.max(vec2(120.0, 160.0));
        self
    }
    /// Number of ribbon lines in the art (default 34).
    pub fn waves(mut self, n: usize) -> Self {
        self.waves = n.clamp(4, 120);
        self
    }
    /// Widget height (default 320).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(self.card.y + 20.0);
        self
    }
    /// Take turns highlighting the cards while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// The two convex pieces of a bookmark-cut card: the body and the raised tab.
fn card_shape(r: Rect, tab: f32, radius: f32) -> (Vec<Pos2>, Vec<Pos2>) {
    let body = Rect::from_min_max(pos2(r.min.x, r.min.y + tab), r.max);
    let body_pts = paint::rounded_rect_points(body, radius, 6);
    let tw = r.width() * 0.46;
    let tab_pts = vec![
        pos2(r.min.x, r.min.y + tab + radius),
        pos2(r.min.x, r.min.y + radius * 0.6),
        pos2(r.min.x + radius * 0.25, r.min.y + radius * 0.15),
        pos2(r.min.x + radius * 0.6, r.min.y),
        pos2(r.min.x + tw, r.min.y),
        pos2(r.min.x + tw + tab * 0.6, r.min.y + tab * 0.5),
        pos2(r.min.x + tw + tab * 1.4, r.min.y + tab + 0.5),
        pos2(r.min.x + radius, r.min.y + tab + 0.5),
    ];
    (body_pts, tab_pts)
}

fn ribbons(p: &egui::Painter, r: Rect, n: usize, color: Color32, time: f32, k: f32) {
    let hi = shade(color, 0.25);
    for i in 0..n {
        let f = i as f32 / n as f32;
        let pts: Vec<Pos2> = (0..=48)
            .map(|j| {
                let u = j as f32 / 48.0;
                let x = r.min.x + u * r.width();
                let twist = (u * 3.4 + time * 0.5 + f * 1.6).sin();
                let y = r.center().y + (u * 5.0 + time * (0.6 + 0.9 * k) + f * 0.9).sin() * r.height() * 0.24 * twist + (f - 0.5) * r.height() * 0.22 * (u * 2.6 + time * 0.3).cos();
                pos2(x, y)
            })
            .collect();
        let c = lerp_c(color, hi, (f * 2.0 - 1.0).abs());
        p.add(Shape::line(pts, Stroke::new(0.9, with_alpha(c, 0.35 + 0.55 * k))));
    }
}

impl Widget for CardBookmark {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let n = self.cards.len();
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, rect);
        let gap = 16.0;
        let total = n as f32 * self.card.x + (n as f32 - 1.0) * gap;
        let scale = (rect.width() / (total + 20.0)).min(1.0);
        let cs = self.card * scale;
        let x0 = rect.center().x - (n as f32 * cs.x + (n as f32 - 1.0) * gap) * 0.5;
        let card_rect = |i: usize| Rect::from_min_size(pos2(x0 + i as f32 * (cs.x + gap), rect.center().y - cs.y * 0.5), cs);
        let mut hovered = None;
        let mut clicked = None;
        for i in 0..n {
            let r = ui.interact(card_rect(i), id.with(("card", i)), Sense::click());
            if r.hovered() {
                hovered = Some(i);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                clicked = Some(i);
            }
        }
        let autoplay = self.autoplay && idle > 1.5;
        let st = glowkit_core::with_state(ui, id, BookmarkState::default, |s| {
            s.active.resize(n, 0.0);
            if let Some(i) = clicked.or(hovered) {
                s.current = i;
                s.auto_t = 2.4;
            } else if autoplay {
                s.auto_t -= wdt;
                if s.auto_t <= 0.0 {
                    s.current = (s.current + 1) % n;
                    s.auto_t = 2.2;
                }
            }
            for i in 0..n {
                let goal = if i == s.current { 1.0 } else { 0.0 };
                s.active[i] += (goal - s.active[i]) * (1.0 - (-7.0 * dt).exp());
            }
            s.clone()
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let idle_fill = if t.dark { hex("#141417") } else { Color32::WHITE };
        for i in 0..n {
            let c = &self.cards[i];
            let k = st.active[i];
            let r = card_rect(i).translate(vec2(0.0, -6.0 * k));
            let tab = 16.0 * scale;
            let radius = 14.0 * scale;
            // glow
            if k > 0.01 {
                paint::glow_ellipse(&painter, r.center() + vec2(0.0, 20.0), vec2(r.width() * 0.75, r.height() * 0.6), with_alpha(c.color, 0.28 * k));
            }
            paint::soft_shadow(&painter, r.shrink(4.0), radius, 18.0, vec2(0.0, 8.0), Color32::from_black_alpha(if t.dark { 120 } else { 26 }));
            let (body, tabp) = card_shape(r, tab, radius);
            let top = lerp_c(idle_fill, shade(c.color, 0.08), k);
            let bottom = lerp_c(idle_fill, shade(c.color, -0.2), k);
            let stops = [top, bottom];
            paint::fill_convex_fn(&painter, &body, 3, paint::linear_fn(r, 90.0, &stops));
            paint::fill_convex_fn(&painter, &tabp, 1, paint::linear_fn(r, 90.0, &stops));
            if !t.dark && k < 0.99 {
                let mut outline = tabp.clone();
                outline.truncate(7);
                painter.add(Shape::line(outline, Stroke::new(1.0, with_alpha(t.border, 1.0 - k))));
                painter.add(Shape::closed_line(body.clone(), Stroke::new(1.0, with_alpha(t.border, 1.0 - k))));
            }
            // brand on the tab line
            let on = lerp_c(if t.dark { hex("#E4E4E7") } else { hex("#18181B") }, Color32::WHITE, k);
            painter.text(pos2(r.max.x - 12.0 * scale, r.min.y + tab * 0.55), Align2::RIGHT_CENTER, format!("— {}®", c.brand), FontId::proportional(8.5 * scale.max(0.8)), on);
            // art panel
            let art = Rect::from_min_max(pos2(r.min.x + 10.0 * scale, r.min.y + tab + 10.0 * scale), pos2(r.max.x - 10.0 * scale, r.min.y + tab + r.height() * 0.5));
            let art_bg = lerp_c(if t.dark { hex("#0B0B0E") } else { hex("#121216") }, shade(c.color, -0.32), k);
            painter.rect_filled(art, 10.0 * scale, art_bg);
            let ap = painter.with_clip_rect(art.shrink(1.0).intersect(painter.clip_rect()));
            ribbons(&ap, art, self.waves, c.color, time + i as f32 * 1.7, k);
            let tg = painter.layout(c.tagline.clone(), FontId::proportional(10.5 * scale.max(0.8)), Color32::WHITE, art.width() * 0.62);
            painter.galley(pos2(art.min.x + 10.0 * scale, art.center().y + 4.0), tg, Color32::WHITE);
            // title
            let title_c = lerp_c(if t.dark { hex("#6B6B73") } else { hex("#A1A1AA") }, Color32::WHITE, k);
            painter.text(pos2(r.min.x + 14.0 * scale, art.max.y + 26.0 * scale), Align2::LEFT_CENTER, &c.title, FontId::proportional(25.0 * scale.max(0.7)), title_c);
            // dots
            for d in 0..3 {
                let on_dot = d == 0;
                let col = if on_dot { lerp_c(t.muted_foreground, c.color, 0.5 + 0.5 * k) } else { with_alpha(c.color, 0.4 + 0.3 * k) };
                painter.circle_filled(pos2(r.min.x + 16.0 * scale + d as f32 * 7.0, r.max.y - 14.0 * scale), 2.0, col);
            }
        }
        response
    }
}
