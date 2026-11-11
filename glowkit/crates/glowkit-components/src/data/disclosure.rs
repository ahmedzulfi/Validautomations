use crate::inputs::kit::{self, UiIcon};
use egui::{pos2, vec2, Align, CornerRadius, CursorIcon, FontId, Id, Layout, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use std::f32::consts::PI;

type UiFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// Show `add` inside a region whose height animates open/closed (content fades and slides).
pub(crate) fn reveal<R>(ui: &mut Ui, id: Id, open: bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let raw = ui.ctx().animate_bool_with_time(id.with("reveal"), open, 0.26);
    let k = glowkit_core::ease(glowkit_core::Easing::InOutCubic, raw);
    if raw <= 0.0 {
        return None;
    }
    let w = ui.available_width();
    let top = ui.cursor().min;
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(top + vec2(0.0, -6.0 * (1.0 - k)), vec2(w, f32::INFINITY))).layout(Layout::top_down(Align::Min)));
    let full: f32 = glowkit_core::state(ui, id.with("reveal_h"), || 0.0);
    let clip_h = if raw >= 1.0 { f32::INFINITY } else { full * k };
    child.set_clip_rect(Rect::from_min_size(top, vec2(w, clip_h)).intersect(ui.clip_rect()));
    child.multiply_opacity(k);
    let r = add(&mut child);
    let h = child.min_rect().height().max(0.0);
    glowkit_core::set_state(ui, id.with("reveal_h"), h);
    ui.allocate_exact_size(vec2(w, if raw >= 1.0 { h } else { h * k }), Sense::hover());
    Some(r)
}

fn chevron(ui: &Ui, center: egui::Pos2, angle: f32, size: f32, color: egui::Color32) {
    let rot = |v: egui::Vec2| center + vec2(v.x * angle.cos() - v.y * angle.sin(), v.x * angle.sin() + v.y * angle.cos());
    let s = size / 2.0;
    ui.painter().add(Shape::line(vec![rot(vec2(-s, -s * 0.5)), rot(vec2(0.0, s * 0.5)), rot(vec2(s, -s * 0.5))], Stroke::new(1.5, color)));
}

/// Vertically stacked sections that expand with an animated height, a rotating chevron and
/// hairline separators. Single-open by default.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Accordion;
/// Accordion::new("faq")
///     .item_text("Is it accessible?", "Yes. It follows the WAI-ARIA design pattern.")
///     .item("Is it animated?", |ui| { ui.label("Yes, with springs."); })
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Accordion<'a> {
    id: Id,
    items: Vec<(String, UiFn<'a>)>,
    multiple: bool,
    default_open: Option<usize>,
    width: Option<f32>,
    card: bool,
}

impl<'a> Accordion<'a> {
    /// An accordion; `id_salt` keeps its open state.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self { id: Id::unique(id_salt), items: Vec::new(), multiple: false, default_open: Some(0), width: None, card: false }
    }
    /// Add a section with custom content.
    pub fn item(mut self, title: impl Into<String>, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.items.push((title.into(), Box::new(content)));
        self
    }
    /// Add a section whose content is a paragraph of muted text.
    pub fn item_text(self, title: impl Into<String>, body: impl Into<String>) -> Self {
        let body = body.into();
        self.item(title, move |ui| {
            let t = Theme::of(ui);
            ui.add(egui::Label::new(egui::RichText::new(body).size(13.5).color(t.muted_foreground)).wrap());
        })
    }
    /// Allow several sections open at once.
    pub fn multiple(mut self, m: bool) -> Self {
        self.multiple = m;
        self
    }
    /// Section open on first show (default the first one; `None` for all closed).
    pub fn default_open(mut self, i: Option<usize>) -> Self {
        self.default_open = i;
        self
    }
    /// Width (default: available, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Render inside a bordered card with padding.
    pub fn card(mut self, c: bool) -> Self {
        self.card = c;
        self
    }

    /// Show the accordion.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let id = ui.make_persistent_id(self.id);
        let n = self.items.len();
        let default = self.default_open;
        let mut open: Vec<bool> = glowkit_core::state(ui, id, || (0..n).map(|i| Some(i) == default).collect());
        open.resize(n, false);
        let bg = ui.painter().add(Shape::Noop);
        let pad = if self.card { 16.0 } else { 0.0 };
        let inner = ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(w);
            ui.spacing_mut().item_spacing.y = 0.0;
            let iw = w - pad * 2.0;
            for (i, (title, content)) in self.items.into_iter().enumerate() {
                let (row, resp) = ui.allocate_exact_size(vec2(w, 52.0), Sense::click());
                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                if resp.clicked() {
                    let new = !open[i];
                    if !self.multiple {
                        open.iter_mut().for_each(|o| *o = false);
                    }
                    open[i] = new;
                }
                let k = glowkit_core::ease(glowkit_core::Easing::InOutCubic, kit::anim(ui, id.with(("chev", i)), open[i], 0.26));
                let hov = kit::anim(ui, id.with(("h", i)), resp.hovered(), 0.12);
                let p = ui.painter();
                let tr = p.text(pos2(row.min.x + pad, row.center().y), egui::Align2::LEFT_CENTER, &title, FontId::proportional(14.5), t.foreground);
                if hov > 0.0 {
                    p.line_segment([pos2(tr.min.x, tr.max.y), pos2(tr.min.x + tr.width() * hov, tr.max.y)], Stroke::new(1.0, with_alpha(t.foreground, 0.6)));
                }
                chevron(ui, pos2(row.max.x - pad - 8.0, row.center().y), PI * k, 9.0, t.muted_foreground.lerp_to_gamma(t.foreground, hov));
                egui::Frame::NONE.inner_margin(egui::Margin { left: pad as i8, right: pad as i8, top: 0, bottom: 0 }).show(ui, |ui| {
                    ui.set_width(iw);
                    reveal(ui, id.with(("body", i)), open[i], |ui| {
                        ui.set_width(iw);
                        content(ui);
                        ui.add_space(16.0);
                    });
                });
                if i + 1 < n {
                    let y = kit::snap(ui, ui.cursor().min.y) + 0.5;
                    ui.painter().line_segment([pos2(row.min.x + pad, y), pos2(row.max.x - pad, y)], Stroke::new(1.0, t.border));
                }
            }
        });
        glowkit_core::set_state(ui, id, open);
        if self.card {
            ui.painter().set(bg, egui::epaint::RectShape::new(inner.response.rect, t.corner(), t.card, t.border_stroke(), StrokeKind::Inside));
        }
        inner.response
    }
}

