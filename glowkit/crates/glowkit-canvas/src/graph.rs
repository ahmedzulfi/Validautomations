//! The caller-owned data model of the [`crate::NodeEditor`]: [`Graph`], [`Node`], [`Port`],
//! [`Edge`], plus pure helpers (geometry, hit-testing, validation, layered auto-layout).

use crate::common::{dist_to_polyline, orthogonal_route};
use egui::{pos2, vec2, Color32, Pos2, Rect, Vec2};

/// Stable node identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct NodeId(pub u64);

/// Stable edge identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct EdgeId(pub u64);

/// Data type carried by a port. Ports connect when their type names match or either is "any"
/// (an empty name).
#[derive(Clone, Debug, PartialEq)]
pub struct PortType {
    /// Type name (`""` = any).
    pub name: String,
    /// Handle / edge color.
    pub color: Option<Color32>,
}

impl PortType {
    /// The wildcard type that connects to everything.
    pub fn any() -> Self {
        Self { name: String::new(), color: None }
    }
    /// A named type with a color.
    pub fn new(name: impl Into<String>, color: Color32) -> Self {
        Self { name: name.into(), color: Some(color) }
    }
    /// Can a value of type `self` flow into `other`?
    pub fn compatible(&self, other: &PortType) -> bool {
        self.name.is_empty() || other.name.is_empty() || self.name == other.name
    }
}

/// An input or output handle on a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    /// Label shown next to the handle (may be empty).
    pub label: String,
    /// Data type.
    pub ty: PortType,
    /// Inputs only: accept several incoming edges (otherwise a new edge replaces the old one).
    pub multi: bool,
}

impl Port {
    /// An untyped port.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), ty: PortType::any(), multi: false }
    }
    /// A typed port.
    pub fn typed(label: impl Into<String>, name: impl Into<String>, color: Color32) -> Self {
        Self { label: label.into(), ty: PortType::new(name, color), multi: false }
    }
    /// Allow several incoming edges.
    pub fn multi(mut self) -> Self {
        self.multi = true;
        self
    }
}

/// Which side of a node a port is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortSide {
    Input,
    Output,
}

/// Reference to one port of one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortRef {
    pub node: NodeId,
    pub side: PortSide,
    pub index: usize,
}

/// Visual shape of a node. `Card` is the classic node-editor card with a header and labelled
/// ports; the others are flowchart symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum NodeShape {
    #[default]
    Card,
    /// Rounded rectangle (process).
    Process,
    /// Stadium / pill (start / end).
    Terminator,
    /// Rhombus (decision).
    Decision,
    /// Slanted parallelogram (input / output).
    Data,
    /// Rectangle with a wavy bottom (document).
    Document,
    /// Circle (connector / event).
    Circle,
}

/// Where ports sit: inputs left / outputs right, or inputs top / outputs bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum PortLayout {
    #[default]
    Horizontal,
    Vertical,
}

/// Run state badge shown on a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum NodeStatus {
    #[default]
    None,
    Running,
    Done,
    Error,
}

/// A node in a [`Graph`].
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    /// Top-left corner in world coordinates.
    pub pos: Pos2,
    /// Width in world units (cards compute their height from content).
    pub width: f32,
    /// Height for non-card shapes.
    pub height: f32,
    pub title: String,
    pub subtitle: String,
    /// A short glyph (1-2 characters) shown in the header badge.
    pub icon: String,
    /// Accent color (defaults to the theme accent).
    pub color: Option<Color32>,
    /// Muted text lines shown under the ports.
    pub body: Vec<String>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub shape: NodeShape,
    pub layout: PortLayout,
    pub status: NodeStatus,
    /// Extra space (world units) reserved at the bottom of a card for custom painting.
    pub custom_height: f32,
    /// Free-form tag for the caller (e.g. the node kind).
    pub tag: String,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            id: NodeId(0),
            pos: Pos2::ZERO,
            width: 200.0,
            height: 64.0,
            title: String::new(),
            subtitle: String::new(),
            icon: String::new(),
            color: None,
            body: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            shape: NodeShape::Card,
            layout: PortLayout::Horizontal,
            status: NodeStatus::None,
            custom_height: 0.0,
            tag: String::new(),
        }
    }
}

