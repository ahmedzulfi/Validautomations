//! Session managers: [`AgentSessionList`] (agent sessions across devices) and
//! [`SessionDetailHeader`] (the control bar of one session).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Where an agent session runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Device {
    /// A laptop or desktop.
    #[default]
    Laptop,
    /// A cloud sandbox.
    Cloud,
    /// A phone.
    Phone,
    /// A terminal / CLI.
    Terminal,
}

/// Paint a small device glyph centered at `c`.
fn device_glyph(p: &Painter, d: Device, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(1.3, col);
    match d {
        Device::Laptop => {
            let scr = Rect::from_center_size(c - vec2(0.0, s * 0.08), vec2(s * 0.72, s * 0.48));
            p.rect_stroke(scr, cr(1.5), st, StrokeKind::Inside);
            p.line_segment([pos2(c.x - s * 0.48, scr.max.y + s * 0.12), pos2(c.x + s * 0.48, scr.max.y + s * 0.12)], st);
        }
        Device::Phone => {
            let r = Rect::from_center_size(c, vec2(s * 0.44, s * 0.74));
            p.rect_stroke(r, cr(2.5), st, StrokeKind::Inside);
            p.line_segment([pos2(c.x - s * 0.06, r.max.y - s * 0.12), pos2(c.x + s * 0.06, r.max.y - s * 0.12)], st);
        }
        Device::Cloud => {
            let circles = [(-0.2, 0.06, 0.17), (0.04, -0.06, 0.24), (0.26, 0.07, 0.16)];
            let base = c.y + s * 0.22;
            let pts: Vec<Pos2> = (0..48)
                .map(|i| {
                    let a = i as f32 / 48.0 * TAU;
                    let dir = vec2(a.cos(), a.sin());
                    let mut r: f32 = 0.0;
                    for (x, y, rr) in circles {
                        // distance along `dir` from the center to the far side of this circle
                        let o = vec2(x, y);
                        let b = o.dot(dir);
                        let disc = b * b - (o.length_sq() - rr * rr);
                        if disc >= 0.0 {
                            r = r.max(b + disc.sqrt());
                        }
                    }
                    let q = c + dir * r * s;
                    pos2(q.x, q.y.min(base))
                })
                .collect();
            p.add(egui::Shape::closed_line(pts, st));
        }
        Device::Terminal => {
            icon(p, Icon::Terminal, c, s * 0.85, col);
        }
    }
}

impl Device {
    fn label(self) -> &'static str {
        match self {
            Device::Laptop => "Laptop",
            Device::Cloud => "Cloud",
            Device::Phone => "Phone",
            Device::Terminal => "CLI",
        }
    }
}

/// One agent session in an [`AgentSessionList`].
#[derive(Clone, Debug, Default)]
pub struct AgentSession {
    /// Stable id.
    pub id: u64,
    /// Session title (usually the task).
    pub title: String,
    /// Repository and branch, e.g. `"acme/web · fix/zod"`.
    pub repo: String,
    /// Device kind.
    pub device: Device,
    /// Device name, e.g. `"MacBook Pro"`.
    pub device_name: String,
    /// `Running` = live, `NeedsInput` = waiting on you, `Idle`, `Done` or `Failed`.
    pub state: RunState,
    /// Last message preview.
    pub preview: String,
    /// Unread message count.
    pub unread: u32,
    /// Seconds since the last activity.
    pub age: f32,
}

impl AgentSession {
    /// A session `id` titled `title` in `state`.
    pub fn new(id: u64, title: impl Into<String>, state: RunState) -> Self {
        Self { id, title: title.into(), state, ..Default::default() }
    }
    /// Repository / branch label.
    pub fn repo(mut self, s: impl Into<String>) -> Self {
        self.repo = s.into();
        self
    }
    /// Device kind and name.
    pub fn device(mut self, d: Device, name: impl Into<String>) -> Self {
        self.device = d;
        self.device_name = name.into();
        self
    }
    /// Last message preview.
    pub fn preview(mut self, s: impl Into<String>) -> Self {
        self.preview = s.into();
        self
    }
    /// Unread count and seconds since last activity.
    pub fn activity(mut self, unread: u32, age: f32) -> Self {
        self.unread = unread;
        self.age = age;
        self
    }
}

fn session_rank(s: RunState) -> u8 {
    match s {
        RunState::NeedsInput => 0,
        RunState::Running => 1,
        RunState::Failed => 2,
        _ => 3,
    }
}

