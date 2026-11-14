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
    let s = (r.width() / 600.0).clamp(0.35, 1.1);
    // backdrop glow
    paint::glow(
        &p,
        pos2(r.center().x, r.min.y + 40.0 * s),
        r.width() * 0.6,
        with_alpha(t.gradient[0], if t.dark { 0.25 } else { 0.12 }),
    );
    // nav
    let ny = r.min.y + 22.0 * s;
    p.circle_filled(pos2(r.min.x + 22.0 * s, ny), 6.0 * s, t.accent);
    super::util::bold_text(
        &p,
        pos2(r.min.x + 34.0 * s, ny),
        Align2::LEFT_CENTER,
        "glowkit",
        FontId::proportional(13.0 * s),
        t.foreground,
    );
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
    p.text(
        chip.center(),
        Align2::CENTER_CENTER,
        "✨ Introducing glowkit",
        FontId::proportional(9.5 * s),
        t.muted_foreground,
    );
    let g = p.layout_no_wrap("Ship UIs that glow".into(), FontId::proportional(34.0 * s), Color32::WHITE);
    let gp = pos2(r.center().x - g.size().x / 2.0, hy - 10.0 * s);
    let gr = Rect::from_min_size(gp, g.size());
    let stops = [t.gradient[0], t.gradient[1], t.gradient[2]];
    paint::galley_with(&p, gp + vec2(0.5, 0.0), &g, paint::linear_fn(gr, 0.0, &stops));
    paint::galley_with(&p, gp, &g, paint::linear_fn(gr, 0.0, &stops));
    p.text(
        pos2(r.center().x, gr.max.y + 14.0 * s),
        Align2::CENTER_CENTER,
        "Animated components for Rust, on every platform.",
        FontId::proportional(11.5 * s),
        t.muted_foreground,
    );
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
        let cr = Rect::from_min_size(
            pos2(r.min.x + 20.0 * s + i as f32 * (cw + gap), cy),
            vec2(cw, (r.max.y - cy - 16.0 * s).max(40.0)),
        );
        p.rect_filled(cr, 10.0 * s, t.card);
        p.rect_stroke(cr, 10.0 * s, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let ir =
            Rect::from_min_size(cr.min + vec2(10.0, 10.0) * s, vec2(cw - 20.0 * s, (cr.height() * 0.45).min(70.0 * s)));
        let c0 = t.chart[i % 6];
        paint::gradient_rect(
            &p,
            ir,
            6.0 * s,
            35.0,
            &[with_alpha(c0, 0.9), with_alpha(mix(c0, t.gradient[1], 0.5), 0.5)],
        );
        p.rect_filled(
            Rect::from_min_size(pos2(ir.min.x, ir.max.y + 10.0 * s), vec2(ir.width() * 0.7, 7.0 * s)),
            3.0,
            with_alpha(t.foreground, 0.6),
        );
        p.rect_filled(
            Rect::from_min_size(pos2(ir.min.x, ir.max.y + 22.0 * s), vec2(ir.width() * 0.5, 6.0 * s)),
            3.0,
            with_alpha(t.muted_foreground, 0.5),
        );
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
    super::util::bold_text(
        &p,
        pos2(r.min.x + 16.0 * s, hero.max.y - 64.0 * s),
        Align2::LEFT_BOTTOM,
        "Good morning",
        FontId::proportional(20.0 * s),
        t.foreground,
    );
    p.text(
        pos2(r.min.x + 16.0 * s, hero.max.y - 44.0 * s),
        Align2::LEFT_CENTER,
        "3 new updates",
        FontId::proportional(11.0 * s),
        t.muted_foreground,
    );
    for i in 0..5 {
        let y = hero.max.y - 20.0 * s + i as f32 * 46.0 * s;
        let row = Rect::from_min_size(pos2(r.min.x + 12.0 * s, y), vec2(w - 24.0 * s, 38.0 * s));
        p.rect_filled(row, 9.0 * s, t.card);
        p.rect_stroke(row, 9.0 * s, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let cc = t.chart[(hue + i + 1) % 6];
        p.rect_filled(
            Rect::from_center_size(pos2(row.min.x + 20.0 * s, row.center().y), vec2(24.0, 24.0) * s),
            6.0 * s,
            cc,
        );
        p.rect_filled(
            Rect::from_min_size(
                pos2(row.min.x + 40.0 * s, row.center().y - 8.0 * s),
                vec2(row.width() * (0.55 - i as f32 * 0.04), 6.0 * s),
            ),
            3.0,
            with_alpha(t.foreground, 0.7),
        );
        p.rect_filled(
            Rect::from_min_size(
                pos2(row.min.x + 40.0 * s, row.center().y + 3.0 * s),
                vec2(row.width() * 0.35, 5.0 * s),
            ),
            3.0,
            with_alpha(t.muted_foreground, 0.5),
        );
    }
}

/// A mountain ridge silhouette filling from a noisy skyline down to the bottom of `rect`.
fn ridge(p: &egui::Painter, rect: Rect, seed: f32, base: f32, amp: f32, top: Color32, bottom: Color32) {
    let n = 80;
    let mut m = egui::epaint::Mesh::default();
    for i in 0..=n {
        let u = i as f32 / n as f32;
        let h = glowkit_core::noise::fbm3(u * 3.0 + seed, seed * 1.7, 0.0, 4) * 0.5 + (u * 6.0 + seed).sin() * 0.12;
        let x = rect.min.x + u * rect.width();
        let y = rect.min.y + rect.height() * (base - amp * h);
        m.colored_vertex(pos2(x, y), top);
        m.colored_vertex(pos2(x, rect.max.y), bottom);
    }
    for i in 0..n as u32 {
        m.add_triangle(i * 2, i * 2 + 1, i * 2 + 3);
        m.add_triangle(i * 2, i * 2 + 3, i * 2 + 2);
    }
    p.add(egui::Shape::mesh(m));
}

/// Painted landscape for the comparison demo: flat "raw" or a vivid graded sunset.
fn landscape(p: &egui::Painter, rect: Rect, vivid: bool) {
    let r = 14.0;
    if vivid {
        paint::gradient_rect(p, rect, r, 90.0, &[hex("#1E1B4B"), hex("#7C3AED"), hex("#EC4899"), hex("#F59E0B")]);
        for i in 0..40u64 {
            let x = glowkit_core::rng::hash01(i * 3 + 1);
            let y = glowkit_core::rng::hash01(i * 5 + 2) * 0.35;
            p.circle_filled(
                pos2(rect.min.x + x * rect.width(), rect.min.y + y * rect.height()),
                0.6 + glowkit_core::rng::hash01(i) * 0.9,
                Color32::from_white_alpha(170),
            );
        }
        let sun = pos2(rect.center().x + rect.width() * 0.12, rect.min.y + rect.height() * 0.62);
        paint::glow(p, sun, rect.height() * 0.7, with_alpha(hex("#FDBA74"), 0.55));
        p.circle_filled(sun, rect.height() * 0.11, hex("#FFF1D6"));
        ridge(p, rect, 1.0, 0.62, 0.3, hex("#9D4EDD"), hex("#5B21B6"));
        ridge(p, rect, 4.0, 0.74, 0.25, hex("#4C1D95"), hex("#2E1065"));
        ridge(p, rect, 9.0, 0.88, 0.18, hex("#1E0B3B"), hex("#0B0618"));
    } else {
        paint::gradient_rect(p, rect, r, 90.0, &[hex("#9CA3AF"), hex("#CBD5E1"), hex("#E5E7EB")]);
        let sun = pos2(rect.center().x + rect.width() * 0.12, rect.min.y + rect.height() * 0.62);
        p.circle_filled(sun, rect.height() * 0.11, hex("#F3F4F6"));
        ridge(p, rect, 1.0, 0.62, 0.3, hex("#94A3B8"), hex("#64748B"));
        ridge(p, rect, 4.0, 0.74, 0.25, hex("#64748B"), hex("#475569"));
        ridge(p, rect, 9.0, 0.88, 0.18, hex("#334155"), hex("#1E293B"));
    }
}

/// Small mock dashboard drawn in an explicit theme (used by the theme toggler demo).
fn mini_app(ui: &mut Ui, rect: Rect, th: &Theme, dark: &mut bool) {
    let p = ui.painter().clone();
    paint::soft_shadow(&p, rect, 14.0, 24.0, vec2(0.0, 10.0), Color32::from_black_alpha(60));
    p.rect_filled(rect, 14.0, th.background);
    p.rect_stroke(rect, 14.0, Stroke::new(1.0, th.border), egui::StrokeKind::Inside);
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), 58.0));
    p.line_segment([head.left_bottom(), head.right_bottom()], Stroke::new(1.0, th.border));
    p.circle_filled(pos2(head.min.x + 26.0, head.center().y), 8.0, th.accent);
    super::util::bold_text(
        &p,
        pos2(head.min.x + 42.0, head.center().y),
        Align2::LEFT_CENTER,
        "Acme Analytics",
        FontId::proportional(15.0),
        th.foreground,
    );
    let tr = Rect::from_center_size(pos2(head.max.x - 32.0, head.center().y), vec2(38.0, 38.0));
    let area = rect;
    ui.put(tr, ThemeToggler::new(dark).size(38.0).reveal(area).reveal_rounding(14.0));
    let labels = [("Revenue", "$48,210", "+12.4%"), ("Users", "9,481", "+4.1%"), ("Churn", "1.8%", "-0.3%")];
    let gap = 14.0;
    let cw = (rect.width() - 40.0 - gap * 2.0) / 3.0;
    for (i, (l, v, d)) in labels.iter().enumerate() {
        let cr =
            Rect::from_min_size(pos2(rect.min.x + 20.0 + i as f32 * (cw + gap), head.max.y + 20.0), vec2(cw, 86.0));
        p.rect_filled(cr, 10.0, th.card);
        p.rect_stroke(cr, 10.0, Stroke::new(1.0, th.border), egui::StrokeKind::Inside);
        p.text(cr.min + vec2(14.0, 14.0), Align2::LEFT_TOP, *l, FontId::proportional(12.0), th.muted_foreground);
        super::util::bold_text(
            &p,
            cr.min + vec2(14.0, 36.0),
            Align2::LEFT_TOP,
            v,
            FontId::proportional(22.0),
            th.foreground,
        );
        p.text(
            pos2(cr.max.x - 14.0, cr.min.y + 14.0),
            Align2::RIGHT_TOP,
            *d,
            FontId::proportional(11.5),
            if d.starts_with('+') { th.success } else { th.destructive },
        );
    }
    let chart =
        Rect::from_min_max(pos2(rect.min.x + 20.0, head.max.y + 122.0), pos2(rect.max.x - 20.0, rect.max.y - 18.0));
    let pts: Vec<egui::Pos2> = (0..12)
        .map(|i| {
            let u = i as f32 / 11.0;
            let v = 0.5 + 0.3 * (u * 5.0).sin() * (1.0 - u * 0.4) - u * 0.25;
            pos2(chart.min.x + u * chart.width(), chart.min.y + v * chart.height())
        })
        .collect();
    let smooth = paint::catmull_rom(&pts, 8);
    paint::area_fill(&p, &smooth, chart.max.y, with_alpha(th.accent, 0.3), with_alpha(th.accent, 0.0));
    paint::gradient_polyline(&p, &smooth, false, 2.0, |_| th.accent);
}

