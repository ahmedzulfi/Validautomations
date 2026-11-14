//! Docs / knowledge base template: left navigation, article, right table of contents.

use egui::{pos2, vec2, Align2, CursorIcon, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, Alert, AlertVariant, CodeBlock, LineIcon, TableOfContents, Toast, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// A docs site / knowledge base inside a top-navigation [`AppShell`]: a grouped page tree on
/// the left, an article with breadcrumbs, callouts and code blocks, a right "On this page"
/// table of contents that follows the scroll position (and scrolls on click), previous / next
/// cards and a "Was this helpful?" prompt.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::DocsApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct DocsApp {
    /// Active top-level route.
    pub route: String,
    /// Top navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// Active page in the left tree.
    pub page: usize,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    active_heading: usize,
    scroll_to: Option<usize>,
    voted: Option<bool>,
}

const TREE: [(&str, &[&str]); 3] = [
    ("Getting started", &["Introduction", "Installation", "Your first app", "Theming"]),
    ("Building apps", &["App shells", "Navigation", "Data tables", "Charts", "Overlays"]),
    ("Reference", &["Theme tokens", "Animation", "Painting helpers"]),
];

const HEADINGS: [(u8, &str); 6] = [
    (1, "Overview"),
    (1, "Create the navigation"),
    (2, "Nested items"),
    (1, "Pick a layout"),
    (2, "Responsive behavior"),
    (1, "Next steps"),
];

impl Default for DocsApp {
    fn default() -> Self {
        Self {
            route: "docs".into(),
            nav: vec![NavSection::new("Main")
                .item(NavItem::new("docs", "Docs").icon(LineIcon::Book))
                .item(NavItem::new("guides", "Guides").icon(AppIcon::Rocket))
                .item(NavItem::new("api", "API reference").icon(UiIcon::Code))
                .item(NavItem::new("changelog", "Changelog").icon(LineIcon::File))
                .item(NavItem::new("community", "Community").icon(LineIcon::Users))],
            notifications: demo_notes(),
            page: 4,
            scoped_theme: false,
            active_heading: 0,
            scroll_to: None,
            voted: None,
        }
    }
}

