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
fn controls<const N: usize>(ui: &mut Ui, key: &str, init: [f32; N], add: impl FnOnce(&mut Ui, &mut [f32; N])) -> [f32; N] {
    let mut v: [f32; N] = mem(ui, key, || init);
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        add(ui, &mut v);
    });
    set_mem(ui, key, v);
    v
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
                ui.add(ClothesLine::new().speed(v[0]).swing(v[1]).clip(clip).size(vec2(ui.available_width(), 300.0)));
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
    ]
}
