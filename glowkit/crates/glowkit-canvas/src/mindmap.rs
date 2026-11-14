//! [`MindMapEditor`]: an auto-laid-out mind map with keyboard editing and animated branches.

use crate::common::{self, qfont};
use crate::freehand::paint_ribbon;
use crate::undo::History;
use crate::view::{Background, CanvasView, InfiniteCanvas};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};
use glowkit_core::{anim::approach, color::with_alpha, paint, Theme};
use std::collections::HashMap;

/// One topic in a [`MindMap`].
#[derive(Clone, Debug, PartialEq)]
pub struct MindNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub text: String,
    pub collapsed: bool,
    /// Branch color override (top-level topics).
    pub color: Option<Color32>,
}

/// A mind map: a forest of topics (the first parentless node is the root). Sibling order is
/// the order in `nodes`. Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MindMap {
    pub nodes: Vec<MindNode>,
}

impl MindMap {
    /// A map with a single root topic.
    pub fn new(root: impl Into<String>) -> Self {
        Self { nodes: vec![MindNode { id: 1, parent: None, text: root.into(), collapsed: false, color: None }] }
    }
    /// Root id.
    pub fn root(&self) -> Option<u64> {
        self.nodes.iter().find(|n| n.parent.is_none()).map(|n| n.id)
    }
    /// Topic by id.
    pub fn get(&self, id: u64) -> Option<&MindNode> {
        self.nodes.iter().find(|n| n.id == id)
    }
    fn get_mut(&mut self, id: u64) -> Option<&mut MindNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }
    /// Children ids in order.
    pub fn children(&self, id: u64) -> Vec<u64> {
        self.nodes.iter().filter(|n| n.parent == Some(id)).map(|n| n.id).collect()
    }
    /// Depth (root = 0).
    pub fn depth(&self, id: u64) -> usize {
        let mut d = 0;
        let mut cur = self.get(id).and_then(|n| n.parent);
        while let Some(p) = cur {
            d += 1;
            cur = self.get(p).and_then(|n| n.parent);
        }
        d
    }
    /// The top-level ancestor (child of the root) of `id`.
    pub fn branch(&self, id: u64) -> Option<u64> {
        let mut cur = id;
        loop {
            let p = self.get(cur)?.parent?;
            if self.get(p)?.parent.is_none() {
                return Some(cur);
            }
            cur = p;
        }
    }
    /// Add a child under `parent` and return its id.
    pub fn add_child(&mut self, parent: u64, text: impl Into<String>) -> u64 {
        let id = self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        // insert after the last descendant-sibling to keep order stable
        let pos = self.nodes.iter().rposition(|n| n.parent == Some(parent)).map(|i| i + 1).unwrap_or(self.nodes.len());
        self.nodes.insert(pos, MindNode { id, parent: Some(parent), text: text.into(), collapsed: false, color: None });
        if let Some(p) = self.get_mut(parent) {
            p.collapsed = false;
        }
        id
    }
    /// Add a sibling right after `of` (a child for the root).
    pub fn add_sibling(&mut self, of: u64, text: impl Into<String>) -> u64 {
        match self.get(of).and_then(|n| n.parent) {
            None => self.add_child(of, text),
            Some(p) => {
                let id = self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                let pos = self.nodes.iter().position(|n| n.id == of).map(|i| i + 1).unwrap_or(self.nodes.len());
                self.nodes
                    .insert(pos, MindNode { id, parent: Some(p), text: text.into(), collapsed: false, color: None });
                id
            }
        }
    }
    /// Remove a topic and its whole subtree (the root cannot be removed).
    pub fn remove(&mut self, id: u64) {
        if self.get(id).is_none_or(|n| n.parent.is_none()) {
            return;
        }
        let mut doomed = vec![id];
        let mut i = 0;
        while i < doomed.len() {
            let kids = self.children(doomed[i]);
            doomed.extend(kids);
            i += 1;
        }
        self.nodes.retain(|n| !doomed.contains(&n.id));
    }
    /// Is `id` hidden by a collapsed ancestor?
    pub fn hidden(&self, id: u64) -> bool {
        let mut cur = self.get(id).and_then(|n| n.parent);
        while let Some(p) = cur {
            let n = self.get(p);
            if n.is_some_and(|n| n.collapsed) {
                return true;
            }
            cur = n.and_then(|n| n.parent);
        }
        false
    }
    /// Number of descendants.
    pub fn descendants(&self, id: u64) -> usize {
        self.children(id).iter().map(|c| 1 + self.descendants(*c)).sum()
    }
}

