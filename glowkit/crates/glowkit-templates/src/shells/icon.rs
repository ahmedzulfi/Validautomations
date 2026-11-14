//! [`AppIcon`]: one icon type for navigation and app chrome, covering the component crate's
//! [`LineIcon`] and [`UiIcon`] sets plus the extra glyphs dashboards need (cart, wallet, play...).

use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};
use glowkit_components::{LineIcon, UiIcon};
use std::f32::consts::{PI, TAU};

/// A painted line icon for shells and templates.
///
/// Wraps [`LineIcon`] and [`UiIcon`] (convert with `.into()`) and adds dashboard glyphs. Icons
/// are stroked on a 24-point grid so they stay sharp at any size and take any color.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_templates::AppIcon;
/// let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(16.0, 16.0));
/// AppIcon::Cart.paint(ui.painter(), rect, ui.visuals().text_color());
/// AppIcon::from(glowkit_components::LineIcon::Home).paint(ui.painter(), rect, egui::Color32::WHITE);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum AppIcon {
    /// Any icon from the component crate's navigation set.
    Line(LineIcon),
    /// Any icon from the component crate's form set.
    Ui(UiIcon),
    Package,
    Cart,
    Wallet,
    Receipt,
    Kanban,
    Key,
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Hash,
    Send,
    Archive,
    Bot,
    Disc,
    Mic,
    Volume,
    Server,
    Tag,
    TrendUp,
    TrendDown,
    Paperclip,
    Reply,
    ReplyAll,
    Forward,
    Flag,
    Bookmark,
    Database,
    Shield,
    Palette,
    Building,
    PieChart,
    Target,
    Rocket,
    Bug,
    ListChecks,
    Library,
    Radio,
    Smile,
    AtSign,
    Refresh,
    External,
}

impl From<LineIcon> for AppIcon {
    fn from(i: LineIcon) -> Self {
        AppIcon::Line(i)
    }
}

impl From<UiIcon> for AppIcon {
    fn from(i: UiIcon) -> Self {
        AppIcon::Ui(i)
    }
}

