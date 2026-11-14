//! SaaS admin / CRM template: customers table and deal pipeline.

use egui::{pos2, vec2, Align2, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, AlertDialog, Button, ButtonVariant, Checkbox, ControlSize, DescriptionList, Dialog, DropdownMenu, Input, LineIcon, MenuEntry, Pagination,
    SegmentedControl, Select, Sheet, SheetSide, Timeline, TimelineItem, Toast, UiIcon,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces, page_header};
use crate::shells::kit::{self, Tone};
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// A customer row in the [`CrmApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct CrmCustomer {
    /// Contact name.
    pub name: String,
    /// Email.
    pub email: String,
    /// Company.
    pub company: String,
    /// 0 active, 1 trial, 2 churned.
    pub status: u8,
    /// Plan name.
    pub plan: String,
    /// Monthly recurring revenue.
    pub mrr: f64,
    /// Last activity.
    pub last_seen: String,
}

/// A deal card in the pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct CrmDeal {
    /// Deal title.
    pub title: String,
    /// Company.
    pub company: String,
    /// Owner.
    pub owner: String,
    /// Value in dollars.
    pub value: f64,
    /// Stage column index.
    pub stage: usize,
    /// Tag.
    pub tag: String,
}

/// A SaaS admin / CRM screen inside an inset [`AppShell`]: a customers table with search,
/// status and plan filters, sortable columns, row selection with a bulk delete confirmation,
/// pagination, a detail sheet and an "Add customer" dialog; plus a drag-and-drop deal pipeline.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::CrmApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct CrmApp {
    /// Active route ("customers", "pipeline", ...).
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Customer records.
    pub customers: Vec<CrmCustomer>,
    /// Pipeline deals.
    pub deals: Vec<CrmDeal>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    query: String,
    status_filter: usize,
    plan_filter: Option<usize>,
    sort: (usize, bool),
    selected: Vec<bool>,
    page: usize,
    detail: Option<usize>,
    sheet_open: bool,
    add_open: bool,
    confirm_delete: bool,
    form: (String, String, String, Option<usize>),
    form_error: bool,
    drag: Option<usize>,
}

const PLANS: [&str; 4] = ["Free", "Starter", "Pro", "Enterprise"];
const STAGES: [&str; 5] = ["Lead", "Qualified", "Proposal", "Negotiation", "Won"];

fn customers() -> Vec<CrmCustomer> {
    let raw = [
        ("Olivia Martin", "Northwind Labs", 0, 2, 499.0, "2 min ago"),
        ("Jackson Lee", "Globex", 0, 3, 2400.0, "1 hour ago"),
        ("Isabella Nguyen", "Initech", 1, 1, 0.0, "3 hours ago"),
        ("William Kim", "Hooli", 0, 2, 799.0, "Yesterday"),
        ("Sofia Davis", "Umbrella Health", 2, 1, 49.0, "3 days ago"),
        ("Liam Johnson", "Stark Industries", 0, 3, 5200.0, "12 min ago"),
        ("Emma Wilson", "Wayne Enterprises", 0, 3, 3900.0, "2 days ago"),
        ("Noah Brown", "Acme Robotics", 1, 2, 0.0, "5 hours ago"),
        ("Ava Garcia", "Cyberdyne", 0, 1, 99.0, "1 week ago"),
        ("Lucas Martinez", "Soylent Foods", 2, 0, 0.0, "1 month ago"),
        ("Mia Chen", "Pied Piper", 0, 2, 699.0, "Just now"),
        ("Ethan Clark", "Vandelay Imports", 0, 1, 149.0, "4 days ago"),
        ("Harper Lewis", "Massive Dynamic", 1, 3, 0.0, "6 hours ago"),
        ("James Walker", "Oscorp", 0, 2, 599.0, "Yesterday"),
        ("Amelia Hall", "Tyrell Corp", 2, 2, 299.0, "2 weeks ago"),
        ("Benjamin Young", "Aperture Science", 0, 3, 7800.0, "30 min ago"),
        ("Charlotte King", "Black Mesa", 0, 1, 79.0, "3 days ago"),
        ("Henry Wright", "Gringotts", 1, 1, 0.0, "1 day ago"),
    ];
    raw.iter()
        .map(|(n, c, s, p, m, l)| CrmCustomer {
            name: (*n).into(),
            email: format!("{}@{}.com", n.split(' ').next().unwrap_or("x").to_lowercase(), c.split(' ').next().unwrap_or("x").to_lowercase()),
            company: (*c).into(),
            status: *s,
            plan: PLANS[*p].into(),
            mrr: *m,
            last_seen: (*l).into(),
        })
        .collect()
}

