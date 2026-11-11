//! The full chat demo: a simulated assistant that thinks, calls a tool and streams Markdown.

use crate::*;
use egui::Ui;
use glowkit_core::Theme;

#[derive(Clone)]
struct Msg {
    role: Role,
    text: String,
    reasoning: String,
    reasoning_secs: f32,
    tool: Option<(String, String, String)>,
    sources: bool,
    branch: usize,
    branches: usize,
}

#[derive(Clone)]
struct Gen {
    started: f64,
    answer: String,
    reasoning: String,
    tool: (String, String, String),
}

#[derive(Clone)]
struct ChatState {
    msgs: Vec<Msg>,
    input: String,
    model: usize,
    attachments: Vec<Attachment>,
    gen: Option<Gen>,
    auto_started: bool,
}

const MODELS: [ModelInfo<'static>; 4] = [
    ModelInfo::new("Sonnet 4.5", "Anthropic").description("Best balance of speed and smarts").tag("New"),
    ModelInfo::new("Haiku 4.5", "Anthropic").description("Fastest for everyday tasks").tag("Fast"),
    ModelInfo::new("GPT-5", "OpenAI").description("Flagship reasoning model"),
    ModelInfo::new("Llama 4 Maverick", "Meta").description("Open weights, runs locally"),
];

pub(crate) const SOURCES: [Source<'static>; 5] = [
    Source::new("The Rust Programming Language: Understanding Ownership", "https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html")
        .snippet("Ownership is a set of rules that govern how a Rust program manages memory."),
    Source::new("egui: an easy-to-use immediate mode GUI in Rust", "https://github.com/emilk/egui").snippet("egui runs on the web, natively, and in your favorite game engine."),
    Source::new("Rust Reference: Lifetimes", "https://doc.rust-lang.org/reference/lifetime-elision.html"),
    Source::new("Rust Blog: Announcing Rust 1.85", "https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html"),
    Source::new("Stack Overflow: When to use Rc vs Arc", "https://stackoverflow.com/questions/rc-vs-arc"),
];

fn answer_for(prompt: &str) -> (String, String, (String, String, String)) {
    let p = prompt.to_lowercase();
    if p.contains("compare") || p.contains("table") || p.contains("vs") {
        (
            "Here's how the three most common Rust GUI approaches compare:\n\n| Library | Paradigm | Web support | Best for |\n|:--|:--|:--:|:--|\n| **egui** | Immediate mode | ✓ | Tools, dashboards, AI apps |\n| iced | Elm architecture | ✓ | Structured apps |\n| Slint | Declarative DSL | ✓ | Embedded & desktop |\n\n> If you want to **ship fast** and keep everything in plain Rust, egui is hard to beat.\n\nWant me to sketch a starter project for one of them?".to_owned(),
            "The user wants a comparison. I'll pick three popular GUI crates, compare paradigm, web support and typical use, and finish with a recommendation.".to_owned(),
            ("search_crates".to_owned(), "{\n  \"query\": \"rust gui\",\n  \"sort\": \"downloads\",\n  \"limit\": 3\n}".to_owned(), "{\n  \"results\": [\"egui\", \"iced\", \"slint\"]\n}".to_owned()),
        )
    } else if p.contains("rust") || p.contains("code") || p.contains("function") || p.contains("write") {
        (
            "Sure! Here's an idiomatic way to **count word frequencies** in Rust:\n\n```rust\nuse std::collections::HashMap;\n\nfn word_counts(text: &str) -> HashMap<&str, usize> {\n    let mut counts = HashMap::new();\n    for word in text.split_whitespace() {\n        *counts.entry(word).or_insert(0) += 1;\n    }\n    counts\n}\n```\n\nA few notes:\n\n1. `split_whitespace()` handles tabs and newlines for you\n2. `entry().or_insert(0)` avoids a double lookup\n3. Borrowing `&str` keys means **zero allocations** per word\n\nLet me know if you'd like a version that ignores case or punctuation.".to_owned(),
            "They want Rust code. A HashMap with the entry API is the idiomatic answer; I should mention why it avoids allocations and double lookups.".to_owned(),
            ("run_code".to_owned(), "{\n  \"language\": \"rust\",\n  \"test\": \"word_counts(\\\"a b a\\\")\"\n}".to_owned(), "{\n  \"ok\": true,\n  \"output\": \"{\\\"a\\\": 2, \\\"b\\\": 1}\"\n}".to_owned()),
        )
    } else {
        let topic: String = prompt.chars().take(60).collect();
        (
            format!(
                "Great question! You asked: *\u{201c}{topic}\u{201d}*\n\nHere's a quick overview:\n\n- **Start simple**: get a working version end to end before polishing\n- **Measure**: profile before optimizing, intuition is often wrong\n- **Iterate**: small, frequent changes are easier to review\n\n### Next steps\n\nTell me a bit more about your goal and I can go deeper, with `code`, tables or a step-by-step plan."
            ),
            format!("The user asked about \u{201c}{topic}\u{201d}. I'll give a short structured overview and offer to go deeper."),
            ("search_web".to_owned(), format!("{{\n  \"query\": \"{}\",\n  \"max_results\": 5\n}}", topic.replace('"', "'")), "{\n  \"results\": 5,\n  \"top\": \"doc.rust-lang.org\"\n}".to_owned()),
        )
    }
}

