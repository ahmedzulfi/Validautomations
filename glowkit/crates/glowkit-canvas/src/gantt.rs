//! [`GanttChart`]: a project timeline with draggable / resizable task bars and dependencies.

use crate::common::{self, arrow_head, qfont, Icon};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::{color::with_alpha, Theme};

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Civil date `(year, month, day)` for days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Day of week for days since 1970-01-01 (0 = Monday ... 6 = Sunday).
pub fn weekday(z: i64) -> u32 {
    ((z + 3).rem_euclid(7)) as u32
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// A task bar. Days are counted from the chart's start date.
#[derive(Clone, Debug, PartialEq)]
pub struct GanttTask {
    pub id: u64,
    pub name: String,
    /// Start (days from [`Gantt::start`]).
    pub start: f32,
    /// Duration in days (`0` = milestone diamond).
    pub duration: f32,
    /// Progress `0..=1`.
    pub progress: f32,
    pub color: Option<Color32>,
    /// Ids of tasks that must finish first.
    pub deps: Vec<u64>,
    /// Assignee initials.
    pub assignee: String,
}

impl GanttTask {
    /// A task.
    pub fn new(id: u64, name: impl Into<String>, start: f32, duration: f32) -> Self {
        Self { id, name: name.into(), start, duration, progress: 0.0, color: None, deps: Vec::new(), assignee: String::new() }
    }
    /// Builder: progress.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = p;
        self
    }
    /// Builder: dependencies.
    pub fn after(mut self, deps: &[u64]) -> Self {
        self.deps = deps.to_vec();
        self
    }
    /// Builder: color.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Builder: assignee.
    pub fn assignee(mut self, a: impl Into<String>) -> Self {
        self.assignee = a.into();
        self
    }
    /// End day.
    pub fn end(&self) -> f32 {
        self.start + self.duration
    }
}

/// A group (swimlane) of tasks.
#[derive(Clone, Debug, PartialEq)]
pub struct GanttGroup {
    pub name: String,
    pub collapsed: bool,
    pub color: Option<Color32>,
    pub tasks: Vec<GanttTask>,
}

impl GanttGroup {
    /// A group.
    pub fn new(name: impl Into<String>, tasks: Vec<GanttTask>) -> Self {
        Self { name: name.into(), collapsed: false, color: None, tasks }
    }
    /// Builder: color for the group's tasks.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// `(start, end)` span of the group.
    pub fn span(&self) -> Option<(f32, f32)> {
        let s = self.tasks.iter().map(|t| t.start).fold(f32::INFINITY, f32::min);
        let e = self.tasks.iter().map(|t| t.end()).fold(f32::NEG_INFINITY, f32::max);
        (s.is_finite() && e.is_finite()).then_some((s, e))
    }
}

/// The chart data. Owned by the caller.
#[derive(Clone, Debug, PartialEq)]
pub struct Gantt {
    /// Calendar date of day 0 `(year, month, day)`.
    pub start: (i32, u32, u32),
    pub groups: Vec<GanttGroup>,
    /// "Today" (days from start) for the today marker.
    pub today: f32,
}

impl Gantt {
    /// Find a task by id.
    pub fn task(&self, id: u64) -> Option<&GanttTask> {
        self.groups.iter().flat_map(|g| g.tasks.iter()).find(|t| t.id == id)
    }
    fn task_mut(&mut self, id: u64) -> Option<&mut GanttTask> {
        self.groups.iter_mut().flat_map(|g| g.tasks.iter_mut()).find(|t| t.id == id)
    }
    /// Does task `id` start before one of its dependencies ends?
    pub fn violates(&self, id: u64) -> bool {
        self.task(id).is_some_and(|t| t.deps.iter().filter_map(|d| self.task(*d)).any(|d| d.end() > t.start + 1e-3))
    }
    /// Calendar date for a day offset.
    pub fn date(&self, day: f32) -> (i32, u32, u32) {
        civil_from_days(days_from_civil(self.start.0, self.start.1, self.start.2) + day.floor() as i64)
    }
}

/// Timeline scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GanttZoom {
    Day,
    #[default]
    Week,
    Month,
}

