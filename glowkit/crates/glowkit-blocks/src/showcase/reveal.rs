use super::util::{bold_text, chevron, idle_time};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};
use glowkit_core::{anim::smoothstep, color::with_alpha, mix, paint, Theme};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct ScratchState {
    cov: Arc<Vec<f32>>,
    cols: usize,
    rows: usize,
    last: Option<Pos2>,
    revealed_at: Option<f64>,
    auto_t: f32,
}

/// What [`ScratchToReveal::show`] returns.
pub struct ScratchResponse {
    /// Response of the scratch area.
    pub response: Response,
    /// Fraction of the foil cleared so far (0..1).
    pub cleared: f32,
    /// True once enough foil has been cleared (stays true until reset).
    pub revealed: bool,
    /// True only on the frame the content got revealed.
    pub just_revealed: bool,
}

/// A scratch-card: a shimmering foil covers your content and is cleared wherever the pointer
/// drags. Once about half is gone the rest dissolves and the card reports `revealed`.
/// When idle it can scratch itself along a zig-zag path (great for demos and screenshots).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let out = glowkit_blocks::ScratchToReveal::new(vec2(300.0, 180.0)).show(ui, |ui| {
///     ui.heading("You won 🎉");
/// });
/// if out.just_revealed { /* confetti! */ }
/// # use egui::vec2;
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct ScratchToReveal {
    size: Vec2,
    brush: f32,
    threshold: f32,
    foil: Option<Vec<Color32>>,
    label: String,
    autoplay: bool,
    reset_after: Option<f32>,
    corner: f32,
}

