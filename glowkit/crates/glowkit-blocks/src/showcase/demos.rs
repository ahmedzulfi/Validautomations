use super::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::{mix, paint, Theme};

/// Lay out children in a horizontally centered row of total width `w`.
fn row(ui: &mut Ui, w: f32, top: f32, add: impl FnOnce(&mut Ui)) {
    ui.add_space(top);
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        ui.add_space(((avail - w) / 2.0).max(0.0));
        add(ui);
    });
}

/// Painted fake landing page used inside the device mockups.
pub(crate) fn mock_site(ui: &mut Ui) {
    let t = Theme::of(ui);
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let s = (r.width() / 600.0).clamp(0.35, 1.4);
    // backdrop glow
    paint::glow(&p, pos2(r.center().x, r.min.y + 40.0 * s), r.width() * 0.6, with_alpha(t.gradient[0], if t.dark { 0.25 } else { 0.12 }));
    // nav
    let ny = r.min.y + 22.0 * s;
    p.circle_filled(pos2(r.min.x + 22.0 * s, ny), 6.0 * s, t.accent);
    super::util::bold_text(&p, pos2(r.min.x + 34.0 * s, ny), Align2::LEFT_CENTER, "glowkit", FontId::proportional(13.0 * s), t.foreground);
    if r.width() > 300.0 {
        let mut x = r.max.x - 20.0 * s;
        for l in ["Blog", "Components", "Docs"] {
            let lr = p.text(pos2(x, ny), Align2::RIGHT_CENTER, l, FontId::proportional(11.0 * s), t.muted_foreground);
            x = lr.min.x - 18.0 * s;
        }
    }
    // hero
    let hy = r.min.y + r.height() * 0.3;
    let chip = Rect::from_center_size(pos2(r.center().x, hy - 30.0 * s), vec2(150.0 * s, 20.0 * s));
    p.rect_filled(chip, 99.0, t.muted);
    p.rect_stroke(chip, 99.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.text(chip.center(), Align2::CENTER_CENTER, "✨ Introducing glowkit", FontId::proportional(9.5 * s), t.muted_foreground);
    let g = p.layout_no_wrap("Ship UIs that glow".into(), FontId::proportional(34.0 * s), Color32::WHITE);
    let gp = pos2(r.center().x - g.size().x / 2.0, hy - 10.0 * s);
    let gr = Rect::from_min_size(gp, g.size());
    let stops = [t.gradient[0], t.gradient[1], t.gradient[2]];
    paint::galley_with(&p, gp + vec2(0.5, 0.0), &g, paint::linear_fn(gr, 0.0, &stops));
    paint::galley_with(&p, gp, &g, paint::linear_fn(gr, 0.0, &stops));
    p.text(pos2(r.center().x, gr.max.y + 14.0 * s), Align2::CENTER_CENTER, "Animated components for Rust, on every platform.", FontId::proportional(11.5 * s), t.muted_foreground);
    let by = gr.max.y + 42.0 * s;
    let b1 = Rect::from_center_size(pos2(r.center().x - 52.0 * s, by), vec2(96.0 * s, 26.0 * s));
    let b2 = Rect::from_center_size(pos2(r.center().x + 52.0 * s, by), vec2(96.0 * s, 26.0 * s));
    p.rect_filled(b1, 7.0 * s, t.primary);
    p.text(b1.center(), Align2::CENTER_CENTER, "Get started", FontId::proportional(11.0 * s), t.primary_foreground);
    p.rect_filled(b2, 7.0 * s, t.muted);
    p.rect_stroke(b2, 7.0 * s, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.text(b2.center(), Align2::CENTER_CENTER, "GitHub", FontId::proportional(11.0 * s), t.foreground);
    // cards
    let cy = by + 30.0 * s;
    let n = if r.width() > 320.0 { 3 } else { 2 };
    let gap = 12.0 * s;
    let cw = (r.width() - 40.0 * s - gap * (n as f32 - 1.0)) / n as f32;
    for i in 0..n {
        let cr = Rect::from_min_size(pos2(r.min.x + 20.0 * s + i as f32 * (cw + gap), cy), vec2(cw, (r.max.y - cy - 16.0 * s).max(40.0)));
        p.rect_filled(cr, 10.0 * s, t.card);
        p.rect_stroke(cr, 10.0 * s, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let ir = Rect::from_min_size(cr.min + vec2(10.0, 10.0) * s, vec2(cw - 20.0 * s, (cr.height() * 0.45).min(70.0 * s)));
        let c0 = t.chart[i % 6];
        paint::gradient_rect(&p, ir, 6.0 * s, 35.0, &[with_alpha(c0, 0.9), with_alpha(mix(c0, t.gradient[1], 0.5), 0.5)]);
        p.rect_filled(Rect::from_min_size(pos2(ir.min.x, ir.max.y + 10.0 * s), vec2(ir.width() * 0.7, 7.0 * s)), 3.0, with_alpha(t.foreground, 0.6));
        p.rect_filled(Rect::from_min_size(pos2(ir.min.x, ir.max.y + 22.0 * s), vec2(ir.width() * 0.5, 6.0 * s)), 3.0, with_alpha(t.muted_foreground, 0.5));
    }
}

/// Painted fake phone app (a music / activity feed).
pub(crate) fn mock_app(ui: &mut Ui, hue: usize) {
    let t = Theme::of(ui);
    let r = ui.max_rect();
    let p = ui.painter().clone();
    let w = r.width();
    let c0 = t.chart[hue % 6];
    let c1 = t.chart[(hue + 2) % 6];
    let hero = Rect::from_min_size(r.min, vec2(w, r.height() * 0.48));
    paint::gradient_rect(&p, hero, 0.0, 90.0, &[c0, c0.lerp_to_gamma(c1, 0.6), t.background]);
    paint::glow(&p, hero.center() - vec2(w * 0.2, 0.0), w * 0.6, with_alpha(Color32::WHITE, 0.15));
    let s = w / 200.0;
    super::util::bold_text(&p, pos2(r.min.x + 16.0 * s, hero.max.y - 64.0 * s), Align2::LEFT_BOTTOM, "Good morning", FontId::proportional(20.0 * s), t.foreground);
    p.text(pos2(r.min.x + 16.0 * s, hero.max.y - 44.0 * s), Align2::LEFT_CENTER, "3 new updates", FontId::proportional(11.0 * s), t.muted_foreground);
    for i in 0..5 {
        let y = hero.max.y - 20.0 * s + i as f32 * 46.0 * s;
        let row = Rect::from_min_size(pos2(r.min.x + 12.0 * s, y), vec2(w - 24.0 * s, 38.0 * s));
        p.rect_filled(row, 9.0 * s, t.card);
        p.rect_stroke(row, 9.0 * s, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let cc = t.chart[(hue + i + 1) % 6];
        p.rect_filled(Rect::from_center_size(pos2(row.min.x + 20.0 * s, row.center().y), vec2(24.0, 24.0) * s), 6.0 * s, cc);
        p.rect_filled(Rect::from_min_size(pos2(row.min.x + 40.0 * s, row.center().y - 8.0 * s), vec2(row.width() * (0.55 - i as f32 * 0.04), 6.0 * s)), 3.0, with_alpha(t.foreground, 0.7));
        p.rect_filled(Rect::from_min_size(pos2(row.min.x + 40.0 * s, row.center().y + 3.0 * s), vec2(row.width() * 0.35, 5.0 * s)), 3.0, with_alpha(t.muted_foreground, 0.5));
    }
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "terminal",
            name: "Terminal",
            category: Category::DataDisplay,
            description: "A macOS terminal window that types out commands, then reveals output line by line with spinners that resolve to check marks.",
            covers: &["Magic UI: Terminal", "Aceternity: Terminal"],
            code: r##"ui.add(Terminal::new()
    .title("glowkit — zsh")
    .line(TerminalLine::command("cargo add glowkit"))
    .line(TerminalLine::output("    Updating crates.io index"))
    .line(TerminalLine::spinner("Resolving dependencies", "Resolved 42 packages"))
    .line(TerminalLine::success("Added glowkit v0.1.0"))
    .line(TerminalLine::command("cargo run --release"))
    .line(TerminalLine::success("Your app is glowing ✨")));"##,
            show: |ui| {
                let w = 600.0;
                row(ui, w, 10.0, |ui| {
                    ui.add(
                        Terminal::new()
                            .size(w, 340.0)
                            .title("glowkit — zsh")
                            .line(TerminalLine::command("cargo add glowkit"))
                            .line(TerminalLine::output("    Updating crates.io index"))
                            .line(TerminalLine::spinner("Resolving dependencies", "Resolved 42 packages"))
                            .line(TerminalLine::success("Added glowkit v0.1.0 (egui 0.37)"))
                            .line(TerminalLine::output("      Features: text, backgrounds, effects, blocks"))
                            .line(TerminalLine::command("cargo run --release"))
                            .line(TerminalLine::spinner("Compiling glowkit-demo", "Finished release in 4.2s"))
                            .line(TerminalLine::success("Your app is glowing ✨")),
                    );
                });
            },
        },
        Demo {
            id: "globe",
            name: "Globe",
            category: Category::DataDisplay,
            description: "A rotating dotted globe with rim glow, pulsing markers and arcs that fly between cities. Drag to spin it.",
            covers: &["Magic UI: Globe", "Aceternity: GitHub Globe", "cobe"],
            code: r##"let cities = [GlobeMarker::new(37.77, -122.42), GlobeMarker::new(51.51, -0.13),
              GlobeMarker::new(35.68, 139.69), GlobeMarker::new(-33.87, 151.21)];
ui.add(Globe::new().radius(170.0).markers(&cities).arcs(&[(0, 1), (1, 2), (2, 3), (0, 2)]));"##,
            show: |ui| {
                let cities = [
                    GlobeMarker::new(37.77, -122.42),
                    GlobeMarker::new(51.51, -0.13),
                    GlobeMarker::new(35.68, 139.69),
                    GlobeMarker::new(-33.87, 151.21),
                    GlobeMarker::new(-23.55, -46.63),
                    GlobeMarker::new(19.08, 72.88),
                    GlobeMarker::new(40.71, -74.0),
                ];
                ui.vertical_centered(|ui| {
                    ui.add(Globe::new().radius(175.0).markers(&cities).arcs(&[(0, 1), (1, 2), (2, 3), (6, 4), (1, 5), (5, 2), (6, 1)]));
                });
            },
        },
        Demo {
            id: "icon-cloud",
            name: "Icon Cloud",
            category: Category::Motion,
            description: "A sphere of icon tiles rotating in 3D, scaled and faded by depth. It drifts toward the pointer; drag to fling it.",
            covers: &["Magic UI: Icon Cloud", "React Bits: Sphere"],
            code: r##"ui.add(IconCloud::new(vec![
    CloudIcon::new("Rs", hex("#F74C00")),
    CloudIcon::new("Go", hex("#00ADD8")),
    CloudIcon::new("TS", hex("#3178C6")),
    // ...
]).radius(150.0));"##,
            show: |ui| {
                let icons: Vec<CloudIcon> = [
                    ("Rs", "#F74C00"), ("Go", "#00ADD8"), ("TS", "#3178C6"), ("JS", "#F7DF1E"), ("Py", "#3776AB"),
                    ("C++", "#00599C"), ("Kt", "#7F52FF"), ("Sw", "#F05138"), ("Rb", "#CC342D"), ("λ", "#5E5086"),
                    ("Zig", "#F7A41D"), ("Lua", "#2C2D72"), ("Ex", "#6E4A7E"), ("C#", "#239120"), ("Dt", "#0175C2"),
                    ("Wasm", "#654FF0"), ("Gl", "#8B5CF6"), ("Ng", "#DD0031"), ("Vue", "#42B883"), ("⚛", "#149ECA"),
                    ("Sv", "#FF3E00"), ("Pg", "#336791"), ("Rd", "#DC382D"), ("Dk", "#2496ED"), ("K8", "#326CE5"),
                    ("Gh", "#24292F"), ("Fi", "#FFCA28"), ("Np", "#CB3837"), ("Vi", "#646CFF"), ("Tw", "#06B6D4"),
                    ("Ex", "#000000"), ("Jl", "#9558B2"), ("Hs", "#5D4F85"), ("Nx", "#111111"), ("Bn", "#FBF0DF"),
                ]
                .iter()
                .map(|(g, c)| CloudIcon::new(*g, hex(c)))
                .collect();
                ui.vertical_centered(|ui| {
                    ui.add(IconCloud::new(icons).radius(165.0).tile(42.0));
                });
            },
        },
        Demo {
            id: "safari-mockup",
            name: "Safari Window",
            category: Category::Blocks,
            description: "A browser window mockup with Safari-style chrome and address bar; your UI renders, clipped, inside the page.",
            covers: &["Magic UI: Safari", "Aceternity: Browser mockup"],
            code: r##"SafariWindow::new().url("glowkit.rs").size(700.0, 400.0).show(ui, |ui| {
    my_landing_page(ui);
});"##,
            show: |ui| {
                let w = (ui.available_width() - 40.0).min(720.0);
                row(ui, w, 6.0, |ui| {
                    SafariWindow::new().url("glowkit.rs").size(w, 400.0).show(ui, mock_site);
                });
                ui.add_space(30.0);
            },
        },
        Demo {
            id: "phone-mockup",
            name: "Phone Mockup",
            category: Category::Blocks,
            description: "Smartphone frames (dynamic-island and punch-hole styles) with status bar and home indicator; content is clipped to the rounded screen.",
            covers: &["Magic UI: iPhone 15 Pro", "Magic UI: Android", "Aceternity: Device mockup"],
            code: r##"PhoneFrame::new().width(200.0).show(ui, |ui| my_app(ui));
PhoneFrame::new().style(PhoneStyle::PunchHole).show(ui, |ui| my_app(ui));"##,
            show: |ui| {
                row(ui, 3.0 * 196.0 + 2.0 * 40.0, 6.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 40.0;
                    PhoneFrame::new().width(188.0).show(ui, |ui| mock_app(ui, 0));
                    PhoneFrame::new().width(188.0).style(PhoneStyle::PunchHole).show(ui, |ui| mock_app(ui, 2));
                    PhoneFrame::new().width(188.0).show(ui, |ui| mock_app(ui, 4));
                });
                ui.add_space(20.0);
            },
        },
        Demo {
            id: "laptop-mockup",
            name: "Laptop Mockup",
            category: Category::Blocks,
            description: "A laptop with thin bezels, webcam and aluminium base; your content renders inside the display.",
            covers: &["Magic UI: Laptop mockup", "Aceternity: Macbook Scroll (static frame)"],
            code: r##"LaptopFrame::new().width(560.0).show(ui, |ui| my_dashboard(ui));"##,
            show: |ui| {
                let w = 560.0;
                row(ui, w * 1.18, 6.0, |ui| {
                    LaptopFrame::new().width(w).show(ui, mock_site);
                });
                ui.add_space(20.0);
            },
        },
    ]
}
