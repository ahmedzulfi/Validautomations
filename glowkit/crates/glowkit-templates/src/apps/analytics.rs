//! Analytics dashboard template.

use egui::{pos2, vec2, Align2, Rect, Ui};
use glowkit_charts::{AreaChart, BarList, Datum, DonutChart, Series, StatTile};
use glowkit_components::{toast, BadgeVariant, Button, ButtonVariant, ControlSize, DataTable, LineIcon, TableCell, TableColumn, Tabs, TabsStyle, Toast, UiIcon};
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user, demo_workspaces, page_header, series};
use crate::shells::kit;
use crate::shells::{AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// A complete analytics dashboard inside an [`AppShell`]: date-range tabs, KPI tiles with
/// sparklines, a revenue area chart, a traffic-source donut, a top-pages bar list and a recent
/// orders table. Shows loading skeletons on first show and when the range changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::AnalyticsApp::default(); // keep it in your app state
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct AnalyticsApp {
    /// Active route.
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell dropdown entries.
    pub notifications: Vec<ShellNotification>,
    /// Selected date range (0 = 7 days, 1 = 30 days, 2 = 90 days, 3 = 12 months).
    pub range: usize,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
}

impl Default for AnalyticsApp {
    fn default() -> Self {
        Self {
            route: "analytics".into(),
            nav: vec![
                NavSection::new("Overview")
                    .item(NavItem::new("analytics", "Analytics").icon(LineIcon::Chart))
                    .item(NavItem::new("realtime", "Realtime").icon(UiIcon::Activity).badge("Live"))
                    .item(NavItem::new("reports", "Reports").icon(LineIcon::File).child(NavItem::new("reports/weekly", "Weekly")).child(NavItem::new("reports/monthly", "Monthly")).child(NavItem::new("reports/custom", "Custom"))),
                NavSection::new("Audience")
                    .item(NavItem::new("audience", "Visitors").icon(LineIcon::Users))
                    .item(NavItem::new("geo", "Locations").icon(LineIcon::Globe))
                    .item(NavItem::new("devices", "Devices").icon(LineIcon::Phone)),
                NavSection::new("Commerce").item(NavItem::new("orders", "Orders").icon(crate::shells::AppIcon::Cart).badge("24")).item(NavItem::new("products", "Products").icon(crate::shells::AppIcon::Package)),
            ],
            notifications: demo_notes(),
            range: 1,
            scoped_theme: false,
        }
    }
}

const RANGES: [&str; 4] = ["7 days", "30 days", "90 days", "12 months"];

impl AnalyticsApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let range = &mut self.range;
        let r = AppShell::new(&mut self.route, &mut self.nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(demo_workspaces("Acme Analytics", "Business"))
            .user(demo_user())
            .notifications(&mut self.notifications)
            .scoped_theme(self.scoped_theme)
            .id_salt("analytics_app")
            .show(ui, |ui, sc| {
                if sc.route.as_str() == "analytics" {
                    dashboard(ui, sc.rect, range);
                } else {
                    placeholder(ui, sc.rect, sc.route);
                }
            });
        let _ = r;
    }
}

