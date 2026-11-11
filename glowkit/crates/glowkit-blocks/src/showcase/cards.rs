use super::util::{bold_text, galley_fine, idle_time};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, hex, mix, paint, rng::hash01, Theme};
use std::f32::consts::TAU;

/// Hover amount for a card: real hover, or a timed autoplay toggle when idle.
fn hover_or_auto(ui: &Ui, resp: &Response, autoplay: bool, period: f32) -> f32 {
    let idle = idle_time(ui, resp.id, resp.hovered());
    let target = if resp.hovered() {
        1.0
    } else if autoplay && idle > 1.0 {
        glowkit_core::animate(ui);
        let ph = (glowkit_core::time(ui) / period).fract();
        if ph < 0.6 {
            1.0
        } else {
            0.0
        }
    } else {
        0.0
    };
    glowkit_core::anim::follow(ui, resp.id.with("hover_k"), target, 5.0)
}

/// A grid mesh over a rounded rect: vertices outside the rounded corners are projected onto
/// the outline, so per-vertex effects get smooth rounded edges.
fn rounded_grid(p: &Painter, rect: Rect, radius: f32, cell: f32, f: impl Fn(Pos2) -> Color32) {
    let cols = (rect.width() / cell).ceil().max(1.0) as usize;
    let rows = (rect.height() / cell).ceil().max(1.0) as usize;
    let mut m = Mesh::default();
    m.reserve_vertices((cols + 1) * (rows + 1));
    for y in 0..=rows {
        for x in 0..=cols {
            let mut q = pos2(rect.min.x + rect.width() * x as f32 / cols as f32, rect.min.y + rect.height() * y as f32 / rows as f32);
            let c = pos2(q.x.clamp(rect.min.x + radius, rect.max.x - radius), q.y.clamp(rect.min.y + radius, rect.max.y - radius));
            let d = q - c;
            if d.length() > radius {
                q = c + d.normalized() * radius;
            }
            m.colored_vertex(q, f(q));
        }
    }
    let w = (cols + 1) as u32;
    for y in 0..rows as u32 {
        for x in 0..cols as u32 {
            let i = y * w + x;
            m.add_triangle(i, i + 1, i + w + 1);
            m.add_triangle(i, i + w + 1, i + w);
        }
    }
    p.add(Shape::mesh(m));
}

/// Huge outlined text; a radial gradient reveal follows the pointer and lights up the outline
/// in color around it. Drifts across the word by itself when the pointer is away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::TextHoverEffect::new("GLOW").size(140.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TextHoverEffect {
    text: String,
    size: f32,
    colors: Vec<Color32>,
    background: Option<Color32>,
    radius: f32,
    autoplay: bool,
}

