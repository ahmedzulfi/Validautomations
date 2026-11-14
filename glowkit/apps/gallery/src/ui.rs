//! The gallery UI: a light-first, tile-based component browser.
//!
//! Layout: top bar (logo, section links, search pill, theme toggle, primary pill), a centered
//! hero, category chips, and a grid of soft rounded tiles that each run their component live
//! (scaled down through a layer transform). Clicking a tile caption opens a detail view with a
//! large stage, Preview / Code tabs and a side panel.

use crate::code::highlight_rust;
use glowkit::core::demo::{Category, Demo};
use glowkit::core::{color::with_alpha, paint};
use glowkit::egui::{
    self, emath::TSTransform, pos2, vec2, Align2, Color32, CornerRadius, FontFamily, FontId, Id, LayerId, Order, Rect,
    RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use glowkit::Theme;
use std::collections::{HashMap, HashSet};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Width demos are laid out at inside grid tiles before being scaled to fit.
const VIRTUAL_W: f32 = 860.0;
const MAX_W: f32 = 1400.0;

#[derive(PartialEq, Clone, Copy)]
pub enum Page {
    Home,
    Demo(usize),
    Start,
}

pub struct Gallery {
    pub demos: Vec<Demo>,
    pub page: Page,
    search: String,
    filter: Option<Category>,
    pub theme_idx: usize,
    pub themes: Vec<Theme>,
    copied: Option<(&'static str, f64)>,
    pub bare: bool,
    show_code: bool,
    /// Measured content height of each demo in tiles (for vertical centering).
    tile_heights: HashMap<&'static str, f32>,
    /// Extra layers (popups, overlays) a tile's demo created, so they get the tile's transform.
    tile_layers: HashMap<&'static str, HashSet<LayerId>>,
    scroll_to_grid: bool,
    /// Offset of the chips row inside the home scroll content (for "scroll to grid").
    grid_offset: f32,
}

// --------------------------------------------------------------------------------------------
// Fonts and type helpers
// --------------------------------------------------------------------------------------------

pub fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions};
    use std::sync::Arc;
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "inter", include_bytes!("../fonts/Inter-Regular.ttf"));
    add(&mut fonts, "inter-semibold", include_bytes!("../fonts/Inter-SemiBold.ttf"));
    add(&mut fonts, "jakarta-bold", include_bytes!("../fonts/PlusJakartaSans-Bold.ttf"));
    add(&mut fonts, "jetbrains", include_bytes!("../fonts/JetBrainsMono-Regular.ttf"));
    let defaults_prop = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let defaults_mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let with = |first: &str, rest: &[String]| {
        let mut v = vec![first.to_owned()];
        v.extend(rest.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with("inter", &defaults_prop));
    fonts.families.insert(FontFamily::Monospace, with("jetbrains", &defaults_mono));
    fonts
        .families
        .insert(FontFamily::Name("display".into()), with("jakarta-bold", &with("inter-semibold", &defaults_prop)));
    fonts.families.insert(FontFamily::Name("semibold".into()), with("inter-semibold", &defaults_prop));
    ctx.set_fonts(fonts);
}

fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

