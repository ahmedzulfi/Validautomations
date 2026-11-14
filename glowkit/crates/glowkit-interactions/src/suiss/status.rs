//! Account and service status: daily uptime bars and an active-sessions list.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::rng::hash01;
use glowkit_core::Theme;

// ---------------------------------------------------------------------------------------------
// UptimeBars

#[derive(Clone, Default)]
struct UptimeState {
    range: usize,
    shown: [f32; 3],
    shown_v: [f32; 3],
    grow: Vec<f32>,
    grow_v: Vec<f32>,
    hover: Option<(usize, usize)>,
    tip: f32,
    tip_v: f32,
    tip_x: f32,
    tip_xv: f32,
    reveal_t: f32,
    g: Gesture,
    cycle: u32,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn day_label(days_ago: usize) -> String {
    // a fixed calendar ending on Nov 14
    let lens = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 10usize;
    let mut d = 14i32 - days_ago as i32;
    while d < 1 {
        m = (m + 11) % 12;
        d += lens[m];
    }
    format!("{} {}", MONTHS[m], d)
}

/// A per-service daily status strip: one bar per day, colored by how the day went.
///
/// Each row is a service with its current status and a run of thin bars (green operational,
/// amber degraded, red down). Hover or press and scrub along a row: the bar under the finger
/// swells on a spring with its neighbours in a soft fisheye, and a tooltip glides along with the
/// date, that day's uptime and any incident. Switch the range (30 / 60 / 90 days) and the bars
/// cascade in again while the uptime figure counts to its new value.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::UptimeBars;
/// ui.add(UptimeBars::new(&["API", "Dashboard"]).days(90));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct UptimeBars {
    services: Vec<String>,
    days: usize,
    magnify: f32,
    seed: u64,
    autoplay: bool,
}

