//! Main menu, pause overlay and the settings panel.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{follow, spring};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;

// ---------------------------------------------------------------------------------------------
// Shared menu list

struct ListOut {
    hovered: Option<usize>,
    clicked: Option<usize>,
}

/// Paint a vertical list of menu entries starting at `top_left`. Hover / selection slides the
/// entry right with a spring and draws a style-specific selection marker.
#[allow(clippy::too_many_arguments)]
fn menu_list(
    ui: &Ui,
    id: Id,
    items: &[&str],
    selected: usize,
    top_left: Pos2,
    width: f32,
    row_h: f32,
    font: &FontId,
    style: GameStyle,
    pal: &GamePalette,
    appear: f32,
    on_scene: bool,
) -> ListOut {
    let mut out = ListOut { hovered: None, clicked: None };
    let painter = ui.painter();
    let (since_confirm, _, _) = changed(ui, id.with("confirm_n"), glowkit_core::state(ui, id.with("confirm_cnt"), || 0.0f32));
    let confirm_idx: usize = glowkit_core::state(ui, id.with("confirm_i"), || usize::MAX);
    for (i, label) in items.iter().enumerate() {
        let r = Rect::from_min_size(top_left + vec2(0.0, i as f32 * row_h), vec2(width, row_h));
        let resp = ui.interact(r, id.with(("item", i)), Sense::click());
        if resp.hovered() {
            out.hovered = Some(i);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            out.clicked = Some(i);
        }
        let on = i == selected;
        let k = spring(ui, id.with(("slide", i)), if on { 1.0 } else { 0.0 }, 380.0, 20.0);
        let a = glowkit_core::anim::stagger(appear, i, 0.07, 0.35);
        if a <= 0.0 {
            continue;
        }
        let ea = glowkit_core::ease(glowkit_core::Easing::OutCubic, a);
        let x = r.min.x + 22.0 * k - (1.0 - ea) * 40.0;
        let cy = r.center().y;
        let flash = if confirm_idx == i && since_confirm < 0.5 { 1.0 - since_confirm / 0.5 } else { 0.0 };
        // selection marker
        if k > 0.01 {
            let band = Rect::from_min_max(pos2(r.min.x, r.min.y + 3.0), pos2(r.min.x + width * k.min(1.0), r.max.y - 3.0));
            match style {
                GameStyle::SciFi => {
                    paint::gradient_rect(painter, band, 0.0, 0.0, &[with_alpha(pal.accent, 0.35 * k * ea), with_alpha(pal.accent, 0.0)]);
                    painter.rect_filled(Rect::from_min_size(band.min, vec2(3.0, band.height())), 0.0, with_alpha(pal.accent, k * ea));
                    let cx = r.min.x + 10.0;
                    for j in 0..2 {
                        let o = j as f32 * 6.0 + (glowkit_core::time(ui) * 3.0).sin().max(0.0) * 2.0;
                        painter.add(Shape::line(vec![pos2(cx + o, cy - 5.0), pos2(cx + 5.0 + o, cy), pos2(cx + o, cy + 5.0)], Stroke::new(2.0, with_alpha(pal.accent, k * ea * (1.0 - j as f32 * 0.4)))));
                    }
                }
                GameStyle::Fantasy => {
                    let ribbon = band.shrink2(vec2(0.0, 2.0));
                    paint::gradient_rect(painter, ribbon, 3.0, 0.0, &[with_alpha(hex("#8E2F25"), 0.85 * k * ea), with_alpha(hex("#8E2F25"), 0.0)]);
                    let tip = pos2(r.min.x + 12.0, cy);
                    let d = 6.0 * k;
                    painter.add(Shape::convex_polygon(vec![tip + vec2(-d, -d), tip + vec2(d, 0.0), tip + vec2(-d, d)], with_alpha(pal.accent, ea), Stroke::NONE));
                }
                GameStyle::Minimal => {
                    painter.rect_filled(band, 8.0, with_alpha(Color32::WHITE, 0.08 * k * ea));
                    painter.circle_filled(pos2(r.min.x + 10.0, cy), 3.0 * k, with_alpha(pal.accent, ea));
                }
            }
        }
        let base_c = if on_scene { pal.hud } else { pal.text };
        let col = mix(with_alpha(base_c, 0.62), if style == GameStyle::Fantasy && !on_scene { hex("#FBEFD2") } else { Color32::WHITE }, k);
        let col = mix(col, pal.accent, flash);
        let fsize = font.size * (1.0 + 0.06 * k);
        let f = FontId::new(fsize, font.family.clone());
        let pos = pos2(x + 8.0, cy);
        if on_scene {
            let glow = with_alpha(pal.accent, 0.55 * k * ea);
            let g = painter.layout_no_wrap(label.to_string(), f, with_alpha(col, ea));
            let rr = Align2::LEFT_CENTER.anchor_size(pos, g.size());
            if glow.a() > 0 {
                blur(painter, rr.min, &g, glow, 4.0);
            }
            painter.galley_with_override_text_color(rr.min + vec2(1.0, 2.0), g.clone(), with_alpha(Color32::BLACK, 0.5 * ea));
            painter.galley(rr.min, g, col);
        } else {
            painter.text(pos, Align2::LEFT_CENTER, *label, f, with_alpha(col, ea));
        }
    }
    out
}

