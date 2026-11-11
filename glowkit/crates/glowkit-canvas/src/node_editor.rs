//! [`NodeEditor`]: a React-Flow style node graph editor on top of [`InfiniteCanvas`].

use crate::common::{self, arrow_head, dashed_polyline, icon_button, panel, point_along, polyline_length, qfont, Icon};
use crate::graph::*;
use crate::undo::History;
use crate::view::{Background, CanvasView, ControlAction, Controls, InfiniteCanvas, MiniMap};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{color::with_alpha, ease, paint, Easing, Theme};

#[derive(Clone, Debug, Default)]
enum Interaction {
    #[default]
    None,
    DragNodes { before: Box<Graph>, grab: Pos2, origins: Vec<(NodeId, Pos2)>, moved: bool },
    Connect { before: Box<Graph>, fixed: PortRef, cursor: Pos2, detached: bool },
    BoxSelect { start: Pos2, end: Pos2, base: Vec<NodeId> },
    Palette { index: usize },
}

/// Caller-owned state of a [`NodeEditor`]: view transform, selection, undo history, clipboard
/// and transient interaction state.
#[derive(Clone, Debug)]
pub struct NodeEditorState {
    /// Pan / zoom.
    pub view: CanvasView,
    /// Selected nodes.
    pub selected_nodes: Vec<NodeId>,
    /// Selected edges.
    pub selected_edges: Vec<EdgeId>,
    /// Undo / redo snapshots of the graph.
    pub history: History<Graph>,
    /// Snap dragged nodes to the grid.
    pub snap: bool,
    /// Grid spacing in world units.
    pub grid: f32,
    /// Show the minimap.
    pub minimap: bool,
    /// Default edge routing.
    pub edge_kind: EdgeKind,
    interaction: Interaction,
    clipboard: (Vec<Node>, Vec<Edge>),
    pastes: u32,
    layout_anim: Option<(f64, Vec<(NodeId, Pos2)>)>,
    spawned: Vec<(NodeId, f64)>,
    help: bool,
    focused: bool,
    toast: Option<(String, f64)>,
    last_input: f64,
    pending_fit: bool,
}

impl Default for NodeEditorState {
    fn default() -> Self {
        Self {
            view: CanvasView::with_limits(0.2, 2.5),
            selected_nodes: Vec::new(),
            selected_edges: Vec::new(),
            history: History::new(100),
            snap: true,
            grid: 20.0,
            minimap: true,
            edge_kind: EdgeKind::Bezier,
            interaction: Interaction::None,
            clipboard: (Vec::new(), Vec::new()),
            pastes: 0,
            layout_anim: None,
            spawned: Vec::new(),
            help: false,
            focused: false,
            toast: None,
            last_input: 0.0,
            pending_fit: false,
        }
    }
}

impl NodeEditorState {
    /// Run the layered auto-layout with a smooth animation (records an undo step).
    pub fn auto_layout(&mut self, graph: &mut Graph, dir: LayoutDirection, now: f64) {
        self.history.record(graph);
        let from: Vec<(NodeId, Pos2)> = graph.nodes.iter().map(|n| (n.id, n.pos)).collect();
        let gap = if dir == LayoutDirection::LeftToRight { vec2(80.0, 36.0) } else { vec2(56.0, 70.0) };
        graph.layout_layered(dir, gap);
        self.layout_anim = Some((now, from));
        self.pending_fit = true;
    }

    /// Select exactly these nodes.
    pub fn select(&mut self, ids: &[NodeId]) {
        self.selected_nodes = ids.to_vec();
        self.selected_edges.clear();
    }

    /// Show a short message at the bottom of the editor.
    pub fn toast(&mut self, msg: impl Into<String>, now: f64) {
        self.toast = Some((msg.into(), now));
    }

    /// Seconds since the user last interacted (useful to drive idle demos).
    pub fn idle_for(&self, now: f64) -> f64 {
        now - self.last_input
    }

    fn mark_spawned(&mut self, ids: &[NodeId], now: f64) {
        for id in ids {
            self.spawned.push((*id, now));
        }
    }
}

/// A node template offered by the editor's palette (drag it onto the canvas).
#[derive(Clone, Debug)]
pub struct PaletteItem {
    /// The node inserted on drop (its position is replaced).
    pub template: Node,
}

/// Result of [`NodeEditor::show`].
pub struct NodeEditorResponse {
    /// Canvas background response.
    pub response: Response,
    /// The graph was modified this frame.
    pub changed: bool,
    /// Edge created this frame, if any.
    pub connected: Option<EdgeId>,
}

type BodyFn<'a> = Box<dyn FnMut(&Painter, Rect, f32, &Node) + 'a>;

/// A node-graph editor (React Flow / xyflow pattern).
///
/// * Drag nodes (multi-selection drags together, snap-to-grid), shift-click to add to the
///   selection, shift-drag on the background for a selection box.
/// * Drag from a handle to connect, with a live preview, magnetic snapping and highlighted valid
///   targets; drag a connected input to detach / reconnect.
/// * Bezier / smooth-step / step / straight edges, animated edges, labels.
/// * Minimap, controls (zoom, fit, lock, layout, snap), node toolbar, undo/redo,
///   copy/paste/duplicate, delete, layered auto-layout, `?` shortcut sheet.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{Graph, Node, NodeEditor, NodeEditorState, Port};
/// # let (mut graph, mut state) = (Graph::new(), NodeEditorState::default());
/// let a = graph.add(Node::new("Trigger", egui::pos2(0.0, 0.0)).output(Port::new("out")));
/// let b = graph.add(Node::new("Action", egui::pos2(260.0, 0.0)).input(Port::new("in")));
/// graph.link((a, 0), (b, 0));
/// NodeEditor::new(&mut graph, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct NodeEditor<'a> {
    graph: &'a mut Graph,
    state: &'a mut NodeEditorState,
    height: f32,
    background: Background,
    minimap: bool,
    controls: bool,
    toolbar: bool,
    arrows: bool,
    palette: &'a [PaletteItem],
    body: Option<BodyFn<'a>>,
    layout_dir: LayoutDirection,
    shortcuts: bool,
    edge_kind: Option<EdgeKind>,
}

