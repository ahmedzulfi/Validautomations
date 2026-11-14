use super::common::{cached_ink, idle_time, lerp_c, wall_dt, Cam, Sheet, Soup, V3, PAPER, PAPER_BACK};
use super::posters::{poster_bg, poster_ink, poster_title, POSTERS};
use egui::{pos2, vec2, Align2, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug, Default)]
struct PleatState {
    offset: f32,
    vel: f32,
    open: f32,
    target: Option<f32>,
    auto_t: f32,
}

/// An endless column of generated posters joined by paper pleats and rolled round a drum.
/// Scroll or drag fast and the folds between the posters are thrown open (real zig-zag paper
/// with light and shadow on each face); at rest they close again and the nearest poster glides
/// to the centre. A title list on the side follows along and can be clicked.
///
/// Link it to a page scroll with [`Self::progress`] (`0..=1` across the whole column).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::PleatedStrip::new().count(12).radius(190.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PleatedStrip {
    count: usize,
    height: f32,
    radius: Option<f32>,
    poster: Vec2,
    pleat: f32,
    progress: Option<f32>,
    autoplay: bool,
}

impl Default for PleatedStrip {
    fn default() -> Self {
        Self::new()
    }
}

impl PleatedStrip {
    /// A strip of the built-in generated posters.
    pub fn new() -> Self {
        Self { count: 12, height: 420.0, radius: None, poster: vec2(160.0, 150.0), pleat: 22.0, progress: None, autoplay: true }
    }
    /// Number of posters (designs repeat; default 12).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.max(2);
        self
    }
    /// Widget height (default 420).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Drum radius in points (default: from the height).
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(60.0));
        self
    }
    /// Poster size in points (default 160 x 150).
    pub fn poster_size(mut self, s: Vec2) -> Self {
        self.poster = s.max(vec2(60.0, 60.0));
        self
    }
    /// Depth of each pleat fold when thrown open (default 22).
    pub fn pleat(mut self, px: f32) -> Self {
        self.pleat = px.clamp(4.0, 60.0);
        self
    }
    /// Link to an outside scroll: `0` first poster, `1` last.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Fling through the posters while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for PleatedStrip {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let n = self.count;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let poster = self.poster;
        let fold = self.pleat;
        let r = self.radius.unwrap_or(self.height * 0.44);
        let drum_x = rect.min.x + rect.width() * 0.42;
        let list = Rect::from_min_max(pos2(rect.min.x + rect.width() * 0.62, rect.min.y + 30.0), pos2(rect.max.x - 20.0, rect.max.y - 20.0));
        let row_h = ((list.height() - 70.0) / n as f32).min(22.0);
        let mut clicked_row = None;
        for i in 0..n {
            let rr = Rect::from_min_size(pos2(list.min.x, list.min.y + 70.0 + i as f32 * row_h), vec2(list.width(), row_h));
            let resp = ui.interact(rr, id.with(("row", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                clicked_row = Some(i);
            }
        }
        let drag = if response.dragged() { response.drag_delta().y } else { 0.0 };
        let dragging = response.dragged();
        let released = response.drag_stopped();
        if response.hovered() {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let wheel = if response.hovered() { ui.input(|i| i.smooth_scroll_delta.y) } else { 0.0 };
        let (progress, autoplay) = (self.progress, self.autoplay && idle > 1.5);
        let nf = n as f32;
        // positions are measured in "posters"; one poster plus its pleat is one unit
        let unit = poster.y;
        let seed = (glowkit_core::time(ui) * 3.1) as u64;
        let st = glowkit_core::with_state(ui, id, PleatState::default, |s| {
            let prev = s.offset;
            if let Some(p) = progress {
                let goal = p * (nf - 1.0);
                s.offset += (goal - s.offset) * (1.0 - (-10.0 * dt).exp());
            } else if dragging {
                s.offset -= drag / unit;
                s.vel = -drag / unit / dt.max(1e-3);
                s.target = None;
            } else {
                s.offset -= wheel / unit;
                if released {
                    s.target = Some((s.offset + s.vel * 0.35).round());
                }
                if wheel != 0.0 {
                    s.target = Some((s.offset - wheel.signum() * 0.4).round());
                }
                if let Some(i) = clicked_row {
                    let cur = s.offset.rem_euclid(nf);
                    let mut d = i as f32 - cur;
                    if d > nf * 0.5 {
                        d -= nf;
                    }
                    if d < -nf * 0.5 {
                        d += nf;
                    }
                    s.target = Some((s.offset + d).round());
                    s.auto_t = 4.0;
                }
                if autoplay {
                    s.auto_t -= wdt;
                    if s.auto_t <= 0.0 {
                        let jump = 1.0 + (glowkit_core::rng::hash01(seed) * 3.0).floor();
                        s.target = Some(s.offset.round() + jump);
                        s.auto_t = 2.6;
                    }
                }
                let goal = s.target.unwrap_or_else(|| s.offset.round());
                let k = 26.0;
                let acc = -k * (s.offset - goal) - 2.0 * k.sqrt() * 0.9 * s.vel;
                s.vel += acc * dt;
                s.offset += s.vel * dt;
            }
            let speed = ((s.offset - prev) / dt.max(1e-3)).abs();
            let want = (speed / 3.0).clamp(0.0, 1.0);
            let rate = if want > s.open { 12.0 } else { 3.0 };
            s.open += (want - s.open) * (1.0 - (-rate * dt).exp());
            if s.offset.abs() > nf * 6.0 {
                let shift = (s.offset / nf).floor() * nf;
                s.offset -= shift;
                s.target = s.target.map(|t| t - shift);
            }
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        // ---- the drum
        let pleat_len = fold * 2.0 * st.open;
        let pitch = poster.y + pleat_len + 2.0;
        let mut cam = Cam::new(pos2(drum_x, rect.center().y));
        cam.focal = 1100.0;
        cam.offset.z = -r;
        let on_drum = |a: f32, depth: f32| -> (f32, f32) {
            let th = a / r;
            let rr = r - depth;
            (rr * th.sin(), rr * th.cos())
        };
        let mut soup = Soup::new();
        let first = (st.offset - 3.0).floor() as i32;
        let last = (st.offset + 3.0).ceil() as i32;
        let w = poster.x;
        for k in first..=last {
            let i = k.rem_euclid(n as i32) as usize;
            let a0 = (k as f32 - st.offset) * pitch - poster.y * 0.5;
            // skip what is round the back
            if (a0 + poster.y) / r < -1.6 || a0 / r > 1.6 {
                continue;
            }
            let design = i % POSTERS;
            let deform = move |u: f32, v: f32| {
                let (y, z) = on_drum(a0 + v * poster.y, 0.0);
                V3 { x: (u - 0.5) * w, y, z }
            };
            let front = move |u: f32, v: f32| poster_bg(design, u, v);
            let ink = cached_ink(ui, id.with(("pleat_poster", design)), 8.0, |ink| poster_ink(ink, design, poster));
            Sheet {
                size: poster,
                cols: 6,
                rows: 16,
                deform: &deform,
                front: &front,
                back: PAPER_BACK,
                ambient: 0.35,
                gloss: 0.25,
                bias: 0.0,
                alpha: 1.0,
                ink_bias: 500.0,
            }
            .emit(&mut soup, &cam, Some(&ink));
            // the pleat after this poster: a valley folded into the drum
            if pleat_len > 0.5 {
                let start = a0 + poster.y + 1.0;
                let depth = fold * st.open;
                let pleat = move |u: f32, v: f32| {
                    let tri = 1.0 - (v * 2.0 - 1.0).abs();
                    let (y, z) = on_drum(start + v * pleat_len, depth * tri);
                    V3 { x: (u - 0.5) * w, y, z }
                };
                let paper = move |_u: f32, v: f32| lerp_c(PAPER, PAPER_BACK, v);
                Sheet {
                    size: vec2(w, pleat_len),
                    cols: 2,
                    rows: 2,
                    deform: &pleat,
                    front: &paper,
                    back: PAPER_BACK,
                    ambient: 0.3,
                    gloss: 0.1,
                    bias: -200.0,
                    alpha: 1.0,
                    ink_bias: 0.0,
                }
                .emit(&mut soup, &cam, None);
            }
        }
        // drum shadow on the floor
        super::common::contact_shadow(&painter, pos2(drum_x + 10.0, rect.max.y - 14.0), vec2(w * 0.7, 8.0), 0.18);
        soup.paint(&painter);
        // fade top and bottom into the background so the drum reads as round
        paint::fade_edges_vertical(&painter, rect, 46.0, t.background);
        // ---- title + list
        let cur = (st.offset.round().rem_euclid(nf)) as usize % n;
        painter.text(list.min, Align2::LEFT_TOP, format!("{:02} / {:02}", cur + 1, n), FontId::monospace(10.0), t.muted_foreground);
        painter.text(list.min + vec2(0.0, 16.0), Align2::LEFT_TOP, poster_title(cur % POSTERS), FontId::proportional(22.0), t.foreground);
        painter.text(list.min + vec2(0.0, 44.0), Align2::LEFT_TOP, "Poster series, 2026", FontId::proportional(11.0), t.muted_foreground);
        for i in 0..n {
            let y = list.min.y + 70.0 + i as f32 * row_h;
            let on = i == cur;
            if on {
                painter.line_segment([pos2(list.min.x, y + 3.0), pos2(list.min.x, y + row_h - 3.0)], Stroke::new(2.0, t.accent));
            }
            let c = if on { t.foreground } else { with_alpha(t.muted_foreground, 0.8) };
            painter.text(pos2(list.min.x + 10.0, y + row_h * 0.5), Align2::LEFT_CENTER, format!("{:02}", i + 1), FontId::monospace(9.5), with_alpha(c, 0.6));
            painter.text(pos2(list.min.x + 36.0, y + row_h * 0.5), Align2::LEFT_CENTER, poster_title(i % POSTERS), FontId::proportional(12.0), c);
            painter.text(pos2(list.max.x, y + row_h * 0.5), Align2::RIGHT_CENTER, format!("{}", 2026 - (i % 3) as i32), FontId::monospace(9.5), with_alpha(c, 0.6));
        }
        painter.text(pos2(list.max.x, rect.max.y - 12.0), Align2::RIGHT_BOTTOM, "Scroll, drag or click a title", FontId::proportional(10.5), t.muted_foreground);
        response
    }
}
