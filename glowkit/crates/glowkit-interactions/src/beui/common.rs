//! Shared plumbing for the `beui` components: the spring "layout morph" every component is built
//! on (a rectangle whose position, size and corner radius ride springs), soft surfaces, painted
//! line icons, idle timers for autoplay and a ghost cursor.

use egui::{pos2, vec2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

/// Corner radius from points.
#[inline]
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// Linear interpolation.
#[inline]
pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The raised card color: the card in light themes, a lifted muted tone in dark ones.
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.06)
    } else {
        t.card
    }
}

/// A second, inset tone for wells and rows sitting on a [`surface`].
pub(crate) fn well(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.11)
    } else {
        mix(t.card, t.muted, 0.85)
    }
}

/// Hover / pressed fill on a [`surface`].
pub(crate) fn hover_fill(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.15)
    } else {
        shade(t.muted, -0.025)
    }
}

/// Hairline around surfaces.
pub(crate) fn hairline(t: &Theme) -> Stroke {
    Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.07) } else { with_alpha(t.border, 0.9) })
}

/// Shadow color scaled by `k` (0..1).
pub(crate) fn shadow_color(t: &Theme, k: f32) -> Color32 {
    if t.dark {
        Color32::from_black_alpha((160.0 * k).clamp(0.0, 255.0) as u8)
    } else {
        Color32::from_rgba_unmultiplied(20, 20, 40, (34.0 * k).clamp(0.0, 255.0) as u8)
    }
}

/// A raised card: soft shadow, fill and hairline.
pub(crate) fn card(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32) {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0).max(0.0);
    if elevation > 0.0 {
        paint::soft_shadow(
            painter,
            rect,
            r,
            10.0 + 14.0 * elevation,
            vec2(0.0, 2.0 + 6.0 * elevation),
            shadow_color(t, elevation.min(1.5)),
        );
    }
    painter.rect(rect, cr(r), surface(t), hairline(t), egui::StrokeKind::Inside);
}

/// Seconds since `active` was last true for widget `id` (starts large so autoplay begins at once).
pub(crate) fn idle_for(ui: &Ui, id: Id, active: bool) -> f32 {
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(
        ui,
        id.with("beui_idle"),
        || now - 1000.0,
        |last| {
            if active {
                *last = now;
            }
            (now - *last) as f32
        },
    )
}

/// Width of a single line of text.
pub(crate) fn text_width(painter: &Painter, s: &str, size: f32) -> f32 {
    painter.layout_no_wrap(s.to_owned(), FontId::proportional(size), Color32::WHITE).size().x
}

/// The layout morph: a rectangle whose center, width, height and corner radius each ride a spring
/// toward `target`, the way a shared-layout animation tweens between two states. Returns the
/// current rectangle and radius.
pub(crate) fn morph(ui: &Ui, id: Id, target: Rect, radius: f32, stiffness: f32, damping: f32) -> (Rect, f32) {
    let c = target.center();
    let x = spring(ui, id.with("mx"), c.x, stiffness, damping);
    let y = spring(ui, id.with("my"), c.y, stiffness, damping);
    let w = spring(ui, id.with("mw"), target.width(), stiffness, damping);
    let h = spring(ui, id.with("mh"), target.height(), stiffness, damping);
    let r = spring(ui, id.with("mr"), radius, stiffness, damping);
    (Rect::from_center_size(pos2(x, y), vec2(w.max(0.0), h.max(0.0))), r.max(0.0))
}

/// Same as [`morph`] but width and height have their own spring settings, so a box can pour open
/// along one axis before the other.
pub(crate) fn morph_axes(ui: &Ui, id: Id, target: Rect, radius: f32, kx: (f32, f32), ky: (f32, f32)) -> (Rect, f32) {
    let c = target.center();
    let x = spring(ui, id.with("mx"), c.x, kx.0, kx.1);
    let y = spring(ui, id.with("my"), c.y, ky.0, ky.1);
    let w = spring(ui, id.with("mw"), target.width(), kx.0, kx.1);
    let h = spring(ui, id.with("mh"), target.height(), ky.0, ky.1);
    let r = spring(ui, id.with("mr"), radius, (kx.0 + ky.0) / 2.0, (kx.1 + ky.1) / 2.0);
    (Rect::from_center_size(pos2(x, y), vec2(w.max(0.0), h.max(0.0))), r.max(0.0))
}

