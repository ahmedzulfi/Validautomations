//! Shared helpers for the suiss components: palette tokens, gesture physics (rubber-banding,
//! flick projection), a phone frame, a ghost finger and painted icons.

use egui::epaint::RectShape;
use egui::{pos2, vec2, Color32, CornerRadius, Id, Painter, Pos2, Rect, Shape, Stroke, TextureId, Ui, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------------------------------------
// Palette

/// Card surface that reads as raised on a `muted` stage in light and dark themes.
pub(crate) fn surface(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.muted, 0.07)
    } else {
        t.card
    }
}

/// The stage behind phone-like previews.
pub(crate) fn stage(t: &Theme) -> Color32 {
    if t.dark {
        shade(t.background, 0.035)
    } else {
        hexa(0xF4, 0xF4, 0xF6)
    }
}

/// A recessed neutral (fields, tracks).
pub(crate) fn recess(t: &Theme) -> Color32 {
    mix(surface(t), t.foreground, if t.dark { 0.08 } else { 0.055 })
}

/// A hairline separator color.
pub(crate) fn hairline(t: &Theme) -> Color32 {
    mix(surface(t), t.foreground, if t.dark { 0.12 } else { 0.09 })
}

/// iOS-style system blue that works in both themes.
pub(crate) fn system_blue(t: &Theme) -> Color32 {
    if t.dark {
        hexa(0x3B, 0x8B, 0xFF)
    } else {
        hexa(0x0A, 0x72, 0xF0)
    }
}

pub(crate) fn hexa(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Corner radius from a float.
pub(crate) fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// A soft, blurred drop shadow under a rounded rect.
pub(crate) fn shadow(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32) {
    if elevation <= 0.01 {
        return;
    }
    let a = if t.dark { 0.55 } else { 0.13 };
    let col = with_alpha(if t.dark { Color32::BLACK } else { hexa(20, 20, 40) }, a * elevation.min(1.6));
    let r = rect.translate(vec2(0.0, 6.0 * elevation)).expand(1.0);
    painter.add(Shape::Rect(RectShape::filled(r, cr(radius), col).with_blur_width(22.0 * elevation)));
}

/// A raised card: shadow, fill, optional hairline.
pub(crate) fn card(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32, fill: Color32) {
    shadow(painter, t, rect, radius, elevation);
    let stroke = if t.dark { Stroke::new(1.0, with_alpha(t.border, 0.8)) } else { Stroke::NONE };
    painter.rect(rect, cr(radius), fill, stroke, egui::StrokeKind::Inside);
}

/// A rounded, textured rectangle (an image with corners), optionally rotated about its center.
pub(crate) fn image_rect(painter: &Painter, rect: Rect, radius: f32, tex: TextureId, uv: Rect, tint: Color32) {
    painter.add(Shape::Rect(RectShape::filled(rect, cr(radius), tint).with_texture(tex, uv)));
}

/// UV sub-rect that "covers" a target of aspect `target_aspect` from a texture of aspect `tex_aspect`.
pub(crate) fn cover_uv(tex_aspect: f32, target_aspect: f32) -> Rect {
    if target_aspect > tex_aspect {
        let h = tex_aspect / target_aspect;
        Rect::from_min_max(pos2(0.0, 0.5 - h / 2.0), pos2(1.0, 0.5 + h / 2.0))
    } else {
        let w = target_aspect / tex_aspect;
        Rect::from_min_max(pos2(0.5 - w / 2.0, 0.0), pos2(0.5 + w / 2.0, 1.0))
    }
}

// ---------------------------------------------------------------------------------------------
// Physics

/// iOS-style rubber band: maps an overscroll distance `x` to a resisted offset that never
/// exceeds `limit`. Feels linear for small pulls and asymptotically stiff for large ones.
pub(crate) fn rubber(x: f32, limit: f32) -> f32 {
    if limit <= 0.0 {
        return 0.0;
    }
    let c = 0.55;
    let s = x.signum();
    s * (1.0 - 1.0 / (x.abs() * c / limit + 1.0)) * limit
}

/// Clamp `x` into `lo..=hi`, rubber-banding whatever spills over by up to `limit`.
pub(crate) fn rubber_clamp(x: f32, lo: f32, hi: f32, limit: f32) -> f32 {
    if x < lo {
        lo + rubber(x - lo, limit)
    } else if x > hi {
        hi + rubber(x - hi, limit)
    } else {
        x
    }
}

/// Where a flick at `velocity` (points / s) would coast to with deceleration rate `rate`
/// (0.998 = UIScrollView normal).
pub(crate) fn project(velocity: f32, rate: f32) -> f32 {
    (velocity / 1000.0) * rate / (1.0 - rate)
}

/// Damping for a spring of `stiffness` with a 0..100 "bounce" amount (0 = critically damped).
pub(crate) fn damping_for(bounce: f32, stiffness: f32) -> f32 {
    2.0 * stiffness.sqrt() * glowkit_core::anim::lerp(1.0, 0.18, (bounce / 100.0).clamp(0.0, 1.0))
}

/// A minimal spring stepper (semi-implicit Euler, sub-stepped).
pub(crate) fn spring_step(value: &mut f32, vel: &mut f32, target: f32, stiffness: f32, damping: f32, dt: f32) {
    let steps = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 16);
    let h = dt / steps as f32;
    for _ in 0..steps {
        let f = -stiffness * (*value - target) - damping * *vel;
        *vel += f * h;
        *value += *vel * h;
    }
}

