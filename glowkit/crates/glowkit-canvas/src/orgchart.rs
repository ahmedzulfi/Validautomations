//! [`OrgChartView`]: a top-down tree diagram (org chart) with tidy layout on a pan / zoom canvas.

use crate::common::{self, avatar, qfont};
use crate::view::{Background, CanvasView, ControlAction, Controls, InfiniteCanvas, MiniMap};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{anim::approach, color::with_alpha, paint, Theme};
use std::collections::HashMap;

/// A person (or any tree item) in an [`OrgChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct OrgPerson {
    pub id: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub title: String,
    /// Department / group color.
    pub color: Option<Color32>,
    pub collapsed: bool,
}

impl OrgPerson {
    /// A person reporting to `parent`.
    pub fn new(id: u64, parent: Option<u64>, name: impl Into<String>, title: impl Into<String>) -> Self {
        Self { id, parent, name: name.into(), title: title.into(), color: None, collapsed: false }
    }
    /// Builder: color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// Tree data for an org chart. Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OrgChart {
    pub people: Vec<OrgPerson>,
}

impl OrgChart {
    /// Direct reports of `id`.
    pub fn children(&self, id: u64) -> Vec<u64> {
        self.people.iter().filter(|p| p.parent == Some(id)).map(|p| p.id).collect()
    }
    /// Person by id.
    pub fn get(&self, id: u64) -> Option<&OrgPerson> {
        self.people.iter().find(|p| p.id == id)
    }
    /// Total number of people below `id`.
    pub fn reports(&self, id: u64) -> usize {
        self.children(id).iter().map(|c| 1 + self.reports(*c)).sum()
    }
    /// Chain of ids from `id` up to the root (inclusive).
    pub fn chain(&self, id: u64) -> Vec<u64> {
        let mut out = vec![id];
        let mut cur = self.get(id).and_then(|p| p.parent);
        while let Some(p) = cur {
            out.push(p);
            cur = self.get(p).and_then(|x| x.parent);
        }
        out
    }
}

