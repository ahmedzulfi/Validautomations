//! [`Whiteboard`]: a tldraw / Excalidraw style infinite whiteboard.

use crate::common::{self, icon_button, paint_icon, panel, qfont, rotate_about, Icon};
use crate::freehand::{paint_ink, InkPoint, InkStyle};
use crate::json::{self, Json};
use crate::undo::History;
use crate::view::{Background, CanvasView, InfiniteCanvas};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Id, Painter, PointerButton, Pos2, Rect, Sense, Shape,
    Stroke, StrokeKind, TextEdit, Ui, Vec2,
};
use glowkit_core::{color::with_alpha, paint, Rng, Theme};

/// The active whiteboard tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Tool {
    #[default]
    Select,
    Hand,
    Pen,
    Highlighter,
    Rect,
    Ellipse,
    Diamond,
    Arrow,
    Line,
    Text,
    Sticky,
    Eraser,
}

impl Tool {
    /// Every tool in toolbar order.
    pub const ALL: [Tool; 12] = [
        Tool::Select,
        Tool::Hand,
        Tool::Pen,
        Tool::Highlighter,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Diamond,
        Tool::Arrow,
        Tool::Line,
        Tool::Text,
        Tool::Sticky,
        Tool::Eraser,
    ];
    fn icon(self) -> Icon {
        match self {
            Tool::Select => Icon::Select,
            Tool::Hand => Icon::Hand,
            Tool::Pen => Icon::Pen,
            Tool::Highlighter => Icon::Highlighter,
            Tool::Rect => Icon::Rect,
            Tool::Ellipse => Icon::Ellipse,
            Tool::Diamond => Icon::Diamond,
            Tool::Arrow => Icon::Arrow,
            Tool::Line => Icon::Line,
            Tool::Text => Icon::Text,
            Tool::Sticky => Icon::Sticky,
            Tool::Eraser => Icon::Eraser,
        }
    }
    /// Keyboard shortcut.
    pub fn key(self) -> egui::Key {
        use egui::Key;
        match self {
            Tool::Select => Key::V,
            Tool::Hand => Key::H,
            Tool::Pen => Key::P,
            Tool::Highlighter => Key::M,
            Tool::Rect => Key::R,
            Tool::Ellipse => Key::O,
            Tool::Diamond => Key::D,
            Tool::Arrow => Key::A,
            Tool::Line => Key::L,
            Tool::Text => Key::T,
            Tool::Sticky => Key::S,
            Tool::Eraser => Key::E,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Hand => "Hand",
            Tool::Pen => "Pen",
            Tool::Highlighter => "Highlighter",
            Tool::Rect => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Diamond => "Diamond",
            Tool::Arrow => "Arrow",
            Tool::Line => "Line",
            Tool::Text => "Text",
            Tool::Sticky => "Sticky note",
            Tool::Eraser => "Eraser",
        }
    }
}

/// Kind of a whiteboard shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Diamond,
    Line,
    Arrow,
    Freehand,
    Highlight,
    Text,
    Sticky,
}

impl ShapeKind {
    fn name(self) -> &'static str {
        match self {
            ShapeKind::Rect => "rect",
            ShapeKind::Ellipse => "ellipse",
            ShapeKind::Diamond => "diamond",
            ShapeKind::Line => "line",
            ShapeKind::Arrow => "arrow",
            ShapeKind::Freehand => "freehand",
            ShapeKind::Highlight => "highlight",
            ShapeKind::Text => "text",
            ShapeKind::Sticky => "sticky",
        }
    }
    fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "rect" => ShapeKind::Rect,
            "ellipse" => ShapeKind::Ellipse,
            "diamond" => ShapeKind::Diamond,
            "line" => ShapeKind::Line,
            "arrow" => ShapeKind::Arrow,
            "freehand" => ShapeKind::Freehand,
            "highlight" => ShapeKind::Highlight,
            "text" => ShapeKind::Text,
            "sticky" => ShapeKind::Sticky,
            _ => return None,
        })
    }
    fn is_linear(self) -> bool {
        matches!(self, ShapeKind::Line | ShapeKind::Arrow)
    }
    fn is_bindable(self) -> bool {
        matches!(self, ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Diamond | ShapeKind::Text | ShapeKind::Sticky)
    }
}

/// "Ink" color: rendered as the theme foreground so black strokes stay visible in dark mode.
pub const INK: Color32 = Color32::BLACK;

/// One shape on the whiteboard (world coordinates).
#[derive(Clone, Debug, PartialEq)]
pub struct WbShape {
    pub id: u64,
    pub kind: ShapeKind,
    /// Box of box-like shapes, text and freehand bounds (unrotated).
    pub rect: Rect,
    /// Rotation in radians around `rect.center()`.
    pub rotation: f32,
    /// Endpoints of lines / arrows.
    pub a: Pos2,
    pub b: Pos2,
    /// Shapes the arrow endpoints are bound to.
    pub bind_a: Option<u64>,
    pub bind_b: Option<u64>,
    /// Freehand samples (world).
    pub points: Vec<InkPoint>,
    /// Stroke color ([`INK`] = theme foreground).
    pub stroke: Color32,
    /// Fill color.
    pub fill: Option<Color32>,
    /// Stroke width.
    pub width: f32,
    /// Hand-drawn, slightly wobbly rendering.
    pub sketchy: bool,
    /// Seed for the sketchy jitter.
    pub seed: u64,
    /// Text of text shapes, sticky notes and shape labels.
    pub text: String,
    /// Font size.
    pub font_size: f32,
}

impl WbShape {
    /// A new shape of `kind` occupying `rect`.
    pub fn new(kind: ShapeKind, rect: Rect) -> Self {
        Self {
            id: 0,
            kind,
            rect,
            rotation: 0.0,
            a: rect.left_top(),
            b: rect.right_bottom(),
            bind_a: None,
            bind_b: None,
            points: Vec::new(),
            stroke: INK,
            fill: None,
            width: 2.0,
            sketchy: false,
            seed: 7,
            text: String::new(),
            font_size: 20.0,
        }
    }
    /// A line or arrow from `a` to `b`.
    pub fn linear(kind: ShapeKind, a: Pos2, b: Pos2) -> Self {
        let mut s = Self::new(kind, Rect::from_two_pos(a, b));
        s.a = a;
        s.b = b;
        s
    }
    /// A text shape.
    pub fn text(pos: Pos2, text: impl Into<String>, size: f32) -> Self {
        let mut s = Self::new(ShapeKind::Text, Rect::from_min_size(pos, vec2(200.0, size * 1.4)));
        s.text = text.into();
        s.font_size = size;
        s
    }
    /// A sticky note.
    pub fn sticky(pos: Pos2, text: impl Into<String>, color: Color32) -> Self {
        let mut s = Self::new(ShapeKind::Sticky, Rect::from_min_size(pos, vec2(170.0, 150.0)));
        s.text = text.into();
        s.fill = Some(color);
        s.font_size = 16.0;
        s
    }
    /// A freehand stroke through `points`.
    pub fn freehand(points: Vec<InkPoint>, highlight: bool) -> Self {
        let mut s = Self::new(if highlight { ShapeKind::Highlight } else { ShapeKind::Freehand }, Rect::NOTHING);
        s.points = points;
        s.recompute_bounds();
        s.width = if highlight { 16.0 } else { 3.0 };
        s
    }
    /// Builder: stroke color.
    pub fn stroke(mut self, c: Color32) -> Self {
        self.stroke = c;
        self
    }
    /// Builder: fill.
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }
    /// Builder: width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Builder: sketchy rendering.
    pub fn sketchy(mut self, on: bool) -> Self {
        self.sketchy = on;
        self
    }
    /// Builder: label / text.
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.text = s.into();
        self
    }
    /// Builder: rotation.
    pub fn rotated(mut self, r: f32) -> Self {
        self.rotation = r;
        self
    }

    fn recompute_bounds(&mut self) {
        match self.kind {
            k if k.is_linear() => self.rect = Rect::from_two_pos(self.a, self.b),
            ShapeKind::Freehand | ShapeKind::Highlight => {
                self.rect = common::bounds_of(self.points.iter().map(|p| p.pos));
                if self.rect.width() < 1.0 {
                    self.rect = self.rect.expand2(vec2(0.5, 0.0));
                }
                if self.rect.height() < 1.0 {
                    self.rect = self.rect.expand2(vec2(0.0, 0.5));
                }
            }
            _ => {}
        }
    }

    /// Move by `d`.
    pub fn translate(&mut self, d: Vec2) {
        self.rect = self.rect.translate(d);
        self.a += d;
        self.b += d;
        for p in &mut self.points {
            p.pos += d;
        }
    }

    /// Change the box, remapping freehand points.
    pub fn set_rect(&mut self, r: Rect) {
        let old = self.rect;
        if matches!(self.kind, ShapeKind::Freehand | ShapeKind::Highlight) {
            let sx = r.width() / old.width().max(1e-3);
            let sy = r.height() / old.height().max(1e-3);
            for p in &mut self.points {
                p.pos = pos2(r.min.x + (p.pos.x - old.min.x) * sx, r.min.y + (p.pos.y - old.min.y) * sy);
            }
        }
        self.rect = r;
    }

    /// Outline polygon (world, rotation applied) of box-like shapes.
    pub fn outline(&self) -> Vec<Pos2> {
        let r = self.rect;
        let local: Vec<Pos2> = match self.kind {
            ShapeKind::Ellipse => (0..48)
                .map(|i| {
                    let a = i as f32 / 48.0 * std::f32::consts::TAU;
                    r.center() + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                })
                .collect(),
            ShapeKind::Diamond => vec![
                pos2(r.center().x, r.top()),
                pos2(r.right(), r.center().y),
                pos2(r.center().x, r.bottom()),
                pos2(r.left(), r.center().y),
            ],
            _ => vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()],
        };
        local.into_iter().map(|p| rotate_about(p, r.center(), self.rotation)).collect()
    }

    /// Does the world point `p` hit this shape (with tolerance `tol`)?
    pub fn hit(&self, p: Pos2, tol: f32) -> bool {
        match self.kind {
            k if k.is_linear() => common::dist_to_segment(p, self.a, self.b) <= tol + self.width,
            ShapeKind::Freehand | ShapeKind::Highlight => {
                let pts: Vec<Pos2> = self.points.iter().map(|q| q.pos).collect();
                common::dist_to_polyline(p, &pts) <= tol + self.width * 0.6
            }
            ShapeKind::Text | ShapeKind::Sticky => common::point_in_polygon(p, &self.outline()),
            _ => {
                let o = self.outline();
                let filled = self.fill.is_some() || !self.text.is_empty();
                (filled && common::point_in_polygon(p, &o)) || {
                    let mut closed = o.clone();
                    closed.push(o[0]);
                    common::dist_to_polyline(p, &closed) <= tol + self.width
                }
            }
        }
    }

    /// World-space bounding box (rotation included).
    pub fn bounds(&self) -> Rect {
        if self.kind.is_linear() {
            return Rect::from_two_pos(self.a, self.b).expand(self.width);
        }
        if self.rotation == 0.0 {
            return self.rect;
        }
        common::bounds_of(self.outline())
    }
}

