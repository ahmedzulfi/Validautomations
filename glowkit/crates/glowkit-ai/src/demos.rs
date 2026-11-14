use crate::*;
use egui::{vec2, Ui};
use glowkit_core::demo::{Category::Ai as C, Demo};
use glowkit_core::{Theme, Trigger};

pub(crate) const SAMPLE_MD: &str = r#"## Ownership in Rust

Rust manages memory through **ownership**: every value has a single *owner*, and the value is dropped when its owner goes out of scope. Read more in [the book](https://doc.rust-lang.org/book/).

1. Each value has an owner
2. There can only be **one owner** at a time
   - moves transfer ownership
   - `clone()` makes a deep copy
3. When the owner goes out of scope, the value is dropped

```rust
fn main() {
    let s = String::from("hello"); // s owns the string
    let t = s;                     // ownership moves to t
    println!("{t}, world!");
}
```

| Concept | Checked at | Cost |
|:--------|:----------:|-----:|
| Ownership | compile time | zero |
| Borrowing | compile time | zero |
| `Rc<T>` | run time | refcount |

> **Tip:** if the borrow checker complains, try restructuring data before reaching for `clone()`.

---

- [x] Read chapter 4
- [ ] Try the exercises
"#;

const STREAM_MD: &str = "### Plan\n\nI'll build the **parser** first, then wire up the renderer:\n\n1. Tokenize the input\n2. Build an AST\n3. Render with `egui`\n\n```py\ndef tokenize(src):\n    return src.split()\n```\n\nThat keeps each step *small* and testable.";

/// Seconds since the demo started, looping every `period` seconds.
fn looped(ui: &Ui, period: f32) -> f32 {
    glowkit_core::animate(ui);
    glowkit_core::time(ui) % period
}

fn label(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.label(egui::RichText::new(text).size(11.5).color(t.muted_foreground));
}

/// A column of fixed width, centered in the preview.
fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.vertical_centered(|ui| {
        ui.set_max_width(width.min(ui.available_width()));
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), add);
    });
}

