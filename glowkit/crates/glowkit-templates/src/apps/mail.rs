//! Mail client template: folders, message list, reading pane and compose dialog.

use egui::{pos2, vec2, Align2, CursorIcon, Key, Rect, Sense, Stroke, Ui};
use glowkit_components::{toast, Button, ButtonVariant, ControlSize, Dialog, Input, LineIcon, Switch, Tabs, TabsStyle, Textarea, Toast, UiIcon};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// One message in the [`MailApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct MailMessage {
    /// Sender name.
    pub from: String,
    /// Sender address.
    pub email: String,
    /// Subject line.
    pub subject: String,
    /// Body text.
    pub body: String,
    /// Relative date.
    pub date: String,
    /// Unread flag.
    pub unread: bool,
    /// Labels ("work", "important").
    pub labels: Vec<String>,
    /// Folder route ("inbox", "sent", "archive", "trash", ...).
    pub folder: String,
}

/// A three-pane mail client inside an [`AppShell`]: folders with unread counts in the sidebar,
/// a searchable message list with All / Unread tabs, and a reading pane with an action toolbar,
/// the message and an inline reply box. ↑ / ↓ move through messages, E archives, Delete trashes
/// (with Undo), C opens the compose dialog.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::MailApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct MailApp {
    /// Active folder route.
    pub route: String,
    /// Folder navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// All messages.
    pub messages: Vec<MailMessage>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    selected: Option<usize>,
    tab: usize,
    query: String,
    reply: String,
    mute: bool,
    compose_open: bool,
    compose: (String, String, String),
    last_removed: Option<(usize, MailMessage)>,
}

fn msg(from: &str, subject: &str, body: &str, date: &str, unread: bool, labels: &[&str], folder: &str) -> MailMessage {
    MailMessage {
        email: format!("{}@{}.com", from.split(' ').next().unwrap_or("x").to_lowercase(), if labels.contains(&"work") { "acme" } else { "example" }),
        from: from.into(),
        subject: subject.into(),
        body: body.into(),
        date: date.into(),
        unread,
        labels: labels.iter().map(|s| s.to_string()).collect(),
        folder: folder.into(),
    }
}

impl Default for MailApp {
    fn default() -> Self {
        Self {
            route: "inbox".into(),
            nav: vec![
                NavSection::new("")
                    .item(NavItem::new("inbox", "Inbox").icon(LineIcon::Inbox))
                    .item(NavItem::new("drafts", "Drafts").icon(LineIcon::File).badge("2"))
                    .item(NavItem::new("sent", "Sent").icon(AppIcon::Send))
                    .item(NavItem::new("junk", "Junk").icon(AppIcon::Archive))
                    .item(NavItem::new("trash", "Trash").icon(LineIcon::Trash))
                    .item(NavItem::new("archive", "Archive").icon(AppIcon::Archive)),
                NavSection::new("Categories")
                    .item(NavItem::new("social", "Social").icon(LineIcon::Users).badge("972"))
                    .item(NavItem::new("updates", "Updates").icon(LineIcon::Info).badge("342"))
                    .item(NavItem::new("forums", "Forums").icon(UiIcon::Inbox).badge("128"))
                    .item(NavItem::new("shopping", "Shopping").icon(AppIcon::Cart).badge("8"))
                    .item(NavItem::new("promotions", "Promotions").icon(AppIcon::Tag).badge("21")),
            ],
            notifications: demo_notes(),
            messages: vec![
                msg("William Smith", "Meeting Tomorrow", "Hi, let's have a meeting tomorrow to discuss the project. I've been reviewing the project details and have some ideas I'd like to share. It's crucial that we align on our next steps to ensure the project's success.\n\nPlease come prepared with any questions or insights you may have. Looking forward to our meeting!\n\nBest regards, William", "09:34 AM", true, &["meeting", "work", "important"], "inbox"),
                msg("Alice Smith", "Re: Project Update", "Thank you for the project update. It looks great! I've gone through the report, and the progress is impressive. The team has done a fantastic job, and I appreciate the hard work everyone has put in.\n\nI have a few minor suggestions that I'll include in the attached document. Let's discuss them during our next meeting.", "Yesterday", true, &["work", "important"], "inbox"),
                msg("Bob Johnson", "Weekend Plans", "Any plans for the weekend? I was thinking of going hiking in the nearby mountains. It's been a while since we had some outdoor fun.\n\nIf you're interested, let me know, and we can plan the details. It'll be a great way to unwind.", "2 days ago", false, &["personal"], "inbox"),
                msg("Emily Davis", "Re: Question about Budget", "I have a question about the budget for the upcoming project. It seems like there's a discrepancy in the allocation of resources.\n\nI've reviewed the budget report and identified a few areas where we might be able to optimize our spending without compromising the project's quality.", "1 week ago", false, &["work", "budget"], "inbox"),
                msg("Michael Wilson", "Important Announcement", "I have an important announcement to make during our team meeting. It pertains to a strategic shift in our approach to the upcoming product launch.\n\nWe've received valuable feedback from our beta testers, and I believe it's time to make some adjustments.", "1 week ago", false, &["meeting", "work"], "inbox"),
                msg("Sarah Brown", "Re: Feedback on Proposal", "Thank you for your feedback on the proposal. It looks great! I'm pleased to hear that you found it promising. The team worked diligently to address all the key points you raised.", "2 weeks ago", false, &["work"], "inbox"),
                msg("David Lee", "New Project Idea", "I have an exciting new project idea to discuss with you. It involves expanding our services to target a niche market that has shown considerable growth.", "3 weeks ago", false, &["meeting", "work", "important"], "inbox"),
                msg("Olivia Wilson", "Vacation Plans", "Let's plan our vacation for next month. What do you think? I've been thinking of visiting a tropical paradise.", "1 month ago", false, &["personal"], "inbox"),
                msg("Ada Lovelace", "Q4 roadmap draft", "Sharing the Q4 roadmap draft. Comments welcome before Friday.", "Nov 3", false, &["work"], "sent"),
            ],
            scoped_theme: false,
            selected: Some(0),
            tab: 0,
            query: String::new(),
            reply: String::new(),
            mute: false,
            compose_open: false,
            compose: (String::new(), String::new(), String::new()),
            last_removed: None,
        }
    }
}

