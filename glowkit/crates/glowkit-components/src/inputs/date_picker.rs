use super::kit::{self, control_radius, UiIcon};
use egui::{pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// A plain calendar date (proleptic Gregorian), ordered chronologically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    /// Year, e.g. 2026.
    pub year: i32,
    /// Month 1..=12.
    pub month: u32,
    /// Day of month 1..=31.
    pub day: u32,
}

impl CalendarDate {
    /// Construct a date (not validated beyond clamping the day to the month length).
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Self { year, month, day: day.clamp(1, days_in_month(year, month)) }
    }

    /// Today's date from the system clock on native targets; `None` on the web (pass it in
    /// with [`Calendar::today`]).
    pub fn today() -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
            Some(Self::from_days(secs.div_euclid(86_400)))
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    /// Date from days since 1970-01-01.
    pub fn from_days(z: i64) -> Self {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        Self { year: (y + if m <= 2 { 1 } else { 0 }) as i32, month: m, day: d }
    }

    /// Days since 1970-01-01.
    pub fn to_days(self) -> i64 {
        let y = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Day of week, 0 = Sunday.
    pub fn weekday(self) -> u32 {
        (self.to_days() + 4).rem_euclid(7) as u32
    }

    /// The date `n` days later (negative for earlier).
    pub fn add_days(self, n: i64) -> Self {
        Self::from_days(self.to_days() + n)
    }

    /// Short human format, e.g. "Nov 11, 2026".
    pub fn format_short(self) -> String {
        format!("{} {}, {}", &MONTHS[(self.month - 1) as usize][..3], self.day, self.year)
    }
}

impl std::fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Number of days in a month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

fn add_month(y: i32, m: u32, delta: i32) -> (i32, u32) {
    let idx = y * 12 + m as i32 - 1 + delta;
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32)
}

enum Sel<'a> {
    Single(&'a mut Option<CalendarDate>),
    Range(&'a mut Option<CalendarDate>, &'a mut Option<CalendarDate>),
}

#[derive(Clone, Copy)]
struct CalState {
    year: i32,
    month: u32,
    dir: f32,
    changed_at: f64,
}

const CELL: f32 = 36.0;

/// A month-grid calendar with today marker, single-date or range selection, multi-month layout
/// and a sliding month transition.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut day = None;
/// use glowkit_components::{Calendar, CalendarDate};
/// ui.add(Calendar::new(&mut day).today(CalendarDate::new(2026, 11, 11)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Calendar<'a> {
    sel: Sel<'a>,
    today: Option<CalendarDate>,
    months: usize,
    framed: bool,
    id_salt: Option<Id>,
}

impl<'a> Calendar<'a> {
    /// Single-date calendar.
    pub fn new(selected: &'a mut Option<CalendarDate>) -> Self {
        Self { sel: Sel::Single(selected), today: CalendarDate::today(), months: 1, framed: true, id_salt: None }
    }
    /// Range calendar: first click sets the start, second the end.
    pub fn range(start: &'a mut Option<CalendarDate>, end: &'a mut Option<CalendarDate>) -> Self {
        Self { sel: Sel::Range(start, end), today: CalendarDate::today(), months: 1, framed: true, id_salt: None }
    }
    /// Override "today" (needed on the web, handy for tests).
    pub fn today(mut self, d: CalendarDate) -> Self {
        self.today = Some(d);
        self
    }
    /// Show this many months side by side (default 1).
    pub fn months(mut self, n: usize) -> Self {
        self.months = n.clamp(1, 3);
        self
    }
    /// Draw a bordered card around the calendar (default on).
    pub fn framed(mut self, f: bool) -> Self {
        self.framed = f;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

fn nav_button(ui: &mut Ui, rect: Rect, id: Id, icon: UiIcon) -> Response {
    let t = Theme::of(ui);
    let r = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let h = kit::anim(ui, id.with("h"), r.hovered(), 0.1);
    ui.painter().rect(rect, CornerRadius::same(6), if h > 0.0 { with_alpha(t.secondary, h) } else { Color32::TRANSPARENT }, Stroke::new(1.0, t.input), StrokeKind::Inside);
    icon.paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(15.0, 15.0)), t.foreground);
    r
}

