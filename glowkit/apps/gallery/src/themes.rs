//! The theme builder page (`#themes`): edit every [`Theme`] token, start from presets or a
//! random harmonious palette, preview real components live, check WCAG contrast and export the
//! Rust code.

use crate::code::highlight_rust;
use crate::ui::{body, display, pill, semibold};
use glowkit::color::{hsv, luminance, with_alpha};
use glowkit::core::playground::{hex_string, lit_color, lit_f32, parse_hex};
use glowkit::core::Rng;
use glowkit::egui::{
    self, emath::TSTransform, pos2, vec2, Align, Align2, Color32, CornerRadius, Id, Layout, Order, Rect, RichText,
    Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use glowkit::Theme;
use std::collections::HashMap;

/// What the page asks the gallery to do.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    UseInGallery,
}

/// Mutable state of preview widgets.
#[derive(Default)]
struct PreviewState {
    email: String,
    prompt: String,
    switch: bool,
    check: bool,
    slider: f32,
}

pub struct ThemeBuilder {
    pub theme: Theme,
    seed: u64,
    hex: HashMap<&'static str, String>,
    preview: PreviewState,
    /// Index into `Theme::presets()` of the preset the theme started from.
    preset: Option<usize>,
}

/// The named color tokens, in editor order, grouped.
const GROUPS: [(&str, &[&str]); 3] = [
    (
        "Surfaces",
        &[
            "background",
            "foreground",
            "card",
            "card_foreground",
            "popover",
            "muted",
            "muted_foreground",
            "border",
            "input",
            "ring",
        ],
    ),
    ("Brand", &["primary", "primary_foreground", "secondary", "secondary_foreground", "accent", "accent_foreground"]),
    ("Status", &["destructive", "success", "warning", "info"]),
];

fn token_mut<'a>(t: &'a mut Theme, name: &str) -> &'a mut Color32 {
    match name {
        "background" => &mut t.background,
        "foreground" => &mut t.foreground,
        "card" => &mut t.card,
        "card_foreground" => &mut t.card_foreground,
        "popover" => &mut t.popover,
        "muted" => &mut t.muted,
        "muted_foreground" => &mut t.muted_foreground,
        "border" => &mut t.border,
        "input" => &mut t.input,
        "ring" => &mut t.ring,
        "primary" => &mut t.primary,
        "primary_foreground" => &mut t.primary_foreground,
        "secondary" => &mut t.secondary,
        "secondary_foreground" => &mut t.secondary_foreground,
        "accent" => &mut t.accent,
        "accent_foreground" => &mut t.accent_foreground,
        "destructive" => &mut t.destructive,
        "success" => &mut t.success,
        "warning" => &mut t.warning,
        "info" => &mut t.info,
        "gradient[0]" => &mut t.gradient[0],
        "gradient[1]" => &mut t.gradient[1],
        "gradient[2]" => &mut t.gradient[2],
        "chart[0]" => &mut t.chart[0],
        "chart[1]" => &mut t.chart[1],
        "chart[2]" => &mut t.chart[2],
        "chart[3]" => &mut t.chart[3],
        "chart[4]" => &mut t.chart[4],
        _ => &mut t.chart[5],
    }
}

fn token(t: &Theme, name: &str) -> Color32 {
    let mut c = t.clone();
    *token_mut(&mut c, name)
}

const GRADIENT: [&str; 3] = ["gradient[0]", "gradient[1]", "gradient[2]"];
const CHART: [&str; 6] = ["chart[0]", "chart[1]", "chart[2]", "chart[3]", "chart[4]", "chart[5]"];

/// WCAG contrast ratio between two opaque colors (1..=21).
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Black or white text, whichever has the higher WCAG contrast on `bg`.
fn best_text(bg: Color32) -> Color32 {
    let (dark, light) = (Color32::from_rgb(10, 10, 12), Color32::WHITE);
    if contrast_ratio(dark, bg) >= contrast_ratio(light, bg) {
        dark
    } else {
        light
    }
}