fn deals() -> Vec<CrmDeal> {
    let raw = [
        ("Annual renewal", "Globex", "Jackson Lee", 28_800.0, 4, "Renewal"),
        ("Platform migration", "Stark Industries", "Liam Johnson", 62_000.0, 3, "Enterprise"),
        ("Team plan · 40 seats", "Pied Piper", "Mia Chen", 16_800.0, 2, "Expansion"),
        ("Pilot program", "Initech", "Isabella Nguyen", 4_500.0, 1, "New"),
        ("Analytics add-on", "Hooli", "William Kim", 9_600.0, 2, "Upsell"),
        ("SSO + audit logs", "Wayne Enterprises", "Emma Wilson", 21_000.0, 3, "Security"),
        ("Inbound demo", "Acme Robotics", "Noah Brown", 7_200.0, 0, "Inbound"),
        ("Conference lead", "Cyberdyne", "Ava Garcia", 3_000.0, 0, "Event"),
        ("Partner referral", "Massive Dynamic", "Harper Lewis", 12_400.0, 1, "Partner"),
        ("Self-serve upgrade", "Black Mesa", "Charlotte King", 1_900.0, 4, "PLG"),
        ("Webinar signup", "Gringotts", "Henry Wright", 2_400.0, 0, "Inbound"),
    ];
    raw.iter()
        .map(|(t, c, o, v, s, g)| CrmDeal { title: (*t).into(), company: (*c).into(), owner: (*o).into(), value: *v, stage: *s, tag: (*g).into() })
        .collect()
}

impl Default for CrmApp {
    fn default() -> Self {
        let customers = customers();
        Self {
            route: "customers".into(),
            nav: vec![
                NavSection::new("Sales")
                    .item(NavItem::new("customers", "Customers").icon(LineIcon::Users).badge("18").shortcut("G C"))
                    .item(NavItem::new("pipeline", "Pipeline").icon(AppIcon::Kanban).shortcut("G P"))
                    .item(NavItem::new("activity", "Activity").icon(UiIcon::Activity))
                    .item(NavItem::new("reports", "Reports").icon(LineIcon::Chart).child(NavItem::new("reports/revenue", "Revenue")).child(NavItem::new("reports/churn", "Churn")).child(NavItem::new("reports/cohorts", "Cohorts"))),
                NavSection::new("Admin")
                    .item(NavItem::new("billing", "Billing").icon(LineIcon::CreditCard))
                    .item(NavItem::new("integrations", "Integrations").icon(LineIcon::Layers).badge("3"))
                    .item(NavItem::new("settings", "Settings").icon(LineIcon::Settings)),
            ],
            notifications: demo_notes(),
            selected: vec![false; customers.len()],
            customers,
            deals: deals(),
            scoped_theme: false,
            query: String::new(),
            status_filter: 0,
            plan_filter: None,
            sort: (4, false),
            page: 1,
            detail: None,
            sheet_open: false,
            add_open: false,
            confirm_delete: false,
            form: (String::new(), String::new(), String::new(), Some(1)),
            form_error: false,
            drag: None,
        }
    }
}

fn status_of(s: u8) -> (&'static str, Tone) {
    match s {
        0 => ("Active", Tone::Success),
        1 => ("Trial", Tone::Info),
        _ => ("Churned", Tone::Danger),
    }
}

