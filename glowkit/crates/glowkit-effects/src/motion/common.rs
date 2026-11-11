//! Shared painting helpers for the motion and cursor modules.

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use glowkit_core::color::{gradient, mix, with_alpha};
use glowkit_core::{paint, Theme};

/// A painter-drawn card: a gradient face with an emoji / glyph, a title and an optional subtitle.
///
/// Used by the card-based motion components ([`crate::StackCards`], [`crate::CardSwap`],
/// [`crate::BounceCards`], [`crate::RingCarousel`], [`crate::CardTrail`]) that need to rotate or
/// skew whole cards, which egui widgets cannot do.
///
/// ```no_run
/// use glowkit_effects::CardFace;
/// let face = CardFace::new("🌊", "Ocean").subtitle("Deep blue").colors(glowkit_core::hex("#0EA5E9"), glowkit_core::hex("#6366F1"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct CardFace {
    /// Large glyph shown on the card (emoji or a short string).
    pub icon: String,
    /// Title line.
    pub title: String,
    /// Optional second line.
    pub subtitle: String,
    /// Gradient colors; `None` picks a theme-derived gradient.
    pub colors: Option<[Color32; 2]>,
}

impl CardFace {
    /// A card with an icon glyph and a title.
    pub fn new(icon: impl Into<String>, title: impl Into<String>) -> Self {
        Self { icon: icon.into(), title: title.into(), subtitle: String::new(), colors: None }
    }
    /// Second, dimmer line of text.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Gradient colors of the card face (top-left to bottom-right).
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = Some([a, b]);
        self
    }

    pub(crate) fn resolved_colors(&self, t: &Theme, index: usize) -> [Color32; 2] {
        self.colors.unwrap_or_else(|| {
            let a = t.chart[index % t.chart.len()];
            let b = t.chart[(index + 1) % t.chart.len()];
            [a, mix(a, b, 0.55)]
        })
    }
}

/// A handful of colorful demo faces.
pub(crate) fn sample_faces() -> Vec<CardFace> {
    use glowkit_core::hex;
    vec![
        CardFace::new("🌊", "Ocean").subtitle("Deep blue calm").colors(hex("#0EA5E9"), hex("#6366F1")),
        CardFace::new("🌅", "Sunset").subtitle("Golden hour").colors(hex("#F97316"), hex("#EC4899")),
        CardFace::new("🌲", "Forest").subtitle("Moss & pine").colors(hex("#10B981"), hex("#0F766E")),
        CardFace::new("🌌", "Nebula").subtitle("Stardust drift").colors(hex("#8B5CF6"), hex("#312E81")),
        CardFace::new("🍋", "Citrus").subtitle("Fresh & bright").colors(hex("#FACC15"), hex("#F97316")),
        CardFace::new("🌸", "Bloom").subtitle("Soft petals").colors(hex("#F472B6"), hex("#A855F7")),
        CardFace::new("❄", "Glacier").subtitle("Ice & light").colors(hex("#67E8F9"), hex("#3B82F6")),
        CardFace::new("🔥", "Ember").subtitle("Warm glow").colors(hex("#EF4444"), hex("#F59E0B")),
    ]
}

/// Fill a (possibly transformed) convex outline with a soft drop shadow made of stacked
/// translucent copies, so rotated cards still cast believable shadows.
pub(crate) fn polygon_shadow(painter: &Painter, outline: &[Pos2], center: Pos2, offset: Vec2, spread: f32, color: Color32) {
    let layers = 8;
    for k in 0..layers {
        let f = (k + 1) as f32 / layers as f32;
        let grow = spread * (1.0 - f);
        let pts: Vec<Pos2> = outline
            .iter()
            .map(|p| {
                let d = *p - center;
                let len = d.length().max(1.0);
                *p + d / len * grow + offset * (0.4 + 0.6 * f)
            })
            .collect();
        painter.add(Shape::convex_polygon(pts, color.gamma_multiply(0.07), Stroke::NONE));
    }
}

/// Everything needed to draw a card face at an arbitrary affine pose.
#[derive(Clone, Copy)]
pub(crate) struct CardPose {
    pub center: Pos2,
    pub size: Vec2,
    pub angle: f32,
    pub scale: f32,
    pub alpha: f32,
    /// Vertical skew (tan of the skew angle), applied before rotation.
    pub skew_y: f32,
    /// Shadow strength 0..1.
    pub shadow: f32,
}

impl CardPose {
    pub fn new(center: Pos2, size: Vec2) -> Self {
        Self { center, size, angle: 0.0, scale: 1.0, alpha: 1.0, skew_y: 0.0, shadow: 1.0 }
    }
    pub fn transform(&self, local: Vec2) -> Pos2 {
        let l = vec2(local.x, local.y + local.x * self.skew_y) * self.scale;
        let (s, c) = self.angle.sin_cos();
        self.center + vec2(l.x * c - l.y * s, l.x * s + l.y * c)
    }
    pub fn outline(&self, radius: f32) -> Vec<Pos2> {
        let r = Rect::from_center_size(Pos2::ZERO, self.size);
        paint::rounded_rect_points(r, radius, 6).into_iter().map(|p| self.transform(p.to_vec2())).collect()
    }
}