/// True when a spring is at rest.
pub(crate) fn settled(value: f32, vel: f32, target: f32) -> bool {
    (value - target).abs() < 0.05 && vel.abs() < 0.5
}

/// Seconds since the widget was last interacted with (or first shown).
pub(crate) fn idle_for(ui: &Ui, id: Id, interacting: bool) -> f32 {
    let now = glowkit_core::time(ui);
    let since = glowkit_core::with_state(
        ui,
        id.with("suiss_idle"),
        || now,
        |t| {
            if interacting {
                *t = now;
            }
            *t
        },
    );
    now - since
}

/// Smooth 0..1 ramp between `a` and `b`.
pub(crate) fn ramp(x: f32, a: f32, b: f32) -> f32 {
    glowkit_core::anim::smoothstep(a, b, x)
}

// ---------------------------------------------------------------------------------------------
// Frames and ghost touches

/// Paint the back of a phone-shaped device (shadow + screen fill) inside `rect` and return the
/// screen rect. Paint content, then call [`phone_bezel`] to mask the screen's rounded corners.
pub(crate) fn phone(painter: &Painter, t: &Theme, rect: Rect, screen: Color32) -> Rect {
    let r = phone_radius(rect);
    shadow(painter, t, rect, r, 1.2);
    let inner = rect.shrink(6.0);
    painter.rect_filled(inner, cr(r - 6.0), screen);
    inner
}

fn phone_radius(rect: Rect) -> f32 {
    34.0_f32.min(rect.width() * 0.16)
}

/// The phone's bezel, painted on top of the screen content so it masks the screen's corners.
pub(crate) fn phone_bezel(painter: &Painter, t: &Theme, rect: Rect) {
    let r = phone_radius(rect);
    let bezel = if t.dark { hexa(0x2A, 0x2A, 0x30) } else { hexa(0x1C, 0x1C, 0x1F) };
    let outer = paint::rounded_rect_points(rect, r, 10);
    let inner = paint::rounded_rect_points(rect.shrink(6.0), r - 6.0, 10);
    if outer.len() == inner.len() && !outer.is_empty() {
        let mut mesh = egui::epaint::Mesh::default();
        let n = outer.len() as u32;
        for (o, i) in outer.iter().zip(&inner) {
            mesh.colored_vertex(*o, bezel);
            mesh.colored_vertex(*i, bezel);
        }
        for k in 0..n {
            let a = 2 * k;
            let b = 2 * ((k + 1) % n);
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(a + 1, b + 1, b);
        }
        painter.add(Shape::mesh(mesh));
    }
    if t.dark {
        painter.rect_stroke(rect, cr(r), Stroke::new(1.0, with_alpha(Color32::WHITE, 0.08)), egui::StrokeKind::Inside);
    }
}