fn note_confirm(ui: &Ui, id: Id, i: usize) {
    let c: f32 = glowkit_core::state(ui, id.with("confirm_cnt"), || 0.0f32);
    glowkit_core::set_state(ui, id.with("confirm_cnt"), c + 1.0);
    glowkit_core::set_state(ui, id.with("confirm_i"), i);
}

fn nav_keys(ui: &Ui, sel: &mut usize, n: usize) -> bool {
    let (up, down, enter) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::Enter)));
    if up {
        *sel = (*sel + n - 1) % n;
    }
    if down {
        *sel = (*sel + 1) % n;
    }
    enter
}

// ---------------------------------------------------------------------------------------------
// Main menu

/// A title-screen main menu over a parallax game scene. The title letters drop in one by one on
/// springs and then breathe with a glow (with a glitch flicker in the sci-fi skin); menu entries
/// stagger in from the left, and the hovered / selected entry slides right behind a style
/// marker. The scene layers follow the pointer for parallax. Arrow keys + Enter work too, and
/// every hover / confirm is reported as a [`GameCue`] for your sound effects.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut sel = 0usize;
/// let mut chosen = None;
/// ui.add(glowkit_interactions::GameMainMenu::new(&mut sel, &["Continue", "New Game", "Settings", "Quit"])
///     .title("Astral Drift")
///     .activated(&mut chosen));
/// ```
pub struct GameMainMenu<'a> {
    selected: &'a mut usize,
    items: &'a [&'a str],
    title: &'a str,
    subtitle: &'a str,
    version: &'a str,
    style: GameStyle,
    palette: Option<GamePalette>,
    height: f32,
    autoplay: bool,
    activated: Option<&'a mut Option<usize>>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameMainMenu<'a> {
    /// A menu over `items`, editing the `selected` index.
    pub fn new(selected: &'a mut usize, items: &'a [&'a str]) -> Self {
        Self {
            selected,
            items,
            title: "Astral Drift",
            subtitle: "",
            version: "",
            style: GameStyle::SciFi,
            palette: None,
            height: 420.0,
            autoplay: false,
            activated: None,
            cues: None,
        }
    }
    /// Game title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Small line under the title.
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = s;
        self
    }
    /// Version string in the corner.
    pub fn version(mut self, v: &'a str) -> Self {
        self.version = v;
        self
    }
    /// Visual style (also picks the backdrop scene).
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Height in points (width fills the available space).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(260.0);
        self
    }
    /// Demo mode: move the selection on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Receives the index of the confirmed entry (click or Enter).
    pub fn activated(mut self, a: &'a mut Option<usize>) -> Self {
        self.activated = Some(a);
        self
    }
    /// Collect feedback cues ([`GameCue::Hover`], [`GameCue::Confirm`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

/// Animated title: letters drop in, then glow.
fn title_text(painter: &Painter, center: Pos2, text: &str, font: &FontId, t: f32, style: GameStyle, pal: &GamePalette) {
    let up = text.to_uppercase();
    let chars: Vec<char> = up.chars().collect();
    let widths: Vec<f32> = chars.iter().map(|c| text_w(painter, &c.to_string(), font) + font.size * 0.04).collect();
    let total: f32 = widths.iter().sum();
    let mut x = center.x - total / 2.0;
    let glitch = style == GameStyle::SciFi && (t * 0.7).fract() > 0.93;
    let (fill, glow, outline) = match style {
        GameStyle::SciFi => (Color32::WHITE, pal.accent, with_alpha(hex("#001018"), 0.0)),
        GameStyle::Fantasy => (hex("#FFE7A8"), hex("#FF9A3D"), hex("#3A1C0A")),
        GameStyle::Minimal => (Color32::WHITE, pal.accent, Color32::TRANSPARENT),
    };
    for (i, ch) in chars.iter().enumerate() {
        let k = glowkit_core::anim::stagger(t, i, 0.06, 0.6);
        if k > 0.0 && !ch.is_whitespace() {
            let e = pop(k);
            let y = center.y - (1.0 - e) * 50.0 + (t * 1.6 + i as f32 * 0.5).sin() * 1.5 * (k >= 1.0) as i32 as f32;
            let s = 0.6 + 0.4 * e;
            let a = k.min(1.0);
            let ff = FontId::new(font.size * s, font.family.clone());
            let g = painter.layout_no_wrap(ch.to_string(), ff, fill);
            let p = pos2(x + widths[i] / 2.0, y);
            let rr = Align2::CENTER_CENTER.anchor_size(p, g.size());
            let pulse = 0.55 + 0.25 * (t * 2.0 + i as f32 * 0.3).sin();
            blur(painter, rr.min, &g, with_alpha(glow, pulse * a), font.size * 0.14);
            if outline.a() > 0 {
                for d in [vec2(2.0, 0.0), vec2(-2.0, 0.0), vec2(0.0, 2.0), vec2(0.0, -2.0), vec2(0.0, 4.0)] {
                    painter.galley_with_override_text_color(rr.min + d, g.clone(), with_alpha(outline, a));
                }
            }
            if glitch && i % 3 == (t * 10.0) as usize % 3 {
                painter.galley_with_override_text_color(rr.min + vec2(-3.0, 0.0), g.clone(), with_alpha(pal.accent2, 0.8));
                painter.galley_with_override_text_color(rr.min + vec2(3.0, 0.0), g.clone(), with_alpha(pal.accent, 0.8));
            }
            if style == GameStyle::Fantasy {
                paint::galley_with(painter, rr.min, &g, |q| mix(hex("#FFF4CC"), hex("#E0A43A"), ((q.y - rr.min.y) / rr.height()).clamp(0.0, 1.0)));
            } else {
                painter.galley(rr.min, g, with_alpha(fill, a));
            }
        }
        x += widths[i];
    }
}