fn team() -> Vec<Person> {
    vec![
        Person::new("Sarah Chen", "Product Designer", hex("#8B5CF6"), hex("#EC4899")),
        Person::new("Marcus Reid", "Rust Engineer", hex("#F97316"), hex("#EAB308")),
        Person::new("Aiko Tanaka", "Founder, Nimbus", hex("#06B6D4"), hex("#3B82F6")),
        Person::new("Diego Alvarez", "Staff Engineer", hex("#22C55E"), hex("#14B8A6")),
        Person::new("Priya Nair", "Design Lead", hex("#F43F5E"), hex("#A855F7")),
        Person::new("Tom Becker", "CTO, Driftwood", hex("#6366F1"), hex("#0EA5E9")),
    ]
}

fn article(ui: &mut Ui) {
    let t = Theme::of(ui);
    let sections = [
        ("Getting started", "glowkit brings the animated components you love from the web to native Rust apps. Every widget is pure egui, so it runs on Windows, macOS, Linux and in the browser through WebAssembly."),
        ("Theming", "Colors come from design tokens. Install a theme once and every component, from shimmering buttons to dotted globes, restyles itself instantly, including in light mode."),
        ("Animation", "Springs, easings and per-widget clocks keep motion smooth and frame-rate independent. One-shot animations stop repainting when they finish, so idle apps stay idle."),
        ("Performance", "Effects are built from a handful of meshes. A typical component costs well under a millisecond per frame in release builds."),
        ("Shipping", "Add the crate, pick a component, and ship something that glows. That is the whole idea."),
    ];
    for (i, (h, body)) in sections.iter().enumerate() {
        let tag = egui::RichText::new(format!("  Part {}  ", i + 1))
            .size(11.0)
            .color(t.accent_foreground)
            .background_color(t.accent);
        ui.label(tag);
        ui.add_space(2.0);
        ui.label(egui::RichText::new(*h).size(20.0).strong().color(t.foreground));
        ui.label(egui::RichText::new(*body).size(14.0).color(t.muted_foreground));
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(520.0), 90.0), egui::Sense::hover());
        paint::gradient_rect(
            ui.painter(),
            r,
            10.0,
            20.0,
            &[with_alpha(t.chart[i % 6], 0.55), with_alpha(t.chart[(i + 1) % 6], 0.25)],
        );
        ui.add_space(18.0);
    }
}

