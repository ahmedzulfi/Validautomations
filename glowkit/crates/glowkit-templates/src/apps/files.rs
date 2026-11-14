//! File manager template: folder tree, grid / list views, breadcrumbs and a details panel.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, AvatarGroup, Breadcrumb, Button, ButtonVariant, ControlSize, Input, LineIcon, Toast, ToggleGroup, UiIcon,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::sidebar_chevron;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// One file or folder in the [`FileManagerApp`] tree.
#[derive(Clone, Debug, PartialEq)]
pub struct FsEntry {
    /// File name.
    pub name: String,
    /// Parent folder index (None = root).
    pub parent: Option<usize>,
    /// Is this a folder?
    pub folder: bool,
    /// Size in bytes (files).
    pub size: u64,
    /// Modified label.
    pub modified: String,
    /// Starred.
    pub starred: bool,
}

/// A file manager inside an inset [`AppShell`]: an expandable folder tree, breadcrumbs, search,
/// a grid / list toggle, folders that open on double-click, selection with a details panel
/// (preview, metadata, sharing) and New folder / Upload actions. Backspace goes up a level.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::FileManagerApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct FileManagerApp {
    /// Active route.
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// The virtual file system.
    pub entries: Vec<FsEntry>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    current: Option<usize>,
    selected: Option<usize>,
    view: usize,
    query: String,
    expanded: Vec<bool>,
}

fn kind(name: &str, folder: bool) -> (&'static str, AppIcon, usize) {
    if folder {
        return ("Folder", AppIcon::Line(LineIcon::Folder), 3);
    }
    match name.rsplit('.').next().unwrap_or("") {
        "png" | "jpg" | "svg" => ("Image", AppIcon::Ui(UiIcon::Image), 2),
        "pdf" => ("PDF document", AppIcon::Line(LineIcon::File), 0),
        "rs" | "ts" | "json" => ("Source code", AppIcon::Ui(UiIcon::Code), 1),
        "mp4" | "mov" => ("Video", AppIcon::Play, 5),
        "mp3" | "wav" => ("Audio", AppIcon::Line(LineIcon::Music), 4),
        "fig" | "sketch" => ("Design file", AppIcon::Palette, 0),
        "zip" => ("Archive", AppIcon::Archive, 3),
        _ => ("Document", AppIcon::Line(LineIcon::File), 1),
    }
}

fn human(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else if b >= 1e3 {
        format!("{:.0} KB", b / 1e3)
    } else {
        format!("{bytes} B")
    }
}

impl Default for FileManagerApp {
    fn default() -> Self {
        let mut v: Vec<FsEntry> = Vec::new();
        let mut add = |name: &str, parent: Option<usize>, folder: bool, size: u64, modified: &str| {
            v.push(FsEntry { name: name.into(), parent, folder, size, modified: modified.into(), starred: false });
            v.len() - 1
        };
        let design = add("Design", None, true, 0, "Today");
        let eng = add("Engineering", None, true, 0, "Yesterday");
        let mkt = add("Marketing", None, true, 0, "Nov 8");
        let _ = add("Q4 roadmap.pdf", None, false, 2_400_000, "Today");
        let _ = add("Brand guidelines.pdf", None, false, 18_200_000, "Nov 2");
        let _ = add("Team offsite.mov", None, false, 412_000_000, "Oct 28");
        let _ = add("Budget 2027.xlsx", None, false, 84_000, "Oct 21");
        let _ = add("Launch playlist.mp3", None, false, 9_800_000, "Oct 12");
        let brand = add("Brand refresh", Some(design), true, 0, "Today");
        add("Pricing page.fig", Some(design), false, 24_000_000, "Today");
        add("Hero illustration.png", Some(design), false, 3_100_000, "Yesterday");
        add("Icon set.svg", Some(design), false, 220_000, "Nov 9");
        add("Logo final.svg", Some(brand), false, 48_000, "Today");
        add("Palette.png", Some(brand), false, 640_000, "Today");
        add("dashboard.rs", Some(eng), false, 14_000, "Yesterday");
        add("schema.json", Some(eng), false, 6_000, "Nov 7");
        add("release-notes.md", Some(eng), false, 9_000, "Nov 6");
        add("backup.zip", Some(eng), false, 1_200_000_000, "Nov 1");
        add("Campaign brief.docx", Some(mkt), false, 320_000, "Nov 8");
        add("Launch video.mp4", Some(mkt), false, 220_000_000, "Nov 5");
        let n = v.len();
        v[3].starred = true;
        v[1].starred = true;
        Self {
            route: "files".into(),
            nav: vec![
                NavSection::new("Drive")
                    .item(NavItem::new("files", "My files").icon(LineIcon::Folder))
                    .item(NavItem::new("shared", "Shared with me").icon(LineIcon::Users).badge("3"))
                    .item(NavItem::new("recent", "Recent").icon(UiIcon::Clock))
                    .item(NavItem::new("starred", "Starred").icon(LineIcon::Star))
                    .item(NavItem::new("trash", "Trash").icon(LineIcon::Trash)),
                NavSection::new("Storage").item(NavItem::new("usage", "Usage").icon(AppIcon::Database)),
            ],
            notifications: demo_notes(),
            entries: v,
            scoped_theme: false,
            current: None,
            selected: Some(3),
            view: 0,
            query: String::new(),
            expanded: vec![true; n],
        }
    }
}