/// The Rust code that recreates `t`, as a diff against its light/dark base.
pub fn export_code(t: &Theme) -> String {
    let base = if t.dark { Theme::dark() } else { Theme::light() };
    let mut s = String::from("use glowkit::color::hex;\nuse glowkit::Theme;\n\n");
    s += &format!("let mut t = Theme::{}();\n", if t.dark { "dark" } else { "light" });
    for (_, names) in GROUPS {
        for n in names {
            let c = token(t, n);
            if c != token(&base, n) {
                s += &format!("t.{n} = {};\n", lit_color(c));
            }
        }
    }
    if t.gradient != base.gradient {
        let g: Vec<String> = t.gradient.iter().map(|c| lit_color(*c)).collect();
        s += &format!("t.gradient = [{}];\n", g.join(", "));
    }
    if t.chart != base.chart {
        let g: Vec<String> = t.chart.iter().map(|c| lit_color(*c)).collect();
        s += &format!("t.chart = [\n    {},\n    {},\n];\n", g[..3].join(", "), g[3..].join(", "));
    }
    if (t.radius - base.radius).abs() > 0.01 {
        s += &format!("t.radius = {};\n", lit_f32(t.radius));
    }
    s += "t.install(ctx);";
    s
}

/// A random, harmonious theme: hue-tinted neutrals, one accent, an analogous / complementary /
/// triadic gradient and an evenly spread chart palette.
pub fn random_theme(seed: u64, dark: bool) -> Theme {
    let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(17));
    let h = r.range(0.0, 360.0);
    let tint = r.range(0.03, 0.12);
    let base = if dark { Theme::dark() } else { Theme::light() };
    let mut t = base.clone();
    t.name = "Custom";
    let sat = r.range(0.55, 0.8);
    let accent = hsv(h, sat, if dark { 0.95 } else { 0.72 });
    if dark {
        t.background = hsv(h, tint * 2.2, 0.06);
        t.card = hsv(h, tint * 2.0, 0.09);
        t.popover = hsv(h, tint * 1.8, 0.11);
        t.muted = hsv(h, tint * 1.6, 0.13);
        t.border = hsv(h, tint * 1.4, 0.2);
        t.input = t.border;
        t.foreground = hsv(h, 0.03, 0.98);
        t.muted_foreground = hsv(h, tint, 0.68);
        t.secondary = t.border;
    } else {
        t.background = hsv(h, tint * 0.15, 1.0);
        t.card = Color32::WHITE;
        t.popover = Color32::WHITE;
        t.muted = hsv(h, tint * 0.5, 0.965);
        t.border = hsv(h, tint * 0.8, 0.9);
        t.input = t.border;
        t.foreground = hsv(h, 0.35, 0.09);
        t.muted_foreground = hsv(h, tint * 1.2, 0.46);
        t.secondary = t.muted;
    }
    t.card_foreground = t.foreground;
    t.secondary_foreground = t.foreground;
    t.primary = if r.bool(0.5) {
        accent
    } else if dark {
        hsv(h, 0.03, 0.98)
    } else {
        hsv(h, 0.3, 0.12)
    };
    t.primary_foreground = best_text(t.primary);
    t.accent = accent;
    t.accent_foreground = best_text(accent);
    t.ring = accent;
    let v = if dark { 0.95 } else { 0.82 };
    let (b, c) = match r.below(3) {
        0 => (h + 35.0, h + 70.0),
        1 => (h + 150.0, h + 180.0),
        _ => (h + 120.0, h + 240.0),
    };
    t.gradient = [accent, hsv(b, 0.7, v), hsv(c, 0.75, v)];
    for (k, slot) in t.chart.iter_mut().enumerate() {
        *slot = hsv(h + k as f32 * 137.5, if dark { 0.6 } else { 0.7 }, if dark { 0.95 } else { 0.78 });
    }
    t.chart[0] = accent;
    t.radius = *r.pick(&[4.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0]);
    t
}

impl ThemeBuilder {
    pub fn new(start: Theme) -> Self {
        let preset = Theme::presets().iter().position(|p| *p == start);
        Self {
            theme: start,
            seed: 1,
            hex: HashMap::new(),
            preview: PreviewState { slider: 0.6, switch: true, ..Default::default() },
            preset,
        }
    }