/// A session manager for coding agents running on several devices, like a Quill / Forge
/// sessions sidebar. A search field and filter tabs (All, Needs input, Live) with counts sit
/// above sections ("Needs you", "Live", "Recent"); each row has a device tile, the title, repo
/// and branch, a last-message preview, an unread pill and the time. Live sessions show a
/// breathing green dot and shimmering preview, sessions waiting on you an amber "Needs input"
/// badge. Rows glide between sections as their state changes; click to select.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{AgentSession, AgentSessionList, Device, RunState};
/// let sessions = vec![
///     AgentSession::new(1, "Fix the failing deploy", RunState::Running).repo("acme/web · fix/zod").device(Device::Laptop, "MacBook"),
///     AgentSession::new(2, "Seed staging", RunState::NeedsInput).repo("acme/data").device(Device::Cloud, "sandbox-2").activity(2, 120.0),
/// ];
/// let mut selected = Some(1);
/// ui.add(AgentSessionList::new(&sessions, &mut selected));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AgentSessionList<'a> {
    sessions: &'a [AgentSession],
    selected: &'a mut Option<u64>,
    width: f32,
    height: f32,
}

impl<'a> AgentSessionList<'a> {
    /// A list over `sessions`; the selected session id is written to `selected`.
    pub fn new(sessions: &'a [AgentSession], selected: &'a mut Option<u64>) -> Self {
        Self { sessions, selected, width: 400.0, height: 520.0 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(280.0);
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
}

impl egui::Widget for AgentSessionList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("session_list");
        let (rect, resp) = alloc_centered(ui, self.width, self.height, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 16.0);
        let pad = 12.0;
        // search
        let mut query: String = glowkit_core::state(ui, id.with("q"), String::new);
        let sr = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(rect.width() - pad * 2.0, 32.0));
        p.rect(sr, cr(9.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
        icon(&p, Icon::Search, pos2(sr.min.x + 15.0, sr.center().y), 13.0, faint(&t));
        let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.min.x + 28.0, sr.min.y + 7.0), pos2(sr.max.x - 40.0, sr.max.y - 4.0))));
        child.add(egui::TextEdit::singleline(&mut query).frame(egui::Frame::NONE).hint_text("Search sessions").font(font(12.5)).desired_width(sr.width() - 70.0));
        let kb = Rect::from_center_size(pos2(sr.max.x - 20.0, sr.center().y), vec2(24.0, 18.0));
        p.rect(kb, cr(4.0), surface(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
        p.text(kb.center(), Align2::CENTER_CENTER, "⌘K", font(9.5), faint(&t));
        glowkit_core::set_state(ui, id.with("q"), query.clone());
        // tabs
        let tab: usize = glowkit_core::state(ui, id.with("tab"), || 0);
        let counts = [
            self.sessions.len(),
            self.sessions.iter().filter(|s| s.state == RunState::NeedsInput).count(),
            self.sessions.iter().filter(|s| s.state == RunState::Running).count(),
        ];
        let ty = sr.max.y + 22.0;
        let mut x = rect.min.x + pad;
        let mut new_tab = tab;
        for (k, label) in ["All", "Needs input", "Live"].iter().enumerate() {
            let lw = text_w(&p, label, font(12.0)) + 34.0;
            let r = Rect::from_min_size(pos2(x, ty - 13.0), vec2(lw, 26.0));
            let tr = ui.interact(r, id.with(("tab", k)), Sense::click());
            if tr.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if tr.clicked() {
                new_tab = k;
            }
            let on = k == tab;
            if on {
                p.rect_filled(r, cr(8.0), track(&t));
            }
            p.text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, *label, font(12.0), if on { t.foreground } else { dim(&t) });
            let c = if k == 1 && counts[1] > 0 { ink(&t, t.warning) } else { faint(&t) };
            p.text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, counts[k].to_string(), mono(10.5), c);
            x = r.max.x + 4.0;
        }
        if new_tab != tab {
            glowkit_core::set_state(ui, id.with("tab"), new_tab);
        }
        let list_top = ty + 20.0;
        hline(&p, &t, rect.min.x, rect.max.x, list_top);
        // order + sections
        let q = query.to_lowercase();
        let mut items: Vec<&AgentSession> = self
            .sessions
            .iter()
            .filter(|s| match tab {
                1 => s.state == RunState::NeedsInput,
                2 => s.state == RunState::Running,
                _ => true,
            })
            .filter(|s| q.is_empty() || s.title.to_lowercase().contains(&q) || s.repo.to_lowercase().contains(&q))
            .collect();
        items.sort_by(|a, b| session_rank(a.state).cmp(&session_rank(b.state)).then(a.age.total_cmp(&b.age)));
        let row_h = 72.0;
        let sec_h = 28.0;
        let clip = Rect::from_min_max(pos2(rect.min.x, list_top + 1.0), pos2(rect.max.x, rect.max.y - 2.0));
        let pc = p.with_clip_rect(clip.intersect(p.clip_rect()));
        let mut y = list_top + 4.0;
        let mut last_sec: Option<u8> = None;
        let mut clicked: Option<u64> = None;
        for s in items {
            let sec = match session_rank(s.state) {
                0 => 0,
                1 => 1,
                _ => 2,
            };
            if last_sec != Some(sec) {
                let name = ["Needs you", "Live", "Recent"][sec as usize];
                text(&pc, pos2(rect.min.x + pad + 4.0, y + sec_h / 2.0 + 2.0), Align2::LEFT_CENTER, name, font(10.5), faint(&t), 120.0);
                y += sec_h;
                last_sec = Some(sec);
            }
            let rid = id.with(("row", s.id));
            let yy = glowkit_core::anim::spring(ui, rid.with("y"), y, 220.0, 26.0);
            y += row_h;
            let rr = Rect::from_min_size(pos2(rect.min.x + 6.0, yy), vec2(rect.width() - 12.0, row_h - 4.0));
            if rr.min.y > clip.max.y {
                continue;
            }
            let hit = ui.interact(rr.intersect(clip), rid, Sense::click());
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                clicked = Some(s.id);
            }
            let sel = *self.selected == Some(s.id);
            let hk = glowkit_core::anim::follow(ui, rid.with("h"), if hit.hovered() { 1.0 } else { 0.0 }, 14.0);
            if sel {
                pc.rect(rr, cr(11.0), mix(surface(&t), t.foreground, if t.dark { 0.07 } else { 0.045 }), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
            } else if hk > 0.01 {
                pc.rect_filled(rr, cr(11.0), with_alpha(t.foreground, 0.03 * hk));
            }
            // device tile
            let dt = Rect::from_min_size(rr.min + vec2(10.0, 12.0), vec2(34.0, 34.0));
            pc.rect(dt, cr(9.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
            device_glyph(&pc, s.device, dt.center(), 20.0, dim(&t));
            // status dot on the tile
            let dc = pos2(dt.max.x - 2.0, dt.max.y - 2.0);
            let col = s.state.color(&t);
            pc.circle_filled(dc, 5.5, surface(&t));
            if s.state == RunState::Running {
                let k = 0.5 + 0.5 * (now * 2.6 + s.id as f32).sin();
                pc.circle_filled(dc, 3.6 + 2.5 * k, with_alpha(col, 0.25 * (1.0 - k)));
            }
            pc.circle_filled(dc, 3.6, if s.state == RunState::Idle || s.state == RunState::Done { mix(t.muted_foreground, surface(&t), 0.3) } else { col });
            let tx = dt.max.x + 10.0;
            let right = rr.max.x - 10.0;
            let age = text(&pc, pos2(right, rr.min.y + 18.0), Align2::RIGHT_CENTER, &fmt_ago(s.age), font(10.5), faint(&t), 70.0);
            let mut tmax = age.min.x - 8.0;
            if s.state == RunState::NeedsInput {
                let b = badge(&pc, &t, pos2(tmax, rr.min.y + 18.0), Align2::RIGHT_CENTER, "Needs input", t.warning, true);
                tmax = b.min.x - 6.0;
            }
            let title_c = if s.unread > 0 || sel { t.foreground } else { mix(t.foreground, dim(&t), 0.25) };
            text(&pc, pos2(tx, rr.min.y + 18.0), Align2::LEFT_CENTER, &s.title, font(13.0), title_c, tmax - tx);
            let meta = format!("{} · {}", s.repo, s.device_name);
            text(&pc, pos2(tx, rr.min.y + 35.0), Align2::LEFT_CENTER, &meta, font(11.0), faint(&t), right - tx);
            let mut pmax = right;
            if s.unread > 0 {
                let label = s.unread.to_string();
                let w = text_w(&pc, &label, font(10.5)) + 12.0;
                let ur = Rect::from_min_size(pos2(right - w.max(18.0), rr.min.y + 45.0), vec2(w.max(18.0), 18.0));
                pc.rect_filled(ur, cr(9.0), t.accent);
                pc.text(ur.center(), Align2::CENTER_CENTER, label, font(10.5), t.accent_foreground);
                pmax = ur.min.x - 8.0;
            }
            let pg = galley1(&pc, &s.preview, font(11.5), dim(&t), pmax - tx);
            let ppos = pos2(tx, rr.min.y + 54.0 - pg.size().y / 2.0);
            if s.state == RunState::Running {
                let gw = pg.size().x.max(1.0);
                let sweep = ((now * 0.5 + s.id as f32 * 0.3).fract() * 1.6 - 0.3) * gw;
                let base = dim(&t);
                let hi = t.foreground;
                paint::galley_with(&pc, ppos, &pg, |q| {
                    let d = ((q.x - ppos.x) - sweep).abs() / 40.0;
                    mix(base, hi, (1.0 - d).clamp(0.0, 1.0))
                });
            } else {
                pc.galley(ppos, pg, dim(&t));
            }
        }
        if let Some(c) = clicked {
            *self.selected = Some(c);
        }
        // bottom fade
        let fade = Rect::from_min_max(pos2(rect.min.x + 1.0, rect.max.y - 24.0), pos2(rect.max.x - 1.0, rect.max.y - 2.0));
        grad_rect(&p, fade, Color32::TRANSPARENT, surface(&t), true);
        glowkit_core::animate(ui);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// SessionDetailHeader

/// An action from a [`SessionDetailHeader`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionAction {
    /// Pause clicked.
    Pause,
    /// Resume clicked.
    Resume,
    /// The Stop button was held to the end.
    Stop,
    /// The "Needs you" pill was clicked.
    OpenInbox,
}

/// The control bar on top of an agent session: the agent's mark, the session title, repo,
/// branch and device, a state badge with a ticking clock, the current step, a token budget
/// meter, a "Needs you" pill with a count, a model chip, Pause / Resume, and a hold-to-stop
/// kill switch whose ring fills while pressed (let go early to cancel).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{RunState, SessionDetailHeader};
/// let action = SessionDetailHeader::new("Deploy Fixer", "Fix the failing production deploy")
///     .meta("acme/web", "fix/missing-zod", "MacBook Pro")
///     .state(RunState::Running, 142.0).step(3, 5, "Running the test suite")
///     .budget(18_400.0, 50_000.0).needs_you(2).model("Nova 2")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SessionDetailHeader {
    agent: String,
    title: String,
    repo: String,
    branch: String,
    device: String,
    state: RunState,
    elapsed: f32,
    step: (usize, usize, String),
    budget: (f32, f32),
    needs: u32,
    model: String,
    width: f32,
}

