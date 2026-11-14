//! Project management template: task list and board views.

use egui::{pos2, vec2, Align2, CursorIcon, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, AvatarGroup, Button, ButtonVariant, Checkbox, ControlSize, LineIcon, SegmentedControl, Toast, ToggleItem, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::sidebar_chevron;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// One task in the [`ProjectsApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectTask {
    /// Title.
    pub title: String,
    /// 0 todo, 1 in progress, 2 in review, 3 done.
    pub status: usize,
    /// 0 low … 3 urgent.
    pub priority: u8,
    /// Assignee name.
    pub assignee: String,
    /// Due label.
    pub due: String,
    /// Overdue flag.
    pub overdue: bool,
    /// Label.
    pub tag: String,
    /// Sub-tasks done / total.
    pub subtasks: (u32, u32),
}

/// A project management screen inside an [`AppShell`]: project header with progress and team
/// avatars, a List / Board toggle (Ctrl+1 / Ctrl+2), collapsible status groups with checkboxes
/// that complete tasks, priority flags and due dates, and a drag-and-drop board. Ctrl+N adds a
/// task.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::ProjectsApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct ProjectsApp {
    /// Active route.
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Tasks.
    pub tasks: Vec<ProjectTask>,
    /// 0 = list, 1 = board.
    pub view: usize,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    collapsed: [bool; 4],
    drag: Option<usize>,
}

const STATUS: [&str; 4] = ["Todo", "In progress", "In review", "Done"];

impl Default for ProjectsApp {
    fn default() -> Self {
        let tk = |title: &str, status: usize, priority: u8, who: &str, due: &str, overdue: bool, tag: &str, sub: (u32, u32)| ProjectTask {
            title: title.into(),
            status,
            priority,
            assignee: who.into(),
            due: due.into(),
            overdue,
            tag: tag.into(),
            subtasks: sub,
        };
        Self {
            route: "projects/web".into(),
            nav: vec![
                NavSection::new("Workspace")
                    .item(NavItem::new("inbox", "Inbox").icon(LineIcon::Inbox).badge("4"))
                    .item(NavItem::new("my-tasks", "My tasks").icon(AppIcon::ListChecks))
                    .item(NavItem::new("goals", "Goals").icon(AppIcon::Target)),
                NavSection::new("Projects").item(
                    NavItem::new("projects", "Active projects")
                        .icon(AppIcon::Kanban)
                        .expanded(true)
                        .child(NavItem::new("projects/web", "Website redesign").badge("12"))
                        .child(NavItem::new("projects/mobile", "Mobile app v3"))
                        .child(NavItem::new("projects/brand", "Brand refresh")),
                ),
                NavSection::new("Team").item(NavItem::new("members", "Members").icon(LineIcon::Users)).item(NavItem::new("timeline", "Timeline").icon(LineIcon::Calendar)),
            ],
            notifications: demo_notes(),
            tasks: vec![
                tk("Design new pricing page", 1, 2, "Mia Chen", "Nov 14", false, "Design", (3, 5)),
                tk("Migrate blog to new CMS", 1, 1, "Liam Johnson", "Nov 12", false, "Engineering", (1, 4)),
                tk("Write launch announcement", 0, 1, "Sofia Davis", "Nov 18", false, "Marketing", (0, 2)),
                tk("Fix mobile nav overflow", 0, 3, "Noah Brown", "Nov 10", true, "Bug", (0, 1)),
                tk("Accessibility audit", 2, 2, "Ada Lovelace", "Nov 13", false, "QA", (6, 8)),
                tk("Hero illustration v2", 2, 1, "Mia Chen", "Nov 15", false, "Design", (2, 2)),
                tk("Set up analytics events", 0, 0, "Liam Johnson", "Nov 21", false, "Engineering", (0, 6)),
                tk("Customer quotes for homepage", 3, 1, "Sofia Davis", "Nov 6", false, "Marketing", (3, 3)),
                tk("New logo exports", 3, 0, "Mia Chen", "Nov 4", false, "Design", (4, 4)),
                tk("Performance budget", 1, 2, "Noah Brown", "Nov 16", false, "Engineering", (2, 3)),
            ],
            view: 0,
            scoped_theme: false,
            collapsed: [false, false, false, true],
            drag: None,
        }
    }
}