impl FileManagerApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .workspaces(demo_workspaces("Acme Drive", "2 TB"))
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("files_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "files" => self.browser(ui, sc.rect),
                other => placeholder(ui, sc.rect.shrink(24.0), other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn children(&self, parent: Option<usize>) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.entries.len()).filter(|&i| self.entries[i].parent == parent).collect();
        v.sort_by_key(|&i| (!self.entries[i].folder, self.entries[i].name.to_lowercase()));
        v
    }

    fn path(&self, mut at: Option<usize>) -> Vec<usize> {
        let mut v = Vec::new();
        while let Some(i) = at {
            v.push(i);
            at = self.entries[i].parent;
        }
        v.reverse();
        v
    }

    fn open(&mut self, folder: Option<usize>) {
        self.current = folder;
        self.selected = None;
        self.query.clear();
    }

    fn browser(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("files", self.current));
        let loading = kit::loading(ui, id, 0.4);
        if ui.input(|i| i.key_pressed(egui::Key::Backspace)) && ui.memory(|m| m.focused().is_none()) {
            if let Some(c) = self.current {
                self.open(self.entries[c].parent);
            }
        }
        let tree_w = 220.0;
        let det_w = if rect.width() > 980.0 { 280.0 } else { 0.0 };
        let parts = kit::split_x(rect, &[tree_w, 0.0, det_w], 0.0);
        let p = ui.painter().clone();
        kit::vline(&p, parts[0].max.x, rect.min.y, rect.max.y, &t);
        if det_w > 0.0 {
            kit::vline(&p, parts[2].min.x, rect.min.y, rect.max.y, &t);
        }
        self.tree(ui, parts[0].shrink2(vec2(8.0, 12.0)));
        // toolbar
        let main = parts[1];
        let tb = Rect::from_min_size(main.min, vec2(main.width(), 56.0));
        kit::hline(&p, main.min.x, main.max.x, tb.max.y, &t);
        let mut crumbs = vec!["My files".to_string()];
        let path = self.path(self.current);
        crumbs.extend(path.iter().map(|&i| self.entries[i].name.clone()));
        let items: Vec<&str> = crumbs.iter().map(|s| s.as_str()).collect();
        let right_w = 330.0f32.min(main.width() * 0.55);
        let clicked = kit::child(
            ui,
            Rect::from_min_max(
                pos2(tb.min.x + 18.0, tb.center().y - 13.0),
                pos2(tb.max.x - right_w - 10.0, tb.center().y + 13.0),
            ),
            |ui| Breadcrumb::new(&items).size(14.0).max_items(3).show(ui),
        );
        if let Some(i) = clicked {
            self.open(if i == 0 { None } else { Some(path[i - 1]) });
        }
        let cy = tb.center().y;
        let mut x = tb.max.x - 16.0;
        let up = kit::put(
            ui,
            Rect::from_min_size(pos2(x - 96.0, cy - 16.0), vec2(96.0, 32.0)),
            Button::new("Upload").icon(UiIcon::Upload).size(ControlSize::Sm).width(96.0),
        );
        if up.clicked() {
            let i = self.entries.len();
            self.entries.push(FsEntry {
                name: format!("Upload {}.png", i),
                parent: self.current,
                folder: false,
                size: 1_800_000,
                modified: "Just now".into(),
                starred: false,
            });
            self.expanded.push(true);
            self.selected = Some(i);
            toast(ui.ctx(), Toast::success("Upload complete").description(format!("Upload {i}.png · 1.8 MB")));
        }
        x -= 104.0;
        let nf = kit::icon_button(
            ui,
            Rect::from_center_size(pos2(x - 16.0, cy), vec2(32.0, 32.0)),
            id.with("newf"),
            AppIcon::Ui(UiIcon::FolderOpen),
            false,
        );
        kit::tip(&nf, "New folder", None);
        if nf.clicked() {
            let i = self.entries.len();
            self.entries.push(FsEntry {
                name: "Untitled folder".into(),
                parent: self.current,
                folder: true,
                size: 0,
                modified: "Just now".into(),
                starred: false,
            });
            self.expanded.push(true);
            self.selected = Some(i);
            toast(ui.ctx(), Toast::success("Folder created"));
        }
        x -= 40.0;
        kit::put(
            ui,
            Rect::from_min_size(pos2(x - 72.0, cy - 16.0), vec2(72.0, 32.0)),
            ToggleGroup::single(&mut self.view, [UiIcon::Grid, UiIcon::List]).size(ControlSize::Sm),
        );
        x -= 80.0;
        let sw = (x - (tb.max.x - right_w)).max(0.0);
        if sw > 80.0 {
            kit::put(
                ui,
                Rect::from_min_size(pos2(x - sw, cy - 16.0), vec2(sw, 32.0)),
                Input::new(&mut self.query)
                    .placeholder("Search")
                    .icon(UiIcon::Search)
                    .clearable(true)
                    .size(ControlSize::Sm)
                    .width(sw)
                    .id_salt("fm_q"),
            );
        }
        // contents
        let q = self.query.to_lowercase();
        let list: Vec<usize> = if q.is_empty() {
            self.children(self.current)
        } else {
            (0..self.entries.len()).filter(|&i| self.entries[i].name.to_lowercase().contains(&q)).collect()
        };
        let area = Rect::from_min_max(pos2(main.min.x, tb.max.y), main.max).shrink(18.0);
        let mut open_folder = None;
        let mut select = None;
        let mut bg_clicked = false;
        kit::child(ui, area, |ui| {
            let bg = ui.interact(area, id.with("bg"), Sense::click());
            bg_clicked = bg.clicked();
            kit::scroll(ui, "fm_scroll", |ui| {
                let w = ui.available_width();
                if list.is_empty() {
                    let r = kit::alloc_row(ui, 220.0);
                    ui.painter().text(
                        r.center() - vec2(0.0, 10.0),
                        Align2::CENTER_CENTER,
                        if q.is_empty() { "This folder is empty" } else { "No files match" },
                        egui::FontId::proportional(14.0),
                        t.foreground,
                    );
                    ui.painter().text(
                        r.center() + vec2(0.0, 12.0),
                        Align2::CENTER_CENTER,
                        "Drop files here or use Upload.",
                        egui::FontId::proportional(12.5),
                        t.muted_foreground,
                    );
                    return;
                }
                if self.view == 0 {
                    let ncol = ((w + 12.0) / 150.0).floor().max(1.0) as usize;
                    ui.spacing_mut().item_spacing.y = 12.0;
                    for chunk in list.chunks(ncol) {
                        let row = kit::alloc_row(ui, 132.0);
                        for (k, r) in kit::cols(row, ncol, 12.0).iter().enumerate() {
                            let Some(&ei) = chunk.get(k) else { continue };
                            if loading {
                                kit::skel_r(ui, *r, t.radius);
                                continue;
                            }
                            let e = &self.entries[ei];
                            let rid = id.with(("tile", ei));
                            let resp = ui.interact(*r, rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            let hv = kit::hover(ui, rid, resp.hovered());
                            let sel = self.selected == Some(ei);
                            let pp = ui.painter().clone();
                            pp.rect_filled(
                                *r,
                                t.radius,
                                if sel { with_alpha(t.accent, 0.12) } else { kit::hover_fill(&t, hv) },
                            );
                            pp.rect_stroke(
                                *r,
                                t.radius,
                                Stroke::new(
                                    1.0,
                                    if sel {
                                        with_alpha(t.accent, 0.6)
                                    } else {
                                        t.border.lerp_to_gamma(t.muted_foreground, hv * 0.3)
                                    },
                                ),
                                StrokeKind::Inside,
                            );
                            let (_, icon, ci) = kind(&e.name, e.folder);
                            let ic = Rect::from_center_size(pos2(r.center().x, r.min.y + 46.0), vec2(56.0, 56.0));
                            let c = t.chart[ci];
                            pp.rect_filled(ic, 12.0, with_alpha(c, if t.dark { 0.16 } else { 0.12 }));
                            icon.paint(&pp, Rect::from_center_size(ic.center(), vec2(26.0, 26.0)), c);
                            if e.starred {
                                glowkit_core::paint::star(
                                    &pp,
                                    r.right_top() + vec2(-14.0, 14.0),
                                    5.5,
                                    2.4,
                                    5,
                                    -std::f32::consts::FRAC_PI_2,
                                    t.warning,
                                );
                            }
                            kit::text_at(
                                &pp,
                                pos2(r.center().x, r.max.y - 34.0),
                                Align2::CENTER_CENTER,
                                &e.name,
                                13.0,
                                t.foreground,
                                r.width() - 16.0,
                            );
                            let sub = if e.folder {
                                format!("{} items", self.children(Some(ei)).len())
                            } else {
                                human(e.size)
                            };
                            kit::text_at(
                                &pp,
                                pos2(r.center().x, r.max.y - 16.0),
                                Align2::CENTER_CENTER,
                                &sub,
                                11.5,
                                t.muted_foreground,
                                r.width() - 16.0,
                            );
                            if resp.double_clicked() && e.folder {
                                open_folder = Some(ei);
                            } else if resp.clicked() {
                                select = Some(ei);
                            }
                        }
                    }
                } else {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let head = kit::alloc_row(ui, 30.0);
                    let cols = [0.0, 140.0, 100.0, 110.0];
                    let pp = ui.painter().clone();
                    for (c, h) in kit::split_x(head.shrink2(vec2(12.0, 0.0)), &cols, 10.0)
                        .iter()
                        .zip(["Name", "Kind", "Size", "Modified"])
                    {
                        kit::text_at(&pp, c.left_center(), Align2::LEFT_CENTER, h, 12.0, t.muted_foreground, c.width());
                    }
                    kit::hline(&pp, head.min.x, head.max.x, head.max.y, &t);
                    for &ei in &list {
                        let r = kit::alloc_row(ui, 40.0);
                        if loading {
                            kit::skel(ui, Rect::from_min_size(r.min + vec2(12.0, 15.0), vec2(r.width() * 0.4, 10.0)));
                            continue;
                        }
                        let e = &self.entries[ei];
                        let rid = id.with(("row", ei));
                        let (resp, _) = kit::row(ui, r, rid, t.radius * 0.6);
                        let sel = self.selected == Some(ei);
                        let pp = ui.painter().clone();
                        if sel {
                            pp.rect_filled(r, t.radius * 0.6, with_alpha(t.accent, 0.12));
                        }
                        let cells = kit::split_x(r.shrink2(vec2(12.0, 0.0)), &cols, 10.0);
                        let (k, icon, ci) = kind(&e.name, e.folder);
                        icon.paint(
                            &pp,
                            Rect::from_center_size(pos2(cells[0].min.x + 9.0, r.center().y), vec2(17.0, 17.0)),
                            t.chart[ci],
                        );
                        kit::text_at(
                            &pp,
                            pos2(cells[0].min.x + 28.0, r.center().y),
                            Align2::LEFT_CENTER,
                            &e.name,
                            13.5,
                            t.foreground,
                            cells[0].width() - 30.0,
                        );
                        kit::text_at(
                            &pp,
                            cells[1].left_center(),
                            Align2::LEFT_CENTER,
                            k,
                            12.5,
                            t.muted_foreground,
                            cells[1].width(),
                        );
                        kit::text_at(
                            &pp,
                            cells[2].left_center(),
                            Align2::LEFT_CENTER,
                            &if e.folder { "—".into() } else { human(e.size) },
                            12.5,
                            t.muted_foreground,
                            cells[2].width(),
                        );
                        kit::text_at(
                            &pp,
                            cells[3].left_center(),
                            Align2::LEFT_CENTER,
                            &e.modified,
                            12.5,
                            t.muted_foreground,
                            cells[3].width(),
                        );
                        if resp.double_clicked() && e.folder {
                            open_folder = Some(ei);
                        } else if resp.clicked() {
                            select = Some(ei);
                        }
                    }
                }
            });
        });
        if let Some(f) = open_folder {
            self.open(Some(f));
        } else if let Some(s) = select {
            self.selected = Some(s);
        } else if bg_clicked {
            self.selected = None;
        }
        if det_w > 0.0 {
            self.details(ui, parts[2].shrink(18.0));
        }
    }

    fn tree(&mut self, ui: &mut Ui, r: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("fm_tree");
        let mut rows: Vec<(Option<usize>, usize)> = vec![(None, 0)];
        fn walk(app: &FileManagerApp, parent: Option<usize>, depth: usize, out: &mut Vec<(Option<usize>, usize)>) {
            for c in app.children(parent) {
                if app.entries[c].folder {
                    out.push((Some(c), depth));
                    if app.expanded[c] {
                        walk(app, Some(c), depth + 1, out);
                    }
                }
            }
        }
        walk(self, None, 1, &mut rows);
        let p = ui.painter().clone();
        kit::text_at(&p, r.min + vec2(8.0, 8.0), Align2::LEFT_CENTER, "Folders", 12.0, t.muted_foreground, 120.0);
        let mut y = r.min.y + 22.0;
        let mut action = None;
        for (fi, depth) in rows {
            let rr = Rect::from_min_size(pos2(r.min.x, y), vec2(r.width(), 30.0));
            y += 32.0;
            if rr.max.y > r.max.y {
                break;
            }
            let rid = id.with(fi);
            let active = self.current == fi;
            let resp = ui.interact(rr, rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, rid, resp.hovered());
            p.rect_filled(
                rr,
                t.radius * 0.6,
                if active { kit::active_fill(&t, t.background) } else { kit::hover_fill(&t, hv) },
            );
            let x = rr.min.x + 8.0 + depth as f32 * 14.0;
            let has_sub = fi.is_some_and(|f| self.children(Some(f)).iter().any(|&c| self.entries[c].folder));
            if has_sub {
                let e = kit::follow(
                    ui.ctx(),
                    rid.with("e"),
                    if fi.is_some_and(|f| self.expanded[f]) { 1.0 } else { 0.0 },
                    16.0,
                );
                sidebar_chevron(
                    &p,
                    pos2(x - 8.0, rr.center().y),
                    8.0,
                    e * std::f32::consts::FRAC_PI_2,
                    t.muted_foreground,
                );
            }
            let (icon, col) = if fi.is_none() {
                (AppIcon::Ui(UiIcon::Home), t.muted_foreground)
            } else {
                (AppIcon::Line(LineIcon::Folder), if active { t.accent } else { t.muted_foreground })
            };
            icon.paint(&p, Rect::from_center_size(pos2(x + 8.0, rr.center().y), vec2(15.0, 15.0)), col);
            let name = fi.map(|f| self.entries[f].name.clone()).unwrap_or_else(|| "My files".into());
            kit::text_at(
                &p,
                pos2(x + 22.0, rr.center().y),
                Align2::LEFT_CENTER,
                &name,
                13.0,
                if active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) },
                rr.max.x - x - 26.0,
            );
            if resp.clicked() {
                let chevron_hit = has_sub && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pp| pp.x < x + 2.0);
                action = Some((fi, chevron_hit));
            }
        }
        if let Some((fi, chev)) = action {
            if chev {
                if let Some(f) = fi {
                    self.expanded[f] = !self.expanded[f];
                }
            } else {
                self.open(fi);
            }
        }
        // storage meter
        let sr = Rect::from_min_max(pos2(r.min.x + 4.0, r.max.y - 60.0), r.max - vec2(4.0, 0.0));
        let used: u64 = self.entries.iter().map(|e| e.size).sum();
        kit::text_at(&p, sr.min + vec2(4.0, 10.0), Align2::LEFT_CENTER, "Storage", 12.5, t.foreground, 100.0);
        kit::text_at(
            &p,
            pos2(sr.max.x - 4.0, sr.min.y + 10.0),
            Align2::RIGHT_CENTER,
            &format!("{} of 2 TB", human(used)),
            11.5,
            t.muted_foreground,
            120.0,
        );
        let track = Rect::from_min_size(sr.min + vec2(4.0, 24.0), vec2(sr.width() - 8.0, 6.0));
        p.rect_filled(track, 3.0, with_alpha(t.foreground, 0.08));
        let frac = (used as f32 / 2e12).max(0.04);
        glowkit_core::paint::gradient_rect(
            &p,
            Rect::from_min_size(track.min, vec2(track.width() * frac, 6.0)),
            3.0,
            0.0,
            &[t.gradient[0], t.gradient[1]],
        );
    }

    fn details(&mut self, ui: &mut Ui, r: Rect) {
        let t = Theme::of(ui);
        let p = ui.painter().clone();
        let Some(si) = self.selected.filter(|s| *s < self.entries.len()) else {
            AppIcon::Line(LineIcon::Info).paint(
                &p,
                Rect::from_center_size(r.center() - vec2(0.0, 20.0), vec2(24.0, 24.0)),
                t.muted_foreground,
            );
            p.text(
                r.center() + vec2(0.0, 12.0),
                Align2::CENTER_CENTER,
                "Select a file to see details",
                egui::FontId::proportional(13.0),
                t.muted_foreground,
            );
            return;
        };
        let e = self.entries[si].clone();
        let (k, icon, ci) = kind(&e.name, e.folder);
        let prev = Rect::from_min_size(r.min, vec2(r.width(), 150.0));
        let c = t.chart[ci];
        p.rect_filled(prev, t.radius, if t.dark { mix(t.card, c, 0.14) } else { mix(Color32::WHITE, c, 0.1) });
        glowkit_core::paint::glow(&p, prev.center(), 70.0, with_alpha(c, 0.25));
        icon.paint(&p, Rect::from_center_size(prev.center(), vec2(48.0, 48.0)), c);
        kit::text_at(&p, pos2(r.min.x, prev.max.y + 22.0), Align2::LEFT_CENTER, &e.name, 15.0, t.foreground, r.width());
        kit::text_at(&p, pos2(r.min.x, prev.max.y + 42.0), Align2::LEFT_CENTER, k, 12.5, t.muted_foreground, r.width());
        let rows = [
            ("Size", if e.folder { format!("{} items", self.children(Some(si)).len()) } else { human(e.size) }),
            ("Modified", e.modified.clone()),
            (
                "Location",
                self.path(e.parent)
                    .iter()
                    .map(|&i| self.entries[i].name.clone())
                    .fold("My files".to_string(), |a, b| format!("{a} / {b}")),
            ),
            ("Owner", "Ada Lovelace".into()),
        ];
        let mut y = prev.max.y + 70.0;
        for (key, v) in rows.iter() {
            kit::text_at(&p, pos2(r.min.x, y), Align2::LEFT_CENTER, key, 12.5, t.muted_foreground, 80.0);
            kit::text_at(&p, pos2(r.max.x, y), Align2::RIGHT_CENTER, v, 12.5, t.foreground, r.width() - 90.0);
            y += 26.0;
        }
        kit::hline(&p, r.min.x, r.max.x, y, &t);
        y += 18.0;
        kit::text_at(&p, pos2(r.min.x, y), Align2::LEFT_CENTER, "Shared with", 12.5, t.muted_foreground, 120.0);
        kit::put(
            ui,
            Rect::from_min_size(pos2(r.max.x - 100.0, y - 13.0), vec2(100.0, 26.0)),
            AvatarGroup::new(&["Mia Chen", "Liam Johnson", "Sofia Davis"]).size(26.0),
        );
        y += 30.0;
        let bw = (r.width() - 8.0) / 2.0;
        if kit::put(
            ui,
            Rect::from_min_size(pos2(r.min.x, y), vec2(bw, 32.0)),
            Button::new("Share").icon(UiIcon::Users).size(ControlSize::Sm).width(bw),
        )
        .clicked()
        {
            ui.ctx().copy_text(format!("https://drive.acme.dev/f/{si}"));
            toast(ui.ctx(), Toast::success("Link copied").description("Anyone in Acme with the link can view."));
        }
        let star_label = if e.starred { "Unstar" } else { "Star" };
        if kit::put(
            ui,
            Rect::from_min_size(pos2(r.min.x + bw + 8.0, y), vec2(bw, 32.0)),
            Button::new(star_label).icon(UiIcon::Star).variant(ButtonVariant::Outline).size(ControlSize::Sm).width(bw),
        )
        .clicked()
        {
            self.entries[si].starred = !e.starred;
        }
    }
}