impl GanttZoom {
    fn px(self) -> f32 {
        match self {
            GanttZoom::Day => 44.0,
            GanttZoom::Week => 18.0,
            GanttZoom::Month => 5.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Move,
    Left,
    Right,
    Progress,
}

#[derive(Clone, Copy, Debug)]
struct GDrag {
    task: u64,
    mode: Mode,
    grab_day: f32,
    start: f32,
    duration: f32,
    progress: f32,
}

/// Caller-owned state of a [`GanttChart`].
#[derive(Clone, Debug)]
pub struct GanttState {
    pub zoom: GanttZoom,
    px: f32,
    /// First visible day.
    pub scroll: f32,
    scroll_y: f32,
    drag: Option<GDrag>,
    init: bool,
}

impl Default for GanttState {
    fn default() -> Self {
        Self { zoom: GanttZoom::Week, px: GanttZoom::Week.px(), scroll: 0.0, scroll_y: 0.0, drag: None, init: false }
    }
}

/// A Gantt chart: grouped rows, task bars with progress fill, milestones, dependency arrows,
/// a today line, day / week / month zoom levels, and bars you can drag to move, resize from
/// either end, or scrub the progress handle of.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{Gantt, GanttChart, GanttGroup, GanttState, GanttTask};
/// # let mut state = GanttState::default();
/// let mut g = Gantt { start: (2026, 11, 2), today: 8.0, groups: vec![GanttGroup::new("Design", vec![
///     GanttTask::new(1, "Wireframes", 0.0, 5.0).progress(1.0),
///     GanttTask::new(2, "Visual design", 5.0, 7.0).progress(0.4).after(&[1]),
/// ])] };
/// GanttChart::new(&mut g, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct GanttChart<'a> {
    data: &'a mut Gantt,
    state: &'a mut GanttState,
    height: f32,
    sidebar: f32,
}

const HEAD: f32 = 52.0;
const ROW: f32 = 31.0;
const GROUP_ROW: f32 = 30.0;

#[derive(Clone, Copy)]
enum RowKind {
    Group(usize),
    Task(usize, usize),
}

impl<'a> GanttChart<'a> {
    /// A chart for `data`.
    pub fn new(data: &'a mut Gantt, state: &'a mut GanttState) -> Self {
        Self { data, state, height: 480.0, sidebar: 230.0 }
    }
    /// Height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Width of the task-name sidebar.
    pub fn sidebar(mut self, w: f32) -> Self {
        self.sidebar = w;
        self
    }

