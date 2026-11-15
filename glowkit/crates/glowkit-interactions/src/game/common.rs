//! Shared machinery for the game UI blocks: visual styles and palettes, framed panels (sci-fi
//! chamfers, parchment and gold, clean glass), the painted game viewport backdrop, item rarity,
//! painted icons, outlined text, stateless particle bursts, screen shake and sound-cue hooks.

use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2};
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::paint;
use glowkit_core::rng::hash01;
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Styles

/// The look of a game UI block. Every block takes one, so a whole HUD can switch skins at once.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameResourceBar, GameStyle};
/// ui.add(GameResourceBar::new(640.0, 1000.0).style(GameStyle::Fantasy));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameStyle {
    /// Holographic HUD: chamfered dark-glass panels, cyan edge light, scanlines.
    #[default]
    SciFi,
    /// Fantasy RPG: parchment panels with double gold borders and corner studs.
    Fantasy,
    /// Clean minimal: rounded smoked glass, hairlines, soft shadows.
    Minimal,
}

impl GameStyle {
    /// All styles, in order.
    pub const ALL: [GameStyle; 3] = [GameStyle::SciFi, GameStyle::Fantasy, GameStyle::Minimal];

    /// The default palette for this style.
    pub fn palette(self) -> GamePalette {
        GamePalette::of(self)
    }

    /// A short display name.
    pub fn label(self) -> &'static str {
        match self {
            GameStyle::SciFi => "Sci-fi",
            GameStyle::Fantasy => "Fantasy",
            GameStyle::Minimal => "Minimal",
        }
    }
}

/// The colors a game block paints with. Start from [`GamePalette::of`] and override fields to
/// match your game's art direction, then pass it with each block's `.palette(..)` setter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamePalette {
    /// Panel fill at the top.
    pub panel: Color32,
    /// Panel fill at the bottom.
    pub panel2: Color32,
    /// Frame line color.
    pub edge: Color32,
    /// Primary highlight (selection, glow).
    pub accent: Color32,
    /// Secondary highlight.
    pub accent2: Color32,
    /// Main text on panels.
    pub text: Color32,
    /// Secondary text on panels.
    pub dim: Color32,
    /// Text drawn straight onto the game scene (HUD text).
    pub hud: Color32,
    /// Positive (heal, gain, ready).
    pub good: Color32,
    /// Negative (damage, loss, enemy).
    pub bad: Color32,
    /// Warning (low ammo, crit).
    pub warn: Color32,
    /// Backdrop sky gradient, top to bottom.
    pub sky: [Color32; 3],
}

impl GamePalette {
    /// The built-in palette for `style`.
    pub fn of(style: GameStyle) -> Self {
        match style {
            GameStyle::SciFi => Self {
                panel: hex("#0E2238E6"),
                panel2: hex("#050E1CF0"),
                edge: hex("#3FE0FF"),
                accent: hex("#3FE0FF"),
                accent2: hex("#FF3FA4"),
                text: hex("#E8FBFF"),
                dim: hex("#7FA8BC"),
                hud: hex("#E8FBFF"),
                good: hex("#4BF08F"),
                bad: hex("#FF4560"),
                warn: hex("#FFB224"),
                sky: [hex("#040814"), hex("#0B1736"), hex("#2A1747")],
            },
            GameStyle::Fantasy => Self {
                panel: hex("#F1E2BF"),
                panel2: hex("#D9BF8C"),
                edge: hex("#C99A3A"),
                accent: hex("#E8B64A"),
                accent2: hex("#8E2F25"),
                text: hex("#3A2614"),
                dim: hex("#7A5B36"),
                hud: hex("#FBEFD2"),
                good: hex("#5BBF4A"),
                bad: hex("#C8342B"),
                warn: hex("#F0A830"),
                sky: [hex("#1B1433"), hex("#4A2A4E"), hex("#C2654A")],
            },
            GameStyle::Minimal => Self {
                panel: hex("#1A1C22EE"),
                panel2: hex("#121318F2"),
                edge: hex("#FFFFFF24"),
                accent: hex("#8FA8FF"),
                accent2: hex("#FF8FB1"),
                text: hex("#F4F5F7"),
                dim: hex("#9AA0AC"),
                hud: hex("#F4F5F7"),
                good: hex("#5EE6A0"),
                bad: hex("#FF6B6B"),
                warn: hex("#FFC75F"),
                sky: [hex("#0D0F14"), hex("#161A24"), hex("#232838")],
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sound / feedback cues

/// A feedback event a block emitted this frame. Pass a `Vec` with a block's `.cues(..)` setter
/// and play a sound, rumble the gamepad or shake the camera for each entry.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut sel = 0usize;
/// let mut cues = Vec::new();
/// ui.add(glowkit_interactions::GameMainMenu::new(&mut sel, &["Play", "Quit"]).cues(&mut cues));
/// for cue in cues {
///     // audio.play(match cue { GameCue::Hover => "tick.ogg", .. })
///     let _ = cue;
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameCue {
    /// The pointer or focus moved onto an item.
    Hover,
    /// An item was confirmed / clicked.
    Confirm,
    /// Back / close.
    Back,
    /// A tab, slider notch or option changed.
    Tick,
    /// A character was typed by a typewriter.
    Type,
    /// Damage was taken.
    Hit,
    /// Health was restored.
    Heal,
    /// A shot was fired.
    Shot,
    /// A reload finished.
    Reload,
    /// An ability came off cooldown.
    Ready,
    /// Something was unlocked (skill, achievement).
    Unlock,
    /// Loot was revealed, with its rarity.
    Loot(GameRarity),
    /// An item was picked up or dropped.
    Equip,
    /// An action was refused.
    Error,
}

pub(crate) fn push_cue(cues: &mut Option<&mut Vec<GameCue>>, c: GameCue) {
    if let Some(v) = cues.as_deref_mut() {
        v.push(c);
    }
}

// ---------------------------------------------------------------------------------------------
// Rarity

/// Item rarity tiers, each with a signature color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum GameRarity {
    /// Grey.
    #[default]
    Common,
    /// Green.
    Uncommon,
    /// Blue.
    Rare,
    /// Purple.
    Epic,
    /// Orange, with an animated beam.
    Legendary,
}

impl GameRarity {
    /// Signature color.
    pub fn color(self) -> Color32 {
        match self {
            GameRarity::Common => hex("#C3C8D2"),
            GameRarity::Uncommon => hex("#4BE07A"),
            GameRarity::Rare => hex("#3D9BFF"),
            GameRarity::Epic => hex("#B862FF"),
            GameRarity::Legendary => hex("#FF9A1F"),
        }
    }
    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            GameRarity::Common => "Common",
            GameRarity::Uncommon => "Uncommon",
            GameRarity::Rare => "Rare",
            GameRarity::Epic => "Epic",
            GameRarity::Legendary => "Legendary",
        }
    }
    /// 0 for common up to 4 for legendary.
    pub fn tier(self) -> usize {
        self as usize
    }
}

// ---------------------------------------------------------------------------------------------
// Items

/// An item shown in hotbars, inventories, tooltips and loot popups.
///
/// ```no_run
/// use glowkit_interactions::{GameIcon, GameItem, GameRarity};
/// let blade = GameItem::new("Dawnbreaker", GameIcon::Sword, GameRarity::Legendary)
///     .kind("Greatsword")
///     .stat("Damage", 148.0)
///     .stat("Crit chance", 12.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct GameItem {
    /// Name.
    pub name: String,
    /// Painted icon.
    pub icon: GameIcon,
    /// Rarity tier.
    pub rarity: GameRarity,
    /// Item type line, e.g. "Greatsword".
    pub kind: String,
    /// Stack count (1 hides the badge).
    pub count: u32,
    /// Named stats, compared against the equipped item in tooltips.
    pub stats: Vec<(String, f32)>,
    /// Flavor text.
    pub flavor: String,
}

