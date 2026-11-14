use super::*;
use crate::buttons::demos::{centered_row, stage};
use egui::{vec2, Color32, RichText, Ui};
use glowkit_core::color::{hex, palettes, with_alpha};
use glowkit_core::demo::{Category::CardsAndBorders as C, Demo};
use glowkit_core::{paint, Theme};

/// Title + description, the standard demo card content.
fn body(ui: &mut Ui, icon: &str, title: &str, text: &str) {
    let t = Theme::of(ui);
    if !icon.is_empty() {
        ui.label(RichText::new(icon).size(22.0).color(t.accent));
        ui.add_space(6.0);
    }
    ui.label(RichText::new(title).size(18.0).color(t.foreground));
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(13.0).color(t.muted_foreground));
}

fn white_body(ui: &mut Ui, title: &str, text: &str) {
    ui.label(RichText::new(title).size(20.0).color(Color32::WHITE));
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(13.0).color(with_alpha(Color32::WHITE, 0.8)));
}

const W: f32 = 270.0;

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "border-beam",
            name: "Border Beam",
            category: C,
            description: "A comet of light travels around the border, fading from one color to another along its tail.",
            covers: &["Magic UI: Border Beam", "Aceternity: Glowing Stars border", "Motion Primitives: Border Trail"],
            code: r##"BorderBeam::new().width(320.0).show(ui, |ui| {
    ui.heading("Login");
});
// or decorate any rect:
BorderBeam::new().duration(4.0).paint(ui, rect);"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 32.0, |ui| {
                        BorderBeam::new().size(vec2(W, 190.0)).show(ui, |ui| {
                            body(ui, "⚡", "Border Beam", "A comet of light circles the card, forever.")
                        });
                        BorderBeam::new()
                            .size(vec2(W, 190.0))
                            .duration(4.0)
                            .from(hex("#22D3EE"))
                            .to(hex("#A855F7"))
                            .show(ui, |ui| {
                                BorderBeam::new()
                                    .duration(4.0)
                                    .delay(2.0)
                                    .from(hex("#F472B6"))
                                    .to(hex("#FBBF24"))
                                    .paint(ui, ui.max_rect().expand(24.0));
                                body(ui, "✦", "Two beams", "Stagger several beams with delay().");
                            });
                        BorderBeam::new()
                            .size(vec2(W, 190.0))
                            .duration(8.0)
                            .length(0.45)
                            .thickness(2.5)
                            .reverse(true)
                            .show(ui, |ui| body(ui, "◎", "Long & reversed", "length(), thickness() and reverse()."));
                    });
                })
            },
        },
        Demo {
            id: "shine-border",
            name: "Shine Border",
            category: C,
            description: "A band of multi-colored light drifts diagonally around the card's border.",
            covers: &["Magic UI: Shine Border"],
            code: r##"ShineBorder::new().colors(&palettes::MAGIC).show(ui, |ui| { ui.heading("Shine"); });"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 32.0, |ui| {
                        ShineBorder::new()
                            .size(vec2(W, 190.0))
                            .show(ui, |ui| body(ui, "✧", "Shine Border", "Magic colors sweeping over the edge."));
                        ShineBorder::new()
                            .size(vec2(W, 190.0))
                            .colors(&palettes::OCEAN)
                            .duration(5.0)
                            .thickness(2.0)
                            .show(ui, |ui| body(ui, "≈", "Ocean", "Any palette, any speed."));
                        ShineBorder::new()
                            .size(vec2(W, 190.0))
                            .colors(&palettes::SUNSET)
                            .duration(9.0)
                            .radius(28.0)
                            .show(ui, |ui| body(ui, "☼", "Sunset", "Soft and slow, with round corners."));
                    });
                })
            },
        },
        Demo {
            id: "magic-card",
            name: "Magic Card",
            category: C,
            description:
                "A spotlight follows the pointer inside the card while the border lights up in a gradient around it.",
            covers: &["Magic UI: Magic Card"],
            code: r##"MagicCard::new().width(300.0).show(ui, |ui| {
    ui.heading("Magic Card");
});"##,
            show: |ui| {
                stage(ui, 220.0, |ui| {
                    centered_row(ui, "a", 24.0, |ui| {
                        MagicCard::new()
                            .size(vec2(W, 210.0))
                            .show(ui, |ui| body(ui, "✦", "Hover me", "The light and border follow your pointer."));
                        MagicCard::new()
                            .size(vec2(W, 210.0))
                            .ambient(true)
                            .show(ui, |ui| body(ui, "◎", "Ambient", "Wanders on its own when idle."));
                        MagicCard::new()
                            .size(vec2(W, 210.0))
                            .ambient(true)
                            .from(hex("#22D3EE"))
                            .to(hex("#34D399"))
                            .gradient_size(160.0)
                            .show(ui, |ui| body(ui, "◆", "Custom colors", "from(), to() and gradient_size()."));
                    });
                })
            },
        },
        Demo {
            id: "spotlight-card",
            name: "Spotlight Card",
            category: C,
            description:
                "A dark card lit by a soft spotlight that follows the pointer, optionally revealing a dot grid.",
            covers: &["React Bits: Spotlight Card", "Aceternity: Card Spotlight", "Motion Primitives: Spotlight"],
            code: r##"SpotlightCard::new().width(300.0).show(ui, |ui| { ui.heading("Spotlight"); });
SpotlightCard::new().dots(true).show(ui, |ui| { /* ... */ });"##,
            show: |ui| {
                stage(ui, 220.0, |ui| {
                    centered_row(ui, "a", 24.0, |ui| {
                        SpotlightCard::new()
                            .size(vec2(W, 210.0))
                            .show(ui, |ui| body(ui, "◉", "Spotlight", "Move the pointer over the card."));
                        SpotlightCard::new()
                            .size(vec2(W, 210.0))
                            .ambient(true)
                            .dots(true)
                            .show(ui, |ui| body(ui, "⁘", "With dots", "The light reveals a hidden dot grid."));
                        SpotlightCard::new()
                            .size(vec2(W, 210.0))
                            .ambient(true)
                            .color(with_alpha(hex("#22D3EE"), 0.25))
                            .show(ui, |ui| body(ui, "◍", "Tinted", "Any color for the light."));
                    });
                })
            },
        },
        Demo {
            id: "tilt-card",
            name: "Tilt Card",
            category: C,
            description:
                "The card tilts in 3D toward the pointer with real perspective, a moving glare and parallax content.",
            covers: &[
                "React Bits: Tilted Card",
                "Aceternity: 3D Card Effect",
                "Vanilla Tilt",
                "Motion Primitives: Tilt",
                "Aceternity UI: 3D Card",
                "Bencho: Tilt card",
            ],
            code: r##"TiltCard::new().size(vec2(260.0, 300.0)).show(ui, |ui| {
    ui.heading("Tilt");
});"##,
            show: |ui| {
                stage(ui, 260.0, |ui| {
                    centered_row(ui, "a", 48.0, |ui| {
                        TiltCard::new()
                            .size(vec2(240.0, 250.0))
                            .show(ui, |ui| body(ui, "◇", "Hover to tilt", "Perspective, glare and parallax content."));
                        TiltCard::new()
                            .size(vec2(240.0, 250.0))
                            .ambient(true)
                            .gradient(hex("#6366F1"), hex("#EC4899"))
                            .show(ui, |ui| white_body(ui, "Gradient", "Sways on its own when idle."));
                        TiltCard::new()
                            .size(vec2(240.0, 250.0))
                            .ambient(true)
                            .max_tilt(22.0)
                            .gradient(hex("#0EA5E9"), hex("#22C55E"))
                            .show(ui, |ui| white_body(ui, "Extra tilt", "max_tilt(22.0)"));
                    });
                })
            },
        },
        Demo {
            id: "glare-card",
            name: "Glare Card",
            category: C,
            description:
                "A diagonal streak of glare sweeps across the card on hover, with a soft sheen under the pointer.",
            covers: &["React Bits: Glare Hover", "Aceternity: Glare Card"],
            code: r##"GlareCard::new().size(vec2(260.0, 320.0)).show(ui, |ui| { ui.heading("Glare"); });"##,
            show: |ui| {
                stage(ui, 250.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        GlareCard::new()
                            .size(vec2(W, 240.0))
                            .show(ui, |ui| body(ui, "☄", "Hover me", "A streak of light sweeps across."));
                        GlareCard::new()
                            .size(vec2(W, 240.0))
                            .auto_every(2.2)
                            .background(hex("#18181B"))
                            .show(ui, |ui| white_body(ui, "Auto glare", "Sweeps every couple of seconds."));
                        GlareCard::new()
                            .size(vec2(W, 240.0))
                            .auto_every(2.8)
                            .angle(-20.0)
                            .color(hex("#C4B5FD"))
                            .opacity(0.6)
                            .background(hex("#4C1D95"))
                            .show(ui, |ui| white_body(ui, "Tinted", "Custom angle and color."));
                    });
                })
            },
        },
        Demo {
            id: "neon-gradient-card",
            name: "Neon Gradient Card",
            category: C,
            description:
                "A card wrapped in an animated two-tone gradient border that bleeds a big neon glow around it.",
            covers: &["Magic UI: Neon Gradient Card"],
            code: r##"NeonGradientCard::new().width(320.0).show(ui, |ui| { ui.heading("Neon"); });"##,
            show: |ui| {
                stage(ui, 220.0, |ui| {
                    centered_row(ui, "a", 56.0, |ui| {
                        NeonGradientCard::new().size(vec2(300.0, 200.0)).show(ui, |ui| {
                            body(ui, "", "Neon Gradient Card", "Pink and cyan light pours out of the border.")
                        });
                        NeonGradientCard::new()
                            .size(vec2(300.0, 200.0))
                            .first(hex("#F59E0B"))
                            .second(hex("#8B5CF6"))
                            .border_size(2.0)
                            .duration(6.0)
                            .show(ui, |ui| body(ui, "", "Amber & violet", "first(), second(), border_size()."));
                    });
                })
            },
        },
        Demo {
            id: "glowing-border-card",
            name: "Glowing Border Card",
            category: C,
            description:
                "The border lights up in a multi-color arc on the side facing the pointer, gliding around as it moves.",
            covers: &["Aceternity: Glowing Effect"],
            code: r##"GlowingBorderCard::new().width(300.0).show(ui, |ui| { ui.heading("Glow"); });"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        GlowingBorderCard::new()
                            .size(vec2(W, 190.0))
                            .show(ui, |ui| body(ui, "☉", "Move near me", "Lights up even from outside the card."));
                        GlowingBorderCard::new()
                            .size(vec2(W, 190.0))
                            .ambient(true)
                            .show(ui, |ui| body(ui, "◌", "Ambient", "Circles the border when idle."));
                        GlowingBorderCard::new()
                            .size(vec2(W, 190.0))
                            .ambient(true)
                            .colors(&palettes::NEON)
                            .spread(90.0)
                            .thickness(2.5)
                            .show(ui, |ui| body(ui, "✺", "Neon, wide", "colors() and spread()."));
                    });
                })
            },
        },
        Demo {
            id: "encrypted-card",
            name: "Encrypted Card",
            category: C,
            description: "Scrambled characters are revealed over a glowing gradient in a circle around the pointer.",
            covers: &["Aceternity: Evervault Card"],
            code: r##"EncryptedCard::new().badge("hover").size(vec2(280.0, 280.0)).show(ui, |_| {});"##,
            show: |ui| {
                stage(ui, 260.0, |ui| {
                    centered_row(ui, "a", 40.0, |ui| {
                        EncryptedCard::new().badge("hover").size(vec2(260.0, 250.0)).show(ui, |_| {});
                        EncryptedCard::new().badge("decrypt").ambient(true).size(vec2(260.0, 250.0)).show(ui, |_| {});
                        EncryptedCard::new()
                            .badge("ocean")
                            .ambient(true)
                            .colors(&palettes::OCEAN)
                            .reveal_radius(110.0)
                            .size(vec2(260.0, 250.0))
                            .show(ui, |_| {});
                    });
                })
            },
        },
        Demo {
            id: "pixel-card",
            name: "Pixel Card",
            category: C,
            description: "Shimmering pixels fill the card on hover, spreading out from the center.",
            covers: &["React Bits: Pixel Card"],
            code: r##"PixelCard::new().size(vec2(240.0, 300.0)).show(ui, |ui| { ui.heading("Pixels"); });"##,
            show: |ui| {
                stage(ui, 260.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        PixelCard::new()
                            .size(vec2(240.0, 250.0))
                            .show(ui, |ui| body(ui, "", "Hover me", "Pixels bloom from the center."));
                        PixelCard::new()
                            .size(vec2(240.0, 250.0))
                            .ambient(true)
                            .show(ui, |ui| body(ui, "", "Accent", "Defaults follow the theme."));
                        PixelCard::new()
                            .size(vec2(240.0, 250.0))
                            .ambient(true)
                            .colors(&[hex("#FECDD3"), hex("#FDA4AF"), hex("#E11D48")])
                            .gap(6.0)
                            .show(ui, |ui| body(ui, "", "Rose", "Custom colors and gap."));
                        PixelCard::new()
                            .size(vec2(240.0, 250.0))
                            .ambient(true)
                            .colors(&[hex("#FEF08A"), hex("#FDE047"), hex("#EAB308")])
                            .gap(9.0)
                            .speed(2.0)
                            .show(ui, |ui| body(ui, "", "Yellow", "Bigger, faster pixels."));
                    });
                })
            },
        },
        Demo {
            id: "glass-card",
            name: "Glass Card",
            category: C,
            description: "A frosted-glass card with a translucent fill, a light-catching rim and a soft shadow.",
            covers: &["Glassmorphism: Card", "Uiverse: Glass card", "Hover.dev: Glass card"],
            code: r##"GlassCard::new().width(300.0).show(ui, |ui| { ui.heading("Frosted"); });"##,
            show: |ui| {
                let r = ui.max_rect();
                let p = ui.painter();
                let time = glowkit_core::time(ui);
                let c = r.center_top() + vec2(0.0, 170.0);
                for (i, col) in [hex("#8B5CF6"), hex("#EC4899"), hex("#22D3EE"), hex("#F59E0B")].iter().enumerate() {
                    let a = time * 0.4 + i as f32 * 1.57;
                    let pos = c + vec2(a.cos() * 300.0, (a * 1.3).sin() * 50.0);
                    paint::glow(p, pos, 150.0, *col);
                }
                glowkit_core::animate(ui);
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        GlassCard::new()
                            .size(vec2(W, 190.0))
                            .show(ui, |ui| body(ui, "❄", "Frosted glass", "Translucent fill with a bright rim."));
                        GlassCard::new()
                            .size(vec2(W, 190.0))
                            .tint(hex("#C4B5FD"))
                            .show(ui, |ui| body(ui, "◈", "Tinted", "tint() mixes a color into the glass."));
                        GlassCard::new()
                            .size(vec2(W, 190.0))
                            .opacity(0.6)
                            .frost(false)
                            .show(ui, |ui| body(ui, "◇", "Clear", "Lower opacity, no grain."));
                    });
                });
            },
        },
        Demo {
            id: "wobble-card",
            name: "Wobble Card",
            category: C,
            description:
                "A bold card wobbles toward the pointer on a springy band while its content drifts the other way.",
            covers: &["Aceternity: Wobble Card"],
            code: r##"WobbleCard::new().width(420.0).show(ui, |ui| { ui.heading("Wobble"); });"##,
            show: |ui| {
                stage(ui, 220.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        WobbleCard::new().size(vec2(420.0, 210.0)).padding(30.0).show(ui, |ui| white_body(ui, "Gippity AI powers the entire universe", "With over 100,000 monthly active bot users, Gippity AI is the most popular AI platform for developers."));
                        WobbleCard::new()
                            .size(vec2(300.0, 210.0))
                            .padding(30.0)
                            .ambient(true)
                            .colors(hex("#DB2777"), hex("#7E22CE"))
                            .show(ui, |ui| white_body(ui, "No shirt, no shoes", "Wobbles by itself when idle."));
                    });
                })
            },
        },
        Demo {
            id: "card-hover-effect",
            name: "Card Hover Effect",
            category: C,
            description: "A soft highlight slab slides between cards in a grid, following the pointer.",
            covers: &["Aceternity: Card Hover Effect"],
            code: r##"let items = [("Stripe", "Payments"), ("Netflix", "Streaming"), ("Google", "Search")];
CardHoverEffect::new(items.len()).columns(3).show(ui, |ui, i| {
    ui.heading(items[i].0);
    ui.label(items[i].1);
});"##,
            show: |ui| {
                ui.add_space(12.0);
                let items = [
                    ("Stripe", "A technology company that builds economic infrastructure for the internet."),
                    ("Netflix", "A streaming service that offers award-winning TV shows and movies."),
                    ("Google", "A multinational technology company focusing on search and ads."),
                    ("Meta", "A technology company that focuses on building products for connection."),
                    ("Amazon", "A multinational company focusing on e-commerce and cloud computing."),
                    ("Microsoft", "A technology company that develops software and devices."),
                ];
                CardHoverEffect::new(items.len()).columns(3).min_height(130.0).ambient(true).show(ui, |ui, i| {
                    let t = Theme::of(ui);
                    ui.label(RichText::new(items[i].0).size(17.0).color(t.foreground));
                    ui.add_space(6.0);
                    ui.label(RichText::new(items[i].1).size(13.0).color(t.muted_foreground));
                });
            },
        },
        Demo {
            id: "hover-lift-card",
            name: "Hover Lift Card",
            category: C,
            description: "The card rises toward you on hover: a deeper shadow, an accent glow and a warmer border.",
            covers: &["Hover.dev: Lift card", "Uiverse: Hover card", "shadcn: Card (hover)"],
            code: r##"HoverLiftCard::new().width(260.0).show(ui, |ui| { ui.heading("Lift"); });"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        HoverLiftCard::new()
                            .size(vec2(W, 180.0))
                            .show(ui, |ui| body(ui, "▲", "Hover me", "Lifts with a spring."));
                        let on = crate::buttons::demos::blink(ui, 1.6, 0.0);
                        HoverLiftCard::new()
                            .size(vec2(W, 180.0))
                            .force_hover(on)
                            .show(ui, |ui| body(ui, "◆", "Lifted", "force_hover() for showcases."));
                        HoverLiftCard::new()
                            .size(vec2(W, 180.0))
                            .force_hover(true)
                            .accent(hex("#22D3EE"))
                            .lift(12.0)
                            .show(ui, |ui| body(ui, "●", "Cyan, higher", "accent() and lift()."));
                    });
                })
            },
        },
        Demo {
            id: "focus-cards",
            name: "Focus Cards",
            category: C,
            description: "The hovered card stays in focus while the others shrink back, dim and haze over.",
            covers: &["Aceternity: Focus Cards"],
            code: r##"FocusCards::new(3).columns(3).show(ui, |ui, i, focused| {
    ui.heading(format!("Card {i}"));
});"##,
            show: |ui| {
                ui.add_space(30.0);
                let titles = ["Forest adventure", "Valley of life", "Sala behta hi jayega"];
                let grads = [
                    [hex("#065F46"), hex("#34D399")],
                    [hex("#1E3A8A"), hex("#22D3EE")],
                    [hex("#7C2D12"), hex("#F59E0B")],
                ];
                FocusCards::new(3).columns(3).min_height(260.0).ambient(true).show(ui, |ui, i, _focused| {
                    let t = Theme::of(ui);
                    let r = ui.max_rect();
                    let p = ui.painter();
                    paint::gradient_rect(p, r, t.radius * 1.4, 60.0, &grads[i]);
                    paint::glow(
                        p,
                        r.center() + vec2(r.width() * 0.22, -r.height() * 0.2),
                        r.width() * 0.45,
                        with_alpha(Color32::WHITE, 0.22),
                    );
                    let shade = egui::Rect::from_min_max(egui::pos2(r.min.x, r.max.y - 80.0), r.max);
                    paint::fill_convex_fn(
                        p,
                        &paint::rounded_rect_points(shade, t.radius * 1.4, 8),
                        4,
                        paint::linear_fn(
                            shade,
                            90.0,
                            &[with_alpha(Color32::BLACK, 0.0), with_alpha(Color32::BLACK, 0.6)],
                        ),
                    );
                    ui.allocate_space(vec2(r.width(), r.height() - 44.0));
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(RichText::new(titles[i]).size(18.0).color(Color32::WHITE));
                    });
                });
            },
        },
        Demo {
            id: "electric-border",
            name: "Electric Border",
            category: C,
            description: "A jittery, crackling border of electricity with layered glow.",
            covers: &["React Bits: Electric Border"],
            code: r##"ElectricBorder::new().width(300.0).show(ui, |ui| { ui.heading("Zap"); });"##,
            show: |ui| {
                stage(ui, 220.0, |ui| {
                    centered_row(ui, "a", 48.0, |ui| {
                        ElectricBorder::new()
                            .size(vec2(W, 200.0))
                            .show(ui, |ui| body(ui, "ϟ", "Electric Border", "Noisy, flickering, glowing."));
                        ElectricBorder::new()
                            .size(vec2(W, 200.0))
                            .color(hex("#F472B6"))
                            .chaos(1.6)
                            .speed(1.4)
                            .show(ui, |ui| body(ui, "ϟ", "More chaos", "chaos() and speed()."));
                        ElectricBorder::new()
                            .size(vec2(W, 200.0))
                            .color(hex("#A3E635"))
                            .chaos(0.6)
                            .radius(40.0)
                            .show(ui, |ui| body(ui, "ϟ", "Calm", "Gentle crackle, round corners."));
                    });
                })
            },
        },
        Demo {
            id: "animated-gradient-border",
            name: "Animated Gradient Border",
            category: C,
            description: "A conic gradient keeps rotating around the border, optionally with a matching glow.",
            covers: &[
                "Uiverse: Animated border card",
                "Magic UI: Animated gradient border",
                "CSS: conic border",
                "Aceternity UI: Background Gradient",
            ],
            code: r##"AnimatedGradientBorder::new().glow(true).show(ui, |ui| { ui.heading("Gradient"); });"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 36.0, |ui| {
                        AnimatedGradientBorder::new()
                            .size(vec2(W, 190.0))
                            .show(ui, |ui| body(ui, "↻", "Rotating", "Theme gradient by default."));
                        AnimatedGradientBorder::new()
                            .size(vec2(W, 190.0))
                            .glow(true)
                            .colors(&palettes::NEON)
                            .show(ui, |ui| body(ui, "✹", "With glow", "glow(true) adds a halo."));
                        AnimatedGradientBorder::new()
                            .size(vec2(W, 190.0))
                            .glow(true)
                            .thickness(3.0)
                            .duration(2.5)
                            .colors(&palettes::RAINBOW)
                            .show(ui, |ui| body(ui, "◉", "Rainbow", "Thicker and faster."));
                    });
                })
            },
        },
        Demo {
            id: "dashed-marching-border",
            name: "Marching Border",
            category: C,
            description: "Dashes march around the border like a selection marquee or a drop zone.",
            covers: &["CSS: Marching ants", "Uiverse: Dashed border animation"],
            code: r##"DashedMarchingBorder::new().show(ui, |ui| { ui.label("Drop files here"); });"##,
            show: |ui| {
                stage(ui, 200.0, |ui| {
                    centered_row(ui, "a", 28.0, |ui| {
                        DashedMarchingBorder::new().size(vec2(W, 180.0)).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                body(ui, "⇪", "Drop files here", "or click to browse");
                            });
                        });
                        DashedMarchingBorder::new()
                            .size(vec2(W, 180.0))
                            .gradient(true)
                            .thickness(2.0)
                            .dash(14.0)
                            .gap(8.0)
                            .show(ui, |ui| body(ui, "≋", "Gradient dashes", "gradient(true)"));
                        DashedMarchingBorder::new()
                            .size(vec2(W, 180.0))
                            .color(hex("#22D3EE"))
                            .dash(4.0)
                            .gap(6.0)
                            .speed(-20.0)
                            .radius(4.0)
                            .show(ui, |ui| body(ui, "⋯", "Dotted, reverse", "Short dashes, negative speed."));
                    });
                })
            },
        },
    ]
}
