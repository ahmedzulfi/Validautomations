use super::common::{icon, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{spring, Clock, Trigger};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{paint, Rng, Theme};
use std::f32::consts::{PI, TAU};

/// A round knockout bracket. Thirty-two painted team badges ring the outside, two halves facing
/// each other; round by round the winners fly inward along curved connectors until the two
/// finalists meet and the champion lands, glowing, in the middle under a trophy. Hover any badge
/// to light up that team's road through the draw.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::KnockoutBracket::new().seed(7).pace(0.8));
/// ```
pub struct KnockoutBracket {
    seed: u64,
    pace: f32,
    size: Option<Vec2>,
    trigger: Trigger,
    gold: Option<Color32>,
}

impl Default for KnockoutBracket {
    fn default() -> Self {
        Self::new()
    }
}

impl KnockoutBracket {
    /// A bracket with seed 1, replaying in a loop.
    pub fn new() -> Self {
        Self { seed: 1, pace: 0.85, size: None, trigger: Trigger::Loop { pause_ms: 3500 }, gold: None }
    }
    /// Seed for the teams and the results.
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    /// Seconds per round.
    pub fn pace(mut self, s: f32) -> Self {
        self.pace = s.clamp(0.2, 3.0);
        self
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// When the draw plays (default: loop).
    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = t;
        self
    }
    /// Champion glow color (default warm gold).
    pub fn gold(mut self, c: Color32) -> Self {
        self.gold = Some(c);
        self
    }
}

#[derive(Clone, Copy)]
struct Flag {
    kind: u8,
    c: [Color32; 3],
}

const FLAG_COLORS: [&str; 10] =
    ["#D62828", "#F8F9FA", "#1D4ED8", "#FACC15", "#16A34A", "#0F172A", "#0EA5E9", "#EA580C", "#7C3AED", "#BE123C"];

fn make_flag(rng: &mut Rng) -> Flag {
    let mut pick = || hex(FLAG_COLORS[rng.below(FLAG_COLORS.len())]);
    let a = pick();
    let mut b = pick();
    while b == a {
        b = pick();
    }
    let mut c = pick();
    while c == a || c == b {
        c = pick();
    }
    Flag { kind: rng.below(6) as u8, c: [a, b, c] }
}

/// Clip a convex polygon by the half plane `dot(p - c, n) <= d`.
fn clip_half(pts: &[Pos2], c: Pos2, n: Vec2, d: f32) -> Vec<Pos2> {
    let mut out = Vec::with_capacity(pts.len() + 2);
    let inside = |p: Pos2| (p - c).dot(n) <= d;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let (ia, ib) = (inside(a), inside(b));
        if ia {
            out.push(a);
        }
        if ia != ib {
            let da = (a - c).dot(n) - d;
            let db = (b - c).dot(n) - d;
            let tt = da / (da - db);
            out.push(a + (b - a) * tt);
        }
    }
    out
}

/// A band of the disc between `lo` and `hi` (in radii) along direction `n`.
fn band(painter: &Painter, c: Pos2, r: f32, n: Vec2, lo: f32, hi: f32, col: Color32) {
    let circle = paint::circle_points(c, r, 36);
    let p = clip_half(&circle, c, n, hi * r);
    let p = clip_half(&p, c, -n, -lo * r);
    if p.len() >= 3 {
        let cen = p.iter().fold(Vec2::ZERO, |a, q| a + q.to_vec2()) / p.len() as f32;
        let mut mesh = egui::epaint::Mesh::default();
        mesh.colored_vertex(cen.to_pos2(), col);
        for q in &p {
            mesh.colored_vertex(*q, col);
        }
        let n = p.len() as u32;
        for i in 0..n {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
        }
        painter.add(Shape::mesh(mesh));
    }
}

fn paint_flag(painter: &Painter, c: Pos2, r: f32, f: Flag, ring: Color32, alpha: f32) {
    let k = |col: Color32| with_alpha(col, alpha);
    painter.circle_filled(c, r + 1.5, k(ring));
    let x = vec2(1.0, 0.0);
    let y = vec2(0.0, 1.0);
    painter.circle_filled(c, r, k(f.c[0]));
    match f.kind {
        0 => {
            band(painter, c, r, x, -1.0, -0.33, k(f.c[0]));
            band(painter, c, r, x, -0.33, 0.33, k(f.c[1]));
            band(painter, c, r, x, 0.33, 1.0, k(f.c[2]));
        }
        1 => {
            band(painter, c, r, y, -1.0, -0.33, k(f.c[0]));
            band(painter, c, r, y, -0.33, 0.33, k(f.c[1]));
            band(painter, c, r, y, 0.33, 1.0, k(f.c[2]));
        }
        2 => {
            band(painter, c, r, y, -1.0, 0.0, k(f.c[0]));
            band(painter, c, r, y, 0.0, 1.0, k(f.c[1]));
        }
        3 => {
            painter.circle_filled(c, r, k(f.c[0]));
            band(painter, c, r, x, -0.45, -0.1, k(f.c[1]));
            band(painter, c, r, y, -0.18, 0.18, k(f.c[1]));
        }
        4 => {
            painter.circle_filled(c, r, k(f.c[0]));
            painter.circle_filled(c, r * 0.42, k(f.c[1]));
        }
        _ => {
            let d = vec2(1.0, 1.0).normalized();
            band(painter, c, r, d, -1.0, 0.0, k(f.c[0]));
            band(painter, c, r, d, 0.0, 1.0, k(f.c[2]));
        }
    }
    // a little gloss
    paint::arc(painter, c, r - 1.2, -2.5, -1.3, 1.0, with_alpha(Color32::WHITE, 0.35 * alpha));
}

