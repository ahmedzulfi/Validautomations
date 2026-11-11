use crate::overlays::kit;
use crate::overlays::menu::{menu_panel, reset_menu};
use crate::overlays::{LineIcon, MenuEntry};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Order, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// A rich link inside a [`NavigationMenu`] panel.
#[derive(Clone, Debug, PartialEq)]
pub struct NavMenuLink {
    /// Title.
    pub title: String,
    /// One-line description.
    pub description: String,
    /// Optional icon.
    pub icon: Option<LineIcon>,
}

impl NavMenuLink {
    /// A link card with title and description.
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self { title: title.into(), description: description.into(), icon: None }
    }
    /// Add an icon.
    pub fn icon(mut self, icon: LineIcon) -> Self {
        self.icon = Some(icon);
        self
    }
}

#[derive(Clone, Debug)]
enum NavSection {
    Panel { label: String, links: Vec<NavMenuLink>, columns: usize, featured: Option<(String, String)> },
    Link { label: String },
}

/// A website-style navigation menu: hovering a trigger opens a large panel of link cards below
/// it. When you move between triggers the panel morphs its size on a spring and the content
/// slides in from the direction you moved (like Radix / shadcn Navigation Menu).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{NavigationMenu, NavMenuLink};
/// NavigationMenu::new()
///     .panel("Components", 2, vec![NavMenuLink::new("Dialog", "A window overlaid on the page.")])
///     .link("Docs")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
#[derive(Default)]
pub struct NavigationMenu {
    sections: Vec<NavSection>,
    within: Option<Rect>,
}

impl NavigationMenu {
    /// An empty menu.
    pub fn new() -> Self {
        Self::default()
    }
    /// A trigger that opens a panel of `links` laid out in `columns`.
    pub fn panel(mut self, label: impl Into<String>, columns: usize, links: Vec<NavMenuLink>) -> Self {
        self.sections.push(NavSection::Panel { label: label.into(), links, columns: columns.max(1), featured: None });
        self
    }
    /// Like [`Self::panel`] with a large gradient "featured" card on the left.
    pub fn featured_panel(mut self, label: impl Into<String>, featured: (&str, &str), links: Vec<NavMenuLink>) -> Self {
        self.sections.push(NavSection::Panel { label: label.into(), links, columns: 1, featured: Some((featured.0.into(), featured.1.into())) });
        self
    }
    /// A plain top-level link.
    pub fn link(mut self, label: impl Into<String>) -> Self {
        self.sections.push(NavSection::Link { label: label.into() });
        self
    }
    /// Clip the panel to `rect`.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Open a panel programmatically (for demos): `index` of the section, or `None` to close.
    pub fn set_open(ui: &Ui, index: Option<usize>) {
        let id = ui.scope_id().with("glowkit_navmenu");
        glowkit_core::set_state(ui, id.with("open"), index);
        glowkit_core::set_state(ui, id.with("forced"), index.is_some());
    }