impl TextHoverEffect {
    /// Outlined `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 120.0, colors: Vec::new(), background: None, radius: 0.0, autoplay: true }
    }
    /// Font size in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Gradient revealed around the pointer (defaults to a warm-to-cool rainbow).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// The surface color behind the text (used to hollow the letters). Defaults to the card color.
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
    /// Reveal radius in points (default: 30% of the text width).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Sweep the reveal across the word when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for TextHoverEffect {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let g = ui.painter().layout_no_wrap(self.text.clone(), font.clone(), Color32::WHITE);
        let gp = ui.painter().layout_no_wrap(self.text.clone(), font, Color32::PLACEHOLDER);
        let size = g.size() + vec2(20.0, 10.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let origin = rect.min + vec2(10.0, 5.0);
        let idle = idle_time(ui, resp.id, resp.hovered());
        let time = glowkit_core::time(ui);
        let auto = pos2(rect.center().x + rect.width() * 0.42 * (time * 0.7).sin(), rect.center().y + rect.height() * 0.25 * (time * 1.3).cos());
        let target = match resp.hover_pos() {
            Some(hp) => hp,
            None if self.autoplay && idle > 0.6 => auto,
            None => pos2(rect.center().x, rect.max.y + 400.0),
        };
        let px = glowkit_core::anim::follow(ui, resp.id.with("px"), target.x, 9.0);
        let py = glowkit_core::anim::follow(ui, resp.id.with("py"), target.y, 9.0);
        let center = pos2(px, py);
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        let colors = if self.colors.is_empty() { vec![hex("#EAB308"), hex("#EF4444"), hex("#3B82F6"), hex("#06B6D4"), hex("#8B5CF6")] } else { self.colors.clone() };
        let radius = if self.radius > 0.0 { self.radius } else { rect.width() * 0.3 };
        let bg = self.background.unwrap_or(t.card);
        let p = ui.painter();
        let outline = if t.dark { mix(bg, t.foreground, 0.22) } else { mix(bg, t.foreground, 0.28) };
        let stroke_w = (self.size / 90.0).clamp(0.8, 2.2);
        let n_dirs = 12;
        let offsets: Vec<Vec2> = (0..n_dirs).map(|i| {
            let a = i as f32 / n_dirs as f32 * TAU;
            vec2(a.cos(), a.sin()) * stroke_w
        }).collect();
        for o in &offsets {
            p.galley(origin + *o, gp.clone(), outline);
        }
        // colored outline around the pointer
        let w = rect.width();
        let cc = colors.clone();
        for o in &offsets {
            galley_fine(p, origin + *o, &g, 10, |q| {
                let d = q.distance(center) / radius;
                let a = (1.0 - smoothstep(0.0, 1.0, d)).powf(1.3);
                if a <= 0.0 {
                    return Color32::TRANSPARENT;
                }
                let u = ((q.x - rect.min.x) / w + (q.y - rect.min.y) / w * 0.3).clamp(0.0, 1.0);
                with_alpha(glowkit_core::color::gradient(&cc, u), a)
            });
        }
        // hollow the letters
        p.galley(origin, gp.clone(), bg);
        // a faint inner fill that also glows near the pointer
        galley_fine(p, origin, &g, 10, |q| {
            let d = q.distance(center) / radius;
            let a = (1.0 - smoothstep(0.0, 1.0, d)) * 0.10;
            let u = ((q.x - rect.min.x) / w).clamp(0.0, 1.0);
            with_alpha(glowkit_core::color::gradient(&cc, u), a)
        });
        resp
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DirState {
    hovered: bool,
    dir: Vec2,
    k: f32,
}

type ScenePainter<'a> = Box<dyn Fn(&Painter, Rect) + 'a>;

/// A card whose caption overlay slides in from the edge the pointer entered through, and out
/// through the edge it leaves by. The "image" is any painting closure.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::DirectionAwareHover::new(|p, r| {
///     glowkit_core::paint::gradient_rect(p, r, 16.0, 45.0, &[glowkit_core::hex("#8B5CF6"), glowkit_core::hex("#EC4899")]);
/// }).title("In the mountains").subtitle("$1299 / night"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DirectionAwareHover<'a> {
    scene: ScenePainter<'a>,
    size: Vec2,
    title: String,
    subtitle: String,
    radius: f32,
    autoplay: bool,
}

impl<'a> DirectionAwareHover<'a> {
    /// A card showing a scene painted by `scene(painter, rect)`.
    pub fn new(scene: impl Fn(&Painter, Rect) + 'a) -> Self {
        Self { scene: Box::new(scene), size: vec2(300.0, 300.0), title: String::new(), subtitle: String::new(), radius: 16.0, autoplay: true }
    }
    /// Card size in points.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Overlay title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Overlay subtitle.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Corner radius.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    /// Demonstrate entries from each side when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

fn edge_dir(rect: Rect, p: Pos2) -> Vec2 {
    let d = p - rect.center();
    let nx = d.x / rect.width();
    let ny = d.y / rect.height();
    if nx.abs() > ny.abs() {
        vec2(nx.signum(), 0.0)
    } else {
        vec2(0.0, ny.signum())
    }
}

impl Widget for DirectionAwareHover<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = resp.id;
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let idle = idle_time(ui, id, resp.hovered());
        let time = glowkit_core::time(ui);
        let (hovered, auto_dir) = if resp.hovered() {
            (true, None)
        } else if self.autoplay && idle > 1.0 {
            glowkit_core::animate(ui);
            let slot = time / 2.2;
            let dirs = [vec2(-1.0, 0.0), vec2(0.0, -1.0), vec2(1.0, 0.0), vec2(0.0, 1.0)];
            let i = slot as usize % 4;
            let f = slot.fract();
            (f < 0.62, Some(if f < 0.62 { dirs[i] } else { dirs[(i + 1) % 4] }))
        } else {
            (false, None)
        };
        let dt = glowkit_core::dt(ui);
        let st = glowkit_core::with_state(ui, id.with("dah"), DirState::default, |s| {
            if hovered != s.hovered {
                s.dir = auto_dir.unwrap_or_else(|| pointer.map(|p| edge_dir(rect, p)).unwrap_or(vec2(0.0, 1.0)));
                s.hovered = hovered;
            }
            s.k = glowkit_core::anim::approach(s.k, if hovered { 1.0 } else { 0.0 }, 9.0, dt);
            *s
        });
        if (st.k - if hovered { 1.0 } else { 0.0 }).abs() > 1e-3 {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let k = st.k;
        let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        (self.scene)(&p, rect);
        // overlay
        let off = st.dir * (1.0 - k) * vec2(rect.width(), rect.height());
        let ov = rect.translate(off);
        let r = self.radius;
        rounded_grid(&p, rect, r, 8.0, |q| {
            if !ov.contains(q) {
                return Color32::TRANSPARENT;
            }
            let u = ((q.y - ov.min.y) / ov.height()).clamp(0.0, 1.0);
            Color32::from_black_alpha((40.0 + 150.0 * u * u) as u8)
        });
        let tp = pos2(ov.min.x + 22.0, ov.max.y - 50.0);
        bold_text(&p, tp, Align2::LEFT_BOTTOM, &self.title, FontId::proportional(20.0), Color32::WHITE);
        p.text(tp + vec2(0.0, 8.0), Align2::LEFT_TOP, &self.subtitle, FontId::proportional(13.5), Color32::from_white_alpha(200));
        resp
    }
}

