use super::common::{cr, hairline, heart_filled, icon, idle_for, ink, shadow_color, surface, MicroIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::{ease, Easing, Spring};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

/// The particle effect a [`LikeButton`] plays when liked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LikeBurst {
    /// A bloom of dots flying out on a ring (default).
    #[default]
    Bloom,
    /// Firework streaks.
    Firework,
}

#[derive(Clone, Copy, Debug)]
struct LikeState {
    pop: Spring,
    since: f64,
    was: bool,
    from: u64,
    to: u64,
}

/// A like pill: heart plus count. Press it and a colored disc swells and bursts into a ring of
/// particles, the heart pops back in filled with a springy overshoot, and the counter rolls the
/// changed digits up like an odometer. Press again to unlike (the digits roll back down).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut liked = false;
/// ui.add(glowkit_effects::LikeButton::new(&mut liked, 1_300));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LikeButton<'a> {
    liked: &'a mut bool,
    count: u64,
    color: Option<Color32>,
    burst: LikeBurst,
    burst_size: f32,
    corner: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl<'a> LikeButton<'a> {
    /// A like button for `liked`; `count` is the number of likes without this one.
    pub fn new(liked: &'a mut bool, count: u64) -> Self {
        Self { liked, count, color: None, burst: LikeBurst::Bloom, burst_size: 0.5, corner: 22.0, dark: false, stroke: false, autoplay: false }
    }
    /// Heart color (default a warm red). Pass the foreground for an "ink" heart.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Burst style.
    pub fn burst(mut self, b: LikeBurst) -> Self {
        self.burst = b;
        self
    }
    /// Burst size (0..1).
    pub fn burst_size(mut self, k: f32) -> Self {
        self.burst_size = k.clamp(0.0, 1.0);
        self
    }
    /// Corner radius of the pill.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Dark (ink) pill.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Toggle itself every couple of seconds while idle (for showcases; off by default).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

impl Widget for LikeButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(15.0);
        let digit_w = ui.painter().layout_no_wrap("0".into(), font.clone(), Color32::WHITE).size().x;
        let comma_w = ui.painter().layout_no_wrap(",".into(), font.clone(), Color32::WHITE).size().x;
        let text_w = |s: &str| s.chars().map(|c| if c == ',' { comma_w } else { digit_w }).sum::<f32>();
        let max_s = group(self.count + 1);
        let size = vec2(48.0 + text_w(&max_s) + 14.0, 42.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let mut response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let id = response.id;
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let idle = idle_for(ui, id, response.hovered());
        let mut toggled = response.clicked();
        let last_auto: f64 = glowkit_core::state(ui, id.with("auto"), || now);
        if self.autoplay && idle > 2.0 && now - last_auto > 2.2 + (id.value() % 9) as f64 * 0.17 {
            toggled = true;
            glowkit_core::set_state(ui, id.with("auto"), now);
        }
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        if toggled {
            *self.liked = !*self.liked;
            response.mark_changed();
        }
        let liked = *self.liked;
        let new_total = self.count + liked as u64;
        let st = glowkit_core::with_state(
            ui,
            id,
            || LikeState { pop: Spring::new(1.0).with(380.0, 13.0), since: -10.0, was: liked, from: new_total, to: new_total },
            |s| {
                if s.was != liked {
                    s.was = liked;
                    s.since = now;
                    s.from = s.to;
                    s.to = new_total;
                    s.pop.value = if liked { 0.25 } else { 0.7 };
                    s.pop.velocity = 0.0;
                }
                // during the bloom (liking) the heart waits inside the disc
                let wait = liked && now - s.since < 0.12;
                s.pop.target = if wait { 0.0 } else { 1.0 };
                s.pop.step(dt);
                *s
            },
        );
        let age = (now - st.since) as f32;
        if age < 1.2 || !st.pop.is_settled() {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, fg) = if self.dark { (ink_bg, ink_fg) } else { (surface(&t), t.foreground) };
        let heart_col = self.color.unwrap_or(hex("#F43F5E"));
        let press = if response.is_pointer_button_down_on() { 0.96 } else { 1.0 };
        let pill = Rect::from_center_size(rect.center(), rect.size() * press);
        let r = self.corner.min(pill.height() / 2.0);
        paint::soft_shadow(painter, pill, r, 14.0, vec2(0.0, 3.0), shadow_color(&t, 0.6));
        painter.rect(pill, cr(r), bg, if self.stroke { hairline(&t) } else { Stroke::NONE }, egui::StrokeKind::Inside);
        let hc = pos2(pill.min.x + 24.0, pill.center().y);
        let hs = 17.0;
        // burst
        if liked && age < 0.9 {
            let size = 0.6 + self.burst_size;
            let disc = (age / 0.14).clamp(0.0, 1.0);
            if age < 0.2 {
                let rr = hs * 0.75 * ease(Easing::OutCubic, disc);
                let col = mix(hex("#C084FC"), heart_col, disc);
                painter.circle_filled(hc, rr, with_alpha(col, 1.0 - ((age - 0.14) / 0.06).clamp(0.0, 1.0)));
            }
            let k = ((age - 0.1) / 0.55).clamp(0.0, 1.0);
            if k > 0.0 && k < 1.0 {
                let e = ease(Easing::OutCubic, k);
                // expanding ring
                let ring_r = hs * (0.6 + 0.7 * e) * size;
                painter.circle_stroke(hc, ring_r, Stroke::new(3.0 * (1.0 - k), with_alpha(heart_col, 0.6 * (1.0 - k))));
                let n = if self.burst == LikeBurst::Bloom { 8 } else { 10 };
                let palette = [heart_col, hex("#FBBF24"), hex("#A78BFA"), hex("#34D399"), hex("#60A5FA")];
                for i in 0..n {
                    let a = i as f32 / n as f32 * TAU - TAU / 4.0;
                    let d = Vec2::angled(a);
                    let col = palette[i % palette.len()];
                    match self.burst {
                        LikeBurst::Bloom => {
                            let dist = hs * (0.7 + 0.9 * e) * size;
                            let rr = 2.6 * (1.0 - k).powf(0.7);
                            painter.circle_filled(hc + d * dist, rr, col);
                            let d2 = Vec2::angled(a + 0.28);
                            painter.circle_filled(hc + d2 * dist * 0.82, rr * 0.7, with_alpha(col, 0.8));
                        }
                        LikeBurst::Firework => {
                            let r0 = hs * (0.55 + 0.8 * e) * size;
                            let len = hs * 0.5 * (1.0 - k) * size;
                            painter.line_segment([hc + d * r0, hc + d * (r0 + len)], Stroke::new(1.8, col));
                        }
                    }
                }
            }
        }
        // heart
        let s = st.pop.value.max(0.0);
        if liked {
            heart_filled(painter, hc, hs * s, heart_col);
        } else {
            icon(painter, MicroIcon::Heart, hc, hs * 0.62 * s.max(0.3), fg, 1.6);
        }
        // rolling counter, right-aligned digit columns
        let from = group(st.from);
        let to = group(st.to);
        let up = st.to >= st.from;
        let roll = ease(Easing::OutBack, (age / 0.42).clamp(0.0, 1.0));
        let x_end = pill.min.x + 44.0 + text_w(&to);
        let clip = painter.with_clip_rect(Rect::from_min_max(pos2(pill.min.x + 38.0, pill.min.y + 6.0), pos2(pill.max.x, pill.max.y - 6.0)));
        let cy = pill.center().y;
        let travel = 16.0;
        let fc: Vec<char> = from.chars().rev().collect();
        let tc: Vec<char> = to.chars().rev().collect();
        let mut x = x_end;
        for i in 0..tc.len().max(fc.len()) {
            let nc = tc.get(i).copied();
            let oc = fc.get(i).copied();
            let w = if nc == Some(',') || (nc.is_none() && oc == Some(',')) { comma_w } else { digit_w };
            x -= w;
            let cx = x + w / 2.0;
            if nc == oc || roll >= 1.0 {
                if let Some(c) = nc {
                    clip.text(pos2(cx, cy), Align2::CENTER_CENTER, c, font.clone(), fg);
                }
            } else {
                let dir = if up { -1.0 } else { 1.0 };
                if let Some(c) = oc {
                    clip.text(pos2(cx, cy + dir * travel * roll), Align2::CENTER_CENTER, c, font.clone(), with_alpha(fg, (1.0 - roll).clamp(0.0, 1.0)));
                }
                if let Some(c) = nc {
                    clip.text(pos2(cx, cy - dir * travel * (1.0 - roll)), Align2::CENTER_CENTER, c, font.clone(), with_alpha(fg, roll.clamp(0.0, 1.0)));
                }
            }
        }
        response
    }
}