/// A whiteboard document: shapes in z-order (last on top). Owned by the caller.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Document {
    pub shapes: Vec<WbShape>,
}

impl Document {
    /// Add a shape (assigning a fresh id) and return the id.
    pub fn add(&mut self, mut s: WbShape) -> u64 {
        let id = self.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        s.id = id;
        if s.seed == 7 {
            s.seed = id.wrapping_mul(2654435761);
        }
        self.shapes.push(s);
        id
    }
    /// Shape by id.
    pub fn get(&self, id: u64) -> Option<&WbShape> {
        self.shapes.iter().find(|s| s.id == id)
    }
    /// Mutable shape by id.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut WbShape> {
        self.shapes.iter_mut().find(|s| s.id == id)
    }
    /// Remove shapes (and unbind arrows pointing at them).
    pub fn remove(&mut self, ids: &[u64]) {
        self.shapes.retain(|s| !ids.contains(&s.id));
        for s in &mut self.shapes {
            if s.bind_a.is_some_and(|b| ids.contains(&b)) {
                s.bind_a = None;
            }
            if s.bind_b.is_some_and(|b| ids.contains(&b)) {
                s.bind_b = None;
            }
        }
    }
    /// Topmost shape hit by `p` (bound arrow ends resolved).
    pub fn hit(&self, p: Pos2, tol: f32) -> Option<u64> {
        self.shapes
            .iter()
            .rev()
            .find(|s| {
                if s.kind.is_linear() {
                    let (a, b) = self.arrow_ends(s);
                    common::dist_to_segment(p, a, b) <= tol + s.width
                } else {
                    s.hit(p, tol)
                }
            })
            .map(|s| s.id)
    }
    /// Topmost shape an arrow endpoint at `p` could bind to.
    pub fn bind_target(&self, p: Pos2, exclude: u64) -> Option<u64> {
        self.shapes
            .iter()
            .rev()
            .find(|s| s.id != exclude && s.kind.is_bindable() && common::point_in_polygon(p, &s.outline()))
            .map(|s| s.id)
    }
    /// Bounds of everything.
    pub fn bounds(&self) -> Rect {
        let ids: Vec<u64> = self.shapes.iter().map(|s| s.id).collect();
        self.bounds_of(&ids)
    }
    /// Bounds of some shapes (arrow endpoints resolved).
    pub fn bounds_of(&self, ids: &[u64]) -> Rect {
        let mut r = Rect::NOTHING;
        for s in self.shapes.iter().filter(|s| ids.contains(&s.id)) {
            if s.kind.is_linear() {
                let (a, b) = self.arrow_ends(s);
                r = r.union(Rect::from_two_pos(a, b));
            } else {
                r = r.union(s.bounds());
            }
        }
        r
    }

    /// Resolve the endpoints of a line / arrow, snapping bound ends to their shape outlines.
    pub fn arrow_ends(&self, s: &WbShape) -> (Pos2, Pos2) {
        let other_a = s.bind_b.and_then(|id| self.get(id)).map(|t| t.rect.center()).unwrap_or(s.b);
        let other_b = s.bind_a.and_then(|id| self.get(id)).map(|t| t.rect.center()).unwrap_or(s.a);
        let a = s.bind_a.and_then(|id| self.get(id)).map(|t| edge_point(t, other_a, 8.0)).unwrap_or(s.a);
        let b = s.bind_b.and_then(|id| self.get(id)).map(|t| edge_point(t, other_b, 8.0)).unwrap_or(s.b);
        (a, b)
    }

    /// Move shapes to the very top (`up`) or bottom of the z-order.
    pub fn reorder(&mut self, ids: &[u64], to_front: bool) {
        let (mut sel, rest): (Vec<WbShape>, Vec<WbShape>) = self.shapes.drain(..).partition(|s| ids.contains(&s.id));
        if to_front {
            self.shapes = rest;
            self.shapes.append(&mut sel);
        } else {
            sel.extend(rest);
            self.shapes = sel;
        }
    }

    /// Move shapes one step forward (`+1`) or backward (`-1`).
    pub fn step(&mut self, ids: &[u64], dir: i32) {
        if dir > 0 {
            for i in (0..self.shapes.len().saturating_sub(1)).rev() {
                if ids.contains(&self.shapes[i].id) && !ids.contains(&self.shapes[i + 1].id) {
                    self.shapes.swap(i, i + 1);
                }
            }
        } else {
            for i in 1..self.shapes.len() {
                if ids.contains(&self.shapes[i].id) && !ids.contains(&self.shapes[i - 1].id) {
                    self.shapes.swap(i, i - 1);
                }
            }
        }
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> String {
        let col = |c: Color32| {
            let [r, g, b, a] = c.to_srgba_unmultiplied();
            json::quote(&format!("#{r:02x}{g:02x}{b:02x}{a:02x}"))
        };
        let opt = |o: Option<u64>| o.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let mut out = String::from("{\"version\":1,\"shapes\":[");
        for (i, s) in self.shapes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let pts: Vec<String> = s
                .points
                .iter()
                .map(|p| format!("{},{},{}", json::num(p.pos.x), json::num(p.pos.y), json::num(p.t)))
                .collect();
            out.push_str(&format!(
                "\n{{\"id\":{},\"kind\":\"{}\",\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"rot\":{},\"ax\":{},\"ay\":{},\"bx\":{},\"by\":{},\"bindA\":{},\"bindB\":{},\"stroke\":{},\"fill\":{},\"width\":{},\"sketchy\":{},\"seed\":{},\"text\":{},\"font\":{},\"points\":[{}]}}",
                s.id,
                s.kind.name(),
                json::num(s.rect.min.x),
                json::num(s.rect.min.y),
                json::num(s.rect.width()),
                json::num(s.rect.height()),
                json::num(s.rotation),
                json::num(s.a.x),
                json::num(s.a.y),
                json::num(s.b.x),
                json::num(s.b.y),
                opt(s.bind_a),
                opt(s.bind_b),
                col(s.stroke),
                s.fill.map(col).unwrap_or_else(|| "null".into()),
                json::num(s.width),
                s.sketchy,
                s.seed % 1_000_000_007,
                json::quote(&s.text),
                json::num(s.font_size),
                pts.join(",")
            ));
        }
        out.push_str("\n]}");
        out
    }

    /// Parse a document produced by [`Document::to_json`].
    pub fn from_json(src: &str) -> Result<Document, String> {
        let v = Json::parse(src)?;
        let parse_col = |j: Option<&Json>| -> Option<Color32> {
            let s = j?.str()?;
            let s = s.trim_start_matches('#');
            if s.len() != 8 {
                return None;
            }
            let b = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
            Some(Color32::from_rgba_unmultiplied(b(0)?, b(2)?, b(4)?, b(6)?))
        };
        let mut doc = Document::default();
        for j in v.get("shapes").map(|s| s.arr()).unwrap_or(&[]) {
            let kind = ShapeKind::from_name(&j.s("kind")).ok_or("unknown shape kind")?;
            let rect = Rect::from_min_size(pos2(j.f("x", 0.0), j.f("y", 0.0)), vec2(j.f("w", 0.0), j.f("h", 0.0)));
            let mut s = WbShape::new(kind, rect);
            s.id = j.f("id", 0.0) as u64;
            s.rotation = j.f("rot", 0.0);
            s.a = pos2(j.f("ax", 0.0), j.f("ay", 0.0));
            s.b = pos2(j.f("bx", 0.0), j.f("by", 0.0));
            s.bind_a = j.get("bindA").and_then(Json::num).map(|v| v as u64);
            s.bind_b = j.get("bindB").and_then(Json::num).map(|v| v as u64);
            s.stroke = parse_col(j.get("stroke")).unwrap_or(INK);
            s.fill = parse_col(j.get("fill"));
            s.width = j.f("width", 2.0);
            s.sketchy = j.get("sketchy").and_then(Json::bool).unwrap_or(false);
            s.seed = j.f("seed", 1.0) as u64;
            s.text = j.s("text");
            s.font_size = j.f("font", 20.0);
            let nums: Vec<f32> = j
                .get("points")
                .map(|p| p.arr())
                .unwrap_or(&[])
                .iter()
                .filter_map(|n| n.num().map(|v| v as f32))
                .collect();
            s.points = nums.chunks_exact(3).map(|c| InkPoint::new(pos2(c[0], c[1]), c[2])).collect();
            doc.shapes.push(s);
        }
        Ok(doc)
    }
}

/// Point on the outline of `target` on the ray from its center toward `toward`, pushed out by `gap`.
fn edge_point(target: &WbShape, toward: Pos2, gap: f32) -> Pos2 {
    let c = target.rect.center();
    let dir = (toward - c).normalized();
    if !dir.is_finite() {
        return c;
    }
    let poly = target.outline();
    let far = c + dir * 1e5;
    let mut best: Option<f32> = None;
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        if let Some(t) = seg_intersect(c, far, p, q) {
            best = Some(best.map_or(t, |b: f32| b.max(t)));
        }
    }
    match best {
        Some(t) => c + (far - c) * t + dir * gap,
        None => c,
    }
}

fn seg_intersect(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> Option<f32> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    let u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

// ------------------------------------------------------------------------------------------------
// Sketchy rendering
// ------------------------------------------------------------------------------------------------

/// A wobbly hand-drawn version of the segment `a..b` (screen space).
pub fn rough_segment(a: Pos2, b: Pos2, rng: &mut Rng, rough: f32) -> Vec<Pos2> {
    let len = a.distance(b);
    let d = (b - a).normalized();
    let n = vec2(-d.y, d.x);
    let bow = rng.range(-1.0, 1.0) * (len * 0.012 + 0.6) * rough;
    let j = |rng: &mut Rng| vec2(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)) * rough * 1.1;
    let a2 = a + j(rng) - d * rng.range(0.0, 2.0) * rough;
    let b2 = b + j(rng) + d * rng.range(0.0, 2.0) * rough;
    let m1 = a2.lerp(b2, 0.35) + n * bow + j(rng) * 0.4;
    let m2 = a2.lerp(b2, 0.7) + n * bow * 0.8 + j(rng) * 0.4;
    let steps = ((len / 6.0) as usize).clamp(4, 24);
    paint::cubic_bezier(a2, m1, m2, b2, steps)
}

