//! Shared building blocks for the form, feedback and data components: control sizes, the
//! animated focus ring, field chrome, and a small set of vector line icons.

use egui::{pos2, vec2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use std::f32::consts::{PI, TAU};

/// Size scale shared by buttons, inputs, selects and toggles: 32 / 36 / 40 points tall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlSize {
    /// 32 pt tall, 13 pt text.
    Sm,
    /// 36 pt tall, 14 pt text (the default).
    #[default]
    Md,
    /// 40 pt tall, 15 pt text.
    Lg,
}

impl ControlSize {
    /// Control height in points.
    pub fn height(self) -> f32 {
        match self {
            ControlSize::Sm => 32.0,
            ControlSize::Md => 36.0,
            ControlSize::Lg => 40.0,
        }
    }
    /// Label font size in points.
    pub fn font(self) -> f32 {
        match self {
            ControlSize::Sm => 13.0,
            ControlSize::Md => 14.0,
            ControlSize::Lg => 15.0,
        }
    }
    /// Horizontal padding for buttons.
    pub fn pad_x(self) -> f32 {
        match self {
            ControlSize::Sm => 12.0,
            ControlSize::Md => 16.0,
            ControlSize::Lg => 22.0,
        }
    }
    /// Icon size in points.
    pub fn icon(self) -> f32 {
        match self {
            ControlSize::Sm => 14.0,
            ControlSize::Md => 16.0,
            ControlSize::Lg => 17.0,
        }
    }
}

/// Corner radius used by controls (a little tighter than cards).
pub(crate) fn control_radius(t: &Theme) -> CornerRadius {
    t.corner_scaled(0.8)
}

/// Animate a bool to 0..1 over `secs` (ease handled by egui).
pub(crate) fn anim(ui: &Ui, id: egui::Id, on: bool, secs: f32) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, secs)
}

/// The shadcn-style focus halo: a soft 3 pt ring outside `rect`, faded by `k`.
pub(crate) fn focus_halo(painter: &Painter, rect: Rect, radius: CornerRadius, color: Color32, k: f32) {
    if k <= 0.01 {
        return;
    }
    let grow = 1.0 + 2.0 * k;
    painter.rect_stroke(rect, radius, Stroke::new(grow, with_alpha(color, 0.32 * k)), StrokeKind::Outside);
}

/// Background fill for text fields.
pub(crate) fn field_fill(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(t.input, 0.30)
    } else {
        t.background
    }
}

/// Paint the chrome of a text-like field (background, border, hover and focus/error ring).
pub(crate) fn field_chrome(
    painter: &Painter,
    rect: Rect,
    t: &Theme,
    hover: f32,
    focus: f32,
    error: bool,
    disabled: bool,
) {
    let r = control_radius(t);
    let mut fill = field_fill(t);
    if disabled {
        fill = if t.dark { with_alpha(t.input, 0.15) } else { t.muted };
    }
    let base = if hover > 0.0 { t.input.lerp_to_gamma(t.muted_foreground, 0.25 * hover) } else { t.input };
    let accent = if error { t.destructive } else { t.ring };
    let border =
        if error { t.destructive.lerp_to_gamma(base, 0.25 * (1.0 - focus)) } else { base.lerp_to_gamma(accent, focus) };
    if !t.dark && !disabled {
        glowkit_core::paint::soft_shadow(
            painter,
            rect,
            r.nw as f32,
            2.0,
            vec2(0.0, 1.0),
            Color32::from_black_alpha(10),
        );
    }
    painter.rect(rect, r, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    focus_halo(painter, rect, r, accent, if error { focus.max(0.35) } else { focus });
}

/// Snap a value to the physical pixel grid.
pub(crate) fn snap(ui: &Ui, v: f32) -> f32 {
    let ppp = ui.ctx().pixels_per_point();
    (v * ppp).round() / ppp
}

/// Paint a single line of text and return its rect.
pub(crate) fn text_at(
    painter: &Painter,
    pos: Pos2,
    align: egui::Align2,
    text: &str,
    size: f32,
    color: Color32,
) -> Rect {
    painter.text(pos, align, text, FontId::proportional(size), color)
}

/// Measure a line of text.
pub(crate) fn text_width(ui: &Ui, text: &str, size: f32) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE).size().x
}

