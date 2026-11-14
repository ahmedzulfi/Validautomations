use crate::*;
use egui::{vec2, Align2, Color32, FontId, Rect, Sense, Ui};
use glowkit_core::demo::{Category::Backgrounds as C, Demo};
use glowkit_core::Theme;

/// Allocate a full-width hero area, paint the background into it and overlay a headline.
fn hero(ui: &mut Ui, title: &str, sub: &str, text: Option<Color32>, paint: impl FnOnce(&Ui, Rect)) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
    paint(ui, rect);
    headline(ui, rect, title, sub, text);
}

fn headline(ui: &Ui, rect: Rect, title: &str, sub: &str, text: Option<Color32>) {
    let t = Theme::of(ui);
    let fg = text.unwrap_or(t.foreground);
    let p = ui.painter_at(rect);
    let c = rect.center();
    let has_sub = !sub.is_empty();
    let ty = if has_sub { c.y - 14.0 } else { c.y };
    for dx in [0.0, 0.6] {
        p.text(egui::pos2(c.x + dx, ty), Align2::CENTER_CENTER, title, FontId::proportional(40.0), fg);
    }
    if has_sub {
        let sub_c = if text.is_some() { fg.gamma_multiply(0.8) } else { t.muted_foreground };
        p.text(egui::pos2(c.x, c.y + 26.0), Align2::CENTER_CENTER, sub, FontId::proportional(16.0), sub_c);
    }
}

const WHITE: Option<Color32> = Some(Color32::WHITE);

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "aurora",
            name: "Aurora",
            category: C,
            description: "Flowing curtains of northern light that drift and shimmer, fading into the background.",
            covers: &["React Bits: Aurora", "Aceternity: Aurora Background"],
            code: r##"ui.add(Aurora::new());
