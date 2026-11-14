use super::common::{icon_tile, sample_faces, theme_card};
use super::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, Ui};
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::demo::{Category::Motion as C, Demo};
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

fn dock_items() -> Vec<DockItem> {
    vec![
        DockItem::new("🏠", "Home").colors(hex("#60A5FA"), hex("#2563EB")).active(true),
        DockItem::new("✉", "Mail").colors(hex("#38BDF8"), hex("#0284C7")),
        DockItem::new("🎵", "Music").colors(hex("#FB7185"), hex("#E11D48")).active(true),
        DockItem::new("📷", "Photos").colors(hex("#FBBF24"), hex("#F97316")),
        DockItem::new("💬", "Messages").colors(hex("#4ADE80"), hex("#16A34A")),
        DockItem::new("📅", "Calendar").colors(hex("#F87171"), hex("#DC2626")),
        DockItem::new("⚙", "Settings").colors(hex("#A1A1AA"), hex("#52525B")),
        DockItem::new("🗑", "Trash").colors(hex("#94A3B8"), hex("#475569")),
    ]
}

fn logo(ui: &mut Ui, icon: &str, name: &str) {
    let t = Theme::of(ui);
    ui.add_space(14.0);
    ui.label(RichText::new(icon).size(22.0).color(t.foreground));
    ui.label(RichText::new(name).size(20.0).strong().color(t.foreground));
    ui.add_space(14.0);
}

fn review(ui: &mut Ui, i: usize) {
    let t = Theme::of(ui);
    const R: [(&str, &str, &str); 6] = [
        ("Ava", "@ava", "Shipped our dashboard in a weekend. The springs feel native."),
        ("Liam", "@liam_rs", "Finally, React Bits energy in a Rust desktop app."),
        ("Mia", "@miacodes", "The marquee and dock alone sold my team on egui."),
        ("Noah", "@noah", "Runs at 120fps in the browser. Wild."),
        ("Zoe", "@zoe.dev", "Theme presets made light mode a non-issue."),
        ("Kai", "@kai", "Docs with live previews. Chef's kiss."),
    ];
    let (name, handle, text) = R[i % R.len()];
    egui::Frame::new()
        .fill(if t.dark { shade(t.card, 0.03) } else { t.card })
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(250.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    paint::gradient_circle(
                        ui.painter(),
                        r.center(),
                        16.0,
                        &[t.chart[i % 6], shade(t.chart[(i + 2) % 6], -0.1)],
                    );
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        &name[..1],
                        FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(name).size(13.5).strong().color(t.foreground));
                        ui.label(RichText::new(handle).size(12.0).color(t.muted_foreground));
                    });
                });
                ui.add_space(4.0);
                ui.label(RichText::new(text).size(13.0).color(t.foreground));
            });
        });
}

fn notification(ui: &mut Ui, i: usize) {
    let t = Theme::of(ui);
    const N: [(&str, &str, &str, &str); 6] = [
        ("💸", "Payment received", "Stripe · $1,250.00 from Acme Inc.", "#22C55E"),
        ("👤", "New user signed up", "maya@studio.dev joined your workspace", "#8B5CF6"),
        ("💬", "New message", "Leo: \"Can we ship this today?\"", "#F59E0B"),
        ("🚀", "Deploy succeeded", "glowkit-web · production · 42s", "#0EA5E9"),
        ("⭐", "New star", "Someone starred glowkit on GitHub", "#EC4899"),
        ("🔔", "Reminder", "Design review in 15 minutes", "#EF4444"),
    ];
    let (icon, title, body, col) = N[i % N.len()];
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 66.0), Sense::hover());
    let p = ui.painter();
    theme_card(p, &t, r, 14.0, 0.6);
    let tile = Rect::from_center_size(pos2(r.min.x + 33.0, r.center().y), vec2(40.0, 40.0));
    p.rect_filled(tile, CornerRadius::same(12), hex(col));
    p.text(tile.center(), Align2::CENTER_CENTER, icon, FontId::proportional(18.0), Color32::WHITE);
    let x = tile.max.x + 12.0;
    let g = p.text(pos2(x, r.center().y - 9.0), Align2::LEFT_CENTER, title, FontId::proportional(14.5), t.foreground);
    p.text(
        pos2(g.max.x + 6.0, r.center().y - 9.0),
        Align2::LEFT_CENTER,
        format!("· {}m ago", 2 + i * 3),
        FontId::proportional(12.0),
        t.muted_foreground,
    );
    p.text(pos2(x, r.center().y + 11.0), Align2::LEFT_CENTER, body, FontId::proportional(12.5), t.muted_foreground);
}

