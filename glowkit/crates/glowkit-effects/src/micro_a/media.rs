//! Media and messaging blocks: a morphing now-playing bar, emoji reactions and a voice note
//! recorder.

use super::common::{
    cr, icon_add_reaction, icon_heart, icon_mic, icon_pause, icon_play, icon_skip, icon_x, idle_for, mmss, soft_card, surface,
    track, track_fill,
};
use crate::scroll::common::{cover_uv, image_rect};
use crate::PaintedScene;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, TextureId, Ui, Vec2};
use glowkit_core::anim::{ease, lerp, Easing, Spring};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::noise::noise2;
use glowkit_core::Theme;

fn bounce_damping(bounce: f32, stiffness: f32) -> f32 {
    2.0 * stiffness.sqrt() * lerp(1.0, 0.18, (bounce / 100.0).clamp(0.0, 1.0))
}

fn lerp_pos(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    pos2(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect::from_min_max(lerp_pos(a.min, b.min, t), lerp_pos(a.max, b.max, t))
}

// ---------------------------------------------------------------------------------------------
// NowPlaying

#[derive(Clone, Copy)]
struct PlayerState {
    morph: Spring,
    expanded: bool,
    playing: bool,
    position: f32,
    liked: bool,
    auto_step: i32,
}

/// A now-playing bar that opens into a full player and folds back as one piece: the cover,
/// title, controls and progress line all glide between the two layouts on a single spring.
/// Click the card to open or fold it; the play button and heart work in both states.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::NowPlaying;
/// ui.add(NowPlaying::new("Glass Harbor", "Side A").morph(50.0).corner(16.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct NowPlaying {
    title: String,
    subtitle: String,
    cover: Option<TextureId>,
    duration: f32,
    width: f32,
    morph: f32,
    corner: f32,
    stroke: bool,
    autoplay: bool,
}

