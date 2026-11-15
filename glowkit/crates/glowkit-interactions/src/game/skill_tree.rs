//! A skill tree with energy flowing along links as nodes unlock.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use std::f32::consts::TAU;

/// One node of a [`GameSkillTree`].
#[derive(Clone, Debug, PartialEq)]
pub struct GameSkillNode {
    /// Name.
    pub name: String,
    /// Short description shown on hover.
    pub desc: String,
    /// Painted icon.
    pub icon: GameIcon,
    /// Position inside the tree, 0..1 on both axes.
    pub pos: Vec2,
    /// Indices of nodes that must be unlocked first (any one of them).
    pub requires: Vec<usize>,
    /// Point cost.
    pub cost: u32,
    /// Whether it is unlocked.
    pub unlocked: bool,
}

impl GameSkillNode {
    /// A node named `name` at `pos` (0..1).
    pub fn new(name: impl Into<String>, icon: GameIcon, pos: Vec2) -> Self {
        Self { name: name.into(), desc: String::new(), icon, pos, requires: Vec::new(), cost: 1, unlocked: false }
    }
    /// Description.
    pub fn desc(mut self, d: impl Into<String>) -> Self {
        self.desc = d.into();
        self
    }
    /// Prerequisites (indices).
    pub fn requires(mut self, r: &[usize]) -> Self {
        self.requires = r.to_vec();
        self
    }
    /// Point cost.
    pub fn cost(mut self, c: u32) -> Self {
        self.cost = c;
        self
    }
    /// Start unlocked.
    pub fn unlocked(mut self, u: bool) -> Self {
        self.unlocked = u;
        self
    }
}

fn available(nodes: &[GameSkillNode], i: usize) -> bool {
    let n = &nodes[i];
    !n.unlocked && (n.requires.is_empty() || n.requires.iter().any(|r| nodes.get(*r).is_some_and(|p| p.unlocked)))
}

#[derive(Clone, Debug, Default)]
struct TreeState {
    unlocked_at: Vec<f64>,
    denied_at: Vec<f64>,
    from: Vec<Option<usize>>,
    last_auto: f64,
}

