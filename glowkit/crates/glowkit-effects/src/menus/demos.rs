use super::*;
use crate::buttons::demos::centered_row;
use crate::motion::common::sample_faces;
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, RichText, Ui};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::{paint, Theme};

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

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// A colorful, slowly moving backdrop with fine stripes and type: great for showing refraction.
fn glass_backdrop(dark: bool, time: f32) -> impl Fn(&Painter, Rect) {
    move |p: &Painter, r: Rect| {
        let (a, b) = if dark { (hex("#0B1026"), hex("#1E1B4B")) } else { (hex("#FDF2F8"), hex("#E0E7FF")) };
        paint::gradient_rect(p, r, 0.0, 20.0, &[a, b]);
        let blobs = [
            (hex("#F472B6"), 0.0f32, 0.55f32),
            (hex("#6366F1"), 2.1, 0.62),
            (hex("#22D3EE"), 4.2, 0.5),
            (hex("#FBBF24"), 1.1, 0.42),
        ];
        for (c, ph, s) in blobs.iter() {
            let x = r.center().x + (time * 0.31 + ph).sin() * r.width() * 0.32;
            let y = r.center().y + (time * 0.43 + ph * 1.7).cos() * r.height() * 0.3;
            let rad = r.height() * s;
            paint::glow(p, pos2(x, y), rad, with_alpha(*c, if dark { 0.85 } else { 0.7 }));
        }
        // diagonal stripes
        let step = 26.0;
        let n = ((r.width() + r.height()) / step) as i32 + 2;
        let sc = with_alpha(if dark { Color32::WHITE } else { hex("#312E81") }, 0.1);
        for k in 0..n {
            let x0 = r.min.x - r.height() + k as f32 * step;
            p.line_segment([pos2(x0, r.max.y), pos2(x0 + r.height(), r.min.y)], egui::Stroke::new(1.2, sc));
        }
        // dots
        let tc = if dark { Color32::WHITE } else { hex("#1E1B4B") };
        for y in 0..((r.height() / 40.0) as i32 + 1) {
            for x in 0..((r.width() / 40.0) as i32 + 1) {
                p.circle_filled(r.min + vec2(20.0 + x as f32 * 40.0, 20.0 + y as f32 * 40.0), 1.6, with_alpha(tc, 0.35));
            }
        }
        // type
        let words = ["GLASS", "LIGHT", "BEND"];
        for (i, w) in words.iter().enumerate() {
            let x = r.min.x + 40.0 + ((time * 26.0 + i as f32 * 260.0) % (r.width() + 200.0)) - 100.0;
            let y = r.min.y + r.height() * (0.25 + 0.27 * i as f32);
            p.text(pos2(x, y), Align2::CENTER_CENTER, w, FontId::proportional(64.0), with_alpha(tc, 0.85));
        }
    }
}

/// Generative "poster" art for the decay card.
fn poster(p: &Painter, r: Rect, palette: [Color32; 3], seed: u32) {
    paint::gradient_rect(p, r, 0.0, 160.0, &[palette[0], palette[1]]);
    let c = r.center() + vec2(0.0, -r.height() * 0.1);
    paint::gradient_circle(p, c, r.width() * 0.32, &[mix(palette[2], Color32::WHITE, 0.5), palette[2]]);
    for k in 0..7 {
        let y = c.y + r.width() * 0.05 + k as f32 * 9.0;
        let w = (r.width() * 0.32).powi(2) - (y - c.y).powi(2);
        if w > 0.0 {
            let hw = w.sqrt();
            p.line_segment([pos2(c.x - hw, y), pos2(c.x + hw, y)], egui::Stroke::new(3.0, palette[1]));
        }
    }
    // mountains
    let base = r.max.y - r.height() * 0.22;
    let pts = [
        pos2(r.min.x, base),
        pos2(r.min.x + r.width() * 0.3, base - r.height() * (0.2 + 0.05 * seed as f32)),
        pos2(r.min.x + r.width() * 0.55, base - r.height() * 0.06),
        pos2(r.min.x + r.width() * 0.8, base - r.height() * 0.24),
        pos2(r.max.x, base - r.height() * 0.1),
    ];
    paint::area_fill(p, &pts, r.max.y, palette[0], palette[0]);
    for k in 0..3 {
        let y = r.min.y + 26.0 + k as f32 * 12.0;
        p.line_segment([pos2(r.min.x + 18.0, y), pos2(r.min.x + 18.0 + 40.0 - k as f32 * 10.0, y)], egui::Stroke::new(2.0, with_alpha(Color32::WHITE, 0.7)));
    }
}