/// Small rotating arc used as an inline loading indicator.
pub(crate) fn mini_spinner(painter: &Painter, center: Pos2, radius: f32, color: Color32, time: f32) {
    let width = (radius * 0.28).max(1.5);
    painter.circle_stroke(center, radius, Stroke::new(width, with_alpha(color, 0.2)));
    let a0 = time * TAU * 1.1;
    glowkit_core::paint::arc(painter, center, radius, a0, a0 + TAU * 0.28, width, color);
}

/// Draw the check mark path, `progress` 0..1 draws it like a pen stroke.
pub(crate) fn check_path(painter: &Painter, rect: Rect, color: Color32, width: f32, progress: f32) {
    if progress <= 0.0 {
        return;
    }
    let p = |x: f32, y: f32| pos2(rect.min.x + rect.width() * x / 24.0, rect.min.y + rect.height() * y / 24.0);
    let pts = [p(5.0, 12.5), p(10.0, 17.5), p(19.5, 7.0)];
    partial_polyline(painter, &pts, progress, Stroke::new(width, color));
}

/// Stroke the first `progress` (0..1) of a polyline's length.
pub(crate) fn partial_polyline(painter: &Painter, pts: &[Pos2], progress: f32, stroke: Stroke) {
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * progress.clamp(0.0, 1.0);
    let mut out = vec![pts[0]];
    for w in pts.windows(2) {
        let d = w[0].distance(w[1]);
        if left >= d {
            out.push(w[1]);
            left -= d;
        } else {
            out.push(w[0] + (w[1] - w[0]) * (left / d.max(1e-4)));
            break;
        }
    }
    if out.len() >= 2 {
        painter.add(Shape::line(out, stroke));
    }
}

/// Vector line icons in a 24-unit grid (lucide-like proportions), so controls never depend on
/// which glyphs a font happens to ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UiIcon {
    Check,
    X,
    Plus,
    Minus,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    ChevronsUpDown,
    Search,
    Eye,
    EyeOff,
    Calendar,
    Upload,
    Download,
    Info,
    AlertCircle,
    AlertTriangle,
    CheckCircle,
    XCircle,
    Copy,
    Trash,
    Mail,
    User,
    Users,
    Settings,
    Star,
    Heart,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    ArrowUpDown,
    Bold,
    Italic,
    Underline,
    AlignLeft,
    AlignCenter,
    AlignRight,
    File,
    Folder,
    FolderOpen,
    Home,
    Lock,
    Image,
    MoreHorizontal,
    Filter,
    Clock,
    Code,
    Sparkles,
    Bell,
    Inbox,
    Globe,
    Link,
    Grid,
    List,
    Zap,
    Dollar,
    Activity,
    Terminal,
}