impl<'a> NodeEditor<'a> {
    /// An editor for `graph` with `state`.
    pub fn new(graph: &'a mut Graph, state: &'a mut NodeEditorState) -> Self {
        Self { graph, state, height: 480.0, background: Background::Dots, minimap: true, controls: true, toolbar: true, arrows: false, palette: &[], body: None, layout_dir: LayoutDirection::LeftToRight, shortcuts: true, edge_kind: None }
    }
    /// Canvas height (width fills the available space).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Background pattern.
    pub fn background(mut self, b: Background) -> Self {
        self.background = b;
        self
    }
    /// Allow the minimap (also toggled by the user via the controls).
    pub fn minimap(mut self, on: bool) -> Self {
        self.minimap = on;
        self
    }
    /// Show the controls panel.
    pub fn controls(mut self, on: bool) -> Self {
        self.controls = on;
        self
    }
    /// Show the floating toolbar above a single selected node.
    pub fn node_toolbar(mut self, on: bool) -> Self {
        self.toolbar = on;
        self
    }
    /// Draw arrow heads at edge targets.
    pub fn arrows(mut self, on: bool) -> Self {
        self.arrows = on;
        self
    }
    /// Node templates the user can drag in from a palette on the left.
    pub fn palette(mut self, items: &'a [PaletteItem]) -> Self {
        self.palette = items;
        self
    }
    /// Direction used by the auto-layout button / `L` key.
    pub fn layout_direction(mut self, d: LayoutDirection) -> Self {
        self.layout_dir = d;
        self
    }
    /// Default edge routing (stored in the state; edges may override it).
    pub fn edge_kind(mut self, k: EdgeKind) -> Self {
        self.edge_kind = Some(k);
        self
    }
    /// Show the `?` shortcut chip / sheet.
    pub fn shortcuts(mut self, on: bool) -> Self {
        self.shortcuts = on;
        self
    }
    /// Custom painter for the reserved body area of card nodes (`Node::custom_height`).
    /// Arguments: painter, screen rect of the area, zoom, node.
    pub fn node_body(mut self, f: impl FnMut(&Painter, Rect, f32, &Node) + 'a) -> Self {
        self.body = Some(Box::new(f));
        self
    }