impl GameItem {
    /// A new item.
    pub fn new(name: impl Into<String>, icon: GameIcon, rarity: GameRarity) -> Self {
        Self { name: name.into(), icon, rarity, kind: String::new(), count: 1, stats: Vec::new(), flavor: String::new() }
    }
    /// Item type line.
    pub fn kind(mut self, k: impl Into<String>) -> Self {
        self.kind = k.into();
        self
    }
    /// Stack count.
    pub fn count(mut self, n: u32) -> Self {
        self.count = n;
        self
    }
    /// Add a stat.
    pub fn stat(mut self, name: impl Into<String>, v: f32) -> Self {
        self.stats.push((name.into(), v));
        self
    }
    /// Flavor text.
    pub fn flavor(mut self, f: impl Into<String>) -> Self {
        self.flavor = f.into();
        self
    }
    /// The icon's natural color (blends its own tint with the rarity).
    pub fn tint(&self) -> Color32 {
        mix(self.icon.tint(), self.rarity.color(), 0.25)
    }
}

// ---------------------------------------------------------------------------------------------
// Fonts and text

/// Heavy display font (the gallery's `"display"` family when installed, else proportional).
pub(crate) fn display_font(ui: &Ui, size: f32) -> FontId {
    let fam = FontFamily::Name("display".into());
    let has = ui.ctx().fonts(|f| f.families().contains(&fam));
    FontId::new(size, if has { fam } else { FontFamily::Proportional })
}

/// Semibold font (the gallery's `"semibold"` family when installed, else proportional).
pub(crate) fn semi_font(ui: &Ui, size: f32) -> FontId {
    let fam = FontFamily::Name("semibold".into());
    let has = ui.ctx().fonts(|f| f.families().contains(&fam));
    FontId::new(size, if has { fam } else { FontFamily::Proportional })
}

/// Text with a dark outline (the classic readable-on-anything game look).
pub(crate) fn outlined(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    outline: Color32,
) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), font, color);
    let r = align.anchor_size(pos, g.size());
    let w = (g.size().y * 0.07).clamp(1.0, 3.0);
    if outline.a() > 0 {
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU;
            painter.galley_with_override_text_color(r.min + vec2(a.cos(), a.sin()) * w, g.clone(), outline);
        }
    }
    painter.galley(r.min, g, color);
    r
}

/// Text with a soft colored glow behind it.
pub(crate) fn glow_text(painter: &Painter, pos: Pos2, align: Align2, text: &str, font: FontId, color: Color32, glow: Color32) -> Rect {
    let g = painter.layout_no_wrap(text.to_owned(), font, color);
    let r = align.anchor_size(pos, g.size());
    if glow.a() > 0 {
        paint::blurred_galley(painter, r.min, &g, glow, (g.size().y * 0.16).max(2.0));
    }
    painter.galley(r.min, g, color);
    r
}

pub(crate) fn text_w(painter: &Painter, s: &str, font: &FontId) -> f32 {
    painter.layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE).size().x
}

// ---------------------------------------------------------------------------------------------
// Panels

/// Chamfered rectangle outline (top-left and bottom-right corners cut).
pub(crate) fn chamfer_points(rect: Rect, c: f32) -> Vec<Pos2> {
    let c = c.min(rect.width() / 3.0).min(rect.height() / 3.0);
    vec![
        pos2(rect.min.x + c, rect.min.y),
        pos2(rect.max.x, rect.min.y),
        pos2(rect.max.x, rect.max.y - c),
        pos2(rect.max.x - c, rect.max.y),
        pos2(rect.min.x, rect.max.y),
        pos2(rect.min.x, rect.min.y + c),
    ]
}

fn closed(painter: &Painter, pts: &[Pos2], stroke: Stroke) {
    let mut v = pts.to_vec();
    v.push(pts[0]);
    painter.add(Shape::line(v, stroke));
}

/// Paint a styled panel frame. `glow` (0..1+) lights the frame (selection, focus).
pub(crate) fn frame(painter: &Painter, rect: Rect, style: GameStyle, pal: &GamePalette, glow: f32) {
    match style {
        GameStyle::SciFi => {
            let c = (rect.height() * 0.22).clamp(4.0, 16.0);
            let pts = chamfer_points(rect, c);
            if glow > 0.01 {
                paint::soft_shadow(painter, rect, 4.0, 18.0, Vec2::ZERO, with_alpha(pal.accent, 0.35 * glow.min(1.5)));
            }
            paint::fill_convex_fn(painter, &pts, 2, paint::linear_fn(rect, 90.0, &[pal.panel, pal.panel2]));
            // scanlines
            let step = 4.0;
            let mut y = rect.min.y + 2.0;
            let sc = with_alpha(pal.accent, 0.035);
            while y < rect.max.y - 1.0 {
                painter.line_segment([pos2(rect.min.x + 2.0, y), pos2(rect.max.x - 2.0, y)], Stroke::new(1.0, sc));
                y += step;
            }
            closed(painter, &pts, Stroke::new(1.0, with_alpha(pal.edge, 0.35 + 0.4 * glow.min(1.0))));
            // bright corner brackets on the uncut corners
            let b = (rect.width().min(rect.height()) * 0.25).clamp(6.0, 18.0);
            let st = Stroke::new(2.0, with_alpha(pal.edge, 0.85));
            let tr = pos2(rect.max.x, rect.min.y);
            painter.line_segment([tr - vec2(b, 0.0), tr], st);
            painter.line_segment([tr, tr + vec2(0.0, b)], st);
            let bl = pos2(rect.min.x, rect.max.y);
            painter.line_segment([bl + vec2(b, 0.0), bl], st);
            painter.line_segment([bl, bl - vec2(0.0, b)], st);
            // chamfer ticks
            painter.line_segment(
                [pos2(rect.min.x + 3.0, rect.min.y + c + 3.0), pos2(rect.min.x + c + 3.0, rect.min.y + 3.0)],
                Stroke::new(1.0, with_alpha(pal.edge, 0.5)),
            );
        }
        GameStyle::Fantasy => {
            let r = 6.0;
            paint::soft_shadow(painter, rect, r, 16.0, vec2(0.0, 5.0), Color32::from_black_alpha(140));
            if glow > 0.01 {
                paint::soft_shadow(painter, rect.expand(2.0), r, 20.0, Vec2::ZERO, with_alpha(pal.accent, 0.45 * glow.min(1.5)));
            }
            let pts = paint::rounded_rect_points(rect, r, 4);
            paint::fill_convex_fn(painter, &pts, 4, |p| {
                let u = (p.y - rect.min.y) / rect.height().max(1.0);
                let n = glowkit_core::noise::noise01(p.x * 0.03, p.y * 0.03, 1.7);
                let base = mix(pal.panel, pal.panel2, u);
                let edge_d = ((p.x - rect.min.x).min(rect.max.x - p.x).min(p.y - rect.min.y).min(rect.max.y - p.y) / 18.0).clamp(0.0, 1.0);
                mix(mix(base, pal.panel2, n * 0.5), glowkit_core::color::shade(pal.panel2, -0.25), (1.0 - edge_d) * 0.55)
            });
            let gold = pal.edge;
            painter.rect_stroke(rect, 6.0, Stroke::new(2.0, gold), StrokeKind::Inside);
            painter.rect_stroke(rect.shrink(5.0), 3.0, Stroke::new(1.0, with_alpha(gold, 0.8)), StrokeKind::Inside);
            let hi = mix(gold, Color32::WHITE, 0.45);
            for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                let p = pos2(rect.min.x + 5.0 + (rect.width() - 10.0) * cx, rect.min.y + 5.0 + (rect.height() - 10.0) * cy);
                let d = 5.0;
                painter.add(Shape::convex_polygon(
                    vec![p + vec2(0.0, -d), p + vec2(d, 0.0), p + vec2(0.0, d), p + vec2(-d, 0.0)],
                    gold,
                    Stroke::new(1.0, glowkit_core::color::shade(gold, -0.3)),
                ));
                painter.circle_filled(p + vec2(-1.0, -1.0), 1.4, hi);
            }
        }
        GameStyle::Minimal => {
            let r = (rect.height() * 0.2).clamp(6.0, 14.0);
            paint::soft_shadow(painter, rect, r, 22.0, vec2(0.0, 8.0), Color32::from_black_alpha(110));
            if glow > 0.01 {
                paint::soft_shadow(painter, rect, r, 16.0, Vec2::ZERO, with_alpha(pal.accent, 0.3 * glow.min(1.5)));
            }
            let pts = paint::rounded_rect_points(rect, r, 6);
            paint::fill_convex_fn(painter, &pts, 2, paint::linear_fn(rect, 90.0, &[pal.panel, pal.panel2]));
            painter.rect_stroke(rect, r, Stroke::new(1.0, mix(pal.edge, pal.accent, glow.clamp(0.0, 1.0) * 0.6)), StrokeKind::Inside);
            painter.line_segment(
                [pos2(rect.min.x + r, rect.min.y + 1.0), pos2(rect.max.x - r, rect.min.y + 1.0)],
                Stroke::new(1.0, with_alpha(Color32::WHITE, 0.08)),
            );
        }
    }
}

