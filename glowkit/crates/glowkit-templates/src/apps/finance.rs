//! Finance dashboard template.

use egui::{pos2, vec2, Align2, Color32, Rect, Ui};
use glowkit_charts::{BarChart, Datum, ProgressList, Series, Sparkline};
use glowkit_components::{toast, Button, ButtonVariant, ControlSize, Dialog, LineIcon, NumberInput, SegmentedControl, Select, Toast, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces, page_header, series};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// A bank account in the [`FinanceApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct FinanceAccount {
    /// Account name.
    pub name: String,
    /// Masked number.
    pub number: String,
    /// Balance.
    pub balance: f64,
}

/// A transaction in the [`FinanceApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct FinanceTxn {
    /// Merchant / counterparty.
    pub merchant: String,
    /// Category.
    pub category: String,
    /// Date label.
    pub date: String,
    /// Signed amount (negative = spend).
    pub amount: f64,
}

/// A personal / business finance dashboard inside a top-navigation [`AppShell`]: a gradient
/// total-balance card, account cards with sparklines, a cash-flow bar chart, spending by category
/// against budgets, recent transactions with income / expense filters and a transfer dialog
/// that actually moves money between accounts.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::FinanceApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct FinanceApp {
    /// Active route.
    pub route: String,
    /// Top navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Accounts.
    pub accounts: Vec<FinanceAccount>,
    /// Transactions, newest first.
    pub transactions: Vec<FinanceTxn>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    filter: usize,
    transfer_open: bool,
    transfer: (Option<usize>, Option<usize>, f64),
    period: Option<usize>,
}

impl Default for FinanceApp {
    fn default() -> Self {
        let tx = |m: &str, c: &str, d: &str, a: f64| FinanceTxn { merchant: m.into(), category: c.into(), date: d.into(), amount: a };
        Self {
            route: "overview".into(),
            nav: vec![NavSection::new("Main")
                .item(NavItem::new("overview", "Overview").icon(LineIcon::Home))
                .item(NavItem::new("transactions", "Transactions").icon(AppIcon::Receipt))
                .item(NavItem::new("cards", "Cards").icon(LineIcon::CreditCard))
                .item(NavItem::new("budgets", "Budgets").icon(AppIcon::PieChart))
                .item(NavItem::new("invest", "Investments").icon(AppIcon::TrendUp).child(NavItem::new("invest/stocks", "Stocks")).child(NavItem::new("invest/crypto", "Crypto")).child(NavItem::new("invest/bonds", "Bonds")))],
            notifications: demo_notes(),
            accounts: vec![
                FinanceAccount { name: "Checking".into(), number: "•••• 4021".into(), balance: 12_480.32 },
                FinanceAccount { name: "Savings".into(), number: "•••• 7788".into(), balance: 48_210.00 },
                FinanceAccount { name: "Business".into(), number: "•••• 1290".into(), balance: 23_904.75 },
            ],
            transactions: vec![
                tx("Stripe payout", "Income", "Today", 8_420.00),
                tx("Whole Foods Market", "Groceries", "Today", -126.43),
                tx("Uber", "Transport", "Yesterday", -24.90),
                tx("Figma", "Software", "Yesterday", -45.00),
                tx("Blue Bottle Coffee", "Dining", "Nov 9", -8.75),
                tx("Rent · November", "Housing", "Nov 1", -2_400.00),
                tx("Client invoice #1042", "Income", "Oct 30", 3_200.00),
                tx("Spotify", "Subscriptions", "Oct 28", -11.99),
                tx("Delta Air Lines", "Travel", "Oct 26", -412.20),
            ],
            scoped_theme: false,
            filter: 0,
            transfer_open: false,
            transfer: (Some(1), Some(0), 500.0),
            period: Some(0),
        }
    }
}

fn cat_icon(cat: &str) -> AppIcon {
    match cat {
        "Income" => AppIcon::TrendUp,
        "Groceries" => AppIcon::Cart,
        "Transport" => AppIcon::Rocket,
        "Software" | "Subscriptions" => AppIcon::Ui(UiIcon::Code),
        "Dining" => AppIcon::Smile,
        "Housing" => AppIcon::Line(LineIcon::Home),
        "Travel" => AppIcon::Line(LineIcon::Globe),
        _ => AppIcon::Wallet,
    }
}

