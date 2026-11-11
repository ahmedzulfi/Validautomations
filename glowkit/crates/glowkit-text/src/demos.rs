use crate::*;
use egui::Ui;
use glowkit_core::color::{hex, palettes};
use glowkit_core::demo::{Category::TextAnimations as C, Demo};
use glowkit_core::Trigger;

/// Center a demo vertically/horizontally inside the preview frame.
pub(crate) fn centered(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical_centered(|ui| {
        ui.add_space(110.0);
        add(ui);
    });
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
                ui.add(BlurText::new("Isn't this so cool?!").split_by(SplitBy::Letters).delay(0.05).size(44.0).trigger(Trigger::Loop { pause_ms: 1800 }));
                ui.add_space(20.0);
                ui.add(BlurText::new("Words drift into focus one at a time").direction(Direction::Bottom).color(hex("#A1A1AA")).size(20.0).trigger(Trigger::Loop { pause_ms: 2000 }));
            }),
        },
    ]
}
