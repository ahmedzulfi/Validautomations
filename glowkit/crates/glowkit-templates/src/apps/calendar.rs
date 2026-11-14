//! Calendar app template: month and week views with events.

use egui::{pos2, vec2, Align2, CursorIcon, Key, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, Button, ButtonVariant, Calendar, CalendarDate, Checkbox, ControlSize, LineIcon, SegmentedControl, Toast, UiIcon};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// An event in the [`CalendarApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct CalEvent {
    /// Day.
    pub date: CalendarDate,
    /// Start hour (e.g. 9.5 = 09:30).
    pub start: f32,
    /// Duration in hours.
    pub hours: f32,
    /// Title.
    pub title: String,
    /// Calendar index (color / filter).
    pub calendar: usize,
}

/// A calendar app inside an inset [`AppShell`]: a mini month picker, toggleable calendars,
/// Month / Week views (M / W), Today and ← / → navigation, event chips with "+n more", week
/// columns with positioned event blocks and a now line, and click-to-create in the week grid.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::CalendarApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct CalendarApp {
    /// Active route.
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Events.
    pub events: Vec<CalEvent>,
    /// The date the views are anchored on ("today" in the demo data).
    pub today: CalendarDate,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    view: usize,
    focus: Option<CalendarDate>,
    shown: [bool; 4],
}

const CALS: [&str; 4] = ["Work", "Personal", "Team", "Holidays"];

