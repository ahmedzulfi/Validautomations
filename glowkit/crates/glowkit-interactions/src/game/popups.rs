//! Loot reveal popup and achievement toast.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use std::f32::consts::TAU;

/// A loot reveal. A charged orb in the item's rarity color gathers sparks and trembles (harder
/// for rarer loot), then bursts: white flash, shock ring, a fountain of particles and rotating
/// god rays, and the item card springs out with its name, a wide-tracked rarity label and its
/// stats fading in one by one. Changing the item (or `replay` key) plays it again.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameIcon, GameItem, GameLootPopup, GameRarity};
/// let item = GameItem::new("Dawnbreaker", GameIcon::Sword, GameRarity::Legendary).stat("Damage", 148.0);
/// ui.add(GameLootPopup::new(&item));
/// ```
pub struct GameLootPopup<'a> {
    item: &'a GameItem,
    replay: u64,
    height: f32,
    shake: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameLootPopup<'a> {
    /// A reveal for `item`.
    pub fn new(item: &'a GameItem) -> Self {
        Self { item, replay: 0, height: 320.0, shake: 1.0, style: GameStyle::SciFi, palette: None, cues: None }
    }
    /// Change this number to replay the reveal for the same item.
    pub fn replay(mut self, key: u64) -> Self {
        self.replay = key;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(220.0);
        self
    }
    /// Screen-shake multiplier (0 = off).
    pub fn shake(mut self, s: f32) -> Self {
        self.shake = s.max(0.0);
        self
    }
    /// Visual style (card frame).
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Collect feedback cues ([`GameCue::Loot`] at the burst).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameLootPopup<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let name_f = display_font(ui, 22.0);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let mut key = self.replay.wrapping_mul(31);
        for b in self.item.name.bytes() {
            key = key.wrapping_mul(131).wrapping_add(b as u64);
        }
        let now = ui.input(|i| i.time);
        let t = glowkit_core::with_state(ui, id.with("key"), || (key, now), |s| {
            if s.0 != key {
                *s = (key, now);
            }
            (now - s.1) as f32
        });
        let charge = 0.75;
        let fired: bool = glowkit_core::state(ui, id.with(("fired", key)), || false);
        if t >= charge && !fired {
            glowkit_core::set_state(ui, id.with(("fired", key)), true);
            push_cue(&mut self.cues, GameCue::Loot(self.item.rarity));
        }
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let rc = self.item.rarity.color();
        let tier = self.item.rarity.tier() as f32;
        let painter = ui.painter();
        let tb = t - charge;
        let amp = (2.0 + tier * 2.5) * self.shake;
        let mut sh = Vec2::ZERO;
        if t < charge {
            let k = t / charge;
            sh = vec2((time * 70.0).sin(), (time * 53.0).cos()) * amp * 0.35 * k * k;
        }
        sh += shake(tb, amp * 2.0, 2.0);
        let c = rect.center() + sh - vec2(0.0, 10.0);
        if t < charge {
            // charging orb + converging sparks
            let k = t / charge;
            paint::glow(painter, c, 60.0 + 40.0 * k, with_alpha(rc, 0.35 + 0.3 * k));
            for i in 0..18u64 {
                let a = glowkit_core::rng::hash01(i * 7 + key) * TAU;
                let d = 140.0 * (1.0 - ((k + glowkit_core::rng::hash01(i + key)) % 1.0));
                let p = c + vec2(a.cos(), a.sin()) * d;
                painter.circle_filled(p, 2.0, with_alpha(mix(rc, Color32::WHITE, 0.5), 0.9));
            }
            let r = 18.0 + 10.0 * k + (time * 30.0).sin() * 2.0 * k;
            painter.circle_filled(c, r, mix(rc, Color32::WHITE, 0.3 + 0.5 * k));
            painter.circle_stroke(c, r + 6.0 + (1.0 - k) * 30.0, Stroke::new(2.0, with_alpha(rc, k)));
            return resp;
        }
        // burst + rays
        let ray_a = (tb / 0.5).min(1.0);
        rays(painter, c, (rect.height() * 0.9) * glowkit_core::ease(glowkit_core::Easing::OutCubic, ray_a), 8 + self.item.rarity.tier() * 2, time * 0.4, with_alpha(rc, 0.22 + 0.06 * tier));
        rays(painter, c, rect.height() * 0.6 * ray_a, 6, -time * 0.25, with_alpha(Color32::WHITE, 0.08));
        paint::glow(painter, c, 150.0, with_alpha(rc, 0.35));
        ring(painter, c, tb, 0.7, 20.0, rect.width().min(560.0) * 0.5, mix(rc, Color32::WHITE, 0.4), 6.0);
        ring(painter, c, tb - 0.12, 0.7, 10.0, rect.width().min(560.0) * 0.35, rc, 3.0);
        burst(painter, c, tb, 24 + self.item.rarity.tier() * 10, key, 520.0, 220.0, rc, 3.4);
        // floating motes
        for i in 0..14u64 {
            let ph = (time * 0.25 + glowkit_core::rng::hash01(i * 13)).fract();
            let x = c.x + (glowkit_core::rng::hash01(i * 3 + 1) - 0.5) * 300.0;
            let y = c.y + 110.0 - ph * 220.0;
            paint::sparkle(painter, pos2(x, y), 3.0 + 3.0 * glowkit_core::rng::hash01(i), time + i as f32, with_alpha(mix(rc, Color32::WHITE, 0.5), (1.0 - ph) * ray_a * 0.9));
        }
        if tb < 0.2 {
            painter.rect_filled(rect, 18.0, with_alpha(Color32::WHITE, 0.6 * (1.0 - tb / 0.2)));
        }
        // card
        let s = pop((tb / 0.45).min(1.0));
        let bob = (time * 1.8).sin() * 3.0;
        let card_w = 230.0;
        let n_stats = self.item.stats.len().min(3);
        let card_h = 150.0 + n_stats as f32 * 18.0;
        let card = Rect::from_center_size(c + vec2(0.0, bob), vec2(card_w, card_h) * s.max(0.01));
        if s > 0.02 {
            paint::soft_shadow(painter, card, 10.0, 30.0, Vec2::ZERO, with_alpha(rc, 0.6));
            frame(painter, card, self.style, &pal, 1.0);
            let ic = Rect::from_center_size(pos2(card.center().x, card.min.y + 46.0 * s), Vec2::splat(64.0 * s));
            slot(painter, ic, self.style, &pal, Some(self.item.rarity), 1.0);
            self.item.icon.paint(painter, ic.center(), 46.0 * s, self.item.tint());
            if self.item.rarity == GameRarity::Legendary {
                paint::gradient_border(painter, ic.shrink(1.0), 4.0, 2.0, paint::comet_fn((time * 0.5).fract(), 0.4, hex("#FFE7A8")));
            }
            let a = ((tb - 0.15) / 0.3).clamp(0.0, 1.0);
            let nc = if self.style == GameStyle::Fantasy { glowkit_core::color::shade(rc, -0.4) } else { rc };
            let nf = FontId::new(name_f.size * s, name_f.family.clone());
            painter.text(pos2(card.center().x, ic.max.y + 20.0 * s), Align2::CENTER_CENTER, &self.item.name, nf, with_alpha(nc, a));
            let label: String = self.item.rarity.label().to_uppercase().chars().flat_map(|ch| [ch, ' ']).collect();
            let lab = format!("{} {}", label.trim_end(), if self.item.kind.is_empty() { String::new() } else { format!("· {}", self.item.kind.to_uppercase()) });
            painter.text(pos2(card.center().x, ic.max.y + 42.0 * s), Align2::CENTER_CENTER, lab.trim(), FontId::monospace(10.0 * s.max(0.1)), with_alpha(pal.dim, a));
            for (i, (name, v)) in self.item.stats.iter().take(3).enumerate() {
                let k = glowkit_core::anim::stagger(tb - 0.35, i, 0.1, 0.3);
                let y = ic.max.y + 64.0 * s + i as f32 * 18.0 * s;
                let dx = (1.0 - k) * 16.0;
                let vs = if v.fract().abs() < 1e-3 { format!("+{}", *v as i64) } else { format!("+{v:.1}") };
                painter.text(pos2(card.min.x + 22.0 + dx, y), Align2::LEFT_CENTER, name, FontId::proportional(12.0 * s.max(0.1)), with_alpha(pal.dim, k));
                painter.text(pos2(card.max.x - 22.0 + dx, y), Align2::RIGHT_CENTER, vs, FontId::monospace(12.0 * s.max(0.1)), with_alpha(if self.style == GameStyle::Fantasy { glowkit_core::color::shade(pal.good, -0.35) } else { pal.good }, k));
            }
        }
        resp
    }
}

