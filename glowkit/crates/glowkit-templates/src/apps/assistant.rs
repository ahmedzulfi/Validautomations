//! AI assistant app template: chat history sidebar, conversation and composer.

use egui::{pos2, vec2, Align2, Rect, Ui};
use glowkit_ai::{Attachment, AttachmentKind, ChatMessage, Conversation, Icon, Loader, LoaderKind, MessageAction, ModelInfo, ModelSelector, PromptInput, Reasoning, Role, Suggestions};
use glowkit_components::{toast, Button, ButtonVariant, ControlSize, LineIcon, Toast, UiIcon};
use glowkit_core::Theme;

use super::common::{demo_notes, demo_user};
use crate::shells::kit;
use crate::shells::{AppIcon, AppShell, NavItem, NavSection, ShellLayout, ShellNotification, Workspace};

/// One conversation in the [`AssistantApp`] history.
#[derive(Clone, Debug, PartialEq)]
pub struct AssistantChat {
    /// Route id ("chat/3").
    pub id: String,
    /// Title shown in the sidebar.
    pub title: String,
    /// Messages: (from the user?, Markdown text).
    pub messages: Vec<(bool, String)>,
}

#[derive(Clone, Debug)]
struct Generation {
    chat: String,
    started: f64,
    answer: String,
    reasoning: String,
}

/// An AI assistant app inside an [`AppShell`]: the sidebar is the chat history (grouped Today /
/// Previous 7 days), the content is a [`Conversation`] of [`ChatMessage`]s with a streamed,
/// reasoning-then-answer reply and a [`PromptInput`] with attachments and a model selector.
/// "New chat" (Ctrl+Shift+O) starts a conversation with suggestion chips.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut app = glowkit_templates::AssistantApp::default();
/// app.show(ui);
/// ```
#[derive(Clone, Debug)]
pub struct AssistantApp {
    /// Active chat route.
    pub route: String,
    /// Chat history navigation.
    pub nav: Vec<NavSection>,
    /// Bell entries.
    pub notifications: Vec<ShellNotification>,
    /// All chats.
    pub chats: Vec<AssistantChat>,
    /// Apply theme changes only inside the app (for embedding).
    pub scoped_theme: bool,
    input: String,
    model: usize,
    attachments: Vec<Attachment>,
    generation: Option<Generation>,
    next_id: usize,
}

const MODELS: [ModelInfo<'static>; 3] = [
    ModelInfo::new("Nova 2", "Lumen Labs").description("Best balance of speed and smarts").tag("New"),
    ModelInfo::new("Nova Flash", "Lumen Labs").description("Fastest for everyday tasks").tag("Fast"),
    ModelInfo::new("Orbit 5", "Orbit AI").description("Flagship reasoning model"),
];

fn chat(id: usize, title: &str, msgs: &[(bool, &str)]) -> AssistantChat {
    AssistantChat { id: format!("chat/{id}"), title: title.into(), messages: msgs.iter().map(|(u, s)| (*u, s.to_string())).collect() }
}

