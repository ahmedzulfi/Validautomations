use super::common::{ghost_cursor, idle_time, sstep, wall_dt};
use egui::epaint::Mesh;
use egui::{vec2, Color32, FontId, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::paint::rounded_rect_points;
use glowkit_core::Theme;
use std::f32::consts::PI;

const N: usize = 180;

#[derive(Clone, Debug)]
struct StringState {
    d: Vec<f32>,
    v: Vec<f32>,
    ghost_t: f32,
}

/// A rounded frame drawn as an elastic string. Bring the pointer close to an edge from outside
/// and the string is plucked outward toward it, stretching thin into a glowing veil; pull away
/// and it snaps back and wobbles (a damped 1D wave travelling round the loop). Whatever you put
/// inside sits in the square, untouched.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ElasticFrame;
/// ElasticFrame::new().size(260.0).show(ui, |ui| {
///     ui.label("Anything goes in here");
/// });
/// ```
#[must_use = "add it with `ui.add(...)` or call `.show(ui, ...)`"]
pub struct ElasticFrame {
    size: f32,
    corner: f32,
    reach: f32,
    tension: f32,
    color: Option<Color32>,
    veil: Option<Color32>,
    label: String,
    height: f32,
    autoplay: bool,
}

impl Default for ElasticFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl ElasticFrame {
    /// A 260-point elastic frame.
    pub fn new() -> Self {
        Self {
            size: 260.0,
            corner: 48.0,
            reach: 90.0,
            tension: 1.0,
            color: None,
            veil: None,
            label: "Bring the pointer to the edge".into(),
            height: 400.0,
            autoplay: true,
        }
    }
    /// Side of the square in points (default 260).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s.max(80.0);
        self
    }
    /// Corner radius (default 48).
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// How far outside the edge the pointer still grabs the string (default 90).
    pub fn reach(mut self, px: f32) -> Self {
        self.reach = px.clamp(10.0, 300.0);
        self
    }
    /// String tension: higher snaps back faster with tighter ripples (default 1).
    pub fn tension(mut self, k: f32) -> Self {
        self.tension = k.clamp(0.2, 3.0);
        self
    }
    /// String color (default: the theme foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Veil color (default: the string color).
    pub fn veil(mut self, c: Color32) -> Self {
        self.veil = Some(c);
        self
    }
    /// Caption shown inside when used as a plain widget.
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// Widget height (default 400).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(self.size + 40.0);
        self
    }
    /// Pluck the edges with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the frame with `add_contents` laid out inside the square.
    pub fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let height = self.height.max(self.size + 40.0);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let sq = Rect::from_center_size(rect.center(), Vec2::splat(self.size));
        // rest outline, evenly resampled
        let rest = resample(&rounded_rect_points(sq, self.corner, 24), N);
        let normals: Vec<Vec2> = (0..N)
            .map(|i| {
                let a = rest[(i + N - 1) % N];
                let b = rest[(i + 1) % N];
                let tan = (b - a).normalized();
                vec2(tan.y, -tan.x)
            })
            .collect();
        // outward: make sure normals point away from the centre
        let normals: Vec<Vec2> = normals.iter().zip(&rest).map(|(n, p)| if n.dot(*p - sq.center()) < 0.0 { -*n } else { *n }).collect();
        let mut pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let st0: StringState = glowkit_core::state(ui, id, || StringState { d: vec![0.0; N], v: vec![0.0; N], ghost_t: 0.0 });
        let mut ghost: Option<Pos2> = None;
        if self.autoplay && idle > 2.0 {
            // circle the frame, dipping in to pluck each side and pulling out again
            let g = st0.ghost_t;
            let side = (g / 2.4).floor();
            let k = (g / 2.4).fract();
            let ang = side * PI * 0.5 + 0.35 + k * 0.6;
            let dir = vec2(ang.cos(), ang.sin());
            let reach = self.size * 0.5 + 10.0 + (k * PI).sin().powf(0.5) * 70.0 * (1.0 - sstep(0.75, 1.0, k)) + sstep(0.75, 1.0, k) * 140.0;
            let p = sq.center() + dir * reach * 1.05;
            ghost = Some(p);
            pointer = Some(p);
        }
        let reach = self.reach;
        let tension = self.tension;
        let st = glowkit_core::with_state(ui, id, || st0.clone(), |s| {
            s.ghost_t += wdt;
            // grab: points near the pointer (along the loop) are pulled toward it
            let mut target: Vec<Option<f32>> = vec![None; N];
            if let Some(p) = pointer {
                let (best, bd) = rest
                    .iter()
                    .enumerate()
                    .map(|(i, q)| (i, q.distance(p)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap_or((0, f32::MAX));
                let out = (p - rest[best]).dot(normals[best]);
                if out > -4.0 && bd < reach {
                    let pull = out.max(0.0);
                    let span = 22.0 + pull * 0.55;
                    for j in 0..N {
                        let mut di = (j as i32 - best as i32).abs() as f32;
                        di = di.min(N as f32 - di);
                        let w = (-(di / span * 4.0).powi(2) * 0.5).exp();
                        if w > 0.02 {
                            target[j] = Some(pull * w);
                        }
                    }
                }
            }
            let steps = 4;
            let h = dt / steps as f32;
            for _ in 0..steps {
                let d = s.d.clone();
                for i in 0..N {
                    let lap = d[(i + N - 1) % N] + d[(i + 1) % N] - 2.0 * d[i];
                    let vlap = s.v[(i + N - 1) % N] + s.v[(i + 1) % N] - 2.0 * s.v[i];
                    // stiffness + a little viscosity along the string so kinks smooth out
                    let mut acc = 2400.0 * tension * lap + 14.0 * vlap - 60.0 * tension * d[i] - 3.2 * s.v[i];
                    if let Some(tg) = target[i] {
                        acc += 900.0 * (tg - d[i]) - 30.0 * s.v[i];
                    }
                    s.v[i] += acc * h;
                }
                for i in 0..N {
                    s.d[i] += s.v[i] * h;
                    s.d[i] = s.d[i].clamp(-30.0, reach + 40.0);
                }
            }
            s.clone()
        });
        glowkit_core::animate(ui);
        // contents
        let inner = sq.shrink(self.corner * 0.35 + 12.0);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)));
        add_contents(&mut child);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let line = self.color.unwrap_or(t.foreground);
        let veil = self.veil.unwrap_or(line);
        let pts: Vec<Pos2> = (0..N).map(|i| rest[i] + normals[i] * st.d[i]).collect();
        // the veil: between the rest line and the stretched string, brightest at the string
        let mut m = Mesh::default();
        for i in 0..N {
            let d = st.d[i].max(0.0);
            let a = (d / 50.0).clamp(0.0, 1.0);
            m.colored_vertex(rest[i], with_alpha(veil, 0.0));
            m.colored_vertex(rest[i] + normals[i] * d * 0.55, with_alpha(veil, 0.10 * a));
            m.colored_vertex(pts[i], with_alpha(veil, 0.32 * a));
            m.colored_vertex(pts[i] + normals[i] * (6.0 + d * 0.12), with_alpha(veil, 0.0));
        }
        for i in 0..N as u32 {
            let j = (i + 1) % N as u32;
            for k in 0..3 {
                let (a, b, c, d) = (i * 4 + k, i * 4 + k + 1, j * 4 + k, j * 4 + k + 1);
                m.add_triangle(a, b, d);
                m.add_triangle(a, d, c);
            }
        }
        painter.add(egui::Shape::mesh(m));
        // the string: thins as it stretches
        let widths: Vec<f32> = (0..N).map(|i| (5.0 / (1.0 + st.d[i].abs() / 14.0)).max(1.2)).collect();
        super::common::ribbon_line(&painter, &pts, &widths, line, true);
        if let Some(g) = ghost {
            ghost_cursor(&painter, t.dark, g, 0.9, false);
        }
        response
    }
}

impl Widget for ElasticFrame {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let label = self.label.clone();
        self.show(ui, |ui| {
            ui.label(egui::RichText::new(label).font(FontId::proportional(12.5)).color(t.muted_foreground));
        })
    }
}

/// Resample a closed polyline into `n` evenly spaced points.
fn resample(pts: &[Pos2], n: usize) -> Vec<Pos2> {
    let m = pts.len();
    let mut cum = Vec::with_capacity(m + 1);
    cum.push(0.0);
    for i in 0..m {
        let l = cum[i] + pts[i].distance(pts[(i + 1) % m]);
        cum.push(l);
    }
    let total = cum[m];
    let mut out = Vec::with_capacity(n);
    let mut seg = 0;
    for k in 0..n {
        let s = k as f32 / n as f32 * total;
        while seg < m - 1 && cum[seg + 1] < s {
            seg += 1;
        }
        let a = pts[seg];
        let b = pts[(seg + 1) % m];
        let len = (cum[seg + 1] - cum[seg]).max(1e-4);
        out.push(a.lerp(b, (s - cum[seg]) / len));
    }
    out
}