impl Widget for Calendar<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = match self.id_salt {
            Some(s) => ui.make_persistent_id(s),
            None => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        };
        let anchor = match &self.sel {
            Sel::Single(s) => **s,
            Sel::Range(a, _) => **a,
        }
        .or(self.today)
        .unwrap_or(CalendarDate::new(2026, 1, 1));
        let mut st: CalState = glowkit_core::state(ui, id, || CalState { year: anchor.year, month: anchor.month, dir: 0.0, changed_at: -10.0 });
        let pad = 12.0;
        let gap = 24.0;
        let month_w = CELL * 7.0;
        let w = pad * 2.0 + month_w * self.months as f32 + gap * (self.months - 1) as f32;
        let h = pad * 2.0 + 32.0 + 8.0 + 24.0 + CELL * 6.0;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if self.framed {
            ui.painter().rect(rect, t.corner(), t.card, t.border_stroke(), StrokeKind::Inside);
        }
        let now = ui.input(|i| i.time);
        // header nav
        let prev = Rect::from_min_size(rect.min + vec2(pad, pad + 2.0), vec2(28.0, 28.0));
        let next = Rect::from_min_size(pos2(rect.max.x - pad - 28.0, rect.min.y + pad + 2.0), vec2(28.0, 28.0));
        if nav_button(ui, prev, id.with("prev"), UiIcon::ChevronLeft).clicked() {
            (st.year, st.month) = add_month(st.year, st.month, -1);
            st.dir = -1.0;
            st.changed_at = now;
        }
        if nav_button(ui, next, id.with("next"), UiIcon::ChevronRight).clicked() {
            (st.year, st.month) = add_month(st.year, st.month, 1);
            st.dir = 1.0;
            st.changed_at = now;
        }
        let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, ((now - st.changed_at) / 0.3) as f32);
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let slide = (1.0 - k) * 18.0 * st.dir;
        let alpha = 0.25 + 0.75 * k;
        let range_hover_id = id.with("hover");
        let hovered_day: Option<CalendarDate> = glowkit_core::state(ui, range_hover_id, || None);
        let mut new_hover = None;
        let mut clicked_day = None;
        let (sel_a, sel_b) = match &self.sel {
            Sel::Single(s) => (**s, **s),
            Sel::Range(a, b) => (**a, **b),
        };
        let is_range = matches!(self.sel, Sel::Range(..));
        // preview range while choosing the end
        let (ra, rb) = match (sel_a, sel_b, hovered_day) {
            (Some(a), None, Some(hv)) if is_range => (Some(a.min(hv)), Some(a.max(hv))),
            (a, b, _) => (a, b),
        };
        for mi in 0..self.months {
            let (y, m) = add_month(st.year, st.month, mi as i32);
            let ox = rect.min.x + pad + mi as f32 * (month_w + gap);
            let painter = ui.painter().with_clip_rect(Rect::from_min_size(pos2(ox - 4.0, rect.min.y), vec2(month_w + 8.0, h)).intersect(ui.clip_rect()));
            let title = format!("{} {}", MONTHS[(m - 1) as usize], y);
            painter.text(pos2(ox + month_w / 2.0 + slide, rect.min.y + pad + 16.0), egui::Align2::CENTER_CENTER, title, FontId::proportional(14.5), with_alpha(t.foreground, alpha));
            let wy = rect.min.y + pad + 32.0 + 8.0;
            for (i, d) in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"].iter().enumerate() {
                painter.text(pos2(ox + CELL * (i as f32 + 0.5), wy + 12.0), egui::Align2::CENTER_CENTER, *d, FontId::proportional(12.5), t.muted_foreground);
            }
            let first = CalendarDate::new(y, m, 1);
            let start = first.add_days(-(first.weekday() as i64));
            let gy = wy + 24.0;
            for cell in 0..42 {
                let d = start.add_days(cell);
                let (cx, cy) = ((cell % 7) as f32, (cell / 7) as f32);
                let r = Rect::from_min_size(pos2(ox + cx * CELL + slide, gy + cy * CELL), vec2(CELL, CELL));
                let inside = d.month == m;
                if !inside && self.months > 1 {
                    continue;
                }
                let cr = r.shrink(1.0);
                let resp_c = ui.interact(cr, id.with(("d", mi, cell)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                if resp_c.hovered() {
                    new_hover = Some(d);
                }
                if resp_c.clicked() {
                    clicked_day = Some(d);
                }
                let selected_end = Some(d) == ra || Some(d) == rb;
                let in_range = is_range && matches!((ra, rb), (Some(a), Some(b)) if d > a && d < b);
                let rad = control_radius(&t);
                if in_range {
                    // continuous band between the ends
                    let band = Rect::from_min_max(pos2(r.min.x, cr.min.y), pos2(r.max.x, cr.max.y));
                    let left = cx == 0.0 || d.day == 1;
                    let right = cx == 6.0 || d.day == days_in_month(d.year, d.month);
                    let c = CornerRadius { nw: if left { rad.nw } else { 0 }, sw: if left { rad.nw } else { 0 }, ne: if right { rad.nw } else { 0 }, se: if right { rad.nw } else { 0 } };
                    painter.rect_filled(band, c, with_alpha(if t.dark { t.secondary } else { t.muted }, alpha));
                }
                if is_range && selected_end && ra.is_some() && rb.is_some() && ra != rb {
                    // half band connecting the end cap to the range
                    let to_right = Some(d) == ra;
                    let half = if to_right { Rect::from_min_max(pos2(r.center().x, cr.min.y), pos2(r.max.x, cr.max.y)) } else { Rect::from_min_max(pos2(r.min.x, cr.min.y), pos2(r.center().x, cr.max.y)) };
                    painter.rect_filled(half, CornerRadius::ZERO, with_alpha(if t.dark { t.secondary } else { t.muted }, alpha));
                }
                let today = Some(d) == self.today;
                let hov = kit::anim(ui, id.with(("dh", mi, cell)), resp_c.hovered() && !selected_end, 0.1);
                if selected_end {
                    let pop = glowkit_core::anim::spring(ui, id.with(("pop", d.to_days())), 1.0, 500.0, 18.0);
                    painter.rect_filled(Rect::from_center_size(cr.center(), cr.size() * pop.clamp(0.0, 1.2)), rad, with_alpha(t.primary, alpha));
                } else if hov > 0.0 {
                    painter.rect_filled(cr, rad, with_alpha(if t.dark { t.secondary } else { t.muted }, hov * alpha));
                } else if today {
                    painter.rect_filled(cr, rad, with_alpha(if t.dark { t.secondary } else { t.muted }, alpha));
                }
                let col = if selected_end {
                    t.primary_foreground
                } else if !inside {
                    with_alpha(t.muted_foreground, 0.5)
                } else {
                    t.foreground
                };
                painter.text(cr.center(), egui::Align2::CENTER_CENTER, d.day.to_string(), FontId::proportional(13.5), with_alpha(col, alpha * col.a() as f32 / 255.0));
                if today && !selected_end {
                    painter.circle_filled(pos2(cr.center().x, cr.max.y - 6.0), 1.6, t.accent);
                }
            }
        }
        if new_hover != hovered_day {
            glowkit_core::set_state(ui, range_hover_id, new_hover);
        }
        if let Some(d) = clicked_day {
            match &mut self.sel {
                Sel::Single(s) => **s = Some(d),
                Sel::Range(a, b) => match (**a, **b) {
                    (Some(sa), None) => {
                        if d < sa {
                            **b = Some(sa);
                            **a = Some(d);
                        } else {
                            **b = Some(d);
                        }
                    }
                    _ => {
                        **a = Some(d);
                        **b = None;
                    }
                },
            }
            if d.month != st.month && self.months == 1 {
                st.dir = if (d.year, d.month) > (st.year, st.month) { 1.0 } else { -1.0 };
                st.year = d.year;
                st.month = d.month;
                st.changed_at = now;
            }
            resp.mark_changed();
        }
        glowkit_core::set_state(ui, id, st);
        resp
    }
}

