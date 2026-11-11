//! glowkit gallery: browse, search and preview every component. Runs natively and on the web.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use glowkit::core::demo::{Category, Demo};
use glowkit::core::{color::with_alpha, paint, text::Glyphs};
use glowkit::egui::{self, pos2, vec2, Align, Color32, FontId, Layout, RichText, Sense, Stroke, Ui};
use glowkit::Theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Home,
    Demo(usize),
}

struct Gallery {
    demos: Vec<Demo>,
    page: Page,
    search: String,
    theme_idx: usize,
    themes: Vec<Theme>,
    copied_at: f64,
    /// Bare mode (no chrome) used by the screenshot tool: `#shot/<id>`.
    bare: bool,
}

impl Gallery {
    fn new(cc: &eframe::CreationContext<'_>, initial: Option<String>) -> Self {
        let themes = Theme::presets();
        themes[0].install(&cc.egui_ctx);
        let demos = glowkit::all_demos();
        let mut g = Self { demos, page: Page::Home, search: String::new(), theme_idx: 0, themes, copied_at: -10.0, bare: false };
        if let Some(route) = initial {
            g.route(&route);
        }
        g
    }

    fn route(&mut self, route: &str) {
        let route = route.trim_start_matches('#');
        let (bare, id) = match route.strip_prefix("shot/") {
            Some(rest) => (true, rest),
            None => (false, route),
        };
        self.bare = bare;
        // optional theme suffix: "<id>@<theme-index>"
        let (id, theme) = match id.split_once('@') {
            Some((a, b)) => (a, b.parse::<usize>().ok()),
            None => (id, None),
        };
        if let Some(t) = theme {
            self.theme_idx = t.min(self.themes.len() - 1);
        }
        self.page = self.demos.iter().position(|d| d.id == id).map(Page::Demo).unwrap_or(Page::Home);
    }