// or behind your own content:
Aurora::new().colors(&[hex("#3A29FF"), hex("#FF94B4"), hex("#FF3232")]).paint(ui, rect);"##,
            show: |ui| hero(ui, "Aurora", "Curtains of light, in pure Rust", None, |ui, r| Aurora::new().paint(ui, r)),
        },
        Demo {
            id: "particles",
            name: "Particles",
            category: C,
            description: "Drifting particles with depth, optional constellation links and pointer repulsion.",
            covers: &["React Bits: Particles", "Magic UI: Particles", "Aceternity: Sparkles", "tsParticles: Links"],
            code: r##"ui.add(Particles::new().count(140).links(true));
ui.add(Particles::new().count(260).twinkle(true)); // sparkles"##,
            show: |ui| hero(ui, "Particles", "Move the pointer to push them around", None, |ui, r| Particles::new().count(140).links(true).paint(ui, r)),
        },
        Demo {
            id: "squares-grid",
            name: "Squares Grid",
            category: C,
            description: "An endlessly scrolling grid; the hovered square fills in and fades as you move on.",
            covers: &["React Bits: Squares"],
            code: r##"ui.add(SquaresGrid::new().direction(Direction::Diagonal).cell_size(40.0).speed(0.5));"##,
            show: |ui| hero(ui, "Squares", "Hover the grid", None, |ui, r| SquaresGrid::new().speed(0.6).auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "dot-grid",
            name: "Dot Grid",
            category: C,
            description: "A field of dots that bulges away from the pointer and springs back with a wobble. Click for a shockwave.",
            covers: &["React Bits: Dot Grid", "Magic UI: Dot Pattern", "Aceternity UI: Grid and Dot Backgrounds"],
            code: r##"ui.add(DotGrid::new().gap(28.0).dot_size(4.0).proximity(140.0));"##,
            show: |ui| hero(ui, "Dot Grid", "Swipe through the dots, click for a shockwave", None, |ui, r| DotGrid::new().auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "grid-pattern",
            name: "Grid Pattern",
            category: C,
            description: "Thin grid lines with squares fading in and out at random, a hover highlight and an optional skew.",
            covers: &["Magic UI: Grid Pattern", "Magic UI: Animated Grid Pattern", "Magic UI: Interactive Grid Pattern", "Aceternity UI: Grid and Dot Backgrounds"],
            code: r##"ui.add(GridPattern::new().squares(30).max_opacity(0.5).skew(-12.0));"##,
            show: |ui| hero(ui, "Grid Pattern", "Squares come and go", None, |ui, r| GridPattern::new().skew(-10.0).auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "retro-grid",
            name: "Retro Grid",
            category: C,
            description: "A perspective grid floor scrolling toward the viewer, fading at the horizon.",
            covers: &["Magic UI: Retro Grid"],
            code: r##"ui.add(RetroGrid::new());
ui.add(RetroGrid::new().color(hex("#FF2E97")).glow(true)); // synthwave"##,
            show: |ui| {
                let t = Theme::of(ui);
                hero(ui, "Retro Grid", "", None, |ui, r| RetroGrid::new().color(t.gradient[1]).glow(true).horizon(0.5).paint(ui, r))
            },
        },
        Demo {
            id: "flickering-grid",
            name: "Flickering Grid",
            category: C,
            description: "A grid of tiny squares each flickering to random opacities, like a quiet LED panel.",
            covers: &["Magic UI: Flickering Grid"],
            code: r##"ui.add(FlickeringGrid::new().square_size(4.0).gap(6.0).flicker_chance(0.3).max_opacity(0.3));"##,
            show: |ui| {
                let t = Theme::of(ui);
                hero(ui, "Flickering Grid", "", None, |ui, r| FlickeringGrid::new().color(t.accent).max_opacity(0.5).flicker_chance(0.6).mask(true).paint(ui, r))
            },
        },
        Demo {
            id: "meteors",
            name: "Meteors",
            category: C,
            description: "Meteor streaks with glowing heads and fading tails falling diagonally at random speeds.",
            covers: &["Magic UI: Meteors", "Aceternity: Meteors"],
            code: r##"ui.add(Meteors::new().count(20));"##,
            show: |ui| hero(ui, "Meteors", "A quiet meteor shower", None, |ui, r| Meteors::new().count(28).paint(ui, r)),
        },
        Demo {
            id: "shooting-stars",
            name: "Shooting Stars",
            category: C,
            description: "Twinkling stars with shooting stars streaking across every couple of seconds.",
            covers: &["Aceternity: Shooting Stars", "Aceternity: Stars Background"],
            code: r##"ui.add(ShootingStars::new().stars(StarField::new().density(1.4)));
ui.add(StarField::new()); // just the stars"##,
            show: |ui| hero(ui, "Shooting Stars", "Make a wish", None, |ui, r| ShootingStars::new().stars(StarField::new().density(1.4)).delay(0.6, 2.0).paint(ui, r)),
        },
        Demo {
            id: "background-beams",
            name: "Background Beams",
            category: C,
            description: "Faint curved paths sweeping across the area with pulses of colored light racing along them.",
            covers: &["Aceternity: Background Beams", "Aceternity: Background Beams With Collision"],
            code: r##"ui.add(BackgroundBeams::new().count(36));"##,
            show: |ui| hero(ui, "Background Beams", "Light racing along the paths", None, |ui, r| BackgroundBeams::new().paint(ui, r)),
        },
        Demo {
            id: "wavy-background",
            name: "Wavy Background",
            category: C,
            description: "Thick, blurred ribbons of color undulating like slow ocean waves.",
            covers: &["Aceternity: Wavy Background"],
            code: r##"ui.add(WavyBackground::new().wave_width(50.0).blur(10.0));"##,
            show: |ui| hero(ui, "Wavy Background", "Soft ribbons of color", None, |ui, r| WavyBackground::new().paint(ui, r)),
        },
        Demo {
            id: "waves",
            name: "Waves",
            category: C,
            description: "Dozens of thin lines warped by flowing noise; the pointer drags them and they spring back.",
            covers: &["React Bits: Waves"],
            code: r##"ui.add(Waves::new().gap(14.0));"##,
            show: |ui| hero(ui, "Waves", "Drag the pointer through the lines", None, |ui, r| Waves::new().auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "threads",
            name: "Threads",
            category: C,
            description: "Fine threads fanning out from a bundle into flowing, rippling strands that lean toward the pointer.",
            covers: &["React Bits: Threads"],
            code: r##"ui.add(Threads::new().count(40).amplitude(1.0));"##,
            show: |ui| hero(ui, "Threads", "", None, |ui, r| Threads::new().paint(ui, r)),
        },
        Demo {
            id: "silk",
            name: "Silk",
            category: C,
            description: "Smooth, undulating folds of shaded silk with soft specular highlights.",
            covers: &["React Bits: Silk"],
            code: r##"ui.add(Silk::new().color(hex("#7B7481")).speed(1.0).rotation(0.0));"##,
            show: |ui| hero(ui, "Silk", "Smooth as it gets", WHITE, |ui, r| Silk::new().paint(ui, r)),
        },
        Demo {
            id: "iridescence",
            name: "Iridescence",
            category: C,
            description: "An oily, iridescent rainbow film that swirls and shifts; the pointer stirs it.",
            covers: &["React Bits: Iridescence"],
            code: r##"ui.add(Iridescence::new());
ui.add(Iridescence::new().color(hex("#A0C4FF"))); // tinted"##,
            show: |ui| hero(ui, "Iridescence", "Light on a soap bubble", Some(Color32::from_rgb(20, 20, 28)), |ui, r| Iridescence::new().paint(ui, r)),
        },
        Demo {
            id: "swirl",
            name: "Swirl",
            category: C,
            description: "Psychedelic paint swirling into a slowly spinning vortex with marbled bands.",
            covers: &["React Bits: Balatro"],
            code: r##"ui.add(Swirl::new().colors([hex("#DE443B"), hex("#006BB4"), hex("#162325")]));"##,
            show: |ui| hero(ui, "Swirl", "", WHITE, |ui, r| Swirl::new().colors([glowkit_core::hex("#DE443B"), glowkit_core::hex("#006BB4"), glowkit_core::hex("#162325")]).paint(ui, r)),
        },
        Demo {
            id: "liquid-metal",
            name: "Liquid Metal",
            category: C,
            description: "Rippling liquid chrome with flowing reflections; the pointer sends ripples through it.",
            covers: &["React Bits: Liquid Chrome"],
            code: r##"ui.add(LiquidMetal::new());
ui.add(LiquidMetal::new().tint(hex("#8FB8FF")).speed(0.6));"##,
            show: |ui| hero(ui, "Liquid Metal", "", WHITE, |ui, r| LiquidMetal::new().paint(ui, r)),
        },
        Demo {
            id: "plasma",
            name: "Plasma",
            category: C,
            description: "Classic demoscene plasma: interfering sine fields mapped to a glowing, looping palette.",
            covers: &["React Bits: Plasma"],
            code: r##"ui.add(Plasma::new());
ui.add(Plasma::new().colors(&palettes::NEON).scale(1.4));"##,
            show: |ui| hero(ui, "Plasma", "", WHITE, |ui, r| Plasma::new().paint(ui, r)),
        },
        Demo {
            id: "lightning",
            name: "Lightning",
            category: C,
            description: "A crackling bolt with a white-hot core, wide glow and flickering branches.",
            covers: &["React Bits: Lightning"],
            code: r##"ui.add(Lightning::new().color(hex("#60A5FA")).intensity(1.2));"##,
            show: |ui| hero(ui, "", "", None, |ui, r| Lightning::new().paint(ui, r)),
        },
        Demo {
            id: "orb",
            name: "Orb",
            category: C,
            description: "A glowing ring of colored light with a breathing edge that ripples and spins faster on hover.",
            covers: &["React Bits: Orb"],
            code: r##"ui.add(Orb::new());
ui.add(Orb::new().hue_shift(120.0).radius(140.0));"##,
            show: |ui| hero(ui, "Orb", "Hover me", None, |ui, r| Orb::new().paint(ui, r)),
        },
        Demo {
            id: "warp-speed",
            name: "Warp Speed",
            category: C,
            description: "Hyperspace star streaks rushing toward the viewer. Press and hold to engage warp.",
            covers: &["React Bits: Hyperspeed", "Magic UI: Warp Background"],
            code: r##"ui.add(WarpSpeed::new().speed(1.0));"##,
            show: |ui| hero(ui, "Warp Speed", "Press and hold to jump", None, |ui, r| WarpSpeed::new().paint(ui, r)),
        },
        Demo {
            id: "galaxy",
            name: "Galaxy",
            category: C,
            description: "A slowly rotating spiral galaxy with a bright core and colorful arms. Stars scatter from the pointer.",
            covers: &["React Bits: Galaxy"],
            code: r##"ui.add(Galaxy::new().arms(2).stars(700));"##,
            show: |ui| hero(ui, "", "", None, |ui, r| Galaxy::new().paint(ui, r)),
        },
        Demo {
            id: "letter-glitch",
            name: "Letter Glitch",
            category: C,
            description: "A wall of monospace letters constantly glitching to new characters and colors, with a vignette.",
            covers: &["React Bits: Letter Glitch"],
            code: r##"ui.add(LetterGlitch::new().colors(&[hex("#2b4539"), hex("#61dca3"), hex("#61b3dc")]));"##,
            show: |ui| hero(ui, "Letter Glitch", "", None, |ui, r| LetterGlitch::new().center_vignette(true).paint(ui, r)),
        },
        Demo {
            id: "matrix-rain",
            name: "Matrix Rain",
            category: C,
            description: "Columns of glyphs raining down with bright heads and fading trails.",
            covers: &["Magic UI: Matrix-style code rain", "CodePen: Matrix Digital Rain"],
            code: r##"ui.add(MatrixRain::new().font_size(14.0));"##,
            show: |ui| hero(ui, "", "", None, |ui, r| MatrixRain::new().paint(ui, r)),
        },
        Demo {
            id: "ripple",
            name: "Ripple",
            category: C,
            description: "Concentric circles breathing in a staggered wave, fading toward the bottom.",
            covers: &["Magic UI: Ripple"],
            code: r##"ui.add(Ripple::new().circles(8));
ui.add(Ripple::new().expanding(true)); // rings grow outward"##,
            show: |ui| hero(ui, "Ripple", "", None, |ui, r| Ripple::new().paint(ui, r)),
        },
        Demo {
            id: "spotlight",
            name: "Spotlight",
            category: C,
            description: "Volumetric light rays fanning down from above, swaying and shimmering, leaning toward the pointer.",
            covers: &["Aceternity: Spotlight", "React Bits: Light Rays", "Magic UI: Light Rays"],
            code: r##"ui.add(Spotlight::new());
ui.add(Spotlight::new().origin(SpotlightOrigin::TopLeft));"##,
            show: |ui| hero(ui, "Spotlight", "Light from above", None, |ui, r| Spotlight::new().paint(ui, r)),
        },
        Demo {
            id: "lamp",
            name: "Lamp",
            category: C,
            description: "A bright bar of light widening into view and casting a fan of colored light upward.",
            covers: &["Aceternity: Lamp Effect"],
            code: r##"let rect = ui.max_rect();
Lamp::new().line_y(0.45).paint(ui, rect);"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                Lamp::new().line_y(0.42).replay(true).paint(ui, rect);
                let t = Theme::of(ui);
                let p = ui.painter_at(rect);
                let c = egui::pos2(rect.center().x, rect.min.y + rect.height() * 0.42 + 60.0);
                for dx in [0.0, 0.6] {
                    p.text(c + vec2(dx, 0.0), Align2::CENTER_CENTER, "Build lamps the right way", FontId::proportional(36.0), t.foreground);
                }
            },
        },
        Demo {
            id: "vortex",
            name: "Vortex",
            category: C,
            description: "Hundreds of glowing particles swirling into a vortex with short colorful trails.",
            covers: &["Aceternity: Vortex"],
            code: r##"ui.add(Vortex::new().count(450));"##,
            show: |ui| hero(ui, "Vortex", "", None, |ui, r| Vortex::new().paint(ui, r)),
        },
        Demo {
            id: "gradient-blobs",
            name: "Gradient Blobs",
            category: C,
            description: "Big soft blobs of color drifting and blending like a lava lamp, one following the pointer.",
            covers: &["Aceternity: Background Gradient Animation", "ShaderGradient", "Magic UI: Animated gradient", "Aceternity UI: Background Gradient Animation"],
            code: r##"ui.add(GradientBlobs::new());"##,
            show: |ui| hero(ui, "Gradient Blobs", "One of them follows you", None, |ui, r| GradientBlobs::new().paint(ui, r)),
        },
        Demo {
            id: "mesh-gradient",
            name: "Mesh Gradient",
            category: C,
            description: "A smoothly animated four-color mesh gradient with organic swirl and distortion.",
            covers: &["Paper Shaders: Mesh Gradient", "Stripe-style mesh gradient"],
            code: r##"ui.add(MeshGradient::new());
ui.add(MeshGradient::new().colors([hex("#E0EAFF"), hex("#241D9A"), hex("#F75092"), hex("#9F50D3")]));"##,
            show: |ui| hero(ui, "Mesh Gradient", "", WHITE, |ui, r| MeshGradient::new().paint(ui, r)),
        },
        Demo {
            id: "snowfall",
            name: "Snowfall",
            category: C,
            description: "Gently falling snow with depth: distant flakes small and slow, near ones big and swaying.",
            covers: &["Magic UI: Snowfall", "react-snowfall"],
            code: r##"ui.add(Snowfall::new().count(220).wind(0.3));"##,
            show: |ui| hero(ui, "Let it snow", "", None, |ui, r| Snowfall::new().count(220).paint(ui, r)),
        },
        Demo {
            id: "grain-noise",
            name: "Grain Noise",
            category: C,
            description: "Animated film grain overlay that gives gradients an analog texture (right half).",
            covers: &["React Bits: Noise", "Paper Shaders: Grain Gradient"],
            code: r##"MeshGradient::new().paint(ui, rect);
GrainNoise::new().opacity(0.12).paint(ui, rect);"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                MeshGradient::new().paint(ui, rect);
                let right = Rect::from_min_max(egui::pos2(rect.center().x, rect.min.y), rect.max);
                GrainNoise::new().opacity(0.22).paint(ui, right);
                let p = ui.painter_at(rect);
                p.vline(rect.center().x, rect.y_range(), egui::Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.5)));
                let f = FontId::proportional(18.0);
                p.text(egui::pos2(rect.center().x - rect.width() * 0.25, rect.center().y), Align2::CENTER_CENTER, "without grain", f.clone(), Color32::WHITE);
                p.text(egui::pos2(rect.center().x + rect.width() * 0.25, rect.center().y), Align2::CENTER_CENTER, "with grain", f, Color32::WHITE);
            },
        },
        Demo {
            id: "background-boxes",
            name: "Background Boxes",
            category: C,
            description: "A skewed isometric grid of boxes that light up in random colors under the pointer.",
            covers: &["Aceternity: Background Boxes"],
            code: r##"ui.add(BackgroundBoxes::new());"##,
            show: |ui| hero(ui, "Background Boxes", "Hover to light them up", None, |ui, r| BackgroundBoxes::new().auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "light-beams",
            name: "Beams",
            category: C,
            description: "Tall columns of soft light with luminous texture flowing down them; rotate for a diagonal look.",
            covers: &["React Bits: Beams"],
            code: r##"ui.add(Beams::new().count(12).rotation(30.0));"##,
            show: |ui| hero(ui, "Beams", "", None, |ui, r| Beams::new().rotation(20.0).paint(ui, r)),
        },
        Demo {
            id: "hex-grid",
            name: "Hex Grid",
            category: C,
            description: "A honeycomb of hexagons with random pulses, a rolling wave of light and a pointer glow.",
            covers: &["Hexagon pattern backgrounds", "Magic UI: Interactive Grid Pattern (hex)"],
            code: r##"ui.add(HexGrid::new().radius(24.0));"##,
            show: |ui| hero(ui, "Hex Grid", "", None, |ui, r| HexGrid::new().auto_hover(true).paint(ui, r)),
        },
        Demo {
            id: "liquid-ether",
            name: "Liquid Ether",
            category: C,
            description: "A real-time fluid simulation: moving the pointer stirs glowing eddies of color that curl, mix and slowly fade. A ghost pointer keeps it alive.",
            covers: &["React Bits: Liquid Ether"],
            code: r##"ui.add(LiquidEther::new());
ui.add(LiquidEther::new().colors(&[hex("#5227FF"), hex("#FF9FFC"), hex("#B19EEF")]).force(1.4));"##,
            show: |ui| hero(ui, "Liquid Ether", "Stir it with your pointer", None, |ui, r| LiquidEther::new().paint(ui, r)),
        },
        Demo {
            id: "grid-scan",
            name: "Grid Scan",
            category: C,
            description: "A wireframe grid tunnel receding into the distance, swept by a glowing scan frame that lights up the lines it passes. The view leans toward the pointer.",
            covers: &["React Bits: Grid Scan"],
            code: r##"ui.add(GridScan::new().scan_color(hex("#FF9FFC")));"##,
            show: |ui| hero(ui, "Grid Scan", "", None, |ui, r| GridScan::new().paint(ui, r)),
        },
        Demo {
            id: "prismatic-burst",
            name: "Prismatic Burst",
            category: C,
            description: "Rays of rainbow light bursting from the center, rotating, flickering and breathing, with shock rings rippling outward.",
            covers: &["React Bits: Prismatic Burst"],
            code: r##"ui.add(PrismaticBurst::new());
ui.add(PrismaticBurst::new().colors(&palettes::MAGIC).rays(64));"##,
            show: |ui| hero(ui, "", "", None, |ui, r| PrismaticBurst::new().paint(ui, r)),
        },
        Demo {
            id: "grid-motion",
            name: "Grid Motion",
            category: C,
            description: "A tilted wall of tiles whose rows slide in opposite directions as the pointer moves, each with its own lag, for a deep parallax feel.",
            covers: &["React Bits: Grid Motion"],
            code: r##"ui.add(GridMotion::new().items(["Rust", "egui", "wasm", "60 fps"]));"##,
            show: |ui| hero(ui, "", "", None, |ui, r| GridMotion::new().paint(ui, r)),
        },
        Demo {
            id: "laser-flow",
            name: "Laser Flow",
            category: C,
            description: "A laser beam falls from the top and splashes across a surface, wrapped in luminous fog that curls outward and up.",
            covers: &["React Bits: Laser Flow"],
            code: r##"ui.add(LaserFlow::new());
ui.add(LaserFlow::new().color(hex("#FF79C6")).fog(1.4));"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                LaserFlow::new().paint(ui, rect);
                let t = Theme::of(ui);
                let card = Rect::from_center_size(egui::pos2(rect.center().x, rect.min.y + rect.height() * 0.66 + 62.0), vec2(300.0, 70.0));
                let p = ui.painter_at(rect);
                p.text(card.center(), Align2::CENTER_CENTER, "Ship at the speed of light", FontId::proportional(18.0), t.foreground.gamma_multiply(0.85));
            },
        },
        Demo {
            id: "metallic-paint",
            name: "Metallic Paint",
            category: C,
            description: "A shape or logo filled with flowing liquid metal: rippling chrome bands, a bevelled rim and a highlight that follows the pointer.",
            covers: &["React Bits: Metallic Paint"],
            code: r##"ui.add(MetallicPaint::new());
ui.add(MetallicPaint::new().shape(MetalShape::Heart).tint(hex("#F9A8D4")));"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                let w = rect.width() / 3.0;
                let shapes = [
                    (MetalShape::Heart, Some(glowkit_core::hex("#FBCFE8"))),
                    (MetalShape::Sparkle, None),
                    (MetalShape::Star, Some(glowkit_core::hex("#FDE68A"))),
                ];
                let t = Theme::of(ui);
                ui.painter().rect_filled(rect, 0.0, t.background);
                for (i, (shape, tint)) in shapes.into_iter().enumerate() {
                    let r = Rect::from_min_size(egui::pos2(rect.min.x + w * i as f32, rect.min.y), vec2(w, rect.height()));
                    let mut m = MetallicPaint::new().shape(shape).id_salt(i).transparent().scale(if i == 1 { 0.78 } else { 0.62 });
                    if let Some(c) = tint {
                        m = m.tint(c);
                    }
                    m.paint(ui, r);
                }
            },
        },
        Demo {
            id: "background-lines",
            name: "Background Lines",
            category: C,
            description: "Long flowing curves sweep across the area, drawing themselves in, lingering and erasing again, each on its own rhythm and in its own color.",
            covers: &["Aceternity UI: Background Lines"],
            code: r##"ui.add(BackgroundLines::new().count(24));"##,
            show: |ui| hero(ui, "Background Lines", "Curves that draw themselves", None, |ui, r| BackgroundLines::new().paint(ui, r)),
        },
        Demo {
            id: "dotted-map",
            name: "Dotted Map",
            category: C,
            description: "A world map drawn as a grid of dots, with glowing arcs flying between cities one after another and markers rippling where they land.",
            covers: &["Magic UI: Dotted Map", "Aceternity UI: World Map"],
            code: r##"ui.add(DottedMap::new());
ui.add(DottedMap::new().connections(&[
    (cities::LONDON, cities::NEW_YORK),
    (cities::LONDON, cities::TOKYO),
]));"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 380.0), Sense::hover());
                DottedMap::new().paint(ui, rect);
            },
        },
    ]
}
