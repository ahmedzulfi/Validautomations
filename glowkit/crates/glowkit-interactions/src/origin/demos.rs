use super::*;
use egui::{vec2, Align, Layout, Rect, Sense, Ui, UiBuilder, Vec2};
use glowkit_core::color::hex;
use glowkit_core::demo::{Category, Demo};
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// A row of live parameter sliders; `vals` persist under `key`.
fn controls<const N: usize>(ui: &mut Ui, key: &str, init: [f32; N], spec: [(&str, f32, f32, usize); N]) -> [f32; N] {
    let mut vals = mem(ui, key, || init);
    ui.add_space(10.0);
    let w = ((ui.available_width() - 12.0 * (N as f32 - 1.0)) / N as f32).max(80.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for (i, (label, lo, hi, dec)) in spec.iter().enumerate() {
            let s = ControlSlider::new(*label, &mut vals[i], *lo..=*hi).width(w);
            let s = if *dec > 0 { s.decimals(*dec) } else { s };
            ui.add(s);
        }
    });
    set_mem(ui, key, vals);
    vals
}

/// Center a left-to-right row inside a full-width box at least `height` tall (measured last frame).
fn centered_row(ui: &mut Ui, height: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with("origin_center");
    let size: Vec2 = glowkit_core::state(ui, id, || Vec2::ZERO);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height.max(size.y)), Sense::hover());
    let inner = Rect::from_center_size(rect.center(), size);
    let r = ui.scope_builder(
        UiBuilder::new().max_rect(Rect::from_min_max(inner.min, rect.max)).layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(24.0, 18.0);
            add(ui);
            ui.min_rect().size()
        },
    );
    if (r.inner - size).length() > 0.5 {
        glowkit_core::set_state(ui, id, r.inner);
        ui.ctx().request_repaint();
    }
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "tornado",
            name: "Tornado",
            category: Category::Backgrounds,
            description: "A strand vortex: hundreds of helical lines pinch into a waist and flare into a swirling floor, with motes and comets riding the funnel; the pointer parts the strands.",
            covers: &["Originkit: Tornado"],
            code: r##"ui.add(Tornado::new().lines(160).twist(3.0).waist(0.24));"##,
            show: |ui| {
                let v = controls(ui, "tornado", [160.0, 3.0, 0.24], [("Lines", 40.0, 320.0, 0), ("Twist", 0.0, 6.0, 1), ("Waist", 0.08, 0.6, 2)]);
                ui.add_space(8.0);
                ui.add(Tornado::new().lines(v[0] as usize).twist(v[1]).waist(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "particle-saturn",
            name: "Particle Saturn",
            category: Category::ThreeD,
            description: "A point-cloud ringed planet: a Fibonacci-sphere body inside wide particle rings with dark divisions; inner rings orbit faster. Drag to spin with inertia.",
            covers: &["Originkit: Particle Saturn"],
            code: r##"ui.add(ParticleSaturn::new().tilt(0.24).spin(1.0));"##,
            show: |ui| {
                let v = controls(ui, "saturn", [0.24, 1.0, 1.0], [("Tilt", -0.8, 0.8, 2), ("Spin", 0.0, 4.0, 1), ("Dot size", 0.5, 2.0, 1)]);
                ui.add_space(8.0);
                ui.add(ParticleSaturn::new().tilt(v[0]).spin(v[1]).dot_size(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "accretion-disc",
            name: "Accretion Disc",
            category: Category::Backgrounds,
            description: "A black hole: thousands of particles orbit an occluding core along glowing log-spiral lanes, with faint polar jets. Drag to orbit the camera.",
            covers: &["Originkit: Accretion Disc"],
            code: r##"ui.add(AccretionDisc::new().arms(2).wind(1.6));"##,
            show: |ui| {
                let v = controls(ui, "accretion", [2.0, 1.8, 1.0], [("Lanes", 1.0, 5.0, 0), ("Wind", 0.6, 3.0, 1), ("Speed", 0.0, 3.0, 1)]);
                ui.add_space(8.0);
                ui.add(AccretionDisc::new().arms(v[0] as usize).wind(v[1]).speed(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "cosmic-orb",
            name: "Cosmic Orb",
            category: Category::ThreeD,
            description: "A glass sphere holding a turning galaxy: drifting nebula, twinkling stars, a lensed rim with chromatic fringes. It leans toward the pointer.",
            covers: &["Originkit: Cosmic Orb"],
            code: r##"ui.add(CosmicOrb::new().radius(140.0).lens(0.45));"##,
            show: |ui| {
                let v = controls(ui, "orb", [140.0, 1.0, 0.45], [("Radius", 60.0, 170.0, 0), ("Speed", 0.0, 4.0, 1), ("Lens", 0.0, 1.0, 2)]);
                ui.add_space(8.0);
                ui.add(CosmicOrb::new().radius(v[0]).speed(v[1]).lens(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "light-cables",
            name: "Light Cables",
            category: Category::Backgrounds,
            description: "A bundle of glowing fibre cables rising from a wide fan, folding through one bend into a bright point, with light pulses running up them; the cables lean toward the pointer.",
            covers: &["Originkit: Light Cables"],
            code: r##"ui.add(LightCables::new().cables(28).bend(0.5));"##,
            show: |ui| {
                let v = controls(ui, "cables", [28.0, 1.0, 0.5], [("Cables", 6.0, 48.0, 0), ("Pulses", 0.0, 3.0, 1), ("Bend", 0.0, 1.0, 2)]);
                ui.add_space(8.0);
                ui.add(LightCables::new().cables(v[0] as usize).pulses(v[1]).bend(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "cursor-ring-field",
            name: "Cursor Ring Field",
            category: Category::CursorAndInteraction,
            description: "A field of scattered capsules lit by a soft ring trailing the pointer: capsules on the ring brighten, turn and get pushed outward, then drift back.",
            covers: &["Originkit: Cursor Ring Field"],
            code: r##"ui.add(CursorRingField::new().ring_radius(110.0).spacing(22.0));"##,
            show: |ui| {
                let v = controls(ui, "ringfield", [110.0, 22.0, 1.0], [("Ring", 40.0, 200.0, 0), ("Spacing", 14.0, 40.0, 0), ("Push", 0.0, 2.5, 1)]);
                ui.add_space(8.0);
                ui.add(CursorRingField::new().ring_radius(v[0]).spacing(v[1]).push(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "wire-terrain",
            name: "Wire Terrain",
            category: Category::Backgrounds,
            description: "An endless flight down a wireframe valley toward a banded horizon sun, with hidden-line ridges; the pointer steers the camera.",
            covers: &["Originkit: Wire Terrain"],
            code: r##"ui.add(WireTerrain::new().speed(1.0).ridges(1.0));"##,
            show: |ui| {
                let v = controls(ui, "terrain", [1.0, 1.0], [("Speed", 0.0, 3.0, 1), ("Ridges", 0.0, 2.0, 1)]);
                ui.add_space(8.0);
                ui.add(WireTerrain::new().speed(v[0]).ridges(v[1]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "ribbon-glow",
            name: "Ribbon Glow",
            category: Category::Backgrounds,
            description: "Layered ribbons of light sweep across the frame, twisting as they go and bending into a bright swirl around the pointer.",
            covers: &["Originkit: Ribbon Glow"],
            code: r##"ui.add(RibbonGlow::new().ribbons(6).width(1.0));"##,
            show: |ui| {
                let v = controls(ui, "ribbons", [6.0, 1.0, 1.0], [("Ribbons", 1.0, 12.0, 0), ("Width", 0.4, 2.5, 1), ("Speed", 0.0, 3.0, 1)]);
                ui.add_space(8.0);
                ui.add(RibbonGlow::new().ribbons(v[0] as usize).width(v[1]).speed(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "matrix-junction",
            name: "Matrix Junction",
            category: Category::Backgrounds,
            description: "Glowing beams converge from every edge on a bright node that throws jittered lightning arcs toward the pointer.",
            covers: &["Originkit: Matrix Junction"],
            code: r##"ui.add(MatrixJunction::new().beams(26).bolts(3));"##,
            show: |ui| {
                let v = controls(ui, "junction", [26.0, 3.0, 1.0], [("Beams", 6.0, 60.0, 0), ("Bolts", 0.0, 8.0, 0), ("Jag", 0.0, 2.0, 1)]);
                ui.add_space(8.0);
                ui.add(MatrixJunction::new().beams(v[0] as usize).bolts(v[1] as usize).jag(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "fluid-trail",
            name: "Fluid Trail",
            category: Category::CursorAndInteraction,
            description: "The pointer stirs a small real-time fluid simulation, leaving glowing, color-cycling dye that curls and fades behind it.",
            covers: &["Originkit: Fluid Trail"],
            code: r##"ui.add(FluidTrail::new().fade(0.8).radius(1.0));"##,
            show: |ui| {
                let v = controls(ui, "fluid", [0.8, 1.0], [("Fade", 0.1, 3.0, 1), ("Radius", 0.4, 3.0, 1)]);
                ui.add_space(8.0);
                ui.add(FluidTrail::new().fade(v[0]).radius(v[1]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "venom-button",
            name: "Venom Button",
            category: Category::Buttons,
            description: "A glossy black goo button: liquid strands stretch out to wrap a cartoon glove cursor when it comes close, then snap back with a wobble when you pull away.",
            covers: &["Originkit: Venom Button"],
            code: r##"if ui.add(VenomButton::new("We are Venom").strands(6)).clicked() { /* ... */ }"##,
            show: |ui| {
                let v = controls(ui, "venom", [6.0, 150.0], [("Strands", 1.0, 12.0, 0), ("Reach", 60.0, 240.0, 0)]);
                ui.add_space(8.0);
                centered_row(ui, 260.0, |ui| {
                    ui.add(VenomButton::new("We are Venom").strands(v[0] as usize).reach(v[1]).size(vec2(520.0, 250.0)));
                });
            },
        },
        Demo {
            id: "spider-button",
            name: "Spider Button",
            category: Category::Buttons,
            description: "A glossy red pill with corner cobwebs that shoots sagging web lines at the pointer, splats a web on click and snaps back when you pull away.",
            covers: &["Originkit: Spider Button"],
            code: r##"ui.add(SpiderButton::new("Spider Button").lines(4));"##,
            show: |ui| {
                let v = controls(ui, "spider", [4.0, 170.0], [("Lines", 1.0, 8.0, 0), ("Reach", 60.0, 260.0, 0)]);
                ui.add_space(8.0);
                centered_row(ui, 260.0, |ui| {
                    ui.add(SpiderButton::new("Spider Button").lines(v[0] as usize).reach(v[1]).size(vec2(520.0, 250.0)));
                });
            },
        },
        Demo {
            id: "velour-bolster",
            name: "Velour Bolster",
            category: Category::Buttons,
            description: "A velvet pill button that inflates into a soft cushion under the pointer, with grazing-angle sheen; it jiggles as you move, dents when pressed and settles flat.",
            covers: &["Originkit: Velour Bolster"],
            code: r##"ui.add(VelourBolster::new("HOVER ME").puff(1.0));"##,
            show: |ui| {
                let v = controls(ui, "velour", [1.0], [("Puff", 0.0, 2.0, 1)]);
                ui.add_space(30.0);
                centered_row(ui, 220.0, |ui| {
                    ui.add(VelourBolster::new("HOVER ME").puff(v[0]));
                    ui.add(VelourBolster::new("PLUM").color(hex("#7E2A6E")).puff(v[0]).size(vec2(170.0, 70.0)));
                    ui.add(VelourBolster::new("CLAY").color(hex("#B5532E")).puff(v[0]).size(vec2(170.0, 70.0)));
                });
            },
        },
        Demo {
            id: "single-glass-keyboard",
            name: "Single Glass Keyboard",
            category: Category::ThreeD,
            description: "A single glass keycap refracting orange, magenta and blue studio light into rainbow fringes; it floats, tilts after the pointer and presses on click.",
            covers: &["Originkit: Single Glass Keyboard"],
            code: r##"ui.add(GlassKeycap::new("esc.").size(120.0));"##,
            show: |ui| {
                let v = controls(ui, "keycap", [120.0, 1.0], [("Size", 70.0, 150.0, 0), ("Float", 0.0, 2.0, 1)]);
                ui.add_space(8.0);
                centered_row(ui, 300.0, |ui| {
                    ui.add(GlassKeycap::new("esc.").size(v[0]).float(v[1]));
                });
            },
        },
        Demo {
            id: "cobalt-sprite",
            name: "Cobalt Sprite",
            category: Category::ThreeD,
            description: "A blue sprite with pill eyes inside a thick glass capsule, clear above and frosted below; it watches the pointer, bobs and blinks.",
            covers: &["Originkit: Cobalt Sprite"],
            code: r##"ui.add(CobaltSprite::new().radius(110.0).frost(0.6));"##,
            show: |ui| {
                let v = controls(ui, "cobalt", [110.0, 0.6], [("Radius", 50.0, 140.0, 0), ("Frost", 0.0, 1.0, 2)]);
                ui.add_space(8.0);
                ui.add(CobaltSprite::new().radius(v[0]).frost(v[1]));
            },
        },
        Demo {
            id: "plush-tile",
            name: "Plush Tile",
            category: Category::MicroInteractions,
            description: "An app-icon tile covered in thousands of short combed strands that carry a mark in the pile; the fur parts under the pointer and stirs in a breeze.",
            covers: &["Originkit: Plush Tile"],
            code: r##"ui.add(PlushTile::new().mark(PlushMark::Ring).size(220.0));"##,
            show: |ui| {
                let v = controls(ui, "plush", [1.0, 1.0], [("Fur length", 0.5, 1.8, 1), ("Breeze", 0.0, 2.0, 1)]);
                ui.add_space(4.0);
                centered_row(ui, 300.0, |ui| {
                    ui.add(PlushTile::new().size(210.0).length(v[0]).breeze(v[1]));
                    ui.add(PlushTile::new().size(150.0).mark(PlushMark::Ring).base(hex("#F7F3EC")).colors([hex("#E2232E"); 4]).length(v[0]).breeze(v[1]));
                    ui.add(PlushTile::new().size(150.0).mark(PlushMark::Heart).base(hex("#2A2A31")).colors([hex("#FF5C8A"); 4]).length(v[0]).breeze(v[1]));
                });
            },
        },
        Demo {
            id: "corolla-gleam",
            name: "Corolla Gleam",
            category: Category::ThreeD,
            description: "A crown of thick iridescent glass slabs standing in a ring: it turns, breathes and leans toward the pointer while thin-film colors slide across the glass.",
            covers: &["Originkit: Corolla Gleam"],
            code: r##"ui.add(CorollaGleam::new().slabs(16).speed(1.0));"##,
            show: |ui| {
                let v = controls(ui, "corolla", [16.0, 1.0], [("Slabs", 6.0, 28.0, 0), ("Speed", 0.0, 3.0, 1)]);
                ui.add_space(8.0);
                ui.add(CorollaGleam::new().slabs(v[0] as usize).speed(v[1]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "gallery-tunnel",
            name: "Gallery Tunnel",
            category: Category::Motion,
            description: "An endless flight through a square wireframe tunnel lined with images and bold color panels; click or hold to accelerate, the pointer steers the view.",
            covers: &["Originkit: Gallery Tunnel"],
            code: r##"ui.add(GalleryTunnel::new().speed(1.0).fill(0.55));"##,
            show: |ui| {
                let v = controls(ui, "tunnel", [1.0, 0.55], [("Speed", 0.0, 3.0, 1), ("Fill", 0.0, 1.0, 2)]);
                ui.add_space(8.0);
                ui.add(GalleryTunnel::new().speed(v[0]).fill(v[1]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "spiral-images",
            name: "Spiral Images",
            category: Category::Motion,
            description: "Image cards flow along an Archimedean spiral from the edge into the center, shrinking and turning as they are drawn into the vortex; hover to slow it.",
            covers: &["Originkit: Spiral Images"],
            code: r##"ui.add(SpiralGallery::new().cards(36).turns(2.6));"##,
            show: |ui| {
                let v = controls(ui, "spiral", [56.0, 2.6, 1.0], [("Cards", 10.0, 70.0, 0), ("Turns", 1.0, 4.5, 1), ("Speed", 0.0, 3.0, 1)]);
                ui.add_space(8.0);
                ui.add(SpiralGallery::new().cards(v[0] as usize).turns(v[1]).speed(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "coverflow-gallery",
            name: "Coverflow Gallery",
            category: Category::Motion,
            description: "A 3D coverflow carousel: the active slide faces you, neighbours swing back in perspective with a floor reflection; autoplays, drag or click to move.",
            covers: &["Originkit: Coverflow Gallery"],
            code: r##"let mut index = 0usize;
ui.add(CoverflowGallery::new(&mut index).angle(0.95).autoplay(3.0));"##,
            show: |ui| {
                let v = controls(ui, "coverflow", [0.95, 3.0], [("Angle", 0.0, 1.3, 2), ("Autoplay", 0.0, 6.0, 1)]);
                ui.add_space(8.0);
                let mut index: usize = mem(ui, "cf_index", || 3);
                ui.add(CoverflowGallery::new(&mut index).angle(v[0]).autoplay(v[1]).size(vec2(ui.available_width(), 380.0)));
                set_mem(ui, "cf_index", index);
            },
        },
        Demo {
            id: "hover-image-reveal",
            name: "Hover Image Reveal",
            category: Category::CursorAndInteraction,
            description: "A list of large names; hovering a row floats an image window that follows the pointer on a spring, tilts with its speed and swaps image per row.",
            covers: &["Originkit: Hover Image Reveal"],
            code: r##"ui.add(HoverImageReveal::new(vec!["Alpenglow".into(), "Lagoon".into(), "Aurora".into()]));"##,
            show: |ui| {
                let v = controls(ui, "reveal", [34.0], [("Font size", 20.0, 48.0, 0)]);
                ui.add_space(8.0);
                let items = ["Alpenglow", "Lagoon", "Northern Aurora", "Dune Walk", "Evergreen"].iter().map(|s| s.to_string()).collect();
                ui.add(HoverImageReveal::new(items).font_size(v[0]));
            },
        },
        Demo {
            id: "cloud-sky",
            name: "Cloud Sky",
            category: Category::Backgrounds,
            description: "An animated sky: cumulus clouds drift on two parallax layers under a cirrus veil, with a low sun glow; the pointer turns the wind. Dusk palette on dark themes.",
            covers: &["Originkit: Cloud Sky"],
            code: r##"ui.add(CloudSky::new().clouds(9).cirrus(0.6));"##,
            show: |ui| {
                let v = controls(ui, "clouds", [9.0, 1.0, 0.6], [("Clouds", 1.0, 16.0, 0), ("Speed", 0.0, 4.0, 1), ("Cirrus", 0.0, 1.0, 2)]);
                ui.add_space(8.0);
                ui.add(CloudSky::new().clouds(v[0] as usize).speed(v[1]).cirrus(v[2]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "still-sunset-meadow",
            name: "Still Sunset Meadow",
            category: Category::Backgrounds,
            description: "A ground-level voxel meadow at sunset: square sun, drifting block clouds, a blocky treeline and swaying cube grass and pixel flowers with a soft out-of-focus foreground.",
            covers: &["Originkit: Still Sunset Meadow"],
            code: r##"ui.add(VoxelMeadow::new().flowers(0.35).breeze(1.0));"##,
            show: |ui| {
                let v = controls(ui, "meadow", [0.35, 1.0], [("Flowers", 0.0, 1.0, 2), ("Breeze", 0.0, 2.0, 1)]);
                ui.add_space(8.0);
                ui.add(VoxelMeadow::new().flowers(v[0]).breeze(v[1]).size(vec2(ui.available_width(), 380.0)));
            },
        },
        Demo {
            id: "glimmer-vale",
            name: "Glimmer Vale",
            category: Category::Backgrounds,
            description: "A night meadow of rolling hills made of twinkling peach-to-violet glitter under a periwinkle sky, with blooming crests and soft daisies drifting in the foreground.",
            covers: &["Originkit: Glimmer Vale"],
            code: r##"ui.add(GlimmerVale::new().glitter(2600).daisies(26));"##,
            show: |ui| {
                let v = controls(ui, "vale", [2600.0, 26.0], [("Glitter", 600.0, 4500.0, 0), ("Daisies", 0.0, 50.0, 0)]);
                ui.add_space(8.0);
                ui.add(GlimmerVale::new().glitter(v[0] as usize).daisies(v[1] as usize).size(vec2(ui.available_width(), 380.0)));
            },
        },
    ]
}