impl CrmApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let n_customers = self.customers.len();
        if let Some(it) = nav.get_mut(0).and_then(|s| s.items.get_mut(0)) {
            it.badge = Some(n_customers.to_string());
        }
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .workspaces(demo_workspaces("Acme CRM", "Growth"))
            .user(demo_user())
            .notifications(&mut notes)
            .scoped_theme(self.scoped_theme)
            .id_salt("crm_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "customers" => self.customers_page(ui, sc.rect, sc.bounds),
                "pipeline" => self.pipeline_page(ui, sc.rect),
                other => placeholder(ui, sc.rect, other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn filtered(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        let mut v: Vec<usize> = (0..self.customers.len())
            .filter(|&i| {
                let c = &self.customers[i];
                (q.is_empty() || c.name.to_lowercase().contains(&q) || c.email.to_lowercase().contains(&q) || c.company.to_lowercase().contains(&q))
                    && (self.status_filter == 0 || c.status as usize == self.status_filter - 1)
                    && self.plan_filter.is_none_or(|p| c.plan == PLANS[p])
            })
            .collect();
        let (col, asc) = self.sort;
        v.sort_by(|&a, &b| {
            let (a, b) = (&self.customers[a], &self.customers[b]);
            let o = match col {
                0 => a.name.cmp(&b.name),
                1 => a.company.cmp(&b.company),
                2 => a.status.cmp(&b.status),
                3 => PLANS.iter().position(|p| *p == a.plan).cmp(&PLANS.iter().position(|p| *p == b.plan)),
                _ => a.mrr.partial_cmp(&b.mrr).unwrap_or(std::cmp::Ordering::Equal),
            };
            if asc {
                o
            } else {
                o.reverse()
            }
        });
        v
    }

    fn customers_page(&mut self, ui: &mut Ui, rect: Rect, bounds: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("crm_customers");
        let loading = kit::loading(ui, id, 0.6);
        self.selected.resize(self.customers.len(), false);
        let mrr: f64 = self.customers.iter().filter(|c| c.status == 0).map(|c| c.mrr).sum();
        let sub = format!("{} accounts · {} MRR", self.customers.len(), kit::money(mrr));
        let mut add = false;
        let top = page_header(ui, rect, "Customers", &sub, |ui| {
            if ui.add(Button::new("Add customer").icon(UiIcon::Plus).size(ControlSize::Sm)).clicked() {
                add = true;
            }
            if ui.add(Button::new("Export").icon(UiIcon::Download).variant(ButtonVariant::Outline).size(ControlSize::Sm)).clicked() {
                toast(ui.ctx(), Toast::success("Export ready").description(format!("{} customers exported to CSV.", self.customers.len())));
            }
        });
        if add {
            self.add_open = true;
            self.form = (String::new(), String::new(), String::new(), Some(1));
            self.form_error = false;
        }
        // toolbar
        let bar = Rect::from_min_size(pos2(rect.min.x, top - 4.0), vec2(rect.width(), 44.0));
        let sel_count = self.selected.iter().filter(|s| **s).count();
        {
            let cy = bar.center().y;
            let mut x = bar.min.x;
            let sid = ui.make_persistent_id("crm_search").with("te");
            if ui.input(|i| i.key_pressed(egui::Key::Slash)) && ui.memory(|m| m.focused().is_none()) {
                ui.memory_mut(|m| m.request_focus(sid));
            }
            let before = self.query.clone();
            kit::put(ui, Rect::from_min_size(pos2(x, cy - 16.0), vec2(260.0, 32.0)), Input::new(&mut self.query).placeholder("Search customers…").icon(UiIcon::Search).shortcut("/").clearable(true).width(260.0).size(ControlSize::Sm).id_salt("crm_search"));
            if before != self.query {
                self.page = 1;
            }
            x += 272.0;
            let sf = self.status_filter;
            kit::put(ui, Rect::from_min_size(pos2(x, cy - 16.0), vec2(300.0, 32.0)), SegmentedControl::new(&mut self.status_filter, ["All", "Active", "Trial", "Churned"]).size(ControlSize::Sm).width(300.0));
            if sf != self.status_filter {
                self.page = 1;
            }
            x += 312.0;
            if bar.max.x - x > 360.0 {
                let pf = self.plan_filter;
                kit::put(ui, Rect::from_min_size(pos2(x, cy - 16.0), vec2(150.0, 32.0)), Select::new(&mut self.plan_filter, PLANS).placeholder("All plans").size(ControlSize::Sm).width(150.0).id_salt("crm_plan"));
                if pf != self.plan_filter {
                    self.page = 1;
                }
                x += 160.0;
                if self.plan_filter.is_some() && kit::put(ui, Rect::from_min_size(pos2(x, cy - 16.0), vec2(84.0, 32.0)), Button::new("Reset").variant(ButtonVariant::Ghost).size(ControlSize::Sm).trailing_icon(UiIcon::X)).clicked() {
                    self.plan_filter = None;
                }
            }
            if sel_count > 0 {
                let br = Rect::from_min_size(pos2(bar.max.x - 110.0, cy - 16.0), vec2(110.0, 32.0));
                if kit::put(ui, br, Button::new(format!("Delete {sel_count}")).icon(UiIcon::Trash).variant(ButtonVariant::Destructive).size(ControlSize::Sm).width(110.0)).clicked() {
                    self.confirm_delete = true;
                }
                kit::text_at(ui.painter(), pos2(br.min.x - 10.0, cy), Align2::RIGHT_CENTER, &format!("{sel_count} selected"), 12.5, t.muted_foreground, 100.0);
            }
        }
        // table
        let rows = self.filtered();
        let per_page = 8usize;
        let pages = rows.len().div_ceil(per_page).max(1);
        self.page = self.page.clamp(1, pages);
        let table = Rect::from_min_max(pos2(rect.min.x, bar.max.y + 8.0), pos2(rect.max.x, rect.max.y - 44.0));
        kit::card(ui.painter(), table, &t);
        let wide = table.width() > 900.0;
        // columns: check, customer, company, status, plan, mrr, last seen, menu
        let widths: Vec<f32> = if wide { vec![44.0, 0.0, 0.0, 110.0, 100.0, 100.0, 120.0, 44.0] } else { vec![44.0, 0.0, 0.0, 100.0, 0.0, 90.0, 0.0, 44.0] };
        let head_h = 40.0;
        let row_h = ((table.height() - head_h) / per_page as f32).clamp(44.0, 56.0);
        let hr = Rect::from_min_size(table.min, vec2(table.width(), head_h));
        let cells = kit::split_x(hr.shrink2(vec2(8.0, 0.0)), &widths, 8.0);
        let p = ui.painter().clone();
        p.rect_filled(hr.shrink(1.0), egui::CornerRadius { nw: t.radius as u8, ne: t.radius as u8, sw: 0, se: 0 }, with_alpha(t.muted, 0.5));
        kit::hline(&p, table.min.x, table.max.x, hr.max.y, &t);
        // header checkbox (select all on page)
        let page_rows: Vec<usize> = rows.iter().skip((self.page - 1) * per_page).take(per_page).copied().collect();
        let all_on = !page_rows.is_empty() && page_rows.iter().all(|&i| self.selected[i]);
        let some_on = page_rows.iter().any(|&i| self.selected[i]);
        let mut all = all_on;
        let cr = Rect::from_center_size(cells[0].center(), vec2(18.0, 18.0));
        if kit::put(ui, cr, Checkbox::new(&mut all).indeterminate(some_on && !all_on).size(16.0)).changed() {
            for &i in &page_rows {
                self.selected[i] = all;
            }
        }
        let heads = ["Customer", "Company", "Status", "Plan", "MRR", "Last seen"];
        for (k, title) in heads.iter().enumerate() {
            let c = cells[k + 1];
            if c.width() < 1.0 {
                continue;
            }
            let right = k == 4;
            let sortable = k <= 4;
            let hid = id.with(("head", k));
            let resp = ui.interact(c, hid, if sortable { Sense::click() } else { Sense::hover() });
            let hv = kit::hover(ui, hid, resp.hovered() && sortable);
            let on = self.sort.0 == k;
            let col = if on { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
            let tr = kit::text_at(&p, if right { pos2(c.max.x - 18.0, c.center().y) } else { c.left_center() }, if right { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, title, 12.5, col, c.width() - 20.0);
            if sortable && (on || hv > 0.01) {
                let ic = if on { if self.sort.1 { UiIcon::ArrowUp } else { UiIcon::ArrowDown } } else { UiIcon::ArrowUpDown };
                let ix = if right { tr.max.x + 9.0 } else { tr.max.x + 10.0 };
                ic.paint(&p, Rect::from_center_size(pos2(ix, c.center().y), vec2(12.0, 12.0)), if on { col } else { with_alpha(col, hv) });
            }
            if resp.clicked() && sortable {
                self.sort = if on { (k, !self.sort.1) } else { (k, k <= 1) };
            }
            if sortable {
                resp.on_hover_cursor(CursorIcon::PointingHand);
            }
        }
        // rows
        let mut open_detail = None;
        let mut delete_one = None;
        for (ri, &ci) in page_rows.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(table.min.x, hr.max.y + ri as f32 * row_h), vec2(table.width(), row_h));
            if loading {
                let cc = kit::split_x(rr.shrink2(vec2(8.0, 0.0)), &widths, 8.0);
                kit::skel_r(ui, Rect::from_center_size(cc[0].center(), vec2(16.0, 16.0)), 4.0);
                kit::skel_r(ui, Rect::from_center_size(pos2(cc[1].min.x + 16.0, rr.center().y), vec2(32.0, 32.0)), 16.0);
                kit::skel(ui, Rect::from_min_size(pos2(cc[1].min.x + 44.0, rr.center().y - 9.0), vec2(cc[1].width() * 0.5, 8.0)));
                kit::skel(ui, Rect::from_min_size(pos2(cc[1].min.x + 44.0, rr.center().y + 3.0), vec2(cc[1].width() * 0.35, 7.0)));
                for c in &cc[2..7] {
                    if c.width() > 1.0 {
                        kit::skel(ui, Rect::from_min_size(pos2(c.min.x, rr.center().y - 4.0), vec2(c.width() * 0.6, 8.0)));
                    }
                }
                continue;
            }
            let rid = id.with(("row", ci));
            let resp = ui.interact(rr.shrink2(vec2(1.0, 0.0)), rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, rid, resp.hovered());
            let sel = self.selected[ci];
            if sel {
                p.rect_filled(rr.shrink2(vec2(1.0, 0.0)), 0.0, with_alpha(t.accent, if t.dark { 0.08 } else { 0.06 }));
            } else if hv > 0.01 {
                p.rect_filled(rr.shrink2(vec2(1.0, 0.0)), 0.0, kit::hover_fill(&t, hv));
            }
            if ri + 1 < page_rows.len() {
                p.line_segment([pos2(rr.min.x + 1.0, rr.max.y), pos2(rr.max.x - 1.0, rr.max.y)], Stroke::new(1.0, with_alpha(t.border, 0.7)));
            }
            let cc = kit::split_x(rr.shrink2(vec2(8.0, 0.0)), &widths, 8.0);
            let c = &self.customers[ci];
            let mut s = sel;
            let cbr = Rect::from_center_size(cc[0].center(), vec2(18.0, 18.0));
            let cb_changed = kit::put(ui, cbr, Checkbox::new(&mut s).size(16.0)).changed();
            if cb_changed {
                self.selected[ci] = s;
            }
            kit::avatar(ui, Rect::from_center_size(pos2(cc[1].min.x + 16.0, rr.center().y), vec2(32.0, 32.0)), &c.name);
            kit::text_at(&p, pos2(cc[1].min.x + 42.0, rr.center().y - 8.0), Align2::LEFT_CENTER, &c.name, 13.5, t.foreground, cc[1].width() - 44.0);
            kit::text_at(&p, pos2(cc[1].min.x + 42.0, rr.center().y + 9.0), Align2::LEFT_CENTER, &c.email, 12.0, t.muted_foreground, cc[1].width() - 44.0);
            kit::text_at(&p, cc[2].left_center(), Align2::LEFT_CENTER, &c.company, 13.0, t.foreground, cc[2].width());
            let (st, tone) = status_of(c.status);
            kit::status_pill(&p, cc[3].left_center(), false, st, kit::tone(&t, tone), &t);
            if cc[4].width() > 1.0 {
                kit::text_at(&p, cc[4].left_center(), Align2::LEFT_CENTER, &c.plan, 13.0, t.muted_foreground, cc[4].width());
            }
            kit::text_at(&p, pos2(cc[5].max.x - 18.0, rr.center().y), Align2::RIGHT_CENTER, &kit::money(c.mrr), 13.0, if c.mrr > 0.0 { t.foreground } else { t.muted_foreground }, cc[5].width());
            if cc[6].width() > 1.0 {
                kit::text_at(&p, cc[6].left_center(), Align2::LEFT_CENTER, &c.last_seen, 12.5, t.muted_foreground, cc[6].width());
            }
            let mr = Rect::from_center_size(cc[7].center(), vec2(28.0, 28.0));
            let mresp = kit::icon_button(ui, mr, rid.with("menu"), AppIcon::Ui(UiIcon::MoreHorizontal), false);
            let entries = vec![
                MenuEntry::item("View details").icon(LineIcon::User),
                MenuEntry::item("Send email").icon(LineIcon::Mail),
                MenuEntry::item("Copy email").icon(LineIcon::Copy),
                MenuEntry::separator(),
                MenuEntry::item("Delete").icon(LineIcon::Trash).destructive(),
            ];
            match DropdownMenu::new(entries).width(190.0).within(bounds).show(&mresp).as_deref() {
                Some("View details") => open_detail = Some(ci),
                Some("Send email") => {
                    toast(ui.ctx(), Toast::success(format!("Email drafted to {}", c.name)).description(c.email.clone()));
                }
                Some("Copy email") => {
                    ui.ctx().copy_text(c.email.clone());
                    toast(ui.ctx(), Toast::new("Copied to clipboard").description(c.email.clone()));
                }
                Some("Delete") => delete_one = Some(ci),
                _ => {}
            }
            if resp.clicked() && !cb_changed && !mresp.clicked() {
                open_detail = Some(ci);
            }
        }
        if page_rows.is_empty() && !loading {
            let er = Rect::from_min_max(pos2(table.min.x, hr.max.y), table.max);
            p.text(er.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, "No customers match your filters", egui::FontId::proportional(14.0), t.foreground);
            p.text(er.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Try a different search or clear the filters.", egui::FontId::proportional(12.5), t.muted_foreground);
        }
        if let Some(i) = delete_one {
            let name = self.customers[i].name.clone();
            self.customers.remove(i);
            self.selected.remove(i);
            toast(ui.ctx(), Toast::success(format!("{name} deleted")).description("The customer and their history were removed."));
        }
        if let Some(i) = open_detail {
            self.detail = Some(i);
            self.sheet_open = true;
        }
        // footer
        let fr = Rect::from_min_max(pos2(rect.min.x, rect.max.y - 34.0), rect.max);
        let shown_from = if rows.is_empty() { 0 } else { (self.page - 1) * per_page + 1 };
        let shown_to = (self.page * per_page).min(rows.len());
        kit::text_at(&p, fr.left_center(), Align2::LEFT_CENTER, &format!("Showing {shown_from}–{shown_to} of {}", rows.len()), 12.5, t.muted_foreground, 220.0);
        kit::child(ui, Rect::from_min_max(pos2(fr.min.x + 220.0, fr.min.y), fr.max), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(Pagination::new(&mut self.page, pages));
            });
        });
        self.overlays(ui, bounds);
    }

    fn overlays(&mut self, ui: &mut Ui, bounds: Rect) {
        let t = Theme::of(ui);
        // detail sheet
        if let Some(i) = self.detail.filter(|i| *i < self.customers.len()) {
            let c = self.customers[i].clone();
            let mut open = self.sheet_open;
            let r = Sheet::new(&mut open, c.name.clone()).description(format!("{} · customer since 2023", c.company)).side(SheetSide::Right).size(400.0).within(bounds).primary("Send email").secondary("Close").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let hr = kit::alloc_row(ui, 64.0);
                kit::avatar(ui, Rect::from_min_size(hr.min, vec2(56.0, 56.0)), &c.name);
                let p = ui.painter().clone();
                kit::text_at(&p, pos2(hr.min.x + 70.0, hr.min.y + 16.0), Align2::LEFT_CENTER, &c.email, 13.5, t.foreground, hr.width() - 80.0);
                let (st, tone) = status_of(c.status);
                kit::status_pill(&p, pos2(hr.min.x + 70.0, hr.min.y + 42.0), false, st, kit::tone(&t, tone), &t);
                let mrr = kit::money(c.mrr);
                ui.add(DescriptionList::new(&[("Company", c.company.as_str()), ("Plan", c.plan.as_str()), ("MRR", mrr.as_str()), ("Last seen", c.last_seen.as_str()), ("Owner", "Ada Lovelace")]).key_width(90.0).width(ui.available_width()));
                ui.label(egui::RichText::new("Activity").size(13.5).color(t.foreground));
                ui.add(Timeline::new(vec![
                    TimelineItem::new("Upgraded plan", "2d ago", "Moved to Pro (annual)"),
                    TimelineItem::new("Support ticket closed", "1w ago", "#4821 · SSO configuration"),
                    TimelineItem::new("Invited 4 teammates", "3w ago", "Design and Marketing"),
                ]));
            });
            if r.primary_clicked {
                toast(ui.ctx(), Toast::success(format!("Email drafted to {}", c.name)).description(c.email.clone()));
            }
            self.sheet_open = open;
        }
        // add dialog
        let mut open = self.add_open;
        let form = &mut self.form;
        let err = self.form_error;
        let r = Dialog::new(&mut open, "Add customer").description("Create a customer record. They won't be emailed.").primary("Create customer").secondary("Cancel").width(440.0).within(bounds).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            let w = ui.available_width();
            let name_err = if err && form.0.trim().is_empty() { Some("Name is required".to_string()) } else { None };
            ui.add(Input::new(&mut form.0).label("Full name").placeholder("Jane Cooper").required(true).error_opt(name_err).width(w).id_salt("crm_f_name"));
            ui.add(Input::new(&mut form.1).label("Email").placeholder("jane@company.com").icon(UiIcon::Mail).width(w).id_salt("crm_f_email"));
            ui.add(Input::new(&mut form.2).label("Company").placeholder("Company name").width(w).id_salt("crm_f_company"));
            ui.add(Select::new(&mut form.3, PLANS).label("Plan").width(w).id_salt("crm_f_plan"));
        });
        if r.primary_clicked {
            if self.form.0.trim().is_empty() {
                self.form_error = true;
                open = true;
            } else {
                let plan = PLANS[self.form.3.unwrap_or(0)].to_string();
                let name = self.form.0.trim().to_string();
                self.customers.insert(
                    0,
                    CrmCustomer {
                        email: if self.form.1.is_empty() { format!("{}@example.com", name.split(' ').next().unwrap_or("new").to_lowercase()) } else { self.form.1.clone() },
                        company: if self.form.2.is_empty() { "—".into() } else { self.form.2.clone() },
                        name: name.clone(),
                        status: 1,
                        plan,
                        mrr: 0.0,
                        last_seen: "Just now".into(),
                    },
                );
                self.selected.insert(0, false);
                self.sort = (4, false);
                toast(ui.ctx(), Toast::success(format!("{name} added")).description("Started a 14-day trial.").action("Undo"));
                open = false;
            }
        }
        self.add_open = open;
        // bulk delete confirm
        let n = self.selected.iter().filter(|s| **s).count();
        let mut c_open = self.confirm_delete;
        let r = AlertDialog::new(&mut c_open, format!("Delete {n} customers?")).description("This permanently removes the selected customers and their history. This action cannot be undone.").confirm("Delete").destructive(true).within(bounds).show(ui);
        if r.confirmed {
            let mut i = 0;
            self.customers.retain(|_| {
                let keep = !self.selected[i];
                i += 1;
                keep
            });
            self.selected = vec![false; self.customers.len()];
            toast(ui.ctx(), Toast::success(format!("{n} customers deleted")));
        }
        self.confirm_delete = c_open;
    }

    fn pipeline_page(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("crm_pipeline");
        let loading = kit::loading(ui, id, 0.6);
        let total: f64 = self.deals.iter().filter(|d| d.stage < 4).map(|d| d.value).sum();
        let top = page_header(ui, rect, "Pipeline", &format!("{} open deals · {} weighted", self.deals.iter().filter(|d| d.stage < 4).count(), kit::money(total * 0.42)), |ui| {
            if ui.add(Button::new("New deal").icon(UiIcon::Plus).size(ControlSize::Sm)).clicked() {
                self.deals.push(CrmDeal { title: "New opportunity".into(), company: "Untitled Co.".into(), owner: "Ada Lovelace".into(), value: 5_000.0, stage: 0, tag: "New".into() });
                toast(ui.ctx(), Toast::success("Deal created").description("Added to Lead. Drag it across stages."));
            }
        });
        let board = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
        let colw = kit::cols(board, STAGES.len(), 12.0);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let released = ui.input(|i| i.pointer.any_released());
        let mut drop_to = None;
        let p = ui.painter().clone();
        for (s, cr) in colw.iter().enumerate() {
            let hovering_drop = self.drag.is_some() && pointer.is_some_and(|pp| cr.contains(pp));
            p.rect_filled(*cr, t.radius, if hovering_drop { with_alpha(t.accent, 0.08) } else { with_alpha(t.muted, if t.dark { 0.5 } else { 0.7 }) });
            if hovering_drop {
                p.rect_stroke(*cr, t.radius, Stroke::new(1.0, with_alpha(t.accent, 0.5)), StrokeKind::Inside);
                if released {
                    drop_to = Some(s);
                }
            }
            let sum: f64 = self.deals.iter().filter(|d| d.stage == s).map(|d| d.value).sum();
            let cnt = self.deals.iter().filter(|d| d.stage == s).count();
            let dot = [t.muted_foreground, t.info, t.chart[0], t.warning, t.success][s];
            p.circle_filled(pos2(cr.min.x + 16.0, cr.min.y + 20.0), 4.0, dot);
            let tr = kit::text_at(&p, pos2(cr.min.x + 26.0, cr.min.y + 20.0), Align2::LEFT_CENTER, STAGES[s], 13.5, t.foreground, cr.width() - 90.0);
            kit::pill(&p, pos2(tr.max.x + 6.0, cr.min.y + 20.0), false, &cnt.to_string(), t.muted_foreground, if t.dark { t.secondary } else { t.background }, 10.5);
            kit::text_at(&p, pos2(cr.max.x - 12.0, cr.min.y + 20.0), Align2::RIGHT_CENTER, &format!("${}", glowkit_charts::format_compact(sum)), 12.0, t.muted_foreground, 70.0);
            let mut y = cr.min.y + 40.0;
            for (di, d) in self.deals.iter().enumerate().filter(|(_, d)| d.stage == s) {
                let card = Rect::from_min_size(pos2(cr.min.x + 8.0, y), vec2(cr.width() - 16.0, 96.0));
                if card.max.y > cr.max.y - 6.0 {
                    kit::text_at(&p, pos2(cr.center().x, cr.max.y - 14.0), Align2::CENTER_CENTER, "More…", 12.0, t.muted_foreground, 100.0);
                    break;
                }
                y += 104.0;
                if loading {
                    kit::skel_r(ui, card, t.radius * 0.8);
                    continue;
                }
                let did = id.with(("deal", di));
                let resp = ui.interact(card, did, Sense::click_and_drag()).on_hover_cursor(CursorIcon::Grab);
                if resp.drag_started() {
                    self.drag = Some(di);
                }
                let dragging = self.drag == Some(di);
                let hv = kit::hover(ui, did, resp.hovered());
                let mut cp = p.clone();
                if dragging {
                    cp.multiply_opacity(0.35);
                }
                deal_card(&cp, card, d, hv, &t);
                if resp.clicked() {
                    toast(ui.ctx(), Toast::new(d.title.clone()).description(format!("{} · {} · {}", d.company, kit::money(d.value), STAGES[d.stage])));
                }
            }
        }
        // drag ghost
        if let (Some(di), Some(pp)) = (self.drag, pointer) {
            if let Some(d) = self.deals.get(di) {
                let w = colw[0].width() - 16.0;
                let ghost = Rect::from_center_size(pp + vec2(0.0, 10.0), vec2(w, 96.0));
                let gp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("ghost")));
                glowkit_core::paint::soft_shadow(&gp, ghost, t.radius, 18.0, vec2(0.0, 8.0), kit::shadow(&t));
                deal_card(&gp, ghost, d, 1.0, &t);
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
        }
        if released {
            if let (Some(di), Some(s)) = (self.drag, drop_to) {
                if self.deals[di].stage != s {
                    self.deals[di].stage = s;
                    let d = self.deals.remove(di);
                    let title = d.title.clone();
                    self.deals.push(d);
                    toast(ui.ctx(), if s == 4 { Toast::success(format!("{title} won!")).description("Nice work. Revenue updated.") } else { Toast::new(format!("Moved to {}", STAGES[s])).description(title) });
                }
            }
            self.drag = None;
        }
    }
}

