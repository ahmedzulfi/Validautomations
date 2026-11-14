//! Team chat / messaging template.

use egui::{pos2, vec2, Align2, CursorIcon, Key, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_components::{toast, AvatarGroup, LineIcon, Toast};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user, demo_workspaces};
use crate::shells::kit;
use crate::shells::{nav_find, AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification};

/// One message in the [`ChatApp`].
#[derive(Clone, Debug, PartialEq)]
pub struct ChatPost {
    /// Channel / DM route.
    pub channel: String,
    /// Author name.
    pub author: String,
    /// Time label.
    pub time: String,
    /// Text.
    pub text: String,
    /// Reactions: emoji, count, reacted by me.
    pub reactions: Vec<(String, u32, bool)>,
    /// Number of thread replies.
    pub replies: u32,
}

/// A team messaging app inside a dual-sidebar [`AppShell`] (rail: Channels / Direct messages;
/// panel: the list). The channel view has a header with topic and members, a message stream
/// with day dividers, grouped consecutive messages, reaction chips you can toggle and thread
/// counts, and a composer (Enter sends, Shift+Enter for a newline). Teammates "type" a reply.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::ChatApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct ChatApp {
    /// Active channel route.
    pub route: String,
    /// Channels and DMs.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// All posts.
    pub posts: Vec<ChatPost>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    draft: String,
    pending: Option<(f64, ChatPost)>,
}

fn post(ch: &str, who: &str, time: &str, text: &str, reactions: &[(&str, u32, bool)], replies: u32) -> ChatPost {
    ChatPost { channel: ch.into(), author: who.into(), time: time.into(), text: text.into(), reactions: reactions.iter().map(|(e, n, m)| (e.to_string(), *n, *m)).collect(), replies }
}

impl Default for ChatApp {
    fn default() -> Self {
        Self {
            route: "c/design".into(),
            nav: vec![
                NavSection::new("Channels")
                    .icon(AppIcon::Hash)
                    .item(NavItem::new("c/general", "general").icon(AppIcon::Hash))
                    .item(NavItem::new("c/design", "design").icon(AppIcon::Hash))
                    .item(NavItem::new("c/engineering", "engineering").icon(AppIcon::Hash).badge("5"))
                    .item(NavItem::new("c/launch", "launch-week").icon(AppIcon::Hash).badge("2"))
                    .item(NavItem::new("c/random", "random").icon(AppIcon::Hash)),
                NavSection::new("Direct messages")
                    .icon(LineIcon::Users)
                    .item(NavItem::new("dm/mia", "Mia Chen").icon(LineIcon::User))
                    .item(NavItem::new("dm/liam", "Liam Johnson").icon(LineIcon::User).badge("1"))
                    .item(NavItem::new("dm/sofia", "Sofia Davis").icon(LineIcon::User)),
                NavSection::new("Saved").icon(AppIcon::Bookmark).item(NavItem::new("saved", "Saved items").icon(AppIcon::Bookmark)),
            ],
            notifications: demo_notes(),
            posts: vec![
                post("c/design", "Mia Chen", "9:12 AM", "Morning! I pushed the new pricing page explorations to Figma. Three directions: minimal, illustrated and a bold gradient one.", &[("👀", 3, false)], 0),
                post("c/design", "Mia Chen", "9:13 AM", "Leaning towards the gradient one for the hero, but I'd love opinions before the 2pm review.", &[], 4),
                post("c/design", "Liam Johnson", "9:31 AM", "Gradient one is 🔥. The contrast on the plan cards is much better than what we ship today.", &[("👍", 2, true), ("🔥", 1, false)], 0),
                post("c/design", "Sofia Davis", "10:02 AM", "Agree. Can we keep the comparison table from the minimal version though? Customers ask about it a lot in sales calls.", &[("✔", 4, false)], 2),
                post("c/design", "Ada Lovelace", "10:05 AM", "+1 to Sofia. Mia, can you merge those two for the review? I'll bring the conversion numbers.", &[], 0),
                post("c/design", "Mia Chen", "10:07 AM", "On it 🚀", &[("🎉", 5, true)], 0),
                post("c/general", "Noah Brown", "8:30 AM", "Reminder: all-hands at 4pm today. Agenda is in the doc pinned to this channel.", &[("👍", 12, false)], 1),
                post("c/engineering", "Liam Johnson", "11:20 AM", "Deploy of web-app v2.14.0 is out. Rollback plan is in the runbook if anything looks off.", &[("🚀", 6, false)], 3),
            ],
            scoped_theme: false,
            draft: String::new(),
            pending: None,
        }
    }
}

