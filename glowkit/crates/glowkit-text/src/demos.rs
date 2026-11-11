use crate::*;
use egui::{vec2, Align, Color32, Layout, RichText, Sense, Ui};
use glowkit_core::color::{hex, palettes};
use glowkit_core::demo::{Category::TextAnimations as C, Demo};
use glowkit_core::{Theme, Trigger};

/// Center a demo vertically/horizontally inside the preview frame.
pub(crate) fn centered(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    centered_at(ui, 110.0, add);
}

/// Like [`centered`] with a custom top offset.
pub(crate) fn centered_at(ui: &mut Ui, top: f32, add: impl FnOnce(&mut Ui)) {
    ui.vertical_centered(|ui| {
        ui.add_space(top);
        add(ui);
    });
}

/// A horizontally centered row (uses last frame's width, so it can hold animated widgets).
pub(crate) fn row(ui: &mut Ui, salt: &str, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("glowkit_row", salt));
    let w: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    ui.horizontal(|ui| {
        let full = ui.available_width();
        ui.add_space(((full - w) / 2.0).max(0.0));
        let left = ui.cursor().min.x;
        add(ui);
        let used = ui.min_rect().max.x - left;
        if (used - w).abs() > 0.5 {
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(id, used));
    });
}

/// Static bold text matching the animated widgets' weight.
pub(crate) fn text(ui: &mut Ui, s: &str, size: f32, color: Option<Color32>) {
    let t = Theme::of(ui);
    let font = crate::util::font(size, false);
    let g = crate::util::layout(ui.painter(), s, &font);
    let bold = crate::util::bold_px(size);
    let (rect, _) = ui.allocate_exact_size(g.size() + vec2(bold, 0.0), Sense::hover());
    crate::util::paint_galley(ui.painter(), rect.min, &g, color.unwrap_or(t.foreground), bold);
}

/// A small muted caption.
pub(crate) fn hint(ui: &mut Ui, s: &str) {
    let t = Theme::of(ui);
    ui.label(RichText::new(s).size(12.5).color(t.muted_foreground));
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "shiny-text",
            name: "Shiny Text",
            category: C,
            description: "A band of light sweeps across the text, like light catching metal.",
            covers: &["React Bits: Shiny Text", "Magic UI: Animated Shiny Text", "Motion Primitives: Text Shimmer"],
            code: r##"ui.add(ShinyText::new("Just some shiny text!").size(40.0).speed(2.5));"##,
            show: |ui| centered(ui, |ui| {
                ui.add(ShinyText::new("Just some shiny text!").size(40.0).speed(2.5));
                ui.add_space(18.0);
                ui.add(ShinyText::new("✨ Introducing glowkit  →").size(16.0).speed(2.0));
            }),
        },
        Demo {
            id: "gradient-text",
            name: "Gradient Text",
            category: C,
            description: "Text filled with a flowing, seamlessly looping gradient. Optional animated border.",
            covers: &["React Bits: Gradient Text", "Magic UI: Animated Gradient Text", "Magic UI: Aurora Text", "Aceternity: Gradient text"],
            code: r##"ui.add(GradientText::new("Add a splash of color!")
    .colors(&[hex("#40ffaa"), hex("#4079ff"), hex("#ff40d9")])
    .size(42.0));
