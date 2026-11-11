//! Crisp vector line icons (painted, no font needed) shared by navigation and overlay components.

use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke};
use std::f32::consts::{PI, TAU};

/// A small set of painted line icons in a clean 24-point-grid style.
///
/// Icons are painted with strokes, so they stay sharp at any size and pick up any color.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::LineIcon;
/// let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(16.0, 16.0));
/// LineIcon::Home.paint(ui.painter(), rect, ui.visuals().text_color());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum LineIcon {
    Home,
    Search,
    Settings,
    User,
    Users,
    Bell,
    Folder,
    File,
    Inbox,
    Calendar,
    Chart,
    Mail,
    Star,
    Heart,
    Plus,
    Minus,
    Check,
    X,
    ChevronRight,
    ChevronLeft,
    ChevronUp,
    ChevronDown,
    ArrowRight,
    ArrowLeft,
    Info,
    Warning,
    CircleCheck,
    CircleX,
    Menu,
    LogOut,
    Copy,
    Trash,
    Edit,
    Moon,
    Sun,
    Command,
    Dots,
    Layers,
    Sparkles,
    Upload,
    Download,
    Globe,
    CreditCard,
    Grid,
    Zap,
    Help,
    Book,
    Music,
    Phone,
    Lock,
    Share,
    Sidebar,
}

