//! Background playgrounds.

use super::{Code, Params, Playground};
use glowkit::backgrounds::*;
use glowkit::color::hex;
use glowkit::core::playground::{lit_colors, lit_f32};
use glowkit::egui::{vec2, Ui};

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "aurora", show: aurora },
        Playground { id: "particles", show: particles },
        Playground { id: "retro-grid", show: retro_grid },
        Playground { id: "meteors", show: meteors },
        Playground { id: "flickering-grid", show: flickering },
        Playground { id: "dot-grid", show: dot_grid },
    ]
}

fn height(_: &mut Params) -> f32 {
    400.0
}

fn size_code(h: f32) -> String {
    format!("vec2(ui.available_width(), {})", lit_f32(h))
}

fn aurora(ui: &mut Ui, p: &mut Params) -> String {
    let a = p.color("Color 1", hex("#3A29FF"));
    let b = p.color("Color 2", hex("#FF94B4"));
    let c = p.color("Color 3", hex("#FF3232"));
    let speed = p.f32("Speed", 1.0, 0.1..=4.0, 0.1);
    let amp = p.f32("Amplitude", 1.0, 0.0..=3.0, 0.05);
    let blend = p.f32("Blend", 0.9, 0.0..=1.0, 0.01);
    let h = height(p);
    let cols = [a, b, c];
    ui.add(Aurora::new().colors(&cols).speed(speed).amplitude(amp).blend(blend).size(vec2(ui.available_width(), h)));
    Code::new("glowkit::backgrounds::Aurora", "Aurora::new()")
        .call("colors", lit_colors(&cols))
        .f("speed", speed)
        .f("amplitude", amp)
        .f("blend", blend)
        .call("size", size_code(h))
        .add()
}

fn particles(ui: &mut Ui, p: &mut Params) -> String {
    let count = p.count("Count", 140, 10..=400);
    let speed = p.f32("Speed", 1.0, 0.0..=4.0, 0.05);
    let size = p.f32("Particle size", 2.0, 0.5..=6.0, 0.1);
    let links = p.bool("Links", true);
    let twinkle = p.bool("Twinkle", false);
    let repel = p.f32("Pointer repel", 0.0, 0.0..=3.0, 0.05);
    let h = height(p);
    ui.add(
        Particles::new()
            .count(count)
            .speed(speed)
            .particle_size(size)
            .links(links)
            .twinkle(twinkle)
            .repel(repel)
            .size(vec2(ui.available_width(), h)),
    );
    Code::new("glowkit::backgrounds::Particles", "Particles::new()")
        .call("count", count)
        .f("speed", speed)
        .f("particle_size", size)
        .b("links", links)
        .b("twinkle", twinkle)
        .f("repel", repel)
        .call("size", size_code(h))
        .add()
}

fn retro_grid(ui: &mut Ui, p: &mut Params) -> String {
    let color = p.color("Line color", hex("#FF2E97"));
    let horizon = p.f32("Horizon", 0.45, 0.1..=0.9, 0.01);
    let cell = p.f32("Cell size", 60.0, 20.0..=140.0, 1.0);
    let speed = p.f32("Speed", 1.0, 0.0..=4.0, 0.05);
    let glow = p.bool("Glow", true);
    let h = height(p);
    ui.add(
        RetroGrid::new()
            .color(color)
            .horizon(horizon)
            .cell_size(cell)
            .speed(speed)
            .glow(glow)
            .size(vec2(ui.available_width(), h)),
    );
    Code::new("glowkit::backgrounds::RetroGrid", "RetroGrid::new()")
        .c("color", color)
        .f("horizon", horizon)
        .f("cell_size", cell)
        .f("speed", speed)
        .b("glow", glow)
        .call("size", size_code(h))
        .add()
}

fn meteors(ui: &mut Ui, p: &mut Params) -> String {
    let count = p.count("Count", 20, 1..=80);
    let angle = p.f32("Angle", 215.0, 180.0..=270.0, 1.0);
    let tail = p.f32("Tail length", 80.0, 10.0..=260.0, 1.0);
    let speed = p.f32("Speed", 1.0, 0.1..=4.0, 0.05);
    let color = p.color("Color", hex("#A1A1AA"));
    let h = height(p);
    ui.add(
        Meteors::new()
            .count(count)
            .angle(angle)
            .tail(tail)
            .speed(speed)
            .color(color)
            .size(vec2(ui.available_width(), h)),
    );
    Code::new("glowkit::backgrounds::Meteors", "Meteors::new()")
        .call("count", count)
        .f("angle", angle)
        .f("tail", tail)
        .f("speed", speed)
        .c("color", color)
        .call("size", size_code(h))
        .add()
}

fn flickering(ui: &mut Ui, p: &mut Params) -> String {
    let sq = p.f32("Square size", 4.0, 1.0..=16.0, 0.5);
    let gap = p.f32("Gap", 6.0, 1.0..=20.0, 0.5);
    let chance = p.f32("Flicker chance", 0.3, 0.0..=1.0, 0.01);
    let max = p.f32("Max opacity", 0.3, 0.05..=1.0, 0.01);
    let color = p.color("Color", hex("#7C3AED"));
    let mask = p.bool("Radial mask", false);
    let h = height(p);
    ui.add(
        FlickeringGrid::new()
            .square_size(sq)
            .gap(gap)
            .flicker_chance(chance)
            .max_opacity(max)
            .color(color)
            .mask(mask)
            .size(vec2(ui.available_width(), h)),
    );
    Code::new("glowkit::backgrounds::FlickeringGrid", "FlickeringGrid::new()")
        .f("square_size", sq)
        .f("gap", gap)
        .f("flicker_chance", chance)
        .f("max_opacity", max)
        .c("color", color)
        .b("mask", mask)
        .call("size", size_code(h))
        .add()
}

fn dot_grid(ui: &mut Ui, p: &mut Params) -> String {
    let gap = p.f32("Gap", 28.0, 10.0..=60.0, 1.0);
    let dot = p.f32("Dot size", 4.0, 1.0..=12.0, 0.5);
    let prox = p.f32("Proximity", 140.0, 20.0..=320.0, 5.0);
    let strength = p.f32("Strength", 1.0, 0.0..=3.0, 0.05);
    let active = p.color("Active color", hex("#7C3AED"));
    let h = height(p);
    ui.add(
        DotGrid::new()
            .gap(gap)
            .dot_size(dot)
            .proximity(prox)
            .strength(strength)
            .active_color(active)
            .size(vec2(ui.available_width(), h)),
    );
    Code::new("glowkit::backgrounds::DotGrid", "DotGrid::new()")
        .f("gap", gap)
        .f("dot_size", dot)
        .f("proximity", prox)
        .f("strength", strength)
        .c("active_color", active)
        .call("size", size_code(h))
        .add()
}