fn rough_closed(painter: &Painter, pts: &[Pos2], seed: u64, stroke: Stroke, rough: f32, curved: bool) {
    let mut rng = Rng::new(seed);
    for pass in 0..2 {
        if curved {
            // jittered closed curve with a small overshoot
            let n = pts.len();
            let start = rng.below(n);
            let mut out = Vec::with_capacity(n + 6);
            for k in 0..=n + 3 {
                let p = pts[(start + k) % n];
                let c = pts.iter().fold(Vec2::ZERO, |a, q| a + q.to_vec2()) / n as f32;
                let dir = (p - c.to_pos2()).normalized();
                out.push(p + dir * rng.range(-1.0, 1.0) * rough * 1.6);
            }
            let w = if pass == 0 { stroke.width } else { stroke.width * 0.7 };
            painter.add(Shape::line(out, Stroke::new(w, stroke.color)));
        } else {
            for i in 0..pts.len() {
                let seg = rough_segment(pts[i], pts[(i + 1) % pts.len()], &mut rng, rough);
                let w = if pass == 0 { stroke.width } else { stroke.width * 0.7 };
                painter.add(Shape::line(seg, Stroke::new(w, stroke.color)));
            }
        }
    }
}

/// Hachure fill (parallel hatch lines) of a convex polygon, clipped exactly.
fn hachure(painter: &Painter, poly: &[Pos2], gap: f32, angle: f32, stroke: Stroke, seed: u64, rough: f32) {
    let b = common::bounds_of(poly.iter().copied());
    let dir = vec2(angle.cos(), angle.sin());
    let nrm = vec2(-dir.y, dir.x);
    let c = b.center();
    let half = b.size().length() / 2.0;
    let mut rng = Rng::new(seed ^ 0x9e37);
    let mut off = -half;
    while off <= half {
        let p0 = c + nrm * off - dir * half;
        let p1 = c + nrm * off + dir * half;
        let mut ts: Vec<f32> = Vec::new();
        for i in 0..poly.len() {
            if let Some(t) = seg_intersect(p0, p1, poly[i], poly[(i + 1) % poly.len()]) {
                ts.push(t);
            }
        }
        if ts.len() >= 2 {
            let (t0, t1) = (ts.iter().copied().fold(f32::MAX, f32::min), ts.iter().copied().fold(f32::MIN, f32::max));
            let (a, bb) = (p0 + (p1 - p0) * t0, p0 + (p1 - p0) * t1);
            if a.distance(bb) > 2.0 {
                let a = a + dir * 1.5;
                let bb = bb - dir * 1.5;
                painter.add(Shape::line(rough_segment(a, bb, &mut rng, rough * 0.5), stroke));
            }
        }
        off += gap;
    }
}

fn ink(c: Color32, t: &Theme) -> Color32 {
    if c == INK {
        t.foreground
    } else {
        c
    }
}