fn labels_for(range: usize) -> Vec<String> {
    match range {
        0 => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().map(|s| s.to_string()).collect(),
        1 => (1..=30).map(|d| format!("Oct {d}")).collect(),
        2 => (1..=13).map(|w| format!("W{}", w + 27)).collect(),
        _ => ["Dec", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov"].iter().map(|s| s.to_string()).collect(),
    }
}

fn dashboard(ui: &mut Ui, rect: Rect, range: &mut usize) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("analytics");
    let mut changed = false;
    let top = page_header(ui, rect, "Analytics", "Traffic, revenue and conversion across all channels.", |ui| {
        if ui.add(Button::new("Export").icon(UiIcon::Download).variant(ButtonVariant::Outline).size(ControlSize::Sm)).clicked() {
            toast(ui.ctx(), Toast::success("Report exported").description(format!("analytics-{}.csv is in your downloads.", RANGES[*range].replace(' ', ""))));
        }
        let before = *range;
        ui.add(Tabs::new(range, &RANGES).style(TabsStyle::Segmented).height(32.0));
        changed = before != *range;
    });
    if changed {
        kit::reset_loading(ui, id.with("load"));
    }
    let loading = kit::loading(ui, id.with("load"), 0.6);
    let body = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
    let k = [0.45, 1.0, 2.6, 9.5][*range];
    let n = [7, 30, 13, 12][*range];
    let labels = labels_for(*range);
    let rev = series(11 + *range as u64, n, 4200.0 * k, 90.0 * k, 900.0 * k);
    let prev = series(91 + *range as u64, n, 3600.0 * k, 60.0 * k, 800.0 * k);
    let visits = series(21 + *range as u64, 12, 900.0, 25.0, 160.0);
    let conv = series(31 + *range as u64, 12, 3.1, 0.02, 0.4);
    let aov = series(41 + *range as u64, 12, 86.0, 0.6, 9.0);
    let total_rev: f64 = rev.iter().sum();
    kit::child(ui, body, |ui| {
        kit::scroll(ui, "analytics_scroll", |ui| {
            ui.spacing_mut().item_spacing.y = 14.0;
            let w = ui.available_width();
            // KPI tiles
            let row = kit::alloc_row(ui, 104.0);
            let ncol = if w > 900.0 { 4 } else { 2 };
            let tiles = kit::cols(row, ncol, 14.0);
            let spark_rev: Vec<f64> = rev.iter().rev().take(12).rev().copied().collect();
            for (i, r) in tiles.iter().enumerate() {
                if loading {
                    kit::skel_r(ui, *r, t.radius);
                    continue;
                }
                let tile = match i {
                    0 => StatTile::new("Revenue", total_rev).prefix("$").delta(12.4 + *range as f64).caption("vs prev.").spark(&spark_rev),
                    1 => StatTile::new("Visitors", visits.iter().sum::<f64>() * k).delta(8.1).caption("vs prev.").spark(&visits),
                    2 => StatTile::new("Conversion", 3.42).suffix("%").decimals(2).delta(-0.8).caption("vs prev.").spark(&conv),
                    _ => StatTile::new("Avg. order", 92.18).prefix("$").decimals(2).delta(4.6).caption("vs prev.").spark(&aov),
                };
                ui.put(*r, tile.size(r.size()).animate(true));
            }
            // revenue + donut
            let row = kit::alloc_row(ui, 300.0);
            let parts = if w > 860.0 { kit::split_x(row, &[0.0, 330.0], 14.0) } else { vec![row] };
            if loading {
                parts.iter().for_each(|r| kit::skel_r(ui, *r, t.radius));
            } else {
                kit::titled_card(ui, parts[0], "Revenue", &format!("{} total · last {}", kit::money(total_rev), RANGES[*range]), |ui, b| {
                    let s = [Series::new("Revenue", rev.clone()), Series::new("Previous period", prev.clone())];
                    ui.add(AreaChart::new(&s).labels(&labels).height(b.height()).format(|v| format!("${}", glowkit_charts::format_compact(v))).id_salt("an_rev"));
                });
                if parts.len() > 1 {
                    kit::titled_card(ui, parts[1], "Traffic sources", "Sessions by channel", |ui, b| {
                        let d = [
                            Datum::new("Organic", 4_820.0 * k),
                            Datum::new("Direct", 2_960.0 * k),
                            Datum::new("Referral", 1_410.0 * k),
                            Datum::new("Social", 1_120.0 * k),
                            Datum::new("Email", 640.0 * k),
                        ];
                        let total: f64 = d.iter().map(|x| x.value).sum();
                        ui.add(DonutChart::new(&d).center_title(glowkit_charts::format_compact(total)).center_caption("Sessions").height(b.height()).id_salt("an_donut"));
                    });
                }
            }
            // top pages + orders
            let row = kit::alloc_row(ui, 330.0);
            let parts = if w > 860.0 { kit::split_x(row, &[340.0, 0.0], 14.0) } else { vec![row] };
            if loading {
                parts.iter().for_each(|r| kit::skel_r(ui, *r, t.radius));
            } else {
                kit::titled_card(ui, parts[0], "Top pages", "Unique views", |ui, _b| {
                    let d = [
                        Datum::new("/", 12_840.0 * k),
                        Datum::new("/pricing", 6_120.0 * k),
                        Datum::new("/blog/egui-in-production", 4_380.0 * k),
                        Datum::new("/docs/getting-started", 3_920.0 * k),
                        Datum::new("/changelog", 2_210.0 * k),
                        Datum::new("/careers", 1_160.0 * k),
                    ];
                    ui.add(BarList::new(&d).row_height(32.0));
                });
                if parts.len() > 1 {
                    kit::titled_card(ui, parts[1], "Recent orders", "Latest transactions from your store", |ui, _b| {
                        let cols = vec![
                            TableColumn::new("Order").fixed(90.0),
                            TableColumn::new("Customer").flex(2.0),
                            TableColumn::new("Status").fixed(110.0),
                            TableColumn::new("Amount").fixed(100.0).right(),
                        ];
                        let data = [
                            ("#3210", "Olivia Martin", "olivia@email.com", "Paid", BadgeVariant::Success, 1999.0),
                            ("#3209", "Jackson Lee", "jackson@email.com", "Pending", BadgeVariant::Warning, 39.0),
                            ("#3208", "Isabella Nguyen", "isabella@email.com", "Refunded", BadgeVariant::Destructive, 299.0),
                            ("#3207", "William Kim", "will@email.com", "Paid", BadgeVariant::Success, 99.0),
                            ("#3206", "Sofia Davis", "sofia@email.com", "Paid", BadgeVariant::Success, 450.0),
                            ("#3205", "Liam Johnson", "liam@email.com", "Shipped", BadgeVariant::Info, 129.0),
                        ];
                        let rows: Vec<Vec<TableCell>> = data
                            .iter()
                            .map(|(o, n, e, s, v, a)| {
                                vec![TableCell::Mono((*o).into()), TableCell::User { name: (*n).into(), email: (*e).into() }, TableCell::Badge((*s).into(), *v), TableCell::Number(*a, kit::money(*a))]
                            })
                            .collect();
                        ui.add(DataTable::new(cols, &rows).row_height(46.0).page_size(4).id_salt("an_orders"));
                    });
                }
            }
        });
    });
}

/// Generic placeholder page for routes a template does not implement.
pub(crate) fn placeholder(ui: &mut Ui, rect: Rect, route: &str) {
    let t = Theme::of(ui);
    let p = ui.painter().clone();
    let title = route.rsplit('/').next().unwrap_or(route);
    let mut title = title.replace('-', " ");
    if let Some(f) = title.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    kit::heading(&p, rect, &title, "This page is part of the template's navigation. Build it from the same pieces.", &t);
    let r = Rect::from_center_size(rect.center() + vec2(0.0, 30.0), vec2(rect.width().min(440.0), 220.0));
    kit::child(ui, r, |ui| {
        ui.add(glowkit_components::EmptyState::new(UiIcon::Grid, format!("Nothing in {title} yet")).description("Connect a data source or create your first item to see it here.").action("Create item").dashed(true).width(r.width()));
    });
    let _ = Align2::CENTER_CENTER;
}