/// Painted "photo" scenes for the hover cards.
fn scene(p: &egui::Painter, r: Rect, which: usize) {
    match which % 3 {
        0 => {
            paint::gradient_rect(p, r, 16.0, 90.0, &[hex("#0EA5E9"), hex("#67E8F9"), hex("#FDE68A")]);
            let sun = pos2(r.center().x + r.width() * 0.2, r.min.y + r.height() * 0.42);
            paint::glow(p, sun, r.height() * 0.5, with_alpha(hex("#FFF7D6"), 0.6));
            p.circle_filled(sun, r.height() * 0.08, hex("#FFFBEB"));
            ridge(p, r, 2.0, 0.6, 0.3, hex("#0F766E"), hex("#134E4A"));
            ridge(p, r, 6.0, 0.78, 0.2, hex("#064E3B"), hex("#022C22"));
        }
        1 => {
            paint::gradient_rect(p, r, 16.0, 90.0, &[hex("#312E81"), hex("#DB2777"), hex("#FB923C")]);
            let sun = pos2(r.center().x - r.width() * 0.15, r.min.y + r.height() * 0.6);
            paint::glow(p, sun, r.height() * 0.6, with_alpha(hex("#FDBA74"), 0.6));
            p.circle_filled(sun, r.height() * 0.12, hex("#FFEDD5"));
            ridge(p, r, 3.0, 0.7, 0.2, hex("#581C87"), hex("#3B0764"));
            ridge(p, r, 8.0, 0.86, 0.12, hex("#1E0B3B"), hex("#0B0618"));
        }
        _ => {
            paint::gradient_rect(p, r, 16.0, 90.0, &[hex("#020617"), hex("#1E3A8A"), hex("#0F172A")]);
            for i in 0..60u64 {
                let x = glowkit_core::rng::hash01(i * 3 + 1);
                let y = glowkit_core::rng::hash01(i * 5 + 2) * 0.6;
                p.circle_filled(
                    pos2(r.min.x + x * r.width(), r.min.y + y * r.height()),
                    0.5 + glowkit_core::rng::hash01(i) * 1.2,
                    Color32::from_white_alpha(200),
                );
            }
            p.circle_filled(
                pos2(r.min.x + r.width() * 0.7, r.min.y + r.height() * 0.25),
                r.height() * 0.07,
                hex("#E0E7FF"),
            );
            ridge(p, r, 5.0, 0.72, 0.25, hex("#1E293B"), hex("#0F172A"));
        }
    }
}