/// Paint a shape. `ts` maps world → screen; `zoom` scales widths.
pub fn paint_wb_shape(painter: &Painter, t: &Theme, doc: &Document, s: &WbShape, ts: &dyn Fn(Pos2) -> Pos2, zoom: f32) {
    let stroke_col = ink(s.stroke, t);
    let w = (s.width * zoom).max(0.6);
    let rough = 1.0 * zoom.sqrt();
    match s.kind {
        ShapeKind::Freehand => {
            paint_ink(
                painter,
                &s.points,
                &InkStyle::pen(s.width * 2.0),
                stroke_col,
                |p| ts(rotate_about(p, s.rect.center(), s.rotation)),
                zoom,
            );
        }
        ShapeKind::Highlight => {
            let pts: Vec<Pos2> =
                s.points.iter().map(|p| ts(rotate_about(p.pos, s.rect.center(), s.rotation))).collect();
            painter.add(Shape::line(
                pts,
                Stroke::new(s.width * zoom, with_alpha(stroke_col, if t.dark { 0.5 } else { 0.4 })),
            ));
        }
        ShapeKind::Line | ShapeKind::Arrow => {
            let (a, b) = doc.arrow_ends(s);
            let (a, b) = (ts(a), ts(b));
            let st = Stroke::new(w, stroke_col);
            let shorten = if s.kind == ShapeKind::Arrow { w * 1.5 } else { 0.0 };
            let dir = (b - a).normalized();
            let b_line = if dir.is_finite() { b - dir * shorten } else { b };
            if s.sketchy {
                let mut rng = Rng::new(s.seed);
                painter.add(Shape::line(rough_segment(a, b_line, &mut rng, rough), st));
                painter.add(Shape::line(rough_segment(a, b_line, &mut rng, rough), Stroke::new(w * 0.7, stroke_col)));
            } else {
                painter.line_segment([a, b_line], st);
            }
            if s.kind == ShapeKind::Arrow && dir.is_finite() {
                let size = (10.0 + s.width * 2.5) * zoom;
                let n = vec2(-dir.y, dir.x);
                let l = b - dir * size + n * size * 0.5;
                let r = b - dir * size - n * size * 0.5;
                painter.add(Shape::line(vec![l, b, r], Stroke::new(w, stroke_col)));
            }
        }
        ShapeKind::Text => {
            let font = qfont(s.font_size * zoom);
            let g = painter.layout(s.text.clone(), font, stroke_col, f32::INFINITY);
            let min = ts(rotate_about(s.rect.min, s.rect.center(), s.rotation));
            painter.add(egui::epaint::TextShape::new(min, g, stroke_col).with_angle(s.rotation));
        }
        ShapeKind::Sticky => {
            let fill = s.fill.unwrap_or(egui::Color32::from_rgb(255, 229, 143));
            let pts: Vec<Pos2> = s.outline().into_iter().map(ts).collect();
            let r = Rect::from_center_size(ts(s.rect.center()), s.rect.size() * zoom);
            paint::soft_shadow(
                painter,
                r.translate(vec2(0.0, 4.0 * zoom)),
                4.0 * zoom,
                14.0 * zoom,
                vec2(0.0, 3.0 * zoom),
                Color32::from_black_alpha(if t.dark { 120 } else { 40 }),
            );
            painter.add(Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
            // folded corner shading
            let fold = 18.0 * zoom;
            let br = pts[2];
            let ux = (pts[1] - pts[2]).normalized();
            let uy = (pts[3] - pts[2]).normalized();
            painter.add(Shape::convex_polygon(
                vec![br + ux * fold, br, br + uy * fold],
                fill.lerp_to_gamma(Color32::BLACK, 0.12),
                Stroke::NONE,
            ));
            let ink_col = Color32::from_rgb(40, 36, 30);
            let pad = 14.0 * zoom;
            let font = qfont(s.font_size * zoom);
            let g = painter.layout(s.text.clone(), font, ink_col, (s.rect.width() * zoom - pad * 2.0).max(10.0));
            let min = ts(rotate_about(s.rect.min + vec2(14.0, 14.0), s.rect.center(), s.rotation));
            painter.add(egui::epaint::TextShape::new(min, g, ink_col).with_angle(s.rotation));
        }
        ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Diamond => {
            let pts: Vec<Pos2> = s.outline().into_iter().map(ts).collect();
            if let Some(f) = s.fill {
                if s.sketchy {
                    hachure(
                        painter,
                        &pts,
                        (7.0 * zoom).max(3.0),
                        -0.72 + s.rotation,
                        Stroke::new((1.4 * zoom).max(0.6), with_alpha(f, 0.85)),
                        s.seed,
                        rough,
                    );
                } else {
                    painter.add(Shape::convex_polygon(pts.clone(), f, Stroke::NONE));
                }
            }
            let st = Stroke::new(w, stroke_col);
            if s.sketchy {
                rough_closed(painter, &pts, s.seed, st, rough, s.kind == ShapeKind::Ellipse);
            } else if s.kind == ShapeKind::Rect && s.rotation == 0.0 {
                let r = Rect::from_min_max(ts(s.rect.min), ts(s.rect.max));
                painter.rect_stroke(r, CornerRadius::same((6.0 * zoom) as u8), st, StrokeKind::Middle);
            } else {
                painter.add(Shape::closed_line(pts, st));
            }
            if !s.text.is_empty() {
                let font = qfont(s.font_size * 0.85 * zoom);
                let g = painter.layout(s.text.clone(), font, t.foreground, (s.rect.width() * zoom * 0.8).max(10.0));
                let c = ts(s.rect.center());
                let min = c - g.size() / 2.0;
                painter.add(
                    egui::epaint::TextShape::new(rotate_about(min, c, s.rotation), g, t.foreground)
                        .with_angle(s.rotation),
                );
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Editor state + widget
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
enum Op {
    #[default]
    None,
    Draw {
        id: u64,
        before: Box<Document>,
    },
    Create {
        id: u64,
        start: Pos2,
        before: Box<Document>,
    },
    Move {
        before: Box<Document>,
        grab: Pos2,
        last: Vec2,
        moved: bool,
    },
    Resize {
        before: Box<Document>,
        id: u64,
        handle: usize,
        orig: Rect,
    },
    Rotate {
        before: Box<Document>,
        id: u64,
        start_angle: f32,
        orig: f32,
    },
    Endpoint {
        before: Box<Document>,
        id: u64,
        end_b: bool,
    },
    Box {
        start: Pos2,
        end: Pos2,
    },
    Erase {
        before: Box<Document>,
        hits: Vec<u64>,
        trail: Vec<(Pos2, f64)>,
    },
}

type ShapeEdit = Box<dyn Fn(&mut WbShape)>;

/// Current drawing style of a [`Whiteboard`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WbStyle {
    pub color: Color32,
    pub width: f32,
    pub fill: bool,
    pub sketchy: bool,
}

impl Default for WbStyle {
    fn default() -> Self {
        Self { color: INK, width: 2.0, fill: false, sketchy: true }
    }
}

/// Caller-owned state of a [`Whiteboard`].
#[derive(Clone, Debug)]
pub struct WhiteboardState {
    pub view: CanvasView,
    pub tool: Tool,
    pub style: WbStyle,
    pub selected: Vec<u64>,
    pub history: History<Document>,
    /// Id of the text / sticky shape being edited.
    pub editing: Option<u64>,
    /// Last exported JSON (set by the export button).
    pub exported: String,
    op: Op,
    clipboard: Vec<WbShape>,
    help: bool,
    toast: Option<(String, f64)>,
    focus_text: bool,
}

impl Default for WhiteboardState {
    fn default() -> Self {
        Self {
            view: CanvasView::with_limits(0.1, 5.0),
            tool: Tool::Select,
            style: WbStyle::default(),
            selected: Vec::new(),
            history: History::new(100),
            editing: None,
            exported: String::new(),
            op: Op::None,
            clipboard: Vec::new(),
            help: false,
            toast: None,
            focus_text: false,
        }
    }
}

/// Palette offered by the style panel.
pub const WB_COLORS: [Color32; 8] = [
    INK,
    Color32::from_rgb(224, 49, 49),
    Color32::from_rgb(240, 140, 0),
    Color32::from_rgb(47, 158, 68),
    Color32::from_rgb(25, 113, 194),
    Color32::from_rgb(156, 54, 181),
    Color32::from_rgb(12, 133, 153),
    Color32::from_rgb(134, 142, 150),
];

/// Sticky-note colors.
pub const STICKY_COLORS: [Color32; 4] = [
    Color32::from_rgb(255, 229, 143),
    Color32::from_rgb(255, 201, 222),
    Color32::from_rgb(183, 225, 255),
    Color32::from_rgb(195, 240, 202),
];

/// An infinite whiteboard (tldraw / Excalidraw pattern): select, hand, pen with
/// pressure-like strokes, highlighter, rectangle, ellipse, diamond, arrows that bind to shapes,
/// lines, text, sticky notes and an eraser; resize / rotate handles, colors and widths,
/// hand-drawn "sketchy" style, layer ordering, undo/redo and JSON export / import.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_canvas::{Document, Whiteboard, WhiteboardState};
/// # let (mut doc, mut state) = (Document::default(), WhiteboardState::default());
/// Whiteboard::new(&mut doc, &mut state).height(480.0).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Whiteboard<'a> {
    doc: &'a mut Document,
    state: &'a mut WhiteboardState,
    height: f32,
    background: Background,
}

impl<'a> Whiteboard<'a> {
    /// A whiteboard editing `doc`.
    pub fn new(doc: &'a mut Document, state: &'a mut WhiteboardState) -> Self {
        Self { doc, state, height: 480.0, background: Background::Dots }
    }
    /// Canvas height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Background pattern.
    pub fn background(mut self, b: Background) -> Self {
        self.background = b;
        self
    }

    /// Show the whiteboard. Returns `true` when the document changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let Whiteboard { doc, state: st, height, background } = self;
        let t = Theme::of(ui);
        let now = ui.input(|i| i.time);
        let id = ui.scope_id().with("glowkit_whiteboard");
        let mut changed = false;
        let mut view = std::mem::take(&mut st.view);
        let hand = st.tool == Tool::Hand;
        InfiniteCanvas::new(&mut view)
            .background(background)
            .height(height)
            .pan_on_drag(hand)
            .id_salt("whiteboard")
            .show(ui, |cv| {
                let rect = cv.rect;
                for s in doc.shapes.iter_mut().filter(|s| s.kind == ShapeKind::Text) {
                    let g = cv.painter.layout_no_wrap(
                        if s.text.is_empty() { " ".into() } else { s.text.clone() },
                        egui::FontId::proportional(s.font_size),
                        Color32::WHITE,
                    );
                    let size = vec2(g.size().x.max(20.0) + 4.0, g.size().y.max(s.font_size * 1.2));
                    if (s.rect.size() - size).length() > 0.5 {
                        s.rect = Rect::from_min_size(s.rect.min, size);
                    }
                }
                if cv.view.needs_fit && !doc.shapes.is_empty() {
                    cv.view.fit_now(rect, doc.bounds(), 70.0);
                }
                let zoom = cv.view.zoom;
                let (pan, rmin) = (cv.view.pan, rect.min);
                let ts = move |p: Pos2| rmin + pan + p.to_vec2() * zoom;
                let tw = move |p: Pos2| ((p - rmin - pan) / zoom).to_pos2();
                let ui = &mut *cv.ui;
                let painter = cv.painter.clone();
                let pointer = ui.input(|i| i.pointer.hover_pos());
                let pw = pointer.map(tw);
                let shift = ui.input(|i| i.modifiers.shift);
                let typing = ui.ctx().text_edit_focused();
                let bg = cv.response.clone();
                let tol = 6.0 / zoom;

                // ---- chrome rects (so canvas logic ignores clicks on them)
                let toolbar_rect = Rect::from_center_size(
                    pos2(rect.center().x, rect.top() + 30.0),
                    vec2(Tool::ALL.len() as f32 * 34.0 + 10.0, 42.0),
                );
                let style_rect = Rect::from_min_size(rect.min + vec2(12.0, 64.0), vec2(176.0, 236.0));

                // ---- selection handles
                let single = (st.selected.len() == 1).then(|| doc.get(st.selected[0])).flatten().cloned();
                let handle_pts: Vec<(usize, Pos2)> = match single.as_ref() {
                    Some(s) if s.kind.is_linear() => {
                        let (a, b) = doc.arrow_ends(s);
                        vec![(100, ts(a)), (101, ts(b))]
                    }
                    Some(s) => handle_points(s, &ts).into_iter().enumerate().collect(),
                    None => Vec::new(),
                };
                let handle_hit = |p: Pos2| -> Option<usize> {
                    handle_pts.iter().find(|(_, h)| h.distance(p) < 9.0).map(|(k, _)| *k)
                };

                // ---- start operations
                let press = ui.input(|i| i.pointer.press_origin());
                if bg.drag_started_by(PointerButton::Primary) && !cv.space_pan && !hand {
                    let start_s = press.unwrap_or_default();
                    let start = tw(start_s);
                    let before = Box::new(doc.clone());
                    match st.tool {
                        Tool::Select => {
                            if let Some(h) = handle_hit(start_s) {
                                let s = single.clone().expect("single");
                                st.op = match h {
                                    100 | 101 => Op::Endpoint { before, id: s.id, end_b: h == 101 },
                                    8 => {
                                        let c = s.rect.center();
                                        Op::Rotate {
                                            before,
                                            id: s.id,
                                            start_angle: (start - c).angle(),
                                            orig: s.rotation,
                                        }
                                    }
                                    _ => Op::Resize { before, id: s.id, handle: h, orig: s.rect },
                                };
                            } else if let Some(hit) = doc.hit(start, tol) {
                                if !st.selected.contains(&hit) {
                                    if shift {
                                        st.selected.push(hit);
                                    } else {
                                        st.selected = vec![hit];
                                    }
                                }
                                st.op = Op::Move { before, grab: start, last: Vec2::ZERO, moved: false };
                            } else {
                                if !shift {
                                    st.selected.clear();
                                }
                                st.op = Op::Box { start, end: start };
                            }
                        }
                        Tool::Pen | Tool::Highlighter => {
                            let mut s =
                                WbShape::freehand(vec![InkPoint::new(start, now as f32)], st.tool == Tool::Highlighter);
                            s.stroke = if st.tool == Tool::Highlighter && st.style.color == INK {
                                Color32::from_rgb(250, 204, 21)
                            } else {
                                st.style.color
                            };
                            s.width =
                                if st.tool == Tool::Highlighter { st.style.width * 7.0 } else { st.style.width * 1.3 };
                            let id = doc.add(s);
                            st.op = Op::Draw { id, before };
                        }
                        Tool::Rect | Tool::Ellipse | Tool::Diamond | Tool::Arrow | Tool::Line => {
                            let kind = match st.tool {
                                Tool::Rect => ShapeKind::Rect,
                                Tool::Ellipse => ShapeKind::Ellipse,
                                Tool::Diamond => ShapeKind::Diamond,
                                Tool::Arrow => ShapeKind::Arrow,
                                _ => ShapeKind::Line,
                            };
                            let mut s = if kind.is_linear() {
                                WbShape::linear(kind, start, start)
                            } else {
                                WbShape::new(kind, Rect::from_min_size(start, Vec2::ZERO))
                            };
                            s.stroke = st.style.color;
                            s.width = st.style.width;
                            s.sketchy = st.style.sketchy;
                            if st.style.fill && !kind.is_linear() {
                                s.fill = Some(with_alpha(ink(st.style.color, &t), 0.22));
                            }
                            if kind.is_linear() {
                                s.bind_a = doc.bind_target(start, 0);
                            }
                            let id = doc.add(s);
                            st.selected = vec![id];
                            st.op = Op::Create { id, start, before };
                        }
                        Tool::Eraser => st.op = Op::Erase { before, hits: Vec::new(), trail: vec![(start, now)] },
                        _ => {}
                    }
                }
                // click: select / place text / sticky
                if bg.clicked() && !hand {
                    let p = tw(pointer.unwrap_or_default());
                    match st.tool {
                        Tool::Select => {
                            if handle_hit(pointer.unwrap_or_default()).is_none() {
                                match doc.hit(p, tol) {
                                    Some(h) if shift => {
                                        if let Some(k) = st.selected.iter().position(|s| *s == h) {
                                            st.selected.remove(k);
                                        } else {
                                            st.selected.push(h);
                                        }
                                    }
                                    Some(h) => st.selected = vec![h],
                                    None => st.selected.clear(),
                                }
                            }
                        }
                        Tool::Text | Tool::Sticky => {
                            st.history.record(doc);
                            let s = if st.tool == Tool::Text {
                                WbShape::text(p - vec2(4.0, 14.0), "", 22.0).stroke(st.style.color)
                            } else {
                                let k = doc.shapes.iter().filter(|s| s.kind == ShapeKind::Sticky).count();
                                WbShape::sticky(p - vec2(85.0, 75.0), "", STICKY_COLORS[k % STICKY_COLORS.len()])
                            };
                            let nid = doc.add(s);
                            st.selected = vec![nid];
                            st.editing = Some(nid);
                            st.focus_text = true;
                            st.tool = Tool::Select;
                            changed = true;
                        }
                        Tool::Eraser => {
                            if let Some(h) = doc.hit(p, tol) {
                                st.history.record(doc);
                                doc.remove(&[h]);
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                }
                if bg.double_clicked() && st.tool == Tool::Select {
                    if let Some(h) = pw.and_then(|p| doc.hit(p, tol)) {
                        if doc.get(h).is_some_and(|s| {
                            matches!(
                                s.kind,
                                ShapeKind::Text
                                    | ShapeKind::Sticky
                                    | ShapeKind::Rect
                                    | ShapeKind::Ellipse
                                    | ShapeKind::Diamond
                            )
                        }) {
                            st.history.record(doc);
                            st.editing = Some(h);
                            st.focus_text = true;
                            st.selected = vec![h];
                        }
                    } else if let Some(p) = pw {
                        // double-click empty space: new text
                        st.history.record(doc);
                        let nid = doc.add(WbShape::text(p - vec2(4.0, 14.0), "", 22.0).stroke(st.style.color));
                        st.selected = vec![nid];
                        st.editing = Some(nid);
                        st.focus_text = true;
                    }
                }

                // ---- continue operations
                let released = ui.input(|i| !i.pointer.primary_down());
                let mut done = false;
                let mut hover_bind: Option<u64> = None;
                match &mut st.op {
                    Op::None => {}
                    Op::Draw { id, before } => {
                        if let (Some(s), Some(p)) = (doc.get_mut(*id), pw) {
                            if s.points.last().is_none_or(|l| l.pos.distance(p) > 0.6 / zoom) {
                                s.points.push(InkPoint::new(p, now as f32));
                                s.recompute_bounds();
                            }
                        }
                        if released {
                            st.history.record(before);
                            changed = true;
                            done = true;
                        }
                    }
                    Op::Create { id, start, before } => {
                        if let (Some(p), Some(s)) = (pw, doc.get(*id).cloned()) {
                            if s.kind.is_linear() {
                                let mut end = p;
                                if shift {
                                    let d = p - *start;
                                    let a = (d.angle() / (std::f32::consts::PI / 12.0)).round()
                                        * (std::f32::consts::PI / 12.0);
                                    end = *start + Vec2::angled(a) * d.length();
                                }
                                hover_bind = doc.bind_target(end, *id).filter(|b| Some(*b) != s.bind_a);
                                if let Some(s) = doc.get_mut(*id) {
                                    s.b = end;
                                    s.bind_b = hover_bind;
                                    s.recompute_bounds();
                                }
                            } else if let Some(s) = doc.get_mut(*id) {
                                let mut d = p - *start;
                                if shift {
                                    let m = d.x.abs().max(d.y.abs());
                                    d = vec2(m * d.x.signum(), m * d.y.signum());
                                }
                                s.rect = Rect::from_two_pos(*start, *start + d);
                            }
                        }
                        if released {
                            if let Some(s) = doc.get_mut(*id) {
                                let tiny = if s.kind.is_linear() {
                                    s.a.distance(s.b) < 4.0 / zoom
                                } else {
                                    s.rect.width() < 4.0 / zoom && s.rect.height() < 4.0 / zoom
                                };
                                if tiny {
                                    if s.kind.is_linear() {
                                        s.b = s.a + vec2(140.0, 0.0);
                                    } else {
                                        s.rect = Rect::from_center_size(s.rect.center(), vec2(140.0, 90.0));
                                    }
                                    s.recompute_bounds();
                                }
                            }
                            st.history.record(before);
                            st.tool = Tool::Select;
                            changed = true;
                            done = true;
                        }
                    }
                    Op::Move { before, grab, last, moved } => {
                        if let Some(p) = pw {
                            let d = p - *grab;
                            let step = d - *last;
                            if step != Vec2::ZERO {
                                for sid in st.selected.clone() {
                                    if let Some(s) = doc.get_mut(sid) {
                                        let bound = s.kind.is_linear() && (s.bind_a.is_some() || s.bind_b.is_some());
                                        s.translate(step);
                                        if bound {
                                            // keep bindings only if the bound shapes move too
                                            if s.bind_a.is_some_and(|b| !st.selected.contains(&b)) {
                                                s.bind_a = None;
                                            }
                                            if s.bind_b.is_some_and(|b| !st.selected.contains(&b)) {
                                                s.bind_b = None;
                                            }
                                        }
                                    }
                                }
                                *last = d;
                                *moved = true;
                                changed = true;
                            }
                        }
                        ui.ctx().set_cursor_icon(CursorIcon::Move);
                        if released {
                            if *moved {
                                st.history.record(before);
                            }
                            done = true;
                        }
                    }
                    Op::Resize { before, id, handle, orig } => {
                        if let (Some(p), Some(s)) = (pw, doc.get(*id).cloned()) {
                            let c = orig.center();
                            let lp = rotate_about(p, c, -s.rotation);
                            let anchor = handle_anchor(*orig, *handle);
                            let (mut min, mut max) = (anchor, lp);
                            match handle {
                                1 | 5 => {
                                    min.x = orig.left();
                                    max.x = orig.right();
                                }
                                3 | 7 => {
                                    min.y = orig.top();
                                    max.y = orig.bottom();
                                }
                                _ => {}
                            }
                            let mut nr = Rect::from_two_pos(min, max);
                            if shift && orig.height() > 0.0 && matches!(handle, 0 | 2 | 4 | 6) {
                                let ar = orig.width() / orig.height();
                                let w = nr.width().max(nr.height() * ar);
                                let h = w / ar;
                                nr = Rect::from_two_pos(
                                    anchor,
                                    anchor + vec2(w * (lp.x - anchor.x).signum(), h * (lp.y - anchor.y).signum()),
                                );
                            }
                            let nr = Rect::from_min_size(nr.min, nr.size().max(Vec2::splat(4.0)));
                            let world_c = c + (rotate_about(nr.center(), c, s.rotation) - c);
                            let final_r = Rect::from_center_size(world_c, nr.size());
                            if let Some(s) = doc.get_mut(*id) {
                                s.set_rect(final_r);
                            }
                            changed = true;
                        }
                        if released {
                            st.history.record(before);
                            done = true;
                        }
                    }
                    Op::Rotate { before, id, start_angle, orig } => {
                        if let (Some(p), Some(s)) = (pw, doc.get_mut(*id)) {
                            let c = s.rect.center();
                            let mut a = *orig + (p - c).angle() - *start_angle;
                            if shift {
                                let step = std::f32::consts::PI / 12.0;
                                a = (a / step).round() * step;
                            }
                            s.rotation = a;
                            changed = true;
                        }
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                        if released {
                            st.history.record(before);
                            done = true;
                        }
                    }
                    Op::Endpoint { before, id, end_b } => {
                        if let Some(p) = pw {
                            hover_bind = doc.bind_target(p, *id);
                            if let Some(s) = doc.get_mut(*id) {
                                if *end_b {
                                    s.b = p;
                                    s.bind_b = hover_bind;
                                } else {
                                    s.a = p;
                                    s.bind_a = hover_bind;
                                }
                                s.recompute_bounds();
                            }
                            changed = true;
                        }
                        if released {
                            st.history.record(before);
                            done = true;
                        }
                    }
                    Op::Box { start, end } => {
                        if let Some(p) = pw {
                            *end = p;
                        }
                        let r = Rect::from_two_pos(*start, *end);
                        st.selected = doc
                            .shapes
                            .iter()
                            .filter(|s| r.contains_rect(doc.bounds_of(&[s.id])))
                            .map(|s| s.id)
                            .collect();
                        if released {
                            done = true;
                        }
                    }
                    Op::Erase { hits, trail, .. } => {
                        if let Some(p) = pw {
                            if trail.last().is_none_or(|l| l.0.distance(p) > 0.5 / zoom) {
                                trail.push((p, now));
                            }
                            if let Some(h) = doc.hit(p, 8.0 / zoom) {
                                if !hits.contains(&h) {
                                    hits.push(h);
                                }
                            }
                        }
                        if released {
                            if let Op::Erase { before, hits, .. } = std::mem::take(&mut st.op) {
                                if !hits.is_empty() {
                                    st.history.record(&before);
                                    doc.remove(&hits);
                                    st.selected.retain(|s| !hits.contains(s));
                                    changed = true;
                                }
                            }
                            done = true;
                        }
                    }
                }
                if done {
                    st.op = Op::None;
                }

                // ---- keyboard
                let keys = ui.rect_contains_pointer(rect) && !typing && st.editing.is_none();
                if keys {
                    use egui::Key;
                    for tool in Tool::ALL {
                        if common::pressed(ui, tool.key()) && !ui.input(|i| i.modifiers.shift) {
                            st.tool = tool;
                        }
                    }
                    let (copy, cut, paste) = common::clipboard_events(ui);
                    if (common::pressed(ui, Key::Delete) || common::pressed(ui, Key::Backspace) || cut)
                        && !st.selected.is_empty()
                    {
                        if cut {
                            st.clipboard = doc.shapes.iter().filter(|s| st.selected.contains(&s.id)).cloned().collect();
                        }
                        st.history.record(doc);
                        doc.remove(&st.selected);
                        st.selected.clear();
                        changed = true;
                    }
                    if copy && !st.selected.is_empty() {
                        st.clipboard = doc.shapes.iter().filter(|s| st.selected.contains(&s.id)).cloned().collect();
                        st.toast = Some((
                            format!(
                                "Copied {} shape{}",
                                st.clipboard.len(),
                                if st.clipboard.len() == 1 { "" } else { "s" }
                            ),
                            now,
                        ));
                    }
                    let dup = common::pressed_cmd(ui, Key::D, false);
                    if (paste && !st.clipboard.is_empty()) || (dup && !st.selected.is_empty()) {
                        st.history.record(doc);
                        let src: Vec<WbShape> = if dup {
                            doc.shapes.iter().filter(|s| st.selected.contains(&s.id)).cloned().collect()
                        } else {
                            st.clipboard.clone()
                        };
                        st.selected.clear();
                        for mut s in src {
                            s.translate(vec2(24.0, 24.0));
                            s.bind_a = None;
                            s.bind_b = None;
                            s.seed = s.seed.wrapping_add(17);
                            let nid = doc.add(s);
                            st.selected.push(nid);
                        }
                        if !dup {
                            for s in &mut st.clipboard {
                                s.translate(vec2(24.0, 24.0));
                            }
                        }
                        changed = true;
                    }
                    if common::pressed_cmd(ui, Key::Z, false) && st.history.undo(doc) {
                        st.selected.retain(|s| doc.get(*s).is_some());
                        changed = true;
                    }
                    if (common::pressed_cmd(ui, Key::Z, true) || common::pressed_cmd(ui, Key::Y, false))
                        && st.history.redo(doc)
                    {
                        st.selected.retain(|s| doc.get(*s).is_some());
                        changed = true;
                    }
                    if common::pressed_cmd(ui, Key::A, false) {
                        st.selected = doc.shapes.iter().map(|s| s.id).collect();
                    }
                    if common::pressed(ui, Key::Escape) {
                        st.selected.clear();
                        st.tool = Tool::Select;
                        st.help = false;
                    }
                    if ui.input(|i| i.key_pressed(Key::OpenBracket)) && !st.selected.is_empty() {
                        st.history.record(doc);
                        doc.step(&st.selected, -1);
                        changed = true;
                    }
                    if ui.input(|i| i.key_pressed(Key::CloseBracket)) && !st.selected.is_empty() {
                        st.history.record(doc);
                        doc.step(&st.selected, 1);
                        changed = true;
                    }
                    if ui.input(|i| i.key_pressed(Key::Num1) && i.modifiers.shift) || common::pressed(ui, Key::F) {
                        cv.view.fit(rect, doc.bounds(), 60.0);
                    }
                    if common::help_pressed(ui) {
                        st.help = !st.help;
                    }
                }

                // ---- paint document
                let erasing: Vec<u64> = match &st.op {
                    Op::Erase { hits, .. } => hits.clone(),
                    _ => Vec::new(),
                };
                let tsd: &dyn Fn(Pos2) -> Pos2 = &ts;
                for s in &doc.shapes {
                    let sb = ts(s.bounds().min);
                    let se = ts(s.bounds().max);
                    if !Rect::from_min_max(sb, se).expand(40.0).intersects(rect) && !s.kind.is_linear() {
                        continue;
                    }
                    if st.editing == Some(s.id) && matches!(s.kind, ShapeKind::Text) {
                        continue;
                    }
                    let mut p = painter.clone();
                    if erasing.contains(&s.id) {
                        p.set_opacity(0.25);
                    }
                    if st.editing == Some(s.id) {
                        let mut c = s.clone();
                        c.text.clear();
                        paint_wb_shape(&p, &t, doc, &c, tsd, zoom);
                    } else {
                        paint_wb_shape(&p, &t, doc, s, tsd, zoom);
                    }
                }
                // hovered shape outline (select tool)
                let hovered = if st.tool == Tool::Select
                    && matches!(st.op, Op::None)
                    && pointer.is_some_and(|p| rect.contains(p) && !toolbar_rect.contains(p) && !style_rect.contains(p))
                {
                    pw.and_then(|p| doc.hit(p, tol))
                } else {
                    None
                };
                if let Some(h) = hovered.filter(|h| !st.selected.contains(h)) {
                    if let Some(s) = doc.get(h) {
                        outline_shape(&painter, doc, s, &ts, Stroke::new(1.5, with_alpha(t.accent, 0.55)));
                    }
                }
                if let Some(b) = hover_bind {
                    if let Some(s) = doc.get(b) {
                        let pts: Vec<Pos2> = s.outline().into_iter().map(ts).collect();
                        painter.add(Shape::closed_line(pts, Stroke::new(2.5, with_alpha(t.accent, 0.8))));
                    }
                }
                // selection chrome
                let mut handle_hover: Option<usize> = None;
                if !st.selected.is_empty() && st.editing.is_none() {
                    if let Some(s) = (st.selected.len() == 1).then(|| doc.get(st.selected[0])).flatten() {
                        if s.kind.is_linear() {
                            let (a, b) = doc.arrow_ends(s);
                            for (k, p) in [ts(a), ts(b)].into_iter().enumerate() {
                                let hov = pointer.is_some_and(|q| q.distance(p) < 9.0);
                                if hov {
                                    handle_hover = Some(100 + k);
                                }
                                painter.circle(
                                    p,
                                    if hov { 6.0 } else { 5.0 },
                                    t.background,
                                    Stroke::new(1.5, t.accent),
                                );
                            }
                        } else {
                            let corners: Vec<Pos2> =
                                [s.rect.left_top(), s.rect.right_top(), s.rect.right_bottom(), s.rect.left_bottom()]
                                    .iter()
                                    .map(|p| ts(rotate_about(*p, s.rect.center(), s.rotation)))
                                    .collect();
                            let pad_corners = expand_quad(&corners, 4.0);
                            painter.add(Shape::closed_line(pad_corners, Stroke::new(1.5, t.accent)));
                            let hp = handle_points(s, &ts);
                            // rotate stem
                            let top_mid =
                                ts(rotate_about(pos2(s.rect.center().x, s.rect.top()), s.rect.center(), s.rotation));
                            painter.line_segment([top_mid, hp[8]], Stroke::new(1.0, with_alpha(t.accent, 0.6)));
                            for (k, p) in hp.iter().enumerate() {
                                let hov = pointer.is_some_and(|q| q.distance(*p) < 8.0);
                                if hov {
                                    handle_hover = Some(k);
                                }
                                if k == 8 {
                                    painter.circle(
                                        *p,
                                        if hov { 6.5 } else { 5.0 },
                                        if hov { t.accent } else { t.background },
                                        Stroke::new(1.5, t.accent),
                                    );
                                } else if !(s.kind == ShapeKind::Text && k % 2 == 1) {
                                    let r = Rect::from_center_size(*p, Vec2::splat(if hov { 10.0 } else { 8.0 }));
                                    painter.rect(
                                        r,
                                        CornerRadius::same(2),
                                        if hov { t.accent } else { t.background },
                                        Stroke::new(1.5, t.accent),
                                        StrokeKind::Middle,
                                    );
                                }
                            }
                        }
                    } else {
                        for sid in &st.selected {
                            if let Some(s) = doc.get(*sid) {
                                outline_shape(&painter, doc, s, &ts, Stroke::new(1.0, with_alpha(t.accent, 0.7)));
                            }
                        }
                        let b = doc.bounds_of(&st.selected);
                        if b.is_positive() {
                            let r = Rect::from_min_max(ts(b.min), ts(b.max)).expand(6.0);
                            common::dashed_polyline(
                                &painter,
                                &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()],
                                5.0,
                                4.0,
                                0.0,
                                Stroke::new(1.2, t.accent),
                            );
                        }
                    }
                }
                if let Op::Box { start, end } = &st.op {
                    let r = Rect::from_two_pos(ts(*start), ts(*end));
                    painter.rect(
                        r,
                        CornerRadius::same(2),
                        with_alpha(t.accent, 0.08),
                        Stroke::new(1.0, t.accent),
                        StrokeKind::Inside,
                    );
                }
                if let Op::Erase { trail, .. } = &st.op {
                    let pts: Vec<(Pos2, f32)> = trail
                        .iter()
                        .filter(|(_, tt)| now - tt < 0.25)
                        .map(|(p, tt)| (ts(*p), (1.0 - ((now - tt) / 0.25) as f32) * 4.0))
                        .collect();
                    crate::freehand::paint_ribbon(&painter, &pts, with_alpha(t.muted_foreground, 0.55));
                    ui.ctx().request_repaint();
                }

                // ---- cursors & tool previews
                if let Some(p) =
                    pointer.filter(|p| rect.contains(*p) && !toolbar_rect.contains(*p) && !style_rect.contains(*p))
                {
                    if !cv.space_pan {
                        match st.tool {
                            Tool::Select => {
                                if let Some(h) = handle_hover.or_else(|| handle_hit(p)) {
                                    let rot = single.as_ref().map(|s| s.rotation).unwrap_or(0.0);
                                    ui.ctx().set_cursor_icon(handle_cursor(h, rot));
                                } else if hovered.is_some()
                                    || st.selected.iter().any(|s| doc.get(*s).is_some_and(|s| s.hit(tw(p), tol)))
                                {
                                    ui.ctx().set_cursor_icon(CursorIcon::Move);
                                }
                            }
                            Tool::Hand => {}
                            Tool::Text => ui.ctx().set_cursor_icon(CursorIcon::Text),
                            Tool::Eraser => {
                                ui.ctx().set_cursor_icon(CursorIcon::None);
                                painter.circle(
                                    p,
                                    7.0,
                                    with_alpha(t.background, 0.6),
                                    Stroke::new(1.5, t.muted_foreground),
                                );
                            }
                            Tool::Pen | Tool::Highlighter => {
                                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                                let c = if st.tool == Tool::Highlighter && st.style.color == INK {
                                    Color32::from_rgb(250, 204, 21)
                                } else {
                                    ink(st.style.color, &t)
                                };
                                let r = if st.tool == Tool::Highlighter {
                                    st.style.width * 3.5
                                } else {
                                    st.style.width * 1.3
                                } * zoom;
                                painter.circle_filled(p, r.max(1.5), with_alpha(c, 0.6));
                            }
                            _ => ui.ctx().set_cursor_icon(CursorIcon::Crosshair),
                        }
                    }
                }

                // ---- text editing overlay
                if let Some(eid) = st.editing {
                    if let Some(s) = doc.get(eid).cloned() {
                        let (font_size, color, area) = match s.kind {
                            ShapeKind::Sticky => (
                                s.font_size,
                                Color32::from_rgb(40, 36, 30),
                                Rect::from_min_size(
                                    ts(s.rect.min + vec2(10.0, 10.0)),
                                    (s.rect.size() - vec2(20.0, 20.0)) * zoom,
                                ),
                            ),
                            ShapeKind::Text => (
                                s.font_size,
                                ink(s.stroke, &t),
                                Rect::from_min_size(
                                    ts(s.rect.min),
                                    vec2(s.rect.width().max(160.0), s.rect.height()) * zoom,
                                ),
                            ),
                            _ => (
                                s.font_size * 0.8,
                                ink(s.stroke, &t),
                                Rect::from_center_size(
                                    ts(s.rect.center()),
                                    vec2(s.rect.width() * 0.8, s.font_size * 1.6) * zoom,
                                ),
                            ),
                        };
                        let mut text = s.text.clone();
                        let te = TextEdit::multiline(&mut text)
                            .font(qfont(font_size * zoom))
                            .text_color(color)
                            .frame(egui::Frame::NONE)
                            .background_color(Color32::TRANSPARENT)
                            .align(if s.kind == ShapeKind::Text || s.kind == ShapeKind::Sticky {
                                Align2::LEFT_TOP
                            } else {
                                Align2::CENTER_TOP
                            })
                            .desired_width(area.width())
                            .hint_text("Type…");
                        let r = ui.put(area, te);
                        if st.focus_text {
                            r.request_focus();
                            st.focus_text = false;
                        }
                        if text != s.text {
                            if let Some(sm) = doc.get_mut(eid) {
                                sm.text = text;
                                if sm.kind == ShapeKind::Text {
                                    let g = painter.layout(
                                        sm.text.clone(),
                                        egui::FontId::proportional(sm.font_size),
                                        Color32::WHITE,
                                        f32::INFINITY,
                                    );
                                    sm.rect = Rect::from_min_size(
                                        sm.rect.min,
                                        vec2(g.size().x.max(40.0) + 8.0, g.size().y.max(sm.font_size * 1.3)),
                                    );
                                }
                            }
                            changed = true;
                        }
                        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        let click_away = ui.input(|i| i.pointer.any_pressed())
                            && pointer.is_some_and(|p| !area.expand(6.0).contains(p));
                        if r.lost_focus() || escape || click_away {
                            st.editing = None;
                            if doc.get(eid).is_some_and(|s| s.kind == ShapeKind::Text && s.text.trim().is_empty()) {
                                doc.remove(&[eid]);
                                st.selected.clear();
                            }
                        }
                    } else {
                        st.editing = None;
                    }
                }

                // ---- toolbar
                panel(&painter, &t, toolbar_rect, 12.0);
                for (k, tool) in Tool::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(toolbar_rect.min + vec2(5.0 + k as f32 * 34.0, 5.0), vec2(32.0, 32.0));
                    let tip = format!("{} — {:?}", tool.name(), tool.key());
                    if icon_button(ui, id.with(("tool", k)), r, tool.icon(), st.tool == *tool, &tip).clicked() {
                        st.tool = *tool;
                        if *tool != Tool::Select {
                            st.editing = None;
                        }
                    }
                    if k == 1 || k == 3 || k == 8 {
                        let x = r.right() + 1.0;
                        painter.line_segment(
                            [pos2(x, r.top() + 7.0), pos2(x, r.bottom() - 7.0)],
                            Stroke::new(1.0, t.border),
                        );
                    }
                    // tiny shortcut letter
                    painter.text(
                        r.right_bottom() + vec2(-3.0, -2.0),
                        Align2::RIGHT_BOTTOM,
                        format!("{:?}", tool.key()),
                        egui::FontId::proportional(7.5),
                        with_alpha(t.muted_foreground, 0.7),
                    );
                }

                // ---- style panel
                panel(&painter, &t, style_rect, 12.0);
                let mut y = style_rect.top() + 12.0;
                painter.text(
                    pos2(style_rect.left() + 12.0, y + 4.0),
                    Align2::LEFT_CENTER,
                    "Stroke",
                    qfont(10.5),
                    t.muted_foreground,
                );
                y += 14.0;
                let sel_ids = st.selected.clone();
                let mut apply: Option<ShapeEdit> = None;
                for (k, c) in WB_COLORS.iter().enumerate() {
                    let col = k % 4;
                    let row = k / 4;
                    let r = Rect::from_min_size(
                        pos2(style_rect.left() + 12.0 + col as f32 * 39.0, y + row as f32 * 32.0),
                        vec2(30.0, 26.0),
                    );
                    let resp = ui.interact(r, id.with(("col", k)), Sense::click());
                    let active = st.style.color == *c;
                    let h = ui.ctx().animate_bool_with_time(id.with(("colh", k)), resp.hovered(), 0.1);
                    if active {
                        painter.rect_stroke(
                            r.expand(1.0),
                            CornerRadius::same(8),
                            Stroke::new(1.5, t.accent),
                            StrokeKind::Outside,
                        );
                    }
                    painter.circle(
                        r.center(),
                        8.5 + h * 1.5,
                        ink(*c, &t),
                        Stroke::new(1.0, with_alpha(t.foreground, 0.15)),
                    );
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        st.style.color = *c;
                        let c = *c;
                        apply = Some(Box::new(move |s: &mut WbShape| {
                            if s.kind != ShapeKind::Sticky {
                                s.stroke = c;
                                if s.fill.is_some() {
                                    s.fill = Some(with_alpha(if c == INK { Color32::GRAY } else { c }, 0.22));
                                }
                            }
                        }));
                    }
                }
                y += 70.0;
                painter.text(
                    pos2(style_rect.left() + 12.0, y + 4.0),
                    Align2::LEFT_CENTER,
                    "Width",
                    qfont(10.5),
                    t.muted_foreground,
                );
                y += 14.0;
                for (k, w) in [1.0f32, 2.0, 4.0].iter().enumerate() {
                    let r = Rect::from_min_size(pos2(style_rect.left() + 12.0 + k as f32 * 52.0, y), vec2(46.0, 28.0));
                    let resp = ui.interact(r, id.with(("w", k)), Sense::click());
                    let active = (st.style.width - w).abs() < 0.01;
                    let h = ui.ctx().animate_bool_with_time(id.with(("wh", k)), resp.hovered(), 0.1);
                    painter.rect_filled(
                        r,
                        CornerRadius::same(7),
                        if active { with_alpha(t.accent, 0.18) } else { with_alpha(t.foreground, 0.04 + 0.05 * h) },
                    );
                    painter.line_segment(
                        [r.left_center() + vec2(10.0, 0.0), r.right_center() - vec2(10.0, 0.0)],
                        Stroke::new(*w * 1.3, if active { t.accent } else { t.foreground }),
                    );
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        st.style.width = *w;
                        let w = *w;
                        apply = Some(Box::new(move |s: &mut WbShape| match s.kind {
                            ShapeKind::Highlight => s.width = w * 7.0,
                            ShapeKind::Freehand => s.width = w * 1.3,
                            _ => s.width = w,
                        }));
                    }
                }
                y += 40.0;
                // toggles: fill, sketchy
                let toggles = [(Icon::Fill, "Fill", st.style.fill), (Icon::Sketch, "Sketchy", st.style.sketchy)];
                for (k, (icon, label, on)) in toggles.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(style_rect.left() + 12.0 + k as f32 * 78.0, y), vec2(74.0, 28.0));
                    let resp = ui.interact(r, id.with(("tg", k)), Sense::click());
                    let h = ui.ctx().animate_bool_with_time(id.with(("tgh", k)), resp.hovered(), 0.1);
                    painter.rect_filled(
                        r,
                        CornerRadius::same(7),
                        if *on { with_alpha(t.accent, 0.18) } else { with_alpha(t.foreground, 0.04 + 0.05 * h) },
                    );
                    paint_icon(
                        &painter,
                        Rect::from_center_size(pos2(r.left() + 15.0, r.center().y), Vec2::splat(14.0)),
                        *icon,
                        if *on { t.accent } else { t.muted_foreground },
                    );
                    painter.text(
                        pos2(r.left() + 27.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        qfont(11.5),
                        if *on { t.foreground } else { t.muted_foreground },
                    );
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        if k == 0 {
                            st.style.fill = !st.style.fill;
                            let (f, c) = (st.style.fill, ink(st.style.color, &t));
                            apply = Some(Box::new(move |s: &mut WbShape| {
                                if matches!(s.kind, ShapeKind::Rect | ShapeKind::Ellipse | ShapeKind::Diamond) {
                                    s.fill = f.then(|| with_alpha(if s.stroke == INK { c } else { s.stroke }, 0.22));
                                }
                            }));
                        } else {
                            st.style.sketchy = !st.style.sketchy;
                            let v = st.style.sketchy;
                            apply = Some(Box::new(move |s: &mut WbShape| s.sketchy = v));
                        }
                    }
                }
                y += 40.0;
                painter.text(
                    pos2(style_rect.left() + 12.0, y + 4.0),
                    Align2::LEFT_CENTER,
                    "Layers",
                    qfont(10.5),
                    t.muted_foreground,
                );
                y += 14.0;
                let mut layer_act = None;
                for (k, (icon, tip)) in [
                    (Icon::Down, "Send to back"),
                    (Icon::ChevronDown, "Send backward  ["),
                    (Icon::Up, "Bring forward  ]"),
                    (Icon::Up, "Bring to front"),
                ]
                .iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(pos2(style_rect.left() + 10.0 + k as f32 * 39.0, y), vec2(34.0, 30.0));
                    if k == 0 || k == 3 {
                        // double chevrons for "to back / to front"
                        let resp = icon_button(ui, id.with(("ly", k)), r, *icon, false, tip);
                        let off = if k == 0 { 4.0 } else { -4.0 };
                        paint_icon(&painter, r.shrink(8.0).translate(vec2(0.0, off)), *icon, t.muted_foreground);
                        if resp.clicked() {
                            layer_act = Some(k);
                        }
                    } else if icon_button(ui, id.with(("ly", k)), r, *icon, false, tip).clicked() {
                        layer_act = Some(k);
                    }
                }
                if let (Some(k), false) = (layer_act, sel_ids.is_empty()) {
                    st.history.record(doc);
                    match k {
                        0 => doc.reorder(&sel_ids, false),
                        1 => doc.step(&sel_ids, -1),
                        2 => doc.step(&sel_ids, 1),
                        _ => doc.reorder(&sel_ids, true),
                    }
                    changed = true;
                }
                if let Some(f) = apply {
                    if !sel_ids.is_empty() {
                        st.history.record(doc);
                        for sid in &sel_ids {
                            if let Some(s) = doc.get_mut(*sid) {
                                f(s);
                            }
                        }
                        changed = true;
                    }
                }

                // ---- top-right actions
                let actions = [
                    (Icon::Undo, "Undo  Ctrl+Z"),
                    (Icon::Redo, "Redo  Ctrl+Shift+Z"),
                    (Icon::Export, "Export JSON to clipboard"),
                    (Icon::Import, "Import last export"),
                    (Icon::Help, "Shortcuts  ?"),
                ];
                let ar = Rect::from_min_size(
                    pos2(rect.right() - 12.0 - (actions.len() as f32 * 32.0 + 8.0), rect.top() + 12.0),
                    vec2(actions.len() as f32 * 32.0 + 8.0, 38.0),
                );
                panel(&painter, &t, ar, 10.0);
                for (k, (icon, tip)) in actions.iter().enumerate() {
                    let r = Rect::from_min_size(ar.min + vec2(4.0 + k as f32 * 32.0, 4.0), Vec2::splat(30.0));
                    let enabled = match k {
                        0 => st.history.can_undo(),
                        1 => st.history.can_redo(),
                        3 => !st.exported.is_empty(),
                        _ => true,
                    };
                    let mut bp = painter.clone();
                    if !enabled {
                        bp.set_opacity(0.4);
                    }
                    if icon_button(ui, id.with(("act", k)), r, *icon, k == 4 && st.help, tip).clicked() && enabled {
                        match k {
                            0 => {
                                st.history.undo(doc);
                                changed = true;
                            }
                            1 => {
                                st.history.redo(doc);
                                changed = true;
                            }
                            2 => {
                                st.exported = doc.to_json();
                                ui.ctx().copy_text(st.exported.clone());
                                st.toast = Some((
                                    format!(
                                        "Exported {} shapes · {:.1} KB JSON copied",
                                        doc.shapes.len(),
                                        st.exported.len() as f32 / 1024.0
                                    ),
                                    now,
                                ));
                            }
                            3 => match Document::from_json(&st.exported) {
                                Ok(d) => {
                                    st.history.record(doc);
                                    *doc = d;
                                    st.selected.clear();
                                    st.toast = Some(("Imported document".into(), now));
                                    changed = true;
                                }
                                Err(e) => st.toast = Some((format!("Import failed: {e}"), now)),
                            },
                            _ => st.help = !st.help,
                        }
                    }
                }
                // zoom readout + status
                let status = format!("{:.0}%   ·   {} shapes", cv.view.zoom * 100.0, doc.shapes.len());
                painter.text(
                    pos2(rect.right() - 14.0, rect.bottom() - 14.0),
                    Align2::RIGHT_BOTTOM,
                    status,
                    qfont(11.0),
                    t.muted_foreground,
                );

                if let Some((msg, t0)) = &st.toast {
                    let age = (now - t0) as f32;
                    if age < 2.0 {
                        let a = (age / 0.15).min(1.0) * ((2.0 - age) / 0.3).min(1.0);
                        let mut tp = painter.clone();
                        tp.set_opacity(a);
                        common::pill(
                            &tp,
                            pos2(rect.center().x, rect.bottom() - 30.0),
                            msg,
                            qfont(12.0),
                            t.foreground,
                            t.popover,
                            Stroke::new(1.0, t.border),
                        );
                        ui.ctx().request_repaint();
                    } else {
                        st.toast = None;
                    }
                }
                let k = ui.ctx().animate_bool_with_time(id.with("help"), st.help, 0.18);
                common::shortcut_overlay(
                    &painter,
                    &t,
                    rect,
                    "Whiteboard shortcuts",
                    &[
                        ("V", "select"),
                        ("H", "hand / pan"),
                        ("P", "pen"),
                        ("M", "highlighter"),
                        ("R", "rectangle"),
                        ("O", "ellipse"),
                        ("D", "diamond"),
                        ("A", "arrow (binds to shapes)"),
                        ("L", "line"),
                        ("T", "text"),
                        ("S", "sticky note"),
                        ("E", "eraser"),
                        ("Shift+Drag", "constrain / snap angle"),
                        ("Ctrl+D", "duplicate"),
                        ("[ ]", "layer back / forward"),
                        ("Ctrl+Z", "undo"),
                        ("Del", "delete"),
                        ("F", "zoom to fit"),
                    ],
                    k,
                );
            });
        st.view = view;
        changed
    }
}