/// Header height of a card node.
pub const HEADER: f32 = 44.0;
/// Height of one port row.
pub const ROW: f32 = 24.0;
/// Height of one body line.
pub const LINE: f32 = 18.0;

impl Node {
    /// A card node titled `title` at `pos`.
    pub fn new(title: impl Into<String>, pos: Pos2) -> Self {
        Self { title: title.into(), pos, ..Default::default() }
    }
    /// A flowchart-shape node.
    pub fn shape(shape: NodeShape, title: impl Into<String>, pos: Pos2) -> Self {
        let (w, h) = match shape {
            NodeShape::Decision => (150.0, 96.0),
            NodeShape::Circle => (72.0, 72.0),
            NodeShape::Terminator => (150.0, 52.0),
            _ => (170.0, 64.0),
        };
        Self { title: title.into(), pos, shape, width: w, height: h, layout: PortLayout::Vertical, inputs: vec![Port::new("").multi()], outputs: vec![Port::new("")], ..Default::default() }
    }
    /// Builder: subtitle.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Builder: header glyph.
    pub fn icon(mut self, s: impl Into<String>) -> Self {
        self.icon = s.into();
        self
    }
    /// Builder: accent color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Builder: width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Builder: add an input.
    pub fn input(mut self, p: Port) -> Self {
        self.inputs.push(p);
        self
    }
    /// Builder: add an output.
    pub fn output(mut self, p: Port) -> Self {
        self.outputs.push(p);
        self
    }
    /// Builder: add a body line.
    pub fn line(mut self, s: impl Into<String>) -> Self {
        self.body.push(s.into());
        self
    }
    /// Builder: status badge.
    pub fn status(mut self, s: NodeStatus) -> Self {
        self.status = s;
        self
    }
    /// Builder: reserve custom-paint space.
    pub fn custom(mut self, h: f32) -> Self {
        self.custom_height = h;
        self
    }
    /// Builder: caller tag.
    pub fn tag(mut self, s: impl Into<String>) -> Self {
        self.tag = s.into();
        self
    }
    /// Builder: port layout.
    pub fn port_layout(mut self, l: PortLayout) -> Self {
        self.layout = l;
        self
    }

    /// Size in world units.
    pub fn size(&self) -> Vec2 {
        if self.shape != NodeShape::Card {
            return vec2(self.width, self.height);
        }
        let rows = if self.layout == PortLayout::Horizontal { self.inputs.len().max(self.outputs.len()) } else { 0 };
        let mut h = HEADER + 8.0 + rows as f32 * ROW + self.body.len() as f32 * LINE + 8.0;
        if self.custom_height > 0.0 {
            h += self.custom_height + 6.0;
        }
        vec2(self.width, h.max(HEADER + 14.0))
    }

    /// World rect.
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size())
    }

    /// Port anchor relative to the node's top-left.
    pub fn port_offset(&self, side: PortSide, index: usize) -> Vec2 {
        let size = self.size();
        let n = match side {
            PortSide::Input => self.inputs.len(),
            PortSide::Output => self.outputs.len(),
        }
        .max(1);
        match self.layout {
            PortLayout::Horizontal if self.shape == NodeShape::Card => {
                let y = HEADER + 8.0 + ROW * 0.5 + index as f32 * ROW;
                vec2(if side == PortSide::Input { 0.0 } else { size.x }, y)
            }
            PortLayout::Horizontal => {
                let y = size.y * (index as f32 + 1.0) / (n as f32 + 1.0);
                vec2(if side == PortSide::Input { 0.0 } else { size.x }, y)
            }
            PortLayout::Vertical => {
                let x = size.x * (index as f32 + 1.0) / (n as f32 + 1.0);
                vec2(x, if side == PortSide::Input { 0.0 } else { size.y })
            }
        }
    }

    /// Port anchor in world space (at `pos`, which may differ from `self.pos` while animating).
    pub fn port_at(&self, pos: Pos2, side: PortSide, index: usize) -> Pos2 {
        pos + self.port_offset(side, index)
    }

    /// Outward direction of a port (edges leave/enter along it).
    pub fn port_dir(&self, side: PortSide) -> Vec2 {
        let s = if side == PortSide::Input { -1.0 } else { 1.0 };
        match self.layout {
            PortLayout::Horizontal => vec2(s, 0.0),
            PortLayout::Vertical => vec2(0.0, s),
        }
    }

    /// The port list for a side.
    pub fn ports(&self, side: PortSide) -> &[Port] {
        match side {
            PortSide::Input => &self.inputs,
            PortSide::Output => &self.outputs,
        }
    }
}