/// An achievement toast. It drops in from the top with a spring, the badge pops with a ring and
/// a light sweep across the card, then a thin timer line runs down before it slides away. Feed
/// it the seconds since the achievement unlocked; outside its lifetime it paints nothing (but
/// keeps its space so layouts don't jump).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameAchievementToast, GameIcon};
/// ui.add(GameAchievementToast::new("Dragonslayer", "Defeat Vorrhak on Hard", GameIcon::Trophy).points(50).age(1.2));
/// ```
pub struct GameAchievementToast<'a> {
    title: &'a str,
    desc: &'a str,
    icon: GameIcon,
    points: u32,
    rarity: GameRarity,
    percent: Option<f32>,
    age: f32,
    duration: f32,
    width: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameAchievementToast<'a> {
    /// A toast for achievement `title`.
    pub fn new(title: &'a str, desc: &'a str, icon: GameIcon) -> Self {
        Self { title, desc, icon, points: 0, rarity: GameRarity::Epic, percent: None, age: 0.0, duration: 5.0, width: 380.0, style: GameStyle::Minimal, palette: None }
    }
    /// Score / points badge.
    pub fn points(mut self, p: u32) -> Self {
        self.points = p;
        self
    }
    /// Tier color.
    pub fn rarity(mut self, r: GameRarity) -> Self {
        self.rarity = r;
        self
    }
    /// "Unlocked by N% of players" line.
    pub fn percent(mut self, p: f32) -> Self {
        self.percent = Some(p);
        self
    }
    /// Seconds since the achievement unlocked.
    pub fn age(mut self, a: f32) -> Self {
        self.age = a;
        self
    }
    /// How long it stays (seconds, excluding the exit).
    pub fn duration(mut self, d: f32) -> Self {
        self.duration = d.max(1.0);
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(240.0);
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

impl egui::Widget for GameAchievementToast<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title_f = display_font(ui, 15.0);
        let h = 84.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, h + 20.0), Sense::hover());
        let t = self.age;
        let exit = 0.45;
        if !ui.is_rect_visible(rect) || t < 0.0 || t > self.duration + exit {
            return resp;
        }
        glowkit_core::animate(ui);
        let painter = ui.painter();
        let rc = self.rarity.color();
        let enter = glowkit_core::ease(glowkit_core::Easing::OutBack, (t / 0.5).min(1.0));
        let leave = if t > self.duration { glowkit_core::ease(glowkit_core::Easing::InCubic, ((t - self.duration) / exit).min(1.0)) } else { 0.0 };
        let y = -(1.0 - enter) * 60.0 - leave * 70.0;
        let mut p2 = painter.clone();
        p2.set_opacity((t / 0.2).min(1.0) * (1.0 - leave));
        let card = Rect::from_min_size(rect.min + vec2(0.0, 10.0 + y), vec2(self.width, h));
        frame(&p2, card, self.style, &pal, 0.6);
        // light sweep
        let sw = ((t - 0.45) / 0.7).clamp(0.0, 1.0);
        if sw > 0.0 && sw < 1.0 {
            let x = card.min.x + card.width() * sw;
            let band = Rect::from_min_max(pos2((x - 40.0).max(card.min.x), card.min.y + 1.0), pos2((x + 40.0).min(card.max.x), card.max.y - 1.0));
            paint::gradient_rect(&p2, band, 0.0, 0.0, &[with_alpha(Color32::WHITE, 0.0), with_alpha(Color32::WHITE, 0.16), with_alpha(Color32::WHITE, 0.0)]);
        }
        // badge
        let bc = pos2(card.min.x + 44.0, card.center().y);
        let bt = t - 0.25;
        let bs = pop((bt / 0.4).clamp(0.0, 1.0));
        if bs > 0.0 {
            paint::glow(&p2, bc, 42.0, with_alpha(rc, 0.4));
            let r = 26.0 * bs;
            paint::gradient_circle(&p2, bc, r, &[mix(rc, Color32::WHITE, 0.35), glowkit_core::color::shade(rc, -0.4)]);
            p2.circle_stroke(bc, r, Stroke::new(2.0, mix(rc, Color32::WHITE, 0.6)));
            p2.circle_stroke(bc, r - 4.0, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.4)));
            self.icon.paint(&p2, bc, 28.0 * bs, Color32::WHITE);
            ring(&p2, bc, bt, 0.6, 26.0, 60.0, rc, 3.0);
            burst(&p2, bc, bt, 12, 17, 200.0, 40.0, rc, 2.4);
        }
        let tx = card.min.x + 84.0;
        let lab = ((t - 0.3) / 0.3).clamp(0.0, 1.0);
        p2.text(pos2(tx, card.min.y + 18.0), Align2::LEFT_CENTER, "ACHIEVEMENT UNLOCKED", FontId::monospace(10.0), with_alpha(rc, lab));
        p2.text(pos2(tx, card.min.y + 38.0), Align2::LEFT_CENTER, self.title, title_f, pal.text);
        let sub = match self.percent {
            Some(p) => format!("{}  ·  {p:.1}% of players", self.desc),
            None => self.desc.to_string(),
        };
        p2.text(pos2(tx, card.min.y + 58.0), Align2::LEFT_CENTER, sub, FontId::proportional(11.5), pal.dim);
        if self.points > 0 {
            let pr = Rect::from_min_size(pos2(card.max.x - 56.0, card.min.y + 10.0), vec2(44.0, 20.0));
            p2.rect_filled(pr, 10.0, with_alpha(rc, 0.22));
            p2.text(pr.center(), Align2::CENTER_CENTER, format!("{}G", self.points), FontId::monospace(11.0), mix(rc, Color32::WHITE, 0.4));
        }
        let rem = (1.0 - t / self.duration).clamp(0.0, 1.0);
        let tl = Rect::from_min_size(pos2(card.min.x + 12.0, card.max.y - 5.0), vec2((card.width() - 24.0) * rem, 2.0));
        p2.rect_filled(tl, 1.0, with_alpha(rc, 0.8));
        resp
    }
}