impl UptimeBars {
    /// A strip per service (up to three).
    pub fn new(services: &[&str]) -> Self {
        Self {
            services: services.iter().take(3).map(|s| s.to_string()).collect(),
            days: 60,
            magnify: 50.0,
            seed: 7,
            autoplay: false,
        }
    }
    /// Initial range: 30, 60 or 90 days.
    pub fn days(mut self, d: usize) -> Self {
        self.days = d;
        self
    }
    /// Fisheye swell of hovered bars, 0..100.
    pub fn magnify(mut self, m: f32) -> Self {
        self.magnify = m.clamp(0.0, 100.0);
        self
    }
    /// Seed of the generated history.
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    /// Scrub the bars and switch ranges on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// (status 0..2, uptime %) for service `s`, `days_ago`.
fn day_status(seed: u64, s: usize, days_ago: usize) -> (u8, f32) {
    let h = hash01(seed.wrapping_mul(1000).wrapping_add((s * 977 + days_ago * 31) as u64));
    if h > 0.975 {
        (2, 97.2 + h * 1.5)
    } else if h > 0.93 {
        (1, 99.4 + (h - 0.93) * 8.0)
    } else {
        (0, 100.0)
    }
}

impl egui::Widget for UptimeBars {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let rows = self.services.len().max(1);
        let row_h = 86.0;
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width().min(640.0), 48.0 + row_h * rows as f32),
            Sense::click_and_drag(),
        );
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let ranges = [30usize, 60, 90];
        let chip = |i: usize| {
            Rect::from_min_size(pos2(rect.max.x - 150.0 + i as f32 * 50.0, rect.min.y + 4.0), vec2(46.0, 26.0))
        };
        let rx = |i: usize| (chip(i).center().x - rect.min.x) / rect.width();
        let ry = |r: usize| (48.0 + r as f32 * row_h + 44.0) / rect.height();
        let script = [
            drag(0.8, 3.0, (0.05, ry(0)), (0.95, ry(0))),
            tap(3.8, rx(2), 18.0 / rect.height()),
            drag(4.8, 6.6, (0.9, ry(1.min(rows - 1))), (0.2, ry(1.min(rows - 1)))),
            tap(7.4, rx(0), 18.0 / rect.height()),
        ];
        let touch = touch(ui, &response, rect, self.autoplay, &script, 9.5);
        let mut st: UptimeState =
            glowkit_core::state(ui, id.with("uptime"), || UptimeState { range: 1, ..Default::default() });
        if st.range >= 3 {
            st.range = 1;
        }
        if st.grow.is_empty() {
            st.range = ranges.iter().position(|r| *r == self.days).unwrap_or(1);
            st.reveal_t = now;
        }
        if touch.ghost && touch.cycle != st.cycle {
            st.cycle = touch.cycle;
            st.range = 1;
            st.reveal_t = now;
        }
        let n = ranges[st.range];
        st.grow.resize(90 * 3, 1.0);
        st.grow_v.resize(90 * 3, 0.0);
        let bars_x0 = rect.min.x;
        let bars_w = rect.width();
        let gap = if n > 60 { 2.0 } else { 3.0 };
        let bw = (bars_w - gap * (n as f32 - 1.0)) / n as f32;
        let bar_rect = |r: usize, i: usize| {
            let y = rect.min.y + 48.0 + r as f32 * row_h + 28.0;
            Rect::from_min_size(pos2(bars_x0 + i as f32 * (bw + gap), y), vec2(bw, 32.0))
        };
        let g = st.g.update(&touch, now, dt);
        if g.tap {
            if let Some(i) = (0..3).find(|&i| chip(i).contains(st.g.last)) {
                if i != st.range {
                    st.range = i;
                    st.reveal_t = now;
                }
            }
        }
        let n = ranges[st.range];
        let bw = (bars_w - gap * (n as f32 - 1.0)) / n as f32;
        let p = touch.pos;
        st.hover = p.and_then(|p| {
            (0..rows).find_map(|r| {
                let a = bar_rect(r, 0);
                if p.y >= a.min.y - 8.0 && p.y <= a.max.y + 8.0 && p.x >= bars_x0 && p.x <= bars_x0 + bars_w {
                    Some((r, (((p.x - bars_x0) / (bw + gap)) as usize).min(n - 1)))
                } else {
                    None
                }
            })
        });
        let mag = self.magnify / 100.0;
        let mut moving = touch.ghost || st.g.down;
        for r in 0..rows {
            for i in 0..n {
                let k = r * 90 + i;
                let mut tgt = 1.0;
                if let Some((hr, hi)) = st.hover {
                    if hr == r {
                        let d = (i as f32 - hi as f32).abs();
                        tgt += mag * 0.45 * (-d * d / 4.0).exp();
                    }
                }
                // cascade in after a range change
                let delay = (n - 1 - i) as f32 * 0.006 + r as f32 * 0.05;
                if now - st.reveal_t < delay {
                    st.grow[k] = 0.15;
                    st.grow_v[k] = 0.0;
                    moving = true;
                    continue;
                }
                spring_step(&mut st.grow[k], &mut st.grow_v[k], tgt, 420.0, 18.0, dt);
                moving |= !settled(st.grow[k] * 40.0, st.grow_v[k] * 40.0, tgt * 40.0);
            }
            // uptime figure counts toward the new average
            let avg: f32 = (0..n).map(|i| day_status(self.seed, r, n - 1 - i).1).sum::<f32>() / n as f32;
            spring_step(&mut st.shown[r], &mut st.shown_v[r], avg, 90.0, 19.0, dt);
            moving |= (st.shown[r] - avg).abs() > 0.001;
        }
        let tt = if st.hover.is_some() { 1.0 } else { 0.0 };
        spring_step(&mut st.tip, &mut st.tip_v, tt, 400.0, 30.0, dt);
        if let Some((r, i)) = st.hover {
            let x = bar_rect(r, i).center().x - rect.min.x;
            if st.tip < 0.05 {
                st.tip_x = x;
            }
            spring_step(&mut st.tip_x, &mut st.tip_xv, x, 500.0, 36.0, dt);
        }
        moving |= !settled(st.tip, st.tip_v, tt) || st.tip_xv.abs() > 0.5;
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("uptime"), st);
        if moving {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(60.0));
        let (green, amber, red) = if t.dark {
            (hexa(0x34, 0xC7, 0x6A), hexa(0xF5, 0xA5, 0x24), hexa(0xEF, 0x44, 0x44))
        } else {
            (hexa(0x4A, 0xC2, 0x6B), hexa(0xF2, 0x9D, 0x0B), hexa(0xDC, 0x26, 0x26))
        };
        text_bold(
            &painter,
            pos2(rect.min.x, rect.min.y + 17.0),
            Align2::LEFT_CENTER,
            "System status",
            15.0,
            t.foreground,
        );
        for (i, days) in ranges.iter().enumerate() {
            let r = chip(i);
            if i == s.range {
                painter.rect_filled(r, cr(13.0), recess(&t));
            }
            text(
                &painter,
                r.center(),
                Align2::CENTER_CENTER,
                format!("{days}d"),
                12.0,
                if i == s.range { t.foreground } else { t.muted_foreground },
            );
        }
        for r in 0..rows {
            let y0 = rect.min.y + 48.0 + r as f32 * row_h;
            text(&painter, pos2(rect.min.x, y0 + 12.0), Align2::LEFT_CENTER, &self.services[r], 13.5, t.foreground);
            let today = day_status(self.seed, r, 0).0;
            let (lbl, col) = match today {
                0 => ("Operational", green),
                1 => ("Degraded", amber),
                _ => ("Outage", red),
            };
            let lr = text(&painter, pos2(rect.max.x, y0 + 12.0), Align2::RIGHT_CENTER, lbl, 12.5, t.muted_foreground);
            painter.circle_filled(pos2(lr.min.x - 9.0, lr.center().y), 3.5, col);
            for i in 0..n {
                let k = r * 90 + i;
                let b = bar_rect(r, i);
                let (stt, _) = day_status(self.seed, r, n - 1 - i);
                let c = [green, amber, red][stt as usize];
                let gsc = s.grow[k].max(0.0);
                let bb = Rect::from_min_max(pos2(b.min.x, b.max.y - b.height() * gsc), b.max);
                let hl = s.hover == Some((r, i));
                painter.rect_filled(bb, cr(bw.min(4.0) / 2.0), if hl { mix(c, t.foreground, 0.15) } else { c });
            }
            let ly = y0 + 74.0;
            text(&painter, pos2(rect.min.x, ly), Align2::LEFT_CENTER, day_label(n - 1), 11.0, t.muted_foreground);
            text(&painter, pos2(rect.max.x, ly), Align2::RIGHT_CENTER, "Today", 11.0, t.muted_foreground);
            let mid = format!("{:.2}% uptime", s.shown[r]);
            let mr = text(&painter, pos2(rect.center().x, ly), Align2::CENTER_CENTER, mid, 11.0, t.muted_foreground);
            let line = Stroke::new(1.0, hairline(&t));
            painter.line_segment([pos2(rect.min.x + 44.0, ly), pos2(mr.min.x - 10.0, ly)], line);
            painter.line_segment([pos2(mr.max.x + 10.0, ly), pos2(rect.max.x - 40.0, ly)], line);
        }
        // tooltip
        if s.tip > 0.01 {
            if let Some((r, i)) = s.hover {
                let (stt, up) = day_status(self.seed, r, n - 1 - i);
                let note = match stt {
                    0 => "No incidents".to_string(),
                    1 => "Elevated latency, 18 min".to_string(),
                    _ => "Partial outage, 42 min".to_string(),
                };
                let w = 168.0;
                let b = bar_rect(r, i);
                let x = (rect.min.x + s.tip_x - w / 2.0).clamp(rect.min.x - 10.0, rect.max.x - w + 10.0);
                let a = s.tip.clamp(0.0, 1.0);
                let tr = Rect::from_min_size(pos2(x, b.min.y - 78.0 + (1.0 - a) * 6.0), vec2(w, 62.0));
                let fill = if t.dark { hexa(0x26, 0x26, 0x2B) } else { Color32::WHITE };
                card(&painter, &t, tr, 10.0, a, with_alpha(fill, a));
                text_bold(
                    &painter,
                    pos2(tr.min.x + 12.0, tr.min.y + 16.0),
                    Align2::LEFT_CENTER,
                    day_label(n - 1 - i),
                    12.5,
                    with_alpha(t.foreground, a),
                );
                text(
                    &painter,
                    pos2(tr.max.x - 12.0, tr.min.y + 16.0),
                    Align2::RIGHT_CENTER,
                    format!("{up:.2}%"),
                    12.0,
                    with_alpha(t.muted_foreground, a),
                );
                let c = [green, amber, red][stt as usize];
                painter.circle_filled(pos2(tr.min.x + 15.0, tr.min.y + 42.0), 3.5, with_alpha(c, a));
                text(
                    &painter,
                    pos2(tr.min.x + 25.0, tr.min.y + 42.0),
                    Align2::LEFT_CENTER,
                    note,
                    12.0,
                    with_alpha(t.muted_foreground, a),
                );
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// ActiveSessions

/// One signed-in device for [`ActiveSessions`].
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceSession {
    /// Browser and OS, e.g. "Firefox on Linux".
    pub name: String,
    /// City and country.
    pub place: String,
    /// IP address.
    pub ip: String,
    /// "Active now" or e.g. "Last active yesterday".
    pub seen: String,
    /// Device kind: 0 laptop, 1 desktop, 2 phone, 3 tablet.
    pub kind: u8,
    /// Optional highlighted badge such as "New location".
    pub badge: Option<String>,
}

impl DeviceSession {
    /// A session row.
    pub fn new(
        name: impl Into<String>,
        place: impl Into<String>,
        ip: impl Into<String>,
        seen: impl Into<String>,
        kind: u8,
    ) -> Self {
        Self { name: name.into(), place: place.into(), ip: ip.into(), seen: seen.into(), kind, badge: None }
    }
    /// A small highlighted badge after the name.
    pub fn badge(mut self, b: impl Into<String>) -> Self {
        self.badge = Some(b.into());
        self
    }
}

fn sample_sessions() -> Vec<DeviceSession> {
    vec![
        DeviceSession::new("Safari on macOS", "Lisbon, Portugal", "198.51.100.188", "Active now", 0),
        DeviceSession::new("Chrome on Windows 11", "Porto, Portugal", "192.0.2.148", "Active now", 1),
        DeviceSession::new("App on Android 15", "Madrid, Spain", "198.51.100.116", "Last active 3 hours ago", 2),
        DeviceSession::new("Firefox on Linux", "Berlin, Germany", "203.0.113.109", "Last active yesterday", 0)
            .badge("New location"),
        DeviceSession::new("Safari on iPadOS", "Lisbon, Portugal", "198.51.100.42", "Last active 2 days ago", 3),
    ]
}

#[derive(Clone, Default)]
struct SessionsState {
    gone: Vec<bool>,
    pending: Vec<Option<f32>>,
    fold: Vec<f32>,
    fold_v: Vec<f32>,
    restore: Option<f32>,
    g: Gesture,
    cycle: u32,
}

/// A settings list of the devices signed in to an account.
///
/// The current device is pinned at the top and cannot be signed out. Every other row has a
/// **Sign out** button: press it and a small spinner spins for a moment (like a real request),
/// then the row folds away, its content sliding aside while the rows below spring up to close
/// the gap. **Sign out other sessions** folds them one after another in a ripple, and
/// **Restore** brings them back unfolding in sequence.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ActiveSessions;
/// ui.add(ActiveSessions::new().delay(0.8));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ActiveSessions {
    sessions: Vec<DeviceSession>,
    delay: f32,
    bounce: f32,
    autoplay: bool,
}

impl Default for ActiveSessions {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveSessions {
    /// A list over a few sample sessions (the first is the current device).
    pub fn new() -> Self {
        Self { sessions: sample_sessions(), delay: 0.8, bounce: 20.0, autoplay: false }
    }
    /// Your sessions; the first one is the current device.
    pub fn sessions(mut self, s: Vec<DeviceSession>) -> Self {
        if !s.is_empty() {
            self.sessions = s;
        }
        self
    }
    /// Seconds each sign-out "request" takes.
    pub fn delay(mut self, d: f32) -> Self {
        self.delay = d.clamp(0.0, 5.0);
        self
    }
    /// Overshoot when rows unfold, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Sign out and restore on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for ActiveSessions {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.sessions.len();
        let row_h = 64.0;
        let w = ui.available_width().min(600.0);
        let head_h = 64.0;
        let (rect, response) =
            ui.allocate_exact_size(vec2(w, head_h + row_h * n as f32 + 44.0), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let mut st: SessionsState = glowkit_core::state(ui, id.with("sessions"), SessionsState::default);
        st.gone.resize(n, false);
        st.pending.resize(n, None);
        st.fold.resize(n, 1.0);
        st.fold_v.resize(n, 0.0);
        let list_top = rect.min.y + head_h;
        let row_y =
            |fold: &[f32], i: usize| list_top + fold[..i].iter().map(|f| f.clamp(0.0, 1.0) * row_h).sum::<f32>();
        let all_btn = Rect::from_min_size(pos2(rect.max.x - 176.0, rect.min.y + 8.0), vec2(176.0, 30.0));
        let total: f32 = st.fold.iter().map(|f| f.clamp(0.0, 1.0) * row_h).sum();
        let restore = Rect::from_center_size(pos2(rect.center().x, list_top + total + 24.0), vec2(170.0, 26.0));
        let ny = |y: f32| (y - rect.min.y) / rect.height();
        let script = [
            tap(1.0, (w - 56.0) / w, ny(list_top + row_h * 2.5)),
            tap(3.2, (all_btn.center().x - rect.min.x) / w, ny(all_btn.center().y)),
            tap(6.4, 0.5, ny(list_top + row_h + 24.0)),
        ];
        let touch = touch(ui, &response, rect, self.autoplay, &script, 9.0);
        if touch.ghost && touch.cycle != st.cycle {
            st.cycle = touch.cycle;
            for i in 0..n {
                st.gone[i] = false;
                st.pending[i] = None;
            }
        }
        let g = st.g.update(&touch, now, dt);
        if g.tap {
            let p = st.g.last;
            if all_btn.contains(p) {
                let mut k = 0;
                for i in 1..n {
                    if !st.gone[i] && st.pending[i].is_none() {
                        st.pending[i] = Some(now + k as f32 * 0.18);
                        k += 1;
                    }
                }
            } else if st.gone.iter().skip(1).all(|g| *g) && restore.expand(10.0).contains(p) {
                st.restore = Some(now);
            } else {
                for i in 1..n {
                    let y = row_y(&st.fold, i);
                    let br = Rect::from_min_size(pos2(rect.max.x - 92.0, y + 14.0), vec2(80.0, 36.0));
                    if br.contains(p) && !st.gone[i] && st.pending[i].is_none() {
                        st.pending[i] = Some(now);
                    }
                }
            }
        }
        let mut moving = st.g.down || touch.ghost;
        if let Some(t0) = st.restore {
            for i in 1..n {
                if now - t0 > 0.09 * (i - 1) as f32 && st.gone[i] {
                    st.gone[i] = false;
                    st.pending[i] = None;
                }
            }
            if st.gone.iter().all(|g| !g) {
                st.restore = None;
            }
            moving = true;
        }
        for i in 0..n {
            if let Some(t0) = st.pending[i] {
                if now - t0 > self.delay {
                    st.gone[i] = true;
                    st.pending[i] = None;
                }
                moving = true;
            }
            let tgt = if st.gone[i] { 0.0 } else { 1.0 };
            spring_step(
                &mut st.fold[i],
                &mut st.fold_v[i],
                tgt,
                260.0,
                damping_for(if tgt > 0.5 { self.bounce } else { 0.0 }, 260.0),
                dt,
            );
            moving |= !settled(st.fold[i] * 60.0, st.fold_v[i] * 60.0, tgt * 60.0);
        }
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("sessions"), st);
        if moving {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(12.0));
        text_bold(
            &painter,
            pos2(rect.min.x + 2.0, rect.min.y + 14.0),
            Align2::LEFT_CENTER,
            "Active sessions",
            14.5,
            t.foreground,
        );
        text(
            &painter,
            pos2(rect.min.x + 2.0, rect.min.y + 34.0),
            Align2::LEFT_CENTER,
            "Devices signed in to your account.",
            12.0,
            t.muted_foreground,
        );
        let others = (1..n).filter(|&i| !s.gone[i]).count();
        let btn_a = if others > 0 { 1.0 } else { 0.4 };
        painter.rect_filled(all_btn, cr(15.0), surface(&t));
        painter.rect_stroke(
            all_btn,
            cr(15.0),
            Stroke::new(1.0, with_alpha(hairline(&t), btn_a)),
            egui::StrokeKind::Inside,
        );
        text(
            &painter,
            all_btn.center(),
            Align2::CENTER_CENTER,
            "Sign out other sessions",
            12.5,
            with_alpha(t.foreground, btn_a),
        );
        let total: f32 = s.fold.iter().map(|f| f.clamp(0.0, 1.0) * row_h).sum();
        let list = Rect::from_min_size(pos2(rect.min.x, list_top), vec2(w, total));
        card(&painter, &t, list, 14.0, 0.3, surface(&t));
        painter.rect_stroke(list, cr(14.0), Stroke::new(1.0, hairline(&t)), egui::StrokeKind::Inside);
        let clip = painter.with_clip_rect(list);
        let green = if t.dark { hexa(0x34, 0xD3, 0x99) } else { hexa(0x16, 0x8A, 0x4A) };
        for (i, sess) in self.sessions.iter().enumerate() {
            let f = s.fold[i].clamp(0.0, 1.0);
            let h = f * row_h;
            if h < 0.5 {
                continue;
            }
            let y = row_y(&s.fold, i);
            let row = Rect::from_min_size(pos2(list.min.x, y), vec2(w, h));
            let rc = clip.with_clip_rect(row.intersect(list));
            let a = f.powf(1.5);
            let dx = -(1.0 - f) * 30.0;
            let cy = y + h / 2.0;
            let ib = Rect::from_center_size(pos2(row.min.x + 34.0 + dx, cy), vec2(32.0, 32.0));
            rc.rect_filled(ib, cr(8.0), with_alpha(recess(&t), a));
            let icon = match sess.kind {
                0 => Glyph::Laptop,
                2 => Glyph::Phone,
                3 => Glyph::Tablet,
                _ => Glyph::Laptop,
            };
            glyph(&rc, icon, ib.center(), 15.0, with_alpha(t.muted_foreground, a));
            let x = row.min.x + 62.0 + dx;
            let nr = text(&rc, pos2(x, cy - 10.0), Align2::LEFT_CENTER, &sess.name, 13.5, with_alpha(t.foreground, a));
            let badge = if i == 0 {
                Some(("This device".to_string(), recess(&t), t.muted_foreground))
            } else {
                sess.badge.clone().map(|b| (b, hexa(0xFE, 0xF3, 0xC7), hexa(0x92, 0x40, 0x0E)))
            };
            if let Some((b, bg, fg)) = badge {
                let bw = text_width(&rc, &b, 10.5) + 12.0;
                let br = Rect::from_min_size(pos2(nr.max.x + 8.0, cy - 19.0), vec2(bw, 18.0));
                rc.rect_filled(
                    br,
                    cr(5.0),
                    with_alpha(if t.dark && i != 0 { mix(bg, Color32::BLACK, 0.7) } else { bg }, a),
                );
                text(
                    &rc,
                    br.center(),
                    Align2::CENTER_CENTER,
                    b,
                    10.5,
                    with_alpha(if t.dark && i != 0 { hexa(0xFC, 0xD3, 0x4D) } else { fg }, a),
                );
            }
            let pr = text(
                &rc,
                pos2(x, cy + 10.0),
                Align2::LEFT_CENTER,
                &sess.place,
                11.5,
                with_alpha(t.muted_foreground, a),
            );
            let ipr = rc.text(
                pos2(pr.max.x + 14.0, cy + 10.0),
                Align2::LEFT_CENTER,
                &sess.ip,
                FontId::monospace(10.5),
                with_alpha(t.muted_foreground, a),
            );
            rc.circle_filled(pos2(pr.max.x + 7.0, cy + 10.0), 1.2, with_alpha(t.muted_foreground, a));
            if ipr.max.x + 120.0 < row.max.x - 90.0 {
                let active = sess.seen.starts_with("Active");
                let sx = ipr.max.x + 12.0;
                if active {
                    rc.circle_filled(pos2(sx + 3.0, cy + 10.0), 3.0, with_alpha(green, a));
                }
                text(
                    &rc,
                    pos2(sx + if active { 10.0 } else { 0.0 }, cy + 10.0),
                    Align2::LEFT_CENTER,
                    &sess.seen,
                    11.5,
                    with_alpha(if active { green } else { t.muted_foreground }, a),
                );
            }
            if i > 0 {
                let br = Rect::from_min_size(pos2(row.max.x - 92.0, cy - 15.0), vec2(80.0, 30.0));
                if let Some(t0) = s.pending[i] {
                    if now >= t0 {
                        let c = br.center();
                        let a0 = now * 8.0;
                        glowkit_core::paint::arc(&rc, c, 7.0, a0, a0 + 4.2, 1.8, with_alpha(t.muted_foreground, a));
                    } else {
                        text(&rc, br.center(), Align2::CENTER_CENTER, "Sign out", 12.5, with_alpha(t.foreground, a));
                    }
                } else {
                    text(&rc, br.center(), Align2::CENTER_CENTER, "Sign out", 12.5, with_alpha(t.foreground, a));
                }
            }
            if i + 1 < n {
                rc.line_segment(
                    [pos2(row.min.x, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)],
                    Stroke::new(1.0, hairline(&t)),
                );
            }
        }
        if (1..n).all(|i| s.gone[i]) && s.fold.iter().skip(1).all(|f| *f < 0.05) {
            text(
                &painter,
                pos2(rect.center().x, list.max.y + 24.0),
                Align2::CENTER_CENTER,
                "↺  Restore demo sessions",
                12.5,
                system_blue(&t),
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