/// Frame a background demo: rounded corners, optional centered headline.
fn bg_demo(ui: &mut Ui, title: &str, light_text: bool, add: impl FnOnce(&mut Ui)) {
    let t = Theme::of(ui);
    let r = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(r.min, vec2(r.width(), 380.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    add(&mut child);
    ui.allocate_rect(rect, egui::Sense::hover());
    let p = ui.painter();
    super::util::corner_mask(p, rect, 12.0, t.card);
    p.rect_stroke(rect, 12.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    if !title.is_empty() {
        let col = if light_text { Color32::WHITE } else { t.foreground };
        let g = p.layout_no_wrap(title.to_owned(), FontId::proportional(40.0), Color32::PLACEHOLDER);
        let pos = rect.center() - g.size() / 2.0;
        paint::glow_ellipse(
            p,
            rect.center(),
            vec2(g.size().x * 0.7, 50.0),
            Color32::from_black_alpha(if light_text { 70 } else { 0 }),
        );
        p.galley(pos + vec2(0.5, 0.0), g.clone(), col);
        p.galley(pos, g, col);
    }
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "meta-balls",
            name: "Meta Balls",
            category: Category::Backgrounds,
            description: "Organic blobs that merge and split like liquid, colored by the dominant blob; one follows the pointer.",
            covers: &["React Bits: Meta Balls", "Magic UI: Blob background"],
            code: r##"ui.add(MetaBalls::new().count(9));
// behind other content: MetaBalls::new().paint_at(ui, rect, pointer_pos)"##,
            show: |ui| bg_demo(ui, "", false, |ui| {
                ui.add(MetaBalls::new().count(9).size(ui.available_width(), 380.0));
            }),
        },
        Demo {
            id: "ball-pit",
            name: "Ball Pit",
            category: Category::Backgrounds,
            description: "Glossy balls with gravity, collisions and bounce; the pointer shoves them around and an invisible hand stirs them when idle.",
            covers: &["React Bits: Ballpit"],
            code: r##"ui.add(BallPit::new().count(40));"##,
            show: |ui| bg_demo(ui, "", false, |ui| {
                ui.add(BallPit::new().count(42).size(ui.available_width(), 380.0));
            }),
        },
        Demo {
            id: "ripple-grid",
            name: "Ripple Grid",
            category: Category::Backgrounds,
            description: "Grid lines rippling with waves from the center and the pointer, crests lit in the accent color.",
            covers: &["React Bits: Ripple Grid", "React Bits: Grid Distortion"],
            code: r##"ui.add(RippleGrid::new().spacing(30.0).amplitude(10.0));"##,
            show: |ui| bg_demo(ui, "Ripple Grid", false, |ui| {
                ui.add(RippleGrid::new().size(ui.available_width(), 380.0));
            }),
        },
        Demo {
            id: "dark-veil",
            name: "Dark Veil",
            category: Category::Backgrounds,
            description: "A dark, slowly flowing veil of tinted light, like light through smoke.",
            covers: &["React Bits: Dark Veil"],
            code: r##"ui.add(DarkVeil::new());"##,
            show: |ui| {
                let dark = Theme::of(ui).dark;
                bg_demo(ui, "Into the veil", dark, |ui| {
                    ui.add(DarkVeil::new().size(ui.available_width(), 380.0));
                })
            },
        },
        Demo {
            id: "color-bends",
            name: "Color Bends",
            category: Category::Backgrounds,
            description: "Bands of vivid color that bend and flow, leaning toward the pointer.",
            covers: &["React Bits: Color Bends", "React Bits: Prism"],
            code: r##"ui.add(ColorBends::new().bands(2.5));"##,
            show: |ui| bg_demo(ui, "Color Bends", true, |ui| {
                ui.add(ColorBends::new().size(ui.available_width(), 380.0));
            }),
        },
        Demo {
            id: "gradient-blinds",
            name: "Gradient Blinds",
            category: Category::Backgrounds,
            description: "Venetian-blind slats of flowing gradient, each lit across its width, with a spotlight that follows the pointer.",
            covers: &["React Bits: Gradient Blinds"],
            code: r##"ui.add(GradientBlinds::new().slats(16).angle(8.0));"##,
            show: |ui| {
                let dark = Theme::of(ui).dark;
                bg_demo(ui, "", dark, |ui| {
                    ui.add(GradientBlinds::new().slats(18).angle(6.0).size(ui.available_width(), 380.0));
                })
            },
        },
        Demo {
            id: "pixel-blast",
            name: "Pixel Blast / Dither",
            category: Category::Backgrounds,
            description: "Animated noise or waves quantized with a 4x4 Bayer ordered dither into pixels, dots or diamonds; ripples burst from the pointer.",
            covers: &["React Bits: Pixel Blast", "React Bits: Dither"],
            code: r##"ui.add(PixelBlast::new().shape(PixelShape::Square).pixel_size(8.0));
ui.add(PixelBlast::new().pattern(PixelPattern::Waves).shape(PixelShape::Circle));"##,
            show: |ui| {
                let w = ui.available_width();
                let half = (w - 16.0) / 2.0;
                bg_demo(ui, "", false, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        ui.add(PixelBlast::new().pixel_size(9.0).size(half, 380.0));
                        ui.add(PixelBlast::new().pattern(PixelPattern::Waves).shape(PixelShape::Circle).pixel_size(7.0).size(half, 380.0));
                    });
                });
            },
        },
        Demo {
            id: "faulty-terminal",
            name: "Faulty Terminal",
            category: Category::Backgrounds,
            description: "A glitchy green CRT: flickering glyphs under a brightness wave, scanlines, rolling interference and row glitches.",
            covers: &["React Bits: Faulty Terminal", "Magic UI: Retro Grid (CRT)"],
            code: r##"ui.add(FaultyTerminal::new().glyph_size(13.0));"##,
            show: |ui| bg_demo(ui, "", false, |ui| {
                ui.add(FaultyTerminal::new().size(ui.available_width(), 380.0));
            }),
        },
        Demo {
            id: "text-hover-effect",
            name: "Text Hover Effect",
            category: Category::TextAnimations,
            description: "A huge outlined word; a radial gradient follows the pointer and lights the outline in color around it.",
            covers: &["Aceternity: Text Hover Effect"],
            code: r##"ui.add(TextHoverEffect::new("GLOWKIT").size(150.0));"##,
            show: |ui| {
                ui.add_space(70.0);
                ui.vertical_centered(|ui| {
                    ui.add(TextHoverEffect::new("GLOWKIT").size(170.0));
                });
            },
        },
        Demo {
            id: "direction-aware-hover",
            name: "Direction Aware Hover",
            category: Category::CardsAndBorders,
            description: "A card whose caption overlay slides in from the edge the pointer entered through, and leaves the way it exits.",
            covers: &["Aceternity: Direction Aware Hover"],
            code: r##"ui.add(DirectionAwareHover::new(|p, r| paint_photo(p, r))
    .title("In the mountains")
    .subtitle("$1299 / night"));"##,
            show: |ui| {
                let w = 3.0 * 260.0 + 2.0 * 24.0;
                row(ui, w, 20.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    let titles = [("Lagoon retreat", "$1299 / night"), ("Desert sunset", "$899 / night"), ("Starlit cabin", "$1049 / night")];
                    let card = Theme::of(ui).card;
                    for (i, (a, b)) in titles.iter().enumerate() {
                        ui.add(DirectionAwareHover::new(move |p, r| {
                            scene(p, r, i);
                            super::util::corner_mask(p, r, 16.0, card);
                        }).size(260.0, 320.0).title(*a).subtitle(*b));
                    }
                });
            },
        },
        Demo {
            id: "glowing-stars",
            name: "Glowing Stars Card",
            category: Category::CardsAndBorders,
            description: "A card with a grid of tiny stars where random ones softly glow; more of them light up on hover.",
            covers: &["Aceternity: Glowing Stars Background Card", "Aceternity UI: Glowing Stars"],
            code: r##"ui.add(GlowingStarsCard::new("Next-gen rendering", "Crisp, animated UI on every platform."));"##,
            show: |ui| {
                let w = 2.0 * 340.0 + 28.0;
                row(ui, w, 30.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 28.0;
                    ui.add(GlowingStarsCard::new("Next-gen rendering", "Crisp, animated UI on every platform, from one Rust codebase.").size(340.0, 280.0));
                    ui.add(GlowingStarsCard::new("Always in sync", "Theme tokens restyle every glowing pixel instantly.").size(340.0, 280.0).color(hex("#C084FC")));
                });
            },
        },
        Demo {
            id: "canvas-reveal",
            name: "Canvas Reveal Effect",
            category: Category::CardsAndBorders,
            description: "On hover a flickering dot-matrix ripples out from the center of the card while the icon lifts and the title appears.",
            covers: &["Aceternity: Canvas Reveal Effect"],
            code: r##"ui.add(CanvasRevealCard::new("Sheetal is Nisha", "✦")
    .colors(&[hex("#EC4899"), hex("#E879F9")]));"##,
            show: |ui| {
                let w = 3.0 * 250.0 + 2.0 * 26.0;
                row(ui, w, 10.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 26.0;
                    ui.add(CanvasRevealCard::new("Sheetal is Nisha", "✦").size(250.0, 340.0).colors(&[hex("#059669"), hex("#10B981")]));
                    ui.add(CanvasRevealCard::new("Nisha is Munni", "◆").size(250.0, 340.0).colors(&[hex("#EC4899"), hex("#E879F9")]).dot_size(2.5));
                    ui.add(CanvasRevealCard::new("Munni is Aditi", "●").size(250.0, 340.0).colors(&[hex("#3B82F6"), hex("#22D3EE")]).dot_size(2.0));
                });
            },
        },
        Demo {
            id: "pin-3d",
            name: "3D Pin",
            category: Category::CardsAndBorders,
            description: "A card that tilts back in perspective on hover while a pin rises from its center with a label and pulsing rings.",
            covers: &["Aceternity: 3D Pin"],
            code: r##"ui.add(Pin3D::new("/glowkit.rs")
    .title("Aceternity UI, in Rust")
    .description("Customizable animated components."));"##,
            show: |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.add(Pin3D::new("/glowkit.rs").title("Aceternity-style UI, in Rust").description("Customizable animated components for egui.").size(320.0, 280.0));
                });
            },
        },
        Demo {
            id: "gemini-effect",
            name: "Gemini Effect",
            category: Category::Motion,
            description: "Five flowing colored curves draw themselves in, converging on a central pill and fanning out again. Bind progress to scroll.",
            covers: &["Aceternity: Google Gemini Effect"],
            code: r##"ui.add(GeminiEffect::new().title("Build with glowkit").button("glowkit.rs"));
// or: GeminiEffect::new().progress(scroll_fraction)"##,
            show: |ui| {
                ui.add(GeminiEffect::new().subtitle("Five strokes, one destination. Scroll-driven or looping."));
            },
        },
        Demo {
            id: "pointer-highlight",
            name: "Pointer Highlight",
            category: Category::TextAnimations,
            description: "A rectangle draws itself around a phrase while a pointer cursor glides to its corner, as if someone just highlighted it.",
            covers: &["Aceternity: Pointer Highlight"],
            code: r##"ui.add(PointerHighlight::new("The best way to grow is to ", "collaborate", "."));"##,
            show: |ui| {
                ui.add_space(90.0);
                ui.vertical_centered(|ui| {
                    ui.add(PointerHighlight::new("The best way to grow is to ", "collaborate", "."));
                    ui.add_space(26.0);
                    let t = Theme::of(ui);
                    ui.add(PointerHighlight::new("Ship it ", "beautifully", " or not at all").size(22.0).color(t.chart[1]));
                });
            },
        },
        Demo {
            id: "cover",
            name: "Cover",
            category: Category::TextAnimations,
            description: "Text in a box that turns into hyperspace on hover: streaking beams, sparkles, glowing corners and jittering text.",
            covers: &["Aceternity: Cover"],
            code: r##"ui.horizontal(|ui| {
    ui.label(RichText::new("Build amazing apps at").size(40.0));
    ui.add(Cover::new("warp speed").size(40.0));
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(110.0);
                let font = FontId::proportional(40.0);
                let g = ui.painter().layout_no_wrap("Build amazing apps at".into(), font.clone(), t.foreground);
                let w = g.size().x + 16.0 + 260.0;
                row(ui, w, 0.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.label(egui::RichText::new("Build amazing apps at").size(40.0).color(t.foreground));
                    ui.add(Cover::new("warp speed").size(40.0));
                });
            },
        },
        Demo {
            id: "script-copy",
            name: "Script Copy Button",
            category: Category::Buttons,
            description: "An install command with package-manager tabs and a copy button whose clipboard morphs into a check with a 'Copied!' bubble.",
            covers: &["Magic UI: Script Copy Button", "Aceternity: Code block copy"],
            code: r##"ui.add(ScriptCopyBox::new(&[
    ("cargo", "cargo add glowkit"),
    ("npm", "npm install glowkit-wasm"),
    ("pnpm", "pnpm add glowkit-wasm"),
    ("bun", "bun add glowkit-wasm"),
]));"##,
            show: |ui| {
                ui.add_space(100.0);
                row(ui, 480.0, 0.0, |ui| {
                    ui.add(ScriptCopyBox::new(&[("cargo", "cargo add glowkit"), ("npm", "npm install glowkit-wasm"), ("pnpm", "pnpm add glowkit-wasm"), ("bun", "bun add glowkit-wasm")]).width(480.0).autoplay(true));
                });
            },
        },
        Demo {
            id: "code-comparison",
            name: "Code Comparison",
            category: Category::DataDisplay,
            description: "Before / after code panes with removed and added line highlights; unfocused lines blur and dim until you hover a pane.",
            covers: &["Magic UI: Code Comparison"],
            code: r##"ui.add(CodeComparison::new(BEFORE, AFTER).filename("main.rs"));
// annotate lines with // [!code --], // [!code ++] or // [!code focus]"##,
            show: |ui| {
                let before = "use eframe::egui;\n\nfn ui(ui: &mut egui::Ui) {\n    let mut style = ui.style_mut();\n    style.visuals.panel_fill = gray(9); // [!code --]\n    style.visuals.override_text_color = None; // [!code --]\n    ui.heading(\"Hello\"); // [!code focus]\n    ui.button(\"Click\"); // [!code focus]\n}";
                let after = "use glowkit::prelude::*;\n\nfn ui(ui: &mut egui::Ui) {\n    Theme::dark().install(ui.ctx()); // [!code ++]\n    ui.add(ShinyText::new(\"Hello\")); // [!code ++]\n    ui.add(ShimmerButton::new(\"Click\")); // [!code ++]\n    ui.add(Globe::new()); // [!code focus]\n}";
                let w = 900.0f32.min(ui.available_width());
                row(ui, w, 30.0, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.add(CodeComparison::new(before, after).filename("main.rs").size(w, 270.0));
                    });
                });
            },
        },
        Demo {
            id: "sticky-banner",
            name: "Sticky Banner",
            category: Category::Overlays,
            description: "A dismissible announcement bar with a flowing gradient; closing slides it up and the page content follows.",
            covers: &["Aceternity: Sticky Banner", "Magic UI: Banner"],
            code: r##"ui.add(StickyBanner::new(&mut open, "glowkit 1.0 is out! 30 new components.")
    .link("Read the announcement"));"##,
            show: |ui| {
                let id = ui.scope_id().with("sb");
                let now = ui.input(|i| i.time);
                let (mut open, since): (bool, f64) = glowkit_core::state(ui, id, || (true, now));
                let t = Theme::of(ui);
                let w = 820.0f32.min(ui.available_width());
                row(ui, w, 10.0, |ui| {
                    let fr = egui::Frame::new().fill(t.background).stroke(t.border_stroke()).corner_radius(14).inner_margin(0).show(ui, |ui| {
                        ui.set_width(w);
                        ui.set_height(330.0);
                        ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let hovered = ui.rect_contains_pointer(ui.max_rect());
                        let was = open;
                        ui.add(StickyBanner::new(&mut open, "glowkit 1.0 is out! 30 new animated components.").link("Read the announcement"));
                        if !hovered && now - since > if open { 4.0 } else { 2.0 } {
                            open = !open;
                        }
                        let since = if open != was { now } else { since };
                        glowkit_core::set_state(ui, id, (open, since));
                        let r = ui.available_rect_before_wrap();
                        let pr = Rect::from_min_size(r.min, vec2(r.width(), 300.0));
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(pr));
                        child.set_clip_rect(pr.intersect(ui.clip_rect()));
                        mock_site(&mut child);
                        });
                    }).response.rect;
                    super::util::corner_mask(ui.painter(), fr, 14.0, t.card);
                    ui.painter().rect_stroke(fr, 14.0, t.border_stroke(), egui::StrokeKind::Inside);
                });
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "tracing-beam",
            name: "Tracing Beam",
            category: Category::Motion,
            description: "A gradient beam beside the content that fills as you scroll, with a glowing head on a spring and a dot that lights up. Scrolls itself when idle.",
            covers: &["Aceternity: Tracing Beam"],
            code: r##"TracingBeam::new().height(380.0).show(ui, |ui| {
    my_article(ui);
});
// or bind it to any progress value:
ui.add(TracingBeam::new().progress(0.4).height(300.0));"##,
            show: |ui| {
                let w = 620.0f32.min(ui.available_width());
                let avail = ui.available_width();
                ui.horizontal(|ui| {
                    ui.add_space(((avail - w) / 2.0).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        TracingBeam::new().height(380.0).show(ui, article);
                    });
                });
            },
        },
        Demo {
            id: "multi-step-loader",
            name: "Multi Step Loader",
            category: Category::Loaders,
            description: "An overlay listing the steps of a long task; each checks off in turn while the current one is highlighted and centered.",
            covers: &["Aceternity: Multi Step Loader"],
            code: r##"ui.add(MultiStepLoader::new(&[
    "Fetching the crate index", "Resolving dependencies", "Compiling shaders",
    "Baking gradients", "Polishing the glow", "Ready to ship",
]).step_duration(1.4));"##,
            show: |ui| {
                let t = Theme::of(ui);
                let rect = ui.available_rect_before_wrap();
                let rect = Rect::from_min_size(rect.min, vec2(rect.width(), 380.0));
                // something "behind" the overlay
                let p = ui.painter();
                for i in 0..6 {
                    let r = Rect::from_min_size(rect.min + vec2(30.0 + (i % 3) as f32 * (rect.width() - 60.0) / 3.0, 30.0 + (i / 3) as f32 * 170.0), vec2((rect.width() - 60.0) / 3.0 - 16.0, 150.0));
                    p.rect_filled(r, 12.0, t.muted);
                    paint::gradient_rect(p, Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(r.width() - 24.0, 70.0)), 8.0, 30.0, &[with_alpha(t.chart[i % 6], 0.7), with_alpha(t.chart[(i + 2) % 6], 0.3)]);
                }
                ui.add(
                    MultiStepLoader::new(&["Fetching the crate index", "Resolving dependencies", "Compiling shaders", "Baking gradients", "Polishing the glow", "Ready to ship"])
                        .step_duration(1.1)
                        .size(rect.width(), 380.0),
                );
            },
        },
        Demo {
            id: "vanish-input",
            name: "Placeholders & Vanish Input",
            category: Category::Inputs,
            description: "An input whose placeholder cycles through prompts; on Enter the text dissolves right-to-left into drifting particles.",
            covers: &["Aceternity: Placeholders And Vanish Input"],
            code: r##"let out = VanishInput::new(&mut query)
    .placeholders(&["What's the first rule of glowkit?", "How do springs work?"])
    .show(ui);
if let Some(q) = out.submitted { search(q); }"##,
            show: |ui| {
                let id = ui.scope_id().with("vi");
                let mut text: String = glowkit_core::state(ui, id, String::new);
                let t = Theme::of(ui);
                ui.add_space(80.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Ask glowkit anything").size(26.0).color(t.foreground));
                    ui.add_space(22.0);
                    let w = 540.0f32.min(ui.available_width() - 20.0);
                    let out = VanishInput::new(&mut text)
                        .placeholders(&["What's the first rule of glowkit?", "How do I make a button glow?", "Write a Rust app that runs everywhere", "Is egui fast enough for animations?"])
                        .width(w)
                        .autoplay(true)
                        .show(ui);
                    if let Some(s) = out.submitted {
                        glowkit_core::set_state(ui, id.with("last"), s);
                    }
                    let last: String = glowkit_core::state(ui, id.with("last"), String::new);
                    if !last.is_empty() {
                        ui.add_space(14.0);
                        ui.label(egui::RichText::new(format!("Submitted: {last}")).size(12.0).color(t.muted_foreground));
                    }
                });
                glowkit_core::set_state(ui, id, text);
            },
        },
        Demo {
            id: "animated-tooltip",
            name: "Animated Tooltip",
            category: Category::CursorAndInteraction,
            description: "Overlapping avatars; the hovered one lifts and a name tooltip pops above it, swinging with the pointer on a loose spring.",
            covers: &["Aceternity: Animated Tooltip", "Magic UI: Avatar Circles"],
            code: r##"let team = [Person::new("Sarah Chen", "Product Designer", hex("#8B5CF6"), hex("#EC4899")), /* ... */];
ui.add(AnimatedTooltip::new(&team));"##,
            show: |ui| {
                let people = team();
                ui.add_space(150.0);
                ui.vertical_centered(|ui| {
                    ui.add(AnimatedTooltip::new(&people).size(60.0));
                    ui.add_space(40.0);
                    let t = Theme::of(ui);
                    ui.label(egui::RichText::new("Trusted by 12,000+ developers").size(13.0).color(t.muted_foreground));
                });
            },
        },
        Demo {
            id: "animated-testimonials",
            name: "Animated Testimonials",
            category: Category::Blocks,
            description: "A stack of tilted photo cards; the active one hops to the front while its quote reveals word by word out of a blur.",
            covers: &["Aceternity: Animated Testimonials"],
            code: r##"let items = [Testimonial::new(Person::new("Sarah Chen", "Product Designer at Nimbus", a, b),
    "glowkit made our desktop app feel like a modern web product."), /* ... */];