fn slide_card(ui: &mut Ui, i: usize) {
    let t = Theme::of(ui);
    let faces = sample_faces();
    let f = &faces[i % faces.len()];
    let r = ui.max_rect();
    let p = ui.painter();
    let art = Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.62));
    let [a, b] = f.resolved_colors(&t, i);
    paint::gradient_rect(p, art, 10.0, 35.0, &[a, b]);
    p.text(art.center(), Align2::CENTER_CENTER, &f.icon, FontId::proportional(44.0), Color32::WHITE);
    p.text(pos2(r.min.x, art.max.y + 14.0), Align2::LEFT_TOP, &f.title, FontId::proportional(17.0), t.foreground);
    p.text(
        pos2(r.min.x, art.max.y + 38.0),
        Align2::LEFT_TOP,
        &f.subtitle,
        FontId::proportional(13.0),
        t.muted_foreground,
    );
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "dock",
            name: "Dock",
            category: C,
            description: "A macOS-style dock: icons magnify around the pointer, show a label and bounce when clicked.",
            covers: &["Magic UI: Dock", "React Bits: Dock", "Aceternity: Floating Dock"],
            code: r##"let items = [DockItem::new("🏠", "Home").active(true), DockItem::new("✉", "Mail")];
let r = Dock::new(&items).magnification(1.75).show(ui);
if let Some(i) = r.clicked { /* open items[i] */ }"##,
            show: |ui| {
                let items = dock_items();
                ui.add_space(20.0);
                Dock::new(&items).autoplay(true).show(ui);
                ui.add_space(24.0);
                let minimal: Vec<DockItem> = ["⌂", "◎", "✎", "☰", "♫", "⚙"].iter().zip(["Home", "Explore", "Write", "Library", "Music", "Settings"]).map(|(i, l)| DockItem::new(*i, l)).collect();
                Dock::new(&minimal).style(DockStyle::Minimal).size(40.0).magnification(1.6).autoplay(true).show(ui);
            },
        },
        Demo {
            id: "marquee",
            name: "Marquee",
            category: C,
            description: "Infinite scrolling strips of any content with faded edges; hover to pause, reverse for variety.",
            covers: &["Magic UI: Marquee", "React Bits: Logo Loop", "Aceternity: Infinite Moving Cards"],
            code: r##"Marquee::new().speed(50.0).pause_on_hover(true).show(ui, |ui| {
    for name in ["Rust", "egui", "WebAssembly"] {
        ui.label(name);
    }
});"##,
            show: |ui| {
                ui.add_space(10.0);
                let logos = [("🦀", "Rustacean"), ("⚡", "Voltage"), ("◆", "Prism"), ("☁", "Nimbus"), ("✦", "Stellar"), ("❖", "Quartz"), ("◐", "Eclipse")];
                Marquee::new().speed(45.0).gap(0.0).id_salt(0).show(ui, |ui| {
                    for (i, n) in logos {
                        logo(ui, i, n);
                    }
                });
                ui.add_space(18.0);
                Marquee::new().speed(30.0).id_salt(1).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for i in 0..3 {
                        review(ui, i);
                    }
                });
                ui.add_space(8.0);
                Marquee::new().speed(30.0).reverse(true).id_salt(2).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for i in 3..6 {
                        review(ui, i);
                    }
                });
            },
        },
        Demo {
            id: "animated-list",
            name: "Animated List",
            category: C,
            description: "Items pop in one after another, newest on top, pushing older ones down: a live notification feed.",
            covers: &["Magic UI: Animated List", "React Bits: Animated List"],
            code: r##"AnimatedList::new(messages.len()).delay(1.2).show(ui, |ui, i| {
    ui.label(messages[i]);
});"##,
            show: |ui| {
                ui.add_space(8.0);
                column(ui, 420.0, |ui| {
                    AnimatedList::new(6).delay(0.9).height(380.0).show(ui, notification);
                });
            },
        },
        Demo {
            id: "stack-cards",
            name: "Stack",
            category: C,
            description: "A draggable card stack: fling the top card away and it tucks itself in at the back.",
            covers: &["React Bits: Stack", "Tinder-style swipe cards"],
            code: r##"let faces = [CardFace::new("🌊", "Ocean"), CardFace::new("🌅", "Sunset")];
ui.add(StackCards::new(&faces).card_size(vec2(200.0, 260.0)));"##,
            show: |ui| {
                let faces = sample_faces();
                ui.add_space(20.0);
                ui.add(StackCards::new(&faces[..5]).autoplay(2.2));
            },
        },
        Demo {
            id: "card-swap",
            name: "Card Swap",
            category: C,
            description: "A skewed 3D stack that cycles: the front card drops away and reappears at the back.",
            covers: &["React Bits: Card Swap"],
            code: r##"ui.add(CardSwap::new(&faces).interval(3.0).skew(6.0));"##,
            show: |ui| {
                let faces = vec![
                    CardFace::new("⚙", "Customizable").subtitle("Every color is a token").colors(hex("#6366F1"), hex("#312E81")),
                    CardFace::new("🛡", "Reliable").subtitle("Pure Rust, no surprises").colors(hex("#0EA5E9"), hex("#1E3A8A")),
                    CardFace::new("✨", "Smooth").subtitle("Springs everywhere").colors(hex("#EC4899"), hex("#7C3AED")),
                    CardFace::new("🌐", "Everywhere").subtitle("Native and WebAssembly").colors(hex("#10B981"), hex("#065F46")),
                ];
                ui.add_space(10.0);
                ui.add(CardSwap::new(&faces).interval(2.4));
            },
        },
        Demo {
            id: "bounce-cards",
            name: "Bounce Cards",
            category: C,
            description: "A fan of cards bounces in one by one; hovering straightens a card and pushes its neighbors aside.",
            covers: &["React Bits: Bounce Cards"],
            code: r##"ui.add(BounceCards::new(&faces).push(70.0));"##,
            show: |ui| {
                let faces = sample_faces();
                ui.add_space(40.0);
                ui.add(BounceCards::new(&faces[..5]).autoplay(true));
            },
        },
        Demo {
            id: "carousel",
            name: "Carousel",
            category: C,
            description: "Drag or swipe between items that snap into place on a spring; neighbors peek in and dots track position.",
            covers: &["React Bits: Carousel", "Embla-style carousel"],
            code: r##"Carousel::new(5).autoplay(3.0).looping(true).show(ui, |ui, i| {
    ui.heading(format!("Slide {i}"));
});"##,
            show: |ui| {
                ui.add_space(14.0);
                Carousel::new(6).item_size(vec2(300.0, 250.0)).autoplay(2.2).looping(true).show(ui, slide_card);
            },
        },
        Demo {
            id: "ring-carousel",
            name: "Ring Carousel",
            category: C,
            description: "Cards on a rotating 3D ring with real perspective. Drag to spin; it coasts and drifts when idle.",
            covers: &["React Bits: Circular Gallery", "React Bits: Rolling Gallery", "3D carousel"],
            code: r##"ui.add(RingCarousel::new(&faces).speed(14.0));"##,
            show: |ui| {
                let faces = sample_faces();
                ui.add_space(20.0);
                ui.add(RingCarousel::new(&faces).speed(16.0));
            },
        },
        Demo {
            id: "folder",
            name: "Folder",
            category: C,
            description: "A folder that opens on click: the flap tilts down, papers fan out and lean toward the pointer.",
            covers: &["React Bits: Folder"],
            code: r##"ui.add(Folder::new().color(hex("#5227FF")).size(1.4).papers(["Q3 plan", "Notes", "Ideas"]));"##,
            show: |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let w = 264.0 * (1.3 + 1.6 + 1.3) + 40.0;
                    ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 20.0;
                    ui.add(Folder::new().color(hex("#5227FF")).size(1.3).autoplay(true).id_salt(1));
                    ui.add(Folder::new().color(hex("#0EA5E9")).size(1.6).papers(["Q3 plan", "Notes", "Ideas"]).open(true).id_salt(2));
                    ui.add(Folder::new().color(hex("#F59E0B")).size(1.3).id_salt(3));
                });
            },
        },
        Demo {
            id: "elastic-slider",
            name: "Elastic Slider",
            category: C,
            description: "Drag past either end and the track stretches like rubber, then snaps back with a wobble.",
            covers: &["React Bits: Elastic Slider"],
            code: r##"ui.add(ElasticSlider::new(&mut volume, 0.0..=100.0).step(1.0));"##,
            show: |ui| {
                ui.add_space(90.0);
                ui.vertical_centered(|ui| {
                    let mut v: f32 = mem(ui, "v", || 50.0);
                    ui.add(ElasticSlider::new(&mut v, 0.0..=100.0).step(1.0).width(300.0));
                    set_mem(ui, "v", v);
                    ui.add_space(30.0);
                    let mut b: f32 = mem(ui, "b", || 0.65);
                    ui.add(ElasticSlider::new(&mut b, 0.0..=1.0).icons("🔅", "🔆").color(Theme::of(ui).accent).width(300.0));
                    set_mem(ui, "b", b);
                });
            },
        },
        Demo {
            id: "stepper",
            name: "Stepper",
            category: C,
            description: "A multi-step flow with animated indicators, sliding step content and a height that springs to fit.",
            covers: &["React Bits: Stepper"],
            code: r##"Stepper::new(&mut step, 4).show(ui, |ui, s| {
    ui.label(format!("Content of step {}", s + 1));
});"##,
            show: |ui| {
                ui.add_space(16.0);
                let mut step: usize = mem(ui, "step", || 1);
                Stepper::new(&mut step, 4).width(480.0).show(ui, |ui, s| {
                    let t = Theme::of(ui);
                    match s {
                        0 => {
                            ui.label(RichText::new("Welcome to glowkit 👋").size(18.0).color(t.foreground));
                            ui.label(RichText::new("Let's set up your first animated component.").color(t.muted_foreground));
                        }
                        1 => {
                            ui.label(RichText::new("Pick a theme").size(18.0).color(t.foreground));
                            ui.label(RichText::new("Every component reads its colors from the active theme, so light and dark both just work.").color(t.muted_foreground));
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                for c in t.chart {
                                    let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
                                    ui.painter().circle_filled(r.center(), 12.0, c);
                                }
                            });
                        }
                        2 => {
                            ui.label(RichText::new("Add it to your UI").size(18.0).color(t.foreground));
                            ui.label(RichText::new("ui.add(ShinyText::new(\"Hello\"));").monospace().color(t.accent));
                        }
                        _ => {
                            ui.label(RichText::new("Ship it 🚀").size(18.0).color(t.foreground));
                            ui.label(RichText::new("Native on Windows, macOS and Linux, and in the browser through WebAssembly.").color(t.muted_foreground));
                        }
                    }
                });
                set_mem(ui, "step", step);
            },
        },
        Demo {
            id: "animated-tabs",
            name: "Animated Tabs",
            category: C,
            description: "A highlight springs between tabs, stretching as it travels; a softer hover highlight follows the pointer.",
            covers: &["Motion Primitives: Animated Background", "Aceternity: Tabs", "Magic UI: Animated tabs", "shadcn: Tabs (animated)"],
            code: r##"ui.add(AnimatedTabs::new(&mut tab, &["Overview", "Usage", "API"]));
ui.add(AnimatedTabs::new(&mut tab, &labels).style(TabHighlight::Underline));"##,
            show: |ui| {
                ui.add_space(30.0);
                let now = glowkit_core::time(ui);
                let auto = ((now / 1.4) as usize) % 4;
                let labels = ["Overview", "Components", "Themes", "Changelog"];
                ui.vertical_centered(|ui| {
                    for (k, style) in [TabHighlight::Pill, TabHighlight::Solid, TabHighlight::Underline].into_iter().enumerate() {
                        let key = format!("tab{k}");
                        let mut sel: usize = mem(ui, &key, || 0);
                        let touched: bool = mem(ui, &format!("{key}_t"), || false);
                        if !touched {
                            sel = (auto + k) % 4;
                            glowkit_core::animate(ui);
                        }
                        let r = ui.add(AnimatedTabs::new(&mut sel, &labels).style(style));
                        if r.changed() {
                            set_mem(ui, &format!("{key}_t"), true);
                        }
                        set_mem(ui, &key, sel);
                        ui.add_space(26.0);
                    }
                });
            },
        },
        Demo {
            id: "expandable-card",
            name: "Expandable Card",
            category: C,
            description: "Compact cards grow in place into a detail panel over a dimmed backdrop, and shrink back on close.",
            covers: &["Aceternity: Expandable Card", "Framer Motion: shared layout"],
            code: r##"let items = [ExpandableItem::new("🎧", "Lo-fi Beats", "Chillhop").body("…").action("Play")];
ui.add(ExpandableCard::new(&items));"##,
            show: |ui| {
                let items = [
                    ExpandableItem::new("🎧", "Midnight Drive", "Lo-fi Collective").action("Play").colors(hex("#8B5CF6"), hex("#312E81")).body("A slow, warm set for late-night coding sessions: dusty drums, soft keys and tape hiss. 42 minutes, no ads, no interruptions."),
                    ExpandableItem::new("🌊", "Tidal", "Ocean Ambience").action("Play").colors(hex("#0EA5E9"), hex("#1E3A8A")).body("Field recordings from the Atlantic coast layered with low synth pads. Great for deep focus."),
                    ExpandableItem::new("🔥", "Ember", "Synthwave Nights").action("Play").colors(hex("#F97316"), hex("#BE123C")).body("Retro arpeggios and gated reverb drums straight out of an 80s road movie."),
                    ExpandableItem::new("🌲", "Pinewood", "Acoustic Mornings").action("Play").colors(hex("#10B981"), hex("#065F46")).body("Fingerpicked guitar and birdsong for slow starts."),
                ];
                ui.add(ExpandableCard::new(&items).autoplay(true));
            },
        },
        Demo {
            id: "flowing-menu",
            name: "Flowing Menu",
            category: C,
            description: "Hovering a row floods it with a contrasting band, entering from the pointer's side, carrying a marquee.",
            covers: &["React Bits: Flowing Menu"],
            code: r##"let r = FlowingMenu::new(&[("🌊", "Mojave"), ("🌲", "Sonoma")]).show(ui);"##,
            show: |ui| {
                ui.add_space(10.0);
                FlowingMenu::new(&[("🏜", "Mojave"), ("🌲", "Sonoma"), ("🏔", "Monterey"), ("🌊", "Sequoia")]).autoplay(true).show(ui);
            },
        },
        Demo {
            id: "gooey-nav",
            name: "Gooey Nav",
            category: C,
            description: "The active pill slides to the clicked item while colorful particles burst out and melt back in.",
            covers: &["React Bits: Gooey Nav"],
            code: r##"ui.add(GooeyNav::new(&mut active, &["Home", "About", "Contact"]));"##,
            show: |ui| {
                ui.add_space(70.0);
                ui.vertical_centered(|ui| {
                    let mut sel: usize = mem(ui, "nav", || 0);
                    ui.add(GooeyNav::new(&mut sel, &["Home", "About", "Work", "Blog", "Contact"]).autoplay(2.0));
                    set_mem(ui, "nav", sel);
                });
            },
        },
        Demo {
            id: "orbiting-circles",
            name: "Orbiting Circles",
            category: C,
            description: "Icons circle a center on concentric rings, each ring with its own speed and direction.",
            covers: &["Magic UI: Orbiting Circles"],
            code: r##"ui.add(OrbitingCircles::new()
    .center("✦")
    .ring(80.0, &["🦀", "⚡"], 12.0)
    .reverse_ring(150.0, &["🌐", "🎨", "📦", "🔒"], 24.0));"##,
            show: |ui| {
                ui.add(
                    OrbitingCircles::new()
                        .center("✦")
                        .ring(78.0, &["🦀", "⚡", "🧩"], 10.0)
                        .icon_size(34.0)
                        .reverse_ring(150.0, &["🌐", "🎨", "📦", "🔒", "🚀"], 22.0)
                        .icon_size(44.0),
                );
            },
        },
        Demo {
            id: "animated-beam",
            name: "Animated Beam",
            category: C,
            description: "Glowing gradient pulses travel along curved paths between nodes: data flowing through a system.",
            covers: &["Magic UI: Animated Beam"],
            code: r##"AnimatedBeam::new(a.center(), hub.center()).curvature(-60.0).paint(ui);
AnimatedBeam::new(hub.center(), b.center()).reverse(true).paint(ui);
// then paint the nodes on top"##,
            show: |ui| {
                let t = Theme::of(ui);
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                let w = rect.width().min(640.0);
                let c = rect.center();
                let hub = c;
                let user = c + vec2(w * 0.42, 0.0);
                let ins: Vec<egui::Pos2> = (0..5).map(|i| c + vec2(-w * 0.42, (i as f32 - 2.0) * 66.0)).collect();
                for (i, p) in ins.iter().enumerate() {
                    AnimatedBeam::new(*p, hub).curvature((i as f32 - 2.0) * -18.0).delay(i as f32 * 0.45).duration(2.6).paint(ui);
                }
                AnimatedBeam::new(hub, user).duration(2.6).colors(t.gradient[1], t.gradient[2]).paint(ui);
                AnimatedBeam::new(hub, user).duration(2.6).delay(1.3).reverse(true).curvature(0.0).colors(t.chart[1], t.chart[0]).paint(ui);
                let p = ui.painter();
                let node = |pos: egui::Pos2, r: f32, icon: &str| {
                    paint::soft_shadow(p, Rect::from_center_size(pos, vec2(r * 2.0, r * 2.0)), r, 16.0, vec2(0.0, 4.0), with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.12 }));
                    p.circle(pos, r, if t.dark { mix(t.card, t.secondary, 0.4) } else { t.card }, Stroke::new(1.0, t.border));
                    p.text(pos, Align2::CENTER_CENTER, icon, FontId::proportional(r * 0.9), t.foreground);
                };
                for (p_, icon) in ins.iter().zip(["📁", "✉", "💬", "📊", "☁"]) {
                    node(*p_, 24.0, icon);
                }
                paint::glow(p, hub, 70.0, with_alpha(t.accent, if t.dark { 0.3 } else { 0.15 }));
                node(hub, 36.0, "✦");
                node(user, 28.0, "👤");
            },
        },
        Demo {
            id: "infinite-scroll",
            name: "Infinite Scroll",
            category: C,
            description: "An endless, wrapping vertical list: scroll or drag with inertia, or let it drift. Edges fade out.",
            covers: &["React Bits: Infinite Scroll", "Magic UI: Marquee (vertical)"],
            code: r##"InfiniteScroll::new(8).autoplay(30.0).show(ui, |ui, i| {
    ui.label(format!("Item {i}"));
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(4.0);
                InfiniteScroll::new(8).width(380.0).height(380.0).autoplay(36.0).show(ui, |ui, i| {
                    let r = ui.max_rect();
                    let p = ui.painter();
                    theme_card(p, &t, r, 14.0, 0.0);
                    let faces = sample_faces();
                    let f = &faces[i % faces.len()];
                    let [a, b] = f.resolved_colors(&t, i);
                    icon_tile(p, Rect::from_center_size(pos2(r.min.x + 38.0, r.center().y), vec2(44.0, 44.0)), 12.0, [a, b], &f.icon, 1.0);
                    p.text(pos2(r.min.x + 74.0, r.center().y - 9.0), Align2::LEFT_CENTER, format!("{} {:02}", f.title, i + 1), FontId::proportional(15.0), t.foreground);
                    p.text(pos2(r.min.x + 74.0, r.center().y + 11.0), Align2::LEFT_CENTER, &f.subtitle, FontId::proportional(12.5), t.muted_foreground);
                });
            },
        },
        Demo {
            id: "masonry",
            name: "Masonry",
            category: C,
            description: "A masonry grid whose tiles glide to their new places on springs whenever the layout changes.",
            covers: &["React Bits: Masonry", "Magic UI: Bento-style grid reflow"],
            code: r##"Masonry::new(&heights).order(&order).columns(4).show(ui, |ui, i, rect| {
    ui.painter().rect_filled(rect, 12, colors[i]);
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                let heights = [150.0, 100.0, 190.0, 120.0, 90.0, 170.0, 130.0, 110.0, 160.0, 95.0];
                let mut order: Vec<usize> = mem(ui, "order", || (0..heights.len()).collect());
                let mut cols: usize = mem(ui, "cols", || 5);
                let now = glowkit_core::time(ui);
                let start: f32 = mem(ui, "start", || now);
                let tick = ((now - start) / 4.0) as u64;
                let last: u64 = mem(ui, "tick", || tick);
                ui.horizontal(|ui| {
                    if ui.button("🔀 Shuffle").clicked() {
                        let mut rng = glowkit_core::Rng::new(tick.wrapping_mul(2654435761) ^ (now * 1000.0) as u64);
                        for i in (1..order.len()).rev() {
                            order.swap(i, rng.below(i + 1));
                        }
                    } else if last != tick {
                        // idle demo: move the first tile to the end so everything reflows one slot
                        order.rotate_left(1);
                    }
                    for c in [3usize, 4, 5] {
                        if ui.selectable_label(cols == c, format!("{c} columns")).clicked() {
                            cols = c;
                        }
                    }
                });
                set_mem(ui, "tick", tick);
                glowkit_core::animate(ui);
                ui.add_space(10.0);
                let faces = sample_faces();
                column(ui, 860.0, |ui| {
                    Masonry::new(&heights).order(&order).columns(cols).show(ui, |ui, i, r| {
                    let f = &faces[i % faces.len()];
                    let [a, b] = f.resolved_colors(&t, i);
                    let p = ui.painter();
                    paint::gradient_rect(p, r, 12.0, 60.0, &[a, b]);
                    p.text(r.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, &f.icon, FontId::proportional(26.0), Color32::WHITE);
                    p.text(r.center() + vec2(0.0, 20.0), Align2::CENTER_CENTER, &f.title, FontId::proportional(13.0), with_alpha(Color32::WHITE, 0.9));
                    });
                });
                set_mem(ui, "order", order);
                set_mem(ui, "cols", cols);
            },
        },
        Demo {
            id: "spring-presence",
            name: "Presence & Transitions",
            category: C,
            description: "Animate content in and out on a spring (fade, slide, scale, pop), and switch panels with transitions.",
            covers: &["Framer Motion: AnimatePresence", "Motion Primitives: Transition Panel", "Magic UI: Blur Fade", "Motion Primitives: In View"],
            code: r##"SpringPresence::new(open).effect(PresenceEffect::Pop).show(ui, |ui| {
    ui.label("Hello!");
});
TransitionPanel::new(tab).show(ui, |ui, i| { ui.label(format!("Panel {i}")); });"##,
            show: |ui| {
                let t = Theme::of(ui);
                let now = glowkit_core::time(ui);
                glowkit_core::animate(ui);
                ui.columns(2, |cols| {
                    let ui = &mut cols[0];
                    ui.label(RichText::new("SpringPresence").size(13.0).color(t.muted_foreground));
                    ui.add_space(6.0);
                    let effects = [(PresenceEffect::SlideUp, "Slide up"), (PresenceEffect::Pop, "Pop"), (PresenceEffect::Scale, "Scale"), (PresenceEffect::SlideLeft, "Slide left")];
                    for (k, (e, name)) in effects.into_iter().enumerate() {
                        let visible = ((now / 1.6 + k as f32 * 0.25) as usize) % 2 == 0;
                        let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                        let mut slot_ui = ui.new_child(egui::UiBuilder::new().max_rect(slot).id_salt(("slot", k)));
                        SpringPresence::new(visible).effect(e).distance(24.0).animate_height(false).id_salt(k).show(&mut slot_ui, |ui| {
                            egui::Frame::new()
                                .fill(if t.dark { shade(t.card, 0.04) } else { t.card })
                                .stroke(Stroke::new(1.0, t.border))
                                .corner_radius(CornerRadius::same(12))
                                .inner_margin(egui::Margin::symmetric(14, 10))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 5.0, t.chart[k]);
                                        ui.label(RichText::new(name).size(14.0).color(t.foreground));
                                        ui.label(RichText::new("PresenceEffect").size(12.0).color(t.muted_foreground));
                                    });
                                });
                        });
                        ui.add_space(8.0);
                    }
                    let ui = &mut cols[1];
                    ui.label(RichText::new("TransitionPanel").size(13.0).color(t.muted_foreground));
                    ui.add_space(6.0);
                    let mut tab: usize = mem(ui, "tp", || 0);
                    let touched: bool = mem(ui, "tp_t", || false);
                    if !touched {
                        tab = ((now / 2.0) as usize) % 3;
                    }
                    if ui.add(AnimatedTabs::new(&mut tab, &["Design", "Develop", "Deploy"])).changed() {
                        set_mem(ui, "tp_t", true);
                    }
                    set_mem(ui, "tp", tab);
                    ui.add_space(10.0);
                    TransitionPanel::new(tab).show(ui, |ui, i| {
                        let (title, body, icon) = [
                            ("Design", "Pick a theme preset or define your own tokens. Every component follows.", "🎨"),
                            ("Develop", "Builders, springs and per-widget state: idiomatic egui all the way down.", "🦀"),
                            ("Deploy", "One codebase for Windows, macOS, Linux and the web via WebAssembly.", "🚀"),
                        ][i];
                        egui::Frame::new()
                            .fill(if t.dark { shade(t.card, 0.04) } else { t.card })
                            .stroke(Stroke::new(1.0, t.border))
                            .corner_radius(CornerRadius::same(14))
                            .inner_margin(egui::Margin::same(18))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(icon).size(26.0));
                                ui.label(RichText::new(title).size(18.0).color(t.foreground));
                                ui.label(RichText::new(body).size(13.5).color(t.muted_foreground));
                            });
                    });
                });
            },
        },
    ]
}