fn pixel_face(ui: &mut Ui, icon: &str, title: &str, sub: &str, bg: Option<[Color32; 2]>) {
    let t = Theme::of(ui);
    let r = ui.max_rect();
    let p = ui.painter();
    let fg = if let Some([a, b]) = bg {
        paint::gradient_rect(p, r, 14.0, 135.0, &[a, b]);
        Color32::WHITE
    } else {
        t.foreground
    };
    p.text(r.center() - vec2(0.0, 22.0), Align2::CENTER_CENTER, icon, FontId::proportional(64.0), fg);
    p.text(r.center() + vec2(0.0, 42.0), Align2::CENTER_CENTER, title, FontId::proportional(20.0), fg);
    p.text(r.center() + vec2(0.0, 66.0), Align2::CENTER_CENTER, sub, FontId::proportional(13.0), with_alpha(fg, 0.7));
}

fn bento_items() -> Vec<BentoItem> {
    vec![
        BentoItem::new("Insights", "Analytics", "Track user behavior"),
        BentoItem::new("Overview", "Dashboard", "Centralized data view"),
        BentoItem::new("Teamwork", "Collaboration", "Work together seamlessly with your whole team, in real time."),
        BentoItem::new("Efficiency", "Automation", "Streamline workflows and let the busywork run itself."),
        BentoItem::new("Connectivity", "Integration", "Connect favorite tools"),
        BentoItem::new("Protection", "Security", "Enterprise-grade protection"),
    ]
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "bubble-menu",
            name: "Bubble Menu",
            category: Category::Navigation,
            description: "A round menu button that bursts into tilted pill links on bouncy springs; hovered pills straighten and flood with color.",
            covers: &["React Bits: Bubble Menu"],
            code: r##"let r = BubbleMenu::new(&["Home", "About", "Projects", "Blog", "Contact"])
    .logo("◉ glowkit")
    .show(ui);
if let Some(i) = r.inner { /* navigate to item i */ }"##,
            show: |ui| {
                ui.add_space(8.0);
                column(ui, 860.0, |ui| {
                    BubbleMenu::new(&["Home", "About", "Projects", "Blog", "Contact"]).autoplay(true).height(350.0).show(ui);
                });
            },
        },
        Demo {
            id: "card-nav",
            name: "Card Nav",
            category: Category::Navigation,
            description: "A rounded navbar whose hamburger expands it downward into a row of colored link cards that slide up one by one.",
            covers: &["React Bits: Card Nav"],
            code: r##"let r = CardNav::new(vec![
    CardNavItem::new("About", &["Company", "Careers"]),
    CardNavItem::new("Projects", &["Featured", "Case Studies"]),
    CardNavItem::new("Contact", &["Email", "Twitter", "LinkedIn"]),
])
.logo("◆ glowkit")
.cta("Get Started")
.show(ui);
if let Some((card, link)) = r.inner { /* ... */ }"##,
            show: |ui| {
                ui.add_space(24.0);
                CardNav::new(vec![
                    CardNavItem::new("About", &["Company", "Careers"]),
                    CardNavItem::new("Projects", &["Featured", "Case Studies"]),
                    CardNavItem::new("Contact", &["Email", "Twitter", "LinkedIn"]),
                ])
                .autoplay(true)
                .show(ui);
            },
        },
        Demo {
            id: "pill-nav",
            name: "Pill Nav",
            category: Category::Navigation,
            description: "A capsule of pill links: hovering raises a circle from below that floods the pill while its label rolls over.",
            covers: &["React Bits: Pill Nav"],
            code: r##"ui.add(PillNav::new(&mut active, &["Home", "About", "Services", "Contact"]));
ui.add(PillNav::new(&mut active, &items).base_color(hex("#7C3AED")).logo("◆"));"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(70.0);
                let mut a: usize = mem(ui, "a", || 0);
                let mut b: usize = mem(ui, "b", || 1);
                let mut c: usize = mem(ui, "c", || 2);
                centered_row(ui, "a", 0.0, |ui| {
                    ui.add(PillNav::new(&mut a, &["Home", "About", "Services", "Contact"]).autoplay(true));
                });
                ui.add_space(40.0);
                centered_row(ui, "b", 0.0, |ui| {
                    ui.add(
                        PillNav::new(&mut b, &["Work", "Studio", "Journal", "Shop", "Say hi"])
                            .base_color(t.accent)
                            .pill_color(if t.dark { hex("#1E1B2E") } else { hex("#F5F3FF") })
                            .pill_text_color(if t.dark { Color32::WHITE } else { t.accent })
                            .hover_text_color(Color32::WHITE)
                            .logo("◆")
                            .autoplay(true),
                    );
                });
                ui.add_space(40.0);
                centered_row(ui, "c", 0.0, |ui| {
                    ui.add(
                        PillNav::new(&mut c, &["Docs", "Components", "Blocks", "Themes"])
                            .base_color(hex("#0EA5E9"))
                            .pill_color(Color32::WHITE)
                            .pill_text_color(hex("#0C4A6E"))
                            .height(38.0)
                            .font_size(13.5)
                            .logo("")
                            .autoplay(true),
                    );
                });
                set_mem(ui, "a", a);
                set_mem(ui, "b", b);
                set_mem(ui, "c", c);
            },
        },
        Demo {
            id: "staggered-menu",
            name: "Staggered Menu",
            category: Category::Navigation,
            description: "A full-height side menu: colored layers sweep in, staggered, followed by a panel of huge numbered links rising into place.",
            covers: &["React Bits: Staggered Menu"],
            code: r##"let r = StaggeredMenu::new(&["Home", "About", "Services", "Contact"])
    .socials(&["Twitter", "GitHub", "LinkedIn"])
    .show(ui, |ui| { ui.heading("Your page"); });
if let Some(i) = r.inner { /* ... */ }"##,
            show: |ui| {
                ui.add_space(4.0);
                column(ui, 880.0, |ui| {
                    StaggeredMenu::new(&["Home", "About", "Services", "Contact"]).autoplay(true).height(400.0).show(ui, |ui| {
                        let t = Theme::of(ui);
                        ui.add_space(40.0);
                        ui.label(RichText::new("Design that").size(46.0).color(t.foreground).strong());
                        ui.label(RichText::new("moves people.").size(46.0).color(t.accent).strong());
                        ui.add_space(12.0);
                        ui.label(RichText::new("A side menu with staggered layers and big, numbered links.").size(15.0).color(t.muted_foreground));
                    });
                });
            },
        },
        Demo {
            id: "infinite-menu",
            name: "Infinite Menu",
            category: Category::Navigation,
            description: "A sphere of round tiles you drag to spin; it coasts, snaps the nearest tile to the front and shows its title and action.",
            covers: &["React Bits: Infinite Menu"],
            code: r##"let items = vec![CardFace::new("🌊", "Ocean").subtitle("Deep blue calm"), /* ... */];
let r = InfiniteMenu::new(items).show(ui);
if let Some(i) = r.inner { /* open item i */ }"##,
            show: |ui| {
                InfiniteMenu::new(sample_faces()).autoplay(true).height(390.0).show(ui);
            },
        },
        Demo {
            id: "magic-bento",
            name: "Magic Bento",
            category: Category::CardsAndBorders,
            description: "A bento grid where a spotlight follows the pointer and lights card borders; the hovered card tilts and sparkles with particles.",
            covers: &["React Bits: Magic Bento"],
            code: r##"let items = vec![
    BentoItem::new("Insights", "Analytics", "Track user behavior"),
    // ... six items use the classic bento layout
];
MagicBento::new(items).glow_color(hex("#8400FF")).show(ui);"##,
            show: |ui| {
                ui.add_space(8.0);
                column(ui, 860.0, |ui| {
                    MagicBento::new(bento_items()).autoplay(true).height(380.0).show(ui);
                });
            },
        },
        Demo {
            id: "chroma-grid",
            name: "Chroma Grid",
            category: Category::CardsAndBorders,
            description: "Profile tiles rendered in grayscale, with a circle of full color that follows the pointer across the grid.",
            covers: &["React Bits: Chroma Grid"],
            code: r##"ChromaGrid::new(vec![
    ChromaItem::new("Ada Park", "Frontend Engineer", "@ada"),
    ChromaItem::new("Leo Diaz", "Product Designer", "@leo"),
])
.spotlight_radius(200.0)
.show(ui);"##,
            show: |ui| {
                ui.add_space(30.0);
                ChromaGrid::new(vec![
                    ChromaItem::new("Ada Park", "Frontend Engineer", "@ada"),
                    ChromaItem::new("Leo Diaz", "Product Designer", "@leo"),
                    ChromaItem::new("Mia Chen", "Data Scientist", "@miac"),
                    ChromaItem::new("Kai Moss", "Rust Developer", "@kai"),
                ])
                .autoplay(true)
                .show(ui);
            },
        },
        Demo {
            id: "profile-card",
            name: "Profile Card",
            category: Category::CardsAndBorders,
            description: "A holographic trading-card profile that tilts toward the pointer while rainbow foil and a glare slide across it.",
            covers: &["React Bits: Profile Card"],
            code: r##"let r = ProfileCard::new("Ada Park", "Software Engineer")
    .handle("adapark")
    .status("Online")
    .show(ui);
if r.inner { /* contact clicked */ }"##,
            show: |ui| {
                centered_row(ui, "cards", 30.0, |ui| {
                    ProfileCard::new("Ada Park", "Software Engineer").handle("adapark").autoplay(true).show(ui);
                    ProfileCard::new("Leo Diaz", "Motion Designer")
                        .handle("leodiaz")
                        .status("Away")
                        .colors(hex("#052E2B"), hex("#22D3EE"))
                        .portrait(2)
                        .autoplay(true)
                        .show(ui);
                });
            },
        },
        Demo {
            id: "glass-icons",
            name: "Glass Icons",
            category: Category::Buttons,
            description: "Frosted glass tiles floating over tilted, vividly colored backplates; hover swings the plate and lifts the glass.",
            covers: &["React Bits: Glass Icons"],
            code: r##"let r = GlassIcons::new(vec![
    GlassIcon::new("📁", "Files").colors(hex("#3B82F6"), hex("#6366F1")),
    GlassIcon::new("📖", "Books"),
    GlassIcon::new("♥", "Health"),
]).show(ui);"##,
            show: |ui| {
                ui.add_space(90.0);
                centered_row(ui, "icons", 0.0, |ui| {
                    GlassIcons::new(vec![
                        GlassIcon::new("📁", "Files").colors(hex("#3B82F6"), hex("#6366F1")),
                        GlassIcon::new("📖", "Books").colors(hex("#A855F7"), hex("#7C3AED")),
                        GlassIcon::new("♥", "Health").colors(hex("#F43F5E"), hex("#E11D48")),
                        GlassIcon::new("☁", "Weather").colors(hex("#6366F1"), hex("#4338CA")),
                        GlassIcon::new("✎", "Notes").colors(hex("#F97316"), hex("#EA580C")),
                        GlassIcon::new("📊", "Stats").colors(hex("#22C55E"), hex("#15803D")),
                    ])
                    .size(76.0)
                    .autoplay(true)
                    .show(ui);
                });
            },
        },
        Demo {
            id: "glass-surface",
            name: "Glass Surface",
            category: Category::CardsAndBorders,
            description: "A refracting glass panel: the backdrop bends and compresses toward its rim with color fringes and specular edges.",
            covers: &["React Bits: Glass Surface"],
            code: r##"GlassSurface::new()
    .size(vec2(360.0, 180.0))
    .refraction(24.0)
    .show(ui, |painter, rect| { /* paint your backdrop */ }, |ui| {
        ui.heading("Glass Surface");
    });"##,
            show: |ui| {
                let t = Theme::of(ui);
                let time = glowkit_core::time(ui);
                glowkit_core::animate(ui);
                GlassSurface::new().size(vec2(380.0, 190.0)).height(380.0).refraction(30.0).blur(2.5).show(
                    ui,
                    glass_backdrop(t.dark, time),
                    |ui| {
                        let fg = if t.dark { Color32::WHITE } else { hex("#1E1B4B") };
                        ui.label(RichText::new("Glass Surface").size(30.0).strong().color(fg));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Refraction, dispersion and light").size(14.0).color(with_alpha(fg, 0.75)));
                    },
                );
            },
        },
        Demo {
            id: "fluid-glass",
            name: "Fluid Glass",
            category: Category::CardsAndBorders,
            description: "A liquid glass lens that glides after the pointer, magnifying and bending everything beneath it with rainbow edges.",
            covers: &["React Bits: Fluid Glass", "React Bits: Glass Surface"],
            code: r##"GlassSurface::new()
    .size(vec2(180.0, 180.0))
    .lens(true)
    .refraction(40.0)
    .magnify(1.15)
    .wobble(1.0)
    .follow_pointer(true)
    .show(ui, |painter, rect| { /* backdrop */ }, |_| {});"##,
            show: |ui| {
                let t = Theme::of(ui);
                let time = glowkit_core::time(ui);
                glowkit_core::animate(ui);
                GlassSurface::new()
                    .size(vec2(190.0, 190.0))
                    .height(380.0)
                    .lens(true)
                    .refraction(60.0)
                    .magnify(1.3)
                    .wobble(1.0)
                    .follow_pointer(true)
                    .autoplay(true)
                    .show(ui, glass_backdrop(t.dark, time * 0.4), |_| {});
            },
        },
        Demo {
            id: "decay-card",
            name: "Decay Card",
            category: Category::CardsAndBorders,
            description: "A poster card whose picture tears and ripples with turbulence as the pointer moves over it, then slowly heals.",
            covers: &["React Bits: Decay Card"],
            code: r##"DecayCard::new().title("Decay").show(ui, |painter, rect| {
    // paint the picture
});"##,
            show: |ui| {
                ui.add_space(10.0);
                centered_row(ui, "decay", 40.0, |ui| {
                    DecayCard::new().size(vec2(250.0, 320.0)).title("Decay").autoplay(true).show(ui, |p, r| {
                        poster(p, r, [hex("#1E1B4B"), hex("#DB2777"), hex("#FBBF24")], 1)
                    });
                    DecayCard::new().size(vec2(250.0, 320.0)).title("Drift").intensity(0.7).autoplay(true).show(ui, |p, r| {
                        poster(p, r, [hex("#042F2E"), hex("#0EA5E9"), hex("#A7F3D0")], 2)
                    });
                });
            },
        },
        Demo {
            id: "sticker-peel",
            name: "Sticker Peel",
            category: Category::CursorAndInteraction,
            description: "A die-cut sticker you drag to peel: the corner folds back along a crease, showing its paper back and a curl shadow.",
            covers: &["React Bits: Sticker Peel"],
            code: r##"ui.add(StickerPeel::new("🦀", "Rust").size(vec2(180.0, 180.0)));"##,
            show: |ui| {
                ui.add_space(40.0);
                centered_row(ui, "stickers", 30.0, |ui| {
                    ui.add(StickerPeel::new("🦀", "Rust").colors(hex("#F97316"), hex("#DC2626")).autoplay(true));
                    ui.add(StickerPeel::new("✦", "glowkit").autoplay(true).radius(90.0));
                    ui.add(StickerPeel::new("🌊", "egui").colors(hex("#06B6D4"), hex("#4F46E5")).radius(18.0).autoplay(true));
                });
            },
        },
        Demo {
            id: "magnet-lines",
            name: "Magnet Lines",
            category: Category::CursorAndInteraction,
            description: "A grid of short lines that all swivel to point at the pointer, glowing in the accent color nearby.",
            covers: &["React Bits: Magnet Lines"],
            code: r##"ui.add(MagnetLines::new(9, 9).size(vec2(320.0, 320.0)));"##,
            show: |ui| {
                ui.add_space(20.0);
                centered_row(ui, "lines", 40.0, |ui| {
                    ui.add(MagnetLines::new(9, 9).size(vec2(320.0, 320.0)).autoplay(true));
                    ui.add(
                        MagnetLines::new(14, 18)
                            .size(vec2(420.0, 320.0))
                            .line_width(2.0)
                            .line_length(0.7)
                            .near_color(hex("#F472B6"))
                            .autoplay(true),
                    );
                });
            },
        },
        Demo {
            id: "pixel-transition",
            name: "Pixel Transition",
            category: Category::CardsAndBorders,
            description: "A card that swaps to its second face through a pixel dissolve: squares pop in at random, then away again.",
            covers: &["React Bits: Pixel Transition"],
            code: r##"PixelTransition::new().grid(10).show(
    ui,
    |ui| { /* first content */ },
    |ui| { /* shown on hover */ },
);"##,
            show: |ui| {
                ui.add_space(40.0);
                centered_row(ui, "px", 36.0, |ui| {
                    PixelTransition::new().size(vec2(250.0, 280.0)).autoplay(true).show(
                        ui,
                        |ui| pixel_face(ui, "🦀", "Hover me", "A pixel dissolve", None),
                        |ui| pixel_face(ui, "✦", "Surprise!", "Second content", Some([hex("#7C3AED"), hex("#DB2777")])),
                    );
                    PixelTransition::new().size(vec2(250.0, 280.0)).grid(14).duration(0.8).pixel_color(hex("#22D3EE")).autoplay(true).show(
                        ui,
                        |ui| pixel_face(ui, "🌙", "Night", "Fine 14 x 14 grid", Some([hex("#0F172A"), hex("#1E3A8A")])),
                        |ui| pixel_face(ui, "☀", "Day", "Cyan pixels", Some([hex("#F59E0B"), hex("#F97316")])),
                    );
                });
            },
        },
        Demo {
            id: "shape-blur",
            name: "Shape Blur",
            category: Category::CursorAndInteraction,
            description: "Soft, out-of-focus outlines that snap into crisp focus wherever the pointer gets close.",
            covers: &["React Bits: Shape Blur"],
            code: r##"ui.add(ShapeBlur::new(ShapeBlurKind::RoundedRect).size(vec2(240.0, 240.0)).blur(18.0));
ui.add(ShapeBlur::new(ShapeBlurKind::Circle).rings(3).colors(hex("#A855F7"), hex("#22D3EE")));"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(40.0);
                centered_row(ui, "shapes", 24.0, |ui| {
                    ui.add(ShapeBlur::new(ShapeBlurKind::RoundedRect).size(vec2(260.0, 260.0)).autoplay(true));
                    ui.add(ShapeBlur::new(ShapeBlurKind::Circle).size(vec2(260.0, 260.0)).rings(3).colors(t.gradient[0], hex("#22D3EE")).autoplay(true));
                    ui.add(ShapeBlur::new(ShapeBlurKind::Star).size(vec2(260.0, 260.0)).colors(t.gradient[1], t.gradient[2]).stroke(2.5).autoplay(true));
                });
            },
        },
    ]
}