fn priority_color(t: &Theme, p: u8) -> egui::Color32 {
    match p {
        3 => t.destructive,
        2 => t.warning,
        1 => t.info,
        _ => t.muted_foreground,
    }
}

impl ProjectsApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(demo_workspaces("Acme Studio", "Team"))
            .user(demo_user())
            .notifications(&mut notes)
            .upgrade_card(true)
            .scoped_theme(self.scoped_theme)
            .id_salt("projects_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "projects/web" => self.project(ui, sc.rect),
                other => placeholder(ui, sc.rect, other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn add_task(&mut self, ui: &Ui) {
        self.tasks.insert(0, ProjectTask { title: "Untitled task".into(), status: 0, priority: 1, assignee: "Ada Lovelace".into(), due: "Nov 20".into(), overdue: false, tag: "General".into(), subtasks: (0, 0) });
        self.collapsed[0] = false;
        toast(ui.ctx(), Toast::success("Task created").description("Added to Todo · press Ctrl+N again for another."));
    }

    fn project(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("pm");
        let loading = kit::loading(ui, id, 0.6);
        let (new_task, v1, v2) = ui.input_mut(|i| (i.consume_key(Modifiers::COMMAND, Key::N), i.consume_key(Modifiers::COMMAND, Key::Num1), i.consume_key(Modifiers::COMMAND, Key::Num2)));
        if new_task {
            self.add_task(ui);
        }
        if v1 {
            self.view = 0;
        }
        if v2 {
            self.view = 1;
        }
        let p = ui.painter().clone();
        // header
        let lr = Rect::from_min_size(rect.min, vec2(44.0, 44.0));
        kit::logo(&p, lr, &t, "W");
        kit::text_at(&p, pos2(lr.max.x + 14.0, rect.min.y + 12.0), Align2::LEFT_CENTER, "Website redesign", 20.0, t.foreground, rect.width() * 0.4);
        let done = self.tasks.iter().filter(|x| x.status == 3).count();
        let total = self.tasks.len().max(1);
        kit::text_at(&p, pos2(lr.max.x + 14.0, rect.min.y + 35.0), Align2::LEFT_CENTER, &format!("{done} of {total} tasks done · due Nov 28"), 13.0, t.muted_foreground, rect.width() * 0.4);
        // right cluster
        let mut add = false;
        let right = Rect::from_min_max(pos2(rect.min.x + rect.width() * 0.45, rect.min.y + 6.0), pos2(rect.max.x, rect.min.y + 38.0));
        kit::child(ui, right, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let b = ui.add(Button::new("New task").icon(UiIcon::Plus).size(ControlSize::Sm));
                kit::tip(&b, "Create a task", Some("Ctrl+N"));
                if b.clicked() {
                    add = true;
                }
                let r = ui.add(SegmentedControl::new(&mut self.view, [ToggleItem::new(UiIcon::List, "List"), ToggleItem::new(UiIcon::Grid, "Board")]).size(ControlSize::Sm));
                kit::tip(&r, "Switch view", Some("Ctrl+1 / Ctrl+2"));
                ui.add(AvatarGroup::new(&["Mia Chen", "Liam Johnson", "Sofia Davis", "Noah Brown", "Ada Lovelace"]).max(4).size(28.0));
            });
        });
        if add {
            self.add_task(ui);
        }
        // progress
        let pr = Rect::from_min_size(pos2(rect.min.x, rect.min.y + 60.0), vec2(rect.width(), 6.0));
        p.rect_filled(pr, 3.0, with_alpha(t.foreground, 0.08));
        let frac = kit::follow(ui.ctx(), id.with("prog"), done as f32 / total as f32, 6.0);
        glowkit_core::paint::gradient_rect(&p, Rect::from_min_size(pr.min, vec2(pr.width() * frac, 6.0)), 3.0, 0.0, &[t.gradient[0], t.gradient[1]]);
        let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 82.0), rect.max);
        if self.view == 0 {
            self.list_view(ui, body, loading);
        } else {
            self.board_view(ui, body, loading);
        }
    }

    fn list_view(&mut self, ui: &mut Ui, body: Rect, loading: bool) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("pm_list");
        kit::child(ui, body, |ui| {
            kit::scroll(ui, "pm_list_scroll", |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for s in [1usize, 2, 0, 3] {
                    let items: Vec<usize> = (0..self.tasks.len()).filter(|&i| self.tasks[i].status == s).collect();
                    let hr = kit::alloc_row(ui, 38.0);
                    let (resp, _) = kit::row(ui, hr, id.with(("grp", s)), t.radius * 0.6);
                    let p = ui.painter().clone();
                    let e = kit::follow(ui.ctx(), id.with(("grp_e", s)), if self.collapsed[s] { 0.0 } else { 1.0 }, 16.0);
                    sidebar_chevron(&p, pos2(hr.min.x + 12.0, hr.center().y), 9.0, e * std::f32::consts::FRAC_PI_2, t.muted_foreground);
                    status_icon(&p, pos2(hr.min.x + 32.0, hr.center().y), s, &t);
                    let tr = kit::text_at(&p, pos2(hr.min.x + 46.0, hr.center().y), Align2::LEFT_CENTER, STATUS[s], 13.5, t.foreground, 200.0);
                    kit::pill(&p, pos2(tr.max.x + 8.0, hr.center().y), false, &items.len().to_string(), t.muted_foreground, t.muted, 10.5);
                    if resp.clicked() {
                        self.collapsed[s] = !self.collapsed[s];
                    }
                    let full = items.len() as f32 * 46.0;
                    if e > 0.001 {
                        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), full * e), Sense::hover());
                        let clip = area.intersect(ui.clip_rect());
                        for (k, &ti) in items.iter().enumerate() {
                            let rr = Rect::from_min_size(pos2(area.min.x, area.min.y + k as f32 * 46.0), vec2(area.width(), 44.0));
                            if !clip.intersects(rr) {
                                continue;
                            }
                            let cp = ui.painter().with_clip_rect(clip);
                            if loading {
                                kit::skel(ui, Rect::from_min_size(pos2(rr.min.x + 40.0, rr.center().y - 4.0), vec2(rr.width() * 0.4, 8.0)));
                                continue;
                            }
                            let rid = id.with(("task", ti));
                            let resp = ui.interact(rr.intersect(clip), rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            let hv = kit::hover(ui, rid, resp.hovered());
                            if hv > 0.01 {
                                cp.rect_filled(rr, t.radius * 0.6, kit::hover_fill(&t, hv));
                            }
                            cp.line_segment([pos2(rr.min.x + 40.0, rr.max.y + 1.0), pos2(rr.max.x, rr.max.y + 1.0)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
                            let tk = self.tasks[ti].clone();
                            let mut checked = tk.status == 3;
                            let cr = Rect::from_center_size(pos2(rr.min.x + 24.0, rr.center().y), vec2(18.0, 18.0));
                            if e > 0.9 && kit::put(ui, cr, Checkbox::new(&mut checked).size(16.0)).changed() {
                                self.tasks[ti].status = if checked { 3 } else { 0 };
                                if checked {
                                    toast(ui.ctx(), Toast::success("Task completed").description(tk.title.clone()).action("Undo"));
                                }
                            }
                            let title_col = if tk.status == 3 { t.muted_foreground } else { t.foreground };
                            let tr = kit::text_at(&cp, pos2(rr.min.x + 46.0, rr.center().y), Align2::LEFT_CENTER, &tk.title, 13.5, title_col, rr.width() * 0.42);
                            if tk.status == 3 {
                                cp.line_segment([pos2(tr.min.x, tr.center().y), pos2(tr.max.x, tr.center().y)], Stroke::new(1.0, t.muted_foreground));
                            }
                            // right side meta
                            let mut x = rr.max.x - 12.0;
                            kit::avatar(ui, Rect::from_center_size(pos2(x - 12.0, rr.center().y), vec2(24.0, 24.0)), &tk.assignee);
                            x -= 36.0;
                            let dr = kit::text_at(&cp, pos2(x, rr.center().y), Align2::RIGHT_CENTER, &tk.due, 12.5, if tk.overdue && tk.status != 3 { t.destructive } else { t.muted_foreground }, 70.0);
                            AppIcon::Line(LineIcon::Calendar).paint(&cp, Rect::from_center_size(pos2(dr.min.x - 10.0, rr.center().y), vec2(13.0, 13.0)), if tk.overdue && tk.status != 3 { t.destructive } else { t.muted_foreground });
                            x = dr.min.x - 28.0;
                            if tk.subtasks.1 > 0 {
                                let sr = kit::text_at(&cp, pos2(x, rr.center().y), Align2::RIGHT_CENTER, &format!("{}/{}", tk.subtasks.0, tk.subtasks.1), 12.0, t.muted_foreground, 50.0);
                                AppIcon::ListChecks.paint(&cp, Rect::from_center_size(pos2(sr.min.x - 10.0, rr.center().y), vec2(13.0, 13.0)), t.muted_foreground);
                                x = sr.min.x - 28.0;
                            }
                            AppIcon::Flag.paint(&cp, Rect::from_center_size(pos2(x - 6.0, rr.center().y), vec2(13.0, 13.0)), priority_color(&t, tk.priority));
                            x -= 22.0;
                            kit::pill(&cp, pos2(x, rr.center().y), true, &tk.tag, t.muted_foreground, t.muted, 11.0);
                            if resp.clicked() {
                                toast(ui.ctx(), Toast::new(tk.title.clone()).description(format!("{} · {} · due {}", STATUS[tk.status], tk.assignee, tk.due)));
                            }
                        }
                    }
                    ui.add_space(8.0);
                }
            });
        });
    }

    fn board_view(&mut self, ui: &mut Ui, body: Rect, loading: bool) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("pm_board");
        let cols = kit::cols(body, 4, 12.0);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let released = ui.input(|i| i.pointer.any_released());
        let mut drop_to = None;
        let p = ui.painter().clone();
        for (s, cr) in cols.iter().enumerate() {
            let over = self.drag.is_some() && pointer.is_some_and(|pp| cr.contains(pp));
            p.rect_filled(*cr, t.radius, if over { with_alpha(t.accent, 0.08) } else { with_alpha(t.muted, if t.dark { 0.45 } else { 0.7 }) });
            if over {
                p.rect_stroke(*cr, t.radius, Stroke::new(1.0, with_alpha(t.accent, 0.5)), StrokeKind::Inside);
                if released {
                    drop_to = Some(s);
                }
            }
            status_icon(&p, pos2(cr.min.x + 18.0, cr.min.y + 20.0), s, &t);
            let n = self.tasks.iter().filter(|x| x.status == s).count();
            let tr = kit::text_at(&p, pos2(cr.min.x + 32.0, cr.min.y + 20.0), Align2::LEFT_CENTER, STATUS[s], 13.5, t.foreground, cr.width() - 70.0);
            kit::pill(&p, pos2(tr.max.x + 6.0, cr.min.y + 20.0), false, &n.to_string(), t.muted_foreground, if t.dark { t.secondary } else { t.background }, 10.5);
            let mut y = cr.min.y + 40.0;
            for (ti, tk) in self.tasks.iter().enumerate().filter(|(_, x)| x.status == s) {
                let card = Rect::from_min_size(pos2(cr.min.x + 8.0, y), vec2(cr.width() - 16.0, 112.0));
                if card.max.y > cr.max.y - 4.0 {
                    break;
                }
                y += 120.0;
                if loading {
                    kit::skel_r(ui, card, t.radius * 0.8);
                    continue;
                }
                let cid = id.with(("card", ti));
                let resp = ui.interact(card, cid, Sense::click_and_drag()).on_hover_cursor(CursorIcon::Grab);
                if resp.drag_started() {
                    self.drag = Some(ti);
                }
                let hv = kit::hover(ui, cid, resp.hovered());
                let mut cp = p.clone();
                if self.drag == Some(ti) {
                    cp.multiply_opacity(0.35);
                }
                task_card(ui, &cp, card, tk, hv, &t);
            }
        }
        if let (Some(ti), Some(pp)) = (self.drag, pointer) {
            if let Some(tk) = self.tasks.get(ti) {
                let gp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("ghost")));
                let ghost = Rect::from_center_size(pp + vec2(0.0, 12.0), vec2(cols[0].width() - 16.0, 112.0));
                glowkit_core::paint::soft_shadow(&gp, ghost, t.radius, 18.0, vec2(0.0, 8.0), kit::shadow(&t));
                task_card(ui, &gp, ghost, tk, 1.0, &t);
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
        }
        if released {
            if let (Some(ti), Some(s)) = (self.drag, drop_to) {
                if self.tasks[ti].status != s {
                    self.tasks[ti].status = s;
                    let title = self.tasks[ti].title.clone();
                    toast(ui.ctx(), if s == 3 { Toast::success("Task completed").description(title) } else { Toast::new(format!("Moved to {}", STATUS[s])).description(title) });
                }
            }
            self.drag = None;
        }
    }
}