impl ScratchToReveal {
    /// A scratch card of `size` points.
    pub fn new(size: Vec2) -> Self {
        Self { size, brush: 26.0, threshold: 0.5, foil: None, label: "Scratch to reveal".into(), autoplay: true, reset_after: None, corner: 16.0 }
    }
    /// Brush radius in points.
    pub fn brush(mut self, r: f32) -> Self {
        self.brush = r.max(2.0);
        self
    }
    /// Fraction (0..1) of foil that must be cleared to count as revealed.
    pub fn threshold(mut self, f: f32) -> Self {
        self.threshold = f.clamp(0.05, 1.0);
        self
    }
    /// Foil gradient colors (defaults to a pastel version of the theme gradient).
    pub fn foil(mut self, colors: &[Color32]) -> Self {
        self.foil = Some(colors.to_vec());
        self
    }
    /// Hint printed on the foil.
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// Scratch automatically along a zig-zag when nobody has touched it for a while.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
    /// Corner radius of the foil (match your content's rounding).
    pub fn corner_radius(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Re-cover the card this many seconds after it was revealed.
    pub fn reset_after(mut self, seconds: f32) -> Self {
        self.reset_after = Some(seconds);
        self
    }

    /// Paint the card with `add` underneath the foil.
    pub fn show(self, ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> ScratchResponse {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::drag());
        // content first
        {
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Center)));
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            add(&mut child);
        }
        let cell = 5.0;
        let cols = (rect.width() / cell).ceil().max(1.0) as usize;
        let rows = (rect.height() / cell).ceil().max(1.0) as usize;
        let id = resp.id.with("scratch");
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let mut st: ScratchState = glowkit_core::state(ui, id, || ScratchState {
            cov: Arc::new(vec![1.0; (cols + 1) * (rows + 1)]),
            cols,
            rows,
            last: None,
            revealed_at: None,
            auto_t: 0.0,
        });
        if st.cols != cols || st.rows != rows {
            st = ScratchState { cov: Arc::new(vec![1.0; (cols + 1) * (rows + 1)]), cols, rows, last: None, revealed_at: None, auto_t: 0.0 };
        }
        if let (Some(at), Some(after)) = (st.revealed_at, self.reset_after) {
            if (now - at) as f32 > after {
                st = ScratchState { cov: Arc::new(vec![1.0; (cols + 1) * (rows + 1)]), cols, rows, last: None, revealed_at: None, auto_t: 0.0 };
            }
        }
        let active = resp.hovered() || resp.dragged();
        let idle = idle_time(ui, resp.id, active);
        // where are we scratching this frame?
        let mut brush_at: Option<Pos2> = None;
        let mut auto_cursor: Option<Pos2> = None;
        if resp.dragged() {
            brush_at = resp.interact_pointer_pos();
        } else if self.autoplay && idle > 1.2 && st.revealed_at.is_none() {
            st.auto_t += dt * 1.6;
            let a = st.auto_t;
            let u = 0.5 + 0.45 * (a * 3.2).sin();
            let tri = 1.0 - ((a * 0.18).fract() * 2.0 - 1.0).abs();
            let v = 0.12 + 0.76 * tri;
            let pp = pos2(rect.min.x + u * rect.width(), rect.min.y + v * rect.height());
            brush_at = Some(pp);
            auto_cursor = Some(pp);
        }
        let mut just = false;
        if let Some(bp) = brush_at {
            let local = (bp - rect.min).to_pos2();
            let from = st.last.unwrap_or(local);
            let cov = Arc::make_mut(&mut st.cov);
            let dist = from.distance(local);
            let steps = ((dist / (self.brush * 0.3)).ceil() as usize).max(1);
            let r = self.brush;
            for s in 0..=steps {
                let c = from.lerp(local, s as f32 / steps as f32);
                let x0 = ((c.x - r) / cell).floor().max(0.0) as usize;
                let x1 = (((c.x + r) / cell).ceil() as usize).min(cols);
                let y0 = ((c.y - r) / cell).floor().max(0.0) as usize;
                let y1 = (((c.y + r) / cell).ceil() as usize).min(rows);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let d = pos2(x as f32 * cell, y as f32 * cell).distance(c);
                        if d < r {
                            let v = smoothstep(r * 0.78, r, d);
                            let i = y * (cols + 1) + x;
                            cov[i] = cov[i].min(v);
                        }
                    }
                }
            }
            st.last = Some(local);
            ui.ctx().request_repaint();
        } else {
            st.last = None;
        }
        let cleared = 1.0 - st.cov.iter().sum::<f32>() / st.cov.len() as f32;
        if st.revealed_at.is_none() && cleared >= self.threshold {
            st.revealed_at = Some(now);
            just = true;
        }
        let fade = match st.revealed_at {
            Some(at) => 1.0 - smoothstep(0.0, 0.6, (now - at) as f32),
            None => 1.0,
        };
        if fade > 0.0 && fade < 1.0 || self.autoplay || self.reset_after.is_some() {
            ui.ctx().request_repaint();
        }

        if ui.is_rect_visible(rect) && fade > 0.0 {
            let p = ui.painter_at(rect);
            let stops = self.foil.clone().unwrap_or_else(|| {
                let base = if t.dark { 0.55 } else { 0.35 };
                vec![mix(t.gradient[0], Color32::WHITE, base), mix(t.gradient[1], Color32::WHITE, base), mix(t.gradient[2], Color32::WHITE, base)]
            });
            let time = glowkit_core::time(ui);
            let band = (time * 0.35).fract() * 2.2 - 0.6;
            let cov = st.cov.clone();
            let w1 = cols + 1;
            let aspect = rect.width() / rect.height();
            let corner = self.corner;
            let grid_rect = Rect::from_min_size(rect.min, vec2(cols as f32 * cell, rows as f32 * cell));
            let mut m = egui::epaint::Mesh::default();
            m.reserve_vertices(w1 * (rows + 1));
            for y in 0..=rows {
                for x in 0..=cols {
                    let uv = vec2(x as f32 / cols as f32, y as f32 / rows as f32);
                    let a = cov[y * w1 + x] * fade;
                    // project vertices that fall outside the rounded rect onto its edge
                    let mut pp = pos2(grid_rect.min.x + uv.x * grid_rect.width(), grid_rect.min.y + uv.y * grid_rect.height());
                    pp.x = pp.x.min(rect.max.x);
                    pp.y = pp.y.min(rect.max.y);
                    let cx = pp.x.clamp(rect.min.x + corner, rect.max.x - corner);
                    let cy = pp.y.clamp(rect.min.y + corner, rect.max.y - corner);
                    let d = pp - pos2(cx, cy);
                    if d.length() > corner {
                        pp = pos2(cx, cy) + d.normalized() * corner;
                    }
                    let col = if a <= 0.0 {
                        Color32::TRANSPARENT
                    } else {
                        let g = glowkit_core::color::gradient(&stops, (uv.x * aspect * 0.5 + uv.y * 0.5) / (aspect * 0.5 + 0.5));
                        let dd = (uv.x * 0.8 + uv.y * 0.4 - band).abs();
                        let shine = (1.0 - dd / 0.18).clamp(0.0, 1.0);
                        let grain = glowkit_core::rng::hash01((x * 7919 + y * 104729) as u64) * 0.08;
                        with_alpha(mix(g, Color32::WHITE, shine * 0.45 + grain), a)
                    };
                    m.colored_vertex(pp, col);
                }
            }
            let w = w1 as u32;
            for y in 0..rows as u32 {
                for x in 0..cols as u32 {
                    let i = y * w + x;
                    m.add_triangle(i, i + 1, i + w + 1);
                    m.add_triangle(i, i + w + 1, i + w);
                }
            }
            p.add(egui::Shape::mesh(m));
            let label_a = fade * (1.0 - cleared * 2.5).clamp(0.0, 1.0);
            if label_a > 0.0 {
                let ink = with_alpha(mix(stops[0], Color32::BLACK, 0.55), label_a);
                bold_text(&p, rect.center(), Align2::CENTER_CENTER, &format!("✦  {}  ✦", self.label), FontId::proportional(16.0), ink);
            }
            if let Some(ac) = auto_cursor {
                // a little coin "scratching" the foil
                paint::soft_shadow(&p, Rect::from_center_size(ac + vec2(2.0, 3.0), Vec2::splat(22.0)), 11.0, 6.0, Vec2::ZERO, Color32::from_black_alpha(80));
                paint::gradient_circle(&p, ac + vec2(8.0, 8.0), 11.0, &[glowkit_core::hex("#FDE68A"), glowkit_core::hex("#F59E0B"), glowkit_core::hex("#B45309")]);
                p.circle_stroke(ac + vec2(8.0, 8.0), 8.0, Stroke::new(1.0, with_alpha(glowkit_core::hex("#92400E"), 0.6)));
            }
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let revealed = st.revealed_at.is_some();
        glowkit_core::set_state(ui, id, st);
        ScratchResponse { response: resp, cleared, revealed, just_revealed: just }
    }
}

