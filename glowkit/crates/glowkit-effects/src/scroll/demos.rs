use super::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, RichText, ScrollArea, Sense, Shape, Stroke, Ui};
use glowkit_core::anim::Easing;
use glowkit_core::color::{hex, shade, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme, Trigger};

/// Lay out `add` in a centered column of `width`.
fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    let w = width.min(ui.available_width());
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(w);
            add(ui);
        });
    });
}

#[derive(Clone, Copy, Default)]
struct AutoScroll {
    off: f32,
    dir: f32,
    wait: f32,
    max: f32,
    idle: f32,
}

/// A vertical `ScrollArea` that glides up and down on its own while the pointer is away.
fn auto_scroll(ui: &mut Ui, key: &str, height: f32, speed: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("auto_scroll", key));
    let dt = super::common::wall_dt(ui);
    let mut st: AutoScroll = glowkit_core::state(ui, id, || AutoScroll { idle: 1.0, dir: 1.0, wait: 0.8, ..Default::default() });
    let mut sa = ScrollArea::vertical().id_salt(("auto_scroll_area", key)).max_height(height).min_scrolled_height(height).auto_shrink([false, false]);
    if st.idle > 0.8 && st.max > 0.0 {
        if st.wait > 0.0 {
            st.wait -= dt;
        } else {
            let edge = (st.off.min(st.max - st.off) / 50.0).clamp(0.2, 1.0);
            st.off += st.dir * speed * edge * dt;
            if st.off >= st.max {
                st.off = st.max;
                st.dir = -1.0;
                st.wait = 1.2;
            } else if st.off <= 0.0 {
                st.off = 0.0;
                st.dir = 1.0;
                st.wait = 1.2;
            }
        }
        sa = sa.vertical_scroll_offset(st.off);
    }
    let out = sa.show(ui, add);
    let hovered = ui.rect_contains_pointer(out.inner_rect);
    st.max = (out.content_size.y - out.inner_rect.height()).max(0.0);
    if hovered {
        st.idle = 0.0;
        st.off = out.state.clamped_offset().y;
    } else {
        st.idle += dt;
    }
    glowkit_core::set_state(ui, id, st);
    glowkit_core::animate(ui);
}

fn icon_tile(ui: &mut Ui, icon: &str, colors: [Color32; 2], size: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let p = ui.painter();
    paint::gradient_rect(p, r, size * 0.28, 45.0, &colors);
    p.text(r.center(), Align2::CENTER_CENTER, icon, FontId::proportional(size * 0.48), Color32::WHITE);
}

fn card_frame(t: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(if t.dark { shade(t.card, 0.04) } else { Color32::WHITE })
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(egui::Margin::same(18))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: if t.dark { Color32::from_black_alpha(90) } else { Color32::from_rgba_unmultiplied(30, 30, 70, 22) },
        })
}

const FEATURES: [(&str, &str, &str, &str, &str); 6] = [
    ("🚀", "Ship faster", "Drop-in widgets with springs and easing already tuned.", "#6366F1", "#8B5CF6"),
    ("🌊", "Fluid motion", "Every transition is interruptible and frame-rate independent.", "#0EA5E9", "#22D3EE"),
    ("🔥", "Zero config", "Theme tokens flow into every component automatically.", "#F97316", "#EF4444"),
    ("🌸", "Delightful", "Small details, like overshoot and stagger, done for you.", "#EC4899", "#A855F7"),
    ("🌲", "Pure Rust", "No JavaScript, no CSS, one binary for desktop and web.", "#10B981", "#14B8A6"),
    ("⭐", "Open source", "MIT licensed. Read it, fork it, make it yours.", "#FACC15", "#F97316"),
];

fn feature_card(ui: &mut Ui, i: usize) {
    let t = Theme::of(ui);
    let (icon, title, body, a, b) = FEATURES[i % FEATURES.len()];
    card_frame(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icon_tile(ui, icon, [hex(a), hex(b)], 46.0);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(RichText::new(title).size(17.0).strong().color(t.foreground));
                ui.label(RichText::new(body).size(13.5).color(t.muted_foreground));
            });
        });
    });
}

fn scroll_hint(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.add_space(26.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(text).size(13.0).color(t.muted_foreground));
        ui.label(RichText::new("↓").size(18.0).color(t.muted_foreground));
    });
    ui.add_space(30.0);
}