/// The little status bar (time + battery) at the top of a phone screen.
pub(crate) fn status_bar(painter: &Painter, screen: Rect, color: Color32) {
    let y = screen.min.y + 15.0;
    painter.text(
        pos2(screen.min.x + 26.0, y),
        egui::Align2::LEFT_CENTER,
        "9:41",
        egui::FontId::proportional(11.5),
        color,
    );
    let bx = screen.max.x - 26.0;
    let b = Rect::from_min_max(pos2(bx - 20.0, y - 5.0), pos2(bx, y + 5.0));
    painter.rect_stroke(b, cr(3.0), Stroke::new(1.0, with_alpha(color, 0.5)), egui::StrokeKind::Inside);
    painter.rect_filled(b.shrink(2.0).with_max_x(b.min.x + 14.0), cr(1.5), color);
    for i in 0..3 {
        let h = 3.0 + i as f32 * 2.0;
        let x = bx - 34.0 + i as f32 * 4.0;
        painter.rect_filled(Rect::from_min_max(pos2(x, y + 4.0 - h), pos2(x + 2.5, y + 4.0)), cr(1.0), color);
    }
}

/// A translucent "ghost" fingertip that shows scripted gestures while the demo autoplays.
pub(crate) fn ghost_touch(painter: &Painter, pos: Pos2, pressed: f32, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let r = 15.0 - 3.0 * pressed;
    painter.circle_filled(pos, r + 6.0 * pressed, with_alpha(hexa(120, 120, 140), 0.12 * alpha * pressed));
    painter.circle_filled(pos, r, with_alpha(hexa(140, 140, 160), (0.28 + 0.14 * pressed) * alpha));
    painter.circle_stroke(pos, r, Stroke::new(1.5, with_alpha(Color32::WHITE, 0.75 * alpha)));
}

/// Piecewise-linear interpolation through `(time, value)` keys with smooth easing per segment.
pub(crate) fn keys(t: f32, k: &[(f32, f32)]) -> f32 {
    if k.is_empty() {
        return 0.0;
    }
    if t <= k[0].0 {
        return k[0].1;
    }
    for w in k.windows(2) {
        let (t0, v0) = w[0];
        let (t1, v1) = w[1];
        if t <= t1 {
            let u = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1.0 };
            let u = glowkit_core::ease(glowkit_core::Easing::InOutCubic, u);
            return v0 + (v1 - v0) * u;
        }
    }
    k[k.len() - 1].1
}

// ---------------------------------------------------------------------------------------------
// Icons

/// Small line icons painted with strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph {
    Trash,
    Archive,
    Flag,
    Pin,
    Share,
    Heart,
    Star,
    Check,
    Close,
    Search,
    Undo,
    Copy,
    Bell,
    Download,
    Mic,
    Phone,
    Laptop,
    Tablet,
    Globe,
    Grip,
    ChevronUp,
    ChevronDown,
    ChevronRight,
    Mail,
    Info,
    Sparkle,
    Timer,
    Music,
    Folder,
    Plus,
    Edit,
}