    /// The page. `in_gallery` is the theme the gallery currently uses (restored after the preview).
    pub fn show(&mut self, ui: &mut Ui, in_gallery: &Theme, custom_in_use: Option<&Theme>) -> Action {
        let g = in_gallery.clone();
        let mut action = Action::None;
        egui::ScrollArea::vertical().id_salt("gk_themes").auto_shrink([false, false]).show(ui, |ui| {
            let (w, pad) = crate::ui::column(ui.available_width());
            ui.add_space(40.0);
            ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.label(RichText::new("Theme builder").font(display(40.0)).color(g.foreground));
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(
                            "Edit every token, preview real components live, check contrast and copy the Rust code.",
                        )
                        .font(body(16.0))
                        .color(g.muted_foreground),
                    );
                    ui.add_space(18.0);
                    action = self.toolbar(ui, &g, custom_in_use);
                    ui.add_space(18.0);
                    let wide = w >= 1060.0;
                    if wide {
                        let editor_w = 380.0;
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;
                            ui.vertical(|ui| {
                                ui.set_width(editor_w);
                                self.editor(ui, &g);
                            });
                            ui.vertical(|ui| {
                                ui.set_width(w - editor_w - 16.0);
                                self.checks(ui, &g);
                                ui.add_space(14.0);
                                self.preview(ui, &g);
                                ui.add_space(14.0);
                                self.export(ui, &g);
                            });
                        });
                    } else {
                        self.checks(ui, &g);
                        ui.add_space(14.0);
                        self.preview(ui, &g);
                        ui.add_space(14.0);
                        self.editor(ui, &g);
                        ui.add_space(14.0);
                        self.export(ui, &g);
                    }
                    ui.add_space(60.0);
                });
            });
        });
        action
    }

    // ----------------------------------------------------------------------------------------
    // Toolbar
    // ----------------------------------------------------------------------------------------

    fn toolbar(&mut self, ui: &mut Ui, g: &Theme, custom_in_use: Option<&Theme>) -> Action {
        let mut action = Action::None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
            ui.label(RichText::new("Start from").font(semibold(13.0)).color(g.muted_foreground));
            for (k, p) in Theme::presets().into_iter().enumerate() {
                if preset_chip(ui, &p, self.preset == Some(k) && self.theme == p).clicked() {
                    self.theme = p;
                    self.preset = Some(k);
                    self.hex.clear();
                }
            }
            ui.add_space(8.0);
            if pill(ui, "✦  Randomize", false, 36.0).on_hover_text("A random harmonious palette").clicked() {
                self.seed = self
                    .seed
                    .wrapping_add(1)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(ui.input(|i| (i.time * 1000.0) as u64));
                self.theme = random_theme(self.seed, self.theme.dark);
                self.preset = None;
                self.hex.clear();
            }
            let mut dark = self.theme.dark as usize;
            if segmented(ui, "gk_tb_base", &["Light", "Dark"], &mut dark, 150.0) {
                self.rebase(dark == 1);
            }
            ui.add_space(8.0);
            let label = match custom_in_use {
                Some(c) if *c == self.theme => "✓  In use",
                Some(_) => "Update gallery theme",
                None => "Use in gallery",
            };
            if pill(ui, label, true, 36.0).clicked() {
                action = Action::UseInGallery;
            }
        });
        action
    }

    /// Switch light/dark base but keep the brand tokens.
    fn rebase(&mut self, dark: bool) {
        if dark == self.theme.dark {
            return;
        }
        let old = self.theme.clone();
        let mut t = if dark { Theme::dark() } else { Theme::light() };
        t.name = "Custom";
        t.accent = old.accent;
        t.accent_foreground = old.accent_foreground;
        t.ring = old.ring;
        t.gradient = old.gradient;
        t.chart = old.chart;
        t.radius = old.radius;
        self.theme = t;
        self.preset = None;
        self.hex.clear();
    }

    // ----------------------------------------------------------------------------------------
    // Editor
    // ----------------------------------------------------------------------------------------

    fn editor(&mut self, ui: &mut Ui, g: &Theme) {
        egui::Frame::new()
            .fill(g.muted)
            .corner_radius(CornerRadius::same(28))
            .inner_margin(egui::Margin::same(20))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 4.0;
                group_title(ui, g, "Shape");
                row(ui, g, "radius", |ui| {
                    let mut r = self.theme.radius;
                    let resp = ui.add(egui::Slider::new(&mut r, 0.0..=24.0).step_by(1.0).suffix(" pt"));
                    if resp.changed() {
                        self.theme.radius = r;
                        self.preset = None;
                    }
                });
                for (title, names) in GROUPS {
                    ui.add_space(10.0);
                    group_title(ui, g, title);
                    for n in names {
                        self.color_row(ui, g, n, n);
                    }
                    if title == "Brand" {
                        ui.add_space(2.0);
                        if small_link(ui, g, "Derive foregrounds from backgrounds").clicked() {
                            let t = &mut self.theme;
                            t.primary_foreground = best_text(t.primary);
                            t.accent_foreground = best_text(t.accent);
                            t.secondary_foreground = t.foreground;
                            t.card_foreground = t.foreground;
                            self.hex.clear();
                        }
                    }
                }
                ui.add_space(10.0);
                group_title(ui, g, "Gradient");
                gradient_strip(ui, &self.theme.gradient);
                for (k, n) in GRADIENT.iter().enumerate() {
                    self.color_row(ui, g, n, ["stop 1", "stop 2", "stop 3"][k]);
                }
                ui.add_space(10.0);
                group_title(ui, g, "Chart");
                swatch_strip(ui, &self.theme.chart);
                for (k, n) in CHART.iter().enumerate() {
                    self.color_row(ui, g, n, ["chart 1", "chart 2", "chart 3", "chart 4", "chart 5", "chart 6"][k]);
                }
            });
    }

    fn color_row(&mut self, ui: &mut Ui, g: &Theme, name: &'static str, label: &str) {
        let id = Id::unique(("gk_tb_hex", name));
        let focused = ui.memory(|m| m.has_focus(id));
        let mut c = token(&self.theme, name);
        let buf = self.hex.entry(name).or_default();
        if !focused {
            *buf = hex_string(c);
        }
        let mut changed = false;
        row(ui, g, label, |ui| {
            let te = ui.add(
                egui::TextEdit::singleline(buf)
                    .id(id)
                    .font(egui::FontId::monospace(12.5))
                    .desired_width(82.0)
                    .char_limit(9)
                    .margin(vec2(8.0, 4.0)),
            );
            if te.changed() {
                if let Some(p) = parse_hex(buf) {
                    c = p;
                    changed = true;
                }
            }
            ui.spacing_mut().interact_size = vec2(30.0, 24.0);
            if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::OnlyBlend).changed() {
                changed = true;
            }
        });
        if changed {
            *token_mut(&mut self.theme, name) = c;
            self.theme.name = "Custom";
            self.preset = None;
        }
    }

    // ----------------------------------------------------------------------------------------
    // Contrast checks
    // ----------------------------------------------------------------------------------------

    fn checks(&mut self, ui: &mut Ui, g: &Theme) {
        let t = &self.theme;
        let pairs = [
            ("Text on background", t.foreground, t.background, true),
            ("Primary button text", t.primary_foreground, t.primary, true),
            ("Card text", t.card_foreground, t.card, false),
            ("Muted text", t.muted_foreground, t.background, false),
            ("Accent text", t.accent_foreground, t.accent, false),
        ];
        let failing: Vec<&str> =
            pairs.iter().filter(|(_, a, b, key)| *key && contrast_ratio(*a, *b) < 4.5).map(|(n, ..)| *n).collect();
        egui::Frame::new()
            .fill(g.muted)
            .corner_radius(CornerRadius::same(24))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Contrast (WCAG 2)").font(semibold(14.0)).color(g.foreground));
                    if !failing.is_empty() {
                        ui.label(
                            RichText::new(format!("⚠  {} below AA 4.5:1", failing.join(" and ")))
                                .font(body(13.0))
                                .color(g.destructive),
                        );
                    }
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for (name, fg, bg, _) in pairs {
                        contrast_chip(ui, g, name, fg, bg);
                    }
                });
            });
    }

    // ----------------------------------------------------------------------------------------
    // Live preview
    // ----------------------------------------------------------------------------------------

    fn preview(&mut self, ui: &mut Ui, g: &Theme) {
        let ctx = ui.ctx().clone();
        let t = self.theme.clone();
        t.install(&ctx);
        ui.scope(|ui| {
            ui.set_style(ctx.global_style());
            egui::Frame::new()
                .fill(t.background)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(28))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Live preview").font(semibold(14.0)).color(t.foreground));
                        ui.label(
                            RichText::new("real components, rendered with your theme")
                                .font(body(13.0))
                                .color(t.muted_foreground),
                        );
                    });
                    ui.add_space(10.0);
                    preview_grid(ui, &mut self.preview);
                });
        });
        g.install(&ctx);
    }

    // ----------------------------------------------------------------------------------------
    // Export
    // ----------------------------------------------------------------------------------------

    fn export(&mut self, ui: &mut Ui, g: &Theme) {
        let code = export_code(&self.theme);
        let copied_id = Id::unique("gk_tb_copied");
        let now = ui.input(|i| i.time);
        let copied_at: f64 = ui.data(|d| d.get_temp(copied_id)).unwrap_or(-10.0);
        egui::Frame::new()
            .fill(g.muted)
            .corner_radius(CornerRadius::same(24))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Export").font(semibold(14.0)).color(g.foreground));
                    ui.label(
                        RichText::new("only the tokens that differ from the base")
                            .font(body(13.0))
                            .color(g.muted_foreground),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if now - copied_at < 1.6 { "Copied" } else { "Copy" };
                        if pill(ui, label, true, 32.0).clicked() {
                            ui.ctx().copy_text(code.clone());
                            ui.data_mut(|d| d.insert_temp(copied_id, now));
                        }
                    });
                });
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(g.background)
                    .stroke(g.border_stroke())
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let mut job = egui::text::LayoutJob::default();
                        highlight_rust(&mut job, &code, g);
                        ui.label(job);
                    });
            });
    }
}

