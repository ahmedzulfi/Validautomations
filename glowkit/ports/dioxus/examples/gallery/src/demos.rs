//! One live demo (and a smaller thumbnail) per component.

use dioxus::prelude::*;
use glowkit_dioxus::prelude::*;

pub fn demo(id: &str) -> Element {
    match id {
        "shiny-text" => rsx! {
            div { class: "g-col",
                ShinyText { text: "Just some shiny text!", size: 44.0 }
                div { class: "g-row-sm",
                    "✨ "
                    ShinyText { text: "Introducing glowkit for Dioxus", size: 16.0, speed: 2.5 }
                    " →"
                }
            }
        },
        "gradient-text" => rsx! {
            div { class: "g-col",
                GradientText { text: "Gradient text", size: 56.0, bold: true }
                GradientText { text: "Add a splash of color!", size: 22.0, border: true, colors: vec!["#40ffaa".into(), "#4079ff".into(), "#ff40d9".into()], speed: 4.0 }
            }
        },
        "blur-text" => rsx! {
            div { class: "g-col",
                BlurText { text: "Isn't this so cool?!", size: 46.0, trigger: Trigger::Loop { pause_ms: 1800 } }
                BlurText { text: "Letter by letter, from below", size: 22.0, split_by: SplitBy::Letters, direction: Direction::Bottom, delay: 0.03, weight: 500, trigger: Trigger::Loop { pause_ms: 2200 } }
            }
        },
        "typewriter" => rsx! {
            div { class: "g-col",
                TypewriterText { strings: vec!["Build fast.".into(), "Ship often.".into(), "Sleep well.".into()], prefix: "We help you ", size: 40.0 }
                TypewriterText { strings: vec!["cargo add glowkit-dioxus".into(), "dx serve --platform web".into()], prefix: "$ ", size: 18.0, monospace: true, bold: false, cursor: CursorStyle::Block, colors: vec!["var(--gk-accent)".into(), "var(--gk-success)".into()] }
            }
        },
        "count-up" => rsx! { CountUpDemo {} },
        "aurora" => rsx! {
            Aurora { class: "g-bg-demo",
                div { class: "g-bg-center",
                    div { class: "g-bg-title", "Aurora" }
                    div { class: "g-bg-sub", "Curtains of light, in pure Rust" }
                }
            }
        },
        "dot-grid" => rsx! {
            DotGrid { class: "g-bg-demo", auto_hover: true,
                div { class: "g-bg-center",
                    div { class: "g-bg-title", "Dot Grid" }
                    div { class: "g-bg-sub", "Move the pointer · click for a shockwave" }
                }
            }
        },
        "shimmer-button" => rsx! {
            div { class: "g-col",
                div { class: "g-row",
                    ShimmerButton { label: "Shimmer Button" }
                    ShimmerButton { label: "Get started", background: "#5b21b6", shimmer_color: "#e9d5ff", " →" }
                    ShimmerButton { label: "Square corners", radius: 8.0, shimmer_color: "#22d3ee", duration: 2.2 }
                }
                ShimmerButton { label: "Large shimmer", size: 20.0 }
            }
        },
        "rainbow-button" => rsx! {
            div { class: "g-col",
                RainbowButton { label: "Get Unlimited Access" }
                div { class: "g-row",
                    RainbowButton { label: "Neon", colors: vec!["#40ffaa".into(), "#4079ff".into(), "#ff40d9".into()], speed: 3.0 }
                    RainbowButton { label: "Pill", radius: 999.0, size: 14.0 }
                }
            }
        },
        "magnetic-button" => rsx! {
            div { class: "g-col",
                div { class: "g-row", style: "gap: 64px;",
                    MagneticButton { label: "Hover me" }
                    MagneticButton { label: "Strong pull", strength: 0.6, range: 90.0, background: "var(--gk-accent)", text_color: "var(--gk-accent-foreground)" }
                }
                MagneticButton { label: "Ambient drift", ambient: true, strength: 0.25 }
            }
        },
        "border-beam" => rsx! {
            div { class: "g-row", style: "align-items: stretch;",
                BorderBeam { width: 320.0,
                    LoginCard {}
                }
                BorderBeam { width: 240.0, duration: 4.0, from: "#22d3ee", to: "#a855f7", length: 0.3, thickness: 2.0, reverse: true,
                    div { class: "g-card-title", "Reverse beam" }
                    p { class: "g-card-copy", "Custom colors, longer tail, counter-clockwise." }
                }
            }
        },
        "spotlight-card" => rsx! {
            div { class: "g-row", style: "align-items: stretch;",
                SpotlightCard { width: 300.0, min_height: 200.0,
                    div { class: "g-card-icon", "◎" }
                    div { class: "g-card-title", "Spotlight" }
                    p { class: "g-card-copy", "Move your pointer across the card: the light follows it." }
                }
                SpotlightCard { width: 300.0, min_height: 200.0, dots: true, ambient: true,
                    div { class: "g-card-icon", "⁂" }
                    div { class: "g-card-title", "Dots + ambient" }
                    p { class: "g-card-copy", "The light wanders on its own and reveals a dot grid." }
                }
            }
        },
        "tilt-card" => rsx! {
            div { class: "g-row", style: "gap: 40px;",
                TiltCard { width: 280.0, min_height: 190.0,
                    div { class: "g-card-icon", "⬡" }
                    div { class: "g-card-title", "Tilt me" }
                    p { class: "g-card-copy", "Spring-driven 3D tilt with a glare that follows the pointer." }
                }
                TiltCard { width: 280.0, min_height: 190.0, ambient: true, gradient: ("#7c3aed".to_string(), "#db2777".to_string()), max_tilt: 10.0,
                    div { style: "color: white;",
                        div { class: "g-card-icon", style: "background: rgb(255 255 255 / .18); color: white;", "✦" }
                        div { class: "g-card-title", "Ambient sway" }
                        p { class: "g-card-copy", style: "color: rgb(255 255 255 / .8);", "Gradient fill, sways on its own when idle." }
                    }
                }
            }
        },
        "marquee" => rsx! { MarqueeDemo {} },
        "dock" => rsx! { DockDemo {} },
        "animated-tabs" => rsx! { TabsDemo {} },
        "toast" => rsx! { ToastDemo {} },
        "chat-message" => rsx! { ChatDemo {} },
        "streaming-text" => rsx! {
            div { class: "g-chat", style: "max-width: 560px;",
                StreamingText { text: STREAM_TEXT, tokens_per_second: 16.0, trigger: Trigger::Loop { pause_ms: 1500 } }
                div { style: "height: 18px" }
                StreamingText { text: "Blur style: every token sharpens into place as it arrives.", style: RevealStyle::Blur, tokens_per_second: 8.0, size: 18.0, trigger: Trigger::Loop { pause_ms: 1500 } }
            }
        },
        "prompt-input" => rsx! { PromptDemo {} },
        "approval-card" => rsx! { ApprovalDemo {} },
        "stat-tile" => rsx! { StatDemo {} },
        _ => rsx! { "Unknown component" },
    }
}

