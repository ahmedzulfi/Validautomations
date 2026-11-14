use super::*;
use egui::{vec2, Ui};
use glowkit_core::demo::{Category, Demo};
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// A row of live parameter sliders under a preview.
fn controls<const N: usize>(
    ui: &mut Ui,
    key: &str,
    init: [f32; N],
    add: impl FnOnce(&mut Ui, &mut [f32; N]),
) -> [f32; N] {
    let mut v: [f32; N] = mem(ui, key, || init);
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        add(ui, &mut v);
    });
    set_mem(ui, key, v);
    v
}

#[derive(Clone, Copy, Default)]
struct AutoScroll {
    off: f32,
    dir: f32,
    wait: f32,
    max: f32,
    idle: f32,
}

/// A vertical `ScrollArea` that glides down and back up on its own while the pointer is away,
/// so scroll-driven components animate in screenshots.
fn auto_scroll(ui: &mut Ui, key: &str, height: f32, speed: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.scope_id().with(("snap_auto_scroll", key));
    let dt = ui.input(|i| i.unstable_dt).clamp(0.0, 0.1);
    let mut st: AutoScroll =
        glowkit_core::state(ui, id, || AutoScroll { idle: 1.0, dir: 1.0, wait: 0.6, ..Default::default() });
    let mut sa = egui::ScrollArea::vertical()
        .id_salt(("snap_auto_scroll_area", key))
        .max_height(height)
        .min_scrolled_height(height)
        .auto_shrink([false, false]);
    if st.idle > 0.8 && st.max > 0.0 {
        if st.wait > 0.0 {
            st.wait -= dt;
        } else {
            let edge = (st.off.min(st.max - st.off) / 60.0).clamp(0.25, 1.0);
            st.off += st.dir * speed * edge * dt;
            if st.off >= st.max {
                st.off = st.max;
                st.dir = -1.0;
                st.wait = 1.2;
            } else if st.off <= 0.0 {
                st.off = 0.0;
                st.dir = 1.0;
                st.wait = 1.0;
            }
        }
        sa = sa.vertical_scroll_offset(st.off);
    }
    let out = sa.show(ui, add);
    let hovered = ui.rect_contains_pointer(out.inner_rect);
    st.max = (out.content_size.y - out.inner_rect.height()).max(0.0);
    if hovered {
        st.idle = 0.0;
        st.off = out.state.clamped_offset().y;
    } else {
        st.idle += dt;
    }
    glowkit_core::set_state(ui, id, st);
    glowkit_core::animate(ui);
}

fn scroll_hint(ui: &mut Ui, text: &str, h: f32) {
    let t = glowkit_core::Theme::of(ui);
    ui.add_space(h * 0.35);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(text).size(13.0).color(t.muted_foreground));
        ui.label(egui::RichText::new("↓").size(18.0).color(t.muted_foreground));
    });
    ui.add_space(h * 0.35);
}

fn slider_w(ui: &Ui, n: usize) -> f32 {
    ((ui.available_width() - 10.0 * (n as f32 - 1.0)) / n as f32).max(80.0)
}

