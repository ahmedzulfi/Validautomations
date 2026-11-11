use crate::*;
use egui::{vec2, Color32, Ui};
use glowkit_core::color::hex;
use glowkit_core::demo::{Category::ThreeD as C, Demo};
use glowkit_core::Theme;

fn chips<T: Clone + PartialEq>(ui: &mut Ui, items: &[T], current: &mut T, name: impl Fn(&T) -> &'static str) {
    for it in items {
        if ui.selectable_label(*current == *it, name(it)).clicked() {
            *current = it.clone();
        }
    }
}

fn hint(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).size(11.5).color(t.muted_foreground)));
}

fn showcase(ui: &mut Ui) {
    Viewport3D::new("showcase")
        .height(360.0)
        .controls(OrbitControls::new().distance(7.6).pitch(0.38).yaw(0.4).auto_rotate(0.15).distance_range(4.0, 20.0))
        .show(ui, |scene, ctx| {
            let t = &ctx.theme;
            let g = t.gradient;
            let items: [(&str, Material); 8] = [
                ("Smooth", Material::smooth(t.chart[0]).specular(0.6).rim(0.3)),
                ("Flat", Material::flat(t.chart[1])),
                ("Toon", Material::toon(t.chart[2], 3)),
                ("Matcap", Material::matcap([glowkit_core::color::shade(g[0], -0.25), g[1], glowkit_core::color::mix(g[2], Color32::WHITE, 0.4)])),
                ("Normal", Material::normal()),
                ("Wireframe", Material::wireframe(t.chart[5]).wire(Wire::new(t.chart[5]).width(1.2).glow(if t.dark { 1.0 } else { 0.0 }))),
                ("Glow", Material::smooth(glowkit_core::color::shade(t.chart[3], if t.dark { -0.35 } else { -0.05 })).emissive(0.25).rim(1.6).rim_color(t.chart[3]).specular(0.2)),
                ("Overlay", Material::smooth(t.chart[4]).wire(Wire::new(Color32::from_white_alpha(140)).width(1.0))),
            ];
            let meshes = [
                ctx.mesh("a", || Mesh::torus_knot(2, 3, 0.75, 0.2, 120, 10)),
                ctx.mesh("b", || Mesh::dodecahedron(0.8)),
                ctx.mesh("c", || Mesh::icosphere(0.75, 2)),
                ctx.mesh("d", || Mesh::torus(0.55, 0.25, 36, 14)),
                ctx.mesh("e", || Mesh::capsule(0.35, 0.7, 24, 6)),
                ctx.mesh("f", || Mesh::icosphere(0.75, 1)),
                ctx.mesh("g", || Mesh::uv_sphere(0.7, 28, 14)),
                ctx.mesh("h", || Mesh::cylinder(0.45, 0.6, 1.1, 24)),
            ];
            for (i, ((name, mat), mesh)) in items.iter().zip(meshes.iter()).enumerate() {
                let (col, row) = (i % 4, i / 4);
                let p = vec3((col as f32 - 1.5) * 2.4, 0.0, (row as f32 - 0.5) * 2.5);
                let tf = Transform::from_translation(p + vec3(0.0, (ctx.time * 1.2 + i as f32).sin() * 0.1, 0.0)).with_euler(ctx.time * 0.4 + i as f32, ctx.time * 0.6, 0.0);
                scene.shadow(p - vec3(0.0, 0.95, 0.0), 0.8, glowkit_core::color::with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.15 }));
                if *name == "Glow" {
                    scene.underlay(|s| s.glow_point(p, 1.3, if t.dark { additive(t.chart[3].gamma_multiply(0.35)) } else { glowkit_core::color::with_alpha(t.chart[3], 0.25) }));
                }
                scene.draw(mesh, tf, mat);
                scene.text(name, p - vec3(0.0, 0.95, -0.95), Vec3::X, -Vec3::Z, 0.26, t.muted_foreground, egui::Align2::CENTER_CENTER);
            }
            scene.grid(-0.96, 12.0, 12, glowkit_core::color::with_alpha(t.muted_foreground, 0.2));
        });
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "viewport-3d",
            name: "3D Viewport",
            category: C,
            description: "The software 3D engine: meshes, materials (smooth, flat, toon, matcap, normal, wireframe, glow), lights, shadows, grid and 3D text, depth-sorted into egui meshes. Drag to orbit.",
            covers: &["three.js: Scene + MeshStandardMaterial", "react-three-fiber: Canvas", "drei: OrbitControls", "drei: ContactShadows", "drei: Text"],
            code: r##"Viewport3D::new("scene")
    .height(360.0)
    .controls(OrbitControls::new().distance(9.0).auto_rotate(0.2))
    .show(ui, |scene, ctx| {
        let knot = ctx.mesh("knot", || Mesh::torus_knot(2, 3, 0.8, 0.2, 120, 10));
        let mat = Material::smooth(ctx.theme.accent).specular(0.6).rim(0.3);
        scene.shadow(vec3(0.0, -1.0, 0.0), 0.8, Color32::from_black_alpha(120));
        scene.draw(&knot, Transform::IDENTITY.with_euler(0.0, ctx.time, 0.0), &mat);
        scene.text("Hello", vec3(0.0, -1.0, 1.0), Vec3::X, -Vec3::Z, 0.3, ctx.theme.foreground, Align2::CENTER_CENTER);
    });"##,
            show: showcase,
        },
        Demo {
            id: "model-viewer",
            name: "Model Viewer",
            category: C,
            description: "A lit 3D model floating over a soft shadow. Drag to orbit with inertia, pinch or ctrl+scroll to zoom; switch models, materials and the wireframe overlay.",
            covers: &["React Bits: Model Viewer", "drei: Stage", "model-viewer (Google)"],
            code: r##"ui.add(ModelViewer::new(Model::Rocket).look(Look::Studio).wireframe(false).height(380.0));

// your own model (OBJ with v / vn / f):
let mesh = Mesh::from_obj(include_str!("model.obj"))?;
ui.add(ModelViewer::new(Model::Custom(Arc::new(mesh))));"##,
            show: |ui| {
                let id = ui.scope_id().with("mv");
                let (mut model, mut look, mut wire): (usize, Look, bool) = glowkit_core::state(ui, id, || (1usize, Look::Studio, false));
                let builtins = [Model::TorusKnot, Model::Rocket, Model::Gem, Model::Mug, Model::Tree, Model::Dodecahedron, Model::Torus];
                ui.horizontal_wrapped(|ui| {
                    let idx: Vec<usize> = (0..builtins.len()).collect();
                    chips(ui, &idx, &mut model, |i| builtins[*i].name());
                    ui.separator();
                    chips(ui, &Look::ALL, &mut look, |l| l.name());
                    ui.separator();
                    ui.checkbox(&mut wire, "Wireframe");
                });
                glowkit_core::set_state(ui, id, (model, look, wire));
                ui.add_space(4.0);
                ui.add(ModelViewer::new(builtins[model].clone()).look(look).wireframe(wire).height(370.0));
            },
        },
        Demo {
            id: "particle-morph",
            name: "Particle Morph",
            category: C,
            description: "Thousands of glowing points morph between a sphere, torus, wave, cube, helix and galaxy with a staggered, eased transition. The pointer pushes particles away; drag to orbit.",
            covers: &["three.js: Points morph hero", "React Bits: Particles", "Aceternity: Sparkles (3D)"],
            code: r##"ui.add(ParticleMorph::new().count(1600).timing(2.2, 1.8).height(380.0));
ui.add(ParticleMorph::new().shapes(&[CloudShape::Sphere, CloudShape::Torus]));"##,
            show: |ui| {
                ui.add(ParticleMorph::new().count(1500).height(380.0));
            },
        },
        Demo {
            id: "wave-terrain",
            name: "Wave Terrain",
            category: C,
            description: "An endless noise landscape the camera flies over: synthwave neon grid with a striped sun, a glowing wireframe, or flat-shaded low-poly hills.",
            covers: &["three.js: Synthwave terrain", "Aceternity: Background grid (3D)", "Spline: Terrain hero"],
            code: r##"ui.add(WaveTerrain::new().style(TerrainStyle::Synthwave).height(320.0));
ui.add(WaveTerrain::new().style(TerrainStyle::Wireframe));
ui.add(WaveTerrain::new().style(TerrainStyle::Solid).amplitude(1.3));"##,
            show: |ui| {
                ui.add(WaveTerrain::new().style(TerrainStyle::Synthwave).height(300.0));
                ui.add_space(8.0);
                ui.columns(2, |c| {
                    c[0].add(WaveTerrain::new().style(TerrainStyle::Wireframe).resolution(30, 24).height(200.0).id_salt("wire"));
                    c[1].add(WaveTerrain::new().style(TerrainStyle::Solid).resolution(28, 22).amplitude(1.2).height(200.0).id_salt("solid"));
                });
            },
        },
        Demo {
            id: "tilt-cubes",
            name: "Tilt Cubes",
            category: C,
            description: "A grid of 3D cubes that tilt to face the pointer; click to send a ripple through the grid. Wanders and ripples on its own when idle.",
            covers: &["React Bits: Cubes"],
            code: r##"ui.add(TiltCubes::new().grid(10, 6).max_tilt(50.0).height(340.0));"##,
            show: |ui| {
                ui.add(TiltCubes::new().grid(12, 6).height(340.0));
                hint(ui, "Move the pointer over the cubes, click for a ripple");
            },
        },
        Demo {
            id: "card-carousel-3d",
            name: "3D Card Carousel",
            category: C,
            description: "A ring of gradient cards spinning in 3D with perspective text and a glossy floor reflection. Drag sideways to spin it with inertia.",
            covers: &["React Bits: Circular Gallery", "React Bits: Dome Gallery", "Swiper: Coverflow", "Aceternity: 3D carousel"],
            code: r##"ui.add(CardCarousel3D::new(Card3D::samples()).speed(0.3).height(360.0));"##,
            show: |ui| {
                ui.add(CardCarousel3D::new(Card3D::samples()).height(370.0));
            },
        },
        Demo {
            id: "flip-card-3d",
            name: "3D Flip Card",
            category: C,
            description: "Cards that flip over in 3D with a springy overshoot when clicked and tilt towards the pointer while hovered.",
            covers: &["Aceternity: 3D Card Effect", "React Bits: Tilted Card", "CSS: flip card"],
            code: r##"let front = Card3D::new("Front", "Click to flip", [hex("#8B5CF6"), hex("#22D3EE")]);
let back = Card3D::new("Back", "Hello there", [hex("#F97316"), hex("#EC4899")]);
ui.add(FlipCard3D::new(front, back).size(vec2(220.0, 300.0)));"##,
            show: |ui| {
                let cards = Card3D::samples();
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let w = 230.0;
                    let total = w * 3.0 + 24.0 * 2.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 24.0;
                    for i in 0..3 {
                        let front = Card3D { subtitle: "Click to flip".into(), ..cards[i * 2].clone() };
                        let back = Card3D { subtitle: "Hi from the back".into(), ..cards[i * 2 + 1].clone() };
                        ui.add(FlipCard3D::new(front, back).size(vec2(w, 310.0)).auto_flip(4.2, i as f32 * 1.4 + 1.0).id_salt(format!("flip{i}")));
                    }
                });
            },
        },
        Demo {
            id: "globe-3d",
            name: "Globe 3D",
            category: C,
            description: "A true-3D lit globe: continents as thousands of dots, glowing arcs with comets between cities, pulsing markers, a graticule and an atmosphere halo. Drag to spin.",
            covers: &["Aceternity: GitHub Globe", "Magic UI: Globe", "Stripe: globe hero", "react-globe.gl"],
            code: r##"ui.add(Globe3D::new().arcs(GlobeArc::samples()).height(380.0));
ui.add(Globe3D::new().arcs(vec![GlobeArc::new((51.5, -0.1), (40.7, -74.0))]));"##,
            show: |ui| {
                ui.add(Globe3D::new().arcs(GlobeArc::samples()).height(390.0));
            },
        },
        Demo {
            id: "galaxy",
            name: "Galaxy",
            category: C,
            description: "A spiral galaxy of thousands of twinkling stars with differential rotation, nebula dust and a glowing core; the camera swoops through it on a cinematic path.",
            covers: &["three.js: Galaxy generator", "React Bits: Galaxy", "Spline: space hero"],
            code: r##"ui.add(Galaxy::new().arms(3).stars(2600).height(380.0));"##,
            show: |ui| {
                ui.add(Galaxy::new().height(380.0));
            },
        },
        Demo {
            id: "starfield-3d",
            name: "Warp Starfield",
            category: C,
            description: "Fly forward through a true-3D field of streaking stars. Hover to steer, press and hold to jump to warp speed.",
            covers: &["React Bits: Hyperspeed", "three.js: Starfield warp", "Aceternity: Shooting stars"],
            code: r##"ui.add(Starfield::new().count(900).speed(1.0).height(340.0));"##,
            show: |ui| {
                ui.add(Starfield::new().height(350.0));
                hint(ui, "Hover to steer, hold the mouse button for warp speed");
            },
        },
        Demo {
            id: "floating-shapes",
            name: "Floating Shapes Hero",
            category: C,
            description: "Glossy gradient primitives float and spin in front of and behind a 3D headline, with pointer parallax: a Spline-style hero section.",
            covers: &["Spline: 3D hero", "Magic UI: Hero (3D shapes)", "Aceternity: Floating shapes"],
            code: r##"ui.add(FloatingShapes::new("Build in 3D", "Pure Rust. No GPU pipeline. Just egui.").height(380.0));"##,
            show: |ui| {
                ui.add(FloatingShapes::new("Build in 3D", "Pure Rust. No GPU pipeline. Just egui.").height(390.0));
            },
        },
        Demo {
            id: "glow-knot",
            name: "Glow Knot",
            category: C,
            description: "A torus knot as glowing neon: a bright core line wrapped in layered bloom, light pulses racing along it and a faint wireframe tube. Drag to orbit.",
            covers: &["three.js: Bloom torus knot", "React Bits: Neon shapes"],
            code: r##"ui.add(GlowKnot::new().knot(2, 3).height(360.0));"##,
            show: |ui| {
                ui.columns(2, |c| {
                    c[0].add(GlowKnot::new().height(360.0).id_salt("k1"));
                    c[1].add(GlowKnot::new().knot(3, 5).colors(&[hex("#22D3EE"), hex("#A855F7"), hex("#F472B6")]).wireframe(false).height(360.0).id_salt("k2"));
                });
            },
        },
        Demo {
            id: "voxel-text",
            name: "Voxel Text",
            category: C,
            description: "Text built from glossy 3D voxel cubes (a tiny pixel font) rippling in a wave, swaying, and periodically bursting apart and snapping back together.",
            covers: &["three.js: TextGeometry", "Spline: 3D text", "React Bits: Pixel text"],
            code: r##"ui.add(VoxelText::new("GLOWKIT").height(300.0));"##,
            show: |ui| {
                ui.add(VoxelText::new("GLOWKIT").height(340.0));
            },
        },
        Demo {
            id: "bar-chart-3d",
            name: "3D Bar Chart",
            category: C,
            description: "Data columns in a 3D grid that grow in with a staggered spring. Hover a bar to read it, drag to orbit, double-click to replay.",
            covers: &["ECharts: Bar3D", "Plotly: 3D bars", "react-three-fiber: data viz"],
            code: r##"let data = vec![vec![3.0, 5.0, 2.0, 6.0, 4.0], vec![4.0, 2.0, 5.0, 3.0, 6.0]];
ui.add(BarChart3D::new(data)
    .columns(&["Mon", "Tue", "Wed", "Thu", "Fri"])
    .rows(&["Team A", "Team B"]));"##,
            show: |ui| {
                let data = vec![
                    vec![3.0, 5.0, 4.2, 6.5, 5.1, 7.4],
                    vec![2.2, 3.4, 5.6, 4.1, 6.8, 5.9],
                    vec![1.6, 2.8, 3.1, 5.2, 4.4, 6.2],
                    vec![1.0, 1.9, 2.6, 3.3, 4.9, 4.1],
                ];
                ui.add(BarChart3D::new(data).columns(&["Jan", "Feb", "Mar", "Apr", "May", "Jun"]).rows(&["Web", "iOS", "Android", "Desktop"]).height(390.0));
            },
        },
        Demo {
            id: "dna-helix",
            name: "DNA Helix",
            category: C,
            description: "A rotating double helix of glossy spheres joined by colored base-pair rungs, fading into depth. Drag to orbit.",
            covers: &["three.js: DNA helix", "Spline: science hero"],
            code: r##"ui.add(Helix::new().pairs(26).turns(2.5).speed(0.7).height(320.0));"##,
            show: |ui| {
                ui.add(Helix::new().height(340.0));
            },
        },
        Demo {
            id: "puzzle-cube",
            name: "Puzzle Cube",
            category: C,
            description: "A 3x3x3 twisty puzzle cube that scrambles itself with smooth eased face turns. Drag to orbit.",
            covers: &["three.js: Rubik's cube", "Spline: cube hero"],
            code: r##"ui.add(PuzzleCube::new().turn_time(0.45).height(360.0));"##,
            show: |ui| {
                ui.add(PuzzleCube::new().height(370.0));
            },
        },
        Demo {
            id: "marquee-3d",
            name: "3D Marquee",
            category: C,
            description: "A tilted perspective wall of cards whose columns scroll in alternating directions and fade out towards the edges.",
            covers: &["Aceternity: 3D Marquee", "Magic UI: Marquee (3D)"],
            code: r##"ui.add(Marquee3D::new(Card3D::samples()).columns(5).speed(0.35).height(360.0));"##,
            show: |ui| {
                ui.add(Marquee3D::new(Card3D::samples()).columns(6).height(380.0));
            },
        },
        Demo {
            id: "solar-system",
            name: "Solar System",
            category: C,
            description: "A glowing sun with planets lit from the sun (real day and night sides) on orbit rings, a ringed giant and an orbiting moon. Drag to orbit, hover planets for names.",
            covers: &["three.js: Solar system", "Spline: planets hero"],
            code: r##"ui.add(SolarSystem::new(Planet::samples()).labels(true).height(380.0));"##,
            show: |ui| {
                ui.add(SolarSystem::new(Planet::samples()).height(390.0));
            },
        },
    ]
}