fn expand_quad(c: &[Pos2], d: f32) -> Vec<Pos2> {
    let center = c.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / c.len() as f32;
    let _ = center;
    let ux = (c[1] - c[0]).normalized();
    let uy = (c[3] - c[0]).normalized();
    vec![c[0] - ux * d - uy * d, c[1] + ux * d - uy * d, c[2] + ux * d + uy * d, c[3] - ux * d + uy * d]
}

fn outline_shape(painter: &Painter, doc: &Document, s: &WbShape, ts: &dyn Fn(Pos2) -> Pos2, stroke: Stroke) {
    match s.kind {
        k if k.is_linear() => {
            let (a, b) = doc.arrow_ends(s);
            painter.line_segment([ts(a), ts(b)], Stroke::new(stroke.width + s.width, with_alpha(stroke.color, 0.35)));
        }
        ShapeKind::Freehand | ShapeKind::Highlight => {
            let pts: Vec<Pos2> =
                s.points.iter().map(|p| ts(rotate_about(p.pos, s.rect.center(), s.rotation))).collect();
            painter.add(Shape::line(pts, Stroke::new(stroke.width + s.width, with_alpha(stroke.color, 0.3))));
        }
        _ => {
            painter.add(Shape::closed_line(s.outline().into_iter().map(ts).collect(), stroke));
        }
    }
}