impl Default for AssistantApp {
    fn default() -> Self {
        let chats = vec![
            chat(
                1,
                "Rust GUI for a dashboard",
                &[
                    (true, "I want to build a desktop dashboard in Rust. Where do I start?"),
                    (false, "Great choice! A solid starting point is **egui** with a dashboard shell:\n\n1. Create the app with `eframe`\n2. Add a sidebar + topbar shell for navigation\n3. Drop KPI tiles and charts into the content area\n\n```rust\nfn ui(&mut self, ui: &mut egui::Ui) {\n    self.app.show(ui);\n}\n```\n\nWant me to sketch the navigation tree for your pages?"),
                ],
            ),
            chat(2, "Quarterly report summary", &[(true, "Summarize the Q3 report in 3 bullets"), (false, "- **Revenue** grew 18% QoQ, led by enterprise plans\n- **Churn** fell to 2.1%, the lowest in two years\n- **Hiring** is on track: 14 of 16 roles filled")]),
            chat(3, "Regex for email validation", &[(true, "Regex for validating emails?"), (false, "A pragmatic pattern:\n\n```\n^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$\n```\n\nIt catches typos without rejecting valid but unusual addresses.")]),
            chat(4, "Trip ideas for Lisbon", &[(true, "3 days in Lisbon, what should I see?"), (false, "Day 1: **Alfama** and the castle. Day 2: **Belém** (and the pastries). Day 3: a day trip to **Sintra**.")]),
            chat(5, "SQL window functions", &[(true, "Explain window functions"), (false, "They compute values **across rows related to the current row** without collapsing them, e.g. `ROW_NUMBER() OVER (PARTITION BY team ORDER BY score DESC)`.")]),
        ];
        let mut app = Self {
            route: "chat/1".into(),
            nav: Vec::new(),
            notifications: demo_notes(),
            chats,
            scoped_theme: false,
            input: String::new(),
            model: 0,
            attachments: vec![Attachment::new("dashboard.rs", AttachmentKind::Code).detail("3.4 KB")],
            generation: None,
            next_id: 6,
        };
        app.rebuild_nav();
        app
    }
}

fn answer_for(prompt: &str) -> (String, String) {
    let p = prompt.to_lowercase();
    if p.contains("table") || p.contains("compare") {
        (
            "Here's a quick comparison:\n\n| Layout | Best for | Collapses to |\n|:--|:--|:--|\n| **Sidebar** | Admin tools | Icon rail |\n| **Inset** | Desktop-native feel | Icon rail |\n| **Top nav** | Few top-level pages | Menu |\n\n> Start with the sidebar; switch layouts with one builder call.".into(),
            "Compare the shell layouts by use case and responsive behavior, then recommend one.".into(),
        )
    } else if p.contains("code") || p.contains("rust") || p.contains("function") {
        (
            "Sure! Here's a tiny helper that formats money with thousands separators:\n\n```rust\nfn money(v: f64) -> String {\n    let s = format!(\"{:.2}\", v);\n    let (int, frac) = s.split_once('.').unwrap();\n    let mut out = String::new();\n    for (i, c) in int.chars().enumerate() {\n        if i > 0 && (int.len() - i) % 3 == 0 { out.push(','); }\n        out.push(c);\n    }\n    format!(\"${out}.{frac}\")\n}\n```\n\nIt avoids allocating per digit and handles any magnitude.".into(),
            "They want code. Keep it short, idiomatic and explain the trick.".into(),
        )
    } else {
        (
            format!("Good question! Here's a short plan for *\u{201c}{}\u{201d}*:\n\n- **Clarify the goal** and who it's for\n- **Sketch** the smallest version that works end to end\n- **Iterate** with feedback, measuring as you go\n\nTell me more and I'll go deeper, with code, tables or a step-by-step plan.", prompt.chars().take(60).collect::<String>()),
            "Give a structured overview and offer to go deeper.".into(),
        )
    }
}

const THINK: f32 = 0.6;
const REASON_CPS: f32 = 80.0;
const ANSWER_CPS: f32 = 95.0;

fn prefix_words(s: &str, chars: usize) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i >= chars && c.is_whitespace() {
            break;
        }
        out.push(c);
    }
    out
}

impl AssistantApp {
    fn rebuild_nav(&mut self) {
        let mut today = NavSection::new("Today");
        let mut week = NavSection::new("Previous 7 days");
        for (i, c) in self.chats.iter().enumerate() {
            let it = NavItem::new(c.id.clone(), c.title.clone()).icon(AppIcon::Line(LineIcon::Mail));
            let it = NavItem { icon: Some(AppIcon::Ui(UiIcon::Sparkles)), ..it };
            if i < 3 {
                today = today.item(it);
            } else {
                week = week.item(it);
            }
        }
        self.nav = vec![today, week];
    }