/// Paint a glyph centered at `c` in a box of `size` points.
pub(crate) fn glyph(painter: &Painter, g: Glyph, c: Pos2, size: f32, color: Color32) {
    let s = size / 2.0;
    let w = (size * 0.1).clamp(1.2, 2.4);
    let st = Stroke::new(w, color);
    let p = |x: f32, y: f32| c + vec2(x * s, y * s);
    let line = |pts: Vec<Pos2>| {
        painter.add(Shape::line(pts, st));
    };
    let ring = |cc: Pos2, r: f32, a0: f32, a1: f32| {
        let n = 24;
        let pts: Vec<Pos2> =
            (0..=n).map(|i| a0 + (a1 - a0) * i as f32 / n as f32).map(|a| cc + vec2(a.cos(), a.sin()) * r).collect();
        painter.add(Shape::line(pts, st));
    };
    match g {
        Glyph::Trash => {
            line(vec![p(-0.75, -0.55), p(0.75, -0.55)]);
            line(vec![p(-0.25, -0.55), p(-0.2, -0.8), p(0.2, -0.8), p(0.25, -0.55)]);
            line(vec![p(-0.55, -0.55), p(-0.45, 0.8), p(0.45, 0.8), p(0.55, -0.55)]);
            line(vec![p(-0.15, -0.25), p(-0.12, 0.5)]);
            line(vec![p(0.15, -0.25), p(0.12, 0.5)]);
        }
        Glyph::Archive => {
            line(vec![p(-0.8, -0.65), p(0.8, -0.65), p(0.8, -0.3), p(-0.8, -0.3), p(-0.8, -0.65)]);
            line(vec![p(-0.65, -0.3), p(-0.65, 0.75), p(0.65, 0.75), p(0.65, -0.3)]);
            line(vec![p(-0.25, 0.05), p(0.25, 0.05)]);
        }
        Glyph::Flag => {
            line(vec![p(-0.55, 0.85), p(-0.55, -0.8)]);
            line(vec![p(-0.55, -0.75), p(0.65, -0.75), p(0.35, -0.35), p(0.65, 0.05), p(-0.55, 0.05)]);
        }
        Glyph::Pin => {
            ring(p(0.0, -0.3), s * 0.42, 0.0, TAU);
            line(vec![p(-0.36, -0.08), p(0.0, 0.85), p(0.36, -0.08)]);
        }
        Glyph::Share => {
            line(vec![p(-0.35, -0.2), p(-0.65, -0.2), p(-0.65, 0.8), p(0.65, 0.8), p(0.65, -0.2), p(0.35, -0.2)]);
            line(vec![p(0.0, 0.3), p(0.0, -0.85)]);
            line(vec![p(-0.32, -0.55), p(0.0, -0.85), p(0.32, -0.55)]);
        }
        Glyph::Heart => {
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let a = TAU * i as f32 / 40.0;
                    let x = 16.0 * a.sin().powi(3);
                    let y = -(13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos());
                    p(x / 19.0, y / 19.0 + 0.05)
                })
                .collect();
            painter.add(Shape::line(pts, st));
        }
        Glyph::Star => {
            let pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let a = -PI / 2.0 + PI * i as f32 / 5.0;
                    let r = if i % 2 == 0 { 0.85 } else { 0.38 };
                    p(a.cos() * r, a.sin() * r + 0.06)
                })
                .collect();
            painter.add(Shape::line(pts, st));
        }
        Glyph::Check => line(vec![p(-0.7, 0.02), p(-0.22, 0.5), p(0.72, -0.5)]),
        Glyph::Close => {
            line(vec![p(-0.6, -0.6), p(0.6, 0.6)]);
            line(vec![p(-0.6, 0.6), p(0.6, -0.6)]);
        }
        Glyph::Plus => {
            line(vec![p(-0.7, 0.0), p(0.7, 0.0)]);
            line(vec![p(0.0, -0.7), p(0.0, 0.7)]);
        }
        Glyph::Search => {
            ring(p(-0.15, -0.15), s * 0.55, 0.0, TAU);
            line(vec![p(0.25, 0.25), p(0.75, 0.75)]);
        }
        Glyph::Undo => {
            ring(p(0.05, 0.1), s * 0.6, -PI * 0.95, PI * 0.55);
            line(vec![p(-0.85, -0.45), p(-0.55, 0.05), p(-0.1, -0.2)]);
        }
        Glyph::Copy => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.35, -0.35), p(0.75, 0.75)),
                cr(s * 0.18),
                st,
                egui::StrokeKind::Middle,
            );
            line(vec![p(-0.75, 0.35), p(-0.75, -0.75), p(0.35, -0.75)]);
        }
        Glyph::Bell => {
            ring(p(0.0, -0.2), s * 0.5, PI, TAU);
            line(vec![p(-0.5, -0.2), p(-0.55, 0.4), p(-0.75, 0.55), p(0.75, 0.55), p(0.55, 0.4), p(0.5, -0.2)]);
            line(vec![p(-0.18, 0.8), p(0.18, 0.8)]);
        }
        Glyph::Download => {
            line(vec![p(0.0, -0.8), p(0.0, 0.35)]);
            line(vec![p(-0.4, -0.05), p(0.0, 0.35), p(0.4, -0.05)]);
            line(vec![p(-0.75, 0.75), p(0.75, 0.75)]);
        }
        Glyph::Mic => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.28, -0.85), p(0.28, 0.2)),
                cr(s * 0.28),
                st,
                egui::StrokeKind::Middle,
            );
            ring(p(0.0, -0.1), s * 0.55, 0.0, PI);
            line(vec![p(0.0, 0.45), p(0.0, 0.85)]);
        }
        Glyph::Phone => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.42, -0.85), p(0.42, 0.85)),
                cr(s * 0.18),
                st,
                egui::StrokeKind::Middle,
            );
            line(vec![p(-0.12, 0.6), p(0.12, 0.6)]);
        }
        Glyph::Tablet => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.65, -0.85), p(0.65, 0.85)),
                cr(s * 0.15),
                st,
                egui::StrokeKind::Middle,
            );
            line(vec![p(-0.1, 0.62), p(0.1, 0.62)]);
        }
        Glyph::Laptop => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.65, -0.65), p(0.65, 0.35)),
                cr(s * 0.1),
                st,
                egui::StrokeKind::Middle,
            );
            line(vec![p(-0.9, 0.62), p(0.9, 0.62)]);
        }
        Glyph::Globe => {
            ring(c, s * 0.82, 0.0, TAU);
            line(vec![p(-0.82, 0.0), p(0.82, 0.0)]);
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = -PI / 2.0 + PI * i as f32 / 16.0;
                    p(a.cos() * 0.38, a.sin() * 0.82)
                })
                .collect();
            painter.add(Shape::line(pts.clone(), st));
            painter.add(Shape::line(pts.iter().map(|q| pos2(2.0 * c.x - q.x, q.y)).collect(), st));
        }
        Glyph::Grip => {
            for y in [-0.45, 0.0, 0.45] {
                line(vec![p(-0.7, y), p(0.7, y)]);
            }
        }
        Glyph::ChevronUp => line(vec![p(-0.6, 0.3), p(0.0, -0.3), p(0.6, 0.3)]),
        Glyph::ChevronDown => line(vec![p(-0.6, -0.3), p(0.0, 0.3), p(0.6, -0.3)]),
        Glyph::ChevronRight => line(vec![p(-0.25, -0.6), p(0.3, 0.0), p(-0.25, 0.6)]),
        Glyph::Mail => {
            painter.rect_stroke(
                Rect::from_min_max(p(-0.8, -0.6), p(0.8, 0.6)),
                cr(s * 0.12),
                st,
                egui::StrokeKind::Middle,
            );
            line(vec![p(-0.75, -0.5), p(0.0, 0.08), p(0.75, -0.5)]);
        }
        Glyph::Info => {
            ring(c, s * 0.82, 0.0, TAU);
            line(vec![p(0.0, -0.1), p(0.0, 0.45)]);
            painter.circle_filled(p(0.0, -0.4), w * 0.7, color);
        }
        Glyph::Sparkle => paint::sparkle(painter, c, s * 0.95, 0.0, color),
        Glyph::Timer => {
            ring(p(0.0, 0.1), s * 0.7, 0.0, TAU);
            line(vec![p(0.0, 0.1), p(0.0, -0.25)]);
            line(vec![p(-0.2, -0.85), p(0.2, -0.85)]);
        }
        Glyph::Music => {
            line(vec![p(-0.3, 0.55), p(-0.3, -0.7), p(0.6, -0.85), p(0.6, 0.4)]);
            painter.circle_filled(p(-0.48, 0.58), s * 0.22, color);
            painter.circle_filled(p(0.42, 0.43), s * 0.22, color);
        }
        Glyph::Folder => {
            line(vec![
                p(-0.85, 0.7),
                p(-0.85, -0.65),
                p(-0.3, -0.65),
                p(-0.15, -0.45),
                p(0.85, -0.45),
                p(0.85, 0.7),
                p(-0.85, 0.7),
            ]);
        }
        Glyph::Edit => {
            line(vec![p(-0.7, 0.7), p(-0.65, 0.35), p(0.45, -0.75), p(0.75, -0.45), p(-0.35, 0.65), p(-0.7, 0.7)]);
        }
    }
}

