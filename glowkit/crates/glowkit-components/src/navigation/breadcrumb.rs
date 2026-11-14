use crate::overlays::kit;
use crate::overlays::{DropdownMenu, LineIcon, MenuEntry};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Separator drawn between [`Breadcrumb`] items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BreadcrumbSeparator {
    /// A small chevron (default).
    #[default]
    Chevron,
    /// A forward slash.
    Slash,
    /// A dot.
    Dot,
}

/// A breadcrumb trail ("Home › Components › Breadcrumb"). Items get an animated underline on
/// hover, the last item is the current page, and long trails collapse their middle into an
/// "…" button that opens a dropdown of the hidden items.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Breadcrumb;
/// if let Some(i) = Breadcrumb::new(&["Home", "Docs", "Components", "Breadcrumb"]).max_items(3).show(ui) {
///     println!("navigate to item {i}");
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Breadcrumb {
    items: Vec<String>,
    separator: BreadcrumbSeparator,
    max_items: usize,
    home_icon: bool,
    size: f32,
}

impl Breadcrumb {
    /// A trail of items, first to last (the last is the current page).
    pub fn new(items: &[&str]) -> Self {
        Self {
            items: items.iter().map(|s| s.to_string()).collect(),
            separator: BreadcrumbSeparator::Chevron,
            max_items: usize::MAX,
            home_icon: false,
            size: 14.0,
        }
    }
    /// Separator style.
    pub fn separator(mut self, s: BreadcrumbSeparator) -> Self {
        self.separator = s;
        self
    }
    /// Collapse the middle into "…" when there are more than `n` items (min 2).
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n.max(2);
        self
    }
    /// Show a home icon before the first item.
    pub fn home_icon(mut self, yes: bool) -> Self {
        self.home_icon = yes;
        self
    }
    /// Font size (default 14).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Show the trail. Returns the index of a clicked (non-current) item.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("glowkit_breadcrumb", &self.items));
        let n = self.items.len();
        if n == 0 {
            return None;
        }
        // which items are visible: first, …, last (max_items - 1)
        let mut shown: Vec<Option<usize>> = (0..n).map(Some).collect();
        let mut hidden: Vec<usize> = Vec::new();
        if n > self.max_items {
            let tail = self.max_items - 1;
            hidden = (1..n - tail).collect();
            shown = vec![Some(0), None];
            shown.extend((n - tail..n).map(Some));
        }
        let font = FontId::proportional(self.size);
        let mut clicked = None;
        let height = self.size + 14.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (k, item) in shown.iter().enumerate() {
                if k > 0 {
                    let (r, _) = ui.allocate_exact_size(vec2(22.0, height), Sense::hover());
                    let c = r.center();
                    let col = with_alpha(t.muted_foreground, 0.7);
                    match self.separator {
                        BreadcrumbSeparator::Chevron => {
                            LineIcon::ChevronRight.paint(ui.painter(), Rect::from_center_size(c, vec2(13.0, 13.0)), col)
                        }
                        BreadcrumbSeparator::Slash => {
                            ui.painter()
                                .line_segment([c + vec2(2.5, -6.0), c + vec2(-2.5, 6.0)], Stroke::new(1.2, col));
                        }
                        BreadcrumbSeparator::Dot => {
                            ui.painter().circle_filled(c, 2.0, col);
                        }
                    }
                }
                match item {
                    None => {
                        let (r, resp) = ui.allocate_exact_size(vec2(30.0, height), Sense::click());
                        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                        let h = kit::hover_anim(ui, id.with("dots"), resp.hovered());
                        let br = Rect::from_center_size(r.center(), vec2(28.0, 24.0));
                        ui.painter().rect_filled(br, 6.0, with_alpha(t.foreground, 0.08 * h));
                        LineIcon::Dots.paint(
                            ui.painter(),
                            Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                            t.muted_foreground.lerp_to_gamma(t.foreground, h),
                        );
                        let entries: Vec<MenuEntry> =
                            hidden.iter().map(|&i| MenuEntry::item(self.items[i].clone())).collect();
                        if let Some(label) = DropdownMenu::new(entries).width(160.0).show(&resp) {
                            clicked = hidden.iter().copied().find(|&i| self.items[i] == label);
                        }
                    }
                    Some(i) => {
                        let i = *i;
                        let current = i + 1 == n;
                        let icon_w = if i == 0 && self.home_icon { 20.0 } else { 0.0 };
                        let g = ui.painter().layout_no_wrap(self.items[i].clone(), font.clone(), Color32::WHITE);
                        let (r, resp) = ui.allocate_exact_size(
                            vec2(g.size().x + icon_w + 4.0, height),
                            if current { Sense::hover() } else { Sense::click() },
                        );
                        let resp = if current { resp } else { resp.on_hover_cursor(CursorIcon::PointingHand) };
                        let h = if current { 0.0 } else { kit::hover_anim(ui, id.with(("i", i)), resp.hovered()) };
                        let col =
                            if current { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, h) };
                        let x = r.min.x + 2.0;
                        if icon_w > 0.0 {
                            LineIcon::Home.paint(
                                ui.painter(),
                                Rect::from_center_size(pos2(x + 7.0, r.center().y), vec2(15.0, 15.0)),
                                col,
                            );
                        }
                        let tp = pos2(x + icon_w, r.center().y - g.size().y / 2.0);
                        ui.painter().text(tp, Align2::LEFT_TOP, &self.items[i], font.clone(), col);
                        if h > 0.01 {
                            let y = tp.y + g.size().y + 1.0;
                            let w = g.size().x * glowkit_core::ease(glowkit_core::Easing::OutCubic, h);
                            ui.painter().line_segment([pos2(tp.x, y), pos2(tp.x + w, y)], Stroke::new(1.0, col));
                        }
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                    }
                }
            }
        });
        clicked
    }
}