impl FinanceApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::TopNav)
            .workspaces(demo_workspaces("Ledger", "Personal"))
            .user(demo_user())
            .notifications(&mut notes)
            .scoped_theme(self.scoped_theme)
            .id_salt("finance_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "overview" => self.overview(ui, sc.rect, sc.bounds),
                other => placeholder(ui, sc.rect, other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn overview(&mut self, ui: &mut Ui, rect: Rect, bounds: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("fin");
        let loading = kit::loading(ui, id, 0.6);
        let mut open_transfer = false;
        let top = page_header(ui, rect, "Good morning, Ada", "Here's your money at a glance for November.", |ui| {
            if ui.add(Button::new("Transfer").icon(UiIcon::ArrowUpDown).size(ControlSize::Sm)).clicked() {
                open_transfer = true;
            }
            ui.add(Select::new(&mut self.period, ["This month", "Last month", "This quarter", "This year"]).size(ControlSize::Sm).width(150.0).id_salt("fin_period"));
        });
        if open_transfer {
            self.transfer_open = true;
        }
        let body = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
        let total: f64 = self.accounts.iter().map(|a| a.balance).sum();
        kit::child(ui, body, |ui| {
            kit::scroll(ui, "fin_scroll", |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                let w = ui.available_width();
                // balance cards
                let row = kit::alloc_row(ui, 168.0);
                let parts = kit::split_x(row, &[if w > 900.0 { 380.0 } else { 0.0 }, 0.0, 0.0, 0.0], 14.0);
                if loading {
                    parts.iter().for_each(|r| kit::skel_r(ui, *r, t.radius * 1.4));
                } else {
                    balance_card(ui, parts[0], total);
                    let sparks = [series(5, 14, 11_000.0, 90.0, 600.0), series(6, 14, 46_000.0, 160.0, 300.0), series(7, 14, 22_000.0, 120.0, 900.0)];
                    for (i, r) in parts[1..].iter().enumerate() {
                        let a = &self.accounts[i];
                        kit::card(ui.painter(), *r, &t);
                        let p = ui.painter().clone();
                        let ic = Rect::from_min_size(r.min + vec2(16.0, 16.0), vec2(34.0, 34.0));
                        p.rect_filled(ic, 9.0, with_alpha(t.chart[i], 0.15));
                        [AppIcon::Wallet, AppIcon::Building, AppIcon::Line(LineIcon::CreditCard)][i].paint(&p, Rect::from_center_size(ic.center(), vec2(17.0, 17.0)), t.chart[i]);
                        kit::text_at(&p, pos2(ic.max.x + 10.0, ic.center().y - 8.0), Align2::LEFT_CENTER, &a.name, 13.5, t.foreground, r.width() - 80.0);
                        kit::text_at(&p, pos2(ic.max.x + 10.0, ic.center().y + 9.0), Align2::LEFT_CENTER, &a.number, 12.0, t.muted_foreground, r.width() - 80.0);
                        kit::text_at(&p, r.min + vec2(16.0, 82.0), Align2::LEFT_CENTER, &kit::money(a.balance), 22.0, t.foreground, r.width() - 32.0);
                        let sr = Rect::from_min_max(pos2(r.min.x + 16.0, r.max.y - 56.0), r.max - vec2(16.0, 14.0));
                        ui.put(sr, Sparkline::new(&sparks[i]).size(sr.size()).color(t.chart[i]));
                    }
                }
                // cash flow + spending
                let row = kit::alloc_row(ui, 330.0);
                let parts = if w > 860.0 { kit::split_x(row, &[0.0, 360.0], 14.0) } else { vec![row] };
                if loading {
                    parts.iter().for_each(|r| kit::skel_r(ui, *r, t.radius));
                } else {
                    kit::titled_card(ui, parts[0], "Cash flow", "Income vs. expenses, last 6 months", |ui, b| {
                        let s = [Series::new("Income", vec![9_200.0, 10_400.0, 9_800.0, 12_100.0, 11_600.0, 13_900.0]).color(t.success), Series::new("Expenses", vec![6_100.0, 7_300.0, 6_900.0, 7_800.0, 8_400.0, 7_200.0]).color(t.chart[0])];
                        ui.add(BarChart::new(&s).labels(&["Jun", "Jul", "Aug", "Sep", "Oct", "Nov"]).radius(5.0).height(b.height()).format(|v| format!("${}", glowkit_charts::format_compact(v))).id_salt("fin_flow"));
                    });
                    if parts.len() > 1 {
                        kit::titled_card(ui, parts[1], "Spending by category", "This month vs. budget", |ui, _b| {
                            let d = [
                                Datum::new("Housing", 2_400.0),
                                Datum::new("Groceries", 612.0),
                                Datum::new("Travel", 412.0),
                                Datum::new("Dining", 286.0),
                                Datum::new("Transport", 154.0),
                                Datum::new("Software", 98.0),
                            ];
                            let targets = [2_400.0, 700.0, 300.0, 350.0, 200.0, 120.0];
                            ui.add(ProgressList::new(&d, &targets).format(|v| format!("${}", glowkit_charts::format_compact(v))));
                        });
                    }
                }
                // transactions
                let n = match self.filter {
                    1 => self.transactions.iter().filter(|x| x.amount > 0.0).count(),
                    2 => self.transactions.iter().filter(|x| x.amount < 0.0).count(),
                    _ => self.transactions.len(),
                };
                let row = kit::alloc_row(ui, 86.0 + n as f32 * 52.0);
                if loading {
                    kit::skel_r(ui, row, t.radius);
                } else {
                    kit::card(ui.painter(), row, &t);
                    let p = ui.painter().clone();
                    kit::text_at(&p, row.min + vec2(18.0, 26.0), Align2::LEFT_CENTER, "Recent transactions", 15.0, t.foreground, 260.0);
                    let fr = Rect::from_min_size(pos2(row.max.x - 18.0 - 260.0, row.min.y + 10.0), vec2(260.0, 32.0));
                    kit::child(ui, fr, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(SegmentedControl::new(&mut self.filter, ["All", "Income", "Expenses"]).size(ControlSize::Sm));
                        });
                    });
                    let mut y = row.min.y + 60.0;
                    for (i, x) in self.transactions.iter().enumerate() {
                        let show = match self.filter {
                            1 => x.amount > 0.0,
                            2 => x.amount < 0.0,
                            _ => true,
                        };
                        if !show {
                            continue;
                        }
                        let rr = Rect::from_min_size(pos2(row.min.x + 10.0, y), vec2(row.width() - 20.0, 48.0));
                        y += 52.0;
                        let (resp, _) = kit::row(ui, rr, id.with(("tx", i)), t.radius * 0.7);
                        let ic = pos2(rr.min.x + 24.0, rr.center().y);
                        let pos = x.amount > 0.0;
                        p.circle_filled(ic, 17.0, if pos { with_alpha(t.success, 0.14) } else { t.muted });
                        cat_icon(&x.category).paint(&p, Rect::from_center_size(ic, vec2(16.0, 16.0)), if pos { t.success } else { t.muted_foreground });
                        kit::text_at(&p, pos2(rr.min.x + 52.0, rr.center().y - 8.0), Align2::LEFT_CENTER, &x.merchant, 13.5, t.foreground, rr.width() * 0.4);
                        kit::text_at(&p, pos2(rr.min.x + 52.0, rr.center().y + 9.0), Align2::LEFT_CENTER, &x.date, 12.0, t.muted_foreground, rr.width() * 0.4);
                        kit::pill(&p, pos2(rr.min.x + rr.width() * 0.55, rr.center().y), false, &x.category, t.muted_foreground, t.muted, 11.0);
                        let amt = if pos { format!("+{}", kit::money(x.amount)) } else { kit::money(x.amount) };
                        kit::text_at(&p, pos2(rr.max.x - 10.0, rr.center().y), Align2::RIGHT_CENTER, &amt, 14.0, if pos { t.success } else { t.foreground }, 140.0);
                        if resp.clicked() {
                            toast(ui.ctx(), Toast::new(x.merchant.clone()).description(format!("{} · {} · {}", x.category, x.date, amt)));
                        }
                    }
                }
            });
        });
        self.transfer_dialog(ui, bounds);
    }

    fn transfer_dialog(&mut self, ui: &mut Ui, bounds: Rect) {
        let names: Vec<String> = self.accounts.iter().map(|a| format!("{} ({})", a.name, a.number)).collect();
        let mut open = self.transfer_open;
        let tr = &mut self.transfer;
        let r = Dialog::new(&mut open, "Transfer money").description("Move funds between your accounts instantly.").primary("Transfer").secondary("Cancel").width(420.0).within(bounds).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            let w = ui.available_width();
            ui.add(Select::new(&mut tr.0, names.clone()).label("From").width(w).id_salt("fin_from"));
            ui.add(Select::new(&mut tr.1, names.clone()).label("To").width(w).id_salt("fin_to"));
            ui.add(NumberInput::new(&mut tr.2).label("Amount").prefix("$").range(1.0..=100_000.0).step(50.0).decimals(2).width(w));
        });
        if r.primary_clicked {
            let (f, to, amt) = self.transfer;
            match (f, to) {
                (Some(f), Some(to)) if f != to && self.accounts[f].balance >= amt => {
                    self.accounts[f].balance -= amt;
                    self.accounts[to].balance += amt;
                    self.transactions.insert(0, FinanceTxn { merchant: format!("Transfer to {}", self.accounts[to].name), category: "Transfer".into(), date: "Just now".into(), amount: -amt });
                    toast(ui.ctx(), Toast::success(format!("{} sent", kit::money(amt))).description(format!("{} → {}", self.accounts[f].name, self.accounts[to].name)).action("Undo"));
                }
                (Some(f), Some(to)) if f == to => {
                    toast(ui.ctx(), Toast::error("Pick two different accounts"));
                    open = true;
                }
                _ => {
                    toast(ui.ctx(), Toast::error("Insufficient funds").description("Lower the amount or choose another account."));
                    open = true;
                }
            }
        }
        self.transfer_open = open;
        let _ = ButtonVariant::Default;
    }
}