const ROUNDS: usize = 5;

#[allow(clippy::needless_range_loop)]
impl egui::Widget for KnockoutBracket {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let id = resp.id;
        let gold = self.gold.unwrap_or(hex("#F59E0B"));
        // teams + results
        let mut rng = Rng::new(self.seed.wrapping_mul(0x9E37_79B9) + 5);
        let flags: Vec<Flag> = (0..32).map(|_| make_flag(&mut rng)).collect();
        let strength: Vec<f32> = (0..32).map(|_| rng.f32()).collect();
        let mut bracket: Vec<Vec<usize>> = vec![(0..32).collect()];
        for r in 0..ROUNDS {
            let prev = &bracket[r];
            let next: Vec<usize> = (0..prev.len() / 2)
                .map(|i| {
                    let (a, b) = (prev[2 * i], prev[2 * i + 1]);
                    let pa = strength[a] + rng.range(-0.25, 0.25);
                    let pb = strength[b] + rng.range(-0.25, 0.25);
                    if pa >= pb {
                        a
                    } else {
                        b
                    }
                })
                .collect();
            bracket.push(next);
        }
        // geometry
        let c = rect.center();
        let big = (rect.height() / 2.0 - 16.0).min(rect.width() / 2.0 - 16.0).max(60.0);
        let radii = [1.0, 0.8, 0.61, 0.43, 0.25];
        let disc = (big * 0.068).clamp(7.0, 15.0);
        let mut angles: Vec<Vec<f32>> = Vec::new();
        let base: Vec<f32> = (0..32)
            .map(|i| {
                let left = i < 16;
                let j = (i % 16) as f32;
                let spread = 150f32.to_radians();
                let a = -spread / 2.0 + j / 15.0 * spread; // top to bottom
                if left {
                    PI - a
                } else {
                    a
                }
            })
            .collect();
        angles.push(base);
        for r in 1..ROUNDS {
            let prev = &angles[r - 1];
            let next = (0..prev.len() / 2)
                .map(|i| {
                    let (a, b) = (prev[2 * i], prev[2 * i + 1]);
                    // average on the circle
                    let v = Vec2::angled(a) + Vec2::angled(b);
                    v.y.atan2(v.x)
                })
                .collect();
            angles.push(next);
        }
        let pos = |r: usize, i: usize| -> Pos2 {
            if r >= ROUNDS {
                c
            } else if r == ROUNDS - 1 {
                c + vec2(if i == 0 { -1.0 } else { 1.0 } * big * radii[r], 0.0)
            } else {
                c + Vec2::angled(angles[r][i]) * big * radii[r]
            }
        };
        let hovered = resp.hover_pos();
        let hit = hovered.and_then(|p| (0..32).find(|&i| pos(0, i).distance(p) < disc + 3.0));
        // timeline
        let pace = self.pace;
        let dur = 0.8 + pace * ROUNDS as f32 + 0.6;
        let e = Clock::triggered(ui, id, self.trigger, dur, resp.hovered(), resp.clicked());
        let round_t = |r: usize| -> f32 { ((e - 0.6 - (r as f32 - 1.0) * pace) / (pace * 0.75)).clamp(0.0, 1.0) };
        let focus = spring(ui, id.with("focus"), if hit.is_some() { 1.0 } else { 0.0 }, 260.0, 26.0);
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter_at(rect);
        let line_col = if t.dark { with_alpha(Color32::WHITE, 0.16) } else { with_alpha(t.foreground, 0.16) };
        let on_path = |r: usize, i: usize| -> bool { hit.is_some_and(|h| bracket[r][i] == h) };
        // champion glow
        let champ_t = round_t(ROUNDS);
        if champ_t > 0.0 {
            paint::glow(&painter, c, big * 0.42 * champ_t, with_alpha(gold, 0.22 * champ_t));
        }
        // connectors (child -> parent), drawn as the winner travels
        for r in 1..=ROUNDS {
            let k = round_t(r);
            if k <= 0.0 {
                continue;
            }
            for i in 0..bracket[r].len() {
                let p = pos(r, i);
                for ch in [2 * i, 2 * i + 1] {
                    let q = pos(r - 1, ch);
                    // an elbow that follows the ring, then drops inward
                    let mid_r = (q - c).length() * 0.5 + (p - c).length() * 0.5;
                    let a_q = (q - c).y.atan2((q - c).x);
                    let elbow = c + Vec2::angled(a_q) * mid_r;
                    let mut pts = Vec::new();
                    pts.push(q);
                    pts.push(elbow);
                    let a_p = (p - c).y.atan2((p - c).x);
                    let mut da = a_p - a_q;
                    while da > PI {
                        da -= TAU;
                    }
                    while da < -PI {
                        da += TAU;
                    }
                    for s in 1..=10 {
                        pts.push(c + Vec2::angled(a_q + da * s as f32 / 10.0) * mid_r);
                    }
                    pts.push(p);
                    let lit = on_path(r - 1, ch) && on_path(r, i);
                    let col = if lit { t.accent } else { with_alpha(line_col, 1.0 - 0.6 * focus) };
                    let w = if lit { 1.8 } else { 1.0 };
                    // reveal along the path
                    let total = pts.len() as f32 - 1.0;
                    let upto = (k * total).max(0.0);
                    let mut shown = Vec::new();
                    for (j, pt) in pts.iter().enumerate() {
                        if (j as f32) <= upto {
                            shown.push(*pt);
                        } else {
                            let a = pts[j - 1];
                            shown.push(a + (*pt - a) * (upto - (j - 1) as f32));
                            break;
                        }
                    }
                    if shown.len() >= 2 {
                        painter.add(Shape::line(shown, Stroke::new(w, col)));
                    }
                }
            }
        }
        // discs
        let ring = if t.dark { hex("#1F1F23") } else { Color32::WHITE };
        let appear = (e / 0.6).clamp(0.0, 1.0);
        for i in 0..32 {
            let p = pos(0, i);
            let lost = bracket[1][i / 2] != i && round_t(1) >= 1.0;
            let dim = if hit.is_some() && !on_path(0, i) { 1.0 - 0.6 * focus } else { 1.0 };
            let a = stagger_in(appear, i) * dim * if lost { 0.55 } else { 1.0 };
            paint_flag(&painter, p, disc * (0.6 + 0.4 * stagger_in(appear, i)), flags[i], ring, a);
            if hit == Some(i) {
                painter.circle_stroke(p, disc + 3.0, Stroke::new(1.5, t.accent));
            }
        }
        for r in 1..=ROUNDS {
            let k = round_t(r);
            if k <= 0.0 {
                continue;
            }
            let eased = glowkit_core::ease(glowkit_core::Easing::OutBack, k);
            for i in 0..bracket[r].len() {
                let team = bracket[r][i];
                let from_i = if bracket[r - 1][2 * i] == team { 2 * i } else { 2 * i + 1 };
                let from = pos(r - 1, from_i);
                let to = pos(r, i);
                let p = from + (to - from) * eased;
                let rr = if r == ROUNDS { disc * (1.0 + 0.9 * eased) } else { disc };
                let lost = r < ROUNDS && round_t(r + 1) >= 1.0 && bracket[r + 1][i / 2] != team;
                let dim = if hit.is_some() && !on_path(r, i) { 1.0 - 0.6 * focus } else { 1.0 };
                paint_flag(
                    &painter,
                    p,
                    rr,
                    flags[team],
                    if r == ROUNDS { gold } else { ring },
                    dim * if lost { 0.55 } else { 1.0 },
                );
            }
        }
        if champ_t > 0.5 {
            let a = ((champ_t - 0.5) * 2.0).min(1.0);
            icon(&painter, BeIcon::Trophy, c - vec2(0.0, disc * 1.9 + 14.0), 14.0, with_alpha(gold, a), 1.6);
        }
        if let Some(h) = hit {
            let reached = (0..=ROUNDS).filter(|&r| bracket[r].contains(&h)).count() - 1;
            let label = ["Round of 32", "Round of 16", "Quarter-final", "Semi-final", "Final", "Champion"][reached];
            painter.text(
                pos2(c.x, rect.max.y - 10.0),
                Align2::CENTER_BOTTOM,
                format!("Reached: {label}"),
                FontId::proportional(12.0),
                mix(t.muted_foreground, t.foreground, focus),
            );
        }
        resp
    }
}

fn stagger_in(appear: f32, i: usize) -> f32 {
    (appear * 1.6 - (i % 16) as f32 / 16.0 * 0.6).clamp(0.0, 1.0)
}