/// A slot well (inventory cell, hotbar slot, icon frame).
pub(crate) fn slot(painter: &Painter, rect: Rect, style: GameStyle, pal: &GamePalette, rarity: Option<GameRarity>, lit: f32) {
    let rc = rarity.map(|r| r.color());
    match style {
        GameStyle::SciFi => {
            let pts = chamfer_points(rect, rect.width() * 0.16);
            let base = hex("#081626E8");
            let top = rc.map(|c| mix(base, c, 0.22)).unwrap_or(hex("#0F2338E8"));
            paint::fill_convex_fn(painter, &pts, 2, paint::linear_fn(rect, 90.0, &[base, top]));
            let ec = rc.unwrap_or(pal.edge);
            closed(painter, &pts, Stroke::new(1.0 + lit, with_alpha(ec, 0.45 + 0.5 * lit.min(1.0))));
        }
        GameStyle::Fantasy => {
            let r = 4.0;
            let base = hex("#2B1D12");
            let top = rc.map(|c| mix(base, c, 0.28)).unwrap_or(hex("#3A2918"));
            let pts = paint::rounded_rect_points(rect, r, 3);
            paint::fill_convex_fn(painter, &pts, 2, paint::radial_fn(rect.center(), rect.width() * 0.75, &[top, base]));
            painter.rect_stroke(rect, r, Stroke::new(2.0, mix(pal.edge, Color32::WHITE, lit.min(1.0) * 0.4)), StrokeKind::Inside);
            if let Some(c) = rc {
                painter.rect_stroke(rect.shrink(3.0), 2.0, Stroke::new(1.0, with_alpha(c, 0.8)), StrokeKind::Inside);
            }
        }
        GameStyle::Minimal => {
            let r = rect.width() * 0.18;
            let base = hex("#1C1E25E8");
            painter.rect_filled(rect, r, rc.map(|c| mix(base, c, 0.14)).unwrap_or(base));
            let ec = rc.map(|c| with_alpha(c, 0.7)).unwrap_or(hex("#FFFFFF1E"));
            painter.rect_stroke(rect, r, Stroke::new(1.0 + lit, mix(ec, pal.accent, lit.min(1.0) * 0.5)), StrokeKind::Inside);
        }
    }
    if let Some(c) = rc {
        if rarity.unwrap_or_default() >= GameRarity::Rare {
            paint::glow_ellipse(painter, rect.center() + vec2(0.0, rect.height() * 0.3), vec2(rect.width() * 0.45, rect.height() * 0.25), with_alpha(c, 0.35));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bars

/// Paint a horizontal fill bar with the given fraction, gradient colors and a top gloss.
pub(crate) fn bar_fill(painter: &Painter, track: Rect, from: f32, to: f32, colors: [Color32; 2], style: GameStyle) {
    let a = from.clamp(0.0, 1.0);
    let b = to.clamp(0.0, 1.0);
    if b - a <= 0.0005 {
        return;
    }
    let r = Rect::from_min_max(pos2(track.min.x + track.width() * a, track.min.y), pos2(track.min.x + track.width() * b, track.max.y));
    let rad = match style {
        GameStyle::Minimal => (track.height() / 2.0).min(r.width() / 2.0),
        GameStyle::Fantasy => 2.0f32.min(r.width() / 2.0),
        GameStyle::SciFi => 0.0,
    };
    let pts = paint::rounded_rect_points(r, rad, 3);
    paint::fill_convex_fn(painter, &pts, 1, paint::linear_fn(r, 90.0, &[colors[1], colors[0], glowkit_core::color::shade(colors[0], -0.25)]));
    let gloss = Rect::from_min_max(r.min + vec2(rad * 0.5, 1.0), pos2(r.max.x - rad * 0.5, r.min.y + r.height() * 0.38));
    if gloss.width() > 1.0 {
        painter.rect_filled(gloss, rad.min(gloss.height() / 2.0), with_alpha(Color32::WHITE, 0.16));
    }
}

/// Paint a bar track for `style`.
pub(crate) fn bar_track(painter: &Painter, track: Rect, style: GameStyle, pal: &GamePalette) {
    match style {
        GameStyle::SciFi => {
            let pts = chamfer_points(track.expand(3.0), 6.0);
            painter.add(Shape::convex_polygon(pts.clone(), hex("#020812D0"), Stroke::NONE));
            closed(painter, &pts, Stroke::new(1.0, with_alpha(pal.edge, 0.55)));
        }
        GameStyle::Fantasy => {
            let o = track.expand(4.0);
            paint::soft_shadow(painter, o, 4.0, 8.0, vec2(0.0, 2.0), Color32::from_black_alpha(140));
            paint::gradient_rect(painter, o, 4.0, 90.0, &[hex("#F3D58A"), hex("#9C6F25")]);
            painter.rect_filled(track.expand(1.0), 2.0, hex("#1A0F08"));
        }
        GameStyle::Minimal => {
            let r = track.height() / 2.0 + 2.0;
            painter.rect_filled(track.expand(2.0), r, hex("#0B0C10D8"));
            painter.rect_stroke(track.expand(2.0), r, Stroke::new(1.0, hex("#FFFFFF1C")), StrokeKind::Inside);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The game viewport backdrop

/// A painted game viewport: a dark rounded scene with a sky gradient, stars, a moon or planet and
/// two or three parallax layers (a neon skyline with a perspective grid for sci-fi, mountains and
/// a pine forest at dusk for fantasy, soft low hills for minimal). Use it behind menus and HUD
/// demos, or as a placeholder before your real game renders.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let rect = ui.available_rect_before_wrap();
/// glowkit_interactions::GameBackdrop::new(glowkit_interactions::GameStyle::SciFi).paint(ui, rect);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct GameBackdrop {
    style: GameStyle,
    palette: Option<GamePalette>,
    parallax: Vec2,
    speed: f32,
    radius: f32,
    dim: f32,
}

impl GameBackdrop {
    /// A backdrop in `style`.
    pub fn new(style: GameStyle) -> Self {
        Self { style, palette: None, parallax: Vec2::ZERO, speed: 1.0, radius: 18.0, dim: 0.0 }
    }
    /// Override the palette (its `sky` colors are used).
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Extra parallax offset (e.g. pointer position, -1..1 per axis).
    pub fn parallax(mut self, p: Vec2) -> Self {
        self.parallax = p;
        self
    }
    /// Scroll speed multiplier (0 = still).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r.max(0.0);
        self
    }
    /// Darken the whole scene (0..1), e.g. under a pause overlay.
    pub fn dim(mut self, d: f32) -> Self {
        self.dim = d.clamp(0.0, 1.0);
        self
    }

    /// Paint into `rect`. Requests repaints while it scrolls.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = glowkit_core::time(ui) * self.speed;
        if self.speed != 0.0 {
            glowkit_core::animate(ui);
        }
        self.paint_at(ui.painter(), rect, t);
    }

    /// Paint with an explicit scene time.
    pub(crate) fn paint_at(&self, painter: &Painter, rect: Rect, t: f32) {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let rad = self.radius;
        let pts = paint::rounded_rect_points(rect, rad, 6);
        paint::fill_convex_fn(painter, &pts, 3, paint::linear_fn(rect, 90.0, &pal.sky));
        let inner = rect.shrink(rad * 0.4);
        let px = self.parallax;
        let w = rect.width();
        let h = rect.height();
        match self.style {
            GameStyle::SciFi => {
                stars(painter, inner, t, 90, 0.75);
                let planet = pos2(rect.min.x + w * 0.74 - px.x * 6.0, rect.min.y + h * 0.26 - px.y * 4.0);
                let pr = h * 0.16;
                paint::glow(painter, planet, pr * 2.6, with_alpha(pal.accent2, 0.22));
                paint::gradient_circle(painter, planet, pr, &[hex("#FF7AC6"), hex("#7A2C8F"), hex("#2A0F3D")]);
                paint::arc(painter, planet, pr * 1.55, PI * 0.95, PI * 2.05, 2.0, with_alpha(hex("#FFC2EA"), 0.5));
                let horizon = rect.min.y + h * 0.66;
                paint::glow_ellipse(painter, pos2(rect.center().x, horizon), vec2(w * 0.55, h * 0.1), with_alpha(pal.accent2, 0.35));
                // far skyline
                let o1 = t * 8.0 + px.x * 10.0;
                silhouette(painter, rect, rad, hex("#1A1440"), hex("#120E30"), |x| {
                    let k = ((x + o1) / 26.0).floor();
                    horizon - 18.0 - hash01(k as i64 as u64 * 7 + 3) * h * 0.22
                });
                let o2 = t * 22.0 + px.x * 22.0;
                silhouette(painter, rect, rad, hex("#0B0A22"), hex("#07061A"), |x| {
                    let k = ((x + o2) / 38.0).floor();
                    horizon + 2.0 - hash01(k as i64 as u64 * 13 + 1) * h * 0.16
                });
                // lit windows on the near layer
                let mut bx = ((rect.min.x + o2) / 38.0).floor() * 38.0 - o2;
                while bx < rect.max.x {
                    let k = ((bx + o2) / 38.0).round();
                    let top = horizon + 2.0 - hash01(k as i64 as u64 * 13 + 1) * h * 0.16;
                    let mut wy = top + 6.0;
                    let mut j = 0u64;
                    while wy < horizon - 4.0 {
                        for c in 0..3u64 {
                            let on = hash01(k as i64 as u64 * 31 + j * 7 + c) > 0.55;
                            let wx = bx + 8.0 + c as f32 * 9.0;
                            if on && wx > inner.min.x && wx < inner.max.x - 4.0 {
                                let col = if hash01(k as u64 + j + c * 3) > 0.7 { pal.accent2 } else { pal.accent };
                                painter.rect_filled(Rect::from_min_size(pos2(wx, wy), vec2(4.0, 3.0)), 0.0, with_alpha(col, 0.55));
                            }
                        }
                        wy += 9.0;
                        j += 1;
                    }
                    bx += 38.0;
                }
                // perspective grid floor
                let floor_c = hex("#06051A");
                silhouette(painter, rect, rad, floor_c, hex("#0A0626"), |_| horizon);
                let gc = pal.accent2;
                let scroll = (t * 0.6).fract();
                for i in 0..10 {
                    let z = (i as f32 + 1.0 - scroll) / 10.0;
                    let y = horizon + (rect.max.y - horizon) * z * z;
                    if y < rect.max.y - rad * 0.3 {
                        let (x0, x1) = row_bounds(rect, rad, y);
                        painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0, with_alpha(gc, 0.15 + 0.5 * z)));
                    }
                }
                let vp = pos2(rect.center().x - px.x * 30.0, horizon);
                for i in -12..=12 {
                    let bx = rect.center().x + i as f32 * w * 0.12;
                    let end = pos2(bx, rect.max.y);
                    let y_end = rect.max.y - rad;
                    let k = (y_end - vp.y) / (end.y - vp.y);
                    let e = vp + (end - vp) * k;
                    if e.x > rect.min.x + 2.0 && e.x < rect.max.x - 2.0 {
                        painter.line_segment([vp + (end - vp) * 0.02, e], Stroke::new(1.0, with_alpha(gc, 0.28)));
                    }
                }
                painter.line_segment([pos2(rect.min.x + 1.0, horizon), pos2(rect.max.x - 1.0, horizon)], Stroke::new(1.5, with_alpha(hex("#FF9AD8"), 0.7)));
            }
            GameStyle::Fantasy => {
                stars(painter, Rect::from_min_max(inner.min, pos2(inner.max.x, rect.min.y + h * 0.5)), t, 50, 0.6);
                let moon = pos2(rect.min.x + w * 0.24 - px.x * 5.0, rect.min.y + h * 0.27 - px.y * 3.0);
                let mr = h * 0.1;
                paint::glow(painter, moon, mr * 4.0, with_alpha(hex("#FFE2B0"), 0.3));
                painter.circle_filled(moon, mr, hex("#FFF1D2"));
                painter.circle_filled(moon + vec2(mr * 0.3, -mr * 0.2), mr * 0.22, hex("#EBD9B4"));
                painter.circle_filled(moon + vec2(-mr * 0.35, mr * 0.3), mr * 0.14, hex("#EBD9B4"));
                let o0 = t * 3.0 + px.x * 6.0;
                silhouette(painter, rect, rad, hex("#5A3352"), hex("#6E3B4E"), |x| {
                    let u = (x + o0) / w;
                    rect.min.y + h * 0.5 - (glowkit_core::noise::fbm3(u * 3.0, 0.3, 0.0, 4) * 0.5 + 0.5) * h * 0.26
                });
                let o1 = t * 9.0 + px.x * 14.0;
                silhouette(painter, rect, rad, hex("#2E1B34"), hex("#3A2238"), |x| {
                    let u = (x + o1) / w;
                    rect.min.y + h * 0.66 - (glowkit_core::noise::fbm3(u * 5.0, 2.3, 0.0, 3) * 0.5 + 0.5) * h * 0.2
                });
                // castle on the mid ridge
                let cx = rect.min.x + w * 0.68 - px.x * 12.0;
                let base_y = rect.min.y + h * 0.55;
                let castle = hex("#2E1B34");
                for (dx, tw, th) in [(-26.0, 12.0, 46.0), (-10.0, 22.0, 30.0), (12.0, 10.0, 58.0), (26.0, 16.0, 36.0)] {
                    let r = Rect::from_min_max(pos2(cx + dx, base_y - th), pos2(cx + dx + tw, base_y + 10.0));
                    painter.rect_filled(r, 0.0, castle);
                    painter.add(Shape::convex_polygon(
                        vec![pos2(r.min.x - 2.0, r.min.y), pos2(r.center().x, r.min.y - tw * 1.1), pos2(r.max.x + 2.0, r.min.y)],
                        castle,
                        Stroke::NONE,
                    ));
                    painter.rect_filled(Rect::from_center_size(r.center() - vec2(0.0, th * 0.2), vec2(2.5, 4.0)), 0.0, hex("#FFC25A"));
                }
                let o2 = t * 26.0 + px.x * 30.0;
                silhouette(painter, rect, rad, hex("#120A16"), hex("#0B060E"), |x| {
                    let xx = x + o2;
                    let k = (xx / 22.0).floor();
                    let f = (xx / 22.0).fract();
                    let th = 26.0 + hash01(k as i64 as u64 * 17) * 40.0;
                    let tri = 1.0 - (f - 0.5).abs() * 2.0;
                    let jag = if ((f * 6.0).fract()) < 0.5 { 0.0 } else { 4.0 };
                    rect.min.y + h * 0.86 - tri * th + jag * tri
                });
                // fireflies
                for i in 0..14u64 {
                    let fx = rect.min.x + w * hash01(i * 3 + 1) + (t * 0.7 + i as f32).sin() * 10.0;
                    let fy = rect.min.y + h * (0.62 + 0.3 * hash01(i * 5 + 2)) + (t * 1.1 + i as f32 * 2.0).cos() * 6.0;
                    let a = 0.5 + 0.5 * (t * 2.0 + i as f32).sin();
                    if inner.contains(pos2(fx, fy)) {
                        paint::glow(painter, pos2(fx, fy), 6.0, with_alpha(hex("#FFD86A"), 0.5 * a));
                        painter.circle_filled(pos2(fx, fy), 1.2, with_alpha(hex("#FFF4C2"), a));
                    }
                }
            }
            GameStyle::Minimal => {
                stars(painter, inner, t, 40, 0.35);
                paint::glow(painter, pos2(rect.min.x + w * 0.7, rect.min.y + h * 0.3), h * 0.6, with_alpha(pal.accent, 0.12));
                paint::glow(painter, pos2(rect.min.x + w * 0.2, rect.min.y + h * 0.7), h * 0.5, with_alpha(pal.accent2, 0.08));
                for (k, (c, amp, base, sp)) in [
                    (hex("#20263A"), 0.12, 0.62, 5.0),
                    (hex("#1A1F2E"), 0.1, 0.74, 12.0),
                    (hex("#131620"), 0.08, 0.86, 24.0),
                ]
                .into_iter()
                .enumerate()
                {
                    let o = t * sp + px.x * (8.0 + 10.0 * k as f32);
                    silhouette(painter, rect, rad, c, c, |x| {
                        let u = (x + o) / w;
                        rect.min.y + h * base - ((u * 4.0 + k as f32).sin() * 0.5 + (u * 9.3 + k as f32 * 2.0).sin() * 0.25 + 0.75) * h * amp
                    });
                }
            }
        }
        if self.dim > 0.0 {
            painter.rect_filled(rect, rad, Color32::from_black_alpha((self.dim * 190.0) as u8));
        }
    }
}

fn stars(painter: &Painter, rect: Rect, t: f32, n: u64, bright: f32) {
    for i in 0..n {
        let x = rect.min.x + rect.width() * hash01(i * 2 + 11);
        let y = rect.min.y + rect.height() * hash01(i * 2 + 12).powf(1.4);
        let tw = 0.5 + 0.5 * (t * (1.0 + hash01(i + 99) * 2.0) + i as f32).sin();
        let s = 0.5 + hash01(i + 7) * 1.1;
        painter.circle_filled(pos2(x, y), s, with_alpha(Color32::WHITE, bright * (0.3 + 0.7 * tw)));
    }
}

/// Horizontal extent of a rounded rect at height `y`.
pub(crate) fn row_bounds(rect: Rect, r: f32, y: f32) -> (f32, f32) {
    let r = r.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let dy = if y < rect.min.y + r {
        rect.min.y + r - y
    } else if y > rect.max.y - r {
        y - (rect.max.y - r)
    } else {
        0.0
    };
    let dx = r - (r * r - dy.min(r) * dy.min(r)).max(0.0).sqrt();
    (rect.min.x + dx, rect.max.x - dx)
}

/// Vertical extent of a rounded rect at `x`.
pub(crate) fn col_bounds(rect: Rect, r: f32, x: f32) -> (f32, f32) {
    let r = r.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let dx = if x < rect.min.x + r {
        rect.min.x + r - x
    } else if x > rect.max.x - r {
        x - (rect.max.x - r)
    } else {
        0.0
    };
    let dy = r - (r * r - dx.min(r) * dx.min(r)).max(0.0).sqrt();
    (rect.min.y + dy, rect.max.y - dy)
}

/// Fill from the curve `top(x)` down to the bottom of the rounded `rect` (terrain layers).
pub(crate) fn silhouette(painter: &Painter, rect: Rect, r: f32, c_top: Color32, c_bot: Color32, top: impl Fn(f32) -> f32) {
    let mut m = Mesh::default();
    let step = 3.0;
    let n = (rect.width() / step).ceil() as usize;
    for i in 0..=n {
        let x = (rect.min.x + i as f32 * step).min(rect.max.x);
        let (y0, y1) = col_bounds(rect, r, x);
        let y = top(x).clamp(y0, y1);
        m.colored_vertex(pos2(x, y), c_top);
        m.colored_vertex(pos2(x, y1), c_bot);
    }
    for i in 0..n as u32 {
        let k = i * 2;
        m.add_triangle(k, k + 1, k + 2);
        m.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(Shape::mesh(m));
}

// ---------------------------------------------------------------------------------------------
// Particles and shake

/// A stateless particle burst `t` seconds after it started: `n` sparks fly out from `center`
/// with gravity, shrink and fade. Returns true while still visible.
#[allow(clippy::too_many_arguments)]
pub(crate) fn burst(painter: &Painter, center: Pos2, t: f32, n: usize, seed: u64, speed: f32, gravity: f32, color: Color32, size: f32) -> bool {
    let life = 1.1;
    if !(0.0..life).contains(&t) {
        return false;
    }
    for i in 0..n as u64 {
        let a = hash01(seed.wrapping_mul(31).wrapping_add(i * 3)) * TAU;
        let sp = speed * (0.35 + 0.65 * hash01(seed.wrapping_add(i * 5 + 1)));
        let l = life * (0.55 + 0.45 * hash01(seed.wrapping_add(i * 7 + 2)));
        if t > l {
            continue;
        }
        let k = t / l;
        let drag = 1.0 - (-t * 3.0).exp();
        let p = center + vec2(a.cos(), a.sin()) * sp * drag / 3.0 + vec2(0.0, gravity * t * t);
        let s = size * (1.0 - k) * (0.6 + 0.6 * hash01(seed + i * 11));
        let c = with_alpha(mix(Color32::WHITE, color, (k * 2.0).min(1.0)), 1.0 - k * k);
        if i % 3 == 0 {
            paint::sparkle(painter, p, s * 1.8, t * 4.0 + i as f32, c);
        } else {
            painter.circle_filled(p, s, c);
        }
    }
    true
}

/// An expanding shock ring.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ring(painter: &Painter, center: Pos2, t: f32, dur: f32, r0: f32, r1: f32, color: Color32, width: f32) {
    if !(0.0..dur).contains(&t) {
        return;
    }
    let k = t / dur;
    let e = 1.0 - (1.0 - k).powi(3);
    painter.circle_stroke(center, r0 + (r1 - r0) * e, Stroke::new(width * (1.0 - k) + 0.5, with_alpha(color, (1.0 - k) * color.a() as f32 / 255.0)));
}

/// Rotating light rays (loot reveals, level ups).
pub(crate) fn rays(painter: &Painter, center: Pos2, r: f32, n: usize, rot: f32, color: Color32) {
    for i in 0..n {
        let a = rot + i as f32 / n as f32 * TAU;
        let half = PI / n as f32 * 0.45;
        let mut m = Mesh::default();
        m.colored_vertex(center, color);
        m.colored_vertex(center + vec2((a - half).cos(), (a - half).sin()) * r, Color32::TRANSPARENT);
        m.colored_vertex(center + vec2((a + half).cos(), (a + half).sin()) * r, Color32::TRANSPARENT);
        m.add_triangle(0, 1, 2);
        painter.add(Shape::mesh(m));
    }
}

/// Screen-shake offset `t` seconds after an impact of `amp` points (decays fast).
pub(crate) fn shake(t: f32, amp: f32, seed: f32) -> Vec2 {
    if !(0.0..0.6).contains(&t) || amp <= 0.0 {
        return Vec2::ZERO;
    }
    let d = (-t * 9.0).exp() * amp;
    vec2(glowkit_core::noise::noise2(t * 40.0, seed), glowkit_core::noise::noise2(seed + 17.0, t * 40.0)) * d * 1.6
}

/// Overshooting pop curve 0..1 (scale in with a bounce).
pub(crate) fn pop(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    glowkit_core::ease(glowkit_core::Easing::OutBack, t)
}

/// Seconds since `active` was last true for widget `id` (starts large so autoplay begins at once).
pub(crate) fn idle_for(ui: &Ui, id: egui::Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("game_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

/// Time since the value under `id` last changed (tracks e.g. HP to time hit effects).
/// Returns (seconds since change, previous value before the change).
pub(crate) fn changed(ui: &Ui, id: egui::Id, v: f32) -> (f32, f32, f32) {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("game_changed"),
        || (v, v, now - 100.0),
        |s| {
            if (s.0 - v).abs() > 1e-4 {
                s.1 = s.0;
                s.0 = v;
                s.2 = now;
            }
            ((now - s.2) as f32, s.1, s.0)
        },
    )
}

/// A small ghost cursor for autoplay.
pub(crate) fn ghost_cursor(painter: &Painter, tip: Pos2, alpha: f32, pressed: bool) {
    if alpha <= 0.01 {
        return;
    }
    let s = if pressed { 0.88 } else { 1.0 };
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y) * s)
        .collect();
    let c = tip + vec2(3.5, 9.0) * s;
    for i in 0..pts.len() {
        painter.add(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % pts.len()]], with_alpha(Color32::WHITE, alpha), Stroke::NONE));
    }
    closed(painter, &pts, Stroke::new(1.2, with_alpha(Color32::BLACK, alpha)));
}