/// How a [`MindMap`] is arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MindLayout {
    /// Root in the middle, branches balanced to the left and right.
    #[default]
    Balanced,
    /// Root on the left, everything grows to the right.
    Right,
    /// Branches radiate around the root.
    Radial,
}

/// Compute node centers for `map`. `size(id)` gives each topic's box size. Collapsed subtrees
/// collapse onto their parent. Returns center positions and which side each node grows to
/// (`+1` right, `-1` left, `0` root / radial).
pub fn mind_layout(map: &MindMap, layout: MindLayout, size: &dyn Fn(u64) -> Vec2) -> HashMap<u64, (Pos2, f32)> {
    let mut out = HashMap::new();
    let Some(root) = map.root() else { return out };
    let hgap = 56.0;
    let vgap = 14.0;
    fn subtree_h(map: &MindMap, id: u64, size: &dyn Fn(u64) -> Vec2, vgap: f32) -> f32 {
        let own = size(id).y + vgap;
        let n = map.get(id);
        if n.is_some_and(|n| n.collapsed) {
            return own;
        }
        let kids = map.children(id);
        let sum: f32 = kids.iter().map(|k| subtree_h(map, *k, size, vgap)).sum();
        own.max(sum)
    }
    #[allow(clippy::too_many_arguments)]
    fn place(
        map: &MindMap,
        id: u64,
        x: f32,
        y_top: f32,
        side: f32,
        size: &dyn Fn(u64) -> Vec2,
        hgap: f32,
        vgap: f32,
        out: &mut HashMap<u64, (Pos2, f32)>,
    ) {
        let h = subtree_h(map, id, size, vgap);
        let s = size(id);
        let c = pos2(x + side * s.x / 2.0, y_top + h / 2.0);
        out.insert(id, (c, side));
        let collapsed = map.get(id).is_some_and(|n| n.collapsed);
        let mut y = y_top + (h - map.children(id).iter().map(|k| subtree_h(map, *k, size, vgap)).sum::<f32>()) / 2.0;
        for k in map.children(id) {
            let kh = subtree_h(map, k, size, vgap);
            if collapsed {
                hide(map, k, c, out);
            } else {
                place(map, k, x + side * (s.x + hgap), y, side, size, hgap, vgap, out);
            }
            y += kh;
        }
    }
    fn hide(map: &MindMap, id: u64, at: Pos2, out: &mut HashMap<u64, (Pos2, f32)>) {
        out.insert(id, (at, 0.0));
        for k in map.children(id) {
            hide(map, k, at, out);
        }
    }
    let rs = size(root);
    out.insert(root, (Pos2::ZERO, 0.0));
    let kids = map.children(root);
    let root_collapsed = map.get(root).is_some_and(|n| n.collapsed);
    if root_collapsed {
        for k in kids {
            hide(map, k, Pos2::ZERO, &mut out);
        }
        return out;
    }
    match layout {
        MindLayout::Right | MindLayout::Balanced => {
            let (mut right, mut left): (Vec<u64>, Vec<u64>) = (Vec::new(), Vec::new());
            if layout == MindLayout::Right {
                right = kids;
            } else {
                let (mut hr, mut hl) = (0.0, 0.0);
                for k in kids {
                    let h = subtree_h(map, k, size, vgap);
                    if hr <= hl {
                        right.push(k);
                        hr += h;
                    } else {
                        left.push(k);
                        hl += h;
                    }
                }
            }
            for (list, side) in [(right, 1.0f32), (left, -1.0f32)] {
                let total: f32 = list.iter().map(|k| subtree_h(map, *k, size, vgap)).sum();
                let mut y = -total / 2.0;
                for k in list {
                    let kh = subtree_h(map, k, size, vgap);
                    place(map, k, side * (rs.x / 2.0 + hgap * 1.3), y, side, size, hgap, vgap, &mut out);
                    y += kh;
                }
            }
        }
        MindLayout::Radial => {
            fn leaves(map: &MindMap, id: u64) -> f32 {
                let kids = map.children(id);
                if kids.is_empty() || map.get(id).is_some_and(|n| n.collapsed) {
                    1.0
                } else {
                    kids.iter().map(|k| leaves(map, *k)).sum()
                }
            }
            #[allow(clippy::too_many_arguments)]
            fn radial(map: &MindMap, id: u64, a0: f32, a1: f32, depth: f32, out: &mut HashMap<u64, (Pos2, f32)>) {
                let collapsed = map.get(id).is_some_and(|n| n.collapsed);
                let me = out.get(&id).map(|p| p.0).unwrap_or(Pos2::ZERO);
                let kids = map.children(id);
                let total: f32 = kids.iter().map(|k| leaves(map, *k)).sum();
                let mut a = a0;
                for k in kids {
                    let span = (a1 - a0) * leaves(map, k) / total.max(1.0);
                    if collapsed {
                        hide(map, k, me, out);
                    } else {
                        let mid = a + span / 2.0;
                        let r = 200.0 + (depth - 1.0) * 175.0;
                        let p = pos2(mid.cos() * r * 1.15, mid.sin() * r * 0.62);
                        out.insert(k, (p, if mid.cos() >= 0.0 { 1.0 } else { -1.0 }));
                        radial(map, k, a, a + span, depth + 1.0, out);
                    }
                    a += span;
                }
            }
            radial(map, root, -std::f32::consts::FRAC_PI_2, std::f32::consts::PI * 1.5, 1.0, &mut out);
        }
    }
    out
}