// --------------------------------------------------------------------------------------------
// Small pieces
// --------------------------------------------------------------------------------------------

fn group_title(ui: &mut Ui, g: &Theme, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text.to_uppercase()).font(semibold(11.5)).color(g.muted_foreground));
    ui.add_space(2.0);
}

/// A bencho-style row: label on the left, control(s) right-aligned.
fn row(ui: &mut Ui, g: &Theme, label: &str, add: impl FnOnce(&mut Ui)) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(10), g.background);
    ui.painter().text(
        pos2(r.min.x + 12.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::monospace(12.0),
        g.foreground,
    );
    let ctl = Rect::from_min_max(pos2(r.min.x + 150.0, r.min.y + 3.0), pos2(r.max.x - 6.0, r.max.y - 3.0));
    ui.scope_builder(UiBuilder::new().max_rect(ctl).layout(Layout::right_to_left(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.spacing_mut().interact_size.y = 24.0;
        add(ui)
    });
}

fn small_link(ui: &mut Ui, g: &Theme, text: &str) -> egui::Response {
    let resp = ui.add(egui::Label::new(RichText::new(text).font(body(12.5)).color(g.accent)).sense(Sense::click()));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn gradient_strip(ui: &mut Ui, stops: &[Color32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    glowkit::paint::gradient_rect(ui.painter(), r, 8.0, 0.0, stops);
}

fn swatch_strip(ui: &mut Ui, cs: &[Color32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let w = r.width() / cs.len() as f32;
    for (k, c) in cs.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(r.min.x + k as f32 * w, r.min.y), vec2(w - 3.0, r.height()));
        ui.painter().rect_filled(cell, CornerRadius::same(5), *c);
    }
}

fn preset_chip(ui: &mut Ui, p: &Theme, selected: bool) -> egui::Response {
    let g = Theme::of(ui);
    let gl = ui.painter().layout_no_wrap(p.name.to_owned(), semibold(13.0), g.foreground);
    let (r, resp) = ui.allocate_exact_size(vec2(gl.size().x + 46.0, 36.0), Sense::click());
    let hov = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let fill = g.background.lerp_to_gamma(g.muted, hov);
    ui.painter().rect(
        r,
        CornerRadius::same(18),
        fill,
        Stroke::new(if selected { 1.6 } else { 1.0 }, if selected { g.foreground } else { g.border }),
        StrokeKind::Inside,
    );
    let c = pos2(r.min.x + 18.0, r.center().y);
    ui.painter().circle(c, 9.0, p.background, Stroke::new(1.0, with_alpha(g.muted_foreground, 0.4)));
    ui.painter().circle_filled(c, 4.5, p.accent);
    ui.painter().galley(pos2(r.min.x + 34.0, r.center().y - gl.size().y / 2.0), gl, g.foreground);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Segmented control; returns true when the selection changed.
pub fn segmented(ui: &mut Ui, id: &str, options: &[&str], sel: &mut usize, width: f32) -> bool {
    let g = Theme::of(ui);
    let (r, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    ui.painter().rect_filled(
        r,
        CornerRadius::same(18),
        glowkit::color::shade(g.muted, if g.dark { 0.05 } else { -0.03 }),
    );
    let n = options.len().max(1) as f32;
    let tw = (r.width() - 8.0) / n;
    let x = ui.ctx().animate_value_with_time(Id::unique((id, "thumb")), *sel as f32, 0.16);
    let thumb = Rect::from_min_size(pos2(r.min.x + 4.0 + x * tw, r.min.y + 4.0), vec2(tw, r.height() - 8.0));
    glowkit::paint::soft_shadow(
        ui.painter(),
        thumb,
        14.0,
        6.0,
        vec2(0.0, 1.0),
        Color32::from_black_alpha(if g.dark { 60 } else { 16 }),
    );
    ui.painter().rect_filled(thumb, CornerRadius::same(14), g.background);
    let mut changed = false;
    for (k, o) in options.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(r.min.x + 4.0 + k as f32 * tw, r.min.y + 4.0), vec2(tw, r.height() - 8.0));
        let resp = ui.interact(cell, Id::unique((id, k)), Sense::click());
        let on = k == *sel;
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            *o,
            semibold(12.5),
            if on { g.foreground } else { g.muted_foreground },
        );
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
            *sel = k;
            changed = true;
        }
    }
    changed
}