impl MailApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        // live unread counts on the folders
        for it in nav[0].items.iter_mut() {
            let n = self.messages.iter().filter(|m| m.folder == it.route && m.unread).count();
            if it.route == "inbox" || it.route == "trash" {
                it.badge = (n > 0).then(|| n.to_string());
            }
        }
        let prev_route = route.clone();
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Sidebar)
            .workspaces(vec![Workspace::new("Ada Lovelace", "ada@acme.dev"), Workspace::new("Acme Support", "support@acme.dev")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("mail_app")
            .show(ui, |ui, sc| self.panes(ui, sc.rect, sc.bounds, sc.route.as_str()));
        if route != prev_route {
            self.selected = self.messages.iter().position(|m| m.folder == route);
        }
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn visible(&self, folder: &str) -> Vec<usize> {
        let q = self.query.to_lowercase();
        (0..self.messages.len())
            .filter(|&i| {
                let m = &self.messages[i];
                m.folder == folder && (self.tab == 0 || m.unread) && (q.is_empty() || m.subject.to_lowercase().contains(&q) || m.from.to_lowercase().contains(&q) || m.body.to_lowercase().contains(&q))
            })
            .collect()
    }

    fn panes(&mut self, ui: &mut Ui, rect: Rect, bounds: Rect, folder: &str) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("mail");
        let loading = kit::loading(ui, id.with(folder), 0.5);
        let list_w = (rect.width() * 0.42).clamp(300.0, 440.0);
        let parts = kit::split_x(rect, &[list_w, 0.0], 0.0);
        let (lr, rr) = (parts[0], parts[1]);
        kit::vline(ui.painter(), lr.max.x, rect.min.y, rect.max.y, &t);
        let vis = self.visible(folder);
        // keyboard
        let typing = ui.memory(|m| m.focused().is_some());
        if !typing && !self.compose_open {
            let (down, up, e, del, c) = ui.input(|i| (i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::J), i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::K), i.key_pressed(Key::E), i.key_pressed(Key::Delete), i.key_pressed(Key::C)));
            let pos = self.selected.and_then(|s| vis.iter().position(|v| *v == s));
            if down && !vis.is_empty() {
                self.selected = Some(vis[pos.map_or(0, |p| (p + 1).min(vis.len() - 1))]);
            }
            if up && !vis.is_empty() {
                self.selected = Some(vis[pos.map_or(0, |p| p.saturating_sub(1))]);
            }
            if e {
                self.move_selected(ui, "archive", &vis);
            }
            if del {
                self.move_selected(ui, "trash", &vis);
            }
            if c {
                self.compose_open = true;
            }
        }
        if let Some(s) = self.selected {
            if let Some(m) = self.messages.get_mut(s) {
                m.unread = false;
            }
        }
        // ---- list pane --------------------------------------------------------------------
        let p = ui.painter().clone();
        let title = folder[..1].to_uppercase() + &folder[1..];
        kit::text_at(&p, pos2(lr.min.x + 18.0, lr.min.y + 28.0), Align2::LEFT_CENTER, &title, 18.0, t.foreground, 160.0);
        let mut compose = false;
        kit::child(ui, Rect::from_min_max(pos2(lr.max.x - 210.0, lr.min.y + 12.0), pos2(lr.max.x - 14.0, lr.min.y + 44.0)), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let b = ui.add(Button::new("Compose").icon(UiIcon::Plus).size(ControlSize::Sm));
                kit::tip(&b, "New message", Some("C"));
                compose = b.clicked();
                ui.add(Tabs::new(&mut self.tab, &["All", "Unread"]).style(TabsStyle::Segmented).height(32.0));
            });
        });
        if compose {
            self.compose_open = true;
            self.compose = (String::new(), String::new(), String::new());
        }
        kit::hline(&p, lr.min.x, lr.max.x, lr.min.y + 56.0, &t);
        ui.put(Rect::from_min_size(pos2(lr.min.x + 14.0, lr.min.y + 68.0), vec2(lr.width() - 28.0, 32.0)), Input::new(&mut self.query).placeholder("Search mail…").icon(UiIcon::Search).clearable(true).size(ControlSize::Sm).width(lr.width() - 28.0).id_salt("mail_q"));
        let list = Rect::from_min_max(pos2(lr.min.x, lr.min.y + 112.0), lr.max);
        let mut clicked = None;
        kit::child(ui, list, |ui| {
            kit::scroll(ui, "mail_list", |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.add_space(2.0);
                for &mi in &vis {
                    let r = kit::alloc_row(ui, 112.0);
                    let r = Rect::from_min_max(pos2(r.min.x + 12.0, r.min.y), pos2(r.max.x - 12.0, r.max.y));
                    if loading {
                        kit::skel_r(ui, r, t.radius);
                        continue;
                    }
                    let m = &self.messages[mi];
                    let sel = self.selected == Some(mi);
                    let rid = id.with(("m", mi));
                    let resp = ui.interact(r, rid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    let hv = kit::hover(ui, rid, resp.hovered());
                    let p = ui.painter();
                    p.rect_filled(r, t.radius, if sel { kit::active_fill(&t, t.background) } else { kit::hover_fill(&t, hv) });
                    p.rect_stroke(r, t.radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    let x = r.min.x + 14.0;
                    let mut nx = x;
                    if m.unread {
                        p.circle_filled(pos2(x + 3.0, r.min.y + 20.0), 4.0, t.info);
                        nx += 14.0;
                    }
                    let dr = kit::text_at(p, pos2(r.max.x - 14.0, r.min.y + 20.0), Align2::RIGHT_CENTER, &m.date, 11.5, if sel { t.foreground } else { t.muted_foreground }, 90.0);
                    kit::text_at(p, pos2(nx, r.min.y + 20.0), Align2::LEFT_CENTER, &m.from, 14.0, t.foreground, dr.min.x - nx - 10.0);
                    kit::text_at(p, pos2(x, r.min.y + 40.0), Align2::LEFT_CENTER, &m.subject, 12.5, t.foreground, r.width() - 28.0);
                    let g = kit::wrapped(p, &m.body, 12.0, t.muted_foreground, r.width() - 28.0, 2);
                    p.galley(pos2(x, r.min.y + 52.0), g, t.muted_foreground);
                    let mut lx = x;
                    for l in &m.labels {
                        let important = l == "work" || l == "important";
                        let pr = kit::pill(p, pos2(lx, r.max.y - 15.0), false, l, if important { t.primary_foreground } else { t.secondary_foreground }, if important { t.primary } else { t.secondary }, 10.5);
                        lx = pr.max.x + 6.0;
                    }
                    if resp.clicked() {
                        clicked = Some(mi);
                    }
                }
                if vis.is_empty() {
                    let r = kit::alloc_row(ui, 200.0);
                    ui.painter().text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, if self.query.is_empty() { "No messages" } else { "No messages match your search" }, egui::FontId::proportional(14.0), t.foreground);
                    ui.painter().text(r.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "You're all caught up.", egui::FontId::proportional(12.5), t.muted_foreground);
                }
            });
        });
        if let Some(c) = clicked {
            self.selected = Some(c);
        }
        // ---- reading pane -----------------------------------------------------------------
        self.reading_pane(ui, rr, &vis, loading);
        self.compose_dialog(ui, bounds);
    }

    fn move_selected(&mut self, ui: &Ui, to: &str, vis: &[usize]) {
        if let Some(s) = self.selected {
            let from = self.messages[s].folder.clone();
            self.last_removed = Some((s, self.messages[s].clone()));
            self.messages[s].folder = to.into();
            let subject = self.messages[s].subject.clone();
            let pos = vis.iter().position(|v| *v == s).unwrap_or(0);
            self.selected = vis.iter().filter(|v| **v != s).nth(pos.min(vis.len().saturating_sub(2))).copied();
            toast(ui.ctx(), Toast::new(if to == "trash" { "Moved to Trash" } else { "Archived" }).description(format!("“{subject}” from {from}")).action("Undo"));
        }
    }

    fn reading_pane(&mut self, ui: &mut Ui, rr: Rect, vis: &[usize], loading: bool) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("mail_read");
        let p = ui.painter().clone();
        // toolbar
        let tb = Rect::from_min_size(rr.min, vec2(rr.width(), 56.0));
        let mut x = tb.min.x + 12.0;
        let mut action = None;
        for (k, (icon, label, key)) in [
            (AppIcon::Archive, "Archive", Some("E")),
            (AppIcon::Line(LineIcon::Warning), "Move to junk", None),
            (AppIcon::Line(LineIcon::Trash), "Move to trash", Some("Del")),
            (AppIcon::Ui(UiIcon::Clock), "Snooze", None),
        ]
        .iter()
        .enumerate()
        {
            let br = Rect::from_center_size(pos2(x + 16.0, tb.center().y), vec2(32.0, 32.0));
            let b = kit::icon_button(ui, br, id.with(("tb", k)), *icon, false);
            kit::tip(&b, label, *key);
            if b.clicked() {
                action = Some(k);
            }
            x += 36.0;
            if k == 2 {
                kit::vline(&p, x + 4.0, tb.center().y - 10.0, tb.center().y + 10.0, &t);
                x += 10.0;
            }
        }
        let mut rx = tb.max.x - 12.0;
        for (k, (icon, label)) in [(AppIcon::Ui(UiIcon::MoreHorizontal), "More"), (AppIcon::Reply, "Forward"), (AppIcon::Reply, "Reply all"), (AppIcon::Reply, "Reply")].iter().enumerate() {
            let br = Rect::from_center_size(pos2(rx - 16.0, tb.center().y), vec2(32.0, 32.0));
            let b = kit::icon_button(ui, br, id.with(("tr", k)), *icon, false);
            kit::tip(&b, label, None);
            if b.clicked() {
                toast(ui.ctx(), Toast::new(*label).description("Opened a reply draft below."));
            }
            rx -= 36.0;
        }
        kit::hline(&p, rr.min.x, rr.max.x, tb.max.y, &t);
        let vis_v = vis.to_vec();
        match action {
            Some(0) => self.move_selected(ui, "archive", &vis_v),
            Some(1) => self.move_selected(ui, "junk", &vis_v),
            Some(2) => self.move_selected(ui, "trash", &vis_v),
            Some(3) => {
                toast(ui.ctx(), Toast::info("Snoozed until tomorrow").description("It will come back at 9:00 AM."));
            }
            _ => {}
        }
        let Some(mi) = self.selected.filter(|s| *s < self.messages.len()) else {
            p.text(rr.center(), Align2::CENTER_CENTER, "No message selected", egui::FontId::proportional(14.0), t.muted_foreground);
            return;
        };
        let body = Rect::from_min_max(pos2(rr.min.x, tb.max.y), rr.max);
        if loading {
            let b = body.shrink(20.0);
            kit::skel_r(ui, Rect::from_min_size(b.min, vec2(40.0, 40.0)), 20.0);
            kit::skel(ui, Rect::from_min_size(b.min + vec2(52.0, 4.0), vec2(b.width() * 0.3, 10.0)));
            kit::skel(ui, Rect::from_min_size(b.min + vec2(52.0, 22.0), vec2(b.width() * 0.45, 9.0)));
            for i in 0..5 {
                kit::skel(ui, Rect::from_min_size(b.min + vec2(0.0, 80.0 + i as f32 * 20.0), vec2(b.width() * (0.9 - 0.08 * (i % 3) as f32), 9.0)));
            }
            return;
        }
        let m = self.messages[mi].clone();
        let hr = Rect::from_min_size(body.min + vec2(20.0, 18.0), vec2(body.width() - 40.0, 52.0));
        kit::avatar(ui, Rect::from_min_size(hr.min, vec2(40.0, 40.0)), &m.from);
        let dr = kit::text_at(&p, pos2(hr.max.x, hr.min.y + 9.0), Align2::RIGHT_CENTER, &m.date, 12.0, t.muted_foreground, 100.0);
        kit::text_at(&p, pos2(hr.min.x + 52.0, hr.min.y + 9.0), Align2::LEFT_CENTER, &m.from, 14.0, t.foreground, dr.min.x - hr.min.x - 64.0);
        kit::text_at(&p, pos2(hr.min.x + 52.0, hr.min.y + 27.0), Align2::LEFT_CENTER, &m.subject, 12.5, t.foreground, hr.width() - 60.0);
        kit::text_at(&p, pos2(hr.min.x + 52.0, hr.min.y + 44.0), Align2::LEFT_CENTER, &format!("Reply-To: {}", m.email), 12.0, t.muted_foreground, hr.width() - 60.0);
        kit::hline(&p, rr.min.x, rr.max.x, hr.max.y + 12.0, &t);
        let reply_h = 150.0;
        let text_r = Rect::from_min_max(pos2(body.min.x + 20.0, hr.max.y + 26.0), pos2(body.max.x - 20.0, body.max.y - reply_h - 12.0));
        kit::child(ui, text_r, |ui| {
            kit::scroll(ui, "mail_body", |ui| {
                ui.label(egui::RichText::new(&m.body).size(13.5).color(t.foreground).line_height(Some(21.0)));
            });
        });
        kit::hline(&p, rr.min.x, rr.max.x, body.max.y - reply_h - 4.0, &t);
        let rb = Rect::from_min_max(pos2(body.min.x + 16.0, body.max.y - reply_h + 8.0), body.max - vec2(16.0, 12.0));
        let mut send = false;
        kit::child(ui, rb, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.add(Textarea::new(&mut self.reply).placeholder(format!("Reply {}…", m.from)).rows(3).width(rb.width()).id_salt("mail_reply"));
            ui.horizontal(|ui| {
                ui.add(Switch::new(&mut self.mute).label("Mute this thread").size(ControlSize::Sm));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    send = ui.add(Button::new("Send").size(ControlSize::Sm).disabled(self.reply.trim().is_empty())).clicked();
                });
            });
        });
        if send {
            self.messages.push(MailMessage { from: "Ada Lovelace".into(), email: "ada@acme.dev".into(), subject: format!("Re: {}", m.subject), body: self.reply.clone(), date: "Just now".into(), unread: false, labels: vec!["work".into()], folder: "sent".into() });
            self.reply.clear();
            toast(ui.ctx(), Toast::success("Reply sent").description(format!("To {}", m.from)).action("Undo"));
        }
    }

    fn compose_dialog(&mut self, ui: &mut Ui, bounds: Rect) {
        let mut open = self.compose_open;
        let c = &mut self.compose;
        let r = Dialog::new(&mut open, "New message").primary("Send").secondary("Discard").width(520.0).within(bounds).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let w = ui.available_width();
            ui.add(Input::new(&mut c.0).label("To").placeholder("name@example.com").icon(UiIcon::Mail).width(w).id_salt("mail_to"));
            ui.add(Input::new(&mut c.1).label("Subject").placeholder("What's this about?").width(w).id_salt("mail_subject"));
            ui.add(Textarea::new(&mut c.2).label("Message").placeholder("Write your message…").rows(6).width(w).id_salt("mail_body_c"));
        });
        if r.primary_clicked {
            if self.compose.0.contains('@') {
                let to = self.compose.0.clone();
                self.messages.push(MailMessage {
                    from: "Ada Lovelace".into(),
                    email: "ada@acme.dev".into(),
                    subject: if self.compose.1.is_empty() { "(no subject)".into() } else { self.compose.1.clone() },
                    body: self.compose.2.clone(),
                    date: "Just now".into(),
                    unread: false,
                    labels: vec![],
                    folder: "sent".into(),
                });
                toast(ui.ctx(), Toast::success("Message sent").description(format!("To {to}")).action("Undo"));
                open = false;
            } else {
                toast(ui.ctx(), Toast::error("Add a recipient").description("Enter a valid email address in To."));
                open = true;
            }
        }
        self.compose_open = open;
        let _ = ButtonVariant::Default;
    }
}
