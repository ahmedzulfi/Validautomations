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
        Demo {
            id: "ai-confirmation",
            name: "Confirmation",
            category: C,
            description: "A tool-approval card: the agent pauses and shows exactly what it wants to run, with Deny / Approve and compact resolved states.",
            covers: &["Vercel AI Elements: Confirmation", "assistant-ui: Human-in-the-loop approval", "Vercel AI SDK: Tool approval"],
            code: r##"ui.add(Confirmation::new(&mut state, "Run shell command?")
    .tool("bash")
    .detail("Directory", "~/projects/storefront")
    .code("rm -rf node_modules && npm install"));"##,
            show: |ui| {
                ui.add_space(6.0);
                let id = ui.scope_id().with("confirm_demo");
                let mut st: [ApprovalState; 3] = glowkit_core::state(ui, id, || [ApprovalState::Pending, ApprovalState::Approved, ApprovalState::Denied]);
                let w = ui.available_width();
                ui.horizontal_top(|ui| {
                    ui.add_space(((w - 860.0) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 20.0;
                    ui.allocate_ui_with_layout(vec2(440.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(440.0);
                        ui.add(Confirmation::new(&mut st[0], "Run shell command?")
                            .tool("bash")
                            .description("The agent wants to reinstall dependencies.")
                            .detail("Directory", "~/projects/storefront")
                            .detail("Network", "registry.npmjs.org")
                            .code("rm -rf node_modules && npm install"));
                    });
                    ui.allocate_ui_with_layout(vec2(400.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(400.0);
                        ui.add(Confirmation::new(&mut st[1], "Write src/config.rs")
                            .tool("write_file")
                            .icon(Icon::Pencil)
                            .detail("Changes", "+12  −3 lines")
                            .resolved_texts("Approved: file written (12 lines)", "Denied"));
                        ui.add_space(12.0);
                        ui.add(Confirmation::new(&mut st[2], "Email 214 customers")
                            .tool("send_email")
                            .detail("Subject", "Your order has shipped")
                            .resolved_texts("Approved", "Denied: the agent will draft it instead"));
                    });
                });
                glowkit_core::set_state(ui, id, st);
            },
        },
        Demo {
            id: "ai-open-in-chat",
            name: "Open In Chat",
            category: C,
            description: "A dropdown button that hands the current prompt to another chat app, with provider badges, hints and a copy row.",
            covers: &["Vercel AI Elements: Open In Chat"],
            code: r##"let providers = [ChatProvider::new("Lumen Chat", "lumen.chat"), ChatProvider::new("Orbit", "orbit.ai")];
if let Some(i) = OpenInChat::new(prompt, &providers).show(ui) {
    // open providers[i] with the prompt
}"##,
            show: |ui| column(ui, 640.0, |ui| {
                let t = Theme::of(ui);
                let prompt = "Write a launch announcement for our new Rust UI library, upbeat but technical.";
                ui.add_space(6.0);
                egui::Frame::new().fill(t.card).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(t.corner()).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new("PROMPT").size(10.5).color(t.muted_foreground).strong());
                    ui.add_space(2.0);
                    ui.add(egui::Label::new(egui::RichText::new(prompt).size(14.0).color(t.foreground)).wrap());
                    ui.add_space(8.0);
                    let providers = [
                        ChatProvider::new("Lumen Chat", "lumen.chat · Nova models"),
                        ChatProvider::new("Orbit", "orbit.ai · Orbit 5"),
                        ChatProvider::new("Helix Assistant", "helix.dev · long context"),
                        ChatProvider::new("Quill", "quill.app · writing"),
                    ];
                    let e = glowkit_core::Clock::elapsed(ui, ui.scope_id().with("oic_clock"));
                    let force = if e < 6.0 { Some(true) } else { None };
                    if e < 6.1 {
                        glowkit_core::animate(ui);
                    }
                    ui.horizontal(|ui| {
                        let _ = OpenInChat::new(prompt, &providers).force_open(force).id_salt("oic").show(ui);
                        label(ui, "  hand the prompt to another assistant");
                    });
                });
                ui.add_space(130.0);
            }),
        },
        Demo {
            id: "ai-toolbar",
            name: "Toolbar",
            category: C,
            description: "A floating icon toolbar for canvases and nodes, with dividers, toggled states, and quick tooltips with shortcut hints.",
            covers: &["Vercel AI Elements: Toolbar", "React Flow: NodeToolbar"],
            code: r##"let items = [
    ToolbarItem::new(Icon::Sparkle, "Ask AI").shortcut("⌘K"),
    ToolbarItem::new(Icon::Pencil, "Edit"),
    ToolbarItem::new(Icon::Copy, "Duplicate").shortcut("⌘D"),
];
if let Some(i) = AiToolbar::new(&items).show_at(ui, node_rect.center_top() - vec2(0.0, 10.0)) {
    // handle items[i]
}"##,
            show: |ui| {
                let t = Theme::of(ui);
                let w = ui.available_width().min(860.0);
                let pad = (ui.available_width() - w) / 2.0;
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), egui::Sense::hover());
                let rect = egui::Rect::from_min_size(rect.min + vec2(pad, 0.0), vec2(w, 360.0));
                let p = ui.painter();
                p.rect_filled(rect, t.corner(), crate::common::raised(&t));
                p.rect_stroke(rect, t.corner(), egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                let dot = glowkit_core::color::with_alpha(t.muted_foreground, 0.22);
                let mut y = rect.min.y + 12.0;
                while y < rect.max.y {
                    let mut x = rect.min.x + 12.0;
                    while x < rect.max.x {
                        p.circle_filled(egui::pos2(x, y), 1.0, dot);
                        x += 20.0;
                    }
                    y += 20.0;
                }
                let node = |p: &egui::Painter, r: egui::Rect, title: &str, sub: &str, sel: bool, icon: Icon| {
                    glowkit_core::paint::soft_shadow(p, r, 10.0, 12.0, vec2(0.0, 4.0), egui::Color32::from_black_alpha(if t.dark { 70 } else { 18 }));
                    p.rect(r, egui::CornerRadius::same(10), t.card, egui::Stroke::new(if sel { 1.5 } else { 1.0 }, if sel { t.accent } else { t.border }), egui::StrokeKind::Inside);
                    let ib = egui::Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(28.0, 28.0));
                    p.rect_filled(ib, egui::CornerRadius::same(7), glowkit_core::color::with_alpha(t.accent, 0.14));
                    paint_icon(p, icon, ib.shrink(7.0), t.accent);
                    p.text(egui::pos2(ib.max.x + 10.0, ib.min.y + 7.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(13.5), t.foreground);
                    p.text(egui::pos2(ib.max.x + 10.0, ib.min.y + 23.0), egui::Align2::LEFT_CENTER, sub, egui::FontId::proportional(11.5), t.muted_foreground);
                };
                let n1 = egui::Rect::from_min_size(rect.min + vec2(w * 0.08, 200.0), vec2(190.0, 54.0));
                let n2 = egui::Rect::from_min_size(rect.min + vec2(w * 0.42, 150.0), vec2(210.0, 54.0));
                let n3 = egui::Rect::from_min_size(rect.min + vec2(w * 0.74, 230.0), vec2(180.0, 54.0));
                for (a, b) in [(n1, n2), (n2, n3)] {
                    let pts = glowkit_core::paint::cubic_bezier(a.right_center(), a.right_center() + vec2(60.0, 0.0), b.left_center() - vec2(60.0, 0.0), b.left_center(), 32);
                    p.add(egui::Shape::line(pts, egui::Stroke::new(1.5, glowkit_core::color::with_alpha(t.muted_foreground, 0.5))));
                }
                node(p, n1, "User prompt", "input", false, Icon::User);
                node(p, n2, "Generate draft", "Nova 2 · 0.7 temp", true, Icon::Sparkle);
                node(p, n3, "Publish", "webhook", false, Icon::Globe);
                let e = looped(ui, 10.0);
                let id = ui.scope_id().with("tb_demo");
                let mut locked: bool = glowkit_core::state(ui, id, || false);
                let items = [
                    ToolbarItem::new(Icon::Sparkle, "Ask AI").shortcut("⌘K"),
                    ToolbarItem::new(Icon::Pencil, "Edit").shortcut("E"),
                    ToolbarItem::new(Icon::Copy, "Duplicate").shortcut("⌘D"),
                    ToolbarItem::new(Icon::Play, "Run node").separator(),
                    ToolbarItem::new(Icon::Lock, "Lock position").active(locked),
                    ToolbarItem::new(Icon::X, "Delete").shortcut("⌫").separator().color(t.destructive),
                ];
                let hovering_toolbar = ui.rect_contains_pointer(rect);
                let tip = if hovering_toolbar { None } else { Some(((e / 1.6) as usize) % items.len()) };
                if AiToolbar::new(&items).force_tooltip(tip).id_salt("node_tb").show_at(ui, n2.center_top() - vec2(0.0, 10.0)) == Some(4) {
                    locked = !locked;
                }
                glowkit_core::set_state(ui, id, locked);
                let side = [
                    ToolbarItem::new(Icon::ArrowUp, "Select").active(true),
                    ToolbarItem::new(Icon::Plus, "Add node").shortcut("N"),
                    ToolbarItem::new(Icon::Code, "Code node"),
                    ToolbarItem::new(Icon::Search, "Find").separator().shortcut("⌘F"),
                ];
                let _ = AiToolbar::new(&side).vertical(true).button_size(30.0).id_salt("side_tb").show_at(ui, egui::pos2(rect.min.x + 14.0, rect.min.y + 90.0));
            },
        },
        Demo {
            id: "ai-jsx-preview",
            name: "JSX Preview",
            category: C,
            description: "Generative UI: streamed JSX-like markup is parsed on every frame (half-written tags and all) and rendered into real themed widgets that fade in.",
            covers: &["prompt-kit: JSX Preview", "Vercel AI Elements: JSX Preview", "Vercel AI SDK: Generative UI"],
            code: r##"// `markup` grows as the model streams
ui.add(JsxPreview::new(&markup));

// or just parse it
let tree = parse_markup("<Card><Title>Hi</Title><Button>Go");"##,
            show: |ui| {
                const SRC: &str = r#"<Card>
  <Row>
    <Avatar name="Ada Lovelace" />
    <Stack>
      <Title>Pro plan</Title>
      <Description>Billed monthly · renews Dec 1</Description>
    </Stack>
  </Row>
  <Separator />
  <Row>
    <Stat label="Tokens used" value="1.2M" delta="+12%" />
    <Stat label="Spend" value="$48.20" delta="-3%" />
  </Row>
  <Progress value="68" />
  <Switch checked>Auto top-up</Switch>
  <Row>
    <Badge variant="success">Active</Badge>
    <Badge>Priority</Badge>
  </Row>
  <Row>
    <Button>Upgrade</Button>
    <Button variant="outline">Manage</Button>
  </Row>
</Card>"#;
                let e = looped(ui, 14.0);
                let n = ((e / 8.0).min(1.0) * SRC.len() as f32) as usize;
                let mut cut = n.min(SRC.len());
                while !SRC.is_char_boundary(cut) {
                    cut -= 1;
                }
                let partial = &SRC[..cut];
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        label(ui, if cut < SRC.len() { "STREAMING MARKUP…" } else { "MARKUP" });
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().id_salt("jsx_src").max_height(330.0).stick_to_bottom(true).show(ui, |ui| {
                            ui.add(CodeBlock::new(if partial.is_empty() { " " } else { partial }).language("jsx").header(false).size(11.5));
                        });
                    });
                    cols[1].vertical(|ui| {
                        label(ui, "RENDERED");
                        ui.add_space(4.0);
                        ui.set_max_width(400.0);
                        ui.add(JsxPreview::new(partial).id_salt(("jsx", (glowkit_core::time(ui) / 14.0) as u32)));
                    });
                });
            },
        },
        Demo {
            id: "ai-audio-player",
            name: "Audio Player",
            category: C,
            description: "An audio player card with a scrubbable waveform, play / pause, elapsed time and a speed pill, plus a compact scrub-bar variant.",
            covers: &["ElevenLabs UI: Audio Player"],
            code: r##"let mut state = AudioPlayerState::new(184.0);
ui.add(AudioPlayer::new(&mut state)
    .title("Welcome message")
    .subtitle("Aria · Narration"));"##,
            show: |ui| column(ui, 620.0, |ui| {
                let id = ui.scope_id().with("player_demo");
                let mut st: [AudioPlayerState; 2] = glowkit_core::state(ui, id, || {
                    let mut a = AudioPlayerState::new(184.0);
                    a.position = 61.0;
                    a.playing = true;
                    let mut b = AudioPlayerState::new(312.0);
                    b.position = 128.0;
                    b.playing = true;
                    b.speed = 1.5;
                    [a, b]
                });
                let t = Theme::of(ui);
                ui.add(AudioPlayer::new(&mut st[0]).title("Welcome to Lumen Labs").subtitle("Aria · Narration · 44.1 kHz").seed(4).looping(true));
                ui.add_space(14.0);
                ui.add(AudioPlayer::new(&mut st[1]).title("Episode 12: Shipping Rust to the web").subtitle("The Render Loop podcast").waveform(false).color(t.chart[1]).looping(true));
                glowkit_core::set_state(ui, id, st);
            }),
        },
        Demo {
            id: "ai-dot-matrix",
            name: "Dot Matrix",
            category: C,
            description: "An LED dot-matrix display with glowing dots and phosphor afterglow: scrolling pixel text, live level bars and animated patterns.",
            covers: &["ElevenLabs UI: Matrix"],
            code: r##"ui.add(DotMatrix::new(64, 9).content(MatrixContent::Text("HELLO WORLD")).gradient(true));
ui.add(DotMatrix::new(24, 10).content(MatrixContent::Levels(&levels)));
ui.add(DotMatrix::new(15, 15).content(MatrixContent::Pattern(MatrixPattern::Ripple)));"##,
            show: |ui| column(ui, 760.0, |ui| {
                let time = glowkit_core::time(ui);
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.add(DotMatrix::new(76, 9).pitch(9.5).content(MatrixContent::Text("GLOWKIT * VOICE AGENT ONLINE * 24 KHZ")).gradient(true).id_salt("dm_text"));
                });
                ui.add_space(14.0);
                let lvl = simulated_level(ui, 2.3);
                let levels: Vec<f32> = (0..12)
                    .map(|i| {
                        let x = i as f32 / 11.0;
                        let n = glowkit_core::noise::noise3(x * 3.0, time * 2.4, 1.0) * 0.5 + 0.5;
                        (n * (0.55 + lvl * 1.1) * (1.15 - x * 0.35)).clamp(0.08, 1.0)
                    })
                    .collect();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    let t = Theme::of(ui);
                    ui.vertical(|ui| {
                        ui.add(DotMatrix::new(24, 12).pitch(8.5).content(MatrixContent::Levels(&levels)).color(t.chart[4]).persistence(0.25).id_salt("dm_lv"));
                        label(ui, "Levels");
                    });
                    for (i, (pat, name)) in [(MatrixPattern::Ripple, "Ripple"), (MatrixPattern::Loader, "Loader"), (MatrixPattern::Rain, "Rain"), (MatrixPattern::Sparkle, "Sparkle")].into_iter().enumerate() {
                        ui.vertical(|ui| {
                            let c = [t.accent, t.chart[1], t.chart[4], t.chart[3]][i];
                            ui.add(DotMatrix::new(13, 13).pitch(8.5).content(MatrixContent::Pattern(pat)).color(c).persistence(if i == 2 { 0.3 } else { 0.12 }).id_salt(("dm_p", i)));
                            label(ui, name);
                        });
                    }
                });
            }),
        },
        Demo {
            id: "ai-mic-selector",
            name: "Mic Selector",
            category: C,
            description: "A microphone picker with a live level meter, a device dropdown with a full segmented meter, and a mute toggle.",
            covers: &["ElevenLabs UI: Mic Selector"],
            code: r##"let devices = ["MacBook Pro Microphone", "Studio USB Mic", "AirPods Pro"];
ui.add(MicSelector::new(&devices, &mut selected, &mut muted).level(mic_level));"##,
            show: |ui| column(ui, 700.0, |ui| {
                let id = ui.scope_id().with("mic_demo");
                let (mut sel, mut muted, mut sel2, mut muted2): (usize, bool, usize, bool) = glowkit_core::state(ui, id, || (1, false, 0, true));
                let devices = ["MacBook Pro Microphone", "Studio USB Condenser", "AirPods Pro", "Webcam C920"];
                let lvl = simulated_level(ui, 0.8);
                glowkit_core::animate(ui);
                let e = glowkit_core::Clock::elapsed(ui, id.with("clock"));
                ui.add_space(6.0);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 40.0;
                    ui.vertical(|ui| {
                        label(ui, "LIVE");
                        ui.add(MicSelector::new(&devices, &mut sel, &mut muted).level(lvl).force_open(if e < 6.0 { Some(true) } else { None }).id_salt("mic1"));
                    });
                    ui.vertical(|ui| {
                        label(ui, "MUTED");
                        ui.add(MicSelector::new(&devices, &mut sel2, &mut muted2).level(lvl).width(250.0).id_salt("mic2"));
                    });
                });
                glowkit_core::set_state(ui, id, (sel, muted, sel2, muted2));
            }),
        },
        Demo {
            id: "ai-speech-input",
            name: "Speech Input",
            category: C,
            description: "A dictate button that springs open into a live waveform with a timer and streaming interim transcript, then shows the final text.",
            covers: &["ElevenLabs UI: Speech Input", "Vercel AI Elements: Speech Input"],
            code: r##"ui.add(SpeechInput::new(&mut phase)
    .level(mic_level)
    .transcript(&interim_text));