ui.add(GradientText::new("🎉 Now with borders").border(true).size(16.0));"##,
            show: |ui| centered(ui, |ui| {
                ui.add(GradientText::new("Add a splash of color!").colors(&palettes::NEON[..3]).size(42.0).bold(true));
                ui.add_space(18.0);
                ui.add(GradientText::new("🎉 Now with animated borders").colors(&palettes::MAGIC).border(true).size(16.0).speed(3.0));
                ui.add_space(18.0);
                ui.add(GradientText::new("Sunset").colors(&palettes::SUNSET).size(28.0).angle(30.0));
            }),
        },
        Demo {
            id: "blur-text",
            name: "Blur Text",
            category: C,
            description: "Words or letters come into focus from a blur, one after another. Click to replay.",
            covers: &["React Bits: Blur Text", "Magic UI: Blur Fade", "Motion Primitives: Text Effect (blur)"],
            code: r##"ui.add(BlurText::new("Isn't this so cool?!")
    .split_by(SplitBy::Letters)
    .direction(Direction::Top)
    .trigger(Trigger::OnClick)
    .size(44.0));"##,
            show: |ui| centered(ui, |ui| {
                let t = Theme::of(ui);
                ui.add(BlurText::new("Isn't this so cool?!").split_by(SplitBy::Letters).delay(0.05).size(44.0).trigger(Trigger::Loop { pause_ms: 1800 }));
                ui.add_space(20.0);
                ui.add(BlurText::new("Words drift into focus one at a time").direction(Direction::Bottom).color(t.muted_foreground).size(20.0).trigger(Trigger::Loop { pause_ms: 2000 }));
            }),
        },
        Demo {
            id: "split-text",
            name: "Split Text",
            category: C,
            description: "Letters (or words) rise into place one after another with a springy overshoot. Optional line mask for a pull-up reveal.",
            covers: &["React Bits: Split Text", "Magic UI: Letter Pull Up", "Magic UI: Word Pull Up", "Magic UI: Text Animate", "Motion Primitives: Text Effect"],
            code: r##"ui.add(SplitText::new("Hello, you!").size(56.0).trigger(Trigger::OnClick));
ui.add(SplitText::new("Words pull up from behind a mask")
    .split_by(SplitBy::Words)
    .mask(true)
    .size(24.0));"##,
            show: |ui| centered_at(ui, 100.0, |ui| {
                let t = Theme::of(ui);
                ui.add(SplitText::new("Hello, you!").size(64.0).trigger(Trigger::Loop { pause_ms: 1600 }));
                ui.add_space(22.0);
                ui.add(SplitText::new("Words pull up from behind a mask").split_by(SplitBy::Words).mask(true).delay(0.09).easing(glowkit_core::Easing::OutCubic).size(24.0).color(t.muted_foreground).trigger(Trigger::Loop { pause_ms: 2000 }));
            }),
        },
        Demo {
            id: "decrypted-text",
            name: "Decrypted Text",
            category: C,
            description: "Characters scramble through random symbols, then lock into place one by one. Hover the lower lines to decode them again.",
            covers: &["React Bits: Decrypted Text", "Magic UI: Hyper Text", "Motion Primitives: Text Scramble"],
            code: r##"ui.add(DecryptedText::new("ACCESS GRANTED").monospace(true).size(40.0));
ui.add(DecryptedText::new("Hover me to decode")
    .trigger(Trigger::OnHover)
    .reveal_from(StaggerFrom::Center));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                ui.add(DecryptedText::new("ACCESS GRANTED").monospace(true).bold(true).size(44.0).trigger(Trigger::Loop { pause_ms: 2200 }));
                ui.add_space(24.0);
                ui.add(DecryptedText::new("Hover me to decode from the center").size(22.0).reveal_from(StaggerFrom::Center).trigger(Trigger::OnHover));
                ui.add_space(12.0);
                ui.add(DecryptedText::new("HYPER TEXT · RANDOM ORDER").monospace(true).size(18.0).reveal_from(StaggerFrom::Random).step(0.04).trigger(Trigger::OnHover));
            }),
        },
        Demo {
            id: "typewriter-text",
            name: "Typewriter",
            category: C,
            description: "Types one or more strings, pauses, deletes and types the next, with a smoothly blinking cursor.",
            covers: &["React Bits: Text Type", "Magic UI: Typing Animation", "Aceternity: Typewriter Effect", "Motion Primitives: Text Effect (typewriter)"],
            code: r##"ui.add(TypewriterText::new(["beautiful apps.", "fast tools.", "games."])
    .prefix("Build ")
    .size(40.0));