/// The gradient "total balance" card.
fn balance_card(ui: &mut Ui, r: Rect, total: f64) {
    let t = Theme::of(ui);
    let p = ui.painter().clone();
    paint::soft_shadow(&p, r, t.radius * 1.4, 18.0, vec2(0.0, 8.0), with_alpha(t.gradient[0], 0.25));
    paint::gradient_rect(&p, r, t.radius * 1.4, 35.0, &[t.gradient[0], t.gradient[1]]);
    // subtle rings
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    for (k, rad) in [90.0, 130.0].iter().enumerate() {
        pc.circle_stroke(r.right_top() + vec2(-30.0, 10.0), *rad, egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.10 + 0.04 * k as f32)));
    }
    let w = Color32::WHITE;
    kit::text_at(&p, r.min + vec2(20.0, 24.0), Align2::LEFT_CENTER, "Total balance", 13.0, with_alpha(w, 0.8), 200.0);
    let shown = kit::follow(ui.ctx(), ui.scope_id().with("bal"), total as f32, 6.0) as f64;
    kit::text_at(&p, r.min + vec2(20.0, 62.0), Align2::LEFT_CENTER, &kit::money(shown), 30.0, w, r.width() - 40.0);
    kit::pill(&p, r.min + vec2(20.0, 98.0), false, "▲ 4.2% this month", w, with_alpha(w, 0.18), 11.5);
    kit::text_at(&p, pos2(r.min.x + 20.0, r.max.y - 24.0), Align2::LEFT_CENTER, "Ada Lovelace", 13.0, with_alpha(w, 0.9), 200.0);
    kit::mono_at(&p, pos2(r.max.x - 20.0, r.max.y - 24.0), Align2::RIGHT_CENTER, "•••• •••• •••• 4021", 12.5, with_alpha(w, 0.9), 200.0);
}