    /// Show the chart. Returns `true` when a task changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let GanttChart { data, state: st, height, sidebar } = self;
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_gantt");
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return false;
        }
        let dt = glowkit_core::dt(ui);
        let mut changed = false;
        let painter = ui.painter_at(rect);
        painter.rect(rect, CornerRadius::same(12), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let side = Rect::from_min_max(rect.min, pos2(rect.left() + sidebar, rect.bottom()));
        let tl = Rect::from_min_max(pos2(side.right(), rect.top()), rect.max);
        let body = Rect::from_min_max(pos2(tl.left(), tl.top() + HEAD), tl.max);
        let pointer = ui.input(|i| i.pointer.hover_pos());

        // ---- zoom animation keeping the center day fixed
        let target_px = st.zoom.px();
        let center_day = st.scroll + tl.width() / 2.0 / st.px;
        if !st.init {
            let first = data.groups.iter().filter_map(|g| g.span()).map(|s| s.0).fold(data.today, f32::min);
            st.scroll = (first - 4.0).min(data.today - tl.width() / st.px * 0.3).floor();
            st.init = true;
        }
        let new_px = glowkit_core::anim::approach(st.px, target_px, 12.0, dt);
        if (new_px - target_px).abs() > 0.01 {
            st.px = new_px;
            st.scroll = center_day - tl.width() / 2.0 / st.px;
            ui.ctx().request_repaint();
        } else if st.px != target_px {
            st.px = target_px;
            st.scroll = center_day - tl.width() / 2.0 / st.px;
        }
        let px = st.px;

        // ---- rows
        let mut rows: Vec<(RowKind, f32, f32)> = Vec::new(); // kind, y (content), h
        let mut y = 0.0;
        for (gi, g) in data.groups.iter().enumerate() {
            rows.push((RowKind::Group(gi), y, GROUP_ROW));
            y += GROUP_ROW;
            if !g.collapsed {
                for ti in 0..g.tasks.len() {
                    rows.push((RowKind::Task(gi, ti), y, ROW));
                    y += ROW;
                }
            }
        }
        let content_h = y;
        let max_sy = (content_h - (body.height() - 8.0)).max(0.0);
        // ---- scroll / pan input
        if ui.rect_contains_pointer(rect) {
            let (d, zoom) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            if (zoom - 1.0).abs() > 1e-3 {
                let z = match st.zoom {
                    GanttZoom::Day if zoom < 1.0 => GanttZoom::Week,
                    GanttZoom::Week if zoom < 1.0 => GanttZoom::Month,
                    GanttZoom::Week if zoom > 1.0 => GanttZoom::Day,
                    GanttZoom::Month if zoom > 1.0 => GanttZoom::Week,
                    z => z,
                };
                if z != st.zoom && !ui.ctx().is_being_dragged(id.with("pan")) {
                    st.zoom = z;
                }
            }
            if d != Vec2::ZERO {
                if d.x.abs() > d.y.abs() || ui.input(|i| i.modifiers.shift) {
                    st.scroll -= (if d.x.abs() > d.y.abs() { d.x } else { d.y }) / px;
                } else if max_sy > 0.0 {
                    st.scroll_y -= d.y;
                } else {
                    st.scroll -= d.y / px;
                }
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        st.scroll_y = st.scroll_y.clamp(0.0, max_sy);
        let pan = ui.interact(body, id.with("pan"), Sense::click_and_drag());
        if pan.dragged() && st.drag.is_none() {
            st.scroll -= pan.drag_delta().x / px;
            st.scroll_y = (st.scroll_y - pan.drag_delta().y).clamp(0.0, max_sy);
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if pan.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        let (scroll, scroll_y) = (st.scroll, st.scroll_y);
        let day_of = move |x: f32| scroll + (x - tl.left()) / px;
        let row_y = move |yy: f32| body.top() + 4.0 + yy - scroll_y;

        // ---- grid: weekends + header
        let bp = painter.with_clip_rect(body);
        let first = st.scroll.floor() as i64 - 1;
        let last = (st.scroll + tl.width() / px).ceil() as i64 + 1;
        let base = days_from_civil(data.start.0, data.start.1, data.start.2);
        for d in first..=last {
            let x = tl.left() + (d as f32 - st.scroll) * px;
            let wd = weekday(base + d);
            if wd >= 5 && st.zoom != GanttZoom::Month {
                bp.rect_filled(Rect::from_min_max(pos2(x, body.top()), pos2(x + px, body.bottom())), CornerRadius::ZERO, with_alpha(t.foreground, if t.dark { 0.025 } else { 0.03 }));
            }
            let (_, _, dd) = civil_from_days(base + d);
            let line = match st.zoom {
                GanttZoom::Day => true,
                GanttZoom::Week => wd == 0,
                GanttZoom::Month => dd == 1,
            };
            if line {
                bp.line_segment([pos2(x, body.top()), pos2(x, body.bottom())], Stroke::new(1.0, with_alpha(t.border, 0.7)));
            }
        }
        // row separators + group bands
        for (kind, yy, h) in &rows {
            let ry = row_y(*yy);
            if let RowKind::Group(_) = kind {
                bp.rect_filled(Rect::from_min_size(pos2(body.left(), ry), vec2(body.width(), *h)), CornerRadius::ZERO, with_alpha(t.foreground, 0.02));
            }
            bp.line_segment([pos2(body.left(), ry + h), pos2(body.right(), ry + h)], Stroke::new(1.0, with_alpha(t.border, 0.45)));
        }
        // header
        let hp = painter.with_clip_rect(Rect::from_min_max(tl.min, pos2(tl.right(), tl.top() + HEAD)));
        hp.line_segment([pos2(rect.left(), tl.top() + HEAD), pos2(rect.right(), tl.top() + HEAD)], Stroke::new(1.0, t.border));
        hp.line_segment([pos2(tl.left(), tl.top() + HEAD / 2.0 + 2.0), pos2(tl.right(), tl.top() + HEAD / 2.0 + 2.0)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
        let top_y = tl.top() + 14.0;
        let bot_y = tl.top() + HEAD - 13.0;
        for d in first..=last {
            let x = tl.left() + (d as f32 - st.scroll) * px;
            let (yy, mm, dd) = civil_from_days(base + d);
            let wd = weekday(base + d);
            let is_today = d as f32 == data.today.floor();
            match st.zoom {
                GanttZoom::Day => {
                    let col = if is_today { t.accent } else if wd >= 5 { t.muted_foreground.gamma_multiply(0.6) } else { t.muted_foreground };
                    hp.text(pos2(x + px / 2.0, bot_y), Align2::CENTER_CENTER, format!("{} {}", ["M", "T", "W", "T", "F", "S", "S"][wd as usize], dd), qfont(11.0), col);
                    if dd == 1 || d == first {
                        hp.text(pos2(x.max(tl.left()) + 8.0, top_y), Align2::LEFT_CENTER, format!("{} {}", MONTHS[mm as usize - 1], yy), qfont(12.0), t.foreground);
                    }
                }
                GanttZoom::Week => {
                    if wd == 0 {
                        hp.line_segment([pos2(x, tl.top() + HEAD / 2.0 + 2.0), pos2(x, tl.top() + HEAD)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
                        hp.text(pos2(x + 6.0, bot_y), Align2::LEFT_CENTER, format!("{} {}", MONTHS[mm as usize - 1], dd), qfont(11.0), t.muted_foreground);
                    }
                    if dd == 1 || d == first {
                        hp.text(pos2(x.max(tl.left()) + 8.0, top_y), Align2::LEFT_CENTER, format!("{} {}", MONTHS[mm as usize - 1], yy), qfont(12.0), t.foreground);
                    }
                }
                GanttZoom::Month => {
                    if dd == 1 {
                        hp.line_segment([pos2(x, tl.top() + HEAD / 2.0 + 2.0), pos2(x, tl.top() + HEAD)], Stroke::new(1.0, with_alpha(t.border, 0.6)));
                        hp.text(pos2(x + 6.0, bot_y), Align2::LEFT_CENTER, MONTHS[mm as usize - 1], qfont(11.0), t.muted_foreground);
                        if mm == 1 || d == first {
                            hp.text(pos2(x.max(tl.left()) + 8.0, top_y), Align2::LEFT_CENTER, yy.to_string(), qfont(12.0), t.foreground);
                        }
                    } else if d == first {
                        hp.text(pos2(tl.left() + 8.0, top_y), Align2::LEFT_CENTER, yy.to_string(), qfont(12.0), t.foreground);
                    }
                }
            }
        }

        // ---- task geometry
        let mut bars: Vec<(u64, Rect, usize)> = Vec::new(); // id, rect, group
        for (kind, yy, h) in &rows {
            if let RowKind::Task(gi, ti) = kind {
                let task = &data.groups[*gi].tasks[*ti];
                let ry = row_y(*yy);
                let x0 = tl.left() + (task.start - st.scroll) * px;
                let x1 = tl.left() + (task.end() - st.scroll) * px;
                let r = if task.duration <= 0.0 { Rect::from_center_size(pos2(x0, ry + h / 2.0), Vec2::splat(16.0)) } else { Rect::from_min_max(pos2(x0, ry + 7.0), pos2(x1.max(x0 + 4.0), ry + h - 7.0)) };
                bars.push((task.id, r, *gi));
            }
        }
        let bar_of = |tid: u64| bars.iter().find(|b| b.0 == tid).map(|b| b.1);
        // interactions
        let mut hovered: Option<u64> = None;
        for (tid, r, _) in &bars {
            if !r.intersects(body) {
                continue;
            }
            let task = data.task(*tid).cloned().expect("task");
            let resp = ui.interact(r.expand2(vec2(4.0, 2.0)).intersect(body), id.with(("bar", *tid)), Sense::click_and_drag());
            let p = if resp.drag_started() { ui.input(|i| i.pointer.press_origin()).or(pointer).unwrap_or_default() } else { pointer.unwrap_or_default() };
            let edge = if task.duration > 0.0 && r.width() > 18.0 {
                if (p.x - r.left()).abs() < 6.0 {
                    Some(Mode::Left)
                } else if (p.x - r.right()).abs() < 6.0 {
                    Some(Mode::Right)
                } else if (p.y - r.bottom()).abs() < 5.0 && (p.x - (r.left() + r.width() * task.progress)).abs() < 7.0 {
                    Some(Mode::Progress)
                } else {
                    None
                }
            } else {
                None
            };
            if resp.hovered() {
                hovered = Some(*tid);
                ui.ctx().set_cursor_icon(match edge {
                    Some(Mode::Left | Mode::Right | Mode::Progress) => CursorIcon::ResizeHorizontal,
                    _ => CursorIcon::Grab,
                });
            }
            if resp.drag_started() {
                let pd = day_of(p.x);
                st.drag = Some(GDrag { task: *tid, mode: edge.unwrap_or(Mode::Move), grab_day: pd, start: task.start, duration: task.duration, progress: task.progress });
            }
        }
        if let Some(d) = st.drag {
            if let Some(p) = pointer {
                let delta = day_of(p.x) - d.grab_day;
                if let Some(task) = data.task_mut(d.task) {
                    let old = (task.start, task.duration, task.progress);
                    match d.mode {
                        Mode::Move => task.start = (d.start + delta).round(),
                        Mode::Left => {
                            let ns = (d.start + delta).round().min(d.start + d.duration - 1.0);
                            task.duration = d.duration + (d.start - ns);
                            task.start = ns;
                        }
                        Mode::Right => task.duration = (d.duration + delta).round().max(1.0),
                        Mode::Progress => task.progress = (d.progress + delta / d.duration.max(1.0)).clamp(0.0, 1.0),
                    }
                    if old != (task.start, task.duration, task.progress) {
                        changed = true;
                    }
                }
                ui.ctx().set_cursor_icon(if d.mode == Mode::Move { CursorIcon::Grabbing } else { CursorIcon::ResizeHorizontal });
            }
            if ui.input(|i| !i.pointer.primary_down()) {
                st.drag = None;
            }
        }
        let active = st.drag.map(|d| d.task).or(hovered);

        // ---- dependencies
        for g in &data.groups {
            for task in &g.tasks {
                for dep in &task.deps {
                    let (Some(a), Some(b)) = (bar_of(*dep), bar_of(task.id)) else { continue };
                    let hl = active == Some(task.id) || active == Some(*dep);
                    let bad = data.task(*dep).is_some_and(|d| d.end() > task.start + 1e-3);
                    let col = if bad { t.destructive } else if hl { t.accent } else { with_alpha(t.muted_foreground, 0.6) };
                    // leave from the bottom of the predecessor (keeps labels clear), enter from the left
                    let pb = b.left_center();
                    let ax = (a.right() - 8.0).max(a.left() + 4.0).min(a.right());
                    let pa = pos2(ax, a.bottom());
                    let pts = if pb.x - 10.0 >= ax {
                        vec![pa, pos2(ax, pb.y), pb]
                    } else {
                        let ym = if pb.y > pa.y { pa.y + 6.0 } else { pa.y - a.height() - 6.0 };
                        let pa = if pb.y > pa.y { pa } else { pos2(ax, a.top()) };
                        vec![pa, pos2(ax, ym), pos2(pb.x - 10.0, ym), pos2(pb.x - 10.0, pb.y), pb]
                    };
                    let path = common::round_corners(&pts, 5.0);
                    if bad {
                        common::dashed_polyline(&bp, &path, 4.0, 3.0, 0.0, Stroke::new(1.4, col));
                    } else {
                        bp.add(Shape::line(path.clone(), Stroke::new(if hl { 1.8 } else { 1.2 }, col)));
                    }
                    arrow_head(&bp, pb, Vec2::X, 6.0, col);
                }
            }
        }
        // ---- bars
        for (tid, r, gi) in &bars {
            if !r.intersects(body) {
                continue;
            }
            let task = data.task(*tid).expect("task");
            let g = &data.groups[*gi];
            let col = task.color.or(g.color).unwrap_or(t.chart[gi % t.chart.len()]);
            let hov = ui.ctx().animate_bool_with_time(id.with(("bh", *tid)), active == Some(*tid), 0.12);
            if task.duration <= 0.0 {
                let c = r.center();
                let s = 8.0 + hov * 1.5;
                bp.add(Shape::convex_polygon(vec![c - vec2(0.0, s), c + vec2(s, 0.0), c + vec2(0.0, s), c - vec2(s, 0.0)], col, Stroke::new(1.5, t.card)));
                bp.text(pos2(c.x + 14.0, c.y), Align2::LEFT_CENTER, &task.name, qfont(12.0), t.foreground);
                continue;
            }
            let rr = CornerRadius::same(6);
            if hov > 0.0 {
                glowkit_core::paint::soft_shadow(&bp, *r, 6.0, 10.0 * hov, vec2(0.0, 3.0), Color32::from_black_alpha(if t.dark { 100 } else { 30 }));
            }
            bp.rect(*r, rr, with_alpha(col, if t.dark { 0.22 } else { 0.18 }), Stroke::new(1.0, with_alpha(col, 0.55 + 0.45 * hov)), StrokeKind::Inside);
            if task.progress > 0.0 {
                let mut pr = *r;
                pr.set_width(r.width() * task.progress);
                bp.rect_filled(pr.shrink(1.0), CornerRadius::same(5), with_alpha(col, if t.dark { 0.7 } else { 0.55 }));
            }
            let label = format!("{}{}", task.name, if task.progress > 0.0 && task.progress < 1.0 { format!("  {:.0}%", task.progress * 100.0) } else { String::new() });
            let gw = bp.layout_no_wrap(label.clone(), qfont(12.0), Color32::WHITE).size().x;
            if gw + 16.0 < r.width() {
                bp.text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, label, qfont(12.0), t.foreground);
            } else {
                bp.text(pos2(r.right() + 8.0, r.center().y), Align2::LEFT_CENTER, label, qfont(12.0), t.foreground.gamma_multiply(0.9));
            }
            if !task.assignee.is_empty() && r.right() + 8.0 + gw + 30.0 < body.right() && gw + 16.0 < r.width() {
                common::avatar(&bp, pos2(r.right() + 14.0, r.center().y), 9.0, &task.assignee, common::color_for(&t, &task.assignee), t.card);
            }
            if hov > 0.05 {
                // grips + progress handle
                for x in [r.left() + 3.5, r.right() - 3.5] {
                    bp.line_segment([pos2(x, r.top() + 5.0), pos2(x, r.bottom() - 5.0)], Stroke::new(2.0, with_alpha(t.foreground, 0.5 * hov)));
                }
                let hx = r.left() + r.width() * task.progress;
                let hy = r.bottom();
                bp.add(Shape::convex_polygon(vec![pos2(hx, hy - 3.0), pos2(hx + 5.0, hy + 4.0), pos2(hx - 5.0, hy + 4.0)], with_alpha(t.foreground, hov), Stroke::NONE));
            }
        }
        // group summary bars
        for (kind, yy, h) in &rows {
            if let RowKind::Group(gi) = kind {
                let g = &data.groups[*gi];
                if let Some((s, e)) = g.span() {
                    let ry = row_y(*yy) + h / 2.0;
                    let x0 = tl.left() + (s - st.scroll) * px;
                    let x1 = tl.left() + (e - st.scroll) * px;
                    let col = g.color.unwrap_or(t.chart[gi % t.chart.len()]);
                    let done: f32 = g.tasks.iter().map(|k| k.progress * k.duration).sum::<f32>() / g.tasks.iter().map(|k| k.duration).sum::<f32>().max(1.0);
                    bp.rect_filled(Rect::from_min_max(pos2(x0, ry - 3.0), pos2(x1, ry + 3.0)), CornerRadius::same(3), with_alpha(col, 0.3));
                    bp.rect_filled(Rect::from_min_max(pos2(x0, ry - 3.0), pos2(x0 + (x1 - x0) * done, ry + 3.0)), CornerRadius::same(3), col);
                    for x in [x0, x1] {
                        bp.add(Shape::convex_polygon(vec![pos2(x - 4.0, ry - 3.0), pos2(x + 4.0, ry - 3.0), pos2(x, ry + 6.0)], col, Stroke::NONE));
                    }
                }
            }
        }
        // ---- today line
        let tx = tl.left() + (data.today - st.scroll) * px;
        if tx > tl.left() && tx < tl.right() {
            let pulse = 0.5 + 0.5 * (glowkit_core::time(ui) * 2.0).sin();
            painter.line_segment([pos2(tx, tl.top() + HEAD), pos2(tx, rect.bottom() - 1.0)], Stroke::new(1.5, t.accent));
            glowkit_core::paint::glow(&painter, pos2(tx, tl.top() + HEAD), 10.0 + 4.0 * pulse, with_alpha(t.accent, 0.35));
            common::pill(&painter, pos2(tx, tl.top() + HEAD), "Today", qfont(10.5), t.accent_foreground, t.accent, Stroke::NONE);
            glowkit_core::animate(ui);
        }

        // ---- sidebar
        painter.rect_filled(Rect::from_min_max(side.min + vec2(1.0, 1.0), pos2(side.right(), side.bottom() - 1.0)), CornerRadius { nw: 11, sw: 11, ne: 0, se: 0 }, t.card);
        painter.line_segment([side.right_top(), side.right_bottom()], Stroke::new(1.0, t.border));
        painter.text(pos2(side.left() + 14.0, side.top() + HEAD / 2.0), Align2::LEFT_CENTER, "Task", qfont(12.0), t.muted_foreground);
        // zoom segmented control
        let seg = Rect::from_min_size(pos2(side.right() - 132.0, side.top() + 12.0), vec2(122.0, 26.0));
        painter.rect_filled(seg, CornerRadius::same(7), t.muted);
        let zooms = [(GanttZoom::Day, "Day"), (GanttZoom::Week, "Week"), (GanttZoom::Month, "Month")];
        let cur = zooms.iter().position(|z| z.0 == st.zoom).unwrap_or(1) as f32;
        let ind = glowkit_core::anim::follow(ui, id.with("seg"), cur, 16.0);
        let sw = seg.width() / 3.0;
        let ir = Rect::from_min_size(pos2(seg.left() + ind * sw, seg.top()), vec2(sw, seg.height())).shrink(2.0);
        painter.rect(ir, CornerRadius::same(5), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
        for (k, (z, label)) in zooms.iter().enumerate() {
            let r = Rect::from_min_size(pos2(seg.left() + k as f32 * sw, seg.top()), vec2(sw, seg.height()));
            let resp = ui.interact(r, id.with(("z", k)), Sense::click());
            painter.text(r.center(), Align2::CENTER_CENTER, *label, qfont(11.5), if st.zoom == *z { t.foreground } else { t.muted_foreground });
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                st.zoom = *z;
            }
        }
        let sp = painter.with_clip_rect(Rect::from_min_max(pos2(side.left(), side.top() + HEAD), side.max));
        let mut toggle = None;
        for (kind, yy, h) in &rows {
            let ry = row_y(*yy);
            let r = Rect::from_min_size(pos2(side.left(), ry), vec2(sidebar, *h));
            if !r.intersects(body) {
                continue;
            }
            match kind {
                RowKind::Group(gi) => {
                    let g = &data.groups[*gi];
                    let resp = ui.interact(r.intersect(Rect::from_min_max(pos2(side.left(), body.top()), side.max)), id.with(("grp", *gi)), Sense::click());
                    let hv = ui.ctx().animate_bool_with_time(id.with(("gh", *gi)), resp.hovered(), 0.1);
                    sp.rect_filled(r, CornerRadius::ZERO, with_alpha(t.foreground, 0.02 + 0.03 * hv));
                    let rot = ui.ctx().animate_bool_with_time(id.with(("gc", *gi)), g.collapsed, 0.15);
                    let ic = Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), Vec2::splat(12.0));
                    let pts: Vec<Pos2> = [(3.5f32, 6.0f32), (8.0, 10.5), (12.5, 6.0)]
                        .iter()
                        .map(|&(x, yv)| common::rotate_about(ic.min + vec2(x, yv) * ic.width() / 16.0, ic.center(), -rot * std::f32::consts::FRAC_PI_2))
                        .collect();
                    sp.add(Shape::line(pts, Stroke::new(1.5, t.muted_foreground)));
                    let col = g.color.unwrap_or(t.chart[gi % t.chart.len()]);
                    sp.circle_filled(pos2(r.left() + 32.0, r.center().y), 4.0, col);
                    sp.text(pos2(r.left() + 42.0, r.center().y), Align2::LEFT_CENTER, &g.name, qfont(13.0), t.foreground);
                    sp.text(pos2(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, g.tasks.len().to_string(), qfont(11.0), t.muted_foreground);
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        toggle = Some(*gi);
                    }
                }
                RowKind::Task(gi, ti) => {
                    let task = &data.groups[*gi].tasks[*ti];
                    let hl = active == Some(task.id);
                    if hl {
                        sp.rect_filled(r, CornerRadius::ZERO, with_alpha(t.accent, 0.08));
                        bp.rect_filled(Rect::from_min_size(pos2(body.left(), ry), vec2(body.width(), *h)), CornerRadius::ZERO, with_alpha(t.accent, 0.04));
                    }
                    let name = common::elide(&sp, &task.name, qfont(12.5), sidebar - 100.0);
                    sp.text(pos2(r.left() + 32.0, r.center().y), Align2::LEFT_CENTER, name, qfont(12.5), t.foreground.gamma_multiply(0.92));
                    let (_, m, d) = data.date(task.start);
                    sp.text(pos2(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, format!("{} {}", MONTHS[m as usize - 1], d), qfont(11.0), t.muted_foreground);
                    if data.violates(task.id) {
                        sp.circle_filled(pos2(r.left() + 20.0, r.center().y), 3.0, t.destructive);
                    }
                }
            }
            sp.line_segment([pos2(side.left(), ry + h), pos2(side.right(), ry + h)], Stroke::new(1.0, with_alpha(t.border, 0.45)));
        }
        if let Some(gi) = toggle {
            data.groups[gi].collapsed = !data.groups[gi].collapsed;
        }
        // tooltip for the dragged task
        if let (Some(d), Some(p)) = (st.drag, pointer) {
            if let Some(task) = data.task(d.task) {
                let (y0, m0, d0) = data.date(task.start);
                let (_, m1, d1) = data.date(task.end() - 1.0);
                let txt = match d.mode {
                    Mode::Progress => format!("{:.0}% done", task.progress * 100.0),
                    _ => format!("{} {} – {} {}, {}  ·  {}d", MONTHS[m0 as usize - 1], d0, MONTHS[m1 as usize - 1], d1, y0, task.duration),
                };
                common::pill(&painter, p + vec2(0.0, -26.0), &txt, qfont(11.5), t.foreground, t.popover, Stroke::new(1.0, t.border));
            }
        }
        let _ = Icon::Dots;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        for z in [-1000, -1, 1, 59, 365, 10957, 20000, 20768] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29) + 1), (2024, 3, 1));
    }

    #[test]
    fn weekdays() {
        assert_eq!(weekday(0), 3); // 1970-01-01 was a Thursday
        assert_eq!(weekday(days_from_civil(2026, 11, 9)), 0); // a Monday
    }

    #[test]
    fn dependency_violation() {
        let g = Gantt {
            start: (2026, 1, 1),
            today: 0.0,
            groups: vec![GanttGroup::new("g", vec![GanttTask::new(1, "a", 0.0, 5.0), GanttTask::new(2, "b", 3.0, 2.0).after(&[1]), GanttTask::new(3, "c", 5.0, 2.0).after(&[1])])],
        };
        assert!(g.violates(2));
        assert!(!g.violates(3));
        assert_eq!(g.groups[0].span(), Some((0.0, 7.0)));
    }
}