/// Text helper: paint `text` and return its rect.
pub(crate) fn text(
    painter: &Painter,
    pos: Pos2,
    align: egui::Align2,
    s: impl ToString,
    size: f32,
    color: Color32,
) -> Rect {
    painter.text(pos, align, s.to_string(), egui::FontId::proportional(size), color)
}

/// Bold-ish text (painted twice with a sub-pixel offset; egui ships no bold face).
pub(crate) fn text_bold(
    painter: &Painter,
    pos: Pos2,
    align: egui::Align2,
    s: impl ToString,
    size: f32,
    color: Color32,
) -> Rect {
    let s = s.to_string();
    painter.text(pos + vec2(0.35, 0.0), align, &s, egui::FontId::proportional(size), color);
    painter.text(pos, align, s, egui::FontId::proportional(size), color)
}

/// Width of `s` at `size`.
pub(crate) fn text_width(painter: &Painter, s: &str, size: f32) -> f32 {
    painter.layout_no_wrap(s.to_string(), egui::FontId::proportional(size), Color32::WHITE).size().x
}

/// A colorful painted avatar disc with initials.
pub(crate) fn avatar(painter: &Painter, c: Pos2, r: f32, seed: usize, initials: &str) {
    let hues = [
        hexa(0xF4, 0x72, 0xB6),
        hexa(0x60, 0xA5, 0xFA),
        hexa(0x34, 0xD3, 0x99),
        hexa(0xFB, 0x92, 0x3C),
        hexa(0xA7, 0x8B, 0xFA),
        hexa(0x22, 0xD3, 0xEE),
        hexa(0xFA, 0xCC, 0x15),
        hexa(0xF8, 0x71, 0x71),
    ];
    let a = hues[seed % hues.len()];
    let b = hues[(seed + 3) % hues.len()];
    let rect = Rect::from_center_size(c, Vec2::splat(r * 2.0));
    let pts = paint::circle_points(c, r, 32);
    paint::fill_convex_fn(painter, &pts, 2, paint::linear_fn(rect, 135.0, &[a, b]));
    painter.text(c, egui::Align2::CENTER_CENTER, initials, egui::FontId::proportional(r * 0.8), Color32::WHITE);
}

