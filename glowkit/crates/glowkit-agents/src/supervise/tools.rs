//! What agents can use: [`McpServerList`] (MCP servers with status, tools and latency) and
//! [`SkillsBrowser`] (skills / plugins to install).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_ai::Icon;
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// An MCP server in an [`McpServerList`].
#[derive(Clone, Debug, Default)]
pub struct McpServer {
    /// Display name, e.g. `"GitHub"`.
    pub name: String,
    /// Transport, e.g. `"stdio"` or `"http"`.
    pub transport: String,
    /// Command or URL.
    pub endpoint: String,
    /// Whether it is switched on.
    pub enabled: bool,
    /// Error message when it failed to connect.
    pub error: Option<String>,
    /// Tool names it offers.
    pub tools: Vec<String>,
    /// Recent round-trip times in milliseconds (oldest first).
    pub latency: Vec<f32>,
}

impl McpServer {
    /// An enabled server.
    pub fn new(name: impl Into<String>, transport: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: transport.into(),
            endpoint: endpoint.into(),
            enabled: true,
            ..Default::default()
        }
    }
    /// Its tool names.
    pub fn tools(mut self, tools: &[&str]) -> Self {
        self.tools = tools.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Recent latency samples in ms.
    pub fn latency(mut self, l: Vec<f32>) -> Self {
        self.latency = l;
        self
    }
    /// Switched on or off.
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
    /// Mark it as failing with `msg`.
    pub fn error(mut self, msg: impl Into<String>) -> Self {
        self.error = Some(msg.into());
        self
    }
}

#[derive(Clone, Default)]
struct McpState {
    was_on: Vec<(String, bool, f32)>,
    open: Option<String>,
}

/// The MCP servers an agent can reach. Each row has a lettered tile, the server name with its
/// transport, the command or URL, a status (Connected, Connecting with a spinner, Error with the
/// message, Off), the number of tools, a latency readout with a small bar history, and a
/// switch. Switching a server on plays a short handshake before it turns green. Click a row to
/// unfold its tool list as chips.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{McpServer, McpServerList};
/// let mut servers = vec![
///     McpServer::new("GitHub", "http", "https://api.githubcopilot.com/mcp").tools(&["create_issue", "search_code"]),
///     McpServer::new("Postgres", "stdio", "npx @mcp/postgres").enabled(false),
/// ];
/// ui.add(McpServerList::new(&mut servers));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct McpServerList<'a> {
    servers: &'a mut Vec<McpServer>,
    width: f32,
}