/// A date field that opens a [`Calendar`] in a popover. Single dates close the popover on pick;
/// ranges close after the end date is chosen.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut due = None;
/// use glowkit_components::DatePicker;
/// ui.add(DatePicker::new(&mut due).placeholder("Pick a date"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DatePicker<'a> {
    sel: Sel<'a>,
    placeholder: String,
    width: f32,
    today: Option<CalendarDate>,
    months: usize,
}

impl<'a> DatePicker<'a> {
    /// Single date picker.
    pub fn new(selected: &'a mut Option<CalendarDate>) -> Self {
        Self { sel: Sel::Single(selected), placeholder: "Pick a date".into(), width: 240.0, today: CalendarDate::today(), months: 1 }
    }
    /// Date range picker (shows two months).
    pub fn range(start: &'a mut Option<CalendarDate>, end: &'a mut Option<CalendarDate>) -> Self {
        Self { sel: Sel::Range(start, end), placeholder: "Pick a date range".into(), width: 280.0, today: CalendarDate::today(), months: 2 }
    }
    /// Text shown while empty.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Trigger width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Override "today".
    pub fn today(mut self, d: CalendarDate) -> Self {
        self.today = Some(d);
        self
    }
    /// Months shown in the popover.
    pub fn months(mut self, n: usize) -> Self {
        self.months = n;
        self
    }
}