/// Progress (0..1) of a morph between two sizes measured along one axis.
pub(crate) fn progress(cur: f32, from: f32, to: f32) -> f32 {
    if (to - from).abs() < 1e-3 {
        1.0
    } else {
        ((cur - from) / (to - from)).clamp(0.0, 1.2)
    }
}

/// A small arrow cursor (used to show autoplay "ghost" interactions).
pub(crate) fn ghost_cursor(painter: &Painter, t: &Theme, tip: Pos2, alpha: f32, pressed: bool) {
    if alpha <= 0.01 {
        return;
    }
    let s = if pressed { 0.88 } else { 1.0 };
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y) * s)
        .collect();
    let (fill, line) = if t.dark { (Color32::WHITE, Color32::BLACK) } else { (Color32::BLACK, Color32::WHITE) };
    let c = tip + vec2(3.5, 9.0) * s;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        painter.add(Shape::convex_polygon(vec![c, a, b], with_alpha(fill, alpha), Stroke::NONE));
    }
    let mut closed = pts.clone();
    closed.push(pts[0]);
    painter.add(Shape::line(closed, Stroke::new(1.2, with_alpha(line, alpha))));
}

/// Two-stop gradient disc with a faint top highlight (avatars, badges).
pub(crate) fn orb(painter: &Painter, center: Pos2, r: f32, colors: [Color32; 2]) {
    let pts = paint::circle_points(center, r, 40);
    let rect = Rect::from_center_size(center, Vec2::splat(r * 2.0));
    paint::fill_convex_fn(painter, &pts, 3, paint::linear_fn(rect, 135.0, &colors));
    paint::arc(painter, center, r - 1.0, -2.6, -1.4, 1.1, with_alpha(Color32::WHITE, 0.35));
}

/// Format a number with thousands separators and two decimals.
pub(crate) fn money(v: f64) -> String {
    let neg = v < 0.0;
    let cents = (v.abs() * 100.0).round() as u64;
    let whole = cents / 100;
    let frac = cents % 100;
    let s = whole.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{}{out}.{frac:02}", if neg { "-" } else { "" })
}

// ---------------------------------------------------------------------------------------------
// Icons

/// Line icons used by the beUI-style components, painted with strokes so they look the same on
/// every platform (no icon font needed).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_interactions::BeIcon::Bell.paint(ui.painter(), egui::pos2(20.0, 20.0), 16.0, egui::Color32::GRAY);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BeIcon {
    /// A plus sign.
    Plus,
    /// A minus sign.
    Minus,
    /// An x.
    Close,
    /// A check mark.
    Check,
    /// Downward chevron.
    ChevronDown,
    /// Right chevron.
    ChevronRight,
    /// Left chevron.
    ChevronLeft,
    /// Arrow pointing up and right.
    ArrowUpRight,
    /// Arrow pointing right.
    ArrowRight,
    /// Arrow pointing up.
    ArrowUp,
    /// Arrow pointing down into a tray.
    Download,
    /// A paper plane.
    Send,
    /// Two overlapping squares.
    Copy,
    /// A box with a lid.
    Archive,
    /// A bell.
    Bell,
    /// A cog.
    Settings,
    /// A magnifying glass.
    Search,
    /// A clock face.
    Clock,
    /// An eye.
    Eye,
    /// An eye with a slash.
    EyeOff,
    /// A push pin.
    Pin,
    /// Three horizontal dots.
    Dots,
    /// A waste bin.
    Trash,
    /// An envelope.
    Mail,
    /// A person.
    User,
    /// A flag.
    Flag,
    /// A speech bubble.
    Chat,
    /// A calendar page.
    Calendar,
    /// A shield.
    Shield,
    /// A page with a folded corner.
    Doc,
    /// Four tiles.
    Grid,
    /// A spreadsheet.
    Table,
    /// A folder.
    Folder,
    /// Two chain links.
    Link,
    /// Three stacked diamonds.
    Layers,
    /// A rocket.
    Rocket,
    /// A painter's palette.
    Palette,
    /// Six grip dots.
    Grip,
    /// A curved undo arrow.
    Undo,
    /// A star outline.
    Star,
    /// A bookmark ribbon.
    Bookmark,
    /// Two arrows swapping.
    Swap,
    /// A credit card.
    Card,
    /// A trending-up line.
    Trend,
    /// A share arrow out of a box.
    Share,
    /// A pencil.
    Pencil,
    /// A cube.
    Cube,
    /// A broadcast antenna.
    Antenna,
    /// A code file.
    Code,
    /// A trophy cup.
    Trophy,
    /// A wallet.
    Wallet,
    /// A sparkle.
    Sparkle,
    /// A circular refresh arrow.
    Refresh,
}

