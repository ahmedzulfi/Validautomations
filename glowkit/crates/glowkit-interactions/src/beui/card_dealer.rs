use super::common::{cr, fill_fan, hairline, hover_fill, idle_for, shadow_color, text_width, well};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::Spring;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::text::paint_rotated;
use glowkit_core::{paint, Rng, Theme};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug)]
struct DealtCard {
    rank: u8, // 1..=13
    suit: u8, // 0 spade, 1 heart, 2 diamond, 3 club
    x: Spring,
    y: Spring,
    rot: Spring,
    flip: Spring,
    leaving: bool,
}

#[derive(Clone, Debug)]
struct DealerState {
    cards: Vec<DealtCard>,
    queue: u32,
    next_deal: f64,
    seed: u64,
    last_auto: f64,
    auto_step: u32,
    hop: f64,
}

/// What the buttons of a [`CardDealer`] do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DealAction {
    Deal(u32),
    Sort,
    Shuffle,
    Collect,
}

/// A card table: a deck in the corner deals cards that arc into a fanned hand, flipping face up
/// in flight. Sort and shuffle send every card gliding to its new place in the fan, and collect
/// sweeps them all back onto the deck. Every card rides its own position, rotation and flip
/// springs, so actions can overlap mid-flight.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CardDealer::new().max_hand(7).spread(9.0));
/// ```
pub struct CardDealer {
    size: Option<Vec2>,
    max_hand: usize,
    spread: f32,
    card_w: f32,
    autoplay: bool,
}

impl Default for CardDealer {
    fn default() -> Self {
        Self::new()
    }
}

impl CardDealer {
    /// A dealer with an empty hand.
    pub fn new() -> Self {
        Self { size: None, max_hand: 7, spread: 8.0, card_w: 74.0, autoplay: true }
    }
    /// Widget size (default: available width x 340).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Most cards the hand holds.
    pub fn max_hand(mut self, n: usize) -> Self {
        self.max_hand = n.clamp(1, 12);
        self
    }
    /// Angle between neighbouring cards in the fan, in degrees.
    pub fn spread(mut self, deg: f32) -> Self {
        self.spread = deg.clamp(0.0, 25.0);
        self
    }
    /// Card width in points (height follows a playing-card ratio).
    pub fn card_width(mut self, w: f32) -> Self {
        self.card_w = w.clamp(36.0, 140.0);
        self
    }
    /// Deal, sort, shuffle and collect on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn fresh(v: f32) -> Spring {
    Spring::new(v).with(170.0, 19.0)
}

impl egui::Widget for CardDealer {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 340.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        let id = resp.id;
        let cw = self.card_w;
        let ch = cw * 1.4;
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let hovering = ui.rect_contains_pointer(rect);
        let idle = idle_for(ui, id, hovering);
        let auto = self.autoplay && idle > 2.5;

        // buttons
        let labels: [(&str, DealAction, bool); 5] = [
            ("Deal", DealAction::Deal(1), true),
            ("Deal 3", DealAction::Deal(3), true),
            ("Sort", DealAction::Sort, false),
            ("Shuffle", DealAction::Shuffle, false),
            ("Collect", DealAction::Collect, false),
        ];
        let painter = ui.painter().clone();
        let bw: Vec<f32> = labels.iter().map(|l| text_width(&painter, l.0, 13.0) + 28.0).collect();
        let total = bw.iter().sum::<f32>() + 8.0 * (labels.len() - 1) as f32;
        let by = rect.max.y - 22.0;
        let mut bx = rect.center().x - total / 2.0;
        let mut btn_rects = Vec::new();
        let mut action = None;
        for (i, (_, act, _)) in labels.iter().enumerate() {
            let r = Rect::from_min_size(pos2(bx, by - 16.0), vec2(bw[i], 32.0));
            bx += bw[i] + 8.0;
            let br = ui.interact(r, id.with(("btn", i)), Sense::click());
            if br.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if br.clicked() {
                action = Some(*act);
            }
            btn_rects.push((r, br.hovered(), br.is_pointer_button_down_on()));
        }

        let deck = pos2(rect.min.x + cw * 0.5 + 24.0, rect.min.y + ch * 0.5 + 14.0);
        let hand_c = pos2(rect.center().x, rect.max.y - 56.0 - ch * 0.62);
        let pivot_r = ch * 3.2;
        let spread = self.spread.to_radians();
        let max_hand = self.max_hand;