/// How an edge is routed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum EdgeKind {
    /// Smooth cubic curve.
    #[default]
    Bezier,
    /// Orthogonal segments with rounded corners.
    SmoothStep,
    /// Orthogonal segments with sharp corners.
    Step,
    /// A straight line.
    Straight,
}

impl EdgeKind {
    /// All kinds (for pickers).
    pub const ALL: [EdgeKind; 4] = [EdgeKind::Bezier, EdgeKind::SmoothStep, EdgeKind::Step, EdgeKind::Straight];
    /// Human label.
    pub fn label(self) -> &'static str {
        match self {
            EdgeKind::Bezier => "Bezier",
            EdgeKind::SmoothStep => "Smooth step",
            EdgeKind::Step => "Step",
            EdgeKind::Straight => "Straight",
        }
    }
}

/// How an edge animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum EdgeAnim {
    #[default]
    None,
    /// Dashes flowing from source to target.
    Dashes,
    /// Glowing dots travelling along the edge.
    Dots,
}

/// A connection from an output port to an input port.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub from_port: usize,
    pub to: NodeId,
    pub to_port: usize,
    /// Routing override (defaults to the editor's edge kind).
    pub kind: Option<EdgeKind>,
    pub anim: EdgeAnim,
    /// Label drawn at the midpoint (empty = none).
    pub label: String,
    /// Color override (defaults to the source port type color or a neutral).
    pub color: Option<Color32>,
}

impl Edge {
    /// Set the animation (chainable on `&mut Edge`, e.g. the result of [`Graph::link`]).
    pub fn animated(&mut self, a: EdgeAnim) -> &mut Self {
        self.anim = a;
        self
    }
    /// Set the label.
    pub fn label(&mut self, s: impl Into<String>) -> &mut Self {
        self.label = s.into();
        self
    }
    /// Set the routing.
    pub fn kind(&mut self, k: EdgeKind) -> &mut Self {
        self.kind = Some(k);
        self
    }
}

/// Why a connection was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectError {
    MissingPort,
    SameNode,
    TypeMismatch,
    Duplicate,
}

/// Direction of the layered auto-layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LayoutDirection {
    #[default]
    LeftToRight,
    TopToBottom,
}

/// A node graph: nodes (in z-order, last on top) and edges. Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    next_id: u64,
}