fn contrast_chip(ui: &mut Ui, g: &Theme, name: &str, fg: Color32, bg: Color32) {
    let ratio = contrast_ratio(fg, bg);
    let (grade, col) = if ratio >= 7.0 {
        ("AAA", g.success)
    } else if ratio >= 4.5 {
        ("AA", g.success)
    } else if ratio >= 3.0 {
        ("AA large", g.warning)
    } else {
        ("Fail", g.destructive)
    };
    let text = format!("{name}  {ratio:.1}:1");
    let gl = ui.painter().layout_no_wrap(text, body(12.5), g.foreground);
    let gg = ui.painter().layout_no_wrap(grade.to_owned(), semibold(11.0), col);
    let w = 34.0 + gl.size().x + 12.0 + gg.size().x + 16.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    ui.painter().rect(r, CornerRadius::same(17), g.background, Stroke::new(1.0, g.border), StrokeKind::Inside);
    let sw = Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(22.0, 22.0));
    ui.painter().rect(sw, CornerRadius::same(11), bg, Stroke::new(1.0, g.border), StrokeKind::Inside);
    ui.painter().text(sw.center(), Align2::CENTER_CENTER, "A", semibold(12.0), fg);
    ui.painter().galley(pos2(r.min.x + 34.0, r.center().y - gl.size().y / 2.0), gl, g.foreground);
    let gr =
        Rect::from_min_size(pos2(r.max.x - gg.size().x - 18.0, r.center().y - 10.0), vec2(gg.size().x + 10.0, 20.0));
    ui.painter().rect_filled(gr, CornerRadius::same(10), with_alpha(col, 0.14));
    ui.painter().galley(pos2(gr.min.x + 5.0, gr.center().y - gg.size().y / 2.0), gg, col);
}