type Scene<'a> = Box<dyn Fn(&Painter, Rect) + 'a>;

/// A before / after comparison: two painted scenes split by a draggable divider with a grab
/// handle. Follows the pointer on hover (optional), and sweeps back and forth on its own when idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_blocks::CompareSlider;
/// ui.add(CompareSlider::new(
///     |p, r| { p.rect_filled(r, 12.0, egui::Color32::DARK_GRAY); },
///     |p, r| { p.rect_filled(r, 12.0, egui::Color32::from_rgb(120, 80, 255)); },
/// ).size(600.0, 340.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CompareSlider<'a> {
    before: Scene<'a>,
    after: Scene<'a>,
    size: Vec2,
    follow_hover: bool,
    autoplay: bool,
    labels: Option<(String, String)>,
    handle_color: Option<Color32>,
}

impl<'a> CompareSlider<'a> {
    /// Compare two scenes painted by closures receiving `(painter, rect)`.
    pub fn new(before: impl Fn(&Painter, Rect) + 'a, after: impl Fn(&Painter, Rect) + 'a) -> Self {
        Self {
            before: Box::new(before),
            after: Box::new(after),
            size: vec2(600.0, 340.0),
            follow_hover: false,
            autoplay: true,
            labels: Some(("Before".into(), "After".into())),
            handle_color: None,
        }
    }
    /// Size in points.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = vec2(w, h);
        self
    }
    /// Move the divider with the pointer on hover (no drag needed).
    pub fn follow_hover(mut self, b: bool) -> Self {
        self.follow_hover = b;
        self
    }
    /// Sweep automatically while idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
    /// Corner labels; `None` hides them.
    pub fn labels(mut self, labels: Option<(&str, &str)>) -> Self {
        self.labels = labels.map(|(a, b)| (a.to_owned(), b.to_owned()));
        self
    }
    /// Divider and handle color (default white).
    pub fn handle_color(mut self, c: Color32) -> Self {
        self.handle_color = Some(c);
        self
    }
}