impl Graph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    fn fresh(&mut self) -> u64 {
        let max = self.nodes.iter().map(|n| n.id.0).chain(self.edges.iter().map(|e| e.id.0)).max().unwrap_or(0);
        self.next_id = self.next_id.max(max) + 1;
        self.next_id
    }

    /// Add a node (assigning a fresh id) and return its id.
    pub fn add(&mut self, mut node: Node) -> NodeId {
        node.id = NodeId(self.fresh());
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Node by id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Mutable node by id.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Port by reference.
    pub fn port(&self, r: PortRef) -> Option<&Port> {
        self.node(r.node).and_then(|n| n.ports(r.side).get(r.index))
    }

    /// Check whether `a` and `b` (in any order) may be connected.
    pub fn can_connect(&self, a: PortRef, b: PortRef) -> Result<(PortRef, PortRef), ConnectError> {
        let (out, inp) = match (a.side, b.side) {
            (PortSide::Output, PortSide::Input) => (a, b),
            (PortSide::Input, PortSide::Output) => (b, a),
            _ => return Err(ConnectError::MissingPort),
        };
        let (po, pi) = (self.port(out).ok_or(ConnectError::MissingPort)?, self.port(inp).ok_or(ConnectError::MissingPort)?);
        if out.node == inp.node {
            return Err(ConnectError::SameNode);
        }
        if !po.ty.compatible(&pi.ty) {
            return Err(ConnectError::TypeMismatch);
        }
        if self.edges.iter().any(|e| e.from == out.node && e.from_port == out.index && e.to == inp.node && e.to_port == inp.index) {
            return Err(ConnectError::Duplicate);
        }
        Ok((out, inp))
    }

    /// Connect two ports (any order). Single inputs drop their previous edge.
    pub fn connect(&mut self, a: PortRef, b: PortRef) -> Result<EdgeId, ConnectError> {
        let (out, inp) = self.can_connect(a, b)?;
        if !self.port(inp).is_some_and(|p| p.multi) {
            self.edges.retain(|e| !(e.to == inp.node && e.to_port == inp.index));
        }
        let id = EdgeId(self.fresh());
        self.edges.push(Edge { id, from: out.node, from_port: out.index, to: inp.node, to_port: inp.index, kind: None, anim: EdgeAnim::None, label: String::new(), color: None });
        Ok(id)
    }

    /// Connect output `from.0` port `from.1` to input `to.0` port `to.1`, returning a mutable edge
    /// for further styling. Panics on invalid connections (meant for building demo graphs).
    pub fn link(&mut self, from: (NodeId, usize), to: (NodeId, usize)) -> &mut Edge {
        let id = self
            .connect(PortRef { node: from.0, side: PortSide::Output, index: from.1 }, PortRef { node: to.0, side: PortSide::Input, index: to.1 })
            .expect("invalid link");
        self.edges.iter_mut().find(|e| e.id == id).expect("edge")
    }

    /// Remove nodes (and their edges).
    pub fn remove_nodes(&mut self, ids: &[NodeId]) {
        self.nodes.retain(|n| !ids.contains(&n.id));
        self.edges.retain(|e| !ids.contains(&e.from) && !ids.contains(&e.to));
    }

    /// Remove edges.
    pub fn remove_edges(&mut self, ids: &[EdgeId]) {
        self.edges.retain(|e| !ids.contains(&e.id));
    }

    /// Move a node to the top of the z-order.
    pub fn bring_to_front(&mut self, id: NodeId) {
        if let Some(i) = self.nodes.iter().position(|n| n.id == id) {
            let n = self.nodes.remove(i);
            self.nodes.push(n);
        }
    }

    /// Bounding box of all nodes (world).
    pub fn bounds(&self) -> Rect {
        self.nodes.iter().fold(Rect::NOTHING, |r, n| r.union(n.rect()))
    }

    /// Topmost node containing `p`.
    pub fn hit_node(&self, p: Pos2) -> Option<NodeId> {
        self.nodes.iter().rev().find(|n| n.rect().contains(p)).map(|n| n.id)
    }

    /// Closest port within `radius` of `p`.
    pub fn hit_port(&self, p: Pos2, radius: f32) -> Option<PortRef> {
        let mut best: Option<(f32, PortRef)> = None;
        for n in self.nodes.iter().rev() {
            for side in [PortSide::Input, PortSide::Output] {
                for i in 0..n.ports(side).len() {
                    let d = n.port_at(n.pos, side, i).distance(p);
                    if d <= radius && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, PortRef { node: n.id, side, index: i }));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Endpoints of an edge: (from position, from direction, to position, to direction).
    pub fn edge_ends(&self, e: &Edge) -> Option<(Pos2, Vec2, Pos2, Vec2)> {
        let a = self.node(e.from)?;
        let b = self.node(e.to)?;
        Some((a.port_at(a.pos, PortSide::Output, e.from_port), a.port_dir(PortSide::Output), b.port_at(b.pos, PortSide::Input, e.to_port), b.port_dir(PortSide::Input)))
    }

    /// Closest edge whose path passes within `radius` of `p` (world), using `kind` routing.
    pub fn hit_edge(&self, p: Pos2, radius: f32, kind: EdgeKind) -> Option<EdgeId> {
        let mut best: Option<(f32, EdgeId)> = None;
        for e in &self.edges {
            if let Some((a, da, b, db)) = self.edge_ends(e) {
                let path = edge_path(e.kind.unwrap_or(kind), a, da, b, db, 1.0);
                let d = dist_to_polyline(p, &path);
                if d <= radius && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, e.id));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Copy `ids` (and the edges between them) shifted by `offset`; returns the new ids.
    pub fn duplicate(&mut self, ids: &[NodeId], offset: Vec2) -> Vec<NodeId> {
        let src: Vec<Node> = self.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect();
        let edges: Vec<Edge> = self.edges.iter().filter(|e| ids.contains(&e.from) && ids.contains(&e.to)).cloned().collect();
        self.paste(&src, &edges, offset)
    }

    /// Insert copies of `nodes` / `edges` (from a clipboard) shifted by `offset`; returns new ids.
    pub fn paste(&mut self, nodes: &[Node], edges: &[Edge], offset: Vec2) -> Vec<NodeId> {
        let mut map = Vec::new();
        for n in nodes {
            let mut c = n.clone();
            c.pos += offset;
            let new = self.add(c);
            map.push((n.id, new));
        }
        let lookup = |id: NodeId| map.iter().find(|m| m.0 == id).map(|m| m.1);
        for e in edges {
            if let (Some(f), Some(t)) = (lookup(e.from), lookup(e.to)) {
                let mut c = e.clone();
                c.id = EdgeId(self.fresh());
                c.from = f;
                c.to = t;
                self.edges.push(c);
            }
        }
        map.into_iter().map(|m| m.1).collect()
    }

    /// Compute layered (Sugiyama-style) positions without applying them.
    ///
    /// Ranks come from the longest path over a DAG (back edges of cycles are ignored), node order
    /// inside each layer is refined with barycenter sweeps, and nodes are pulled toward the
    /// average height of their neighbours.
    pub fn layered_positions(&self, dir: LayoutDirection, gap: Vec2) -> Vec<(NodeId, Pos2)> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }
        let idx = |id: NodeId| self.nodes.iter().position(|nd| nd.id == id);
        let mut succ = vec![Vec::new(); n];
        for e in &self.edges {
            if let (Some(a), Some(b)) = (idx(e.from), idx(e.to)) {
                if a != b && !succ[a].contains(&b) {
                    succ[a].push(b);
                }
            }
        }
        // break cycles with a DFS: keep only tree / forward / cross edges
        let mut state = vec![0u8; n];
        let mut dag = vec![Vec::new(); n];
        fn dfs(u: usize, succ: &[Vec<usize>], state: &mut [u8], dag: &mut [Vec<usize>]) {
            state[u] = 1;
            for &v in &succ[u] {
                if state[v] == 1 {
                    continue; // back edge
                }
                dag[u].push(v);
                if state[v] == 0 {
                    dfs(v, succ, state, dag);
                }
            }
            state[u] = 2;
        }
        let mut order: Vec<usize> = (0..n).collect();
        let main_pos = |i: usize| if dir == LayoutDirection::LeftToRight { self.nodes[i].pos.x } else { self.nodes[i].pos.y };
        let cross_pos = |i: usize| if dir == LayoutDirection::LeftToRight { self.nodes[i].pos.y } else { self.nodes[i].pos.x };
        order.sort_by(|a, b| main_pos(*a).total_cmp(&main_pos(*b)));
        for &u in &order {
            if state[u] == 0 {
                dfs(u, &succ, &mut state, &mut dag);
            }
        }
        // longest-path ranks (Kahn)
        let mut indeg = vec![0usize; n];
        for d in &dag {
            for &v in d {
                indeg[v] += 1;
            }
        }
        let mut rank = vec![0usize; n];
        let mut queue: Vec<usize> = (0..n).filter(|&u| indeg[u] == 0).collect();
        let mut qi = 0;
        while qi < queue.len() {
            let u = queue[qi];
            qi += 1;
            for &v in &dag[u] {
                rank[v] = rank[v].max(rank[u] + 1);
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    queue.push(v);
                }
            }
        }
        let layers_n = rank.iter().copied().max().unwrap_or(0) + 1;
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); layers_n];
        let mut by_cross: Vec<usize> = (0..n).collect();
        by_cross.sort_by(|a, b| cross_pos(*a).total_cmp(&cross_pos(*b)));
        for u in by_cross {
            layers[rank[u]].push(u);
        }
        let mut pred = vec![Vec::new(); n];
        for (u, d) in dag.iter().enumerate() {
            for &v in d {
                pred[v].push(u);
            }
        }
        // barycenter sweeps
        let mut slot = vec![0.0f32; n];
        let renumber = |layers: &Vec<Vec<usize>>, slot: &mut Vec<f32>| {
            for l in layers {
                for (k, &u) in l.iter().enumerate() {
                    slot[u] = k as f32;
                }
            }
        };
        renumber(&layers, &mut slot);
        for sweep in 0..6 {
            let down = sweep % 2 == 0;
            let range: Vec<usize> = if down { (1..layers_n).collect() } else { (0..layers_n.saturating_sub(1)).rev().collect() };
            for l in range {
                let neigh = if down { &pred } else { &dag };
                let mut keyed: Vec<(f32, usize)> = layers[l]
                    .iter()
                    .map(|&u| {
                        let ns = &neigh[u];
                        let k = if ns.is_empty() { slot[u] } else { ns.iter().map(|&v| slot[v]).sum::<f32>() / ns.len() as f32 };
                        (k, u)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
                layers[l] = keyed.into_iter().map(|k| k.1).collect();
                renumber(&layers, &mut slot);
            }
        }
        // coordinates
        let size = |u: usize| {
            let s = self.nodes[u].size();
            if dir == LayoutDirection::LeftToRight { s } else { vec2(s.y, s.x) }
        };
        let mut main = 0.0;
        let mut cross = vec![0.0f32; n]; // center along the cross axis
        let mut main_of = vec![0.0f32; n];
        for (li, l) in layers.iter().enumerate() {
            let thick = l.iter().map(|&u| size(u).x).fold(0.0, f32::max);
            // desired centers from predecessors
            let total: f32 = l.iter().map(|&u| size(u).y).sum::<f32>() + gap.y * (l.len().saturating_sub(1)) as f32;
            let mut y = -total / 2.0;
            let mut want: Vec<f32> = Vec::with_capacity(l.len());
            for &u in l {
                let h = size(u).y;
                let base = y + h / 2.0;
                y += h + gap.y;
                let w = if li > 0 && !pred[u].is_empty() { pred[u].iter().map(|&v| cross[v]).sum::<f32>() / pred[u].len() as f32 } else { base };
                want.push(w);
            }
            // resolve overlaps keeping order: forward then backward pass, then re-center
            let mut pos = want.clone();
            for k in 1..l.len() {
                let min = pos[k - 1] + (size(l[k - 1]).y + size(l[k]).y) / 2.0 + gap.y;
                pos[k] = pos[k].max(min);
            }
            for k in (0..l.len().saturating_sub(1)).rev() {
                let max = pos[k + 1] - (size(l[k + 1]).y + size(l[k]).y) / 2.0 - gap.y;
                pos[k] = pos[k].min(max);
            }
            for (k, &u) in l.iter().enumerate() {
                cross[u] = pos[k];
                main_of[u] = main + (thick - size(u).x) / 2.0;
            }
            main += thick + gap.x;
        }
        (0..n)
            .map(|u| {
                let s = size(u);
                let p = pos2(main_of[u], cross[u] - s.y / 2.0);
                let p = if dir == LayoutDirection::LeftToRight { p } else { pos2(p.y, p.x) };
                (self.nodes[u].id, p)
            })
            .collect()
    }

    /// Apply [`Graph::layered_positions`].
    pub fn layout_layered(&mut self, dir: LayoutDirection, gap: Vec2) {
        for (id, p) in self.layered_positions(dir, gap) {
            if let Some(n) = self.node_mut(id) {
                n.pos = p;
            }
        }
    }
}

