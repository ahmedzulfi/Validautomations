//! Agent workflow graph: nodes joined by bezier edges with flowing dots.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// The role of a [`WorkflowNode`]; decides its icon and accent color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeKind {
    /// Entry point (webhook, chat message, schedule).
    #[default]
    Trigger,
    /// A language-model call.
    Llm,
    /// A tool / API call.
    Tool,
    /// A branch.
    Condition,
    /// Final output.
    Output,
}

impl NodeKind {
    fn icon(self) -> Icon {
        match self {
            NodeKind::Trigger => Icon::Bolt,
            NodeKind::Llm => Icon::Sparkle,
            NodeKind::Tool => Icon::Gear,
            NodeKind::Condition => Icon::Code,
            NodeKind::Output => Icon::ArrowRight,
        }
    }
    fn label(self) -> &'static str {
        match self {
            NodeKind::Trigger => "TRIGGER",
            NodeKind::Llm => "LLM",
            NodeKind::Tool => "TOOL",
            NodeKind::Condition => "CONDITION",
            NodeKind::Output => "OUTPUT",
        }
    }
    fn color(self, t: &Theme) -> Color32 {
        match self {
            NodeKind::Trigger => t.warning,
            NodeKind::Llm => t.accent,
            NodeKind::Tool => t.info,
            NodeKind::Condition => t.chart[2],
            NodeKind::Output => t.success,
        }
    }
}

/// Run state of a [`WorkflowNode`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeStatus {
    /// Not run.
    #[default]
    Idle,
    /// Running: pulsing outline.
    Running,
    /// Finished: small check.
    Done,
}

/// A node of a [`Workflow`]. `pos` is the node's center in normalized canvas coordinates
/// (0..1 on both axes) so graphs scale with the available space.
#[derive(Clone, Copy, Debug)]
pub struct WorkflowNode<'a> {
    /// Kind (icon + color).
    pub kind: NodeKind,
    /// Title, e.g. "Summarize".
    pub title: &'a str,
    /// Muted second line, e.g. "claude-sonnet".
    pub subtitle: &'a str,
    /// Center position, normalized 0..1.
    pub pos: Pos2,
    /// Run state.
    pub status: NodeStatus,
}

impl<'a> WorkflowNode<'a> {
    /// A node at normalized position (`x`, `y`).
    pub const fn new(kind: NodeKind, title: &'a str, subtitle: &'a str, x: f32, y: f32) -> Self {
        Self { kind, title, subtitle, pos: Pos2 { x, y }, status: NodeStatus::Idle }
    }
    /// Set the run state.
    pub const fn status(mut self, s: NodeStatus) -> Self {
        self.status = s;
        self
    }
}

/// An edge from node index `from` to node index `to`; `active` edges show flowing dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowEdge {
    /// Source node index.
    pub from: usize,
    /// Target node index.
    pub to: usize,
    /// Animate data flowing along it.
    pub active: bool,
}

impl WorkflowEdge {
    /// An animated edge.
    pub const fn new(from: usize, to: usize) -> Self {
        Self { from, to, active: true }
    }
    /// Set whether dots flow along it.
    pub const fn active(mut self, on: bool) -> Self {
        self.active = on;
        self
    }
}

/// A node-graph canvas for agent workflows: dotted background, node cards (trigger, LLM, tool,
/// output) with ports, smooth bezier edges and animated dots flowing along active edges.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{NodeKind, Workflow, WorkflowEdge, WorkflowNode};
/// let nodes = [
///     WorkflowNode::new(NodeKind::Trigger, "Chat message", "on message", 0.12, 0.5),
///     WorkflowNode::new(NodeKind::Llm, "Answer", "claude-sonnet", 0.5, 0.5),
///     WorkflowNode::new(NodeKind::Output, "Reply", "stream", 0.88, 0.5),
/// ];
/// let edges = [WorkflowEdge::new(0, 1), WorkflowEdge::new(1, 2)];
/// ui.add(Workflow::new(&nodes, &edges).height(260.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Workflow<'a> {
    nodes: &'a [WorkflowNode<'a>],
    edges: &'a [WorkflowEdge],
    height: f32,
    node_size: egui::Vec2,
    grid: bool,
}

impl<'a> Workflow<'a> {
    /// A graph of `nodes` joined by `edges`.
    pub fn new(nodes: &'a [WorkflowNode<'a>], edges: &'a [WorkflowEdge]) -> Self {
        Self { nodes, edges, height: 300.0, node_size: vec2(176.0, 58.0), grid: true }
    }
    /// Canvas height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Node card size.
    pub fn node_size(mut self, s: egui::Vec2) -> Self {
        self.node_size = s;
        self
    }
    /// Dotted background grid (default on).
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }
}

