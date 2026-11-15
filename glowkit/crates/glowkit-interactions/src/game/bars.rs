//! Resource bars (health, mana, stamina, shield) with a delayed damage trail and heal glow, and
//! a wide boss bar with phase notches.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Id, Painter, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;

/// What a [`GameResourceBar`] measures; picks its colors and badge icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameResourceKind {
    /// Red health.
    #[default]
    Health,
    /// Blue mana.
    Mana,
    /// Yellow stamina.
    Stamina,
    /// Cyan shield / armor.
    Shield,
}

impl GameResourceKind {
    fn colors(self) -> [Color32; 2] {
        match self {
            GameResourceKind::Health => [hex("#E5233D"), hex("#FF7A6E")],
            GameResourceKind::Mana => [hex("#2F6BFF"), hex("#7FD0FF")],
            GameResourceKind::Stamina => [hex("#E6A817"), hex("#FFE27A")],
            GameResourceKind::Shield => [hex("#22B8D8"), hex("#C9FAFF")],
        }
    }
    fn icon(self) -> GameIcon {
        match self {
            GameResourceKind::Health => GameIcon::Heart,
            GameResourceKind::Mana => GameIcon::ManaPotion,
            GameResourceKind::Stamina => GameIcon::Bolt,
            GameResourceKind::Shield => GameIcon::Shield,
        }
    }
    fn tag(self) -> &'static str {
        match self {
            GameResourceKind::Health => "HP",
            GameResourceKind::Mana => "MP",
            GameResourceKind::Stamina => "SP",
            GameResourceKind::Shield => "AR",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BarAnim {
    last: f32,
    shown: f32,
    trail: f32,
    hold_until: f64,
    hit_at: f64,
    hit_amt: f32,
    heal_at: f64,
    heal_from: f32,
    trail_start: f32,
    frame: f64,
}

struct BarNow {
    shown: f32,
    trail: f32,
    hit_t: f32,
    hit_amt: f32,
    heal_t: f32,
    heal_from: f32,
}

/// Advance the trail / heal animation for fraction `v`.
fn bar_anim(ui: &Ui, id: Id, v: f32) -> BarNow {
    let now = ui.input(|i| i.time);
    let r = glowkit_core::with_state(
        ui,
        id.with("bar_anim"),
        || BarAnim {
            last: v,
            shown: v,
            trail: v,
            hold_until: 0.0,
            hit_at: -100.0,
            hit_amt: 0.0,
            heal_at: -100.0,
            heal_from: v,
            trail_start: v,
            frame: now,
        },
        |s| {
            // wall-clock step so the trail reads the same at any frame rate
            let dt = ((now - s.frame) as f32).clamp(0.0, 0.25);
            s.frame = now;
            if v < s.last - 1e-5 {
                s.trail = s.trail.max(s.last);
                s.trail_start = s.trail;
                s.hold_until = now + 0.55;
                s.hit_amt = if now - s.hit_at < 0.3 { s.hit_amt + (s.last - v) } else { s.last - v };
                s.hit_at = now;
            } else if v > s.last + 1e-5 {
                s.heal_at = now;
                s.heal_from = s.shown.min(s.last);
            }
            s.last = v;
            if v < s.shown {
                s.shown = v;
            } else {
                s.shown = glowkit_core::anim::approach(s.shown, v, 4.0, dt);
                if (v - s.shown).abs() < 0.002 {
                    s.shown = v;
                }
            }
            if now > s.hold_until {
                let k = (-((now - s.hold_until) as f32) * 6.0).exp();
                s.trail = s.shown + (s.trail_start - s.shown).max(0.0) * k;
                if (s.trail - s.shown).abs() < 0.002 {
                    s.trail = s.shown;
                }
            }
            s.trail = s.trail.max(s.shown);
            BarNow {
                shown: s.shown,
                trail: s.trail,
                hit_t: (now - s.hit_at) as f32,
                hit_amt: s.hit_amt,
                heal_t: (now - s.heal_at) as f32,
                heal_from: s.heal_from,
            }
        },
    );
    if (r.trail - r.shown).abs() > 1e-4 || (r.shown - v).abs() > 1e-4 || r.hit_t < 1.2 || r.heal_t < 1.2 {
        ui.ctx().request_repaint();
    }
    r
}

/// Paint fill, trail, heal ghost and flashes inside `track`.
#[allow(clippy::too_many_arguments)]
fn paint_bar(
    painter: &Painter,
    track: Rect,
    a: &BarNow,
    v: f32,
    colors: [Color32; 2],
    style: GameStyle,
    pal: &GamePalette,
    time: f32,
    seed: u64,
) {
    // damage trail: white flash, then a warm chunk that drains after a beat
    if a.trail > a.shown {
        let k = (a.hit_t / 0.12).clamp(0.0, 1.0);
        let chunk = mix(Color32::WHITE, hex("#FFB547"), k);
        bar_fill(painter, track, a.shown, a.trail, [chunk, mix(chunk, Color32::WHITE, 0.3)], style);
    }
    // heal ghost leading the fill
    if v > a.shown {
        let pulse = 0.6 + 0.4 * (time * 14.0).sin();
        bar_fill(
            painter,
            track,
            a.shown,
            v,
            [with_alpha(pal.good, 0.55 * pulse), with_alpha(mix(pal.good, Color32::WHITE, 0.6), 0.6 * pulse)],
            style,
        );
    }
    bar_fill(painter, track, 0.0, a.shown, colors, style);
    let edge_x = track.min.x + track.width() * a.shown;
    // heal glow sweep
    if a.heal_t < 1.0 {
        let k = a.heal_t;
        let g = with_alpha(pal.good, 0.55 * (1.0 - k));
        paint::soft_shadow(painter, track, 3.0, 14.0, vec2(0.0, 0.0), g);
        let sx = track.min.x + track.width() * (a.heal_from + (a.shown - a.heal_from) * (k * 1.6).min(1.0));
        paint::glow_ellipse(
            painter,
            pos2(sx, track.center().y),
            vec2(18.0, track.height()),
            with_alpha(Color32::WHITE, 0.6 * (1.0 - k)),
        );
        for i in 0..6u64 {
            let ph = (k * 1.3 + i as f32 * 0.17).fract();
            let x = track.min.x
                + track.width()
                    * (a.heal_from + (a.shown - a.heal_from).max(0.05) * glowkit_core::rng::hash01(seed + i * 3));
            let y = track.min.y - ph * 22.0;
            let s = 3.0 * (1.0 - ph);
            painter.text(
                pos2(x, y),
                Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(7.0 + s * 2.0),
                with_alpha(pal.good, (1.0 - ph) * (1.0 - k)),
            );
        }
    }
    // hit flash on the cut edge + chips
    if a.hit_t < 0.6 && a.shown < 1.0 {
        let k = a.hit_t / 0.6;
        paint::glow_ellipse(
            painter,
            pos2(edge_x, track.center().y),
            vec2(14.0, track.height() * 1.2),
            with_alpha(Color32::WHITE, 0.7 * (1.0 - k)),
        );
        burst(painter, pos2(edge_x + 4.0, track.center().y), a.hit_t, 8, seed, 90.0, 120.0, hex("#FF9F5A"), 2.4);
    }
    // stand-in for the edge highlight
    if a.shown > 0.01 && a.shown < 0.999 {
        painter.line_segment(
            [pos2(edge_x, track.min.y + 1.0), pos2(edge_x, track.max.y - 1.0)],
            Stroke::new(1.5, with_alpha(Color32::WHITE, 0.7)),
        );
    }
}

fn ticks(painter: &Painter, track: Rect, n: usize) {
    if !(2..=60).contains(&n) {
        return;
    }
    for i in 1..n {
        let x = track.min.x + track.width() * i as f32 / n as f32;
        let major = i % 5 == 0;
        let y0 = if major { track.min.y } else { track.max.y - track.height() * 0.45 };
        painter.line_segment(
            [pos2(x, y0), pos2(x, track.max.y)],
            Stroke::new(1.0, Color32::from_black_alpha(if major { 150 } else { 100 })),
        );
    }
}

/// A health / mana / stamina / shield bar that feels like a game. Taking damage cuts the fill
/// instantly, flashes the lost chunk white and leaves it hanging as a warm "trail" for half a
/// second before it drains (so players read how much they lost), with a jolt of screen shake and
/// sparks from the cut. Healing pours in with a green ghost, a bright sweep and rising pluses.
/// Low health pulses red.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameResourceBar, GameResourceKind, GameStyle};
/// ui.add(GameResourceBar::new(640.0, 1000.0).kind(GameResourceKind::Health).style(GameStyle::SciFi).segments(10));
/// ```
pub struct GameResourceBar {
    current: f32,
    max: f32,
    kind: GameResourceKind,
    style: GameStyle,
    palette: Option<GamePalette>,
    colors: Option<[Color32; 2]>,
    width: f32,
    height: f32,
    segments: usize,
    shake: f32,
    numbers: bool,
    badge: bool,
}

impl GameResourceBar {
    /// A bar at `current` out of `max`.
    pub fn new(current: f32, max: f32) -> Self {
        Self {
            current,
            max,
            kind: GameResourceKind::Health,
            style: GameStyle::SciFi,
            palette: None,
            colors: None,
            width: 300.0,
            height: 18.0,
            segments: 10,
            shake: 4.0,
            numbers: true,
            badge: true,
        }
    }
    /// What the bar measures (colors + icon).
    pub fn kind(mut self, k: GameResourceKind) -> Self {
        self.kind = k;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Override the fill gradient (dark, light).
    pub fn colors(mut self, c: [Color32; 2]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Bar width in points (excluding the badge).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(60.0);
        self
    }
    /// Bar height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(6.0, 48.0);
        self
    }
    /// Number of tick segments (0 or 1 for none).
    pub fn segments(mut self, n: usize) -> Self {
        self.segments = n;
        self
    }
    /// Screen-shake strength on damage, in points (0 = off).
    pub fn shake(mut self, s: f32) -> Self {
        self.shake = s.max(0.0);
        self
    }
    /// Show "640 / 1000" on the bar.
    pub fn numbers(mut self, on: bool) -> Self {
        self.numbers = on;
        self
    }
    /// Show the round icon badge on the left.
    pub fn badge(mut self, on: bool) -> Self {
        self.badge = on;
        self
    }
}

impl egui::Widget for GameResourceBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let h = self.height;
        let bw = if self.badge { h * 2.2 } else { 0.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width + bw + 8.0, (h * 2.2).max(h + 16.0)), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let v = if self.max > 0.0 { (self.current / self.max).clamp(0.0, 1.0) } else { 0.0 };
        let a = bar_anim(ui, resp.id, v);
        let time = glowkit_core::time(ui);
        let sh = shake(a.hit_t, self.shake * (a.hit_amt * 8.0).clamp(0.3, 1.5), resp.id.value() as f32 % 97.0);
        let painter = ui.painter();
        let track = Rect::from_min_size(pos2(rect.min.x + bw + 6.0, rect.center().y - h / 2.0), vec2(self.width, h))
            .translate(sh);
        let colors = self.colors.unwrap_or_else(|| self.kind.colors());
        let low = self.kind == GameResourceKind::Health && v < 0.25 && v > 0.0;
        if low {
            glowkit_core::animate(ui);
            let p = 0.5 + 0.5 * (time * 6.0).sin();
            paint::soft_shadow(painter, track.expand(2.0), 4.0, 18.0, vec2(0.0, 0.0), with_alpha(pal.bad, 0.5 * p));
        }
        bar_track(painter, track, self.style, &pal);
        paint_bar(painter, track, &a, v, colors, self.style, &pal, time, resp.id.value());
        ticks(painter, track, self.segments);
        if self.numbers && h >= 12.0 {
            let f = egui::FontId::monospace((h * 0.62).clamp(9.0, 16.0));
            let txt = format!("{} / {}", self.current.max(0.0).round() as i64, self.max.round() as i64);
            outlined(
                painter,
                track.center(),
                Align2::CENTER_CENTER,
                &txt,
                f,
                Color32::WHITE,
                Color32::from_black_alpha(200),
            );
        }
        if self.badge {
            let c = pos2(rect.min.x + bw / 2.0, rect.center().y) + sh * 0.6;
            let r = bw * 0.46;
            let hit_pop = if a.hit_t < 0.25 { 1.0 - a.hit_t / 0.25 } else { 0.0 };
            let heal_pop = if a.heal_t < 0.4 { (a.heal_t / 0.4 * std::f32::consts::PI).sin() } else { 0.0 };
            let s = 1.0 + 0.12 * heal_pop - 0.08 * hit_pop;
            match self.style {
                GameStyle::SciFi => {
                    let pts: Vec<_> = (0..6)
                        .map(|i| {
                            let ang = i as f32 / 6.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_6;
                            c + vec2(ang.cos(), ang.sin()) * r * s
                        })
                        .collect();
                    painter.add(egui::Shape::convex_polygon(
                        pts.clone(),
                        hex("#06101EE8"),
                        Stroke::new(1.5, with_alpha(pal.edge, 0.8)),
                    ));
                }
                GameStyle::Fantasy => {
                    painter.circle_filled(c, r * s, hex("#2B1D12"));
                    painter.circle_stroke(c, r * s, Stroke::new(2.5, pal.edge));
                }
                GameStyle::Minimal => {
                    painter.circle_filled(c, r * s, hex("#15171CEE"));
                    painter.circle_stroke(c, r * s, Stroke::new(1.0, hex("#FFFFFF22")));
                }
            }
            if heal_pop > 0.0 {
                paint::glow(painter, c, r * 2.0, with_alpha(pal.good, 0.4 * heal_pop));
            }
            let ic = mix(colors[1], Color32::WHITE, hit_pop * 0.7);
            self.kind.icon().paint(painter, c, r * 1.05 * s, ic);
            if !self.numbers || h < 12.0 {
                painter.text(
                    pos2(track.min.x, track.min.y - 3.0),
                    Align2::LEFT_BOTTOM,
                    self.kind.tag(),
                    egui::FontId::monospace(10.0),
                    pal.hud,
                );
            }
        }
        resp
    }
}

