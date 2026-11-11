//! A small set of vector icons drawn with strokes, so the AI components need no icon font.

use egui::{pos2, Color32, Painter, Pos2, Rect, Shape, Stroke};
use std::f32::consts::{PI, TAU};

/// A line icon painted with [`paint_icon`]. All icons are designed on a 24 x 24 grid.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
/// glowkit_ai::paint_icon(ui.painter(), glowkit_ai::Icon::Copy, rect, egui::Color32::GRAY);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Copy,
    Check,
    X,
    Plus,
    Refresh,
    ThumbUp,
    ThumbDown,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    ArrowUp,
    ArrowDown,
    ArrowRight,
    Stop,
    Mic,
    Paperclip,
    Sparkle,
    Globe,
    Code,
    Gear,
    Search,
    File,
    Image,
    Eye,
    External,
    Clock,
    Bot,
    User,
    Bolt,
    Lock,
    Pencil,
    More,
    Bookmark,
    Lightbulb,
    Terminal,
    Download,
    AlertCircle,
    Circle,
    Play,
    Pause,
    Brain,
    History,
}

struct Pen<'a> {
    painter: &'a Painter,
    rect: Rect,
    stroke: Stroke,
    color: Color32,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.rect.min.x + x / 24.0 * self.rect.width(), self.rect.min.y + y / 24.0 * self.rect.height())
    }
    fn s(&self, v: f32) -> f32 {
        v / 24.0 * self.rect.width()
    }
    fn line(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.add(Shape::line(pts, self.stroke));
    }
    fn closed(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.add(Shape::closed_line(pts, self.stroke));
    }
    fn circle(&self, x: f32, y: f32, r: f32) {
        self.painter.circle_stroke(self.p(x, y), self.s(r), self.stroke);
    }
    fn dot(&self, x: f32, y: f32, r: f32) {
        self.painter.circle_filled(self.p(x, y), self.s(r), self.color);
    }
    /// Arc around (x, y), angles in degrees (0 = 3 o'clock, clockwise).
    fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
        let n = (((a1 - a0).abs() / 360.0) * 40.0).ceil().max(4.0) as usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                self.p(x + a.cos() * r, y + a.sin() * r)
            })
            .collect();
        self.painter.add(Shape::line(pts.clone(), self.stroke));
        pts
    }
    fn rrect(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) {
        let rect = Rect::from_min_max(self.p(x0, y0), self.p(x1, y1));
        let cr = egui::CornerRadius::same(self.s(r).round() as u8);
        self.painter.rect_stroke(rect, cr, self.stroke, egui::StrokeKind::Middle);
    }
    fn rrect_fill(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) {
        let rect = Rect::from_min_max(self.p(x0, y0), self.p(x1, y1));
        let cr = egui::CornerRadius::same(self.s(r).round() as u8);
        self.painter.rect_filled(rect, cr, self.color);
    }
    fn fill(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.add(Shape::convex_polygon(pts, self.color, Stroke::NONE));
    }
}

/// Paint `icon` inside `rect` (square works best) with a stroke width scaled to the size.
pub fn paint_icon(painter: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let w = (rect.width() / 24.0 * 1.9).clamp(1.1, 3.0);
    paint_icon_width(painter, icon, rect, color, w);
}

