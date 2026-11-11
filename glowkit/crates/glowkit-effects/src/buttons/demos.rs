use super::*;
use egui::Ui;
use glowkit_core::color::{hex, palettes};
use glowkit_core::demo::{Category::Buttons as C, Demo};

/// Lay out `add` as a horizontally centered row (width measured on the previous frame).
pub(crate) fn centered_row(ui: &mut Ui, salt: &str, spacing: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("glowkit_centered_row", salt));
    let w: f32 = glowkit_core::state(ui, id, || 0.0);
    let avail = ui.available_width();
    let r = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.add_space(((avail - w) / 2.0).max(0.0));
        let start = ui.cursor().min.x;
        add(ui);
        ui.min_rect().max.x - start
    });
    let measured = r.inner;
    if (measured - w).abs() > 0.5 {
        glowkit_core::set_state(ui, id, measured);
        ui.ctx().request_repaint();
    }
}

/// Vertical stack of centered rows, vertically centered in a ~360pt preview.
pub(crate) fn stage(ui: &mut Ui, content_height: f32, add: impl FnOnce(&mut Ui)) {
    ui.add_space(((340.0 - content_height) / 2.0).max(8.0));
    add(ui);
}

/// Cycles between `false` and `true` every `period` seconds (for "hovered" showcase variants).
pub(crate) fn blink(ui: &Ui, period: f32, offset: f32) -> bool {
    ((glowkit_core::time(ui) + offset) / period).floor() as i64 % 2 == 1
}

fn caption(ui: &mut Ui, text: &str) {
    let t = glowkit_core::Theme::of(ui);
    ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).size(12.0).color(t.muted_foreground)));
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "shimmer-button",
            name: "Shimmer Button",
            category: C,
            description: "A dark pill with a spark of light racing around its border and an inner glow that rises on hover.",
            covers: &["Magic UI: Shimmer Button", "Uiverse: Shimmer button"],
            code: r##"if ui.add(ShimmerButton::new("Shimmer Button")).clicked() { /* ... */ }