fn body(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Width and left offset of the centered content column inside `avail` points.
fn column(avail: f32) -> (f32, f32) {
    let cw = (avail - 40.0).clamp(200.0, MAX_W);
    (cw, ((avail - cw) / 2.0).max(0.0))
}

// --------------------------------------------------------------------------------------------
// Small building blocks
// --------------------------------------------------------------------------------------------

/// A pill button. `dark` = primary (filled with the foreground color).
fn pill(ui: &mut Ui, text: &str, dark: bool, height: f32) -> egui::Response {
    let t = Theme::of(ui);
    let font = semibold(14.0);
    let g = ui.painter().layout_no_wrap(text.to_owned(), font, Color32::WHITE);
    let size = vec2(g.size().x + height * 0.9, height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    let cr = CornerRadius::same((height / 2.0) as u8);
    let (fill, fg, stroke) = if dark {
        (t.primary.lerp_to_gamma(t.muted_foreground, hov * 0.18), t.primary_foreground, Stroke::NONE)
    } else {
        (t.background.lerp_to_gamma(t.muted, hov), t.foreground, Stroke::new(1.0, t.border))
    };
    p.rect(rect, cr, fill, stroke, StrokeKind::Inside);
    p.galley_with_override_text_color(rect.center() - g.size() / 2.0, g, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A round icon button; `paint_icon` draws inside the given rect with the given color.
fn round_button(
    ui: &mut Ui,
    size: f32,
    filled: bool,
    paint_icon: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    if filled {
        p.circle_filled(rect.center(), size / 2.0, t.primary);
        paint_icon(p, rect.shrink(size * 0.28), t.primary_foreground);
    } else {
        p.circle(rect.center(), size / 2.0 - 0.5, t.background.lerp_to_gamma(t.muted, hov), Stroke::new(1.0, t.border));
        paint_icon(p, rect.shrink(size * 0.28), t.foreground);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn sun_icon(p: &egui::Painter, r: Rect, c: Color32) {
    let ctr = r.center();
    p.circle_stroke(ctr, r.width() * 0.22, Stroke::new(1.6, c));
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let d = vec2(a.cos(), a.sin());
        p.line_segment([ctr + d * r.width() * 0.36, ctr + d * r.width() * 0.5], Stroke::new(1.6, c));
    }
}

fn moon_icon(p: &egui::Painter, r: Rect, c: Color32, bg: Color32) {
    let ctr = r.center();
    p.circle_filled(ctr, r.width() * 0.42, c);
    p.circle_filled(ctr + vec2(r.width() * 0.2, -r.width() * 0.16), r.width() * 0.36, bg);
}

fn arrow_icon(p: &egui::Painter, r: Rect, c: Color32, left: bool) {
    let s = Stroke::new(1.7, c);
    let (a, b) = if left { (r.right_center(), r.left_center()) } else { (r.left_center(), r.right_center()) };
    p.line_segment([a, b], s);
    let dir = if left { 1.0 } else { -1.0 };
    let h = r.width() * 0.32;
    p.line_segment([b, b + vec2(dir * h, -h)], s);
    p.line_segment([b, b + vec2(dir * h, h)], s);
}

fn logo(p: &egui::Painter, r: Rect, t: &Theme, time: f32) {
    p.rect_filled(r, CornerRadius::same((r.width() * 0.32) as u8), t.primary);
    paint::sparkle(p, r.center(), r.width() * 0.32, time * 0.5, t.primary_foreground);
}

fn chip(ui: &mut Ui, text: &str, count: Option<usize>, selected: bool) -> egui::Response {
    let t = Theme::of(ui);
    let font = semibold(13.5);
    let label = match count {
        Some(n) => format!("{text}  {n}"),
        None => text.to_owned(),
    };
    let g = ui.painter().layout_no_wrap(label, font, Color32::WHITE);
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 30.0, 36.0), Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let sel = ui.ctx().animate_bool_with_time(resp.id.with("sel"), selected, 0.18);
    let fill = t
        .muted
        .lerp_to_gamma(glowkit::color::shade(t.muted, if t.dark { 0.04 } else { -0.03 }), hov)
        .lerp_to_gamma(t.primary, sel);
    let fg = t.foreground.lerp_to_gamma(t.primary_foreground, sel);
    ui.painter().rect_filled(rect, CornerRadius::same(18), fill);
    ui.painter().galley_with_override_text_color(rect.center() - g.size() / 2.0, g, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A centered block of mixed-color text (spans of (text, emphasized)).
fn centered_rich(ui: &mut Ui, spans: &[(&str, bool)], size: f32, wrap: f32) {
    let t = Theme::of(ui);
    let mut job = egui::text::LayoutJob::default();
    for (s, strong) in spans {
        job.append(
            s,
            0.0,
            egui::TextFormat {
                font_id: body(size),
                color: if *strong { t.foreground } else { t.muted_foreground },
                line_height: Some(size * 1.55),
                ..Default::default()
            },
        );
    }
    job.wrap.max_width = wrap;
    job.halign = egui::Align::Center;
    let g = ui.painter().layout_job(job);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::hover());
    ui.painter().galley(pos2(rect.center().x, rect.min.y), g, t.foreground);
}

/// Centered single-line text.
fn centered_text(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> Rect {
    let g = ui.painter().layout(text.to_owned(), font, color, ui.available_width().min(900.0));
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::hover());
    let pos = pos2(rect.center().x - g.size().x / 2.0, rect.min.y);
    ui.painter().galley(pos, g, color);
    Rect::from_min_size(pos, rect.size())
}

/// Soft dot field (decoration behind the hero), fading out radially.
fn dot_field(p: &egui::Painter, area: Rect, t: &Theme, time: f32) {
    let step = 18.0;
    let c = area.center();
    let r = area.width().max(area.height()) * 0.5;
    let mut y = area.min.y;
    let mut row = 0;
    while y < area.max.y {
        let mut x = area.min.x + if row % 2 == 0 { 0.0 } else { step / 2.0 };
        while x < area.max.x {
            let d = pos2(x, y).distance(c) / r;
            let wave = 0.5 + 0.5 * ((x * 0.02 + y * 0.015 - time * 0.8).sin());
            let a = ((1.0 - d).clamp(0.0, 1.0)).powf(1.4) * (0.25 + 0.55 * wave);
            if a > 0.02 {
                p.circle_filled(pos2(x, y), 1.3, with_alpha(t.muted_foreground, a * 0.7));
            }
            x += step;
        }
        y += step * 0.9;
        row += 1;
    }
}

// --------------------------------------------------------------------------------------------
// Gallery
// --------------------------------------------------------------------------------------------

impl Gallery {
    pub fn new(ctx: &egui::Context, demos: Vec<Demo>, initial: Option<String>) -> Self {
        install_fonts(ctx);
        let themes = Theme::presets();
        // Light first.
        let theme_idx = themes.iter().position(|t| !t.dark).unwrap_or(0);
        themes[theme_idx].install(ctx);
        let mut g = Self {
            demos,
            page: Page::Home,
            search: String::new(),
            filter: None,
            theme_idx,
            themes,
            copied: None,
            bare: false,
            show_code: false,
            tile_heights: HashMap::new(),
            tile_layers: HashMap::new(),
            scroll_to_grid: false,
            grid_offset: 0.0,
        };
        if let Some(route) = initial {
            g.route(&route);
        }
        g
    }

    pub fn route(&mut self, route: &str) {
        let route = route.trim_start_matches('#');
        let (bare, id) = match route.strip_prefix("shot/") {
            Some(rest) => (true, rest),
            None => (false, route),
        };
        self.bare = bare;
        let (id, theme) = match id.split_once('@') {
            Some((a, b)) => (a, b.parse::<usize>().ok()),
            None => (id, None),
        };
        if let Some(t) = theme {
            self.theme_idx = t.min(self.themes.len() - 1);
        }
        self.show_code = false;
        self.page = if id == "start" {
            Page::Start
        } else {
            self.demos.iter().position(|d| d.id == id).map(Page::Demo).unwrap_or(Page::Home)
        };
    }

    fn open(&mut self, page: Page) {
        self.page = page;
        self.show_code = false;
        match page {
            Page::Demo(i) => crate::set_hash(self.demos[i].id),
            Page::Start => crate::set_hash("start"),
            Page::Home => crate::set_hash(""),
        }
    }

    fn matches(&self, d: &Demo) -> bool {
        if let Some(c) = self.filter {
            if d.category != c {
                return false;
            }
        }
        if self.search.is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        d.name.to_lowercase().contains(&q)
            || d.id.contains(&q)
            || d.description.to_lowercase().contains(&q)
            || d.category.label().to_lowercase().contains(&q)
            || d.covers.iter().any(|c| c.to_lowercase().contains(&q))
    }

    fn copy(&mut self, ui: &Ui, key: &'static str, text: &str) {
        ui.ctx().copy_text(text.to_owned());
        self.copied = Some((key, ui.input(|i| i.time)));
    }

    fn just_copied(&self, ui: &Ui, key: &str) -> bool {
        matches!(self.copied, Some((k, at)) if k == key && ui.input(|i| i.time) - at < 1.6)
    }

    // ----------------------------------------------------------------------------------------
    // Top bar
    // ----------------------------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let bar = ui.available_rect_before_wrap();
        let bar = Rect::from_min_size(bar.min, vec2(bar.width(), 68.0));
        ui.allocate_rect(bar, Sense::hover());
        let narrow = bar.width() < 900.0;
        let time = glowkit::core::time(ui);

        // left: logo + wordmark + links
        let mut x = bar.min.x + 24.0;
        let cy = bar.center().y;
        let lr = Rect::from_center_size(pos2(x + 15.0, cy), vec2(30.0, 30.0));
        let logo_resp = ui.interact(lr.expand2(vec2(60.0, 4.0)), Id::unique("gk_logo"), Sense::click());
        logo(ui.painter(), lr, &t, time);
        ui.painter().text(pos2(x + 40.0, cy), Align2::LEFT_CENTER, "glowkit", display(19.0), t.foreground);
        if logo_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.filter = None;
            self.search.clear();
            self.open(Page::Home);
        }
        x += 140.0;
        if !narrow {
            let links: [(&str, Option<Category>); 5] = [
                ("Components", None),
                ("Templates", Some(Category::Templates)),
                ("Charts", Some(Category::Charts)),
                ("AI", Some(Category::Ai)),
                ("3D", Some(Category::ThreeD)),
            ];
            for (label, cat) in links {
                let active = self.page == Page::Home && self.filter == cat && (cat.is_some() || self.search.is_empty());
                let g = ui.painter().layout_no_wrap(label.to_owned(), body(14.5), t.foreground);
                let r = Rect::from_min_size(pos2(x, cy - 14.0), vec2(g.size().x + 20.0, 28.0));
                let resp = ui.interact(r, Id::unique(("gk_nav", label)), Sense::click());
                let col = if active || resp.hovered() { t.foreground } else { t.muted_foreground };
                ui.painter().galley_with_override_text_color(pos2(r.min.x + 10.0, cy - g.size().y / 2.0), g, col);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.filter = cat;
                    self.search.clear();
                    self.scroll_to_grid = true;
                    self.open(Page::Home);
                }
                x = r.max.x + 2.0;
            }
        }

        // right: theme toggle, GitHub, primary pill
        let right = bar.max.x - 24.0;
        let primary_w = 132.0;
        let pr = Rect::from_min_max(pos2(right - primary_w, cy - 20.0), pos2(right, cy + 20.0));
        ui.scope_builder(UiBuilder::new().max_rect(pr), |ui| {
            if pill(ui, "Get started", true, 40.0).clicked() {
                self.open(Page::Start);
            }
        });
        let tr = Rect::from_center_size(pos2(pr.min.x - 30.0, cy), vec2(40.0, 40.0));
        let dark = t.dark;
        let bg = t.background;
        let toggled = ui
            .scope_builder(UiBuilder::new().max_rect(tr), |ui| {
                round_button(ui, 40.0, false, |p, r, c| if dark { sun_icon(p, r, c) } else { moon_icon(p, r, c, bg) })
                    .on_hover_text(if dark { "Light theme" } else { "Dark theme" })
                    .clicked()
            })
            .inner;
        if toggled {
            self.theme_idx = self.themes.iter().position(|th| th.dark != dark).unwrap_or(0);
        }
        if !narrow {
            let gr = Rect::from_center_size(pos2(tr.min.x - 46.0, cy), vec2(70.0, 28.0));
            let resp = ui.interact(gr, Id::unique("gk_gh"), Sense::click());
            let col = if resp.hovered() { t.foreground } else { t.muted_foreground };
            ui.painter().text(gr.center(), Align2::CENTER_CENTER, "GitHub", body(14.5), col);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/glowkit-rs/glowkit"));
            }
        }

        // center: search pill
        let sw = if narrow { (bar.width() - 140.0 - 24.0 - 220.0).max(120.0) } else { 280.0 };
        let sx = if narrow { bar.min.x + 170.0 } else { bar.center().x - sw / 2.0 + 60.0 };
        let sr = Rect::from_min_size(pos2(sx, cy - 20.0), vec2(sw, 40.0));
        let search_id = Id::unique("gk_search");
        let focused = ui.memory(|m| m.has_focus(search_id));
        let fh = ui.ctx().animate_bool_with_time(search_id.with("f"), focused, 0.15);
        ui.painter().rect(
            sr,
            CornerRadius::same(20),
            t.muted,
            Stroke::new(1.0 + fh, with_alpha(t.ring, 0.5 * fh)),
            StrokeKind::Outside,
        );
        let ic = pos2(sr.min.x + 22.0, cy);
        ui.painter().circle_stroke(ic - vec2(1.5, 1.5), 6.0, Stroke::new(1.6, t.muted_foreground));
        ui.painter().line_segment([ic + vec2(3.0, 3.0), ic + vec2(7.0, 7.0)], Stroke::new(1.6, t.muted_foreground));
        let kbd = if cfg!(target_os = "macos") { "⌘ K" } else { "Ctrl K" };
        if !focused && self.search.is_empty() {
            ui.painter().text(
                sr.right_center() - vec2(16.0, 0.0),
                Align2::RIGHT_CENTER,
                kbd,
                body(12.0),
                with_alpha(t.muted_foreground, 0.8),
            );
        }
        let er = Rect::from_min_max(pos2(sr.min.x + 38.0, sr.min.y + 8.0), pos2(sr.max.x - 58.0, sr.max.y - 8.0));
        let changed = ui
            .scope_builder(UiBuilder::new().max_rect(er), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .id(search_id)
                        .frame(egui::Frame::NONE)
                        .hint_text(RichText::new("Search").color(t.muted_foreground))
                        .font(body(14.5))
                        .desired_width(er.width()),
                )
                .changed()
            })
            .inner;
        if changed && self.page != Page::Home {
            self.open(Page::Home);
        }
        if changed && !self.search.is_empty() {
            self.scroll_to_grid = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            ui.memory_mut(|m| m.request_focus(search_id));
        }
        if focused && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.search.clear();
        }
        let _ = (VERSION, sr);
    }

    // ----------------------------------------------------------------------------------------
    // Home
    // ----------------------------------------------------------------------------------------

    fn home(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        let time = glowkit::core::time(ui);
        let mut area = egui::ScrollArea::vertical().id_salt("gk_home").auto_shrink([false, false]);
        if self.scroll_to_grid {
            self.scroll_to_grid = false;
            area = area.vertical_scroll_offset((self.grid_offset - 12.0).max(0.0));
        }
        let mut grid_offset = self.grid_offset;
        area.show(ui, |ui| {
            let content_top = ui.min_rect().min.y;
            let full = ui.available_rect_before_wrap();
            let (w, pad) = column(ui.available_width());
            // hero
            if self.search.is_empty() {
                let hero_top = ui.cursor().min.y;
                ui.add_space(84.0);
                let deco = Rect::from_min_max(
                    pos2(full.max.x - 420.0, hero_top + 40.0),
                    pos2(full.max.x - 20.0, hero_top + 360.0),
                );
                dot_field(ui.painter(), deco, &t, time);
                dot_field(ui.painter(), deco.translate(vec2(-(full.width() - 460.0), 60.0)), &t, time + 2.0);
                let title = if ui.available_width() > 760.0 { 60.0 } else { 40.0 };
                centered_text(ui, "Animated UI Components", display(title), t.foreground);
                centered_text(ui, "for Rust", display(title), t.muted_foreground.lerp_to_gamma(t.foreground, 0.35));
                ui.add_space(18.0);
                centered_rich(
                    ui,
                    &[
                        ("A library of ", false),
                        ("animated egui components", true),
                        (" and ", false),
                        ("micro-interactions", true),
                        (" you can ship in native Windows, macOS and Linux apps, or on the web.", false),
                    ],
                    17.0,
                    600.0,
                );
                ui.add_space(22.0);
                // platform badges
                let labels = ["Windows", "macOS", "Linux", "WebAssembly"];
                let font = semibold(12.5);
                let widths: Vec<f32> = labels
                    .iter()
                    .map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), t.foreground).size().x + 22.0)
                    .collect();
                let total = widths.iter().sum::<f32>() + 8.0 * (labels.len() - 1) as f32;
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
                let mut bx = row.center().x - total / 2.0;
                for (l, bw) in labels.iter().zip(widths) {
                    let r = Rect::from_min_size(pos2(bx, row.min.y), vec2(bw, 28.0));
                    ui.painter().rect(
                        r,
                        CornerRadius::same(8),
                        t.background,
                        Stroke::new(1.0, t.border),
                        StrokeKind::Inside,
                    );
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, *l, font.clone(), t.foreground);
                    bx += bw + 8.0;
                }
                ui.add_space(30.0);
                // CTAs
                let cta_w = 196.0 + 12.0 + 212.0;
                let (cr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
                let left = Rect::from_min_size(pos2(cr.center().x - cta_w / 2.0, cr.min.y), vec2(196.0, 46.0));
                let right = Rect::from_min_size(pos2(left.max.x + 12.0, cr.min.y), vec2(212.0, 46.0));
                ui.scope_builder(UiBuilder::new().max_rect(left), |ui| {
                    if pill(ui, "Browse components", true, 46.0).clicked() {
                        self.scroll_to_grid = true;
                    }
                });
                let copied = self.just_copied(ui, "install");
                let clicked = ui
                    .scope_builder(UiBuilder::new().max_rect(right), |ui| {
                        pill(ui, if copied { "Copied to clipboard" } else { "$ cargo add glowkit" }, false, 46.0)
                            .clicked()
                    })
                    .inner;
                if clicked {
                    self.copy(ui, "install", "cargo add glowkit");
                }
                ui.add_space(22.0);
                let cats = Category::ALL.iter().filter(|c| self.demos.iter().any(|d| d.category == **c)).count();
                centered_text(
                    ui,
                    &format!(
                        "{} components across {} categories · v{VERSION} · MIT or Apache-2.0",
                        self.demos.len(),
                        cats
                    ),
                    body(13.5),
                    t.muted_foreground,
                );
                ui.add_space(72.0);
            } else {
                ui.add_space(28.0);
            }

            // chips
            grid_offset = ui.cursor().min.y - content_top;
            let chips_rect = Rect::from_min_size(pos2(ui.min_rect().min.x + pad, ui.cursor().min.y), vec2(w, 0.0));
            let chips =
                ui.scope_builder(UiBuilder::new().max_rect(chips_rect.with_max_y(chips_rect.min.y + 4000.0)), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(w);
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        if chip(ui, "All", Some(self.demos.len()), self.filter.is_none()).clicked() {
                            self.filter = None;
                        }
                        for cat in Category::ALL {
                            let n = self.demos.iter().filter(|d| d.category == cat).count();
                            if n == 0 {
                                continue;
                            }
                            if chip(ui, cat.label(), Some(n), self.filter == Some(cat)).clicked() {
                                self.filter = if self.filter == Some(cat) { None } else { Some(cat) };
                            }
                        }
                    });
                });
            let _ = chips;
            ui.add_space(18.0);

            // grid
            let items: Vec<usize> = (0..self.demos.len()).filter(|&i| self.matches(&self.demos[i])).collect();
            if items.is_empty() {
                ui.add_space(60.0);
                centered_text(
                    ui,
                    &format!("No components match \u{201C}{}\u{201D}", self.search),
                    semibold(17.0),
                    t.foreground,
                );
                ui.add_space(6.0);
                centered_text(ui, "Try a different word, or clear the search.", body(14.0), t.muted_foreground);
                return;
            }
            let gap = 16.0;
            let cols = (((w + gap) / (360.0 + gap)).floor() as usize).clamp(1, 3);
            let tile_w = (w - gap * (cols - 1) as f32) / cols as f32;
            let tile_h = (tile_w * 0.74).clamp(240.0, 380.0);
            let caption = 52.0;
            let viewport = ui.clip_rect();
            for chunk in items.chunks(cols) {
                let (row, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), tile_h + caption + gap), Sense::hover());
                for (k, &i) in chunk.iter().enumerate() {
                    let x = row.min.x + pad + k as f32 * (tile_w + gap);
                    let tile = Rect::from_min_size(pos2(x, row.min.y), vec2(tile_w, tile_h));
                    let tile_resp = ui.interact(tile, Id::unique(("gk_tile_click", self.demos[i].id)), Sense::click());
                    let hovered = tile_resp.hovered();
                    self.tile(ui, i, tile, viewport, hovered);
                    let cap = Rect::from_min_size(pos2(tile.min.x, tile.max.y), vec2(tile_w, caption));
                    let cap_clicked = self.caption(ui, i, cap, hovered);
                    if tile_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || cap_clicked {
                        self.open(Page::Demo(i));
                    }
                }
            }
            ui.add_space(40.0);
            footer(ui, &t);
        });
        self.grid_offset = grid_offset;
    }

    /// One grid tile: soft rounded surface with the demo running live, scaled to fit.
    fn tile(&mut self, ui: &mut Ui, i: usize, tile: Rect, viewport: Rect, hovered: bool) {
        let t = Theme::of(ui);
        let d = self.demos[i];
        let cr = CornerRadius::same(28);
        let hov = ui.ctx().animate_bool_with_time(Id::unique(("gk_tile_h", d.id)), hovered, 0.15);
        let fill = t.muted.lerp_to_gamma(glowkit::color::shade(t.muted, if t.dark { 0.03 } else { -0.025 }), hov);
        ui.painter().rect_filled(tile, cr, fill);
        if !viewport.intersects(tile) {
            return;
        }
        let s = (tile.width() / VIRTUAL_W).min(1.0);
        let vh = tile.height() / s;
        let measured = self.tile_heights.get(d.id).copied().unwrap_or(vh * 0.6);
        let top = ((vh - measured) / 2.0).max(12.0);
        let translation = tile.min.to_vec2() - tile.min.to_vec2() * s;
        let transform = TSTransform::new(translation, s);
        // clip in layer space: the tile's virtual rect, intersected with the visible viewport
        let virt = Rect::from_min_size(tile.min, vec2(VIRTUAL_W, vh));
        let vp = viewport.intersect(tile.shrink(1.0));
        let vp_layer = Rect::from_min_max(
            pos2((vp.min.x - translation.x) / s, (vp.min.y - translation.y) / s),
            pos2((vp.max.x - translation.x) / s, (vp.max.y - translation.y) / s),
        );
        let clip = virt.intersect(vp_layer);
        let ctx = ui.ctx().clone();
        let before: HashSet<LayerId> = ctx.memory(|m| m.layer_ids().collect());
        let area_id = Id::unique(("gk_tile", d.id));
        let resp = egui::Area::new(area_id)
            .order(Order::Middle)
            .fixed_pos(tile.min)
            .constrain(false)
            .interactable(false)
            .show(&ctx, |ui| {
                ui.set_clip_rect(clip);
                ui.set_width(VIRTUAL_W);
                ui.add_space(top);
                let inner = egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 0)).show(ui, |ui| {
                    ui.set_width(VIRTUAL_W - 48.0);
                    ui.push_id(("tile", d.id), |ui| (d.show)(ui));
                });
                inner.response.rect.height()
            });
        let h = resp.inner;
        if h.is_finite() && h > 0.0 {
            let prev = self.tile_heights.entry(d.id).or_insert(h);
            if (*prev - h).abs() > 2.0 {
                *prev = h;
            }
        }
        let layer = resp.response.layer_id;
        ctx.set_transform_layer(layer, transform);
        let after: HashSet<LayerId> = ctx.memory(|m| m.layer_ids().collect());
        let extra = self.tile_layers.entry(d.id).or_default();
        for l in after.difference(&before) {
            if *l != layer {
                extra.insert(*l);
            }
        }
        for l in extra.iter() {
            ctx.set_transform_layer(*l, transform);
        }
    }

    /// Caption under a tile: name, category and an open arrow. Returns true when clicked.
    fn caption(&mut self, ui: &mut Ui, i: usize, cap: Rect, tile_hovered: bool) -> bool {
        let t = Theme::of(ui);
        let d = self.demos[i];
        let resp = ui.interact(cap, Id::unique(("gk_cap", d.id)), Sense::click());
        let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered() || tile_hovered, 0.12);
        let p = ui.painter();
        p.text(pos2(cap.min.x + 6.0, cap.min.y + 14.0), Align2::LEFT_TOP, d.name, semibold(14.5), t.foreground);
        p.text(
            pos2(cap.min.x + 6.0, cap.min.y + 33.0),
            Align2::LEFT_TOP,
            d.category.label(),
            body(12.5),
            t.muted_foreground,
        );
        let ar = Rect::from_center_size(pos2(cap.max.x - 20.0, cap.min.y + 26.0), vec2(30.0, 30.0));
        p.circle(ar.center(), 15.0, t.background.lerp_to_gamma(t.muted, hov), Stroke::new(1.0, t.border));
        let a = Rect::from_center_size(ar.center() + vec2(hov * 2.0, 0.0), vec2(11.0, 11.0));
        arrow_icon(p, a, t.foreground, false);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    // ----------------------------------------------------------------------------------------
    // Detail
    // ----------------------------------------------------------------------------------------

    fn detail(&mut self, ui: &mut Ui, i: usize) {
        let t = Theme::of(ui);
        let d = self.demos[i];
        let n = self.demos.len();
        egui::ScrollArea::vertical().id_salt(("gk_detail", d.id)).auto_shrink([false, false]).show(ui, |ui| {
            let (w, pad) = column(ui.available_width());
            ui.add_space(12.0);
            // back link
            ui.horizontal(|ui| {
                ui.add_space(pad);
                let g = ui.painter().layout_no_wrap("All components".to_owned(), body(14.0), t.foreground);
                let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 30.0, 30.0), Sense::click());
                let col = if resp.hovered() { t.foreground } else { t.muted_foreground };
                arrow_icon(
                    ui.painter(),
                    Rect::from_center_size(pos2(r.min.x + 8.0, r.center().y), vec2(11.0, 11.0)),
                    col,
                    true,
                );
                ui.painter().galley_with_override_text_color(
                    pos2(r.min.x + 22.0, r.center().y - g.size().y / 2.0),
                    g,
                    col,
                );
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.open(Page::Home);
                }
            });
            ui.add_space(8.0);
            let stacked = w < 980.0;
            let panel_w = if stacked { w } else { 340.0 };
            let stage_w = if stacked { w } else { w - panel_w - 14.0 };
            let stage_h = (ui.available_height() - 40.0).clamp(560.0, 820.0);
            let (area, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), if stacked { stage_h + 14.0 + 420.0 } else { stage_h }),
                Sense::hover(),
            );
            let stage = Rect::from_min_size(pos2(area.min.x + pad, area.min.y), vec2(stage_w, stage_h));
            let panel = if stacked {
                Rect::from_min_size(pos2(stage.min.x, stage.max.y + 14.0), vec2(panel_w, 420.0))
            } else {
                Rect::from_min_size(pos2(stage.max.x + 14.0, stage.min.y), vec2(panel_w, stage_h))
            };
            self.stage(ui, i, stage);
            self.side_panel(ui, i, panel);
            ui.add_space(40.0);
            let _ = n;
        });
    }

    fn stage(&mut self, ui: &mut Ui, i: usize, stage: Rect) {
        let t = Theme::of(ui);
        let d = self.demos[i];
        let n = self.demos.len();
        ui.painter().rect_filled(stage, CornerRadius::same(32), t.muted);
        // segmented Preview / Code
        let seg = Rect::from_min_size(stage.min + vec2(14.0, 14.0), vec2(164.0, 38.0));
        ui.painter().rect_filled(
            seg,
            CornerRadius::same(19),
            glowkit::color::shade(t.muted, if t.dark { 0.05 } else { -0.04 }),
        );
        let tabs = ["Preview", "Code"];
        let thumb_x =
            ui.ctx().animate_value_with_time(Id::unique("gk_seg"), if self.show_code { 1.0 } else { 0.0 }, 0.18);
        let tw = (seg.width() - 8.0) / 2.0;
        let thumb =
            Rect::from_min_size(pos2(seg.min.x + 4.0 + thumb_x * tw, seg.min.y + 4.0), vec2(tw, seg.height() - 8.0));
        paint::soft_shadow(
            ui.painter(),
            thumb,
            15.0,
            6.0,
            vec2(0.0, 1.0),
            Color32::from_black_alpha(if t.dark { 60 } else { 18 }),
        );
        ui.painter().rect_filled(thumb, CornerRadius::same(15), t.background);
        for (k, label) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(seg.min.x + 4.0 + k as f32 * tw, seg.min.y + 4.0),
                vec2(tw, seg.height() - 8.0),
            );
            let resp = ui.interact(r, Id::unique(("gk_tab", k)), Sense::click());
            let on = (k == 1) == self.show_code;
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                *label,
                semibold(13.5),
                if on { t.foreground } else { t.muted_foreground },
            );
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.show_code = k == 1;
            }
        }
        // prev / next
        let nr = Rect::from_center_size(pos2(stage.max.x - 32.0, stage.min.y + 33.0), vec2(36.0, 36.0));
        let pr = nr.translate(vec2(-44.0, 0.0));
        let go_prev = ui
            .scope_builder(UiBuilder::new().max_rect(pr), |ui| {
                round_button(ui, 36.0, false, |p, r, c| arrow_icon(p, r, c, true))
                    .on_hover_text(self.demos[(i + n - 1) % n].name)
                    .clicked()
            })
            .inner;
        let go_next = ui
            .scope_builder(UiBuilder::new().max_rect(nr), |ui| {
                round_button(ui, 36.0, false, |p, r, c| arrow_icon(p, r, c, false))
                    .on_hover_text(self.demos[(i + 1) % n].name)
                    .clicked()
            })
            .inner;
        if go_prev {
            self.open(Page::Demo((i + n - 1) % n));
        }
        if go_next {
            self.open(Page::Demo((i + 1) % n));
        }
        if ui.input(|inp| inp.key_pressed(egui::Key::ArrowRight) && inp.modifiers.alt) {
            self.open(Page::Demo((i + 1) % n));
        }
        let inner = Rect::from_min_max(stage.min + vec2(24.0, 70.0), stage.max - vec2(24.0, 24.0));
        if self.show_code {
            ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
                egui::ScrollArea::both().id_salt(("gk_code", d.id)).show(ui, |ui| {
                    egui::Frame::new()
                        .fill(t.background)
                        .stroke(t.border_stroke())
                        .corner_radius(CornerRadius::same(18))
                        .inner_margin(egui::Margin::same(20))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let mut job = egui::text::LayoutJob::default();
                            highlight_rust(&mut job, d.code, &t);
                            ui.label(job);
                        });
                });
            });
        } else {
            // vertically center the demo using last frame's measured height
            let key = Id::unique(("gk_stage_h", d.id));
            let measured: f32 = ui.data(|m| m.get_temp(key)).unwrap_or(360.0);
            let top = ((inner.height() - measured) / 2.0).max(0.0);
            let child = Rect::from_min_max(inner.min + vec2(0.0, top), inner.max);
            let h = ui
                .scope_builder(UiBuilder::new().max_rect(child), |ui| {
                    ui.set_clip_rect(stage.shrink(2.0).intersect(ui.clip_rect()));
                    ui.push_id(("stage", d.id), |ui| (d.show)(ui));
                    ui.min_rect().height()
                })
                .inner;
            ui.data_mut(|m| m.insert_temp(key, h));
        }
    }

    fn side_panel(&mut self, ui: &mut Ui, i: usize, panel: Rect) {
        let t = Theme::of(ui);
        let d = self.demos[i];
        ui.painter().rect_filled(panel, CornerRadius::same(32), t.muted);
        let inner = panel.shrink(22.0);
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(RichText::new(d.category.label().to_uppercase()).font(semibold(11.5)).color(t.muted_foreground));
            ui.label(RichText::new(d.name).font(display(24.0)).color(t.foreground));
            ui.add(
                egui::Label::new(
                    RichText::new(d.description).font(body(14.5)).color(t.muted_foreground).line_height(Some(22.0)),
                )
                .wrap(),
            );
            ui.add_space(6.0);
            if !d.covers.is_empty() {
                ui.label(RichText::new("Equivalent to").font(semibold(13.0)).color(t.foreground));
                let row_w = ui.available_width();
                let shown = d.covers.len().min(6);
                for c in &d.covers[..shown] {
                    let (lib, item) = c.split_once(':').map(|(a, b)| (a.trim(), b.trim())).unwrap_or(("", c));
                    let (r, _) = ui.allocate_exact_size(vec2(row_w, 34.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(12), t.background);
                    let lib_g = ui.painter().layout_no_wrap(lib.to_owned(), body(12.0), t.muted_foreground);
                    let lib_w = lib_g.size().x;
                    let mut job = egui::text::LayoutJob::simple_singleline(item.to_owned(), body(13.0), t.foreground);
                    job.wrap.max_width = (r.width() - lib_w - 36.0).max(40.0);
                    job.wrap.max_rows = 1;
                    job.wrap.overflow_character = Some('\u{2026}');
                    let item_g = ui.painter().layout_job(job);
                    ui.painter().galley(
                        pos2(r.min.x + 12.0, r.center().y - item_g.size().y / 2.0),
                        item_g,
                        t.foreground,
                    );
                    ui.painter().galley(
                        pos2(r.max.x - 12.0 - lib_w, r.center().y - lib_g.size().y / 2.0),
                        lib_g,
                        t.muted_foreground,
                    );
                }
                if d.covers.len() > shown {
                    ui.label(
                        RichText::new(format!("+{} more", d.covers.len() - shown))
                            .font(body(12.5))
                            .color(t.muted_foreground),
                    );
                }
            }
            // theme presets
            ui.add_space(8.0);
            ui.label(RichText::new("Theme").font(semibold(13.0)).color(t.foreground));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for k in 0..self.themes.len() {
                    let th = &self.themes[k];
                    let sel = k == self.theme_idx;
                    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                    ui.painter().circle_filled(r.center(), 14.0, th.background);
                    ui.painter().circle_filled(r.center(), 7.0, th.accent);
                    ui.painter().circle_stroke(
                        r.center(),
                        14.0,
                        Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { t.foreground } else { t.border }),
                    );
                    if resp.on_hover_text(th.name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.theme_idx = k;
                    }
                }
            });
        });
        // bottom actions
        let b2 = Rect::from_min_max(pos2(inner.min.x, inner.max.y - 44.0), inner.max);
        let b1 = b2.translate(vec2(0.0, -54.0));
        let code_copied = self.just_copied(ui, "code");
        let install_copied = self.just_copied(ui, "install");
        let c1 = ui
            .scope_builder(UiBuilder::new().max_rect(b1), |ui| {
                wide_pill(ui, if code_copied { "Copied" } else { "Copy code" }, false)
            })
            .inner;
        let c2 = ui
            .scope_builder(UiBuilder::new().max_rect(b2), |ui| {
                wide_pill(ui, if install_copied { "Copied to clipboard" } else { "cargo add glowkit" }, true)
            })
            .inner;
        if c1 {
            self.copy(ui, "code", d.code);
        }
        if c2 {
            self.copy(ui, "install", "cargo add glowkit");
        }
    }

    // ----------------------------------------------------------------------------------------
    // Get started
    // ----------------------------------------------------------------------------------------

    fn start(&mut self, ui: &mut Ui) {
        let t = Theme::of(ui);
        egui::ScrollArea::vertical().id_salt("gk_start").auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width().min(760.0);
            let pad = ((ui.available_width() - w) / 2.0).max(20.0);
            ui.add_space(64.0);
            centered_text(ui, "Get started", display(44.0), t.foreground);
            ui.add_space(8.0);
            centered_rich(ui, &[("Add glowkit next to eframe, install a theme, and drop components into any egui layout.", false)], 16.0, 560.0);
            ui.add_space(36.0);
            let steps: [(&str, &str, &str); 3] = [
                ("Add the crates", "Every category is a cargo feature, all on by default.", "[dependencies]\nglowkit = \"0.1\"\neframe = \"0.37\""),
                (
                    "Install a theme",
                    "One call restyles every glowkit component and egui's own widgets.",
                    "use glowkit::prelude::*;\n\n// light, dark, ocean, ember, matrix, or your own tokens\nTheme::light().install(&cc.egui_ctx);",
                ),
                (
                    "Add components",
                    "Builders that feel like egui: ui.add(Thing::new(..)).",
                    "use glowkit::{backgrounds::Aurora, effects::ShimmerButton, text::ShinyText};\n\nAurora::new().paint(ui, ui.max_rect());\nui.add(ShinyText::new(\"Hello from Rust\").size(48.0));\nif ui.add(ShimmerButton::new(\"Get started\")).clicked() {\n    // ...\n}",
                ),
            ];
            for (k, (title, sub, code)) in steps.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 14.0, t.primary);
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{}", k + 1), semibold(13.0), t.primary_foreground);
                            ui.label(RichText::new(*title).font(semibold(17.0)).color(t.foreground));
                        });
                        ui.label(RichText::new(*sub).font(body(14.5)).color(t.muted_foreground));
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(t.muted)
                            .corner_radius(CornerRadius::same(18))
                            .inner_margin(egui::Margin::same(18))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let mut job = egui::text::LayoutJob::default();
                                highlight_rust(&mut job, code, &t);
                                ui.label(job);
                            });
                    });
                });
                ui.add_space(26.0);
            }
            ui.horizontal(|ui| {
                ui.add_space(pad);
                if pill(ui, "Browse all components", true, 44.0).clicked() {
                    self.filter = None;
                    self.open(Page::Home);
                }
            });
            ui.add_space(60.0);
        });
    }

    // ----------------------------------------------------------------------------------------
    // Frame
    // ----------------------------------------------------------------------------------------

    pub fn frame(&mut self, ui: &mut Ui) {
        let theme = self.themes[self.theme_idx].clone();
        if Theme::current(ui.ctx()) != theme {
            theme.install(ui.ctx());
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
        egui::Panel::top("gk_top")
            .frame(egui::Frame::new().fill(theme.background))
            .show_separator_line(false)
            .show(ui, |ui| self.top_bar(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.background)).show(ui, |ui| match self.page {
            Page::Home => self.home(ui),
            Page::Demo(i) => self.detail(ui, i),
            Page::Start => self.start(ui),
        });
    }
}

