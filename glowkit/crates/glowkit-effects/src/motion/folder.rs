use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

fn fill_poly(painter: &egui::Painter, pts: &[Pos2], top: Color32, bottom: Color32) {
    if pts.len() < 3 {
        return;
    }
    let (mut y0, mut y1) = (f32::MAX, f32::MIN);
    for p in pts {
        y0 = y0.min(p.y);
        y1 = y1.max(p.y);
    }
    let mut m = Mesh::default();
    let c = pts.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / pts.len() as f32;
    let col = |p: Pos2| mix(top, bottom, ((p.y - y0) / (y1 - y0).max(1.0)).clamp(0.0, 1.0));
    m.colored_vertex(c.to_pos2(), col(c.to_pos2()));
    for p in pts {
        m.colored_vertex(*p, col(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(m));
}

/// A folder icon that opens when clicked: the front flap tilts down and papers fan out,
/// each one leaning toward the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::Folder::new().size(1.6).color(glowkit_core::hex("#5227FF")));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Folder {
    color: Option<Color32>,
    size: f32,
    papers: Vec<String>,
    open: Option<bool>,
    autoplay: bool,
    salt: u64,
}

impl Default for Folder {
    fn default() -> Self {
        Self::new()
    }
}

impl Folder {
    /// An accent-colored folder holding three papers.
    pub fn new() -> Self {
        Self { color: None, size: 1.0, papers: vec![String::new(), String::new(), String::new()], open: None, autoplay: false, salt: 0 }
    }
    /// Folder color. Defaults to the theme accent.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Scale factor (1.0 = 110 x 84 points).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.clamp(0.3, 5.0);
        self
    }
    /// Short labels written on the papers (up to three).
    pub fn papers<S: Into<String>>(mut self, labels: impl IntoIterator<Item = S>) -> Self {
        self.papers = labels.into_iter().take(3).map(Into::into).collect();
        while self.papers.len() < 3 {
            self.papers.push(String::new());
        }
        self
    }
    /// Force the open state instead of toggling on click.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }
    /// Open and close on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }
}

