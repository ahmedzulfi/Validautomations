//! Text animation playgrounds.

use super::{center, variant, Code, Params, Playground};
use glowkit::anim::Trigger;
use glowkit::color::hex;
use glowkit::core::playground::{lit_colors, lit_str};
use glowkit::egui::Ui;
use glowkit::text::*;

const TRIGGERS: [&str; 4] = ["OnAppear", "Loop", "OnHover", "OnClick"];

fn trigger(i: usize) -> Trigger {
    [Trigger::OnAppear, Trigger::Loop { pause_ms: 1200 }, Trigger::OnHover, Trigger::OnClick][i.min(3)]
}

fn trigger_code(i: usize) -> String {
    if i == 1 {
        "Trigger::Loop { pause_ms: 1200 }".into()
    } else {
        variant("Trigger", &TRIGGERS, i)
    }
}

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "shiny-text", show: shiny },
        Playground { id: "gradient-text", show: gradient },
        Playground { id: "blur-text", show: blur },
        Playground { id: "typewriter-text", show: typewriter },
        Playground { id: "count-up", show: count_up },
        Playground { id: "glitch-text", show: glitch },
        Playground { id: "wave-text", show: wave },
        Playground { id: "decrypted-text", show: decrypted },
        Playground { id: "sparkles-text", show: sparkles },
    ]
}

fn shiny(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "Just some shiny text!");
    let size = p.f32("Size", 44.0, 16.0..=96.0, 1.0);
    let speed = p.f32("Seconds per sweep", 2.5, 0.5..=8.0, 0.1);
    let band = p.f32("Band width", 0.25, 0.05..=0.8, 0.01);
    let angle = p.f32("Angle", 20.0, -60.0..=60.0, 1.0);
    center(ui, |ui| ui.add(ShinyText::new(&text).size(size).speed(speed).band(band).angle(angle)));
    Code::new("glowkit::text::ShinyText", format!("ShinyText::new({})", lit_str(&text)))
        .f("size", size)
        .f("speed", speed)
        .f("band", band)
        .f("angle", angle)
        .add()
}

fn gradient(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "Add a splash of color!");
    let size = p.f32("Size", 44.0, 16.0..=96.0, 1.0);
    let a = p.color("Color 1", hex("#40FFAA"));
    let b = p.color("Color 2", hex("#4079FF"));
    let c = p.color("Color 3", hex("#FF40D9"));
    let border = p.bool("Border", false);
    let cols = [a, b, c];
    center(ui, |ui| ui.add(GradientText::new(&text).colors(&cols).size(size).border(border)));
    Code::new("glowkit::text::GradientText", format!("GradientText::new({})", lit_str(&text)))
        .call("colors", lit_colors(&cols))
        .f("size", size)
        .b("border", border)
        .add()
}

fn blur(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "Isn't this so cool?!");
    let split = p.choice("Split by", &["Words", "Letters"], 1);
    let dir = p.choice("Direction", &["Top", "Bottom", "Left", "Right", "None"], 0);
    let trig = p.choice("Trigger", &TRIGGERS, 1);
    let blur = p.f32("Blur", 10.0, 0.0..=30.0, 0.5);
    let size = p.f32("Size", 44.0, 16.0..=96.0, 1.0);
    center(ui, |ui| {
        ui.add(
            BlurText::new(&text)
                .split_by([SplitBy::Words, SplitBy::Letters][split])
                .direction([Direction::Top, Direction::Bottom, Direction::Left, Direction::Right, Direction::None][dir])
                .trigger(trigger(trig))
                .blur(blur)
                .size(size),
        )
    });
    Code::new(
        "glowkit::text::{BlurText, Direction, SplitBy}; glowkit::anim::Trigger",
        format!("BlurText::new({})", lit_str(&text)),
    )
    .call("split_by", variant("SplitBy", &["Words", "Letters"], split))
    .call("direction", variant("Direction", &["Top", "Bottom", "Left", "Right", "None"], dir))
    .call("trigger", trigger_code(trig))
    .f("blur", blur)
    .f("size", size)
    .add()
}

fn typewriter(ui: &mut Ui, p: &mut Params) -> String {
    let prefix = p.text("Prefix", "Build ");
    let words = p.text("Words (comma separated)", "beautiful apps., fast tools., games.");
    let size = p.f32("Size", 40.0, 16.0..=80.0, 1.0);
    let cps = p.f32("Typing speed (chars/s)", 14.0, 2.0..=40.0, 1.0);
    let cursor = p.choice("Cursor", &["Bar", "Block", "Underscore", "None"], 0);
    let mono = p.bool("Monospace", false);
    let list: Vec<String> = words.split(',').map(|w| w.trim().to_owned()).filter(|w| !w.is_empty()).collect();
    let list = if list.is_empty() { vec![String::from("…")] } else { list };
    let cs = [CursorStyle::Bar, CursorStyle::Block, CursorStyle::Underscore, CursorStyle::None][cursor];
    center(ui, |ui| {
        ui.add(
            TypewriterText::new(list.clone()).prefix(&prefix).size(size).typing_speed(cps).cursor(cs).monospace(mono),
        )
    });
    let lits: Vec<String> = list.iter().map(|w| lit_str(w)).collect();
    Code::new("glowkit::text::{CursorStyle, TypewriterText}", format!("TypewriterText::new([{}])", lits.join(", ")))
        .s("prefix", &prefix)
        .f("size", size)
        .f("typing_speed", cps)
        .call("cursor", variant("CursorStyle", &["Bar", "Block", "Underscore", "None"], cursor))
        .b("monospace", mono)
        .add()
}