// ---------------------------------------------------------------------------------------------
// Touch input: the real pointer, or a scripted ghost finger while the demo idles

/// One scripted finger stroke in normalized (0..1) coordinates of the component rect.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stroke2 {
    /// Press time (seconds into the cycle).
    pub t0: f32,
    /// Release time.
    pub t1: f32,
    /// Press position.
    pub a: (f32, f32),
    /// Release position.
    pub b: (f32, f32),
    /// Seconds the finger rests at `a` before moving.
    pub hold: f32,
}

/// A tap at `(x, y)` at time `t`.
pub(crate) const fn tap(t: f32, x: f32, y: f32) -> Stroke2 {
    Stroke2 { t0: t, t1: t + 0.09, a: (x, y), b: (x, y), hold: 0.0 }
}

/// A drag from `a` to `b` between `t0` and `t1`.
pub(crate) const fn drag(t0: f32, t1: f32, a: (f32, f32), b: (f32, f32)) -> Stroke2 {
    Stroke2 { t0, t1, a, b, hold: 0.0 }
}

/// Press at `a`, rest for `hold` seconds, then drag to `b`, releasing at `t1`.
pub(crate) const fn hold_drag(t0: f32, hold: f32, t1: f32, a: (f32, f32), b: (f32, f32)) -> Stroke2 {
    Stroke2 { t0, t1, a, b, hold }
}

/// The pointer as the component sees it this frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Touch {
    /// Pointer position (hovering or pressed), if any.
    pub pos: Option<Pos2>,
    /// Whether the (real or ghost) finger is down.
    pub down: bool,
    /// True while the ghost script drives the component.
    pub ghost: bool,
    /// Which loop of the script is playing (changes when the script restarts).
    pub cycle: u32,
    /// Seconds into the current ghost cycle.
    pub t: f32,
    /// Ghost finger visibility 0..1 (for painting).
    pub alpha: f32,
}