impl Widget for DatePicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut resp) = ui.allocate_exact_size(vec2(self.width, 36.0), Sense::click());
        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        let popup_id = resp.id.with("date_popup");
        let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let hover = kit::anim(ui, resp.id.with("h"), resp.hovered(), 0.12);
        let focus = kit::anim(ui, resp.id.with("f"), open, 0.16);
        kit::field_chrome(ui.painter(), rect, &t, hover, focus, false, false);
        UiIcon::Calendar.paint(ui.painter(), Rect::from_center_size(pos2(rect.min.x + 20.0, rect.center().y), vec2(16.0, 16.0)), t.muted_foreground);
        let text = match &self.sel {
            Sel::Single(s) => s.map(|d| d.format_short()),
            Sel::Range(a, b) => match (**a, **b) {
                (Some(a), Some(b)) => Some(format!("{} – {}", a.format_short(), b.format_short())),
                (Some(a), None) => Some(format!("{} – …", a.format_short())),
                _ => None,
            },
        };
        let (s, col) = match text {
            Some(s) => (s, t.foreground),
            None => (self.placeholder.clone(), t.muted_foreground),
        };
        ui.painter().with_clip_rect(rect.shrink(1.0)).text(pos2(rect.min.x + 38.0, rect.center().y), egui::Align2::LEFT_CENTER, s, FontId::proportional(14.0), col);
        if resp.clicked() || (matches!(self.sel, Sel::Range(..)) && !open && ui.input(|i| i.time) < 1.5) {
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }
        let mut changed = false;
        let is_single = matches!(self.sel, Sel::Single(_));
        let today = self.today;
        let months = self.months;
        let sel = self.sel;
        egui::Popup::new(popup_id, ui.ctx().clone(), rect, ui.layer_id())
            .open_memory(None::<egui::SetOpenCommand>)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(6.0)
            .frame(
                egui::Frame::new()
                    .fill(t.popover)
                    .stroke(t.border_stroke())
                    .corner_radius(t.corner())
                    .inner_margin(egui::Margin::same(0))
                    .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if t.dark { 100 } else { 30 }) }),
            )
            .show(|ui| {
                let mut cal = match sel {
                    Sel::Single(s) => Calendar::new(s),
                    Sel::Range(a, b) => Calendar::range(a, b),
                }
                .months(months)
                .framed(false)
                .id_salt(popup_id.with("cal"));
                if let Some(d) = today {
                    cal = cal.today(d);
                }
                if ui.add(cal).changed() {
                    changed = true;
                }
            });
        if changed {
            resp.mark_changed();
            if is_single {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
        }
        resp
    }
}