    fn matches(&self, d: &Demo) -> bool {
        if self.search.is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        d.name.to_lowercase().contains(&q)
            || d.id.contains(&q)
            || d.description.to_lowercase().contains(&q)
            || d.covers.iter().any(|c| c.to_lowercase().contains(&q))
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        ui.add_space(4.0);
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("🔍  Search components…").desired_width(f32::INFINITY));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if nav_item(ui, "✦  Overview", self.page == Page::Home, None) {
                self.page = Page::Home;
            }
            for cat in Category::ALL {
                let items: Vec<usize> = (0..self.demos.len()).filter(|&i| self.demos[i].category == cat && self.matches(&self.demos[i])).collect();
                if items.is_empty() {
                    continue;
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(cat.label().to_uppercase()).size(10.5).color(t.muted_foreground).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(items.len().to_string()).size(10.5).color(t.muted_foreground));
                    });
                });
                for i in items {
                    let d = self.demos[i];
                    if nav_item(ui, d.name, self.page == Page::Demo(i), None) {
                        self.page = Page::Demo(i);
                        set_hash(d.id);
                    }
                }
            }
            ui.add_space(20.0);
        });
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        ui.horizontal(|ui| {
            ui.set_height(44.0);
            let (r, resp) = ui.allocate_exact_size(vec2(118.0, 30.0), Sense::click());
            let painter = ui.painter_at(r.expand(4.0));
            let time = glowkit::core::time(ui);
            paint::glow(&painter, pos2(r.min.x + 12.0, r.center().y), 14.0, with_alpha(t.gradient[0], 0.6));
            paint::sparkle(&painter, pos2(r.min.x + 12.0, r.center().y), 9.0, time * 0.6, t.foreground);
            let g = Glyphs::layout(&painter, "glowkit", FontId::proportional(19.0), None);
            let pos = pos2(r.min.x + 28.0, r.center().y - g.size.y / 2.0);
            let stops = [t.gradient[0], t.gradient[1], t.gradient[2], t.gradient[0]];
            let grect = egui::Rect::from_min_size(pos, g.size);
            paint::galley_with(&painter, pos, &g.galley, |p| {
                let u = (p.x - grect.min.x) / grect.width().max(1.0);
                glowkit::color::gradient_cyclic(&stops, u * 0.5 - time * 0.15)
            });
            ui.ctx().request_repaint();
            if resp.clicked() {
                self.page = Page::Home;
            }
            ui.label(RichText::new(format!("v{VERSION}")).size(11.0).color(t.muted_foreground));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let names: Vec<&str> = self.themes.iter().map(|t| t.name).collect();
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(format!("🎨 {}", names[self.theme_idx]))
                    .show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut self.theme_idx, i, *n);
                        }
                    });
                ui.hyperlink_to(RichText::new("GitHub").color(t.muted_foreground), "https://github.com/glowkit-rs/glowkit");
                ui.label(RichText::new(format!("{} components", self.demos.len())).color(t.muted_foreground));
            });
        });
    }

    fn home(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("Animated components for Rust apps").size(34.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} components across {} categories. Pure Rust, built on egui: ship them in native Windows, macOS and Linux apps, or on the web through WebAssembly.",
                    self.demos.len(),
                    Category::ALL.iter().filter(|c| self.demos.iter().any(|d| d.category == **c)).count()
                ))
                .size(15.0)
                .color(t.muted_foreground),
            );
            ui.add_space(20.0);
            for cat in Category::ALL {
                let items: Vec<usize> = (0..self.demos.len()).filter(|&i| self.demos[i].category == cat && self.matches(&self.demos[i])).collect();
                if items.is_empty() {
                    continue;
                }
                ui.add_space(12.0);
                ui.label(RichText::new(format!("{}  {}", cat.icon(), cat.label())).size(18.0).strong());
                ui.add_space(6.0);
                let w = ui.available_width();
                let cols = ((w / 250.0).floor() as usize).max(1);
                let cw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
                for chunk in items.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for &i in chunk {
                            let d = self.demos[i];
                            let (r, resp) = ui.allocate_exact_size(vec2(cw, 74.0), Sense::click());
                            let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.15);
                            let p = ui.painter();
                            p.rect_filled(r, t.corner(), t.card);
                            p.rect_stroke(r, t.corner(), Stroke::new(1.0, t.border.lerp_to_gamma(t.accent, hover * 0.7)), egui::StrokeKind::Inside);
                            p.text(r.min + vec2(14.0, 14.0), egui::Align2::LEFT_TOP, d.name, FontId::proportional(14.5), t.foreground);
                            let desc = p.layout(d.description.to_owned(), FontId::proportional(11.5), t.muted_foreground, cw - 28.0);
                            let desc = if desc.rows.len() > 2 {
                                let mut job = egui::text::LayoutJob::simple(d.description.to_owned(), FontId::proportional(11.5), t.muted_foreground, cw - 28.0);
                                job.wrap.max_rows = 2;
                                job.wrap.break_anywhere = false;
                                job.wrap.overflow_character = Some('…');
                                ui.fonts_mut(|f| f.layout_job(job))
                            } else {
                                desc
                            };
                            p.galley(r.min + vec2(14.0, 36.0), desc, t.muted_foreground);
                            if resp.clicked() {
                                self.page = Page::Demo(i);
                                set_hash(d.id);
                            }
                            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        }
                    });
                    ui.add_space(4.0);
                }
            }
            ui.add_space(40.0);
        });
    }

    fn demo_page(&mut self, ui: &mut Ui, i: usize) {
        let t = Theme::of(ui);
        let d = self.demos[i];
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(18.0);
            ui.label(RichText::new(d.category.label()).size(12.0).color(t.accent));
            ui.label(RichText::new(d.name).size(28.0).strong());
            ui.label(RichText::new(d.description).size(14.5).color(t.muted_foreground));
            if !d.covers.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Covers").size(11.5).color(t.muted_foreground));
                    for c in d.covers {
                        chip(ui, c);
                    }
                });
            }
            ui.add_space(12.0);
            preview_frame(ui, d);
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Usage").size(15.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let now = ui.input(|i| i.time);
                    let label = if now - self.copied_at < 1.5 { "✔ Copied" } else { "📋 Copy" };
                    if ui.button(label).clicked() {
                        ui.ctx().copy_text(d.code.to_owned());
                        self.copied_at = now;
                    }
                });
            });
            code_block(ui, d.code);
            ui.add_space(14.0);
            // prev / next
            ui.horizontal(|ui| {
                if i > 0 && ui.button(format!("← {}", self.demos[i - 1].name)).clicked() {
                    self.page = Page::Demo(i - 1);
                    set_hash(self.demos[i - 1].id);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if i + 1 < self.demos.len() && ui.button(format!("{} →", self.demos[i + 1].name)).clicked() {
                        self.page = Page::Demo(i + 1);
                        set_hash(self.demos[i + 1].id);
                    }
                });
            });
            ui.add_space(40.0);
        });
    }
}

fn preview_frame(ui: &mut Ui, d: Demo) {
    let t = Theme::of(ui);
    egui::Frame::new()
        .fill(t.card)
        .stroke(t.border_stroke())
        .corner_radius(t.corner())
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(360.0);
            ui.push_id(d.id, |ui| (d.show)(ui));
        });
}

fn chip(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    egui::Frame::new()
        .fill(t.muted)
        .stroke(t.border_stroke())
        .corner_radius(egui::CornerRadius::same(99))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(t.muted_foreground));
        });
}

fn code_block(ui: &mut Ui, code: &str) {
    let t = Theme::of(ui);
    egui::Frame::new()
        .fill(if t.dark { glowkit::color::hex("#0B0B0E") } else { t.muted })
        .stroke(t.border_stroke())
        .corner_radius(t.corner())
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut job = egui::text::LayoutJob::default();
            highlight_rust(&mut job, code, &t);
            ui.label(job);
        });
}

