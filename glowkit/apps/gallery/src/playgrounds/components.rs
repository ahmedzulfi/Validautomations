//! Everyday component playgrounds (loaders & progress).

use super::{center, variant, Code, Params, Playground};
use glowkit::color::hex;
use glowkit::components::*;
use glowkit::core::playground::{lit_color, lit_f32};
use glowkit::egui::Ui;

pub fn all() -> Vec<Playground> {
    vec![Playground { id: "spinner", show: spinner }, Playground { id: "progress-ring", show: progress_ring }]
}

const KINDS: [&str; 18] = [
    "Ring",
    "Arc",
    "Gradient",
    "DualRing",
    "Dots",
    "Bars",
    "Pulse",
    "Orbit",
    "Bounce",
    "Wave",
    "Grid",
    "Infinity",
    "Ripple",
    "Clock",
    "Typing",
    "SquareFlip",
    "Helix",
    "Spiral",
];

fn spinner(ui: &mut Ui, p: &mut Params) -> String {
    let kind = p.choice("Kind", &KINDS, 7);
    let size = p.f32("Size", 64.0, 16.0..=160.0, 1.0);
    let speed = p.f32("Speed", 1.0, 0.2..=4.0, 0.05);
    let custom = p.bool("Custom color", false);
    let color = p.color("Color", hex("#22C55E"));
    let k = [
        SpinnerKind::Ring,
        SpinnerKind::Arc,
        SpinnerKind::Gradient,
        SpinnerKind::DualRing,
        SpinnerKind::Dots,
        SpinnerKind::Bars,
        SpinnerKind::Pulse,
        SpinnerKind::Orbit,
        SpinnerKind::Bounce,
        SpinnerKind::Wave,
        SpinnerKind::Grid,
        SpinnerKind::Infinity,
        SpinnerKind::Ripple,
        SpinnerKind::Clock,
        SpinnerKind::Typing,
        SpinnerKind::SquareFlip,
        SpinnerKind::Helix,
        SpinnerKind::Spiral,
    ][kind];
    center(ui, |ui| {
        ui.add_space(60.0);
        let mut s = Spinner::new(k).size(size).speed(speed);
        if custom {
            s = s.color(color);
        }
        ui.add(s);
    });
    Code::new(
        "glowkit::components::{Spinner, SpinnerKind}",
        format!("Spinner::new({})", variant("SpinnerKind", &KINDS, kind)),
    )
    .f("size", size)
    .f("speed", speed)
    .call_if(custom, "color", lit_color(color))
    .add()
}

fn progress_ring(ui: &mut Ui, p: &mut Params) -> String {
    let value = p.f32("Value", 0.72, 0.0..=1.0, 0.01);
    let size = p.f32("Size", 140.0, 40.0..=260.0, 1.0);
    let thickness = p.f32("Thickness", 12.0, 2.0..=32.0, 0.5);
    let gradient = p.bool("Gradient", true);
    let sub = p.text("Sublabel", "Complete");
    center(ui, |ui| {
        ui.add_space(40.0);
        ui.add(ProgressRing::new(value).size(size).thickness(thickness).gradient(gradient).sublabel(&sub))
    });
    Code::new("glowkit::components::ProgressRing", format!("ProgressRing::new({})", lit_f32(value)))
        .f("size", size)
        .f("thickness", thickness)
        .b("gradient", gradient)
        .s("sublabel", &sub)
        .add()
}
