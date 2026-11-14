use super::*;
use egui::{pos2, vec2, Align, Layout, Rect, Sense, Ui, UiBuilder};
use glowkit_core::demo::{Category::MicroInteractions as C, Demo};

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
fn bench<P>(ui: &mut Ui, height: f32, p: &mut P, component: impl FnOnce(&mut Ui, &P), controls: impl FnOnce(&mut Ui, &mut P)) {
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
                        ui.add(
                            PhotoFan::new()
                                .bounce(p[0])
                                .scatter(p[1])
                                .spread(p[2])
                                .cards(p[3] as usize)
                                .corner(p[4])
                                .autoplay(true),
                        );
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
    ]
}