impl BeIcon {
    /// Paint the icon centered on `center`, `size` points tall, with a stroke scaled to the size.
    pub fn paint(self, painter: &Painter, center: Pos2, size: f32, color: Color32) {
        icon(painter, self, center, size, color, (size / 11.0).clamp(1.1, 2.2));
    }
}

/// Paint `which` centered on `c` in a box `size` tall with stroke width `width`.
pub(crate) fn icon(painter: &Painter, which: BeIcon, c: Pos2, size: f32, color: Color32, width: f32) {
    let s = size / 2.0;
    let p = |x: f32, y: f32| c + vec2(x, y) * s;
    let st = Stroke::new(width, color);
    let line = |pts: Vec<Pos2>| {
        painter.add(Shape::line(pts, st));
    };
    let ring = |cx: f32, cy: f32, r: f32| {
        painter.circle_stroke(p(cx, cy), r * s, st);
    };
    let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        painter.rect_stroke(Rect::from_min_max(p(x0, y0), p(x1, y1)), cr(r * s), st, egui::StrokeKind::Middle);
    };
    match which {
        BeIcon::Plus => {
            line(vec![p(-0.75, 0.0), p(0.75, 0.0)]);
            line(vec![p(0.0, -0.75), p(0.0, 0.75)]);
        }
        BeIcon::Minus => line(vec![p(-0.75, 0.0), p(0.75, 0.0)]),
        BeIcon::Close => {
            line(vec![p(-0.6, -0.6), p(0.6, 0.6)]);
            line(vec![p(0.6, -0.6), p(-0.6, 0.6)]);
        }
        BeIcon::Check => line(vec![p(-0.7, 0.05), p(-0.2, 0.55), p(0.75, -0.5)]),
        BeIcon::ChevronDown => line(vec![p(-0.55, -0.25), p(0.0, 0.3), p(0.55, -0.25)]),
        BeIcon::ChevronRight => line(vec![p(-0.25, -0.55), p(0.3, 0.0), p(-0.25, 0.55)]),
        BeIcon::ChevronLeft => line(vec![p(0.25, -0.55), p(-0.3, 0.0), p(0.25, 0.55)]),
        BeIcon::ArrowUpRight => {
            line(vec![p(-0.6, 0.6), p(0.6, -0.6)]);
            line(vec![p(-0.3, -0.6), p(0.6, -0.6), p(0.6, 0.3)]);
        }
        BeIcon::ArrowRight => {
            line(vec![p(-0.75, 0.0), p(0.75, 0.0)]);
            line(vec![p(0.15, -0.6), p(0.75, 0.0), p(0.15, 0.6)]);
        }
        BeIcon::ArrowUp => {
            line(vec![p(0.0, 0.75), p(0.0, -0.75)]);
            line(vec![p(-0.6, -0.15), p(0.0, -0.75), p(0.6, -0.15)]);
        }
        BeIcon::Download => {
            line(vec![p(0.0, -0.8), p(0.0, 0.3)]);
            line(vec![p(-0.45, -0.15), p(0.0, 0.3), p(0.45, -0.15)]);
            line(vec![p(-0.8, 0.35), p(-0.8, 0.8), p(0.8, 0.8), p(0.8, 0.35)]);
        }
        BeIcon::Send => {
            line(vec![p(-0.8, -0.1), p(0.8, -0.75), p(0.15, 0.8), p(-0.1, 0.05), p(-0.8, -0.1)]);
            line(vec![p(-0.1, 0.05), p(0.8, -0.75)]);
        }
        BeIcon::Copy => {
            rrect(-0.75, -0.35, 0.35, 0.8, 0.2);
            line(vec![p(-0.35, -0.45), p(-0.35, -0.8), p(0.8, -0.8), p(0.8, 0.35), p(0.45, 0.35)]);
        }
        BeIcon::Archive => {
            rrect(-0.85, -0.75, 0.85, -0.3, 0.1);
            line(vec![p(-0.7, -0.3), p(-0.7, 0.8), p(0.7, 0.8), p(0.7, -0.3)]);
            line(vec![p(-0.25, 0.1), p(0.25, 0.1)]);
        }
        BeIcon::Bell => {
            let mut pts = vec![p(-0.75, 0.45)];
            for i in 0..=12 {
                let a = PI + i as f32 / 12.0 * PI;
                pts.push(c + vec2(a.cos() * 0.5, a.sin() * 0.55 - 0.05) * s + vec2(0.0, -0.1 * s));
            }
            pts.insert(1, p(-0.5, 0.2));
            pts.push(p(0.5, 0.2));
            pts.push(p(0.75, 0.45));
            pts.push(p(-0.75, 0.45));
            line(pts);
            line(vec![p(-0.2, 0.72), p(0.2, 0.72)]);
        }
        BeIcon::Settings => {
            ring(0.0, 0.0, 0.28);
            let mut pts = Vec::new();
            for i in 0..=48 {
                let a = i as f32 / 48.0 * TAU;
                let tooth = ((a * 6.0).cos() * 2.2).clamp(-1.0, 1.0) * 0.5 + 0.5;
                let r = 0.6 + 0.18 * tooth;
                pts.push(p(a.cos() * r, a.sin() * r));
            }
            line(pts);
        }
        BeIcon::Search => {
            ring(-0.15, -0.15, 0.55);
            line(vec![p(0.25, 0.25), p(0.75, 0.75)]);
        }
        BeIcon::Clock => {
            ring(0.0, 0.0, 0.8);
            line(vec![p(0.0, -0.45), p(0.0, 0.0), p(0.35, 0.2)]);
        }
        BeIcon::Eye | BeIcon::EyeOff => {
            let mut top = Vec::new();
            let mut bot = Vec::new();
            for i in 0..=16 {
                let u = i as f32 / 16.0 * 2.0 - 1.0;
                let h = (1.0 - u * u).sqrt() * 0.5;
                top.push(p(u * 0.85, -h));
                bot.push(p(u * 0.85, h));
            }
            line(top);
            line(bot);
            ring(0.0, 0.0, 0.22);
            if which == BeIcon::EyeOff {
                line(vec![p(-0.75, -0.75), p(0.75, 0.75)]);
            }
        }
        BeIcon::Pin => {
            line(vec![p(-0.35, -0.8), p(0.35, -0.8)]);
            line(vec![p(-0.2, -0.8), p(-0.25, -0.1), p(-0.55, 0.2), p(0.55, 0.2), p(0.25, -0.1), p(0.2, -0.8)]);
            line(vec![p(0.0, 0.2), p(0.0, 0.85)]);
        }
        BeIcon::Dots => {
            for x in [-0.6, 0.0, 0.6] {
                painter.circle_filled(p(x, 0.0), width * 0.85, color);
            }
        }
        BeIcon::Trash => {
            line(vec![p(-0.8, -0.5), p(0.8, -0.5)]);
            line(vec![p(-0.3, -0.5), p(-0.3, -0.8), p(0.3, -0.8), p(0.3, -0.5)]);
            line(vec![p(-0.6, -0.5), p(-0.5, 0.8), p(0.5, 0.8), p(0.6, -0.5)]);
        }
        BeIcon::Mail => {
            rrect(-0.85, -0.6, 0.85, 0.6, 0.15);
            line(vec![p(-0.8, -0.5), p(0.0, 0.1), p(0.8, -0.5)]);
        }
        BeIcon::User => {
            ring(0.0, -0.35, 0.36);
            let mut pts = Vec::new();
            for i in 0..=14 {
                let a = PI + i as f32 / 14.0 * PI;
                pts.push(p(a.cos() * 0.68, 0.8 + a.sin() * 0.62));
            }
            line(pts);
        }
        BeIcon::Flag => {
            line(vec![p(-0.6, 0.85), p(-0.6, -0.8)]);
            line(vec![
                p(-0.6, -0.75),
                p(0.1, -0.85),
                p(0.25, -0.65),
                p(0.75, -0.7),
                p(0.75, 0.15),
                p(0.2, 0.2),
                p(0.05, 0.0),
                p(-0.6, 0.1),
            ]);
        }
        BeIcon::Chat => {
            line(vec![
                p(-0.8, -0.55),
                p(-0.8, 0.35),
                p(-0.45, 0.35),
                p(-0.45, 0.8),
                p(0.05, 0.35),
                p(0.8, 0.35),
                p(0.8, -0.55),
                p(-0.8, -0.55),
            ]);
        }
        BeIcon::Calendar => {
            rrect(-0.8, -0.6, 0.8, 0.8, 0.18);
            line(vec![p(-0.8, -0.2), p(0.8, -0.2)]);
            line(vec![p(-0.4, -0.85), p(-0.4, -0.45)]);
            line(vec![p(0.4, -0.85), p(0.4, -0.45)]);
        }
        BeIcon::Shield => {
            line(vec![
                p(0.0, -0.85),
                p(0.7, -0.6),
                p(0.65, 0.1),
                p(0.35, 0.55),
                p(0.0, 0.85),
                p(-0.35, 0.55),
                p(-0.65, 0.1),
                p(-0.7, -0.6),
                p(0.0, -0.85),
            ]);
            line(vec![p(-0.3, 0.0), p(-0.05, 0.25), p(0.35, -0.2)]);
        }
        BeIcon::Doc => {
            line(vec![p(0.2, -0.85), p(-0.65, -0.85), p(-0.65, 0.85), p(0.65, 0.85), p(0.65, -0.4), p(0.2, -0.85)]);
            line(vec![p(0.2, -0.85), p(0.2, -0.4), p(0.65, -0.4)]);
            line(vec![p(-0.3, 0.15), p(0.3, 0.15)]);
            line(vec![p(-0.3, 0.45), p(0.3, 0.45)]);
        }
        BeIcon::Grid => {
            for (x, y) in [(-0.42, -0.42), (0.42, -0.42), (-0.42, 0.42), (0.42, 0.42)] {
                rrect(x - 0.3, y - 0.3, x + 0.3, y + 0.3, 0.1);
            }
        }
        BeIcon::Table => {
            rrect(-0.8, -0.8, 0.8, 0.8, 0.2);
            line(vec![p(-0.8, -0.27), p(0.8, -0.27)]);
            line(vec![p(-0.8, 0.27), p(0.8, 0.27)]);
            line(vec![p(-0.27, -0.27), p(-0.27, 0.8)]);
        }
        BeIcon::Folder => {
            line(vec![
                p(-0.85, -0.6),
                p(-0.25, -0.6),
                p(-0.05, -0.35),
                p(0.85, -0.35),
                p(0.85, 0.7),
                p(-0.85, 0.7),
                p(-0.85, -0.6),
            ]);
        }
        BeIcon::Link => {
            for (dx, dy) in [(-0.22, 0.22), (0.22, -0.22)] {
                let cc = p(dx, dy);
                let d = vec2(1.0, -1.0).normalized();
                let n = d.rot90();
                let hl = 0.38 * s;
                let rr = 0.26 * s;
                let mut pts = Vec::new();
                for i in 0..=20 {
                    let a = i as f32 / 20.0 * PI;
                    pts.push(cc + d * hl + (n * a.cos() + d * a.sin()) * rr);
                }
                for i in 0..=20 {
                    let a = i as f32 / 20.0 * PI;
                    pts.push(cc - d * hl - (n * a.cos() + d * a.sin()) * rr);
                }
                pts.push(pts[0]);
                painter.add(Shape::line(pts, st));
            }
        }
        BeIcon::Layers => {
            for (k, dy) in [(0, -0.35), (1, 0.05), (2, 0.45)] {
                let y = dy;
                if k == 0 {
                    line(vec![p(0.0, y - 0.4), p(0.8, y), p(0.0, y + 0.4), p(-0.8, y), p(0.0, y - 0.4)]);
                } else {
                    line(vec![p(-0.8, y - 0.05), p(0.0, y + 0.35), p(0.8, y - 0.05)]);
                }
            }
        }
        BeIcon::Rocket => {
            line(vec![p(-0.35, 0.35), p(-0.05, -0.25), p(0.75, -0.75), p(0.25, 0.05), p(-0.35, 0.35)]);
            line(vec![p(-0.2, 0.05), p(-0.65, 0.05), p(-0.4, -0.25), p(-0.05, -0.25)]);
            line(vec![p(-0.05, 0.2), p(-0.05, 0.65), p(0.25, 0.4), p(0.25, 0.05)]);
            line(vec![p(-0.45, 0.45), p(-0.75, 0.75)]);
        }
        BeIcon::Palette => {
            let mut pts = Vec::new();
            for i in 0..=36 {
                let a = 0.9 + i as f32 / 36.0 * (TAU - 0.9);
                pts.push(p(a.cos() * 0.8, a.sin() * 0.8));
            }
            pts.push(p(0.25, 0.35));
            pts.push(p(0.5, 0.62));
            line(pts);
            for (x, y) in [(-0.35, -0.35), (0.15, -0.5), (-0.5, 0.15)] {
                painter.circle_filled(p(x, y), width * 0.9, color);
            }
        }
        BeIcon::Grip => {
            for x in [-0.3, 0.3] {
                for y in [-0.6, 0.0, 0.6] {
                    painter.circle_filled(p(x, y), width * 0.95, color);
                }
            }
        }
        BeIcon::Undo => {
            let mut pts = Vec::new();
            for i in 0..=16 {
                let a = -PI / 2.0 + i as f32 / 16.0 * PI;
                pts.push(p(0.1 + a.cos() * 0.6, 0.0 + a.sin() * 0.6));
            }
            pts.push(p(-0.5, 0.6));
            pts.reverse();
            let start = p(0.1, -0.6);
            pts.push(start);
            pts.push(p(-0.75, -0.6));
            line(pts);
            line(vec![p(-0.4, -0.95), p(-0.75, -0.6), p(-0.4, -0.25)]);
        }
        BeIcon::Star => {
            let mut pts = Vec::new();
            for i in 0..=10 {
                let a = -PI / 2.0 + i as f32 / 10.0 * TAU;
                let r = if i % 2 == 0 { 0.85 } else { 0.38 };
                pts.push(p(a.cos() * r, a.sin() * r + 0.05));
            }
            line(pts);
        }
        BeIcon::Bookmark => {
            line(vec![p(-0.55, -0.8), p(0.55, -0.8), p(0.55, 0.85), p(0.0, 0.45), p(-0.55, 0.85), p(-0.55, -0.8)]);
        }
        BeIcon::Swap => {
            line(vec![p(-0.75, -0.3), p(0.75, -0.3)]);
            line(vec![p(0.4, -0.65), p(0.75, -0.3), p(0.4, 0.05)]);
            line(vec![p(0.75, 0.35), p(-0.75, 0.35)]);
            line(vec![p(-0.4, 0.0), p(-0.75, 0.35), p(-0.4, 0.7)]);
        }
        BeIcon::Card => {
            rrect(-0.85, -0.6, 0.85, 0.6, 0.15);
            line(vec![p(-0.85, -0.2), p(0.85, -0.2)]);
        }
        BeIcon::Trend => {
            line(vec![p(-0.85, 0.5), p(-0.25, -0.1), p(0.1, 0.25), p(0.8, -0.5)]);
            line(vec![p(0.3, -0.5), p(0.8, -0.5), p(0.8, 0.0)]);
        }
        BeIcon::Share => {
            line(vec![p(0.0, 0.3), p(0.0, -0.85)]);
            line(vec![p(-0.4, -0.45), p(0.0, -0.85), p(0.4, -0.45)]);
            line(vec![p(-0.35, -0.1), p(-0.75, -0.1), p(-0.75, 0.8), p(0.75, 0.8), p(0.75, -0.1), p(0.35, -0.1)]);
        }
        BeIcon::Pencil => {
            line(vec![p(-0.75, 0.75), p(-0.65, 0.3), p(0.4, -0.75), p(0.75, -0.4), p(-0.3, 0.65), p(-0.75, 0.75)]);
            line(vec![p(0.15, -0.5), p(0.5, -0.15)]);
        }
        BeIcon::Cube => {
            line(vec![
                p(0.0, -0.85),
                p(0.75, -0.45),
                p(0.75, 0.45),
                p(0.0, 0.85),
                p(-0.75, 0.45),
                p(-0.75, -0.45),
                p(0.0, -0.85),
            ]);
            line(vec![p(-0.75, -0.45), p(0.0, -0.05), p(0.75, -0.45)]);
            line(vec![p(0.0, -0.05), p(0.0, 0.85)]);
        }
        BeIcon::Antenna => {
            painter.circle_filled(p(0.0, -0.1), width * 1.1, color);
            line(vec![p(0.0, -0.1), p(-0.35, 0.85)]);
            line(vec![p(0.0, -0.1), p(0.35, 0.85)]);
            for (r, a) in [(0.45, 0.7), (0.8, 0.7)] {
                paint::arc(painter, p(0.0, -0.1), r * s, -a - PI / 2.0 - 0.6, -PI / 2.0 - 0.9 + 0.3, width, color);
                paint::arc(painter, p(0.0, -0.1), r * s, -PI / 2.0 + 0.6, -PI / 2.0 + 1.3 + 0.2, width, color);
            }
        }
        BeIcon::Code => {
            line(vec![p(0.2, -0.85), p(-0.65, -0.85), p(-0.65, 0.85), p(0.65, 0.85), p(0.65, -0.4), p(0.2, -0.85)]);
            line(vec![p(-0.15, -0.05), p(-0.38, 0.2), p(-0.15, 0.45)]);
            line(vec![p(0.15, -0.05), p(0.38, 0.2), p(0.15, 0.45)]);
        }
        BeIcon::Trophy => {
            line(vec![p(-0.5, -0.8), p(0.5, -0.8), p(0.45, -0.1), p(0.0, 0.25), p(-0.45, -0.1), p(-0.5, -0.8)]);
            line(vec![p(-0.5, -0.6), p(-0.85, -0.6), p(-0.75, -0.2), p(-0.42, -0.1)]);
            line(vec![p(0.5, -0.6), p(0.85, -0.6), p(0.75, -0.2), p(0.42, -0.1)]);
            line(vec![p(0.0, 0.25), p(0.0, 0.6)]);
            line(vec![p(-0.4, 0.8), p(0.4, 0.8)]);
        }
        BeIcon::Wallet => {
            rrect(-0.85, -0.6, 0.85, 0.7, 0.2);
            rrect(0.2, -0.15, 0.85, 0.3, 0.1);
        }
        BeIcon::Sparkle => {
            paint::sparkle(painter, c, s * 0.95, 0.0, color);
        }
        BeIcon::Refresh => {
            paint::arc(painter, c, s * 0.68, -PI * 0.35, PI * 1.45, width, color);
            let a1 = -PI * 0.35;
            let tip = c + Vec2::angled(a1) * s * 0.68;
            line(vec![tip + vec2(-0.45, -0.1) * s, tip, tip + vec2(0.05, 0.45) * s]);
        }
    }
}

/// Fill a star-shaped polygon as a triangle fan around `center` (no feathering spikes on cusps),
/// with a hairline rim in the same color for anti-aliasing.
pub(crate) fn fill_fan(painter: &Painter, center: Pos2, pts: &[Pos2], color: Color32) {
    if pts.len() < 3 {
        return;
    }
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(center, color);
    for q in pts {
        mesh.colored_vertex(*q, color);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
    let mut rim = pts.to_vec();
    rim.push(pts[0]);
    painter.add(Shape::line(rim, Stroke::new(0.7, color)));
}