impl NowPlaying {
    /// A player for a track called `title` with a `subtitle` (artist, album side, ...).
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
            cover: None,
            duration: 214.0,
            width: 270.0,
            morph: 50.0,
            corner: 16.0,
            stroke: false,
            autoplay: false,
        }
    }
    /// Cover art texture (default: a painted scene).
    pub fn cover(mut self, tex: TextureId) -> Self {
        self.cover = Some(tex);
        self
    }
    /// Track length in seconds.
    pub fn duration(mut self, secs: f32) -> Self {
        self.duration = secs.max(1.0);
        self
    }
    /// Card width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Springiness of the open / fold morph, 0..100.
    pub fn morph(mut self, m: f32) -> Self {
        self.morph = m.clamp(0.0, 100.0);
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
    /// Play and open / fold by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for NowPlaying {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width;
        let (hc, he) = (78.0, 196.0);
        let (outer, outer_resp) = ui.allocate_exact_size(vec2(ui.available_width(), he + 12.0), Sense::hover());
        let id = outer_resp.id;
        let dt = glowkit_core::dt(ui);
        let mut st: PlayerState = glowkit_core::state(ui, id.with("player"), || PlayerState {
            morph: Spring::new(0.0),
            expanded: false,
            playing: false,
            position: 52.0,
            liked: false,
            auto_step: -1,
        });
        let m = st.morph.value;
        let card_h = lerp(hc, he, m);
        let card = Rect::from_center_size(outer.center(), vec2(w, card_h));
        let o = card.min;
        // element layouts: compact → expanded (local coordinates)
        let art = lerp_rect(Rect::from_min_size(pos2(10.0, 10.0), vec2(42.0, 42.0)), Rect::from_min_size(pos2(12.0, 12.0), vec2(56.0, 56.0)), m)
            .translate(o.to_vec2());
        let title_p = lerp_pos(pos2(62.0, 23.0), pos2(80.0, 30.0), m) + o.to_vec2();
        let sub_p = lerp_pos(pos2(62.0, 40.0), pos2(80.0, 49.0), m) + o.to_vec2();
        let play_c = lerp_pos(pos2(w - 64.0, 31.0), pos2(w / 2.0, 155.0), m) + o.to_vec2();
        let play_r = lerp(15.0, 20.0, m);
        let prev_c = lerp_pos(pos2(w - 96.0, 31.0), pos2(w / 2.0 - 46.0, 155.0), m) + o.to_vec2();
        let next_c = lerp_pos(pos2(w - 32.0, 31.0), pos2(w / 2.0 + 46.0, 155.0), m) + o.to_vec2();
        let bar_y = lerp(66.0, 92.0, m) + o.y;
        let bar = Rect::from_min_max(pos2(o.x + lerp(10.0, 12.0, m), bar_y - 1.5), pos2(o.x + w - lerp(10.0, 12.0, m), bar_y + 1.5));
        let heart_c = pos2(o.x + w - 24.0, o.y + 24.0 + (1.0 - m) * 6.0);

        let card_resp = ui.interact(card, id.with("card"), Sense::click());
        let play_resp = ui.interact(Rect::from_center_size(play_c, Vec2::splat(play_r * 2.0 + 4.0)), id.with("play"), Sense::click());
        let prev_resp = ui.interact(Rect::from_center_size(prev_c, Vec2::splat(30.0)), id.with("prev"), Sense::click());
        let next_resp = ui.interact(Rect::from_center_size(next_c, Vec2::splat(30.0)), id.with("next"), Sense::click());
        let heart_resp = ui.interact(Rect::from_center_size(heart_c, Vec2::splat(28.0)), id.with("heart"), Sense::click());
        let any_hover = card_resp.hovered() || play_resp.hovered();
        let idle = idle_for(ui, id, any_hover);
        if play_resp.clicked() {
            st.playing = !st.playing;
        } else if prev_resp.clicked() {
            st.position = 0.0;
        } else if next_resp.clicked() {
            st.position = 0.0;
            st.playing = true;
        } else if heart_resp.clicked() && m > 0.5 {
            st.liked = !st.liked;
        } else if card_resp.clicked() {
            st.expanded = !st.expanded;
        }
        if self.autoplay && !any_hover && idle > 1.0 {
            let k = ((idle - 1.0) / 2.6) as i32;
            if k != st.auto_step {
                st.auto_step = k;
                st.expanded = k % 2 == 0;
                st.playing = true;
                if k % 4 == 2 {
                    st.liked = !st.liked;
                }
            }
        }
        if st.playing {
            st.position = (st.position + dt) % self.duration;
        }
        let stiff = 200.0;
        st.morph.target = if st.expanded { 1.0 } else { 0.0 };
        st.morph.stiffness = stiff;
        st.morph.damping = bounce_damping(self.morph * 0.6, stiff);
        st.morph.step(dt);
        glowkit_core::set_state(ui, id.with("player"), st);
        let like = glowkit_core::anim::spring(ui, id.with("like"), if st.liked { 1.0 } else { 0.0 }, 300.0, 12.0);
        let play_press = glowkit_core::anim::spring(ui, id.with("pp"), if play_resp.is_pointer_button_down_on() { 0.88 } else { 1.0 }, 500.0, 20.0);
        if !st.morph.is_settled() || st.playing || self.autoplay {
            glowkit_core::animate(ui);
        }
        if play_resp.hovered() || card_resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(card) {
            return card_resp;
        }
        let painter = ui.painter();
        soft_card(painter, &t, card, self.corner, 1.0 + m * 0.6, self.stroke);
        let tex = self.cover.unwrap_or_else(|| PaintedScene::texture(ui.ctx(), 7));
        image_rect(painter, art, lerp(8.0, 12.0, m), tex, cover_uv(PaintedScene::ASPECT, 1.0), Color32::WHITE);
        painter.text(title_p, Align2::LEFT_CENTER, &self.title, FontId::proportional(lerp(14.0, 15.5, m)), t.foreground);
        painter.text(sub_p, Align2::LEFT_CENTER, &self.subtitle, FontId::proportional(lerp(11.5, 12.5, m)), t.muted_foreground);
        let ink = t.foreground;
        let fade_full = ((m - 0.5) * 2.0).clamp(0.0, 1.0);
        icon_skip(painter, prev_c, lerp(20.0, 24.0, m), -1.0, ink);
        icon_skip(painter, next_c, lerp(20.0, 24.0, m), 1.0, ink);
        let pr = play_r * play_press;
        painter.circle_filled(play_c, pr, ink);
        let on_ink = glowkit_core::color::contrast_text(ink);
        if st.playing {
            icon_pause(painter, play_c, pr * 1.6, on_ink);
        } else {
            icon_play(painter, play_c + vec2(pr * 0.08, 0.0), pr * 1.3, on_ink);
        }
        painter.rect_filled(bar, cr(1.5), track(&t));
        let f = st.position / self.duration;
        painter.rect_filled(Rect::from_min_max(bar.min, pos2(lerp(bar.min.x, bar.max.x, f), bar.max.y)), cr(1.5), ink);
        if fade_full > 0.0 {
            let tc = with_alpha(t.muted_foreground, fade_full);
            painter.text(pos2(bar.min.x, bar.max.y + 12.0), Align2::LEFT_CENTER, mmss(st.position), FontId::proportional(11.0), tc);
            painter.text(
                pos2(bar.max.x, bar.max.y + 12.0),
                Align2::RIGHT_CENTER,
                format!("-{}", mmss(self.duration - st.position)),
                FontId::proportional(11.0),
                tc,
            );
            let hs = 18.0 * (1.0 + (like - like.clamp(0.0, 1.0)).abs() * 0.8);
            let hc = mix(t.muted_foreground, t.destructive, like.clamp(0.0, 1.0));
            icon_heart(painter, heart_c, hs, with_alpha(hc, fade_full), like.clamp(0.0, 1.0) * fade_full);
        }
        card_resp
    }
}

