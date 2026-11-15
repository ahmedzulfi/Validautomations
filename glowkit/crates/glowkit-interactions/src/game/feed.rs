//! Floating damage numbers, the kill feed and the scoreboard.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use glowkit_core::rng::hash01;

// ---------------------------------------------------------------------------------------------
// Damage numbers

/// The flavor of a floating number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameDamageKind {
    /// White, pops and arcs away.
    #[default]
    Normal,
    /// Big, golden, shakes in with a starburst.
    Crit,
    /// Green "+N" rising straight up.
    Heal,
    /// Grey "MISS" sliding sideways.
    Miss,
    /// Small poison-green damage over time.
    Poison,
    /// Cyan absorbed-by-shield number.
    Shield,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Hit {
    pos: Pos2,
    amount: i64,
    kind: GameDamageKind,
    born: f64,
    seed: u64,
}

/// The live list of floating numbers. Keep one in your game state, [`spawn`](Self::spawn) a
/// number whenever something is hit, and draw it every frame with [`GameDamageNumbers`].
/// Positions are in points relative to the top-left of the area the numbers are drawn in (in a
/// game: project the hit point to screen space).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameDamageFeed {
    hits: Vec<Hit>,
    counter: u64,
}

impl GameDamageFeed {
    /// An empty feed.
    pub fn new() -> Self {
        Self::default()
    }
    /// Spawn a number at `pos` (local points). Reads the current time from `ui`.
    pub fn spawn(&mut self, ui: &Ui, pos: Pos2, amount: i64, kind: GameDamageKind) {
        let now = ui.input(|i| i.time);
        self.counter = self.counter.wrapping_add(1);
        self.hits.push(Hit { pos, amount, kind, born: now, seed: self.counter.wrapping_mul(2654435761) });
        if self.hits.len() > 64 {
            self.hits.remove(0);
        }
    }
    /// Remove finished numbers (called automatically by [`GameDamageNumbers`] users; cheap).
    pub fn prune(&mut self, ui: &Ui) {
        let now = ui.input(|i| i.time);
        self.hits.retain(|h| now - h.born < 1.6);
    }
    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.hits.len()
    }
    /// True when nothing is floating.
    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }
}

/// Floating combat text. Normal hits pop out and arc away, crits slam in big and golden with a
/// starburst and a shake, heals rise in green, misses slide off, damage-over-time ticks drift up
/// small. All numbers carry a dark outline so they read on any scene.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameDamageFeed, GameDamageKind, GameDamageNumbers};
/// let mut feed = GameDamageFeed::new();
/// feed.spawn(ui, egui::pos2(200.0, 120.0), 1284, GameDamageKind::Crit);
/// ui.add(GameDamageNumbers::new(&feed).size(egui::vec2(400.0, 240.0)));
/// ```
pub struct GameDamageNumbers<'a> {
    feed: &'a GameDamageFeed,
    size: Option<Vec2>,
    scale: f32,
}