impl Widget for Workflow<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let cr = t.corner_scaled(1.2);
        painter.rect_filled(rect, cr, if t.dark { mix(t.background, t.card, 0.5) } else { mix(t.muted, t.card, 0.5) });
        if self.grid {
            let step = 20.0;
            let dot = with_alpha(t.muted_foreground, if t.dark { 0.22 } else { 0.3 });
            let mut y = rect.min.y + step / 2.0;
            while y < rect.max.y {
                let mut x = rect.min.x + step / 2.0;
                while x < rect.max.x {
                    painter.circle_filled(pos2(x, y), 0.9, dot);
                    x += step;
                }
                y += step;
            }
        }
        let ns = self.node_size;
        let inner = rect.shrink2(vec2(ns.x / 2.0 + 16.0, ns.y / 2.0 + 16.0));
        let center = |n: &WorkflowNode| {
            pos2(
                inner.min.x + n.pos.x.clamp(0.0, 1.0) * inner.width(),
                inner.min.y + n.pos.y.clamp(0.0, 1.0) * inner.height(),
            )
        };
        let time = glowkit_core::time(ui);
        let mut animate = false;
        // edges
        for (ei, e) in self.edges.iter().enumerate() {
            let (Some(a), Some(b)) = (self.nodes.get(e.from), self.nodes.get(e.to)) else { continue };
            let pa = center(a) + vec2(ns.x / 2.0, 0.0);
            let pb = center(b) - vec2(ns.x / 2.0, 0.0);
            let dx = ((pb.x - pa.x).abs() * 0.5).max(40.0);
            let pts = paint::cubic_bezier(pa, pa + vec2(dx, 0.0), pb - vec2(dx, 0.0), pb, 48);
            let base = if e.active {
                mix(common::border_strong(&t), a.kind.color(&t), 0.35)
            } else {
                common::border_strong(&t)
            };
            paint::gradient_polyline(&painter, &pts, false, 1.6, |_| base);
            if e.active {
                animate = true;
                let col = b.kind.color(&t);
                // dots flow from a to b
                for k in 0..3 {
                    let u = ((time * 0.45 + k as f32 / 3.0 + ei as f32 * 0.13).fract()).clamp(0.0, 1.0);
                    let idx = u * (pts.len() - 1) as f32;
                    let i0 = idx.floor() as usize;
                    let i1 = (i0 + 1).min(pts.len() - 1);
                    let p = pts[i0].lerp(pts[i1], idx.fract());
                    let fade = (u * 6.0).min(1.0) * ((1.0 - u) * 6.0).min(1.0);
                    paint::glow(&painter, p, 9.0, with_alpha(col, 0.35 * fade));
                    painter.circle_filled(p, 2.6, with_alpha(mix(col, Color32::WHITE, 0.35), fade));
                }
            }
        }
        // nodes
        for n in self.nodes {
            let c = center(n);
            let r = Rect::from_center_size(c, ns);
            let kc = n.kind.color(&t);
            paint::soft_shadow(
                &painter,
                r,
                10.0,
                14.0,
                vec2(0.0, 4.0),
                Color32::from_black_alpha(if t.dark { 90 } else { 22 }),
            );
            painter.rect_filled(r, CornerRadius::same(10), t.card);
            let stroke_col = match n.status {
                NodeStatus::Running => {
                    animate = true;
                    let pulse = (time * 3.0).sin() * 0.5 + 0.5;
                    paint::glow_ellipse(&painter, c, ns * 0.75, with_alpha(kc, 0.10 + 0.08 * pulse));
                    mix(t.border, kc, 0.55 + 0.35 * pulse)
                }
                _ => t.border,
            };
            painter.rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, stroke_col), StrokeKind::Inside);
            // icon tile
            let ib = Rect::from_center_size(pos2(r.min.x + 25.0, c.y), vec2(30.0, 30.0));
            painter.rect_filled(ib, CornerRadius::same(8), with_alpha(kc, if t.dark { 0.16 } else { 0.12 }));
            painter.rect_stroke(ib, CornerRadius::same(8), Stroke::new(1.0, with_alpha(kc, 0.3)), StrokeKind::Inside);
            paint_icon(&painter, n.kind.icon(), ib.shrink(8.0), kc);
            let tx = ib.max.x + 10.0;
            let tag = common::text(
                &painter,
                pos2(tx, r.min.y + 11.0),
                Align2::LEFT_TOP,
                n.kind.label(),
                9.5,
                with_alpha(kc, 0.95),
            );
            if !n.subtitle.is_empty() {
                let sub = painter.with_clip_rect(Rect::from_min_max(r.min, pos2(r.max.x - 20.0, r.max.y)));
                common::text(
                    &sub,
                    pos2(tag.max.x + 6.0, tag.center().y),
                    Align2::LEFT_CENTER,
                    format!("·  {}", n.subtitle),
                    10.5,
                    t.muted_foreground,
                );
            }
            let tg = ui.painter().layout_no_wrap(n.title.to_owned(), egui::FontId::proportional(13.0), t.foreground);
            common::paint_galley_fx(&painter, pos2(tx, tag.max.y + 3.0), &tg, |_| common::Fx {
                bold: true,
                ..Default::default()
            });
            if n.status == NodeStatus::Done {
                let cc = pos2(r.max.x - 12.0, r.min.y + 12.0);
                painter.circle_filled(cc, 6.0, t.success);
                paint_icon(
                    &painter,
                    Icon::Check,
                    Rect::from_center_size(cc, vec2(8.0, 8.0)),
                    if t.dark { Color32::from_rgb(5, 25, 12) } else { Color32::WHITE },
                );
            }
            // ports
            for (px, used) in [(r.min.x, n.kind != NodeKind::Trigger), (r.max.x, n.kind != NodeKind::Output)] {
                if used {
                    painter.circle_filled(pos2(px, c.y), 4.0, t.card);
                    painter.circle_stroke(pos2(px, c.y), 4.0, Stroke::new(1.4, common::border_strong(&t)));
                }
            }
        }
        painter.rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        if animate {
            glowkit_core::animate(ui);
        }
        resp
    }
}