impl egui::Widget for GameMainMenu<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title_f = display_font(ui, (self.height * 0.13).clamp(30.0, 64.0));
        let item_f = display_font(ui, 20.0);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        glowkit_core::animate(ui);
        let t = glowkit_core::anim::Clock::elapsed(ui, id.with("clock"));
        let time = glowkit_core::time(ui);
        let n = self.items.len().max(1);
        // parallax from the pointer
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let target = ptr.map(|p| (p - rect.center()) / (rect.size() / 2.0)).unwrap_or(Vec2::ZERO);
        let px = vec2(follow(ui, id.with("px"), target.x, 4.0), follow(ui, id.with("py"), target.y, 4.0));
        GameBackdrop::new(self.style).palette(pal).parallax(px).speed(0.35).paint_at(ui.painter(), rect, time * 0.35);
        // left shade for legibility
        paint::gradient_rect(ui.painter(), Rect::from_min_max(rect.min, pos2(rect.min.x + w * 0.55, rect.max.y)), 18.0, 0.0, &[Color32::from_black_alpha(150), Color32::TRANSPARENT]);
        let wide = w > 620.0;
        let title_c = if wide { pos2(rect.min.x + w * 0.3, rect.min.y + self.height * 0.2) } else { pos2(rect.center().x, rect.min.y + self.height * 0.18) };
        title_text(ui.painter(), title_c - px * 6.0, self.title, &title_f, t, self.style, &pal);
        if !self.subtitle.is_empty() {
            let a = ((t - 0.9) / 0.5).clamp(0.0, 1.0);
            let sf = semi_font(ui, 13.0);
            outlined(ui.painter(), title_c + vec2(0.0, title_f.size * 0.62), Align2::CENTER_CENTER, &self.subtitle.to_uppercase(), sf, with_alpha(pal.hud, 0.8 * a), with_alpha(Color32::BLACK, 0.5 * a));
        }
        let row_h = 40.0;
        let list_w = 260.0f32.min(w - 40.0);
        let list_tl = if wide {
            pos2(rect.min.x + w * 0.3 - list_w / 2.0, rect.min.y + self.height * 0.38)
        } else {
            pos2(rect.center().x - list_w / 2.0, rect.min.y + self.height * 0.34)
        };
        let mut sel = (*self.selected).min(n - 1);
        let enter = nav_keys(ui, &mut sel, n);
        let out = menu_list(ui, id, self.items, sel, list_tl, list_w, row_h, &item_f, self.style, &pal, (t - 0.5).max(0.0), true);
        let idle = idle_for(ui, id, out.hovered.is_some() || enter);
        if let Some(h) = out.hovered {
            sel = h;
        } else if self.autoplay && idle > 2.0 {
            let k = ((idle - 2.0) / 1.4) as usize;
            sel = [0, 1, 2, 1][k % 4].min(n - 1);
        }
        if sel != *self.selected {
            *self.selected = sel;
            push_cue(&mut self.cues, GameCue::Hover);
        }
        let confirm = out.clicked.or(if enter { Some(sel) } else { None });
        if let Some(c) = confirm {
            note_confirm(ui, id, c);
            push_cue(&mut self.cues, GameCue::Confirm);
            if let Some(a) = self.activated.as_deref_mut() {
                *a = Some(c);
            }
        }
        let painter = ui.painter();
        let foot = rect.max.y - 20.0;
        let hint_a = 0.5 + 0.3 * (time * 2.5).sin();
        let hints = "↑↓  NAVIGATE     ⏎  SELECT";
        painter.text(pos2(rect.max.x - 22.0, foot), Align2::RIGHT_CENTER, hints, FontId::monospace(11.0), with_alpha(pal.hud, hint_a));
        if !self.version.is_empty() {
            painter.text(pos2(rect.min.x + 22.0, foot), Align2::LEFT_CENTER, self.version, FontId::monospace(11.0), with_alpha(pal.hud, 0.5));
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Pause menu

/// A pause overlay. When `open` turns on, the scene dims and darkens toward the edges, a panel
/// pops in on a spring with a big PAUSED heading, optional info rows (chapter, play time) and
/// the menu entries staggering in; closing reverses it. Draw it over your game view.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut sel = 0usize;
/// ui.add(glowkit_interactions::GamePauseMenu::new(true, &mut sel, &["Resume", "Settings", "Quit to Title"])
///     .info(&[("Chapter", "III: The Sunken Vault"), ("Play time", "12h 04m")]));
/// ```
pub struct GamePauseMenu<'a> {
    open: bool,
    selected: &'a mut usize,
    items: &'a [&'a str],
    info: &'a [(&'a str, &'a str)],
    style: GameStyle,
    palette: Option<GamePalette>,
    height: f32,
    backdrop: bool,
    activated: Option<&'a mut Option<usize>>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GamePauseMenu<'a> {
    /// A pause menu (shown while `open`) over `items`.
    pub fn new(open: bool, selected: &'a mut usize, items: &'a [&'a str]) -> Self {
        Self { open, selected, items, info: &[], style: GameStyle::SciFi, palette: None, height: 400.0, backdrop: false, activated: None, cues: None }
    }
    /// Label / value rows shown above the menu.
    pub fn info(mut self, rows: &'a [(&'a str, &'a str)]) -> Self {
        self.info = rows;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Height in points (width fills the available space).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(240.0);
        self
    }
    /// Paint a stand-in [`GameBackdrop`] scene underneath (for previews).
    pub fn backdrop(mut self, on: bool) -> Self {
        self.backdrop = on;
        self
    }
    /// Receives the index of the confirmed entry.
    pub fn activated(mut self, a: &'a mut Option<usize>) -> Self {
        self.activated = Some(a);
        self
    }
    /// Collect feedback cues.
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GamePauseMenu<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let head_f = display_font(ui, 34.0);
        let item_f = display_font(ui, 18.0);
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let open_k = spring(ui, id.with("open"), if self.open { 1.0 } else { 0.0 }, 260.0, 20.0);
        let (since_open, _, _) = changed(ui, id.with("open_t"), if self.open { 1.0 } else { 0.0 });
        if self.backdrop {
            glowkit_core::animate(ui);
            // the game keeps running behind (slows to a halt while paused)
            let speed_t: f32 = glowkit_core::with_state(ui, id.with("scene_t"), || 0.0f32, |s| {
                *s += (1.0 - open_k.clamp(0.0, 1.0)) * 0.016;
                *s
            });
            GameBackdrop::new(self.style).palette(pal).dim(0.0).paint_at(ui.painter(), rect, speed_t * 1.5 + 3.0);
        }
        let painter = ui.painter();
        let ok = open_k.clamp(0.0, 1.0);
        if ok > 0.001 {
            painter.rect_filled(rect, 18.0, Color32::from_black_alpha((150.0 * ok) as u8));
        }
        let mut sel = (*self.selected).min(self.items.len().max(1) - 1);
        if open_k > 0.02 {
            let pw = 340.0f32.min(w - 32.0);
            let ph = 92.0 + self.info.len() as f32 * 22.0 + self.items.len() as f32 * 42.0 + 30.0;
            let scale = 0.85 + 0.15 * open_k;
            let center = rect.center() + vec2(0.0, (1.0 - open_k) * 30.0);
            let panel = Rect::from_center_size(center, vec2(pw, ph) * scale);
            let mut p2 = painter.clone();
            p2.set_opacity(ok);
            frame(&p2, panel, self.style, &pal, 0.4);
            let tc = pal.text;
            glow_text(&p2, pos2(panel.center().x, panel.min.y + 40.0), Align2::CENTER_CENTER, "PAUSED", head_f, tc, with_alpha(pal.accent, if self.style == GameStyle::Fantasy { 0.0 } else { 0.5 }));
            let mut y = panel.min.y + 74.0;
            for (k, v) in self.info {
                p2.text(pos2(panel.min.x + 24.0, y), Align2::LEFT_CENTER, k.to_uppercase(), FontId::monospace(10.5), pal.dim);
                p2.text(pos2(panel.max.x - 24.0, y), Align2::RIGHT_CENTER, *v, FontId::proportional(12.5), tc);
                y += 22.0;
            }
            p2.line_segment([pos2(panel.min.x + 20.0, y), pos2(panel.max.x - 20.0, y)], Stroke::new(1.0, with_alpha(pal.dim, 0.35)));
            if self.open {
                let enter = nav_keys(ui, &mut sel, self.items.len().max(1));
                let out = menu_list(ui, id, self.items, sel, pos2(panel.min.x + 18.0, y + 10.0), pw - 36.0, 42.0, &item_f, self.style, &pal, (since_open - 0.1).max(0.0), false);
                if let Some(h) = out.hovered {
                    sel = h;
                }
                if let Some(c) = out.clicked.or(if enter { Some(sel) } else { None }) {
                    note_confirm(ui, id, c);
                    push_cue(&mut self.cues, GameCue::Confirm);
                    if let Some(a) = self.activated.as_deref_mut() {
                        *a = Some(c);
                    }
                }
            }
        }
        if sel != *self.selected {
            *self.selected = sel;
            push_cue(&mut self.cues, GameCue::Hover);
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Settings

/// The data behind a [`GameSettingsPanel`]. Keep one in your game, load / save it however you
/// like and apply the values when they change.
#[derive(Clone, Debug, PartialEq)]
pub struct GameSettings {
    /// Graphics preset index into [`GameSettings::QUALITY`].
    pub quality: usize,
    /// Render scale in percent.
    pub render_scale: f32,
    /// Field of view in degrees.
    pub fov: f32,
    /// Vertical sync.
    pub vsync: bool,
    /// Bloom post effect.
    pub bloom: bool,
    /// Motion blur.
    pub motion_blur: bool,
    /// Master volume 0..100.
    pub master: f32,
    /// Music volume 0..100.
    pub music: f32,
    /// Effects volume 0..100.
    pub sfx: f32,
    /// Voice volume 0..100.
    pub voice: f32,
    /// Show subtitles.
    pub subtitles: bool,
    /// Mouse sensitivity 0.1..5.
    pub sensitivity: f32,
    /// Invert the Y axis.
    pub invert_y: bool,
    /// Key bindings: (action, key name).
    pub bindings: Vec<(String, String)>,
}

impl GameSettings {
    /// Names of the quality presets.
    pub const QUALITY: [&'static str; 5] = ["Low", "Medium", "High", "Ultra", "Cinematic"];
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            quality: 2,
            render_scale: 100.0,
            fov: 90.0,
            vsync: true,
            bloom: true,
            motion_blur: false,
            master: 80.0,
            music: 60.0,
            sfx: 75.0,
            voice: 90.0,
            subtitles: true,
            sensitivity: 1.6,
            invert_y: false,
            bindings: [("Move forward", "W"), ("Move back", "S"), ("Jump", "Space"), ("Interact", "E"), ("Reload", "R"), ("Map", "M")]
                .iter()
                .map(|(a, k)| (a.to_string(), k.to_string()))
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SetState {
    tab: usize,
    prev_tab: usize,
    tab_at: f64,
    rebinding: Option<usize>,
}

/// A tabbed game settings panel: Graphics, Audio and Controls. The active tab's underline glides
/// on a spring and content slides in from the side you moved toward. Rows include segmented
/// sliders (drag or scroll), springy toggles, ◀ value ▶ selectors and key rebind rows: click a
/// key cap, it pulses "press a key", and the next key press is captured (Esc cancels; duplicate
/// bindings turn red).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut settings = glowkit_interactions::GameSettings::default();
/// ui.add(glowkit_interactions::GameSettingsPanel::new(&mut settings));
/// ```
pub struct GameSettingsPanel<'a> {
    settings: &'a mut GameSettings,
    style: GameStyle,
    palette: Option<GamePalette>,
    width: f32,
    autoplay: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameSettingsPanel<'a> {
    /// A panel editing `settings`.
    pub fn new(settings: &'a mut GameSettings) -> Self {
        Self { settings, style: GameStyle::SciFi, palette: None, width: 600.0, autoplay: false, cues: None }
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Maximum width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }
    /// Demo mode: flip through tabs while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Tick`] on changes).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

enum RowKind<'b> {
    Slider(&'b mut f32, f32, f32, &'static str, usize),
    Toggle(&'b mut bool),
    Select(&'b mut usize, &'static [&'static str]),
}

struct RowCtx<'p> {
    pal: GamePalette,
    style: GameStyle,
    painter: &'p Painter,
    font: FontId,
}

/// One labelled settings row. Returns true if the value changed.
fn row(ui: &Ui, rc: &RowCtx, id: Id, rect: Rect, label: &str, kind: RowKind, alpha: f32) -> bool {
    let p = rc.painter;
    let pal = &rc.pal;
    let hover = ui.interact(rect, id.with("row"), Sense::hover()).hovered();
    let hk = follow(ui, id.with("hk"), if hover { 1.0 } else { 0.0 }, 16.0);
    if hk > 0.01 {
        p.rect_filled(rect, 4.0, with_alpha(pal.accent, 0.08 * hk * alpha));
    }
    p.text(pos2(rect.min.x + 14.0, rect.center().y), Align2::LEFT_CENTER, label, rc.font.clone(), with_alpha(pal.text, alpha));
    let ctrl = Rect::from_min_max(pos2(rect.max.x - 250.0f32.min(rect.width() * 0.55), rect.min.y), pos2(rect.max.x - 14.0, rect.max.y));
    let mut changed_v = false;
    match kind {
        RowKind::Slider(v, lo, hi, suffix, decimals) => {
            let track = Rect::from_min_max(pos2(ctrl.min.x, ctrl.center().y - 5.0), pos2(ctrl.max.x - 52.0, ctrl.center().y + 5.0));
            let r = ui.interact(track.expand2(vec2(6.0, 8.0)), id.with("sl"), Sense::click_and_drag());
            let old = *v;
            if r.dragged() || r.clicked() {
                if let Some(pp) = r.interact_pointer_pos() {
                    let k = ((pp.x - track.min.x) / track.width()).clamp(0.0, 1.0);
                    *v = lo + (hi - lo) * k;
                }
            }
            if r.hovered() {
                let sc = ui.input(|i| i.smooth_scroll_delta.y);
                if sc.abs() > 0.5 {
                    *v = (*v + sc.signum() * (hi - lo) / 50.0).clamp(lo, hi);
                }
            }
            let m = 10f32.powi(decimals as i32);
            *v = (*v * m).round() / m;
            changed_v = (old - *v).abs() > 1e-6 && ((old - lo) / (hi - lo) * 20.0).floor() != ((*v - lo) / (hi - lo) * 20.0).floor();
            let k = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0);
            let shown = follow(ui, id.with("shown"), k, 22.0);
            let segs = 20;
            let sw = track.width() / segs as f32;
            for i in 0..segs {
                let sr = Rect::from_min_size(pos2(track.min.x + i as f32 * sw + 1.0, track.min.y), vec2(sw - 2.0, track.height()));
                let on = (i as f32 + 0.5) / segs as f32 <= shown;
                let c = if on {
                    mix(pal.accent, if rc.style == GameStyle::Fantasy { pal.accent2 } else { Color32::WHITE }, 0.25 * (i as f32 / segs as f32))
                } else {
                    with_alpha(pal.dim, 0.25)
                };
                let skew = if rc.style == GameStyle::SciFi { 3.0 } else { 0.0 };
                p.add(Shape::convex_polygon(
                    vec![sr.left_bottom(), sr.left_top() + vec2(skew, 0.0), sr.right_top() + vec2(skew, 0.0), sr.right_bottom()],
                    with_alpha(c, alpha),
                    Stroke::NONE,
                ));
            }
            let kx = track.min.x + track.width() * shown;
            let active = r.dragged() || r.hovered();
            let ks = spring(ui, id.with("ks"), if active { 1.3 } else { 1.0 }, 500.0, 18.0);
            let kc = pos2(kx, track.center().y);
            if active {
                paint::glow(p, kc, 16.0, with_alpha(pal.accent, 0.5 * alpha));
            }
            let d = 7.0 * ks;
            p.add(Shape::convex_polygon(vec![kc + vec2(0.0, -d), kc + vec2(d * 0.8, 0.0), kc + vec2(0.0, d), kc + vec2(-d * 0.8, 0.0)], with_alpha(Color32::WHITE, alpha), Stroke::new(1.5, with_alpha(pal.accent, alpha))));
            let txt = if decimals == 0 { format!("{}{suffix}", v.round() as i64) } else { format!("{:.*}{suffix}", decimals, *v) };
            p.text(pos2(ctrl.max.x, ctrl.center().y), Align2::RIGHT_CENTER, txt, FontId::monospace(12.5), with_alpha(pal.text, alpha));
        }
        RowKind::Toggle(b) => {
            let tr = Rect::from_min_size(pos2(ctrl.max.x - 52.0, ctrl.center().y - 12.0), vec2(52.0, 24.0));
            let r = ui.interact(tr.expand(4.0), id.with("tg"), Sense::click());
            if r.clicked() {
                *b = !*b;
                changed_v = true;
            }
            let k = spring(ui, id.with("tgk"), if *b { 1.0 } else { 0.0 }, 420.0, 20.0);
            let bg = mix(with_alpha(pal.dim, 0.3), pal.accent, k.clamp(0.0, 1.0));
            let rad = if rc.style == GameStyle::SciFi { 3.0 } else { 12.0 };
            p.rect_filled(tr, rad, with_alpha(bg, alpha));
            let kx = tr.min.x + 12.0 + (tr.width() - 24.0) * k;
            let stretch = (1.0 - (k - 0.5).abs() * 2.0).max(0.0) * 6.0;
            let knob = Rect::from_center_size(pos2(kx, tr.center().y), vec2(18.0 + stretch, 18.0));
            p.rect_filled(knob, if rc.style == GameStyle::SciFi { 2.0 } else { 9.0 }, with_alpha(Color32::WHITE, alpha));
            p.text(pos2(tr.min.x - 10.0, tr.center().y), Align2::RIGHT_CENTER, if *b { "ON" } else { "OFF" }, FontId::monospace(11.0), with_alpha(if *b { pal.accent } else { pal.dim }, alpha));
        }
        RowKind::Select(i, opts) => {
            let n = opts.len().max(1);
            let left = Rect::from_center_size(pos2(ctrl.min.x + 12.0, ctrl.center().y), Vec2::splat(24.0));
            let right = Rect::from_center_size(pos2(ctrl.max.x - 12.0, ctrl.center().y), Vec2::splat(24.0));
            let lr = ui.interact(left, id.with("l"), Sense::click());
            let rr = ui.interact(right, id.with("r"), Sense::click());
            let mut dir = 0.0;
            if lr.clicked() {
                *i = (*i + n - 1) % n;
                dir = -1.0;
            }
            if rr.clicked() {
                *i = (*i + 1) % n;
                dir = 1.0;
            }
            if dir != 0.0 {
                changed_v = true;
                glowkit_core::set_state(ui, id.with("dir"), dir);
            }
            let (since, _, _) = changed(ui, id.with("sel"), *i as f32);
            let d: f32 = glowkit_core::state(ui, id.with("dir"), || 1.0f32);
            let k = glowkit_core::ease(glowkit_core::Easing::OutBack, (since / 0.3).min(1.0));
            for (r, pts_dir, hov) in [(left, -1.0, lr.hovered()), (right, 1.0, rr.hovered())] {
                let c = r.center();
                let s = if hov { 7.0 } else { 6.0 };
                let col = if hov { pal.accent } else { pal.dim };
                p.add(Shape::convex_polygon(vec![c + vec2(pts_dir * s, 0.0), c + vec2(-pts_dir * s * 0.7, -s), c + vec2(-pts_dir * s * 0.7, s)], with_alpha(col, alpha), Stroke::NONE));
            }
            let mid = ctrl.center() + vec2((1.0 - k) * 24.0 * d, 0.0);
            p.text(mid, Align2::CENTER_CENTER, opts[(*i).min(n - 1)].to_uppercase(), FontId::proportional(13.5), with_alpha(pal.text, alpha * k.clamp(0.0, 1.0)));
            // pips
            let pw = 10.0;
            let x0 = ctrl.center().x - n as f32 * pw / 2.0;
            for j in 0..n {
                let c = pos2(x0 + (j as f32 + 0.5) * pw, ctrl.max.y - 6.0);
                p.rect_filled(Rect::from_center_size(c, vec2(6.0, 2.0)), 1.0, with_alpha(if j == *i { pal.accent } else { pal.dim }, alpha * if j == *i { 1.0 } else { 0.35 }));
            }
            if since < 0.35 {
                ui.ctx().request_repaint();
            }
        }
    }
    changed_v
}

impl egui::Widget for GameSettingsPanel<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let tab_f = display_font(ui, 15.0);
        let row_f = semi_font(ui, 13.5);
        let title_f = display_font(ui, 20.0);
        let w = self.width.min(ui.available_width());
        let row_h = 40.0;
        let h = 92.0 + 7.0 * row_h + 44.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let mut st: SetState = glowkit_core::state(ui, id.with("st"), SetState::default);
        let painter = ui.painter().clone();
        frame(&painter, rect, self.style, &pal, 0.0);
        painter.text(pos2(rect.min.x + 22.0, rect.min.y + 26.0), Align2::LEFT_CENTER, "SETTINGS", title_f, pal.text);
        // tabs
        let tabs = [("Graphics", GameIcon::Star), ("Audio", GameIcon::Bolt), ("Controls", GameIcon::Target)];
        let tw = (w - 44.0) / 3.0;
        let tab_y = rect.min.y + 52.0;
        let mut any = false;
        for (i, (name, icon)) in tabs.iter().enumerate() {
            let tr = Rect::from_min_size(pos2(rect.min.x + 22.0 + i as f32 * tw, tab_y), vec2(tw, 30.0));
            let r = ui.interact(tr, id.with(("tab", i)), Sense::click());
            any |= r.hovered();
            if r.clicked() && st.tab != i {
                st.prev_tab = st.tab;
                st.tab = i;
                st.tab_at = now;
                st.rebinding = None;
                push_cue(&mut self.cues, GameCue::Tick);
            }
            let on = st.tab == i;
            let c = if on { pal.text } else if r.hovered() { mix(pal.dim, pal.text, 0.5) } else { pal.dim };
            let gw = text_w(&painter, &name.to_uppercase(), &tab_f) + 22.0;
            let x = tr.center().x - gw / 2.0;
            icon.paint(&painter, pos2(x + 7.0, tr.center().y), 14.0, if on { pal.accent } else { c });
            painter.text(pos2(x + 20.0, tr.center().y), Align2::LEFT_CENTER, name.to_uppercase(), tab_f.clone(), c);
        }
        let idle = idle_for(ui, id, any || resp.hovered() && ui.input(|i| i.pointer.any_down()));
        if self.autoplay && idle > 3.0 && st.rebinding.is_none() {
            let want = (((idle - 3.0) / 2.8) as usize + 1) % 3;
            if want != st.tab {
                st.prev_tab = st.tab;
                st.tab = want;
                st.tab_at = now;
            }
            ui.ctx().request_repaint();
        }
        let ux = spring(ui, id.with("ux"), st.tab as f32, 300.0, 22.0);
        let under = Rect::from_min_size(pos2(rect.min.x + 22.0 + ux * tw + 12.0, tab_y + 31.0), vec2(tw - 24.0, 3.0));
        painter.line_segment([pos2(rect.min.x + 22.0, tab_y + 32.5), pos2(rect.max.x - 22.0, tab_y + 32.5)], Stroke::new(1.0, with_alpha(pal.dim, 0.3)));
        paint::glow_ellipse(&painter, under.center(), vec2(under.width() * 0.6, 8.0), with_alpha(pal.accent, 0.45));
        painter.rect_filled(under, 1.5, pal.accent);
        // content transition
        let k = ((now - st.tab_at) as f32 / 0.35).clamp(0.0, 1.0);
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, k);
        let dir = if st.tab >= st.prev_tab { 1.0 } else { -1.0 };
        let off = (1.0 - e) * 40.0 * dir;
        let body = Rect::from_min_max(pos2(rect.min.x + 12.0, tab_y + 44.0), pos2(rect.max.x - 12.0, rect.max.y - 40.0));
        let clip_p = painter.with_clip_rect(body.expand2(vec2(0.0, 4.0)));
        let rc = RowCtx { pal, style: self.style, painter: &clip_p, font: row_f };
        let mut ch = false;
        let rows_at = |i: usize| Rect::from_min_size(pos2(body.min.x + off, body.min.y + i as f32 * row_h), vec2(body.width(), row_h - 4.0));
        let s = &mut *self.settings;
        let tid = id.with(("tab_body", st.tab));
        match st.tab {
            0 => {
                ch |= row(ui, &rc, tid.with(0), rows_at(0), "Quality preset", RowKind::Select(&mut s.quality, &GameSettings::QUALITY), e);
                ch |= row(ui, &rc, tid.with(1), rows_at(1), "Render scale", RowKind::Slider(&mut s.render_scale, 50.0, 200.0, "%", 0), e);
                ch |= row(ui, &rc, tid.with(2), rows_at(2), "Field of view", RowKind::Slider(&mut s.fov, 60.0, 120.0, "°", 0), e);
                ch |= row(ui, &rc, tid.with(3), rows_at(3), "V-Sync", RowKind::Toggle(&mut s.vsync), e);
                ch |= row(ui, &rc, tid.with(4), rows_at(4), "Bloom", RowKind::Toggle(&mut s.bloom), e);
                ch |= row(ui, &rc, tid.with(5), rows_at(5), "Motion blur", RowKind::Toggle(&mut s.motion_blur), e);
            }
            1 => {
                ch |= row(ui, &rc, tid.with(0), rows_at(0), "Master volume", RowKind::Slider(&mut s.master, 0.0, 100.0, "", 0), e);
                ch |= row(ui, &rc, tid.with(1), rows_at(1), "Music", RowKind::Slider(&mut s.music, 0.0, 100.0, "", 0), e);
                ch |= row(ui, &rc, tid.with(2), rows_at(2), "Effects", RowKind::Slider(&mut s.sfx, 0.0, 100.0, "", 0), e);
                ch |= row(ui, &rc, tid.with(3), rows_at(3), "Voice", RowKind::Slider(&mut s.voice, 0.0, 100.0, "", 0), e);
                ch |= row(ui, &rc, tid.with(4), rows_at(4), "Subtitles", RowKind::Toggle(&mut s.subtitles), e);
            }
            _ => {
                ch |= row(ui, &rc, tid.with(0), rows_at(0), "Sensitivity", RowKind::Slider(&mut s.sensitivity, 0.1, 5.0, "", 1), e);
                ch |= row(ui, &rc, tid.with(1), rows_at(1), "Invert Y axis", RowKind::Toggle(&mut s.invert_y), e);
                // key rebind rows
                let mut capture: Option<String> = None;
                let mut cancel = false;
                if st.rebinding.is_some() {
                    ui.input(|i| {
                        for ev in &i.events {
                            if let egui::Event::Key { key, pressed: true, .. } = ev {
                                if *key == egui::Key::Escape {
                                    cancel = true;
                                } else {
                                    capture = Some(key.name().to_string());
                                }
                            }
                        }
                    });
                }
                let n_b = s.bindings.len().min(5);
                for bi in 0..n_b {
                    let r = rows_at(2 + bi);
                    let bid = tid.with(("bind", bi));
                    let hover = ui.interact(r, bid.with("row"), Sense::hover()).hovered();
                    if hover {
                        rc.painter.rect_filled(r, 4.0, with_alpha(pal.accent, 0.08 * e));
                    }
                    let (action, key) = s.bindings[bi].clone();
                    rc.painter.text(pos2(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, &action, rc.font.clone(), with_alpha(pal.text, e));
                    let waiting = st.rebinding == Some(bi);
                    let dup = s.bindings.iter().filter(|(_, k)| *k == key).count() > 1;
                    let label = if waiting { "PRESS A KEY".to_string() } else { key.to_uppercase() };
                    let cw = text_w(rc.painter, &label, &FontId::monospace(12.0)).max(18.0) + 20.0;
                    let cap = Rect::from_min_size(pos2(r.max.x - 14.0 - cw, r.center().y - 13.0), vec2(cw, 26.0));
                    let cr = ui.interact(cap, bid.with("cap"), Sense::click());
                    if cr.clicked() {
                        st.rebinding = if waiting { None } else { Some(bi) };
                        push_cue(&mut self.cues, GameCue::Confirm);
                    }
                    let pulse = if waiting { 0.5 + 0.5 * (now as f32 * 8.0).sin() } else { 0.0 };
                    if waiting {
                        ui.ctx().request_repaint();
                        paint::soft_shadow(rc.painter, cap, 4.0, 12.0, Vec2::ZERO, with_alpha(pal.accent, 0.6 * pulse));
                    }
                    let edge = if dup { pal.bad } else if waiting || cr.hovered() { pal.accent } else { with_alpha(pal.dim, 0.6) };
                    let fill = if self.style == GameStyle::Fantasy { hex("#2B1D12") } else { hex("#0B0F18") };
                    rc.painter.rect_filled(cap.translate(vec2(0.0, 2.0)), 4.0, with_alpha(Color32::BLACK, 0.5 * e));
                    rc.painter.rect_filled(cap, 4.0, with_alpha(fill, e));
                    rc.painter.rect_stroke(cap, 4.0, Stroke::new(1.2, with_alpha(edge, e)), egui::StrokeKind::Inside);
                    rc.painter.text(cap.center(), Align2::CENTER_CENTER, label, FontId::monospace(12.0), with_alpha(if waiting { mix(pal.accent, Color32::WHITE, pulse) } else if dup { pal.bad } else { Color32::WHITE }, e));
                    if waiting {
                        if let Some(k) = capture.take() {
                            s.bindings[bi].1 = k;
                            st.rebinding = None;
                            ch = true;
                        } else if cancel {
                            st.rebinding = None;
                            push_cue(&mut self.cues, GameCue::Back);
                        }
                    }
                }
            }
        }
        if ch {
            push_cue(&mut self.cues, GameCue::Tick);
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        // footer
        let fy = rect.max.y - 22.0;
        painter.line_segment([pos2(rect.min.x + 22.0, fy - 16.0), pos2(rect.max.x - 22.0, fy - 16.0)], Stroke::new(1.0, with_alpha(pal.dim, 0.25)));
        let mut x = rect.min.x + 22.0;
        for (k, a) in [("Esc", "Back"), ("Tab", "Switch tab"), ("R", "Reset")] {
            let kw = text_w(&painter, k, &FontId::monospace(10.5)) + 10.0;
            let kr = Rect::from_min_size(pos2(x, fy - 9.0), vec2(kw, 18.0));
            painter.rect_stroke(kr, 3.0, Stroke::new(1.0, with_alpha(pal.dim, 0.7)), egui::StrokeKind::Inside);
            painter.text(kr.center(), Align2::CENTER_CENTER, k, FontId::monospace(10.5), pal.text);
            painter.text(pos2(kr.max.x + 6.0, fy), Align2::LEFT_CENTER, a, FontId::proportional(11.5), pal.dim);
            x = kr.max.x + 6.0 + text_w(&painter, a, &FontId::proportional(11.5)) + 18.0;
        }
        resp
    }
}
