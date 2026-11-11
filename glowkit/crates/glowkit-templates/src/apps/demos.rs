use glowkit_core::demo::{Category, Demo};

use super::common::run_demo;
use super::*;

fn embedded<A: Default>(f: impl FnOnce(&mut A)) -> A {
    let mut a = A::default();
    f(&mut a);
    a
}

pub fn all() -> Vec<Demo> {
    vec![Demo {
        id: "app-analytics",
        name: "Analytics Dashboard",
        category: Category::Templates,
        description: "KPI tiles with sparklines, revenue area chart, traffic donut, top pages and recent orders, with date-range tabs that reload the data behind skeletons.",
        covers: &["shadcn/ui: Dashboard example", "Tremor: Analytics dashboard", "Tailwind UI: Stats + charts dashboard"],
        code: r##"let mut app = AnalyticsApp::default(); // store in your app state
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<AnalyticsApp>(|a| a.scoped_theme = true), AnalyticsApp::show),
    }]
}