impl AppIcon {
    /// Paint the icon centered in `rect` (uses the smaller side).
    pub fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        match self {
            AppIcon::Line(i) => i.paint(painter, rect, color),
            AppIcon::Ui(i) => i.paint(painter, rect, color),
            _ => self.paint_extra(painter, rect, color),
        }
    }

    fn paint_extra(self, painter: &Painter, rect: Rect, color: Color32) {
        let s = rect.width().min(rect.height());
        let o = rect.center() - vec2(s, s) / 2.0;
        let k = s / 24.0;
        let p = |x: f32, y: f32| pos2(o.x + x * k, o.y + y * k);
        let st = Stroke::new((s / 24.0 * 1.8).max(1.1), color);
        let line = |pts: &[(f32, f32)]| {
            painter.add(Shape::line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
        };
        let closed = |pts: &[(f32, f32)]| {
            painter.add(Shape::closed_line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
        };
        let fill = |pts: &[(f32, f32)]| {
            painter.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| p(x, y)).collect(), color, Stroke::NONE));
        };
        let circle = |x: f32, y: f32, r: f32| {
            painter.circle_stroke(p(x, y), r * k, st);
        };
        let dot = |x: f32, y: f32, r: f32| {
            painter.circle_filled(p(x, y), r * k, color);
        };
        let arc = |x: f32, y: f32, r: f32, a0: f32, a1: f32| {
            let n = 20;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = a0 + (a1 - a0) * i as f32 / n as f32;
                    p(x + a.cos() * r, y + a.sin() * r)
                })
                .collect();
            painter.add(Shape::line(pts, st));
        };
        let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
            painter.rect_stroke(Rect::from_min_max(p(x0, y0), p(x1, y1)), r * k, st, StrokeKind::Middle);
        };
        match self {
            AppIcon::Line(_) | AppIcon::Ui(_) => {}
            AppIcon::Package => {
                closed(&[(12.0, 2.5), (20.5, 7.0), (20.5, 17.0), (12.0, 21.5), (3.5, 17.0), (3.5, 7.0)]);
                line(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
                line(&[(12.0, 11.5), (12.0, 21.5)]);
                line(&[(7.8, 4.8), (16.2, 9.3)]);
            }
            AppIcon::Cart => {
                line(&[(2.0, 3.0), (5.0, 3.0), (7.5, 15.0), (18.5, 15.0), (20.5, 7.0), (6.0, 7.0)]);
                dot(9.0, 19.5, 1.6);
                dot(17.0, 19.5, 1.6);
            }
            AppIcon::Wallet => {
                rrect(3.0, 6.0, 21.0, 20.0, 2.5);
                line(&[(5.0, 6.0), (16.0, 3.0), (17.0, 6.0)]);
                rrect(15.0, 11.0, 21.0, 15.0, 1.0);
                dot(17.5, 13.0, 0.9);
            }
            AppIcon::Receipt => {
                line(&[(5.0, 21.0), (5.0, 3.0), (19.0, 3.0), (19.0, 21.0), (16.5, 19.5), (14.0, 21.0), (12.0, 19.5), (10.0, 21.0), (7.5, 19.5), (5.0, 21.0)]);
                line(&[(8.5, 8.0), (15.5, 8.0)]);
                line(&[(8.5, 12.0), (15.5, 12.0)]);
                line(&[(8.5, 16.0), (12.5, 16.0)]);
            }
            AppIcon::Kanban => {
                rrect(3.0, 3.0, 21.0, 21.0, 2.5);
                line(&[(8.0, 7.0), (8.0, 15.0)]);
                line(&[(12.0, 7.0), (12.0, 11.0)]);
                line(&[(16.0, 7.0), (16.0, 17.0)]);
            }
            AppIcon::Key => {
                circle(7.5, 15.5, 4.5);
                line(&[(10.7, 12.3), (20.0, 3.0)]);
                line(&[(16.0, 7.0), (19.0, 10.0)]);
                line(&[(18.5, 4.5), (21.0, 7.0)]);
            }
            AppIcon::Play => {
                fill(&[(7.0, 4.0), (19.5, 12.0), (7.0, 20.0)]);
            }
            AppIcon::Pause => {
                fill(&[(6.0, 4.0), (10.0, 4.0), (10.0, 20.0), (6.0, 20.0)]);
                fill(&[(14.0, 4.0), (18.0, 4.0), (18.0, 20.0), (14.0, 20.0)]);
            }
            AppIcon::SkipBack => {
                fill(&[(19.0, 5.0), (19.0, 19.0), (8.0, 12.0)]);
                line(&[(5.5, 5.0), (5.5, 19.0)]);
            }
            AppIcon::SkipForward => {
                fill(&[(5.0, 5.0), (16.0, 12.0), (5.0, 19.0)]);
                line(&[(18.5, 5.0), (18.5, 19.0)]);
            }
            AppIcon::Shuffle => {
                line(&[(3.0, 7.0), (7.0, 7.0), (15.0, 17.0), (20.0, 17.0)]);
                line(&[(3.0, 17.0), (7.0, 17.0), (15.0, 7.0), (20.0, 7.0)]);
                line(&[(17.5, 4.5), (20.0, 7.0), (17.5, 9.5)]);
                line(&[(17.5, 14.5), (20.0, 17.0), (17.5, 19.5)]);
            }
            AppIcon::Repeat => {
                line(&[(4.0, 11.0), (4.0, 9.0), (6.0, 7.0), (20.0, 7.0)]);
                line(&[(17.0, 4.0), (20.0, 7.0), (17.0, 10.0)]);
                line(&[(20.0, 13.0), (20.0, 15.0), (18.0, 17.0), (4.0, 17.0)]);
                line(&[(7.0, 14.0), (4.0, 17.0), (7.0, 20.0)]);
            }
            AppIcon::Hash => {
                line(&[(9.5, 3.5), (7.5, 20.5)]);
                line(&[(16.5, 3.5), (14.5, 20.5)]);
                line(&[(4.0, 9.0), (20.0, 9.0)]);
                line(&[(4.0, 15.0), (20.0, 15.0)]);
            }
            AppIcon::Send => {
                closed(&[(3.0, 11.0), (21.0, 3.0), (13.0, 21.0), (11.0, 13.0)]);
                line(&[(11.0, 13.0), (21.0, 3.0)]);
            }
            AppIcon::Archive => {
                rrect(3.0, 4.0, 21.0, 8.5, 1.0);
                line(&[(4.5, 8.5), (4.5, 20.0), (19.5, 20.0), (19.5, 8.5)]);
                line(&[(10.0, 12.5), (14.0, 12.5)]);
            }
            AppIcon::Bot => {
                rrect(4.0, 8.0, 20.0, 20.0, 3.0);
                line(&[(12.0, 8.0), (12.0, 4.5)]);
                dot(12.0, 3.8, 1.3);
                dot(9.0, 13.5, 1.4);
                dot(15.0, 13.5, 1.4);
                line(&[(2.0, 13.0), (2.0, 16.0)]);
                line(&[(22.0, 13.0), (22.0, 16.0)]);
            }
            AppIcon::Disc => {
                circle(12.0, 12.0, 9.0);
                circle(12.0, 12.0, 2.5);
            }
            AppIcon::Mic => {
                rrect(9.0, 3.0, 15.0, 14.0, 3.0);
                arc(12.0, 11.0, 6.5, 0.0, PI);
                line(&[(12.0, 17.5), (12.0, 21.0)]);
            }
            AppIcon::Volume => {
                closed(&[(4.0, 9.0), (8.0, 9.0), (13.0, 4.5), (13.0, 19.5), (8.0, 15.0), (4.0, 15.0)]);
                arc(13.0, 12.0, 4.0, -PI / 3.5, PI / 3.5);
                arc(13.0, 12.0, 7.5, -PI / 3.2, PI / 3.2);
            }
            AppIcon::Server => {
                rrect(3.5, 3.5, 20.5, 10.5, 2.0);
                rrect(3.5, 13.5, 20.5, 20.5, 2.0);
                dot(7.0, 7.0, 1.0);
                dot(7.0, 17.0, 1.0);
            }
            AppIcon::Tag => {
                closed(&[(3.0, 3.0), (12.0, 3.0), (21.0, 12.0), (12.0, 21.0), (3.0, 12.0)]);
                dot(8.0, 8.0, 1.4);
            }
            AppIcon::TrendUp => {
                line(&[(2.5, 18.0), (9.0, 11.5), (13.0, 15.5), (21.0, 7.0)]);
                line(&[(15.0, 7.0), (21.0, 7.0), (21.0, 13.0)]);
            }
            AppIcon::TrendDown => {
                line(&[(2.5, 6.0), (9.0, 12.5), (13.0, 8.5), (21.0, 17.0)]);
                line(&[(15.0, 17.0), (21.0, 17.0), (21.0, 11.0)]);
            }
            AppIcon::Paperclip => {
                line(&[(18.0, 11.0), (11.0, 18.0)]);
                arc(8.0, 15.0, 4.2, PI * 0.75, PI * 1.75);
                line(&[(5.0, 12.0), (13.0, 4.0)]);
                arc(15.5, 6.5, 3.5, PI * 1.25, PI * 2.25);
                line(&[(18.0, 9.0), (10.0, 17.0)]);
            }
            AppIcon::Reply => {
                line(&[(9.0, 5.0), (3.0, 11.0), (9.0, 17.0)]);
                line(&[(3.0, 11.0), (14.0, 11.0)]);
                arc(14.0, 17.0, 6.0, -PI / 2.0, 0.0);
                line(&[(20.0, 17.0), (20.0, 20.0)]);
            }
            AppIcon::ReplyAll => {
                line(&[(11.0, 6.0), (6.0, 11.0), (11.0, 16.0)]);
                line(&[(7.0, 5.0), (2.0, 10.0), (7.0, 15.0)]);
                line(&[(6.0, 11.0), (15.0, 11.0)]);
                arc(15.0, 17.0, 6.0, -PI / 2.0, 0.0);
                line(&[(21.0, 17.0), (21.0, 20.0)]);
            }
            AppIcon::Forward => {
                line(&[(15.0, 5.0), (21.0, 11.0), (15.0, 17.0)]);
                line(&[(21.0, 11.0), (10.0, 11.0)]);
                arc(10.0, 17.0, 6.0, PI, PI * 1.5);
                line(&[(4.0, 17.0), (4.0, 20.0)]);
            }
            AppIcon::Flag => {
                line(&[(5.0, 21.0), (5.0, 3.5)]);
                closed(&[(5.0, 4.0), (19.0, 4.0), (16.0, 8.5), (19.0, 13.0), (5.0, 13.0)]);
            }
            AppIcon::Bookmark => {
                closed(&[(6.0, 3.0), (18.0, 3.0), (18.0, 21.0), (12.0, 16.5), (6.0, 21.0)]);
            }
            AppIcon::Database => {
                arc(12.0, 6.0, 8.0, 0.0, TAU);
                line(&[(4.0, 6.0), (4.0, 18.0)]);
                line(&[(20.0, 6.0), (20.0, 18.0)]);
                arc(12.0, 12.0, 8.0, 0.0, PI);
                arc(12.0, 18.0, 8.0, 0.0, PI);
            }
            AppIcon::Shield => {
                closed(&[(12.0, 2.5), (20.0, 5.5), (20.0, 12.0), (17.0, 17.5), (12.0, 21.5), (7.0, 17.5), (4.0, 12.0), (4.0, 5.5)]);
            }
            AppIcon::Palette => {
                arc(12.0, 12.0, 9.0, PI * 0.35, PI * 2.25);
                line(&[(15.2, 20.4), (14.0, 17.0), (17.0, 15.0), (20.5, 14.5)]);
                dot(7.5, 11.0, 1.3);
                dot(10.0, 7.0, 1.3);
                dot(15.0, 7.5, 1.3);
            }
            AppIcon::Building => {
                rrect(4.0, 3.0, 15.0, 21.0, 1.0);
                line(&[(15.0, 9.0), (20.0, 9.0), (20.0, 21.0)]);
                line(&[(2.5, 21.0), (21.5, 21.0)]);
                for (x, y) in [(7.5, 7.0), (11.5, 7.0), (7.5, 11.0), (11.5, 11.0), (7.5, 15.0), (11.5, 15.0)] {
                    dot(x, y, 0.9);
                }
            }
            AppIcon::PieChart => {
                arc(12.0, 12.0, 9.0, -PI / 2.0, PI * 1.5);
                line(&[(12.0, 3.0), (12.0, 12.0), (21.0, 12.0)]);
            }
            AppIcon::Target => {
                circle(12.0, 12.0, 9.0);
                circle(12.0, 12.0, 5.0);
                dot(12.0, 12.0, 1.5);
            }
            AppIcon::Rocket => {
                closed(&[(12.0, 2.5), (16.0, 7.0), (16.0, 16.0), (8.0, 16.0), (8.0, 7.0)]);
                line(&[(8.0, 12.0), (4.5, 15.5), (4.5, 19.0), (8.0, 16.0)]);
                line(&[(16.0, 12.0), (19.5, 15.5), (19.5, 19.0), (16.0, 16.0)]);
                circle(12.0, 9.5, 1.6);
                line(&[(10.5, 19.0), (12.0, 22.0), (13.5, 19.0)]);
            }
            AppIcon::Bug => {
                rrect(7.5, 7.0, 16.5, 20.0, 4.5);
                line(&[(12.0, 11.0), (12.0, 20.0)]);
                line(&[(3.5, 10.0), (7.5, 12.0)]);
                line(&[(20.5, 10.0), (16.5, 12.0)]);
                line(&[(3.5, 15.5), (7.5, 15.5)]);
                line(&[(20.5, 15.5), (16.5, 15.5)]);
                line(&[(9.5, 7.0), (8.0, 4.0)]);
                line(&[(14.5, 7.0), (16.0, 4.0)]);
            }
            AppIcon::ListChecks => {
                line(&[(3.0, 6.0), (4.8, 7.8), (8.0, 4.5)]);
                line(&[(3.0, 13.0), (4.8, 14.8), (8.0, 11.5)]);
                line(&[(11.0, 6.5), (21.0, 6.5)]);
                line(&[(11.0, 13.5), (21.0, 13.5)]);
                line(&[(11.0, 19.5), (21.0, 19.5)]);
                dot(5.0, 19.5, 1.2);
            }
            AppIcon::Library => {
                line(&[(5.0, 3.5), (5.0, 20.5)]);
                line(&[(9.5, 3.5), (9.5, 20.5)]);
                line(&[(13.5, 4.5), (18.5, 20.0)]);
                line(&[(3.0, 20.5), (21.0, 20.5)]);
            }
            AppIcon::Radio => {
                circle(12.0, 12.0, 2.0);
                arc(12.0, 12.0, 6.0, -PI / 4.0, PI / 4.0);
                arc(12.0, 12.0, 6.0, PI * 0.75, PI * 1.25);
                arc(12.0, 12.0, 10.0, -PI / 4.0, PI / 4.0);
                arc(12.0, 12.0, 10.0, PI * 0.75, PI * 1.25);
            }
            AppIcon::Smile => {
                circle(12.0, 12.0, 9.0);
                arc(12.0, 12.0, 5.0, PI * 0.2, PI * 0.8);
                dot(9.0, 9.5, 1.2);
                dot(15.0, 9.5, 1.2);
            }
            AppIcon::AtSign => {
                circle(12.0, 12.0, 3.8);
                arc(12.0, 12.0, 9.0, PI * 0.15, PI * 2.0);
                line(&[(15.8, 9.0), (15.8, 14.0)]);
                arc(18.3, 14.0, 2.5, 0.0, PI);
            }
            AppIcon::Refresh => {
                arc(12.0, 12.0, 8.0, -PI * 0.2, PI * 1.45);
                line(&[(20.5, 3.5), (20.5, 8.5), (15.5, 8.5)]);
            }
            AppIcon::External => {
                line(&[(14.0, 3.5), (20.5, 3.5), (20.5, 10.0)]);
                line(&[(20.5, 3.5), (11.0, 13.0)]);
                line(&[(17.0, 14.0), (17.0, 20.5), (3.5, 20.5), (3.5, 7.0), (10.0, 7.0)]);
            }
        }
    }
}