fn wide_pill(ui: &mut Ui, text: &str, dark: bool) -> bool {
    let t = Theme::of(ui);
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let cr = CornerRadius::same((rect.height() / 2.0) as u8);
    let (fill, fg, stroke) = if dark {
        (t.primary.lerp_to_gamma(t.muted_foreground, hov * 0.18), t.primary_foreground, Stroke::NONE)
    } else {
        (t.background.lerp_to_gamma(t.muted, hov * 0.5), t.foreground, Stroke::new(1.0, t.border))
    };
    ui.painter().rect(rect, cr, fill, stroke, StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, semibold(14.0), fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn footer(ui: &mut Ui, t: &Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 80.0), Sense::hover());
    ui.painter().hline(r.x_range().shrink(20.0), r.min.y, Stroke::new(1.0, t.border));
    ui.painter().text(
        pos2(r.min.x + 26.0, r.center().y),
        Align2::LEFT_CENTER,
        format!("glowkit v{VERSION} · animated components for Rust · MIT or Apache-2.0"),
        body(13.0),
        t.muted_foreground,
    );
}

/// The plain preview used by the screenshot route (`#shot/<id>`): a soft stage with the demo.
pub fn preview_frame(ui: &mut Ui, d: Demo) {
    let t = Theme::of(ui);
    egui::Frame::new().fill(t.muted).corner_radius(CornerRadius::same(28)).inner_margin(egui::Margin::same(24)).show(
        ui,
        |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(360.0);
            ui.push_id(d.id, |ui| (d.show)(ui));
        },
    );
}