    /// Show the menu row. Returns the title of a clicked link (or a top-level link label).
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = Theme::of(ui);
        let ctx = ui.ctx().clone();
        let id = ui.scope_id().with("glowkit_navmenu");
        let font = FontId::proportional(14.0);
        let now = ctx.input(|i| i.time);
        let h = 36.0;
        // triggers
        let widths: Vec<f32> = self
            .sections
            .iter()
            .map(|s| {
                let (l, chevron) = match s {
                    NavSection::Panel { label, .. } => (label, 18.0),
                    NavSection::Link { label } => (label, 0.0),
                };
                ui.painter().layout_no_wrap(l.clone(), font.clone(), Color32::WHITE).size().x + 28.0 + chevron
            })
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + 4.0 * widths.len() as f32;
        let (row, _) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
        let mut open: Option<usize> = glowkit_core::state(ui, id.with("open"), || None);
        let mut left_at: Option<f64> = glowkit_core::state(ui, id.with("left"), || None);
        let panel_rect: Option<Rect> = glowkit_core::state(ui, id.with("panel_rect"), || None);
        let mut clicked = None;
        let mut x = row.min.x;
        let mut trig_rects = Vec::new();
        let mut any_trigger_hover = false;
        for (i, s) in self.sections.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, row.min.y), vec2(widths[i], h));
            trig_rects.push(r);
            let resp = ui.interact(r, id.with(("trig", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let is_panel = matches!(s, NavSection::Panel { .. });
            if resp.hovered() {
                any_trigger_hover = true;
                if is_panel {
                    open = Some(i);
                } else {
                    open = None;
                }
            }
            if resp.clicked() {
                match s {
                    NavSection::Link { label } => clicked = Some(label.clone()),
                    NavSection::Panel { .. } => open = if open == Some(i) { None } else { Some(i) },
                }
            }
            let hv = kit::hover_anim(ui, id.with(("th", i)), resp.hovered() || open == Some(i));
            let p = ui.painter();
            p.rect_filled(r, t.radius * 0.7, with_alpha(t.foreground, 0.07 * hv));
            let label = match s {
                NavSection::Panel { label, .. } | NavSection::Link { label } => label,
            };
            let col = t.muted_foreground.lerp_to_gamma(t.foreground, 0.55 + 0.45 * hv);
            p.text(pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, label, font.clone(), col);
            if is_panel {
                let rot = kit::follow_ctx(&ctx, id.with(("rot", i)), if open == Some(i) { 1.0 } else { 0.0 }, 16.0);
                let c = pos2(r.max.x - 16.0, r.center().y + 1.0);
                let pts: Vec<egui::Pos2> = [(-3.5f32, -1.5f32), (0.0, 2.0), (3.5, -1.5)]
                    .iter()
                    .map(|&(dx, dy)| {
                        let dy = dy * (1.0 - 2.0 * rot);
                        c + vec2(dx, dy)
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(1.3, col)));
            }
            x += widths[i] + 4.0;
        }
        // close when the pointer leaves both the row and the panel for a moment
        let pointer = ctx.pointer_hover_pos();
        let over_panel = pointer.is_some_and(|pp| panel_rect.is_some_and(|r| r.expand(6.0).contains(pp)));
        let over_gap = pointer.is_some_and(|pp| open.is_some() && panel_rect.is_some_and(|r| Rect::from_min_max(pos2(r.min.x.min(row.min.x), row.max.y), pos2(r.max.x.max(row.max.x), r.min.y + 2.0)).contains(pp)));
        let forced: bool = glowkit_core::state(ui, id.with("forced"), || false);
        if open.is_some() && !any_trigger_hover && !over_panel && !over_gap && pointer.is_some() {
            let l = *left_at.get_or_insert(now);
            if now - l > 0.25 && !forced {
                open = None;
                left_at = None;
            } else {
                ctx.request_repaint();
            }
        } else {
            left_at = None;
        }
        if any_trigger_hover || over_panel {
            glowkit_core::set_state(ui, id.with("forced"), false);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = None;
        }
        let shown: Option<usize> = glowkit_core::state(ui, id.with("shown"), || None);
        if open.is_some() && open != shown {
            let dir = match (shown, open) {
                (Some(a), Some(b)) => if b > a { 1.0 } else { -1.0 },
                _ => 0.0,
            };
            glowkit_core::set_state(ui, id.with("dir"), dir);
            glowkit_core::set_state(ui, id.with("switched"), now);
            glowkit_core::set_state(ui, id.with("shown"), open);
        }
        let shown = open.or(shown);
        let p_open = kit::follow_ctx(&ctx, id.with("p"), if open.is_some() { 1.0 } else { 0.0 }, 16.0);
        if let Some(si) = shown {
            if p_open > 0.01 {
                if let NavSection::Panel { links, columns, featured, .. } = &self.sections[si] {
                    let pad = 10.0;
                    let cell_w = 236.0;
                    let cell_h = 68.0;
                    let feat_w = if featured.is_some() { 210.0 } else { 0.0 };
                    let cols = (*columns).min(links.len().max(1));
                    let rows_n = links.len().div_ceil(cols);
                    let pw = pad * 2.0 + feat_w + if feat_w > 0.0 { 8.0 } else { 0.0 } + cols as f32 * cell_w + (cols as f32 - 1.0) * 4.0;
                    let ph = pad * 2.0 + rows_n as f32 * cell_h + (rows_n as f32 - 1.0).max(0.0) * 4.0;
                    let ph = if featured.is_some() { ph.max(220.0) } else { ph };
                    let sw = glowkit_core::anim::spring(ui, id.with("pw"), pw, 300.0, 30.0);
                    let sh = glowkit_core::anim::spring(ui, id.with("ph"), ph, 300.0, 30.0);
                    let sx = glowkit_core::anim::spring(ui, id.with("px"), row.min.x, 300.0, 30.0);
                    let rect = Rect::from_min_size(pos2(sx, row.max.y + 8.0 - (1.0 - p_open) * 6.0), vec2(sw, sh));
                    glowkit_core::set_state(ui, id.with("panel_rect"), Some(rect));
                    let switched: f64 = glowkit_core::state(ui, id.with("switched"), || now);
                    let dir: f32 = glowkit_core::state(ui, id.with("dir"), || 0.0);
                    let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, ((now - switched) as f32 / 0.28).clamp(0.0, 1.0));
                    if k < 1.0 {
                        ctx.request_repaint();
                    }
                    let content_dx = dir * (1.0 - k) * 40.0;
                    let clip = self.within.map(|w| w.expand(400.0)).unwrap_or_else(|| ctx.content_rect());
                    let lid = id.with("panel");
                    // the arrow indicator under the active trigger
                    let tr = trig_rects[si];
                    let ax = glowkit_core::anim::spring(ui, id.with("ax"), tr.center().x, 300.0, 30.0);
                    let res = kit::float_layer(&ctx, lid, Order::Foreground, rect.expand(12.0), clip, 0.97 + 0.03 * p_open, pos2(ax, rect.min.y), p_open, |ui| {
                        ui.interact(rect, lid.with("block"), Sense::hover());
                        let pnt = ui.painter().clone();
                        kit::panel(&pnt, rect, &t, t.radius);
                        kit::arrow(&pnt, pos2(ax, rect.min.y - 6.0), vec2(0.0, 1.0), 6.0, t.popover, t.border);
                        let mut cp = pnt.with_clip_rect(rect.shrink(1.0).intersect(clip));
                        cp.multiply_opacity(k);
                        let mut out = None;
                        let ox = rect.min.x + pad + content_dx;
                        let oy = rect.min.y + pad;
                        if let Some((ft, fd)) = featured {
                            let fr = Rect::from_min_size(pos2(ox, oy), vec2(feat_w, ph - pad * 2.0));
                            paint::gradient_rect(&cp, fr, t.radius * 0.8, 135.0, &[with_alpha(t.gradient[0], 0.9), with_alpha(t.gradient[1], 0.75)]);
                            LineIcon::Sparkles.paint(&cp, Rect::from_min_size(fr.min + vec2(18.0, 18.0), vec2(26.0, 26.0)), Color32::WHITE);
                            let dg = cp.layout(fd.clone(), FontId::proportional(13.0), with_alpha(Color32::WHITE, 0.85), feat_w - 36.0);
                            let dy = fr.max.y - 18.0 - dg.size().y;
                            cp.text(pos2(fr.min.x + 18.0, dy - 8.0), Align2::LEFT_BOTTOM, ft, FontId::proportional(17.0), Color32::WHITE);
                            cp.galley(pos2(fr.min.x + 18.0, dy), dg, Color32::WHITE);
                            let fresp = ui.interact(fr, lid.with("feat"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            if fresp.clicked() {
                                out = Some(ft.clone());
                            }
                        }
                        let gx = ox + feat_w + if feat_w > 0.0 { 8.0 } else { 0.0 };
                        for (li, l) in links.iter().enumerate() {
                            let (c, r) = (li % cols, li / cols);
                            let cr = Rect::from_min_size(pos2(gx + c as f32 * (cell_w + 4.0), oy + r as f32 * (cell_h + 4.0)), vec2(cell_w, cell_h));
                            let resp = ui.interact(cr, lid.with(("l", si, li)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            let hv = kit::hover_anim(ui, lid.with(("l", si, li)), resp.hovered());
                            cp.rect_filled(cr, t.radius * 0.7, kit::row_highlight(&t).gamma_multiply(hv));
                            let mut tx = cr.min.x + 12.0;
                            if let Some(ic) = l.icon {
                                let ib = Rect::from_min_size(pos2(tx, cr.min.y + 12.0), vec2(28.0, 28.0));
                                cp.rect_filled(ib, 7.0, mix(t.popover, t.accent, 0.15 + 0.1 * hv));
                                ic.paint(&cp, ib.shrink(6.5), t.accent);
                                tx += 38.0;
                            }
                            cp.text(pos2(tx, cr.min.y + 12.0), Align2::LEFT_TOP, &l.title, FontId::proportional(13.8), t.foreground);
                            let mut job = egui::text::LayoutJob::simple(l.description.clone(), FontId::proportional(12.5), t.muted_foreground, cr.max.x - tx - 10.0);
                            job.wrap.max_rows = 2;
                            job.wrap.overflow_character = Some('…');
                            let g = ui.fonts_mut(|f| f.layout_job(job));
                            cp.galley(pos2(tx, cr.min.y + 31.0), g, t.muted_foreground);
                            if resp.clicked() {
                                out = Some(l.title.clone());
                            }
                        }
                        out
                    });
                    ctx.move_to_top(egui::LayerId::new(Order::Foreground, lid));
                    if let Some(c) = res.inner {
                        clicked = Some(c);
                        open = None;
                    }
                }
            } else if open.is_none() {
                glowkit_core::set_state(ui, id.with("shown"), None::<usize>);
                glowkit_core::set_state(ui, id.with("panel_rect"), None::<Rect>);
            }
        }
        glowkit_core::set_state(ui, id.with("open"), open);
        glowkit_core::set_state(ui, id.with("left"), left_at);
        clicked
    }
}

/// A desktop-app menubar ("File  Edit  View  Help"): click a title to open its menu, then
/// glide across titles to switch menus; ← / → move between menus, ↑ ↓ Enter pick items.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Menubar, MenuEntry};
/// let picked = Menubar::new()
///     .menu("File", vec![MenuEntry::item("New Tab").shortcut("Ctrl+T")])
///     .menu("Edit", vec![MenuEntry::item("Undo").shortcut("Ctrl+Z")])
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
#[derive(Default)]
pub struct Menubar {
    menus: Vec<(String, Vec<MenuEntry>)>,
    within: Option<Rect>,
}

impl Menubar {
    /// An empty menubar.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a menu.
    pub fn menu(mut self, title: impl Into<String>, entries: Vec<MenuEntry>) -> Self {
        self.menus.push((title.into(), entries));
        self
    }
    /// Keep menus inside `rect`.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Open a menu programmatically (for demos).
    pub fn set_open(ui: &Ui, index: Option<usize>) {
        glowkit_core::set_state(ui, ui.scope_id().with("glowkit_menubar").with("open"), index);
    }
    /// Show the bar. Returns the label of an activated item.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = Theme::of(ui);
        let ctx = ui.ctx().clone();
        let id = ui.scope_id().with("glowkit_menubar");
        let font = FontId::proportional(13.5);
        let widths: Vec<f32> = self.menus.iter().map(|(m, _)| ui.painter().layout_no_wrap(m.clone(), font.clone(), Color32::WHITE).size().x + 22.0).collect();
        let total = widths.iter().sum::<f32>() + 8.0 + 2.0 * widths.len() as f32;
        let (bar, _) = ui.allocate_exact_size(vec2(total, 40.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(bar, t.radius * 0.8, t.background);
        p.rect_stroke(bar, t.radius * 0.8, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let mut open: Option<usize> = glowkit_core::state(ui, id.with("open"), || None);
        let rects: Vec<Rect> = glowkit_core::state(ui, id.with("rects"), Vec::new);
        let mut x = bar.min.x + 4.0;
        let mut trig = Vec::new();
        let mut trigger_clicked = false;
        for (i, (title, _)) in self.menus.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, bar.min.y + 4.0), vec2(widths[i], 32.0));
            trig.push(r);
            let resp = ui.interact(r, id.with(("t", i)), Sense::click());
            if resp.clicked() {
                trigger_clicked = true;
                open = if open == Some(i) { None } else { Some(i) };
                reset_menu(&ctx, id.with(("m", i)));
            }
            if resp.hovered() && open.is_some() && open != Some(i) {
                open = Some(i);
                reset_menu(&ctx, id.with(("m", i)));
            }
            let hv = kit::hover_anim(ui, id.with(("th", i)), resp.hovered() || open == Some(i));
            let fill = if open == Some(i) { t.muted } else { with_alpha(t.foreground, 0.05 * hv) };
            ui.painter().rect_filled(r, t.radius * 0.55, fill);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, title, font.clone(), t.foreground);
            x += widths[i] + 2.0;
        }
        let pressed_outside = {
            let (pressed, pos) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
            pressed && pos.is_some_and(|pp| !rects.iter().any(|r| r.contains(pp)) && !bar.contains(pp))
        };
        if pressed_outside && !trigger_clicked {
            open = None;
        }
        let mut result = None;
        let bounds = self.within.unwrap_or_else(|| ctx.content_rect());
        let clip = self.within.map(|w| w.expand(60.0)).unwrap_or(bounds);
        let mut new_rects = Vec::new();
        let n = self.menus.len();
        for (i, (_, entries)) in self.menus.iter().enumerate() {
            let mid = id.with(("m", i));
            let p_open = kit::follow_ctx(&ctx, mid.with("p"), if open == Some(i) { 1.0 } else { 0.0 }, if open == Some(i) { 20.0 } else { 30.0 });
            let pop = kit::spring_ctx(&ctx, mid.with("pop"), if open == Some(i) { 1.0 } else { 0.0 }, 420.0, 28.0);
            if p_open < 0.01 && open != Some(i) {
                continue;
            }
            let anchor = Rect::from_min_max(trig[i].min, pos2(trig[i].max.x, bar.max.y - 2.0));
            let out = menu_panel(&ctx, mid, anchor, false, entries, p_open, pop, open == Some(i), bounds, clip, 0.0);
            if open == Some(i) {
                new_rects.extend(out.rects.iter().copied());
                if out.close {
                    open = None;
                }
                if out.key_left && n > 0 {
                    open = Some((i + n - 1) % n);
                    reset_menu(&ctx, id.with(("m", (i + n - 1) % n)));
                } else if out.key_right && n > 0 {
                    open = Some((i + 1) % n);
                    reset_menu(&ctx, id.with(("m", (i + 1) % n)));
                }
            }
            if out.activated.is_some() {
                result = out.activated;
            }
        }
        glowkit_core::set_state(ui, id.with("rects"), new_rects);
        glowkit_core::set_state(ui, id.with("open"), open);
        result
    }
}
