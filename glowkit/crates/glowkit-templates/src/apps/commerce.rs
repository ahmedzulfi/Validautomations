//! E-commerce admin template: orders, products grid and inventory.

use egui::{pos2, vec2, Align2, Color32, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{
    toast, Alert, AlertVariant, Button, ButtonVariant, ControlSize, Input, LineIcon, Select, Tabs, TabsStyle, Toast,
    ToggleGroup, UiIcon,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

use super::analytics::placeholder;
use super::common::{demo_notes, demo_user, demo_workspaces, page_header};
use crate::shells::kit::{self, Tone};
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// A product in the [`CommerceApp`] catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopProduct {
    /// Name.
    pub name: String,
    /// Category.
    pub category: String,
    /// Price.
    pub price: f64,
    /// Units in stock.
    pub stock: u32,
    /// Reorder threshold.
    pub threshold: u32,
    /// Average rating (0–5).
    pub rating: f32,
    /// Featured on the storefront.
    pub featured: bool,
    /// Art seed.
    pub hue: usize,
}

/// An order in the [`CommerceApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct ShopOrder {
    /// Order number.
    pub number: u32,
    /// Customer name.
    pub customer: String,
    /// Date label.
    pub date: String,
    /// Item count.
    pub items: u32,
    /// Paid?
    pub paid: bool,
    /// Fulfilled?
    pub fulfilled: bool,
    /// Total.
    pub total: f64,
}

/// An e-commerce admin inside a floating-sidebar [`AppShell`]: an orders list with status tabs
/// and fulfil actions, a filterable product grid (search, category, sort, grid / list toggle,
/// featured stars) and an inventory view with low-stock alerts and one-click reordering.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::CommerceApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct CommerceApp {
    /// Active route ("orders", "products", "inventory").
    pub route: String,
    /// Sidebar navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Catalog.
    pub products: Vec<ShopProduct>,
    /// Orders.
    pub orders: Vec<ShopOrder>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    order_tab: usize,
    query: String,
    category: usize,
    sort: Option<usize>,
    view: usize,
}

const CATS: [&str; 5] = ["All", "Lighting", "Audio", "Furniture", "Accessories"];

impl Default for CommerceApp {
    fn default() -> Self {
        let p = |n: &str, c: &str, price: f64, stock: u32, th: u32, r: f32, hue: usize| ShopProduct {
            name: n.into(),
            category: c.into(),
            price,
            stock,
            threshold: th,
            rating: r,
            featured: hue % 3 == 0,
            hue,
        };
        let o = |n: u32, c: &str, d: &str, items: u32, paid: bool, ful: bool, total: f64| ShopOrder {
            number: n,
            customer: c.into(),
            date: d.into(),
            items,
            paid,
            fulfilled: ful,
            total,
        };
        Self {
            route: "orders".into(),
            nav: vec![
                NavSection::new("Store")
                    .item(NavItem::new("home", "Home").icon(LineIcon::Home))
                    .item(
                        NavItem::new("orders", "Orders")
                            .icon(AppIcon::Cart)
                            .badge("6")
                            .child(NavItem::new("orders", "All orders"))
                            .child(NavItem::new("drafts", "Drafts"))
                            .child(NavItem::new("abandoned", "Abandoned checkouts"))
                            .expanded(true),
                    )
                    .item(NavItem::new("products", "Products").icon(AppIcon::Package))
                    .item(NavItem::new("inventory", "Inventory").icon(AppIcon::Archive).badge("3"))
                    .item(NavItem::new("customers", "Customers").icon(LineIcon::Users)),
                NavSection::new("Sales channels")
                    .item(NavItem::new("storefront", "Online store").icon(LineIcon::Globe))
                    .item(NavItem::new("pos", "Point of sale").icon(AppIcon::Receipt)),
            ],
            notifications: demo_notes(),
            products: vec![
                p("Aurora Desk Lamp", "Lighting", 89.0, 42, 10, 4.7, 0),
                p("Halo Floor Light", "Lighting", 249.0, 4, 8, 4.5, 1),
                p("Pulse Wireless Buds", "Audio", 129.0, 118, 25, 4.3, 2),
                p("Studio Monitor S2", "Audio", 399.0, 9, 6, 4.8, 3),
                p("Oak Standing Desk", "Furniture", 649.0, 2, 5, 4.6, 4),
                p("Mesh Task Chair", "Furniture", 329.0, 27, 8, 4.2, 5),
                p("Felt Desk Mat", "Accessories", 39.0, 210, 40, 4.4, 6),
                p("Cable Organizer Kit", "Accessories", 24.0, 7, 30, 4.1, 7),
            ],
            orders: vec![
                o(3210, "Olivia Martin", "Today, 10:42", 3, true, false, 1_047.0),
                o(3209, "Jackson Lee", "Today, 09:15", 1, false, false, 129.0),
                o(3208, "Isabella Nguyen", "Yesterday", 2, true, true, 418.0),
                o(3207, "William Kim", "Yesterday", 1, true, false, 649.0),
                o(3206, "Sofia Davis", "Nov 8", 4, true, true, 302.0),
                o(3205, "Liam Johnson", "Nov 8", 2, false, false, 728.0),
                o(3204, "Emma Wilson", "Nov 7", 1, true, true, 89.0),
                o(3203, "Noah Brown", "Nov 7", 5, true, false, 1_284.0),
                o(3202, "Ava Garcia", "Nov 6", 2, true, true, 63.0),
            ],
            scoped_theme: false,
            order_tab: 0,
            query: String::new(),
            category: 0,
            sort: Some(0),
            view: 0,
        }
    }
}