pub fn thumb(id: &str) -> Element {
    match id {
        "shiny-text" => rsx! { ShinyText { text: "Shiny text", size: 30.0 } },
        "gradient-text" => rsx! { GradientText { text: "Gradient", size: 34.0, bold: true } },
        "blur-text" => rsx! { BlurText { text: "Blur in", size: 30.0, trigger: Trigger::Loop { pause_ms: 2000 } } },
        "typewriter" => rsx! { TypewriterText { strings: vec!["Typewriter".into(), "Effect".into()], size: 26.0 } },
        "count-up" => rsx! { CountUp { to: 12480.0, prefix: "$", size: 34.0 } },
        "aurora" => rsx! { Aurora { class: "g-thumb-bg" } },
        "dot-grid" => rsx! { DotGrid { class: "g-thumb-bg", gap: 20.0, auto_hover: true } },
        "shimmer-button" => rsx! { ShimmerButton { label: "Shimmer" } },
        "rainbow-button" => rsx! { RainbowButton { label: "Rainbow" } },
        "magnetic-button" => rsx! { MagneticButton { label: "Magnetic", ambient: true, strength: 0.2 } },
        "border-beam" => rsx! { BorderBeam { width: 180.0, padding: 16.0, div { class: "g-card-title", "Border beam" } } },
        "spotlight-card" => rsx! { SpotlightCard { width: 180.0, padding: 16.0, ambient: true, dots: true, div { class: "g-card-title", "Spotlight" } } },
        "tilt-card" => rsx! { TiltCard { width: 170.0, padding: 16.0, ambient: true, div { class: "g-card-title", "Tilt card" } } },
        "marquee" => rsx! {
            div { style: "width: 100%;",
                Marquee { speed: 30.0, fade: 30.0, render: move |_| rsx! { for t in ["Rust", "Dioxus", "egui", "WASM", "CSS"] { span { class: "g-tag", "{t}" } } } }
            }
        },
        "dock" => rsx! { Dock { items: dock_items(), size: 28.0, gap: 6.0, magnification: 1.6, autoplay: true, labels: false } },
        "animated-tabs" => rsx! { TabsThumb {} },
        "toast" => rsx! {
            div { class: "g-toast-static",
                svg { class: "gk-toast-icon", view_box: "0 0 24 24", fill: "none", stroke: "var(--gk-success)", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                    path { d: "M22 11.08V12a10 10 0 1 1-5.93-9.14 M22 4 12 14.01l-3-3" } }
                div { div { class: "gk-toast-title", "Event created" } div { class: "gk-toast-desc", "Sunday at 9:00 AM" } }
            }
        },
        "chat-message" => rsx! {
            div { style: "width: 100%; display: grid; gap: 8px; font-size: 12px;",
                ChatMessage { role: Role::User, content: "What's glowkit?", size: 12.0 }
                ChatMessage { role: Role::Assistant, content: "An animated UI kit, now on **Dioxus**.", size: 12.0 }
            }
        },
        "streaming-text" => rsx! { div { style: "max-width: 220px;", StreamingText { text: "Tokens fade in one by one as the answer streams…", size: 13.0, trigger: Trigger::Loop { pause_ms: 1200 } } } },
        "prompt-input" => rsx! { PromptThumb {} },
        "approval-card" => rsx! {
            div { class: "g-mini-approval",
                span { class: "gk-risk-badge", style: "--gk-risk: var(--gk-warning)", span { class: "gk-risk-dot" } "Medium" }
                code { "pnpm add zod@3" }
                span { class: "gk-btn-solid", style: "height: 26px; font-size: 12px;", "Approve" }
            }
        },
        "stat-tile" => rsx! { StatTile { title: "Revenue", value: 45231.89, prefix: "$", decimals: 2, delta: 20.1, spark: vec![12.0, 18.0, 15.0, 22.0, 28.0, 26.0, 34.0], width: 200.0, height: 120.0 } },
        _ => rsx! {},
    }
}

