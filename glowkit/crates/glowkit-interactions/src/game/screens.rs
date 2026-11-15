//! Full-screen moments: loading screen, level-up screen and the character select carousel.

use super::common::*;
use super::dialogue::GamePortrait;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{follow, spring};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use glowkit_core::rng::hash01;
use std::f32::consts::TAU;

fn spinner(painter: &egui::Painter, c: Pos2, r: f32, t: f32, style: GameStyle, pal: &GamePalette) {
    match style {
        GameStyle::SciFi => {
            for i in 0..6 {
                let a0 = t * 2.0 + i as f32 * TAU / 6.0;
                let alpha = 0.25 + 0.75 * (((t * 3.0 - i as f32 * 0.5).sin()) * 0.5 + 0.5);
                paint::arc_band(painter, c, r * 0.7, r, a0, a0 + 0.7, with_alpha(pal.accent, alpha));
            }
        }
        GameStyle::Fantasy => {
            painter.circle_stroke(c, r, Stroke::new(1.5, pal.accent));
            painter.circle_stroke(c, r * 0.72, Stroke::new(1.0, with_alpha(pal.accent, 0.6)));
            for i in 0..8 {
                let a = t * 0.8 + i as f32 * TAU / 8.0;
                let p = c + vec2(a.cos(), a.sin()) * r * 0.86;
                paint::star(painter, p, 3.5, 1.5, 4, a, pal.accent);
            }
            paint::star(painter, c, r * 0.45, r * 0.18, 4, -t * 1.2, with_alpha(pal.accent, 0.9));
        }
        GameStyle::Minimal => {
            painter.circle_stroke(c, r * 0.8, Stroke::new(3.0, with_alpha(Color32::WHITE, 0.15)));
            let a = t * 4.0;
            paint::arc(painter, c, r * 0.8, a, a + 1.6 + (t * 2.0).sin() * 0.8, 3.0, Color32::WHITE);
        }
    }
}