/// Handle positions (screen): 0..8 = nw, n, ne, e, se, s, sw, w; 8 = rotate.
fn handle_points(s: &WbShape, ts: &dyn Fn(Pos2) -> Pos2) -> Vec<Pos2> {
    let r = s.rect;
    let c = r.center();
    let local = [
        r.left_top(),
        pos2(c.x, r.top()),
        r.right_top(),
        pos2(r.right(), c.y),
        r.right_bottom(),
        pos2(c.x, r.bottom()),
        r.left_bottom(),
        pos2(r.left(), c.y),
    ];
    let mut v: Vec<Pos2> = local.iter().map(|p| ts(rotate_about(*p, c, s.rotation))).collect();
    // padding so handles sit on the padded selection box
    let sc = ts(c);
    for p in v.iter_mut() {
        let d = (*p - sc).normalized();
        if d.is_finite() {
            *p += d * 4.0;
        }
    }
    let up = rotate_about(pos2(0.0, -1.0), Pos2::ZERO, s.rotation).to_vec2();
    v.push(v[1] + up * 22.0);
    v
}

/// The fixed corner/edge when dragging handle `h`.
fn handle_anchor(r: Rect, h: usize) -> Pos2 {
    match h {
        0 => r.right_bottom(),
        1 => pos2(r.left(), r.bottom()),
        2 => r.left_bottom(),
        3 => pos2(r.left(), r.top()),
        4 => r.left_top(),
        5 => pos2(r.left(), r.top()),
        6 => r.right_top(),
        _ => pos2(r.right(), r.top()),
    }
}

