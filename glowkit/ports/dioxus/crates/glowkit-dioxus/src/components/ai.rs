//! AI chat components: messages (with lightweight Markdown), streaming text and the composer.

use crate::anim::{sleep_ms, use_replay};
use dioxus::prelude::*;
use glowkit_tokens::text::token_ends;
use glowkit_tokens::Trigger;

// ---------------------------------------------------------------------------------------------
// Icons (Lucide-style 24x24 stroke paths)
// ---------------------------------------------------------------------------------------------

/// Small built-in stroke icons used by the AI components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Copy,
    Check,
    Refresh,
    ThumbUp,
    ThumbDown,
    Pencil,
    Mic,
    More,
    Plus,
    ArrowUp,
    Stop,
    Sparkle,
    File,
    Image,
    Code,
    Link,
    X,
    Terminal,
    Clock,
    Shield,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Icon::Copy => "M8 8h12v12H8z M16 8V4H4v12h4",
            Icon::Check => "M20 6 9 17l-5-5",
            Icon::Refresh => "M3 12a9 9 0 0 1 15-6.7L21 8 M21 3v5h-5 M21 12a9 9 0 0 1-15 6.7L3 16 M3 21v-5h5",
            Icon::ThumbUp => "M7 10v12 M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.76a2 2 0 0 0 1.79-1.11L12 2a3.13 3.13 0 0 1 3 3.88Z",
            Icon::ThumbDown => "M17 14V2 M9 18.12 10 14H4.17a2 2 0 0 1-1.92-2.56l2.33-8A2 2 0 0 1 6.5 2H20a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2.76a2 2 0 0 0-1.79 1.11L12 22a3.13 3.13 0 0 1-3-3.88Z",
            Icon::Pencil => "M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z",
            Icon::Mic => "M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z M19 10v2a7 7 0 0 1-14 0v-2 M12 19v3",
            Icon::More => "M5 12h.01 M12 12h.01 M19 12h.01",
            Icon::Plus => "M12 5v14 M5 12h14",
            Icon::ArrowUp => "M12 19V5 M5 12l7-7 7 7",
            Icon::Stop => "M7 7h10v10H7z",
            Icon::Sparkle => "M12 3l1.9 5.8L20 11l-6.1 2.2L12 19l-1.9-5.8L4 11l6.1-2.2Z",
            Icon::File => "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z M14 2v6h6",
            Icon::Image => "M3 3h18v18H3z M9 9h.01 M21 15l-5-5L5 21",
            Icon::Code => "M16 18l6-6-6-6 M8 6l-6 6 6 6",
            Icon::Link => "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71 M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71",
            Icon::X => "M18 6 6 18 M6 6l12 12",
            Icon::Terminal => "M4 17l6-6-6-6 M12 19h8",
            Icon::Clock => "M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20 M12 6v6l4 2",
            Icon::Shield => "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z",
        }
    }
}