/// A loading screen: the level art drifts slowly behind a vignette, the level name sits large in
/// the corner, gameplay tips rotate with a slide, and a progress bar fills with a moving sheen
/// beside a styled spinner. At 100% the bar flares and "Press any key" pulses.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GameLoadingScreen::new(0.62)
///     .level("The Sunken Vault", "Chapter III")
///     .tips(&["Hold Shift to sprint.", "Parry right before a hit lands to stagger enemies."]));
/// ```
pub struct GameLoadingScreen<'a> {
    progress: f32,
    level: &'a str,
    chapter: &'a str,
    tips: &'a [&'a str],
    tip_secs: f32,
    height: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameLoadingScreen<'a> {
    /// A loading screen at `progress` (0..1).
    pub fn new(progress: f32) -> Self {
        Self { progress, level: "", chapter: "", tips: &[], tip_secs: 4.5, height: 360.0, style: GameStyle::SciFi, palette: None }
    }
    /// Level name and a small line above it.
    pub fn level(mut self, name: &'a str, chapter: &'a str) -> Self {
        self.level = name;
        self.chapter = chapter;
        self
    }
    /// Tips to rotate through.
    pub fn tips(mut self, t: &'a [&'a str]) -> Self {
        self.tips = t;
        self
    }
    /// Seconds per tip.
    pub fn tip_seconds(mut self, s: f32) -> Self {
        self.tip_secs = s.max(1.0);
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(220.0);
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

impl egui::Widget for GameLoadingScreen<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let level_f = display_font(ui, (self.height * 0.11).clamp(24.0, 44.0));
        let tip_f = semi_font(ui, 13.5);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        glowkit_core::animate(ui);
        let id = resp.id;
        let time = glowkit_core::time(ui);
        let p = follow(ui, id.with("p"), self.progress.clamp(0.0, 1.0), 5.0);
        GameBackdrop::new(self.style).palette(pal).parallax(vec2((time * 0.1).sin(), 0.0) * 2.0).paint_at(ui.painter(), rect, time * 0.25);
        let painter = ui.painter();
        paint::gradient_rect(painter, rect, 18.0, 90.0, &[Color32::from_black_alpha(20), Color32::from_black_alpha(200)]);
        // level name
        let lx = rect.min.x + 32.0;
        if !self.chapter.is_empty() {
            outlined(painter, pos2(lx, rect.max.y - 128.0), Align2::LEFT_CENTER, &self.chapter.to_uppercase(), FontId::monospace(12.0), with_alpha(pal.accent, 0.95), Color32::from_black_alpha(120));
        }
        glow_text(painter, pos2(lx, rect.max.y - 98.0), Align2::LEFT_CENTER, self.level, level_f, Color32::WHITE, with_alpha(pal.accent, 0.35));
        // tip
        if !self.tips.is_empty() {
            let cyc = time / self.tip_secs;
            let i = cyc as usize % self.tips.len();
            let f = cyc.fract() * self.tip_secs;
            let a = (f / 0.4).min(1.0) * ((self.tip_secs - f) / 0.4).min(1.0);
            let dx = (1.0 - (f / 0.4).min(1.0)) * 20.0;
            let tw = (w * 0.5).min(460.0);
            let tr = Rect::from_min_size(pos2(lx + dx, rect.max.y - 72.0), vec2(tw, 30.0));
            let g = painter.layout(self.tips[i].to_string(), tip_f, with_alpha(Color32::WHITE, 0.9 * a), tw - 50.0);
            painter.rect_filled(Rect::from_min_size(tr.min, vec2(3.0, g.size().y.max(18.0))), 1.0, with_alpha(pal.accent, a));
            painter.text(pos2(tr.min.x + 12.0, tr.min.y + 1.0), Align2::LEFT_TOP, "TIP", FontId::monospace(10.0), with_alpha(pal.accent, a));
            painter.galley(pos2(tr.min.x + 42.0, tr.min.y), g, Color32::WHITE);
        }
        // bar
        let bar = Rect::from_min_max(pos2(lx, rect.max.y - 30.0), pos2(rect.max.x - 90.0, rect.max.y - 26.0));
        painter.rect_filled(bar, 2.0, with_alpha(Color32::WHITE, 0.15));
        let fill = Rect::from_min_size(bar.min, vec2(bar.width() * p, bar.height()));
        paint::gradient_rect(painter, fill, 2.0, 0.0, &[pal.accent2, pal.accent]);
        let sheen = (time * 0.7).fract();
        let sx = fill.min.x + fill.width() * sheen;
        if fill.width() > 30.0 {
            let band = Rect::from_min_max(pos2((sx - 30.0).max(fill.min.x), fill.min.y), pos2((sx + 30.0).min(fill.max.x), fill.max.y));
            paint::gradient_rect(painter, band, 0.0, 0.0, &[with_alpha(Color32::WHITE, 0.0), with_alpha(Color32::WHITE, 0.7), with_alpha(Color32::WHITE, 0.0)]);
        }
        paint::glow(painter, fill.right_center(), 12.0, with_alpha(pal.accent, 0.8));
        let done = p > 0.995;
        if done {
            let pulse = 0.5 + 0.5 * (time * 4.0).sin();
            paint::soft_shadow(painter, bar, 2.0, 14.0, Vec2::ZERO, with_alpha(pal.accent, 0.6 * pulse));
            outlined(painter, pos2(rect.center().x, rect.max.y - 50.0), Align2::CENTER_CENTER, "PRESS ANY KEY", FontId::monospace(13.0), with_alpha(Color32::WHITE, 0.4 + 0.6 * pulse), Color32::from_black_alpha(150));
        }
        painter.text(pos2(bar.max.x + 12.0, bar.center().y), Align2::LEFT_CENTER, format!("{:.0}%", p * 100.0), FontId::monospace(12.0), Color32::WHITE);
        spinner(painter, pos2(rect.max.x - 40.0, rect.max.y - 60.0), 16.0, time, self.style, &pal);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Level up

/// A level-up celebration. Rays wheel open behind "LEVEL UP", which slams in letter by letter
/// with a shake; the level number rolls over from the old value, a shock ring and confetti go
/// off, and the stat gains stagger in with green "+N" pops. Changing `level` replays it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::GameLevelUpScreen::new(24).stats(&[("Strength", 31.0, 34.0), ("Vitality", 22.0, 25.0)]));
/// ```
pub struct GameLevelUpScreen<'a> {
    level: u32,
    stats: &'a [(&'a str, f32, f32)],
    note: &'a str,
    height: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    backdrop: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameLevelUpScreen<'a> {
    /// Celebrate reaching `level`.
    pub fn new(level: u32) -> Self {
        Self { level, stats: &[], note: "+1 Skill Point", height: 360.0, style: GameStyle::Fantasy, palette: None, backdrop: true, cues: None }
    }
    /// Stat rows: (name, before, after).
    pub fn stats(mut self, s: &'a [(&'a str, f32, f32)]) -> Self {
        self.stats = s;
        self
    }
    /// Small reward line at the bottom.
    pub fn note(mut self, n: &'a str) -> Self {
        self.note = n;
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(240.0);
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
    /// Paint a dimmed [`GameBackdrop`] behind (default on).
    pub fn backdrop(mut self, on: bool) -> Self {
        self.backdrop = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Unlock`] when it fires).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameLevelUpScreen<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let big = display_font(ui, (self.height * 0.15).clamp(30.0, 62.0));
        let num_f = display_font(ui, (self.height * 0.2).clamp(36.0, 84.0));
        let stat_f = semi_font(ui, 13.5);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        glowkit_core::animate(ui);
        let now = ui.input(|i| i.time);
        let lvl = self.level;
        let (t, fresh) = glowkit_core::with_state(ui, id.with("lv"), || (lvl, now, false), |s| {
            if s.0 != lvl {
                *s = (lvl, now, false);
            }
            let fresh = !s.2;
            s.2 = true;
            ((now - s.1) as f32, fresh)
        });
        if fresh {
            push_cue(&mut self.cues, GameCue::Unlock);
        }
        let time = now as f32;
        let gold = if self.style == GameStyle::Fantasy { hex("#FFC94A") } else { pal.accent };
        let sh = shake(t - 0.45, 8.0, 3.0);
        if self.backdrop {
            GameBackdrop::new(self.style).palette(pal).dim(0.55).paint_at(ui.painter(), rect, time * 0.2);
        }
        let painter = ui.painter();
        let c = pos2(rect.center().x, rect.min.y + self.height * 0.3) + sh;
        let open = glowkit_core::ease(glowkit_core::Easing::OutCubic, (t / 0.8).min(1.0));
        rays(painter, c, self.height * 0.9 * open, 14, time * 0.3, with_alpha(gold, 0.18));
        paint::glow(painter, c, self.height * 0.5 * open, with_alpha(gold, 0.3));
        ring(painter, c, t - 0.45, 0.8, 20.0, w.min(700.0) * 0.55, gold, 6.0);
        burst(painter, c, t - 0.45, 40, 7, 600.0, 260.0, gold, 3.4);
        // confetti
        let conf = [hex("#FFC94A"), hex("#FF5A8A"), hex("#5AD1FF"), hex("#7CFF8A"), hex("#B86BFF")];
        for i in 0..60u64 {
            let tt = t - 0.45 - hash01(i * 5) * 0.4;
            if !(0.0..3.2).contains(&tt) {
                continue;
            }
            let a = -std::f32::consts::FRAC_PI_2 + (hash01(i * 3 + 1) - 0.5) * 2.4;
            let sp = 260.0 + 260.0 * hash01(i * 7 + 2);
            let drag = 1.0 - (-tt * 2.5).exp();
            let p = c + vec2(a.cos(), a.sin()) * sp * drag / 2.5 + vec2((tt * 3.0 + i as f32).sin() * 12.0, 90.0 * tt * tt * 0.5 + 40.0 * tt);
            if !rect.shrink(4.0).contains(p) {
                continue;
            }
            let rot = tt * (4.0 + hash01(i) * 6.0);
            let s = vec2(7.0, 3.5 * (rot * 2.0).cos().abs() + 0.5);
            let (sn, cs) = rot.sin_cos();
            let corner = |x: f32, y: f32| p + vec2(x * cs - y * sn, x * sn + y * cs);
            let fa = (1.0 - (tt - 2.4).max(0.0) / 0.8).clamp(0.0, 1.0);
            painter.add(Shape::convex_polygon(
                vec![corner(-s.x / 2.0, -s.y / 2.0), corner(s.x / 2.0, -s.y / 2.0), corner(s.x / 2.0, s.y / 2.0), corner(-s.x / 2.0, s.y / 2.0)],
                with_alpha(conf[i as usize % conf.len()], fa),
                Stroke::NONE,
            ));
        }
        // LEVEL UP letters
        let text = "LEVEL UP";
        let chars: Vec<char> = text.chars().collect();
        let widths: Vec<f32> = chars.iter().map(|ch| text_w(painter, &ch.to_string(), &big) + big.size * 0.06).collect();
        let total: f32 = widths.iter().sum();
        let mut x = c.x - total / 2.0;
        for (i, ch) in chars.iter().enumerate() {
            let k = glowkit_core::anim::stagger(t - 0.1, i, 0.045, 0.3);
            if k > 0.0 && *ch != ' ' {
                let s = 1.0 + 2.0 * (1.0 - glowkit_core::ease(glowkit_core::Easing::OutCubic, k));
                let f = FontId::new(big.size * s, big.family.clone());
                let p = pos2(x + widths[i] / 2.0, c.y);
                let g = painter.layout_no_wrap(ch.to_string(), f, Color32::WHITE);
                let r = Align2::CENTER_CENTER.anchor_size(p, g.size());
                blur(painter, r.min, &g, with_alpha(gold, 0.7 * k), 6.0);
                for d in [vec2(0.0, 3.0), vec2(2.0, 0.0), vec2(-2.0, 0.0), vec2(0.0, -2.0)] {
                    painter.galley_with_override_text_color(r.min + d, g.clone(), with_alpha(hex("#2A1600"), k));
                }
                paint::galley_with(painter, r.min, &g, |q| with_alpha(mix(Color32::WHITE, gold, ((q.y - r.min.y) / r.height()).clamp(0.0, 1.0)), k));
            }
            x += widths[i];
        }
        // level number roll
        let nk = ((t - 0.6) / 0.6).clamp(0.0, 1.0);
        let ne = glowkit_core::ease(glowkit_core::Easing::OutBack, nk);
        let nc = pos2(rect.center().x, c.y + big.size * 0.55 + num_f.size * 0.55) + sh * 0.5;
        let clip = Rect::from_center_size(nc, vec2(240.0, num_f.size * 1.15));
        let pc = painter.with_clip_rect(clip.intersect(rect));
        let old = format!("{}", lvl.saturating_sub(1));
        let new = format!("{lvl}");
        if nk < 1.0 {
            outlined(&pc, nc - vec2(0.0, num_f.size * ne), Align2::CENTER_CENTER, &old, num_f.clone(), with_alpha(Color32::WHITE, 0.6 * (1.0 - nk)), Color32::from_black_alpha(180));
        }
        if nk > 0.0 {
            let g = pc.layout_no_wrap(new.clone(), num_f.clone(), Color32::WHITE);
            let r = Align2::CENTER_CENTER.anchor_size(nc + vec2(0.0, num_f.size * (1.0 - ne)), g.size());
            blur(&pc, r.min, &g, with_alpha(gold, 0.6), 6.0);
            outlined(&pc, nc + vec2(0.0, num_f.size * (1.0 - ne)), Align2::CENTER_CENTER, &new, num_f.clone(), Color32::WHITE, Color32::from_black_alpha(200));
        }
        // stats
        let sy0 = nc.y + num_f.size * 0.75;
        let cols = self.stats.len().clamp(1, 4);
        let colw = (w * 0.8 / cols as f32).min(170.0);
        let x0 = rect.center().x - colw * cols as f32 / 2.0;
        for (i, (name, before, after)) in self.stats.iter().enumerate().take(8) {
            let k = glowkit_core::anim::stagger(t - 1.0, i, 0.12, 0.35);
            if k <= 0.0 {
                continue;
            }
            let row = i / cols;
            let col = i % cols;
            let cc = pos2(x0 + (col as f32 + 0.5) * colw, sy0 + row as f32 * 44.0 + (1.0 - k) * 14.0);
            let a = k;
            outlined(painter, cc - vec2(0.0, 9.0), Align2::CENTER_CENTER, &name.to_uppercase(), FontId::monospace(10.5), with_alpha(pal.hud, 0.8 * a), with_alpha(Color32::BLACK, 0.6 * a));
            let val = format!("{}", *after as i64);
            let d = after - before;
            let vw = text_w(painter, &val, &stat_f);
            outlined(painter, cc + vec2(-6.0, 9.0), Align2::CENTER_CENTER, &val, stat_f.clone(), with_alpha(Color32::WHITE, a), with_alpha(Color32::BLACK, 0.7 * a));
            let pk = glowkit_core::anim::stagger(t - 1.2, i, 0.12, 0.25);
            let ps = 1.0 + 0.5 * (1.0 - pk) * (pk > 0.0) as i32 as f32;
            if pk > 0.0 {
                outlined(painter, cc + vec2(vw / 2.0 - 2.0, 9.0), Align2::LEFT_CENTER, &format!("+{}", d as i64), FontId::proportional(12.5 * ps), with_alpha(hex("#6BF59A"), pk), with_alpha(Color32::BLACK, 0.7 * pk));
            }
        }
        if !self.note.is_empty() {
            let k = ((t - 1.8) / 0.4).clamp(0.0, 1.0);
            let pulse = 0.75 + 0.25 * (time * 3.0).sin();
            let nr = Rect::from_center_size(pos2(rect.center().x, rect.max.y - 26.0), vec2(text_w(painter, self.note, &FontId::monospace(12.0)) + 36.0, 26.0));
            let mut p2 = painter.clone();
            p2.set_opacity(k);
            p2.rect_filled(nr, 13.0, with_alpha(gold, 0.18 * pulse));
            p2.rect_stroke(nr, 13.0, Stroke::new(1.0, with_alpha(gold, 0.8)), egui::StrokeKind::Inside);
            p2.text(nr.center(), Align2::CENTER_CENTER, self.note.to_uppercase(), FontId::monospace(12.0), mix(gold, Color32::WHITE, 0.4));
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Character select

/// A playable character for [`GameCharacterSelect`].
#[derive(Clone, Debug, PartialEq)]
pub struct GameCharacter {
    /// Name.
    pub name: String,
    /// Class / role.
    pub class: String,
    /// Short bio.
    pub bio: String,
    /// Painted portrait.
    pub portrait: GamePortrait,
    /// Signature color.
    pub color: Color32,
    /// Stats, 0..1 each.
    pub stats: Vec<(String, f32)>,
}

impl GameCharacter {
    /// A character.
    pub fn new(name: impl Into<String>, class: impl Into<String>, portrait: GamePortrait) -> Self {
        Self { name: name.into(), class: class.into(), bio: String::new(), color: portrait.trim, portrait, stats: Vec::new() }
    }
    /// Bio text.
    pub fn bio(mut self, b: impl Into<String>) -> Self {
        self.bio = b.into();
        self
    }
    /// Add a stat (0..1).
    pub fn stat(mut self, name: impl Into<String>, v: f32) -> Self {
        self.stats.push((name.into(), v.clamp(0.0, 1.0)));
        self
    }
    /// Signature color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        self
    }
}

/// A character select carousel. Cards ride a spring along an arc: the chosen hero stands tall in
/// the middle with a colored aura, neighbours shrink, dim and fall back. The details panel slides
/// the name in and refills the stat bars for each pick, and the confirm button breathes in the
/// character's color. Click a side card, use the arrows, drag, or press Left / Right / Enter.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameCharacter, GameCharacterSelect, GamePortrait};
/// let roster = vec![GameCharacter::new("Aria", "Vanguard", GamePortrait::hero()).stat("Power", 0.7)];
/// let mut sel = 0usize;
/// ui.add(GameCharacterSelect::new(&roster, &mut sel));
/// ```
pub struct GameCharacterSelect<'a> {
    roster: &'a [GameCharacter],
    selected: &'a mut usize,
    height: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    autoplay: bool,
    confirmed: Option<&'a mut bool>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameCharacterSelect<'a> {
    /// A carousel over `roster`, editing `selected`.
    pub fn new(roster: &'a [GameCharacter], selected: &'a mut usize) -> Self {
        Self { roster, selected, height: 400.0, style: GameStyle::SciFi, palette: None, autoplay: false, confirmed: None, cues: None }
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(300.0);
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
    /// Demo mode: browse on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Set to true when the player confirms.
    pub fn confirmed(mut self, c: &'a mut bool) -> Self {
        self.confirmed = Some(c);
        self
    }
    /// Collect feedback cues ([`GameCue::Tick`], [`GameCue::Confirm`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameCharacterSelect<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let name_f = display_font(ui, 30.0);
        let card_f = display_font(ui, 14.0);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::click_and_drag());
        if !ui.is_rect_visible(rect) || self.roster.is_empty() {
            return resp;
        }
        let id = resp.id;
        glowkit_core::animate(ui);
        let time = glowkit_core::time(ui);
        let n = self.roster.len();
        let mut sel = (*self.selected).min(n - 1);
        let wide = w > 640.0;
        let stage = if wide { Rect::from_min_max(rect.min, pos2(rect.min.x + w * 0.62, rect.max.y)) } else { Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + self.height * 0.62)) };
        let card_h = (stage.height() * 0.72).min(300.0);
        let card_w = card_h * 0.66;
        let spacing = card_w * 0.78;
        // input
        let (left, right, enter) = ui.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::Enter)));
        if left {
            sel = (sel + n - 1) % n;
        }
        if right {
            sel = (sel + 1) % n;
        }
        let drag = glowkit_core::with_state(ui, id.with("drag"), || 0.0f32, |d| *d);
        let mut drag_now = drag;
        if resp.dragged() {
            drag_now += resp.drag_delta().x;
        }
        if resp.drag_stopped() {
            let steps = (-drag_now / spacing).round() as i64;
            sel = ((sel as i64 + steps).rem_euclid(n as i64)) as usize;
            drag_now = 0.0;
        }
        glowkit_core::set_state(ui, id.with("drag"), drag_now);
        let la = Rect::from_center_size(pos2(stage.min.x + 26.0, stage.center().y), vec2(36.0, 60.0));
        let ra = Rect::from_center_size(pos2(stage.max.x - 26.0, stage.center().y), vec2(36.0, 60.0));
        let lr = ui.interact(la, id.with("la"), Sense::click());
        let rr = ui.interact(ra, id.with("ra"), Sense::click());
        if lr.clicked() {
            sel = (sel + n - 1) % n;
        }
        if rr.clicked() {
            sel = (sel + 1) % n;
        }
        // card clicks
        let pos_f = spring(ui, id.with("pos"), sel as f32, 170.0, 20.0) - drag_now / spacing;
        let card_rect = |i: usize| {
            let d = i as f32 - pos_f;
            let s = 1.0 / (1.0 + d.abs() * 0.32);
            let c = pos2(stage.center().x + d * spacing * (1.0 - d.abs().min(2.0) * 0.08), stage.center().y + d.abs() * 10.0 - 6.0);
            (Rect::from_center_size(c, vec2(card_w, card_h) * s), d, s)
        };
        let mut any = lr.hovered() || rr.hovered() || resp.dragged();
        for i in 0..n {
            let (r, d, _) = card_rect(i);
            if d.abs() < 2.5 {
                let cr = ui.interact(r, id.with(("card", i)), Sense::click());
                any |= cr.hovered();
                if cr.clicked() {
                    sel = i;
                }
            }
        }
        // confirm button
        let btn = if wide {
            Rect::from_min_size(pos2(stage.max.x + 20.0, rect.max.y - 66.0), vec2(rect.max.x - stage.max.x - 44.0, 44.0))
        } else {
            Rect::from_center_size(pos2(rect.center().x, rect.max.y - 30.0), vec2(220.0, 40.0))
        };
        let br = ui.interact(btn, id.with("btn"), Sense::click());
        any |= br.hovered();
        let idle = idle_for(ui, id, any);
        if self.autoplay && idle > 2.0 {
            let k = ((idle - 2.0) / 2.2) as usize;
            sel = (k + 1) % n;
        }
        if sel != *self.selected {
            *self.selected = sel;
            push_cue(&mut self.cues, GameCue::Tick);
        }
        if br.clicked() || enter {
            push_cue(&mut self.cues, GameCue::Confirm);
            if let Some(c) = self.confirmed.as_deref_mut() {
                *c = true;
            }
            glowkit_core::anim::Clock::restart(ui, id.with("confirm"));
        }
        let (since, _, _) = changed(ui, id.with("sel"), sel as f32);
        let hero = &self.roster[sel];
        // backdrop tinted by the hero
        let mut bp = pal;
        bp.sky = [pal.sky[0], mix(pal.sky[1], hero.color, 0.25), mix(pal.sky[2], hero.color, 0.4)];
        GameBackdrop::new(self.style).palette(bp).dim(0.35).paint_at(ui.painter(), rect, time * 0.2);
        let painter = ui.painter();
        paint::glow_ellipse(painter, pos2(stage.center().x, stage.center().y + card_h * 0.45), vec2(card_w * 1.3, 26.0), with_alpha(hero.color, 0.55));
        // cards back to front
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| card_rect(*b).1.abs().total_cmp(&card_rect(*a).1.abs()));
        let hint = |r: Rect| r.intersects(stage);
        for i in order {
            let (r, d, s) = card_rect(i);
            if d.abs() > 2.6 || !hint(r) {
                continue;
            }
            let ch = &self.roster[i];
            let center = d.abs() < 0.5;
            let dim = (d.abs() * 0.45).min(0.75);
            if center {
                paint::glow(painter, r.center(), card_h * 0.75, with_alpha(ch.color, 0.35));
            }
            paint::soft_shadow(painter, r, 12.0, 24.0, vec2(0.0, 10.0), Color32::from_black_alpha(160));
            let pts = paint::rounded_rect_points(r, 12.0 * s, 4);
            paint::fill_convex_fn(painter, &pts, 3, paint::linear_fn(r, 90.0, &[mix(ch.portrait.bg[0], hex("#000000"), 0.2), ch.portrait.bg[1]]));
            let pr = Rect::from_min_max(r.min + vec2(0.0, r.height() * 0.06), pos2(r.max.x, r.max.y - r.height() * 0.2));
            let pp = Rect::from_center_size(pr.center(), vec2(pr.height().min(r.width()), pr.height()));
            ch.portrait.paint(painter, pp.intersect(r), time + i as f32, false);
            // name band
            let band = Rect::from_min_max(pos2(r.min.x, r.max.y - r.height() * 0.2), r.max);
            painter.rect_filled(band, egui::CornerRadius { nw: 0, ne: 0, sw: (12.0 * s) as u8, se: (12.0 * s) as u8 }, hex("#05070CE0"));
            painter.rect_filled(Rect::from_min_size(band.min, vec2(band.width(), 2.0)), 0.0, ch.color);
            painter.text(band.center() - vec2(0.0, 6.0 * s), Align2::CENTER_CENTER, ch.name.to_uppercase(), FontId::new(card_f.size * s, card_f.family.clone()), Color32::WHITE);
            painter.text(band.center() + vec2(0.0, 10.0 * s), Align2::CENTER_CENTER, &ch.class, FontId::proportional(10.5 * s), with_alpha(ch.color, 0.95));
            let edge = if center { mix(ch.color, Color32::WHITE, 0.3) } else { with_alpha(Color32::WHITE, 0.15) };
            painter.rect_stroke(r, 12.0 * s, Stroke::new(if center { 2.5 } else { 1.0 }, edge), egui::StrokeKind::Inside);
            if dim > 0.0 {
                painter.rect_filled(r, 12.0 * s, Color32::from_black_alpha((dim * 230.0) as u8));
            }
        }
        // arrows
        for (r, dir, hov) in [(la, -1.0, lr.hovered()), (ra, 1.0, rr.hovered())] {
            let c = r.center() + vec2(dir * if hov { 3.0 } else { 0.0 }, 0.0);
            let col = if hov { pal.accent } else { with_alpha(Color32::WHITE, 0.7) };
            painter.add(Shape::line(vec![c + vec2(-dir * 7.0, -14.0), c + vec2(dir * 7.0, 0.0), c + vec2(-dir * 7.0, 14.0)], Stroke::new(3.0, col)));
        }
        // details
        let det = if wide { Rect::from_min_max(pos2(stage.max.x + 20.0, rect.min.y + 36.0), pos2(rect.max.x - 24.0, btn.min.y - 12.0)) } else { Rect::from_min_max(pos2(rect.min.x + 24.0, stage.max.y), pos2(rect.max.x - 24.0, btn.min.y - 6.0)) };
        let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, (since / 0.4).min(1.0));
        let mut p2 = painter.clone();
        p2.set_opacity(e);
        let dx = (1.0 - e) * 30.0;
        outlined(&p2, pos2(det.min.x + dx, det.min.y + 6.0), Align2::LEFT_TOP, &hero.class.to_uppercase(), FontId::monospace(11.0), hero.color, Color32::from_black_alpha(150));
        let nr = glow_text(&p2, pos2(det.min.x + dx, det.min.y + 22.0), Align2::LEFT_TOP, &hero.name.to_uppercase(), name_f, Color32::WHITE, with_alpha(hero.color, 0.5));
        let mut y = nr.max.y + 8.0;
        if wide && !hero.bio.is_empty() {
            let g = p2.layout(hero.bio.clone(), FontId::proportional(12.0), with_alpha(Color32::WHITE, 0.8), det.width());
            let gh = g.size().y;
            p2.galley(pos2(det.min.x + dx, y), g, Color32::WHITE);
            y += gh + 14.0;
        }
        for (i, (name, v)) in hero.stats.iter().enumerate() {
            if y > det.max.y - 10.0 {
                break;
            }
            let k = glowkit_core::anim::stagger(since - 0.1, i, 0.07, 0.5);
            p2.text(pos2(det.min.x, y), Align2::LEFT_CENTER, name.to_uppercase(), FontId::monospace(10.0), with_alpha(Color32::WHITE, 0.75));
            let bar = Rect::from_min_size(pos2(det.min.x + 80.0, y - 3.0), vec2(det.width() - 80.0, 6.0));
            let segs = 10;
            let sw = bar.width() / segs as f32;
            let fill = v * glowkit_core::ease(glowkit_core::Easing::OutCubic, k);
            for s in 0..segs {
                let sr = Rect::from_min_size(pos2(bar.min.x + s as f32 * sw, bar.min.y), vec2(sw - 2.0, bar.height()));
                let on = (s as f32 + 0.5) / segs as f32 <= fill;
                p2.rect_filled(sr, 1.0, if on { mix(hero.color, Color32::WHITE, 0.15) } else { with_alpha(Color32::WHITE, 0.12) });
            }
            y += 20.0;
        }
        // button
        let pulse = 0.5 + 0.5 * (time * 2.5).sin();
        let ct = glowkit_core::anim::Clock::elapsed(ui, id.with("confirm"));
        let press = if ct < 0.25 { 1.0 - ct / 0.25 } else { 0.0 };
        let b = btn.shrink(press * 3.0);
        paint::soft_shadow(painter, b, 6.0, 18.0, Vec2::ZERO, with_alpha(hero.color, 0.35 + 0.35 * pulse + 0.3 * br.hovered() as i32 as f32));
        let bpts = chamfer_points(b, 10.0);
        paint::fill_convex_fn(painter, &bpts, 2, paint::linear_fn(b, 0.0, &[hero.color, mix(hero.color, Color32::WHITE, 0.3)]));
        painter.text(b.center(), Align2::CENTER_CENTER, "SELECT", display_font(ui, 16.0), glowkit_core::color::contrast_text(hero.color));
        if ct < 0.6 {
            ring(painter, b.center(), ct, 0.6, 20.0, 140.0, hero.color, 3.0);
        }
        // roster pips
        let py = if wide { rect.max.y - 18.0 } else { stage.max.y - 8.0 };
        for i in 0..n {
            let on = i == sel;
            let c = pos2(stage.center().x + (i as f32 - (n as f32 - 1.0) / 2.0) * 14.0, py);
            painter.circle_filled(c, if on { 4.0 } else { 2.5 }, if on { hero.color } else { with_alpha(Color32::WHITE, 0.35) });
        }
        resp
    }
}
