use super::*;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::{hex, palettes, with_alpha};
use glowkit_core::demo::{Category::CursorAndInteraction as C, Demo};
use glowkit_core::{paint, Theme};

/// Allocate a demo stage with a soft dotted backdrop and a hint, returning its rect.
fn stage(ui: &mut Ui, height: f32, hint: &str) -> Rect {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect(rect, t.corner(), t.background, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let step = 22.0;
    let dot = with_alpha(t.muted_foreground, if t.dark { 0.16 } else { 0.22 });
    let mut y = rect.min.y + step / 2.0;
    while y < rect.max.y {
        let mut x = rect.min.x + step / 2.0;
        while x < rect.max.x {
            p.circle_filled(pos2(x, y), 1.0, dot);
            x += step;
        }
        y += step;
    }
    if !hint.is_empty() {
        p.text(rect.center(), Align2::CENTER_CENTER, hint, FontId::proportional(15.0), with_alpha(t.muted_foreground, 0.85));
    }
    rect
}

/// Split the available width into `n` equal stages.
fn stages(ui: &mut Ui, n: usize, height: f32, hints: &[&str]) -> Vec<Rect> {
    let full = ui.available_width();
    let gap = 16.0;
    let w = (full - gap * (n as f32 - 1.0)) / n as f32;
    let mut out = Vec::new();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for i in 0..n {
            ui.allocate_ui(vec2(w, height), |ui| {
                ui.set_width(w);
                out.push(stage(ui, height, hints.get(i).copied().unwrap_or("")));
            });
        }
    });
    out
}

fn chip(ui: &mut Ui, text: &str) -> egui::Response {
    let t = Theme::of(ui);
    egui::Frame::new()
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(18, 12))
        .show(ui, |ui| ui.label(RichText::new(text).size(15.0).color(t.foreground)))
        .response
}

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "click-spark",
            name: "Click Spark",
            category: C,
            description: "Lines of light burst outward from every click. Wraps any area without stealing its clicks.",
            covers: &["React Bits: Click Spark"],
            code: r##"// over an existing rect