/// A card whose top half is a grid of tiny stars; random ones glow and fade, and more of them
/// light up on hover. Title and description underneath.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::GlowingStarsCard::new("Next.js 14 is here", "All about the new release."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GlowingStarsCard {
    title: String,
    description: String,
    size: Vec2,
    grid: (usize, usize),
    color: Option<Color32>,
    autoplay: bool,
}

impl GlowingStarsCard {
    /// A card with a title and description.
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self { title: title.into(), description: description.into(), size: vec2(320.0, 260.0), grid: (18, 6), color: None, autoplay: true }
    }
    /// Card size.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Number of star columns and rows.
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.grid = (cols.max(1), rows.max(1));
        self
    }
    /// Glow color (defaults to a cool blue).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Pulse more stars periodically when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for GlowingStarsCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hk = hover_or_auto(ui, &resp, self.autoplay, 4.0);
        glowkit_core::animate(ui);
        let p = ui.painter_at(rect.expand(16.0));
        let r = 16.0;
        let bg_a = if t.dark { hex("#0B0B12") } else { hex("#FFFFFF") };
        let bg_b = if t.dark { mix(hex("#0B0B12"), t.accent, 0.12) } else { mix(hex("#FFFFFF"), t.accent, 0.06) };
        paint::soft_shadow(&p, rect, r, 24.0, vec2(0.0, 8.0), Color32::from_black_alpha(if t.dark { 110 } else { 25 }));
        paint::gradient_rect(&p, rect, r, 110.0, &[bg_a, bg_b]);
        p.rect_stroke(rect, r, Stroke::new(1.0, t.border.lerp_to_gamma(t.accent, hk * 0.4)), StrokeKind::Inside);
        let glow_c = self.color.unwrap_or(if t.dark { hex("#60A5FA") } else { hex("#2563EB") });
        let (cols, rows) = self.grid;
        let area = Rect::from_min_size(rect.min + vec2(24.0, 26.0), vec2(rect.width() - 48.0, rect.height() * 0.42));
        let time = glowkit_core::time(ui);
        let epoch = (time / 1.3).floor() as u64 + 1000;
        let f = (time / 1.3).fract();
        let density = 0.10 + 0.32 * hk;
        let mut m = Mesh::default();
        let dim = if t.dark { Color32::from_white_alpha(46) } else { Color32::from_black_alpha(40) };
        let mut glows = Vec::new();
        for y in 0..rows {
            for x in 0..cols {
                let i = (y * cols + x) as u64;
                let on_now = hash01(i * 92821 + epoch * 7919) < density;
                let on_prev = hash01(i * 92821 + (epoch - 1) * 7919) < density;
                let g = match (on_prev, on_now) {
                    (true, true) => 1.0,
                    (false, false) => 0.0,
                    (false, true) => smoothstep(0.0, 0.45, f),
                    (true, false) => 1.0 - smoothstep(0.0, 0.45, f),
                };
                let c = pos2(area.min.x + area.width() * (x as f32 + 0.5) / cols as f32, area.min.y + area.height() * (y as f32 + 0.5) / rows as f32);
                let col = dim.lerp_to_gamma(glow_c, g);
                super::util::mesh_dot(&mut m, c, 1.0 + g * 0.8, col);
                if g > 0.05 {
                    glows.push((c, g));
                }
            }
        }
        for (c, g) in glows {
            paint::glow(&p, c, 9.0 * g, with_alpha(glow_c, 0.55 * g));
        }
        p.add(Shape::mesh(m));
        let ty = rect.max.y - 86.0;
        bold_text(&p, pos2(rect.min.x + 24.0, ty), Align2::LEFT_TOP, &self.title, FontId::proportional(19.0), t.foreground);
        let desc = p.layout(self.description.clone(), FontId::proportional(13.5), t.muted_foreground, rect.width() - 100.0);
        p.galley(pos2(rect.min.x + 24.0, ty + 30.0), desc, t.muted_foreground);
        // arrow button
        let bc = pos2(rect.max.x - 38.0, rect.max.y - 40.0);
        p.circle_filled(bc, 15.0, with_alpha(t.foreground, 0.08 + 0.08 * hk));
        p.circle_stroke(bc, 15.0, Stroke::new(1.0, with_alpha(t.foreground, 0.15)));
        let ax = bc.x - 5.0 + hk * 2.0;
        p.line_segment([pos2(ax, bc.y), pos2(ax + 10.0, bc.y)], Stroke::new(1.6, t.foreground));
        super::util::chevron(&p, pos2(ax + 9.0, bc.y), 8.0, vec2(1.0, 0.0), Stroke::new(1.6, t.foreground));
        resp
    }
}