// when the recognizer finishes: phase = SpeechPhase::Done"##,
            show: |ui| column(ui, 600.0, |ui| {
                const TEXT: &str = "Book a table for four at the Italian place near the office, Friday around seven thirty, and send the invite to the design team.";
                let id = ui.scope_id().with("speech_demo");
                let (mut phase, mut auto, mut t0): (SpeechPhase, bool, f64) = glowkit_core::state(ui, id, || (SpeechPhase::Idle, true, -1.0));
                let now = ui.input(|i| i.time);
                if t0 < 0.0 {
                    t0 = now - 2.6; // start mid-recording so previews show the live state
                }
                let e = (now - t0) as f32;
                let lvl = simulated_level(ui, 4.1);
                glowkit_core::animate(ui);
                if auto {
                    let cyc = e % 14.0;
                    phase = if cyc < 1.2 {
                        SpeechPhase::Idle
                    } else if cyc < 8.0 {
                        SpeechPhase::Recording
                    } else if cyc < 9.6 {
                        SpeechPhase::Processing
                    } else {
                        SpeechPhase::Done
                    };
                }
                let words: Vec<&str> = TEXT.split(' ').collect();
                let rec_e = if auto { ((e % 14.0) - 1.2).max(0.0) } else { 6.0 };
                let shown = if phase == SpeechPhase::Recording && auto {
                    let k = ((rec_e / 6.4) * words.len() as f32) as usize;
                    words[..k.min(words.len())].join(" ")
                } else {
                    TEXT.to_owned()
                };
                ui.add_space(10.0);
                let before = phase;
                let mic = if phase == SpeechPhase::Recording { lvl } else { 0.0 };
                ui.add(SpeechInput::new(&mut phase).level(mic).transcript(&shown).id_salt("speech"));
                if phase != before {
                    auto = false;
                }
                if !auto && phase == SpeechPhase::Processing {
                    phase = SpeechPhase::Done;
                }
                ui.add_space(14.0);
                label(ui, if auto { "Autoplaying: click the control to take over" } else { "Click to record, click stop to finish" });
                glowkit_core::set_state(ui, id, (phase, auto, t0));
            }),
        },
        Demo {
            id: "ai-scrub-bar",
            name: "Scrub Bar",
            category: C,
            description: "A media timeline scrubber with buffered and played ranges, a hover time tooltip, chapter markers and a thumb that grows on hover.",
            covers: &["ElevenLabs UI: Scrub Bar", "Media Chrome: Time Range", "Vidstack: Time Slider"],
            code: r##"let markers = [ScrubMarker::new(42.0, "Intro"), ScrubMarker::new(118.0, "Demo")];
ui.add(ScrubBar::new(&mut position, duration)
    .buffered(position + 30.0)
    .markers(&markers)
    .show_times(true));"##,
            show: |ui| column(ui, 700.0, |ui| {
                let t = Theme::of(ui);
                let id = ui.scope_id().with("scrub_demo");
                let (mut pos, mut pos2): (f32, f32) = glowkit_core::state(ui, id, || (74.0, 40.0));
                let dt = glowkit_core::dt(ui);
                glowkit_core::animate(ui);
                pos = (pos + dt) % 240.0;
                // "video" frame
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 200.0), egui::Sense::hover());
                let time = glowkit_core::time(ui);
                glowkit_core::paint::gradient_rect(ui.painter(), r, t.radius, 120.0 + (time * 0.2).sin() * 15.0, &[glowkit_core::hex("#1E1B4B"), glowkit_core::hex("#7C3AED"), glowkit_core::hex("#DB2777")]);
                glowkit_core::paint::glow(ui.painter(), r.center() + vec2((time * 0.5).sin() * 120.0, 10.0), 120.0, glowkit_core::color::with_alpha(glowkit_core::hex("#F59E0B"), 0.35));
                ui.painter().text(r.left_bottom() + vec2(18.0, -20.0), egui::Align2::LEFT_BOTTOM, "Keynote: building voice agents", egui::FontId::proportional(16.0), egui::Color32::WHITE);
                ui.add_space(6.0);
                let markers = [
                    ScrubMarker::new(30.0, "Intro"),
                    ScrubMarker::new(96.0, "Live demo").color(t.chart[1]),
                    ScrubMarker::new(170.0, "Q&A").color(t.chart[3]),
                ];
                let buf = (pos + 45.0).min(240.0);
                ui.add(ScrubBar::new(&mut pos, 240.0).buffered(buf).markers(&markers).show_times(true));
                ui.add_space(22.0);
                label(ui, "SEGMENTED BUFFER · CUSTOM COLOR");
                ui.add(ScrubBar::new(&mut pos2, 180.0).buffered_ranges(&[(0.0, 62.0), (90.0, 128.0)]).color(t.chart[4]).thickness(6.0));
                glowkit_core::set_state(ui, id, (pos, pos2));
            }),
        },
        Demo {
            id: "ai-transcript-viewer",
            name: "Transcript Viewer",
            category: C,
            description: "A transcript synced to playback: the current word rides an accent pill that glides along, spoken words brighten, and clicking any word seeks.",
            covers: &["ElevenLabs UI: Transcript Viewer"],
            code: r##"let words = TranscriptWord::timeline(&[("Agent", "Hi! How can I help?")], 0.0, 2.6);
if let Some(seek) = TranscriptViewer::new(&words, position).height(220.0).show(ui) {
    position = seek;
}"##,
            show: |ui| column(ui, 680.0, |ui| {
                let t = Theme::of(ui);
                let words = TranscriptWord::timeline(
                    &[
                        ("Agent", "Hi, thanks for calling Lumen support. How can I help you today?"),
                        ("Maya", "Hey! My order shipped last week, but the tracking page hasn't updated since Tuesday."),
                        ("Agent", "Sorry about that. I can see it left our warehouse on Monday and it's now with the regional carrier. It should arrive by Friday."),
                        ("Maya", "Great, could you text me when it's out for delivery?"),
                        ("Agent", "Of course. I've turned on SMS updates for this order."),
                    ],
                    0.4,
                    2.7,
                );
                let dur = words.last().map(|w| w.end + 1.0).unwrap_or(1.0);
                let id = ui.scope_id().with("transcript_demo");
                let (mut pos, mut playing): (f32, bool) = glowkit_core::state(ui, id, || (6.5, true));
                if playing {
                    pos += glowkit_core::dt(ui);
                    if pos > dur {
                        pos = 0.0;
                    }
                    glowkit_core::animate(ui);
                }
                egui::Frame::new().fill(t.card).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(t.corner_scaled(1.2)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if crate::media::play_button(ui, playing, 36.0, t.accent).clicked() {
                            playing = !playing;
                        }
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.add_space(6.0);
                            ui.add(ScrubBar::new(&mut pos, dur).buffered(dur).show_times(true));
                        });
                    });
                    ui.add_space(6.0);
                    if let Some(s) = TranscriptViewer::new(&words, pos).height(230.0).id_salt("tv").show(ui) {
                        pos = s;
                    }
                });
                glowkit_core::set_state(ui, id, (pos, playing));
            }),
        },
    ]
}
