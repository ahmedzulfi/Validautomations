//! A floating pill at the top of the screen that morphs between a compact live activity, an
//! expanded card and brief notices.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Context, Id, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

/// A notice shown by [`LiveIsland`]: a title, a short body and an accent color.
#[derive(Clone, Debug, PartialEq)]
pub struct IslandNotice {
    /// First line.
    pub title: String,
    /// Second line.
    pub body: String,
    /// Accent of the leading badge.
    pub accent: Color32,
    /// Initials or a short glyph in the badge.
    pub badge: String,
}

impl IslandNotice {
    /// A notice with a title and body.
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into(), accent: hexa(0x34, 0xC7, 0x59), badge: "M".into() }
    }
    /// Badge color.
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = c;
        self
    }
    /// Badge text (one or two letters).
    pub fn badge(mut self, b: impl Into<String>) -> Self {
        self.badge = b.into();
        self
    }
}

#[derive(Clone, Copy, Default)]
struct Dim {
    v: f32,
    vel: f32,
}

#[derive(Clone, Default)]
struct IslandState {
    expanded: bool,
    notice: Option<(IslandNotice, f32)>,
    queue: Vec<IslandNotice>,
    w: Dim,
    h: Dim,
    r: Dim,
    press: Dim,
    pull: f32,
    g: Gesture,
    cycle: u32,
    started: f32,
    auto_notice: u32,
    init: bool,
}

/// The pill that lives at the top of a phone screen, like a dynamic island.
///
/// It shows a compact live activity (a countdown timer with a ticking ring). Tap it, or pull it
/// down, and it springs open into a large rounded card with the full activity and buttons; tap
/// again or push it up to fold it back. Notices pushed with [`LiveIsland::notify`] temporarily
/// take over the pill (it stretches sideways to fit them) and then hand it back to the activity.
/// Width, height and corner radius all ride on springs, so every change wobbles slightly as if
/// the pill were made of something soft.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{LiveIsland, IslandNotice};
/// ui.add(LiveIsland::new("island").bounce(45.0));
/// if ui.button("Ping").clicked() {
///     LiveIsland::notify(ui.ctx(), "island", IslandNotice::new("Maya", "Running 5 min late"));
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LiveIsland {
    id_salt: Id,
    size: Option<Vec2>,
    bounce: f32,
    duration: f32,
    timer: f32,
    label: String,
    accent: Color32,
    autoplay: bool,
}

impl LiveIsland {
    /// An island; notices are addressed to it with the same `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            size: None,
            bounce: 45.0,
            duration: 2.6,
            timer: 300.0,
            label: "Timer".into(),
            accent: hexa(0xFF, 0x9F, 0x0A),
            autoplay: false,
        }
    }
    /// Queue a notice on the island created with `id_salt`.
    pub fn notify(ctx: &Context, id_salt: impl std::hash::Hash + std::fmt::Debug, notice: IslandNotice) {
        let id = Id::unique(id_salt).with("live_island");
        ctx.data_mut(|d| {
            let st: &mut IslandState = d.get_temp_mut_or_default(id);
            st.queue.push(notice);
        });
    }
    /// Area the island lives in (it sits at the top center). Default: available width x 220.
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Wobble of the morph, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// How long a notice stays, in seconds.
    pub fn notice_duration(mut self, s: f32) -> Self {
        self.duration = s.clamp(0.8, 10.0);
        self
    }
    /// Length of the demo countdown in seconds.
    pub fn timer(mut self, seconds: f32) -> Self {
        self.timer = seconds.max(5.0);
        self
    }
    /// Activity label shown when expanded.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
    /// Accent color of the activity.
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = c;
        self
    }
    /// Tap the island and post notices on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SCRIPT: [Stroke2; 4] = [
    tap(1.0, 0.5, 0.06),
    tap(3.6, 0.5, 0.2),
    drag(8.6, 9.1, (0.5, 0.06), (0.5, 0.2)),
    drag(11.4, 11.8, (0.5, 0.35), (0.5, 0.12)),
];

fn mmss(s: f32) -> String {
    let s = s.max(0.0) as i32;
    format!("{}:{:02}", s / 60, s % 60)
}