/// A card that, on hover, fills with an animated dot-matrix that ripples outward from the
/// center, while the icon slides up and the title fades in. Corner "+" marks frame it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::CanvasRevealCard::new("Sheetal is Nisha", "★"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CanvasRevealCard {
    title: String,
    icon: String,
    size: Vec2,
    colors: Vec<Color32>,
    dot: f32,
    autoplay: bool,
}

impl CanvasRevealCard {
    /// A card with a title and a big glyph icon.
    pub fn new(title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self { title: title.into(), icon: icon.into(), size: vec2(260.0, 340.0), colors: Vec::new(), dot: 3.0, autoplay: true }
    }
    /// Card size.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Dot colors (defaults to the theme's first chart color).
    pub fn colors(mut self, c: &[Color32]) -> Self {
        self.colors = c.to_vec();
        self
    }
    /// Dot size in points (spacing is ~2.3x this).
    pub fn dot_size(mut self, s: f32) -> Self {
        self.dot = s.max(1.0);
        self
    }
    /// Reveal on a timer when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for CanvasRevealCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hk = hover_or_auto(ui, &resp, self.autoplay, 4.2);
        let p = ui.painter_at(rect.expand(8.0));
        let bg = if t.dark { t.card } else { Color32::WHITE };
        p.rect_filled(rect, 0.0, bg);
        let time = glowkit_core::time(ui);
        if hk > 0.01 {
            glowkit_core::animate(ui);
            let colors = if self.colors.is_empty() { vec![t.chart[0], mix(t.chart[0], t.chart[1], 0.5)] } else { self.colors.clone() };
            let step = self.dot * 2.3;
            let cols = (rect.width() / step) as usize;
            let rows = (rect.height() / step) as usize;
            let c = rect.center();
            let maxd = rect.size().length() / 2.0;
            let front = hk * maxd * 1.15;
            let mut m = Mesh::default();
            let ox = rect.min.x + (rect.width() - cols as f32 * step) / 2.0;
            let oy = rect.min.y + (rect.height() - rows as f32 * step) / 2.0;
            let tick = (time * 5.0) as u64;
            for y in 0..rows {
                for x in 0..cols {
                    let q = pos2(ox + (x as f32 + 0.5) * step, oy + (y as f32 + 0.5) * step);
                    let d = q.distance(c);
                    let reveal = smoothstep(d - 0.0, d + 30.0, front);
                    if reveal <= 0.0 {
                        continue;
                    }
                    let i = (y * cols + x) as u64;
                    let phase = (hash01(i * 13 + 1) * 6.0) as u64;
                    let lv = hash01(i * 7 + (tick + phase) / 3 * 131);
                    let op = [0.15, 0.15, 0.25, 0.3, 0.45, 0.5, 0.6, 0.8, 0.9, 1.0][(lv * 10.0) as usize % 10];
                    let col = colors[(hash01(i * 3 + 5) * colors.len() as f32) as usize % colors.len()];
                    // ring at the wavefront is brighter
                    let edge = 1.0 + 0.8 * (1.0 - ((front - d) / 30.0).abs().min(1.0));
                    let a = (op * reveal * edge).min(1.0);
                    super::util::mesh_rect(&mut m, Rect::from_center_size(q, Vec2::splat(self.dot)), with_alpha(col, a));
                }
            }
            p.add(Shape::mesh(m));
            // fade the top toward the card color for depth
            let top = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() * 0.45));
            paint::quad(&p, [top.left_top(), top.right_top(), top.right_bottom(), top.left_bottom()], [with_alpha(bg, 0.85), with_alpha(bg, 0.85), with_alpha(bg, 0.0), with_alpha(bg, 0.0)]);
        }
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        // corner plus marks
        let pc = t.foreground.gamma_multiply(0.8);
        for cpt in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
            p.line_segment([cpt - vec2(7.0, 0.0), cpt + vec2(7.0, 0.0)], Stroke::new(1.5, pc));
            p.line_segment([cpt - vec2(0.0, 7.0), cpt + vec2(0.0, 7.0)], Stroke::new(1.5, pc));
        }
        // icon + title
        let ic = rect.center() - vec2(0.0, hk * 18.0);
        let icon_col = t.foreground.lerp_to_gamma(Color32::WHITE, if t.dark { hk } else { 0.0 });
        p.text(ic, Align2::CENTER_CENTER, &self.icon, FontId::proportional(54.0), with_alpha(icon_col, 1.0 - hk * 0.0));
        let title_a = smoothstep(0.3, 1.0, hk);
        if title_a > 0.0 {
            let ty = rect.center().y + 44.0 - title_a * 8.0;
            bold_text(&p, pos2(rect.center().x, ty), Align2::CENTER_CENTER, &self.title, FontId::proportional(20.0), with_alpha(t.foreground, title_a));
        }
        resp
    }
}