ui.add(TypewriterText::new(["cargo add glowkit"])
    .monospace(true)
    .cursor(CursorStyle::Block));"##,
            show: |ui| centered_at(ui, 105.0, |ui| {
                let t = Theme::of(ui);
                ui.add(TypewriterText::new(["beautiful apps.", "blazing-fast tools.", "delightful games."]).prefix("Build ").colors(&[t.gradient[0], t.gradient[1], t.chart[1]]).size(40.0));
                ui.add_space(26.0);
                ui.add(TypewriterText::new(["$ cargo add glowkit", "$ cargo run --release"]).monospace(true).bold(false).cursor(CursorStyle::Block).cursor_color(t.success).color(t.muted_foreground).size(18.0).typing_speed(18.0));
            }),
        },
        Demo {
            id: "rotating-text",
            name: "Rotating Text",
            category: C,
            description: "Cycles through words with a per-letter staggered slide inside a pill that resizes to fit, or a blur flip.",
            covers: &["React Bits: Rotating Text", "Magic UI: Word Rotate", "Aceternity: Flip Words", "Motion Primitives: Text Loop"],
            code: r##"ui.horizontal(|ui| {
    ui.label("Creative");
    ui.add(RotatingText::new(["thinking", "coding", "components!"]).pill(true));
});
ui.add(RotatingText::new(["fast", "safe", "beautiful"]).mode(RotateMode::Blur));"##,
            show: |ui| centered_at(ui, 100.0, |ui| {
                let t = Theme::of(ui);
                row(ui, "a", |ui| {
                    text(ui, "Creative", 40.0, None);
                    ui.add_space(4.0);
                    ui.add(RotatingText::new(["thinking", "coding", "components!"]).pill(true).size(40.0).stagger_from(StaggerFrom::End));
                });
                ui.add_space(30.0);
                row(ui, "b", |ui| {
                    text(ui, "Rust UIs that feel", 28.0, Some(t.muted_foreground));
                    ui.add_space(2.0);
                    ui.add(RotatingText::new(["fast.", "safe.", "beautiful.", "native."]).mode(RotateMode::Blur).color(t.accent).size(28.0).interval(2.4));
                });
            }),
        },
        Demo {
            id: "count-up",
            name: "Count Up",
            category: C,
            description: "Numbers count to their target with easing, tabular digits, separators, decimals and prefixes. Changing the target animates from the current value.",
            covers: &["React Bits: Count Up", "Magic UI: Number Ticker", "Motion Primitives: Animated Number"],
            code: r##"ui.add(CountUp::new(12_480.5).prefix("$").decimals(2).size(56.0));
ui.add(CountUp::new(98.6).decimals(1).suffix("%"));
ui.add(CountUp::new(100.0).from(0.0).duration(3.0).trigger(Trigger::OnClick));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                let t = Theme::of(ui);
                row(ui, "stats", |ui| {
                    let stat = |ui: &mut Ui, w: CountUp, label: &str| {
                        ui.allocate_ui_with_layout(vec2(250.0, 120.0), Layout::top_down(Align::Center), |ui| {
                            ui.add(w);
                            ui.add_space(2.0);
                            hint(ui, label);
                        });
                    };
                    stat(ui, CountUp::new(12_480.5).prefix("$").decimals(2).size(46.0).trigger(Trigger::Loop { pause_ms: 2500 }), "Revenue this month");
                    stat(ui, CountUp::new(98.6).decimals(1).suffix("%").size(46.0).color(t.success).trigger(Trigger::Loop { pause_ms: 2500 }), "Uptime");
                    stat(ui, CountUp::new(1_000_000.0).suffix("+").size(46.0).affix_color(t.accent).trigger(Trigger::Loop { pause_ms: 2500 }), "Downloads");
                });
                ui.add_space(14.0);
                ui.add(CountUp::new(0.0).from(10.0).size(30.0).color(t.muted_foreground).prefix("T−").duration(3.0).easing(glowkit_core::Easing::Linear).trigger(Trigger::Loop { pause_ms: 1500 }));
            }),
        },
        Demo {
            id: "sliding-number",
            name: "Sliding Number",
            category: C,
            description: "An odometer: each digit rolls vertically on a spring when the value changes, carrying past 9 like a real counter.",
            covers: &["Motion Primitives: Sliding Number", "React Bits: Counter", "Magic UI: Number Ticker (rolling)"],
            code: r##"let mut value = 1234.0;
ui.add(SlidingNumber::new(value).size(72.0));
if ui.button("+1").clicked() { value += 1.0; }"##,
            show: |ui| centered_at(ui, 70.0, |ui| {
                let t = Theme::of(ui);
                let id = ui.scope_id().with("sliding_value");
                let mut v: f64 = ui.data(|d| d.get_temp(id)).unwrap_or(1234.0);
                ui.add(SlidingNumber::new(v).size(80.0));
                ui.add_space(14.0);
                row(ui, "buttons", |ui| {
                    for (label, d) in [("−10", -10.0), ("−1", -1.0), ("+1", 1.0), ("+10", 10.0), ("+137", 137.0)] {
                        if ui.button(RichText::new(label).size(14.0)).clicked() {
                            v += d;
                        }
                    }
                    if ui.button(RichText::new("Random").size(14.0)).clicked() {
                        let r = glowkit_core::rng::hash01(ui.input(|i| i.time).to_bits());
                        v = (r * 99_999.0).round() as f64;
                    }
                });
                ui.data_mut(|d| d.insert_temp(id, v));
                ui.add_space(26.0);
                let time = glowkit_core::time(ui);
                let k = (time / 1.6) as u64;
                let live = 4_200.0 + k as f64 * 3.0 + (glowkit_core::rng::hash01(k) * 9.0).floor() as f64;
                glowkit_core::animate(ui);
                row(ui, "live", |ui| {
                    hint(ui, "Live visitors");
                    ui.add_space(6.0);
                    ui.add(SlidingNumber::new(live).size(26.0).color(t.accent));
                    ui.add_space(26.0);
                    hint(ui, "Price");
                    ui.add_space(6.0);
                    ui.add(SlidingNumber::new(19.0 + (k % 7) as f64 * 1.25).decimals(2).prefix("$").size(26.0).bold(false));
                });
            }),
        },
        Demo {
            id: "glitch-text",
            name: "Glitch Text",
            category: C,
            description: "Slices of the text jump sideways with red/cyan channel splits in short, violent bursts.",
            covers: &["React Bits: Glitch Text", "Cult UI: Glitch Text"],
            code: r##"ui.add(GlitchText::new("GLITCH").size(110.0));
ui.add(GlitchText::new("hover only").hover_only(true).size(32.0));"##,
            show: |ui| centered_at(ui, 85.0, |ui| {
                ui.add(GlitchText::new("GLITCH").size(120.0));
                ui.add_space(10.0);
                ui.add(GlitchText::new("SYSTEM FAILURE").monospace(true).size(26.0).intensity(0.7).speed(0.8));
            }),
        },
        Demo {
            id: "fuzzy-text",
            name: "Fuzzy Text",
            category: C,
            description: "Thin rows of the text jitter sideways every frame like a noisy signal; hover to crank up the fuzz.",
            covers: &["React Bits: Fuzzy Text"],
            code: r##"ui.add(FuzzyText::new("404").size(150.0));
ui.add(FuzzyText::new("not found").size(40.0).base_intensity(0.2));"##,
            show: |ui| centered_at(ui, 60.0, |ui| {
                let t = Theme::of(ui);
                ui.add(FuzzyText::new("404").size(160.0));
                ui.add(FuzzyText::new("not found").size(40.0).base_intensity(0.25).hover_intensity(0.7).color(t.muted_foreground));
            }),
        },
        Demo {
            id: "circular-text",
            name: "Circular Text",
            category: C,
            description: "Text set around a slowly rotating ring; hovering spins it faster.",
            covers: &["React Bits: Circular Text", "Magic UI: Spinning Text"],
            code: r##"ui.add(CircularText::new("GLOWKIT • ANIMATED • RUST • ").center_text("✦"));
ui.add(CircularText::new("hover to slow down * ").on_hover(HoverSpin::SlowDown));"##,
            show: |ui| centered_at(ui, 30.0, |ui| {
                let t = Theme::of(ui);
                row(ui, "rings", |ui| {
                    ui.add(CircularText::new("GLOWKIT • ANIMATED • RUST UI • ").size(26.0).center_text("✦"));
                    ui.add_space(70.0);
                    ui.add(CircularText::new("SPINNING TEXT ✺ HOVER ME ✺ ").size(18.0).radius(90.0).color(t.accent).period(-10.0).on_hover(HoverSpin::Reverse).center_text("⟳").center_color(t.foreground));
                });
            }),
        },
        Demo {
            id: "scroll-velocity",
            name: "Scroll Velocity",
            category: C,
            description: "Huge rows of text marquee in opposite directions forever; scroll over them to kick the speed (scrolling up reverses).",
            covers: &["React Bits: Scroll Velocity", "Magic UI: Scroll Based Velocity", "Magic UI: Marquee (text)"],
            code: r##"ui.add(ScrollVelocity::new(["Pure Rust", "Animated UI"]).size(64.0));"##,
            show: |ui| centered_at(ui, 85.0, |ui| {
                let t = Theme::of(ui);
                ui.add(ScrollVelocity::new(["glowkit for egui", "Scroll over me"]).size(64.0));
                ui.add_space(4.0);
                ui.add(ScrollVelocity::new(["native · web · wasm · windows · macos · linux"]).size(20.0).separator("  ·  ").speed(40.0).bold(false).colors(&[t.accent]));
            }),
        },
        Demo {
            id: "wave-text",
            name: "Wave Text",
            category: C,
            description: "Letters bob on a travelling sine wave, optionally with a gradient riding the crest.",
            covers: &["Animata: Wave Text", "Magic UI: Text Animate (wave)", "Cult UI: Text Wave"],
            code: r##"ui.add(WaveText::new("Riding the wave").size(52.0));
ui.add(WaveText::new("with a gradient").colors(&theme.gradient));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                let t = Theme::of(ui);
                ui.add(WaveText::new("Riding the wave ~").size(56.0));
                ui.add_space(14.0);
                ui.add(WaveText::new("smooth, colorful and calm").colors(&t.gradient).size(30.0).period(2.4).wavelength(16.0).amplitude(6.0));
            }),
        },
        Demo {
            id: "proximity-text",
            name: "Proximity Text",
            category: C,
            description: "Letters near the pointer swell, thicken and shift color, nudging their neighbours aside.",
            covers: &["React Bits: Variable Proximity", "React Bits: Text Pressure", "Magic UI: Dock-style text"],
            code: r##"ui.add(ProximityText::new("Hover me!").size(64.0).highlight(theme.accent));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                let t = Theme::of(ui);
                ui.add(ProximityText::new("Variable Proximity").size(64.0).highlight(t.accent).auto(true));
                ui.add_space(14.0);
                ui.add(ProximityText::new("move your pointer across these letters").size(22.0).max_scale(1.5).falloff(Falloff::Linear).color(t.muted_foreground).highlight(t.foreground));
            }),
        },
        Demo {
            id: "true-focus",
            name: "True Focus",
            category: C,
            description: "One word at a time snaps into focus inside animated corner brackets while the rest stay blurred.",
            covers: &["React Bits: True Focus"],
            code: r##"ui.add(TrueFocus::new("True Focus").size(64.0));
ui.add(TrueFocus::new("Hover each word").manual(true).size(28.0));"##,
            show: |ui| centered_at(ui, 85.0, |ui| {
                let t = Theme::of(ui);
                ui.add(TrueFocus::new("True Focus Effect").size(64.0));
                ui.add_space(24.0);
                ui.add(TrueFocus::new("Hover over each word").manual(true).frame_color(t.chart[1]).size(26.0).blur(3.0));
            }),
        },
        Demo {
            id: "sparkles-text",
            name: "Sparkles Text",
            category: C,
            description: "Text with little four-pointed stars that pop in, twinkle and fade around it.",
            covers: &["Magic UI: Sparkles Text", "Aceternity: Sparkles (text)"],
            code: r##"ui.add(SparklesText::new("Sparkles Text").size(64.0));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                ui.add(SparklesText::new("Sparkles Text").size(68.0).count(12));
                ui.add_space(8.0);
                ui.add(SparklesText::new("✨ magic, but in Rust").size(22.0).colors(&[hex("#FBBF24"), hex("#F472B6")]).count(6).bold(false));
            }),
        },
        Demo {
            id: "morphing-text",
            name: "Morphing Text",
            category: C,
            description: "Words melt into each other: the old one dissolves into a blur as the next condenses out of it.",
            covers: &["Magic UI: Morphing Text", "Motion Primitives: Text Morph"],
            code: r##"ui.add(MorphingText::new(["Hello", "Morphing", "Text", "Animation"]).size(72.0));"##,
            show: |ui| centered_at(ui, 110.0, |ui| {
                ui.add(MorphingText::new(["Hello", "Morphing", "Text", "Animation", "glowkit"]).size(80.0));
            }),
        },
        Demo {
            id: "text-generate",
            name: "Text Generate",
            category: C,
            description: "A paragraph streams in word by word, each word fading out of a blur, like an AI response.",
            covers: &["Aceternity: Text Generate Effect", "Magic UI: Text Animate (blurIn by word)"],
            code: r##"ui.add(TextGenerate::new("Oxygen gets you high. In a catastrophic emergency, we're taking giant panicked breaths.")
    .max_width(640.0)
    .size(24.0));"##,
            show: |ui| centered_at(ui, 70.0, |ui| {
                ui.add(TextGenerate::new("Oxygen gets you high. In a catastrophic emergency, we're taking giant, panicked breaths. Suddenly you become euphoric, docile. You accept your fate. It's all right here: emergency water landing, six hundred miles an hour.")
                    .max_width(680.0).size(24.0).bold(true).trigger(Trigger::Loop { pause_ms: 2500 }));
            }),
        },
        Demo {
            id: "highlight-text",
            name: "Highlight Text",
            category: C,
            description: "Animated emphasis drawn onto a phrase: marker swipe, underline, box, hand-drawn circle or strike-through.",
            covers: &["Aceternity: Hero Highlight", "Magic UI: Highlighter", "Rough Notation-style annotations"],
            code: r##"ui.add(HighlightText::new("Everything is a copy, of a copy, of a copy.")
    .phrase("copy, of a copy")
    .size(36.0));
