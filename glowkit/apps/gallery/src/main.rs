//! glowkit gallery: browse, search and preview every component. Runs natively and on the web.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod code;
mod ui;

use glowkit::egui;
use ui::Gallery;

struct App(Gallery);

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        if let Some(h) = web::take_hash_change() {
            self.0.route(&h);
        }
        self.0.frame(ui);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn set_hash(_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_hash(id: &str) {
    web::set_hash(id);
}

/// Machine-readable list of every demo (used to generate CATALOG.md and the website catalog).
#[cfg(not(target_arch = "wasm32"))]
fn dump_json() -> String {
    fn esc(s: &str) -> String {
        let mut o = String::with_capacity(s.len() + 2);
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        format!("\"{o}\"")
    }
    let items: Vec<String> = glowkit::all_demos()
        .iter()
        .map(|d| {
            let covers: Vec<String> = d.covers.iter().map(|c| esc(c)).collect();
            format!(
                "  {{\"id\": {}, \"name\": {}, \"category\": {}, \"description\": {}, \"covers\": [{}], \"code\": {}}}",
                esc(d.id),
                esc(d.name),
                esc(d.category.label()),
                esc(d.description),
                covers.join(", "),
                esc(d.code)
            )
        })
        .collect();
    format!("[\n{}\n]\n", items.join(",\n"))
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let initial = std::env::args().nth(1);
    if initial.as_deref() == Some("--dump-json") {
        print!("{}", dump_json());
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 880.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("glowkit"),
        ..Default::default()
    };
    eframe::run_native(
        "glowkit gallery",
        options,
        Box::new(|cc| Ok(Box::new(App(Gallery::new(&cc.egui_ctx, glowkit::all_demos(), initial))))),
    )
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    thread_local! {
        static LAST: RefCell<String> = RefCell::new(String::new());
    }

    pub fn hash() -> String {
        web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default()
    }

    pub fn set_hash(id: &str) {
        let h = format!("#{id}");
        LAST.with(|l| *l.borrow_mut() = h.clone());
        if let Some(w) = web_sys::window() {
            if let Ok(hist) = w.history() {
                let _ = hist.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&h));
            }
        }
    }

    /// Returns the new hash if the URL hash changed since the last frame (e.g. back button).
    pub fn take_hash_change() -> Option<String> {
        let h = hash();
        LAST.with(|l| {
            let mut l = l.borrow_mut();
            if *l != h {
                *l = h.clone();
                Some(h)
            } else {
                None
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("glowkit_canvas")
            .expect("missing #glowkit_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let initial = web::hash();
        let start = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| Ok(Box::new(App(Gallery::new(&cc.egui_ctx, glowkit::all_demos(), Some(initial)))))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match start {
                Ok(_) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("<p>Failed to start the gallery: {e:?}</p>")),
            }
        }
    });
}
