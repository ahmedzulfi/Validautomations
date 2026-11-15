//! Under the hood: [`LogStream`] (live tailing logs with level filters) and [`SandboxStatus`]
//! (the container an agent runs in).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::PI;

/// Severity of a [`LogLine`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum LogLevel {
    /// Verbose.
    Debug,
    /// Normal.
    #[default]
    Info,
    /// Something odd.
    Warn,
    /// Something broke.
    Error,
}

impl LogLevel {
    const ALL: [LogLevel; 4] = [LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
    fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
    fn color(self, t: &Theme) -> Color32 {
        match self {
            LogLevel::Debug => t.muted_foreground,
            LogLevel::Info => t.info,
            LogLevel::Warn => t.warning,
            LogLevel::Error => t.destructive,
        }
    }
}

/// One line in a [`LogStream`].
#[derive(Clone, Debug, Default)]
pub struct LogLine {
    /// Timestamp in seconds since the session started.
    pub at: f32,
    /// Severity.
    pub level: LogLevel,
    /// Emitting agent or tool.
    pub source: String,
    /// The message.
    pub msg: String,
}

impl LogLine {
    /// A line at `at` seconds.
    pub fn new(at: f32, level: LogLevel, source: impl Into<String>, msg: impl Into<String>) -> Self {
        Self { at, level, source: source.into(), msg: msg.into() }
    }
}

#[derive(Clone)]
struct LogState {
    hidden: [bool; 4],
    follow: bool,
    offset: f32,
    seen: usize,
    arrivals: Vec<(usize, f32)>,
}

/// Live, tailing logs. A header with a live dot, level filter chips (DEBUG / INFO / WARN /
/// ERROR) that show rolling counts and toggle each level, and a Follow switch. Lines show a
/// timestamp, a level tag, the source and the message; new lines slide in at the bottom with a
/// brief highlight while following. Scroll up to read back: following pauses and a "Jump to
/// latest" pill appears with the number of new lines.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{LogLevel, LogLine, LogStream};
/// let lines = vec![
///     LogLine::new(0.4, LogLevel::Info, "planner", "Plan has 6 steps"),
///     LogLine::new(2.1, LogLevel::Warn, "shell", "npm WARN deprecated glob@7"),
/// ];
/// ui.add(LogStream::new(&lines).height(280.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct LogStream<'a> {
    lines: &'a [LogLine],
    width: f32,
    height: f32,
    title: String,
}

impl<'a> LogStream<'a> {
    /// A stream over `lines` (oldest first).
    pub fn new(lines: &'a [LogLine]) -> Self {
        Self { lines, width: 820.0, height: 300.0, title: "Logs".into() }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(360.0);
        self
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(140.0);
        self
    }
    /// Header title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
}