/// Like [`paint_icon`] with an explicit stroke width in points.
pub fn paint_icon_width(painter: &Painter, icon: Icon, rect: Rect, color: Color32, width: f32) {
    let pen = Pen { painter, rect, stroke: Stroke::new(width, color), color };
    match icon {
        Icon::Copy => {
            pen.rrect(8.0, 8.0, 20.0, 20.0, 2.5);
            pen.line(&[(16.0, 4.5), (16.0, 4.0), (6.5, 4.0), (4.0, 6.5), (4.0, 16.0), (4.5, 16.0)]);
        }
        Icon::Check => pen.line(&[(4.5, 12.5), (9.5, 17.5), (19.5, 6.5)]),
        Icon::X => {
            pen.line(&[(6.0, 6.0), (18.0, 18.0)]);
            pen.line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Plus => {
            pen.line(&[(12.0, 5.0), (12.0, 19.0)]);
            pen.line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Refresh | Icon::History => {
            let ccw = icon == Icon::History;
            if ccw {
                pen.arc(12.0, 12.0, 8.0, 200.0, 520.0);
                pen.line(&[(3.5, 5.5), (4.6, 9.4), (8.6, 8.6)]);
                pen.line(&[(12.0, 8.0), (12.0, 12.0), (15.0, 14.0)]);
            } else {
                pen.arc(12.0, 12.0, 8.0, -20.0, 290.0);
                pen.line(&[(20.5, 5.0), (19.6, 9.3), (15.4, 8.6)]);
            }
        }
        Icon::ThumbUp | Icon::ThumbDown => {
            let f = |(x, y): (f32, f32)| if icon == Icon::ThumbDown { (x, 24.0 - y) } else { (x, y) };
            let hand = [(8.0, 10.5), (11.5, 3.5), (12.8, 3.5), (14.2, 5.0), (13.4, 9.5), (18.6, 9.5), (20.3, 11.6), (18.6, 19.0), (17.0, 20.0), (8.0, 20.0)];
            let hand: Vec<(f32, f32)> = hand.into_iter().map(f).collect();
            pen.closed(&hand);
            let a = f((3.5, 10.5));
            let b = f((8.0, 20.0));
            pen.rrect(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1), 1.2);
        }
        Icon::ChevronDown => pen.line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]),
        Icon::ChevronUp => pen.line(&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)]),
        Icon::ChevronLeft => pen.line(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]),
        Icon::ChevronRight => pen.line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
        Icon::ArrowUp => {
            pen.line(&[(12.0, 19.0), (12.0, 5.0)]);
            pen.line(&[(6.0, 11.0), (12.0, 5.0), (18.0, 11.0)]);
        }
        Icon::ArrowDown => {
            pen.line(&[(12.0, 5.0), (12.0, 19.0)]);
            pen.line(&[(6.0, 13.0), (12.0, 19.0), (18.0, 13.0)]);
        }
        Icon::ArrowRight => {
            pen.line(&[(5.0, 12.0), (19.0, 12.0)]);
            pen.line(&[(13.0, 6.0), (19.0, 12.0), (13.0, 18.0)]);
        }
        Icon::Stop => pen.rrect_fill(6.5, 6.5, 17.5, 17.5, 2.0),
        Icon::Mic => {
            pen.rrect(9.0, 3.0, 15.0, 14.5, 3.0);
            pen.arc(12.0, 11.0, 6.5, 0.0, 180.0);
            pen.line(&[(12.0, 17.5), (12.0, 21.0)]);
            pen.line(&[(8.5, 21.0), (15.5, 21.0)]);
        }
        Icon::Paperclip => {
            // a stretched "U" inside another, open at the top-right
            let a = pen.arc(12.0, 16.0, 4.5, 0.0, 180.0);
            let b = pen.arc(12.0, 8.0, 3.0, 180.0, 360.0);
            let c = pen.arc(12.5, 15.0, 1.5, 0.0, 180.0);
            painter.line_segment([a[0], pen.p(16.5, 8.0)], pen.stroke);
            painter.line_segment([*a.last().unwrap(), b[0]], pen.stroke);
            painter.line_segment([*b.last().unwrap(), pen.p(15.0, 15.0)], pen.stroke);
            painter.line_segment([*c.last().unwrap(), pen.p(11.0, 9.5)], pen.stroke);
        }
        Icon::Sparkle => {
            glowkit_core::paint::sparkle(painter, pen.p(10.0, 13.0), pen.s(8.5), 0.0, color);
            glowkit_core::paint::sparkle(painter, pen.p(18.5, 5.5), pen.s(3.6), 0.0, color);
        }
        Icon::Globe => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(3.0, 12.0), (21.0, 12.0)]);
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = i as f32 / 24.0 * TAU;
                    pen.p(12.0 + a.cos() * 4.0, 12.0 + a.sin() * 9.0)
                })
                .collect();
            painter.add(Shape::closed_line(pts, pen.stroke));
        }
        Icon::Code => {
            pen.line(&[(8.0, 7.0), (3.0, 12.0), (8.0, 17.0)]);
            pen.line(&[(16.0, 7.0), (21.0, 12.0), (16.0, 17.0)]);
        }
        Icon::Gear => {
            pen.circle(12.0, 12.0, 3.0);
            pen.circle(12.0, 12.0, 6.8);
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let (s, c) = a.sin_cos();
                pen.line(&[(12.0 + c * 7.0, 12.0 + s * 7.0), (12.0 + c * 9.6, 12.0 + s * 9.6)]);
            }
        }
        Icon::Search => {
            pen.circle(10.5, 10.5, 6.5);
            pen.line(&[(15.5, 15.5), (20.0, 20.0)]);
        }
        Icon::File => {
            pen.closed(&[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)]);
            pen.line(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
        }
        Icon::Image => {
            pen.rrect(3.5, 4.5, 20.5, 19.5, 2.5);
            pen.dot(9.0, 9.5, 1.6);
            pen.line(&[(4.0, 17.5), (10.0, 12.0), (14.0, 15.5), (16.5, 13.0), (20.0, 16.5)]);
        }
        Icon::Eye => {
            let top: Vec<(f32, f32)> = (0..=16)
                .map(|i| {
                    let u = i as f32 / 16.0;
                    (2.5 + u * 19.0, 12.0 - (u * PI).sin() * 6.5)
                })
                .collect();
            let bottom: Vec<(f32, f32)> = top.iter().rev().map(|&(x, y)| (x, 24.0 - y)).collect();
            let mut all = top;
            all.extend(bottom);
            pen.closed(&all);
            pen.circle(12.0, 12.0, 2.8);
        }
        Icon::External => {
            pen.line(&[(11.0, 5.0), (5.0, 5.0), (5.0, 19.0), (19.0, 19.0), (19.0, 13.0)]);
            pen.line(&[(11.0, 13.0), (20.0, 4.0)]);
            pen.line(&[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0)]);
        }
        Icon::Clock => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(12.0, 7.0), (12.0, 12.0), (15.5, 14.0)]);
        }
        Icon::Bot => {
            pen.rrect(4.0, 8.0, 20.0, 20.0, 3.5);
            pen.line(&[(12.0, 8.0), (12.0, 4.5)]);
            pen.dot(12.0, 3.8, 1.3);
            pen.dot(9.0, 13.5, 1.4);
            pen.dot(15.0, 13.5, 1.4);
            pen.line(&[(2.0, 13.0), (2.0, 15.0)]);
            pen.line(&[(22.0, 13.0), (22.0, 15.0)]);
        }
        Icon::User => {
            pen.circle(12.0, 8.0, 4.0);
            pen.arc(12.0, 21.5, 7.5, 180.0, 360.0);
        }
        Icon::Bolt => pen.closed(&[(13.0, 2.5), (4.5, 13.5), (11.5, 13.5), (10.5, 21.5), (19.5, 10.5), (12.5, 10.5)]),
        Icon::Lock => {
            pen.rrect(5.0, 11.0, 19.0, 21.0, 2.0);
            pen.arc(12.0, 9.0, 4.0, 180.0, 360.0);
            pen.line(&[(8.0, 9.0), (8.0, 11.0)]);
            pen.line(&[(16.0, 9.0), (16.0, 11.0)]);
        }
        Icon::Pencil => {
            pen.closed(&[(15.5, 4.5), (19.5, 8.5), (8.5, 19.5), (4.0, 20.0), (4.5, 15.5)]);
            pen.line(&[(13.0, 7.0), (17.0, 11.0)]);
        }
        Icon::More => {
            for x in [5.5, 12.0, 18.5] {
                pen.dot(x, 12.0, 1.5);
            }
        }
        Icon::Bookmark => pen.closed(&[(6.0, 3.5), (18.0, 3.5), (18.0, 20.5), (12.0, 16.5), (6.0, 20.5)]),
        Icon::Lightbulb => {
            pen.arc(12.0, 9.5, 6.0, 140.0, 400.0);
            pen.line(&[(7.4, 13.4), (9.5, 16.0), (9.5, 17.5), (14.5, 17.5), (14.5, 16.0), (16.6, 13.4)]);
            pen.line(&[(10.0, 20.5), (14.0, 20.5)]);
        }
        Icon::Terminal => {
            pen.line(&[(5.0, 7.0), (10.0, 12.0), (5.0, 17.0)]);
            pen.line(&[(12.5, 17.0), (19.0, 17.0)]);
        }
        Icon::Download => {
            pen.line(&[(12.0, 4.0), (12.0, 15.0)]);
            pen.line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]);
            pen.line(&[(4.0, 16.0), (4.0, 20.0), (20.0, 20.0), (20.0, 16.0)]);
        }
        Icon::AlertCircle => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(12.0, 7.5), (12.0, 13.0)]);
            pen.dot(12.0, 16.3, 1.3);
        }
        Icon::Circle => pen.circle(12.0, 12.0, 8.0),
        Icon::Play => pen.fill(&[(7.0, 4.5), (19.0, 12.0), (7.0, 19.5)]),
        Icon::Pause => {
            pen.rrect_fill(6.0, 5.0, 10.0, 19.0, 1.2);
            pen.rrect_fill(14.0, 5.0, 18.0, 19.0, 1.2);
        }
        Icon::Brain => {
            // two lobes
            pen.arc(9.0, 9.0, 4.5, 150.0, 330.0);
            pen.arc(15.0, 9.0, 4.5, 210.0, 390.0);
            pen.arc(8.0, 14.5, 4.0, 90.0, 230.0);
            pen.arc(16.0, 14.5, 4.0, -50.0, 90.0);
            pen.line(&[(8.0, 18.5), (12.0, 18.5), (16.0, 18.5)]);
            pen.line(&[(12.0, 6.0), (12.0, 18.5)]);
        }
    }
}