pub fn snippet(id: &str) -> &'static str {
    match id {
        "shiny-text" => "ShinyText { text: \"Just some shiny text!\", size: 44.0 }",
        "gradient-text" => "GradientText {\n    text: \"Add a splash of color!\",\n    border: true,\n    colors: vec![\"#40ffaa\".into(), \"#4079ff\".into(), \"#ff40d9\".into()],\n}",
        "blur-text" => "BlurText {\n    text: \"Isn't this so cool?!\",\n    split_by: SplitBy::Letters,\n    direction: Direction::Bottom,\n    trigger: Trigger::Loop { pause_ms: 1800 },\n}",
        "typewriter" => "TypewriterText {\n    strings: vec![\"Build fast.\".into(), \"Ship often.\".into()],\n    prefix: \"We help you \",\n    size: 40.0,\n}",
        "count-up" => "CountUp { to: 12_480.0, prefix: \"$\", decimals: 2, size: 56.0 }",
        "aurora" => "Aurora { amplitude: 1.2,\n    h1 { \"Aurora\" }\n}",
        "dot-grid" => "DotGrid { gap: 28.0, proximity: 140.0, auto_hover: true,\n    h1 { \"Dot Grid\" }\n}",
        "shimmer-button" => "ShimmerButton {\n    label: \"Shimmer Button\",\n    onclick: move |_| { /* ... */ },\n}",
        "rainbow-button" => "RainbowButton { label: \"Get Unlimited Access\" }",
        "magnetic-button" => "MagneticButton { label: \"Hover me\", strength: 0.35, range: 60.0 }",
        "border-beam" => "BorderBeam { width: 320.0, duration: 6.0,\n    LoginForm {}\n}",
        "spotlight-card" => "SpotlightCard { dots: true, ambient: true,\n    h3 { \"Spotlight\" }\n}",
        "tilt-card" => "TiltCard { max_tilt: 14.0, glare: true,\n    h3 { \"Tilt me\" }\n}",
        "marquee" => "Marquee { speed: 50.0,\n    render: move |_| rsx! {\n        for n in LOGOS { span { \"{n}\" } }\n    },\n}",
        "dock" => "Dock {\n    items: vec![DockItem::new(\"🏠\", \"Home\"), DockItem::new(\"✉️\", \"Mail\").active(true)],\n    magnification: 1.8,\n    onselect: move |i| { /* open items[i] */ },\n}",
        "animated-tabs" => "let selected = use_signal(|| 0);\nAnimatedTabs {\n    selected,\n    labels: vec![\"Overview\".into(), \"Usage\".into(), \"API\".into()],\n}",
        "toast" => "// wrap the app in ToastProvider {}\nlet toaster = use_toaster();\ntoaster.push(Toast::success(\"Event created\")\n    .description(\"Sunday, December 03 at 9:00 AM\")\n    .action(\"Undo\"));",
        "chat-message" => "ChatMessage { role: Role::User, content: \"Hi!\" }\nChatMessage {\n    role: Role::Assistant,\n    content: \"Rendered from **Markdown**\",\n    actions: true,\n}",
        "streaming-text" => "StreamingText {\n    text: \"Hello! How can I help you today?\",\n    tokens_per_second: 14.0,\n}",
        "prompt-input" => "let text = use_signal(String::new);\nPromptInput {\n    value: text,\n    generating,\n    onsubmit: move |prompt| { /* send */ },\n}",
        "approval-card" => "ApprovalCard {\n    agent: \"Deploy Fixer\", tool: \"shell\",\n    command: \"pnpm add zod@3\",\n    risk: Risk::Medium,\n    ondecide: move |(decision, command)| { /* ... */ },\n}",
        "stat-tile" => "StatTile {\n    title: \"Revenue\", value: 45_231.89, prefix: \"$\",\n    decimals: 2, delta: 20.1, caption: \"vs last month\",\n    spark: vec![12.0, 18.0, 15.0, 22.0, 28.0, 26.0, 34.0],\n}",
        _ => "",
    }
}