/// Render an [`Icon`] as inline SVG.
#[component]
pub fn IconSvg(icon: Icon, #[props(default = 16.0)] size: f32, #[props(default = 2.0)] stroke: f32) -> Element {
    rsx! {
        svg { class: "gk-icon", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "{stroke}", stroke_linecap: "round", stroke_linejoin: "round",
            path { d: icon.path() }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Lightweight Markdown
// ---------------------------------------------------------------------------------------------

fn inline(text: &str) -> Element {
    // **bold**, *italic*, `code`
    let mut parts: Vec<(u8, String)> = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    let mut mode = 0u8; // 0 plain, 1 bold, 2 italic, 3 code
    while let Some(c) = chars.next() {
        match c {
            '`' if mode == 0 || mode == 3 => {
                parts.push((mode, std::mem::take(&mut cur)));
                mode = if mode == 3 { 0 } else { 3 };
            }
            '*' if mode != 3 && chars.peek() == Some(&'*') => {
                chars.next();
                parts.push((mode, std::mem::take(&mut cur)));
                mode = if mode == 1 { 0 } else { 1 };
            }
            '*' if mode == 0 || mode == 2 => {
                parts.push((mode, std::mem::take(&mut cur)));
                mode = if mode == 2 { 0 } else { 2 };
            }
            _ => cur.push(c),
        }
    }
    parts.push((mode, cur));
    rsx! {
        for (m, s) in parts.into_iter().filter(|(_, s)| !s.is_empty()) {
            match m {
                1 => rsx! { strong { "{s}" } },
                2 => rsx! { em { "{s}" } },
                3 => rsx! { code { class: "gk-md-code", "{s}" } },
                _ => rsx! { "{s}" },
            }
        }
    }
}

/// Render a small, safe subset of Markdown: headings, paragraphs, bullet and numbered lists,
/// fenced code blocks, `inline code`, **bold** and *italic*.
pub fn markdown(src: &str) -> Element {
    enum Block {
        P(String),
        H(usize, String),
        Ul(Vec<String>),
        Ol(Vec<String>),
        Code(String, String),
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        let t = line.trim_start();
        if let Some(lang) = t.strip_prefix("```") {
            let mut code = String::new();
            for l in lines.by_ref() {
                if l.trim_start().starts_with("```") {
                    break;
                }
                code.push_str(l);
                code.push('\n');
            }
            blocks.push(Block::Code(lang.trim().to_string(), code.trim_end().to_string()));
        } else if t.starts_with('#') {
            let lvl = t.chars().take_while(|c| *c == '#').count().min(4);
            blocks.push(Block::H(lvl, t[lvl..].trim().to_string()));
        } else if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            match blocks.last_mut() {
                Some(Block::Ul(v)) => v.push(item.to_string()),
                _ => blocks.push(Block::Ul(vec![item.to_string()])),
            }
        } else if t.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            let item = t.split_once(". ").map(|(_, s)| s.to_string()).unwrap_or_default();
            match blocks.last_mut() {
                Some(Block::Ol(v)) => v.push(item),
                _ => blocks.push(Block::Ol(vec![item])),
            }
        } else if t.is_empty() {
            blocks.push(Block::P(String::new()));
        } else {
            match blocks.last_mut() {
                Some(Block::P(p)) if !p.is_empty() => {
                    p.push(' ');
                    p.push_str(t);
                }
                _ => blocks.push(Block::P(t.to_string())),
            }
        }
    }
    rsx! {
        div { class: "gk-md",
            for b in blocks {
                match b {
                    Block::P(p) if p.is_empty() => rsx! {},
                    Block::P(p) => rsx! { p { {inline(&p)} } },
                    Block::H(1, h) => rsx! { h3 { {inline(&h)} } },
                    Block::H(_, h) => rsx! { h4 { {inline(&h)} } },
                    Block::Ul(items) => rsx! { ul { for i in items { li { {inline(&i)} } } } },
                    Block::Ol(items) => rsx! { ol { for i in items { li { {inline(&i)} } } } },
                    Block::Code(lang, code) => rsx! {
                        div { class: "gk-md-pre",
                            if !lang.is_empty() { div { class: "gk-md-lang", "{lang}" } }
                            pre { code { "{code}" } }
                        }
                    },
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// StreamingText
// ---------------------------------------------------------------------------------------------

/// How each newly revealed token appears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RevealStyle {
    /// Tokens fade in (and rise a pixel or two).
    #[default]
    Fade,
    /// Tokens appear instantly, typewriter style.
    Typewriter,
    /// Tokens sharpen from a soft blur while fading in.
    Blur,
}

/// Reveals a full response token by token, the way chat UIs show a streaming answer: every new
/// token fades in and a blinking caret rides along the end. Drive it by time
/// (`tokens_per_second`) or by an explicit `progress` (0..1).
///
/// Mirrors `glowkit_ai::StreamingText`: `progress`, `tokens_per_second`, `style`,
/// `fade_duration`, `size`, `color`, `caret`, `trigger` (`OnAppear` or `Loop`), plus `onfinish`.
#[component]
pub fn StreamingText(
    #[props(into)] text: String,
    progress: Option<f32>,
    #[props(default = 14.0)] tokens_per_second: f32,
    #[props(default)] style: RevealStyle,
    #[props(default = 0.45)] fade_duration: f32,
    #[props(default = 15.0)] size: f32,
    #[props(into)] color: Option<String>,
    #[props(default = true)] caret: bool,
    #[props(default)] trigger: Trigger,
    onfinish: Option<EventHandler<()>>,
) -> Element {
    let ends = token_ends(&text);
    let total = ends.len();
    let replay = use_replay(trigger, 0.0);
    let mut count = use_signal(|| 0usize);
    let mut tps = use_signal(|| tokens_per_second);
    if *tps.peek() != tokens_per_second {
        tps.set(tokens_per_second);
    }
    let mut total_sig = use_signal(|| total);
    if *total_sig.peek() != total {
        total_sig.set(total);
    }
    let timed = progress.is_none();
    let loop_pause = match trigger {
        Trigger::Loop { pause_ms } => Some(pause_ms),
        _ => None,
    };
    // time-driven reveal (restarts on replay)
    use_effect(move || {
        let _gen = replay.key();
        if !timed {
            return;
        }
        count.set(0);
        spawn(async move {
            loop {
                let n = *total_sig.peek();
                if *count.peek() >= n {
                    if let Some(h) = onfinish {
                        h.call(());
                    }
                    match loop_pause {
                        Some(p) => {
                            sleep_ms(p as f64 + 1500.0).await;
                            count.set(0);
                            continue;
                        }
                        None => break,
                    }
                }
                sleep_ms(1000.0 / tps.peek().max(0.5) as f64).await;
                count += 1;
            }
        });
    });
    let shown = match progress {
        Some(p) => ((p.clamp(0.0, 1.0) * total as f32).round() as usize).min(total),
        None => count().min(total),
    };
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0usize;
    let mut tokens = Vec::with_capacity(shown);
    for &e in ends.iter().take(shown) {
        tokens.push(chars[start..e].iter().collect::<String>());
        start = e;
    }
    let done = shown >= total;
    let anim = match style {
        RevealStyle::Fade => "gk-tok gk-tok-fade",
        RevealStyle::Typewriter => "gk-tok",
        RevealStyle::Blur => "gk-tok gk-tok-blur",
    };
    let color = color.unwrap_or_else(|| "var(--gk-foreground)".into());
    rsx! {
        span { class: "gk-stream", style: "font-size:{size}px; color:{color}; --gk-fade:{fade_duration}s;",
            for (i, tok) in tokens.into_iter().enumerate() {
                span { key: "{replay.key()}-{i}", class: anim, "{tok}" }
            }
            if caret {
                span { class: if done { "gk-caret gk-caret-blink" } else { "gk-caret" } }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// ChatMessage
// ---------------------------------------------------------------------------------------------

/// Who wrote a [`ChatMessage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The human: right-aligned bubble.
    User,
    /// The model: left-aligned, avatar + Markdown, no bubble.
    Assistant,
    /// A system notice: centered, muted.
    System,
}

/// An action button under a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageAction {
    Copy,
    Regenerate,
    Like,
    Dislike,
    Edit,
    Speak,
    More,
}

impl MessageAction {
    fn icon(self) -> Icon {
        match self {
            MessageAction::Copy => Icon::Copy,
            MessageAction::Regenerate => Icon::Refresh,
            MessageAction::Like => Icon::ThumbUp,
            MessageAction::Dislike => Icon::ThumbDown,
            MessageAction::Edit => Icon::Pencil,
            MessageAction::Speak => Icon::Mic,
            MessageAction::More => Icon::More,
        }
    }
    fn label(self) -> &'static str {
        match self {
            MessageAction::Copy => "Copy",
            MessageAction::Regenerate => "Regenerate",
            MessageAction::Like => "Good response",
            MessageAction::Dislike => "Bad response",
            MessageAction::Edit => "Edit",
            MessageAction::Speak => "Read aloud",
            MessageAction::More => "More",
        }
    }
}

/// A row of small ghost icon buttons: copy (turns into a check), regenerate, thumbs up / down
/// (remembered)... Mirrors `glowkit_ai::MessageActions`.
#[component]
pub fn MessageActions(
    #[props(default = vec![MessageAction::Copy, MessageAction::Regenerate, MessageAction::Like, MessageAction::Dislike])]
    actions: Vec<MessageAction>,
    #[props(default = 28.0)] size: f32,
    onaction: Option<EventHandler<MessageAction>>,
) -> Element {
    let mut copied = use_signal(|| false);
    let mut feedback = use_signal(|| None::<bool>);
    rsx! {
        div { class: "gk-msg-actions",
            for a in actions {
                {
                    let active = match a {
                        MessageAction::Like => feedback() == Some(true),
                        MessageAction::Dislike => feedback() == Some(false),
                        _ => false,
                    };
                    let icon = if a == MessageAction::Copy && copied() { Icon::Check } else { a.icon() };
                    rsx! {
                        button {
                            class: if active { "gk-icon-btn gk-on" } else { "gk-icon-btn" },
                            style: "width:{size}px; height:{size}px;",
                            title: a.label(),
                            "aria-label": a.label(),
                            onclick: move |_| {
                                match a {
                                    MessageAction::Copy => {
                                        copied.set(true);
                                        spawn(async move {
                                            sleep_ms(1500.0).await;
                                            copied.set(false);
                                        });
                                    }
                                    MessageAction::Like => feedback.set(if feedback() == Some(true) { None } else { Some(true) }),
                                    MessageAction::Dislike => feedback.set(if feedback() == Some(false) { None } else { Some(false) }),
                                    _ => {}
                                }
                                if let Some(h) = onaction {
                                    h.call(a);
                                }
                            },
                            IconSvg { icon, size: size * 0.5 }
                        }
                    }
                }
            }
        }
    }
}

/// A chat message: a right-aligned bubble for the user, avatar + Markdown for the assistant, a
/// centered notice for the system.
///
/// Mirrors `glowkit_ai::ChatMessage`: `role`, `content`, `name`, `timestamp`, `avatar`,
/// `user_avatar` (initials), `actions` (+ `actions_always_visible`), `streaming` (reveal the
/// content with [`StreamingText`]), `size`, `max_bubble` (fraction of the width), `bubble`.
#[component]
pub fn ChatMessage(
    role: Role,
    #[props(into)] content: String,
    #[props(into)] name: Option<String>,
    #[props(into)] timestamp: Option<String>,
    #[props(default = true)] avatar: bool,
    #[props(into)] user_avatar: Option<String>,
    #[props(default)] actions: bool,
    #[props(default)] actions_always_visible: bool,
    #[props(default)] streaming: bool,
    #[props(default = 14.0)] size: f32,
    #[props(default = 0.78)] max_bubble: f32,
    #[props(into)] bubble: Option<String>,
    onaction: Option<EventHandler<MessageAction>>,
) -> Element {
    let always = if actions_always_visible { "gk-msg gk-msg-actions-on" } else { "gk-msg" };
    match role {
        Role::System => rsx! {
            div { class: "gk-msg gk-msg-system", style: "font-size:{size * 0.9}px", "{content}" }
        },
        Role::User => {
            let bg = bubble.unwrap_or_else(|| "var(--gk-secondary)".into());
            rsx! {
                div { class: "{always} gk-msg-user", style: "font-size:{size}px;",
                    div { class: "gk-msg-col", style: "max-width:{max_bubble * 100.0}%;",
                        if name.is_some() || timestamp.is_some() {
                            div { class: "gk-msg-meta",
                                if let Some(n) = &name { span { class: "gk-msg-name", "{n}" } }
                                if let Some(ts) = &timestamp { span { "{ts}" } }
                            }
                        }
                        div { class: "gk-bubble", style: "background:{bg};", "{content}" }
                        if actions {
                            MessageActions { actions: vec![MessageAction::Copy, MessageAction::Edit], onaction: move |a| if let Some(h) = onaction { h.call(a) } }
                        }
                    }
                    if avatar {
                        if let Some(ini) = &user_avatar { div { class: "gk-avatar gk-avatar-user", "{ini}" } }
                    }
                }
            }
        }
        Role::Assistant => rsx! {
            div { class: "{always} gk-msg-assistant", style: "font-size:{size}px;",
                if avatar {
                    div { class: "gk-avatar gk-avatar-ai", IconSvg { icon: Icon::Sparkle, size: 15.0, stroke: 2.2 } }
                }
                div { class: "gk-msg-col",
                    if name.is_some() || timestamp.is_some() {
                        div { class: "gk-msg-meta",
                            if let Some(n) = &name { span { class: "gk-msg-name", "{n}" } }
                            if let Some(ts) = &timestamp { span { "{ts}" } }
                        }
                    }
                    if streaming {
                        div { class: "gk-md", p { StreamingText { text: content.clone(), size } } }
                    } else {
                        {markdown(&content)}
                    }
                    if actions {
                        MessageActions { onaction: move |a| if let Some(h) = onaction { h.call(a) } }
                    }
                }
            }
        },
    }
}

// ---------------------------------------------------------------------------------------------
// PromptInput
// ---------------------------------------------------------------------------------------------

/// Kind of an [`Attachment`]; picks the chip icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentKind {
    #[default]
    File,
    Image,
    Code,
    Link,
}

/// A file attached to a prompt, shown as a removable chip.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub kind: AttachmentKind,
    pub detail: String,
}

impl Attachment {
    pub fn new(name: impl Into<String>, kind: AttachmentKind) -> Self {
        Self { name: name.into(), kind, detail: String::new() }
    }
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = d.into();
        self
    }
}

/// The chat composer: a rounded, auto-growing input with placeholder, attachment chips, a
/// toolbar (attach, any custom children such as a model pill, character count, mic) and a send
/// button that turns into a stop button while generating. Enter sends, Shift+Enter adds a
/// newline. When focused, a soft animated gradient glow wraps the box.
///
/// Mirrors `glowkit_ai::PromptInput`: `value` (the `&mut String`, as a signal), `placeholder`,
/// `attachments`, `generating`, `max_chars`, `max_height`, `min_rows`, `glow`,
/// `attach_button`, `mic_button`; events `onsubmit`, `onstop`, `onattach`, `onmic`, `onremove`.
#[component]
pub fn PromptInput(
    value: Signal<String>,
    #[props(into, default = "Ask anything…".to_string())] placeholder: String,
    #[props(default)] attachments: Vec<Attachment>,
    #[props(default)] generating: bool,
    max_chars: Option<usize>,
    #[props(default = 200.0)] max_height: f32,
    #[props(default = 1)] min_rows: usize,
    #[props(default = true)] glow: bool,
    #[props(default = true)] attach_button: bool,
    #[props(default)] mic_button: bool,
    onsubmit: Option<EventHandler<String>>,
    onstop: Option<EventHandler<()>>,
    onattach: Option<EventHandler<()>>,
    onmic: Option<EventHandler<()>>,
    onremove: Option<EventHandler<usize>>,
    children: Element,
) -> Element {
    let mut value = value;
    let mut focused = use_signal(|| false);
    let text = value();
    let len = text.chars().count();
    let over = max_chars.is_some_and(|m| len > m);
    let can_send = !text.trim().is_empty() && !over;
    let mut submit = move || {
        let t = value.peek().trim().to_string();
        if t.is_empty() || max_chars.is_some_and(|m| t.chars().count() > m) {
            return;
        }
        value.set(String::new());
        if let Some(h) = onsubmit {
            h.call(t);
        }
    };
    let mut classes = vec!["gk-prompt"];
    if focused() {
        classes.push("gk-focus");
    }
    if glow {
        classes.push("gk-prompt-glow");
    }
    let min_h = min_rows.max(1) as f32 * 22.0 + 4.0;
    rsx! {
        div { class: classes.join(" "),
            div { class: "gk-prompt-box",
                if !attachments.is_empty() {
                    div { class: "gk-chips",
                        for (i, a) in attachments.iter().enumerate() {
                            div { key: "{i}-{a.name}", class: "gk-chip",
                                IconSvg { icon: match a.kind { AttachmentKind::File => Icon::File, AttachmentKind::Image => Icon::Image, AttachmentKind::Code => Icon::Code, AttachmentKind::Link => Icon::Link }, size: 14.0 }
                                span { class: "gk-chip-name", "{a.name}" }
                                if !a.detail.is_empty() { span { class: "gk-chip-detail", "{a.detail}" } }
                                button { class: "gk-chip-x", "aria-label": "Remove",
                                    onclick: move |_| if let Some(h) = onremove { h.call(i) },
                                    IconSvg { icon: Icon::X, size: 12.0 }
                                }
                            }
                        }
                    }
                }
                textarea {
                    class: "gk-prompt-text",
                    rows: "{min_rows.max(1)}",
                    style: "min-height:{min_h}px; max-height:{max_height}px;",
                    placeholder: "{placeholder}",
                    value: "{text}",
                    oninput: move |e| value.set(e.value()),
                    onfocus: move |_| focused.set(true),
                    onblur: move |_| focused.set(false),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && !e.modifiers().shift() {
                            e.prevent_default();
                            submit();
                        }
                    },
                }
                div { class: "gk-prompt-bar",
                    if attach_button {
                        button { class: "gk-icon-btn gk-round", title: "Attach", "aria-label": "Attach",
                            onclick: move |_| if let Some(h) = onattach { h.call(()) },
                            IconSvg { icon: Icon::Plus, size: 16.0 }
                        }
                    }
                    {children}
                    div { class: "gk-grow" }
                    if let Some(m) = max_chars {
                        span { class: if over { "gk-count-chars gk-over" } else { "gk-count-chars" }, "{len}/{m}" }
                    }
                    if mic_button {
                        button { class: "gk-icon-btn gk-round", title: "Voice", "aria-label": "Voice",
                            onclick: move |_| if let Some(h) = onmic { h.call(()) },
                            IconSvg { icon: Icon::Mic, size: 16.0 }
                        }
                    }
                    if generating {
                        button { class: "gk-send gk-send-stop", title: "Stop", "aria-label": "Stop",
                            onclick: move |_| if let Some(h) = onstop { h.call(()) },
                            span { class: "gk-stop-square" }
                        }
                    } else {
                        button { class: if can_send { "gk-send" } else { "gk-send gk-disabled" }, title: "Send", "aria-label": "Send",
                            disabled: !can_send,
                            onclick: move |_| submit(),
                            IconSvg { icon: Icon::ArrowUp, size: 16.0, stroke: 2.4 }
                        }
                    }
                }
            }
        }
    }
}