// ---------------------------------------------------------------------------------------------
// EmojiReactions

/// Which emoji palette an [`EmojiReactions`] picker offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReactionSet {
    /// Thumbs up, party, eyes, check, rocket, fire.
    #[default]
    Work,
    /// Laughing, surprised, sad, angry, thumbs up, heart.
    Classic,
    /// Six colored hearts.
    Hearts,
}

impl ReactionSet {
    fn emoji(self) -> [(&'static str, &'static str, Color32); 6] {
        match self {
            ReactionSet::Work => [
                ("👍", "Agree", hex("#F59E0B")),
                ("🎉", "Celebrate", hex("#EC4899")),
                ("👀", "Looking", hex("#64748B")),
                ("✅", "Done", hex("#22C55E")),
                ("🚀", "Ship it", hex("#6366F1")),
                ("🔥", "On fire", hex("#F97316")),
            ],
            ReactionSet::Classic => [
                ("😂", "Haha", hex("#F59E0B")),
                ("😮", "Wow", hex("#EAB308")),
                ("😢", "Sad", hex("#38BDF8")),
                ("😡", "Angry", hex("#EF4444")),
                ("👍", "Like", hex("#3B82F6")),
                ("❤", "Love", hex("#F43F5E")),
            ],
            ReactionSet::Hearts => [
                ("❤", "Red", hex("#EF4444")),
                ("❤", "Orange", hex("#F97316")),
                ("❤", "Yellow", hex("#EAB308")),
                ("❤", "Green", hex("#22C55E")),
                ("❤", "Blue", hex("#3B82F6")),
                ("❤", "Purple", hex("#A855F7")),
            ],
        }
    }
}

#[derive(Clone)]
struct Chip {
    emoji: usize,
    count: u32,
    mine: bool,
    changed_at: f32,
    up: bool,
    born: f32,
}

#[derive(Clone)]
struct ReactState {
    open: bool,
    chips: Vec<Chip>,
    flying: Option<(usize, f32, Pos2)>,
    auto_step: i32,
    set: ReactionSet,
}

/// A chat message with reaction chips. Press the add-reaction button and the message bubble
/// morphs into a pill of emoji that magnify under the pointer (with a label); pick one and it
/// flies down in an arc to its reaction chip while the count rolls up.
///
/// The emoji are drawn with the UI font and tinted per reaction, so they read clearly on every
/// theme.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{EmojiReactions, ReactionSet};
/// ui.add(EmojiReactions::new("Merged the dark mode PR").set(ReactionSet::Work).magnify(50.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct EmojiReactions {
    message: String,
    set: ReactionSet,
    magnify: f32,
    bounce: f32,
    corner: f32,
    stroke: bool,
    autoplay: bool,
}