impl ChatApp {
    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        // opening a channel clears its badge
        for s in nav.iter_mut() {
            for it in s.items.iter_mut() {
                if it.route == route {
                    it.badge = None;
                }
            }
        }
        let title = nav_find(&nav, &route).map(|i| i.label.clone()).unwrap_or_default();
        AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Dual)
            .workspaces(demo_workspaces("Acme Team", "Business+"))
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("chat_app")
            .show(ui, |ui, sc| self.channel(ui, sc.rect, sc.route.as_str(), &title));
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
    }

    fn channel(&mut self, ui: &mut Ui, rect: Rect, ch: &str, title: &str) {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("chat");
        let now = ui.input(|i| i.time);
        let loading = kit::loading(ui, id.with(ch), 0.45);
        let p = ui.painter().clone();
        let is_dm = ch.starts_with("dm/");
        // header
        let hr = Rect::from_min_size(rect.min, vec2(rect.width(), 60.0));
        let mut x = hr.min.x + 22.0;
        if is_dm {
            kit::avatar(ui, Rect::from_center_size(pos2(x + 14.0, hr.center().y), vec2(28.0, 28.0)), title);
            x += 36.0;
        } else {
            AppIcon::Hash.paint(&p, Rect::from_center_size(pos2(x + 8.0, hr.center().y - 8.0), vec2(16.0, 16.0)), t.muted_foreground);
            x += 22.0;
        }
        let tr = kit::text_at(&p, pos2(x, hr.center().y - (if is_dm { 0.0 } else { 8.0 })), Align2::LEFT_CENTER, title, 15.0, t.foreground, 260.0);
        if !is_dm {
            kit::text_at(&p, pos2(hr.min.x + 22.0, hr.center().y + 12.0), Align2::LEFT_CENTER, "Design reviews, critiques and inspiration · Review today at 2pm", 12.0, t.muted_foreground, hr.width() - 260.0);
        }
        let _ = tr;
        let mut rx = hr.max.x - 16.0;
        for (k, (icon, label)) in [(AppIcon::Line(LineIcon::Info), "Channel details"), (AppIcon::Bookmark, "Pinned messages"), (AppIcon::Line(LineIcon::Search), "Search in channel")].iter().enumerate() {
            let br = Rect::from_center_size(pos2(rx - 16.0, hr.center().y), vec2(32.0, 32.0));
            let b = kit::icon_button(ui, br, id.with(("h", k)), *icon, false);
            kit::tip(&b, label, None);
            if b.clicked() {
                toast(ui.ctx(), Toast::new(*label));
            }
            rx -= 36.0;
        }
        if !is_dm {
            let gr = Rect::from_min_size(pos2(rx - 104.0, hr.center().y - 14.0), vec2(100.0, 28.0));
            kit::put(ui, gr, AvatarGroup::new(&["Mia Chen", "Liam Johnson", "Sofia Davis", "Noah Brown", "Ada Lovelace"]).max(3).size(26.0).extra(9));
        }
        kit::hline(&p, rect.min.x, rect.max.x, hr.max.y, &t);
        // composer
        let comp_h = 104.0;
        let cr = Rect::from_min_max(pos2(rect.min.x + 20.0, rect.max.y - comp_h), rect.max - vec2(20.0, 18.0));
        // stream
        let stream = Rect::from_min_max(pos2(rect.min.x, hr.max.y), pos2(rect.max.x, cr.min.y - 8.0));
        let idxs: Vec<usize> = (0..self.posts.len()).filter(|&i| self.posts[i].channel == ch).collect();
        let typing = self.pending.as_ref().filter(|(_, p)| p.channel == ch).map(|(_, p)| p.author.clone());
        let mut react: Option<(usize, usize)> = None;
        kit::child(ui, stream, |ui| {
            egui::ScrollArea::vertical().id_salt(("chat_stream", ch)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                ui.add_space(12.0);
                // day divider
                let dr = kit::alloc_row(ui, 30.0);
                let pp = ui.painter().clone();
                pp.line_segment([pos2(dr.min.x + 20.0, dr.center().y), pos2(dr.max.x - 20.0, dr.center().y)], Stroke::new(1.0, t.border));
                let g = kit::trunc(&pp, "Today", 11.5, t.muted_foreground, 100.0);
                let pr = Rect::from_center_size(dr.center(), g.size() + vec2(20.0, 8.0));
                pp.rect_filled(pr, pr.height() / 2.0, t.background);
                pp.rect_stroke(pr, pr.height() / 2.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
                pp.galley(pr.center() - g.size() / 2.0, g, t.muted_foreground);
                if loading {
                    for k in 0..4 {
                        let r = kit::alloc_row(ui, 76.0);
                        kit::skel_r(ui, Rect::from_min_size(r.min + vec2(20.0, 10.0), vec2(36.0, 36.0)), 8.0);
                        kit::skel(ui, Rect::from_min_size(r.min + vec2(68.0, 12.0), vec2(120.0, 9.0)));
                        kit::skel(ui, Rect::from_min_size(r.min + vec2(68.0, 30.0), vec2((w - 120.0) * (0.8 - 0.15 * (k % 2) as f32), 9.0)));
                    }
                    return;
                }
                if idxs.is_empty() {
                    let r = kit::alloc_row(ui, 160.0);
                    ui.painter().text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, format!("This is the start of {title}"), egui::FontId::proportional(15.0), t.foreground);
                    ui.painter().text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "Say hi 👋", egui::FontId::proportional(12.5), t.muted_foreground);
                }
                let mut prev_author: Option<&str> = None;
                for &pi in &idxs {
                    let m = &self.posts[pi];
                    let first = prev_author != Some(m.author.as_str());
                    prev_author = Some(m.author.as_str());
                    let text_w = w - 68.0 - 24.0;
                    let g = ui.painter().layout(m.text.clone(), egui::FontId::proportional(13.5), t.foreground, text_w);
                    let extra = if m.reactions.is_empty() && m.replies == 0 { 0.0 } else { 30.0 };
                    let h = g.size().y + if first { 30.0 } else { 8.0 } + extra + 6.0;
                    let r = kit::alloc_row(ui, h);
                    let rid = id.with(("post", pi));
                    let resp = ui.interact(r, rid, Sense::hover());
                    let hv = kit::hover(ui, rid, resp.hovered());
                    let pp = ui.painter().clone();
                    if hv > 0.01 {
                        pp.rect_filled(r, 0.0, kit::hover_fill(&t, hv * 0.8));
                    }
                    let mut y = r.min.y + 4.0;
                    if first {
                        kit::avatar(ui, Rect::from_min_size(pos2(r.min.x + 20.0, r.min.y + 6.0), vec2(36.0, 36.0)), &m.author);
                        let nr = kit::text_at(&pp, pos2(r.min.x + 68.0, r.min.y + 14.0), Align2::LEFT_CENTER, &m.author, 13.5, t.foreground, 200.0);
                        kit::text_at(&pp, pos2(nr.max.x + 8.0, r.min.y + 14.0), Align2::LEFT_CENTER, &m.time, 11.5, t.muted_foreground, 80.0);
                        y = r.min.y + 26.0;
                    } else if hv > 0.3 {
                        kit::text_at(&pp, pos2(r.min.x + 56.0, y + 9.0), Align2::RIGHT_CENTER, &m.time.replace(" AM", "").replace(" PM", ""), 10.5, t.muted_foreground, 40.0);
                    }
                    pp.galley(pos2(r.min.x + 68.0, y), g.clone(), t.foreground);
                    y += g.size().y + 6.0;
                    let mut rx0 = r.min.x + 68.0;
                    for (ri, (e, n, mine)) in m.reactions.iter().enumerate() {
                        let label = format!("{e} {n}");
                        let gw = kit::text_w(&pp, &label, 12.0) + 16.0;
                        let cr = Rect::from_min_size(pos2(rx0, y), vec2(gw, 24.0));
                        let cid = rid.with(("re", ri));
                        let cresp = ui.interact(cr, cid, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        let chv = kit::hover(ui, cid, cresp.hovered());
                        pp.rect_filled(cr, 12.0, if *mine { with_alpha(t.accent, 0.16) } else { mix(t.muted, t.foreground, 0.04 * chv) });
                        pp.rect_stroke(cr, 12.0, Stroke::new(1.0, if *mine { with_alpha(t.accent, 0.6) } else { t.border }), StrokeKind::Inside);
                        pp.text(cr.center(), Align2::CENTER_CENTER, &label, egui::FontId::proportional(12.0), if *mine { t.accent } else { t.foreground });
                        if cresp.clicked() {
                            react = Some((pi, ri));
                        }
                        rx0 = cr.max.x + 6.0;
                    }
                    if m.replies > 0 {
                        let label = format!("{} replies", m.replies);
                        let lr = kit::text_at(&pp, pos2(rx0 + if m.reactions.is_empty() { 0.0 } else { 6.0 }, y + 12.0), Align2::LEFT_CENTER, &label, 12.5, t.accent, 120.0);
                        let lresp = ui.interact(lr.expand(4.0), rid.with("thread"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        if lresp.clicked() {
                            toast(ui.ctx(), Toast::new("Thread").description(format!("{} replies to {}", m.replies, m.author)));
                        }
                    }
                }
                if let Some(who) = &typing {
                    let r = kit::alloc_row(ui, 44.0);
                    let tr = Rect::from_min_size(pos2(r.min.x + 68.0, r.min.y + 6.0), vec2(80.0, 32.0));
                    kit::child(ui, tr, |ui| {
                        ui.add(glowkit_ai::TypingIndicator::new().bubble(true).size(6.0));
                    });
                    kit::text_at(ui.painter(), pos2(tr.min.x + 72.0, r.center().y), Align2::LEFT_CENTER, &format!("{who} is typing…"), 12.0, t.muted_foreground, 200.0);
                }
                ui.add_space(10.0);
            });
        });
        if let Some((pi, ri)) = react {
            let r = &mut self.posts[pi].reactions[ri];
            r.2 = !r.2;
            if r.2 {
                r.1 += 1;
            } else {
                r.1 = r.1.saturating_sub(1);
            }
        }
        // composer
        let p = ui.painter().clone();
        let focused = ui.memory(|m| m.has_focus(id.with("composer")));
        let fe = kit::follow(ui.ctx(), id.with("cfocus"), if focused { 1.0 } else { 0.0 }, 14.0);
        p.rect_filled(cr, t.radius, if t.dark { mix(t.background, t.foreground, 0.03) } else { t.background });
        p.rect_stroke(cr, t.radius, Stroke::new(1.0, t.border.lerp_to_gamma(t.ring, fe * 0.8)), StrokeKind::Inside);
        let er = Rect::from_min_max(cr.min + vec2(14.0, 10.0), pos2(cr.max.x - 14.0, cr.max.y - 40.0));
        let mut send = false;
        kit::child(ui, er, |ui| {
            let te = egui::TextEdit::multiline(&mut self.draft)
                .id(id.with("composer"))
                .hint_text(format!("Message {}{title}", if is_dm { "" } else { "#" }))
                .frame(egui::Frame::NONE)
                .font(egui::FontId::proportional(13.5))
                .desired_rows(2)
                .desired_width(er.width())
                .return_key(Some(egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, Key::Enter)));
            let resp = ui.add(te);
            if resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift) {
                send = true;
            }
        });
        // toolbar
        let ty = cr.max.y - 20.0;
        let mut x = cr.min.x + 10.0;
        for (k, (icon, label)) in [(AppIcon::Line(LineIcon::Plus), "Attach"), (AppIcon::Smile, "Emoji"), (AppIcon::AtSign, "Mention someone"), (AppIcon::Paperclip, "Upload file")].iter().enumerate() {
            let br = Rect::from_center_size(pos2(x + 14.0, ty), vec2(28.0, 28.0));
            let b = kit::icon_button(ui, br, id.with(("ct", k)), *icon, false);
            kit::tip(&b, label, None);
            if b.clicked() {
                match k {
                    1 => self.draft.push_str(" 🎉"),
                    2 => self.draft.push_str(" @Mia "),
                    _ => {
                        toast(ui.ctx(), Toast::new("Attachments").description("Drop files here to share them."));
                    }
                }
            }
            x += 32.0;
        }
        kit::text_at(&p, pos2(cr.max.x - 52.0, ty), Align2::RIGHT_CENTER, "Enter to send · Shift+Enter for newline", 11.5, t.muted_foreground, 260.0);
        let sr = Rect::from_center_size(pos2(cr.max.x - 26.0, ty), vec2(32.0, 28.0));
        let can = !self.draft.trim().is_empty();
        let sresp = ui.interact(sr, id.with("send"), if can { Sense::click() } else { Sense::hover() });
        let shv = kit::hover(ui, id.with("send"), sresp.hovered() && can);
        p.rect_filled(sr, t.radius * 0.6, if can { t.primary.gamma_multiply(0.9 + 0.1 * shv) } else { t.muted });
        AppIcon::Send.paint(&p, Rect::from_center_size(sr.center(), vec2(15.0, 15.0)), if can { t.primary_foreground } else { t.muted_foreground });
        if sresp.clicked() {
            send = true;
        }
        if send && can {
            let text = self.draft.trim().to_string();
            self.draft.clear();
            self.posts.push(ChatPost { channel: ch.into(), author: "Ada Lovelace".into(), time: "Just now".into(), text, reactions: vec![], replies: 0 });
            let who = if is_dm { title.to_string() } else { "Mia Chen".to_string() };
            self.pending = Some((now + 1.8, ChatPost { channel: ch.into(), author: who, time: "Just now".into(), text: "Sounds good! I'll take a look and get back to you before lunch.".into(), reactions: vec![], replies: 0 }));
        }
        if let Some((at, _)) = &self.pending {
            if now >= *at {
                if let Some((_, post)) = self.pending.take() {
                    self.posts.push(post);
                }
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
}