/// Page navigation: Previous / numbered pages with ellipses / Next. The active page's outline
/// slides between numbers on a spring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut page = 1;
/// ui.add(glowkit_components::Pagination::new(&mut page, 20));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Pagination<'a> {
    page: &'a mut usize,
    total: usize,
    siblings: usize,
    labels: bool,
    filled: bool,
}

impl<'a> Pagination<'a> {
    /// `page` is 1-based, `total` the number of pages.
    pub fn new(page: &'a mut usize, total: usize) -> Self {
        Self { page, total: total.max(1), siblings: 1, labels: true, filled: false }
    }
    /// How many numbers to show on each side of the current page (default 1).
    pub fn siblings(mut self, n: usize) -> Self {
        self.siblings = n;
        self
    }
    /// Show "Previous" / "Next" text next to the chevrons (default true).
    pub fn labels(mut self, yes: bool) -> Self {
        self.labels = yes;
        self
    }
    /// Fill the active page with the primary color instead of an outline.
    pub fn filled(mut self, yes: bool) -> Self {
        self.filled = yes;
        self
    }
}

fn page_slots(page: usize, total: usize, siblings: usize) -> Vec<Option<usize>> {
    let slots = siblings * 2 + 5;
    if total <= slots {
        return (1..=total).map(Some).collect();
    }
    let left = page.saturating_sub(siblings).max(1);
    let right = (page + siblings).min(total);
    let show_l = left > 2;
    let show_r = right < total - 1;
    let mut v = Vec::new();
    if !show_l && show_r {
        let count = 3 + 2 * siblings;
        v.extend((1..=count).map(Some));
        v.push(None);
        v.push(Some(total));
    } else if show_l && !show_r {
        let count = 3 + 2 * siblings;
        v.push(Some(1));
        v.push(None);
        v.extend((total - count + 1..=total).map(Some));
    } else {
        v.push(Some(1));
        v.push(None);
        v.extend((left..=right).map(Some));
        v.push(None);
        v.push(Some(total));
    }
    v
}