/// A skill tree. Links between locked nodes are faint; once a parent is unlocked its links to
/// children shimmer with drifting motes, and unlocking a child sends a bright surge of energy
/// down the link before the node pops, rings and sparkles into its lit state. Hover a node for
/// its card; clicking without enough points shakes it. Points are taken from `points`.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameIcon, GameSkillNode, GameSkillTree};
/// let mut nodes = vec![
///     GameSkillNode::new("Ember", GameIcon::Fire, egui::vec2(0.5, 0.85)).unlocked(true),
///     GameSkillNode::new("Fireball", GameIcon::Fire, egui::vec2(0.3, 0.5)).requires(&[0]),
/// ];
/// let mut points = 3u32;
/// ui.add(GameSkillTree::new(&mut nodes, &mut points));
/// ```
pub struct GameSkillTree<'a> {
    nodes: &'a mut Vec<GameSkillNode>,
    points: &'a mut u32,
    title: &'a str,
    height: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    autoplay: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameSkillTree<'a> {
    /// A tree over `nodes`, spending `points`.
    pub fn new(nodes: &'a mut Vec<GameSkillNode>, points: &'a mut u32) -> Self {
        Self { nodes, points, title: "Skills", height: 380.0, style: GameStyle::SciFi, palette: None, autoplay: false, cues: None }
    }
    /// Panel title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Height in points (width fills the available space).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
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
    /// Demo mode: unlock nodes on its own while idle and reset when done (modifies the data).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Unlock`], [`GameCue::Error`], [`GameCue::Hover`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameSkillTree<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title_f = display_font(ui, 18.0);
        let card_f = display_font(ui, 14.0);
        let w = ui.available_width().min(720.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        glowkit_core::animate(ui);
        let now = ui.input(|i| i.time);
        let time = now as f32;
        let n = self.nodes.len();
        let mut st: TreeState = glowkit_core::state(ui, id.with("tree"), TreeState::default);
        st.unlocked_at.resize(n, -100.0);
        st.denied_at.resize(n, -100.0);
        st.from.resize(n, None);
        let area = Rect::from_min_max(rect.min + vec2(40.0, 56.0), rect.max - vec2(40.0, 34.0));
        let at = |p: Vec2| pos2(area.min.x + p.x * area.width(), area.min.y + p.y * area.height());
        let nr = 22.0;
        // interaction
        let mut hovered = None;
        let mut try_unlock: Option<usize> = None;
        for i in 0..n {
            let r = ui.interact(Rect::from_center_size(at(self.nodes[i].pos), Vec2::splat(nr * 2.2)), id.with(("node", i)), Sense::click());
            if r.hovered() {
                hovered = Some(i);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                try_unlock = Some(i);
            }
        }
        let idle = idle_for(ui, id, hovered.is_some());
        if self.autoplay && idle > 2.0 && now - st.last_auto > 1.5 {
            st.last_auto = now;
            let cand = (0..n).find(|i| available(self.nodes, *i) && self.nodes[*i].cost <= *self.points);
            if let Some(c) = cand {
                try_unlock = Some(c);
            } else {
                for (i, nd) in self.nodes.iter_mut().enumerate() {
                    if !nd.requires.is_empty() {
                        nd.unlocked = false;
                        st.unlocked_at[i] = -100.0;
                    }
                }
                *self.points = (n as u32).saturating_sub(1).max(3);
            }
        }
        if let Some(i) = try_unlock {
            if available(self.nodes, i) && self.nodes[i].cost <= *self.points {
                *self.points -= self.nodes[i].cost;
                st.from[i] = self.nodes[i].requires.iter().copied().find(|r| self.nodes.get(*r).is_some_and(|p| p.unlocked));
                self.nodes[i].unlocked = true;
                st.unlocked_at[i] = now;
                push_cue(&mut self.cues, GameCue::Unlock);
            } else if !self.nodes[i].unlocked {
                st.denied_at[i] = now;
                push_cue(&mut self.cues, GameCue::Error);
            }
        }
        glowkit_core::set_state(ui, id.with("tree"), st.clone());
        let painter = ui.painter();
        frame(painter, rect, self.style, &pal, 0.0);
        // background pattern
        let pc = with_alpha(if self.style == GameStyle::Fantasy { pal.dim } else { pal.accent }, 0.08);
        let step = 26.0;
        let mut y = rect.min.y + 50.0;
        let mut row = 0;
        while y < rect.max.y - 10.0 {
            let mut x = rect.min.x + 14.0 + if row % 2 == 0 { 0.0 } else { step / 2.0 };
            while x < rect.max.x - 10.0 {
                painter.circle_filled(pos2(x, y), 1.2, pc);
                x += step;
            }
            y += step * 0.866;
            row += 1;
        }
        painter.text(pos2(rect.min.x + 22.0, rect.min.y + 26.0), Align2::LEFT_CENTER, self.title.to_uppercase(), title_f.clone(), pal.text);
        // points counter
        let (psince, _, _) = changed(ui, id.with("pts"), *self.points as f32);
        let pp = 1.0 + 0.3 * (1.0 - (psince / 0.3).min(1.0));
        let pt = format!("{}", self.points);
        let px = rect.max.x - 22.0;
        let painter = ui.painter();
        painter.text(pos2(px - 30.0, rect.min.y + 26.0), Align2::RIGHT_CENTER, "SKILL POINTS", FontId::monospace(11.0), pal.dim);
        let pc2 = pos2(px - 12.0, rect.min.y + 26.0);
        paint::glow(painter, pc2, 18.0 * pp, with_alpha(pal.accent, 0.35));
        painter.circle_filled(pc2, 12.0 * pp, if self.style == GameStyle::Fantasy { hex("#2B1D12") } else { hex("#06101E") });
        painter.circle_stroke(pc2, 12.0 * pp, Stroke::new(1.5, pal.accent));
        painter.text(pc2, Align2::CENTER_CENTER, pt, FontId::proportional(13.0 * pp), Color32::WHITE);
        // links
        for i in 0..n {
            for &p in &self.nodes[i].requires {
                let Some(parent) = self.nodes.get(p) else { continue };
                let a = at(parent.pos);
                let b = at(self.nodes[i].pos);
                let dir = (b - a).normalized();
                let a = a + dir * nr;
                let b = b - dir * nr;
                let child = &self.nodes[i];
                let ut = (now - st.unlocked_at[i]) as f32;
                let via = st.from[i] == Some(p) || st.from[i].is_none();
                if child.unlocked && parent.unlocked && via {
                    let fill = if ut < 0.45 { glowkit_core::ease(glowkit_core::Easing::InOutCubic, ut / 0.45) } else { 1.0 };
                    painter.line_segment([a, b], Stroke::new(4.0, with_alpha(pal.dim, 0.25)));
                    let m = a + (b - a) * fill;
                    painter.line_segment([a, m], Stroke::new(7.0, with_alpha(pal.accent, 0.25)));
                    painter.line_segment([a, m], Stroke::new(3.0, mix(pal.accent, Color32::WHITE, 0.3)));
                    if fill < 1.0 {
                        paint::glow(painter, m, 18.0, with_alpha(Color32::WHITE, 0.8));
                    }
                    // flowing pulses
                    let len = (b - a).length().max(1.0);
                    for k in 0..3 {
                        let t = ((time * 60.0 / len) + k as f32 / 3.0).fract() * fill;
                        painter.circle_filled(a + (b - a) * t, 2.0, with_alpha(Color32::WHITE, 0.7));
                    }
                } else if parent.unlocked && !child.unlocked {
                    let len = (b - a).length().max(1.0);
                    let dash = 7.0;
                    let mut d = 0.0;
                    let shift = (time * 20.0) % (dash * 2.0);
                    while d < len {
                        let s0 = ((d + shift - dash * 2.0).max(0.0)).min(len);
                        let s1 = (d + shift - dash).clamp(0.0, len);
                        if s1 > s0 {
                            painter.line_segment([a + dir * s0, a + dir * s1], Stroke::new(2.0, with_alpha(pal.accent, 0.55)));
                        }
                        d += dash * 2.0;
                    }
                } else {
                    painter.line_segment([a, b], Stroke::new(2.0, with_alpha(pal.dim, 0.25)));
                }
            }
        }
        // nodes
        for i in 0..n {
            let nd = &self.nodes[i];
            let mut c = at(nd.pos);
            let ut = (now - st.unlocked_at[i]) as f32 - 0.42;
            let dt = (now - st.denied_at[i]) as f32;
            if dt < 0.4 {
                c.x += (dt * 60.0).sin() * 6.0 * (1.0 - dt / 0.4);
            }
            let avail = available(self.nodes, i);
            let affordable = avail && nd.cost <= *self.points;
            let hov = hovered == Some(i);
            let target = if hov { 1.12 } else { 1.0 };
            let mut s = spring(ui, id.with(("ns", i)), target, 420.0, 18.0);
            let lit = nd.unlocked && ut >= 0.0;
            if nd.unlocked && (0.0..0.5).contains(&ut) {
                s += 0.35 * (ut / 0.5 * std::f32::consts::PI).sin();
            }
            let r = nr * s;
            if lit {
                paint::glow(painter, c, r * 2.4, with_alpha(pal.accent, 0.3));
            } else if affordable {
                let p = 0.5 + 0.5 * (time * 3.0 + i as f32).sin();
                paint::glow(painter, c, r * 2.0, with_alpha(pal.accent, 0.2 * p));
            }
            let (fill, edge) = if lit {
                (if self.style == GameStyle::Fantasy { hex("#5A2A18") } else { mix(hex("#06101E"), pal.accent, 0.35) }, mix(pal.accent, Color32::WHITE, 0.3))
            } else if avail {
                (if self.style == GameStyle::Fantasy { hex("#3A2918") } else { hex("#0A1424") }, with_alpha(pal.accent, if affordable { 0.9 } else { 0.4 }))
            } else {
                (if self.style == GameStyle::Fantasy { hex("#4A3A28") } else { hex("#0A0E16") }, with_alpha(pal.dim, 0.4))
            };
            match self.style {
                GameStyle::SciFi => {
                    let pts: Vec<Pos2> = (0..6).map(|k| {
                        let a = k as f32 / 6.0 * TAU;
                        c + vec2(a.cos(), a.sin()) * r
                    }).collect();
                    painter.add(Shape::convex_polygon(pts, fill, Stroke::new(2.0, edge)));
                }
                GameStyle::Fantasy => {
                    painter.circle_filled(c, r, fill);
                    painter.circle_stroke(c, r, Stroke::new(3.0, if lit { pal.edge } else { edge }));
                    painter.circle_stroke(c, r - 4.0, Stroke::new(1.0, with_alpha(pal.edge, 0.5)));
                }
                GameStyle::Minimal => {
                    painter.circle_filled(c, r, fill);
                    painter.circle_stroke(c, r, Stroke::new(1.5, edge));
                }
            }
            let ic = if lit { nd.icon.tint() } else if avail { mix(nd.icon.tint(), hex("#707680"), 0.5) } else { hex("#4A505C") };
            nd.icon.paint(painter, c, r * 1.05, ic);
            if nd.unlocked {
                ring(painter, c, ut, 0.6, r, r * 2.6, pal.accent, 3.0);
                burst(painter, c, ut, 16, 5 + i as u64, 260.0, 60.0, pal.accent, 2.6);
                if (0.0..0.6).contains(&ut) {
                    rays(painter, c, r * 3.0, 10, ut * 2.0, with_alpha(pal.accent, 0.35 * (1.0 - ut / 0.6)));
                }
            }
            if !nd.unlocked && nd.cost > 1 {
                let b = c + vec2(r * 0.75, r * 0.75);
                painter.circle_filled(b, 8.0, hex("#000000D0"));
                painter.text(b, Align2::CENTER_CENTER, nd.cost.to_string(), FontId::proportional(10.0), if affordable { pal.accent } else { pal.dim });
            }
        }
        // hover card
        let show = hovered.or(if self.autoplay && idle > 2.0 {
            (0..n).filter(|i| self.nodes[*i].unlocked).max_by(|a, b| st.unlocked_at[*a].total_cmp(&st.unlocked_at[*b]))
        } else {
            None
        });
        if let Some(i) = show {
            let nd = &self.nodes[i];
            let c = at(nd.pos);
            let cw = 200.0;
            let g = painter.layout(nd.desc.clone(), FontId::proportional(11.5), pal.dim, cw - 24.0);
            let chh = 50.0 + g.size().y + 22.0;
            let mut tl = c + vec2(nr + 14.0, -chh / 2.0);
            if tl.x + cw > rect.max.x - 6.0 {
                tl.x = c.x - nr - 14.0 - cw;
            }
            tl.y = tl.y.clamp(rect.min.y + 6.0, rect.max.y - chh - 6.0);
            let card = Rect::from_min_size(tl, vec2(cw, chh));
            frame(painter, card, self.style, &pal, 0.5);
            painter.text(card.min + vec2(12.0, 18.0), Align2::LEFT_CENTER, &nd.name, card_f.clone(), pal.text);
            let status = if nd.unlocked { ("UNLOCKED", pal.good) } else if available(self.nodes, i) { ("AVAILABLE", pal.accent) } else { ("LOCKED", pal.dim) };
            let sc = if self.style == GameStyle::Fantasy { glowkit_core::color::shade(status.1, -0.35) } else { status.1 };
            painter.text(card.min + vec2(12.0, 36.0), Align2::LEFT_CENTER, status.0, FontId::monospace(10.0), sc);
            painter.text(pos2(card.max.x - 12.0, card.min.y + 36.0), Align2::RIGHT_CENTER, format!("{} PT", nd.cost), FontId::monospace(10.0), pal.dim);
            painter.galley(card.min + vec2(12.0, 50.0), g, pal.dim);
        }
        resp
    }
}
