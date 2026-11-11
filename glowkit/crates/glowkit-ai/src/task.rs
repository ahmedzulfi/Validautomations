//! Agent plans / task checklists.

use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::loader::paint_shimmer;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// State of a [`TaskItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskStatus {
    /// Not started: hollow circle.
    #[default]
    Pending,
    /// In progress: spinner, shimmering label.
    Running,
    /// Finished: check mark (pops in).
    Done,
    /// Failed: red cross.
    Failed,
}

/// One entry of a [`TaskList`].
#[derive(Clone, Copy, Debug)]
pub struct TaskItem<'a> {
    /// What the agent does.
    pub label: &'a str,
    /// Current state.
    pub status: TaskStatus,
    /// Optional muted detail (file name, command...).
    pub detail: Option<&'a str>,
}

impl<'a> TaskItem<'a> {
    /// A task with a label and status.
    pub const fn new(label: &'a str, status: TaskStatus) -> Self {
        Self { label, status, detail: None }
    }
    /// Muted detail shown after the label.
    pub const fn detail(mut self, d: &'a str) -> Self {
        self.detail = Some(d);
        self
    }
}

/// An agent's plan: a collapsible card with a title, "3 of 5" progress and a checklist whose items
/// go from pending to a spinner to a check mark.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{TaskItem, TaskList, TaskStatus};
/// ui.add(TaskList::new("Refactor the parser", &[
///     TaskItem::new("Read parser.rs", TaskStatus::Done),
///     TaskItem::new("Write failing tests", TaskStatus::Running),
///     TaskItem::new("Fix tokenizer", TaskStatus::Pending),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TaskList<'a> {
    title: &'a str,
    description: Option<&'a str>,
    items: &'a [TaskItem<'a>],
    default_open: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> TaskList<'a> {
    /// A plan titled `title`.
    pub fn new(title: &'a str, items: &'a [TaskItem<'a>]) -> Self {
        Self { title, description: None, items, default_open: true, id_salt: None }
    }
    /// A muted line under the title.
    pub fn description(mut self, d: &'a str) -> Self {
        self.description = Some(d);
        self
    }
    /// Expanded when first shown (default on).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    /// Stable id for open state and check animations.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

fn status_icon(ui: &Ui, id: egui::Id, c: egui::Pos2, status: TaskStatus, t: &Theme) {
    let p = ui.painter();
    let done = common::anim_bool(ui, id, status == TaskStatus::Done, 0.35);
    match status {
        TaskStatus::Pending => {
            p.circle_stroke(c, 7.0, Stroke::new(1.3, common::border_strong(t)));
        }
        TaskStatus::Running => {
            common::spinner(p, c, 7.0, 1.6, t.accent, glowkit_core::time(ui));
            glowkit_core::animate(ui);
        }
        TaskStatus::Done => {
            let k = glowkit_core::ease(glowkit_core::Easing::OutBack, done);
            p.circle_filled(c, 8.0 * k.max(0.0), t.success);
            paint_icon(p, Icon::Check, Rect::from_center_size(c, vec2(10.0, 10.0) * k.clamp(0.0, 1.2)), if t.dark { Color32::from_rgb(5, 25, 12) } else { Color32::WHITE });
        }
        TaskStatus::Failed => {
            p.circle_filled(c, 8.0, with_alpha(t.destructive, 0.15));
            paint_icon(p, Icon::X, Rect::from_center_size(c, vec2(9.0, 9.0)), t.destructive);
        }
    }
}

impl Widget for TaskList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        let (_, open) = common::toggle_state(ui, id, self.default_open, false);
        let width = ui.available_width();
        let top = ui.cursor().min;
        let cr = t.corner_scaled(1.0);
        let bg = ui.painter().add(egui::Shape::Noop);
        let done = self.items.iter().filter(|i| i.status == TaskStatus::Done).count();
        let total = self.items.len();
        let running = self.items.iter().any(|i| i.status == TaskStatus::Running);
        let resp = ui
            .scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(top, vec2(width, f32::INFINITY))), |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let head_h = if self.description.is_some() { 64.0 } else { 50.0 };
                let (hr, hresp) = ui.allocate_exact_size(vec2(width, head_h), Sense::click());
                let hresp = hresp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if hresp.clicked() {
                    common::toggle_state(ui, id, self.default_open, true);
                }
                let p = ui.painter();
                let tg = common::galley(ui, self.title, 14.0, t.foreground);
                let ty = if self.description.is_some() { hr.min.y + 14.0 } else { hr.center().y - tg.size().y / 2.0 };
                // title
                let ix = hr.min.x + 16.0;
                paint_icon(p, Icon::Sparkle, Rect::from_center_size(pos2(ix + 7.0, ty + tg.size().y / 2.0), vec2(15.0, 15.0)), t.accent);
                if running {
                    paint_shimmer(ui, pos2(ix + 24.0, ty), self.title, 14.0, mix(t.muted_foreground, t.foreground, 0.4), t.foreground, 2.0);
                } else {
                    common::paint_galley_fx(ui.painter(), pos2(ix + 24.0, ty), &tg, |_| common::Fx { bold: true, ..Default::default() });
                }
                if let Some(d) = self.description {
                    common::text(ui.painter(), pos2(ix + 24.0, ty + tg.size().y + 4.0), Align2::LEFT_TOP, d, 12.5, t.muted_foreground);
                }
                // progress
                let p = ui.painter();
                let chev = pos2(hr.max.x - 20.0, ty + tg.size().y / 2.0);
                common::chevron(p, chev, 9.0, t.muted_foreground, open);
                let count = format!("{done} of {total}");
                let cr2 = common::text(p, pos2(chev.x - 16.0, chev.y), Align2::RIGHT_CENTER, count, 12.0, t.muted_foreground);
                let bar = Rect::from_min_size(pos2(cr2.min.x - 70.0, chev.y - 2.0), vec2(58.0, 4.0));
                let frac = glowkit_core::anim::follow(ui, id.with("prog"), if total > 0 { done as f32 / total as f32 } else { 0.0 }, 8.0);
                let p = ui.painter();
                p.rect_filled(bar, CornerRadius::same(2), with_alpha(t.muted_foreground, 0.18));
                if frac > 0.0 {
                    p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * frac, bar.height())), CornerRadius::same(2), t.success);
                }
                common::reveal(ui, id.with("body"), open, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let r = ui.available_rect_before_wrap();
                    ui.painter().line_segment([pos2(r.min.x + 1.0, r.min.y), pos2(r.max.x - 1.0, r.min.y)], Stroke::new(1.0, with_alpha(t.border, 0.8)));
                    ui.add_space(6.0);
                    for (i, it) in self.items.iter().enumerate() {
                        let (row, _) = ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
                        let c = pos2(row.min.x + 23.0, row.center().y);
                        status_icon(ui, id.with(("item", i)), c, it.status, &t);
                        let col = match it.status {
                            TaskStatus::Done => t.muted_foreground,
                            TaskStatus::Pending => mix(t.foreground, t.muted_foreground, 0.35),
                            _ => t.foreground,
                        };
                        let x = row.min.x + 42.0;
                        let g = common::galley(ui, it.label, 13.5, col);
                        let lr = Rect::from_min_size(pos2(x, row.center().y - g.size().y / 2.0), g.size());
                        if it.status == TaskStatus::Running {
                            paint_shimmer(ui, lr.min, it.label, 13.5, mix(t.muted_foreground, t.foreground, 0.5), t.foreground, 1.6);
                        } else {
                            ui.painter().galley(lr.min, g, col);
                        }
                        if let Some(d) = it.detail {
                            let dg = ui.painter().layout_no_wrap(d.to_owned(), egui::FontId::monospace(11.5), t.muted_foreground);
                            let dr = Rect::from_min_size(pos2(lr.max.x + 10.0, row.center().y - 10.0), vec2(dg.size().x + 12.0, 20.0));
                            if dr.max.x < row.max.x - 8.0 {
                                let p = ui.painter();
                                p.rect_filled(dr, CornerRadius::same(5), common::raised(&t));
                                p.rect_stroke(dr, CornerRadius::same(5), Stroke::new(1.0, with_alpha(t.border, 0.8)), StrokeKind::Inside);
                                p.galley(pos2(dr.min.x + 6.0, dr.center().y - dg.size().y / 2.0), dg, t.muted_foreground);
                            }
                        }
                    }
                    ui.add_space(8.0);
                });
            })
            .response;
        let rect = Rect::from_min_size(top, vec2(width, resp.rect.height()));
        ui.painter().set(bg, egui::Shape::rect_filled(rect, cr, t.card));
        ui.painter().rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
        resp
    }
}
