use crate::*;
use egui::Ui;
use glowkit_core::demo::{Category::ThreeD as C, Demo};

fn chips<T: Clone + PartialEq>(ui: &mut Ui, items: &[T], current: &mut T, name: impl Fn(&T) -> &'static str) {
    ui.horizontal_wrapped(|ui| {
        for it in items {
            if ui.selectable_label(*current == *it, name(it)).clicked() {
                *current = it.clone();
            }
        }
    });
}

pub fn all() -> Vec<Demo> {
    vec![Demo {
        id: "model-viewer",
        name: "Model Viewer",
        category: C,
        description: "A lit 3D model floating over a soft shadow. Drag to orbit with inertia, pinch or ctrl+scroll to zoom, switch models and materials.",
        covers: &["React Bits: Model Viewer", "three.js: OrbitControls", "react-three-fiber: Stage"],
        code: r##"ui.add(ModelViewer::new(Model::Rocket).look(Look::Studio).wireframe(false).height(360.0));

// your own model:
let mesh = Mesh::from_obj(include_str!("model.obj"))?;
ui.add(ModelViewer::new(Model::Custom(Arc::new(mesh))));"##,
        show: |ui| {
            let id = ui.scope_id().with("mv");
            let (mut model, mut look, mut wire): (usize, Look, bool) = glowkit_core::state(ui, id, || (0usize, Look::Studio, false));
            let names: Vec<usize> = (0..7).collect();
            let builtins = [Model::TorusKnot, Model::Rocket, Model::Gem, Model::Mug, Model::Tree, Model::Dodecahedron, Model::Torus];
            chips(ui, &names, &mut model, |i| builtins[*i].name());
            ui.horizontal(|ui| {
                chips(ui, &Look::ALL, &mut look, |l| l.name());
                ui.separator();
                ui.checkbox(&mut wire, "Wireframe");
            });
            glowkit_core::set_state(ui, id, (model, look, wire));
            ui.add(ModelViewer::new(builtins[model].clone()).look(look).wireframe(wire).height(380.0));
        },
    }]
}