ClickSpark::new().count(10).radius(28.0).show(ui, rect);
// or as its own area
ui.add(ClickSpark::new().color(hex("#F472B6")).autoplay(true));"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["Click anywhere", "Bigger, pinker sparks"]);
                ClickSpark::new().autoplay(true).count(8).radius(26.0).id_salt(1).show(ui, r[0]);
                ClickSpark::new().autoplay(true).count(12).radius(44.0).length(16.0).width(2.5).color(hex("#F472B6")).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "click-ripple",
            name: "Click Ripple",
            category: C,
            description: "Concentric rings ripple out from each click like a drop hitting water.",
            covers: &["Material: Ripple", "Magic UI: Ripple (on click)"],
            code: r##"ClickRipple::new().rings(3).max_radius(140.0).show(ui, rect);"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["Click to ripple", ""]);
                ClickRipple::new().autoplay(true).id_salt(1).show(ui, r[0]);
                ClickRipple::new().autoplay(true).rings(5).max_radius(170.0).duration(1.6).color(hex("#22D3EE")).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "pointer-trail",
            name: "Pointer Trail",
            category: C,
            description: "A glowing comet tail chases the pointer and shrinks away when it stops.",
            covers: &["React Bits: Splash Cursor (spirit)", "Magic UI: Cursor trail", "21st.dev: Pointer trail"],
            code: r##"PointerTrail::new().width(12.0).lifetime(0.45).show(ui, rect);"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["Move your pointer", ""]);
                PointerTrail::new().autoplay(true).id_salt(1).show(ui, r[0]);
                PointerTrail::new().autoplay(true).width(20.0).lifetime(0.8).colors(&palettes::OCEAN).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "ribbon-trail",
            name: "Ribbons",
            category: C,
            description: "Silky ribbons chase the pointer on springs of different stiffness, weaving around each other.",
            covers: &["React Bits: Ribbons"],
            code: r##"RibbonTrail::new()
    .colors(&[hex("#FC8EAC"), hex("#8B5CF6"), hex("#22D3EE")])
    .thickness(18.0)
    .show(ui, rect);"##,
            show: |ui| {
                let rect = stage(ui, 360.0, "Move your pointer around");
                RibbonTrail::new().autoplay(true).colors(&[hex("#FC8EAC"), hex("#8B5CF6"), hex("#22D3EE"), hex("#FBBF24")]).thickness(18.0).show(ui, rect);
            },
        },
        Demo {
            id: "pixel-trail",
            name: "Pixel Trail",
            category: C,
            description: "Cells of an invisible grid light up under the pointer and fade out, leaving a pixelated trail.",
            covers: &["React Bits: Pixel Trail"],
            code: r##"PixelTrail::new().cell(24.0).fade(0.6).show(ui, rect);
PixelTrail::new().cell(14.0).rounding(0.5).rainbow(true).brush(1).show(ui, rect);"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["", ""]);
                PixelTrail::new().autoplay(true).cell(26.0).fade(0.7).id_salt(1).show(ui, r[0]);
                PixelTrail::new().autoplay(true).cell(14.0).rounding(0.5).rainbow(true).brush(1).fade(0.9).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "card-trail",
            name: "Card Trail",
            category: C,
            description: "Moving the pointer drops a trail of cards that pop in, tilt and shrink away (image trail).",
            covers: &["React Bits: Image Trail", "Codrops: Image Trail"],
            code: r##"CardTrail::new()
    .faces(vec![CardFace::new("🌊", "Ocean"), CardFace::new("🌅", "Sunset")])
    .spacing(70.0)
    .show(ui, rect);"##,
            show: |ui| {
                let rect = stage(ui, 380.0, "Drag your pointer across");
                CardTrail::new().autoplay(true).show(ui, rect);
            },
        },
        Demo {
            id: "blob-cursor",
            name: "Blob Cursor",
            category: C,
            description: "Gooey blobs chase the pointer with growing lag and melt into each other like liquid.",
            covers: &["React Bits: Blob Cursor", "Gooey cursor"],
            code: r##"BlobCursor::new().sizes(&[56.0, 84.0, 44.0]).goo(26.0).show(ui, rect);"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["", ""]);
                BlobCursor::new().autoplay(true).id_salt(1).show(ui, r[0]);
                BlobCursor::new().autoplay(true).sizes(&[40.0, 54.0, 66.0, 48.0, 30.0]).goo(20.0).colors(&palettes::OCEAN).inner_dot(false).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "smooth-cursor",
            name: "Smooth Cursor",
            category: C,
            description: "A custom cursor glides after the pointer on a spring, turning to face its direction of travel.",
            covers: &["Magic UI: Smooth Cursor"],
            code: r##"SmoothCursor::new().size(14.0).spring(400.0, 34.0).show(ui, rect);"##,
            show: |ui| {
                let r = stages(ui, 2, 340.0, &["Snappy", "Lazy & big"]);
                SmoothCursor::new().autoplay(true).id_salt(1).show(ui, r[0]);
                SmoothCursor::new().autoplay(true).size(20.0).spring(120.0, 14.0).color(Theme::of(ui).accent).id_salt(2).show(ui, r[1]);
            },
        },
        Demo {
            id: "following-pointer",
            name: "Following Pointer",
            category: C,
            description: "A collaborator-style pointer with a name tag that trails the cursor over a card.",
            covers: &["Aceternity: Following Pointer", "Liveblocks-style live cursors"],
            code: r##"let card = ui.add(my_card).rect;
FollowingPointer::new("Ada Lovelace").color(hex("#0EA5E9")).show(ui, card);"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(380.0, 330.0), Sense::hover());
                    let p = ui.painter_at(rect.expand(30.0));
                    crate::motion::common::theme_card(&p, &t, rect, 16.0, 1.0);
                    let img = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(rect.width() - 28.0, 160.0));
                    paint::gradient_rect(&p, img, 10.0, 35.0, &palettes::MAGIC);
                    p.text(img.center(), Align2::CENTER_CENTER, "🚀", FontId::proportional(54.0), Color32::WHITE);
                    let x = rect.min.x + 18.0;
                    p.text(pos2(x, img.max.y + 18.0), Align2::LEFT_TOP, "Shipping glowkit 1.0", FontId::proportional(19.0), t.foreground);
                    p.text(pos2(x, img.max.y + 48.0), Align2::LEFT_TOP, "Animated components for egui, in pure Rust.", FontId::proportional(13.5), t.muted_foreground);
                    let btn = Rect::from_min_size(pos2(x, rect.max.y - 52.0), vec2(110.0, 34.0));
                    p.rect_filled(btn, CornerRadius::same(8), t.primary);
                    p.text(btn.center(), Align2::CENTER_CENTER, "Read more", FontId::proportional(13.0), t.primary_foreground);
                    p.text(pos2(rect.max.x - 18.0, btn.center().y), Align2::RIGHT_CENTER, "2 min read", FontId::proportional(12.0), t.muted_foreground);
                    FollowingPointer::new("Ada Lovelace").autoplay(true).color(hex("#0EA5E9")).show(ui, rect);
                });
            },
        },
        Demo {
            id: "crosshair",
            name: "Crosshair",
            category: C,
            description: "Hairlines track the pointer across the area, with an intersection ring and live coordinates.",
            covers: &["React Bits: Crosshair"],
            code: r##"Crosshair::new().coordinates(true).show(ui, rect);"##,
            show: |ui| {
                let rect = stage(ui, 360.0, "");
                let t = Theme::of(ui);
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, "Aim anywhere", FontId::proportional(40.0), with_alpha(t.foreground, 0.85));
                Crosshair::new().autoplay(true).show(ui, rect);
            },
        },
        Demo {
            id: "target-cursor",
            name: "Target Cursor",
            category: C,
            description: "Spinning corner brackets follow the pointer and snap around whatever target it hovers.",
            covers: &["React Bits: Target Cursor"],
            code: r##"let a = ui.button("Docs").rect;
let b = ui.button("GitHub").rect;
TargetCursor::new().targets([a, b]).show(ui, area);"##,
            show: |ui| {
                let area = stage(ui, 340.0, "");
                let mut targets = Vec::new();
                let inner = Rect::from_center_size(area.center(), vec2(area.width(), 120.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Center)), |ui| {
                    ui.label(RichText::new("Hover the targets").size(22.0).color(Theme::of(ui).foreground));
                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        let labels = ["✦  Components", "⚡ Fast", "🦀 Pure Rust", "🌐 WebAssembly"];
                        let total: f32 = 4.0 * 150.0;
                        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                        ui.spacing_mut().item_spacing.x = 20.0;
                        for l in labels {
                            targets.push(chip(ui, l).rect);
                        }
                    });
                });
                TargetCursor::new().autoplay(true).targets(targets).show(ui, area);
            },
        },
        Demo {
            id: "magnet",
            name: "Magnet",
            category: C,
            description: "Wraps any content and pulls it toward a nearby pointer, springing back when it leaves.",
            covers: &["React Bits: Magnet", "Magic UI: Magnetic button", "Motion Primitives: Magnetic"],
            code: r##"Magnet::new().strength(0.4).padding(80.0).show(ui, |ui| {
    ui.button("Hover near me")
});"##,
            show: |ui| {
                let t = Theme::of(ui);
                ui.add_space(110.0);
                ui.horizontal(|ui| {
                    let w = 3.0 * 170.0 + 2.0 * 60.0;
                    ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                    ui.spacing_mut().item_spacing.x = 60.0;
                    for (i, (label, strength)) in [("Gentle", 0.2), ("Magnetic", 0.4), ("Sticky", 0.7)].into_iter().enumerate() {
                        Magnet::new().strength(strength).padding(90.0).autoplay(true).id_salt(i).show(ui, |ui| {
                            let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 120.0), Sense::click());
                            let p = ui.painter();
                            crate::motion::common::theme_card(p, &t, rect, 16.0, 1.0);
                            p.text(rect.center() - vec2(0.0, 14.0), Align2::CENTER_CENTER, ["🧲", "✨", "🍯"][i], FontId::proportional(30.0), t.foreground);
                            p.text(rect.center() + vec2(0.0, 26.0), Align2::CENTER_CENTER, label, FontId::proportional(15.0), t.foreground);
                            resp
                        });
                    }
                });
            },
        },
        Demo {
            id: "confetti",
            name: "Confetti",
            category: C,
            description: "Confetti bursts from a point, tumbles in 3D and flutters down. Fire it from code or on click.",
            covers: &["Magic UI: Confetti", "canvas-confetti", "Magic UI: Confetti side cannons"],
            code: r##"let confetti = ConfettiBurst::new().count(120);
if ui.button("🎉 Celebrate").clicked() {
    confetti.fire(ui, area.center());
}
confetti.show(ui, area);"##,
            show: |ui| {
                let area = stage(ui, 360.0, "");
                let confetti = ConfettiBurst::new().count(110).autoplay(true);
                let btn_rect = Rect::from_center_size(area.center(), vec2(170.0, 46.0));
                let resp = ui.interact(btn_rect, ui.scope_id().with("celebrate"), Sense::click());
                let t = Theme::of(ui);
                let p = ui.painter();
                let hover = if resp.hovered() { 0.06 } else { 0.0 };
                paint::soft_shadow(p, btn_rect, 23.0, 20.0, vec2(0.0, 6.0), with_alpha(t.accent, 0.35));
                paint::gradient_rect(p, btn_rect, 23.0, 0.0, &[glowkit_core::color::shade(t.gradient[0], hover), glowkit_core::color::shade(t.gradient[1], hover)]);
                p.text(btn_rect.center(), Align2::CENTER_CENTER, "🎉  Celebrate", FontId::proportional(16.0), Color32::WHITE);
                if resp.clicked() {
                    confetti.fire_with(ui, btn_rect.center(), -90.0, 80.0);
                }
                confetti.fire_on_click(false).show(ui, area);
            },
        },
        Demo {
            id: "fireworks",
            name: "Fireworks",
            category: C,
            description: "Rockets streak up and burst into glowing, falling sparks. Click to launch your own.",
            covers: &["Magic UI: Confetti (fireworks)", "tsParticles: Fireworks"],
            code: r##"Fireworks::new().interval(0.8).show(ui, rect);"##,
            show: |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 360.0), Sense::hover());
                let p = ui.painter_at(rect);
                paint::gradient_rect(&p, rect, Theme::of(ui).radius, 90.0, &[hex("#020617"), hex("#0B1029"), hex("#1E1B4B")]);
                let mut rng = glowkit_core::Rng::new(5);
                let now = glowkit_core::time(ui);
                for _ in 0..70 {
                    let s = pos2(rng.range(rect.min.x, rect.max.x), rng.range(rect.min.y, rect.max.y - 40.0));
                    let tw = 0.5 + 0.5 * (now * rng.range(1.0, 3.0) + rng.range(0.0, 6.0)).sin();
                    p.circle_filled(s, rng.range(0.5, 1.3), with_alpha(Color32::WHITE, 0.25 + 0.5 * tw));
                }
                p.text(pos2(rect.center().x, rect.max.y - 28.0), Align2::CENTER_CENTER, "Click to launch", FontId::proportional(13.0), with_alpha(Color32::WHITE, 0.55));
                Fireworks::new().interval(0.7).show(ui, rect);
            },
        },
    ]
}