impl egui::Widget for LogStream<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("log_stream");
        let mut st: LogState = glowkit_core::state(ui, id.with("st"), || LogState {
            hidden: [true, false, false, false],
            follow: true,
            offset: 0.0,
            seen: 0,
            arrivals: Vec::new(),
        });
        let (rect, resp) = alloc_centered(ui, self.width, self.height, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 14.0;
        let header = 46.0;
        let hy = rect.min.y + header / 2.0;
        icon(&p, Icon::Terminal, pos2(rect.min.x + pad + 7.0, hy), 14.0, dim(&t));
        let tr = text(&p, pos2(rect.min.x + pad + 22.0, hy), Align2::LEFT_CENTER, &self.title, font(13.5), t.foreground, 120.0);
        let k = (now * 1.2).fract();
        let dc = pos2(tr.max.x + 12.0, hy);
        if st.follow {
            p.circle_filled(dc, 3.0 + 5.0 * k, with_alpha(t.success, 0.3 * (1.0 - k)));
        }
        p.circle_filled(dc, 3.0, if st.follow { t.success } else { t.muted_foreground });
        // level chips
        let mut x = dc.x + 16.0;
        for (li, lv) in LogLevel::ALL.iter().enumerate() {
            let n = self.lines.iter().filter(|l| l.level == *lv).count();
            let label = format!("{} {}", lv.label(), n);
            let w = text_w(&p, &label, mono(10.5)) + 18.0;
            let r = Rect::from_min_size(pos2(x, hy - 11.0), vec2(w, 22.0));
            let cresp = ui.interact(r, id.with(("lv", li)), Sense::click());
            if cresp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if cresp.clicked() {
                st.hidden[li] = !st.hidden[li];
            }
            let on = !st.hidden[li];
            let c = lv.color(&t);
            let fill = if on { tint(&t, c) } else { Color32::TRANSPARENT };
            p.rect(r, cr(6.0), fill, Stroke::new(1.0, if on { with_alpha(c, 0.3) } else { hair(&t) }), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, &label, mono(10.5), if on { ink(&t, c) } else { faint(&t) });
            x = r.max.x + 6.0;
        }
        // follow switch
        let sw = Rect::from_min_size(pos2(rect.max.x - pad - 30.0, hy - 8.0), vec2(30.0, 16.0));
        let mut follow = st.follow;
        if switch(ui, sw, id.with("follow"), &mut follow, t.success).clicked() && follow {
            st.offset = 0.0;
        }
        st.follow = follow;
        text(&p, pos2(sw.min.x - 8.0, hy), Align2::RIGHT_CENTER, "Follow", font(11.5), dim(&t), 60.0);
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        // body
        let body = Rect::from_min_max(pos2(rect.min.x + 1.0, rect.min.y + header + 1.0), pos2(rect.max.x - 1.0, rect.max.y - 1.0));
        let lh = 21.0;
        let visible: Vec<usize> = (0..self.lines.len()).filter(|i| !st.hidden[LogLevel::ALL.iter().position(|l| *l == self.lines[*i].level).unwrap_or(0)]).collect();
        // arrivals
        if self.lines.len() > st.seen {
            for i in st.seen..self.lines.len() {
                st.arrivals.push((i, now));
            }
            st.seen = self.lines.len();
        } else if self.lines.len() < st.seen {
            st.seen = self.lines.len();
            st.arrivals.clear();
        }
        st.arrivals.retain(|(_, at)| now - at < 1.5);
        // scrolling
        let bresp = ui.interact(body, id.with("body"), Sense::hover());
        if bresp.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if dy != 0.0 {
                st.offset = (st.offset + dy).max(0.0);
                st.follow = st.offset < 1.0;
            }
        }
        let content_h = visible.len() as f32 * lh + 10.0;
        st.offset = st.offset.min((content_h - body.height()).max(0.0));
        if st.follow {
            st.offset = 0.0;
        }
        let off = glowkit_core::anim::follow(ui, id.with("off"), st.offset, 14.0);
        let pc = p.with_clip_rect(body.intersect(p.clip_rect()));
        let bottom = body.max.y - 6.0 + off;
        let mut new_below = 0;
        for (row, &i) in visible.iter().rev().enumerate() {
            let l = &self.lines[i];
            let arrival = st.arrivals.iter().find(|(k, _)| *k == i).map(|(_, at)| now - at);
            let slide = arrival.map(|a| (1.0 - (a / 0.35).clamp(0.0, 1.0)).powi(2)).unwrap_or(0.0);
            let y = bottom - (row as f32 + 1.0) * lh + slide * lh * 0.6;
            if y > body.max.y {
                new_below += 1;
                continue;
            }
            if y + lh < body.min.y {
                break;
            }
            let lr = Rect::from_min_size(pos2(body.min.x, y), vec2(body.width(), lh));
            if let Some(a) = arrival {
                let f = (1.0 - a / 1.5).clamp(0.0, 1.0);
                pc.rect_filled(lr, cr(0.0), with_alpha(t.accent, 0.08 * f));
            } else if l.level == LogLevel::Error {
                pc.rect_filled(lr, cr(0.0), with_alpha(t.destructive, if t.dark { 0.08 } else { 0.045 }));
            }
            let pa = faded(&pc, 1.0 - slide);
            let cy = lr.center().y;
            let ts = format!("{:02}:{:02}.{:01}", (l.at / 60.0) as u32, l.at as u32 % 60, ((l.at.fract()) * 10.0) as u32);
            pa.text(pos2(lr.min.x + pad, cy), Align2::LEFT_CENTER, ts, mono(10.5), faint(&t));
            let c = l.level.color(&t);
            pa.text(pos2(lr.min.x + pad + 66.0, cy), Align2::LEFT_CENTER, l.level.label(), mono(10.5), ink(&t, c));
            let sr = text(&pa, pos2(lr.min.x + pad + 116.0, cy), Align2::LEFT_CENTER, &l.source, mono(10.5), mix(dim(&t), t.foreground, 0.3), 90.0);
            let mc = if l.level == LogLevel::Error { ink(&t, t.destructive) } else if l.level == LogLevel::Debug { dim(&t) } else { t.foreground };
            let mx = (lr.min.x + pad + 214.0).max(sr.max.x + 10.0);
            text(&pa, pos2(mx, cy), Align2::LEFT_CENTER, &l.msg, mono(11.0), mc, lr.max.x - mx - pad);
        }
        // fade the top edge
        let fade = Rect::from_min_size(body.min, vec2(body.width(), 18.0));
        grad_rect(&pc, fade, surface(&t), Color32::TRANSPARENT, true);
        // jump pill
        if !st.follow {
            let label = if new_below > 0 { format!("{new_below} newer · Jump to latest") } else { "Jump to latest".into() };
            let w = button_w(&p, &label, true);
            let b = Rect::from_center_size(pos2(body.center().x, body.max.y - 22.0), vec2(w, 28.0));
            paint::soft_shadow(&p, b, 14.0, 10.0, vec2(0.0, 2.0), Color32::from_black_alpha(if t.dark { 80 } else { 24 }));
            if button(ui, b, id.with("jump"), &label, Some(Icon::ArrowDown), Btn::Primary).clicked() {
                st.follow = true;
                st.offset = 0.0;
            }
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        if (off - 0.0).abs() > 0.01 || !self.lines.is_empty() {
            glowkit_core::animate(ui);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// SandboxStatus

/// The container or VM an agent works in, as a card: an icon tile with the sandbox name, image
/// and state, a ticking uptime, three radial gauges (CPU, memory, disk) whose arcs glide and turn
/// amber / red past 75% / 90%, a CPU history sparkline, network in / out, forwarded ports as
/// chips, and Terminal / Restart / Stop actions.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{RunState, SandboxStatus};
/// ui.add(SandboxStatus::new("sbx-7f3a2c").image("ubuntu-24.04 · python 3.12").state(RunState::Running)
///     .cpu(0.42).memory(2.8, 8.0).disk(11.0, 40.0).uptime(1834.0).port(3000, "preview"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SandboxStatus {
    name: String,
    image: String,
    state: RunState,
    cpu: f32,
    mem: (f32, f32),
    disk: (f32, f32),
    uptime: f32,
    net: (f32, f32),
    history: Vec<f32>,
    ports: Vec<(u16, String)>,
    width: f32,
}

impl SandboxStatus {
    /// A sandbox called `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            image: String::new(),
            state: RunState::Running,
            cpu: 0.0,
            mem: (0.0, 1.0),
            disk: (0.0, 1.0),
            uptime: 0.0,
            net: (0.0, 0.0),
            history: Vec::new(),
            ports: Vec::new(),
            width: 520.0,
        }
    }
    /// Image / runtime description.
    pub fn image(mut self, s: impl Into<String>) -> Self {
        self.image = s.into();
        self
    }
    /// Sandbox state.
    pub fn state(mut self, s: RunState) -> Self {
        self.state = s;
        self
    }
    /// CPU load 0..1.
    pub fn cpu(mut self, f: f32) -> Self {
        self.cpu = f.clamp(0.0, 1.0);
        self
    }
    /// Memory used and total in GB.
    pub fn memory(mut self, used: f32, total: f32) -> Self {
        self.mem = (used, total.max(0.01));
        self
    }
    /// Disk used and total in GB.
    pub fn disk(mut self, used: f32, total: f32) -> Self {
        self.disk = (used, total.max(0.01));
        self
    }
    /// Uptime in seconds.
    pub fn uptime(mut self, s: f32) -> Self {
        self.uptime = s;
        self
    }
    /// Network in / out in MB/s.
    pub fn network(mut self, rx: f32, tx: f32) -> Self {
        self.net = (rx, tx);
        self
    }
    /// CPU history samples 0..1 (oldest first).
    pub fn cpu_history(mut self, h: Vec<f32>) -> Self {
        self.history = h;
        self
    }
    /// Add a forwarded port.
    pub fn port(mut self, port: u16, label: impl Into<String>) -> Self {
        self.ports.push((port, label.into()));
        self
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(340.0);
        self
    }
}