impl EmojiReactions {
    /// A message bubble showing `message`.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), set: ReactionSet::Work, magnify: 50.0, bounce: 45.0, corner: 20.0, stroke: false, autoplay: false }
    }
    /// Emoji palette.
    pub fn set(mut self, s: ReactionSet) -> Self {
        self.set = s;
        self
    }
    /// How much emoji grow under the pointer, 0..100.
    pub fn magnify(mut self, m: f32) -> Self {
        self.magnify = m.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of the morph and pops, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Bubble corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Draw hairline borders.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Open the picker and react by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn paint_emoji(painter: &egui::Painter, c: Pos2, size: f32, e: (&str, &str, Color32), alpha: f32) {
    if alpha <= 0.01 || size < 1.0 {
        return;
    }
    painter.text(c, Align2::CENTER_CENTER, e.0, FontId::proportional(size), with_alpha(e.2, alpha));
}

impl egui::Widget for EmojiReactions {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let set = self.set.emoji();
        let (outer, outer_resp) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
        let id = outer_resp.id;
        let now = glowkit_core::time(ui);
        let mut st: ReactState = glowkit_core::state(ui, id.with("react"), || ReactState {
            open: false,
            chips: vec![
                Chip { emoji: 4, count: 3, mine: false, changed_at: -10.0, up: true, born: -10.0 },
                Chip { emoji: 5, count: 2, mine: false, changed_at: -10.0, up: true, born: -10.0 },
            ],
            flying: None,
            auto_step: -1,
            set: self.set,
        });
        if st.set != self.set {
            st.set = self.set;
            st.flying = None;
        }
        let font = FontId::proportional(15.0);
        let msg = ui.painter().layout_no_wrap(self.message.clone(), font.clone(), t.foreground);
        let bubble_closed = msg.size() + vec2(40.0, 26.0);
        let slot = 36.0;
        let bubble_open = vec2(slot * 6.0 + 16.0, 46.0);
        let left = outer.center().x - bubble_closed.x.max(bubble_open.x) / 2.0;
        let top = outer.min.y + 22.0;
        let stiff = 260.0;
        let damp = bounce_damping(self.bounce, stiff);
        let open = glowkit_core::anim::spring(ui, id.with("open"), if st.open { 1.0 } else { 0.0 }, stiff, damp);
        let oc = open.clamp(0.0, 1.0);
        let bsize = vec2(lerp(bubble_closed.x, bubble_open.x, open), lerp(bubble_closed.y, bubble_open.y, oc));
        let bubble = Rect::from_min_size(pos2(left, top + (bubble_closed.y - bsize.y) / 2.0), bsize);
        // chips row
        let chips_y = top + bubble_closed.y + 10.0;
        let chip_h = 34.0;
        let mut x = left;
        let mut chip_rects = Vec::new();
        for (i, ch) in st.chips.iter().enumerate() {
            let born = ease(Easing::OutBack, ((now - ch.born) / 0.35).clamp(0.0, 1.0));
            let cw = 66.0 * born;
            let r = Rect::from_min_size(pos2(x, chips_y), vec2(cw, chip_h));
            chip_rects.push((i, r, born));
            x += cw + 8.0 * born;
        }
        let add_rect = Rect::from_min_size(pos2(x, chips_y), vec2(chip_h, chip_h));
        let add_resp = ui.interact(add_rect, id.with("add"), Sense::click());
        let bubble_resp = ui.interact(bubble, id.with("bubble"), Sense::click());
        let mut chip_clicked = None;
        for (i, r, _) in &chip_rects {
            let rr = ui.interact(*r, id.with(("chip", *i)), Sense::click());
            if rr.clicked() {
                chip_clicked = Some(*i);
            }
            if rr.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
        let hover = bubble_resp.hover_pos().filter(|_| oc > 0.6);
        let any = add_resp.hovered() || bubble_resp.hovered() || chip_clicked.is_some();
        let idle = idle_for(ui, id, any);
        let slot_c = |k: usize| pos2(bubble.min.x + 8.0 + slot * (k as f32 + 0.5), bubble.center().y);
        let mut hovered_slot = hover.map(|p| (((p.x - bubble.min.x - 8.0) / slot).floor() as i32).clamp(0, 5) as usize);
        let mut pick = if bubble_resp.clicked() && st.open { hovered_slot } else { None };
        if add_resp.clicked() {
            st.open = !st.open;
        }
        if let Some(i) = chip_clicked {
            let ch = &mut st.chips[i];
            ch.up = !ch.mine;
            if ch.mine {
                ch.count = ch.count.saturating_sub(1);
            } else {
                ch.count += 1;
            }
            ch.mine = !ch.mine;
            ch.changed_at = now;
        }
        let mut auto_hover_x = None;
        if self.autoplay && !any && idle > 1.0 {
            let cycle = 5.5;
            let p = (idle - 1.0) % cycle;
            let k = ((idle - 1.0) / cycle) as i32;
            if p < 0.4 {
                st.open = false;
            } else if p < 3.2 {
                st.open = true;
                if p > 1.0 {
                    let u = ((p - 1.0) / 2.0).clamp(0.0, 1.0);
                    let target = [4.0, 1.0, 3.0, 5.0][k.rem_euclid(4) as usize];
                    let fx = lerp(0.0, target + 0.5, ease(Easing::InOutSine, u));
                    auto_hover_x = Some(fx);
                    hovered_slot = Some((fx.floor() as usize).min(5));
                }
            } else if st.auto_step != k {
                st.auto_step = k;
                pick = hovered_slot.or(Some([4usize, 1, 3, 5][k.rem_euclid(4) as usize]));
                if pick.is_none() {
                    pick = Some(4);
                }
            }
        }
        if let Some(k) = pick {
            st.flying = Some((k, now, slot_c(k)));
            st.open = false;
        }
        // landing
        if let Some((k, t0, _)) = st.flying {
            if now - t0 > 0.55 {
                st.flying = None;
                if let Some(ch) = st.chips.iter_mut().find(|c| c.emoji == k) {
                    if !ch.mine {
                        ch.count += 1;
                        ch.mine = true;
                        ch.up = true;
                        ch.changed_at = now;
                    }
                } else {
                    st.chips.push(Chip { emoji: k, count: 1, mine: true, changed_at: now, up: true, born: now });
                    if st.chips.len() > 4 {
                        st.chips.remove(0);
                    }
                }
            }
        }
        let flying = st.flying;
        let chips = st.chips.clone();
        glowkit_core::set_state(ui, id.with("react"), st);
        glowkit_core::animate(ui);
        if add_resp.hovered() || (bubble_resp.hovered() && oc > 0.6) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(outer) {
            return outer_resp;
        }
        let painter = ui.painter();
        let radius = self.corner.min(bubble.height() / 2.0);
        soft_card(painter, &t, bubble, radius, 0.6 + oc * 0.6, self.stroke);
        // message text fades out as the bubble opens
        let ta = (1.0 - oc * 2.2).clamp(0.0, 1.0);
        if ta > 0.0 {
            painter.galley(bubble.center() - msg.size() / 2.0, msg.clone(), with_alpha(t.foreground, ta));
        }
        // emoji row
        let mag = self.magnify / 100.0;
        let hover_fx = auto_hover_x.or_else(|| hover.map(|p| (p.x - bubble.min.x - 8.0) / slot));
        for (k, e) in set.iter().enumerate() {
            let appear = ease(Easing::OutBack, ((oc - 0.35 - k as f32 * 0.06) / 0.4).clamp(0.0, 1.0));
            if appear <= 0.0 {
                continue;
            }
            let near = hover_fx.map(|fx| (-(fx - (k as f32 + 0.5)).powi(2) / 0.9).exp()).unwrap_or(0.0);
            let s = 22.0 * appear * (1.0 + mag * 0.8 * near);
            let c = slot_c(k) - vec2(0.0, mag * 8.0 * near);
            paint_emoji(painter, c, s, *e, appear.min(1.0));
        }
        if let (Some(k), true) = (hovered_slot, oc > 0.8) {
            let label = set[k].1;
            let g = painter.layout_no_wrap(label.into(), FontId::proportional(11.5), Color32::WHITE);
            let tip = Rect::from_center_size(slot_c(k) - vec2(0.0, 44.0), g.size() + vec2(14.0, 8.0));
            let tip_bg = if t.dark { Color32::from_gray(240) } else { Color32::from_gray(24) };
            painter.rect_filled(tip, cr(6.0), tip_bg);
            painter.text(tip.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.5), glowkit_core::color::contrast_text(tip_bg));
        }
        // chips
        for (i, r, born) in chip_rects {
            if born < 0.02 {
                continue;
            }
            let ch = &chips[i];
            let fill = if ch.mine { mix(surface(&t), t.accent, 0.1) } else { surface(&t) };
            soft_card_chip(painter, &t, r, fill, ch.mine, self.stroke);
            paint_emoji(painter, pos2(r.min.x + 20.0, r.center().y), 17.0 * born, set[ch.emoji], born);
            // rolling count
            let roll = ((now - ch.changed_at) / 0.3).clamp(0.0, 1.0);
            let e = ease(Easing::OutCubic, roll);
            let num_p = pos2(r.min.x + 46.0, r.center().y);
            let dir = if ch.up { 1.0 } else { -1.0 };
            let f = FontId::proportional(13.5);
            painter.with_clip_rect(r).text(num_p + vec2(0.0, (1.0 - e) * 14.0 * dir), Align2::CENTER_CENTER, ch.count.to_string(), f.clone(), with_alpha(t.foreground, e * born));
            if roll < 1.0 {
                let old = if ch.up { ch.count.saturating_sub(1) } else { ch.count + 1 };
                painter.with_clip_rect(r).text(num_p - vec2(0.0, e * 14.0 * dir), Align2::CENTER_CENTER, old.to_string(), f, with_alpha(t.foreground, 1.0 - e));
            }
        }
        let add_scale = glowkit_core::anim::spring(ui, id.with("add_s"), if add_resp.hovered() { 1.08 } else { 1.0 }, 400.0, 16.0);
        let ar = Rect::from_center_size(add_rect.center(), add_rect.size() * add_scale);
        soft_card_chip(painter, &t, ar, surface(&t), false, self.stroke);
        icon_add_reaction(painter, ar.center(), 20.0, mix(t.muted_foreground, t.foreground, 0.4 + oc * 0.6));
        // flying emoji (on top)
        if let Some((k, t0, from)) = flying {
            let u = ((now - t0) / 0.55).clamp(0.0, 1.0);
            let target = chips.iter().position(|c| c.emoji == k).map(|i| pos2(left + i as f32 * 74.0 + 20.0, chips_y + chip_h / 2.0)).unwrap_or(pos2(x + 20.0, chips_y + chip_h / 2.0));
            let e = ease(Easing::InOutCubic, u);
            let p = lerp_pos(from, target, e) - vec2(0.0, (u * std::f32::consts::PI).sin() * 46.0);
            paint_emoji(painter, p, lerp(30.0, 17.0, e), set[k], 1.0);
        }
        outer_resp
    }
}