fn font_for(depth: usize) -> f32 {
    match depth {
        0 => 18.0,
        1 => 14.5,
        _ => 13.0,
    }
}

fn measure_node(painter: &egui::Painter, map: &MindMap, nid: u64) -> Vec2 {
    let n = map.get(nid);
    let d = map.depth(nid);
    let text = n.map(|n| if n.text.is_empty() { " ".to_owned() } else { n.text.clone() }).unwrap_or_default();
    let g = painter.layout_no_wrap(text, egui::FontId::proportional(font_for(d)), Color32::WHITE);
    let pad = match d {
        0 => vec2(28.0, 18.0),
        1 => vec2(18.0, 12.0),
        _ => vec2(8.0, 8.0),
    };
    (g.size() + pad * 2.0).max(vec2(if d == 0 { 120.0 } else { 40.0 }, 0.0))
}

/// Caller-owned state of a [`MindMapEditor`].
#[derive(Clone, Debug)]
pub struct MindMapState {
    pub view: CanvasView,
    pub layout: MindLayout,
    pub selected: Option<u64>,
    pub editing: Option<u64>,
    pub history: History<MindMap>,
    anim: HashMap<u64, (Pos2, f32)>,
    focus_edit: bool,
    help: bool,
}

impl Default for MindMapState {
    fn default() -> Self {
        Self {
            view: CanvasView::with_limits(0.25, 2.5),
            layout: MindLayout::Balanced,
            selected: None,
            editing: None,
            history: History::new(100),
            anim: HashMap::new(),
            focus_edit: false,
            help: false,
        }
    }
}

/// A mind map (MindNode / XMind / Miro mind-map pattern) on an infinite canvas: automatic
/// balanced, right-growing or radial layout; Tab adds a child, Enter a sibling; Space folds a
/// branch with an animated collapse; curved, tapered, color-coded branches.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{MindMap, MindMapEditor, MindMapState};
/// # let mut state = MindMapState::default();
/// let mut map = MindMap::new("Product launch");
/// let root = map.root().unwrap();
/// map.add_child(root, "Marketing");
/// MindMapEditor::new(&mut map, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MindMapEditor<'a> {
    map: &'a mut MindMap,
    state: &'a mut MindMapState,
    height: f32,
}