impl Default for CalendarApp {
    fn default() -> Self {
        let today = CalendarDate::new(2026, 11, 14);
        let ev = |d: i64, s: f32, h: f32, t: &str, c: usize| CalEvent { date: today.add_days(d), start: s, hours: h, title: t.into(), calendar: c };
        Self {
            route: "calendar".into(),
            nav: vec![
                NavSection::new("Plan")
                    .item(NavItem::new("calendar", "Calendar").icon(LineIcon::Calendar))
                    .item(NavItem::new("tasks", "Tasks").icon(AppIcon::ListChecks).badge("4"))
                    .item(NavItem::new("bookings", "Booking pages").icon(UiIcon::Link)),
                NavSection::new("Workspace").item(NavItem::new("people", "People").icon(LineIcon::Users)).item(NavItem::new("rooms", "Rooms").icon(AppIcon::Building)),
            ],
            notifications: demo_notes(),
            events: vec![
                ev(0, 9.0, 1.0, "Design review", 0),
                ev(0, 12.5, 1.0, "Lunch with Mia", 1),
                ev(0, 15.0, 1.5, "Pricing workshop", 2),
                ev(-1, 10.0, 0.5, "Standup", 2),
                ev(-2, 10.0, 0.5, "Standup", 2),
                ev(-2, 14.0, 2.0, "Customer interviews", 0),
                ev(-3, 10.0, 0.5, "Standup", 2),
                ev(-3, 16.0, 1.0, "1:1 Grace", 0),
                ev(-4, 10.0, 0.5, "Standup", 2),
                ev(-4, 11.0, 1.0, "Roadmap sync", 0),
                ev(1, 8.0, 1.5, "Morning run", 1),
                ev(1, 18.0, 2.5, "Dinner party", 1),
                ev(2, 10.0, 0.5, "Standup", 2),
                ev(2, 13.0, 3.0, "Launch rehearsal", 0),
                ev(3, 10.0, 0.5, "Standup", 2),
                ev(4, 9.0, 8.0, "Offsite day", 2),
                ev(12, 9.0, 8.0, "Thanksgiving", 3),
                ev(5, 15.0, 1.0, "Dentist", 1),
                ev(-8, 11.0, 1.0, "Board prep", 0),
                ev(-10, 19.0, 2.0, "Concert", 1),
                ev(8, 14.0, 1.0, "Quarterly review", 0),
                ev(10, 10.0, 1.0, "Hiring panel", 2),
            ],
            today,
            scoped_theme: false,
            view: 1,
            focus: Some(today),
            shown: [true; 4],
        }
    }
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DOW: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn hour_label(h: f32) -> String {
    let hh = h.floor() as u32;
    let mm = ((h - h.floor()) * 60.0).round() as u32;
    let (h12, ap) = if hh == 0 { (12, "AM") } else if hh < 12 { (hh, "AM") } else if hh == 12 { (12, "PM") } else { (hh - 12, "PM") };
    if mm == 0 {
        format!("{h12} {ap}")
    } else {
        format!("{h12}:{mm:02} {ap}")
    }
}

impl CalendarApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .workspaces(demo_workspaces("Acme Calendar", "Workspace"))
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("calendar_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "calendar" => self.page(ui, sc.rect),
                other => placeholder(ui, sc.rect.shrink(24.0), other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn page(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("cal");
        let mut focus = self.focus.unwrap_or(self.today);
        if !ui.memory(|m| m.focused().is_some()) {
            let (l, r, m, w, tt) = ui.input(|i| (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::M), i.key_pressed(Key::W), i.key_pressed(Key::T)));
            let step = if self.view == 0 { 30 } else { 7 };
            if l {
                focus = focus.add_days(-step);
            }
            if r {
                focus = focus.add_days(step);
            }
            if m {
                self.view = 0;
            }
            if w {
                self.view = 1;
            }
            if tt {
                focus = self.today;
            }
        }
        let left_w = if rect.width() > 900.0 { 304.0 } else { 0.0 };
        let parts = kit::split_x(rect, &[left_w, 0.0], 0.0);
        let p = ui.painter().clone();
        if left_w > 0.0 {
            kit::vline(&p, parts[0].max.x, rect.min.y, rect.max.y, &t);
            let lr = parts[0].shrink(16.0);
            if kit::put(ui, Rect::from_min_size(lr.min, vec2(lr.width(), 34.0)), Button::new("New event").icon(UiIcon::Plus).size(ControlSize::Sm).width(lr.width())).clicked() {
                self.add_event(ui, focus, 11.0);
            }
            let cr = Rect::from_min_size(lr.min + vec2(0.0, 46.0), vec2(lr.width(), 290.0));
            let mut sel = Some(focus);
            kit::child(ui, cr, |ui| {
                ui.add(Calendar::new(&mut sel).today(self.today).framed(false).id_salt("cal_mini"));
            });
            if let Some(s) = sel {
                focus = s;
            }
            let mut y = cr.max.y + 12.0;
            kit::text_at(&p, pos2(lr.min.x, y), Align2::LEFT_CENTER, "My calendars", 12.5, t.muted_foreground, 200.0);
            y += 14.0;
            for (i, name) in CALS.iter().enumerate() {
                let r = Rect::from_min_size(pos2(lr.min.x, y), vec2(lr.width(), 28.0));
                y += 30.0;
                kit::child(ui, Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(lr.width() - 40.0, 22.0)), |ui| ui.add(Checkbox::new(&mut self.shown[i]).label(*name).color(t.chart[i]).size(16.0)));
                let n = self.events.iter().filter(|e| e.calendar == i).count();
                kit::text_at(&p, pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, &n.to_string(), 12.0, t.muted_foreground, 40.0);
            }
        }
        // header
        let main = parts[1];
        let hr = Rect::from_min_size(main.min, vec2(main.width(), 60.0));
        let title = if self.view == 0 { format!("{} {}", MONTHS[focus.month as usize - 1], focus.year) } else {
            let ws = focus.add_days(-(focus.weekday() as i64));
            let we = ws.add_days(6);
            if ws.month == we.month { format!("{} {}–{}, {}", &MONTHS[ws.month as usize - 1][..3], ws.day, we.day, ws.year) } else { format!("{} {} – {} {}, {}", &MONTHS[ws.month as usize - 1][..3], ws.day, &MONTHS[we.month as usize - 1][..3], we.day, we.year) }
        };
        let cy = hr.center().y;
        if kit::put(ui, Rect::from_min_size(pos2(hr.min.x + 18.0, cy - 16.0), vec2(70.0, 32.0)), Button::new("Today").variant(ButtonVariant::Outline).size(ControlSize::Sm).width(70.0)).clicked() {
            focus = self.today;
        }
        let step = if self.view == 0 { 30 } else { 7 };
        let pv = kit::icon_button(ui, Rect::from_center_size(pos2(hr.min.x + 112.0, cy), vec2(30.0, 30.0)), id.with("prev"), LineIcon::ChevronLeft.into(), false);
        kit::tip(&pv, "Previous", Some("←"));
        if pv.clicked() {
            focus = focus.add_days(-step);
        }
        let nx = kit::icon_button(ui, Rect::from_center_size(pos2(hr.min.x + 144.0, cy), vec2(30.0, 30.0)), id.with("next"), LineIcon::ChevronRight.into(), false);
        kit::tip(&nx, "Next", Some("→"));
        if nx.clicked() {
            focus = focus.add_days(step);
        }
        kit::text_at(&p, pos2(hr.min.x + 170.0, cy), Align2::LEFT_CENTER, &title, 18.0, t.foreground, hr.width() - 420.0);
        let sr = Rect::from_min_size(pos2(hr.max.x - 18.0 - 180.0, cy - 16.0), vec2(180.0, 32.0));
        let segr = kit::put(ui, sr, SegmentedControl::new(&mut self.view, ["Month", "Week"]).size(ControlSize::Sm).width(180.0));
        kit::tip(&segr, "Switch view", Some("M / W"));
        kit::hline(&p, main.min.x, main.max.x, hr.max.y, &t);
        self.focus = Some(focus);
        let grid = Rect::from_min_max(pos2(main.min.x, hr.max.y), main.max);
        let loading = kit::loading(ui, id.with(self.view), 0.4);
        if self.view == 0 {
            self.month(ui, grid, focus, loading);
        } else {
            self.week(ui, grid, focus, loading);
        }
    }