fn count_up(ui: &mut Ui, p: &mut Params) -> String {
    let to = p.f32("Target", 12_480.5, 0.0..=100_000.0, 0.5) as f64;
    let decimals = p.count("Decimals", 2, 0..=4);
    let prefix = p.text("Prefix", "$");
    let duration = p.f32("Duration (s)", 2.0, 0.2..=6.0, 0.1);
    let size = p.f32("Size", 56.0, 16.0..=110.0, 1.0);
    let trig = p.choice("Trigger", &TRIGGERS, 1);
    center(ui, |ui| {
        ui.add(CountUp::new(to).prefix(&prefix).decimals(decimals).duration(duration).size(size).trigger(trigger(trig)))
    });
    Code::new(
        "glowkit::text::CountUp; glowkit::anim::Trigger",
        format!("CountUp::new({})", glowkit::core::playground::lit_f32(to as f32)),
    )
    .s("prefix", &prefix)
    .call("decimals", decimals)
    .f("duration", duration)
    .f("size", size)
    .call("trigger", trigger_code(trig))
    .add()
}

fn glitch(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "GLITCH");
    let size = p.f32("Size", 110.0, 24.0..=160.0, 1.0);
    let intensity = p.f32("Intensity", 1.0, 0.0..=3.0, 0.05);
    let left = p.color("Left channel", hex("#FF2E63"));
    let right = p.color("Right channel", hex("#08D9D6"));
    let hover = p.bool("Hover only", false);
    center(ui, |ui| {
        ui.add(GlitchText::new(&text).size(size).intensity(intensity).channels(left, right).hover_only(hover))
    });
    Code::new("glowkit::text::GlitchText", format!("GlitchText::new({})", lit_str(&text)))
        .f("size", size)
        .f("intensity", intensity)
        .call(
            "channels",
            format!("{}, {}", glowkit::core::playground::lit_color(left), glowkit::core::playground::lit_color(right)),
        )
        .b("hover_only", hover)
        .add()
}

fn wave(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "Riding the wave");
    let size = p.f32("Size", 56.0, 16.0..=110.0, 1.0);
    let amp = p.f32("Amplitude", 10.0, 0.0..=40.0, 0.5);
    let period = p.f32("Period (s)", 1.6, 0.3..=5.0, 0.1);
    let wl = p.f32("Wavelength (letters)", 8.0, 2.0..=30.0, 0.5);
    let grad = p.bool("Theme gradient", true);
    let t = glowkit::Theme::of(ui);
    center(ui, |ui| {
        let mut w = WaveText::new(&text).size(size).amplitude(amp).period(period).wavelength(wl);
        if grad {
            w = w.colors(&t.gradient);
        }
        ui.add(w)
    });
    Code::new("glowkit::text::WaveText; glowkit::Theme", format!("WaveText::new({})", lit_str(&text)))
        .pre(if grad { "let theme = Theme::of(ui);" } else { "" })
        .f("size", size)
        .f("amplitude", amp)
        .f("period", period)
        .f("wavelength", wl)
        .call_if(grad, "colors", "&theme.gradient")
        .add()
}

fn decrypted(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "ACCESS GRANTED");
    let size = p.f32("Size", 44.0, 16.0..=96.0, 1.0);
    let from = p.choice("Reveal from", &["Start", "End", "Center", "Random"], 0);
    let trig = p.choice("Trigger", &TRIGGERS, 1);
    let step = p.f32("Seconds per letter", 0.05, 0.01..=0.3, 0.01);
    let mono = p.bool("Monospace", true);
    let sf = [StaggerFrom::Start, StaggerFrom::End, StaggerFrom::Center, StaggerFrom::Random][from];
    center(ui, |ui| {
        ui.add(DecryptedText::new(&text).size(size).reveal_from(sf).trigger(trigger(trig)).step(step).monospace(mono))
    });
    Code::new(
        "glowkit::text::{DecryptedText, StaggerFrom}; glowkit::anim::Trigger",
        format!("DecryptedText::new({})", lit_str(&text)),
    )
    .f("size", size)
    .call("reveal_from", variant("StaggerFrom", &["Start", "End", "Center", "Random"], from))
    .call("trigger", trigger_code(trig))
    .f("step", step)
    .b("monospace", mono)
    .add()
}

fn sparkles(ui: &mut Ui, p: &mut Params) -> String {
    let text = p.text("Text", "Sparkles Text");
    let size = p.f32("Size", 64.0, 20.0..=120.0, 1.0);
    let count = p.count("Sparkles", 10, 0..=40);
    let a = p.color("Sparkle 1", hex("#9E7AFF"));
    let b = p.color("Sparkle 2", hex("#FE8BBB"));
    let bold = p.bool("Bold", true);
    let cols = [a, b];
    center(ui, |ui| ui.add(SparklesText::new(&text).size(size).count(count).colors(&cols).bold(bold)));
    Code::new("glowkit::text::SparklesText", format!("SparklesText::new({})", lit_str(&text)))
        .f("size", size)
        .call("count", count)
        .call("colors", lit_colors(&cols))
        .b("bold", bold)
        .add()
}