fn initial() -> ChatState {
    ChatState {
        msgs: vec![
            Msg { role: Role::User, text: "How does ownership work in Rust?".into(), reasoning: String::new(), reasoning_secs: 0.0, tool: None, sources: false, branch: 0, branches: 1 },
            Msg {
                role: Role::Assistant,
                text: "Rust tracks **ownership** at compile time: every value has exactly one owner, and it's dropped when the owner goes out of scope.\n\n- Assigning a `String` *moves* it, so the old binding can't be used\n- Borrowing with `&` lets you read without taking ownership\n\nThis gives you memory safety **without a garbage collector**.".into(),
                reasoning: "Explain the three ownership rules briefly, with moves and borrows as examples.".into(),
                reasoning_secs: 3.0,
                tool: None,
                sources: true,
                branch: 1,
                branches: 2,
            },
        ],
        input: String::new(),
        model: 0,
        attachments: vec![Attachment::new("main.rs", AttachmentKind::Code).detail("2.1 KB")],
        gen: None,
        auto_started: false,
    }
}

fn start(st: &mut ChatState, prompt: String, now: f64) {
    let (answer, reasoning, tool) = answer_for(&prompt);
    st.msgs.push(Msg { role: Role::User, text: prompt, reasoning: String::new(), reasoning_secs: 0.0, tool: None, sources: false, branch: 0, branches: 1 });
    st.gen = Some(Gen { started: now, answer, reasoning, tool });
}

// timeline of a simulated response (seconds)
const T_THINK: f32 = 0.5;
const REASON_CPS: f32 = 70.0;
const TOOL_SECS: f32 = 1.3;
const ANSWER_CPS: f32 = 85.0;

fn prefix_tokens(s: &str, chars: usize) -> String {
    // reveal whole words so tokens arrive like an LLM stream
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i >= chars && c.is_whitespace() {
            break;
        }
        out.push(c);
    }
    out
}

fn finish(st: &mut ChatState, g: &Gen, answer: String, reasoning_secs: f32) {
    st.msgs.push(Msg {
        role: Role::Assistant,
        text: answer,
        reasoning: g.reasoning.clone(),
        reasoning_secs,
        tool: Some(g.tool.clone()),
        sources: true,
        branch: 0,
        branches: 1,
    });
    st.gen = None;
}

