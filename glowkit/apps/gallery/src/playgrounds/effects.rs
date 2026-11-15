//! Buttons, cards & borders, cursor effects, motion and micro-interaction playgrounds.

use super::{center, flag, hint, set_flag, variant, Code, Params, Playground};
use glowkit::color::hex;
use glowkit::core::playground::{lit_color, lit_colors, lit_f32, lit_str};
use glowkit::effects::*;
use glowkit::egui::{self, vec2, Rect, Sense, Ui};
use glowkit::Theme;

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "shimmer-button", show: shimmer_button },
        Playground { id: "shiny-button", show: shiny_button },
        Playground { id: "rainbow-button", show: rainbow_button },
        Playground { id: "pulsating-button", show: pulsating_button },
        Playground { id: "magnetic-button", show: magnetic_button },
        Playground { id: "glow-button", show: glow_button },
        Playground { id: "neon-button", show: neon_button },
        Playground { id: "ripple-button", show: ripple_button },
        Playground { id: "star-border-button", show: star_border_button },
        Playground { id: "border-beam", show: border_beam },
        Playground { id: "shine-border", show: shine_border },
        Playground { id: "magic-card", show: magic_card },
        Playground { id: "spotlight-card", show: spotlight_card },
        Playground { id: "tilt-card", show: tilt_card },
        Playground { id: "electric-border", show: electric_border },
        Playground { id: "neon-gradient-card", show: neon_card },
        Playground { id: "animated-gradient-border", show: gradient_border },
        Playground { id: "click-spark", show: click_spark },
        Playground { id: "confetti", show: confetti },
        Playground { id: "dock", show: dock },
        Playground { id: "marquee", show: marquee },
        Playground { id: "orbiting-circles", show: orbiting },
        Playground { id: "like", show: like },
        Playground { id: "liq-toggle", show: liquid_toggle },
        Playground { id: "slide-confirm", show: slide_confirm },
        Playground { id: "emoji-reactions", show: emoji_reactions },
    ]
}

// --------------------------------------------------------------------------------------------
// Buttons
// --------------------------------------------------------------------------------------------

fn label(p: &mut Params, default: &str) -> String {
    p.text("Label", default)
}

fn font_size(p: &mut Params) -> f32 {
    p.f32("Font size", 16.0, 11.0..=28.0, 0.5)
}