impl<'a> McpServerList<'a> {
    /// A list over `servers` (the switches write `enabled`).
    pub fn new(servers: &'a mut Vec<McpServer>) -> Self {
        Self { servers, width: 720.0 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
}

enum McpStatus {
    Off,
    Connecting(f32),
    Connected,
    Error,
}

impl egui::Widget for McpServerList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("mcp_list");
        let mut st: McpState = glowkit_core::state(ui, id.with("st"), McpState::default);
        // track on transitions
        for s in self.servers.iter() {
            match st.was_on.iter_mut().find(|(n, _, _)| *n == s.name) {
                Some(e) => {
                    if s.enabled && !e.1 {
                        e.2 = now;
                    }
                    e.1 = s.enabled;
                }
                None => st.was_on.push((s.name.clone(), s.enabled, -100.0)),
            }
        }
        let status = |s: &McpServer, st: &McpState| -> McpStatus {
            if !s.enabled {
                return McpStatus::Off;
            }
            let since = st.was_on.iter().find(|(n, _, _)| *n == s.name).map(|e| now - e.2).unwrap_or(100.0);
            if since < 1.4 {
                McpStatus::Connecting(since / 1.4)
            } else if s.error.is_some() {
                McpStatus::Error
            } else {
                McpStatus::Connected
            }
        };
        let row_h = 64.0;
        let mut opens = Vec::new();
        let mut body = 0.0;
        for s in self.servers.iter() {
            let o = glowkit_core::anim::spring(
                ui,
                id.with(("open", &s.name)),
                if st.open.as_deref() == Some(s.name.as_str()) { 1.0 } else { 0.0 },
                300.0,
                28.0,
            )
            .max(0.0);
            let chips_h = 14.0 + (s.tools.len() as f32 / 4.0).ceil() * 28.0;
            body += row_h + chips_h * o;
            opens.push((o, chips_h));
        }
        let header = 60.0;
        let h = header + body + 8.0;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        let connected = self.servers.iter().filter(|s| matches!(status(s, &st), McpStatus::Connected)).count();
        let tools: usize =
            self.servers.iter().filter(|s| matches!(status(s, &st), McpStatus::Connected)).map(|s| s.tools.len()).sum();
        let hy = rect.min.y + header / 2.0;
        text(&p, pos2(rect.min.x + pad, hy - 8.0), Align2::LEFT_CENTER, "MCP servers", font(14.0), t.foreground, 200.0);
        text(
            &p,
            pos2(rect.min.x + pad, hy + 10.0),
            Align2::LEFT_CENTER,
            &format!("{connected} connected · {tools} tools available"),
            font(11.5),
            dim(&t),
            300.0,
        );
        let aw = button_w(&p, "Add server", true);
        let ab = Rect::from_min_size(pos2(rect.max.x - pad - aw, hy - 15.0), vec2(aw, 30.0));
        let _ = button(ui, ab, id.with("add"), "Add server", Some(Icon::Plus), Btn::Secondary);
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        let mut y = rect.min.y + header;
        let mut toggles: Vec<usize> = Vec::new();
        let mut open_click: Option<String> = None;
        for (i, s) in self.servers.iter().enumerate() {
            let (o, chips_h) = opens[i];
            let rr = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), row_h + chips_h * o));
            y = rr.max.y;
            let head = Rect::from_min_size(rr.min, vec2(rr.width() - 70.0, row_h));
            let hit = ui.interact(head, id.with(("row", i)), Sense::click());
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                p.rect_filled(
                    Rect::from_min_size(rr.min, vec2(rr.width(), row_h)),
                    cr(0.0),
                    with_alpha(t.foreground, 0.02),
                );
            }
            if hit.clicked() {
                open_click = Some(s.name.clone());
            }
            let stt = status(s, &st);
            let cy = rr.min.y + row_h / 2.0;
            let off = matches!(stt, McpStatus::Off);
            let pa = faded(&p, if off { 0.55 } else { 1.0 });
            let tile = Rect::from_center_size(pos2(rr.min.x + pad + 18.0, cy), vec2(36.0, 36.0));
            let col = series(&t, i);
            pa.rect_filled(tile, cr(10.0), tint(&t, col));
            pa.text(
                tile.center(),
                Align2::CENTER_CENTER,
                s.name.chars().next().unwrap_or('?').to_string(),
                font(15.0),
                ink(&t, col),
            );
            let tx = tile.max.x + 12.0;
            let nr = text(&pa, pos2(tx, cy - 9.0), Align2::LEFT_CENTER, &s.name, font(13.0), t.foreground, 180.0);
            chip(&pa, &t, pos2(nr.max.x + 8.0, cy - 9.0), Align2::LEFT_CENTER, &s.transport, true);
            let ep_w = rect.width() * 0.36;
            text(&pa, pos2(tx, cy + 10.0), Align2::LEFT_CENTER, &s.endpoint, mono(10.5), faint(&t), ep_w);
            // status column
            let sx = rect.min.x + rect.width() * 0.5;
            match stt {
                McpStatus::Off => {
                    pa.circle_filled(pos2(sx + 4.0, cy - 9.0), 3.5, t.muted_foreground);
                    text(&pa, pos2(sx + 13.0, cy - 9.0), Align2::LEFT_CENTER, "Off", font(12.0), dim(&t), 80.0);
                }
                McpStatus::Connecting(k) => {
                    spinner(&p, pos2(sx + 4.0, cy - 9.0), 5.0, t.info, now, 1.6);
                    text(
                        &p,
                        pos2(sx + 15.0, cy - 9.0),
                        Align2::LEFT_CENTER,
                        "Connecting…",
                        font(12.0),
                        ink(&t, t.info),
                        100.0,
                    );
                    let bar = Rect::from_min_size(pos2(sx, cy + 8.0), vec2(90.0, 3.0));
                    meter(&p, &t, bar, ease(Easing::OutCubic, k), t.info);
                    glowkit_core::animate(ui);
                }
                McpStatus::Connected => {
                    let k = (now * 0.8 + i as f32 * 0.3).fract();
                    p.circle_filled(pos2(sx + 4.0, cy - 9.0), 3.5 + 4.0 * k, with_alpha(t.success, 0.25 * (1.0 - k)));
                    p.circle_filled(pos2(sx + 4.0, cy - 9.0), 3.5, t.success);
                    text(
                        &p,
                        pos2(sx + 13.0, cy - 9.0),
                        Align2::LEFT_CENTER,
                        "Connected",
                        font(12.0),
                        t.foreground,
                        100.0,
                    );
                    text(
                        &p,
                        pos2(sx, cy + 10.0),
                        Align2::LEFT_CENTER,
                        &format!("{} tools", s.tools.len()),
                        font(11.0),
                        dim(&t),
                        80.0,
                    );
                    glowkit_core::animate(ui);
                }
                McpStatus::Error => {
                    p.circle_filled(pos2(sx + 4.0, cy - 9.0), 3.5, t.destructive);
                    text(
                        &p,
                        pos2(sx + 13.0, cy - 9.0),
                        Align2::LEFT_CENTER,
                        "Error",
                        font(12.0),
                        ink(&t, t.destructive),
                        100.0,
                    );
                    text(
                        &p,
                        pos2(sx, cy + 10.0),
                        Align2::LEFT_CENTER,
                        s.error.as_deref().unwrap_or(""),
                        font(11.0),
                        ink(&t, t.destructive),
                        rect.width() * 0.22,
                    );
                }
            }
            // latency
            let lx = rect.min.x + rect.width() * 0.74;
            if matches!(stt, McpStatus::Connected) && !s.latency.is_empty() {
                let last = *s.latency.last().unwrap();
                let lc = if last > 300.0 { ink(&t, t.warning) } else { t.foreground };
                text(&p, pos2(lx, cy - 9.0), Align2::LEFT_CENTER, &format!("{last:.0} ms"), mono(11.5), lc, 70.0);
                let n = s.latency.len().min(12);
                let hi = s.latency.iter().cloned().fold(1.0, f32::max);
                for (k, v) in s.latency[s.latency.len() - n..].iter().enumerate() {
                    let bh = (v / hi * 12.0).max(2.0);
                    let r = Rect::from_min_max(
                        pos2(lx + k as f32 * 5.0, cy + 16.0 - bh),
                        pos2(lx + k as f32 * 5.0 + 3.0, cy + 16.0),
                    );
                    p.rect_filled(
                        r,
                        cr(1.0),
                        if *v > 300.0 { with_alpha(t.warning, 0.8) } else { mix(track(&t), t.foreground, 0.35) },
                    );
                }
            } else {
                text(&p, pos2(lx, cy - 9.0), Align2::LEFT_CENTER, "—", mono(11.5), faint(&t), 40.0);
            }
            // switch
            let sw = Rect::from_center_size(pos2(rr.max.x - pad - 18.0, cy), vec2(36.0, 20.0));
            let mut on = s.enabled;
            if switch(ui, sw, id.with(("sw", i)), &mut on, t.success).clicked() {
                toggles.push(i);
            }
            // chips
            if o > 0.01 {
                let pc = faded(&p.with_clip_rect(rr.intersect(p.clip_rect())), o.min(1.0));
                let cw = (rect.width() - pad * 2.0 - tx + rect.min.x + pad - 18.0) / 4.0;
                for (k, tool) in s.tools.iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(tx + (k % 4) as f32 * (cw + 6.0), rr.min.y + row_h - 4.0 + (k / 4) as f32 * 28.0),
                        vec2(cw, 22.0),
                    );
                    pc.rect(r, cr(6.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
                    icon(&pc, Icon::Gear, pos2(r.min.x + 11.0, r.center().y), 10.0, faint(&t));
                    text(
                        &pc,
                        pos2(r.min.x + 21.0, r.center().y),
                        Align2::LEFT_CENTER,
                        tool,
                        mono(10.5),
                        dim(&t),
                        r.width() - 26.0,
                    );
                }
            }
            if i + 1 < self.servers.len() {
                hline(&p, &t, rr.min.x + pad, rr.max.x - pad, rr.max.y);
            }
        }
        for i in toggles {
            self.servers[i].enabled = !self.servers[i].enabled;
        }
        if let Some(n) = open_click {
            st.open = if st.open.as_deref() == Some(n.as_str()) { None } else { Some(n) };
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// SkillsBrowser

/// A skill or plugin in a [`SkillsBrowser`].
#[derive(Clone, Debug)]
pub struct Skill {
    /// Name.
    pub name: String,
    /// Author or source.
    pub author: String,
    /// One or two lines of description.
    pub desc: String,
    /// Category used by the tabs.
    pub category: String,
    /// Icon on its tile.
    pub icon: Icon,
    /// Installed or not.
    pub installed: bool,
    /// Install count.
    pub installs: u32,
}

impl Skill {
    /// A skill `name` by `author` in `category`.
    pub fn new(name: impl Into<String>, author: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            author: author.into(),
            desc: String::new(),
            category: category.into(),
            icon: Icon::Sparkle,
            installed: false,
            installs: 0,
        }
    }
    /// Description.
    pub fn desc(mut self, s: impl Into<String>) -> Self {
        self.desc = s.into();
        self
    }
    /// Tile icon.
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = i;
        self
    }
    /// Already installed.
    pub fn installed(mut self, on: bool) -> Self {
        self.installed = on;
        self
    }
    /// Install count.
    pub fn installs(mut self, n: u32) -> Self {
        self.installs = n;
        self
    }
}