    /// Show the editor.
    pub fn show(self, ui: &mut Ui) -> NodeEditorResponse {
        let NodeEditor { graph, state: st, height, background, minimap, controls, toolbar, arrows, palette, mut body, layout_dir, shortcuts, edge_kind } = self;
        if let Some(k) = edge_kind {
            st.edge_kind = k;
        }
        let t = Theme::of(ui);
        let now = ui.input(|i| i.time);
        let id = ui.scope_id().with("glowkit_node_editor");
        let shift = ui.input(|i| i.modifiers.shift);
        let mut changed = false;
        let mut connected = None;
        if ui.input(|i| i.pointer.any_down() || !i.events.is_empty() && i.events.iter().any(|e| matches!(e, egui::Event::Key { .. }))) {
            st.last_input = now;
        }
        let boxing = matches!(st.interaction, Interaction::BoxSelect { .. });
        let mut view = std::mem::take(&mut st.view);
        let resp = InfiniteCanvas::new(&mut view).background(background).gap(st.grid).height(height).pan_on_drag(!shift && !boxing).id_salt("node_editor").show(ui, |cv| {
            let rect = cv.rect;
            if cv.view.needs_fit && !graph.nodes.is_empty() {
                cv.view.fit_now(rect, graph.bounds(), 50.0);
            }
            if st.pending_fit && !graph.nodes.is_empty() {
                cv.view.fit(rect, graph.bounds(), 50.0);
                st.pending_fit = false;
            }
            let zoom = cv.view.zoom;
            let (pan, rmin) = (cv.view.pan, rect.min);
            let ts = move |p: Pos2| rmin + pan + p.to_vec2() * zoom;
            let tw = move |p: Pos2| ((p - rmin - pan) / zoom).to_pos2();
            let pointer = cv.ui.input(|i| i.pointer.hover_pos());
            let pw = pointer.map(tw);
            let ui = &mut *cv.ui;
            let painter = cv.painter.clone();
            if ui.input(|i| i.pointer.any_pressed()) {
                st.focused = pointer.is_some_and(|p| rect.contains(p));
            }
            let typing = ui.ctx().text_edit_focused();
            let time = now as f32;

            // ---------------------------------------------------------------- animations
            let layout_t = st.layout_anim.as_ref().map(|(t0, _)| ((now - t0) / 0.65) as f32);
            if layout_t.is_some_and(|k| k >= 1.0) {
                st.layout_anim = None;
            } else if layout_t.is_some() {
                ui.ctx().request_repaint();
            }
            st.spawned.retain(|(_, t0)| now - t0 < 0.5);
            let disp: Vec<Pos2> = graph.nodes.iter().map(|n| disp_of(n, &st.layout_anim, layout_t)).collect();
            if !st.spawned.is_empty() {
                ui.ctx().request_repaint();
            }
            let screen_rect = |i: usize| Rect::from_min_size(ts(disp[i]), graph.nodes[i].size() * zoom);
            let port_screen = |i: usize, side: PortSide, k: usize| ts(graph.nodes[i].port_at(disp[i], side, k));

            // ---------------------------------------------------------------- interaction: nodes & ports
            let interactive = !cv.space_pan;
            let mut node_resps: Vec<(usize, Response)> = Vec::new();
            for (i, n) in graph.nodes.iter().enumerate() {
                let r = screen_rect(i);
                if interactive && r.intersects(rect) {
                    node_resps.push((i, ui.interact(r.intersect(rect), id.with(("node", n.id.0)), Sense::click_and_drag())));
                }
            }
            let hs = (16.0 * zoom).clamp(12.0, 22.0);
            let mut port_resps: Vec<(PortRef, Pos2, Response)> = Vec::new();
            for (i, n) in graph.nodes.iter().enumerate() {
                for side in [PortSide::Input, PortSide::Output] {
                    for k in 0..n.ports(side).len() {
                        let p = port_screen(i, side, k);
                        if interactive && rect.contains(p) {
                            let r = Rect::from_center_size(p, Vec2::splat(hs));
                            let pr = PortRef { node: n.id, side, index: k };
                            port_resps.push((pr, p, ui.interact(r, id.with(("port", n.id.0, side == PortSide::Input, k)), Sense::drag())));
                        }
                    }
                }
            }
            let hovered_port = port_resps.iter().find(|p| p.2.hovered()).map(|p| p.0);
            let hovered_node = node_resps.iter().rev().find(|(_, r)| r.hovered()).map(|(i, _)| graph.nodes[*i].id);

            // edge geometry (screen space)
            let edge_paths: Vec<(EdgeId, Vec<Pos2>)> = graph
                .edges
                .iter()
                .filter_map(|e| {
                    let ai = graph.nodes.iter().position(|n| n.id == e.from)?;
                    let bi = graph.nodes.iter().position(|n| n.id == e.to)?;
                    let a = port_screen(ai, PortSide::Output, e.from_port);
                    let b = port_screen(bi, PortSide::Input, e.to_port);
                    let da = graph.nodes[ai].port_dir(PortSide::Output);
                    let db = graph.nodes[bi].port_dir(PortSide::Input);
                    Some((e.id, edge_path(e.kind.unwrap_or(st.edge_kind), a, da, b, db, zoom)))
                })
                .collect();
            let hovered_edge = if hovered_node.is_none() && hovered_port.is_none() && matches!(st.interaction, Interaction::None) {
                pointer.filter(|p| rect.contains(*p)).and_then(|p| {
                    edge_paths
                        .iter()
                        .map(|(eid, path)| (common::dist_to_polyline(p, path), *eid))
                        .filter(|(d, _)| *d < 7.0)
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                        .map(|x| x.1)
                })
            } else {
                None
            };

            // port drag → connect
            for (pr, _, r) in &port_resps {
                if r.drag_started_by(PointerButton::Primary) {
                    let before = Box::new(graph.clone());
                    let mut fixed = *pr;
                    let mut detached = false;
                    if pr.side == PortSide::Input && !graph.port(*pr).is_some_and(|p| p.multi) {
                        if let Some(e) = graph.edges.iter().find(|e| e.to == pr.node && e.to_port == pr.index).cloned() {
                            graph.remove_edges(&[e.id]);
                            fixed = PortRef { node: e.from, side: PortSide::Output, index: e.from_port };
                            detached = true;
                            changed = true;
                        }
                    }
                    st.interaction = Interaction::Connect { before, fixed, cursor: pw.unwrap_or_default(), detached };
                }
            }
            // node drag / click
            for (i, r) in &node_resps {
                let nid = graph.nodes[*i].id;
                if r.drag_started_by(PointerButton::Primary) && matches!(st.interaction, Interaction::None) {
                    if !st.selected_nodes.contains(&nid) {
                        if shift {
                            st.selected_nodes.push(nid);
                        } else {
                            st.select(&[nid]);
                        }
                    }
                    let origins = st.selected_nodes.iter().filter_map(|s| graph.node(*s).map(|n| (n.id, n.pos))).collect();
                    st.interaction = Interaction::DragNodes { before: Box::new(graph.clone()), grab: pw.unwrap_or_default(), origins, moved: false };
                }
                if r.clicked() {
                    if shift {
                        if let Some(k) = st.selected_nodes.iter().position(|s| *s == nid) {
                            st.selected_nodes.remove(k);
                        } else {
                            st.selected_nodes.push(nid);
                        }
                    } else {
                        st.select(&[nid]);
                    }
                }
                if r.hovered() && matches!(st.interaction, Interaction::None) && hovered_port.is_none() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
            }
            if let Some(&last) = st.selected_nodes.last() {
                if graph.nodes.last().map(|n| n.id) != Some(last) && matches!(st.interaction, Interaction::DragNodes { .. }) {
                    for s in st.selected_nodes.clone() {
                        graph.bring_to_front(s);
                    }
                }
            }
            // background click / box select
            let bg = &cv.response;
            if bg.clicked() {
                match hovered_edge {
                    Some(eid) if shift => {
                        if !st.selected_edges.contains(&eid) {
                            st.selected_edges.push(eid);
                        }
                    }
                    Some(eid) => {
                        st.selected_nodes.clear();
                        st.selected_edges = vec![eid];
                    }
                    None => {
                        st.selected_nodes.clear();
                        st.selected_edges.clear();
                    }
                }
            }
            if bg.drag_started_by(PointerButton::Primary) && shift && !cv.space_pan {
                let start = pw.unwrap_or_default();
                st.interaction = Interaction::BoxSelect { start, end: start, base: st.selected_nodes.clone() };
            }
            if hovered_edge.is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }

            // continue interactions
            let released = ui.input(|i| i.pointer.any_released() || !i.pointer.any_down());
            let mut finish = false;
            match &mut st.interaction {
                Interaction::DragNodes { before, grab, origins, moved } => {
                    if let Some(p) = pw {
                        let mut d = p - *grab;
                        if st.snap && !origins.is_empty() {
                            let o = origins[0].1;
                            d = vec2(common::snap(o.x + d.x, st.grid) - o.x, common::snap(o.y + d.y, st.grid) - o.y);
                        }
                        if d.length() > 0.5 || *moved {
                            for (nid, o) in origins.iter() {
                                if let Some(n) = graph.node_mut(*nid) {
                                    if n.pos != *o + d {
                                        n.pos = *o + d;
                                        *moved = true;
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    if released {
                        if *moved {
                            st.history.record(before);
                        }
                        finish = true;
                    }
                }
                Interaction::Connect { before, fixed, cursor, detached } => {
                    if let Some(p) = pw {
                        *cursor = p;
                    }
                    ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                    if released {
                        let target = best_target(graph, *fixed, *cursor, 30.0 / zoom);
                        if let Some(tg) = target {
                            if let Ok(eid) = graph.connect(*fixed, tg) {
                                st.history.record(before);
                                connected = Some(eid);
                                changed = true;
                            }
                        } else if *detached {
                            st.history.record(before);
                        }
                        finish = true;
                    }
                }
                Interaction::BoxSelect { start, end, base } => {
                    if let Some(p) = pw {
                        *end = p;
                    }
                    let sel = Rect::from_two_pos(*start, *end);
                    let mut s = base.clone();
                    for n in &graph.nodes {
                        if sel.intersects(n.rect()) && !s.contains(&n.id) {
                            s.push(n.id);
                        }
                    }
                    st.selected_nodes = s;
                    ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                    if released {
                        finish = true;
                    }
                }
                Interaction::Palette { index } => {
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    if released {
                        if let (Some(p), Some(item)) = (pointer, palette.get(*index)) {
                            if rect.contains(p) && p.x > rect.left() + 150.0 {
                                st.history.record(graph);
                                let mut n = item.template.clone();
                                let w = tw(p) - n.size() / 2.0;
                                n.pos = if st.snap { pos2(common::snap(w.x, st.grid), common::snap(w.y, st.grid)) } else { w };
                                let nid = graph.add(n);
                                st.spawned.push((nid, now));
                                st.selected_nodes = vec![nid];
                                st.selected_edges.clear();
                                changed = true;
                            }
                        }
                        finish = true;
                    }
                }
                Interaction::None => {}
            }
            if finish {
                st.interaction = Interaction::None;
            }

            // ---------------------------------------------------------------- keyboard
            let keys_active = (st.focused || ui.rect_contains_pointer(rect)) && !typing;
            if keys_active {
                use egui::Key;
                let (copy, cut, paste) = common::clipboard_events(ui);
                let del = common::pressed(ui, Key::Delete) || common::pressed(ui, Key::Backspace);
                if (del || cut) && (!st.selected_nodes.is_empty() || !st.selected_edges.is_empty()) {
                    if cut {
                        copy_selection(graph, st);
                    }
                    st.history.record(graph);
                    graph.remove_nodes(&st.selected_nodes);
                    graph.remove_edges(&st.selected_edges);
                    st.selected_nodes.clear();
                    st.selected_edges.clear();
                    changed = true;
                }
                if copy && !st.selected_nodes.is_empty() {
                    copy_selection(graph, st);
                    st.toast(format!("Copied {} node{}", st.clipboard.0.len(), if st.clipboard.0.len() == 1 { "" } else { "s" }), now);
                }
                if paste && !st.clipboard.0.is_empty() {
                    st.history.record(graph);
                    st.pastes += 1;
                    let off = Vec2::splat(st.grid * 2.0 * st.pastes as f32);
                    let (n, e) = st.clipboard.clone();
                    let ids = graph.paste(&n, &e, off);
                    st.mark_spawned(&ids, now);
                    st.select(&ids);
                    changed = true;
                }
                if common::pressed_cmd(ui, Key::D, false) && !st.selected_nodes.is_empty() {
                    st.history.record(graph);
                    let ids = graph.duplicate(&st.selected_nodes, Vec2::splat(st.grid * 2.0));
                    st.mark_spawned(&ids, now);
                    st.select(&ids);
                    changed = true;
                }
                if common::pressed_cmd(ui, Key::Z, false) && st.history.undo(graph) {
                    prune_selection(graph, st);
                    st.toast("Undo", now);
                    changed = true;
                }
                if (common::pressed_cmd(ui, Key::Z, true) || common::pressed_cmd(ui, Key::Y, false)) && st.history.redo(graph) {
                    prune_selection(graph, st);
                    st.toast("Redo", now);
                    changed = true;
                }
                if common::pressed_cmd(ui, Key::A, false) {
                    st.selected_nodes = graph.nodes.iter().map(|n| n.id).collect();
                }
                if common::pressed(ui, Key::Escape) {
                    st.selected_nodes.clear();
                    st.selected_edges.clear();
                    if let Interaction::Connect { before, .. } = std::mem::take(&mut st.interaction) {
                        *graph = *before;
                    }
                    st.help = false;
                }
                if common::pressed(ui, Key::F) {
                    cv.view.fit(rect, graph.bounds(), 50.0);
                }
                if common::pressed(ui, Key::L) && !graph.nodes.is_empty() {
                    st.auto_layout(graph, layout_dir, now);
                    st.toast("Auto layout", now);
                    changed = true;
                }
                if common::pressed(ui, Key::Plus) || common::pressed(ui, Key::Equals) {
                    cv.view.zoom_in(rect);
                }
                if common::pressed(ui, Key::Minus) {
                    cv.view.zoom_out(rect);
                }
                if shortcuts && common::help_pressed(ui) {
                    st.help = !st.help;
                }
                let arrows_step = ui.input(|i| {
                    vec2(
                        (i.key_pressed(Key::ArrowRight) as i32 - i.key_pressed(Key::ArrowLeft) as i32) as f32,
                        (i.key_pressed(Key::ArrowDown) as i32 - i.key_pressed(Key::ArrowUp) as i32) as f32,
                    )
                });
                if arrows_step != Vec2::ZERO && !st.selected_nodes.is_empty() {
                    st.history.record(graph);
                    let d = arrows_step * if shift { 1.0 } else { st.grid };
                    for s in &st.selected_nodes {
                        if let Some(n) = graph.node_mut(*s) {
                            n.pos += d;
                        }
                    }
                    changed = true;
                }
            }

            // ---------------------------------------------------------------- recompute geometry after edits
            let layout_t = st.layout_anim.as_ref().map(|(t0, _)| ((now - t0) / 0.65) as f32);
            let disp: Vec<Pos2> = graph.nodes.iter().map(|n| disp_of(n, &st.layout_anim, layout_t)).collect();
            let screen_rect = |i: usize| Rect::from_min_size(ts(disp[i]), graph.nodes[i].size() * zoom);
            let port_screen = |i: usize, side: PortSide, k: usize| ts(graph.nodes[i].port_at(disp[i], side, k));
            let spawned = st.spawned.clone();
            let spawn_k = |nid: NodeId| spawned.iter().find(|(i, _)| *i == nid).map(|(_, t0)| ((now - t0) / 0.35).clamp(0.0, 1.0) as f32);
            let edge_paths: Vec<(EdgeId, Vec<Pos2>)> = graph
                .edges
                .iter()
                .filter_map(|e| {
                    let ai = graph.nodes.iter().position(|n| n.id == e.from)?;
                    let bi = graph.nodes.iter().position(|n| n.id == e.to)?;
                    let a = port_screen(ai, PortSide::Output, e.from_port);
                    let b = port_screen(bi, PortSide::Input, e.to_port);
                    let da = graph.nodes[ai].port_dir(PortSide::Output);
                    let db = graph.nodes[bi].port_dir(PortSide::Input);
                    Some((e.id, edge_path(e.kind.unwrap_or(st.edge_kind), a, da, b, db, zoom)))
                })
                .collect();

            // ---------------------------------------------------------------- paint edges
            let neutral = with_alpha(t.muted_foreground, if t.dark { 0.75 } else { 0.85 });
            let ew = (2.0 * zoom).clamp(1.2, 3.5);
            for (eid, path) in edge_paths.iter() {
                let Some(e) = graph.edges.iter().find(|e| e.id == *eid) else { continue };
                let src_color = graph.port(PortRef { node: e.from, side: PortSide::Output, index: e.from_port }).and_then(|p| p.ty.color);
                let base = e.color.or(src_color).unwrap_or(neutral);
                let sel = st.selected_edges.contains(eid) || (st.selected_nodes.contains(&e.from) && st.selected_nodes.contains(&e.to));
                let hov = ui.ctx().animate_bool_with_time(id.with(("eh", eid.0)), hovered_edge == Some(*eid), 0.12);
                let sel_k = ui.ctx().animate_bool_with_time(id.with(("es", eid.0)), sel, 0.15);
                let col = base.lerp_to_gamma(t.accent, sel_k).lerp_to_gamma(t.foreground, hov * 0.35);
                if sel_k > 0.0 {
                    painter.add(Shape::line(path.clone(), Stroke::new(ew + 6.0 * sel_k, with_alpha(t.accent, 0.18 * sel_k))));
                }
                match e.anim {
                    EdgeAnim::None => {
                        painter.add(Shape::line(path.clone(), Stroke::new(ew + hov, col)));
                    }
                    EdgeAnim::Dashes => {
                        painter.add(Shape::line(path.clone(), Stroke::new(ew, with_alpha(col, 0.25))));
                        dashed_polyline(&painter, path, 7.0 * zoom, 6.0 * zoom, -time * 34.0 * zoom, Stroke::new(ew + hov, col));
                        ui.ctx().request_repaint();
                    }
                    EdgeAnim::Dots => {
                        painter.add(Shape::line(path.clone(), Stroke::new(ew + hov, with_alpha(col, 0.55))));
                        let len = polyline_length(path);
                        let spacing = (90.0 * zoom).max(30.0);
                        let count = (len / spacing).ceil().max(1.0) as usize;
                        for k in 0..count {
                            let d = (time * 70.0 * zoom + k as f32 * spacing).rem_euclid(len.max(1.0));
                            let (p, _) = point_along(path, d);
                            paint::glow(&painter, p, 9.0 * zoom, with_alpha(col, 0.55));
                            painter.circle_filled(p, (2.6 * zoom).max(1.5), col.lerp_to_gamma(Color32::WHITE, 0.4));
                        }
                        ui.ctx().request_repaint();
                    }
                }
                if arrows && path.len() >= 2 {
                    let n = path.len();
                    let tip = path[n - 1];
                    let mut k = n - 2;
                    while k > 0 && path[k].distance(tip) < 2.0 {
                        k -= 1;
                    }
                    arrow_head(&painter, tip, tip - path[k], (9.0 * zoom).max(5.0), col);
                }
                if !e.label.is_empty() && zoom > 0.35 {
                    let (mid, _) = point_along(path, polyline_length(path) / 2.0);
                    common::pill(&painter, mid, &e.label, qfont(10.5 * zoom), if sel { t.accent } else { t.muted_foreground }, t.popover, Stroke::new(1.0, if sel { with_alpha(t.accent, 0.6) } else { t.border }));
                }
            }

            // ---------------------------------------------------------------- paint nodes
            let connecting = match &st.interaction {
                Interaction::Connect { fixed, cursor, .. } => Some((*fixed, *cursor)),
                _ => None,
            };
            let snap_target = connecting.and_then(|(f, c)| best_target(graph, f, c, 30.0 / zoom));
            let dragging_ids: Vec<NodeId> = match &st.interaction {
                Interaction::DragNodes { origins, moved: true, .. } => origins.iter().map(|o| o.0).collect(),
                _ => Vec::new(),
            };
            for (i, n) in graph.nodes.iter().enumerate() {
                let mut r = screen_rect(i);
                if !r.expand(40.0).intersects(rect) {
                    continue;
                }
                let mut alpha = 1.0;
                if let Some(k) = spawn_k(n.id) {
                    let s = 0.85 + 0.15 * ease(Easing::OutBack, k);
                    r = Rect::from_center_size(r.center(), r.size() * s);
                    alpha = k;
                }
                let sel = ui.ctx().animate_bool_with_time(id.with(("ns", n.id.0)), st.selected_nodes.contains(&n.id), 0.15);
                let hov = ui.ctx().animate_bool_with_time(id.with(("nh", n.id.0)), hovered_node == Some(n.id), 0.12);
                let lift = ui.ctx().animate_bool_with_time(id.with(("nl", n.id.0)), dragging_ids.contains(&n.id), 0.12);
                let mut p = painter.clone();
                p.set_opacity(alpha);
                let accent = n.color.unwrap_or(t.accent);
                paint_node(&p, &t, n, r, zoom, sel, hov, lift, time);
                if n.custom_height > 0.0 && n.shape == NodeShape::Card {
                    if let Some(f) = body.as_mut() {
                        let h = n.custom_height * zoom;
                        let area = Rect::from_min_max(pos2(r.left() + 12.0 * zoom, r.bottom() - 8.0 * zoom - h), pos2(r.right() - 12.0 * zoom, r.bottom() - 8.0 * zoom));
                        f(&p.with_clip_rect(area.intersect(rect)), area, zoom, n);
                    }
                }
                // handles
                for side in [PortSide::Input, PortSide::Output] {
                    for (k, port) in n.ports(side).iter().enumerate() {
                        let pr = PortRef { node: n.id, side, index: k };
                        let c = port_screen(i, side, k);
                        let col = port.ty.color.unwrap_or(if n.shape == NodeShape::Card { t.muted_foreground } else { accent });
                        let is_connected = graph.edges.iter().any(|e| (side == PortSide::Output && e.from == n.id && e.from_port == k) || (side == PortSide::Input && e.to == n.id && e.to_port == k));
                        let h = ui.ctx().animate_bool_with_time(id.with(("ph", n.id.0, side == PortSide::Input, k)), hovered_port == Some(pr) || snap_target == Some(pr), 0.1);
                        let mut rad = (4.5 * zoom).max(3.0) * (1.0 + 0.45 * h);
                        let mut a = 1.0;
                        if let Some((fixed, _)) = connecting {
                            if pr == fixed {
                                rad *= 1.3;
                            } else if graph.can_connect(fixed, pr).is_ok() {
                                let pulse = 0.5 + 0.5 * (time * 6.0).sin();
                                paint::glow(&p, c, rad * 3.2, with_alpha(col.lerp_to_gamma(t.accent, 0.4), 0.35 + 0.25 * pulse));
                                p.circle_stroke(c, rad + 3.0 + 1.5 * pulse, Stroke::new(1.2, with_alpha(t.accent, 0.7)));
                            } else {
                                a = 0.3;
                            }
                        }
                        if h > 0.0 {
                            paint::glow(&p, c, rad * 3.0, with_alpha(col, 0.35 * h));
                        }
                        let fill = if is_connected || h > 0.5 { col } else { t.card };
                        p.circle(c, rad, with_alpha(fill, a), Stroke::new((1.6 * zoom).clamp(1.0, 2.2), with_alpha(col, a)));
                    }
                }
            }
            for (_, _, r) in &port_resps {
                if r.hovered() && matches!(st.interaction, Interaction::None) {
                    ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                }
            }

            // ---------------------------------------------------------------- overlays
            if let Some((fixed, cursor)) = connecting {
                if let Some(fi) = graph.nodes.iter().position(|n| n.id == fixed.node) {
                    let a = port_screen(fi, fixed.side, fixed.index);
                    let da = graph.nodes[fi].port_dir(fixed.side);
                    let (b, db, ok) = match snap_target.and_then(|tg| graph.nodes.iter().position(|n| n.id == tg.node).map(|ti| (tg, ti))) {
                        Some((tg, ti)) => (port_screen(ti, tg.side, tg.index), graph.nodes[ti].port_dir(tg.side), true),
                        None => (ts(cursor), -da, false),
                    };
                    let path = edge_path(st.edge_kind, a, da, b, db, zoom);
                    let col = if ok { t.accent } else { t.muted_foreground };
                    painter.add(Shape::line(path.clone(), Stroke::new(ew + 4.0, with_alpha(col, 0.15))));
                    dashed_polyline(&painter, &path, 6.0, 4.0, -time * 40.0, Stroke::new(ew, col));
                    ui.ctx().request_repaint();
                    if ok {
                        painter.circle_stroke(b, 9.0, Stroke::new(2.0, t.accent));
                    } else {
                        painter.circle(b, 4.0, t.background, Stroke::new(1.5, col));
                    }
                }
            }
            if let Interaction::BoxSelect { start, end, .. } = &st.interaction {
                let r = Rect::from_two_pos(ts(*start), ts(*end));
                painter.rect(r, CornerRadius::same(3), with_alpha(t.accent, 0.08), Stroke::new(1.0, with_alpha(t.accent, 0.8)), StrokeKind::Inside);
            }

            // node toolbar
            let single = if st.selected_nodes.len() == 1 && matches!(st.interaction, Interaction::None) { graph.nodes.iter().position(|n| n.id == st.selected_nodes[0]) } else { None };
            if toolbar {
                if let Some(i) = single {
                    let nid = graph.nodes[i].id;
                    let r = screen_rect(i);
                    let k = ui.ctx().animate_bool_with_time(id.with(("tb", nid.0)), true, 0.15);
                    let size = vec2(3.0 * 28.0 + 8.0, 34.0);
                    let tb = Rect::from_center_size(pos2(r.center().x, r.top() - 26.0 + (1.0 - k) * 6.0), size);
                    if rect.contains_rect(tb) {
                        panel(&painter, &t, tb, 9.0);
                        let mut action = None;
                        for (j, (icon, tip)) in [(Icon::Copy, "Duplicate (Ctrl+D)"), (Icon::Fill, "Cycle color"), (Icon::Trash, "Delete (Del)")].into_iter().enumerate() {
                            let br = Rect::from_min_size(tb.min + vec2(4.0 + j as f32 * 28.0, 3.0), Vec2::splat(28.0));
                            if icon_button(ui, id.with(("tbb", j)), br, icon, false, tip).clicked() {
                                action = Some(j);
                            }
                            if j == 1 {
                                // show the node color as a swatch over the button
                                let c = graph.nodes[i].color.unwrap_or(t.accent);
                                painter.circle(br.center(), 6.5, c, Stroke::new(2.0, t.popover));
                            }
                        }
                        match action {
                            Some(0) => {
                                st.history.record(graph);
                                let ids = graph.duplicate(&[nid], Vec2::splat(st.grid * 2.0));
                                st.mark_spawned(&ids, now);
                                st.select(&ids);
                                changed = true;
                            }
                            Some(1) => {
                                st.history.record(graph);
                                if let Some(n) = graph.node_mut(nid) {
                                    let cur = n.color.unwrap_or(t.accent);
                                    let k = t.chart.iter().position(|c| *c == cur).map(|k| (k + 1) % t.chart.len()).unwrap_or(0);
                                    n.color = Some(t.chart[k]);
                                }
                                changed = true;
                            }
                            Some(2) => {
                                st.history.record(graph);
                                graph.remove_nodes(&[nid]);
                                st.selected_nodes.clear();
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // palette
            if !palette.is_empty() {
                let row = 40.0;
                let pr = Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(136.0, 30.0 + row * palette.len() as f32));
                panel(&painter, &t, pr, 10.0);
                painter.text(pr.min + vec2(12.0, 16.0), Align2::LEFT_CENTER, "SHAPES", qfont(10.0), t.muted_foreground);
                for (k, item) in palette.iter().enumerate() {
                    let r = Rect::from_min_size(pr.min + vec2(6.0, 28.0 + k as f32 * row), vec2(pr.width() - 12.0, row - 4.0));
                    let resp = ui.interact(r, id.with(("pal", k)), Sense::click_and_drag());
                    let h = ui.ctx().animate_bool_with_time(id.with(("palh", k)), resp.hovered() || resp.dragged(), 0.12);
                    painter.rect_filled(r, CornerRadius::same(7), with_alpha(t.foreground, 0.06 * h));
                    let icon_r = Rect::from_center_size(pos2(r.left() + 20.0, r.center().y), vec2(28.0, 20.0));
                    let col = item.template.color.unwrap_or(t.accent);
                    let mut mini = item.template.clone();
                    mini.title.clear();
                    paint_shape_outline(&painter, &mini, icon_r, with_alpha(col, 0.18), Stroke::new(1.4, col));
                    painter.text(pos2(r.left() + 42.0, r.center().y), Align2::LEFT_CENTER, &item.template.title, qfont(12.0), t.foreground.gamma_multiply(0.8 + 0.2 * h));
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                    }
                    if resp.drag_started() {
                        st.interaction = Interaction::Palette { index: k };
                    }
                }
                if let (Interaction::Palette { index }, Some(p)) = (&st.interaction, pointer) {
                    if let Some(item) = palette.get(*index) {
                        let n = &item.template;
                        let r = Rect::from_center_size(p, n.size() * zoom);
                        let mut gp = painter.clone();
                        gp.set_opacity(0.7);
                        paint_node(&gp, &t, n, r, zoom, 0.0, 0.0, 1.0, time);
                    }
                }
            }

            // minimap & controls
            if minimap && st.minimap && rect.width() > 520.0 {
                let items: Vec<(Rect, Color32)> = graph.nodes.iter().map(|n| (n.rect(), n.color.unwrap_or(t.accent).gamma_multiply(if st.selected_nodes.contains(&n.id) { 1.0 } else { 0.55 }))).collect();
                MiniMap::new(&mut *cv.view, rect).items(&items).show(ui);
            }
            if controls {
                let act = Controls::new(&mut *cv.view, rect)
                    .horizontal(true)
                    .extra(Icon::Undo, "Undo (Ctrl+Z)", false)
                    .extra(Icon::Redo, "Redo (Ctrl+Shift+Z)", false)
                    .extra(Icon::Layout, "Auto layout (L)", false)
                    .extra(Icon::Magnet, "Snap to grid", st.snap)
                    .extra(Icon::Map, "Minimap", st.minimap)
                    .show(ui);
                match act {
                    Some(ControlAction::Fit) => cv.view.fit(rect, graph.bounds(), 50.0),
                    Some(ControlAction::Extra(0)) => {
                        if st.history.undo(graph) {
                            prune_selection(graph, st);
                            changed = true;
                        }
                    }
                    Some(ControlAction::Extra(1)) => {
                        if st.history.redo(graph) {
                            prune_selection(graph, st);
                            changed = true;
                        }
                    }
                    Some(ControlAction::Extra(2)) => {
                        st.auto_layout(graph, layout_dir, now);
                        changed = true;
                    }
                    Some(ControlAction::Extra(3)) => st.snap = !st.snap,
                    Some(ControlAction::Extra(4)) => st.minimap = !st.minimap,
                    _ => {}
                }
            }
            // toast
            if let Some((msg, t0)) = &st.toast {
                let age = (now - t0) as f32;
                if age < 1.6 {
                    let a = (age / 0.15).min(1.0) * ((1.6 - age) / 0.3).min(1.0);
                    let mut tp = painter.clone();
                    tp.set_opacity(a);
                    common::pill(&tp, pos2(rect.center().x, rect.bottom() - 30.0 - 6.0 * a), msg, qfont(12.0), t.foreground, t.popover, Stroke::new(1.0, t.border));
                    ui.ctx().request_repaint();
                } else {
                    st.toast = None;
                }
            }
            if shortcuts {
                let hr = Rect::from_min_size(pos2(rect.right() - 108.0, rect.top() + 12.0), vec2(96.0, 26.0));
                let hresp = ui.interact(hr, id.with("help"), Sense::click());
                common::shortcut_hint_chip(&painter, &t, hr.right_bottom());
                if hresp.clicked() {
                    st.help = !st.help;
                }
                let k = ui.ctx().animate_bool_with_time(id.with("helpk"), st.help, 0.18);
                common::shortcut_overlay(
                    &painter,
                    &t,
                    rect,
                    "Node editor shortcuts",
                    &[
                        ("Drag", "move node / pan"),
                        ("Shift+Drag", "box select"),
                        ("Shift+Click", "add to selection"),
                        ("Space+Drag", "pan anywhere"),
                        ("Ctrl+Wheel", "zoom to cursor"),
                        ("Del", "delete selection"),
                        ("Ctrl+C", "copy"),
                        ("Ctrl+V", "paste"),
                        ("Ctrl+D", "duplicate"),
                        ("Ctrl+Z", "undo"),
                        ("Ctrl+Shift+Z", "redo"),
                        ("Ctrl+A", "select all"),
                        ("F", "fit view"),
                        ("L", "auto layout"),
                        ("Arrows", "nudge selection"),
                        ("Esc", "clear / cancel"),
                    ],
                    k,
                );
            }
        });
        st.view = view;
        NodeEditorResponse { response: resp.response, changed, connected }
    }
}

fn disp_of(n: &Node, anim: &Option<(f64, Vec<(NodeId, Pos2)>)>, k: Option<f32>) -> Pos2 {
    if let (Some((_, from)), Some(k)) = (anim, k) {
        if let Some((_, p)) = from.iter().find(|(id, _)| *id == n.id) {
            return p.lerp(n.pos, ease(Easing::InOutCubic, k.min(1.0)));
        }
    }
    n.pos
}

fn copy_selection(graph: &Graph, st: &mut NodeEditorState) {
    let nodes: Vec<Node> = graph.nodes.iter().filter(|n| st.selected_nodes.contains(&n.id)).cloned().collect();
    let edges: Vec<Edge> = graph.edges.iter().filter(|e| st.selected_nodes.contains(&e.from) && st.selected_nodes.contains(&e.to)).cloned().collect();
    st.clipboard = (nodes, edges);
    st.pastes = 0;
}

fn prune_selection(graph: &Graph, st: &mut NodeEditorState) {
    st.selected_nodes.retain(|id| graph.node(*id).is_some());
    st.selected_edges.retain(|id| graph.edges.iter().any(|e| e.id == *id));
}

/// Closest port that `fixed` may connect to within `radius` (world units) of `p`.
fn best_target(graph: &Graph, fixed: PortRef, p: Pos2, radius: f32) -> Option<PortRef> {
    let mut best: Option<(f32, PortRef)> = None;
    for n in &graph.nodes {
        if n.id == fixed.node {
            continue;
        }
        let side = if fixed.side == PortSide::Output { PortSide::Input } else { PortSide::Output };
        for k in 0..n.ports(side).len() {
            let pr = PortRef { node: n.id, side, index: k };
            let d = n.port_at(n.pos, side, k).distance(p);
            let inside = n.rect().contains(p);
            let d = if inside { d * 0.5 } else { d };
            if (d <= radius || inside) && graph.can_connect(fixed, pr).is_ok() && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, pr));
            }
        }
    }
    best.map(|b| b.1)
}

/// Outline points of a flowchart shape inside `r` (screen).
pub fn shape_points(shape: NodeShape, r: Rect) -> Vec<Pos2> {
    match shape {
        NodeShape::Card | NodeShape::Process => paint::rounded_rect_points(r, (r.height() * 0.16).min(10.0), 4),
        NodeShape::Terminator => paint::rounded_rect_points(r, r.height() / 2.0, 8),
        NodeShape::Decision => vec![pos2(r.center().x, r.top()), pos2(r.right(), r.center().y), pos2(r.center().x, r.bottom()), pos2(r.left(), r.center().y)],
        NodeShape::Data => {
            let k = r.height() * 0.28;
            vec![pos2(r.left() + k, r.top()), pos2(r.right(), r.top()), pos2(r.right() - k, r.bottom()), pos2(r.left(), r.bottom())]
        }
        NodeShape::Document => {
            let wave = r.height() * 0.12;
            let mut v = vec![r.left_top(), r.right_top()];
            let n = 24;
            for i in 0..=n {
                let f = 1.0 - i as f32 / n as f32;
                let x = r.left() + r.width() * f;
                let y = r.bottom() - wave + (f * std::f32::consts::TAU).sin() * wave * 0.8;
                v.push(pos2(x, y));
            }
            v
        }
        NodeShape::Circle => paint::circle_points(r.center(), r.width().min(r.height()) / 2.0, 40),
    }
}

/// Fill a star-shaped polygon (fan from its centroid) and stroke its outline.
fn fill_outline(painter: &Painter, pts: &[Pos2], fill: Color32, stroke: Stroke) {
    if pts.len() < 3 {
        return;
    }
    let c = pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c.to_pos2(), fill);
    for p in pts {
        mesh.colored_vertex(*p, fill);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(mesh);
    painter.add(Shape::closed_line(pts.to_vec(), stroke));
}

fn paint_shape_outline(painter: &Painter, n: &Node, r: Rect, fill: Color32, stroke: Stroke) {
    fill_outline(painter, &shape_points(if n.shape == NodeShape::Card { NodeShape::Process } else { n.shape }, r), fill, stroke);
}

/// Paint one node (card or flowchart shape) into the screen rect `r`.
#[allow(clippy::too_many_arguments)]
pub fn paint_node(p: &Painter, t: &Theme, n: &Node, r: Rect, zoom: f32, sel: f32, hov: f32, lift: f32, time: f32) {
    let accent = n.color.unwrap_or(t.accent);
    let rad = 10.0 * zoom;
    let shadow = Color32::from_black_alpha(((if t.dark { 120.0 } else { 26.0 }) * (1.0 + lift * 0.6)) as u8);
    if n.shape != NodeShape::Card {
        let pts = shape_points(n.shape, r);
        paint::soft_shadow(p, Rect::from_center_size(r.center() + vec2(0.0, 5.0 * zoom), r.size() * if n.shape == NodeShape::Decision { 0.6 } else { 0.92 }), r.height() * 0.3, (14.0 + 12.0 * lift) * zoom, vec2(0.0, (3.0 + 5.0 * lift) * zoom), shadow);
        if sel > 0.0 {
            let ring = shape_points(n.shape, r.expand(4.0 * zoom));
            p.add(Shape::closed_line(ring, Stroke::new(2.0, with_alpha(t.accent, 0.8 * sel))));
        }
        let fill = t.card.lerp_to_gamma(accent, if t.dark { 0.14 } else { 0.08 });
        let stroke_col = accent.lerp_to_gamma(t.foreground, 0.15 * hov);
        fill_outline(p, &pts, fill, Stroke::new((1.6 * zoom).clamp(1.0, 2.5), stroke_col));
        if zoom * 13.0 > 5.0 {
            let font = qfont(12.5 * zoom);
            let max_w = r.width() * if n.shape == NodeShape::Decision { 0.62 } else { 0.86 };
            let txt = common::elide(p, &n.title, font.clone(), max_w);
            p.text(r.center(), Align2::CENTER_CENTER, txt, font, t.foreground);
            if n.shape == NodeShape::Decision && n.outputs.len() >= 2 {
                for (k, lbl) in [(0usize, "yes"), (1, "no")] {
                    let a = n.port_at(Pos2::ZERO, PortSide::Output, k).to_vec2() * zoom;
                    p.text(r.min + a + vec2(if k == 0 { -8.0 } else { 8.0 } * zoom, 10.0 * zoom), if k == 0 { Align2::RIGHT_TOP } else { Align2::LEFT_TOP }, lbl, qfont(10.0 * zoom), t.muted_foreground);
                }
            }
        }
        return;
    }
    // ---- card
    paint::soft_shadow(p, r.translate(vec2(0.0, 2.0 * zoom)), rad, (16.0 + 14.0 * lift) * zoom, vec2(0.0, (4.0 + 6.0 * lift) * zoom), shadow);
    if sel > 0.0 {
        p.rect_stroke(r.expand(3.0 * zoom), CornerRadius::same((rad + 3.0 * zoom) as u8), Stroke::new(5.0 * zoom, with_alpha(t.accent, 0.16 * sel)), StrokeKind::Outside);
    }
    let border = t.border.lerp_to_gamma(t.muted_foreground, 0.35 * hov).lerp_to_gamma(t.accent, sel);
    p.rect(r, CornerRadius::same(rad as u8), t.card, Stroke::new(if sel > 0.5 { 1.6 } else { 1.0 }, border), StrokeKind::Inside);
    let header = Rect::from_min_size(r.min, vec2(r.width(), HEADER * zoom));
    let hr = (rad as u8).saturating_sub(1);
    p.rect_filled(header.shrink(1.0), CornerRadius { nw: hr, ne: hr, sw: 0, se: 0 }, with_alpha(accent, if t.dark { 0.09 } else { 0.06 }));
    p.line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(1.0, t.border));
    let text_ok = zoom * 13.0 > 5.5;
    // icon badge
    let badge = Rect::from_center_size(pos2(r.left() + 24.0 * zoom, header.center().y), Vec2::splat(26.0 * zoom));
    p.rect_filled(badge, CornerRadius::same((7.0 * zoom) as u8), with_alpha(accent, 0.2));
    p.rect_stroke(badge, CornerRadius::same((7.0 * zoom) as u8), Stroke::new(1.0, with_alpha(accent, 0.45)), StrokeKind::Inside);
    if text_ok {
        let glyph = if n.icon.is_empty() { n.title.chars().next().map(|c| c.to_string()).unwrap_or_default() } else { n.icon.clone() };
        p.text(badge.center(), Align2::CENTER_CENTER, glyph, qfont(13.0 * zoom), accent.lerp_to_gamma(t.foreground, if t.dark { 0.25 } else { 0.0 }));
    }
    let tx = r.left() + 46.0 * zoom;
    let max_w = r.right() - tx - 30.0 * zoom;
    if text_ok {
        let tf = qfont(13.5 * zoom);
        if n.subtitle.is_empty() {
            p.text(pos2(tx, header.center().y), Align2::LEFT_CENTER, common::elide(p, &n.title, tf.clone(), max_w), tf, t.foreground);
        } else {
            p.text(pos2(tx, header.center().y - 7.5 * zoom), Align2::LEFT_CENTER, common::elide(p, &n.title, tf.clone(), max_w), tf, t.foreground);
            let sf = qfont(10.5 * zoom);
            p.text(pos2(tx, header.center().y + 8.5 * zoom), Align2::LEFT_CENTER, common::elide(p, &n.subtitle, sf.clone(), max_w), sf, t.muted_foreground);
        }
    } else {
        // level-of-detail placeholder bars
        p.rect_filled(Rect::from_min_size(pos2(tx, header.center().y - 3.0 * zoom), vec2(max_w * 0.7, 6.0 * zoom)), CornerRadius::same(2), with_alpha(t.foreground, 0.25));
    }
    // status
    let sc = pos2(r.right() - 16.0 * zoom, header.center().y);
    match n.status {
        NodeStatus::None => {}
        NodeStatus::Running => {
            let a0 = time * 5.0;
            paint::arc(p, sc, 6.0 * zoom, a0, a0 + 4.2, (2.0 * zoom).max(1.0), t.info);
            p.circle_stroke(sc, 6.0 * zoom, Stroke::new((2.0 * zoom).max(1.0), with_alpha(t.info, 0.2)));
        }
        NodeStatus::Done => {
            p.circle_filled(sc, 7.0 * zoom, with_alpha(t.success, 0.2));
            common::paint_icon(p, Rect::from_center_size(sc, Vec2::splat(10.0 * zoom)), Icon::Check, t.success);
        }
        NodeStatus::Error => {
            p.circle_filled(sc, 7.0 * zoom, with_alpha(t.destructive, 0.2));
            p.text(sc, Align2::CENTER_CENTER, "!", qfont(11.0 * zoom), t.destructive);
        }
    }
    if !text_ok {
        return;
    }
    // port labels
    let lf = qfont(11.5 * zoom);
    if n.layout == PortLayout::Horizontal {
        for (k, port) in n.inputs.iter().enumerate() {
            let y = r.top() + n.port_offset(PortSide::Input, k).y * zoom;
            if !port.label.is_empty() {
                p.text(pos2(r.left() + 14.0 * zoom, y), Align2::LEFT_CENTER, &port.label, lf.clone(), t.foreground.gamma_multiply(0.85));
            }
        }
        for (k, port) in n.outputs.iter().enumerate() {
            let y = r.top() + n.port_offset(PortSide::Output, k).y * zoom;
            if !port.label.is_empty() {
                p.text(pos2(r.right() - 14.0 * zoom, y), Align2::RIGHT_CENTER, &port.label, lf.clone(), t.foreground.gamma_multiply(0.85));
            }
            if !port.ty.name.is_empty() {
                let w = p.layout_no_wrap(port.label.clone(), lf.clone(), Color32::WHITE).size().x;
                p.text(pos2(r.right() - 20.0 * zoom - w, y), Align2::RIGHT_CENTER, &port.ty.name, qfont(9.5 * zoom), with_alpha(port.ty.color.unwrap_or(t.muted_foreground), 0.8));
            }
        }
    }
    let rows = if n.layout == PortLayout::Horizontal { n.inputs.len().max(n.outputs.len()) } else { 0 };
    let mut y = r.top() + (HEADER + 8.0 + rows as f32 * ROW) * zoom + LINE * 0.5 * zoom;
    let bf = qfont(11.0 * zoom);
    for line in &n.body {
        p.text(pos2(r.left() + 14.0 * zoom, y), Align2::LEFT_CENTER, common::elide(p, line, bf.clone(), r.width() - 28.0 * zoom), bf.clone(), t.muted_foreground);
        y += LINE * zoom;
    }
}