impl<'a> MindMapEditor<'a> {
    /// An editor for `map`.
    pub fn new(map: &'a mut MindMap, state: &'a mut MindMapState) -> Self {
        Self { map, state, height: 480.0 }
    }
    /// Height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Show the editor. Returns `true` when the map changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let MindMapEditor { map, state: st, height } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_mindmap");
        let dt = glowkit_core::dt(ui);
        let mut changed = false;
        let mut view = std::mem::take(&mut st.view);
        InfiniteCanvas::new(&mut view).background(Background::Dots).height(height).id_salt("mindmap").show(ui, |cv| {
            let rect = cv.rect;
            let ui = &mut *cv.ui;
            let painter = cv.painter.clone();
            let root = map.root();
            let target = mind_layout(map, st.layout, &|k| measure_node(&painter, map, k));
            if cv.view.needs_fit {
                let b = target.iter().fold(Rect::NOTHING, |r, (k, (p, _))| {
                    r.union(Rect::from_center_size(*p, measure_node(&painter, map, *k)))
                });
                cv.view.fit_now(rect, b, 50.0);
            }
            // animate positions / visibility
            let mut moving = false;
            for (k, (p, _)) in &target {
                let vis = if map.hidden(*k) { 0.0 } else { 1.0 };
                let e = st.anim.entry(*k).or_insert_with(|| {
                    let parent_pos =
                        map.get(*k).and_then(|n| n.parent).and_then(|pp| target.get(&pp)).map(|x| x.0).unwrap_or(*p);
                    (parent_pos, 0.0)
                });
                let np = pos2(approach(e.0.x, p.x, 11.0, dt), approach(e.0.y, p.y, 11.0, dt));
                let na = approach(e.1, vis, 12.0, dt);
                if (np - *p).length() > 0.2 || (na - vis).abs() > 0.01 {
                    moving = true;
                    *e = (np, na);
                } else {
                    *e = (*p, vis);
                }
            }
            st.anim.retain(|k, _| target.contains_key(k));
            if moving {
                ui.ctx().request_repaint();
            }
            let zoom = cv.view.zoom;
            let (pan, rmin) = (cv.view.pan, rect.min);
            let ts = move |p: Pos2| rmin + pan + p.to_vec2() * zoom;
            let tw = move |p: Pos2| ((p - rmin - pan) / zoom).to_pos2();
            let pos_of = |k: u64| st.anim.get(&k).copied().unwrap_or((Pos2::ZERO, 0.0));
            let branch_color = |k: u64| -> Color32 {
                match map.branch(k) {
                    Some(b) => {
                        let idx = map.children(root.unwrap_or(0)).iter().position(|c| *c == b).unwrap_or(0);
                        map.get(b).and_then(|n| n.color).unwrap_or(t.chart[idx % t.chart.len()])
                    }
                    None => t.accent,
                }
            };

            // ---- branches
            for n in &map.nodes {
                let Some(par) = n.parent else { continue };
                let (cp, ca) = pos_of(n.id);
                if ca < 0.02 {
                    continue;
                }
                let (pp, _) = pos_of(par);
                let ps = measure_node(&painter, map, par);
                let cs = measure_node(&painter, map, n.id);
                let side = if (cp.x - pp.x).abs() < 1.0 { 1.0 } else { (cp.x - pp.x).signum() };
                let d_par = map.depth(par);
                let start = if d_par == 0 && st.layout == MindLayout::Radial {
                    pp
                } else {
                    pp + vec2(side * ps.x / 2.0, if d_par >= 2 { ps.y / 2.0 - 2.0 } else { 0.0 })
                };
                let depth = map.depth(n.id);
                let end = cp + vec2(-side * cs.x / 2.0, if depth >= 2 { cs.y / 2.0 - 2.0 } else { 0.0 });
                let (a, b) = (ts(start), ts(end));
                let dx = (b.x - a.x) * 0.5;
                let curve = if st.layout == MindLayout::Radial && d_par == 0 {
                    paint::cubic_bezier(a, a + (b - a) * 0.3, b - vec2(dx, 0.0) * 0.6, b, 24)
                } else {
                    paint::cubic_bezier(a, a + vec2(dx, 0.0), b - vec2(dx, 0.0), b, 24)
                };
                let w0 = match depth {
                    1 => 5.0,
                    2 => 2.6,
                    _ => 1.6,
                } * zoom.sqrt();
                let w1 = (w0 * 0.55).max(1.0);
                let col = with_alpha(branch_color(n.id), ca);
                let ribbon: Vec<(Pos2, f32)> = curve
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (*p, (w0 + (w1 - w0) * i as f32 / (curve.len() - 1) as f32) * 0.5))
                    .collect();
                paint_ribbon(&painter, &ribbon, col);
                if depth >= 2 {
                    // underline of the child topic
                    let u0 = ts(cp + vec2(-cs.x / 2.0, cs.y / 2.0 - 2.0));
                    let u1 = ts(cp + vec2(cs.x / 2.0, cs.y / 2.0 - 2.0));
                    painter.line_segment([u0, u1], Stroke::new(w1, col));
                }
            }

            // ---- nodes
            let mut clicked = None;
            let mut dbl = None;
            let mut toggle = None;
            for n in &map.nodes {
                let (p, a) = pos_of(n.id);
                if a < 0.02 {
                    continue;
                }
                let depth = map.depth(n.id);
                let s = measure_node(&painter, map, n.id);
                let r = Rect::from_center_size(ts(p), s * zoom * (0.85 + 0.15 * a));
                if !r.expand(30.0).intersects(rect) {
                    continue;
                }
                let resp = ui.interact(r, id.with(("n", n.id)), Sense::click());
                let hov = ui.ctx().animate_bool_with_time(id.with(("h", n.id)), resp.hovered(), 0.12);
                let sel = ui.ctx().animate_bool_with_time(id.with(("s", n.id)), st.selected == Some(n.id), 0.15);
                if resp.clicked() {
                    clicked = Some(n.id);
                }
                if resp.double_clicked() {
                    dbl = Some(n.id);
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                let mut pp = painter.clone();
                pp.set_opacity(a);
                let col = branch_color(n.id);
                let fs = font_for(depth) * zoom;
                match depth {
                    0 => {
                        paint::soft_shadow(
                            &pp,
                            r,
                            r.height() / 2.0,
                            20.0 * zoom,
                            vec2(0.0, 6.0 * zoom),
                            Color32::from_black_alpha(if t.dark { 140 } else { 40 }),
                        );
                        paint::gradient_rect(&pp, r, r.height() / 2.0, 20.0, &t.gradient);
                        if sel > 0.0 {
                            pp.rect_stroke(
                                r.expand(4.0),
                                CornerRadius::same((r.height() / 2.0 + 4.0) as u8),
                                Stroke::new(2.0, with_alpha(t.foreground, 0.7 * sel)),
                                StrokeKind::Outside,
                            );
                        }
                    }
                    1 => {
                        paint::soft_shadow(
                            &pp,
                            r,
                            10.0 * zoom,
                            12.0 * zoom,
                            vec2(0.0, 3.0 * zoom),
                            Color32::from_black_alpha(if t.dark { 90 } else { 22 }),
                        );
                        pp.rect(
                            r,
                            CornerRadius::same((10.0 * zoom) as u8),
                            t.card.lerp_to_gamma(col, 0.12 + 0.06 * hov),
                            Stroke::new(1.5 + sel, col.lerp_to_gamma(t.foreground, 0.2 * hov)),
                            StrokeKind::Inside,
                        );
                        if sel > 0.0 {
                            pp.rect_stroke(
                                r.expand(3.0),
                                CornerRadius::same((10.0 * zoom + 3.0) as u8),
                                Stroke::new(3.0, with_alpha(col, 0.35 * sel)),
                                StrokeKind::Outside,
                            );
                        }
                    }
                    _ => {
                        if hov > 0.0 || sel > 0.0 {
                            pp.rect_filled(
                                r,
                                CornerRadius::same((6.0 * zoom) as u8),
                                with_alpha(col, 0.08 * hov + 0.16 * sel),
                            );
                        }
                        if sel > 0.0 {
                            pp.rect_stroke(
                                r,
                                CornerRadius::same((6.0 * zoom) as u8),
                                Stroke::new(1.5, with_alpha(col, sel)),
                                StrokeKind::Inside,
                            );
                        }
                    }
                }
                if st.editing != Some(n.id) && fs > 4.0 {
                    let tc = if depth == 0 { Color32::WHITE } else { t.foreground };
                    let dy = if depth >= 2 { -2.0 * zoom } else { 0.0 };
                    pp.text(r.center() + vec2(0.0, dy), Align2::CENTER_CENTER, &n.text, qfont(fs), tc);
                }
                // fold toggle
                let kids = map.children(n.id);
                if depth == 0 && n.collapsed && !kids.is_empty() {
                    let c = pos2(r.right() + 14.0 * zoom.max(0.7), r.center().y);
                    let cnt = format!("+{}", map.descendants(n.id));
                    let g = pp.layout_no_wrap(cnt.clone(), qfont(11.0 * zoom.max(0.7)), Color32::WHITE);
                    let br = Rect::from_center_size(
                        c + vec2(g.size().x / 2.0, 0.0),
                        vec2(g.size().x + 12.0, 20.0 * zoom.max(0.7)),
                    );
                    let bresp = ui.interact(br, id.with(("fold", n.id)), Sense::click());
                    pp.rect(
                        br,
                        CornerRadius::same((br.height() / 2.0) as u8),
                        t.accent,
                        Stroke::new(1.5, t.background),
                        StrokeKind::Outside,
                    );
                    pp.text(br.center(), Align2::CENTER_CENTER, cnt, qfont(11.0 * zoom.max(0.7)), t.accent_foreground);
                    if bresp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        toggle = Some(n.id);
                    }
                }
                if !kids.is_empty() && depth > 0 {
                    let side =
                        pos_of(n.id).0.x - map.get(n.id).and_then(|x| x.parent).map(|pp| pos_of(pp).0.x).unwrap_or(0.0);
                    let side = if side >= 0.0 { 1.0 } else { -1.0 };
                    let dot_c = pos2(
                        if side > 0.0 { r.right() + 9.0 * zoom } else { r.left() - 9.0 * zoom },
                        if depth >= 2 { r.bottom() - 2.0 * zoom } else { r.center().y },
                    );
                    let dr = Rect::from_center_size(dot_c, Vec2::splat(16.0 * zoom.max(0.7)));
                    let dresp = ui.interact(dr, id.with(("fold", n.id)), Sense::click());
                    let dh = ui.ctx().animate_bool_with_time(id.with(("fh", n.id)), dresp.hovered(), 0.1);
                    if n.collapsed {
                        let cnt = map.descendants(n.id).to_string();
                        let rad = (8.0 + 1.5 * dh) * zoom.max(0.7);
                        pp.circle(dot_c, rad, col, Stroke::new(1.5, t.background));
                        pp.text(
                            dot_c,
                            Align2::CENTER_CENTER,
                            cnt,
                            qfont(9.5 * zoom.max(0.7)),
                            glowkit_core::color::contrast_text(col),
                        );
                    } else if hov > 0.0 || dh > 0.0 || st.selected == Some(n.id) {
                        let rad = (5.5 + 1.5 * dh) * zoom.max(0.7);
                        pp.circle(
                            dot_c,
                            rad,
                            t.background,
                            Stroke::new(1.5, with_alpha(col, (hov + dh).max(sel).min(1.0))),
                        );
                        pp.line_segment(
                            [dot_c - vec2(rad * 0.5, 0.0), dot_c + vec2(rad * 0.5, 0.0)],
                            Stroke::new(1.3, with_alpha(col, (hov + dh).max(sel).min(1.0))),
                        );
                    }
                    if dresp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        toggle = Some(n.id);
                    }
                }
            }
            if let Some(k) = toggle {
                st.history.record(map);
                if let Some(n) = map.get_mut(k) {
                    n.collapsed = !n.collapsed;
                }
                changed = true;
            } else if let Some(k) = clicked {
                st.selected = Some(k);
                if st.editing.is_some_and(|e| e != k) {
                    st.editing = None;
                }
            }
            if let Some(k) = dbl {
                st.editing = Some(k);
                st.focus_edit = true;
            }
            if cv.response.clicked() {
                st.selected = None;
                st.editing = None;
            }
            if cv.response.double_clicked() && map.root().is_some() {
                // double-click empty space: new topic on the root
                let _ = tw;
                st.history.record(map);
                let nid = map.add_child(map.root().unwrap_or(1), "New topic");
                st.selected = Some(nid);
                st.editing = Some(nid);
                st.focus_edit = true;
                changed = true;
            }