#[derive(Clone, Default)]
struct SkillState {
    query: String,
    tab: usize,
    installing: Vec<(String, f32)>,
    known: Vec<(String, bool)>,
}

/// A browser for agent skills and plugins: a search field, category tabs, and a grid of cards
/// with an icon tile, name, author, a two-line description and install count. Install fills a
/// progress bar across the button and morphs it into "Installed" with a check; hovering an
/// installed one offers Remove. Cards lift on hover and the grid reflows with the width.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{Skill, SkillsBrowser};
/// let mut skills = vec![
///     Skill::new("PDF tools", "Lumen Labs", "Documents").desc("Read, fill and merge PDFs").installs(48_200),
///     Skill::new("Postgres", "Community", "Data").desc("Query and migrate databases").installed(true),
/// ];
/// ui.add(SkillsBrowser::new(&mut skills));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SkillsBrowser<'a> {
    skills: &'a mut Vec<Skill>,
    width: f32,
    card_w: f32,
}

impl<'a> SkillsBrowser<'a> {
    /// A browser over `skills` (Install writes `installed`).
    pub fn new(skills: &'a mut Vec<Skill>) -> Self {
        Self { skills, width: 880.0, card_w: 260.0 }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Preferred card width in points.
    pub fn card_width(mut self, w: f32) -> Self {
        self.card_w = w.clamp(200.0, 360.0);
        self
    }
}

impl egui::Widget for SkillsBrowser<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let now = glowkit_core::time(ui);
        let id = ui.next_auto_id().with("skills");
        let mut st: SkillState = glowkit_core::state(ui, id.with("st"), SkillState::default);
        let mut cats: Vec<String> = vec!["All".into()];
        for s in self.skills.iter() {
            if !cats.contains(&s.category) {
                cats.push(s.category.clone());
            }
        }
        cats.push("Installed".into());
        let q = st.query.to_lowercase();
        let shown: Vec<usize> = (0..self.skills.len())
            .filter(|i| {
                let s = &self.skills[*i];
                let cat_ok = match st.tab {
                    0 => true,
                    k if k == cats.len() - 1 => s.installed,
                    k => cats.get(k).is_some_and(|c| *c == s.category),
                };
                cat_ok && (q.is_empty() || s.name.to_lowercase().contains(&q) || s.desc.to_lowercase().contains(&q))
            })
            .collect();
        let avail = ui.available_width().min(self.width);
        let gap = 12.0;
        let cols = (((avail + gap) / (self.card_w + gap)).floor() as usize).max(1);
        let cw = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = 150.0;
        let rows = shown.len().div_ceil(cols).max(1);
        let header = 92.0;
        let target = header + rows as f32 * (ch + gap);
        let h = glowkit_core::anim::spring(ui, id.with("h"), target, 260.0, 30.0);
        let (rect, resp) = alloc_centered(ui, avail, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        // search
        let sr = Rect::from_min_size(rect.min, vec2(rect.width().min(360.0), 36.0));
        p.rect(sr, cr(10.0), surface(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
        icon(&p, Icon::Search, pos2(sr.min.x + 16.0, sr.center().y), 13.0, faint(&t));
        let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(
            pos2(sr.min.x + 30.0, sr.min.y + 9.0),
            pos2(sr.max.x - 10.0, sr.max.y - 4.0),
        )));
        child.add(
            egui::TextEdit::singleline(&mut st.query)
                .frame(egui::Frame::NONE)
                .hint_text("Search skills and plugins")
                .font(font(12.5))
                .desired_width(sr.width() - 44.0),
        );
        let inst = self.skills.iter().filter(|s| s.installed).count();
        text(
            &p,
            pos2(rect.max.x, sr.center().y),
            Align2::RIGHT_CENTER,
            &format!("{inst} installed · {} available", self.skills.len()),
            font(12.0),
            dim(&t),
            260.0,
        );
        // tabs
        let ty = sr.max.y + 26.0;
        let mut x = rect.min.x;
        for (k, c) in cats.iter().enumerate() {
            let w = text_w(&p, c, font(12.0)) + 22.0;
            let r = Rect::from_min_size(pos2(x, ty - 14.0), vec2(w, 28.0));
            let tr = ui.interact(r, id.with(("tab", k)), Sense::click());
            if tr.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if tr.clicked() {
                st.tab = k;
            }
            let on = st.tab == k;
            if on {
                p.rect_filled(r, cr(14.0), t.foreground);
            } else if tr.hovered() {
                p.rect_filled(r, cr(14.0), track(&t));
            }
            p.text(r.center(), Align2::CENTER_CENTER, c, font(12.0), if on { surface(&t) } else { dim(&t) });
            x = r.max.x + 6.0;
        }
        // cards
        let top = rect.min.y + header;
        let mut flips: Vec<usize> = Vec::new();
        let mut start: Vec<String> = Vec::new();
        st.installing.retain(|(_, at)| now - at < 1.2);
        // installs flipped from outside still play the fill
        let first = st.known.is_empty();
        for s in self.skills.iter() {
            match st.known.iter_mut().find(|k| k.0 == s.name) {
                Some(k) => {
                    if s.installed && !k.1 && !first && !st.installing.iter().any(|x| x.0 == s.name) {
                        st.installing.push((s.name.clone(), now));
                    }
                    k.1 = s.installed;
                }
                None => st.known.push((s.name.clone(), s.installed)),
            }
        }
        for (slot, &i) in shown.iter().enumerate() {
            let s = &self.skills[i];
            let target = pos2(rect.min.x + (slot % cols) as f32 * (cw + gap), top + (slot / cols) as f32 * (ch + gap));
            let cid = id.with(("card", &s.name));
            let pos = glowkit_core::anim::spring2(ui, cid.with("pos"), target.to_vec2(), 240.0, 26.0);
            let appear = glowkit_core::anim::follow(ui, cid.with("ap"), 1.0, 7.0);
            let r0 = Rect::from_min_size(pos.to_pos2(), vec2(cw, ch));
            let hov = ui.rect_contains_pointer(r0);
            let k = glowkit_core::anim::follow(ui, cid.with("h"), if hov { 1.0 } else { 0.0 }, 12.0);
            let r = r0.translate(vec2(0.0, -2.0 * k));
            let pa = faded(&p, appear);
            panel_fill(&pa, &t, r, 14.0, surface(&t), 0.5 + k);
            let tile = Rect::from_min_size(r.min + vec2(14.0, 14.0), vec2(38.0, 38.0));
            let col = series(&t, i * 2 + 1);
            pa.rect_filled(tile, cr(10.0), tint(&t, col));
            icon(&pa, s.icon, tile.center(), 18.0, ink(&t, col));
            let tx = tile.max.x + 12.0;
            text(
                &pa,
                pos2(tx, tile.min.y + 10.0),
                Align2::LEFT_CENTER,
                &s.name,
                font(13.5),
                t.foreground,
                r.max.x - tx - 12.0,
            );
            text(
                &pa,
                pos2(tx, tile.min.y + 28.0),
                Align2::LEFT_CENTER,
                &s.author,
                font(11.0),
                faint(&t),
                r.max.x - tx - 12.0,
            );
            para(&pa, pos2(r.min.x + 14.0, tile.max.y + 10.0), &s.desc, font(12.0), dim(&t), r.width() - 28.0, 2);
            let by = r.max.y - 24.0;
            icon(&pa, Icon::Download, pos2(r.min.x + 20.0, by), 11.0, faint(&t));
            text(
                &pa,
                pos2(r.min.x + 30.0, by),
                Align2::LEFT_CENTER,
                &fmt_tok(s.installs as f32),
                font(11.0),
                faint(&t),
                60.0,
            );
            // install button
            let br = Rect::from_min_size(pos2(r.max.x - 14.0 - 96.0, by - 14.0), vec2(96.0, 28.0));
            let installing =
                st.installing.iter().find(|(n, _)| *n == s.name).map(|(_, at)| ((now - at) / 0.9).min(1.0));
            let bresp = ui.interact(br, cid.with("btn"), Sense::click());
            if bresp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            let bh = glowkit_core::anim::follow(ui, cid.with("bh"), if bresp.hovered() { 1.0 } else { 0.0 }, 14.0);
            if let Some(f) = installing {
                pa.rect_filled(br, cr(9.0), track(&t));
                let fr = Rect::from_min_size(br.min, vec2(br.width() * ease(Easing::InOutCubic, f), br.height()));
                pa.with_clip_rect(fr).rect_filled(br, cr(9.0), t.foreground);
                pa.text(
                    br.center(),
                    Align2::CENTER_CENTER,
                    "Installing",
                    font(12.0),
                    if f > 0.55 { surface(&t) } else { t.foreground },
                );
                glowkit_core::animate(ui);
            } else if s.installed {
                let rm = bh > 0.5;
                let c = if rm { t.destructive } else { t.success };
                pa.rect(br, cr(9.0), tint(&t, c), Stroke::new(1.0, with_alpha(c, 0.3)), StrokeKind::Inside);
                let label = if rm { "Remove" } else { "Installed" };
                let lw = text_w(&pa, label, font(12.0));
                let x0 = br.center().x - (lw + 16.0) / 2.0;
                icon(&pa, if rm { Icon::X } else { Icon::Check }, pos2(x0 + 6.0, br.center().y), 12.0, ink(&t, c));
                pa.text(pos2(x0 + 16.0, br.center().y), Align2::LEFT_CENTER, label, font(12.0), ink(&t, c));
                if bresp.clicked() {
                    flips.push(i);
                }
            } else {
                pa.rect(
                    br,
                    cr(9.0),
                    mix(surface(&t), well(&t), 0.5 + bh),
                    Stroke::new(1.0, hair(&t)),
                    StrokeKind::Inside,
                );
                let x0 = br.center().x - 30.0;
                icon(&pa, Icon::Plus, pos2(x0 + 6.0, br.center().y), 12.0, t.foreground);
                pa.text(pos2(x0 + 16.0, br.center().y), Align2::LEFT_CENTER, "Install", font(12.0), t.foreground);
                if bresp.clicked() {
                    start.push(s.name.clone());
                }
            }
        }
        if shown.is_empty() {
            p.text(pos2(rect.center().x, top + 60.0), Align2::CENTER_CENTER, "No skills match", font(13.0), dim(&t));
        }
        // finish installs
        for (name, at) in st.installing.iter() {
            if now - at >= 0.9 {
                if let Some(s) = self.skills.iter_mut().find(|s| s.name == *name) {
                    s.installed = true;
                }
            }
        }
        for name in start {
            st.installing.push((name, now));
        }
        for i in flips {
            self.skills[i].installed = false;
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        let _: Color32 = t.accent;
        resp
    }
}