// --------------------------------------------------------------------------------------------
// Preview grid (runs inside the edited theme)
// --------------------------------------------------------------------------------------------

struct Cell {
    title: &'static str,
    height: f32,
    show: fn(&mut Ui, &mut PreviewState),
}

#[allow(clippy::vec_init_then_push)]
fn cells() -> Vec<Cell> {
    #[allow(unused_mut)]
    let mut v: Vec<Cell> = Vec::new();
    #[cfg(feature = "components")]
    v.push(Cell { title: "Buttons", height: 132.0, show: cell_buttons });
    #[cfg(feature = "effects")]
    v.push(Cell { title: "Magic buttons", height: 132.0, show: cell_magic_buttons });
    #[cfg(feature = "text")]
    v.push(Cell { title: "Text effects", height: 132.0, show: cell_text });
    #[cfg(feature = "charts")]
    v.push(Cell { title: "Line chart", height: 220.0, show: cell_line });
    #[cfg(feature = "effects")]
    v.push(Cell { title: "Magic card", height: 190.0, show: cell_magic_card });
    #[cfg(feature = "components")]
    v.push(Cell { title: "Form", height: 230.0, show: cell_form });
    #[cfg(feature = "charts")]
    v.push(Cell { title: "Bar chart", height: 220.0, show: cell_bar });
    #[cfg(feature = "ai")]
    v.push(Cell { title: "AI chat", height: 240.0, show: cell_chat });
    #[cfg(feature = "charts")]
    v.push(Cell { title: "Donut", height: 220.0, show: cell_donut });
    #[cfg(feature = "templates")]
    v.push(Cell { title: "Template", height: 290.0, show: cell_template });
    #[cfg(feature = "effects")]
    v.push(Cell { title: "Border beam", height: 170.0, show: cell_beam });
    #[cfg(feature = "ai")]
    v.push(Cell { title: "Prompt", height: 170.0, show: cell_prompt });
    #[cfg(feature = "charts")]
    v.push(Cell { title: "Stat tile", height: 150.0, show: cell_stat });
    #[cfg(feature = "components")]
    v.push(Cell { title: "Progress", height: 150.0, show: cell_progress });
    #[cfg(feature = "ai")]
    v.push(Cell { title: "Context & loaders", height: 200.0, show: cell_context });
    #[cfg(feature = "components")]
    v.push(Cell { title: "Badges & status", height: 120.0, show: cell_badges });
    v
}