fn gauge(p: &egui::Painter, t: &Theme, c: egui::Pos2, r: f32, frac: f32, label: &str, value: &str) {
    let a0 = PI * 0.75;
    let sweep = PI * 1.5;
    paint::arc(p, c, r, a0, a0 + sweep, 6.0, track(t));
    let col = if frac > 0.9 { t.destructive } else if frac > 0.75 { t.warning } else { t.foreground };
    let col = if t.dark && col == t.foreground { mix(t.foreground, t.info, 0.4) } else { col };
    if frac > 0.005 {
        paint::arc(p, c, r, a0, a0 + sweep * frac.clamp(0.0, 1.0), 6.0, col);
    }
    p.text(c - vec2(0.0, 3.0), Align2::CENTER_CENTER, format!("{:.0}%", frac * 100.0), mono(14.0), t.foreground);
    p.text(c + vec2(0.0, 13.0), Align2::CENTER_CENTER, label, font(10.5), faint(t));
    p.text(c + vec2(0.0, r + 6.0), Align2::CENTER_CENTER, value, font(11.0), dim(t));
}

impl egui::Widget for SandboxStatus {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("sandbox");
        let h = 308.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        // header
        let ib = Rect::from_min_size(rect.min + vec2(pad, 16.0), vec2(38.0, 38.0));
        p.rect(ib, cr(10.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
        // a little stacked-box glyph
        for k in 0..3 {
            let r = Rect::from_center_size(ib.center() + vec2(0.0, (k as f32 - 1.0) * 6.0), vec2(18.0, 4.0));
            p.rect_filled(r, cr(1.5), with_alpha(t.foreground, 0.25 + 0.2 * k as f32));
        }
        if self.state == RunState::Running {
            p.circle_filled(pos2(ib.max.x - 4.0, ib.min.y + 4.0), 4.0, surface(&t));
            p.circle_filled(pos2(ib.max.x - 4.0, ib.min.y + 4.0), 2.8, t.success);
        }
        text(&p, pos2(ib.max.x + 12.0, ib.min.y + 11.0), Align2::LEFT_CENTER, &self.name, mono(13.0), t.foreground, 200.0);
        text(&p, pos2(ib.max.x + 12.0, ib.min.y + 28.0), Align2::LEFT_CENTER, &self.image, font(11.5), dim(&t), rect.width() - 220.0);
        let sb = state_badge(&p, &t, pos2(rect.max.x - pad, ib.min.y + 11.0), Align2::RIGHT_CENTER, self.state, now);
        let _ = sb;
        text(&p, pos2(rect.max.x - pad, ib.min.y + 30.0), Align2::RIGHT_CENTER, &format!("up {}", fmt_secs(self.uptime)), mono(11.0), faint(&t), 120.0);
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + 70.0);
        // gauges
        let gy = rect.min.y + 128.0;
        let cpu = glowkit_core::anim::follow(ui, id.with("cpu"), self.cpu, 4.0);
        let mem = glowkit_core::anim::follow(ui, id.with("mem"), self.mem.0 / self.mem.1, 4.0);
        let disk = glowkit_core::anim::follow(ui, id.with("disk"), self.disk.0 / self.disk.1, 4.0);
        let gw = (rect.width() - pad * 2.0) / 3.0;
        let gvals = [
            ("CPU", cpu, format!("{} vCPU", 4)),
            ("Memory", mem, format!("{:.1} / {:.0} GB", self.mem.0, self.mem.1)),
            ("Disk", disk, format!("{:.0} / {:.0} GB", self.disk.0, self.disk.1)),
        ];
        for (k, (label, f, v)) in gvals.iter().enumerate() {
            let c = pos2(rect.min.x + pad + gw * (k as f32 + 0.5), gy);
            gauge(&p, &t, c, 34.0, *f, label, v);
        }
        // cpu history + net
        let hy = rect.min.y + 196.0;
        hline(&p, &t, rect.min.x + pad, rect.max.x - pad, hy);
        let sr = Rect::from_min_max(pos2(rect.min.x + pad, hy + 14.0), pos2(rect.center().x + 40.0, hy + 46.0));
        text(&p, pos2(sr.min.x, sr.min.y), Align2::LEFT_CENTER, "CPU, last 2 min", font(10.5), faint(&t), 120.0);
        let spark = Rect::from_min_max(pos2(sr.min.x, sr.min.y + 10.0), sr.max);
        sparkline(&p, spark, &self.history, if t.dark { t.info } else { t.foreground });
        let nx = rect.center().x + 64.0;
        for (k, (label, v, ic)) in [("in", self.net.0, Icon::ArrowDown), ("out", self.net.1, Icon::ArrowUp)].iter().enumerate() {
            let y = hy + 20.0 + k as f32 * 20.0;
            icon(&p, *ic, pos2(nx + 6.0, y), 11.0, dim(&t));
            text(&p, pos2(nx + 18.0, y), Align2::LEFT_CENTER, label, font(11.0), faint(&t), 30.0);
            text(&p, pos2(rect.max.x - pad, y), Align2::RIGHT_CENTER, &format!("{v:.1} MB/s"), mono(11.0), t.foreground, 90.0);
        }
        // footer: ports + actions
        let fy = rect.max.y - 26.0;
        hline(&p, &t, rect.min.x, rect.max.x, rect.max.y - 52.0);
        let mut x = rect.min.x + pad;
        for (port, label) in &self.ports {
            let r = chip(&p, &t, pos2(x, fy), Align2::LEFT_CENTER, &format!(":{port} {label}"), true);
            p.circle_filled(pos2(r.min.x - 0.0, r.center().y), 0.0, t.success);
            x = r.max.x + 6.0;
        }
        let bw = 30.0;
        let mut bx = rect.max.x - pad;
        for (k, (ic, tip)) in [(Icon::Stop, "Stop"), (Icon::Refresh, "Restart"), (Icon::Terminal, "Terminal")].iter().enumerate() {
            let b = Rect::from_min_size(pos2(bx - bw, fy - 14.0), vec2(bw, 28.0));
            let r = button(ui, b, id.with(("act", k)), "", Some(*ic), Btn::Secondary);
            r.on_hover_text(*tip);
            bx = b.min.x - 6.0;
        }
        if self.state.is_active() {
            glowkit_core::animate(ui);
        }
        resp
    }
}