ui.add(AnimatedTestimonials::new(&items));"##,
            show: |ui| {
                let quotes = [
                    "glowkit made our desktop app feel like a modern web product overnight. The animations are buttery and the API is exactly what you'd expect from egui.",
                    "We replaced a whole Electron shell with a single Rust binary. Our users only noticed that everything got faster and prettier.",
                    "The attention to detail is unreal. Springs, blurs, glows: it all just works, even in the browser through WebAssembly.",
                    "I shipped a polished dashboard in an afternoon. The theme tokens mean light mode came for free.",
                ];
                let items: Vec<Testimonial> = team().into_iter().zip(quotes).map(|(p, q)| Testimonial::new(p, q)).collect();
                let w = 820.0f32.min(ui.available_width());
                row(ui, w, 16.0, |ui| {
                    ui.add(AnimatedTestimonials::new(&items).size(w, 330.0));
                });
            },
        },
        Demo {
            id: "tweet-card",
            name: "Tweet Card",
            category: Category::DataDisplay,
            description: "A social post card with verified badge, highlighted mentions and tags, optional media and a like button with a particle burst.",
            covers: &["Magic UI: Tweet Card", "Aceternity: Social card"],
            code: r##"let mut tweet = Tweet::new("glowkit", "glowkit_rs", "Shipping 30 new components today ✨ #rustlang");
tweet.media = Some([hex("#8B5CF6"), hex("#EC4899")]);
ui.add(TweetCard::new(&tweet).width(420.0));"##,
            show: |ui| {
                let mut a = Tweet::new("glowkit", "glowkit_rs", "Shipping 30 animated components for egui today ✨ Globes, terminals, scratch cards and more. Pure Rust, runs everywhere. #rustlang #egui glowkit.rs");
                a.media = Some([hex("#7C3AED"), hex("#EC4899")]);
                a.likes = 2_481;
                a.views = 184_000;
                let mut b = Tweet::new("Sarah Chen", "sarahdesigns", "Honestly did not expect a native app to feel this smooth. @glowkit_rs nailed the springs. 🤯");
                b.avatar = [hex("#06B6D4"), hex("#3B82F6")];
                b.time = "5h".into();
                b.verified = false;
                b.likes = 96;
                b.replies = 4;
                b.reposts = 7;
                b.views = 3_210;
                let w = 2.0 * 390.0 + 24.0;
                row(ui, w, 6.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.add(TweetCard::new(&a).width(390.0));
                    ui.vertical(|ui| {
                        ui.add(TweetCard::new(&b).width(390.0));
                    });
                });
            },
        },
        Demo {
            id: "theme-toggler",
            name: "Animated Theme Toggler",
            category: Category::Buttons,
            description: "A sun that morphs into a crescent moon; switching sweeps a circular reveal of the new theme across the area.",
            covers: &["Magic UI: Animated Theme Toggler", "Aceternity: Theme toggle"],
            code: r##"let area = app_rect;
ui.add(ThemeToggler::new(&mut dark).size(38.0).reveal(area));"##,
            show: |ui| {
                let id = ui.scope_id().with("tt");
                let mut dark: bool = glowkit_core::state(ui, id, || true);
                let size = vec2(560.0, 300.0);
                let rect = super::util::centered_rect(ui, size, 10.0);
                let hovered = ui.rect_contains_pointer(rect);
                let idle = super::util::idle_time(ui, id, hovered);
                let last: f64 = glowkit_core::state(ui, id.with("last"), || 0.0);
                let now = ui.input(|i| i.time);
                if idle > 2.0 && now - last > 3.2 {
                    dark = !dark;
                    glowkit_core::set_state(ui, id.with("last"), now);
                }
                glowkit_core::animate(ui);
                let th = if dark { Theme::dark() } else { Theme::light() };
                mini_app(ui, rect, &th, &mut dark);
                glowkit_core::set_state(ui, id, dark);
                ui.add_space(24.0);
            },
        },
        Demo {
            id: "day-night-switch",
            name: "Day / Night Switch",
            category: Category::Inputs,
            description: "A sliding switch: sunny sky with clouds slides into a starry night as the sun knob turns into a cratered moon.",
            covers: &["Magic UI: Animated Theme Toggler (switch)", "Uiverse: Day night toggle"],
            code: r##"ui.add(DayNightSwitch::new(&mut night).width(96.0));"##,
            show: |ui| {
                let id = ui.scope_id().with("dn");
                let now = ui.input(|i| i.time);
                let mut flags: [bool; 3] = glowkit_core::state(ui, id, || [false, true, false]);
                let auto = ((now / 2.2) as i64 % 2) == 1;
                ui.add_space(110.0);
                row(ui, 96.0 + 140.0 + 70.0 + 80.0, 0.0, |ui| {
                    ui.spacing_mut().item_spacing.x = 40.0;
                    ui.set_min_height(70.0);
                    ui.add(DayNightSwitch::new(&mut flags[0]).width(96.0));
                    let mut a = auto;
                    ui.add(DayNightSwitch::new(&mut a).width(140.0));
                    let mut b = !auto;
                    ui.add(DayNightSwitch::new(&mut b).width(70.0));
                    ui.add(ThemeToggler::new(&mut flags[1]).size(44.0));
                });
                glowkit_core::animate(ui);
                glowkit_core::set_state(ui, id, flags);
            },
        },
        Demo {
            id: "scratch-to-reveal",
            name: "Scratch To Reveal",
            category: Category::CursorAndInteraction,
            description: "A shimmering foil you scratch away with the pointer; past 50% the rest dissolves and it reports `revealed`. Self-scratches when idle.",
            covers: &["Magic UI: Scratch To Reveal"],
            code: r##"let out = ScratchToReveal::new(vec2(360.0, 210.0)).brush(24.0).show(ui, |ui| {
    ui.heading("🎉 You won 50% off!");
});
if out.just_revealed { celebrate(); }"##,
            show: |ui| {
                let size = vec2(380.0, 220.0);
                let rect = super::util::centered_rect(ui, size, 30.0);
                let t = Theme::of(ui);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                let out = ScratchToReveal::new(size).reset_after(2.5).show(&mut child, |ui| {
                    let r = ui.max_rect();
                    let p = ui.painter();
                    paint::gradient_rect(p, r, 16.0, 135.0, &[mix(t.card, t.gradient[0], 0.25), mix(t.card, t.gradient[1], 0.2)]);
                    p.rect_stroke(r, 16.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    p.text(r.center() - vec2(0.0, 52.0), Align2::CENTER_CENTER, "🎉", FontId::proportional(34.0), t.foreground);
                    let g = p.layout_no_wrap("You won 50% off".into(), FontId::proportional(28.0), Color32::WHITE);
                    let gp = r.center() - vec2(g.size().x / 2.0, 14.0);
                    let stops = [t.gradient[0], t.gradient[1], t.gradient[2]];
                    let gr = Rect::from_min_size(gp, g.size());
                    paint::galley_with(p, gp + vec2(0.5, 0.0), &g, paint::linear_fn(gr, 0.0, &stops));
                    paint::galley_with(p, gp, &g, paint::linear_fn(gr, 0.0, &stops));
                    let code = Rect::from_center_size(r.center() + vec2(0.0, 52.0), vec2(150.0, 30.0));
                    p.rect_filled(code, 8.0, t.muted);
                    p.rect_stroke(code, 8.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                    p.text(code.center(), Align2::CENTER_CENTER, "GLOW-2026", FontId::monospace(14.0), t.foreground);
                });
                let msg = if out.revealed { "Revealed ✔" } else { "Drag to scratch" };
                ui.painter().text(pos2(rect.center().x, rect.max.y + 22.0), Align2::CENTER_CENTER, format!("{msg}  ·  {:.0}% cleared", out.cleared * 100.0), FontId::proportional(12.5), t.muted_foreground);
                ui.add_space(40.0);
            },
        },
        Demo {
            id: "compare-slider",
            name: "Compare Slider",
            category: Category::CursorAndInteraction,
            description: "Before / after comparison of two painted scenes with a draggable divider and grab handle; sweeps on its own when idle.",
            covers: &["Aceternity: Compare", "Motion Primitives: Image Comparison", "Magic UI: Image comparison", "Bencho: Image compare"],
            code: r##"ui.add(CompareSlider::new(
    |p, r| paint_raw_photo(p, r),
    |p, r| paint_graded_photo(p, r),
).size(640.0, 340.0));"##,
            show: |ui| {
                let w = 660.0f32.min(ui.available_width() - 20.0);
                let card = Theme::of(ui).card;
                row(ui, w, 6.0, |ui| {
                    ui.add(
                        CompareSlider::new(
                            |p, r| {
                                landscape(p, r, false);
                                super::util::corner_mask(p, r, 14.0, card);
                            },
                            |p, r| {
                                landscape(p, r, true);
                                super::util::corner_mask(p, r, 14.0, card);
                            },
                        )
                        .size(w, 350.0),
                    );
                });
                ui.add_space(10.0);
            },
        },
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