fn status_icon(p: &egui::Painter, c: egui::Pos2, s: usize, t: &Theme) {
    let col = [t.muted_foreground, t.warning, t.info, t.success][s];
    match s {
        0 => {
            p.circle_stroke(c, 6.0, Stroke::new(1.5, col));
        }
        3 => {
            p.circle_filled(c, 7.0, col);
            p.add(egui::Shape::line(vec![c + vec2(-3.0, 0.0), c + vec2(-1.0, 2.5), c + vec2(3.5, -2.5)], Stroke::new(1.6, t.background)));
        }
        _ => {
            p.circle_stroke(c, 6.0, Stroke::new(1.5, col));
            let frac = if s == 1 { 0.5 } else { 0.75 };
            glowkit_core::paint::arc_band(p, c, 0.0, 4.0, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac, col);
        }
    }
}

fn task_card(ui: &Ui, p: &egui::Painter, card: Rect, tk: &ProjectTask, hv: f32, t: &Theme) {
    if hv > 0.01 {
        glowkit_core::paint::soft_shadow(p, card, t.radius * 0.8, 10.0 * hv, vec2(0.0, 3.0), with_alpha(egui::Color32::BLACK, if t.dark { 0.3 } else { 0.08 } * hv));
    }
    p.rect_filled(card, t.radius * 0.8, t.card);
    p.rect_stroke(card, t.radius * 0.8, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, 0.3 * hv)), StrokeKind::Inside);
    let tr = kit::pill(p, card.left_top() + vec2(12.0, 18.0), false, &tk.tag, t.accent, with_alpha(t.accent, 0.12), 10.5);
    AppIcon::Flag.paint(p, Rect::from_center_size(pos2(tr.max.x + 14.0, tr.center().y), vec2(12.0, 12.0)), priority_color(t, tk.priority));
    let g = kit::wrapped(p, &tk.title, 13.5, t.foreground, card.width() - 24.0, 2);
    p.galley(card.left_top() + vec2(12.0, 34.0), g, t.foreground);
    if tk.subtasks.1 > 0 {
        let pr = Rect::from_min_size(pos2(card.min.x + 12.0, card.max.y - 34.0), vec2(card.width() - 70.0, 4.0));
        p.rect_filled(pr, 2.0, with_alpha(t.foreground, 0.08));
        p.rect_filled(Rect::from_min_size(pr.min, vec2(pr.width() * tk.subtasks.0 as f32 / tk.subtasks.1 as f32, 4.0)), 2.0, if tk.subtasks.0 == tk.subtasks.1 { t.success } else { t.accent });
        kit::text_at(p, pos2(card.max.x - 12.0, pr.center().y), Align2::RIGHT_CENTER, &format!("{}/{}", tk.subtasks.0, tk.subtasks.1), 11.0, t.muted_foreground, 50.0);
    }
    kit::text_at(p, pos2(card.min.x + 12.0, card.max.y - 14.0), Align2::LEFT_CENTER, &tk.due, 11.5, if tk.overdue && tk.status != 3 { t.destructive } else { t.muted_foreground }, 80.0);
    let ar = Rect::from_center_size(pos2(card.max.x - 22.0, card.max.y - 16.0), vec2(22.0, 22.0));
    glowkit_components::Avatar::new(tk.assignee.clone()).paint(ui, ar);
    let _ = ButtonVariant::Default;
}