fn soft_card_chip(painter: &egui::Painter, t: &Theme, r: Rect, fill: Color32, mine: bool, stroke: bool) {
    super::common::soft_card_fill(painter, t, r, r.height() / 2.0, 0.5, stroke, fill);
    if mine {
        painter.rect_stroke(r, cr(r.height() / 2.0), egui::Stroke::new(1.2, with_alpha(t.accent, 0.55)), egui::StrokeKind::Inside);
    }
}

// ---------------------------------------------------------------------------------------------
// VoiceNote

#[derive(Clone, Copy, Debug, PartialEq)]
enum VoicePhase {
    Idle,
    Recording,
    Playback,
}

#[derive(Clone)]
struct VoiceState {
    phase: VoicePhase,
    started: f32,
    samples: Vec<f32>,
    last_sample: f32,
    drag_x: f32,
    cancel: bool,
    holding: bool,
    playing: bool,
    play_pos: f32,
    length: f32,
    binned_at: f32,
    auto_phase: i32,
}

/// A press-and-hold voice recorder. Hold the pill to record: it stretches into a live waveform
/// with a pulsing dot and timer. Slide left to bin the take (the pill turns red and releases
/// into the trash), or let go to keep it and get a playback bar with a play button.
///
/// No audio is captured: the waveform is simulated, which is what a UI demo needs. Wire
/// [`VoiceNote::levels`] to real input levels for a working recorder.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::VoiceNote;
/// ui.add(VoiceNote::new().sensitivity(60.0).bars(28).corner(28.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct VoiceNote {
    label: String,
    sensitivity: f32,
    bounce: f32,
    bars: usize,
    corner: f32,
    stroke: bool,
    level: Option<f32>,
    autoplay: bool,
}