const STREAM_TEXT: &str = "Sure! glowkit for Dioxus reuses the same design tokens, easing curves and spring model as the egui original. Each token you see here fades in as it arrives, and the caret rides along the end of the stream.";

#[component]
fn LoginCard() -> Element {
    rsx! {
        div { class: "g-card-title", "Login" }
        p { class: "g-card-copy", "Enter your credentials to access your account." }
        div { class: "g-field", label { "Email" } input { class: "g-input", placeholder: "you@example.com" } }
        div { class: "g-field", label { "Password" } input { class: "g-input", r#type: "password", placeholder: "••••••••" } }
        div { class: "g-row", style: "justify-content: space-between; margin-top: 12px;",
            button { class: "gk-btn-outline", "Register" }
            button { class: "gk-btn-solid", "Login" }
        }
    }
}

#[component]
fn CountUpDemo() -> Element {
    let mut target = use_signal(|| 12_480.0);
    rsx! {
        div { class: "g-col",
            CountUp { to: target(), prefix: "$", decimals: 2, size: 64.0 }
            div { class: "g-row",
                CountUp { to: 98.6, decimals: 1, suffix: "%", size: 28.0, affix_color: "var(--gk-muted-foreground)" }
                CountUp { to: 1_000_000.0, size: 28.0, separator: " ", suffix: " users", affix_color: "var(--gk-muted-foreground)", easing: Easing::OutBack, duration: 1.6 }
            }
            button { class: "gk-btn-outline", onclick: move |_| target.set((target() * 1.37).round() % 1_000_000.0), "Change target" }
        }
    }
}

#[component]
fn MarqueeDemo() -> Element {
    const LOGOS: [&str; 8] = ["Rust", "Dioxus", "egui", "WebAssembly", "Tauri", "Bevy", "wgpu", "Leptos"];
    const QUOTES: [(&str, &str); 4] = [
        ("Ada", "The springs feel exactly like the native app."),
        ("Linus", "One theme struct, two UI frameworks. Lovely."),
        ("Grace", "The dock magnification is buttery smooth."),
        ("Ken", "Swapped egui for Dioxus and kept the look."),
    ];
    rsx! {
        div { class: "g-col", style: "width: 100%; gap: 22px;",
            Marquee { speed: 45.0,
                render: move |_| rsx! { for l in LOGOS { span { class: "g-logo-chip", "{l}" } } }
            }
            Marquee { speed: 30.0, reverse: true, gap: 16.0,
                render: move |_| rsx! {
                    for (who, q) in QUOTES {
                        div { class: "g-quote", p { "“{q}”" } span { "— {who}" } }
                    }
                }
            }
        }
    }
}

fn dock_items() -> Vec<DockItem> {
    vec![
        DockItem::new("🏠", "Home").colors("#60a5fa", "#2563eb"),
        DockItem::new("✉️", "Mail").colors("#38bdf8", "#0284c7").active(true),
        DockItem::new("🎵", "Music").colors("#fb7185", "#e11d48"),
        DockItem::new("📷", "Photos").colors("#fbbf24", "#f97316"),
        DockItem::new("💬", "Messages").colors("#4ade80", "#16a34a").active(true),
        DockItem::new("📅", "Calendar").colors("#f8fafc", "#e2e8f0"),
        DockItem::new("⚙️", "Settings").colors("#9ca3af", "#4b5563"),
        DockItem::new("🗑️", "Trash").colors("#cbd5e1", "#94a3b8"),
    ]
}

#[component]
fn DockDemo() -> Element {
    let toaster = use_toaster();
    let items = dock_items();
    let names: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
    let minimal = vec![
        DockItem::new("⌂", "Home"),
        DockItem::new("◎", "Search"),
        DockItem::new("✎", "Write"),
        DockItem::new("☰", "Menu"),
        DockItem::new("♫", "Music"),
        DockItem::new("⚙", "Settings"),
    ];
    rsx! {
        div { class: "g-col", style: "width: 100%; gap: 8px;",
            Dock { items, onselect: move |i: usize| { toaster.push(Toast::new(format!("Opened {}", names[i]))); } }
            Dock { items: minimal, style: DockStyle::Minimal, size: 40.0, autoplay: true }
        }
    }
}

#[component]
fn TabsDemo() -> Element {
    let a = use_signal(|| 1usize);
    let b = use_signal(|| 2usize);
    let c = use_signal(|| 0usize);
    let labels: Vec<String> = ["Overview", "Components", "Themes", "Changelog"].iter().map(|s| s.to_string()).collect();
    rsx! {
        div { class: "g-col", style: "gap: 26px;",
            AnimatedTabs { selected: a, labels: labels.clone() }
            AnimatedTabs { selected: b, labels: labels.clone(), style: TabHighlight::Solid }
            AnimatedTabs { selected: c, labels: labels.clone(), style: TabHighlight::Underline }
            p { class: "g-muted", "Selected: {labels[a()]} · {labels[b()]} · {labels[c()]}" }
        }
    }
}

#[component]
fn TabsThumb() -> Element {
    let a = use_signal(|| 0usize);
    rsx! { AnimatedTabs { selected: a, labels: vec!["Home".into(), "Docs".into(), "API".into()], font_size: 12.0 } }
}

#[component]
fn ToastDemo() -> Element {
    let toaster = use_toaster();
    let mut n = use_signal(|| 0u32);
    use_hook(move || {
        toaster.push(Toast::success("Event created").description("Sunday, December 03 at 9:00 AM").action("Undo").duration(Some(30.0)));
        toaster.push(Toast::info("Update available").description("glowkit-dioxus 0.1.1 is out.").duration(Some(30.0)));
    });
    rsx! {
        div { class: "g-col",
            p { class: "g-muted", "Toasts stack in the bottom-right corner. Hover the stack to fan it out." }
            div { class: "g-row",
                button { class: "gk-btn-outline", onclick: move |_| { n += 1; toaster.push(Toast::new(format!("Notification #{}", n())).description("A neutral toast")); }, "Default" }
                button { class: "gk-btn-outline", onclick: move |_| { toaster.push(Toast::success("Saved").description("Your changes are live.").action("Undo")); }, "Success" }
                button { class: "gk-btn-outline", onclick: move |_| { toaster.push(Toast::error("Deploy failed").description("Build exited with code 1.")); }, "Error" }
                button { class: "gk-btn-outline", onclick: move |_| { toaster.push(Toast::warning("Disk almost full").description("92% used on /dev/vda.")); }, "Warning" }
                button { class: "gk-btn-outline",
                    onclick: move |_| {
                        let id = toaster.push(Toast::loading("Uploading…"));
                        spawn(async move {
                            glowkit_dioxus::anim::sleep_ms(1800.0).await;
                            toaster.update(id, Toast::success("Uploaded").description("report.pdf · 2.4 MB"));
                        });
                    },
                    "Loading → Success"
                }
            }
        }
    }
}

#[component]
fn ChatDemo() -> Element {
    let mut streaming_key = use_signal(|| 0u32);
    rsx! {
        div { class: "g-chat",
            ChatMessage { role: Role::System, content: "Conversation started · AI assistant" }
            ChatMessage { role: Role::User, content: "Can you port glowkit to Dioxus?", user_avatar: "JD" }
            ChatMessage {
                role: Role::Assistant,
                name: "Assistant",
                timestamp: "just now",
                actions: true,
                actions_always_visible: true,
                content: "Yes! The port keeps the **same tokens** and motion:\n\n- Themes become CSS custom properties\n- Ambient motion uses `@keyframes`\n- Interactive motion uses `use_spring`\n\n```rust\nShimmerButton {{ label: \"Get started\" }}\n```",
            }
            ChatMessage { role: Role::User, content: "And streaming?", user_avatar: "JD" }
            div { key: "{streaming_key}",
                ChatMessage { role: Role::Assistant, streaming: true, content: "Streaming replies reveal token by token, each one fading in behind a blinking caret, just like the egui version." }
            }
            button { class: "gk-btn-ghost", style: "align-self: flex-start;", onclick: move |_| streaming_key += 1, "↻ Replay stream" }
        }
    }
}

#[component]
fn PromptDemo() -> Element {
    let text = use_signal(String::new);
    let mut generating = use_signal(|| false);
    let mut sent = use_signal(Vec::<String>::new);
    let mut attachments = use_signal(|| vec![Attachment::new("design.png", AttachmentKind::Image).detail("1.2 MB"), Attachment::new("main.rs", AttachmentKind::Code)]);
    rsx! {
        div { class: "g-col", style: "width: min(640px, 100%); gap: 14px;",
            for s in sent() {
                ChatMessage { role: Role::User, content: s }
            }
            PromptInput {
                value: text,
                generating: generating(),
                attachments: attachments(),
                max_chars: 500,
                mic_button: true,
                onsubmit: move |p: String| {
                    sent.write().push(p);
                    generating.set(true);
                    spawn(async move {
                        glowkit_dioxus::anim::sleep_ms(2500.0).await;
                        generating.set(false);
                    });
                },
                onstop: move |_| generating.set(false),
                onremove: move |i: usize| { attachments.write().remove(i); },
                span { class: "gk-model-pill", "✦ glowkit-1" }
            }
            p { class: "g-muted", "Enter sends · Shift+Enter for a new line · focus it to see the glow" }
        }
    }
}

#[component]
fn PromptThumb() -> Element {
    let text = use_signal(String::new);
    rsx! { div { style: "width: 230px;", PromptInput { value: text, attach_button: false, placeholder: "Ask anything…" } } }
}

#[component]
fn ApprovalDemo() -> Element {
    rsx! {
        div { class: "g-col", style: "gap: 14px;",
            ApprovalCard { agent: "Deploy Fixer", tool: "shell", command: "pnpm add zod@3 && pnpm build", reason: "The build fails because `zod` is missing from package.json.", risk: Risk::Medium, waiting: 42.0 }
            ApprovalCard { agent: "Refund Resolver", tool: "stripe.refunds.create", command: "charge=ch_3P1x… amount=1240 currency=usd", reason: "Customer was double-charged for order #8812.", risk: Risk::High, waiting: 7.0 }
            ApprovalCard { agent: "Docs Bot", tool: "fs.read", command: "README.md", risk: Risk::Low, decision: Decision::Approved }
        }
    }
}

#[component]
fn StatDemo() -> Element {
    rsx! {
        div { class: "g-row", style: "gap: 14px; align-items: stretch; flex-wrap: wrap; justify-content: center; max-width: 470px;",
            StatTile { title: "Total revenue", value: 45_231.89, prefix: "$", decimals: 2, delta: 20.1, caption: "vs last month", spark: vec![12.0, 18.0, 15.0, 22.0, 20.0, 28.0, 26.0, 34.0, 31.0, 40.0], width: 210.0 }
            StatTile { title: "Subscriptions", value: 2350.0, prefix: "+", delta: 180.1, caption: "vs last month", spark: vec![4.0, 6.0, 5.0, 9.0, 12.0, 11.0, 16.0, 19.0, 24.0], width: 210.0 }
            StatTile { title: "Churn rate", value: 2.4, suffix: "%", decimals: 1, delta: -0.6, invert_delta: true, caption: "vs last month", spark: vec![30.0, 28.0, 29.0, 25.0, 22.0, 23.0, 19.0, 17.0], kind: SparkKind::Line, width: 210.0 }
            StatTile { title: "Avg. latency", value: 182.0, suffix: " ms", delta: 12.4, invert_delta: true, caption: "vs last week", spark: vec![120.0, 140.0, 132.0, 160.0, 150.0, 175.0, 168.0, 182.0], kind: SparkKind::Bar, width: 210.0 }
        }
    }
}