/// A painted, animated "video": a dusk landscape with parallax hills, drifting sun and fireflies.
fn video_scene(p: &Painter, r: Rect, secs: f32) {
    let s = secs + 4.0;
    let sky = [hex("#1E1B4B"), hex("#7C3AED"), hex("#F472B6"), hex("#FDBA74")];
    paint::shader_grid(p, r, 1, 10, |uv| glowkit_core::color::gradient(&sky, uv.y * 1.05 + 0.05 * (s * 0.2).sin()));
    let sun = pos2(r.min.x + r.width() * (0.62 + 0.05 * (s * 0.15).sin()), r.min.y + r.height() * (0.5 + 0.04 * (s * 0.3).cos()));
    paint::glow(p, sun, r.height() * 0.5, with_alpha(hex("#FDE68A"), 0.55));
    p.circle_filled(sun, r.height() * 0.09, hex("#FFF7ED"));
    let layers: [(f32, f32, f32, f32, &str); 3] =
        [(0.62, 0.07, 3.0, 0.12, "#9D4EDD"), (0.72, 0.06, 4.5, 0.3, "#5B21B6"), (0.84, 0.05, 6.0, 0.6, "#2E1065")];
    for (k, (base, amp, freq, speed, col)) in layers.iter().enumerate() {
        let n = 48;
        let mut m = Mesh::default();
        let c = hex(col);
        for i in 0..=n {
            let u = i as f32 / n as f32;
            let x = r.min.x + u * r.width();
            let ph = u * freq + s * speed + k as f32 * 2.0;
            let y = r.min.y + r.height() * (base - amp * (ph.sin() * 0.6 + (ph * 2.3).sin() * 0.3 + (ph * 5.1).sin() * 0.1));
            m.colored_vertex(pos2(x, y), c);
            m.colored_vertex(pos2(x, r.max.y), shade(c, -0.25));
        }
        for i in 0..n as u32 {
            let a = i * 2;
            m.add_triangle(a, a + 1, a + 2);
            m.add_triangle(a + 1, a + 3, a + 2);
        }
        p.add(Shape::mesh(m));
    }
    for i in 0..26u64 {
        let x = r.min.x + r.width() * ((hash01(i) + s * 0.01 * (0.5 + hash01(i + 9))).rem_euclid(1.0));
        let y = r.min.y + r.height() * (0.55 + 0.4 * hash01(i + 40) + 0.03 * (s * 1.3 + i as f32).sin());
        let tw = 0.5 + 0.5 * (s * 3.0 + i as f32 * 2.1).sin();
        paint::glow(p, pos2(x, y), 6.0, with_alpha(hex("#FDE68A"), 0.5 * tw));
    }
    p.text(
        r.left_top() + vec2(18.0, 16.0),
        Align2::LEFT_TOP,
        "Golden Hour · a glowkit short",
        FontId::proportional(13.0),
        with_alpha(Color32::WHITE, 0.85),
    );
}

/// A vector star chart with tiny labels: stays crisp under the lens.
fn star_chart(p: &Painter, r: Rect) {
    let bg = [hex("#020617"), hex("#0B1E3F"), hex("#1E1B4B")];
    let c = r.center();
    let rad = r.size().length() * 0.6;
    paint::shader_grid(p, r, 8, 6, |uv| {
        let q = pos2(r.min.x + uv.x * r.width(), r.min.y + uv.y * r.height());
        glowkit_core::color::gradient(&bg, q.distance(c) / rad)
    });
    let s = r.width() / 520.0;
    paint::glow(p, r.min + vec2(r.width() * 0.25, r.height() * 0.35), 140.0 * s, with_alpha(hex("#7C3AED"), 0.22));
    paint::glow(p, r.min + vec2(r.width() * 0.75, r.height() * 0.7), 160.0 * s, with_alpha(hex("#0EA5E9"), 0.18));
    for i in 0..220u64 {
        let q = r.min + vec2(hash01(i * 3) * r.width(), hash01(i * 3 + 1) * r.height());
        let b = hash01(i * 3 + 2);
        p.circle_filled(q, (0.4 + b * 1.1) * s, with_alpha(Color32::WHITE, 0.25 + 0.6 * b));
    }
    let consts: [(&str, [(f32, f32); 5]); 3] = [
        ("Cassiopeia", [(0.1, 0.25), (0.17, 0.15), (0.24, 0.24), (0.31, 0.14), (0.38, 0.22)]),
        ("Lyra", [(0.55, 0.2), (0.6, 0.32), (0.66, 0.42), (0.6, 0.5), (0.54, 0.4)]),
        ("Orion", [(0.72, 0.6), (0.78, 0.7), (0.84, 0.78), (0.9, 0.66), (0.8, 0.55)]),
    ];
    for (name, pts) in consts {
        let pp: Vec<_> = pts.iter().map(|(x, y)| r.min + vec2(x * r.width(), y * r.height())).collect();
        p.add(Shape::line(pp.clone(), Stroke::new(1.0 * s, with_alpha(hex("#93C5FD"), 0.45))));
        for (k, q) in pp.iter().enumerate() {
            paint::glow(p, *q, 7.0 * s, with_alpha(hex("#BFDBFE"), 0.6));
            p.circle_filled(*q, 2.0 * s, Color32::WHITE);
            if k == 0 {
                p.text(*q + vec2(6.0, 6.0) * s, Align2::LEFT_TOP, name, FontId::proportional(9.0 * s), with_alpha(hex("#BFDBFE"), 0.85));
                p.text(
                    *q + vec2(6.0, 17.0) * s,
                    Align2::LEFT_TOP,
                    format!("RA {:02}h {:02}m · mag {:.1}", (k + 3) * 4 % 24, (name.len() * 7) % 60, 2.1 + name.len() as f32 * 0.1),
                    FontId::monospace(6.0 * s),
                    with_alpha(Color32::WHITE, 0.5),
                );
            }
        }
    }
    // a moon with craters
    let m = r.min + vec2(r.width() * 0.4, r.height() * 0.7);
    paint::glow(p, m, 60.0 * s, with_alpha(hex("#E0E7FF"), 0.25));
    paint::gradient_circle(p, m, 26.0 * s, &[hex("#F8FAFC"), hex("#CBD5E1")]);
    for (dx, dy, cr) in [(-8.0, -6.0, 5.0), (7.0, 5.0, 3.5), (2.0, -12.0, 2.5), (-4.0, 10.0, 3.0)] {
        p.circle_filled(m + vec2(dx, dy) * s, cr * s, with_alpha(hex("#94A3B8"), 0.5));
    }
    p.text(r.max - vec2(12.0, 10.0) * s, Align2::RIGHT_BOTTOM, "NORTHERN SKY · WINTER", FontId::monospace(8.0 * s), with_alpha(Color32::WHITE, 0.45));
}