pub(super) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "snap-crumple-reveal",
            name: "Crumple Reveal",
            category: Category::CardsAndBorders,
            description: "Balls of screwed-up paper on a desk; click one and it smooths out into the letter or photo it was, still faintly creased.",
            covers: &["Snapblocks: Crumple Reveal"],
            code: r##"ui.add(CrumpleReveal::new(vec![
    CrumpleItem::photo(1, "lake, june"),
    CrumpleItem::note(&["Back at six.", "Feed the cat!"], "J."),
]).creases(1.0));"##,
            show: |ui| {
                let v = mem(ui, "crv", || [1.0f32, 1.0]);
                ui.add(CrumpleReveal::default().creases(v[0]).speed(v[1]).size(vec2(ui.available_width(), 380.0)));
                controls(ui, "crv", [1.0, 1.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Creases", &mut v[0], 0.0..=2.0).decimals(1).width(w));
                    ui.add(ControlSlider::new("Speed", &mut v[1], 0.4..=2.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "snap-crumple-carousel",
            name: "Crumple Carousel",
            category: Category::ThreeD,
            description: "Posters in a row where only the focused one lies flat; neighbours crinkle and the outer ones are balled up, smoothing out as they reach the middle.",
            covers: &["Snapblocks: Crumple Carousel"],
            code: r##"ui.add(CrumpleCarousel::new().count(8).falloff(1.0));"##,
            show: |ui| {
                let v = mem(ui, "ccv", || [1.0f32, 1.0]);
                ui.add(CrumpleCarousel::new().falloff(v[0]).spread(v[1]).size(vec2(ui.available_width(), 380.0)));
                controls(ui, "ccv", [1.0, 1.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Falloff", &mut v[0], 0.5..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Spread", &mut v[1], 0.7..=1.5).decimals(2).width(w));
                });
            },
        },
        Demo {
            id: "snap-tear-ticket",
            name: "Tear Ticket",
            category: Category::MicroInteractions,
            description: "A train ticket whose stub tears off like paper: the perforation gives hole by hole, the corner peels, and a freed stub flutters away before a new one prints.",
            covers: &["Snapblocks: Tear Ticket"],
            code: r##"if ui.add(TearTicket::new("Porto", "Lyon").berth("42")).changed() {
    // stub torn off
}"##,
            show: |ui| {
                let v = mem(ui, "ttv", || [1.0f32, 1.0]);
                ui.add(TearTicket::new("Porto", "Lyon").resistance(v[0]).scale(v[1]).size(vec2(ui.available_width(), 320.0)));
                controls(ui, "ttv", [1.0, 1.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Resistance", &mut v[0], 0.4..=2.0).decimals(1).width(w));
                    ui.add(ControlSlider::new("Scale", &mut v[1], 0.8..=1.3).decimals(2).width(w));
                });
            },
        },
        Demo {
            id: "snap-receipt-printer",
            name: "Receipt Printer",
            category: Category::Blocks,
            description: "A pricing section where a thermal printer prints the price: switching plans rewinds and reprints the receipt, the sea changes hour, and you can tear it off.",
            covers: &["Snapblocks: Receipt Printer"],
            code: r##"ui.add(ReceiptPrinter::new().headline("Bill the hours.", "Keep the change."));"##,
            show: |ui| {
                let v = mem(ui, "rpv", || [1.0f32]);
                ui.add(ReceiptPrinter::new().feed_speed(v[0]).size(vec2(ui.available_width(), 430.0)));
                controls(ui, "rpv", [1.0], |ui, v| {
                    let w = slider_w(ui, 1);
                    ui.add(ControlSlider::new("Feed speed", &mut v[0], 0.4..=2.5).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "snap-pour",
            name: "Pour",
            category: Category::Inputs,
            description: "A filter bar of liquid ink: tap a tag and the count pill pours into it, floods and swallows it while the count rolls; tap again and it drains, necks and snaps free.",
            covers: &["Snapblocks: Pour"],
            code: r##"ui.add(PourFilter::new(&["Sauna", "Lakeside", "Wood stove", "Dogs OK"], &mut selected)
    .total(86).noun("cabins"));"##,
            show: |ui| {
                let mut sel: Vec<bool> = mem(ui, "pour_sel", Vec::new);
                let v = mem(ui, "pourv", || [1.0f32]);
                ui.add(PourFilter::new(&["Sauna", "Lakeside", "Wood stove", "Dogs OK", "Off-grid", "Hot tub"], &mut sel).total(86).noun("cabins").viscosity(v[0]).height(200.0));
                set_mem(ui, "pour_sel", sel);
                controls(ui, "pourv", [1.0], |ui, v| {
                    let w = slider_w(ui, 1);
                    ui.add(ControlSlider::new("Viscosity", &mut v[0], 0.4..=2.0).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "snap-tuner",
            name: "Tuner",
            category: Category::Inputs,
            description: "A ruler picker as an FM dial: fling the tick tape and it coasts onto a channel while digits roll in the needle, and preset stations lock in with a name.",
            covers: &["Snapblocks: Tuner"],
            code: r##"ui.add(TunerDial::new(&mut freq).station(91.2, "Harbour Jazz").station(99.9, "Night Owl"));"##,
            show: |ui| {
                let mut f: f32 = mem(ui, "tuner_f", || 88.6);
                let v = mem(ui, "tunv", || [8.0f32, 1.0]);
                ui.add(TunerDial::new(&mut f).spacing(v[0]).friction(v[1]).height(260.0));
                set_mem(ui, "tuner_f", f);
                controls(ui, "tunv", [8.0, 1.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Tick spacing", &mut v[0], 5.0..=14.0).suffix("px").width(w));
                    ui.add(ControlSlider::new("Friction", &mut v[1], 0.3..=3.0).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "snap-turtle-password",
            name: "Turtle Password",
            category: Category::Inputs,
            description: "A password field watched by a clay tortoise: every rule you meet builds its shell up, from a rising dome to knobs, rim spikes and gold studs.",
            covers: &["Snapblocks: Turtle Password"],
            code: r##"ui.add(TurtlePassword::new(&mut password).name("Shelby"));"##,
            show: |ui| {
                let mut pw: String = mem(ui, "turtle_pw", String::new);
                let v = mem(ui, "turv", || [1.0f32]);
                ui.add(TurtlePassword::new(&mut pw).bounce(v[0]));
                set_mem(ui, "turtle_pw", pw);
                controls(ui, "turv", [1.0], |ui, v| {
                    let w = slider_w(ui, 1);
                    ui.add(ControlSlider::new("Shell bounce", &mut v[0], 0.3..=2.0).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "snap-cookie-frog",
            name: "Cookie Frog",
            category: Category::Overlays,
            description: "A cookie banner with a clay frog: switch a category on and its cookie pops out, the tongue snaps it from the air, and a full set earns a smile and a crown.",
            covers: &["Snapblocks: Cookie Frog"],
            code: r##"ui.add(CookieFrog::new(&mut prefs).name("Pip"));"##,
            show: |ui| {
                let mut prefs: Vec<bool> = mem(ui, "frog_prefs", Vec::new);
                let v = mem(ui, "frogv", || [1.0f32]);
                ui.add(CookieFrog::new(&mut prefs).tongue_speed(v[0]));
                set_mem(ui, "frog_prefs", prefs);
                controls(ui, "frogv", [1.0], |ui, v| {
                    let w = slider_w(ui, 1);
                    ui.add(ControlSlider::new("Tongue speed", &mut v[0], 0.5..=2.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "snap-clothes-line",
            name: "Clothes Line",
            category: Category::Motion,
            description: "Pictures pegged to a sagging rope that drifts past; every card is a pendulum, so they swing and settle as the line speeds up or stops.",
            covers: &["Snapblocks: Clothes Line"],
            code: r##"ui.add(ClothesLine::new().clip(LineClip::Peg).speed(46.0));"##,
            show: |ui| {
                let v = mem(ui, "clv", || [46.0f32, 1.0, 0.0]);
                let clip = [LineClip::Peg, LineClip::Binder, LineClip::Tape][(v[2].round() as usize).min(2)];
                ui.add(ClothesLine::new().speed(v[0]).swing(v[1]).clip(clip).size(vec2(ui.available_width(), 270.0)));
                controls(ui, "clv", [46.0, 1.0, 0.0], |ui, v| {
                    let w = slider_w(ui, 3);
                    ui.add(ControlSlider::new("Speed", &mut v[0], 0.0..=160.0).width(w));
                    ui.add(ControlSlider::new("Swing", &mut v[1], 0.0..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Clip (peg/binder/tape)", &mut v[2], 0.0..=2.0).width(w));
                });
            },
        },
        Demo {
            id: "snap-unfurl",
            name: "Unfurl",
            category: Category::Buttons,
            description: "A parchment button that levitates, tips into a rolled scroll and drops its bottom roller; the charter inks itself in line by line and a wax seal stamps it shut.",
            covers: &["Snapblocks: Unfurl"],
            code: r##"ui.add(UnfurlScroll::new("Unfurl").title("The Charter of", "Small Delights"));"##,
            show: |ui| {
                let v = mem(ui, "unfv", || [2.8f32, 190.0]);
                ui.add(UnfurlScroll::new("Unfurl").duration(v[0]).paper_width(v[1]).height(440.0));
                controls(ui, "unfv", [2.8, 190.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Duration", &mut v[0], 1.0..=5.0).decimals(1).suffix("s").width(w));
                    ui.add(ControlSlider::new("Paper width", &mut v[1], 150.0..=260.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "snap-pleated-strip",
            name: "Pleated Strip",
            category: Category::ThreeD,
            description: "An endless column of posters joined by paper pleats and rolled round a drum; scrolling fast throws the folds open, at rest they close and the nearest poster centres.",
            covers: &["Snapblocks: Pleated Strip"],
            code: r##"ui.add(PleatedStrip::new().count(12).pleat(22.0));"##,
            show: |ui| {
                let v = mem(ui, "plv", || [22.0f32, 180.0]);
                ui.add(PleatedStrip::new().pleat(v[0]).radius(v[1]).height(420.0));
                controls(ui, "plv", [22.0, 180.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Pleat depth", &mut v[0], 6.0..=50.0).suffix("px").width(w));
                    ui.add(ControlSlider::new("Drum radius", &mut v[1], 120.0..=320.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "snap-elastic-frame",
            name: "Elastic Frame",
            category: Category::CardsAndBorders,
            description: "A rounded frame drawn as an elastic string: bring the pointer to an edge and it plucks outward into a glowing veil, then springs back with a wobble.",
            covers: &["Snapblocks: Elastic Frame"],
            code: r##"ElasticFrame::new().size(260.0).show(ui, |ui| {
    ui.label("Anything goes in here");
});"##,
            show: |ui| {
                let v = mem(ui, "efv", || [1.0f32, 90.0, 48.0]);
                ui.add(ElasticFrame::new().tension(v[0]).reach(v[1]).corner(v[2]).height(400.0));
                controls(ui, "efv", [1.0, 90.0, 48.0], |ui, v| {
                    let w = slider_w(ui, 3);
                    ui.add(ControlSlider::new("Tension", &mut v[0], 0.3..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Reach", &mut v[1], 30.0..=160.0).suffix("px").width(w));
                    ui.add(ControlSlider::new("Corner", &mut v[2], 0.0..=110.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "snap-fluted-glass",
            name: "Fluted Glass",
            category: Category::Backgrounds,
            description: "A picture behind reeded glass: each rib is its own lens holding a squeezed slice of the image, smeared into spikes, bending harder near the pointer.",
            covers: &["Snapblocks: Fluted Glass"],
            code: r##"ui.add(FlutedGlass::new().ribs(30).strength(1.0).caption("Every edge, gently bent."));"##,
            show: |ui| {
                let v = mem(ui, "fgv", || [30.0f32, 1.0, 5.0]);
                ui.add(FlutedGlass::new().ribs(v[0] as usize).strength(v[1]).scene(v[2] as usize).caption("Every edge, gently bent.").size(vec2(ui.available_width(), 380.0)));
                controls(ui, "fgv", [30.0, 1.0, 5.0], |ui, v| {
                    let w = slider_w(ui, 3);
                    ui.add(ControlSlider::new("Ribs", &mut v[0], 8.0..=80.0).width(w));
                    ui.add(ControlSlider::new("Strength", &mut v[1], 0.0..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Scene", &mut v[2], 0.0..=7.0).width(w));
                });
            },
        },
        Demo {
            id: "snap-meniscus",
            name: "Meniscus",
            category: Category::MicroInteractions,
            description: "A drop of liquid glass that pulls free of its dock and melts into whatever it touches, refracting and colour-fringing the pool beneath; tap to frost it.",
            covers: &["Snapblocks: Meniscus"],
            code: r##"ui.add(Meniscus::new("Lido, four o'clock", "Pool floor, lane four, clear glass"));"##,
            show: |ui| {
                let v = mem(ui, "mnv", || [1.0f32, 1.0]);
                ui.add(Meniscus::new("Lido, four o'clock", "Pool floor, lane four, clear glass").melt(v[0]).refraction(v[1]).size(vec2(ui.available_width(), 400.0)));
                controls(ui, "mnv", [1.0, 1.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Melt", &mut v[0], 0.3..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Refraction", &mut v[1], 0.0..=2.5).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "snap-ink-reveal",
            name: "Ink Reveal",
            category: Category::TextAnimations,
            description: "Every word starts faint and inks in as the scroll reaches it, with a soft leading edge; accented words light up with a highlighter swept behind them.",
            covers: &["Snapblocks: Ink Reveal"],
            code: r##"ScrollArea::vertical().show(ui, |ui| {
    ui.add(InkReveal::new("Most interfaces sit perfectly still. This one *waits* for you."));
});"##,
            show: |ui| {
                let v = mem(ui, "irv", || [0.12f32, 26.0]);
                auto_scroll(ui, "ink_reveal", 380.0, 70.0, |ui| {
                    scroll_hint(ui, "Scroll to ink the words in", 380.0);
                    ui.horizontal(|ui| {
                        let w = ui.available_width().min(560.0);
                        ui.add_space((ui.available_width() - w) * 0.5);
                        ui.add(InkReveal::new("Most interfaces sit perfectly still. This one waits for you. Every word lights as it passes, so attention moves at the speed of your *scroll* and the page finally feels *alive.*")
                            .softness(v[0])
                            .font(egui::FontId::proportional(v[1]))
                            .width(w));
                    });
                    ui.add_space(170.0);
                });
                controls(ui, "irv", [0.12, 26.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Softness", &mut v[0], 0.02..=0.4).decimals(2).width(w));
                    ui.add(ControlSlider::new("Size", &mut v[1], 16.0..=40.0).suffix("px").width(w));
                });
            },
        },
        Demo {
            id: "snap-ink-trail",
            name: "Ink Trail",
            category: Category::CursorAndInteraction,
            description: "A surface the cursor paints into: a glowing trail that drifts on a slow noise current and fades, like an ember on dark themes and wet ink on light ones.",
            covers: &["Snapblocks: Ink Trail"],
            code: r##"ui.add(InkTrail::new().text("Leave a mark.").life(1.6));"##,
            show: |ui| {
                let v = mem(ui, "itv", || [26.0f32, 1.6, 1.0]);
                ui.add(InkTrail::new().text("Leave a mark.").width(v[0]).life(v[1]).drift(v[2]).size(vec2(ui.available_width(), 360.0)));
                controls(ui, "itv", [26.0, 1.6, 1.0], |ui, v| {
                    let w = slider_w(ui, 3);
                    ui.add(ControlSlider::new("Width", &mut v[0], 6.0..=60.0).suffix("px").width(w));
                    ui.add(ControlSlider::new("Life", &mut v[1], 0.4..=4.0).decimals(1).suffix("s").width(w));
                    ui.add(ControlSlider::new("Drift", &mut v[2], 0.0..=3.0).decimals(1).width(w));
                });
            },
        },
        Demo {
            id: "snap-petal-dissolve",
            name: "Petal Dissolve",
            category: Category::TextAnimations,
            description: "A word erodes into tumbling red and white petals, then time runs backwards and every petal flies home to rebuild it.",
            covers: &["Snapblocks: Petal Dissolve"],
            code: r##"ui.add(PetalDissolve::new("Ember").size(72.0));"##,
            show: |ui| {
                let v = mem(ui, "pdv", || [1.0f32, 7.0]);
                ui.add(PetalDissolve::new("Ember").wind(v[0]).density(v[1] as usize).size(84.0).height(340.0));
                controls(ui, "pdv", [1.0, 7.0], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Wind", &mut v[0], 0.0..=2.5).decimals(1).width(w));
                    ui.add(ControlSlider::new("Density", &mut v[1], 3.0..=12.0).width(w));
                });
            },
        },
        Demo {
            id: "snap-aperture-zoom",
            name: "Aperture Zoom",
            category: Category::Motion,
            description: "A hero that starts as a small rounded window onto an image and opens like a camera aperture as you scroll, the headline's words sliding apart.",
            covers: &["Snapblocks: Aperture Zoom"],
            code: r##"ScrollArea::vertical().show(ui, |ui| {
    ui.add(ApertureZoom::new("Open", "wide."));
});"##,
            show: |ui| {
                let v = mem(ui, "azv", || [1.3f32, 2.6]);
                auto_scroll(ui, "aperture", 380.0, 160.0, |ui| {
                    ui.add(ApertureZoom::new("Open", "wide.").zoom(v[0]).track(v[1]).height(380.0));
                    scroll_hint(ui, "The page carries on as normal from here.", 300.0);
                });
                controls(ui, "azv", [1.3, 2.6], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Start zoom", &mut v[0], 1.0..=2.0).decimals(2).width(w));
                    ui.add(ControlSlider::new("Track", &mut v[1], 1.5..=5.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "snap-zoom-reveal",
            name: "Zoom Reveal",
            category: Category::Motion,
            description: "A small pill of footage between two words opens into a full-bleed frame as the page scrolls, the words riding its edges out and a play label surfacing.",
            covers: &["Snapblocks: Zoom Reveal"],
            code: r##"ScrollArea::vertical().show(ui, |ui| {
    ui.add(ZoomReveal::new("North", "Light").play_label("Begin the film"));
});"##,
            show: |ui| {
                let v = mem(ui, "zrv", || [28.0f32, 2.6]);
                auto_scroll(ui, "zoom_reveal", 380.0, 160.0, |ui| {
                    ui.add(ZoomReveal::new("North", "Light").font_size(v[0]).track(v[1]).height(380.0));
                    scroll_hint(ui, "The page carries on as normal from here.", 300.0);
                });
                controls(ui, "zrv", [28.0, 2.6], |ui, v| {
                    let w = slider_w(ui, 2);
                    ui.add(ControlSlider::new("Word size", &mut v[0], 16.0..=48.0).suffix("px").width(w));
                    ui.add(ControlSlider::new("Track", &mut v[1], 1.5..=5.0).decimals(1).suffix("x").width(w));
                });
            },
        },
        Demo {
            id: "snap-card-bookmark",
            name: "Card Bookmark",
            category: Category::CardsAndBorders,
            description: "Cards cut like bookmarks with silky wave-ribbon art; the active one fills with its colour, lifts and glows while the others wait in the dark.",
            covers: &["Snapblocks: Card Bookmark"],
            code: r##"ui.add(CardBookmark::new());"##,
            show: |ui| {
                let v = mem(ui, "cbv", || [34.0f32]);
                ui.add(CardBookmark::new().waves(v[0] as usize).height(330.0));
                controls(ui, "cbv", [34.0], |ui, v| {
                    let w = slider_w(ui, 1);
                    ui.add(ControlSlider::new("Ribbons", &mut v[0], 6.0..=80.0).width(w));
                });
            },
        },
    ]
}