/// A card that tilts back in perspective on hover while a "pin" rises from its center: a beam,
/// a label pill and pulsing elliptical rings on the card's surface.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::Pin3D::new("glowkit.rs").title("Aceternity-style pin").description("Customizable Rust components."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Pin3D {
    label: String,
    title: String,
    description: String,
    size: Vec2,
    colors: [Color32; 2],
    autoplay: bool,
}

impl Pin3D {
    /// A pinned card whose pin shows `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            title: String::new(),
            description: String::new(),
            size: vec2(300.0, 300.0),
            colors: [hex("#06B6D4"), hex("#8B5CF6")],
            autoplay: true,
        }
    }
    /// Card title.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Card description.
    pub fn description(mut self, s: impl Into<String>) -> Self {
        self.description = s.into();
        self
    }
    /// Card size.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Pin / image gradient colors.
    pub fn colors(mut self, a: Color32, b: Color32) -> Self {
        self.colors = [a, b];
        self
    }
    /// Tilt on a timer when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for Pin3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let total = self.size + vec2(0.0, 130.0);
        let (outer, resp) = ui.allocate_exact_size(total, Sense::hover());
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let rect = Rect::from_min_size(pos2(outer.min.x, outer.max.y - self.size.y), self.size);
        let hk = hover_or_auto(ui, &resp, self.autoplay, 4.0);
        let time = glowkit_core::time(ui);
        if hk > 0.0 {
            glowkit_core::animate(ui);
        }
        let p = ui.painter_at(outer.expand(20.0));
        // perspective tilt around the card's horizontal axis
        let theta = hk * 0.65;
        let persp = 700.0;
        let c = rect.center();
        let proj = |q: Pos2| -> (Pos2, f32) {
            let dy = q.y - c.y;
            let dx = q.x - c.x;
            let z = dy * theta.sin(); // positive = further away (top goes back)
            let y = dy * theta.cos();
            let s = persp / (persp - z);
            (pos2(c.x + dx * s, c.y + y * s), s)
        };
        let outline: Vec<Pos2> = paint::subdivide_closed(&paint::rounded_rect_points(rect, 18.0, 6), 20.0).into_iter().map(|q| proj(q).0).collect();
        let bg = if t.dark { hex("#0A0A0F") } else { hex("#FFFFFF") };
        let bg2 = if t.dark { mix(bg, self.colors[1], 0.12) } else { mix(bg, self.colors[1], 0.05) };
        paint::fill_convex_fn(&p, &outline, 4, |q| bg.lerp_to_gamma(bg2, ((q.y - rect.min.y) / rect.height()).clamp(0.0, 1.0)));
        p.add(Shape::closed_line(outline.clone(), Stroke::new(1.0, t.border.lerp_to_gamma(self.colors[0], hk * 0.5))));
        // content, projected
        let (tp, ts) = proj(rect.min + vec2(20.0, 26.0));
        bold_text(&p, tp, Align2::LEFT_TOP, &self.title, FontId::proportional(17.0 * ts), t.foreground);
        let (dp, ds) = proj(rect.min + vec2(20.0, 54.0));
        p.text(dp, Align2::LEFT_TOP, &self.description, FontId::proportional(13.0 * ds), t.muted_foreground);
        let img = Rect::from_min_max(rect.min + vec2(20.0, 86.0), rect.max - vec2(20.0, 20.0));
        let img_pts: Vec<Pos2> = paint::subdivide_closed(&paint::rounded_rect_points(img, 10.0, 6), 16.0).into_iter().map(|q| proj(q).0).collect();
        let (c0, c1) = (self.colors[0], self.colors[1]);
        paint::fill_convex_fn(&p, &img_pts, 6, |q| {
            let u = ((q.x - img.min.x) / img.width() * 0.6 + (q.y - img.min.y) / img.height() * 0.4).clamp(0.0, 1.0);
            mix(c0, c1, u).gamma_multiply(0.9)
        });
        // pin
        if hk > 0.01 {
            let base = proj(c).0;
            for i in 0..3 {
                let ph = ((time * 0.6 + i as f32 / 3.0) % 1.0) as f32;
                let rr = 10.0 + ph * 90.0;
                let a = (1.0 - ph) * hk;
                let squash = 0.32 + 0.5 * (1.0 - hk);
                p.add(Shape::ellipse_filled(base, vec2(rr, rr * squash), with_alpha(mix(c0, Color32::BLACK, 0.3), a * 0.35)));
                p.add(Shape::ellipse_stroke(base, vec2(rr, rr * squash), Stroke::new(1.5, with_alpha(mix(c0, Color32::WHITE, 0.3), a))));
            }
            let h = 190.0 * hk;
            let top = base - vec2(0.0, h);
            let beam: Vec<Pos2> = (0..=16).map(|i| base.lerp(top, i as f32 / 16.0)).collect();
            paint::gradient_polyline(&p, &beam, false, 1.5, |u| with_alpha(c0, (1.0 - u) * 0.2 + u * hk));
            paint::glow_ellipse(&p, base.lerp(top, 0.5), vec2(5.0, h * 0.5), with_alpha(c0, 0.25 * hk));
            paint::glow(&p, base, 12.0, with_alpha(c0, 0.6 * hk));
            p.circle_filled(base, 2.5, with_alpha(Color32::WHITE, hk));
            // label pill
            let g = p.layout_no_wrap(self.label.clone(), FontId::proportional(13.0), Color32::PLACEHOLDER);
            let pill = Rect::from_center_size(top - vec2(0.0, 16.0), g.size() + vec2(26.0, 12.0));
            paint::soft_shadow(&p, pill, pill.height() / 2.0, 12.0, vec2(0.0, 4.0), Color32::from_black_alpha((90.0 * hk) as u8));
            p.rect_filled(pill, pill.height() / 2.0, with_alpha(hex("#09090B"), hk));
            p.rect_stroke(pill, pill.height() / 2.0, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.12 * hk)), StrokeKind::Inside);
            p.galley(pill.center() - g.size() / 2.0, g.clone(), with_alpha(Color32::WHITE, hk));
            let ul: Vec<Pos2> = (0..=16).map(|i| pos2(pill.min.x + 12.0 + (pill.width() - 24.0) * i as f32 / 16.0, pill.max.y - 0.5)).collect();
            paint::gradient_polyline(&p, &ul, false, 1.2, |u| with_alpha(c0, hk * (1.0 - (u * 2.0 - 1.0).abs())));
        }
        resp
    }
}