fn photo_scene(p: &Painter, r: Rect, tex: egui::TextureId) {
    let uv = super::common::cover_uv(PaintedScene::ASPECT, r.width() / r.height());
    let mut m = Mesh::with_texture(tex);
    m.add_rect_with_uv(r, uv, Color32::WHITE);
    p.add(Shape::mesh(m));
    // a tiny caption that rewards zooming
    let cap = Rect::from_min_size(r.left_bottom() + vec2(12.0, -34.0), vec2(150.0, 22.0));
    p.rect_filled(cap, CornerRadius::same(6), with_alpha(Color32::BLACK, 0.45));
    p.text(cap.center(), Align2::CENTER_CENTER, "Nº 07 · Alpenglow ridge, 2,341 m", FontId::proportional(8.0 * r.width() / 330.0), Color32::WHITE);
}

fn dashboard(ui: &mut Ui) {
    let t = Theme::of(ui);
    let r = ui.max_rect();
    let p = ui.painter();
    p.text(r.min, Align2::LEFT_TOP, "Overview", FontId::proportional(18.0), t.foreground);
    let pill = Rect::from_min_size(pos2(r.max.x - 170.0, r.min.y - 2.0), vec2(170.0, 26.0));
    p.rect(pill, CornerRadius::same(99), t.muted, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.text(pill.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, "⌕  Search…", FontId::proportional(12.0), t.muted_foreground);
    let stats = [("Revenue", "$45,231", "+20.1%"), ("Subscriptions", "+2,350", "+180%"), ("Active now", "573", "+201")];
    let tw = (r.width() - 24.0) / 3.0;
    for (i, (label, value, delta)) in stats.iter().enumerate() {
        let tile = Rect::from_min_size(pos2(r.min.x + i as f32 * (tw + 12.0), r.min.y + 38.0), vec2(tw, 74.0));
        p.rect(tile, CornerRadius::same(12), t.card, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        p.text(tile.min + vec2(14.0, 12.0), Align2::LEFT_TOP, *label, FontId::proportional(12.0), t.muted_foreground);
        p.text(tile.min + vec2(14.0, 30.0), Align2::LEFT_TOP, *value, FontId::proportional(22.0), t.foreground);
        p.text(tile.right_top() + vec2(-14.0, 14.0), Align2::RIGHT_TOP, *delta, FontId::proportional(11.5), t.success);
    }
    let chart = Rect::from_min_max(pos2(r.min.x, r.min.y + 124.0), pos2(r.min.x + r.width() * 0.62, r.max.y));
    p.rect(chart, CornerRadius::same(12), t.card, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.text(chart.min + vec2(14.0, 10.0), Align2::LEFT_TOP, "Monthly sales", FontId::proportional(12.5), t.foreground);
    let bars = [0.45, 0.62, 0.38, 0.8, 0.55, 0.7, 0.92, 0.6, 0.75, 0.5, 0.85, 0.66];
    let inner = Rect::from_min_max(chart.min + vec2(14.0, 34.0), chart.max - vec2(14.0, 12.0));
    let bw = inner.width() / bars.len() as f32;
    for (i, h) in bars.iter().enumerate() {
        let b = Rect::from_min_max(pos2(inner.min.x + i as f32 * bw + 3.0, inner.max.y - inner.height() * h), pos2(inner.min.x + (i + 1) as f32 * bw - 3.0, inner.max.y));
        paint::gradient_rect(p, b, 4.0, 90.0, &[t.gradient[0], t.gradient[2]]);
    }
    let list = Rect::from_min_max(pos2(chart.max.x + 12.0, chart.min.y), r.max);
    p.rect(list, CornerRadius::same(12), t.card, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.text(list.min + vec2(14.0, 10.0), Align2::LEFT_TOP, "Recent sales", FontId::proportional(12.5), t.foreground);
    for (i, (n, a)) in [("Olivia M.", "+$1,999"), ("Jackson L.", "+$39"), ("Isabella N.", "+$299"), ("William K.", "+$99")].iter().enumerate() {
        let y = list.min.y + 42.0 + i as f32 * 26.0;
        if y > list.max.y - 10.0 {
            break;
        }
        p.circle_filled(pos2(list.min.x + 24.0, y), 9.0, t.chart[i % 6]);
        p.text(pos2(list.min.x + 40.0, y), Align2::LEFT_CENTER, *n, FontId::proportional(12.0), t.foreground);
        p.text(pos2(list.max.x - 14.0, y), Align2::RIGHT_CENTER, *a, FontId::proportional(12.0), t.foreground);
    }
}

fn apple_cards() -> Vec<AppleCard> {
    let body = "Lorem-free copy, promise: this panel grew out of the card you clicked. Its cover image shrank into the \
                left column while the text faded in, and the whole thing sits on a spring, so you can interrupt it at any time. \
                Click outside or the close button to send it home.";
    vec![
        AppleCard::new("Artificial Intelligence", "You can do more with AI.").body(body),
        AppleCard::new("Productivity", "Enhance your productivity.").body(body),
        AppleCard::new("Product", "Launching the new spatial headset.").body(body),
        AppleCard::new("Travel", "Maps that fit in your pocket.").body(body),
        AppleCard::new("Photography", "Shoot in the golden hour.").body(body),
        AppleCard::new("Hiring", "Join the team building it.").body(body),
        AppleCard::new("Outdoors", "Trails worth the early alarm.").body(body),
        AppleCard::new("Music", "Lavender skies, lo-fi nights.").body(body),
    ]
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "animated-content",
            name: "Animated Content",
            category: Category::Motion,
            description: "Wrap anything and it slides, scales and fades into place as it scrolls into view.",
            covers: &["React Bits: Animated Content"],
            code: r##"AnimatedContent::new()
    .from(SlideFrom::Below)
    .distance(80.0)
    .scale(0.92)
    .easing(Easing::OutBack)
    .show(ui, |ui| {
        ui.heading("Hello there");
    });"##,
            show: |ui| {
                auto_scroll(ui, "animated-content", 380.0, 70.0, |ui| {
                    scroll_hint(ui, "Scroll to reveal");
                    column(ui, 560.0, |ui| {
                        let dirs = [SlideFrom::Below, SlideFrom::Left, SlideFrom::Right, SlideFrom::Below, SlideFrom::Left, SlideFrom::Right];
                        for (i, d) in dirs.iter().enumerate() {
                            AnimatedContent::new()
                                .from(*d)
                                .distance(if matches!(d, SlideFrom::Below) { 70.0 } else { 140.0 })
                                .scale(if i % 2 == 0 { 0.9 } else { 1.0 })
                                .easing(if i % 3 == 0 { Easing::OutBack } else { Easing::OutCubic })
                                .duration(0.75)
                                .threshold(0.25)
                                .once(false)
                                .id_salt(i)
                                .show(ui, |ui| feature_card(ui, i));
                            ui.add_space(16.0);
                        }
                    });
                    ui.add_space(30.0);
                });
            },
        },
        Demo {
            id: "fade-content",
            name: "Fade Content",
            category: Category::Motion,
            description: "Content starts soft, ghosted and transparent, then sharpens into focus when it scrolls into view.",
            covers: &["React Bits: Fade Content", "Magic UI: Blur Fade"],
            code: r##"FadeContent::new().blur(8.0).rise(16.0).duration(1.0).show(ui, |ui| {
    ui.label("Soft focus, then crisp.");
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                auto_scroll(ui, "fade-content", 380.0, 60.0, |ui| {
                    scroll_hint(ui, "Scroll to focus");
                    column(ui, 600.0, |ui| {
                        let quotes = [
                            ("Simplicity is the soul of efficiency.", "A note taped to every monitor"),
                            ("Make it work, make it right, make it fast.", "The order matters"),
                            ("Motion should explain, never decorate.", "Design principle no. 3"),
                        ];
                        for (i, (q, by)) in quotes.iter().enumerate() {
                            FadeContent::new().blur(9.0).rise(18.0).once(false).threshold(0.3).id_salt(("q", i)).show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new(*q).size(30.0).strong().color(t.foreground));
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(*by).size(14.0).color(t.muted_foreground));
                                });
                            });
                            ui.add_space(30.0);
                            FadeContent::new().blur(12.0).once(false).threshold(0.3).duration(1.2).id_salt(("img", i)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let w = (ui.available_width() - 16.0) / 3.0;
                                    for k in 0..3 {
                                        let tex = PaintedScene::texture(ui.ctx(), i * 3 + k);
                                        let uv = super::common::cover_uv(PaintedScene::ASPECT, w / 120.0);
                                        ui.add(egui::Image::new((tex, vec2(w, 120.0))).uv(uv).corner_radius(14));
                                    }
                                });
                            });
                            ui.add_space(40.0);
                        }
                    });
                });
            },
        },
        Demo {
            id: "scroll-stack",
            name: "Scroll Stack",
            category: Category::Motion,
            description: "Cards pin near the top as you scroll and pile into a stack, each one shrinking back under the next.",
            covers: &["React Bits: Scroll Stack"],
            code: r##"ScrollStack::new(5)
    .card_size(vec2(600.0, 220.0))
    .base_scale(0.86)
    .autoplay(80.0)
    .show(ui, |ui, i| {
        ui.heading(format!("Card {}", i + 1));
    });"##,
            show: |ui| {
                let colors = vec![
                    [hex("#6366F1"), hex("#8B5CF6")],
                    [hex("#0EA5E9"), hex("#2563EB")],
                    [hex("#EC4899"), hex("#F97316")],
                    [hex("#10B981"), hex("#0D9488")],
                    [hex("#F59E0B"), hex("#EF4444")],
                ];
                let texts = [
                    ("Plan", "Sketch the idea, pick the springs, list the states."),
                    ("Build", "Compose widgets; every one is theme-aware out of the box."),
                    ("Polish", "Tune easing and stagger until it feels inevitable."),
                    ("Ship", "One binary on desktop, one wasm file on the web."),
                    ("Repeat", "Users notice. They always notice."),
                ];
                ScrollStack::new(5).card_size(vec2(600.0, 210.0)).height(380.0).colors(colors).autoplay(90.0).show(ui, |ui, i| {
                    let (title, body) = texts[i];
                    ui.label(RichText::new(format!("0{}", i + 1)).size(14.0).color(with_alpha(Color32::WHITE, 0.7)));
                    ui.label(RichText::new(title).size(34.0).strong().color(Color32::WHITE));
                    ui.add_space(4.0);
                    ui.label(RichText::new(body).size(15.0).color(with_alpha(Color32::WHITE, 0.85)));
                });
            },
        },
        Demo {
            id: "gradual-blur",
            name: "Gradual Blur",
            category: Category::Motion,
            description: "A progressive blur at the edge of a scroller: content softens and fades in steps as it leaves.",
            covers: &["React Bits: Gradual Blur", "Magic UI: Progressive Blur", "Motion Primitives: Progressive Blur"],
            code: r##"GradualBlur::new().edge(BlurEdge::Bottom).height(110.0).strength(5.0).show(ui, |ui| {
    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        for i in 0..40 { ui.label(format!("Row {i}")); }
    });
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.columns(2, |cols| {
                    GradualBlur::new().edge(BlurEdge::Bottom).height(120.0).strength(5.0).show(&mut cols[0], |ui| {
                        auto_scroll(ui, "gb-feed", 360.0, 40.0, |ui| {
                            ui.add_space(6.0);
                            let posts = [
                                ("Designing with springs", "Why critically damped motion feels calm, and when to break the rule.", "6 min"),
                                ("The case for painted UIs", "Meshes, gradients and a tessellator are all you need.", "4 min"),
                                ("Light themes, honestly", "Shadows, contrast and the one color you always forget.", "8 min"),
                                ("Shipping egui to the web", "A small wasm file and a canvas: that's the whole stack.", "5 min"),
                                ("Easing curve field guide", "Out-cubic, out-back and the elusive in-out-quint.", "3 min"),
                                ("Fake blur, real results", "Offset copies at low alpha fool the eye remarkably well.", "7 min"),
                                ("Stagger is a spice", "A pinch makes lists come alive, a handful makes them slow.", "4 min"),
                            ];
                            for (i, (title, ex, min)) in posts.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    icon_tile(ui, FEATURES[i % 6].0, [hex(FEATURES[i % 6].3), hex(FEATURES[i % 6].4)], 38.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(RichText::new(*title).size(15.5).strong().color(t.foreground));
                                        ui.label(RichText::new(*ex).size(12.5).color(t.muted_foreground));
                                        ui.label(RichText::new(format!("{min} read")).size(11.0).color(t.accent));
                                    });
                                });
                                ui.add_space(14.0);
                            }
                        });
                    });
                    GradualBlur::new().edge(BlurEdge::Top).height(70.0).strength(4.0).bands(5).show(&mut cols[1], |ui| {
                        GradualBlur::new().edge(BlurEdge::Bottom).height(110.0).strength(6.0).show(ui, |ui| {
                            auto_scroll(ui, "gb-grid", 360.0, 55.0, |ui| {
                                for row in 0..5 {
                                    ui.horizontal(|ui| {
                                        let w = (ui.available_width() - 10.0) / 2.0;
                                        for k in 0..2 {
                                            let i = row * 2 + k;
                                            let tex = PaintedScene::texture(ui.ctx(), i);
                                            let uv = super::common::cover_uv(PaintedScene::ASPECT, w / 130.0);
                                            ui.add(egui::Image::new((tex, vec2(w, 130.0))).uv(uv).corner_radius(14));
                                        }
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "flying-posters",
            name: "Flying Posters",
            category: Category::ThreeD,
            description: "Posters ride a curved 3D drum: they swell as they pass the middle and tilt away over the top. Scroll or drag to fly.",
            covers: &["React Bits: Flying Posters"],
            code: r##"ui.add(FlyingPosters::new().textures(&my_textures).columns(3).speed(40.0));"##,
            show: |ui| {
                ui.add(FlyingPosters::new().height(400.0).columns(5).poster_size(vec2(140.0, 190.0)));
            },
        },
        Demo {
            id: "apple-cards-carousel",
            name: "Apple Cards Carousel",
            category: Category::Motion,
            description: "Tall full-bleed image cards in a scrolling row; clicking one morphs it into a large detail dialog.",
            covers: &["Aceternity UI: Apple Cards Carousel"],
            code: r##"let cards = [
    AppleCard::new("Productivity", "Enhance your productivity.").body("…"),
    AppleCard::new("Travel", "Maps that fit in your pocket.").texture(my_texture),
];
ui.add(AppleCardsCarousel::new(&cards));"##,
            show: |ui| {
                let cards = apple_cards();
                ui.add(AppleCardsCarousel::new(&cards).card_size(vec2(214.0, 320.0)).autoplay(true));
            },
        },
        Demo {
            id: "container-scroll",
            name: "Container Scroll",
            category: Category::Motion,
            description: "A device mockup tilted back like an opening laptop lid rotates flat as it scrolls into view.",
            covers: &["Aceternity UI: Container Scroll Animation"],
            code: r##"egui::ScrollArea::vertical().show(ui, |ui| {
    ContainerScroll::new()
        .subtitle("Unleash the power of")
        .title("Scroll Animations")
        .show(ui, |ui| dashboard(ui));
});"##,
            show: |ui| {
                auto_scroll(ui, "container-scroll", 400.0, 70.0, |ui| {
                    scroll_hint(ui, "Scroll down");
                    ui.add_space(250.0);
                    ContainerScroll::new()
                        .subtitle("Unleash the power of")
                        .title("Scroll Animations")
                        .size(vec2(720.0, 330.0))
                        .show(ui, dashboard);
                    ui.add_space(20.0);
                });
            },
        },
        Demo {
            id: "hero-parallax",
            name: "Hero Parallax",
            category: Category::Motion,
            description: "Rows of product tiles start tilted and faded in 3D, then rotate flat and slide in opposite directions as you scroll.",
            covers: &["Aceternity UI: Hero Parallax"],
            code: r##"let products: Vec<_> = names.iter().map(|n| ParallaxProduct::new(*n)).collect();
ui.add(HeroParallax::new(&products).title("The Ultimate\ndevelopment studio"));"##,
            show: |ui| {
                let names = [
                    "Moonbeam", "Northwind", "Lumen", "Driftwood", "Polaris", "Kite", "Ember Studio", "Tidal", "Quartz", "Halcyon",
                    "Atlas", "Nimbus", "Solstice", "Verdant", "Prism",
                ];
                let products: Vec<_> = names.iter().map(|n| ParallaxProduct::new(*n)).collect();
                ui.add(HeroParallax::new(&products).height(420.0));
            },
        },
        Demo {
            id: "images-slider",
            name: "Images Slider",
            category: Category::Motion,
            description: "Full-bleed slides cross-fade with a zoom-in entrance and a slow Ken Burns push, under a headline and call to action.",
            covers: &["Aceternity UI: Images Slider"],
            code: r##"ui.add(
    ImagesSlider::new()
        .textures(&slides)
        .interval(4.0)
        .title("The hero section slideshow\nnobody asked for")
        .action("Join now"),
);"##,
            show: |ui| {
                ui.add(ImagesSlider::new().height(380.0).interval(3.5).title("The hero section slideshow\nnobody asked for").action("Join now"));
            },
        },
        Demo {
            id: "lens",
            name: "Lens",
            category: Category::CursorAndInteraction,
            description: "A magnifying glass over any painted scene: the content is re-painted larger inside a glassy circular lens.",
            covers: &["Magic UI: Lens", "Aceternity UI: Lens"],
            code: r##"Lens::new().zoom(2.4).radius(70.0).show(ui, vec2(520.0, 320.0), |painter, rect| {
    paint_my_scene(painter, rect);
});"##,
            show: |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let total = 520.0 + 330.0 + 20.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 20.0;
                    let rounded_scene = |p: &Painter, r: Rect, f: &dyn Fn(&Painter, Rect)| f(p, r);
                    Lens::new().zoom(2.6).radius(74.0).autoplay(true).id_salt(1).show(ui, vec2(520.0, 340.0), |p, r| rounded_scene(p, r, &star_chart));
                    let tex = PaintedScene::texture(ui.ctx(), 0);
                    Lens::new().zoom(2.0).radius(56.0).autoplay(true).id_salt(2).show(ui, vec2(330.0, 340.0), move |p, r| photo_scene(p, r, tex));
                });
            },
        },
        Demo {
            id: "link-preview",
            name: "Link Preview",
            category: Category::Overlays,
            description: "Hovering a link springs a preview card up from the text; it leans toward the pointer.",
            covers: &["Aceternity UI: Link Preview"],
            code: r##"ui.horizontal_wrapped(|ui| {
    ui.label("Build it in");
    ui.add(LinkPreview::new("Rust", "https://www.rust-lang.org").texture(screenshot, 1.6));
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(170.0);
                column(ui, 640.0, |ui| {
                    let size = 22.0;
                    let txt = |ui: &mut Ui, s: &str| {
                        ui.label(RichText::new(s).size(size).color(t.muted_foreground));
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        txt(ui, "Build your interface in");
                        ui.add(LinkPreview::new("Rust", "https://www.rust-lang.org").font_size(size).scene(6).autoplay(0.0));
                        txt(ui, ", paint it with");
                        ui.add(LinkPreview::new("egui", "https://www.egui.rs").font_size(size).scene(5).autoplay(2.0));
                        txt(ui, "and ship the very same app to the browser with");
                        ui.add(LinkPreview::new("WebAssembly", "https://webassembly.org").font_size(size).scene(2).autoplay(4.0));
                        txt(ui, ".");
                    });
                });
                ui.add_space(30.0);
            },
        },
        Demo {
            id: "sticky-scroll-reveal",
            name: "Sticky Scroll Reveal",
            category: Category::Motion,
            description: "Text sections scroll past a sticky panel that swaps its content and tints the backdrop for each one.",
            covers: &["Aceternity UI: Sticky Scroll Reveal"],
            code: r##"let sections = [
    StickySection::new("Collaborative editing", "Work together in real time…").icon("💬"),
    StickySection::new("Version control", "Never lose a change…").icon("⚙"),
];
ui.add(StickyScrollReveal::new(&sections));"##,
            show: |ui| {
                let sections = [
                    StickySection::new("Collaborative Editing", "Work together in real time with your team, clients and stakeholders. Comment, iterate and decide without the email ping-pong.")
                        .icon("💬")
                        .colors(hex("#06B6D4"), hex("#10B981")),
                    StickySection::new("Real time changes", "See changes as they happen. Every edit streams to everyone instantly, so the whole team stays in sync.")
                        .icon("🚀")
                        .colors(hex("#EC4899"), hex("#6366F1")),
                    StickySection::new("Version control", "Experience real-time history without the merge headaches. Every state is one click away.")
                        .icon("⚙")
                        .colors(hex("#F97316"), hex("#EAB308")),
                    StickySection::new("Running out of content", "Keep going: the panel keeps up, the colors keep shifting, and the text keeps scrolling smoothly.")
                        .icon("⭐")
                        .colors(hex("#8B5CF6"), hex("#0EA5E9")),
                ];
                ui.add(StickyScrollReveal::new(&sections).height(380.0).panel_size(vec2(320.0, 210.0)));
            },
        },
        Demo {
            id: "svg-mask-effect",
            name: "Mask Reveal",
            category: Category::CursorAndInteraction,
            description: "A circular mask follows the pointer and reveals an alternate version of the content underneath; it swells on hover.",
            covers: &["Aceternity UI: SVG Mask Effect"],
            code: r##"SvgMaskEffect::new().hover_radius(140.0).show(
    ui,
    |ui, rect| { /* paint the normal content */ },
    |ui, rect| { /* paint the hidden version, including its background */ },
);"##,
            show: |ui| {
                SvgMaskEffect::new().height(370.0).hover_radius(140.0).show(
                    ui,
                    |ui, r| {
                        let t = Theme::of(ui);
                        let p = ui.painter();
                        let g = p.layout(
                            "The interface you build should feel alive: springs, glows and reveals, painted in pure Rust.".into(),
                            FontId::proportional(38.0),
                            t.foreground,
                            r.width().min(720.0),
                        );
                        let pos = r.center() - g.size() / 2.0;
                        p.galley(pos, g, t.foreground);
                    },
                    |ui, r| {
                        let p = ui.painter();
                        paint::gradient_rect(p, r, 0.0, 30.0, &[hex("#F59E0B"), hex("#F97316"), hex("#EC4899")]);
                        let g = p.layout(
                            "The first rule of glowkit: nothing is a stylesheet. Every pixel here is a triangle we chose.".into(),
                            FontId::proportional(38.0),
                            hex("#1C1917"),
                            r.width().min(720.0),
                        );
                        let pos = r.center() - g.size() / 2.0;
                        p.galley(pos, g, hex("#1C1917"));
                    },
                );
            },
        },
        Demo {
            id: "text-reveal-card",
            name: "Text Reveal Card",
            category: Category::TextAnimations,
            description: "Drag or hover across the card: left of a glowing divider, a hidden line of text shows through with twinkling stars.",
            covers: &["Aceternity UI: Text Reveal Card"],
            code: r##"ui.add(
    TextRevealCard::new("Hover to look closer", "Painted, not styled")
        .title("Sometimes, you just need to see it.")
        .description("Drag across the card to reveal the hidden line."),
);"##,
            show: |ui| {
                ui.add_space(26.0);
                ui.add(
                    TextRevealCard::new("Hover to look closer", "Painted, not styled")
                        .title("Sometimes, you just need to see it.")
                        .description("Drag or hover across the card to reveal the hidden line.")
                        .size(vec2(640.0, 300.0)),
                );
            },
        },
        Demo {
            id: "pixel-image",
            name: "Pixel Image",
            category: Category::Motion,
            description: "An image resolves from a few big pixels to full detail: tiles pop in, then the mosaic sharpens step by step.",
            covers: &["Magic UI: Pixel Image"],
            code: r##"ui.add(
    PixelImage::new()
        .texture(my_texture, 1.5)
        .grid(6, 4)
        .trigger(Trigger::Loop { pause_ms: 1500 }),
);"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(30.0);
                ui.horizontal(|ui| {
                    let w = 260.0;
                    let total = w * 3.0 + 40.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 20.0;
                    for (i, (scene, grid)) in [(2usize, (4usize, 3usize)), (0, (6, 4)), (6, (8, 6))].iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.add(
                                PixelImage::new()
                                    .scene(*scene)
                                    .grid(grid.0, grid.1)
                                    .size(vec2(w, 190.0))
                                    .duration(3.0 + i as f32 * 0.3)
                                    .trigger(Trigger::Loop { pause_ms: 1800 })
                                    .id_salt(i),
                            );
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("{} · {}×{} start", PaintedScene::name(*scene), grid.0, grid.1)).size(13.0).color(t.muted_foreground));
                        });
                    }
                });
            },
        },
        Demo {
            id: "hero-video-dialog",
            name: "Hero Video Dialog",
            category: Category::Overlays,
            description: "A video thumbnail with a pulsing glass play button that opens an animated player dialog.",
            covers: &["Magic UI: Hero Video Dialog"],
            code: r##"HeroVideoDialog::new()
    .animation(VideoDialogAnimation::FromThumbnail)
    .show(ui, |painter, rect, seconds| {
        paint_video_frame(painter, rect, seconds);
    });"##,
            show: |ui| {
                ui.add_space(4.0);
                HeroVideoDialog::new().size(vec2(470.0, 264.0)).autoplay(true).show(ui, video_scene);
            },
        },
    ]
}