impl Widget for Pagination<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        *self.page = (*self.page).clamp(1, self.total);
        let slots = page_slots(*self.page, self.total, self.siblings);
        let cell = 36.0;
        let gap = 4.0;
        let font = FontId::proportional(13.5);
        let nav_w = |label: &str| {
            if self.labels {
                ui.painter().layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).size().x + 40.0
            } else {
                cell
            }
        };
        let prev_w = nav_w("Previous");
        let next_w = nav_w("Next");
        let total_w = prev_w + next_w + slots.len() as f32 * (cell + gap) + gap;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(total_w, cell), Sense::hover());
        let id = resp.id.with("glowkit_pagination");
        let mut x = rect.min.x;
        let mut new_page = *self.page;
        let btn = |ui: &mut Ui, r: Rect, key: usize, enabled: bool| -> (Response, f32) {
            let resp = ui.interact(r, id.with(("b", key)), if enabled { Sense::click() } else { Sense::hover() });
            let resp = if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
            let h = kit::hover_anim(ui, id.with(("b", key)), resp.hovered() && enabled);
            ui.painter().rect_filled(r, t.radius * 0.7, with_alpha(t.foreground, 0.07 * h));
            (resp, h)
        };
        // prev
        let pr = Rect::from_min_size(pos2(x, rect.min.y), vec2(prev_w, cell));
        let can_prev = *self.page > 1;
        let (presp, _) = btn(ui, pr, 0, can_prev);
        let pcol = if can_prev { t.foreground } else { with_alpha(t.muted_foreground, 0.5) };
        if self.labels {
            LineIcon::ChevronLeft.paint(
                ui.painter(),
                Rect::from_center_size(pos2(pr.min.x + 16.0, pr.center().y), vec2(14.0, 14.0)),
                pcol,
            );
            ui.painter().text(
                pos2(pr.min.x + 28.0, pr.center().y),
                Align2::LEFT_CENTER,
                "Previous",
                font.clone(),
                pcol,
            );
        } else {
            LineIcon::ChevronLeft.paint(ui.painter(), Rect::from_center_size(pr.center(), vec2(14.0, 14.0)), pcol);
        }
        if presp.clicked() {
            new_page -= 1;
        }
        x += prev_w + gap;
        let mut active_x = None;
        let mut cells = Vec::new();
        for (k, s) in slots.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(cell, cell));
            match s {
                Some(pg) => {
                    let (r2, _) = btn(ui, r, 10 + k, true);
                    if r2.clicked() {
                        new_page = *pg;
                    }
                    if *pg == *self.page {
                        active_x = Some(r.min.x);
                    }
                    cells.push((r, Some(*pg)));
                }
                None => cells.push((r, None)),
            }
            x += cell + gap;
        }
        // active indicator
        if let Some(ax) = active_x {
            let sx = glowkit_core::anim::spring(ui, id.with("ax"), ax, 400.0, 32.0);
            let ar = Rect::from_min_size(pos2(sx, rect.min.y), vec2(cell, cell));
            if self.filled {
                ui.painter().rect_filled(ar, t.radius * 0.7, t.primary);
            } else {
                ui.painter().rect_filled(ar, t.radius * 0.7, t.background);
                ui.painter().rect_stroke(ar, t.radius * 0.7, Stroke::new(1.0, t.border), StrokeKind::Inside);
            }
        }
        for (r, s) in cells {
            match s {
                Some(pg) => {
                    let active = pg == *self.page;
                    let col = if active && self.filled { t.primary_foreground } else { t.foreground };
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, pg.to_string(), font.clone(), col);
                }
                None => LineIcon::Dots.paint(
                    ui.painter(),
                    Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                    t.muted_foreground,
                ),
            }
        }
        // next
        let nr = Rect::from_min_size(pos2(x, rect.min.y), vec2(next_w, cell));
        let can_next = *self.page < self.total;
        let (nresp, _) = btn(ui, nr, 1, can_next);
        let ncol = if can_next { t.foreground } else { with_alpha(t.muted_foreground, 0.5) };
        if self.labels {
            ui.painter().text(pos2(nr.max.x - 28.0, nr.center().y), Align2::RIGHT_CENTER, "Next", font.clone(), ncol);
            LineIcon::ChevronRight.paint(
                ui.painter(),
                Rect::from_center_size(pos2(nr.max.x - 16.0, nr.center().y), vec2(14.0, 14.0)),
                ncol,
            );
        } else {
            LineIcon::ChevronRight.paint(ui.painter(), Rect::from_center_size(nr.center(), vec2(14.0, 14.0)), ncol);
        }
        if nresp.clicked() {
            new_page += 1;
        }
        let new_page = new_page.clamp(1, self.total);
        if new_page != *self.page {
            *self.page = new_page;
            resp.mark_changed();
        }
        resp
    }
}