/// Paint a [`CardFace`] at a pose. `index` picks the default theme colors.
pub(crate) fn paint_card_face(painter: &Painter, t: &Theme, face: &CardFace, index: usize, pose: CardPose) {
    if pose.alpha <= 0.003 || pose.scale <= 0.01 {
        return;
    }
    let radius = (t.radius * 1.6).min(pose.size.min_elem() * 0.2);
    let outline = pose.outline(radius);
    let [c0, c1] = face.resolved_colors(t, index);
    let a = pose.alpha;
    if pose.shadow > 0.0 {
        let sc = if t.dark { Color32::BLACK } else { Color32::from_rgb(40, 40, 90).gamma_multiply(0.45) };
        polygon_shadow(painter, &outline, pose.center, vec2(0.0, 10.0 * pose.scale), 16.0 * pose.scale, sc.gamma_multiply(a * pose.shadow));
    }
    // gradient evaluated in the card's local space so it rotates with the card
    let (s, co) = (-pose.angle).sin_cos();
    let center = pose.center;
    let half = pose.size * pose.scale / 2.0;
    let stops = [c0, c1];
    paint::fill_convex_fn(painter, &outline, 6, |p| {
        let d = p - center;
        let l = vec2(d.x * co - d.y * s, d.x * s + d.y * co);
        let u = ((l.x / half.x.max(1.0)) * 0.5 + (l.y / half.y.max(1.0)) * 0.5) * 0.5 + 0.5;
        let col = gradient(&stops, u);
        // soft top highlight
        let hl = ((-l.y / half.y.max(1.0)) * 0.5 + 0.5).powi(3) * 0.18;
        mix(col, Color32::WHITE, hl).gamma_multiply(a)
    });
    // glossy rim
    let mut rim = outline.clone();
    rim.push(rim[0]);
    painter.add(Shape::line(rim, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.22 * a))));

    let sc = pose.scale;
    let h = pose.size.y;
    // icon
    let icon_pos = pose.transform(vec2(0.0, -h * 0.12));
    if !face.icon.is_empty() {
        glowkit_core::text::paint_rotated(
            painter,
            &face.icon,
            icon_pos,
            FontId::proportional((h * 0.26 * sc).max(1.0)),
            with_alpha(Color32::WHITE, 0.95 * a),
            pose.angle,
        );
    }
    let title_y = if face.subtitle.is_empty() { h * 0.24 } else { h * 0.2 };
    glowkit_core::text::paint_rotated(
        painter,
        &face.title,
        pose.transform(vec2(0.0, title_y)),
        FontId::proportional((h * 0.095 * sc).clamp(1.0, 28.0)),
        with_alpha(Color32::WHITE, a),
        pose.angle,
    );
    if !face.subtitle.is_empty() {
        glowkit_core::text::paint_rotated(
            painter,
            &face.subtitle,
            pose.transform(vec2(0.0, h * 0.33)),
            FontId::proportional((h * 0.06 * sc).clamp(1.0, 16.0)),
            with_alpha(Color32::WHITE, 0.75 * a),
            pose.angle,
        );
    }
}