fn simulated_level(ui: &Ui, seed: f32) -> f32 {
    let t = glowkit_core::time(ui);
    let n = glowkit_core::noise::noise3(t * 2.3, seed, 0.0) * 0.5 + 0.5;
    let syll = ((t * 5.5).sin() * 0.5 + 0.5).powf(1.5);
    let phrase = if (t * 0.4 + seed).sin() > -0.3 { 1.0 } else { 0.15 };
    ((n * 0.6 + syll * 0.5) * phrase).clamp(0.0, 1.0)
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "ai-chat",
            name: "AI Chat",
            category: C,
            description: "A complete, working chat: type a message and a simulated assistant thinks, calls a tool, cites sources and streams a Markdown answer.",
            covers: &["Vercel AI Elements: Chatbot example", "assistant-ui: Thread", "shadcn-chatbot-kit: Chat", "prompt-kit: Chat UI", "simple-ai: Chat"],
            code: r##"Conversation::new().show(ui, |ui| {
    for m in &messages {
        ChatMessage::new(m.role, &m.text).actions(true).show(ui, |ui| {
            ui.add(Reasoning::new(&m.reasoning).duration(m.secs));
            ui.add(ToolCall::new("search_web").status(ToolStatus::Completed));
        });
    }
});
let out = PromptInput::new(&mut input).generating(busy).show(ui, |ui| {
    ui.add(ModelSelector::new(&MODELS, &mut model).ghost(true));
});
if let Some(prompt) = out.submitted { /* send to your model */ }"##,
            show: crate::chat_demo::show,
        },
        Demo {
            id: "ai-markdown",
            name: "Markdown",
            category: C,
            description: "Streaming-safe Markdown: headings, lists, tables, quotes, links and highlighted code blocks with a copy button.",
            covers: &["Streamdown: Markdown", "Vercel AI Elements: Response", "prompt-kit: Markdown", "assistant-ui: Markdown Text", "shadcn-chatbot-kit: Markdown Renderer", "prompt-kit: Code Block", "Vercel AI Elements: Code Block"],
            code: r##"ui.add(Markdown::new(&response_text));
// while streaming: half-written syntax never flashes
ui.add(Markdown::new(&partial).caret(true).fade_tail(24));"##,
            show: |ui| column(ui, 760.0, |ui| {
                ui.add(Markdown::new(SAMPLE_MD));
            }),
        },
        Demo {
            id: "ai-streaming-text",
            name: "Streaming Response",
            category: C,
            description: "Reveals a response token by token with a per-token fade and a blinking caret; works for plain text or live Markdown.",
            covers: &["Vercel AI Elements: Response", "prompt-kit: Response Stream", "Kibo UI: AI Response", "llm-ui: Streaming", "ElevenLabs UI: Response"],
            code: r##"ui.add(StreamingText::new(text).tokens_per_second(14.0));
ui.add(StreamingText::new(markdown).markdown(true).progress(p));"##,
            show: |ui| column(ui, 720.0, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                label(ui, "FADE");
                ui.add(StreamingText::new("Large language models generate text one token at a time, so a good interface reveals each new piece smoothly instead of jumping.").tokens_per_second(9.0).trigger(Trigger::Loop { pause_ms: 1500 }).id_salt("s1"));
                ui.add_space(8.0);
                label(ui, "BLUR");
                ui.add(StreamingText::new("Each word sharpens into focus as it arrives.").style(RevealStyle::Blur).tokens_per_second(6.0).trigger(Trigger::Loop { pause_ms: 1500 }).id_salt("s2"));
                ui.add_space(8.0);
                label(ui, "MARKDOWN");
                let p = (looped(ui, 9.0) / 7.0).min(1.0);
                ui.add(StreamingText::new(STREAM_MD).markdown(true).progress(p).id_salt("s3"));
            }),
        },
        Demo {
            id: "ai-chat-message",
            name: "Chat Message",
            category: C,
            description: "User bubbles on the right, assistant answers on the left with an avatar, hover-revealed actions and a branch switcher.",
            covers: &["Vercel AI Elements: Message", "Vercel AI Elements: Actions", "Vercel AI Elements: Branch", "prompt-kit: Message", "assistant-ui: Message", "assistant-ui: Branch Picker", "shadcn-chatbot-kit: Chat Message", "prompt-kit: Feedback Bar", "ElevenLabs UI: Message"],
            code: r##"ui.add(ChatMessage::new(Role::User, "Can you explain closures?").timestamp("2:14 PM"));
let out = ChatMessage::new(Role::Assistant, answer)
    .name("Assistant").timestamp("2:14 PM")
    .actions(true)
    .branch(&mut branch, 3)
    .show(ui, |_| {});"##,
            show: |ui| column(ui, 720.0, |ui| {
                ui.spacing_mut().item_spacing.y = 18.0;
                ui.add(ChatMessage::new(Role::User, "Can you explain closures in one paragraph?").timestamp("2:14 PM").user_avatar("JD").actions(true).id_salt("m1"));
                let id = ui.scope_id().with("branch");
                let mut b: usize = glowkit_core::state(ui, id, || 1);
                let answers = [
                    "A closure is a function value that remembers the variables around it.",
                    "A **closure** is an anonymous function that can capture variables from its surrounding scope. In Rust, closures implement `Fn`, `FnMut` or `FnOnce` depending on how they use what they capture.",
                    "Think of a closure as a function plus a backpack of captured variables.",
                ];
                let text = answers[b.min(2)];
                ChatMessage::new(Role::Assistant, text)
                    .name("Assistant")
                    .timestamp("2:14 PM")
                    .actions(true)
                    .actions_always_visible(true)
                    .branch(&mut b, 3)
                    .id_salt("m2")
                    .show(ui, |_| {});
                glowkit_core::set_state(ui, id, b);
                ui.add(ChatMessage::new(Role::System, "Model switched to Nova Flash"));
            }),
        },
        Demo {
            id: "ai-conversation",
            name: "Conversation",
            category: C,
            description: "A scroll area that sticks to the bottom as messages stream in; scroll up and a floating button glides you back down.",
            covers: &["Vercel AI Elements: Conversation", "prompt-kit: Chat Container", "prompt-kit: Scroll Button", "assistant-ui: Thread Viewport", "ElevenLabs UI: Conversation"],
            code: r##"Conversation::new().height(400.0).fade_edges(bg).show(ui, |ui| {
    for m in &messages { ui.add(ChatMessage::new(m.role, &m.text)); }
});"##,
            show: |ui| column(ui, 720.0, |ui| {
                let t = Theme::of(ui);
                let n = (glowkit_core::time(ui) / 1.6) as usize % 14 + 4;
                glowkit_core::animate(ui);
                Conversation::new().height(330.0).fade_edges(t.card).id_salt("conv_demo").show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 14.0;
                    for i in 0..n {
                        if i % 2 == 0 {
                            ui.add(ChatMessage::new(Role::User, &format!("Message #{} from the user", i / 2 + 1)).id_salt(i));
                        } else {
                            ui.add(ChatMessage::new(Role::Assistant, "Here is a reply that keeps the conversation going, new content arrives at the bottom and the view follows it.").id_salt(i));
                        }
                    }
                });
                label(ui, "Scroll up to reveal the scroll-to-bottom button.");
            }),
        },
        Demo {
            id: "ai-prompt-input",
            name: "Prompt Input",
            category: C,
            description: "An auto-growing composer with attachment chips, model pill, character count, mic, and a send button that becomes stop while generating.",
            covers: &["Vercel AI Elements: Prompt Input", "prompt-kit: Prompt Input", "Kokonut UI: AI Input", "assistant-ui: Composer", "shadcn-chatbot-kit: Message Input", "simple-ai: Chat Input", "prompt-kit: File Upload"],
            code: r##"let out = PromptInput::new(&mut text)
    .attachments(&mut files)
    .generating(busy)
    .max_chars(4000)
    .show(ui, |ui| { ui.add(ModelSelector::new(&models, &mut m).ghost(true)); });
if let Some(prompt) = out.submitted { send(prompt); }
if out.stop { cancel(); }"##,
            show: |ui| column(ui, 680.0, |ui| {
                #[derive(Clone)]
                struct S {
                    text: String,
                    files: Vec<Attachment>,
                    busy_until: f64,
                    model: usize,
                    last: String,
                }
                let id = ui.scope_id().with("prompt_demo");
                let mut s: S = glowkit_core::state(ui, id, || S {
                    text: String::new(),
                    files: vec![Attachment::new("design.png", AttachmentKind::Image).detail("1.4 MB"), Attachment::new("api.rs", AttachmentKind::Code).detail("3 KB")],
                    busy_until: 0.0,
                    model: 0,
                    last: String::new(),
                });
                let now = ui.input(|i| i.time);
                let models = [ModelInfo::new("Nova 2", "Lumen Labs"), ModelInfo::new("Orbit 5", "Orbit AI"), ModelInfo::new("Helix Pro", "Helix")];
                ui.add_space(40.0);
                let mut model = s.model;
                let out = PromptInput::new(&mut s.text).attachments(&mut s.files).generating(now < s.busy_until).max_chars(500).mic_button(true).id_salt("pi1").show(ui, |ui| {
                    ui.add(ModelSelector::new(&models, &mut model).ghost(true));
                });
                s.model = model;
                if let Some(p) = out.submitted {
                    s.last = p;
                    s.busy_until = now + 3.0;
                }
                if out.stop {
                    s.busy_until = 0.0;
                }
                if out.attach {
                    s.files.push(Attachment::new("report.pdf", AttachmentKind::File).detail("820 KB"));
                }
                if now < s.busy_until {
                    ui.ctx().request_repaint();
                }
                ui.add_space(14.0);
                if !s.last.is_empty() {
                    label(ui, &format!("Sent: {}", s.last));
                } else {
                    label(ui, "Click the box and type. Enter sends, Shift+Enter adds a newline.");
                }
                ui.add_space(18.0);
                let mut idle = String::from("Generating: the send button turns into a stop button");
                let _ = PromptInput::new(&mut idle).generating(true).attach_button(false).glow(false).id_salt("pi2").show(ui, |_| {});
                glowkit_core::set_state(ui, id, s);
            }),
        },
        Demo {
            id: "ai-suggestions",
            name: "Suggestions",
            category: C,
            description: "Rounded prompt suggestion chips that wrap, lift slightly and highlight on hover.",
            covers: &["Vercel AI Elements: Suggestion", "prompt-kit: Prompt Suggestion", "assistant-ui: Thread Suggestions", "shadcn-chatbot-kit: Prompt Suggestions"],
            code: r##"if let Some(i) = Suggestions::new(&["Summarize", "Translate"]).show(ui) {
    input = items[i].to_string();
}"##,
            show: |ui| column(ui, 720.0, |ui| {
                ui.add_space(60.0);
                let id = ui.scope_id().with("sugg");
                let mut picked: String = glowkit_core::state(ui, id, String::new);
                let items = ["Explain quantum computing", "Write a haiku about Rust", "Plan a 3-day trip to Kyoto", "Debug my SQL query", "Summarize this article", "Draft a polite follow-up email"];
                if let Some(i) = Suggestions::new(&items).centered(true).show(ui) {
                    picked = items[i].to_owned();
                }
                ui.add_space(24.0);
                let items2 = ["Make it shorter", "More formal", "Add examples"];
                if let Some(i) = Suggestions::new(&items2).icon(Icon::Sparkle).centered(true).size(12.5).show(ui) {
                    picked = items2[i].to_owned();
                }
                ui.add_space(20.0);
                ui.vertical_centered(|ui| label(ui, if picked.is_empty() { "Pick a suggestion" } else { &picked }));
                glowkit_core::set_state(ui, id, picked);
            }),
        },
        Demo {
            id: "ai-reasoning",
            name: "Reasoning & Chain of Thought",
            category: C,
            description: "A collapsible \"Thinking…\" block that shimmers while streaming and folds to \"Thought for 4s\", plus a step timeline with icons and connectors.",
            covers: &["Vercel AI Elements: Reasoning", "Vercel AI Elements: Chain of Thought", "prompt-kit: Reasoning", "prompt-kit: Thinking Bar", "assistant-ui: Reasoning", "prompt-kit: Steps"],
            code: r##"ui.add(Reasoning::new(&thoughts).streaming(still_thinking));
ui.add(ChainOfThought::new(vec![
    ThoughtStep::new(Icon::Search, "Searching the web").chips(&["rust-lang.org"]),
    ThoughtStep::new(Icon::Brain, "Comparing results").status(StepStatus::Active),
]));"##,
            show: |ui| column(ui, 680.0, |ui| {
                ui.spacing_mut().item_spacing.y = 18.0;
                let e = looped(ui, 9.0);
                let text = "The user asks for the fastest way to parse JSON in Rust. **serde_json** is the default choice; for hot paths *simd-json* is faster but needs mutable input. I should mention both and recommend benchmarking with real payloads.";
                let chars = ((e - 0.3).max(0.0) * 55.0) as usize;
                let shown: String = text.chars().take(chars).collect();
                ui.add(Reasoning::new(&shown).streaming(chars < text.chars().count()).id_salt("r1"));
                let step = (e / 1.6) as usize;
                let st = |i: usize| if step > i { StepStatus::Complete } else if step == i { StepStatus::Active } else { StepStatus::Pending };
                ui.add(
                    ChainOfThought::new(vec![
                        ThoughtStep::new(Icon::Search, "Searching for JSON parser benchmarks").chips(&["github.com", "docs.rs", "reddit.com"]).status(st(0)),
                        ThoughtStep::new(Icon::File, "Reading the serde_json documentation").description("Zero-copy deserialization with borrowed &str fields").status(st(1)),
                        ThoughtStep::new(Icon::Code, "Comparing throughput and ergonomics").status(st(2)),
                        ThoughtStep::new(Icon::Check, "Writing the recommendation").status(st(3)),
                    ])
                    .id_salt("cot"),
                );
            }),
        },
        Demo {
            id: "ai-tool-call",
            name: "Tool Call",
            category: C,
            description: "A card for function calls: tool name, status badge (pending, running with spinner, completed, error) and collapsible parameters and result.",
            covers: &["Vercel AI Elements: Tool", "assistant-ui: Tool UI", "prompt-kit: Tool", "Kibo UI: AI Tool"],
            code: r##"ui.add(ToolCall::new("get_weather")
    .status(ToolStatus::Completed)
    .input(r#"{ "city": "Berlin" }"#)
    .output(r#"{ "temp": 21, "sky": "clear" }"#));"##,
            show: |ui| column(ui, 680.0, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.add(ToolCall::new("get_weather").description("Weather API").status(ToolStatus::Completed).input("{\n  \"city\": \"Berlin\",\n  \"units\": \"metric\"\n}").output("{\n  \"temp\": 21,\n  \"sky\": \"clear\",\n  \"wind_kph\": 9\n}").default_open(true).icon(Icon::Globe).id_salt("t1"));
                ui.add(ToolCall::new("search_docs").status(ToolStatus::Running).input("{ \"query\": \"egui ScrollArea stick_to_bottom\" }").icon(Icon::Search).id_salt("t2"));
                ui.add(ToolCall::new("send_email").status(ToolStatus::Pending).description("Awaiting approval").icon(Icon::ArrowRight).id_salt("t3"));
                ui.add(ToolCall::new("run_sql").status(ToolStatus::Error).input("SELECT * FROM users WHERE id = ?").error("permission denied for table users").icon(Icon::Terminal).id_salt("t4"));
            }),
        },
        Demo {
            id: "ai-sources",
            name: "Sources & Citations",
            category: C,
            description: "\"Used 5 sources\" with stacked favicons that expands into links, and inline citation pills with hover cards.",
            covers: &["Vercel AI Elements: Sources", "Vercel AI Elements: Inline Citation", "prompt-kit: Source", "Kibo UI: AI Sources"],
            code: r##"ui.add(Sources::new(&sources));
ui.horizontal_wrapped(|ui| {
    ui.label("Rust has no garbage collector");
    ui.add(InlineCitation::new(&sources[..2]));
});"##,
            show: |ui| column(ui, 680.0, |ui| {
                let t = Theme::of(ui);
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add(Sources::new(&crate::chat_demo::SOURCES).default_open(true).id_salt("src"));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let txt = |ui: &mut Ui, s: &str| {
                        ui.label(egui::RichText::new(s).size(14.0).color(t.foreground));
                    };
                    txt(ui, "Rust guarantees memory safety without a garbage collector");
                    ui.add(InlineCitation::new(&crate::chat_demo::SOURCES[..1]).id_salt("c1"));
                    txt(ui, "and egui runs both natively and in the browser");
                    ui.add(InlineCitation::new(&crate::chat_demo::SOURCES[1..4]).id_salt("c2"));
                    txt(ui, ". Hover a citation.");
                });
            }),
        },
        Demo {
            id: "ai-loaders",
            name: "AI Loaders",
            category: C,
            description: "Typing indicator and loader variants for model activity: dots, typing, pulse, wave, circular, bars, text shimmer and text blink.",
            covers: &["prompt-kit: Loader", "prompt-kit: Text Shimmer", "Vercel AI Elements: Loader", "Vercel AI Elements: Shimmer", "shadcn-chatbot-kit: Typing Indicator", "Kibo UI: AI Loader", "ElevenLabs UI: Shimmering Text"],
            code: r##"ui.add(TypingIndicator::new().bubble(true));
ui.add(Loader::new(LoaderKind::Wave));
ui.add(ShimmerText::new("Thinking…"));"##,
            show: |ui| {
                ui.add_space(50.0);
                let kinds = [
                    (LoaderKind::Dots, "Dots"),
                    (LoaderKind::Typing, "Typing"),
                    (LoaderKind::Pulse, "Pulse"),
                    (LoaderKind::Wave, "Wave"),
                    (LoaderKind::Circular, "Circular"),
                    (LoaderKind::Bars, "Bars"),
                ];
                ui.columns(kinds.len(), |cols| {
                    for (c, (k, name)) in cols.iter_mut().zip(kinds) {
                        c.vertical_centered(|ui| {
                            ui.add(Loader::new(k).size(32.0));
                            ui.add_space(12.0);
                            label(ui, name);
                        });
                    }
                });
                ui.add_space(56.0);
                ui.columns(3, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.add(TypingIndicator::new().bubble(true));
                        ui.add_space(10.0);
                        label(ui, "Typing indicator");
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.add(Loader::new(LoaderKind::TextShimmer).text("Thinking").size(24.0));
                        ui.add_space(14.0);
                        label(ui, "Text shimmer");
                    });
                    cols[2].vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.add(Loader::new(LoaderKind::TextBlink).text("Generating").size(24.0));
                        ui.add_space(14.0);
                        label(ui, "Text blink");
                    });
                });
            },
        },
        Demo {
            id: "ai-voice-orb",
            name: "Voice Orb",
            category: C,
            description: "An organic glowing orb for voice agents that breathes when idle and swells with the audio level when listening or speaking.",
            covers: &["ElevenLabs UI: Orb", "Vercel AI Elements: Persona", "Magic UI: Orb-like audio visualizers"],
            code: r##"ui.add(VoiceOrb::new()
    .state(VoiceState::Speaking)
    .amplitude(audio_level)
    .size(200.0));"##,
            show: |ui| {
                ui.add_space(10.0);
                let lvl = simulated_level(ui, 0.0);
                let states = [(VoiceState::Idle, "Idle", 0.0), (VoiceState::Listening, "Listening", lvl * 0.7), (VoiceState::Thinking, "Thinking", 0.0), (VoiceState::Speaking, "Speaking", lvl)];
                let t = Theme::of(ui);
                ui.columns(4, |cols| {
                    for (i, (c, (s, name, a))) in cols.iter_mut().zip(states).enumerate() {
                        c.vertical_centered(|ui| {
                            let colors = match i {
                                1 => [t.chart[1], t.chart[5], t.chart[0]],
                                3 => [t.gradient[1], t.gradient[0], t.gradient[2]],
                                _ => t.gradient,
                            };
                            ui.add(VoiceOrb::new().state(s).amplitude(a).size(150.0).colors(colors).id_salt(("orb", i)));
                            label(ui, name);
                        });
                    }
                });
            },
        },
        Demo {
            id: "ai-waveform",
            name: "Waveform & Voice Button",
            category: C,
            description: "A scrolling live waveform for recording, mirrored audio bars, and a mic button with pulsing rings while listening.",
            covers: &["ElevenLabs UI: Waveform", "ElevenLabs UI: Live Waveform", "ElevenLabs UI: Bar Visualizer", "ElevenLabs UI: Voice Button", "Vercel AI Elements: Speech Input"],
            code: r##"ui.add(LiveWaveform::new(mic_level).active(recording));
ui.add(AudioBars::new(&levels));
if ui.add(VoiceButton::new(listening).amplitude(mic_level)).clicked() {
    listening = !listening;
}"##,
            show: |ui| column(ui, 700.0, |ui| {
                let id = ui.scope_id().with("wave_demo");
                let mut listening: bool = glowkit_core::state(ui, id, || true);
                let lvl = simulated_level(ui, 1.7);
                label(ui, "LIVE WAVEFORM");
                ui.add(LiveWaveform::new(if listening { lvl } else { 0.0 }).active(listening).height(70.0).id_salt("lw"));
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        label(ui, "AUDIO BARS");
                        ui.add_space(8.0);
                        let time = glowkit_core::time(ui);
                        let levels: Vec<f32> = (0..32)
                            .map(|i| {
                                let x = i as f32 / 31.0;
                                let env = (1.0 - (x * 2.0 - 1.0).powi(2)).max(0.0);
                                let n = glowkit_core::noise::noise3(x * 4.0, time * 2.0, 3.0) * 0.5 + 0.5;
                                if listening { (env * (0.25 + 0.75 * n) * (0.4 + lvl)).min(1.0) } else { 0.05 }
                            })
                            .collect();
                        ui.add(AudioBars::new(&levels).height(64.0).bar(5.0, 4.0));
                    });
                    ui.add_space(40.0);
                    ui.vertical(|ui| {
                        if ui.add(VoiceButton::new(listening).amplitude(lvl)).clicked() {
                            listening = !listening;
                        }
                    });
                    ui.vertical(|ui| {
                        ui.add_space(40.0);
                        label(ui, if listening { "Listening… click to stop" } else { "Click to speak" });
                    });
                });
                glowkit_core::set_state(ui, id, listening);
            }),
        },
        Demo {
            id: "ai-task-list",
            name: "Task List / Plan",
            category: C,
            description: "An agent's plan as a collapsible checklist: items go from pending to a spinner to a check that pops in, with a progress count.",
            covers: &["Vercel AI Elements: Task", "Vercel AI Elements: Plan", "Vercel AI Elements: Queue", "assistant-ui: Tool progress"],
            code: r##"ui.add(TaskList::new("Refactor the parser", &[
    TaskItem::new("Read parser.rs", TaskStatus::Done),
    TaskItem::new("Write failing tests", TaskStatus::Running),
    TaskItem::new("Fix the tokenizer", TaskStatus::Pending),
]));"##,
            show: |ui| column(ui, 640.0, |ui| {
                let e = looped(ui, 10.0);
                let step = (e / 1.4) as usize;
                let s = |i: usize| if step > i { TaskStatus::Done } else if step == i { TaskStatus::Running } else { TaskStatus::Pending };
                let items = [
                    TaskItem::new("Read the existing parser", s(0)).detail("src/parser.rs"),
                    TaskItem::new("Write failing tests for nested lists", s(1)).detail("tests/lists.rs"),
                    TaskItem::new("Fix the tokenizer", s(2)),
                    TaskItem::new("Run the full test suite", s(3)).detail("cargo test"),
                    TaskItem::new("Summarize the changes", s(4)),
                ];
                ui.add(TaskList::new("Fix nested list parsing", &items).description("5 steps · started 12s ago").id_salt("plan"));
                ui.add_space(14.0);
                let items2 = [TaskItem::new("Install dependencies", TaskStatus::Done), TaskItem::new("Deploy to staging", TaskStatus::Failed).detail("exit code 1")];
                ui.add(TaskList::new("Deploy preview", &items2).default_open(true).id_salt("plan2"));
            }),
        },
        Demo {
            id: "ai-context-meter",
            name: "Context Meter",
            category: C,
            description: "Token usage of the model's context window as a compact ring with a hover breakdown, or a segmented bar with legend and cost.",
            covers: &["Vercel AI Elements: Context", "assistant-ui: Context usage"],
            code: r##"ui.add(ContextMeter::new(48_200, 128_000)
    .segment("Input", 30_100)
    .segment("Output", 12_900)
    .segment("Reasoning", 5_200)
    .cost(0.0241));"##,
            show: |ui| column(ui, 520.0, |ui| {
                ui.add_space(20.0);
                let e = looped(ui, 12.0);
                let used = (20_000.0 + e / 12.0 * 100_000.0) as u64;
                ui.horizontal(|ui| {
                    ui.add(ContextMeter::new(used, 128_000).segment("Input", used * 6 / 10).segment("Output", used * 3 / 10).segment("Reasoning", used / 10).model("Nova 2").cost(used as f64 * 3e-6));
                    label(ui, "  ← hover for the breakdown");
                });
                ui.add_space(30.0);
                ui.add(ContextMeter::new(used, 128_000).style(MeterStyle::Bar).segment("Input", used * 6 / 10).segment("Output", used * 3 / 10).segment("Reasoning", used / 10).model("Nova 2").cost(used as f64 * 3e-6));
            }),
        },
        Demo {
            id: "ai-artifact",
            name: "Artifact",
            category: C,
            description: "A titled panel for generated output with segmented Preview / Code tabs and action buttons.",
            covers: &["Vercel AI Elements: Artifact", "Claude: Artifacts panel", "assistant-ui: Artifacts"],
            code: r##"Artifact::new("Pricing card").subtitle("Component · v2")
    .tabs(&["Preview", "Code"])
    .actions(&[(Icon::Copy, "Copy"), (Icon::Download, "Download")])
    .show(ui, |ui, tab| { /* body for tab */ });"##,
            show: |ui| column(ui, 720.0, |ui| {
                let t = Theme::of(ui);
                let code = "fn pricing_card(ui: &mut Ui, plan: &Plan) {\n    ui.heading(&plan.name);\n    ui.label(format!(\"${}/mo\", plan.price));\n    if ui.button(\"Choose plan\").clicked() {\n        select(plan);\n    }\n}";
                Artifact::new("Pricing card")
                    .subtitle("egui component · v2")
                    .icon(Icon::Code)
                    .tabs(&["Preview", "Code"])
                    .actions(&[(Icon::Copy, "Copy"), (Icon::Download, "Download"), (Icon::External, "Open")])
                    .id_salt("artifact")
                    .show(ui, |ui, tab| {
                        if tab == 1 {
                            ui.add(CodeBlock::new(code).language("rust"));
                        } else {
                            let card_w = (ui.available_width() - 2.0 * 12.0) / 3.0 - 34.0;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                for (name, price, hi) in [("Hobby", "0", false), ("Pro", "20", true), ("Team", "60", false)] {
                                    egui::Frame::new()
                                        .fill(if hi { glowkit_core::color::with_alpha(t.accent, 0.08) } else { t.card })
                                        .stroke(egui::Stroke::new(1.0, if hi { t.accent } else { t.border }))
                                        .corner_radius(t.corner())
                                        .inner_margin(egui::Margin::same(16))
                                        .show(ui, |ui| {
                                            ui.set_width(card_w);
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing.y = 4.0;
                                                ui.label(egui::RichText::new(name).size(13.0).color(t.muted_foreground));
                                                ui.label(egui::RichText::new(format!("${price}/mo")).size(24.0).color(t.foreground));
                                                ui.add_space(6.0);
                                                ui.label(egui::RichText::new("Unlimited chats").size(12.0).color(t.muted_foreground));
                                            });
                                        });
                                }
                            });
                        }
                    });
            }),
        },
        Demo {
            id: "ai-image-gen",
            name: "Image Generation",
            category: C,
            description: "A shimmering, slowly shifting gradient placeholder with progress that resolves into the generated image.",
            covers: &["Vercel AI Elements: Image", "prompt-kit: Image (loading)", "Kibo UI: AI Image"],
            code: r##"ui.add(ImageGen::new(progress).size(vec2(320.0, 240.0)));
// when done, pass your texture: .texture(tex_id)"##,
            show: |ui| {
                ui.add_space(10.0);
                let e = looped(ui, 8.0);
                let p = (e / 5.0).min(1.0);
                let t = Theme::of(ui);
                ui.columns(3, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.add(ImageGen::new(p).size(vec2(260.0, 260.0)).seed(3).background(t.card));
                        ui.add_space(8.0);
                        label(ui, "Looping: resolves at 100%");
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.add(ImageGen::new(0.35).size(vec2(260.0, 260.0)).seed(9).label("Upscaling").background(t.card).colors([t.chart[1], t.chart[5], t.chart[0]]));
                        ui.add_space(8.0);
                        label(ui, "Generating");
                    });
                    cols[2].vertical_centered(|ui| {
                        ui.add(ImageGen::new(1.0).size(vec2(260.0, 260.0)).seed(5).background(t.card));
                        ui.add_space(8.0);
                        label(ui, "Done");
                    });
                });
            },
        },
        Demo {
            id: "ai-workflow",
            name: "Agent Workflow",
            category: C,
            description: "A node graph for agent pipelines: trigger, LLM, tool and output nodes joined by bezier edges with data flowing along them.",
            covers: &["simple-ai: Workflow", "Vercel AI Elements: Canvas", "Vercel AI Elements: Node", "Vercel AI Elements: Edge", "React Flow: AI workflow"],
            code: r##"ui.add(Workflow::new(&[
    WorkflowNode::new(NodeKind::Trigger, "Chat message", "webhook", 0.0, 0.5),
    WorkflowNode::new(NodeKind::Llm, "Classify", "haiku", 0.5, 0.5),
    WorkflowNode::new(NodeKind::Output, "Reply", "stream", 1.0, 0.5),
], &[WorkflowEdge::new(0, 1), WorkflowEdge::new(1, 2)]));"##,
            show: |ui| {
                let e = looped(ui, 8.0);
                let st = |i: usize| {
                    let s = (e / 1.6) as usize;
                    if s > i {
                        NodeStatus::Done
                    } else if s == i {
                        NodeStatus::Running
                    } else {
                        NodeStatus::Idle
                    }
                };
                let nodes = [
                    WorkflowNode::new(NodeKind::Trigger, "New message", "webhook", 0.0, 0.5).status(st(0)),
                    WorkflowNode::new(NodeKind::Llm, "Classify intent", "haiku", 0.36, 0.5).status(st(1)),
                    WorkflowNode::new(NodeKind::Tool, "Search docs", "vector db", 0.7, 0.08).status(st(2)),
                    WorkflowNode::new(NodeKind::Tool, "Create ticket", "linear", 0.7, 0.92).status(st(2)),
                    WorkflowNode::new(NodeKind::Output, "Reply", "stream", 1.0, 0.5).status(st(3)),
                ];
                let edges = [WorkflowEdge::new(0, 1), WorkflowEdge::new(1, 2), WorkflowEdge::new(1, 3).active(false), WorkflowEdge::new(2, 4), WorkflowEdge::new(3, 4).active(false)];
                ui.add(Workflow::new(&nodes, &edges).height(360.0));
            },
        },
        Demo {
            id: "ai-model-selector",
            name: "Model Selector & Checkpoint",
            category: C,
            description: "A model picker pill with provider badges and a dropdown, plus a \"Restore checkpoint\" conversation divider.",
            covers: &["Vercel AI Elements: Model Selector", "Vercel AI Elements: Checkpoint", "prompt-kit: Model Selector", "assistant-ui: Model Picker", "ElevenLabs UI: Voice Picker"],
            code: r##"ui.add(ModelSelector::new(&models, &mut selected));
if ui.add(Checkpoint::new().time("2 min ago")).clicked() { restore(); }"##,
            show: |ui| column(ui, 640.0, |ui| {
                let id = ui.scope_id().with("models");
                let mut sel: usize = glowkit_core::state(ui, id, || 0);
                let models = [
                    ModelInfo::new("Nova 2", "Lumen Labs").description("Best for coding and agents").tag("New"),
                    ModelInfo::new("Nova Ultra", "Lumen Labs").description("Most capable for hard problems"),
                    ModelInfo::new("Orbit 5", "Orbit AI").description("Flagship reasoning model"),
                    ModelInfo::new("Orbit Mini", "Orbit AI").description("Fast and affordable").tag("Fast"),
                    ModelInfo::new("Helix Pro", "Helix").description("Long context, multimodal"),
                ];
                ui.add_space(10.0);
                ui.add(ModelSelector::new(&models, &mut sel));
                glowkit_core::set_state(ui, id, sel);
                ui.add_space(170.0);
                ui.add(ChatMessage::new(Role::User, "Let's try a different approach.").id_salt("cp_u"));
                ui.add_space(8.0);
                if ui.add(Checkpoint::new().time("2 min ago")).clicked() {
                    glowkit_core::set_state(ui, id, 0usize);
                }
            }),
        },
        Demo {
            id: "ai-web-preview",
            name: "Web Preview",
            category: C,
            description: "A browser-like frame for generated web apps: navigation buttons, an editable address bar and a content panel.",
            covers: &["Vercel AI Elements: Web Preview", "v0: Preview panel"],
            code: r##"WebPreview::new(&mut url).height(280.0).show(ui, |ui| {
    // paint the page
});"##,
            show: |ui| column(ui, 760.0, |ui| {
                let t = Theme::of(ui);
                let id = ui.scope_id().with("webprev");
                let mut url: String = glowkit_core::state(ui, id, || "https://acme-landing.preview.app/".to_owned());
                let out = WebPreview::new(&mut url).height(300.0).loading(looped(ui, 6.0) < 1.2).show(ui, |ui| {
                    let r = ui.max_rect();
                    let p = ui.painter();
                    glowkit_core::paint::gradient_rect(p, egui::Rect::from_min_size(r.min, vec2(r.width(), 150.0)), 0.0, 120.0, &[glowkit_core::color::with_alpha(t.gradient[0], 0.25), glowkit_core::color::with_alpha(t.gradient[1], 0.12)]);
                    p.text(r.min + vec2(32.0, 48.0), egui::Align2::LEFT_TOP, "Ship faster with Acme", egui::FontId::proportional(26.0), t.foreground);
                    p.text(r.min + vec2(32.0, 86.0), egui::Align2::LEFT_TOP, "The landing page your AI just built.", egui::FontId::proportional(14.0), t.muted_foreground);
                    let b = egui::Rect::from_min_size(r.min + vec2(32.0, 116.0), vec2(118.0, 34.0));
                    p.rect_filled(b, egui::CornerRadius::same(8), t.primary);
                    p.text(b.center(), egui::Align2::CENTER_CENTER, "Get started", egui::FontId::proportional(13.0), t.primary_foreground);
                    for i in 0..3 {
                        let c = egui::Rect::from_min_size(r.min + vec2(32.0 + i as f32 * ((r.width() - 64.0) / 3.0), 180.0), vec2((r.width() - 64.0) / 3.0 - 12.0, 90.0));
                        p.rect_filled(c, egui::CornerRadius::same(10), crate::common::raised(&t));
                        p.rect_stroke(c, egui::CornerRadius::same(10), egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                        p.rect_filled(egui::Rect::from_min_size(c.min + vec2(14.0, 16.0), vec2(c.width() * 0.5, 10.0)), egui::CornerRadius::same(4), glowkit_core::color::with_alpha(t.muted_foreground, 0.35));
                        p.rect_filled(egui::Rect::from_min_size(c.min + vec2(14.0, 36.0), vec2(c.width() * 0.75, 8.0)), egui::CornerRadius::same(4), glowkit_core::color::with_alpha(t.muted_foreground, 0.18));
                        p.rect_filled(egui::Rect::from_min_size(c.min + vec2(14.0, 52.0), vec2(c.width() * 0.6, 8.0)), egui::CornerRadius::same(4), glowkit_core::color::with_alpha(t.muted_foreground, 0.18));
                    }
                });
                let _ = out;
                glowkit_core::set_state(ui, id, url);
            }),
        },
    ]
}