/// A wide boss health bar: the boss name in big glowing type above a long framed bar with phase
/// notches. Damage leaves the same delayed trail as [`GameResourceBar`]; crossing a phase notch
/// fires a flash, a shock ring and sparks, and below the last phase the frame pulses with rage.
/// An optional armor layer sits on top of the health.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GameBossBar::new("Vorrhak", 52_000.0, 80_000.0)
///     .title("The Ashen Tyrant")
///     .phases(&[0.66, 0.33]));
/// ```
pub struct GameBossBar<'a> {
    name: &'a str,
    title: &'a str,
    current: f32,
    max: f32,
    armor: f32,
    phases: &'a [f32],
    style: GameStyle,
    palette: Option<GamePalette>,
    width: Option<f32>,
}

impl<'a> GameBossBar<'a> {
    /// A boss named `name` at `current` / `max` health.
    pub fn new(name: &'a str, current: f32, max: f32) -> Self {
        Self {
            name,
            title: "",
            current,
            max,
            armor: 0.0,
            phases: &[],
            style: GameStyle::SciFi,
            palette: None,
            width: None,
        }
    }
    /// Subtitle under the name.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Armor layer as a fraction of the bar (0..1).
    pub fn armor(mut self, a: f32) -> Self {
        self.armor = a.clamp(0.0, 1.0);
        self
    }
    /// Phase thresholds as fractions of max health, e.g. `&[0.66, 0.33]`.
    pub fn phases(mut self, p: &'a [f32]) -> Self {
        self.phases = p;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Width (default: available width, up to 640).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w.max(160.0));
        self
    }
}