impl Widget for Accordion<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}

/// A header row with a toggle button that reveals extra content with an animated height.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Collapsible;
/// Collapsible::new("@peduarte starred 3 repositories").show(ui, |ui| {
///     ui.label("@radix-ui/colors");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct Collapsible {
    title: String,
    default_open: bool,
    width: Option<f32>,
}

impl Collapsible {
    /// A collapsible with a header title.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), default_open: false, width: None }
    }
    /// Start expanded.
    pub fn default_open(mut self, o: bool) -> Self {
        self.default_open = o;
        self
    }
    /// Width (default: available, at most 420).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Show the header and, when open, `content`.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(420.0));
        let id = ui.make_persistent_id(("collapsible", &self.title));
        let mut open: bool = glowkit_core::state(ui, id, || self.default_open);
        let mut out = None;
        ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(w);
            let (row, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
            ui.painter().text(pos2(row.min.x + 4.0, row.center().y), egui::Align2::LEFT_CENTER, &self.title, FontId::proportional(14.0), t.foreground);
            let br = Rect::from_center_size(pos2(row.max.x - 16.0, row.center().y), vec2(32.0, 32.0));
            let b = ui.interact(br, id.with("btn"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let h = kit::anim(ui, id.with("h"), b.hovered(), 0.12);
            if h > 0.0 {
                ui.painter().rect_filled(br, CornerRadius::same(8), with_alpha(t.secondary, h));
            }
            let k = kit::anim(ui, id.with("k"), open, 0.25);
            let icon = if k > 0.5 { UiIcon::ChevronUp } else { UiIcon::ChevronsUpDown };
            icon.paint(ui.painter(), Rect::from_center_size(br.center(), vec2(15.0, 15.0)), t.foreground);
            if b.clicked() {
                open = !open;
            }
            out = reveal(ui, id.with("body"), open, content);
        });
        glowkit_core::set_state(ui, id, open);
        out
    }
}

/// A node in a [`TreeView`].
#[derive(Clone, Debug, Default)]
pub struct TreeNode {
    /// Display label.
    pub label: String,
    /// Icon (folders default to a folder icon, leaves to a file icon).
    pub icon: Option<UiIcon>,
    /// Child nodes.
    pub children: Vec<TreeNode>,
    /// Optional muted text on the right (size, count...).
    pub detail: Option<String>,
}

impl TreeNode {
    /// A leaf node.
    pub fn leaf(label: impl Into<String>) -> Self {
        Self { label: label.into(), ..Default::default() }
    }
    /// A branch node with children.
    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        Self { label: label.into(), children, ..Default::default() }
    }
    /// Set the icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Set the right-aligned detail text.
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }
}

/// An expandable tree (file explorer style) with indentation guides, rotating chevrons,
/// animated expand / collapse and a selected row. The selection is the `/`-joined label path.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut sel: Option<String> = None;
/// use glowkit_components::{TreeNode, TreeView};
/// let nodes = vec![TreeNode::branch("src", vec![TreeNode::leaf("main.rs"), TreeNode::leaf("lib.rs")])];
/// ui.add(TreeView::new(&nodes, &mut sel));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TreeView<'a> {
    nodes: &'a [TreeNode],
    selected: &'a mut Option<String>,
    open_depth: usize,
    width: Option<f32>,
    guides: bool,
}