impl LineIcon {
    /// Paint the icon centered in `rect` (uses the smaller side) with a default stroke width.
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        let s = rect.width().min(rect.height());
        self.paint_with(painter, rect, color, (s / 24.0 * 1.8).max(1.1));
    }

    /// Paint the icon with an explicit stroke width (in points).
    pub fn paint_with(self, painter: &Painter, rect: Rect, color: Color32, width: f32) {
        let s = rect.width().min(rect.height());
        let o = rect.center() - vec2(s, s) / 2.0;
        let k = s / 24.0;
        let p = |x: f32, y: f32| pos2(o.x + x * k, o.y + y * k);
        let st = Stroke::new(width, color);
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
        let arc = |x: f32, y: f32, r: f32, a0: f32, a1: f32| {
            let n = 24;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = a0 + (a1 - a0) * i as f32 / n as f32;
                    p(x + a.cos() * r, y + a.sin() * r)
                })
                .collect();
            painter.add(Shape::line(pts, st));
        };
        let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
            painter.rect_stroke(
                Rect::from_min_max(p(x0, y0), p(x1, y1)),
                r * k,
                st,
                egui::StrokeKind::Middle,
            );
        };
        match self {
            LineIcon::Home => {
                line(&[(3.0, 10.5), (12.0, 3.0), (21.0, 10.5)]);
                line(&[(5.5, 9.0), (5.5, 21.0), (18.5, 21.0), (18.5, 9.0)]);
                line(&[(10.0, 21.0), (10.0, 15.0), (14.0, 15.0), (14.0, 21.0)]);
            }
            LineIcon::Search => {
                circle(11.0, 11.0, 7.0);
                line(&[(16.2, 16.2), (21.0, 21.0)]);
            }
            LineIcon::Settings => {
                circle(12.0, 12.0, 3.0);
                circle(12.0, 12.0, 7.5);
                for i in 0..8 {
                    let a = i as f32 / 8.0 * TAU;
                    line(&[(12.0 + a.cos() * 7.5, 12.0 + a.sin() * 7.5), (12.0 + a.cos() * 10.0, 12.0 + a.sin() * 10.0)]);
                }
            }
            LineIcon::User => {
                circle(12.0, 8.0, 4.0);
                arc(12.0, 21.0, 7.5, PI, TAU);
            }
            LineIcon::Users => {
                circle(9.0, 8.0, 3.5);
                arc(9.0, 20.5, 6.5, PI, TAU);
                arc(16.5, 8.0, 3.0, -PI / 2.0, PI / 2.0);
                arc(17.5, 20.5, 4.5, -PI / 2.0, 0.0);
            }
            LineIcon::Bell => {
                arc(12.0, 10.0, 6.0, PI, TAU);
                line(&[(6.0, 10.0), (6.0, 15.0), (4.0, 17.5), (20.0, 17.5), (18.0, 15.0), (18.0, 10.0)]);
                arc(12.0, 19.5, 2.0, 0.0, PI);
            }
            LineIcon::Folder => {
                closed(&[(3.0, 6.0), (9.0, 6.0), (11.0, 8.5), (21.0, 8.5), (21.0, 19.0), (3.0, 19.0)]);
            }
            LineIcon::File => {
                closed(&[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)]);
                line(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
            }
            LineIcon::Inbox => {
                rrect(3.0, 5.0, 21.0, 19.0, 2.0);
                line(&[(3.0, 13.0), (8.0, 13.0), (9.5, 16.0), (14.5, 16.0), (16.0, 13.0), (21.0, 13.0)]);
            }
            LineIcon::Calendar => {
                rrect(3.5, 5.0, 20.5, 21.0, 2.0);
                line(&[(3.5, 10.0), (20.5, 10.0)]);
                line(&[(8.0, 3.0), (8.0, 7.0)]);
                line(&[(16.0, 3.0), (16.0, 7.0)]);
            }
            LineIcon::Chart => {
                line(&[(3.0, 3.0), (3.0, 21.0), (21.0, 21.0)]);
                line(&[(8.0, 17.0), (8.0, 13.0)]);
                line(&[(13.0, 17.0), (13.0, 8.0)]);
                line(&[(18.0, 17.0), (18.0, 5.0)]);
            }
            LineIcon::Mail => {
                rrect(3.0, 5.0, 21.0, 19.0, 2.0);
                line(&[(3.5, 6.5), (12.0, 13.0), (20.5, 6.5)]);
            }
            LineIcon::Star => {
                let pts: Vec<(f32, f32)> = (0..10)
                    .map(|i| {
                        let a = i as f32 / 10.0 * TAU - PI / 2.0;
                        let r = if i % 2 == 0 { 9.5 } else { 4.2 };
                        (12.0 + a.cos() * r, 12.8 + a.sin() * r)
                    })
                    .collect();
                closed(&pts);
            }
            LineIcon::Heart => {
                let mut pts = Vec::new();
                for i in 0..=40 {
                    let t = i as f32 / 40.0 * TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    pts.push((12.0 + x * 0.56, 11.5 - y * 0.56));
                }
                closed(&pts);
            }
            LineIcon::Plus => {
                line(&[(12.0, 5.0), (12.0, 19.0)]);
                line(&[(5.0, 12.0), (19.0, 12.0)]);
            }
            LineIcon::Minus => line(&[(5.0, 12.0), (19.0, 12.0)]),
            LineIcon::Check => line(&[(5.0, 12.5), (10.0, 17.5), (19.5, 7.0)]),
            LineIcon::X => {
                line(&[(6.0, 6.0), (18.0, 18.0)]);
                line(&[(18.0, 6.0), (6.0, 18.0)]);
            }
            LineIcon::ChevronRight => line(&[(9.0, 5.5), (15.5, 12.0), (9.0, 18.5)]),
            LineIcon::ChevronLeft => line(&[(15.0, 5.5), (8.5, 12.0), (15.0, 18.5)]),
            LineIcon::ChevronDown => line(&[(5.5, 9.0), (12.0, 15.5), (18.5, 9.0)]),
            LineIcon::ChevronUp => line(&[(5.5, 15.0), (12.0, 8.5), (18.5, 15.0)]),
            LineIcon::ArrowRight => {
                line(&[(4.0, 12.0), (20.0, 12.0)]);
                line(&[(13.5, 5.5), (20.0, 12.0), (13.5, 18.5)]);
            }
            LineIcon::ArrowLeft => {
                line(&[(4.0, 12.0), (20.0, 12.0)]);
                line(&[(10.5, 5.5), (4.0, 12.0), (10.5, 18.5)]);
            }
            LineIcon::Info => {
                circle(12.0, 12.0, 9.5);
                line(&[(12.0, 11.0), (12.0, 16.5)]);
                dot(12.0, 7.8, 1.2);
            }
            LineIcon::Help => {
                circle(12.0, 12.0, 9.5);
                arc(12.0, 9.5, 2.8, PI, TAU + 0.6);
                line(&[(14.2, 11.2), (12.0, 13.0), (12.0, 14.0)]);
                dot(12.0, 17.0, 1.2);
            }
            LineIcon::Warning => {
                closed(&[(12.0, 3.0), (22.0, 20.5), (2.0, 20.5)]);
                line(&[(12.0, 9.5), (12.0, 14.0)]);
                dot(12.0, 17.2, 1.2);
            }
            LineIcon::CircleCheck => {
                circle(12.0, 12.0, 9.5);
                line(&[(8.0, 12.3), (10.8, 15.0), (16.0, 9.5)]);
            }
            LineIcon::CircleX => {
                circle(12.0, 12.0, 9.5);
                line(&[(9.0, 9.0), (15.0, 15.0)]);
                line(&[(15.0, 9.0), (9.0, 15.0)]);
            }
            LineIcon::Menu => {
                line(&[(4.0, 6.5), (20.0, 6.5)]);
                line(&[(4.0, 12.0), (20.0, 12.0)]);
                line(&[(4.0, 17.5), (20.0, 17.5)]);
            }
            LineIcon::LogOut => {
                line(&[(9.0, 4.0), (5.0, 4.0), (5.0, 20.0), (9.0, 20.0)]);
                line(&[(10.0, 12.0), (20.0, 12.0)]);
                line(&[(16.0, 8.0), (20.0, 12.0), (16.0, 16.0)]);
            }
            LineIcon::Copy => {
                rrect(9.0, 9.0, 20.0, 20.0, 2.0);
                line(&[(5.0, 15.0), (4.0, 15.0), (4.0, 4.0), (15.0, 4.0), (15.0, 5.0)]);
            }
            LineIcon::Trash => {
                line(&[(3.5, 6.5), (20.5, 6.5)]);
                line(&[(5.5, 6.5), (6.5, 21.0), (17.5, 21.0), (18.5, 6.5)]);
                line(&[(9.0, 6.5), (9.0, 3.5), (15.0, 3.5), (15.0, 6.5)]);
                line(&[(10.0, 11.0), (10.0, 17.0)]);
                line(&[(14.0, 11.0), (14.0, 17.0)]);
            }
            LineIcon::Edit => {
                closed(&[(4.0, 20.0), (4.5, 16.0), (16.0, 4.5), (19.5, 8.0), (8.0, 19.5)]);
                line(&[(13.5, 7.0), (17.0, 10.5)]);
            }
            LineIcon::Moon => {
                arc(12.0, 12.0, 8.5, 0.35 * PI, 1.95 * PI);
                arc(16.5, 8.5, 6.0, 0.62 * PI, 1.32 * PI);
            }
            LineIcon::Sun => {
                circle(12.0, 12.0, 4.0);
                for i in 0..8 {
                    let a = i as f32 / 8.0 * TAU;
                    line(&[(12.0 + a.cos() * 7.0, 12.0 + a.sin() * 7.0), (12.0 + a.cos() * 9.5, 12.0 + a.sin() * 9.5)]);
                }
            }
            LineIcon::Command => {
                line(&[(9.0, 6.5), (9.0, 17.5)]);
                line(&[(15.0, 6.5), (15.0, 17.5)]);
                line(&[(6.5, 9.0), (17.5, 9.0)]);
                line(&[(6.5, 15.0), (17.5, 15.0)]);
                arc(6.5, 6.5, 2.5, 0.0, -1.5 * PI);
                arc(17.5, 6.5, 2.5, PI / 2.0, -PI);
                arc(6.5, 17.5, 2.5, -PI / 2.0, -2.0 * PI);
                arc(17.5, 17.5, 2.5, PI, -PI / 2.0);
            }
            LineIcon::Dots => {
                dot(5.5, 12.0, 1.6);
                dot(12.0, 12.0, 1.6);
                dot(18.5, 12.0, 1.6);
            }
            LineIcon::Layers => {
                closed(&[(12.0, 3.0), (21.0, 8.0), (12.0, 13.0), (3.0, 8.0)]);
                line(&[(3.0, 12.5), (12.0, 17.5), (21.0, 12.5)]);
                line(&[(3.0, 16.5), (12.0, 21.5), (21.0, 16.5)]);
            }
            LineIcon::Sparkles => {
                glowkit_core::paint::sparkle(painter, p(10.0, 13.0), 8.0 * k, 0.0, color);
                glowkit_core::paint::sparkle(painter, p(18.5, 5.5), 3.8 * k, 0.0, color);
            }
            LineIcon::Upload => {
                line(&[(4.0, 15.0), (4.0, 20.0), (20.0, 20.0), (20.0, 15.0)]);
                line(&[(12.0, 15.5), (12.0, 4.0)]);
                line(&[(7.5, 8.5), (12.0, 4.0), (16.5, 8.5)]);
            }
            LineIcon::Download => {
                line(&[(4.0, 15.0), (4.0, 20.0), (20.0, 20.0), (20.0, 15.0)]);
                line(&[(12.0, 4.0), (12.0, 15.5)]);
                line(&[(7.5, 11.0), (12.0, 15.5), (16.5, 11.0)]);
            }
            LineIcon::Globe => {
                circle(12.0, 12.0, 9.5);
                line(&[(2.5, 12.0), (21.5, 12.0)]);
                let pts: Vec<(f32, f32)> = (0..=24)
                    .map(|i| {
                        let a = i as f32 / 24.0 * TAU;
                        (12.0 + a.cos() * 4.2, 12.0 + a.sin() * 9.5)
                    })
                    .collect();
                closed(&pts);
            }
            LineIcon::CreditCard => {
                rrect(2.5, 5.0, 21.5, 19.0, 2.0);
                line(&[(2.5, 10.0), (21.5, 10.0)]);
                line(&[(6.0, 15.0), (10.0, 15.0)]);
            }
            LineIcon::Grid => {
                rrect(3.5, 3.5, 10.5, 10.5, 1.5);
                rrect(13.5, 3.5, 20.5, 10.5, 1.5);
                rrect(3.5, 13.5, 10.5, 20.5, 1.5);
                rrect(13.5, 13.5, 20.5, 20.5, 1.5);
            }
            LineIcon::Zap => {
                closed(&[(13.0, 2.5), (4.5, 13.5), (11.5, 13.5), (10.5, 21.5), (19.5, 10.5), (12.5, 10.5)]);
            }
            LineIcon::Book => {
                line(&[(4.0, 19.0), (4.0, 5.0), (7.0, 3.0), (20.0, 3.0), (20.0, 17.0), (7.0, 17.0), (4.0, 19.0), (7.0, 21.0), (20.0, 21.0)]);
            }
            LineIcon::Music => {
                line(&[(9.0, 18.0), (9.0, 5.0), (20.0, 3.0), (20.0, 16.0)]);
                circle(6.5, 18.0, 2.6);
                circle(17.5, 16.0, 2.6);
            }
            LineIcon::Phone => {
                rrect(6.5, 2.5, 17.5, 21.5, 2.5);
                line(&[(11.0, 18.5), (13.0, 18.5)]);
            }
            LineIcon::Lock => {
                rrect(4.5, 11.0, 19.5, 21.0, 2.0);
                arc(12.0, 11.0, 4.5, PI, TAU);
                line(&[(7.5, 11.0), (7.5, 10.0)]);
            }
            LineIcon::Share => {
                circle(18.0, 5.5, 2.6);
                circle(6.0, 12.0, 2.6);
                circle(18.0, 18.5, 2.6);
                line(&[(8.3, 10.8), (15.7, 6.7)]);
                line(&[(8.3, 13.2), (15.7, 17.3)]);
            }
            LineIcon::Sidebar => {
                rrect(3.0, 4.0, 21.0, 20.0, 2.0);
                line(&[(9.0, 4.0), (9.0, 20.0)]);
            }
        }
    }
}