impl CommerceApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let low = self.products.iter().filter(|p| p.stock < p.threshold).count();
        if let Some(it) = nav.get_mut(0).and_then(|s| s.items.get_mut(3)) {
            it.badge = if low > 0 { Some(low.to_string()) } else { None };
        }
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Floating)
            .workspaces(demo_workspaces("Northwind Store", "Shopify Plus"))
            .user(demo_user())
            .notifications(&mut notes)
            .scoped_theme(self.scoped_theme)
            .id_salt("commerce_app")
            .show(ui, |ui, sc| match sc.route.as_str() {
                "orders" => self.orders_page(ui, sc.rect, sc.route),
                "products" => self.products_page(ui, sc.rect),
                "inventory" => self.inventory_page(ui, sc.rect),
                other => placeholder(ui, sc.rect, other),
            });
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn orders_page(&mut self, ui: &mut Ui, rect: Rect, route: &mut String) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("shop_orders");
        let loading = kit::loading(ui, id, 0.6);
        let unf = self.orders.iter().filter(|o| !o.fulfilled).count();
        let top = page_header(ui, rect, "Orders", &format!("{} orders · {unf} to fulfil", self.orders.len()), |ui| {
            if ui.add(Button::new("Create order").icon(UiIcon::Plus).size(ControlSize::Sm)).clicked() {
                let n = self.orders.iter().map(|o| o.number).max().unwrap_or(3200) + 1;
                self.orders.insert(
                    0,
                    ShopOrder {
                        number: n,
                        customer: "Walk-in customer".into(),
                        date: "Just now".into(),
                        items: 1,
                        paid: false,
                        fulfilled: false,
                        total: 89.0,
                    },
                );
                toast(ui.ctx(), Toast::success(format!("Draft order #{n} created")));
            }
            if ui
                .add(Button::new("Export").icon(UiIcon::Download).variant(ButtonVariant::Outline).size(ControlSize::Sm))
                .clicked()
            {
                toast(ui.ctx(), Toast::success("Orders exported").description("orders.csv · 9 rows"));
            }
        });
        let mut y = top;
        // inventory alert
        let low: Vec<String> = self.products.iter().filter(|p| p.stock < p.threshold).map(|p| p.name.clone()).collect();
        if !low.is_empty() {
            let ar = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), 62.0));
            let resp = kit::child(ui, ar, |ui| {
                ui.add(
                    Alert::new(format!("{} products are running low", low.len()))
                        .description(format!("{} — reorder before the weekend rush.", low.join(", ")))
                        .variant(AlertVariant::Warning)
                        .width(ar.width()),
                )
            });
            let br = Rect::from_min_size(pos2(ar.max.x - 130.0, ar.center().y - 15.0), vec2(116.0, 30.0));
            if kit::put(
                ui,
                br,
                Button::new("Review stock")
                    .variant(ButtonVariant::Outline)
                    .size(ControlSize::Sm)
                    .trailing_icon(UiIcon::ArrowRight),
            )
            .clicked()
            {
                *route = "inventory".into();
            }
            y += resp.rect.height().max(56.0) + 14.0;
        }
        // KPIs
        let kr = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), 74.0));
        let paid_total: f64 = self.orders.iter().filter(|o| o.paid).map(|o| o.total).sum();
        let kpis = [
            ("Orders today", "2".to_string(), "+18% vs last Tue", Tone::Success),
            ("Revenue (7d)", kit::money(paid_total), "+9.4% vs prev. week", Tone::Success),
            ("Avg. fulfilment", "1.6 days".to_string(), "-0.3 days", Tone::Success),
            ("Returns", "1.2%".to_string(), "+0.2 pts", Tone::Danger),
        ];
        for (i, r) in kit::cols(kr, 4, 14.0).iter().enumerate() {
            if loading {
                kit::skel_r(ui, *r, t.radius);
                continue;
            }
            kit::card(ui.painter(), *r, &t);
            let p = ui.painter();
            let (l, v, d, tone) = &kpis[i];
            kit::text_at(
                p,
                r.min + vec2(16.0, 18.0),
                Align2::LEFT_CENTER,
                l,
                12.5,
                t.muted_foreground,
                r.width() - 32.0,
            );
            let vr =
                kit::text_at(p, r.min + vec2(16.0, 46.0), Align2::LEFT_CENTER, v, 20.0, t.foreground, r.width() - 32.0);
            kit::text_at(
                p,
                pos2(vr.max.x + 10.0, r.min.y + 48.0),
                Align2::LEFT_CENTER,
                d,
                11.5,
                kit::tone(&t, *tone),
                r.max.x - vr.max.x - 24.0,
            );
        }
        y = kr.max.y + 14.0;
        // tabs
        let tabs = ["All", "Unfulfilled", "Unpaid", "Fulfilled"];
        kit::child(ui, Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), 36.0)), |ui| {
            ui.add(Tabs::new(&mut self.order_tab, &tabs).style(TabsStyle::Underline).height(34.0));
        });
        y += 44.0;
        let list: Vec<usize> = (0..self.orders.len())
            .filter(|&i| {
                let o = &self.orders[i];
                match self.order_tab {
                    1 => !o.fulfilled,
                    2 => !o.paid,
                    3 => o.fulfilled,
                    _ => true,
                }
            })
            .collect();
        let tr = Rect::from_min_max(pos2(rect.min.x, y), rect.max);
        kit::card(ui.painter(), tr, &t);
        let widths = [90.0, 0.0, 120.0, 70.0, 110.0, 120.0, 100.0, 110.0];
        let head = Rect::from_min_size(tr.min, vec2(tr.width(), 38.0));
        let p = ui.painter().clone();
        for (k, (c, title)) in kit::split_x(head.shrink2(vec2(16.0, 0.0)), &widths, 10.0)
            .iter()
            .zip(["Order", "Customer", "Date", "Items", "Payment", "Fulfilment", "Total", ""])
            .enumerate()
        {
            let right = k == 6;
            kit::text_at(
                &p,
                if right { c.right_center() } else { c.left_center() },
                if right { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER },
                title,
                12.0,
                t.muted_foreground,
                c.width(),
            );
        }
        kit::hline(&p, tr.min.x, tr.max.x, head.max.y, &t);
        let rh = 46.0;
        for (ri, &oi) in list.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(tr.min.x, head.max.y + ri as f32 * rh), vec2(tr.width(), rh));
            if rr.max.y > tr.max.y - 2.0 {
                break;
            }
            let cells = kit::split_x(rr.shrink2(vec2(16.0, 0.0)), &widths, 10.0);
            if loading {
                for c in cells.iter().take(7) {
                    kit::skel(ui, Rect::from_min_size(pos2(c.min.x, rr.center().y - 4.0), vec2(c.width() * 0.6, 8.0)));
                }
                continue;
            }
            let (resp, _) = kit::row(ui, rr.shrink2(vec2(1.0, 0.0)), id.with(("o", oi)), 0.0);
            if ri > 0 {
                p.line_segment(
                    [pos2(tr.min.x + 1.0, rr.min.y), pos2(tr.max.x - 1.0, rr.min.y)],
                    Stroke::new(1.0, with_alpha(t.border, 0.6)),
                );
            }
            let o = self.orders[oi].clone();
            kit::mono_at(
                &p,
                cells[0].left_center(),
                Align2::LEFT_CENTER,
                &format!("#{}", o.number),
                12.5,
                t.foreground,
                cells[0].width(),
            );
            kit::avatar(
                ui,
                Rect::from_center_size(pos2(cells[1].min.x + 12.0, rr.center().y), vec2(24.0, 24.0)),
                &o.customer,
            );
            kit::text_at(
                &p,
                pos2(cells[1].min.x + 32.0, rr.center().y),
                Align2::LEFT_CENTER,
                &o.customer,
                13.0,
                t.foreground,
                cells[1].width() - 34.0,
            );
            kit::text_at(
                &p,
                cells[2].left_center(),
                Align2::LEFT_CENTER,
                &o.date,
                12.5,
                t.muted_foreground,
                cells[2].width(),
            );
            kit::text_at(
                &p,
                cells[3].left_center(),
                Align2::LEFT_CENTER,
                &format!("{} item{}", o.items, if o.items == 1 { "" } else { "s" }),
                12.5,
                t.muted_foreground,
                cells[3].width(),
            );
            let (pl, pt) = if o.paid { ("Paid", Tone::Success) } else { ("Pending", Tone::Warning) };
            kit::status_pill(&p, cells[4].left_center(), false, pl, kit::tone(&t, pt), &t);
            let (fl, ft) = if o.fulfilled { ("Fulfilled", Tone::Neutral) } else { ("Unfulfilled", Tone::Info) };
            kit::status_pill(&p, cells[5].left_center(), false, fl, kit::tone(&t, ft), &t);
            kit::text_at(
                &p,
                cells[6].right_center(),
                Align2::RIGHT_CENTER,
                &kit::money(o.total),
                13.0,
                t.foreground,
                cells[6].width(),
            );
            if !o.fulfilled {
                let br = Rect::from_min_size(pos2(cells[7].max.x - 84.0, rr.center().y - 14.0), vec2(84.0, 28.0));
                if kit::put(
                    ui,
                    br,
                    Button::new("Fulfil").variant(ButtonVariant::Outline).size(ControlSize::Sm).width(84.0),
                )
                .clicked()
                {
                    self.orders[oi].fulfilled = true;
                    toast(
                        ui.ctx(),
                        Toast::success(format!("Order #{} fulfilled", o.number))
                            .description(format!("Shipping label created for {}.", o.customer))
                            .action("Undo"),
                    );
                }
            } else {
                AppIcon::Ui(UiIcon::CheckCircle).paint(
                    &p,
                    Rect::from_center_size(pos2(cells[7].max.x - 12.0, rr.center().y), vec2(16.0, 16.0)),
                    t.success,
                );
            }
            if resp.clicked() {
                toast(
                    ui.ctx(),
                    Toast::new(format!("Order #{}", o.number)).description(format!(
                        "{} · {} · {}",
                        o.customer,
                        kit::money(o.total),
                        o.date
                    )),
                );
            }
        }
        if list.is_empty() && !loading {
            p.text(
                tr.center(),
                Align2::CENTER_CENTER,
                "No orders here. Nice work!",
                egui::FontId::proportional(14.0),
                t.muted_foreground,
            );
        }
    }

    fn products_page(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("shop_products");
        let loading = kit::loading(ui, id, 0.6);
        let top = page_header(
            ui,
            rect,
            "Products",
            &format!(
                "{} products · {} featured",
                self.products.len(),
                self.products.iter().filter(|p| p.featured).count()
            ),
            |ui| {
                if ui.add(Button::new("Add product").icon(UiIcon::Plus).size(ControlSize::Sm)).clicked() {
                    let hue = self.products.len();
                    self.products.push(ShopProduct {
                        name: format!("New product {}", hue + 1),
                        category: "Accessories".into(),
                        price: 49.0,
                        stock: 0,
                        threshold: 10,
                        rating: 0.0,
                        featured: false,
                        hue,
                    });
                    toast(
                        ui.ctx(),
                        Toast::success("Product created").description("Add photos and stock before publishing."),
                    );
                }
            },
        );
        let bar = Rect::from_min_size(pos2(rect.min.x, top - 4.0), vec2(rect.width(), 44.0));
        {
            let cy = bar.center().y;
            kit::put(
                ui,
                Rect::from_min_size(pos2(bar.min.x, cy - 16.0), vec2(220.0, 32.0)),
                Input::new(&mut self.query)
                    .placeholder("Search products…")
                    .icon(UiIcon::Search)
                    .clearable(true)
                    .width(220.0)
                    .size(ControlSize::Sm)
                    .id_salt("shop_q"),
            );
            let right_w = 84.0 + 8.0 + 180.0;
            let chips =
                Rect::from_min_max(pos2(bar.min.x + 232.0, cy - 16.0), pos2(bar.max.x - right_w - 8.0, cy + 16.0));
            kit::child(ui, chips, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (i, c) in CATS.iter().enumerate() {
                        if ui.add(glowkit_components::Chip::new(*c).selected(self.category == i)).clicked() {
                            self.category = i;
                        }
                    }
                });
            });
            kit::put(
                ui,
                Rect::from_min_size(pos2(bar.max.x - 72.0, cy - 16.0), vec2(72.0, 32.0)),
                ToggleGroup::single(&mut self.view, [UiIcon::Grid, UiIcon::List]).size(ControlSize::Sm),
            );
            kit::put(
                ui,
                Rect::from_min_size(pos2(bar.max.x - right_w, cy - 16.0), vec2(180.0, 32.0)),
                Select::new(&mut self.sort, ["Featured", "Price: low to high", "Price: high to low", "Best rated"])
                    .size(ControlSize::Sm)
                    .width(180.0)
                    .id_salt("shop_sort"),
            );
        }
        let q = self.query.to_lowercase();
        let mut idx: Vec<usize> = (0..self.products.len())
            .filter(|&i| {
                (self.category == 0 || self.products[i].category == CATS[self.category])
                    && (q.is_empty() || self.products[i].name.to_lowercase().contains(&q))
            })
            .collect();
        match self.sort {
            Some(1) => idx.sort_by(|a, b| self.products[*a].price.total_cmp(&self.products[*b].price)),
            Some(2) => idx.sort_by(|a, b| self.products[*b].price.total_cmp(&self.products[*a].price)),
            Some(3) => idx.sort_by(|a, b| self.products[*b].rating.total_cmp(&self.products[*a].rating)),
            _ => idx.sort_by_key(|i| !self.products[*i].featured),
        }
        let area = Rect::from_min_max(pos2(rect.min.x, bar.max.y + 16.0), rect.max);
        kit::child(ui, area, |ui| {
            kit::scroll(ui, "shop_grid", |ui| {
                let w = ui.available_width();
                if self.view == 0 {
                    let ncol = ((w + 14.0) / 220.0).floor().max(1.0) as usize;
                    for chunk in idx.chunks(ncol) {
                        let row = kit::alloc_row(ui, 268.0);
                        for (k, r) in kit::cols(row, ncol, 14.0).iter().enumerate() {
                            if let Some(&pi) = chunk.get(k) {
                                if loading {
                                    kit::skel_r(ui, *r, t.radius);
                                } else {
                                    self.product_card(ui, *r, pi);
                                }
                            }
                        }
                        ui.add_space(6.0);
                    }
                } else {
                    for &pi in &idx {
                        let r = kit::alloc_row(ui, 64.0);
                        if loading {
                            kit::skel_r(ui, r, t.radius);
                            continue;
                        }
                        self.product_row(ui, r, pi);
                    }
                }
                if idx.is_empty() {
                    let r = kit::alloc_row(ui, 160.0);
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        "No products match",
                        egui::FontId::proportional(14.0),
                        t.muted_foreground,
                    );
                }
            });
        });
    }

    fn product_card(&mut self, ui: &mut Ui, r: Rect, pi: usize) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("prod", pi));
        let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hv = kit::hover(ui, id, resp.hovered());
        let r = r.translate(vec2(0.0, -2.0 * hv));
        let p = ui.painter().clone();
        if hv > 0.01 {
            paint::soft_shadow(
                &p,
                r,
                t.radius,
                16.0 * hv,
                vec2(0.0, 6.0),
                with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.1 } * hv),
            );
        }
        kit::card(&p, r, &t);
        let pr = &self.products[pi];
        let img = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(r.width() - 16.0, 150.0));
        product_art(&p, img, pr.hue, &t);
        // stock badge
        let low = pr.stock < pr.threshold;
        let (sl, st) = if pr.stock == 0 {
            ("Out of stock".to_string(), Tone::Danger)
        } else if low {
            (format!("Low · {}", pr.stock), Tone::Warning)
        } else {
            (format!("{} in stock", pr.stock), Tone::Success)
        };
        let sc = kit::tone(&t, st);
        kit::pill(
            &p,
            img.left_top() + vec2(10.0, 18.0),
            false,
            &sl,
            if t.dark { mix(sc, Color32::WHITE, 0.4) } else { glowkit_core::color::shade(sc, -0.2) },
            with_alpha(t.card, 0.92),
            11.0,
        );
        // featured star
        let sr = Rect::from_center_size(img.right_top() + vec2(-20.0, 20.0), vec2(28.0, 28.0));
        let sresp = ui.interact(sr, id.with("star"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        p.circle_filled(sr.center(), 14.0, with_alpha(t.card, 0.92));
        let star_c = if pr.featured { t.warning } else { t.muted_foreground };
        if pr.featured {
            paint::star(&p, sr.center(), 7.5, 3.4, 5, -std::f32::consts::FRAC_PI_2, star_c);
        } else {
            LineIcon::Star.paint(&p, Rect::from_center_size(sr.center(), vec2(15.0, 15.0)), star_c);
        }
        kit::tip(&sresp, if pr.featured { "Remove from featured" } else { "Feature on storefront" }, None);
        let name = pr.name.clone();
        kit::text_at(
            &p,
            pos2(r.min.x + 14.0, img.max.y + 20.0),
            Align2::LEFT_CENTER,
            &pr.name,
            14.0,
            t.foreground,
            r.width() - 28.0,
        );
        kit::text_at(
            &p,
            pos2(r.min.x + 14.0, img.max.y + 40.0),
            Align2::LEFT_CENTER,
            &pr.category,
            12.0,
            t.muted_foreground,
            r.width() - 90.0,
        );
        kit::text_at(
            &p,
            pos2(r.min.x + 14.0, img.max.y + 72.0),
            Align2::LEFT_CENTER,
            &kit::money(pr.price),
            16.0,
            t.foreground,
            120.0,
        );
        if pr.rating > 0.0 {
            paint::star(
                &p,
                pos2(r.max.x - 48.0, img.max.y + 72.0),
                6.0,
                2.7,
                5,
                -std::f32::consts::FRAC_PI_2,
                t.warning,
            );
            kit::text_at(
                &p,
                pos2(r.max.x - 14.0, img.max.y + 72.0),
                Align2::RIGHT_CENTER,
                &format!("{:.1}", pr.rating),
                12.5,
                t.muted_foreground,
                40.0,
            );
        }
        if sresp.clicked() {
            let f = !self.products[pi].featured;
            self.products[pi].featured = f;
            toast(
                ui.ctx(),
                Toast::new(if f { format!("{name} is featured") } else { format!("{name} removed from featured") }),
            );
        } else if resp.clicked() {
            toast(ui.ctx(), Toast::new(name).description("Product editor would open here."));
        }
    }

    fn product_row(&mut self, ui: &mut Ui, r: Rect, pi: usize) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("prow", pi));
        let (resp, _) = kit::row(ui, r, id, t.radius);
        let p = ui.painter().clone();
        p.rect_stroke(r, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
        let pr = &self.products[pi];
        product_art(&p, Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(48.0, 48.0)), pr.hue, &t);
        kit::text_at(
            &p,
            pos2(r.min.x + 70.0, r.center().y - 8.0),
            Align2::LEFT_CENTER,
            &pr.name,
            13.5,
            t.foreground,
            r.width() * 0.4,
        );
        kit::text_at(
            &p,
            pos2(r.min.x + 70.0, r.center().y + 9.0),
            Align2::LEFT_CENTER,
            &pr.category,
            12.0,
            t.muted_foreground,
            r.width() * 0.4,
        );
        let low = pr.stock < pr.threshold;
        kit::status_pill(
            &p,
            pos2(r.min.x + r.width() * 0.55, r.center().y),
            false,
            &format!("{} in stock", pr.stock),
            kit::tone(&t, if low { Tone::Warning } else { Tone::Success }),
            &t,
        );
        kit::text_at(
            &p,
            pos2(r.max.x - 16.0, r.center().y),
            Align2::RIGHT_CENTER,
            &kit::money(pr.price),
            14.0,
            t.foreground,
            100.0,
        );
        if resp.clicked() {
            toast(ui.ctx(), Toast::new(pr.name.clone()).description("Product editor would open here."));
        }
    }

    fn inventory_page(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("shop_inv");
        let loading = kit::loading(ui, id, 0.5);
        let low = self.products.iter().filter(|p| p.stock < p.threshold).count();
        let top = page_header(ui, rect, "Inventory", &format!("{low} SKUs below their reorder point"), |ui| {
            if ui
                .add(Button::new("Reorder all low").icon(UiIcon::Upload).size(ControlSize::Sm).disabled(low == 0))
                .clicked()
            {
                for p in self.products.iter_mut().filter(|p| p.stock < p.threshold) {
                    p.stock += p.threshold * 3;
                }
                toast(
                    ui.ctx(),
                    Toast::success(format!("{low} purchase orders sent"))
                        .description("Suppliers were emailed. ETA 3–5 days."),
                );
            }
        });
        let tr = Rect::from_min_max(pos2(rect.min.x, top), rect.max);
        kit::card(ui.painter(), tr, &t);
        let widths = [0.0, 120.0, 220.0, 90.0, 120.0];
        let head = Rect::from_min_size(tr.min, vec2(tr.width(), 38.0));
        let p = ui.painter().clone();
        for (c, title) in kit::split_x(head.shrink2(vec2(16.0, 0.0)), &widths, 12.0).iter().zip([
            "Product",
            "SKU",
            "Stock level",
            "Available",
            "",
        ]) {
            kit::text_at(&p, c.left_center(), Align2::LEFT_CENTER, title, 12.0, t.muted_foreground, c.width());
        }
        kit::hline(&p, tr.min.x, tr.max.x, head.max.y, &t);
        let mut order: Vec<usize> = (0..self.products.len()).collect();
        order.sort_by(|a, b| {
            let ra = self.products[*a].stock as f32 / self.products[*a].threshold.max(1) as f32;
            let rb = self.products[*b].stock as f32 / self.products[*b].threshold.max(1) as f32;
            ra.total_cmp(&rb)
        });
        for (ri, &pi) in order.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(tr.min.x, head.max.y + ri as f32 * 58.0), vec2(tr.width(), 58.0));
            if rr.max.y > tr.max.y {
                break;
            }
            let cells = kit::split_x(rr.shrink2(vec2(16.0, 0.0)), &widths, 12.0);
            if loading {
                kit::skel(
                    ui,
                    Rect::from_min_size(pos2(cells[0].min.x, rr.center().y - 5.0), vec2(cells[0].width() * 0.5, 10.0)),
                );
                kit::skel(
                    ui,
                    Rect::from_min_size(pos2(cells[2].min.x, rr.center().y - 3.0), vec2(cells[2].width(), 6.0)),
                );
                continue;
            }
            if ri > 0 {
                p.line_segment(
                    [pos2(tr.min.x + 1.0, rr.min.y), pos2(tr.max.x - 1.0, rr.min.y)],
                    Stroke::new(1.0, with_alpha(t.border, 0.6)),
                );
            }
            let pr = self.products[pi].clone();
            product_art(
                &p,
                Rect::from_min_size(pos2(cells[0].min.x, rr.center().y - 18.0), vec2(36.0, 36.0)),
                pr.hue,
                &t,
            );
            kit::text_at(
                &p,
                pos2(cells[0].min.x + 48.0, rr.center().y),
                Align2::LEFT_CENTER,
                &pr.name,
                13.5,
                t.foreground,
                cells[0].width() - 50.0,
            );
            kit::mono_at(
                &p,
                cells[1].left_center(),
                Align2::LEFT_CENTER,
                &format!("NW-{:04}", 1200 + pi * 37),
                12.0,
                t.muted_foreground,
                cells[1].width(),
            );
            let frac = (pr.stock as f32 / (pr.threshold as f32 * 4.0)).clamp(0.0, 1.0);
            let low = pr.stock < pr.threshold;
            let col = if pr.stock == 0 {
                t.destructive
            } else if low {
                t.warning
            } else {
                t.success
            };
            let track = Rect::from_min_size(pos2(cells[2].min.x, rr.center().y - 3.0), vec2(cells[2].width(), 6.0));
            p.rect_filled(track, 3.0, with_alpha(t.foreground, 0.08));
            let fw = kit::follow(ui.ctx(), id.with(("bar", pi)), frac, 8.0);
            p.rect_filled(Rect::from_min_size(track.min, vec2(track.width() * fw, 6.0)), 3.0, col);
            let mark = track.min.x + track.width() * 0.25;
            p.line_segment(
                [pos2(mark, track.min.y - 3.0), pos2(mark, track.max.y + 3.0)],
                Stroke::new(1.0, t.muted_foreground),
            );
            kit::text_at(
                &p,
                cells[3].left_center(),
                Align2::LEFT_CENTER,
                &pr.stock.to_string(),
                13.5,
                if low { col } else { t.foreground },
                cells[3].width(),
            );
            let br = Rect::from_min_size(pos2(cells[4].max.x - 100.0, rr.center().y - 14.0), vec2(100.0, 28.0));
            if kit::put(
                ui,
                br,
                Button::new("Reorder")
                    .variant(if low { ButtonVariant::Default } else { ButtonVariant::Outline })
                    .size(ControlSize::Sm)
                    .width(100.0),
            )
            .clicked()
            {
                self.products[pi].stock += pr.threshold * 2;
                toast(
                    ui.ctx(),
                    Toast::success(format!("Reordered {}", pr.name))
                        .description(format!("+{} units on the way", pr.threshold * 2)),
                );
            }
        }
    }
}

/// Generated product "photo": a soft gradient tile with a glyph.
fn product_art(p: &egui::Painter, r: Rect, hue: usize, t: &Theme) {
    let a = t.chart[hue % 6];
    let b = t.chart[(hue + 2) % 6];
    let bg = if t.dark { mix(t.card, a, 0.22) } else { mix(Color32::WHITE, a, 0.16) };
    p.rect_filled(r, t.radius * 0.7, bg);
    paint::glow(p, r.center() + vec2(r.width() * 0.18, -r.height() * 0.15), r.height() * 0.55, with_alpha(b, 0.35));
    let icons = [
        AppIcon::Line(LineIcon::Zap),
        AppIcon::Line(LineIcon::Star),
        AppIcon::Volume,
        AppIcon::Radio,
        AppIcon::Building,
        AppIcon::Line(LineIcon::User),
        AppIcon::Line(LineIcon::Grid),
        AppIcon::Paperclip,
    ];
    let s = r.height() * 0.38;
    icons[hue % icons.len()].paint(
        p,
        Rect::from_center_size(r.center(), vec2(s, s)),
        if t.dark { mix(a, Color32::WHITE, 0.3) } else { glowkit_core::color::shade(a, -0.15) },
    );
}
