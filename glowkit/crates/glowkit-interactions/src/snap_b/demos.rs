use super::*;
use egui::{pos2, vec2, Align, Layout, Rect, Sense, Ui, UiBuilder};
use glowkit_core::demo::{Category, Demo};
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Parameter values for a demo, persisted per demo.
fn params<const N: usize>(ui: &Ui, init: [f32; N]) -> [f32; N] {
    mem(ui, "snap_b_params", || init)
}

/// Run `f` inside `rect`, vertically centered using last frame's measured height.
fn centered(ui: &mut Ui, rect: Rect, key: &str, f: impl FnOnce(&mut Ui)) {
    let mid = ui.scope_id().with(("snap_b_centered", key));
    let measured: f32 = glowkit_core::state(ui, mid, || rect.height() * 0.5);
    let top = ((rect.height() - measured) / 2.0).max(0.0);
    let child = Rect::from_min_max(rect.min + vec2(0.0, top), rect.max);
    let h = ui
        .scope_builder(UiBuilder::new().max_rect(child).layout(Layout::top_down(Align::Center)), |ui| {
            f(ui);
            ui.min_rect().height()
        })
        .inner;
    glowkit_core::set_state(ui, mid, h);
}

/// Live component on the left, a column of parameter sliders on the right (stacked on narrow
/// screens). Parameters persist between frames.
fn bench<const N: usize>(
    ui: &mut Ui,
    height: f32,
    init: [f32; N],
    component: impl FnOnce(&mut Ui, &[f32; N]),
    controls: impl FnOnce(&mut Ui, &mut [f32; N]),
) {
    let mut p = params(ui, init);
    let w = ui.available_width();
    if w > 640.0 {
        let panel_w = 220.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        let left = Rect::from_min_size(rect.min, vec2(w - panel_w - 24.0, height));
        let right = Rect::from_min_max(pos2(rect.max.x - panel_w, rect.min.y), rect.max);
        centered(ui, left, "stage", |ui| component(ui, &p));
        centered(ui, right, "panel", |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            controls(ui, &mut p);
        });
    } else {
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        centered(ui, rect, "stage", |ui| component(ui, &p));
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        controls(ui, &mut p);
    }
    set_mem(ui, "snap_b_params", p);
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "snap-molten-text",
            name: "Molten Text",
            category: Category::TextAnimations,
            description: "A headline filled with churning molten metal: domain-warped lava, bevelled letter edges and a heat glow.",
            covers: &["Snapblocks: Molten Text"],
            code: r##"ui.add(MoltenText::new("Molten.").size(110.0).heat(0.6).bevel(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [0.6, 1.0, 1.0],
                    |ui, p| {
                        ui.add(MoltenText::new("Molten.").size(112.0).heat(p[0]).bevel(p[1]).speed(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Heat", &mut p[0], 0.0..=1.0).decimals(2));
                        ui.add(ControlSlider::new("Bevel", &mut p[1], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Speed", &mut p[2], 0.0..=3.0).decimals(1).suffix("x"));
                    },
                );
            },
        },
        Demo {
            id: "snap-chrome-text",
            name: "Chrome Text",
            category: Category::TextAnimations,
            description: "Liquid chrome lettering: a sky-horizon-ground environment reflected in bevelled type, rippling, with a sweeping glint.",
            covers: &["Snapblocks: Chrome Text"],
            code: r##"ui.add(ChromeText::new("Chrome.").size(110.0).ripple(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [1.0, 3.2, 0.0],
                    |ui, p| {
                        let mut c = ChromeText::new("Chrome.").size(112.0).ripple(p[0]).glint_every(p[1]);
                        if p[2] > 0.5 {
                            c = c.tint(glowkit_core::hex("#F3C969"));
                        }
                        ui.add(c);
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Ripple", &mut p[0], 0.0..=3.0).decimals(1));
                        ui.add(ControlSlider::new("Glint every", &mut p[1], 1.0..=8.0).decimals(1).suffix("s"));
                        ui.add(ControlSlider::new("Gold", &mut p[2], 0.0..=1.0));
                    },
                );
            },
        },
        Demo {
            id: "snap-grain-text",
            name: "Grain Text",
            category: Category::TextAnimations,
            description: "A headline filled with a slow drifting gradient of light under real per-pixel film grain.",
            covers: &["Snapblocks: Grain Text"],
            code: r##"ui.add(GrainText::new("Grainy.").size(110.0).drift(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [1.0, 1.0],
                    |ui, p| {
                        ui.add(GrainText::new("Grainy.").size(112.0).drift(p[0]).speed(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Drift", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Speed", &mut p[1], 0.0..=4.0).decimals(1).suffix("x"));
                    },
                );
            },
        },
        Demo {
            id: "snap-dither-text",
            name: "Dither Text",
            category: Category::TextAnimations,
            description: "A headline printed in four inks through an ordered-dither screen, with a band of light sweeping across it.",
            covers: &["Snapblocks: Dither Text"],
            code: r##"ui.add(DitherText::new("DITHER").size(120.0).cell(4.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [4.0, 4.5, 1.0],
                    |ui, p| {
                        ui.add(DitherText::new("DITHER").size(124.0).cell(p[0]).sweep_every(p[1]).speed(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Cell", &mut p[0], 2.0..=10.0).suffix("px"));
                        ui.add(ControlSlider::new("Sweep every", &mut p[1], 1.5..=10.0).decimals(1).suffix("s"));
                        ui.add(ControlSlider::new("Speed", &mut p[2], 0.0..=3.0).decimals(1).suffix("x"));
                    },
                );
            },
        },
        Demo {
            id: "snap-dither-lens",
            name: "Dither Lens",
            category: Category::CursorAndInteraction,
            description: "A photograph that turns into an ordered-dither print inside a ragged, flickering zone that follows the pointer.",
            covers: &["Snapblocks: Dither Lens"],
            code: r##"ui.add(DitherLens::new().radius(92.0).cell(5.0).ragged(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [92.0, 5.0, 1.0],
                    |ui, p| {
                        let w = ui.available_width().min(640.0);
                        ui.add(DitherLens::new().size(vec2(w, 340.0)).radius(p[0]).cell(p[1]).ragged(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Radius", &mut p[0], 30.0..=180.0).suffix("px"));
                        ui.add(ControlSlider::new("Cell", &mut p[1], 3.0..=12.0).suffix("px"));
                        ui.add(ControlSlider::new("Ragged", &mut p[2], 0.0..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-halftone-logo",
            name: "Halftone Logo",
            category: Category::Backgrounds,
            description: "A logo built from a rotated halftone dot screen with a wave of ink rolling through the dots.",
            covers: &["Snapblocks: Halftone Logo"],
            code: r##"ui.add(HalftoneLogo::new("lumen").pitch(7.0).angle(20.0).wave(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [7.0, 20.0, 1.0],
                    |ui, p| {
                        ui.add(HalftoneLogo::new("lumen").size(104.0).pitch(p[0]).angle(p[1]).wave(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Pitch", &mut p[0], 4.0..=14.0).decimals(1).suffix("px"));
                        ui.add(ControlSlider::new("Angle", &mut p[1], 0.0..=90.0).suffix("°"));
                        ui.add(ControlSlider::new("Wave", &mut p[2], 0.0..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-holo-card",
            name: "Holo Card",
            category: Category::CardsAndBorders,
            description: "A holographic foil card that tilts toward the pointer in 3D, its thin-film sheen sliding with the angle.",
            covers: &["Snapblocks: Holo Card"],
            code: r##"ui.add(HoloCard::new().title("Lifetime").subtitle("Every component, forever.").tilt(16.0));"##,
            show: |ui| {
                bench(
                    ui,
                    470.0,
                    [16.0, 1.0],
                    |ui, p| {
                        ui.add(HoloCard::new().tilt(p[0]).foil(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Tilt", &mut p[0], 0.0..=30.0).suffix("°"));
                        ui.add(ControlSlider::new("Foil", &mut p[1], 0.0..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-glass-tabs",
            name: "Glass Tabs",
            category: Category::Navigation,
            description: "A tab switch whose selection is a thick liquid-glass lens that magnifies the track, squeezes in flight and wobbles as it lands.",
            covers: &["Snapblocks: Glass Tabs"],
            code: r##"ui.add(GlassTabs::new(&mut tab, ["Work", "Studio", "Journal", "Contact"]).wobble(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    300.0,
                    [1.0, 64.0, 2.0],
                    |ui, p| {
                        let mut tab = mem(ui, "tab", || 2usize);
                        ui.add(GlassTabs::new(&mut tab, ["Work", "Studio", "Journal", "Contact"]).wobble(p[0]).height(p[1]).autoplay(p[2]));
                        set_mem(ui, "tab", tab);
                        ui.add_space(18.0);
                        ui.weak("Click a tab, drag the lens, or use ← →");
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Wobble", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Height", &mut p[1], 40.0..=90.0).suffix("px"));
                        ui.add(ControlSlider::new("Autoplay", &mut p[2], 0.0..=5.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-morph-navbar",
            name: "Morph Navbar",
            category: Category::Navigation,
            description: "A dark pill nav that springs open into its own dropdown, one shell resizing between a picture reel, an accordion and a link list.",
            covers: &["Snapblocks: Morph Navbar"],
            code: r##"ui.add(MorphNavbar::new().brand("Fieldhouse").items("Studio", "Projects", "Journal"));"##,
            show: |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(MorphNavbar::new());
                });
            },
        },
        Demo {
            id: "snap-liquid-glass-button",
            name: "Liquid Glass Button",
            category: Category::Buttons,
            description: "Glass pills that blur and bend what is behind them like a thick lens, with rim highlights chasing the pointer.",
            covers: &["Snapblocks: Liquid Glass Button"],
            code: r##"if ui.add(LiquidGlassButton::new("Get started").icon(GlassGlyph::Chevron)).clicked() {}
ui.add(LiquidGlassButton::new("Tinted").variant(GlassVariant::Tinted(hex("#3B3FB8"))).icon(GlassGlyph::Plus));"##,
            show: |ui| {
                bench(
                    ui,
                    340.0,
                    [1.0, 54.0],
                    |ui, p| {
                        let stage = ui.available_rect_before_wrap();
                        let stage = Rect::from_min_size(stage.min, vec2(stage.width(), 300.0));
                        LiquidGlassButton::paint_dot_backdrop(ui, stage);
                        ui.add_space(30.0);
                        ui.add(LiquidGlassButton::new("Get started").icon(GlassGlyph::Chevron).refraction(p[0]).height(p[1] + 6.0));
                        ui.add_space(24.0);
                        ui.horizontal(|ui| {
                            let w = 3.0 * (p[1] * 3.4) + 40.0;
                            ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                            ui.add(LiquidGlassButton::new("Frosted").variant(GlassVariant::Frosted).icon(GlassGlyph::Arrow).refraction(p[0]).height(p[1]));
                            ui.add(LiquidGlassButton::new("Tinted").variant(GlassVariant::Tinted(glowkit_core::hex("#3B3FB8"))).icon(GlassGlyph::Plus).refraction(p[0]).height(p[1]));
                            ui.add(LiquidGlassButton::new("Disabled").disabled(true).icon(GlassGlyph::Chevron).height(p[1]));
                        });
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Refraction", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Height", &mut p[1], 36.0..=72.0).suffix("px"));
                    },
                );
            },
        },
        Demo {
            id: "snap-dna-carousel",
            name: "DNA Carousel",
            category: Category::ThreeD,
            description: "Pictures wound onto two opposing helix strands that drift past, the far ones blurred and colour-fringed like a lens edge.",
            covers: &["Snapblocks: DNA Carousel"],
            code: r##"ui.add(DnaCarousel::new().twist(1.0).speed(0.25).fringe(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    380.0,
                    [1.0, 0.25, 1.0],
                    |ui, p| {
                        ui.add(DnaCarousel::new().twist(p[0]).speed(p[1]).fringe(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Twist", &mut p[0], 0.3..=2.5).decimals(1));
                        ui.add(ControlSlider::new("Drift", &mut p[1], 0.0..=1.5).decimals(2));
                        ui.add(ControlSlider::new("Fringe", &mut p[2], 0.0..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-arc-carousel",
            name: "Arc Carousel",
            category: Category::ThreeD,
            description: "Cards riding a very large circle, the focused one upright at the apex and the rest tilted tangent to the curve.",
            covers: &["Snapblocks: Arc Carousel"],
            code: r##"let items = vec![ArcItem::new("Old Quarter", "Lanes that bend toward the river."), /* ... */];
ui.add(ArcCarousel::new(&items).curve(1.0).autoplay(2.8));"##,
            show: |ui| {
                bench(
                    ui,
                    400.0,
                    [1.0, 2.8],
                    |ui, p| {
                        let items = [
                            ArcItem::new("Alpenglow", "Peaks that hold the last light long after sunset."),
                            ArcItem::new("Lagoon", "Shallow water, warm sand and nowhere to be."),
                            ArcItem::new("Aurora", "A night sky that will not sit still."),
                            ArcItem::new("Dunes", "Wind-drawn ridges that move a little every day."),
                            ArcItem::new("Evergreen", "Mist between the pines before the town wakes."),
                            ArcItem::new("Neon City", "Streets narrow enough to hear every sign hum."),
                            ArcItem::new("Glacier", "Blue ice and a lake as still as glass."),
                            ArcItem::new("Lavender", "Rolling rows of purple to the horizon."),
                        ];
                        ui.add(ArcCarousel::new(&items).curve(p[0]).autoplay(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Curve", &mut p[0], 0.4..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Autoplay", &mut p[1], 0.0..=6.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-wave-gallery",
            name: "Wave Gallery",
            category: Category::ThreeD,
            description: "An endless ribbon of glossy cards bent into a wave in depth, titles printed into the surface, far cards sinking into fog.",
            covers: &["Snapblocks: Wave Gallery"],
            code: r##"ui.add(WaveGallery::new().amplitude(1.0).speed(40.0));"##,
            show: |ui| {
                bench(
                    ui,
                    370.0,
                    [1.0, 40.0],
                    |ui, p| {
                        ui.add(WaveGallery::new().amplitude(p[0]).speed(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Amplitude", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Drift", &mut p[1], 0.0..=160.0).suffix("px/s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-image-tunnel",
            name: "Image Tunnel",
            category: Category::ThreeD,
            description: "An endless 3D corridor with pictures on all four walls and a fine wireframe gliding toward you; click to pause, hold to rush.",
            covers: &["Snapblocks: Image Tunnel"],
            code: r##"ui.add(ImageTunnel::new().title("Into the archive").speed(1.0).density(1.4));"##,
            show: |ui| {
                bench(
                    ui,
                    380.0,
                    [1.0, 1.4],
                    |ui, p| {
                        ui.add(ImageTunnel::new().speed(p[0]).density(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=3.0).decimals(1).suffix("x"));
                        ui.add(ControlSlider::new("Density", &mut p[1], 0.3..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-spiral-slider",
            name: "Spiral Slider",
            category: Category::ThreeD,
            description: "Curved picture panels wound onto a helix that screws along its own track, the far side frosted through the gaps.",
            covers: &["Snapblocks: Spiral Slider"],
            code: r##"ui.add(SpiralSlider::new().radius(150.0).turns(1.0).frost(1.0));"##,
            show: |ui| {
                bench(
                    ui,
                    380.0,
                    [150.0, 1.0, 1.0],
                    |ui, p| {
                        ui.add(SpiralSlider::new().radius(p[0]).turns(p[1]).frost(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Radius", &mut p[0], 80.0..=220.0).suffix("px"));
                        ui.add(ControlSlider::new("Screw", &mut p[1], 0.5..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Frost", &mut p[2], 0.0..=1.0).decimals(2));
                    },
                );
            },
        },
        Demo {
            id: "snap-warp-grid",
            name: "Warp Grid",
            category: Category::Backgrounds,
            description: "A fine grid that bends into a gravity well around a dark sphere under the pointer, glowing where it is pulled hardest.",
            covers: &["Snapblocks: Warp Grid"],
            code: r##"WarpGrid::new().cell(28.0).strength(1.0).paint(ui, rect);"##,
            show: |ui| {
                bench(
                    ui,
                    380.0,
                    [28.0, 1.0, 80.0],
                    |ui, p| {
                        let w = ui.available_width();
                        let (rect, _) = ui.allocate_exact_size(vec2(w, 370.0), Sense::hover());
                        WarpGrid::new().cell(p[0]).strength(p[1]).well(p[2]).paint(ui, rect);
                        let t = glowkit_core::Theme::of(ui);
                        let painter = ui.painter_at(rect);
                        painter.text(rect.center() + vec2(0.0, -34.0), egui::Align2::CENTER_CENTER, "F I E L D  /  0 1", egui::FontId::monospace(11.0), t.accent);
                        painter.text(rect.center() + vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, "Bend space.", super::common::display_font(ui, 54.0), t.foreground);
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Cell", &mut p[0], 12.0..=60.0).suffix("px"));
                        ui.add(ControlSlider::new("Strength", &mut p[1], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Well", &mut p[2], 20.0..=160.0).suffix("px"));
                    },
                );
            },
        },
        Demo {
            id: "snap-marble-scroll",
            name: "Marble Scroll",
            category: Category::Motion,
            description: "A scroll story of full-bleed pictures whose seams tear and melt into marbling as they cross the view, strongest when scrolling fast.",
            covers: &["Snapblocks: Marble Scroll"],
            code: r##"// owns its viewport (wheel / drag / autoplay)
ui.add(MarbleScroll::new().titles(["Bloom", "Drift", "Thaw", "Glow", "Still"]).marble(1.0));
// or follow a ScrollArea: MarbleScroll::new().linked(true), or drive it: .progress(p)"##,
            show: |ui| {
                bench(
                    ui,
                    390.0,
                    [1.0, 3.2],
                    |ui, p| {
                        ui.add(MarbleScroll::new().marble(p[0]).autoplay(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Marble", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Autoplay", &mut p[1], 0.0..=6.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-mosaic-trail",
            name: "Mosaic Trail",
            category: Category::CursorAndInteraction,
            description: "A field of tiny greyscale thumbnails that swell and bloom into colour along the pointer's path, then shrink back to dots.",
            covers: &["Snapblocks: Mosaic Trail"],
            code: r##"ui.add(MosaicTrail::new().cell(26.0).reach(70.0).fade(1.1));"##,
            show: |ui| {
                bench(
                    ui,
                    370.0,
                    [26.0, 70.0, 1.1],
                    |ui, p| {
                        ui.add(MosaicTrail::new().size(vec2(ui.available_width(), 360.0)).cell(p[0]).reach(p[1]).fade(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Cell", &mut p[0], 14.0..=48.0).suffix("px"));
                        ui.add(ControlSlider::new("Reach", &mut p[1], 20.0..=160.0).suffix("px"));
                        ui.add(ControlSlider::new("Fade", &mut p[2], 0.2..=4.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-step-path",
            name: "Step Path",
            category: Category::Motion,
            description: "Process steps joined by one dashed line hopping dot to dot; it draws in on scroll, steps fading up as it arrives, then a darker stretch travels along it.",
            covers: &["Snapblocks: Step Path"],
            code: r##"let steps = vec![PathStep::new("Listen", "..."), PathStep::new("Sketch", "..."), /* ... */];
ui.add(StepPath::new(&steps));            // draws in as it scrolls into view
ui.add(StepPath::new(&steps).progress(p)); // or drive it yourself"##,
            show: |ui| {
                bench(
                    ui,
                    360.0,
                    [1.0],
                    |ui, p| {
                        let steps = [
                            PathStep::new("Listen", "Two weeks with the people who will actually use it."),
                            PathStep::new("Sketch", "Paper first, then rough flows we can poke holes in."),
                            PathStep::new("Build", "Small slices shipped weekly, each one measured."),
                            PathStep::new("Tune", "We stay on until the numbers settle and the team is happy."),
                        ];
                        ui.add(StepPath::new(&steps).hop(p[0]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Hop", &mut p[0], 0.0..=2.0).decimals(1));
                    },
                );
            },
        },
        Demo {
            id: "snap-quote-deck",
            name: "Quote Deck",
            category: Category::CardsAndBorders,
            description: "A testimonial deck fanned across the width: the focused quote upright, the rest leaning away, shrinking and dimming.",
            covers: &["Snapblocks: Quote Deck"],
            code: r##"let quotes = vec![Quote::new("Every detail was argued for.", "Mira Okafor", "Design lead", "Northwind"), /* ... */];
ui.add(QuoteDeck::new(&quotes).lean(1.0).autoplay(3.6));"##,
            show: |ui| {
                bench(
                    ui,
                    450.0,
                    [1.0, 3.6],
                    |ui, p| {
                        let quotes = [
                            Quote::new("Calm, quick and honest. The launch felt easy for once.", "Theo Marsh", "Founder", "Saltmarsh"),
                            Quote::new("They argued for every detail and they were right every time.", "Mira Okafor", "Design lead", "Northwind").stars(5),
                            Quote::new("We came for a refresh and left with a voice that sounds like us.", "Jonas Pike", "CEO", "Pike & Sons").stars(4),
                            Quote::new("The new onboarding paid for the whole project in a month.", "Ada Lindqvist", "Head of growth", "Kiln").stars(5),
                            Quote::new("Clear plans, no surprises, and a team we would hire again.", "Sam Reyes", "COO", "Fieldwork").stars(5),
                        ];
                        ui.add(QuoteDeck::new(&quotes).lean(p[0]).autoplay(p[1]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Lean", &mut p[0], 0.0..=2.0).decimals(1));
                        ui.add(ControlSlider::new("Autoplay", &mut p[1], 0.0..=8.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-story-deck",
            name: "Story Deck",
            category: Category::CardsAndBorders,
            description: "Customer stories beside a deck of portrait cards you can throw; autoplay runs with a progress ring round the next button.",
            covers: &["Snapblocks: Story Deck"],
            code: r##"let stories = vec![Story::new("We shipped in half the time.", "Ana Ruiz", "CTO, Brightline", "2x", "faster releases"), /* ... */];
ui.add(StoryDeck::new(&stories).autoplay(5.0));"##,
            show: |ui| {
                bench(
                    ui,
                    410.0,
                    [5.0],
                    |ui, p| {
                        let stories = [
                            Story::new("We stopped arguing about the roadmap and started shipping it.", "Ana Ruiz", "CTO, Brightline", "2.4x", "faster release cadence"),
                            Story::new("Our support queue halved the week the new flows went live.", "Kofi Mensah", "Head of CX, Parcel", "-52%", "support tickets"),
                            Story::new("It finally feels like one product instead of five.", "Lena Vogt", "VP Product, Atlas", "+38%", "weekly active teams"),
                            Story::new("Investors asked who designed it. Twice.", "Ravi Shah", "Founder, Tidepool", "$12M", "series A, closed"),
                        ];
                        ui.add(StoryDeck::new(&stories).autoplay(p[0]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Autoplay", &mut p[0], 0.0..=10.0).decimals(1).suffix("s"));
                    },
                );
            },
        },
        Demo {
            id: "snap-particle-globe",
            name: "Particle Globe",
            category: Category::ThreeD,
            description: "Continents drawn as thousands of points on a slowly rotating sphere; drag to spin it and it keeps its momentum.",
            covers: &["Snapblocks: Particle Globe"],
            code: r##"ui.add(ParticleGlobe::new().radius(160.0).density(12_000).speed(0.18));"##,
            show: |ui| {
                bench(
                    ui,
                    380.0,
                    [160.0, 12000.0, 0.18],
                    |ui, p| {
                        ui.add(ParticleGlobe::new().radius(p[0]).density(p[1] as usize).speed(p[2]));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Radius", &mut p[0], 80.0..=175.0).suffix("px"));
                        ui.add(ControlSlider::new("Points", &mut p[1], 3000.0..=30000.0).step(1000.0));
                        ui.add(ControlSlider::new("Spin", &mut p[2], 0.0..=1.0).decimals(2));
                    },
                );
            },
        },
    ]
}