fn shimmer_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Shimmer Button");
    let size = font_size(p);
    let bg = p.color("Background", hex("#0A0A0F"));
    let shimmer = p.color("Shimmer", hex("#FFFFFF"));
    let duration = p.f32("Seconds per lap", 3.0, 0.5..=8.0, 0.1);
    let radius = p.f32("Corner radius", 99.0, 0.0..=99.0, 1.0);
    center(ui, |ui| {
        ui.add(
            ShimmerButton::new(&l).size(size).background(bg).shimmer_color(shimmer).duration(duration).radius(radius),
        )
    });
    Code::new("glowkit::effects::ShimmerButton", format!("ShimmerButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("background", bg)
        .c("shimmer_color", shimmer)
        .f("duration", duration)
        .f("radius", radius)
        .stmt("if ui.add(", ").clicked() {\n    // ...\n}")
}

fn shiny_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Shiny Button");
    let size = font_size(p);
    let shine = p.color("Shine", hex("#FFFFFF"));
    let period = p.f32("Seconds per sweep", 2.6, 0.5..=8.0, 0.1);
    let radius = p.f32("Corner radius", 12.0, 0.0..=40.0, 1.0);
    center(ui, |ui| ui.add(ShinyButton::new(&l).size(size).shine_color(shine).period(period).radius(radius)));
    Code::new("glowkit::effects::ShinyButton", format!("ShinyButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("shine_color", shine)
        .f("period", period)
        .f("radius", radius)
        .add()
}

fn rainbow_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Get Unlimited Access");
    let size = font_size(p);
    let palette = p.choice("Palette", &["RAINBOW", "AURORA", "OCEAN", "SUNSET", "MAGIC"], 0);
    let speed = p.f32("Speed", 1.0, 0.1..=4.0, 0.05);
    let radius = p.f32("Corner radius", 12.0, 0.0..=99.0, 1.0);
    use glowkit::color::palettes as pal;
    let cols: &[egui::Color32] = match palette {
        1 => &pal::AURORA,
        2 => &pal::OCEAN,
        3 => &pal::SUNSET,
        4 => &pal::MAGIC,
        _ => &pal::RAINBOW,
    };
    center(ui, |ui| ui.add(RainbowButton::new(&l).size(size).colors(cols).speed(speed).radius(radius)));
    Code::new(
        "glowkit::effects::RainbowButton; glowkit::color::palettes",
        format!("RainbowButton::new({})", lit_str(&l)),
    )
    .f("size", size)
    .call("colors", format!("&palettes::{}", ["RAINBOW", "AURORA", "OCEAN", "SUNSET", "MAGIC"][palette]))
    .f("speed", speed)
    .f("radius", radius)
    .add()
}

fn pulsating_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Join the waitlist");
    let size = font_size(p);
    let bg = p.color("Background", hex("#7C3AED"));
    let pulse = p.color("Pulse", hex("#A78BFA"));
    let duration = p.f32("Pulse period (s)", 1.5, 0.4..=4.0, 0.1);
    let spread = p.f32("Spread", 12.0, 2.0..=40.0, 0.5);
    center(ui, |ui| {
        ui.add_space(20.0);
        ui.add(PulsatingButton::new(&l).size(size).background(bg).pulse_color(pulse).duration(duration).spread(spread))
    });
    Code::new("glowkit::effects::PulsatingButton", format!("PulsatingButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("background", bg)
        .c("pulse_color", pulse)
        .f("duration", duration)
        .f("spread", spread)
        .add()
}

fn magnetic_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Hover me");
    let size = font_size(p);
    let strength = p.f32("Strength", 0.4, 0.0..=1.0, 0.01);
    let range = p.f32("Range", 120.0, 20.0..=300.0, 5.0);
    let ambient = p.bool("Ambient drift", true);
    center(ui, |ui| {
        ui.add_space(40.0);
        ui.add(MagneticButton::new(&l).size(size).strength(strength).range(range).ambient(ambient));
        ui.add_space(40.0);
    });
    Code::new("glowkit::effects::MagneticButton", format!("MagneticButton::new({})", lit_str(&l)))
        .f("size", size)
        .f("strength", strength)
        .f("range", range)
        .b("ambient", ambient)
        .add()
}

fn glow_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Start building");
    let size = font_size(p);
    let a = p.color("Color 1", hex("#8B5CF6"));
    let b = p.color("Color 2", hex("#EC4899"));
    let intensity = p.f32("Glow intensity", 1.0, 0.0..=3.0, 0.05);
    let radius = p.f32("Corner radius", 12.0, 0.0..=99.0, 1.0);
    let cols = [a, b];
    center(ui, |ui| {
        ui.add_space(20.0);
        ui.add(GlowButton::new(&l).size(size).colors(&cols).intensity(intensity).radius(radius))
    });
    Code::new("glowkit::effects::GlowButton", format!("GlowButton::new({})", lit_str(&l)))
        .f("size", size)
        .call("colors", lit_colors(&cols))
        .f("intensity", intensity)
        .f("radius", radius)
        .add()
}