pub(crate) fn show(ui: &mut Ui) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with("ai_chat_demo");
    let mut st: ChatState = glowkit_core::state(ui, id, initial);
    let now = ui.input(|i| i.time);
    if !st.auto_started {
        st.auto_started = true;
        start(&mut st, "Compare the popular Rust GUI libraries in a table".into(), now);
    }
    let width = ui.available_width();
    let col_w = width.min(780.0);
    ui.vertical_centered(|ui| {
        ui.set_max_width(col_w);
        // header
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.set_min_height(32.0);
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Rust questions").size(14.0).color(t.foreground));
            ui.label(egui::RichText::new("·  Private").size(12.5).color(t.muted_foreground));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::common::icon_button(ui, Icon::Pencil, 30.0, None, "New chat").clicked() {
                    let mut fresh = initial();
                    fresh.msgs.clear();
                    fresh.auto_started = true;
                    fresh.attachments.clear();
                    st = fresh;
                }
                let used = 18_400 + st.msgs.iter().map(|m| m.text.len() as u64 * 3).sum::<u64>();
                ui.add(ContextMeter::new(used, 200_000).segment("Input", used * 6 / 10).segment("Output", used * 3 / 10).segment("Reasoning", used / 10).model(MODELS[st.model].name).cost(used as f64 * 0.000_003));
            });
        });
        ui.add_space(6.0);
        let conv_h = 470.0;
        let mut regenerate = false;
        Conversation::new().height(conv_h).fade_edges(t.card).id_salt("chat_demo_conv").show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 22.0;
            ui.add_space(8.0);
            if st.msgs.is_empty() && st.gen.is_none() {
                ui.add_space(110.0);
                ui.vertical_centered(|ui| {
                    ui.add(Avatar::assistant(22.0));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("How can I help you today?").size(22.0).color(t.foreground));
                });
            }
            let n = st.msgs.len();
            for (i, m) in st.msgs.iter_mut().enumerate() {
                match m.role {
                    Role::User => {
                        ui.add(ChatMessage::new(Role::User, &m.text).id_salt(("u", i)));
                    }
                    _ => {
                        let last = i + 1 == n && st.gen.is_none();
                        let mut msg = ChatMessage::new(Role::Assistant, &m.text).actions(true).actions_always_visible(last).id_salt(("a", i));
                        if m.branches > 1 {
                            msg = msg.branch(&mut m.branch, m.branches);
                        }
                        let (reasoning, secs, tool, sources) = (&m.reasoning, m.reasoning_secs, &m.tool, m.sources);
                        let out = msg.show(ui, |ui| {
                            if !reasoning.is_empty() {
                                ui.add(Reasoning::new(reasoning).duration(secs).id_salt(("r", i)));
                            }
                            if let Some((name, input, output)) = tool {
                                ui.add(ToolCall::new(name).status(ToolStatus::Completed).input(input).output(output).id_salt(("t", i)));
                            }
                            if sources {
                                ui.add(Sources::new(&SOURCES[..4]).id_salt(("s", i)));
                            }
                        });
                        if out.action == Some(MessageAction::Regenerate) && last {
                            regenerate = true;
                        }
                    }
                }
            }
            // in-flight response
            if let Some(g) = st.gen.clone() {
                let e = (now - g.started) as f32;
                let reason_secs = g.reasoning.chars().count() as f32 / REASON_CPS;
                let t_tool = T_THINK + reason_secs;
                let t_answer = t_tool + TOOL_SECS;
                let answer_chars = ((e - t_answer).max(0.0) * ANSWER_CPS) as usize;
                let answer = if e > t_answer { prefix_tokens(&g.answer, answer_chars) } else { String::new() };
                let done = answer_chars > g.answer.chars().count();
                if done {
                    finish(&mut st, &g, g.answer.clone(), reason_secs + T_THINK);
                } else {
                    ChatMessage::new(Role::Assistant, &answer).streaming(e > t_answer).id_salt(("a", n)).show(ui, |ui| {
                        if e < T_THINK {
                            ui.add(Loader::new(LoaderKind::TextShimmer).text("Thinking").size(20.0));
                        } else {
                            let r_chars = ((e - T_THINK) * REASON_CPS) as usize;
                            let r = prefix_tokens(&g.reasoning, r_chars);
                            ui.add(Reasoning::new(&r).streaming(e < t_tool).id_salt(("r", n)));
                        }
                        if e >= t_tool {
                            let status = if e < t_answer { ToolStatus::Running } else { ToolStatus::Completed };
                            let mut tc = ToolCall::new(&g.tool.0).status(status).input(&g.tool.1).id_salt(("t", n));
                            if status == ToolStatus::Completed {
                                tc = tc.output(&g.tool.2);
                            }
                            ui.add(tc);
                        }
                        if e >= t_answer {
                            ui.add(Sources::new(&SOURCES[..4]).id_salt(("s", n)));
                        }
                    });
                    ui.ctx().request_repaint();
                }
            }
            ui.add_space(8.0);
        });
        if regenerate {
            if let Some(last_user) = st.msgs.iter().rev().find(|m| m.role == Role::User).map(|m| m.text.clone()) {
                st.msgs.pop();
                st.msgs.pop();
                start(&mut st, last_user, now);
            }
        }
        ui.add_space(6.0);
        if st.gen.is_none() && st.input.is_empty() {
            let items = ["Write a Rust function that counts words", "Compare Rust GUI libraries in a table", "Explain lifetimes simply"];
            if let Some(i) = Suggestions::new(&items).icon(Icon::Sparkle).size(12.5).show(ui) {
                st.input = items[i].to_owned();
            }
            ui.add_space(4.0);
        }
        let generating = st.gen.is_some();
        let mut model = st.model;
        let out = PromptInput::new(&mut st.input)
            .placeholder("Message the assistant…  (Enter to send, Shift+Enter for a newline)")
            .attachments(&mut st.attachments)
            .generating(generating)
            .max_chars(4000)
            .mic_button(true)
            .id_salt("chat_demo_prompt")
            .show(ui, |ui| {
                ui.add(ModelSelector::new(&MODELS, &mut model).ghost(true));
            });
        st.model = model;
        if out.attach {
            let k = st.attachments.len();
            st.attachments.push(match k % 3 {
                0 => Attachment::new("screenshot.png", AttachmentKind::Image).detail("320 KB"),
                1 => Attachment::new("notes.pdf", AttachmentKind::File).detail("1.2 MB"),
                _ => Attachment::new("lib.rs", AttachmentKind::Code).detail("4 KB"),
            });
        }
        if out.stop {
            if let Some(g) = st.gen.clone() {
                let e = (now - g.started) as f32;
                let t_answer = T_THINK + g.reasoning.chars().count() as f32 / REASON_CPS + TOOL_SECS;
                let partial = prefix_tokens(&g.answer, ((e - t_answer).max(0.0) * ANSWER_CPS) as usize);
                finish(&mut st, &g, if partial.is_empty() { "*Generation stopped.*".into() } else { partial }, e.min(t_answer));
            }
        }
        if let Some(p) = out.submitted {
            start(&mut st, p, now);
        }
    });
    glowkit_core::set_state(ui, id, st);
}