            // ---- editing overlay
            let mut edit_ended = false;
            if let Some(eid) = st.editing {
                if let Some(n) = map.get(eid).cloned() {
                    let depth = map.depth(eid);
                    let (p, _) = pos_of(eid);
                    let s = measure_node(&painter, map, eid).max(vec2(140.0, 0.0));
                    let r = Rect::from_center_size(ts(p), s * zoom);
                    let mut text = n.text.clone();
                    let col = if depth == 0 { Color32::WHITE } else { t.foreground };
                    let resp = ui.put(
                        r.shrink2(vec2(6.0, 2.0) * zoom),
                        TextEdit::singleline(&mut text)
                            .font(qfont(font_for(depth) * zoom))
                            .text_color(col)
                            .frame(egui::Frame::NONE)
                            .align(Align2::CENTER_CENTER),
                    );
                    if st.focus_edit {
                        resp.request_focus();
                        // select all on entry
                        if let Some(mut state) = TextEdit::load_state(ui.ctx(), resp.id) {
                            let range = egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(text.chars().count()),
                            );
                            state.cursor.set_char_range(Some(range));
                            state.store(ui.ctx(), resp.id);
                        }
                        st.focus_edit = false;
                    }
                    if text != n.text {
                        if let Some(m) = map.get_mut(eid) {
                            m.text = text;
                        }
                        changed = true;
                    }
                    let (enter, tab, esc) = ui.input(|i| {
                        (
                            i.key_pressed(egui::Key::Enter),
                            i.key_pressed(egui::Key::Tab),
                            i.key_pressed(egui::Key::Escape),
                        )
                    });
                    if enter || tab || esc || resp.lost_focus() {
                        st.editing = None;
                        edit_ended = true;
                        let empty = map.get(eid).is_some_and(|n| n.text.trim().is_empty() && n.parent.is_some());
                        if empty {
                            // an abandoned new topic disappears
                            let parent = map.get(eid).and_then(|n| n.parent);
                            map.remove(eid);
                            st.selected = parent;
                            changed = true;
                        } else if enter {
                            // Enter commits and starts a sibling (mind-map convention)
                            st.history.record(map);
                            let nid = map.add_sibling(eid, "");
                            st.selected = Some(nid);
                            st.editing = Some(nid);
                            st.focus_edit = true;
                            changed = true;
                        }
                        if tab && !empty {
                            st.history.record(map);
                            let nid = map.add_child(eid, "");
                            st.selected = Some(nid);
                            st.editing = Some(nid);
                            st.focus_edit = true;
                            changed = true;
                        }
                    }
                    if !resp.has_focus() && !st.focus_edit {
                        common::lock_focus(ui, resp.id);
                    }
                } else {
                    st.editing = None;
                }
            }

            // ---- keyboard
            let keys =
                ui.rect_contains_pointer(rect) && st.editing.is_none() && !edit_ended && !ui.ctx().text_edit_focused();
            if keys {
                use egui::Key;
                let sel = st.selected.or(root);
                if let Some(s) = sel {
                    let tab = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab));
                    if tab {
                        st.history.record(map);
                        let nid = map.add_child(s, "");
                        st.selected = Some(nid);
                        st.editing = Some(nid);
                        st.focus_edit = true;
                        changed = true;
                    }
                    if common::pressed(ui, Key::Enter) {
                        st.history.record(map);
                        let nid = map.add_sibling(s, "");
                        st.selected = Some(nid);
                        st.editing = Some(nid);
                        st.focus_edit = true;
                        changed = true;
                    }
                    if common::pressed(ui, Key::F2) {
                        st.editing = Some(s);
                        st.focus_edit = true;
                    }
                    if common::pressed(ui, Key::Space) && !map.children(s).is_empty() {
                        st.history.record(map);
                        if let Some(n) = map.get_mut(s) {
                            n.collapsed = !n.collapsed;
                        }
                        changed = true;
                    }
                    if (common::pressed(ui, Key::Delete) || common::pressed(ui, Key::Backspace)) && Some(s) != root {
                        st.history.record(map);
                        let parent = map.get(s).and_then(|n| n.parent);
                        map.remove(s);
                        st.selected = parent;
                        changed = true;
                    }
                    // arrow navigation in layout space
                    let dir = ui.input(|i| {
                        vec2(
                            (i.key_pressed(Key::ArrowRight) as i32 - i.key_pressed(Key::ArrowLeft) as i32) as f32,
                            (i.key_pressed(Key::ArrowDown) as i32 - i.key_pressed(Key::ArrowUp) as i32) as f32,
                        )
                    });
                    if dir != Vec2::ZERO {
                        let here = target.get(&s).map(|x| x.0).unwrap_or_default();
                        let best = target
                            .iter()
                            .filter(|(k, _)| **k != s && !map.hidden(**k))
                            .filter_map(|(k, (p, _))| {
                                let d = *p - here;
                                let along = d.dot(dir);
                                (along > 1.0).then(|| (along + (d - dir * along).length() * 2.0, *k))
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0));
                        if let Some((_, k)) = best {
                            st.selected = Some(k);
                        }
                    }
                }
                if common::pressed_cmd(ui, Key::Z, false) && st.history.undo(map) {
                    changed = true;
                }
                if (common::pressed_cmd(ui, Key::Z, true) || common::pressed_cmd(ui, Key::Y, false))
                    && st.history.redo(map)
                {
                    changed = true;
                }
                if common::pressed(ui, Key::F) {
                    let b = target.iter().filter(|(k, _)| !map.hidden(**k)).fold(Rect::NOTHING, |r, (k, (p, _))| {
                        r.union(Rect::from_center_size(*p, measure_node(&painter, map, *k)))
                    });
                    cv.view.fit(rect, b, 50.0);
                }
                if common::help_pressed(ui) {
                    st.help = !st.help;
                }
            }
            if st.selected.is_some() && ui.rect_contains_pointer(rect) {
                // keep Tab for us instead of egui focus navigation
                ui.memory_mut(|m| m.set_focus_lock_filter(id, egui::EventFilter { tab: true, ..Default::default() }));
            }

            // ---- layout switcher + hints
            let modes =
                [(MindLayout::Balanced, "Balanced"), (MindLayout::Right, "Right"), (MindLayout::Radial, "Radial")];
            let seg = Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(240.0, 32.0));
            common::panel(&painter, &t, seg, 9.0);
            let cur = modes.iter().position(|m| m.0 == st.layout).unwrap_or(0) as f32;
            let ind = glowkit_core::anim::follow(ui, id.with("seg"), cur, 16.0);
            let sw = (seg.width() - 8.0) / 3.0;
            painter.rect_filled(
                Rect::from_min_size(pos2(seg.left() + 4.0 + ind * sw, seg.top() + 4.0), vec2(sw, seg.height() - 8.0)),
                CornerRadius::same(6),
                with_alpha(t.accent, 0.18),
            );
            for (k, (m, label)) in modes.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(seg.left() + 4.0 + k as f32 * sw, seg.top() + 4.0),
                    vec2(sw, seg.height() - 8.0),
                );
                let resp = ui.interact(r, id.with(("mode", k)), Sense::click());
                painter.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    *label,
                    qfont(12.0),
                    if st.layout == *m { t.foreground } else { t.muted_foreground },
                );
                if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() && st.layout != *m {
                    st.layout = *m;
                    let b = mind_layout(map, *m, &|k| measure_node(&painter, map, k))
                        .iter()
                        .fold(Rect::NOTHING, |r, (k, (p, _))| {
                            r.union(Rect::from_center_size(*p, measure_node(&painter, map, *k)))
                        });
                    cv.view.fit(rect, b, 50.0);
                }
            }
            let hint_y = rect.bottom() - 22.0;
            let mut x = rect.left() + 14.0;
            for (k, label) in
                [("Tab", "child"), ("Enter", "sibling"), ("Space", "fold"), ("F2", "rename"), ("Del", "delete")]
            {
                x += common::keycap(&painter, &t, pos2(x, hint_y), k) + 6.0;
                let r = painter.text(pos2(x, hint_y), Align2::LEFT_CENTER, label, qfont(11.5), t.muted_foreground);
                x = r.right() + 14.0;
            }
            let _ = tw;
        });
        st.view = view;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MindMap {
        let mut m = MindMap::new("root");
        let a = m.add_child(1, "a");
        let b = m.add_child(1, "b");
        m.add_child(a, "a1");
        m.add_child(a, "a2");
        m.add_child(b, "b1");
        m
    }

    #[test]
    fn structure() {
        let mut m = sample();
        assert_eq!(m.children(1).len(), 2);
        assert_eq!(m.depth(4), 2);
        assert_eq!(m.branch(4), Some(2));
        let s = m.add_sibling(4, "a1.5");
        assert_eq!(m.get(s).unwrap().parent, Some(2));
        assert_eq!(m.children(2), vec![4, s, 5]);
        m.remove(2);
        assert!(m.get(4).is_none() && m.get(5).is_none());
        m.remove(1); // root survives
        assert!(m.root().is_some());
    }

    #[test]
    fn balanced_layout_has_two_sides_and_no_overlap() {
        let m = sample();
        let size = |_: u64| vec2(80.0, 30.0);
        let pos = mind_layout(&m, MindLayout::Balanced, &size);
        let a = pos[&2].0;
        let b = pos[&3].0;
        assert!(a.x * b.x < 0.0, "branches on opposite sides");
        let a1 = pos[&4].0;
        let a2 = pos[&5].0;
        assert!((a1.y - a2.y).abs() >= 30.0);
        assert!(a1.x.abs() > a.x.abs());
    }

    #[test]
    fn collapse_hides_descendants() {
        let mut m = sample();
        m.get_mut(2).unwrap().collapsed = true;
        assert!(m.hidden(4));
        assert!(!m.hidden(2));
        let pos = mind_layout(&m, MindLayout::Right, &|_| vec2(80.0, 30.0));
        assert_eq!(pos[&4].0, pos[&2].0);
        assert_eq!(m.descendants(2), 2);
    }

    #[test]
    fn radial_layout_spreads() {
        let m = sample();
        let pos = mind_layout(&m, MindLayout::Radial, &|_| vec2(80.0, 30.0));
        assert!(pos.values().all(|(p, _)| p.x.is_finite() && p.y.is_finite()));
        assert!(pos[&2].0.distance(Pos2::ZERO) > 100.0);
    }
}