impl egui::Widget for CompareSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(self.size, Sense::click_and_drag());
        let id = resp.id;
        let active = resp.dragged() || (self.follow_hover && resp.hovered());
        let idle = idle_time(ui, id, active || resp.hovered());
        let time = glowkit_core::time(ui);
        let pos = glowkit_core::with_state(ui, id.with("pos"), || 0.5f32, |v| {
            if let Some(pp) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()).or(if self.follow_hover { resp.hover_pos() } else { None }) {
                *v = ((pp.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
            } else if self.autoplay && idle > 1.5 {
                let target = 0.5 + 0.36 * (time * 0.9).sin();
                let k = ((idle - 1.5) / 1.5).clamp(0.0, 1.0);
                *v += (target - *v) * (0.08 * k);
            }
            *v
        });
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let x = rect.min.x + rect.width() * pos;
        let left = Rect::from_min_max(rect.min, pos2(x, rect.max.y));
        let right = Rect::from_min_max(pos2(x, rect.min.y), rect.max);
        let base = ui.painter().clone();
        (self.before)(&base.with_clip_rect(left.intersect(base.clip_rect())), rect);
        (self.after)(&base.with_clip_rect(right.intersect(base.clip_rect())), rect);
        let p = ui.painter_at(rect.expand(20.0));
        let hc = self.handle_color.unwrap_or(Color32::WHITE);
        if let Some((a, b)) = &self.labels {
            let font = FontId::proportional(12.0);
            for (txt, right_side) in [(a, false), (b, true)] {
                let g = p.layout_no_wrap(txt.clone(), font.clone(), Color32::PLACEHOLDER);
                let pill = if right_side {
                    Rect::from_min_size(pos2(rect.max.x - 14.0 - g.size().x - 20.0, rect.min.y + 14.0), g.size() + vec2(20.0, 10.0))
                } else {
                    Rect::from_min_size(rect.min + vec2(14.0, 14.0), g.size() + vec2(20.0, 10.0))
                };
                let vis = if right_side { smoothstep(pill.min.x - 30.0, pill.min.x + 10.0, x) } else { smoothstep(pill.max.x + 30.0, pill.max.x - 10.0, x) };
                let a = 1.0 - vis;
                p.rect_filled(pill, 99.0, Color32::from_black_alpha((110.0 * a) as u8));
                p.rect_stroke(pill, 99.0, Stroke::new(1.0, Color32::from_white_alpha((40.0 * a) as u8)), StrokeKind::Inside);
                p.galley(pill.min + vec2(10.0, 5.0), g, with_alpha(Color32::WHITE, a));
            }
        }
        // divider
        let pclip = p.with_clip_rect(rect.intersect(p.clip_rect()));
        paint::glow_ellipse(&pclip, pos2(x, rect.center().y), vec2(14.0, rect.height() * 0.6), with_alpha(t.accent, 0.35));
        pclip.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], Stroke::new(2.0, hc));
        let hcen = pos2(x, rect.center().y);
        let grab = glowkit_core::anim::follow(ui, id.with("grab"), if resp.hovered() || resp.dragged() { 1.0 } else { 0.0 }, 12.0);
        let hr = 17.0 + grab * 3.0;
        paint::soft_shadow(&p, Rect::from_center_size(hcen, Vec2::splat(hr * 2.0)), hr, 12.0, vec2(0.0, 3.0), Color32::from_black_alpha(90));
        p.circle_filled(hcen, hr, hc);
        let ink = mix(hc, Color32::BLACK, 0.75);
        chevron(&p, hcen - vec2(5.0, 0.0), 8.0, vec2(-1.0, 0.0), Stroke::new(2.0, ink));
        chevron(&p, hcen + vec2(5.0, 0.0), 8.0, vec2(1.0, 0.0), Stroke::new(2.0, ink));
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        resp
    }
}