fn preview_grid(ui: &mut Ui, st: &mut PreviewState) {
    let t = Theme::of(ui);
    let w = ui.available_width();
    let gap = 12.0;
    let cols = if w > 820.0 {
        3
    } else if w > 520.0 {
        2
    } else {
        1
    };
    let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
    let top = ui.cursor().min;
    let mut heights = vec![0.0f32; cols];
    for (k, cell) in cells().into_iter().enumerate() {
        let col = (0..cols).min_by(|a, b| heights[*a].total_cmp(&heights[*b])).unwrap_or(0);
        let r = Rect::from_min_size(pos2(top.x + col as f32 * (cw + gap), top.y + heights[col]), vec2(cw, cell.height));
        heights[col] += cell.height + gap;
        if !ui.is_rect_visible(r) {
            continue;
        }
        let radius = CornerRadius::same((t.radius * 1.4).round().clamp(0.0, 40.0) as u8);
        ui.painter().rect(r, radius, t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
        ui.painter().text(r.min + vec2(14.0, 12.0), Align2::LEFT_TOP, cell.title, semibold(12.0), t.muted_foreground);
        let inner = Rect::from_min_max(r.min + vec2(14.0, 34.0), r.max - vec2(14.0, 10.0));
        ui.scope_builder(UiBuilder::new().max_rect(inner).id_salt(("gk_tb_cell", k)), |ui| {
            ui.set_clip_rect(inner.expand(8.0).intersect(ui.clip_rect()));
            (cell.show)(ui, st);
        });
    }
    let total = heights.iter().copied().fold(0.0, f32::max);
    ui.allocate_rect(Rect::from_min_size(top, vec2(w, (total - gap).max(0.0))), Sense::hover());
}

#[cfg(feature = "components")]
fn cell_buttons(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::components::{Button, ButtonVariant};
    ui.horizontal_wrapped(|ui| {
        ui.add(Button::new("Primary"));
        ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
        ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
        ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
    });
}

#[cfg(feature = "effects")]
fn cell_magic_buttons(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::effects::{GlowButton, ShimmerButton};
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.add(ShimmerButton::new("Shimmer").size(14.0));
        ui.add(GlowButton::new("Glow").size(14.0));
    });
}

#[cfg(feature = "text")]
fn cell_text(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::text::{GradientText, ShinyText};
    let t = Theme::of(ui);
    ui.add(GradientText::new("Glowing gradient").colors(&t.gradient).size(28.0));
    ui.add(ShinyText::new("Shiny muted text").size(18.0));
}

#[cfg(feature = "charts")]
fn months() -> [&'static str; 6] {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
}

#[cfg(feature = "charts")]
fn cell_line(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::charts::{LineChart, Series};
    let s = [
        Series::new("Desktop", [186.0, 305.0, 237.0, 273.0, 209.0, 314.0]),
        Series::new("Mobile", [80.0, 200.0, 120.0, 190.0, 130.0, 240.0]),
    ];
    ui.add(LineChart::new(&s).labels(&months()).dots(true).height(170.0).legend(false).id_salt("tb_line"));
}

#[cfg(feature = "charts")]
fn cell_bar(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::charts::{BarChart, Series};
    let s = [
        Series::new("Revenue", [42.0, 58.0, 51.0, 74.0, 69.0, 92.0]),
        Series::new("Costs", [30.0, 36.0, 40.0, 44.0, 47.0, 51.0]),
    ];
    ui.add(BarChart::new(&s).labels(&months()).radius(4.0).height(170.0).legend(false).id_salt("tb_bar"));
}

#[cfg(feature = "charts")]
fn cell_donut(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::charts::{Datum, PieChart};
    let d = [
        Datum::new("Chrome", 275.0),
        Datum::new("Safari", 200.0),
        Datum::new("Firefox", 187.0),
        Datum::new("Edge", 120.0),
    ];
    ui.add(
        PieChart::new(&d)
            .inner_radius(0.62)
            .pad_angle(2.0)
            .rounded(true)
            .center_caption("Visitors")
            .height(170.0)
            .id_salt("tb_donut"),
    );
}

#[cfg(feature = "charts")]
fn cell_stat(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::charts::StatTile;
    let spark = [12.0, 14.0, 13.0, 17.0, 16.0, 19.0, 18.0, 22.0, 24.0];
    ui.add(
        StatTile::new("Monthly revenue", 48_210.0)
            .prefix("$")
            .delta(12.4)
            .spark(&spark)
            .size(vec2(ui.available_width(), 96.0)),
    );
}

#[cfg(feature = "effects")]
fn cell_magic_card(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::effects::MagicCard;
    MagicCard::new().ambient(true).min_height(130.0).show(ui, |ui| {
        ui.heading("Magic card");
        ui.label("Gradient spotlight in your accent.");
    });
}