// ---------------------------------------------------------------------------------------------
// Icons

/// Painted game icons (no icon font or textures needed): weapons, gear, consumables and
/// ability glyphs.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::GameIcon;
/// GameIcon::Potion.paint(ui.painter(), egui::pos2(40.0, 40.0), 32.0, GameIcon::Potion.tint());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameIcon {
    /// Straight sword.
    Sword,
    /// Battle axe.
    Axe,
    /// Bow.
    Bow,
    /// Magic staff.
    Staff,
    /// Kite shield.
    Shield,
    /// Helmet.
    Helmet,
    /// Chest armor.
    Armor,
    /// Boots.
    Boots,
    /// Ring.
    Ring,
    /// Health potion.
    Potion,
    /// Mana potion.
    ManaPotion,
    /// Cut gem.
    Gem,
    /// Scroll.
    Scroll,
    /// Key.
    Key,
    /// Gold coin.
    Coin,
    /// Bomb / grenade.
    Bomb,
    /// Fireball.
    Fire,
    /// Frost shard.
    Frost,
    /// Lightning bolt.
    Bolt,
    /// Heal cross.
    Heal,
    /// Dash chevrons.
    Dash,
    /// Skull.
    Skull,
    /// Star.
    Star,
    /// Heart.
    Heart,
    /// Rifle.
    Rifle,
    /// Pistol.
    Pistol,
    /// Crosshair target.
    Target,
    /// Poison drop / leaf.
    Poison,
    /// Trophy cup.
    Trophy,
    /// Treasure map.
    Map,
}