ui.add(ShimmerButton::new("Get started").background(hex("#4C1D95")).duration(2.0));"##,
            show: |ui| stage(ui, 120.0, |ui| {
                centered_row(ui, "a", 28.0, |ui| {
                    ui.add(ShimmerButton::new("Shimmer Button"));
                    ui.add(ShimmerButton::new("Get started  →").background(hex("#4C1D95")).duration(2.2).size(16.0));
                    ui.add(ShimmerButton::new("Square corners").radius(8.0).shimmer_color(hex("#67E8F9")).duration(3.6));
                });
                ui.add_space(36.0);
                centered_row(ui, "b", 28.0, |ui| {
                    ui.add(ShimmerButton::new("Large shimmer").size(20.0).duration(4.0));
                });
            }),
        },
        Demo {
            id: "shiny-button",
            name: "Shiny Button",
            category: C,
            description: "A subtle surface button whose label and border catch a band of light sweeping across.",
            covers: &["Magic UI: Shiny Button", "Motion Primitives: Glow Effect (shine)"],
            code: r##"ui.add(ShinyButton::new("Shiny Button"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 28.0, |ui| {
                    ui.add(ShinyButton::new("Shiny Button").size(16.0));
                    ui.add(ShinyButton::new("✦ New: glowkit 1.0").radius(99.0).period(3.2));
                    ui.add(ShinyButton::new("Accent shine").shine_color(hex("#A78BFA")).period(2.0));
                });
            }),
        },
        Demo {
            id: "rainbow-button",
            name: "Rainbow Button",
            category: C,
            description: "A high-contrast button with a flowing rainbow border and a blurred rainbow glow pooled beneath it.",
            covers: &["Magic UI: Rainbow Button"],
            code: r##"ui.add(RainbowButton::new("Get Unlimited Access"));
ui.add(RainbowButton::new("Aurora").colors(&palettes::AURORA).radius(99.0));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 36.0, |ui| {
                    ui.add(RainbowButton::new("Get Unlimited Access").size(16.0));
                    ui.add(RainbowButton::new("Aurora pill").colors(&palettes::NEON).radius(99.0));
                    ui.add(RainbowButton::new("Sunset").colors(&palettes::SUNSET).speed(1.2));
                });
            }),
        },
        Demo {
            id: "pulsating-button",
            name: "Pulsating Button",
            category: C,
            description: "A solid call-to-action that sends soft rings of color pulsing outward like a heartbeat.",
            covers: &["Magic UI: Pulsating Button", "Uiverse: Pulse button"],
            code: r##"ui.add(PulsatingButton::new("Join the waitlist"));
ui.add(PulsatingButton::new("Live").background(hex("#EF4444")).duration(1.1));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 44.0, |ui| {
                    ui.add(PulsatingButton::new("Join the waitlist").size(16.0));
                    ui.add(PulsatingButton::new("● Live now").background(hex("#EF4444")).duration(1.1).radius(99.0));
                    ui.add(PulsatingButton::new("Claim offer").background(hex("#10B981")).spread(18.0).duration(2.2));
                });
            }),
        },
        Demo {
            id: "ripple-button",
            name: "Ripple Button",
            category: C,
            description: "Material-style ink ripples spread from wherever you click, clipped to the button's rounded shape.",
            covers: &["Magic UI: Ripple Button", "Material: Ripple", "Uiverse: Ripple button"],
            code: r##"ui.add(RippleButton::new("Click me"));
ui.add(RippleButton::new("Filled").background(hex("#6366F1")));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 28.0, |ui| {
                    ui.add(RippleButton::new("Click me").size(16.0));
                    ui.add(RippleButton::new("Auto ripple").auto_ripple(1.1));
                    ui.add(RippleButton::new("Filled").background(hex("#6366F1")).auto_ripple(1.6).radius(99.0));
                });
                ui.add_space(14.0);
                caption(ui, "Click anywhere on a button");
            }),
        },
        Demo {
            id: "interactive-hover-button",
            name: "Interactive Hover Button",
            category: C,
            description: "A dot beside the label swells to fill the pill on hover while a new label and arrow slide in.",
            covers: &["Magic UI: Interactive Hover Button"],
            code: r##"ui.add(InteractiveHoverButton::new("Book a demo"));
ui.add(InteractiveHoverButton::new("Hover me").hover_label("Let's go"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 28.0, |ui| {
                    ui.add(InteractiveHoverButton::new("Book a demo").size(16.0));
                    let on = blink(ui, 1.6, 0.0);
                    ui.add(InteractiveHoverButton::new("Hover me").hover_label("Let's go").color(hex("#8B5CF6")).force_hover(on));
                    ui.add(InteractiveHoverButton::new("Always hovered").force_hover(true));
                });
            }),
        },
        Demo {
            id: "moving-border-button",
            name: "Moving Border",
            category: C,
            description: "A glowing blob of light endlessly orbits the button's thin border.",
            covers: &["Aceternity: Moving Border", "Uiverse: Animated border button"],
            code: r##"ui.add(MovingBorderButton::new("Borders are cool"));
ui.add(MovingBorderButton::new("Pink").color(hex("#EC4899")).duration(2.0));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(MovingBorderButton::new("Borders are cool").size(16.0));
                    ui.add(MovingBorderButton::new("Faster & pink").color(hex("#EC4899")).duration(2.0));
                    ui.add(MovingBorderButton::new("Round").color(hex("#A3E635")).radius(99.0).duration(4.0));
                });
            }),
        },
        Demo {
            id: "hover-border-gradient",
            name: "Hover Border Gradient",
            category: C,
            description: "A highlight circles the pill's border; on hover the light spreads over the whole border in color.",
            covers: &["Aceternity: Hover Border Gradient"],
            code: r##"ui.add(HoverBorderGradient::new("Aceternity UI"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(HoverBorderGradient::new("▲  Aceternity UI").size(16.0));
                    ui.add(HoverBorderGradient::new("Counter-clockwise").clockwise(false).duration(2.5));
                    let on = blink(ui, 1.8, 0.4);
                    ui.add(HoverBorderGradient::new("Hover state").force_hover(on).hover_color(hex("#EC4899")));
                });
            }),
        },
        Demo {
            id: "star-border-button",
            name: "Star Border",
            category: C,
            description: "Two stars of light glide along the top and bottom edges in opposite directions.",
            covers: &["React Bits: Star Border"],
            code: r##"ui.add(StarBorderButton::new("Star Border"));
ui.add(StarBorderButton::new("Cyan").color(hex("#22D3EE")).duration(3.0));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(StarBorderButton::new("Star Border").size(16.0));
                    ui.add(StarBorderButton::new("Cyan stars").color(hex("#22D3EE")).duration(3.0));
                    ui.add(StarBorderButton::new("Magenta").color(hex("#F472B6")).duration(6.0).radius(99.0));
                });
            }),
        },
        Demo {
            id: "magnetic-button",
            name: "Magnetic Button",
            category: C,
            description: "The button is pulled toward the pointer as it approaches and springs back when it leaves.",
            covers: &["Motion Primitives: Magnetic", "Hover.dev: Magnetic button", "Uiverse: Magnetic button"],
            code: r##"ui.add(MagneticButton::new("Hover me").strength(0.4));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 80.0, |ui| {
                    ui.add(MagneticButton::new("Hover near me").size(16.0));
                    ui.add(MagneticButton::new("Ambient drift").ambient(true).background(hex("#8B5CF6")));
                    ui.add(MagneticButton::new("Strong pull").strength(0.6).range(90.0));
                });
                ui.add_space(26.0);
                caption(ui, "Move the pointer close to a button");
            }),
        },
        Demo {
            id: "glow-button",
            name: "Glow Button",
            category: C,
            description: "A glossy gradient pill floating on a breathing pool of its own colored light; it lifts on hover.",
            covers: &["Uiverse: Glow button", "Hover.dev: Glow button", "Magic UI: Neon glow"],
            code: r##"ui.add(GlowButton::new("Start building"));
ui.add(GlowButton::new("Ocean").colors(&palettes::OCEAN));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 40.0, |ui| {
                    ui.add(GlowButton::new("Start building").size(16.0));
                    ui.add(GlowButton::new("Ocean").colors(&palettes::OCEAN[..2]));
                    ui.add(GlowButton::new("Sunset").colors(&[hex("#F97316"), hex("#EC4899")]).intensity(1.3));
                });
            }),
        },
        Demo {
            id: "neon-button",
            name: "Neon Button",
            category: C,
            description: "An outlined neon tube with glowing text that occasionally flickers, and floods with light on hover.",
            covers: &["Uiverse: Neon button", "CodePen: Neon button"],
            code: r##"ui.add(NeonButton::new("NEON").color(hex("#22D3EE")));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 40.0, |ui| {
                    ui.add(NeonButton::new("NEON LIGHTS").color(hex("#22D3EE")).size(16.0));
                    ui.add(NeonButton::new("OPEN 24/7").color(hex("#F472B6")));
                    let on = blink(ui, 1.8, 0.0);
                    ui.add(NeonButton::new("HOVER").color(hex("#A3E635")).force_hover(on));
                });
            }),
        },
        Demo {
            id: "fill-hover-button",
            name: "Liquid Fill Button",
            category: C,
            description: "Liquid with a wavy surface rises inside the outline on hover, flipping the label color as it passes.",
            covers: &["Uiverse: Liquid fill button", "Hover.dev: Fill button", "CodePen: Liquid button"],
            code: r##"ui.add(FillHoverButton::new("Dive in"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(FillHoverButton::new("Hover to fill").size(16.0));
                    let on = blink(ui, 2.2, 0.0);
                    ui.add(FillHoverButton::new("Dive in").color(hex("#0EA5E9")).force_hover(on).radius(99.0));
                    ui.add(FillHoverButton::new("Filled").color(hex("#F59E0B")).force_hover(true));
                });
            }),
        },
        Demo {
            id: "stateful-button",
            name: "Stateful Button",
            category: C,
            description: "Idle, then a spinner while loading, then a drawn check mark on success, resizing smoothly between states.",
            covers: &["Aceternity: Stateful Button", "Uiverse: Loading button"],
            code: r##"// keep `state: StatefulButtonState` in your app
if ui.add(StatefulButton::new("Send message", &mut state).success_label("Sent!")).clicked() {
    // start work... later: state = StatefulButtonState::Success;
}"##,
            show: |ui| stage(ui, 110.0, |ui| {
                let id = ui.scope_id().with("stateful");
                let mut a: StatefulButtonState = glowkit_core::state(ui, id, StatefulButtonState::default);
                let mut b: StatefulButtonState = glowkit_core::state(ui, id.with("b"), StatefulButtonState::default);
                if b == StatefulButtonState::Idle && blink(ui, 1.5, 0.0) {
                    b = StatefulButtonState::Loading;
                }
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(StatefulButton::new("Send message", &mut a).loading_label("Sending").success_label("Sent!").simulate(1.6).size(16.0));
                    ui.add(StatefulButton::new("Deploy", &mut b).loading_label("Deploying").success_label("Deployed").simulate(1.4).reset_after(Some(1.5)).color(hex("#0EA5E9")));
                });
                glowkit_core::set_state(ui, id, a);
                glowkit_core::set_state(ui, id.with("b"), b);
                ui.add_space(18.0);
                caption(ui, "Click to start (the work is simulated)");
            }),
        },
        Demo {
            id: "subscribe-button",
            name: "Animated Subscribe Button",
            category: C,
            description: "A toggle that slides between two labels, gaining a check mark and a new color when active.",
            covers: &["Magic UI: Animated Subscribe Button"],
            code: r##"// keep `subscribed: bool` in your app
ui.add(SubscribeButton::new(&mut subscribed));
ui.add(SubscribeButton::new(&mut following).labels("Follow", "Following"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                let id = ui.scope_id().with("subscribe");
                let mut a: bool = glowkit_core::state(ui, id, || false);
                let mut b = blink(ui, 1.8, 0.0);
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(SubscribeButton::new(&mut a).size(16.0));
                    ui.add(SubscribeButton::new(&mut b).labels("Follow", "Following").color(hex("#8B5CF6")));
                    glowkit_core::animate(ui);
                });
                glowkit_core::set_state(ui, id, a);
            }),
        },
        Demo {
            id: "copy-button",
            name: "Copy Button",
            category: C,
            description: "Copies text to the clipboard; the copy icon morphs into a check mark, then back.",
            covers: &["Magic UI: Animated copy button", "Aceternity: Copy button", "shadcn: Copy button"],
            code: r##"ui.add(CopyButton::new("cargo add glowkit"));
ui.add(CopyButton::new("npm i glowkit").label("Copy"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 24.0, |ui| {
                    ui.add(CopyButton::new("cargo add glowkit"));
                    ui.add(CopyButton::new("cargo add glowkit").label("Copy"));
                    ui.add(CopyButton::new("cargo add glowkit").label("Copy link").demo_loop(true));
                    ui.add(CopyButton::new("cargo add glowkit").demo_loop(true).size(18.0));
                });
            }),
        },
        Demo {
            id: "particle-burst-button",
            name: "Particle Burst Button",
            category: C,
            description: "Click to burst confetti, sparkles and dots out of the button (hold to keep spraying).",
            covers: &["Magic UI: Cool Mode", "Uiverse: Confetti button"],
            code: r##"ui.add(ParticleBurstButton::new("Cool mode"));"##,
            show: |ui| stage(ui, 70.0, |ui| {
                ui.add_space(70.0);
                centered_row(ui, "a", 60.0, |ui| {
                    ui.add(ParticleBurstButton::new("Click for confetti").size(16.0));
                    ui.add(ParticleBurstButton::new("Auto burst").auto_burst(1.4).colors(&palettes::RAINBOW).background(hex("#8B5CF6")));
                });
            }),
        },
        Demo {
            id: "push-button-3d",
            name: "3D Push Button",
            category: C,
            description: "A chunky key you physically push: it pops up on hover and sinks on press with a bouncy spring.",
            covers: &["Josh Comeau: 3D button", "Neobrutalism: Button", "RetroUI: Button", "Uiverse: 3D button"],
            code: r##"ui.add(PushButton3D::new("Push me"));
ui.add(PushButton3D::new("Brutal").style(PushStyle::Brutal));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 36.0, |ui| {
                    ui.add(PushButton3D::new("Push me").size(16.0));
                    ui.add(PushButton3D::new("Delete").color(hex("#EF4444")).depth(8.0));
                    ui.add(PushButton3D::new("Neobrutal").style(PushStyle::Brutal));
                    ui.add(PushButton3D::new("Mint").style(PushStyle::Brutal).color(hex("#86EFAC")));
                });
            }),
        },
        Demo {
            id: "pixel-button",
            name: "Pixel Button",
            category: C,
            description: "A retro 8-bit button with stair-stepped corners, a chunky bevel and a pixel drop shadow.",
            covers: &["8bitcn: Button", "NES.css: Button", "RetroUI: Pixel button"],
            code: r##"ui.add(PixelButton::new("PRESS START").blink(true));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 28.0, |ui| {
                    ui.add(PixelButton::new("PRESS START").blink(true).size(16.0));
                    ui.add(PixelButton::new("CONTINUE").color(hex("#22C55E")));
                    ui.add(PixelButton::new("GAME OVER").color(hex("#EF4444")).pixel(4.0));
                });
            }),
        },
        Demo {
            id: "gradient-outline-button",
            name: "Gradient Outline Button",
            category: C,
            description: "A transparent button outlined by a rotating gradient that floods the button on hover.",
            covers: &["Uiverse: Gradient border button", "Hover.dev: Gradient button"],
            code: r##"ui.add(GradientOutlineButton::new("Explore"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(GradientOutlineButton::new("Explore").size(16.0));
                    ui.add(GradientOutlineButton::new("Ocean").colors(&palettes::OCEAN).radius(10.0));
                    let on = blink(ui, 1.8, 0.0);
                    ui.add(GradientOutlineButton::new("Hover state").colors(&palettes::SUNSET).force_hover(on));
                });
            }),
        },
        Demo {
            id: "arrow-slide-button",
            name: "Arrow Slide Button",
            category: C,
            description: "On hover the label nudges aside and the arrow chip shoots its arrow out while a new one slides in.",
            covers: &["Hover.dev: Arrow button", "Uiverse: Arrow button"],
            code: r##"ui.add(ArrowSlideButton::new("Continue"));"##,
            show: |ui| stage(ui, 110.0, |ui| {
                centered_row(ui, "a", 32.0, |ui| {
                    ui.add(ArrowSlideButton::new("Continue").size(16.0));
                    ui.add(ArrowSlideButton::new("Get started").chip_color(hex("#F59E0B")));
                    let on = blink(ui, 1.4, 0.0);
                    ui.add(ArrowSlideButton::new("Next step").force_hover(on));
                });
            }),
        },
    ]
}