/// A tiny Rust syntax highlighter for the code panel.
fn highlight_rust(job: &mut egui::text::LayoutJob, code: &str, t: &Theme) {
    use glowkit::color::hex;
    let font = FontId::monospace(12.5);
    let kw = ["use", "let", "mut", "fn", "pub", "struct", "impl", "for", "in", "if", "else", "match", "return", "move", "self", "Self", "true", "false", "as", "const", "while", "loop"];
    let (c_kw, c_str, c_com, c_num, c_ty, c_fn, c_txt) = if t.dark {
        (hex("#C792EA"), hex("#C3E88D"), hex("#636A80"), hex("#F78C6C"), hex("#FFCB6B"), hex("#82AAFF"), hex("#D4D4D8"))
    } else {
        (hex("#7C3AED"), hex("#15803D"), hex("#9CA3AF"), hex("#C2410C"), hex("#B45309"), hex("#1D4ED8"), hex("#27272A"))
    };
    let fmt = |s: &str, c: Color32, job: &mut egui::text::LayoutJob| {
        job.append(s, 0.0, egui::TextFormat { font_id: font.clone(), color: c, ..Default::default() });
    };
    for line in code.split_inclusive('\n') {
        let (body, comment) = match line.find("//") {
            Some(i) if !line[..i].contains('"') => (&line[..i], Some(&line[i..])),
            _ => (line, None),
        };
        let chars: Vec<char> = body.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '"' {
                    if chars[j] == '\\' {
                        j += 1;
                    }
                    j += 1;
                }
                let s: String = chars[i..(j + 1).min(chars.len())].iter().collect();
                fmt(&s, c_str, job);
                i = j + 1;
            } else if c.is_alphabetic() || c == '_' {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                let next = chars.get(j).copied();
                let col = if kw.contains(&w.as_str()) {
                    c_kw
                } else if w.chars().next().is_some_and(|c| c.is_uppercase()) {
                    c_ty
                } else if next == Some('(') || next == Some('!') {
                    c_fn
                } else {
                    c_txt
                };
                fmt(&w, col, job);
                i = j;
            } else if c.is_ascii_digit() {
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                fmt(&w, c_num, job);
                i = j;
            } else {
                let mut j = i;
                while j < chars.len() && !(chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '"') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                fmt(&w, c_txt, job);
                i = j;
            }
        }
        if let Some(cm) = comment {
            fmt(cm, c_com, job);
        }
    }
}

fn nav_item(ui: &mut Ui, label: &str, selected: bool, _badge: Option<&str>) -> bool {
    let t = Theme::of(ui);
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() || selected, 0.12);
    if hover > 0.0 {
        let fill = if selected { with_alpha(t.accent, 0.16) } else { t.muted.gamma_multiply(hover) };
        ui.painter().rect_filled(r, t.corner_scaled(0.6), fill);
    }
    if selected {
        ui.painter().rect_filled(egui::Rect::from_min_size(r.min + vec2(0.0, 6.0), vec2(2.5, r.height() - 12.0)), 2.0, t.accent);
    }
    let col = if selected { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hover) };
    ui.painter().text(r.left_center() + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, label, FontId::proportional(13.0), col);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let theme = self.themes[self.theme_idx].clone();
        if Theme::current(ui.ctx()) != theme {
            theme.install(ui.ctx());
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(h) = web::take_hash_change() {
            self.route(&h);
        }
        if self.bare {
            if let Page::Demo(i) = self.page {
                let d = self.demos[i];
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme.background).inner_margin(egui::Margin::same(16)))
                    .show(ui, |ui| preview_frame(ui, d));
                return;
            }
        }
        egui::Panel::top("top")
            .frame(egui::Frame::new().fill(theme.background).inner_margin(egui::Margin::symmetric(16, 4)).stroke(theme.border_stroke()))
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(250.0)
            .frame(egui::Frame::new().fill(theme.background).inner_margin(egui::Margin::same(12)).stroke(theme.border_stroke()))
            .show(ui, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.background).inner_margin(egui::Margin::symmetric(32, 0)))
            .show(ui, |ui| match self.page {
                Page::Home => self.home(ui),
                Page::Demo(i) => self.demo_page(ui, i),
            });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_hash(_id: &str) {}

#[cfg(target_arch = "wasm32")]
fn set_hash(id: &str) {
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1360.0, 880.0]).with_min_inner_size([900.0, 600.0]).with_title("glowkit gallery"),
        ..Default::default()
    };
    eframe::run_native("glowkit gallery", options, Box::new(|cc| Ok(Box::new(Gallery::new(cc, initial)))))
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
            .start(canvas, eframe::WebOptions::default(), Box::new(move |cc| Ok(Box::new(Gallery::new(cc, Some(initial))))))
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match start {
                Ok(_) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("<p>Failed to start the gallery: {e:?}</p>")),
            }
        }
    });
}