    fn new_chat(&mut self) {
        let id = format!("chat/{}", self.next_id);
        self.next_id += 1;
        self.chats.insert(0, AssistantChat { id: id.clone(), title: "New chat".into(), messages: Vec::new() });
        self.rebuild_nav();
        self.route = id;
    }

    /// Draw the app into all available space.
    pub fn show(&mut self, ui: &mut Ui) {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::O)) {
            self.new_chat();
        }
        let mut route = std::mem::take(&mut self.route);
        let mut nav = std::mem::take(&mut self.nav);
        let mut notes = std::mem::take(&mut self.notifications);
        let mut new_chat = false;
        let r = AppShell::new(&mut route, &mut nav)
            .layout(ShellLayout::Inset)
            .workspaces(vec![Workspace::new("Lumen Assistant", "Pro plan")])
            .user(demo_user())
            .notifications(&mut notes)
            .padding(0.0)
            .scoped_theme(self.scoped_theme)
            .id_salt("assistant_app")
            .actions(|ui| {
                let b = ui.add(Button::new("New chat").icon(UiIcon::Plus).variant(ButtonVariant::Outline).size(ControlSize::Sm));
                kit::tip(&b, "Start a new conversation", Some("Ctrl+Shift+O"));
                new_chat = b.clicked();
            })
            .show(ui, |ui, sc| self.chat_view(ui, sc.rect, sc.route.as_str()));
        self.route = route;
        self.nav = nav;
        self.notifications = notes;
        if new_chat {
            self.new_chat();
        }
        let _ = r;
    }

    fn chat_view(&mut self, ui: &mut Ui, rect: Rect, route: &str) {
        let t = Theme::of(ui);
        let now = ui.input(|i| i.time);
        let Some(ci) = self.chats.iter().position(|c| c.id == route) else {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, "Pick a conversation", egui::FontId::proportional(14.0), t.muted_foreground);
            return;
        };
        let col_w = (rect.width() - 48.0).min(760.0);
        let col = Rect::from_center_size(pos2(rect.center().x, rect.center().y), vec2(col_w, rect.height()));
        let prompt_h = 132.0;
        let generating = self.generation.as_ref().is_some_and(|g| g.chat == route);
        let empty = self.chats[ci].messages.is_empty() && !generating;
        let conv_rect = Rect::from_min_max(pos2(col.min.x, col.min.y + 8.0), pos2(col.max.x, col.max.y - prompt_h - if empty { 0.0 } else { 0.0 }));
        let mut regenerate = false;
        kit::child(ui, conv_rect, |ui| {
            Conversation::new().height(conv_rect.height()).fade_edges(t.background).id_salt(("assistant_conv", route)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 22.0;
                ui.add_space(12.0);
                if empty {
                    ui.add_space((conv_rect.height() * 0.28).max(20.0));
                    ui.vertical_centered(|ui| {
                        ui.add(glowkit_ai::Avatar::assistant(22.0));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("How can I help you today?").size(22.0).color(t.foreground));
                        ui.label(egui::RichText::new("Ask anything, attach files, or pick a suggestion below.").size(13.0).color(t.muted_foreground));
                    });
                }
                let n = self.chats[ci].messages.len();
                for (i, (user, text)) in self.chats[ci].messages.iter().enumerate() {
                    if *user {
                        ui.add(ChatMessage::new(Role::User, text).id_salt((route, "u", i)));
                    } else {
                        let last = i + 1 == n && !generating;
                        let out = ChatMessage::new(Role::Assistant, text).actions(true).actions_always_visible(last).id_salt((route, "a", i)).show(ui, |_| {});
                        if out.action == Some(MessageAction::Regenerate) && last {
                            regenerate = true;
                        }
                        if out.action == Some(MessageAction::Copy) {
                            toast(ui.ctx(), Toast::success("Copied to clipboard"));
                        }
                    }
                }
                if let Some(g) = self.generation.clone().filter(|g| g.chat == route) {
                    let e = (now - g.started) as f32;
                    let reason_t = g.reasoning.chars().count() as f32 / REASON_CPS;
                    let t_answer = THINK + reason_t;
                    let chars = ((e - t_answer).max(0.0) * ANSWER_CPS) as usize;
                    let answer = if e > t_answer { prefix_words(&g.answer, chars) } else { String::new() };
                    if chars > g.answer.chars().count() {
                        self.chats[ci].messages.push((false, g.answer.clone()));
                        self.generation = None;
                    } else {
                        ChatMessage::new(Role::Assistant, &answer).streaming(e > t_answer).id_salt((route, "gen")).show(ui, |ui| {
                            if e < THINK {
                                ui.add(Loader::new(LoaderKind::TextShimmer).text("Thinking").size(20.0));
                            } else {
                                let r = prefix_words(&g.reasoning, ((e - THINK) * REASON_CPS) as usize);
                                ui.add(Reasoning::new(&r).streaming(e < t_answer).id_salt((route, "reason")));
                            }
                        });
                        ui.ctx().request_repaint();
                    }
                }
                ui.add_space(8.0);
            });
        });
        if regenerate {
            if let Some(last_user) = self.chats[ci].messages.iter().rev().find(|m| m.0).map(|m| m.1.clone()) {
                self.chats[ci].messages.pop();
                let (answer, reasoning) = answer_for(&last_user);
                self.generation = Some(Generation { chat: route.into(), started: now, answer, reasoning });
            }
        }
        // composer
        let pr = Rect::from_min_max(pos2(col.min.x, col.max.y - prompt_h), pos2(col.max.x, col.max.y - 14.0));
        let mut submitted = None;
        kit::child(ui, pr, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if empty && self.input.is_empty() {
                let items = ["Compare the shell layouts in a table", "Write a Rust function to format money", "Plan a product launch"];
                if let Some(i) = Suggestions::new(&items).icon(Icon::Sparkle).size(12.5).show(ui) {
                    submitted = Some(items[i].to_string());
                }
            }
            let mut model = self.model;
            let out = PromptInput::new(&mut self.input)
                .placeholder("Message the assistant…  (Enter to send, Shift+Enter for a newline)")
                .attachments(&mut self.attachments)
                .generating(generating)
                .max_chars(4000)
                .mic_button(true)
                .id_salt("assistant_prompt")
                .show(ui, |ui| {
                    ui.add(ModelSelector::new(&MODELS, &mut model).ghost(true).id_salt("assistant_model"));
                });
            self.model = model;
            if out.attach {
                self.attachments.push(Attachment::new("screenshot.png", AttachmentKind::Image).detail("320 KB"));
            }
            if out.mic {
                toast(ui.ctx(), Toast::info("Listening…").description("Voice input is a stub in this template."));
            }
            if out.stop {
                if let Some(g) = self.generation.take() {
                    let e = (now - g.started) as f32;
                    let t_answer = THINK + g.reasoning.chars().count() as f32 / REASON_CPS;
                    let partial = prefix_words(&g.answer, ((e - t_answer).max(0.0) * ANSWER_CPS) as usize);
                    self.chats[ci].messages.push((false, if partial.is_empty() { "*Generation stopped.*".into() } else { partial }));
                }
            }
            if let Some(s) = out.submitted {
                submitted = Some(s);
            }
        });
        if let Some(prompt) = submitted {
            if self.chats[ci].messages.is_empty() {
                self.chats[ci].title = prompt.chars().take(32).collect();
                self.rebuild_nav();
            }
            self.chats[ci].messages.push((true, prompt.clone()));
            self.attachments.clear();
            self.input.clear();
            let (answer, reasoning) = answer_for(&prompt);
            self.generation = Some(Generation { chat: route.into(), started: now, answer, reasoning });
        }
    }
}