/// A tapered, color-graded ribbon through `pts` (head first). `width(t)` and `color(t)` get
/// `t = 0` at the head and `1` at the tail. Edges are feathered for anti-aliasing.
pub(crate) fn tapered_ribbon(painter: &Painter, pts: &[Pos2], width: impl Fn(f32) -> f32, color: impl Fn(f32) -> Color32) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut lens = Vec::with_capacity(n);
    let mut total = 0.0;
    lens.push(0.0);
    for i in 1..n {
        total += pts[i].distance(pts[i - 1]);
        lens.push(total);
    }
    if total < 0.5 {
        return;
    }
    let feather = 1.0;
    let mut mesh = Mesh::default();
    for i in 0..n {
        let p = pts[i];
        let prev = pts[i.saturating_sub(1)];
        let next = pts[(i + 1).min(n - 1)];
        let mut d = next - prev;
        if d.length_sq() < 1e-6 {
            d = vec2(1.0, 0.0);
        }
        let normal = d.normalized().rot90();
        let t = lens[i] / total;
        let w = (width(t) / 2.0).max(0.0);
        let col = color(t);
        mesh.colored_vertex(p + normal * (w + feather), Color32::TRANSPARENT);
        mesh.colored_vertex(p + normal * w, col);
        mesh.colored_vertex(p - normal * w, col);
        mesh.colored_vertex(p - normal * (w + feather), Color32::TRANSPARENT);
    }
    for k in 0..n - 1 {
        let a = (k * 4) as u32;
        let b = ((k + 1) * 4) as u32;
        for s in 0..3 {
            mesh.add_triangle(a + s, b + s, b + s + 1);
            mesh.add_triangle(a + s, b + s + 1, a + s + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// Polynomial smooth minimum: blends two signed distances so shapes melt into each other.
#[inline]
pub(crate) fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Signed distance from `p` to a rounded box.
#[inline]
pub(crate) fn sd_round_rect(p: Pos2, rect: Rect, r: f32) -> f32 {
    let c = rect.center();
    let half = rect.size() / 2.0 - Vec2::splat(r);
    let q = vec2((p.x - c.x).abs() - half.x, (p.y - c.y).abs() - half.y);
    let outside = vec2(q.x.max(0.0), q.y.max(0.0)).length();
    outside + q.x.max(q.y).min(0.0) - r
}

/// Fill the region where `field(p) < 0` inside `bounds` using marching squares on a grid of
/// `cell`-sized squares, with an anti-aliased outline. This is how the "gooey" (metaball)
/// effects are drawn without a GPU filter.
pub(crate) fn fill_field(painter: &Painter, bounds: Rect, cell: f32, field: impl Fn(Pos2) -> f32, color: impl Fn(Pos2) -> Color32) {
    let cols = ((bounds.width() / cell).ceil() as usize).clamp(1, 400);
    let rows = ((bounds.height() / cell).ceil() as usize).clamp(1, 400);
    let cw = bounds.width() / cols as f32;
    let ch = bounds.height() / rows as f32;
    let mut vals = vec![0.0f32; (cols + 1) * (rows + 1)];
    let mut any = false;
    for j in 0..=rows {
        for i in 0..=cols {
            let v = field(pos2(bounds.min.x + i as f32 * cw, bounds.min.y + j as f32 * ch));
            any |= v < 0.0;
            vals[j * (cols + 1) + i] = v;
        }
    }
    if !any {
        return;
    }
    let mut mesh = Mesh::default();
    let mut edges: Vec<[Pos2; 2]> = Vec::new();
    let at = |i: usize, j: usize| vals[j * (cols + 1) + i];
    let pt = |i: usize, j: usize| pos2(bounds.min.x + i as f32 * cw, bounds.min.y + j as f32 * ch);
    for j in 0..rows {
        for i in 0..cols {
            let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let v: [f32; 4] = corners.map(|(a, b)| at(a, b));
            let inside = v.map(|x| x < 0.0);
            let count = inside.iter().filter(|b| **b).count();
            if count == 0 {
                continue;
            }
            let p: [Pos2; 4] = corners.map(|(a, b)| pt(a, b));
            if count == 4 {
                let base = mesh.vertices.len() as u32;
                for q in p {
                    mesh.vertices.push(Vertex { pos: q, uv: WHITE_UV, color: color(q) });
                }
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
                continue;
            }
            let mut poly: Vec<Pos2> = Vec::with_capacity(8);
            let mut crossings: Vec<Pos2> = Vec::with_capacity(4);
            for k in 0..4 {
                let n = (k + 1) % 4;
                if inside[k] {
                    poly.push(p[k]);
                }
                if inside[k] != inside[n] {
                    let t = v[k] / (v[k] - v[n]);
                    let x = p[k].lerp(p[n], t.clamp(0.0, 1.0));
                    poly.push(x);
                    crossings.push(x);
                }
            }
            let base = mesh.vertices.len() as u32;
            for q in &poly {
                mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: color(*q) });
            }
            for k in 1..poly.len().saturating_sub(1) {
                mesh.add_triangle(base, base + k as u32, base + k as u32 + 1);
            }
            if crossings.len() == 2 {
                edges.push([crossings[0], crossings[1]]);
            } else if crossings.len() == 4 {
                edges.push([crossings[0], crossings[1]]);
                edges.push([crossings[2], crossings[3]]);
            }
        }
    }
    painter.add(Shape::mesh(mesh));
    // anti-aliased rim: egui strokes are feathered, so tracing the contour hides stair-stepping
    for [a, b] in edges {
        let c = color(a.lerp(b, 0.5));
        painter.line_segment([a, b], Stroke::new(1.2, c));
    }
}

/// A rounded "icon tile" with a glyph centered on it.
pub(crate) fn icon_tile(painter: &Painter, rect: Rect, radius: f32, colors: [Color32; 2], icon: &str, alpha: f32) {
    let pts = paint::rounded_rect_points(rect, radius, 6);
    let stops = [colors[0].gamma_multiply(alpha), colors[1].gamma_multiply(alpha)];
    paint::fill_convex_fn(painter, &pts, 4, paint::linear_fn(rect, 60.0, &stops));
    let mut rim = pts;
    rim.push(rim[0]);
    painter.add(Shape::line(rim, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.25 * alpha))));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(rect.height() * 0.5),
        with_alpha(Color32::WHITE, alpha),
    );
}

/// Paint a plain theme card (shadow, fill, hairline border).
pub(crate) fn theme_card(painter: &Painter, t: &Theme, rect: Rect, radius: f32, elevation: f32) {
    if elevation > 0.0 {
        let sc = if t.dark { Color32::from_black_alpha(140) } else { Color32::from_rgba_unmultiplied(20, 20, 50, 34) };
        paint::soft_shadow(painter, rect, radius, 18.0 * elevation, vec2(0.0, 6.0 * elevation), sc);
    }
    let cr = egui::CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);
    let fill = if t.dark { glowkit_core::color::shade(t.card, 0.03) } else { t.card };
    painter.rect(rect, cr, fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
}