fn handle_cursor(h: usize, rotation: f32) -> CursorIcon {
    if h >= 100 {
        return CursorIcon::Crosshair;
    }
    if h == 8 {
        return CursorIcon::Grab;
    }
    let steps = ((rotation / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(4) as usize;
    let base =
        [CursorIcon::ResizeNwSe, CursorIcon::ResizeVertical, CursorIcon::ResizeNeSw, CursorIcon::ResizeHorizontal];
    base[(h % 4 + steps) % 4]
}

/// Helper used by demos: a smooth synthetic pen stroke through `pts` (world) with natural timing.
pub fn synthetic_stroke(pts: &[Pos2]) -> Vec<InkPoint> {
    let dense = paint::catmull_rom(pts, 10);
    let mut t = 0.0;
    let mut out = Vec::with_capacity(dense.len());
    for (i, p) in dense.iter().enumerate() {
        if i > 0 {
            let d = p.distance(dense[i - 1]);
            // slow at the ends, faster in the middle
            let k = i as f32 / dense.len() as f32;
            let speed = 200.0 + 900.0 * (k * std::f32::consts::PI).sin();
            t += d / speed;
        }
        out.push(InkPoint::new(*p, t));
    }
    out
}

/// Silence an unused warning for `Id` in some cfgs.
#[allow(dead_code)]
fn _id(_: Id) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Document {
        let mut d = Document::default();
        let a = d.add(
            WbShape::new(ShapeKind::Rect, Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 60.0)))
                .fill(Color32::RED)
                .label("Box \"A\""),
        );
        let b = d.add(
            WbShape::new(ShapeKind::Ellipse, Rect::from_min_size(pos2(300.0, 0.0), vec2(100.0, 60.0))).sketchy(true),
        );
        let mut arrow = WbShape::linear(ShapeKind::Arrow, pos2(50.0, 30.0), pos2(350.0, 30.0));
        arrow.bind_a = Some(a);
        arrow.bind_b = Some(b);
        d.add(arrow);
        d.add(WbShape::freehand(synthetic_stroke(&[pos2(0.0, 200.0), pos2(50.0, 220.0), pos2(100.0, 190.0)]), false));
        d.add(WbShape::sticky(pos2(500.0, 0.0), "Line one\nline two", STICKY_COLORS[1]));
        d
    }

    #[test]
    fn json_roundtrip() {
        let d = sample();
        let s = d.to_json();
        let back = Document::from_json(&s).expect("parse");
        assert_eq!(back.shapes.len(), d.shapes.len());
        for (x, y) in d.shapes.iter().zip(back.shapes.iter()) {
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.text, y.text);
            assert_eq!(x.bind_a, y.bind_a);
            assert!((x.rect.min - y.rect.min).length() < 0.02);
            assert_eq!(x.points.len(), y.points.len());
            assert_eq!(x.fill.is_some(), y.fill.is_some());
        }
        assert!(Document::from_json("{\"shapes\":[{\"kind\":\"blob\"}]}").is_err());
    }

    #[test]
    fn arrow_binds_to_outline() {
        let d = sample();
        let arrow = d.shapes.iter().find(|s| s.kind == ShapeKind::Arrow).unwrap();
        let (a, b) = d.arrow_ends(arrow);
        // start sits just outside the right edge of the rect, end just outside the ellipse
        assert!((a.x - 108.0).abs() < 0.5, "{a:?}");
        assert!((b.x - 292.0).abs() < 0.5, "{b:?}");
    }

    #[test]
    fn hit_testing() {
        let d = sample();
        assert_eq!(d.hit(pos2(50.0, 30.0), 2.0), Some(1)); // filled rect interior
        assert_eq!(d.hit(pos2(200.0, 31.0), 2.0), Some(3)); // the arrow between the shapes
        assert!(d.hit(pos2(1000.0, 1000.0), 2.0).is_none());
        let rotated = WbShape::new(ShapeKind::Rect, Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 10.0)))
            .fill(Color32::RED)
            .rotated(std::f32::consts::FRAC_PI_2);
        assert!(rotated.hit(pos2(50.0, 40.0), 1.0));
        assert!(!rotated.hit(pos2(95.0, 5.0), 1.0));
    }

    #[test]
    fn layer_order() {
        let mut d = sample();
        let first = d.shapes[0].id;
        d.reorder(&[first], true);
        assert_eq!(d.shapes.last().unwrap().id, first);
        d.step(&[first], -1);
        assert_eq!(d.shapes[d.shapes.len() - 2].id, first);
        d.reorder(&[first], false);
        assert_eq!(d.shapes[0].id, first);
    }

    #[test]
    fn remove_unbinds() {
        let mut d = sample();
        d.remove(&[1]);
        assert!(d.shapes.iter().all(|s| s.bind_a != Some(1)));
    }

    #[test]
    fn resize_remaps_freehand() {
        let mut s = WbShape::freehand(synthetic_stroke(&[pos2(0.0, 0.0), pos2(10.0, 10.0), pos2(20.0, 0.0)]), false);
        let r = s.rect;
        s.set_rect(Rect::from_min_size(r.min, r.size() * 2.0));
        let b = common::bounds_of(s.points.iter().map(|p| p.pos));
        assert!((b.width() - r.width() * 2.0).abs() < 0.1);
    }
}