#[cfg(feature = "effects")]
fn cell_beam(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::effects::BorderBeam;
    BorderBeam::new().min_height(110.0).show(ui, |ui| {
        ui.heading("Border beam");
        ui.label("Uses the theme gradient.");
    });
}

#[cfg(feature = "components")]
fn cell_form(ui: &mut Ui, st: &mut PreviewState) {
    use glowkit::components::{Checkbox, Input, Switch};
    ui.add(Input::new(&mut st.email).placeholder("you@example.com").label("Email").id_salt("tb_email"));
    ui.add_space(4.0);
    ui.add(Switch::new(&mut st.switch).label("Notifications"));
    ui.add(Checkbox::new(&mut st.check).label("Remember me"));
    ui.add(egui::Slider::new(&mut st.slider, 0.0..=1.0).show_value(false));
}

#[cfg(feature = "components")]
fn cell_progress(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::components::{ProgressRing, Spinner, SpinnerKind};
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        ui.add(ProgressRing::new(0.72).size(86.0).thickness(8.0).gradient(true));
        ui.add(Spinner::new(SpinnerKind::Orbit).size(36.0));
        ui.add(Spinner::new(SpinnerKind::Gradient).size(36.0));
    });
}

#[cfg(feature = "components")]
fn cell_badges(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::components::{Badge, BadgeVariant};
    let t = Theme::of(ui);
    ui.horizontal_wrapped(|ui| {
        ui.add(Badge::new("Default"));
        ui.add(Badge::new("Secondary").variant(BadgeVariant::Secondary));
        ui.add(Badge::new("Outline").variant(BadgeVariant::Outline));
        ui.add(Badge::new("Success").color(t.success));
        ui.add(Badge::new("Warning").color(t.warning));
        ui.add(Badge::new("Info").color(t.info));
        ui.add(Badge::new("Error").variant(BadgeVariant::Destructive));
    });
}

#[cfg(feature = "ai")]
fn cell_chat(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::ai::{ChatMessage, Role};
    ui.add(ChatMessage::new(Role::User, "Make the dashboard feel calmer.").id_salt("tb_q"));
    ui.add(
        ChatMessage::new(Role::Assistant, "Softer surfaces, one accent and a bit more radius.")
            .avatar(true)
            .id_salt("tb_a"),
    );
}

#[cfg(feature = "ai")]
fn cell_prompt(ui: &mut Ui, st: &mut PreviewState) {
    use glowkit::ai::PromptInput;
    let _ =
        PromptInput::new(&mut st.prompt).placeholder("Ask anything…").min_rows(1).id_salt("tb_prompt").show(ui, |_| {});
}

#[cfg(feature = "ai")]
fn cell_context(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::ai::{ContextMeter, Loader, LoaderKind, MeterStyle, TypingIndicator};
    ui.add(
        ContextMeter::new(48_200, 128_000).segment("Input", 30_100).segment("Output", 18_100).style(MeterStyle::Bar),
    );
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.add(TypingIndicator::new().bubble(true));
        ui.add(Loader::new(LoaderKind::Wave));
    });
}

#[cfg(feature = "templates")]
fn cell_template(ui: &mut Ui, _: &mut PreviewState) {
    use glowkit::templates::{AuthLayout, LoginPage};
    let rect = ui.max_rect();
    let scale = 0.36;
    let ctx = ui.ctx().clone();
    let translation = rect.min.to_vec2() - rect.min.to_vec2() * scale;
    let clip = rect.intersect(ui.clip_rect());
    let to_layer = |p: egui::Pos2| pos2((p.x - translation.x) / scale, (p.y - translation.y) / scale);
    let clip_layer = Rect::from_min_max(to_layer(clip.min), to_layer(clip.max));
    let virt = rect.size() / scale;
    let style = ctx.global_style();
    let resp = egui::Area::new(Id::unique("gk_tb_template"))
        .order(Order::Middle)
        .fixed_pos(rect.min)
        .constrain(false)
        .interactable(false)
        .show(&ctx, |ui| {
            ui.set_style(style);
            ui.set_clip_rect(clip_layer);
            ui.set_width(virt.x);
            LoginPage::new("Glowkit").layout(AuthLayout::Split).height(virt.y).id_salt("tb_login").show(ui);
        });
    ctx.set_transform_layer(resp.response.layer_id, TSTransform::new(translation, scale));
    ui.allocate_rect(rect, Sense::hover());
}