        let st = glowkit_core::with_state(
            ui,
            id,
            || DealerState { cards: Vec::new(), queue: 0, next_deal: 0.0, seed: 11, last_auto: now, auto_step: 0, hop: -10.0 },
            |s| {
                if auto && now - s.last_auto > 1.25 {
                    s.last_auto = now;
                    let script = [
                        DealAction::Deal(3),
                        DealAction::Deal(1),
                        DealAction::Deal(1),
                        DealAction::Sort,
                        DealAction::Shuffle,
                        DealAction::Deal(1),
                        DealAction::Sort,
                        DealAction::Collect,
                    ];
                    action = Some(script[s.auto_step as usize % script.len()]);
                    s.auto_step += 1;
                }
                match action {
                    Some(DealAction::Deal(k)) => s.queue += k,
                    Some(DealAction::Sort) => {
                        s.cards.sort_by_key(|c| (c.leaving, c.suit, c.rank));
                        s.hop = now;
                    }
                    Some(DealAction::Shuffle) => {
                        s.seed += 1;
                        let mut rng = Rng::new(s.seed * 977);
                        let live: Vec<usize> = (0..s.cards.len()).filter(|&i| !s.cards[i].leaving).collect();
                        for k in (1..live.len()).rev() {
                            let j = rng.below(k + 1);
                            s.cards.swap(live[k], live[j]);
                        }
                        s.hop = now;
                    }
                    Some(DealAction::Collect) => {
                        s.queue = 0;
                        for c in &mut s.cards {
                            c.leaving = true;
                        }
                    }
                    None => {}
                }
                let live = s.cards.iter().filter(|c| !c.leaving).count();
                if s.queue > 0 && now >= s.next_deal {
                    if live < max_hand {
                        s.seed += 1;
                        let mut rng = Rng::new(s.seed * 7919 + 1);
                        let card = loop {
                            let rank = 1 + rng.below(13) as u8;
                            let suit = rng.below(4) as u8;
                            if !s.cards.iter().any(|c| c.rank == rank && c.suit == suit) {
                                break (rank, suit);
                            }
                        };
                        s.cards.push(DealtCard {
                            rank: card.0,
                            suit: card.1,
                            x: fresh(deck.x),
                            y: fresh(deck.y),
                            rot: fresh(0.0),
                            flip: Spring::new(0.0).with(120.0, 15.0),
                            leaving: false,
                        });
                    }
                    s.queue -= 1;
                    s.next_deal = now + 0.14;
                }
                // targets
                let live: Vec<usize> = (0..s.cards.len()).filter(|&i| !s.cards[i].leaving).collect();
                let m = live.len();
                let hop = ((now - s.hop) as f32 / 0.35).clamp(0.0, 1.0);
                for (k, &i) in live.iter().enumerate() {
                    let a = (k as f32 - (m as f32 - 1.0) / 2.0) * spread;
                    let c = &mut s.cards[i];
                    let lift = (hop * PI).sin() * 16.0 * if hop < 1.0 { 1.0 } else { 0.0 };
                    c.x.target = hand_c.x + a.sin() * pivot_r;
                    c.y.target = hand_c.y + pivot_r * (1.0 - a.cos()) - lift;
                    c.rot.target = a;
                    c.flip.target = 1.0;
                }
                for c in s.cards.iter_mut().filter(|c| c.leaving) {
                    c.x.target = deck.x;
                    c.y.target = deck.y;
                    c.rot.target = 0.0;
                    c.flip.target = 0.0;
                }
                let mut busy = s.queue > 0 || hop < 1.0;
                for c in &mut s.cards {
                    c.x.step(dt);
                    c.y.step(dt);
                    c.rot.step(dt);
                    c.flip.step(dt);
                    busy |= !(c.x.is_settled() && c.y.is_settled() && c.rot.is_settled());
                }
                s.cards.retain(|c| !(c.leaving && (c.x.value - deck.x).abs() < 2.0 && (c.y.value - deck.y).abs() < 2.0));
                (s.cards.clone(), busy)
            },
        );
        if st.1 || auto {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        // deck: a few stacked backs
        for k in (0..3).rev() {
            let o = vec2(k as f32 * 1.5, k as f32 * 1.5);
            card_back(&painter, &t, deck + o, cw, ch, 0.0, 1.0, if k == 0 { 0.7 } else { 0.3 });
        }
        for c in &st.0 {
            let sx = (c.flip.value * PI).cos(); // 1 = back, -1 = face
            let p = pos2(c.x.value, c.y.value);
            if sx > 0.0 {
                card_back(&painter, &t, p, cw, ch, c.rot.value, sx, 0.8);
            } else {
                card_face(&painter, &t, p, cw, ch, c.rot.value, -sx, c.rank, c.suit);
            }
        }
        // buttons
        for (i, (label, _, primary)) in labels.iter().enumerate() {
            let (r, hov, down) = btn_rects[i];
            let r = if down { r.shrink(1.0) } else { r };
            let (fill, fg) = if *primary {
                (if hov { mix(t.accent, Color32::WHITE, 0.1) } else { t.accent }, t.accent_foreground)
            } else {
                (if hov { hover_fill(&t) } else { well(&t) }, t.foreground)
            };
            painter.rect(r, cr(16.0), fill, if *primary { Stroke::NONE } else { hairline(&t) }, egui::StrokeKind::Inside);
            painter.text(r.center(), Align2::CENTER_CENTER, *label, FontId::proportional(13.0), fg);
        }
        resp
    }
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn card_outline(center: Pos2, w: f32, h: f32, angle: f32, sx: f32) -> Vec<Pos2> {
    let local = paint::rounded_rect_points(Rect::from_center_size(Pos2::ZERO, vec2(w, h)), w * 0.1, 5);
    let sx = sx.max(0.04);
    let mut out: Vec<Pos2> = Vec::with_capacity(local.len());
    for p in &local {
        let q = center + rot(vec2(p.x * sx, p.y), angle);
        if out.last().is_none_or(|l: &Pos2| l.distance(q) > 0.3) {
            out.push(q);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn card_back(painter: &Painter, t: &Theme, c: Pos2, w: f32, h: f32, angle: f32, sx: f32, shadow: f32) {
    let outline = card_outline(c + vec2(0.0, 3.0), w, h, angle, sx);
    painter.add(Shape::convex_polygon(outline, shadow_color(t, shadow * 0.8), Stroke::NONE));
    let base = if t.dark { hex("#2A2A30") } else { hex("#2B2B33") };
    let outline = card_outline(c, w, h, angle, sx);
    painter.add(Shape::convex_polygon(outline, base, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.12))));
    let inner = card_outline(c, w - 10.0, h - 10.0, angle, sx);
    painter.add(Shape::closed_line(inner, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.14))));
    // diagonal stripes inside the inner frame
    let hw = (w - 14.0) / 2.0;
    let hh = (h - 14.0) / 2.0;
    let mut k = -hh - hw;
    while k < hh + hw {
        // line y = x + k clipped to the box
        let x0 = (-hw).max(-hh - k);
        let x1 = hw.min(hh - k);
        if x1 > x0 {
            let a = c + rot(vec2(x0 * sx, x0 + k), angle);
            let b = c + rot(vec2(x1 * sx, x1 + k), angle);
            painter.line_segment([a, b], Stroke::new(2.4, with_alpha(Color32::WHITE, 0.07)));
        }
        k += 9.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn card_face(painter: &Painter, t: &Theme, c: Pos2, w: f32, h: f32, angle: f32, sx: f32, rank: u8, suit: u8) {
    let outline = card_outline(c + vec2(0.0, 4.0), w, h, angle, sx);
    painter.add(Shape::convex_polygon(outline, shadow_color(t, 0.9), Stroke::NONE));
    let outline = card_outline(c, w, h, angle, sx);
    painter.add(Shape::convex_polygon(outline, hex("#FCFCFD"), Stroke::new(1.0, hex("#D9D9DE"))));
    if sx < 0.55 {
        return;
    }
    let red = suit == 1 || suit == 2;
    let col = if red { hex("#F43F5E") } else { hex("#18181B") };
    let label = match rank {
        1 => "A".to_string(),
        11 => "J".to_string(),
        12 => "Q".to_string(),
        13 => "K".to_string(),
        n => n.to_string(),
    };
    let at = |x: f32, y: f32| c + rot(vec2(x * w * sx, y * h), angle);
    paint_rotated(painter, &label, at(-0.33, -0.39), FontId::proportional(w * 0.2), col, angle);
    suit_shape(painter, at(-0.33, -0.25), w * 0.09, angle, suit, col);
    suit_shape(painter, at(0.0, 0.02), w * 0.24, angle, suit, col);
    paint_rotated(painter, &label, at(0.33, 0.39), FontId::proportional(w * 0.2), col, angle + PI);
    suit_shape(painter, at(0.33, 0.25), w * 0.09, angle + PI, suit, col);
}

/// Paint a playing-card suit symbol (0 spade, 1 heart, 2 diamond, 3 club).
fn suit_shape(painter: &Painter, c: Pos2, s: f32, angle: f32, suit: u8, col: Color32) {
    let tf = |x: f32, y: f32| c + rot(vec2(x, y) * s, angle);
    let fan = |pts: Vec<Pos2>, center: Pos2| fill_fan(painter, center, &pts, col);
    let heart = |flip: f32| -> Vec<Pos2> {
        (0..40)
            .map(|i| {
                let a = i as f32 / 40.0 * TAU;
                let x = 16.0 * a.sin().powi(3);
                let y = -(13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos());
                tf(x / 16.0, flip * (y + 2.0) / 16.0)
            })
            .collect()
    };
    match suit {
        1 => fan(heart(1.0), tf(0.0, 0.1)),
        2 => {
            fill_fan(painter, c, &[tf(0.0, -1.0), tf(0.72, 0.0), tf(0.0, 1.0), tf(-0.72, 0.0)], col);
        }
        0 => {
            fan(heart(-1.0), tf(0.0, -0.1));
            fill_fan(painter, tf(0.0, 0.7), &[tf(0.0, 0.2), tf(0.3, 1.0), tf(-0.3, 1.0)], col);
        }
        _ => {
            for (x, y) in [(0.0, -0.45), (-0.45, 0.15), (0.45, 0.15)] {
                painter.circle_filled(tf(x, y), s * 0.42, col);
            }
            fill_fan(painter, tf(0.0, 0.6), &[tf(0.0, 0.0), tf(0.3, 1.0), tf(-0.3, 1.0)], col);
        }
    }
}