impl egui::Widget for LiveIsland {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(vec2(ui.available_width(), 220.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let sid = self.id_salt.with("live_island");
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 14.0);
        let mut st: IslandState = glowkit_core::state(ui, sid, IslandState::default);
        let compact = vec2(rect.width() * 0.52, 32.0).min(vec2(200.0, 32.0));
        if !st.init {
            st.init = true;
            st.w.v = compact.x;
            st.h.v = compact.y;
            st.r.v = 16.0;
            st.started = now;
        }
        if touch.ghost && touch.cycle != st.cycle {
            st.cycle = touch.cycle;
            st.expanded = false;
            st.auto_notice = 0;
        }
        if touch.ghost {
            // notices arrive on their own in the demo loop
            let k = if touch.t > 5.4 { 1 } else { 0 } + if touch.t > 12.6 { 1 } else { 0 };
            if k > st.auto_notice {
                st.auto_notice = k;
                st.queue.push(if k == 1 {
                    IslandNotice::new("Maya Chen", "On my way, 5 min").badge("MC").accent(hexa(0x34, 0xC7, 0x59))
                } else {
                    IslandNotice::new("AirPods Pro", "Connected").badge("◖◗").accent(hexa(0x0A, 0x84, 0xFF))
                });
            }
        }
        if st.notice.as_ref().is_some_and(|(_, t0)| now - t0 > self.duration) {
            st.notice = None;
        }
        if st.notice.is_none() && !st.queue.is_empty() {
            let n = st.queue.remove(0);
            st.notice = Some((n, now));
        }

        let full = vec2(rect.width() - 16.0, 150.0_f32.min(rect.height() - 20.0));
        let notice_sz = vec2(rect.width() - 34.0, 62.0);
        let (tw, th, tr) = if st.notice.is_some() {
            (notice_sz.x, notice_sz.y, 31.0)
        } else if st.expanded {
            (full.x, full.y, 38.0)
        } else {
            (compact.x, compact.y, 16.0)
        };
        let top = rect.min.y + 8.0;
        let pill = Rect::from_min_size(pos2(rect.center().x - st.w.v / 2.0, top), vec2(st.w.v, st.h.v + st.pull));

        // gestures
        let g = st.g.update(&touch, now, dt);
        if st.g.down && pill.expand(10.0).contains(st.g.start) {
            let dy = st.g.last.y - st.g.start.y;
            st.pull =
                if st.expanded { rubber(dy.min(0.0), 40.0) + rubber(dy.max(0.0), 20.0) } else { rubber(dy, 46.0) };
        }
        if g.released {
            let on_pill = pill.expand(10.0).contains(st.g.start);
            let dy = st.g.last.y - st.g.start.y;
            if on_pill && !g.tap {
                if !st.expanded && (dy > 22.0 || st.g.vel.y > 500.0) {
                    st.expanded = true;
                } else if st.expanded && (dy < -22.0 || st.g.vel.y < -500.0) {
                    st.expanded = false;
                }
                st.h.vel += st.g.vel.y.clamp(-900.0, 900.0) * 0.5;
            } else if g.tap {
                if on_pill && st.notice.is_none() {
                    st.expanded = !st.expanded;
                } else if !on_pill {
                    st.expanded = false;
                }
            } else if !on_pill && st.expanded && dy < -20.0 {
                st.expanded = false;
            }
            st.h.v += st.pull;
            st.pull = 0.0;
        }
        let press_t = if st.g.down && pill.expand(10.0).contains(st.g.start) { 1.0 } else { 0.0 };

        let stiff = 230.0;
        let damp = damping_for(self.bounce, stiff);
        spring_step(&mut st.w.v, &mut st.w.vel, tw, stiff, damp, dt);
        spring_step(&mut st.h.v, &mut st.h.vel, th, stiff * 0.9, damping_for(self.bounce, stiff * 0.9), dt);
        spring_step(&mut st.r.v, &mut st.r.vel, tr, stiff, damp, dt);
        spring_step(&mut st.press.v, &mut st.press.vel, press_t, 500.0, 30.0, dt);
        let notice = st.notice.clone();
        let moving = !settled(st.w.v, st.w.vel, tw) || !settled(st.h.v, st.h.vel, th) || st.g.down;
        let started = st.started;
        let expanded = st.expanded;
        let (w, h, r, press) = (st.w.v, st.h.v + st.pull, st.r.v, st.press.v);
        glowkit_core::set_state(ui, sid, st);
        // the timer ticks, so keep repainting at a calm rate
        if moving || notice.is_some() || touch.ghost {
            ui.ctx().request_repaint();
        } else {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect.expand(30.0));
        let scale = 1.0 - press * 0.03;
        let sz = vec2(w, h) * scale;
        let pill = Rect::from_min_size(pos2(rect.center().x - sz.x / 2.0, top + (h - sz.y) * 0.5), sz);
        let radius = r.min(sz.y / 2.0).min(sz.x / 2.0);
        if h > 40.0 {
            shadow(&painter, &t, pill, radius, ((h - 40.0) / 80.0).clamp(0.0, 1.2));
        }
        painter.rect_filled(pill, cr(radius), Color32::BLACK);
        if t.dark {
            painter.rect_stroke(
                pill,
                cr(radius),
                egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.08)),
                egui::StrokeKind::Inside,
            );
        }
        let remaining = self.timer - ((now - started) % self.timer);
        let frac = remaining / self.timer;
        let white = Color32::WHITE;
        let dim = with_alpha(white, 0.6);
        let ac = self.accent;
        let content = painter.with_clip_rect(pill.shrink(2.0));
        // compact
        let a_compact = if notice.is_some() { 0.0 } else { 1.0 - ramp(h, compact.y + 8.0, compact.y + 40.0) };
        if a_compact > 0.01 {
            let cy = pill.center().y;
            let ring_c = pos2(pill.min.x + 18.0, cy);
            paint::arc(&content, ring_c, 7.5, 0.0, TAU, 2.4, with_alpha(ac, 0.25 * a_compact));
            paint::arc(&content, ring_c, 7.5, -PI / 2.0, -PI / 2.0 + TAU * frac, 2.4, with_alpha(ac, a_compact));
            text(
                &content,
                pos2(pill.max.x - 14.0, cy),
                Align2::RIGHT_CENTER,
                mmss(remaining),
                13.0,
                with_alpha(ac, a_compact),
            );
        }
        // expanded
        let a_exp = if notice.is_some() {
            0.0
        } else if expanded {
            ramp(h, full.y * 0.6, full.y * 0.95)
        } else {
            ramp(h, full.y * 0.85, full.y)
        };
        if a_exp > 0.01 {
            let x0 = pill.min.x + 22.0;
            let y0 = pill.min.y + 22.0;
            text(&content, pos2(x0, y0 + 6.0), Align2::LEFT_CENTER, &self.label, 13.0, with_alpha(ac, a_exp));
            text(&content, pos2(x0, y0 + 26.0), Align2::LEFT_CENTER, "Pasta water", 12.0, with_alpha(dim, a_exp));
            text_bold(
                &content,
                pos2(x0 - 1.0, y0 + 64.0),
                Align2::LEFT_CENTER,
                mmss(remaining),
                40.0,
                with_alpha(white, a_exp),
            );
            let rc = pos2(pill.max.x - 52.0, y0 + 34.0);
            paint::arc(&content, rc, 26.0, 0.0, TAU, 5.0, with_alpha(ac, 0.22 * a_exp));
            paint::arc(&content, rc, 26.0, -PI / 2.0, -PI / 2.0 + TAU * frac, 5.0, with_alpha(ac, a_exp));
            let by = pill.max.y - 28.0;
            let b1 = pos2(pill.max.x - 76.0, by);
            let b2 = pos2(pill.max.x - 34.0, by);
            if pill.max.y - 10.0 > by {
                content.circle_filled(b1, 16.0, with_alpha(mix(ac, Color32::BLACK, 0.55), a_exp));
                for dx in [-3.0, 3.0] {
                    content.rect_filled(
                        Rect::from_center_size(b1 + vec2(dx, 0.0), vec2(3.0, 11.0)),
                        cr(1.0),
                        with_alpha(ac, a_exp),
                    );
                }
                content.circle_filled(b2, 16.0, with_alpha(hexa(60, 60, 66), a_exp));
                glyph(&content, Glyph::Close, b2, 10.0, with_alpha(white, a_exp));
            }
        }
        // notice
        if let Some((n, t0)) = &notice {
            let a = ramp(w, compact.x + 10.0, notice_sz.x - 6.0)
                * (1.0 - ramp(now - t0, self.duration - 0.25, self.duration));
            if a > 0.01 {
                let c = pos2(pill.min.x + 32.0, pill.center().y);
                content.circle_filled(c, 19.0, with_alpha(n.accent, a));
                text(&content, c, Align2::CENTER_CENTER, &n.badge, 12.0, with_alpha(white, a));
                text_bold(
                    &content,
                    pos2(c.x + 30.0, c.y - 9.0),
                    Align2::LEFT_CENTER,
                    &n.title,
                    13.0,
                    with_alpha(white, a),
                );
                text(&content, pos2(c.x + 30.0, c.y + 10.0), Align2::LEFT_CENTER, &n.body, 12.0, with_alpha(dim, a));
                // a little audio-wave flourish on the right
                for i in 0..4 {
                    let hh = 5.0 + 9.0 * (0.5 + 0.5 * (now * 7.0 + i as f32 * 1.3).sin());
                    let x = pill.max.x - 40.0 + i as f32 * 5.0;
                    content.rect_filled(
                        Rect::from_center_size(pos2(x, c.y), vec2(2.6, hh)),
                        cr(1.3),
                        with_alpha(n.accent, a),
                    );
                }
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