impl UiIcon {
    /// Paint the icon centered in `rect` (use a square rect) with the given color.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        let s = rect.width().min(rect.height());
        let rect = Rect::from_center_size(rect.center(), vec2(s, s));
        let k = s / 24.0;
        let w = (2.0 * k).clamp(1.15, 2.2);
        let st = Stroke::new(w, color);
        let o = rect.min;
        let p = |x: f32, y: f32| pos2(o.x + x * k, o.y + y * k);
        let line = |pts: &[(f32, f32)]| {
            painter.add(Shape::line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
        };
        let closed = |pts: &[(f32, f32)]| {
            painter.add(Shape::closed_line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
        };
        let circle = |x: f32, y: f32, r: f32| {
            painter.circle_stroke(p(x, y), r * k, st);
        };
        let dot = |x: f32, y: f32, r: f32| {
            painter.circle_filled(p(x, y), r * k, color);
        };
        let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
            painter.rect_stroke(
                Rect::from_min_max(p(x0, y0), p(x1, y1)),
                CornerRadius::same((r * k).round() as u8),
                st,
                StrokeKind::Middle,
            );
        };
        let arc = |cx: f32, cy: f32, r: f32, a0: f32, a1: f32| -> Vec<Pos2> {
            let n = 16;
            (0..=n)
                .map(|i| {
                    let a = a0 + (a1 - a0) * i as f32 / n as f32;
                    p(cx + r * a.cos(), cy + r * a.sin())
                })
                .collect()
        };
        match self {
            UiIcon::Check => line(&[(20.0, 6.0), (9.0, 17.0), (4.0, 12.0)]),
            UiIcon::X => {
                line(&[(18.0, 6.0), (6.0, 18.0)]);
                line(&[(6.0, 6.0), (18.0, 18.0)]);
            }
            UiIcon::Plus => {
                line(&[(12.0, 5.0), (12.0, 19.0)]);
                line(&[(5.0, 12.0), (19.0, 12.0)]);
            }
            UiIcon::Minus => line(&[(5.0, 12.0), (19.0, 12.0)]),
            UiIcon::ChevronDown => line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]),
            UiIcon::ChevronUp => line(&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)]),
            UiIcon::ChevronLeft => line(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]),
            UiIcon::ChevronRight => line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
            UiIcon::ChevronsUpDown => {
                line(&[(7.0, 15.0), (12.0, 20.0), (17.0, 15.0)]);
                line(&[(7.0, 9.0), (12.0, 4.0), (17.0, 9.0)]);
            }
            UiIcon::Search => {
                circle(11.0, 11.0, 7.0);
                line(&[(21.0, 21.0), (16.2, 16.2)]);
            }
            UiIcon::Eye | UiIcon::EyeOff => {
                let mut pts = glowkit_core::paint::quad_bezier(p(2.0, 12.0), p(12.0, 2.5), p(22.0, 12.0), 14);
                pts.extend(
                    glowkit_core::paint::quad_bezier(p(22.0, 12.0), p(12.0, 21.5), p(2.0, 12.0), 14)
                        .into_iter()
                        .skip(1),
                );
                painter.add(Shape::closed_line(pts, st));
                circle(12.0, 12.0, 3.0);
                if self == UiIcon::EyeOff {
                    line(&[(3.0, 3.0), (21.0, 21.0)]);
                }
            }
            UiIcon::Calendar => {
                rrect(3.0, 5.0, 21.0, 21.0, 2.5);
                line(&[(3.0, 10.0), (21.0, 10.0)]);
                line(&[(8.0, 3.0), (8.0, 7.0)]);
                line(&[(16.0, 3.0), (16.0, 7.0)]);
            }
            UiIcon::Upload => {
                line(&[(12.0, 15.0), (12.0, 3.0)]);
                line(&[(7.0, 8.0), (12.0, 3.0), (17.0, 8.0)]);
                line(&[(3.0, 15.0), (3.0, 20.0), (21.0, 20.0), (21.0, 15.0)]);
            }
            UiIcon::Download => {
                line(&[(12.0, 3.0), (12.0, 15.0)]);
                line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]);
                line(&[(3.0, 15.0), (3.0, 20.0), (21.0, 20.0), (21.0, 15.0)]);
            }
            UiIcon::Info => {
                circle(12.0, 12.0, 10.0);
                line(&[(12.0, 16.0), (12.0, 11.5)]);
                dot(12.0, 8.0, 1.2);
            }
            UiIcon::AlertCircle => {
                circle(12.0, 12.0, 10.0);
                line(&[(12.0, 7.5), (12.0, 12.5)]);
                dot(12.0, 16.0, 1.2);
            }
            UiIcon::AlertTriangle => {
                closed(&[(12.0, 3.0), (22.0, 20.5), (2.0, 20.5)]);
                line(&[(12.0, 9.0), (12.0, 13.5)]);
                dot(12.0, 17.0, 1.2);
            }
            UiIcon::CheckCircle => {
                circle(12.0, 12.0, 10.0);
                line(&[(8.0, 12.0), (11.0, 15.0), (16.5, 9.5)]);
            }
            UiIcon::XCircle => {
                circle(12.0, 12.0, 10.0);
                line(&[(15.0, 9.0), (9.0, 15.0)]);
                line(&[(9.0, 9.0), (15.0, 15.0)]);
            }
            UiIcon::Copy => {
                rrect(8.0, 8.0, 21.0, 21.0, 2.0);
                line(&[(4.0, 16.0), (4.0, 5.0), (5.0, 4.0), (16.0, 4.0)]);
            }
            UiIcon::Trash => {
                line(&[(3.0, 6.0), (21.0, 6.0)]);
                line(&[(5.0, 6.0), (6.0, 21.0), (18.0, 21.0), (19.0, 6.0)]);
                line(&[(9.0, 6.0), (9.0, 3.0), (15.0, 3.0), (15.0, 6.0)]);
            }
            UiIcon::Mail => {
                rrect(2.0, 5.0, 22.0, 19.0, 2.0);
                line(&[(2.5, 7.0), (12.0, 13.0), (21.5, 7.0)]);
            }
            UiIcon::User => {
                circle(12.0, 8.0, 4.0);
                painter.add(Shape::line(arc(12.0, 21.0, 7.5, PI, TAU), st));
            }
            UiIcon::Users => {
                circle(9.0, 8.0, 3.6);
                painter.add(Shape::line(arc(9.0, 20.5, 6.5, PI, TAU), st));
                painter.add(Shape::line(arc(16.0, 8.0, 3.6, -PI * 0.5, PI * 0.5), st));
                painter.add(Shape::line(arc(17.0, 20.5, 5.0, PI * 1.5, TAU), st));
            }
            UiIcon::Settings => {
                let mut pts = Vec::new();
                for i in 0..8 {
                    let base = i as f32 / 8.0 * TAU;
                    let tw = 0.2;
                    for (da, r) in [(-0.39 + 0.02, 7.6), (-tw, 7.9), (-tw * 0.8, 10.2), (tw * 0.8, 10.2), (tw, 7.9)] {
                        let a = base + da;
                        pts.push(p(12.0 + r * a.cos(), 12.0 + r * a.sin()));
                    }
                }
                painter.add(Shape::closed_line(pts, st));
                circle(12.0, 12.0, 3.0);
            }
            UiIcon::Star => {
                let pts: Vec<Pos2> = (0..10)
                    .map(|i| {
                        let a = i as f32 / 10.0 * TAU - PI / 2.0;
                        let r = if i % 2 == 0 { 10.0 } else { 4.3 };
                        p(12.0 + r * a.cos(), 12.6 + r * a.sin())
                    })
                    .collect();
                painter.add(Shape::closed_line(pts, st));
            }
            UiIcon::Heart => {
                let mut pts = vec![p(12.0, 20.5)];
                pts.extend(arc(7.5, 8.75, 5.25, PI * 0.75, PI * 1.75));
                pts.push(p(12.0, 5.6));
                pts.extend(arc(16.5, 8.75, 5.25, PI * 1.25, PI * 2.25));
                painter.add(Shape::closed_line(pts, st));
            }
            UiIcon::ArrowRight => {
                line(&[(5.0, 12.0), (19.0, 12.0)]);
                line(&[(12.0, 5.0), (19.0, 12.0), (12.0, 19.0)]);
            }
            UiIcon::ArrowLeft => {
                line(&[(19.0, 12.0), (5.0, 12.0)]);
                line(&[(12.0, 5.0), (5.0, 12.0), (12.0, 19.0)]);
            }
            UiIcon::ArrowUp => {
                line(&[(12.0, 19.0), (12.0, 5.0)]);
                line(&[(5.0, 12.0), (12.0, 5.0), (19.0, 12.0)]);
            }
            UiIcon::ArrowDown => {
                line(&[(12.0, 5.0), (12.0, 19.0)]);
                line(&[(5.0, 12.0), (12.0, 19.0), (19.0, 12.0)]);
            }
            UiIcon::ArrowUpDown => {
                line(&[(8.0, 4.0), (8.0, 20.0)]);
                line(&[(4.0, 8.0), (8.0, 4.0), (12.0, 8.0)]);
                line(&[(16.0, 20.0), (16.0, 4.0)]);
                line(&[(12.0, 16.0), (16.0, 20.0), (20.0, 16.0)]);
            }
            UiIcon::Bold => {
                let mut a = vec![p(6.5, 12.0), p(6.5, 4.0), p(13.5, 4.0)];
                a.extend(arc(13.5, 8.0, 4.0, -PI / 2.0, PI / 2.0));
                a.push(p(6.5, 12.0));
                a.push(p(14.5, 12.0));
                a.extend(arc(14.5, 16.0, 4.0, -PI / 2.0, PI / 2.0));
                a.push(p(6.5, 20.0));
                a.push(p(6.5, 12.0));
                painter.add(Shape::line(a, Stroke::new(w * 1.25, color)));
            }
            UiIcon::Italic => {
                line(&[(19.0, 4.0), (10.0, 4.0)]);
                line(&[(14.0, 20.0), (5.0, 20.0)]);
                line(&[(15.0, 4.0), (9.0, 20.0)]);
            }
            UiIcon::Underline => {
                let mut a = vec![p(6.0, 4.0)];
                a.extend(arc(12.0, 10.0, 6.0, PI, 0.0));
                a.push(p(18.0, 4.0));
                painter.add(Shape::line(a, st));
                line(&[(4.0, 21.0), (20.0, 21.0)]);
            }
            UiIcon::AlignLeft => {
                line(&[(3.0, 6.0), (21.0, 6.0)]);
                line(&[(3.0, 12.0), (15.0, 12.0)]);
                line(&[(3.0, 18.0), (18.0, 18.0)]);
            }
            UiIcon::AlignCenter => {
                line(&[(3.0, 6.0), (21.0, 6.0)]);
                line(&[(7.0, 12.0), (17.0, 12.0)]);
                line(&[(5.0, 18.0), (19.0, 18.0)]);
            }
            UiIcon::AlignRight => {
                line(&[(3.0, 6.0), (21.0, 6.0)]);
                line(&[(9.0, 12.0), (21.0, 12.0)]);
                line(&[(6.0, 18.0), (21.0, 18.0)]);
            }
            UiIcon::File => {
                closed(&[
                    (14.0, 2.0),
                    (6.0, 2.0),
                    (4.5, 3.5),
                    (4.5, 20.5),
                    (6.0, 22.0),
                    (18.0, 22.0),
                    (19.5, 20.5),
                    (19.5, 7.5),
                ]);
                line(&[(14.0, 2.0), (14.0, 7.5), (19.5, 7.5)]);
            }
            UiIcon::Folder => {
                closed(&[
                    (2.5, 5.5),
                    (3.5, 4.0),
                    (9.5, 4.0),
                    (11.5, 7.0),
                    (20.5, 7.0),
                    (21.5, 8.0),
                    (21.5, 19.0),
                    (20.5, 20.0),
                    (3.5, 20.0),
                    (2.5, 19.0),
                ]);
            }
            UiIcon::FolderOpen => {
                line(&[
                    (5.0, 20.0),
                    (3.5, 20.0),
                    (2.5, 19.0),
                    (2.5, 5.0),
                    (3.5, 4.0),
                    (9.5, 4.0),
                    (11.5, 7.0),
                    (18.0, 7.0),
                    (19.0, 8.0),
                    (19.0, 10.0),
                ]);
                closed(&[(5.0, 10.0), (22.0, 10.0), (19.0, 20.0), (2.5, 20.0)]);
            }
            UiIcon::Home => {
                closed(&[(3.0, 10.0), (12.0, 3.0), (21.0, 10.0), (21.0, 21.0), (3.0, 21.0)]);
                line(&[(9.0, 21.0), (9.0, 14.0), (15.0, 14.0), (15.0, 21.0)]);
            }
            UiIcon::Lock => {
                rrect(4.0, 11.0, 20.0, 21.5, 2.0);
                let mut a = vec![p(7.5, 11.0)];
                a.extend(arc(12.0, 7.5, 4.5, PI, TAU));
                a.push(p(16.5, 11.0));
                painter.add(Shape::line(a, st));
            }
            UiIcon::Image => {
                rrect(3.0, 3.0, 21.0, 21.0, 2.5);
                circle(9.0, 9.0, 2.0);
                line(&[(21.0, 15.0), (16.0, 10.0), (5.0, 21.0)]);
            }
            UiIcon::MoreHorizontal => {
                dot(5.0, 12.0, 1.7);
                dot(12.0, 12.0, 1.7);
                dot(19.0, 12.0, 1.7);
            }
            UiIcon::Filter => {
                closed(&[(3.0, 4.0), (21.0, 4.0), (14.0, 12.5), (14.0, 20.0), (10.0, 18.0), (10.0, 12.5)])
            }
            UiIcon::Clock => {
                circle(12.0, 12.0, 10.0);
                line(&[(12.0, 6.0), (12.0, 12.0), (16.0, 14.0)]);
            }
            UiIcon::Code => {
                line(&[(16.0, 18.0), (22.0, 12.0), (16.0, 6.0)]);
                line(&[(8.0, 6.0), (2.0, 12.0), (8.0, 18.0)]);
            }
            UiIcon::Sparkles => {
                glowkit_core::paint::sparkle(painter, p(10.0, 13.0), 8.0 * k, 0.0, color);
                glowkit_core::paint::sparkle(painter, p(18.5, 5.5), 4.0 * k, 0.0, color);
                glowkit_core::paint::sparkle(painter, p(19.0, 18.5), 2.8 * k, 0.0, color);
            }
            UiIcon::Bell => {
                let mut a = vec![p(3.5, 17.0)];
                a.push(p(5.5, 14.5));
                a.extend(arc(12.0, 9.0, 6.5, PI, TAU).into_iter().map(|q| q + vec2(0.0, 1.0 * k)));
                a.push(p(18.5, 14.5));
                a.push(p(20.5, 17.0));
                painter.add(Shape::closed_line(a, st));
                painter.add(Shape::line(arc(12.0, 19.5, 2.2, 0.0, PI), st));
            }
            UiIcon::Inbox => {
                closed(&[(5.5, 4.0), (18.5, 4.0), (22.0, 12.0), (22.0, 20.0), (2.0, 20.0), (2.0, 12.0)]);
                line(&[(2.0, 12.0), (8.0, 12.0), (10.0, 15.0), (14.0, 15.0), (16.0, 12.0), (22.0, 12.0)]);
            }
            UiIcon::Globe => {
                circle(12.0, 12.0, 10.0);
                line(&[(2.0, 12.0), (22.0, 12.0)]);
                let mut a = arc(12.0, 12.0, 10.0, -PI / 2.0, PI / 2.0);
                for q in a.iter_mut() {
                    q.x = p(12.0, 0.0).x + (q.x - p(12.0, 0.0).x) * 0.42;
                }
                let b: Vec<Pos2> = a.iter().map(|q| pos2(2.0 * p(12.0, 0.0).x - q.x, q.y)).collect();
                painter.add(Shape::line(a, st));
                painter.add(Shape::line(b, st));
            }
            UiIcon::Link => {
                let r = Rect::from_center_size(p(8.5, 15.5), vec2(11.0 * k, 6.5 * k));
                let rot = |q: Pos2, c: Pos2| {
                    let d = q - c;
                    let (s, co) = (-PI / 4.0).sin_cos();
                    c + vec2(d.x * co - d.y * s, d.x * s + d.y * co)
                };
                for c in [p(8.8, 15.2), p(15.2, 8.8)] {
                    let rr = Rect::from_center_size(c, r.size());
                    let pts: Vec<Pos2> = glowkit_core::paint::rounded_rect_points(rr, 3.25 * k, 6)
                        .into_iter()
                        .map(|q| rot(q, c))
                        .collect();
                    painter.add(Shape::closed_line(pts, st));
                }
            }
            UiIcon::Grid => {
                rrect(3.0, 3.0, 10.5, 10.5, 1.5);
                rrect(13.5, 3.0, 21.0, 10.5, 1.5);
                rrect(3.0, 13.5, 10.5, 21.0, 1.5);
                rrect(13.5, 13.5, 21.0, 21.0, 1.5);
            }
            UiIcon::List => {
                for y in [6.0, 12.0, 18.0] {
                    dot(4.0, y, 1.2);
                    line(&[(8.0, y), (21.0, y)]);
                }
            }
            UiIcon::Zap => closed(&[(13.0, 2.0), (4.0, 14.0), (12.0, 14.0), (11.0, 22.0), (20.0, 10.0), (12.0, 10.0)]),
            UiIcon::Dollar => {
                line(&[(12.0, 2.0), (12.0, 22.0)]);
                let mut a = vec![p(17.0, 6.0), p(10.0, 6.0)];
                a.extend(arc(10.0, 9.0, 3.0, -PI / 2.0, -PI * 1.5));
                a.push(p(14.0, 12.0));
                a.extend(arc(14.0, 15.0, 3.0, -PI / 2.0, PI / 2.0));
                a.push(p(6.0, 18.0));
                painter.add(Shape::line(a, st));
            }
            UiIcon::Activity => line(&[(22.0, 12.0), (18.0, 12.0), (15.0, 21.0), (9.0, 3.0), (6.0, 12.0), (2.0, 12.0)]),
            UiIcon::Terminal => {
                line(&[(4.0, 17.0), (10.0, 11.0), (4.0, 5.0)]);
                line(&[(12.0, 19.0), (20.0, 19.0)]);
            }
        }
    }

    /// Allocate a `size`-point square and paint the icon there.
    pub fn show(self, ui: &mut Ui, size: f32, color: Color32) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, color);
        }
        resp
    }
}