impl egui::Widget for GameBossBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let w = self.width.unwrap_or_else(|| ui.available_width().min(640.0));
        let name_f = display_font(ui, 22.0);
        let title_f = semi_font(ui, 12.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 82.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let v = if self.max > 0.0 { (self.current / self.max).clamp(0.0, 1.0) } else { 0.0 };
        let a = bar_anim(ui, resp.id, v);
        let time = glowkit_core::time(ui);
        // phase crossing
        let phase_idx = self.phases.iter().filter(|p| v <= **p).count();
        let (since_phase, _, _) = changed(ui, resp.id.with("phase"), phase_idx as f32);
        let enraged = !self.phases.is_empty() && phase_idx == self.phases.len() && v > 0.0;
        if enraged || since_phase < 1.2 {
            glowkit_core::animate(ui);
        }
        let sh = shake(a.hit_t, 3.0 * (a.hit_amt * 20.0).clamp(0.2, 1.5), 3.0) + shake(since_phase, 9.0, 5.0);
        let painter = ui.painter();
        let track = Rect::from_min_size(pos2(rect.min.x + 10.0, rect.min.y + 46.0), vec2(w - 20.0, 16.0)).translate(sh);
        let name_col = match self.style {
            GameStyle::Fantasy => hex("#FBEFD2"),
            _ => pal.hud,
        };
        let glow_c = if enraged { pal.bad } else { pal.accent };
        let up = self.name.to_uppercase();
        glow_text(
            painter,
            pos2(rect.center().x, rect.min.y + 18.0) + sh * 0.5,
            Align2::CENTER_CENTER,
            &up,
            name_f,
            name_col,
            with_alpha(glow_c, 0.6),
        );
        if !self.title.is_empty() {
            outlined(
                painter,
                pos2(rect.center().x, rect.min.y + 36.0),
                Align2::CENTER_CENTER,
                self.title,
                title_f,
                with_alpha(name_col, 0.8),
                Color32::from_black_alpha(120),
            );
        }
        if enraged {
            let p = 0.5 + 0.5 * (time * 7.0).sin();
            paint::soft_shadow(painter, track.expand(3.0), 4.0, 22.0, vec2(0.0, 0.0), with_alpha(pal.bad, 0.65 * p));
        }
        bar_track(painter, track, self.style, &pal);
        let colors = if enraged { [hex("#B3121F"), hex("#FF5A3D")] } else { [hex("#C21A33"), hex("#FF6A5E")] };
        paint_bar(painter, track, &a, v, colors, self.style, &pal, time, 77);
        if self.armor > 0.0 {
            let ar = Rect::from_min_size(track.min, vec2(track.width(), track.height() * 0.35));
            bar_fill(painter, ar, 0.0, self.armor, [hex("#9FB4C8"), hex("#E9F2FA")], self.style);
        }
        for (i, p) in self.phases.iter().enumerate() {
            let x = track.min.x + track.width() * p.clamp(0.0, 1.0);
            let passed = v <= *p;
            let c = if passed { with_alpha(pal.hud, 0.45) } else { pal.accent };
            painter.line_segment(
                [pos2(x, track.min.y - 2.0), pos2(x, track.max.y + 2.0)],
                Stroke::new(2.0, Color32::from_black_alpha(200)),
            );
            let d = 5.0;
            let tip = pos2(x, track.max.y + 5.0);
            painter.add(egui::Shape::convex_polygon(
                vec![tip + vec2(0.0, -d), tip + vec2(d, 0.0), tip + vec2(0.0, d), tip + vec2(-d, 0.0)],
                c,
                Stroke::new(1.0, Color32::from_black_alpha(180)),
            ));
            if passed && i + 1 == phase_idx && since_phase < 1.2 {
                ring(painter, pos2(x, track.center().y), since_phase, 0.8, 6.0, 70.0, with_alpha(pal.warn, 1.0), 4.0);
                burst(painter, pos2(x, track.center().y), since_phase, 22, 9 + i as u64, 240.0, 160.0, pal.warn, 3.2);
            }
        }
        if since_phase < 0.3 && phase_idx > 0 {
            paint::soft_shadow(
                painter,
                track,
                3.0,
                30.0,
                vec2(0.0, 0.0),
                with_alpha(Color32::WHITE, 0.5 * (1.0 - since_phase / 0.3)),
            );
        }
        let pct = format!("{:.0}%", v * 100.0);
        outlined(
            painter,
            pos2(track.max.x, track.max.y + 4.0),
            Align2::RIGHT_TOP,
            &pct,
            egui::FontId::monospace(11.0),
            pal.hud,
            Color32::from_black_alpha(160),
        );
        if !self.phases.is_empty() {
            let ph = format!("PHASE {}", phase_idx + 1);
            outlined(
                painter,
                pos2(track.min.x, track.max.y + 4.0),
                Align2::LEFT_TOP,
                &ph,
                egui::FontId::monospace(11.0),
                if enraged { pal.bad } else { pal.hud },
                Color32::from_black_alpha(160),
            );
        }
        resp
    }
}