/// Seconds of inactivity before the ghost takes over.
pub(crate) const GHOST_DELAY: f32 = 1.6;

/// Resolve the touch for this frame. When `autoplay` is on and the user has left the widget
/// alone for a moment, a ghost finger plays `script` (looping every `cycle` seconds).
pub(crate) fn touch(
    ui: &Ui,
    response: &egui::Response,
    rect: Rect,
    autoplay: bool,
    script: &[Stroke2],
    cycle: f32,
) -> Touch {
    let real_down = response.is_pointer_button_down_on();
    let hovered = response.hovered() || real_down || response.contains_pointer();
    let idle = idle_for(ui, response.id, hovered);
    if autoplay && !hovered && idle > GHOST_DELAY && !script.is_empty() {
        let e = idle - GHOST_DELAY;
        let cyc = (e / cycle) as u32;
        let t = e % cycle;
        // Track strokes explicitly so a slow frame rate can never skip a press or a release:
        // every stroke is seen as at least press (at `a`) -> move (at `b`) -> release.
        let key = response.id.with("suiss_ghost_track");
        let mut tr: (u32, usize, Option<usize>, u8) = glowkit_core::state(ui, key, || (cyc, 0, None, 0));
        if tr.0 != cyc {
            tr = (cyc, 0, None, 0);
        }
        let (mut p, mut down) = ghost_pos(t, script);
        match tr.2 {
            None => {
                if let Some(s) = script.get(tr.1) {
                    if t >= s.t0 {
                        tr.2 = Some(tr.1);
                        tr.3 = 0;
                        down = true;
                        if t > s.t1 {
                            p = s.a;
                        }
                    } else {
                        down = false;
                    }
                } else {
                    down = false;
                }
            }
            Some(k) => {
                let s = script[k];
                if t <= s.t1 {
                    down = true;
                    tr.3 = 1;
                } else if tr.3 == 0 {
                    down = true;
                    p = s.b;
                    tr.3 = 1;
                } else {
                    down = false;
                    p = s.b;
                    tr.2 = None;
                    tr.1 = k + 1;
                }
            }
        }
        glowkit_core::set_state(ui, key, tr);
        let first = script[0].t0;
        let last = script[script.len() - 1].t1;
        let alpha = ramp(t, first - 0.6, first - 0.2) * (1.0 - ramp(t, last + 0.3, last + 0.7));
        let pos = pos2(rect.min.x + p.0 * rect.width(), rect.min.y + p.1 * rect.height());
        ui.ctx().request_repaint();
        return Touch { pos: Some(pos), down, ghost: true, cycle: cyc + 1, t, alpha };
    }
    if autoplay && !hovered {
        // keep time flowing so the ghost can wake up
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    }
    let pos = response.interact_pointer_pos().or(response.hover_pos());
    Touch { pos, down: real_down, ghost: false, cycle: 0, t: 0.0, alpha: 0.0 }
}

fn ghost_pos(t: f32, script: &[Stroke2]) -> ((f32, f32), bool) {
    let lerp2 = |a: (f32, f32), b: (f32, f32), u: f32| (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
    if t < script[0].t0 {
        let a = (script[0].a.0 + 0.12, script[0].a.1 + 0.25);
        let u = ramp(t, script[0].t0 - 0.9, script[0].t0);
        return (lerp2(a, script[0].a, u), false);
    }
    for (i, s) in script.iter().enumerate() {
        if t <= s.t1 && t >= s.t0 {
            let m0 = s.t0 + s.hold;
            let u = ((t - m0) / (s.t1 - m0).max(1e-3)).clamp(0.0, 1.0);
            // ease in, then carry speed through release (a flick)
            let u = u * u * (2.0 - u);
            return (lerp2(s.a, s.b, u), true);
        }
        if let Some(n) = script.get(i + 1) {
            if t > s.t1 && t < n.t0 {
                let u = ((t - s.t1) / (n.t0 - s.t1).max(1e-3)).clamp(0.0, 1.0);
                let u = glowkit_core::ease(glowkit_core::Easing::InOutCubic, u);
                return (lerp2(s.b, n.a, u), false);
            }
        }
    }
    let l = script[script.len() - 1];
    let u = ramp(t, l.t1, l.t1 + 0.8);
    (lerp2(l.b, (l.b.0 + 0.1, l.b.1 + 0.2), u), false)
}

/// Per-widget gesture tracker: press / move / release, velocity, taps and double taps.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Gesture {
    pub down: bool,
    pub start: Pos2,
    pub last: Pos2,
    pub vel: Vec2,
    pub t_down: f32,
    pub moved: f32,
    pub last_tap: f32,
    pub last_tap_pos: Pos2,
}