impl<'a> GameDamageNumbers<'a> {
    /// Draw the numbers in `feed`.
    pub fn new(feed: &'a GameDamageFeed) -> Self {
        Self { feed, size: None, scale: 1.0 }
    }
    /// Area size (default: available width x 240).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Text scale multiplier.
    pub fn scale(mut self, s: f32) -> Self {
        self.scale = s.clamp(0.3, 4.0);
        self
    }
    /// Paint into an existing `rect` (overlay use) instead of allocating space.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let display = display_font(ui, 22.0);
        if !self.feed.hits.is_empty() {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter();
        for h in &self.feed.hits {
            let t = (now - h.born) as f32;
            let life = match h.kind {
                GameDamageKind::Crit => 1.4,
                GameDamageKind::Poison => 0.9,
                _ => 1.1,
            };
            if !(0.0..life).contains(&t) {
                continue;
            }
            let k = t / life;
            let fade = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
            let side = hash01(h.seed) * 2.0 - 1.0;
            let base = rect.min + h.pos.to_vec2();
            let (text, col, size, pos, rot) = match h.kind {
                GameDamageKind::Normal => {
                    let e = 1.0 - (-t * 5.0).exp();
                    let p = base + vec2(side * 40.0 * e, -60.0 * e + 40.0 * t * t);
                    let s = 1.0 + 0.6 * (1.0 - (t / 0.12).min(1.0));
                    (thousands(h.amount), Color32::WHITE, 20.0 * s, p, 0.0)
                }
                GameDamageKind::Crit => {
                    let e = 1.0 - (-t * 4.0).exp();
                    let p = base + vec2(side * 20.0 * e, -46.0 * e) + shake(t, 6.0, h.seed as f32 % 50.0);
                    let s = if t < 0.1 { 2.6 - 1.0 * (t / 0.1) } else { 1.6 - 0.2 * ((t - 0.1) / 0.3).min(1.0) };
                    (format!("{}!", thousands(h.amount)), mix(hex("#FFE14D"), hex("#FF7A1F"), (t * 3.0).min(1.0)), 24.0 * s, p, side * 0.12)
                }
                GameDamageKind::Heal => {
                    let e = 1.0 - (-t * 3.0).exp();
                    (format!("+{}", thousands(h.amount)), hex("#5BF08F"), 20.0 * (1.0 + 0.4 * (1.0 - (t / 0.15).min(1.0))), base + vec2(0.0, -70.0 * e), 0.0)
                }
                GameDamageKind::Miss => {
                    let e = 1.0 - (-t * 4.0).exp();
                    ("MISS".to_string(), hex("#B8C0CC"), 16.0, base + vec2(side.signum() * 50.0 * e, -20.0 * e), 0.0)
                }
                GameDamageKind::Poison => (thousands(h.amount), hex("#A8F05A"), 14.0, base + vec2(side * 10.0, -40.0 * k), 0.0),
                GameDamageKind::Shield => {
                    let e = 1.0 - (-t * 5.0).exp();
                    (thousands(h.amount), hex("#7DF0FF"), 18.0, base + vec2(side * 30.0 * e, -50.0 * e), 0.0)
                }
            };
            if h.kind == GameDamageKind::Crit && t < 0.5 {
                let bk = t / 0.5;
                paint::star(painter, pos, 46.0 * (0.6 + bk), 18.0 * (0.6 + bk), 8, t * 2.0, with_alpha(hex("#FF9A1F"), 0.45 * (1.0 - bk)));
                paint::glow(painter, pos, 60.0, with_alpha(hex("#FFC94A"), 0.4 * (1.0 - bk)));
                burst(painter, pos, t, 12, h.seed, 260.0, 120.0, hex("#FFC94A"), 2.6);
            }
            let f = FontId::new(size * self.scale, display.family.clone());
            let outline = Color32::from_black_alpha((220.0 * fade) as u8);
            if rot.abs() > 0.001 {
                for d in [vec2(2.0, 0.0), vec2(-2.0, 0.0), vec2(0.0, 2.0), vec2(0.0, -2.0), vec2(2.0, 2.0)] {
                    glowkit_core::text::paint_rotated(painter, &text, pos + d, f.clone(), outline, rot);
                }
                glowkit_core::text::paint_rotated(painter, &text, pos, f, with_alpha(col, fade), rot);
            } else {
                outlined(painter, pos, Align2::CENTER_CENTER, &text, f, with_alpha(col, fade), outline);
            }
        }
    }
}

impl egui::Widget for GameDamageNumbers<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self.size.unwrap_or_else(|| vec2(ui.available_width(), 240.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Kill feed

/// One kill feed line.
#[derive(Clone, Debug, PartialEq)]
pub struct GameKillEntry {
    /// Killer name.
    pub killer: String,
    /// Victim name.
    pub victim: String,
    /// Weapon icon.
    pub weapon: GameIcon,
    /// Headshot marker.
    pub headshot: bool,
    /// Killer is on the player's team.
    pub killer_ally: bool,
    /// The local player was involved (highlighted).
    pub own: bool,
    /// Seconds since the kill.
    pub age: f32,
}

impl GameKillEntry {
    /// `killer` got `victim` with `weapon`, `age` seconds ago.
    pub fn new(killer: impl Into<String>, victim: impl Into<String>, weapon: GameIcon, age: f32) -> Self {
        Self { killer: killer.into(), victim: victim.into(), weapon, headshot: false, killer_ally: true, own: false, age }
    }
    /// Headshot.
    pub fn headshot(mut self, h: bool) -> Self {
        self.headshot = h;
        self
    }
    /// Whether the killer is an ally (blue) or enemy (red).
    pub fn ally(mut self, a: bool) -> Self {
        self.killer_ally = a;
        self
    }
    /// Highlight as the local player's kill / death.
    pub fn own(mut self, o: bool) -> Self {
        self.own = o;
        self
    }
}

/// A kill feed. New kills punch in from the right with a flash, older ones fade after a few
/// seconds and the rest glide up to close the gap. Your own kills get a bright frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameIcon, GameKillEntry, GameKillFeed};
/// let entries = vec![GameKillEntry::new("Vex", "Halcyon", GameIcon::Rifle, 0.4).headshot(true).own(true)];
/// ui.add(GameKillFeed::new(&entries));
/// ```
pub struct GameKillFeed<'a> {
    entries: &'a [GameKillEntry],
    width: f32,
    life: f32,
    max_rows: usize,
    ally: Color32,
    enemy: Color32,
}

impl<'a> GameKillFeed<'a> {
    /// A feed over `entries` (oldest first).
    pub fn new(entries: &'a [GameKillEntry]) -> Self {
        Self { entries, width: 360.0, life: 6.0, max_rows: 6, ally: hex("#5AB8FF"), enemy: hex("#FF5A5A") }
    }
    /// Width in points (rows are right-aligned).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(200.0);
        self
    }
    /// Seconds a row stays.
    pub fn life(mut self, s: f32) -> Self {
        self.life = s.max(1.0);
        self
    }
    /// Maximum rows shown.
    pub fn max_rows(mut self, n: usize) -> Self {
        self.max_rows = n.max(1);
        self
    }
    /// Team colors (ally, enemy).
    pub fn colors(mut self, ally: Color32, enemy: Color32) -> Self {
        self.ally = ally;
        self.enemy = enemy;
        self
    }
}