/// Route an edge between two port anchors (`da` / `db` point outward from each node).
/// `zoom` scales the curvature offsets when routing in screen space.
pub fn edge_path(kind: EdgeKind, a: Pos2, da: Vec2, b: Pos2, db: Vec2, zoom: f32) -> Vec<Pos2> {
    match kind {
        EdgeKind::Straight => vec![a, b],
        EdgeKind::Bezier => {
            let d = a.distance(b);
            let k = (d * 0.45).clamp(30.0 * zoom, 180.0 * zoom);
            // stronger pull when the target is "behind" the source
            let behind = (b - a).dot(da) < 0.0;
            let k = if behind { k.max(80.0 * zoom) } else { k };
            let n = ((d / 8.0) as usize).clamp(16, 64);
            glowkit_core::paint::cubic_bezier(a, a + da * k, b + db * k, b, n)
        }
        EdgeKind::SmoothStep => orthogonal_route(a, da, b, db, 20.0 * zoom, 10.0 * zoom),
        EdgeKind::Step => orthogonal_route(a, da, b, db, 20.0 * zoom, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> (Graph, Vec<NodeId>) {
        let mut g = Graph::new();
        let ids: Vec<NodeId> = (0..4).map(|i| g.add(Node::new(format!("n{i}"), pos2(i as f32 * 7.0, 0.0)).input(Port::new("in")).output(Port::new("out")))).collect();
        g.link((ids[0], 0), (ids[1], 0));
        g.link((ids[1], 0), (ids[2], 0));
        g.link((ids[0], 0), (ids[3], 0));
        (g, ids)
    }

    #[test]
    fn ids_are_unique() {
        let (g, ids) = chain();
        let mut all: Vec<u64> = ids.iter().map(|i| i.0).chain(g.edges.iter().map(|e| e.id.0)).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4 + 3);
    }

    #[test]
    fn validation() {
        let mut g = Graph::new();
        let a = g.add(Node::new("a", Pos2::ZERO).output(Port::typed("f", "float", Color32::RED)));
        let b = g.add(Node::new("b", Pos2::ZERO).input(Port::typed("c", "color", Color32::BLUE)).input(Port::new("any")));
        let out = PortRef { node: a, side: PortSide::Output, index: 0 };
        let typed_in = PortRef { node: b, side: PortSide::Input, index: 0 };
        let any_in = PortRef { node: b, side: PortSide::Input, index: 1 };
        assert_eq!(g.can_connect(out, typed_in), Err(ConnectError::TypeMismatch));
        assert!(g.connect(any_in, out).is_ok()); // reversed order is fine
        assert_eq!(g.connect(out, any_in), Err(ConnectError::Duplicate));
        assert_eq!(g.can_connect(out, PortRef { node: a, side: PortSide::Output, index: 0 }), Err(ConnectError::MissingPort));
    }

    #[test]
    fn single_input_replaces_edge() {
        let (mut g, ids) = chain();
        let extra = g.add(Node::new("x", Pos2::ZERO).output(Port::new("o")));
        g.link((extra, 0), (ids[1], 0));
        assert_eq!(g.edges.iter().filter(|e| e.to == ids[1]).count(), 1);
    }

    #[test]
    fn remove_drops_edges() {
        let (mut g, ids) = chain();
        g.remove_nodes(&[ids[1]]);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 1);
    }

    #[test]
    fn duplicate_copies_internal_edges() {
        let (mut g, ids) = chain();
        let new = g.duplicate(&[ids[0], ids[1]], vec2(10.0, 10.0));
        assert_eq!(new.len(), 2);
        assert!(g.edges.iter().any(|e| e.from == new[0] && e.to == new[1]));
        assert_eq!(g.node(new[0]).unwrap().pos, g.node(ids[0]).unwrap().pos + vec2(10.0, 10.0));
    }

    #[test]
    fn layered_layout_respects_edges() {
        let (mut g, ids) = chain();
        // add a cycle to make sure it terminates
        g.link((ids[2], 0), (ids[0], 0));
        g.layout_layered(LayoutDirection::LeftToRight, vec2(60.0, 30.0));
        let x = |i: usize| g.node(ids[i]).unwrap().pos.x;
        assert!(x(1) > x(0));
        assert!(x(2) > x(1));
        assert!(x(3) > x(0));
        // nodes in the same layer do not overlap
        let r1 = g.node(ids[1]).unwrap().rect();
        let r3 = g.node(ids[3]).unwrap().rect();
        assert!(!r1.intersects(r3));
    }

    #[test]
    fn hit_testing() {
        let (g, ids) = chain();
        let n = g.node(ids[3]).unwrap();
        let p = n.port_at(n.pos, PortSide::Output, 0);
        let hit = g.hit_port(p + vec2(2.0, 1.0), 6.0).unwrap();
        assert_eq!(hit.side, PortSide::Output);
        assert_eq!(g.hit_node(n.rect().center()), Some(ids[3]));
    }

    #[test]
    fn edge_paths_end_at_ports() {
        for k in EdgeKind::ALL {
            let p = edge_path(k, pos2(0.0, 0.0), Vec2::X, pos2(200.0, 80.0), -Vec2::X, 1.0);
            assert!(p[0].distance(pos2(0.0, 0.0)) < 1e-3);
            assert!(p.last().unwrap().distance(pos2(200.0, 80.0)) < 1e-3);
        }
    }
}