impl Widget for Folder {
    fn ui(self, ui: &mut Ui) -> Response {
        let s = self.size;
        let w = 110.0 * s;
        let h = 84.0 * s;
        let (rect, response) = ui.allocate_exact_size(vec2(w * 2.4, h * 2.3), Sense::click());
        let id = response.id.with(self.salt);
        let body = Rect::from_center_size(rect.center() + vec2(0.0, h * 0.42), vec2(w, h));
        let over = response.hover_pos().map(|p| body.expand(10.0).contains(p)).unwrap_or(false);
        if over {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let mut open: bool = glowkit_core::state(ui, id.with("open"), || false);
        if response.clicked() && over {
            open = !open;
            glowkit_core::set_state(ui, id.with("open"), open);
        }
        if let Some(o) = self.open {
            open = o;
        } else if self.autoplay && !response.hovered() {
            open = (glowkit_core::time(ui) / 2.2).floor() as i64 % 2 == 1;
            glowkit_core::animate(ui);
        }
        let o = glowkit_core::anim::spring(ui, id.with("o"), if open { 1.0 } else { 0.0 }, 220.0, 15.0);
        let lift = glowkit_core::anim::spring(ui, id.with("lift"), if over && !open { 1.0 } else { 0.0 }, 300.0, 20.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let t = Theme::of(ui);
        let color = self.color.unwrap_or(t.accent);
        let painter = ui.painter_at(rect.expand(20.0));
        let body = body.translate(vec2(0.0, -6.0 * lift * s));
        let r = 10.0 * s;
        paint::soft_shadow(&painter, body, r, 24.0 * s, vec2(0.0, 10.0 * s), with_alpha(Color32::BLACK, if t.dark { 0.5 } else { 0.18 }));
        // back panel with tab
        let back = shade(color, -0.1);
        let tab = Rect::from_min_size(body.min - vec2(0.0, 10.0 * s), vec2(w * 0.36, 20.0 * s));
        painter.rect_filled(tab, egui::CornerRadius::same((6.0 * s) as u8), back);
        painter.rect_filled(body, egui::CornerRadius::same(r as u8), back);
        // papers
        let paper_size = [vec2(w * 0.7, h * 0.8), vec2(w * 0.8, h * 0.7), vec2(w * 0.9, h * 0.6)];
        let closed_off = [vec2(0.0, -h * 0.02), vec2(0.0, h * 0.06), vec2(0.0, h * 0.13)];
        let open_off = [vec2(-w * 0.42, -h * 0.62), vec2(w * 0.38, -h * 0.6), vec2(-w * 0.02, -h * 0.88)];
        let open_rot = [-15f32, 15.0, 5.0];
        let pointer = response.hover_pos();
        let paper_cols = [mix(Color32::from_gray(230), color, 0.06), mix(Color32::from_gray(242), color, 0.04), Color32::WHITE];
        for i in 0..3 {
            let base = body.center() + vec2(0.0, h * 0.05) + closed_off[i].lerp(open_off[i], o) + vec2(0.0, -h * 0.12 * lift);
            let mut magnet = Vec2::ZERO;
            if let Some(p) = pointer {
                if o > 0.8 && p.distance(base) < w * 0.55 {
                    magnet = (p - base) * 0.15;
                }
            }
            let m = glowkit_core::anim::spring2(ui, id.with(("mag", i)), magnet, 260.0, 18.0);
            let center = base + m;
            let ang = (open_rot[i] * o).to_radians();
            let pr = Rect::from_center_size(Pos2::ZERO, paper_size[i]);
            let pts: Vec<Pos2> = paint::rounded_rect_points(pr, 8.0 * s, 4)
                .into_iter()
                .map(|p| {
                    let (sn, cs) = ang.sin_cos();
                    center + vec2(p.x * cs - p.y * sn, p.x * sn + p.y * cs)
                })
                .collect();
            painter.add(Shape::convex_polygon(pts, paper_cols[i], Stroke::new(1.0, with_alpha(Color32::BLACK, 0.06))));
            // writing on the paper
            let (sn, cs) = ang.sin_cos();
            let tf = |x: f32, y: f32| center + vec2(x * cs - y * sn, x * sn + y * cs);
            let label = &self.papers[i];
            if !label.is_empty() && o > 0.5 {
                glowkit_core::text::paint_rotated(&painter, label, tf(0.0, -paper_size[i].y * 0.28), FontId::proportional(10.5 * s), with_alpha(Color32::from_gray(60), (o - 0.5) * 2.0), ang);
            }
            for k in 0..3 {
                let y = -paper_size[i].y * 0.08 + k as f32 * 8.0 * s;
                let lw = paper_size[i].x * if k == 2 { 0.4 } else { 0.62 };
                painter.line_segment([tf(-lw / 2.0, y), tf(lw / 2.0, y)], Stroke::new(2.0 * s.min(1.5), Color32::from_gray(200)));
            }
        }
        // front flap: hinged at the bottom, tilts (skews + squashes) as it opens
        let front_top = body.min.y + h * 0.16;
        let squash = 1.0 - 0.38 * o;
        let skew = (15.0f32 * o).to_radians().tan();
        let fr = Rect::from_min_max(pos2(body.min.x, front_top), body.max);
        let pts: Vec<Pos2> = paint::rounded_rect_points(fr, r, 6)
            .into_iter()
            .map(|p| {
                let dy = body.max.y - p.y;
                let ny = body.max.y - dy * squash;
                pos2(p.x + (body.max.y - ny) * skew, ny)
            })
            .collect();
        fill_poly(&painter, &pts, shade(color, 0.06), shade(color, -0.04));
        painter.add(Shape::closed_line(pts, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.18))));
        response
    }
}