impl egui::Widget for GameKillFeed<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let name_f = semi_font(ui, 13.0);
        let row_h = 30.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, self.max_rows as f32 * row_h + 4.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let live: Vec<&GameKillEntry> = self.entries.iter().filter(|e| e.age >= 0.0 && e.age < self.life + 0.4).collect();
        let live = &live[live.len().saturating_sub(self.max_rows)..];
        if !live.is_empty() {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter();
        for (slot_i, e) in live.iter().enumerate() {
            let eid: Id = id.with((&e.killer, &e.victim, (e.weapon as u8)));
            let y = spring(ui, eid.with("y"), rect.min.y + slot_i as f32 * row_h, 300.0, 24.0);
            let enter = glowkit_core::ease(glowkit_core::Easing::OutBack, (e.age / 0.35).min(1.0));
            let fade = if e.age > self.life { 1.0 - (e.age - self.life) / 0.4 } else { 1.0 };
            let a = fade.clamp(0.0, 1.0) * (e.age / 0.1).min(1.0);
            let kc = if e.killer_ally { self.ally } else { self.enemy };
            let vc = if e.killer_ally { self.enemy } else { self.ally };
            let kw = text_w(painter, &e.killer, &name_f);
            let vw = text_w(painter, &e.victim, &name_f);
            let iw = 34.0 + if e.headshot { 20.0 } else { 0.0 };
            let total = kw + vw + iw + 36.0;
            let x1 = rect.max.x + (1.0 - enter) * 80.0;
            let r = Rect::from_min_max(pos2(x1 - total, y + 2.0), pos2(x1, y + row_h - 2.0));
            painter.rect_filled(r, 4.0, Color32::from_black_alpha((150.0 * a) as u8));
            if e.own {
                painter.rect_stroke(r, 4.0, Stroke::new(1.5, with_alpha(Color32::WHITE, 0.85 * a)), egui::StrokeKind::Inside);
                paint::soft_shadow(painter, r, 4.0, 10.0, Vec2::ZERO, with_alpha(hex("#FF3B4A"), 0.35 * a));
            }
            if e.age < 0.3 {
                painter.rect_filled(r, 4.0, with_alpha(Color32::WHITE, 0.35 * (1.0 - e.age / 0.3)));
            }
            let cy = r.center().y;
            let mut x = r.min.x + 12.0;
            painter.text(pos2(x, cy), Align2::LEFT_CENTER, &e.killer, name_f.clone(), with_alpha(kc, a));
            x += kw + 8.0;
            e.weapon.paint(painter, pos2(x + 14.0, cy), 26.0, with_alpha(hex("#E6EAF0"), a));
            x += 34.0;
            if e.headshot {
                GameIcon::Skull.paint(painter, pos2(x + 6.0, cy), 14.0, with_alpha(hex("#FFD27A"), a));
                x += 20.0;
            }
            painter.text(pos2(x + 4.0, cy), Align2::LEFT_CENTER, &e.victim, name_f.clone(), with_alpha(vc, a));
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Scoreboard

/// One player row on a [`GameScoreboard`].
#[derive(Clone, Debug, PartialEq)]
pub struct GameScoreRow {
    /// Player name.
    pub name: String,
    /// Team index (0 or 1).
    pub team: usize,
    /// Kills.
    pub kills: u32,
    /// Deaths.
    pub deaths: u32,
    /// Assists.
    pub assists: u32,
    /// Score (rows sort by it).
    pub score: u32,
    /// Ping in ms.
    pub ping: u32,
    /// The local player.
    pub local: bool,
    /// Currently dead (dimmed with a skull).
    pub dead: bool,
}

impl GameScoreRow {
    /// A row for `name` on `team`.
    pub fn new(name: impl Into<String>, team: usize) -> Self {
        Self { name: name.into(), team, kills: 0, deaths: 0, assists: 0, score: 0, ping: 40, local: false, dead: false }
    }
    /// Kills / deaths / assists.
    pub fn kda(mut self, k: u32, d: u32, a: u32) -> Self {
        self.kills = k;
        self.deaths = d;
        self.assists = a;
        self
    }
    /// Score.
    pub fn score(mut self, s: u32) -> Self {
        self.score = s;
        self
    }
    /// Ping in ms.
    pub fn ping(mut self, p: u32) -> Self {
        self.ping = p;
        self
    }
    /// Mark as the local player.
    pub fn local(mut self, l: bool) -> Self {
        self.local = l;
        self
    }
    /// Mark as dead.
    pub fn dead(mut self, d: bool) -> Self {
        self.dead = d;
        self
    }
}

/// A two-team scoreboard. Each team gets a header with its name, color stripe and total; rows
/// stagger in when it opens, re-sort by score with springy row moves when scores change (the
/// changed score pops), the local player glows, dead players dim with a skull, and ping shows
/// as colored signal bars.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameScoreRow, GameScoreboard};
/// let rows = vec![GameScoreRow::new("Vex", 0).kda(12, 3, 4).score(2450).local(true)];
/// ui.add(GameScoreboard::new(&rows).teams(["Aurora", "Obsidian"]));
/// ```
pub struct GameScoreboard<'a> {
    rows: &'a [GameScoreRow],
    teams: [&'a str; 2],
    team_scores: Option<[u32; 2]>,
    colors: [Color32; 2],
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameScoreboard<'a> {
    /// A scoreboard over `rows`.
    pub fn new(rows: &'a [GameScoreRow]) -> Self {
        Self { rows, teams: ["Alpha", "Bravo"], team_scores: None, colors: [hex("#4DA6FF"), hex("#FF5A5A")], style: GameStyle::SciFi, palette: None }
    }
    /// Team names.
    pub fn teams(mut self, t: [&'a str; 2]) -> Self {
        self.teams = t;
        self
    }
    /// Team round scores (default: the sum of player scores).
    pub fn team_scores(mut self, s: [u32; 2]) -> Self {
        self.team_scores = Some(s);
        self
    }
    /// Team colors.
    pub fn colors(mut self, c: [Color32; 2]) -> Self {
        self.colors = c;
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
}

impl egui::Widget for GameScoreboard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let head_f = display_font(ui, 18.0);
        let name_f = semi_font(ui, 13.0);
        let w = ui.available_width().min(900.0);
        let wide = w > 640.0;
        let row_h = 28.0;
        let per_team = (0..2).map(|t| self.rows.iter().filter(|r| r.team == t).count()).max().unwrap_or(0) as f32;
        let team_h = 64.0 + per_team * row_h;
        let h = if wide { team_h + 20.0 } else { team_h * 2.0 + 30.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let appear = glowkit_core::anim::Clock::elapsed(ui, id.with("appear"));
        if appear < 2.0 {
            ui.ctx().request_repaint();
        }
        let time = glowkit_core::time(ui);
        let painter = ui.painter();
        for team in 0..2 {
            let tr = if wide {
                let cw = (w - 16.0) / 2.0;
                Rect::from_min_size(rect.min + vec2(team as f32 * (cw + 16.0), 10.0), vec2(cw, team_h))
            } else {
                Rect::from_min_size(rect.min + vec2(0.0, 10.0 + team as f32 * (team_h + 10.0)), vec2(w, team_h))
            };
            let tc = self.colors[team];
            frame(painter, tr, self.style, &pal, 0.0);
            painter.rect_filled(Rect::from_min_size(tr.min + vec2(2.0, 2.0), vec2(tr.width() - 4.0, 3.0)), 1.0, tc);
            let mut rows: Vec<&GameScoreRow> = self.rows.iter().filter(|r| r.team == team).collect();
            rows.sort_by(|a, b| b.score.cmp(&a.score));
            let total = self.team_scores.map(|s| s[team]).unwrap_or_else(|| rows.iter().map(|r| r.score).sum());
            painter.text(pos2(tr.min.x + 16.0, tr.min.y + 24.0), Align2::LEFT_CENTER, self.teams[team].to_uppercase(), head_f.clone(), tc);
            glow_text(painter, pos2(tr.max.x - 16.0, tr.min.y + 24.0), Align2::RIGHT_CENTER, &thousands(total as i64), head_f.clone(), pal.text, with_alpha(tc, 0.5));
            // columns
            let cols = [("K", 0.52), ("D", 0.6), ("A", 0.68), ("SCORE", 0.82), ("PING", 0.95)];
            let hy = tr.min.y + 48.0;
            painter.text(pos2(tr.min.x + 40.0, hy), Align2::LEFT_CENTER, "PLAYER", FontId::monospace(10.0), pal.dim);
            for (c, fx) in cols {
                painter.text(pos2(tr.min.x + tr.width() * fx, hy), Align2::CENTER_CENTER, c, FontId::monospace(10.0), pal.dim);
            }
            for (rank, r) in rows.iter().enumerate() {
                let rid = id.with(("row", &r.name));
                let target_y = tr.min.y + 58.0 + rank as f32 * row_h;
                let y = spring(ui, rid.with("y"), target_y, 260.0, 20.0);
                let k = glowkit_core::anim::stagger(appear, rank + team * 2, 0.06, 0.4);
                let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, k);
                let rr = Rect::from_min_size(pos2(tr.min.x + 8.0 + (1.0 - e) * 30.0, y), vec2(tr.width() - 16.0, row_h - 3.0));
                let a = e * if r.dead { 0.5 } else { 1.0 };
                if r.local {
                    let p = 0.6 + 0.4 * (time * 3.0).sin();
                    paint::gradient_rect(painter, rr, 3.0, 0.0, &[with_alpha(pal.accent, 0.35 * e), with_alpha(pal.accent, 0.08 * e)]);
                    painter.rect_stroke(rr, 3.0, Stroke::new(1.0, with_alpha(pal.accent, 0.7 * p * e)), egui::StrokeKind::Inside);
                    ui.ctx().request_repaint();
                } else if rank % 2 == 0 {
                    painter.rect_filled(rr, 3.0, with_alpha(if self.style == GameStyle::Fantasy { pal.dim } else { Color32::WHITE }, 0.05 * e));
                }
                let cy = rr.center().y;
                painter.text(pos2(rr.min.x + 14.0, cy), Align2::CENTER_CENTER, (rank + 1).to_string(), FontId::monospace(11.0), with_alpha(pal.dim, e));
                if r.dead {
                    GameIcon::Skull.paint(painter, pos2(rr.min.x + 36.0, cy), 13.0, with_alpha(pal.dim, e));
                }
                let nx = rr.min.x + if r.dead { 48.0 } else { 32.0 };
                painter.text(pos2(nx, cy), Align2::LEFT_CENTER, &r.name, name_f.clone(), with_alpha(if r.local { pal.text } else { mix(pal.text, pal.dim, 0.2) }, a));
                let vals = [r.kills.to_string(), r.deaths.to_string(), r.assists.to_string()];
                for (i, v) in vals.iter().enumerate() {
                    painter.text(pos2(tr.min.x + tr.width() * cols[i].1, cy), Align2::CENTER_CENTER, v, FontId::monospace(12.0), with_alpha(pal.text, a));
                }
                let (since, prev, _) = changed(ui, rid.with("score"), r.score as f32);
                let popk = if since < 0.5 && (r.score as f32) > prev { 1.0 - since / 0.5 } else { 0.0 };
                if popk > 0.0 {
                    ui.ctx().request_repaint();
                }
                painter.text(pos2(tr.min.x + tr.width() * cols[3].1, cy), Align2::CENTER_CENTER, thousands(r.score as i64), FontId::monospace(12.5 * (1.0 + 0.35 * popk)), with_alpha(mix(pal.text, pal.warn, popk), a));
                // ping bars
                let bars = if r.ping < 50 { 4 } else if r.ping < 90 { 3 } else if r.ping < 150 { 2 } else { 1 };
                let pc = match bars {
                    4 | 3 => pal.good,
                    2 => pal.warn,
                    _ => pal.bad,
                };
                let px = tr.min.x + tr.width() * cols[4].1 - 9.0;
                for b in 0..4 {
                    let bh = 4.0 + b as f32 * 3.0;
                    let br = Rect::from_min_max(pos2(px + b as f32 * 5.0, cy + 7.0 - bh), pos2(px + b as f32 * 5.0 + 3.0, cy + 7.0));
                    painter.rect_filled(br, 0.5, with_alpha(if b < bars { pc } else { pal.dim }, if b < bars { a } else { 0.25 * a }));
                }
            }
        }
        resp
    }
}
