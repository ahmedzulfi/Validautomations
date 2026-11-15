//! AI component playgrounds.

use super::{center, variant, Code, Params, Playground};
use glowkit::ai::*;
use glowkit::anim::Trigger;
use glowkit::color::hex;
use glowkit::core::playground::{lit_f32, lit_str};
use glowkit::egui::Ui;

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "ai-streaming-text", show: streaming },
        Playground { id: "ai-voice-orb", show: voice_orb },
        Playground { id: "ai-loaders", show: loaders },
        Playground { id: "ai-context-meter", show: context_meter },
        Playground { id: "ai-chat-message", show: chat_message },
        Playground { id: "ai-dot-matrix", show: dot_matrix },
        Playground { id: "ai-waveform", show: waveform },
    ]
}

fn streaming(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text(
        "Text",
        "Closures in Rust capture variables from their environment. They come in three flavours, Fn, FnMut and FnOnce, depending on how they use what they capture.",
    );
    let tps = p.f32("Tokens per second", 14.0, 2.0..=60.0, 1.0);
    let style = p.choice("Reveal", &["Fade", "Typewriter", "Blur"], 0);
    let size = p.f32("Size", 17.0, 11.0..=28.0, 0.5);
    let caret = p.bool("Caret", true);
    let lp = p.bool("Loop", true);
    ui.add_space(40.0);
    ui.add(
        StreamingText::new(&text)
            .tokens_per_second(tps)
            .style([RevealStyle::Fade, RevealStyle::Typewriter, RevealStyle::Blur][style])
            .size(size)
            .caret(caret)
            .trigger(if lp { Trigger::Loop { pause_ms: 1500 } } else { Trigger::OnAppear }),
    );
    Code::new("glowkit::ai::{RevealStyle, StreamingText}; glowkit::anim::Trigger", "StreamingText::new(text)")
        .pre(format!("let text = {};", lit_str(&text)))
        .f("tokens_per_second", tps)
        .call("style", variant("RevealStyle", &["Fade", "Typewriter", "Blur"], style))
        .f("size", size)
        .b("caret", caret)
        .call("trigger", if lp { "Trigger::Loop { pause_ms: 1500 }" } else { "Trigger::OnAppear" })
        .add()
}

const VOICE: [&str; 4] = ["Idle", "Listening", "Thinking", "Speaking"];

fn voice_orb(ui: &mut Ui, p: &mut Params) -> String {
    let state = p.choice("State", &VOICE, 3);
    let amp = p.f32("Amplitude", 0.6, 0.0..=1.0, 0.01);
    let size = p.f32("Size", 220.0, 80.0..=340.0, 5.0);
    let a = p.color("Color 1", hex("#8B5CF6"));
    let b = p.color("Color 2", hex("#22D3EE"));
    let c = p.color("Color 3", hex("#EC4899"));
    let st = [VoiceState::Idle, VoiceState::Listening, VoiceState::Thinking, VoiceState::Speaking][state];
    center(ui, |ui| ui.add(VoiceOrb::new().state(st).amplitude(amp).size(size).colors([a, b, c])));
    Code::new("glowkit::ai::{VoiceOrb, VoiceState}", "VoiceOrb::new()")
        .call("state", variant("VoiceState", &VOICE, state))
        .f("amplitude", amp)
        .f("size", size)
        .call(
            "colors",
            format!(
                "[{}, {}, {}]",
                glowkit::core::playground::lit_color(a),
                glowkit::core::playground::lit_color(b),
                glowkit::core::playground::lit_color(c)
            ),
        )
        .add()
}

const LOADERS: [&str; 8] = ["Dots", "Typing", "Pulse", "Wave", "Circular", "Bars", "TextShimmer", "TextBlink"];

fn loaders(ui: &mut Ui, p: &mut Params) -> String {
    let kind = p.choice("Kind", &LOADERS, 3);
    let size = p.f32("Size", 28.0, 10.0..=64.0, 1.0);
    let text = p.text("Text", "Thinking…");
    let custom = p.bool("Custom color", false);
    let color = p.color("Color", hex("#8B5CF6"));
    let k = [
        LoaderKind::Dots,
        LoaderKind::Typing,
        LoaderKind::Pulse,
        LoaderKind::Wave,
        LoaderKind::Circular,
        LoaderKind::Bars,
        LoaderKind::TextShimmer,
        LoaderKind::TextBlink,
    ][kind];
    center(ui, |ui| {
        ui.add_space(60.0);
        let mut l = Loader::new(k).size(size).text(&text);
        if custom {
            l = l.color(color);
        }
        ui.add(l);
    });
    Code::new("glowkit::ai::{Loader, LoaderKind}", format!("Loader::new({})", variant("LoaderKind", &LOADERS, kind)))
        .f("size", size)
        .s("text", &text)
        .call_if(custom, "color", glowkit::core::playground::lit_color(color))
        .add()
}

