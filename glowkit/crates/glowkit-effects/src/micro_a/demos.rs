use super::common::{cr, raised, track};
use super::*;
use egui::{pos2, vec2, Align, Align2, CursorIcon, FontId, Layout, Rect, Sense, Ui, UiBuilder};
use glowkit_core::color::mix;
use glowkit_core::demo::{Category::MicroInteractions as C, Demo};
use glowkit_core::Theme;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Run `f` inside `rect`, vertically centered using last frame's measured height.
fn centered(ui: &mut Ui, rect: Rect, key: &str, f: impl FnOnce(&mut Ui)) {
    let mid = ui.scope_id().with(("centered", key));
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

/// The block layout: the live component on the left, a column of parameter sliders on the right
/// (stacked below on narrow screens).
fn bench<P>(
    ui: &mut Ui,
    height: f32,
    p: &mut P,
    component: impl FnOnce(&mut Ui, &mut P),
    controls: impl FnOnce(&mut Ui, &mut P),
) {
    let w = ui.available_width();
    let wide = w > 620.0;
    if wide {
        let panel_w = 230.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        let left = Rect::from_min_size(rect.min, vec2(w - panel_w - 28.0, height));
        let right = Rect::from_min_max(pos2(rect.max.x - panel_w, rect.min.y), rect.max);
        centered(ui, left, "stage", |ui| component(ui, p));
        centered(ui, right, "panel", |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            controls(ui, p);
        });
    } else {
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        centered(ui, rect, "stage", |ui| component(ui, p));
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        controls(ui, p);
    }
}

/// Parameter values for a demo, persisted per demo.
fn params<const N: usize>(ui: &Ui, key: &str, init: [f32; N]) -> [f32; N] {
    mem(ui, key, || init)
}