impl DocsApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::TopNav)
            .workspaces(demo_workspaces("glowkit docs", "v0.1"))
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("docs_app")
            .show(ui, |ui, sc| self.layout(ui, sc.rect));
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn page_title(&self) -> (&'static str, &'static str) {
        let mut k = 0;
        for (sec, pages) in TREE.iter() {
            for p in pages.iter() {
                if k == self.page {
                    return (sec, p);
                }
                k += 1;
            }
        }
        ("", "")
    }

    fn layout(&mut self, ui: &mut Ui, rect: Rect) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("docs");
        let toc_w = if rect.width() > 1000.0 { 230.0 } else { 0.0 };
        let parts = kit::split_x(rect, &[250.0, 0.0, toc_w], 0.0);
        let p = ui.painter().clone();
        kit::vline(&p, parts[0].max.x, rect.min.y, rect.max.y, &t);
        // left tree
        let mut pick = None;
        kit::child(ui, parts[0].shrink2(vec2(14.0, 16.0)), |ui| {
            kit::scroll(ui, "docs_tree", |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let mut k = 0;
                for (sec, pages) in TREE.iter() {
                    let r = kit::alloc_row(ui, 30.0);
                    kit::text_at(
                        ui.painter(),
                        pos2(r.min.x + 8.0, r.center().y + 2.0),
                        Align2::LEFT_CENTER,
                        sec,
                        13.0,
                        t.foreground,
                        r.width(),
                    );
                    for pg in pages.iter() {
                        let r = kit::alloc_row(ui, 30.0);
                        let rid = id.with(("p", k));
                        let resp = ui.interact(r, rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        let hv = kit::hover(ui, rid, resp.hovered());
                        let on = k == self.page;
                        let pp = ui.painter();
                        pp.line_segment(
                            [pos2(r.min.x + 8.0, r.min.y), pos2(r.min.x + 8.0, r.max.y)],
                            Stroke::new(if on { 2.0 } else { 1.0 }, if on { t.accent } else { t.border }),
                        );
                        if hv > 0.01 && !on {
                            pp.rect_filled(
                                r.shrink2(vec2(0.0, 1.0)).translate(vec2(10.0, 0.0)),
                                t.radius * 0.5,
                                kit::hover_fill(&t, hv),
                            );
                        }
                        kit::text_at(
                            pp,
                            pos2(r.min.x + 22.0, r.center().y),
                            Align2::LEFT_CENTER,
                            pg,
                            13.0,
                            if on { t.accent } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) },
                            r.width() - 26.0,
                        );
                        if resp.clicked() {
                            pick = Some(k);
                        }
                        k += 1;
                    }
                    ui.add_space(10.0);
                }
            });
        });
        if let Some(k) = pick {
            if k != self.page {
                self.page = k;
                self.active_heading = 0;
                self.scroll_to = Some(0);
                kit::reset_loading(ui, id.with("load"));
            }
        }
        let loading = kit::loading(ui, id.with("load"), 0.4);
        // article
        let art = parts[1];
        let col_w = (art.width() - 64.0).min(720.0);
        let (sec, title) = self.page_title();
        let mut heading_y: Vec<f32> = Vec::new();
        let mut viewport_top = 0.0;
        let scroll_to = self.scroll_to.take();
        let mut next_prev = None;
        kit::child(ui, art, |ui| {
            egui::ScrollArea::vertical().id_salt(("docs_article", self.page)).auto_shrink([false, false]).show(ui, |ui| {
                viewport_top = ui.clip_rect().min.y;
                ui.add_space(28.0);
                let x0 = ui.max_rect().min.x + (ui.available_width() - col_w) / 2.0;
                let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos2(x0, ui.cursor().min.y), vec2(col_w, 10_000.0))));
                let ui2 = &mut cu;
                ui2.spacing_mut().item_spacing.y = 14.0;
                let r = kit::alloc_row(ui2, 18.0);
                kit::text_at(ui2.painter(), r.left_center(), Align2::LEFT_CENTER, &format!("Docs  ›  {sec}  ›  {title}"), 12.5, t.muted_foreground, col_w);
                let r = kit::alloc_row(ui2, 40.0);
                kit::text_at(ui2.painter(), r.left_center(), Align2::LEFT_CENTER, title, 32.0, t.foreground, col_w);
                let r = kit::alloc_row(ui2, 20.0);
                kit::text_at(ui2.painter(), r.left_center(), Align2::LEFT_CENTER, "Compose a sidebar, a topbar and your pages into a complete desktop app.  ·  6 min read  ·  Updated Nov 10", 13.5, t.muted_foreground, col_w);
                if loading {
                    for i in 0..10 {
                        let r = kit::alloc_row(ui2, 12.0);
                        kit::skel(ui2, Rect::from_min_size(r.min, vec2(col_w * (0.95 - 0.1 * (i % 4) as f32), 10.0)));
                    }
                    return;
                }
                let para = |ui: &mut Ui, s: &str| {
                    ui.label(egui::RichText::new(s).size(14.5).color(t.foreground.gamma_multiply(0.92)).line_height(Some(24.0)));
                };
                let heading = |ui: &mut Ui, level: u8, s: &str, ys: &mut Vec<f32>| {
                    ui.add_space(if level == 1 { 10.0 } else { 2.0 });
                    let r = kit::alloc_row(ui, if level == 1 { 30.0 } else { 24.0 });
                    ys.push(r.min.y);
                    kit::text_at(ui.painter(), r.left_center(), Align2::LEFT_CENTER, s, if level == 1 { 22.0 } else { 17.0 }, t.foreground, col_w);
                    if level == 1 {
                        kit::hline(ui.painter(), r.min.x, r.min.x + col_w, r.max.y + 6.0, &t);
                        ui.add_space(4.0);
                    }
                    if scroll_to == Some(ys.len() - 1) {
                        ui.scroll_to_rect(r, Some(egui::Align::TOP));
                    }
                };
                heading(ui2, 1, HEADINGS[0].1, &mut heading_y);
                para(ui2, "An app shell is the frame your product lives in: navigation on the side, a topbar with search, notifications and the account menu, and a content area for your pages. glowkit's AppShell draws all of it from a navigation tree you own, so routing stays plain Rust data.");
                ui2.add(Alert::new("Tip").description("Keep the shell in your app state and call show once per frame. Everything else is regular egui.").variant(AlertVariant::Info).width(col_w));
                heading(ui2, 1, HEADINGS[1].1, &mut heading_y);
                para(ui2, "Describe your pages as sections of NavItems. Each item has a route string; the active route is a &mut String, so navigating is just an assignment.");
                ui2.add(CodeBlock::new("let mut nav = vec![\n    NavSection::new(\"Platform\")\n        .item(NavItem::new(\"dashboard\", \"Dashboard\").icon(LineIcon::Home))\n        .item(NavItem::new(\"inbox\", \"Inbox\").icon(LineIcon::Inbox).badge(\"12\")),\n];").title("main.rs").language("rust"));
                heading(ui2, 2, HEADINGS[2].1, &mut heading_y);
                para(ui2, "Items can nest. Parents expand and collapse with a rotating chevron and a guide line; in the icon rail, clicking a parent jumps to its first child.");
                heading(ui2, 1, HEADINGS[3].1, &mut heading_y);
                para(ui2, "Seven layouts share the same data: Sidebar, Inset, Floating, Dual, TopNav, Inspector and BottomNav. Switching is one builder call, so you can try them all in minutes.");
                ui2.add(Alert::new("Heads up").description("Overlays (dialogs, sheets, the command palette) are confined to the shell, so the app also works embedded in a bigger window.").variant(AlertVariant::Warning).width(col_w));
                heading(ui2, 2, HEADINGS[4].1, &mut heading_y);
                para(ui2, "Below about 1040 points the sidebar collapses to an icon rail automatically; below 720 it becomes a drawer opened from the topbar. Ctrl+B toggles it at any size.");
                heading(ui2, 1, HEADINGS[5].1, &mut heading_y);
                para(ui2, "Browse the app templates for complete screens built on the shell: analytics, CRM, mail, chat, settings and more.");
                // helpful + prev/next
                let r = kit::alloc_row(ui2, 44.0);
                let pp = ui2.painter().clone();
                pp.rect_stroke(r, t.radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
                kit::text_at(&pp, pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, match self.voted { Some(true) => "Thanks for the feedback!", Some(false) => "Thanks, we'll improve this page.", None => "Was this page helpful?" }, 13.5, t.foreground, 300.0);
                if self.voted.is_none() {
                    for (k, label) in ["Yes", "No"].iter().enumerate() {
                        let br = Rect::from_min_size(pos2(r.max.x - 140.0 + k as f32 * 64.0, r.center().y - 14.0), vec2(58.0, 28.0));
                        if kit::put(ui2, br, glowkit_components::Button::new(*label).variant(glowkit_components::ButtonVariant::Outline).size(glowkit_components::ControlSize::Sm).width(58.0)).clicked() {
                            self.voted = Some(k == 0);
                            toast(ui2.ctx(), Toast::success("Feedback sent"));
                        }
                    }
                }
                let r = kit::alloc_row(ui2, 70.0);
                let total: usize = TREE.iter().map(|x| x.1.len()).sum();
                for (k, cr) in kit::cols(r, 2, 14.0).iter().enumerate() {
                    let target = if k == 0 { self.page.checked_sub(1) } else { (self.page + 1 < total).then_some(self.page + 1) };
                    let Some(tg) = target else { continue };
                    let cid = id.with(("np", k));
                    let resp = ui2.interact(*cr, cid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    let hv = kit::hover(ui2, cid, resp.hovered());
                    let pp = ui2.painter();
                    pp.rect_stroke(*cr, t.radius, Stroke::new(1.0, t.border.lerp_to_gamma(t.accent, hv * 0.7)), StrokeKind::Inside);
                    let mut idx = 0;
                    let mut name = "";
                    for (_, pages) in TREE.iter() {
                        for pg in pages.iter() {
                            if idx == tg {
                                name = pg;
                            }
                            idx += 1;
                        }
                    }
                    let (al, x) = if k == 0 { (Align2::LEFT_CENTER, cr.min.x + 16.0) } else { (Align2::RIGHT_CENTER, cr.max.x - 16.0) };
                    kit::text_at(pp, pos2(x, cr.min.y + 22.0), al, if k == 0 { "Previous" } else { "Next" }, 12.0, t.muted_foreground, cr.width() - 32.0);
                    kit::text_at(pp, pos2(x, cr.min.y + 44.0), al, &format!("{}{}{}", if k == 0 { "‹ " } else { "" }, name, if k == 1 { " ›" } else { "" }), 15.0, t.accent, cr.width() - 32.0);
                    if resp.clicked() {
                        next_prev = Some(tg);
                    }
                }
                ui2.add_space(30.0);
                let used = cu.min_rect().height();
                ui.allocate_space(vec2(1.0, used));
            });
        });
        if let Some(tg) = next_prev {
            self.page = tg;
            self.scroll_to = Some(0);
            kit::reset_loading(ui, id.with("load"));
        }
        // active heading = last heading above the viewport top + margin
        if !heading_y.is_empty() {
            let mut a = 0;
            for (i, y) in heading_y.iter().enumerate() {
                if *y <= viewport_top + 80.0 {
                    a = i;
                }
            }
            self.active_heading = a;
        }
        // toc
        if toc_w > 0.0 {
            let tr = parts[2].shrink2(vec2(18.0, 28.0));
            let clicked = kit::child(ui, tr, |ui| {
                TableOfContents::new(&HEADINGS, self.active_heading).title(Some("On this page")).show(ui)
            });
            if let Some(i) = clicked {
                self.active_heading = i;
                self.scroll_to = Some(i);
            }
            let er = Rect::from_min_size(pos2(tr.min.x, tr.min.y + 230.0), vec2(tr.width(), 30.0));
            let resp = ui.interact(er, id.with("edit"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover(ui, id.with("edit"), resp.hovered());
            AppIcon::External.paint(
                &p,
                Rect::from_center_size(pos2(er.min.x + 7.0, er.center().y), vec2(13.0, 13.0)),
                t.muted_foreground,
            );
            kit::text_at(
                &p,
                pos2(er.min.x + 22.0, er.center().y),
                Align2::LEFT_CENTER,
                "Edit this page",
                12.5,
                t.muted_foreground.lerp_to_gamma(t.foreground, hv),
                160.0,
            );
            if resp.clicked() {
                toast(ui.ctx(), Toast::new("Opening editor").description("docs/app-shells.md"));
            }
            let _ = with_alpha;
        }
    }
}