fn context_meter(ui: &mut Ui, p: &mut Params) -> String {
    let used = p.int("Used tokens", 48_200, 0..=200_000) as u64;
    let max = p.choice("Context window", &["32k", "128k", "200k"], 1);
    let style = p.choice("Style", &["Ring", "Bar"], 1);
    let model = p.text("Model", "Nova 2 Pro");
    let cost = p.f32("Cost ($)", 0.0241, 0.0..=1.0, 0.0001);
    let maxv = [32_000u64, 128_000, 200_000][max];
    let input = used * 62 / 100;
    let output = used * 27 / 100;
    let reasoning = used - input - output;
    center(ui, |ui| {
        ui.add_space(30.0);
        ui.set_max_width(460.0);
        ui.add(
            ContextMeter::new(used, maxv)
                .segment("Input", input)
                .segment("Output", output)
                .segment("Reasoning", reasoning)
                .model(&model)
                .cost(cost as f64)
                .style([MeterStyle::Ring, MeterStyle::Bar][style]),
        );
    });
    Code::new("glowkit::ai::{ContextMeter, MeterStyle}", format!("ContextMeter::new({used}, {maxv})"))
        .call("segment", format!("\"Input\", {input}"))
        .call("segment", format!("\"Output\", {output}"))
        .call("segment", format!("\"Reasoning\", {reasoning}"))
        .s("model", &model)
        .call("cost", format!("{:.4}", cost))
        .call("style", variant("MeterStyle", &["Ring", "Bar"], style))
        .add()
}

fn chat_message(ui: &mut Ui, p: &mut Params) -> String {
    let role = p.choice("Role", &["User", "Assistant"], 1);
    let content = p.text(
        "Content",
        "Closures capture variables from the scope they are defined in. Use `move` to take ownership.",
    );
    let name = p.text("Name", "Assistant");
    let ts = String::from("2:14 PM");
    let avatar = p.bool("Avatar", true);
    let actions = p.bool("Actions always visible", true);
    let streaming = p.bool("Streaming", false);
    let r = [Role::User, Role::Assistant][role];
    ui.add_space(30.0);
    ui.add(ChatMessage::new(Role::User, "Can you explain closures?").timestamp("2:13 PM").id_salt("q"));
    ui.add_space(8.0);
    ui.add(
        ChatMessage::new(r, &content)
            .name(&name)
            .timestamp(&ts)
            .avatar(avatar)
            .actions(true)
            .actions_always_visible(actions)
            .streaming(streaming)
            .id_salt("a"),
    );
    Code::new(
        "glowkit::ai::{ChatMessage, Role}",
        format!("ChatMessage::new({}, {})", variant("Role", &["User", "Assistant"], role), lit_str(&content)),
    )
    .s("name", &name)
    .s("timestamp", &ts)
    .b("avatar", avatar)
    .b("actions", true)
    .b("actions_always_visible", actions)
    .b("streaming", streaming)
    .add()
}

const PATTERNS: [&str; 5] = ["Wave", "Ripple", "Rain", "Loader", "Sparkle"];

fn dot_matrix(ui: &mut Ui, p: &mut Params) -> String {
    let mode = p.choice("Content", &["Text", "Pattern"], 0);
    let text = p.text("Text", "HELLO WORLD");
    let pattern = p.choice("Pattern", &PATTERNS, 1);
    let cols = p.count("Columns", 64, 8..=96);
    let rows = p.count("Rows", 9, 5..=24);
    let pitch = 9.0;
    let gradient = p.bool("Gradient", true);
    let content = if mode == 0 {
        MatrixContent::Text(&text)
    } else {
        MatrixContent::Pattern(
            [
                MatrixPattern::Wave,
                MatrixPattern::Ripple,
                MatrixPattern::Rain,
                MatrixPattern::Loader,
                MatrixPattern::Sparkle,
            ][pattern],
        )
    };
    center(ui, |ui| {
        ui.add_space(40.0);
        ui.add(DotMatrix::new(cols, rows).content(content).pitch(pitch).gradient(gradient));
    });
    let content_code = if mode == 0 {
        format!("MatrixContent::Text({})", lit_str(&text))
    } else {
        format!("MatrixContent::Pattern({})", variant("MatrixPattern", &PATTERNS, pattern))
    };
    let uses = if mode == 0 {
        "glowkit::ai::{DotMatrix, MatrixContent}"
    } else {
        "glowkit::ai::{DotMatrix, MatrixContent, MatrixPattern}"
    };
    Code::new(uses, format!("DotMatrix::new({cols}, {rows})"))
        .call("content", content_code)
        .f("pitch", pitch)
        .b("gradient", gradient)
        .add()
}

fn waveform(ui: &mut Ui, p: &mut Params) -> String {
    let active = p.bool("Active", true);
    let height = p.f32("Height", 96.0, 24.0..=200.0, 1.0);
    let bar_w = p.f32("Bar width", 3.0, 1.0..=10.0, 0.5);
    let gap = p.f32("Bar gap", 2.0, 0.0..=8.0, 0.5);
    let rate = p.f32("Bars per second", 30.0, 5.0..=80.0, 1.0);
    let color = p.color("Color", hex("#8B5CF6"));
    let t = glowkit::core::time(ui);
    let level = (0.35 + 0.35 * (t * 2.3).sin() * (t * 0.7).cos() + 0.2 * (t * 7.0).sin().abs()).clamp(0.0, 1.0);
    ui.add_space(80.0);
    ui.add(LiveWaveform::new(level).active(active).height(height).bar(bar_w, gap).rate(rate).color(color));
    glowkit::core::animate(ui);
    Code::new("glowkit::ai::LiveWaveform", "LiveWaveform::new(mic_level)")
        .b("active", active)
        .f("height", height)
        .call("bar", format!("{}, {}", lit_f32(bar_w), lit_f32(gap)))
        .f("rate", rate)
        .c("color", color)
        .add()
}