impl<'a> TreeView<'a> {
    /// A tree over `nodes`, storing the selected path in `selected`.
    pub fn new(nodes: &'a [TreeNode], selected: &'a mut Option<String>) -> Self {
        Self { nodes, selected, open_depth: 1, width: None, guides: true }
    }
    /// Branches shallower than this start expanded (default 1: top level open).
    pub fn open_depth(mut self, d: usize) -> Self {
        self.open_depth = d;
        self
    }
    /// Width (default: available, at most 320).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Draw vertical indentation guides (default on).
    pub fn guides(mut self, g: bool) -> Self {
        self.guides = g;
        self
    }
}

const ROW_H: f32 = 30.0;
const INDENT: f32 = 18.0;

struct TreeCtx<'s> {
    id: Id,
    w: f32,
    open_depth: usize,
    guides: bool,
    selected: &'s mut Option<String>,
    changed: bool,
}

fn tree_rows(ui: &mut Ui, cx: &mut TreeCtx<'_>, nodes: &[TreeNode], depth: usize, prefix: &str) {
    let t = Theme::of(ui);
    for node in nodes {
        let path = if prefix.is_empty() { node.label.clone() } else { format!("{prefix}/{}", node.label) };
        let nid = cx.id.with(&path);
        let is_branch = !node.children.is_empty();
        let mut open: bool = glowkit_core::state(ui, nid, || depth < cx.open_depth);
        let (row, resp) = ui.allocate_exact_size(vec2(cx.w, ROW_H), Sense::click());
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            if is_branch {
                open = !open;
                glowkit_core::set_state(ui, nid, open);
            }
            *cx.selected = Some(path.clone());
            cx.changed = true;
        }
        let selected = cx.selected.as_deref() == Some(path.as_str());
        let hov = kit::anim(ui, nid.with("h"), resp.hovered(), 0.1);
        let sel = kit::anim(ui, nid.with("s"), selected, 0.14);
        let p = ui.painter();
        let r = CornerRadius::same(6);
        if sel > 0.0 {
            p.rect_filled(row, r, with_alpha(if t.dark { t.secondary } else { t.muted }, sel));
            p.rect_filled(Rect::from_min_size(row.min + vec2(0.0, 7.0), vec2(2.5, (ROW_H - 14.0) * sel)), CornerRadius::same(1), t.accent);
        } else if hov > 0.0 {
            p.rect_filled(row, r, with_alpha(if t.dark { t.secondary } else { t.muted }, 0.6 * hov));
        }
        let x0 = row.min.x + 8.0 + depth as f32 * INDENT;
        if cx.guides {
            for d in 0..depth {
                let gx = kit::snap(ui, row.min.x + 8.0 + d as f32 * INDENT + 7.0) + 0.5;
                p.line_segment([pos2(gx, row.min.y), pos2(gx, row.max.y)], Stroke::new(1.0, t.border));
            }
        }
        let cy = row.center().y;
        if is_branch {
            let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, kit::anim(ui, nid.with("chev"), open, 0.2));
            chevron(ui, pos2(x0 + 7.0, cy), -PI / 2.0 + PI / 2.0 * k, 8.0, t.muted_foreground);
        }
        let icon = node.icon.unwrap_or(if is_branch { if open { UiIcon::FolderOpen } else { UiIcon::Folder } } else { UiIcon::File });
        let icol = if is_branch { t.accent } else { t.muted_foreground };
        icon.paint(p, Rect::from_center_size(pos2(x0 + 26.0, cy), vec2(15.0, 15.0)), icol);
        p.text(pos2(x0 + 40.0, cy), egui::Align2::LEFT_CENTER, &node.label, FontId::proportional(13.5), if selected { t.foreground } else { t.foreground.lerp_to_gamma(t.muted_foreground, 0.15) });
        if let Some(d) = &node.detail {
            p.text(pos2(row.max.x - 10.0, cy), egui::Align2::RIGHT_CENTER, d, FontId::proportional(12.0), t.muted_foreground);
        }
        if is_branch {
            let children = &node.children;
            reveal(ui, nid.with("kids"), open, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                tree_rows(ui, cx, children, depth + 1, &path);
            });
        }
    }
}

impl Widget for TreeView<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let w = self.width.unwrap_or_else(|| ui.available_width().min(320.0));
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        let mut cx = TreeCtx { id, w, open_depth: self.open_depth, guides: self.guides, selected: self.selected, changed: false };
        let mut inner = ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            tree_rows(ui, &mut cx, self.nodes, 0, "");
        });
        if cx.changed {
            inner.response.mark_changed();
        }
        inner.response
    }
}