impl Default for VoiceNote {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceNote {
    /// A "Hold to record" pill.
    pub fn new() -> Self {
        Self { label: "Hold to record".into(), sensitivity: 60.0, bounce: 40.0, bars: 28, corner: 28.0, stroke: false, level: None, autoplay: false }
    }
    /// Idle label.
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// How tall the simulated waveform gets, 0..100.
    pub fn sensitivity(mut self, s: f32) -> Self {
        self.sensitivity = s.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of the pill morph, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Number of waveform bars shown (8..=64).
    pub fn bars(mut self, n: usize) -> Self {
        self.bars = n.clamp(8, 64);
        self
    }
    /// Pill corner radius in points.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Draw a hairline border.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Feed a real input level (0..1) for this frame instead of the simulated one.
    pub fn levels(mut self, level: f32) -> Self {
        self.level = Some(level.clamp(0.0, 1.0));
        self
    }
    /// Record, play back and clear by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for VoiceNote {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (outer, outer_resp) = ui.allocate_exact_size(vec2(ui.available_width(), 120.0), Sense::hover());
        let id = outer_resp.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let mut st: VoiceState = glowkit_core::state(ui, id.with("voice"), || VoiceState {
            phase: VoicePhase::Idle,
            started: 0.0,
            samples: Vec::new(),
            last_sample: 0.0,
            drag_x: 0.0,
            cancel: false,
            holding: false,
            playing: false,
            play_pos: 0.0,
            length: 0.0,
            binned_at: -10.0,
            auto_phase: -1,
        });
        let font = FontId::proportional(15.0);
        let label_w = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), t.foreground).size().x;
        let idle_w = label_w + 78.0;
        let rec_w = 256.0;
        let play_w = 250.0;
        let target_w = match st.phase {
            VoicePhase::Idle => idle_w,
            VoicePhase::Recording => rec_w,
            VoicePhase::Playback => play_w,
        };
        let stiff = 260.0;
        let damp = bounce_damping(self.bounce, stiff);
        let w = glowkit_core::anim::spring(ui, id.with("w"), target_w, stiff, damp);
        let hgt = 52.0;
        let center = pos2(outer.center().x, outer.min.y + 40.0);
        let shift = glowkit_core::anim::spring(ui, id.with("shift"), st.drag_x.max(-80.0) * 0.35, 400.0, 30.0);
        let pill = Rect::from_center_size(center + vec2(shift, 0.0), vec2(w, hgt));
        let resp = ui.interact(pill, id.with("pill"), Sense::click_and_drag());
        let down = resp.is_pointer_button_down_on();
        let idle = idle_for(ui, id, resp.hovered() || down);
        // pointer drag offset while holding
        let press_origin = ui.input(|i| i.pointer.press_origin());
        let cur = ui.input(|i| i.pointer.interact_pos());
        // auto script
        let mut auto_hold = None;
        let mut auto_click_play = false;
        let mut auto_close = false;
        if self.autoplay && !resp.hovered() && !down && idle > 0.8 {
            let cycle = 8.0;
            let p = (idle - 0.8) % cycle;
            let k = ((idle - 0.8) / cycle) as i32;
            let phase = if p < 2.6 { 0 } else if p < 6.8 { 1 } else { 2 };
            match phase {
                0 => auto_hold = Some(if k % 3 == 2 && p > 1.8 { -90.0 } else { 0.0 }),
                1 => {
                    if st.auto_phase != 1 {
                        auto_click_play = true;
                    }
                }
                _ => {
                    if st.auto_phase != 2 {
                        auto_close = true;
                    }
                }
            }
            st.auto_phase = phase;
        } else if !self.autoplay || down {
            st.auto_phase = -1;
        }
        let holding_now = down || auto_hold.is_some();
        let drag_now = if let Some(dx) = auto_hold {
            dx
        } else if down {
            match (press_origin, cur) {
                (Some(a), Some(b)) => (b.x - a.x).min(0.0),
                _ => 0.0,
            }
        } else {
            0.0
        };
        let close_rect = Rect::from_center_size(pos2(pill.max.x - 24.0, pill.center().y), Vec2::splat(26.0));
        let play_rect = Rect::from_center_size(pos2(pill.min.x + 26.0, pill.center().y), Vec2::splat(38.0));
        let close_resp = ui.interact(close_rect, id.with("close"), Sense::click());
        let play_resp = ui.interact(play_rect, id.with("play"), Sense::click());
        match st.phase {
            VoicePhase::Idle => {
                if holding_now && !st.holding {
                    st.phase = VoicePhase::Recording;
                    st.started = now;
                    st.samples.clear();
                    st.last_sample = now;
                    st.cancel = false;
                }
            }
            VoicePhase::Recording => {
                st.drag_x = drag_now;
                st.cancel = drag_now < -70.0;
                if !st.cancel && now - st.last_sample > 0.075 {
                    st.last_sample = now;
                    let el = now - st.started;
                    let raw = self.level.unwrap_or_else(|| {
                        let n = noise2(el * 3.1, 7.3) * 0.5 + 0.5;
                        let syll = ((el * 5.3).sin() * 0.5 + 0.5).powf(0.6);
                        (n * 0.75 + 0.25) * (0.35 + 0.65 * syll)
                    });
                    st.samples.push((raw * (0.35 + self.sensitivity / 100.0 * 0.9)).clamp(0.06, 1.0));
                }
                if !holding_now {
                    st.drag_x = 0.0;
                    if st.cancel || st.samples.len() < 3 {
                        st.phase = VoicePhase::Idle;
                        st.binned_at = now;
                    } else {
                        st.phase = VoicePhase::Playback;
                        st.length = now - st.started;
                        st.play_pos = 0.0;
                        st.playing = false;
                    }
                }
            }
            VoicePhase::Playback => {
                if play_resp.clicked() || auto_click_play {
                    if st.play_pos >= st.length {
                        st.play_pos = 0.0;
                    }
                    st.playing = !st.playing || auto_click_play;
                }
                if close_resp.clicked() || auto_close {
                    st.phase = VoicePhase::Idle;
                    st.playing = false;
                }
                if st.playing {
                    st.play_pos += dt;
                    if st.play_pos >= st.length {
                        st.play_pos = st.length;
                        st.playing = false;
                    }
                }
            }
        }
        st.holding = holding_now;
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("voice"), st);
        glowkit_core::animate(ui);
        let rec_a = glowkit_core::anim::follow(ui, id.with("rec_a"), if s.phase == VoicePhase::Recording { 1.0 } else { 0.0 }, 12.0);
        let play_a = glowkit_core::anim::follow(ui, id.with("play_a"), if s.phase == VoicePhase::Playback { 1.0 } else { 0.0 }, 12.0);
        let idle_a = glowkit_core::anim::follow(ui, id.with("idle_a"), if s.phase == VoicePhase::Idle { 1.0 } else { 0.0 }, 12.0);
        let cancel_a = glowkit_core::anim::follow(ui, id.with("cancel_a"), if s.cancel && s.phase == VoicePhase::Recording { 1.0 } else { 0.0 }, 14.0);
        if resp.hovered() || play_resp.hovered() || close_resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let painter = ui.painter();
        let radius = self.corner.min(hgt / 2.0);
        soft_card(painter, &t, pill, radius, 1.0, self.stroke);
        let clip = painter.with_clip_rect(pill.shrink(1.0));
        let red = t.destructive;
        // idle content
        if idle_a > 0.01 {
            let bin = ((now - s.binned_at) / 0.4).clamp(0.0, 1.0);
            let a = idle_a * if bin < 1.0 { bin } else { 1.0 };
            let total = 18.0 + 10.0 + label_w;
            let x0 = pill.center().x - total / 2.0;
            icon_mic(&clip, pos2(x0 + 9.0, pill.center().y), 22.0, with_alpha(t.foreground, a));
            clip.text(pos2(x0 + 28.0, pill.center().y), Align2::LEFT_CENTER, &self.label, font.clone(), with_alpha(t.foreground, a));
        }
        let wave_bar = |painter: &egui::Painter, x: f32, amp: f32, col: Color32| {
            let h = (4.0 + amp * 26.0).max(3.0);
            painter.rect_filled(Rect::from_center_size(pos2(x, pill.center().y), vec2(2.4, h)), cr(1.2), col);
        };
        // recording content
        if rec_a > 0.01 {
            let a = rec_a;
            let el = now - s.started;
            let left_x = pill.min.x + 22.0;
            // dot (or red X when cancelling)
            let pulse = 0.55 + 0.45 * (el * 5.0).sin().abs();
            clip.circle_filled(pos2(left_x, pill.center().y), 4.5, with_alpha(red, a * pulse * (1.0 - cancel_a)));
            if cancel_a > 0.01 {
                let r = 12.0 * ease(Easing::OutBack, cancel_a);
                clip.circle_filled(pos2(left_x + 4.0, pill.center().y), r, with_alpha(red, a));
                icon_x(&clip, pos2(left_x + 4.0, pill.center().y), 8.0, 1.6, with_alpha(Color32::WHITE, a * cancel_a));
            }
            clip.text(
                pos2(left_x + 16.0, pill.center().y),
                Align2::LEFT_CENTER,
                mmss(el),
                FontId::proportional(14.0),
                with_alpha(t.foreground, a * (1.0 - cancel_a)),
            );
            // live waveform, newest on the right; empty slots as dots
            let n = self.bars;
            let wx0 = left_x + 58.0;
            let wx1 = pill.max.x - 20.0;
            let stepx = (wx1 - wx0) / n as f32;
            let col = mix(t.foreground, track_fill(&t), cancel_a);
            for k in 0..n {
                let si = s.samples.len() as i64 - (n - k) as i64;
                let x = wx0 + (k as f32 + 0.5) * stepx;
                if si >= 0 {
                    wave_bar(&clip, x, s.samples[si as usize], with_alpha(col, a));
                } else {
                    clip.circle_filled(pos2(x, pill.center().y), 1.3, with_alpha(col, a * 0.8));
                }
            }
            let hint = if cancel_a > 0.5 { "Release to cancel" } else { "‹  Slide to cancel" };
            let hc = mix(t.muted_foreground, red, cancel_a);
            painter.text(pos2(center.x, pill.max.y + 20.0), Align2::CENTER_CENTER, hint, FontId::proportional(12.5), with_alpha(hc, a));
        }
        // playback content
        if play_a > 0.01 {
            let a = play_a;
            let pc = play_rect.center();
            let press = if play_resp.is_pointer_button_down_on() { 0.9 } else { 1.0 };
            clip.circle_filled(pc, 17.0 * press, with_alpha(t.foreground, a));
            let on_ink = with_alpha(glowkit_core::color::contrast_text(t.foreground), a);
            if s.playing {
                icon_pause(&clip, pc, 26.0, on_ink);
            } else {
                icon_play(&clip, pc + vec2(1.5, 0.0), 22.0, on_ink);
            }
            let n = self.bars.min(30);
            let wx0 = pc.x + 28.0;
            let wx1 = pill.max.x - 92.0;
            let stepx = (wx1 - wx0) / n as f32;
            let frac = if s.length > 0.0 { s.play_pos / s.length } else { 0.0 };
            for k in 0..n {
                let si = (k as f32 / n as f32 * s.samples.len() as f32) as usize;
                let amp = s.samples.get(si).copied().unwrap_or(0.1);
                let x = wx0 + (k as f32 + 0.5) * stepx;
                let played = (k as f32 + 0.5) / n as f32 <= frac;
                let col = if played { t.foreground } else { mix(track_fill(&t), t.foreground, 0.15) };
                wave_bar(&clip, x, amp * 0.85, with_alpha(col, a));
            }
            let remaining = if s.playing || s.play_pos > 0.0 { s.play_pos } else { s.length };
            clip.text(pos2(pill.max.x - 52.0, pill.center().y), Align2::RIGHT_CENTER, mmss(remaining.max(0.0) + 0.999), FontId::proportional(14.0), with_alpha(t.muted_foreground, a));
            let xc = if close_resp.hovered() { t.foreground } else { t.muted_foreground };
            if close_resp.hovered() {
                clip.circle_filled(close_rect.center(), 13.0, with_alpha(track(&t), a));
            }
            icon_x(&clip, close_rect.center(), 9.0, 1.5, with_alpha(xc, a));
        }
        resp
    }
}