/// A small segmented switch matching [`ControlSlider`]'s look; `value` holds the option index.
fn choice(ui: &mut Ui, label: &str, value: &mut f32, options: &[&str]) {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let font = FontId::proportional(13.0);
    ui.painter().text(
        pos2(rect.min.x + 2.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        mix(t.muted_foreground, t.foreground, 0.55),
    );
    let label_w = ui.painter().layout_no_wrap(label.into(), font.clone(), t.foreground).size().x;
    let text_w: Vec<f32> =
        options.iter().map(|o| ui.painter().layout_no_wrap((*o).into(), font.clone(), t.foreground).size().x).collect();
    let room = w - label_w - 14.0 - 6.0 - text_w.iter().sum::<f32>();
    let pad = (room / options.len() as f32).clamp(10.0, 22.0);
    let opt_w: Vec<f32> = text_w.iter().map(|x| x + pad).collect();
    let total: f32 = opt_w.iter().sum::<f32>() + 6.0;
    let seg = Rect::from_min_max(pos2(rect.max.x - total, rect.min.y), rect.max);
    let radius = seg.height() / 2.0;
    ui.painter().rect_filled(seg, cr(radius), track(&t));
    let sel = (*value as usize).min(options.len() - 1);
    let mut x = seg.min.x + 3.0;
    let mut sel_x = x;
    let mut xs = Vec::new();
    for (i, w) in opt_w.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, seg.min.y + 3.0), vec2(*w, seg.height() - 6.0));
        let resp = ui.interact(r, ui.scope_id().with(("choice", label, i)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if resp.clicked() {
            *value = i as f32;
        }
        if i == sel {
            sel_x = x;
        }
        xs.push(r);
        x += w;
    }
    let kx = glowkit_core::anim::spring(ui, ui.scope_id().with(("choice_knob", label)), sel_x, 420.0, 30.0);
    let kw = glowkit_core::anim::spring(ui, ui.scope_id().with(("choice_w", label)), opt_w[sel], 420.0, 30.0);
    let knob = Rect::from_min_size(pos2(kx, seg.min.y + 3.0), vec2(kw, seg.height() - 6.0));
    ui.painter().rect_filled(knob, cr(knob.height() / 2.0), raised(&t));
    for (i, r) in xs.iter().enumerate() {
        let c = if i == sel { t.foreground } else { t.muted_foreground };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, options[i], font.clone(), c);
    }
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "card-stack",
            name: "Card Stack",
            category: C,
            description: "A deck of photos that fans out as you reach it: hover draws a card up, click flips it into the middle.",
            covers: &["Bencho: Card stack"],
            code: r##"ui.add(PhotoFan::new().cards(5).spread(85.0).bounce(5.0).corner(14.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [5.0, 0.0, 85.0, 5.0, 14.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(PhotoFan::new().bounce(p[0]).scatter(p[1]).spread(p[2]).cards(p[3] as usize).corner(p[4]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Scatter", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Spread", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Cards", &mut p[3], 2.0..=8.0));
                        ui.add(ControlSlider::new("Corner", &mut p[4], 0.0..=32.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "ascii-wake",
            name: "ASCII Wake",
            category: C,
            description: "A cube drawn in ink dots; the pointer drags a wake of characters through it that cools back to dots, and a press sends a ring out.",
            covers: &["Bencho: ASCII wake"],
            code: r##"ui.add(AsciiWake::new().glyphs(GlyphSet::Ascii).reach(60.0).trail(1.2).pull(40.0).cell(10.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [0.0, 60.0, 1.2, 40.0, 10.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let g = [GlyphSet::Ascii, GlyphSet::Blocks, GlyphSet::Dots, GlyphSet::Binary][p[0] as usize % 4];
                        ui.add(AsciiWake::new().glyphs(g).reach(p[1]).trail(p[2]).pull(p[3]).cell(p[4]).autoplay(true));
                    },
                    |ui, p| {
                        choice(ui, "Glyphs", &mut p[0], &["#", "▒", "•", "01"]);
                        ui.add(ControlSlider::new("Reach", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Trail", &mut p[2], 0.2..=4.0).decimals(1).suffix("s"));
                        ui.add(ControlSlider::new("Pull", &mut p[3], 0.0..=100.0));
                        ui.add(ControlSlider::new("Size", &mut p[4], 6.0..=18.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "wake-slider",
            name: "Wake Slider",
            category: C,
            description: "A range made of thin bars with no thumb; drag fast and the bars you sweep past spring up in a wake behind the hand.",
            covers: &["Bencho: Wake slider"],
            code: r##"let mut value = 50.0;
ui.add(WakeSlider::new(&mut value).bars(36).wake(60.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [60.0, 36.0, 50.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let mut v = p[2];
                        ui.add(WakeSlider::new(&mut v).wake(p[0]).bars(p[1] as usize).autoplay(true));
                        p[2] = v;
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Wake", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bars", &mut p[1], 8.0..=72.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "folding-bill",
            name: "Folding Bill",
            category: C,
            description: "A banknote simulated as cloth: grab it, drag it and fold it over itself; plain or with an engraved print.",
            covers: &["Bencho: Folding bill"],
            code: r##"ui.add(FoldingBill::new().print(BillPrint::Plain).stiffness(40.0).settle(0.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [0.0, 40.0, 0.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let print = if p[0] as usize == 0 { BillPrint::Plain } else { BillPrint::Print };
                        ui.add(FoldingBill::new().print(print).stiffness(p[1]).settle(p[2]).autoplay(true));
                    },
                    |ui, p| {
                        choice(ui, "Print", &mut p[0], &["Plain", "Print"]);
                        ui.add(ControlSlider::new("Stiffness", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Settle", &mut p[2], 0.0..=100.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "heat-word",
            name: "Heat Map",
            category: C,
            description: "Gray dots that melt into a glowing heat map under your cursor; press for the next shape.",
            covers: &["Bencho: Heat map"],
            code: r##"ui.add(HeatBlobs::new().palette(HeatPalette::Neon).heat(60.0).area(50.0).softness(40.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [0.0, 60.0, 50.0, 40.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let pal = [HeatPalette::Neon, HeatPalette::Fire, HeatPalette::Sea, HeatPalette::Pop][p[0] as usize % 4];
                        ui.add(HeatBlobs::new().palette(pal).heat(p[1]).area(p[2]).softness(p[3]).autoplay(true));
                    },
                    |ui, p| {
                        choice(ui, "Colors", &mut p[0], &["Neon", "Fire", "Sea", "Pop"]);
                        ui.add(ControlSlider::new("Heat", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Area", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Softness", &mut p[3], 0.0..=100.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "generate",
            name: "Generate",
            category: C,
            description: "Press Generate and an image resolves under frosted glass, broad shapes first.",
            covers: &["Bencho: Generate"],
            code: r##"ui.add(GenerateCard::new().pace(5.0).ripple(50.0).corner(20.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [5.0, 50.0, 20.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(GenerateCard::new().pace(p[0]).ripple(p[1]).corner(p[2]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Pace", &mut p[0], 1.0..=10.0).decimals(1).suffix("s"));
                        ui.add(ControlSlider::new("Ripple", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[2], 0.0..=40.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "image-accordion",
            name: "Image Accordion",
            category: C,
            description: "Hover a photo strip and it opens wide while its neighbours give way around it.",
            covers: &["Bencho: Image accordion"],
            code: r##"ui.add(ImageAccordion::new().open(50.0).reach(35.0).bounce(10.0).gap(8.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [50.0, 35.0, 10.0, 8.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(ImageAccordion::new().open(p[0]).reach(p[1]).bounce(p[2]).gap(p[3]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Open", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Reach", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bounce", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Gap", &mut p[3], 0.0..=24.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "magnet-select",
            name: "Magnetic Select",
            category: C,
            description: "Press an option and it swells into an iridescent orb as the rest are pushed out of its way.",
            covers: &["Bencho: Magnetic select"],
            code: r##"let mut choice = 0;
ui.add(MagneticSelect::new(&mut choice).options(7).pull(55.0).bounce(55.0).give(50.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [1.0, 55.0, 55.0, 50.0, 0.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let mut sel = p[4] as usize;
                        let r = if p[0] as usize == 0 { 16.0 } else { 24.0 };
                        ui.add(MagneticSelect::new(&mut sel).radius(r).pull(p[1]).bounce(p[2]).give(p[3]).autoplay(true));
                        p[4] = sel as f32;
                    },
                    |ui, p| {
                        choice(ui, "Size", &mut p[0], &["Small", "Large"]);
                        ui.add(ControlSlider::new("Pull", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bounce", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Give", &mut p[3], 0.0..=100.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "checklist",
            name: "Checklist",
            category: C,
            description: "Tick a task and the box pops full, the check draws itself in and the words strike out.",
            covers: &["Bencho: Checklist"],
            code: r##"let mut tasks = vec![TodoItem::new("Water the ferns"), TodoItem::new("Reply to Ana")];
ui.add(TaskChecklist::new(&mut tasks).bounce(50.0).corner(20.0).box_size(18.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [50.0, 20.0, 18.0]);
                let mut tasks: Vec<TodoItem> = mem(ui, "tasks", || {
                    vec![TodoItem::new("Water the ferns"), TodoItem::new("Renew the domain"), TodoItem::new("Sketch the poster")]
                });
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(TaskChecklist::new(&mut tasks).bounce(p[0]).corner(p[1]).box_size(p[2]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[1], 0.0..=32.0).suffix("px"));
                        ui.add(ControlSlider::new("Box", &mut p[2], 12.0..=28.0).suffix("px"));
                    },
                );
                set_mem(ui, "tasks", tasks);
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "liq-toggle",
            name: "Liquid Toggle",
            category: C,
            description: "A switch whose droplet stretches as it slides across the track and snaps round at the end.",
            covers: &["Bencho: Liquid toggle"],
            code: r##"let mut on = false;
ui.add(LiquidToggle::new(&mut on).speed(50.0).stretch(36.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [50.0, 36.0, 0.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let mut on = p[2] > 0.5;
                        ui.add(LiquidToggle::new(&mut on).speed(p[0]).stretch(p[1]).autoplay(true));
                        p[2] = if on { 1.0 } else { 0.0 };
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Speed", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Stretch", &mut p[1], 0.0..=100.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "sound",
            name: "Now Playing",
            category: C,
            description: "A now-playing bar that opens into a full player and folds back as one piece.",
            covers: &["Bencho: Now playing"],
            code: r##"ui.add(NowPlaying::new("Glass Harbor", "Side A").morph(50.0).corner(16.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [50.0, 16.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(NowPlaying::new("Glass Harbor", "Side A").morph(p[0]).corner(p[1]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Morph", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[1], 0.0..=32.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "slosh",
            name: "Slosh Slider",
            category: C,
            description: "A slider whose fill has weight: drag fast and the liquid sloshes past the handle before settling.",
            covers: &["Bencho: Slosh slider"],
            code: r##"let mut level = 0.6;
ui.add(SloshSlider::new(&mut level).viscosity(15.0).momentum(55.0).tilt(45.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [15.0, 55.0, 20.0, 45.0, 0.6]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let mut v = p[4];
                        ui.add(SloshSlider::new(&mut v).viscosity(p[0]).momentum(p[1]).corner(p[2]).tilt(p[3]).autoplay(true));
                        p[4] = v;
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Viscosity", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Momentum", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[2], 0.0..=20.0).suffix("px"));
                        ui.add(ControlSlider::new("Tilt", &mut p[3], 0.0..=100.0));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "stacked-bars",
            name: "Stacked Bars",
            category: C,
            description: "A month of visits as stacked bars: run along them and one day is ringed while a tooltip glides beside it; Day / Week regrows the bars.",
            covers: &["Bencho: Stacked bars"],
            code: r##"ui.add(StackedBars::new().bars(20).gap(6.0).monochrome(false));"##,
            show: |ui| {
                let mut p = params(ui, "p", [1.0, 20.0, 6.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(StackedBars::new().monochrome(p[0] as usize == 0).bars(p[1] as usize).gap(p[2]).autoplay(true));
                    },
                    |ui, p| {
                        choice(ui, "Colour", &mut p[0], &["Mono", "Colour"]);
                        ui.add(ControlSlider::new("Bars", &mut p[1], 8.0..=32.0));
                        ui.add(ControlSlider::new("Gap", &mut p[2], 0.0..=10.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "emoji-reactions",
            name: "Emoji Reactions",
            category: C,
            description: "Press the button and the message turns into a pill of emoji; pick one and it flies down to its chip as the count rolls up.",
            covers: &["Bencho: Emoji reactions"],
            code: r##"ui.add(EmojiReactions::new("Merged the dark mode PR").set(ReactionSet::Work).magnify(50.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [0.0, 50.0, 45.0, 20.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let set = [ReactionSet::Work, ReactionSet::Classic, ReactionSet::Hearts][p[0] as usize % 3];
                        ui.add(EmojiReactions::new("Merged the dark mode PR").set(set).magnify(p[1]).bounce(p[2]).corner(p[3]).autoplay(true));
                    },
                    |ui, p| {
                        choice(ui, "Set", &mut p[0], &["Work", "Classic", "Hearts"]);
                        ui.add(ControlSlider::new("Magnify", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bounce", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[3], 0.0..=24.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "voice-note",
            name: "Voice Note",
            category: C,
            description: "Hold to record a waveform, slide left to bin it, let go to play it back.",
            covers: &["Bencho: Voice note"],
            code: r##"ui.add(VoiceNote::new().sensitivity(60.0).bounce(40.0).bars(28).corner(28.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [60.0, 40.0, 28.0, 28.0]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        ui.add(VoiceNote::new().sensitivity(p[0]).bounce(p[1]).bars(p[2] as usize).corner(p[3]).autoplay(true));
                    },
                    |ui, p| {
                        ui.add(ControlSlider::new("Sensitivity", &mut p[0], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bounce", &mut p[1], 0.0..=100.0));
                        ui.add(ControlSlider::new("Bars", &mut p[2], 12.0..=48.0));
                        ui.add(ControlSlider::new("Corner", &mut p[3], 0.0..=28.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "time-scrubber",
            name: "Time Scrubber",
            category: C,
            description: "Drag the ruler to set a time; fling it and it coasts with momentum to the nearest step.",
            covers: &["Bencho: Time scrubber"],
            code: r##"let mut minutes = 9 * 60 + 30;
ui.add(TimeScrubber::new(&mut minutes).step(15).twenty_four_hour(false).momentum(50.0));"##,
            show: |ui| {
                let mut p = params(ui, "p", [1.0, 0.0, 50.0, 20.0, (9 * 60 + 30) as f32]);
                bench(
                    ui,
                    380.0,
                    &mut p,
                    |ui, p| {
                        let mut m = p[4] as u32;
                        let step = [5, 15, 30][p[0] as usize % 3];
                        ui.add(TimeScrubber::new(&mut m).step(step).twenty_four_hour(p[1] as usize == 1).momentum(p[2]).corner(p[3]).autoplay(true));
                        p[4] = m as f32;
                    },
                    |ui, p| {
                        choice(ui, "Step", &mut p[0], &["5", "15", "30"]);
                        choice(ui, "Format", &mut p[1], &["12h", "24h"]);
                        ui.add(ControlSlider::new("Momentum", &mut p[2], 0.0..=100.0));
                        ui.add(ControlSlider::new("Corner", &mut p[3], 0.0..=32.0).suffix("px"));
                    },
                );
                set_mem(ui, "p", p);
            },
        },
    ]
}