fn neon_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "NEON");
    let size = p.f32("Font size", 20.0, 11.0..=36.0, 0.5);
    let color = p.color("Color", hex("#22D3EE"));
    let flicker = p.bool("Flicker", true);
    let lit = p.bool("Always lit", false);
    let radius = p.f32("Corner radius", 10.0, 0.0..=40.0, 1.0);
    center(ui, |ui| {
        ui.add_space(20.0);
        ui.add(NeonButton::new(&l).size(size).color(color).flicker(flicker).force_hover(lit).radius(radius))
    });
    Code::new("glowkit::effects::NeonButton", format!("NeonButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("color", color)
        .b("flicker", flicker)
        .b("force_hover", lit)
        .f("radius", radius)
        .add()
}

fn ripple_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Click me");
    let size = font_size(p);
    let bg = p.color("Background", hex("#6366F1"));
    let ripple = p.color("Ripple", hex("#FFFFFF"));
    let duration = p.f32("Ripple duration (s)", 0.6, 0.2..=2.0, 0.05);
    let auto = p.f32("Auto ripple every (s)", 1.5, 0.0..=4.0, 0.1);
    center(ui, |ui| {
        ui.add(
            RippleButton::new(&l).size(size).background(bg).ripple_color(ripple).duration(duration).auto_ripple(auto),
        )
    });
    Code::new("glowkit::effects::RippleButton", format!("RippleButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("background", bg)
        .c("ripple_color", ripple)
        .f("duration", duration)
        .f("auto_ripple", auto)
        .add()
}

fn star_border_button(ui: &mut Ui, p: &mut Params) -> String {
    let l = label(p, "Star Border");
    let size = font_size(p);
    let color = p.color("Star color", hex("#22D3EE"));
    let duration = p.f32("Seconds per lap", 4.0, 0.5..=10.0, 0.1);
    let radius = p.f32("Corner radius", 20.0, 0.0..=40.0, 1.0);
    center(ui, |ui| ui.add(StarBorderButton::new(&l).size(size).color(color).duration(duration).radius(radius)));
    Code::new("glowkit::effects::StarBorderButton", format!("StarBorderButton::new({})", lit_str(&l)))
        .f("size", size)
        .c("color", color)
        .f("duration", duration)
        .f("radius", radius)
        .add()
}

// --------------------------------------------------------------------------------------------
// Cards & borders
// --------------------------------------------------------------------------------------------

fn card_body(ui: &mut Ui, title: &str, text: &str) {
    ui.heading(title);
    ui.label(text);
}

fn card_code(title: &str, text: &str) -> String {
    format!("ui.heading({});\nui.label({});", lit_str(title), lit_str(text))
}

fn card_params(p: &mut Params, title: &str, text: &str) -> (String, String, f32) {
    (p.text("Title", title), text.to_owned(), 340.0)
}

fn stage_pad(ui: &mut Ui) {
    ui.add_space(24.0);
}

fn border_beam(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Border Beam", "A comet of light circles the card, forever.");
    let from = p.color("Head", hex("#22D3EE"));
    let to = p.color("Tail", hex("#A855F7"));
    let duration = p.f32("Seconds per lap", 6.0, 1.0..=15.0, 0.1);
    let length = p.f32("Beam length", 0.18, 0.03..=0.6, 0.01);
    let thickness = p.f32("Thickness", 1.5, 0.5..=6.0, 0.1);
    center(ui, |ui| {
        stage_pad(ui);
        BorderBeam::new()
            .width(w)
            .from(from)
            .to(to)
            .duration(duration)
            .length(length)
            .thickness(thickness)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::BorderBeam", "BorderBeam::new()")
        .f("width", w)
        .c("from", from)
        .c("to", to)
        .f("duration", duration)
        .f("length", length)
        .f("thickness", thickness)
        .show(&card_code(&title, &text))
}

fn shine_border(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Shine Border", "A soft gradient sweeps around the edge.");
    let a = p.color("Color 1", hex("#9E7AFF"));
    let b = p.color("Color 2", hex("#FE8BBB"));
    let c = p.color("Color 3", hex("#FFBD7A"));
    let duration = p.f32("Seconds per sweep", 8.0, 1.0..=20.0, 0.5);
    let thickness = p.f32("Thickness", 2.0, 0.5..=8.0, 0.1);
    let cols = [a, b, c];
    center(ui, |ui| {
        stage_pad(ui);
        ShineBorder::new()
            .width(w)
            .colors(&cols)
            .duration(duration)
            .thickness(thickness)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::ShineBorder", "ShineBorder::new()")
        .f("width", w)
        .call("colors", lit_colors(&cols))
        .f("duration", duration)
        .f("thickness", thickness)
        .show(&card_code(&title, &text))
}

fn magic_card(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Magic Card", "A gradient spotlight follows your pointer.");
    let from = p.color("Border from", hex("#9E7AFF"));
    let to = p.color("Border to", hex("#FE8BBB"));
    let gsize = p.f32("Spotlight size", 220.0, 60.0..=500.0, 5.0);
    let ambient = p.bool("Ambient (no pointer)", true);
    center(ui, |ui| {
        stage_pad(ui);
        MagicCard::new()
            .width(w)
            .min_height(200.0)
            .from(from)
            .to(to)
            .gradient_size(gsize)
            .ambient(ambient)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::MagicCard", "MagicCard::new()")
        .f("width", w)
        .call("min_height", "200.0")
        .c("from", from)
        .c("to", to)
        .f("gradient_size", gsize)
        .b("ambient", ambient)
        .show(&card_code(&title, &text))
}

fn spotlight_card(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Spotlight", "A soft light follows the pointer across the card.");
    let color = p.color("Light", hex("#8B5CF6"));
    let radius = p.f32("Light radius", 260.0, 60.0..=600.0, 5.0);
    let dots = p.bool("Dot pattern", false);
    let ambient = p.bool("Ambient (no pointer)", true);
    center(ui, |ui| {
        stage_pad(ui);
        SpotlightCard::new()
            .width(w)
            .min_height(200.0)
            .color(color)
            .radius_light(radius)
            .dots(dots)
            .ambient(ambient)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::SpotlightCard", "SpotlightCard::new()")
        .f("width", w)
        .call("min_height", "200.0")
        .c("color", color)
        .f("radius_light", radius)
        .b("dots", dots)
        .b("ambient", ambient)
        .show(&card_code(&title, &text))
}

fn tilt_card(ui: &mut Ui, p: &mut Params) -> String {
    let title = p.text("Title", "Tilt");
    let text = "Tilts toward the pointer in 3D, with a glare.".to_owned();
    let max = p.f32("Max tilt (deg)", 14.0, 0.0..=35.0, 0.5);
    let scale = p.f32("Hover scale", 1.04, 1.0..=1.2, 0.01);
    let parallax = p.f32("Parallax", 1.0, 0.0..=3.0, 0.05);
    let glare = p.bool("Glare", true);
    let ambient = p.bool("Ambient (no pointer)", true);
    center(ui, |ui| {
        stage_pad(ui);
        TiltCard::new()
            .size(vec2(280.0, 320.0))
            .max_tilt(max)
            .scale(scale)
            .parallax(parallax)
            .glare(glare)
            .ambient(ambient)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::TiltCard", "TiltCard::new()")
        .call("size", "vec2(280.0, 320.0)")
        .f("max_tilt", max)
        .f("scale", scale)
        .f("parallax", parallax)
        .b("glare", glare)
        .b("ambient", ambient)
        .show(&card_code(&title, &text))
}

fn electric_border(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Zap", "Crackling arcs of electricity trace the border.");
    let color = p.color("Arc color", hex("#22D3EE"));
    let speed = p.f32("Speed", 1.0, 0.1..=4.0, 0.05);
    let chaos = p.f32("Chaos", 1.0, 0.0..=2.0, 0.05);
    let thickness = p.f32("Thickness", 1.5, 0.5..=5.0, 0.1);
    center(ui, |ui| {
        stage_pad(ui);
        ElectricBorder::new()
            .width(w)
            .min_height(180.0)
            .color(color)
            .speed(speed)
            .chaos(chaos)
            .thickness(thickness)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::ElectricBorder", "ElectricBorder::new()")
        .f("width", w)
        .call("min_height", "180.0")
        .c("color", color)
        .f("speed", speed)
        .f("chaos", chaos)
        .f("thickness", thickness)
        .show(&card_code(&title, &text))
}

fn neon_card(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Neon", "Two neon colors chase each other around the card.");
    let first = p.color("First", hex("#FF2975"));
    let second = p.color("Second", hex("#00FFF1"));
    let border = p.f32("Border size", 2.0, 0.5..=8.0, 0.1);
    let duration = p.f32("Seconds per rotation", 4.0, 0.5..=12.0, 0.1);
    let glow = p.f32("Glow", 1.0, 0.0..=3.0, 0.05);
    center(ui, |ui| {
        ui.add_space(40.0);
        NeonGradientCard::new()
            .width(w)
            .min_height(180.0)
            .first(first)
            .second(second)
            .border_size(border)
            .duration(duration)
            .glow(glow)
            .show(ui, |ui| card_body(ui, &title, &text));
        ui.add_space(40.0);
    });
    Code::new("glowkit::effects::NeonGradientCard", "NeonGradientCard::new()")
        .f("width", w)
        .call("min_height", "180.0")
        .c("first", first)
        .c("second", second)
        .f("border_size", border)
        .f("duration", duration)
        .f("glow", glow)
        .show(&card_code(&title, &text))
}

fn gradient_border(ui: &mut Ui, p: &mut Params) -> String {
    let (title, text, w) = card_params(p, "Gradient Border", "A conic gradient spins around the edge.");
    let a = p.color("Color 1", hex("#40FFAA"));
    let b = p.color("Color 2", hex("#4079FF"));
    let c = p.color("Color 3", hex("#FF40D9"));
    let duration = p.f32("Seconds per rotation", 4.0, 0.5..=12.0, 0.1);
    let glow = p.bool("Glow", true);
    let cols = [a, b, c];
    center(ui, |ui| {
        ui.add_space(30.0);
        AnimatedGradientBorder::new()
            .width(w)
            .min_height(180.0)
            .colors(&cols)
            .duration(duration)
            .glow(glow)
            .show(ui, |ui| card_body(ui, &title, &text));
    });
    Code::new("glowkit::effects::AnimatedGradientBorder", "AnimatedGradientBorder::new()")
        .f("width", w)
        .call("min_height", "180.0")
        .call("colors", lit_colors(&cols))
        .f("duration", duration)
        .b("glow", glow)
        .show(&card_code(&title, &text))
}

// --------------------------------------------------------------------------------------------
// Cursor & interaction
// --------------------------------------------------------------------------------------------

/// A soft rounded area to click in.
fn click_area(ui: &mut Ui, h: f32, caption: &str) -> Rect {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(20),
        t.background,
        egui::Stroke::new(1.0, t.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(14.0),
        t.muted_foreground,
    );
    rect
}

fn click_spark(ui: &mut Ui, p: &mut Params) -> String {
    let color = p.color("Color", hex("#F472B6"));
    let count = p.count("Sparks", 10, 3..=32);
    let radius = p.f32("Radius", 32.0, 8.0..=120.0, 1.0);
    let length = p.f32("Spark length", 12.0, 2.0..=40.0, 0.5);
    let width = p.f32("Spark width", 2.0, 0.5..=6.0, 0.1);
    let duration = p.f32("Duration (s)", 0.45, 0.1..=2.0, 0.05);
    let rect = click_area(ui, 380.0, "Click anywhere");
    ClickSpark::new()
        .color(color)
        .count(count)
        .radius(radius)
        .length(length)
        .width(width)
        .duration(duration)
        .autoplay(true)
        .show(ui, rect);
    Code::new("glowkit::effects::ClickSpark", "ClickSpark::new()")
        .c("color", color)
        .call("count", count)
        .f("radius", radius)
        .f("length", length)
        .f("width", width)
        .f("duration", duration)
        .stmt("", ".show(ui, rect);")
}

fn confetti(ui: &mut Ui, p: &mut Params) -> String {
    let count = p.count("Pieces", 120, 10..=400);
    let spread = p.f32("Spread (deg)", 70.0, 10.0..=360.0, 1.0);
    let velocity = p.f32("Velocity", 1.0, 0.2..=3.0, 0.05);
    let gravity = p.f32("Gravity", 1.0, 0.0..=3.0, 0.05);
    let scalar = p.f32("Piece size", 1.0, 0.3..=3.0, 0.05);
    let area = click_area(ui, 380.0, "");
    let confetti = ConfettiBurst::new().count(count).spread(spread).velocity(velocity).gravity(gravity).scalar(scalar);
    let btn = Rect::from_center_size(area.center(), vec2(170.0, 44.0));
    let fired = ui.put(btn, egui::Button::new("🎉  Celebrate").corner_radius(22)).clicked();
    let first = flag(ui, "confetti_first", true);
    if fired || first {
        confetti.fire(ui, btn.center());
        set_flag(ui, "confetti_first", false);
    }
    confetti.show(ui, area);
    Code::new("glowkit::effects::ConfettiBurst", "ConfettiBurst::new()")
        .call("count", count)
        .f("spread", spread)
        .f("velocity", velocity)
        .f("gravity", gravity)
        .f("scalar", scalar)
        .stmt(
            "let confetti = ",
            ";\nif ui.button(\"🎉 Celebrate\").clicked() {\n    confetti.fire(ui, area.center());\n}\nconfetti.show(ui, area);",
        )
}

// --------------------------------------------------------------------------------------------
// Motion
// --------------------------------------------------------------------------------------------

fn dock(ui: &mut Ui, p: &mut Params) -> String {
    let mag = p.f32("Magnification", 1.75, 1.0..=3.0, 0.05);
    let size = p.f32("Icon size", 48.0, 28.0..=80.0, 1.0);
    let distance = p.f32("Distance", 140.0, 40.0..=320.0, 5.0);
    let style = p.choice("Style", &["Tiles", "Minimal"], 0);
    let labels = p.bool("Labels", true);
    let autoplay = p.bool("Autoplay", true);
    let items = [
        DockItem::new("🏠", "Home").colors(hex("#60A5FA"), hex("#2563EB")).active(true),
        DockItem::new("✉", "Mail").colors(hex("#38BDF8"), hex("#0284C7")),
        DockItem::new("🎵", "Music").colors(hex("#FB7185"), hex("#E11D48")),
        DockItem::new("📷", "Photos").colors(hex("#FBBF24"), hex("#F97316")),
        DockItem::new("💬", "Messages").colors(hex("#4ADE80"), hex("#16A34A")),
        DockItem::new("⚙", "Settings").colors(hex("#A1A1AA"), hex("#52525B")),
    ];
    ui.add_space(30.0);
    Dock::new(&items)
        .magnification(mag)
        .size(size)
        .distance(distance)
        .style([DockStyle::Tiles, DockStyle::Minimal][style])
        .labels(labels)
        .autoplay(autoplay)
        .show(ui);
    Code::new("glowkit::effects::{Dock, DockItem, DockStyle}", "Dock::new(&items)")
        .pre("let items = [\n    DockItem::new(\"🏠\", \"Home\").active(true),\n    DockItem::new(\"✉\", \"Mail\"),\n    DockItem::new(\"🎵\", \"Music\"),\n];")
        .f("magnification", mag)
        .f("size", size)
        .f("distance", distance)
        .call("style", variant("DockStyle", &["Tiles", "Minimal"], style))
        .b("labels", labels)
        .b("autoplay", autoplay)
        .stmt("let r = ", ".show(ui);\nif let Some(i) = r.clicked { /* open items[i] */ }")
}

fn marquee(ui: &mut Ui, p: &mut Params) -> String {
    let speed = p.f32("Speed (pt/s)", 50.0, 5.0..=200.0, 1.0);
    let reverse = p.bool("Reverse", false);
    let pause = p.bool("Pause on hover", true);
    let gap = p.f32("Gap", 24.0, 0.0..=80.0, 1.0);
    let fade = p.f32("Edge fade", 80.0, 0.0..=200.0, 1.0);
    let words = p.text("Items (comma separated)", "Rust, egui, WebAssembly, glowkit, Native, 60 fps");
    let list: Vec<String> = words.split(',').map(|w| w.trim().to_owned()).filter(|w| !w.is_empty()).collect();
    let t = Theme::of(ui);
    ui.add_space(60.0);
    Marquee::new().speed(speed).reverse(reverse).pause_on_hover(pause).gap(gap).fade(fade).show(ui, |ui| {
        for name in &list {
            egui::Frame::new()
                .fill(t.background)
                .stroke(t.border_stroke())
                .corner_radius(14)
                .inner_margin(egui::Margin::symmetric(18, 12))
                .show(ui, |ui| ui.label(egui::RichText::new(name).size(18.0).strong()));
        }
    });
    let lits: Vec<String> = list.iter().map(|w| lit_str(w)).collect();
    Code::new("glowkit::effects::Marquee", "Marquee::new()")
        .f("speed", speed)
        .b("reverse", reverse)
        .b("pause_on_hover", pause)
        .f("gap", gap)
        .f("fade", fade)
        .show(&format!("for name in [{}] {{\n    ui.label(name);\n}}", lits.join(", ")))
}

fn orbiting(ui: &mut Ui, p: &mut Params) -> String {
    let center_icon = p.text("Center", "✦");
    let inner_r = p.f32("Inner radius", 80.0, 40.0..=140.0, 1.0);
    let inner_period = p.f32("Inner period (s)", 12.0, 2.0..=40.0, 0.5);
    let outer_r = p.f32("Outer radius", 150.0, 100.0..=190.0, 1.0);
    let outer_period = p.f32("Outer period (s)", 24.0, 2.0..=60.0, 0.5);
    let path = p.bool("Show path", true);
    ui.add(
        OrbitingCircles::new()
            .center(&center_icon)
            .path(path)
            .ring(inner_r, &["🦀", "⚡", "🧩"], inner_period)
            .reverse_ring(outer_r, &["🌐", "🎨", "📦", "🔒", "🚀"], outer_period),
    );
    Code::new("glowkit::effects::OrbitingCircles", "OrbitingCircles::new()")
        .s("center", &center_icon)
        .b("path", path)
        .call("ring", format!("{}, &[\"🦀\", \"⚡\", \"🧩\"], {}", lit_f32(inner_r), lit_f32(inner_period)))
        .call(
            "reverse_ring",
            format!("{}, &[\"🌐\", \"🎨\", \"📦\", \"🔒\", \"🚀\"], {}", lit_f32(outer_r), lit_f32(outer_period)),
        )
        .add()
}

// --------------------------------------------------------------------------------------------
// Micro-interactions
// --------------------------------------------------------------------------------------------

fn like(ui: &mut Ui, p: &mut Params) -> String {
    let count = p.int("Count", 1299, 0..=99_999) as u64;
    let burst = p.choice("Burst", &["Bloom", "Firework"], 0);
    let burst_size = p.f32("Burst size", 1.0, 0.3..=3.0, 0.05);
    let color = p.color("Color", hex("#F43F5E"));
    let dark = p.bool("Dark", false);
    let autoplay = p.bool("Autoplay", true);
    let mut liked = flag(ui, "liked", false);
    center(ui, |ui| {
        ui.add_space(40.0);
        ui.add(
            LikeButton::new(&mut liked, count)
                .burst([LikeBurst::Bloom, LikeBurst::Firework][burst])
                .burst_size(burst_size)
                .color(color)
                .dark(dark)
                .autoplay(autoplay),
        );
    });
    set_flag(ui, "liked", liked);
    Code::new("glowkit::effects::{LikeBurst, LikeButton}", format!("LikeButton::new(&mut liked, {count})"))
        .call("burst", variant("LikeBurst", &["Bloom", "Firework"], burst))
        .f("burst_size", burst_size)
        .c("color", color)
        .b("dark", dark)
        .b("autoplay", autoplay)
        .add()
}

fn liquid_toggle(ui: &mut Ui, p: &mut Params) -> String {
    let width = p.f32("Width", 120.0, 60.0..=200.0, 1.0);
    let stretch = p.f32("Stretch", 36.0, 0.0..=80.0, 1.0);
    let on_color = p.color("On color", hex("#22C55E"));
    let stroke = p.bool("Stroke", true);
    let autoplay = p.bool("Autoplay", true);
    let mut on = flag(ui, "on", false);
    center(ui, |ui| {
        ui.add_space(60.0);
        ui.add(
            LiquidToggle::new(&mut on)
                .size(vec2(width, width / 2.0))
                .stretch(stretch)
                .on_color(on_color)
                .stroke(stroke)
                .autoplay(autoplay),
        );
    });
    set_flag(ui, "on", on);
    Code::new("glowkit::effects::LiquidToggle", "LiquidToggle::new(&mut on)")
        .call("size", format!("vec2({}, {})", lit_f32(width), lit_f32(width / 2.0)))
        .f("stretch", stretch)
        .c("on_color", on_color)
        .b("stroke", stroke)
        .b("autoplay", autoplay)
        .add()
}

fn slide_confirm(ui: &mut Ui, p: &mut Params) -> String {
    let l = p.text("Label", "Slide to confirm");
    let done = p.text("Confirmed label", "Confirmed");
    let width = p.f32("Width", 320.0, 200.0..=520.0, 5.0);
    let height = p.f32("Height", 60.0, 40.0..=90.0, 1.0);
    let dark = p.bool("Dark", false);
    let autoplay = p.bool("Autoplay", true);
    center(ui, |ui| {
        ui.add_space(60.0);
        ui.add(
            SlideToConfirm::new(&l).confirmed_label(&done).width(width).height(height).dark(dark).autoplay(autoplay),
        );
    });
    Code::new("glowkit::effects::SlideToConfirm", format!("SlideToConfirm::new({})", lit_str(&l)))
        .s("confirmed_label", &done)
        .f("width", width)
        .f("height", height)
        .b("dark", dark)
        .b("autoplay", autoplay)
        .stmt("if ui.add(", ").changed() {\n    // confirmed\n}")
}

fn emoji_reactions(ui: &mut Ui, p: &mut Params) -> String {
    let msg = p.text("Message", "Merged the dark mode PR");
    let set = p.choice("Set", &["Work", "Classic", "Hearts"], 0);
    let magnify = p.f32("Magnify", 50.0, 0.0..=120.0, 1.0);
    let bounce = p.f32("Bounce", 1.0, 0.0..=3.0, 0.05);
    let autoplay = p.bool("Autoplay", true);
    center(ui, |ui| {
        ui.add_space(40.0);
        ui.add(
            EmojiReactions::new(&msg)
                .set([ReactionSet::Work, ReactionSet::Classic, ReactionSet::Hearts][set])
                .magnify(magnify)
                .bounce(bounce)
                .autoplay(autoplay),
        );
    });
    let _ = (hint, lit_color);
    Code::new("glowkit::effects::{EmojiReactions, ReactionSet}", format!("EmojiReactions::new({})", lit_str(&msg)))
        .call("set", variant("ReactionSet", &["Work", "Classic", "Hearts"], set))
        .f("magnify", magnify)
        .f("bounce", bounce)
        .b("autoplay", autoplay)
        .add()
}