    fn add_event(&mut self, ui: &Ui, date: CalendarDate, start: f32) {
        self.events.push(CalEvent { date, start, hours: 1.0, title: "New event".into(), calendar: 0 });
        toast(ui.ctx(), Toast::success("Event created").description(format!("{} · {}", date.format_short(), hour_label(start))).action("Undo"));
    }

    fn visible(&self, d: CalendarDate) -> Vec<&CalEvent> {
        let mut v: Vec<&CalEvent> = self.events.iter().filter(|e| e.date == d && self.shown[e.calendar]).collect();
        v.sort_by(|a, b| a.start.total_cmp(&b.start));
        v
    }

    fn month(&mut self, ui: &mut Ui, grid: Rect, focus: CalendarDate, loading: bool) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("cal_month");
        let p = ui.painter().clone();
        let first = CalendarDate::new(focus.year, focus.month, 1);
        let start = first.add_days(-(first.weekday() as i64));
        let head = Rect::from_min_size(grid.min, vec2(grid.width(), 30.0));
        let cw = grid.width() / 7.0;
        for (k, d) in DOW.iter().enumerate() {
            kit::text_at(&p, pos2(head.min.x + cw * (k as f32 + 0.5), head.center().y), Align2::CENTER_CENTER, d, 12.0, t.muted_foreground, cw);
        }
        let body = Rect::from_min_max(pos2(grid.min.x, head.max.y), grid.max);
        let rh = body.height() / 6.0;
        let mut pick = None;
        for i in 0..42 {
            let d = start.add_days(i);
            let (row, col) = (i / 7, i % 7);
            let cell = Rect::from_min_size(pos2(body.min.x + col as f32 * cw, body.min.y + row as f32 * rh), vec2(cw, rh));
            let in_month = d.month == focus.month;
            let cid = id.with(i);
            let resp = ui.interact(cell, cid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, cid, resp.hovered());
            if !in_month {
                p.rect_filled(cell, 0.0, with_alpha(t.muted, 0.35));
            }
            if hv > 0.01 {
                p.rect_filled(cell, 0.0, kit::hover_fill(&t, hv));
            }
            p.rect_stroke(cell, 0.0, Stroke::new(1.0, with_alpha(t.border, 0.8)), StrokeKind::Inside);
            let today = d == self.today;
            let nc = pos2(cell.min.x + 18.0, cell.min.y + 16.0);
            if today {
                p.circle_filled(nc, 12.0, t.accent);
            }
            kit::text_at(&p, nc, Align2::CENTER_CENTER, &d.day.to_string(), 12.5, if today { t.accent_foreground } else if in_month { t.foreground } else { t.muted_foreground }, 30.0);
            if loading {
                if (i * 7) % 5 < 2 {
                    kit::skel(ui, Rect::from_min_size(cell.min + vec2(6.0, 34.0), vec2(cw - 12.0, 16.0)));
                }
                continue;
            }
            let evs = self.visible(d);
            let max = (((rh - 34.0) / 20.0).floor() as usize).max(1);
            for (k, e) in evs.iter().enumerate() {
                let er = Rect::from_min_size(pos2(cell.min.x + 5.0, cell.min.y + 32.0 + k as f32 * 20.0), vec2(cw - 10.0, 18.0));
                if k + 1 == max && evs.len() > max {
                    kit::text_at(&p, er.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, &format!("+{} more", evs.len() - k), 11.5, t.muted_foreground, cw - 20.0);
                    break;
                }
                let c = t.chart[e.calendar];
                p.rect_filled(er, 4.0, with_alpha(c, if t.dark { 0.2 } else { 0.14 }));
                p.rect_filled(Rect::from_min_size(er.min, vec2(3.0, er.height())), 1.5, c);
                kit::text_at(&p, er.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, &format!("{} {}", hour_label(e.start).replace(" AM", "a").replace(" PM", "p"), e.title), 11.5, if t.dark { mix(c, egui::Color32::WHITE, 0.55) } else { glowkit_core::color::shade(c, -0.3) }, cw - 20.0);
            }
            if resp.double_clicked() {
                pick = Some((d, true));
            } else if resp.clicked() {
                pick = Some((d, false));
            }
        }
        if let Some((d, create)) = pick {
            self.focus = Some(d);
            if create {
                self.add_event(ui, d, 9.0);
            } else {
                self.view = 1;
            }
        }
    }

    fn week(&mut self, ui: &mut Ui, grid: Rect, focus: CalendarDate, loading: bool) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("cal_week");
        let ws = focus.add_days(-(focus.weekday() as i64));
        let gutter = 56.0;
        let head = Rect::from_min_size(grid.min, vec2(grid.width(), 54.0));
        let cw = (grid.width() - gutter) / 7.0;
        let p = ui.painter().clone();
        for k in 0..7 {
            let d = ws.add_days(k);
            let x = head.min.x + gutter + cw * (k as f32 + 0.5);
            let today = d == self.today;
            kit::text_at(&p, pos2(x, head.min.y + 16.0), Align2::CENTER_CENTER, DOW[k as usize], 11.5, if today { t.accent } else { t.muted_foreground }, cw);
            if today {
                p.circle_filled(pos2(x, head.min.y + 36.0), 14.0, t.accent);
            }
            kit::text_at(&p, pos2(x, head.min.y + 36.0), Align2::CENTER_CENTER, &d.day.to_string(), 15.0, if today { t.accent_foreground } else { t.foreground }, cw);
        }
        kit::hline(&p, grid.min.x, grid.max.x, head.max.y, &t);
        let body = Rect::from_min_max(pos2(grid.min.x, head.max.y), grid.max);
        let (h0, h1) = (7.0f32, 21.0f32);
        let mut create = None;
        kit::child(ui, body, |ui| {
            kit::scroll(ui, "cal_week_scroll", |ui| {
                let hh = 52.0;
                let area = kit::alloc_row(ui, (h1 - h0) * hh);
                let pp = ui.painter().clone();
                for h in 0..=((h1 - h0) as i32) {
                    let y = area.min.y + h as f32 * hh;
                    pp.line_segment([pos2(area.min.x + gutter, y), pos2(area.max.x, y)], Stroke::new(1.0, with_alpha(t.border, 0.7)));
                    if h > 0 {
                        kit::text_at(&pp, pos2(area.min.x + gutter - 8.0, y), Align2::RIGHT_CENTER, &hour_label(h0 + h as f32), 11.0, t.muted_foreground, gutter);
                    }
                }
                for k in 0..7 {
                    let x = area.min.x + gutter + cw * k as f32;
                    pp.line_segment([pos2(x, area.min.y), pos2(x, area.max.y)], Stroke::new(1.0, with_alpha(t.border, 0.7)));
                    let d = ws.add_days(k);
                    let col = Rect::from_min_max(pos2(x, area.min.y), pos2(x + cw, area.max.y));
                    if d == self.today {
                        pp.rect_filled(col, 0.0, with_alpha(t.accent, 0.04));
                    }
                    let cresp = ui.interact(col, id.with(("col", k)), Sense::click());
                    if cresp.clicked() {
                        if let Some(pos) = cresp.interact_pointer_pos() {
                            let hour = (h0 + ((pos.y - area.min.y) / hh * 2.0).floor() / 2.0).clamp(h0, h1 - 1.0);
                            create = Some((d, hour));
                        }
                    }
                    if loading {
                        if k % 2 == 0 {
                            kit::skel_r(ui, Rect::from_min_size(pos2(x + 4.0, area.min.y + hh * (2.0 + k as f32 * 0.5)), vec2(cw - 8.0, hh * 1.2)), 6.0);
                        }
                        continue;
                    }
                    for (ei, e) in self.visible(d).iter().enumerate() {
                        let top = area.min.y + (e.start.max(h0) - h0) * hh;
                        let bot = area.min.y + ((e.start + e.hours).min(h1) - h0) * hh;
                        let er = Rect::from_min_max(pos2(x + 3.0, top + 1.5), pos2(x + cw - 3.0, bot - 1.5));
                        let c = t.chart[e.calendar];
                        let eid = id.with(("ev", k, ei));
                        let resp = ui.interact(er, eid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        let hv = kit::hover(ui, eid, resp.hovered());
                        pp.rect_filled(er, 6.0, with_alpha(c, if t.dark { 0.24 + 0.08 * hv } else { 0.16 + 0.08 * hv }));
                        pp.rect_filled(Rect::from_min_size(er.min, vec2(3.0, er.height())), 1.5, c);
                        let fg = if t.dark { mix(c, egui::Color32::WHITE, 0.6) } else { glowkit_core::color::shade(c, -0.35) };
                        kit::text_at(&pp, er.min + vec2(9.0, 11.0), Align2::LEFT_CENTER, &e.title, 12.0, fg, er.width() - 14.0);
                        if er.height() > 34.0 {
                            kit::text_at(&pp, er.min + vec2(9.0, 27.0), Align2::LEFT_CENTER, &format!("{} – {}", hour_label(e.start), hour_label(e.start + e.hours)), 11.0, with_alpha(fg, 0.8), er.width() - 14.0);
                        }
                        if resp.clicked() {
                            toast(ui.ctx(), Toast::new(e.title.clone()).description(format!("{} · {} – {} · {}", e.date.format_short(), hour_label(e.start), hour_label(e.start + e.hours), CALS[e.calendar])));
                        }
                    }
                }
                // now line (demo clock: 10:40 on "today")
                let ti = self.today.to_days() - ws.to_days();
                if (0..7).contains(&ti) {
                    let y = area.min.y + (10.66 - h0) * hh;
                    let x0 = area.min.x + gutter + cw * ti as f32;
                    pp.line_segment([pos2(x0, y), pos2(x0 + cw, y)], Stroke::new(2.0, t.destructive));
                    pp.circle_filled(pos2(x0, y), 4.5, t.destructive);
                }
            });
        });
        if let Some((d, h)) = create {
            self.add_event(ui, d, h);
        }
    }
}