impl Default for Gesture {
    fn default() -> Self {
        Self {
            down: false,
            start: Pos2::ZERO,
            last: Pos2::ZERO,
            vel: Vec2::ZERO,
            t_down: 0.0,
            moved: 0.0,
            last_tap: -10.0,
            last_tap_pos: Pos2::ZERO,
        }
    }
}

/// What happened this frame.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GFrame {
    pub pressed: bool,
    pub released: bool,
    pub delta: Vec2,
    pub tap: bool,
    pub double_tap: bool,
}

impl Gesture {
    /// Advance with this frame's touch.
    pub fn update(&mut self, touch: &Touch, now: f32, dt: f32) -> GFrame {
        let mut f = GFrame::default();
        match (self.down, touch.down, touch.pos) {
            (false, true, Some(p)) => {
                self.down = true;
                self.start = p;
                self.last = p;
                self.vel = Vec2::ZERO;
                self.t_down = now;
                self.moved = 0.0;
                f.pressed = true;
            }
            (true, true, Some(p)) => {
                f.delta = p - self.last;
                if dt > 0.0 {
                    let v = f.delta / dt;
                    self.vel = self.vel * 0.55 + v * 0.45;
                }
                self.moved = self.moved.max((p - self.start).length());
                self.last = p;
            }
            (true, false, _) | (true, true, None) => {
                self.down = false;
                f.released = true;
                if self.moved < 8.0 && (touch.ghost || now - self.t_down < 0.35) {
                    f.tap = true;
                    let window = if touch.ghost { 1.2 } else { 0.38 };
                    if now - self.last_tap < window && (self.last - self.last_tap_pos).length() < 40.0 {
                        f.double_tap = true;
                        self.last_tap = -10.0;
                    } else {
                        self.last_tap = now;
                        self.last_tap_pos = self.last;
                    }
                }
            }
            _ => {}
        }
        f
    }
}

/// A 1D scroll position with flick momentum and rubber-banded edges.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Scroller {
    /// Scroll offset (content moved up by this much).
    pub s: f32,
    /// Velocity in points per second.
    pub v: f32,
}

impl Scroller {
    /// Follow a finger delta (positive = finger moved down), resisting beyond `min..max`.
    pub fn drag(&mut self, finger_dy: f32, min: f32, max: f32) {
        let over = if self.s < min {
            min - self.s
        } else if self.s > max {
            self.s - max
        } else {
            0.0
        };
        let k = 1.0 / (1.0 + over / 60.0).powi(2);
        self.s -= finger_dy * k;
    }

    /// Coast after release: decelerate inside bounds, spring back outside them.
    /// Returns true while still moving.
    pub fn step(&mut self, min: f32, max: f32, dt: f32) -> bool {
        let max = max.max(min);
        if self.s < min || self.s > max {
            let target = self.s.clamp(min, max);
            // a stiff critically damped spring, but let momentum carry into the band first
            spring_step(&mut self.s, &mut self.v, target, 180.0, 2.0 * 180f32.sqrt(), dt);
            if (self.s - target).abs() < 0.3 && self.v.abs() < 5.0 {
                self.s = target;
                self.v = 0.0;
            }
        } else {
            self.s += self.v * dt;
            self.v *= (0.998f32).powf(dt * 1000.0);
            if self.v.abs() < 4.0 {
                self.v = 0.0;
            }
        }
        self.v != 0.0 || self.s < min || self.s > max
    }
}