impl SessionDetailHeader {
    /// A header for `agent` working on `title`.
    pub fn new(agent: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            title: title.into(),
            repo: String::new(),
            branch: String::new(),
            device: String::new(),
            state: RunState::Running,
            elapsed: 0.0,
            step: (0, 0, String::new()),
            budget: (0.0, 1.0),
            needs: 0,
            model: String::new(),
            width: 900.0,
        }
    }
    /// Repository, branch and device.
    pub fn meta(mut self, repo: impl Into<String>, branch: impl Into<String>, device: impl Into<String>) -> Self {
        self.repo = repo.into();
        self.branch = branch.into();
        self.device = device.into();
        self
    }
    /// Session state and elapsed seconds.
    pub fn state(mut self, s: RunState, elapsed: f32) -> Self {
        self.state = s;
        self.elapsed = elapsed;
        self
    }
    /// Current step, total and its label.
    pub fn step(mut self, i: usize, of: usize, label: impl Into<String>) -> Self {
        self.step = (i, of, label.into());
        self
    }
    /// Tokens used and the token budget.
    pub fn budget(mut self, used: f32, limit: f32) -> Self {
        self.budget = (used, limit.max(1.0));
        self
    }
    /// Number of items waiting on a person.
    pub fn needs_you(mut self, n: u32) -> Self {
        self.needs = n;
        self
    }
    /// Model label.
    pub fn model(mut self, s: impl Into<String>) -> Self {
        self.model = s.into();
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(520.0);
        self
    }

    /// Show it; returns the action taken this frame.
    pub fn show(self, ui: &mut Ui) -> Option<SessionAction> {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let id = ui.next_auto_id().with("session_header");
        let h = 112.0;
        let (rect, _) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 16.0);
        let pad = 16.0;
        let mut out = None;
        // top row
        let ty = rect.min.y + 32.0;
        let init: String = self.agent.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
        let ac = pos2(rect.min.x + pad + 20.0, ty);
        if self.state == RunState::Running {
            let k = (now * 0.9).fract();
            p.rect_stroke(Rect::from_center_size(ac, vec2(40.0 + 10.0 * k, 40.0 + 10.0 * k)), cr(12.0 + 3.0 * k), Stroke::new(1.0, with_alpha(t.info, 0.4 * (1.0 - k))), StrokeKind::Outside);
        }
        avatar(&p, &t, ac, 40.0, &init, t.chart[0]);
        let tx = ac.x + 30.0;
        let tr = text(&p, pos2(tx, ty - 9.0), Align2::LEFT_CENTER, &self.title, font(15.0), t.foreground, rect.width() * 0.42);
        let sb = state_badge(&p, &t, pos2(tr.max.x + 10.0, ty - 9.0), Align2::LEFT_CENTER, self.state, now);
        text(&p, pos2(sb.max.x + 8.0, ty - 9.0), Align2::LEFT_CENTER, &fmt_secs(self.elapsed), mono(11.5), dim(&t), 80.0);
        // meta row
        let mut mx = tx;
        let my = ty + 11.0;
        let r = text(&p, pos2(mx, my), Align2::LEFT_CENTER, &self.agent, font(11.5), dim(&t), 140.0);
        mx = r.max.x + 10.0;
        for (ic, s) in [(Icon::Bookmark, &self.repo), (Icon::Code, &self.branch), (Icon::Globe, &self.device)] {
            if s.is_empty() {
                continue;
            }
            icon(&p, ic, pos2(mx + 6.0, my), 11.0, faint(&t));
            let r = text(&p, pos2(mx + 15.0, my), Align2::LEFT_CENTER, s, if ic == Icon::Code { mono(11.0) } else { font(11.5) }, dim(&t), 160.0);
            mx = r.max.x + 12.0;
        }
        // right controls
        let cy = ty;
        let mut x = rect.max.x - pad;
        // hold to stop
        let sw = 86.0;
        let sr = Rect::from_min_size(pos2(x - sw, cy - 16.0), vec2(sw, 32.0));
        let sresp = ui.interact(sr, id.with("stop"), Sense::click_and_drag());
        if sresp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let holding = sresp.is_pointer_button_down_on() && self.state.is_active();
        let (hold, fired) = glowkit_core::with_state(ui, id.with("hold"), || (0.0f32, false), |(v, f)| {
            if holding {
                *v = (*v + dt / 1.1).min(1.0);
            } else {
                *v = (*v - dt * 3.0).max(0.0);
                *f = false;
            }
            let fire = *v >= 1.0 && !*f;
            if fire {
                *f = true;
            }
            (*v, fire)
        });
        if fired {
            out = Some(SessionAction::Stop);
        }
        p.rect(sr, cr(10.0), mix(tint(&t, t.destructive), with_alpha(t.destructive, 0.2), hold), Stroke::new(1.0, with_alpha(t.destructive, 0.3)), StrokeKind::Inside);
        if hold > 0.0 {
            let fill = Rect::from_min_size(sr.min, vec2(sr.width() * hold, sr.height()));
            p.with_clip_rect(fill).rect_filled(sr, cr(10.0), with_alpha(t.destructive, 0.85));
            glowkit_core::animate(ui);
        }
        let rc = pos2(sr.min.x + 18.0, sr.center().y);
        let fg = if hold > 0.5 { Color32::WHITE } else { ink(&t, t.destructive) };
        p.circle_stroke(rc, 7.0, Stroke::new(1.5, with_alpha(fg, 0.35)));
        if hold > 0.0 {
            paint::arc(&p, rc, 7.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * hold, 1.6, fg);
        }
        p.rect_filled(Rect::from_center_size(rc, vec2(5.0, 5.0)), cr(1.0), fg);
        p.text(pos2(rc.x + 13.0, sr.center().y), Align2::LEFT_CENTER, if holding { "Hold…" } else { "Stop" }, font(12.0), fg);
        if sresp.hovered() && hold == 0.0 {
            let lp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("tip")));
            let tr = Rect::from_min_size(pos2(sr.min.x - 30.0, sr.max.y + 6.0), vec2(116.0, 24.0));
            panel_fill(&lp, &t, tr, 7.0, if t.dark { t.popover } else { t.card }, 1.0);
            lp.text(tr.center(), Align2::CENTER_CENTER, "Hold to stop the run", font(11.0), dim(&t));
        }
        x = sr.min.x - 8.0;
        // pause / resume
        let paused = self.state == RunState::Paused;
        let label = if paused { "Resume" } else { "Pause" };
        let pw = button_w(&p, label, true);
        let pr = Rect::from_min_size(pos2(x - pw, cy - 16.0), vec2(pw, 32.0));
        if button(ui, pr, id.with("pause"), label, Some(if paused { Icon::Play } else { Icon::Pause }), Btn::Secondary).clicked() {
            out = Some(if paused { SessionAction::Resume } else { SessionAction::Pause });
        }
        x = pr.min.x - 8.0;
        if self.needs > 0 {
            let label = "Needs you";
            let w = text_w(&p, label, font(12.0)) + 46.0;
            let nr = Rect::from_min_size(pos2(x - w, cy - 16.0), vec2(w, 32.0));
            let nresp = ui.interact(nr, id.with("needs"), Sense::click());
            if nresp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if nresp.clicked() {
                out = Some(SessionAction::OpenInbox);
            }
            let k = 0.5 + 0.5 * (now * 2.5).sin();
            p.rect(nr, cr(16.0), surface(&t), Stroke::new(1.0, mix(hair(&t), t.warning, 0.4 + 0.4 * k)), StrokeKind::Inside);
            p.text(pos2(nr.min.x + 12.0, nr.center().y), Align2::LEFT_CENTER, label, font(12.0), t.foreground);
            let cb = Rect::from_center_size(pos2(nr.max.x - 16.0, nr.center().y), vec2(20.0, 18.0));
            p.rect_filled(cb, cr(9.0), t.foreground);
            p.text(cb.center(), Align2::CENTER_CENTER, self.needs.to_string(), font(10.5), surface(&t));
            glowkit_core::animate(ui);
        }
        // bottom strip: step, budget, model
        let by = rect.max.y - 24.0;
        hline(&p, &t, rect.min.x, rect.max.x, rect.max.y - 48.0);
        let (si, sn, sl) = &self.step;
        let mut bx = rect.min.x + pad;
        if *sn > 0 {
            for k in 0..*sn {
                let c = pos2(bx + 4.0 + k as f32 * 12.0, by);
                let done = k + 1 < *si;
                let cur = k + 1 == *si;
                if cur {
                    p.circle_filled(c, 4.0, self.state.color(&t));
                    if self.state == RunState::Running {
                        let q = (now * 1.5).fract();
                        p.circle_stroke(c, 4.0 + q * 4.0, Stroke::new(1.0, with_alpha(self.state.color(&t), 1.0 - q)));
                    }
                } else {
                    p.circle_filled(c, 3.0, if done { mix(t.foreground, surface(&t), 0.2) } else { track(&t) });
                }
            }
            bx += *sn as f32 * 12.0 + 8.0;
            let r = text(&p, pos2(bx, by), Align2::LEFT_CENTER, &format!("Step {si} of {sn}"), font(11.5), t.foreground, 100.0);
            text(&p, pos2(r.max.x + 8.0, by), Align2::LEFT_CENTER, sl, font(11.5), dim(&t), rect.width() * 0.3);
        }
        let mut rx = rect.max.x - pad;
        if !self.model.is_empty() {
            let r = chip(&p, &t, pos2(rx, by), Align2::RIGHT_CENTER, &self.model, false);
            rx = r.min.x - 16.0;
        }
        let (used, lim) = self.budget;
        let f = glowkit_core::anim::follow(ui, id.with("bud"), used / lim, 5.0);
        let bw = 120.0;
        let bar = Rect::from_min_size(pos2(rx - bw, by - 2.0), vec2(bw, 4.0));
        let bc = if f > 1.0 { t.destructive } else if f > 0.8 { t.warning } else { t.foreground };
        meter(&p, &t, bar, f, bc);
        text(&p, pos2(bar.min.x - 8.0, by), Align2::RIGHT_CENTER, &format!("{} / {} tokens", fmt_tok(used), fmt_tok(lim)), font(11.0), dim(&t), 160.0);
        if self.state.is_active() {
            glowkit_core::animate(ui);
        }
        let _ = Device::Laptop.label();
        out
    }
}