impl GameIcon {
    /// The icon's natural color.
    pub fn tint(self) -> Color32 {
        use GameIcon::*;
        hex(match self {
            Sword | Axe | Rifle | Pistol => "#D8DEE9",
            Bow | Staff => "#C9965A",
            Shield | Helmet | Armor => "#AFC3D6",
            Boots => "#B68B5E",
            Ring | Coin | Trophy | Star => "#FFC94A",
            Potion | Heart => "#FF4D6A",
            ManaPotion | Frost => "#5CB8FF",
            Gem => "#B86BFF",
            Scroll | Map => "#E9D3A2",
            Key => "#FFD36A",
            Bomb => "#5D6675",
            Fire => "#FF7A2E",
            Bolt => "#FFE14D",
            Heal | Poison => "#5BE584",
            Dash => "#7DF0FF",
            Skull => "#ECE6D8",
            Target => "#FF5A5A",
        })
    }

    /// Paint centered at `c`, `size` points across.
    pub fn paint(self, painter: &Painter, c: Pos2, size: f32, color: Color32) {
        use GameIcon::*;
        let s = size / 2.0;
        let p = |x: f32, y: f32| c + vec2(x, y) * s;
        let dark = glowkit_core::color::shade(color, -0.45);
        let lite = mix(color, Color32::WHITE, 0.55);
        let st = |w: f32, col: Color32| Stroke::new((w * s).max(1.0), col);
        let poly = |pts: Vec<Pos2>, col: Color32| {
            painter.add(Shape::convex_polygon(pts, col, Stroke::NONE));
        };
        match self {
            Sword => {
                poly(vec![p(-0.62, 0.62), p(0.55, -0.55), p(0.72, -0.72), p(0.62, -0.47), p(-0.5, 0.68)], color);
                painter.line_segment([p(-0.52, 0.52), p(0.6, -0.6)], st(0.06, lite));
                painter.line_segment([p(-0.62, 0.22), p(-0.22, 0.62)], st(0.16, hex("#C99A3A")));
                painter.line_segment([p(-0.42, 0.42), p(-0.78, 0.78)], st(0.13, hex("#6B4423")));
                painter.circle_filled(p(-0.82, 0.82), 0.1 * s, hex("#C99A3A"));
            }
            Axe => {
                painter.line_segment([p(-0.6, 0.8), p(0.35, -0.7)], st(0.14, hex("#7A4E2A")));
                poly(vec![p(0.05, -0.55), p(0.6, -0.85), p(0.85, -0.3), p(0.6, 0.15), p(0.2, -0.1)], color);
                painter.line_segment([p(0.6, -0.85), p(0.85, -0.3)], st(0.08, lite));
                painter.line_segment([p(0.85, -0.3), p(0.6, 0.15)], st(0.08, lite));
            }
            Bow => {
                let pts: Vec<Pos2> = (0..=20).map(|i| {
                    let a = -1.15 + 2.3 * i as f32 / 20.0;
                    p(-0.55 + a.cos() * 0.95 - 0.35, a.sin() * 0.85)
                }).collect();
                painter.add(Shape::line(pts, st(0.14, color)));
                painter.line_segment([p(0.05, -0.77), p(0.05, 0.77)], st(0.04, lite));
                painter.line_segment([p(-0.6, 0.0), p(0.85, 0.0)], st(0.06, hex("#D8DEE9")));
                poly(vec![p(0.85, 0.0), p(0.62, -0.12), p(0.62, 0.12)], hex("#D8DEE9"));
            }
            Staff => {
                painter.line_segment([p(-0.55, 0.85), p(0.25, -0.35)], st(0.13, color));
                paint::glow(painter, p(0.38, -0.55), 0.6 * s, with_alpha(hex("#7DE3FF"), 0.7));
                painter.circle_filled(p(0.38, -0.55), 0.24 * s, hex("#BFF3FF"));
                painter.circle_stroke(p(0.38, -0.55), 0.32 * s, st(0.07, dark));
            }
            Shield => {
                let pts = vec![p(-0.7, -0.72), p(0.7, -0.72), p(0.66, 0.05), p(0.0, 0.85), p(-0.66, 0.05)];
                poly(pts.clone(), color);
                poly(vec![p(-0.7, -0.72), p(0.0, -0.72), p(0.0, 0.85), p(-0.66, 0.05)], dark);
                closed(painter, &pts, st(0.08, hex("#C99A3A")));
                paint::star(painter, p(0.0, -0.1), 0.3 * s, 0.13 * s, 4, 0.0, hex("#FFD36A"));
            }
            Helmet => {
                let dome: Vec<Pos2> = (0..=16).map(|i| {
                    let a = PI + PI * i as f32 / 16.0;
                    p(a.cos() * 0.7, 0.1 + a.sin() * 0.75)
                }).collect();
                poly(dome, color);
                poly(vec![p(-0.7, 0.1), p(0.7, 0.1), p(0.6, 0.7), p(0.15, 0.7), p(-0.15, 0.7), p(-0.6, 0.7)], color);
                painter.rect_filled(Rect::from_min_max(p(-0.5, 0.05), p(0.5, 0.22)), 1.0, Color32::from_black_alpha(220));
                painter.line_segment([p(0.0, -0.6), p(0.0, 0.7)], st(0.08, lite));
            }
            Armor => {
                poly(vec![p(-0.75, -0.5), p(-0.3, -0.75), p(0.3, -0.75), p(0.75, -0.5), p(0.55, 0.0), p(-0.55, 0.0)], color);
                poly(vec![p(-0.5, -0.1), p(0.5, -0.1), p(0.45, 0.8), p(-0.45, 0.8)], color);
                poly(vec![p(-0.25, -0.75), p(0.25, -0.75), p(0.0, -0.45)], dark);
                painter.line_segment([p(0.0, -0.4), p(0.0, 0.75)], st(0.06, dark));
                painter.line_segment([p(-0.45, 0.3), p(0.45, 0.3)], st(0.06, hex("#C99A3A")));
            }
            Boots => {
                poly(vec![p(-0.45, -0.75), p(0.1, -0.75), p(0.1, 0.4), p(-0.45, 0.4)], color);
                poly(vec![p(-0.45, 0.2), p(0.3, 0.2), p(0.75, 0.45), p(0.75, 0.75), p(-0.45, 0.75)], color);
                painter.rect_filled(Rect::from_min_max(p(-0.5, 0.65), p(0.8, 0.82)), 1.0, dark);
                painter.line_segment([p(-0.45, -0.5), p(0.1, -0.5)], st(0.08, lite));
            }
            Ring => {
                painter.circle_stroke(p(0.0, 0.2), 0.52 * s, st(0.16, color));
                poly(vec![p(0.0, -0.85), p(0.3, -0.55), p(0.0, -0.3), p(-0.3, -0.55)], hex("#FF5A8A"));
                poly(vec![p(0.0, -0.85), p(0.3, -0.55), p(0.0, -0.55)], hex("#FFB3C9"));
            }
            Potion | ManaPotion => {
                painter.circle_filled(p(0.0, 0.25), 0.6 * s, with_alpha(Color32::WHITE, 0.25));
                let liquid: Vec<Pos2> = (0..=16).map(|i| {
                    let a = -0.15 + (PI + 0.3) * i as f32 / 16.0;
                    p(a.cos() * 0.55, 0.25 + a.sin() * 0.55)
                }).collect();
                poly(liquid, color);
                painter.circle_stroke(p(0.0, 0.25), 0.6 * s, st(0.07, lite));
                painter.rect_filled(Rect::from_min_max(p(-0.2, -0.65), p(0.2, -0.3)), 1.0, with_alpha(Color32::WHITE, 0.3));
                painter.rect_filled(Rect::from_min_max(p(-0.26, -0.85), p(0.26, -0.6)), 2.0, hex("#9A6A3A"));
                painter.circle_filled(p(-0.22, 0.05), 0.1 * s, with_alpha(Color32::WHITE, 0.7));
            }
            Gem => {
                poly(vec![p(-0.75, -0.25), p(-0.4, -0.7), p(0.4, -0.7), p(0.75, -0.25), p(0.0, 0.8)], color);
                poly(vec![p(-0.4, -0.7), p(0.4, -0.7), p(0.2, -0.25), p(-0.2, -0.25)], lite);
                poly(vec![p(-0.75, -0.25), p(-0.2, -0.25), p(0.0, 0.8)], dark);
                painter.line_segment([p(-0.75, -0.25), p(0.75, -0.25)], st(0.04, lite));
            }
            Scroll | Map => {
                poly(vec![p(-0.6, -0.6), p(0.6, -0.6), p(0.6, 0.6), p(-0.6, 0.6)], color);
                painter.circle_filled(p(-0.6, 0.0), 0.0, color);
                painter.rect_filled(Rect::from_min_max(p(-0.75, -0.75), p(0.75, -0.55)), 3.0, dark);
                painter.rect_filled(Rect::from_min_max(p(-0.75, 0.55), p(0.75, 0.75)), 3.0, dark);
                if self == Map {
                    let route = [p(-0.4, 0.35), p(-0.1, 0.1), p(-0.3, -0.15), p(0.15, -0.3)];
                    for w in route.windows(2) {
                        painter.line_segment([w[0], w[1]], st(0.05, hex("#8E2F25")));
                    }
                    painter.line_segment([p(0.22, -0.4), p(0.42, -0.2)], st(0.08, hex("#C8342B")));
                    painter.line_segment([p(0.42, -0.4), p(0.22, -0.2)], st(0.08, hex("#C8342B")));
                } else {
                    for k in 0..4 {
                        let y = -0.3 + k as f32 * 0.2;
                        painter.line_segment([p(-0.4, y), p(if k == 3 { 0.1 } else { 0.4 }, y)], st(0.05, dark));
                    }
                }
            }
            Key => {
                painter.circle_stroke(p(-0.4, -0.35), 0.3 * s, st(0.14, color));
                painter.line_segment([p(-0.2, -0.15), p(0.7, 0.7)], st(0.14, color));
                painter.line_segment([p(0.4, 0.4), p(0.6, 0.2)], st(0.12, color));
                painter.line_segment([p(0.55, 0.55), p(0.75, 0.35)], st(0.12, color));
            }
            Coin => {
                painter.circle_filled(p(0.06, 0.06), 0.72 * s, dark);
                painter.circle_filled(c, 0.72 * s, color);
                painter.circle_stroke(c, 0.52 * s, st(0.06, dark));
                painter.text(c, Align2::CENTER_CENTER, "$", FontId::proportional(0.8 * s), dark);
            }
            Bomb => {
                painter.circle_filled(p(0.0, 0.15), 0.62 * s, color);
                painter.circle_filled(p(-0.2, -0.05), 0.16 * s, with_alpha(Color32::WHITE, 0.35));
                painter.rect_filled(Rect::from_min_max(p(-0.18, -0.65), p(0.18, -0.42)), 1.0, dark);
                painter.line_segment([p(0.0, -0.6), p(0.35, -0.85)], st(0.06, hex("#C9965A")));
                paint::sparkle(painter, p(0.42, -0.88), 0.25 * s, 0.3, hex("#FFD36A"));
            }
            Fire => {
                let outer = vec![p(0.0, -0.9), p(0.45, -0.3), p(0.62, 0.2), p(0.45, 0.65), p(0.0, 0.82), p(-0.45, 0.65), p(-0.62, 0.2), p(-0.35, -0.2)];
                poly(outer, color);
                poly(vec![p(0.0, -0.3), p(0.3, 0.2), p(0.25, 0.55), p(0.0, 0.7), p(-0.25, 0.55), p(-0.3, 0.2)], hex("#FFC14D"));
                poly(vec![p(0.0, 0.1), p(0.15, 0.4), p(0.0, 0.6), p(-0.15, 0.4)], hex("#FFF2C2"));
            }
            Frost => {
                for k in 0..3 {
                    let a = k as f32 * PI / 3.0;
                    let d = vec2(a.cos(), a.sin()) * 0.8 * s;
                    painter.line_segment([c - d, c + d], st(0.1, color));
                    for sgn in [-1.0, 1.0] {
                        let q = c + d * 0.6 * sgn;
                        let perp = vec2(-d.y, d.x) * 0.2;
                        painter.line_segment([q, q + d * 0.25 * sgn + perp], st(0.07, lite));
                        painter.line_segment([q, q + d * 0.25 * sgn - perp], st(0.07, lite));
                    }
                }
            }
            Bolt => {
                poly(vec![p(0.15, -0.9), p(-0.5, 0.1), p(0.0, 0.1)], color);
                poly(vec![p(0.0, -0.1), p(0.5, -0.1), p(-0.15, 0.9)], color);
                poly(vec![p(-0.05, -0.1), p(0.15, -0.9), p(0.05, 0.12)], color);
                painter.line_segment([p(0.1, -0.7), p(-0.3, 0.0)], st(0.05, lite));
            }
            Heal => {
                painter.rect_filled(Rect::from_center_size(c, vec2(0.45, 1.5) * s), 0.12 * s, color);
                painter.rect_filled(Rect::from_center_size(c, vec2(1.5, 0.45) * s), 0.12 * s, color);
                painter.rect_filled(Rect::from_center_size(p(0.0, -0.45), vec2(0.18, 0.4) * s), 1.0, lite);
            }
            Dash => {
                for k in 0..3 {
                    let x = -0.65 + k as f32 * 0.45;
                    let col = with_alpha(color, 0.45 + 0.27 * k as f32);
                    painter.add(Shape::line(vec![p(x, -0.6), p(x + 0.4, 0.0), p(x, 0.6)], st(0.16, col)));
                }
            }
            Skull => {
                painter.circle_filled(p(0.0, -0.15), 0.62 * s, color);
                painter.rect_filled(Rect::from_min_max(p(-0.35, 0.2), p(0.35, 0.72)), 0.15 * s, color);
                painter.circle_filled(p(-0.25, -0.12), 0.18 * s, Color32::from_black_alpha(230));
                painter.circle_filled(p(0.25, -0.12), 0.18 * s, Color32::from_black_alpha(230));
                poly(vec![p(0.0, 0.08), p(0.08, 0.25), p(-0.08, 0.25)], Color32::from_black_alpha(230));
                for k in 0..3 {
                    let x = -0.17 + k as f32 * 0.17;
                    painter.line_segment([p(x, 0.45), p(x, 0.72)], st(0.05, Color32::from_black_alpha(200)));
                }
            }
            Star => {
                paint::star(painter, c, 0.85 * s, 0.38 * s, 5, 0.0, color);
                paint::star(painter, p(-0.05, -0.05), 0.4 * s, 0.18 * s, 5, 0.0, lite);
            }
            Heart => {
                painter.circle_filled(p(-0.32, -0.2), 0.38 * s, color);
                painter.circle_filled(p(0.32, -0.2), 0.38 * s, color);
                poly(vec![p(-0.68, -0.08), p(0.68, -0.08), p(0.0, 0.78)], color);
                painter.circle_filled(p(-0.38, -0.3), 0.12 * s, lite);
            }
            Rifle => {
                poly(vec![p(-0.9, -0.12), p(0.5, -0.12), p(0.5, 0.08), p(-0.9, 0.08)], color);
                painter.line_segment([p(0.5, -0.04), p(0.95, -0.04)], st(0.07, color));
                poly(vec![p(-0.9, -0.12), p(-0.55, -0.12), p(-0.55, 0.3), p(-0.95, 0.4)], dark);
                poly(vec![p(-0.2, 0.08), p(-0.02, 0.08), p(-0.1, 0.5), p(-0.28, 0.5)], dark);
                poly(vec![p(0.1, 0.08), p(0.3, 0.08), p(0.32, 0.38), p(0.14, 0.38)], color);
                painter.rect_filled(Rect::from_min_max(p(-0.3, -0.3), p(0.15, -0.14)), 1.0, dark);
            }
            Pistol => {
                poly(vec![p(-0.6, -0.4), p(0.75, -0.4), p(0.75, -0.08), p(-0.6, -0.08)], color);
                poly(vec![p(-0.6, -0.08), p(-0.15, -0.08), p(-0.25, 0.75), p(-0.7, 0.7)], dark);
                painter.circle_stroke(p(0.05, 0.08), 0.15 * s, st(0.06, dark));
                painter.line_segment([p(-0.5, -0.3), p(0.6, -0.3)], st(0.04, lite));
            }
            Target => {
                painter.circle_stroke(c, 0.65 * s, st(0.1, color));
                painter.circle_stroke(c, 0.32 * s, st(0.08, color));
                painter.circle_filled(c, 0.1 * s, color);
                for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                    painter.line_segment([c + d * 0.5 * s, c + d * 0.92 * s], st(0.09, color));
                }
            }
            Poison => {
                poly(vec![p(0.0, -0.85), p(0.45, -0.1), p(0.55, 0.25), p(0.35, 0.65), p(0.0, 0.8), p(-0.35, 0.65), p(-0.55, 0.25), p(-0.45, -0.1)], color);
                painter.circle_filled(p(-0.18, 0.15), 0.12 * s, lite);
                painter.circle_filled(p(0.15, 0.4), 0.08 * s, dark);
            }
            Trophy => {
                poly(vec![p(-0.55, -0.75), p(0.55, -0.75), p(0.45, -0.1), p(0.15, 0.15), p(-0.15, 0.15), p(-0.45, -0.1)], color);
                painter.circle_stroke(p(-0.55, -0.4), 0.22 * s, st(0.08, color));
                painter.circle_stroke(p(0.55, -0.4), 0.22 * s, st(0.08, color));
                painter.rect_filled(Rect::from_min_max(p(-0.1, 0.1), p(0.1, 0.5)), 0.0, dark);
                painter.rect_filled(Rect::from_min_max(p(-0.45, 0.5), p(0.45, 0.78)), 2.0, color);
                painter.line_segment([p(-0.3, -0.6), p(-0.25, -0.2)], st(0.07, lite));
            }
        }
    }
}

/// Format an integer with thousands separators.
pub(crate) fn thousands(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if v < 0 {
        format!("-{out}")
    } else {
        out
    }
}