/// Top-down tidy tree layout. Returns the top-left of every node's box (`size`); children of a
/// collapsed node are stacked onto their parent. When `compact` is set, a node whose children
/// are all leaves stacks them vertically (saves width on wide teams).
pub fn tree_layout(chart: &OrgChart, size: Vec2, gap: Vec2, compact: bool) -> HashMap<u64, Pos2> {
    let mut out = HashMap::new();
    let roots: Vec<u64> = chart.people.iter().filter(|p| p.parent.is_none()).map(|p| p.id).collect();
    fn stacked(chart: &OrgChart, id: u64, compact: bool) -> bool {
        let kids = chart.children(id);
        compact && kids.len() > 1 && kids.iter().all(|k| chart.children(*k).is_empty())
    }
    fn width(chart: &OrgChart, id: u64, size: Vec2, gap: Vec2, compact: bool) -> f32 {
        let kids = chart.children(id);
        if kids.is_empty() || chart.get(id).is_some_and(|p| p.collapsed) {
            return size.x;
        }
        if stacked(chart, id, compact) {
            return size.x + 28.0;
        }
        let w: f32 = kids.iter().map(|k| width(chart, *k, size, gap, compact)).sum::<f32>() + gap.x * (kids.len() - 1) as f32;
        w.max(size.x)
    }
    fn hide(chart: &OrgChart, id: u64, at: Pos2, out: &mut HashMap<u64, Pos2>) {
        for k in chart.children(id) {
            out.insert(k, at);
            hide(chart, k, at, out);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn place(chart: &OrgChart, id: u64, left: f32, y: f32, size: Vec2, gap: Vec2, compact: bool, out: &mut HashMap<u64, Pos2>) {
        let w = width(chart, id, size, gap, compact);
        let me = pos2(left + (w - size.x) / 2.0, y);
        let collapsed = chart.get(id).is_some_and(|p| p.collapsed);
        let kids = chart.children(id);
        if stacked(chart, id, compact) && !collapsed {
            let me = pos2(left, y);
            out.insert(id, me);
            for (i, k) in kids.iter().enumerate() {
                out.insert(*k, pos2(left + 28.0, y + (size.y + gap.y * 0.5) * (i + 1) as f32));
            }
            return;
        }
        out.insert(id, me);
        if collapsed {
            hide(chart, id, me, out);
            return;
        }
        let mut x = left;
        for k in kids {
            let kw = width(chart, k, size, gap, compact);
            place(chart, k, x, y + size.y + gap.y, size, gap, compact, out);
            x += kw + gap.x;
        }
    }
    let mut x = 0.0;
    for r in roots {
        let w = width(chart, r, size, gap, compact);
        place(chart, r, x, 0.0, size, gap, compact, &mut out);
        x += w + gap.x * 2.0;
    }
    out
}

/// Caller-owned state of an [`OrgChartView`].
#[derive(Clone, Debug)]
pub struct OrgChartState {
    pub view: CanvasView,
    pub selected: Option<u64>,
    /// Stack leaf teams vertically.
    pub compact: bool,
    anim: HashMap<u64, (Pos2, f32)>,
}

impl Default for OrgChartState {
    fn default() -> Self {
        Self { view: CanvasView::with_limits(0.2, 2.0), selected: None, compact: true, anim: HashMap::new() }
    }
}

/// An org chart / tree diagram on an infinite canvas: tidy top-down layout, person cards with
/// avatars, collapsible teams (with report counts), the selected person's chain of command
/// highlighted, compact stacking of leaf teams, controls and a minimap.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{OrgChart, OrgChartState, OrgChartView, OrgPerson};
/// # let mut state = OrgChartState::default();
/// let mut chart = OrgChart { people: vec![
///     OrgPerson::new(1, None, "Ada Park", "CEO"),
///     OrgPerson::new(2, Some(1), "Lin Chen", "CTO"),
/// ] };
/// OrgChartView::new(&mut chart, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OrgChartView<'a> {
    chart: &'a mut OrgChart,
    state: &'a mut OrgChartState,
    height: f32,
}

const CARD: Vec2 = vec2(212.0, 72.0);

impl<'a> OrgChartView<'a> {
    /// A view of `chart`.
    pub fn new(chart: &'a mut OrgChart, state: &'a mut OrgChartState) -> Self {
        Self { chart, state, height: 480.0 }
    }
    /// Height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Show the chart. Returns the id of a clicked person.
    pub fn show(self, ui: &mut Ui) -> Option<u64> {
        let OrgChartView { chart, state: st, height } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_org");
        let dt = glowkit_core::dt(ui);
        let mut clicked_out = None;
        let mut view = std::mem::take(&mut st.view);
        InfiniteCanvas::new(&mut view).background(Background::Dots).height(height).id_salt("org").show(ui, |cv| {
            let rect = cv.rect;
            let target = tree_layout(chart, CARD, vec2(28.0, 64.0), st.compact);
            let bounds = target.values().fold(Rect::NOTHING, |r, p| r.union(Rect::from_min_size(*p, CARD)));
            if cv.view.needs_fit {
                cv.view.fit_now(rect, bounds, 40.0);
            }
            let mut moving = false;
            for p in &chart.people {
                let tp = target.get(&p.id).copied().unwrap_or_default();
                let hidden = hidden(chart, p.id);
                let vis = if hidden { 0.0 } else { 1.0 };
                let e = st.anim.entry(p.id).or_insert((tp, vis));
                let np = pos2(approach(e.0.x, tp.x, 12.0, dt), approach(e.0.y, tp.y, 12.0, dt));
                let na = approach(e.1, vis, 14.0, dt);
                if (np - tp).length() > 0.2 || (na - vis).abs() > 0.01 {
                    moving = true;
                    *e = (np, na);
                } else {
                    *e = (tp, vis);
                }
            }
            if moving {
                ui_repaint(cv.ui);
            }
            let zoom = cv.view.zoom;
            let (pan, rmin) = (cv.view.pan, rect.min);
            let ts = move |p: Pos2| rmin + pan + p.to_vec2() * zoom;
            let ui = &mut *cv.ui;
            let painter = cv.painter.clone();
            let chain = st.selected.map(|s| chart.chain(s)).unwrap_or_default();
            let pos_of = |k: u64| st.anim.get(&k).copied().unwrap_or((Pos2::ZERO, 0.0));

            // connectors
            for p in &chart.people {
                let Some(par) = p.parent else { continue };
                let (cp, ca) = pos_of(p.id);
                if ca < 0.02 {
                    continue;
                }
                let (pp, _) = pos_of(par);
                let hl = chain.contains(&p.id) && chain.contains(&par);
                let col = if hl { t.accent } else { with_alpha(t.muted_foreground, 0.45 * ca) };
                let w = if hl { 2.0 } else { 1.3 };
                let stacked_parent = st.compact && {
                    let kids = chart.children(par);
                    kids.len() > 1 && kids.iter().all(|k| chart.children(*k).is_empty()) && !chart.get(par).is_some_and(|x| x.collapsed)
                };
                let pts = if stacked_parent {
                    let a = pp + vec2(14.0, CARD.y);
                    let b = cp + vec2(0.0, CARD.y / 2.0);
                    vec![ts(a), ts(pos2(a.x, b.y)), ts(b)]
                } else {
                    let a = pp + vec2(CARD.x / 2.0, CARD.y);
                    let b = cp + vec2(CARD.x / 2.0, 0.0);
                    let my = a.y + (b.y - a.y) * 0.5;
                    vec![ts(a), ts(pos2(a.x, my)), ts(pos2(b.x, my)), ts(b)]
                };
                let pts = common::round_corners(&pts, 8.0 * zoom);
                painter.add(Shape::line(pts, Stroke::new(w * zoom.sqrt(), col)));
            }

            // cards
            let mut toggle = None;
            let mut clicked = None;
            for p in &chart.people {
                let (wp, a) = pos_of(p.id);
                if a < 0.02 {
                    continue;
                }
                let r0 = Rect::from_min_size(ts(wp), CARD * zoom);
                let r = Rect::from_center_size(r0.center(), r0.size() * (0.8 + 0.2 * a));
                if !r.expand(30.0).intersects(rect) {
                    continue;
                }
                let resp = ui.interact(r, id.with(("p", p.id)), Sense::click());
                let hov = ui.ctx().animate_bool_with_time(id.with(("h", p.id)), resp.hovered(), 0.12);
                let sel = ui.ctx().animate_bool_with_time(id.with(("s", p.id)), st.selected == Some(p.id), 0.15);
                let in_chain = chain.contains(&p.id);
                if resp.clicked() {
                    clicked = Some(p.id);
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                let mut cp = painter.clone();
                cp.set_opacity(a);
                let col = p.color.unwrap_or(t.accent);
                let rr = r.translate(vec2(0.0, -2.0 * hov * zoom));
                paint::soft_shadow(&cp, rr, 10.0 * zoom, (12.0 + 8.0 * hov) * zoom, vec2(0.0, (3.0 + 3.0 * hov) * zoom), Color32::from_black_alpha(if t.dark { 110 } else { 26 }));
                let border = t.border.lerp_to_gamma(col, 0.5 * hov).lerp_to_gamma(t.accent, sel.max(if in_chain { 0.6 } else { 0.0 }));
                cp.rect(rr, CornerRadius::same((10.0 * zoom) as u8), t.card, Stroke::new(1.0 + sel, border), StrokeKind::Inside);
                // top color bar
                let bar = Rect::from_min_size(rr.min + vec2(14.0 * zoom, 0.0), vec2(rr.width() - 28.0 * zoom, 3.0 * zoom));
                cp.rect_filled(bar, CornerRadius::same(2), col);
                if zoom * 13.0 > 5.0 {
                    let initials: String = p.name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect();
                    avatar(&cp, pos2(rr.left() + 32.0 * zoom, rr.center().y), 18.0 * zoom, &initials, col, with_alpha(col, 0.4));
                    let tx = rr.left() + 60.0 * zoom;
                    let mw = rr.right() - tx - 10.0 * zoom;
                    cp.text(pos2(tx, rr.center().y - 8.0 * zoom), Align2::LEFT_CENTER, common::elide(&cp, &p.name, qfont(13.5 * zoom), mw), qfont(13.5 * zoom), t.foreground);
                    cp.text(pos2(tx, rr.center().y + 10.0 * zoom), Align2::LEFT_CENTER, common::elide(&cp, &p.title, qfont(11.0 * zoom), mw), qfont(11.0 * zoom), t.muted_foreground);
                }
                // collapse toggle
                let n = chart.reports(p.id);
                if n > 0 {
                    let c = pos2(rr.center().x, rr.bottom());
                    let label = if p.collapsed { format!("+{n}") } else { "−".to_owned() };
                    let g = cp.layout_no_wrap(label.clone(), qfont(10.5 * zoom.max(0.6)), t.foreground);
                    let br = Rect::from_center_size(c, vec2(g.size().x + 14.0 * zoom.max(0.6), 18.0 * zoom.max(0.6)));
                    let bresp = ui.interact(br, id.with(("t", p.id)), Sense::click());
                    let bh = ui.ctx().animate_bool_with_time(id.with(("th", p.id)), bresp.hovered(), 0.1);
                    let fill = if p.collapsed { col } else { t.card.lerp_to_gamma(col, 0.15 * bh) };
                    let fg = if p.collapsed { glowkit_core::color::contrast_text(col) } else { t.muted_foreground.lerp_to_gamma(t.foreground, bh) };
                    cp.rect(br, CornerRadius::same((br.height() / 2.0) as u8), fill, Stroke::new(1.0, if p.collapsed { col } else { t.border }), StrokeKind::Inside);
                    cp.text(br.center(), Align2::CENTER_CENTER, label, qfont(10.5 * zoom.max(0.6)), fg);
                    if bresp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(if p.collapsed { "Expand team" } else { "Collapse team" }).clicked() {
                        toggle = Some(p.id);
                    }
                }
            }
            if let Some(k) = toggle {
                if let Some(p) = chart.people.iter_mut().find(|p| p.id == k) {
                    p.collapsed = !p.collapsed;
                }
            } else if let Some(k) = clicked {
                st.selected = if st.selected == Some(k) { None } else { Some(k) };
                clicked_out = Some(k);
            }
            if cv.response.clicked() {
                st.selected = None;
            }
            // chrome
            let items: Vec<(Rect, Color32)> = chart.people.iter().filter(|p| !hidden(chart, p.id)).map(|p| (Rect::from_min_size(target.get(&p.id).copied().unwrap_or_default(), CARD), p.color.unwrap_or(t.accent))).collect();
            MiniMap::new(&mut *cv.view, rect).items(&items).show(ui);
            let compact = st.compact;
            match Controls::new(&mut *cv.view, rect).extra(crate::Icon::Layout, "Compact teams", compact).show(ui) {
                Some(ControlAction::Fit) => cv.view.fit(rect, bounds, 40.0),
                Some(ControlAction::Extra(0)) => {
                    st.compact = !st.compact;
                    let nb = tree_layout(chart, CARD, vec2(28.0, 64.0), st.compact).values().fold(Rect::NOTHING, |r, p| r.union(Rect::from_min_size(*p, CARD)));
                    cv.view.fit(rect, nb, 40.0);
                }
                _ => {}
            }
            if let Some(s) = st.selected.and_then(|s| chart.get(s)) {
                let txt = format!("{}  ·  {}  ·  {} report{}", s.name, s.title, chart.reports(s.id), if chart.reports(s.id) == 1 { "" } else { "s" });
                common::pill(&painter, pos2(rect.center().x, rect.top() + 24.0), &txt, qfont(12.0), t.foreground, t.popover, Stroke::new(1.0, t.border));
            }
        });
        st.view = view;
        clicked_out
    }
}

fn ui_repaint(ui: &Ui) {
    ui.ctx().request_repaint();
}

fn hidden(chart: &OrgChart, id: u64) -> bool {
    let mut cur = chart.get(id).and_then(|p| p.parent);
    while let Some(p) = cur {
        let n = chart.get(p);
        if n.is_some_and(|n| n.collapsed) {
            return true;
        }
        cur = n.and_then(|n| n.parent);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> OrgChart {
        OrgChart {
            people: vec![
                OrgPerson::new(1, None, "A", "ceo"),
                OrgPerson::new(2, Some(1), "B", "x"),
                OrgPerson::new(3, Some(1), "C", "x"),
                OrgPerson::new(4, Some(2), "D", "x"),
                OrgPerson::new(5, Some(2), "E", "x"),
                OrgPerson::new(6, Some(3), "F", "x"),
            ],
        }
    }

    #[test]
    fn tidy_layout_no_overlap_and_centered_parent() {
        let c = chart();
        let size = vec2(100.0, 40.0);
        let pos = tree_layout(&c, size, vec2(20.0, 30.0), false);
        let rects: Vec<Rect> = pos.values().map(|p| Rect::from_min_size(*p, size)).collect();
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                assert!(!rects[i].intersects(rects[j].shrink(0.5)), "overlap");
            }
        }
        // parent B centered over D and E
        let b = pos[&2].x + 50.0;
        let de = (pos[&4].x + pos[&5].x) / 2.0 + 50.0;
        assert!((b - de).abs() < 1e-3);
        assert!(pos[&4].y > pos[&2].y);
    }

    #[test]
    fn compact_stacks_leaves() {
        let c = chart();
        let pos = tree_layout(&c, vec2(100.0, 40.0), vec2(20.0, 30.0), true);
        assert_eq!(pos[&4].x, pos[&5].x);
        assert!(pos[&5].y > pos[&4].y);
    }

    #[test]
    fn chain_and_reports() {
        let c = chart();
        assert_eq!(c.chain(4), vec![4, 2, 1]);
        assert_eq!(c.reports(1), 5);
        let mut c2 = c.clone();
        c2.people[1].collapsed = true;
        assert!(hidden(&c2, 4));
        let pos = tree_layout(&c2, vec2(100.0, 40.0), vec2(20.0, 30.0), false);
        assert_eq!(pos[&4], pos[&2]);
    }
}