fn deal_card(p: &egui::Painter, card: Rect, d: &CrmDeal, hv: f32, t: &Theme) {
    if hv > 0.01 {
        glowkit_core::paint::soft_shadow(p, card, t.radius * 0.8, 10.0 * hv, vec2(0.0, 3.0), with_alpha(egui::Color32::BLACK, if t.dark { 0.3 } else { 0.08 } * hv));
    }
    p.rect_filled(card, t.radius * 0.8, t.card);
    p.rect_stroke(card, t.radius * 0.8, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, 0.3 * hv)), StrokeKind::Inside);
    kit::text_at(p, card.left_top() + vec2(12.0, 18.0), Align2::LEFT_CENTER, &d.title, 13.5, t.foreground, card.width() - 24.0);
    kit::text_at(p, card.left_top() + vec2(12.0, 37.0), Align2::LEFT_CENTER, &d.company, 12.0, t.muted_foreground, card.width() - 24.0);
    kit::pill(p, card.left_top() + vec2(12.0, 62.0), false, &d.tag, t.accent, with_alpha(t.accent, 0.12), 10.5);
    kit::text_at(p, pos2(card.max.x - 12.0, card.min.y + 62.0), Align2::RIGHT_CENTER, &kit::money(d.value).replace(".00", ""), 13.0, t.foreground, 90.0);
    let initials: String = d.owner.split(' ').filter_map(|w| w.chars().next()).collect();
    kit::text_at(p, card.left_top() + vec2(12.0, 83.0), Align2::LEFT_CENTER, &format!("{} · {}", initials, d.owner), 11.5, t.muted_foreground, card.width() - 24.0);
    let _ = Id::NULL;
}