ui.add(HighlightText::new("circled").style(HighlightStyle::Circle));"##,
            show: |ui| centered_at(ui, 70.0, |ui| {
                ui.add(HighlightText::new("With insomnia, nothing's real. Everything is far away. Everything is a copy, of a copy, of a copy.")
                    .phrase("copy, of a copy, of a copy.").size(34.0).wrap(720.0).duration(1.4).trigger(Trigger::Loop { pause_ms: 2500 }));
                ui.add_space(18.0);
                row(ui, "styles", |ui| {
                    let l = Trigger::Loop { pause_ms: 2200 };
                    ui.add(HighlightText::new("underline").style(HighlightStyle::Underline).size(24.0).trigger(l));
                    ui.add_space(18.0);
                    ui.add(HighlightText::new("boxed in").style(HighlightStyle::Box).size(24.0).trigger(l));
                    ui.add_space(18.0);
                    ui.add(HighlightText::new("circled").style(HighlightStyle::Circle).size(24.0).trigger(l));
                    ui.add_space(18.0);
                    ui.add(HighlightText::new("crossed out").style(HighlightStyle::StrikeThrough).size(24.0).trigger(l));
                });
            }),
        },
        Demo {
            id: "colorful-text",
            name: "Colorful Text",
            category: C,
            description: "Each letter takes a color from a palette; every few seconds they reshuffle in a little blur-and-bounce wave.",
            covers: &["Aceternity: Colourful Text"],
            code: r##"ui.horizontal(|ui| {
    ui.label("The best");
    ui.add(ColorfulText::new("components").size(44.0));
});"##,
            show: |ui| centered_at(ui, 110.0, |ui| {
                row(ui, "a", |ui| {
                    text(ui, "Build", 48.0, None);
                    ui.add_space(6.0);
                    ui.add(ColorfulText::new("colorful").size(48.0));
                    ui.add_space(6.0);
                    text(ui, "interfaces", 48.0, None);
                });
                ui.add_space(14.0);
                ui.add(ColorfulText::new("every letter, its own color").colors(&palettes::RAINBOW).interval(2.0).bold(false).size(22.0));
            }),
        },
        Demo {
            id: "gradual-spacing",
            name: "Gradual Spacing",
            category: C,
            description: "Letters start squeezed together and spread out to their natural spacing as they fade in.",
            covers: &["Magic UI: Gradual Spacing"],
            code: r##"ui.add(GradualSpacing::new("Gradual Spacing").size(56.0));"##,
            show: |ui| centered_at(ui, 110.0, |ui| {
                let t = Theme::of(ui);
                ui.add(GradualSpacing::new("Gradual Spacing").size(60.0).trigger(Trigger::Loop { pause_ms: 1800 }));
                ui.add_space(14.0);
                ui.add(GradualSpacing::new("L E T T E R S   B R E A T H E").size(18.0).bold(false).color(t.muted_foreground).easing(glowkit_core::Easing::OutBack).trigger(Trigger::Loop { pause_ms: 1800 }));
            }),
        },
        Demo {
            id: "text-roll",
            name: "Text Roll",
            category: C,
            description: "Letters roll up and out while copies roll in from below, staggered. Holds while hovered; great for menus.",
            covers: &["Motion Primitives: Text Roll", "Cult UI: Text Roll", "Magic UI: Flip Text"],
            code: r##"ui.add(TextRoll::new("Hover me").size(48.0));
ui.add(TextRoll::new("Looping").trigger(Trigger::Loop { pause_ms: 800 }));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                ui.add(TextRoll::new("ROLLING LETTERS").size(60.0).trigger(Trigger::Loop { pause_ms: 900 }));
                ui.add_space(26.0);
                row(ui, "menu", |ui| {
                    for w in ["Home", "Work", "About", "Contact"] {
                        ui.add(TextRoll::new(w).size(24.0).center_out(true));
                        ui.add_space(22.0);
                    }
                });
                hint(ui, "hover the menu");
            }),
        },
        Demo {
            id: "shuffle-text",
            name: "Shuffle Text",
            category: C,
            description: "Each letter is a little slot-machine reel that spins through random letters and lands on the real one.",
            covers: &["React Bits: Shuffle", "Magic UI: Hyper Text (reel)"],
            code: r##"ui.add(ShuffleText::new("SHUFFLE").size(72.0).trigger(Trigger::OnHover));
ui.add(ShuffleText::new("Sideways").direction(ShuffleDirection::Right));"##,
            show: |ui| centered_at(ui, 100.0, |ui| {
                let t = Theme::of(ui);
                ui.add(ShuffleText::new("SHUFFLE").size(80.0).trigger(Trigger::Loop { pause_ms: 1500 }));
                ui.add_space(12.0);
                ui.add(ShuffleText::new("Reels slide sideways too").direction(ShuffleDirection::Right).size(26.0).color(t.muted_foreground).stagger(0.03).trigger(Trigger::Loop { pause_ms: 1500 }));
            }),
        },
        Demo {
            id: "scroll-reveal-text",
            name: "Scroll Reveal",
            category: C,
            description: "A paragraph whose words brighten progressively from faint to full, driven by time, your own progress value or its scroll position.",
            covers: &["React Bits: Scroll Reveal", "Magic UI: Text Reveal", "Aceternity: Text Reveal"],
            code: r##"ui.add(ScrollRevealText::new("When does a man die? ...").scroll_linked());
ui.add(ScrollRevealText::new("Driven by a slider").progress(progress));"##,
            show: |ui| centered_at(ui, 80.0, |ui| {
                ui.add(ScrollRevealText::new("When does a man die? When he is hit by a bullet? No! When he suffers a disease? No! When he ate a soup made out of a poisonous mushroom? No! A man dies when he is forgotten!")
                    .max_width(760.0).size(30.0).duration(4.5).trigger(Trigger::Loop { pause_ms: 1500 }));
            }),
        },
        Demo {
            id: "line-shadow-text",
            name: "Line Shadow Text",
            category: C,
            description: "Bold text with an offset shadow made of diagonal stripes that keep sliding through it.",
            covers: &["Magic UI: Line Shadow Text"],
            code: r##"ui.add(LineShadowText::new("Shadow").size(110.0));
ui.add(LineShadowText::new("colored").shadow_color(theme.accent));"##,
            show: |ui| centered_at(ui, 80.0, |ui| {
                let t = Theme::of(ui);
                row(ui, "a", |ui| {
                    text(ui, "Ship", 110.0, None);
                    ui.add_space(14.0);
                    ui.add(LineShadowText::new("Faster").size(110.0).color(t.accent).shadow_color(t.accent));
                });
            }),
        },
        Demo {
            id: "falling-text",
            name: "Falling Text",
            category: C,
            description: "Click and the words drop under gravity, tumble, bounce and pile up; drag through the pile to shove them, click again to send them home.",
            covers: &["React Bits: Falling Text"],
            code: r##"ui.add(FallingText::new("React Bits is a library of animated and interactive components")
    .highlight(&["animated", "interactive"])
    .height(300.0));"##,
            show: |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.add(FallingText::new("glowkit is a library of animated and interactive components for egui, written in pure Rust. Click to reset, click again to drop!")
                        .highlight(&["animated", "interactive", "Rust"])
                        .size(28.0).width(ui.available_width().min(820.0)).height(320.0).trigger(Trigger::OnAppear));
                    ui.add_space(6.0);
                    hint(ui, "click the box to reset, click again to drop · drag through the pile");
                });
            },
        },
        Demo {
            id: "animated-underline",
            name: "Animated Underline",
            category: C,
            description: "Link text whose underline draws in from the left on hover and slides out to the right when the pointer leaves.",
            covers: &["Hover.dev: Underline links", "Aceternity: Navbar hover underline", "Cult UI: Animated link"],
            code: r##"if ui.add(AnimatedUnderline::new("Docs")).clicked() { /* ... */ }
ui.add(AnimatedUnderline::new("Home").active(true));
ui.add(AnimatedUnderline::new("From the middle").origin(UnderlineOrigin::Center));"##,
            show: |ui| centered_at(ui, 120.0, |ui| {
                row(ui, "nav", |ui| {
                    for (i, w) in ["Home", "Components", "Docs", "Showcase", "Blog"].iter().enumerate() {
                        ui.add(AnimatedUnderline::new(*w).size(20.0).active(i == 0));
                        ui.add_space(22.0);
                    }
                });
                ui.add_space(28.0);
                row(ui, "b", |ui| {
                    ui.add(AnimatedUnderline::new("Grows from the center").origin(UnderlineOrigin::Center).size(26.0).bold(true));
                    ui.add_space(30.0);
                    ui.add(AnimatedUnderline::new("Right to left").origin(UnderlineOrigin::Right).size(26.0).bold(true));
                });
                ui.add_space(8.0);
                hint(ui, "hover the links");
            }),
        },
        Demo {
            id: "scramble-hover",
            name: "Scramble Hover",
            category: C,
            description: "Letters dissolve into flickering symbols while hovered and resolve back left to right when the pointer leaves.",
            covers: &["Magic UI: Hyper Text (hover)", "Cult UI: Text Scramble", "Fancy Components: Scramble Hover"],
            code: r##"ui.add(ScrambleHover::new("Hover to scramble").monospace(true).size(28.0));"##,
            show: |ui| centered_at(ui, 95.0, |ui| {
                let t = Theme::of(ui);
                ui.add(ScrambleHover::new("HOVER TO SCRAMBLE").monospace(true).bold(true).size(40.0).auto(3.0));
                ui.add_space(22.0);
                for w in ["Projects", "Experiments", "Writing", "Contact"] {
                    ui.add(ScrambleHover::new(w).size(22.0).color(t.muted_foreground));
                    ui.add_space(4.0);
                }
            }),
        },
        Demo {
            id: "box-reveal",
            name: "Box Reveal",
            category: C,
            description: "A solid box sweeps across each line and slides away, uncovering text that rises in behind it.",
            covers: &["Magic UI: Box Reveal"],
            code: r##"ui.add(BoxReveal::new("Magic UI.").size(64.0));
ui.add(BoxReveal::new("UI library for egui").delay(0.2).size(24.0));"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(80.0);
                row(ui, "box", |ui| {
                    ui.vertical(|ui| {
                        let l = Trigger::Loop { pause_ms: 2500 };
                        ui.add(BoxReveal::new("glowkit.").size(72.0).trigger(l));
                        ui.add_space(6.0);
                        ui.add(BoxReveal::new("Animated components for egui").size(24.0).color(t.muted_foreground).delay(0.25).trigger(l));
                        ui.add_space(14.0);
                        ui.add(BoxReveal::new("→ 100% Rust · wasm · native").size(18.0).bold(false).box_color(t.chart[1]).delay(0.5).trigger(l));
                    });
                });
            },
        },
    ]
}
