use super::common::{idle_time, lerp_c, sstep, wall_dt};
use egui::epaint::text::cursor::CCursor;
use egui::epaint::Mesh;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Ui, Vec2, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::noise::noise3;
use glowkit_core::{hex, Theme};

/// A paragraph that inks in word by word as the reader scrolls. Every word starts faint and
/// darkens as the reading line reaches it, with a soft leading edge rather than a hard cut.
/// Words wrapped in `*asterisks*` light up in an accent colour with a highlighter swept in
/// behind them.
///
/// Inside a `ScrollArea` it follows the viewport on its own; set [`Self::progress`] to drive it
/// from anything else.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::InkReveal::new(
///     "Most interfaces sit perfectly still. This one *waits* for you.",
/// ));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct InkReveal {
    text: String,
    font: FontId,
    width: Option<f32>,
    progress: Option<f32>,
    softness: f32,
    ink: Option<Color32>,
    faint: Option<Color32>,
    accent: Option<Color32>,
    highlight: Option<Color32>,
}

impl InkReveal {
    /// A paragraph; `*word*` marks accented words.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: FontId::proportional(26.0),
            width: None,
            progress: None,
            softness: 0.12,
            ink: None,
            faint: None,
            accent: None,
            highlight: None,
        }
    }
    /// Font (default proportional 26).
    pub fn font(mut self, f: FontId) -> Self {
        self.font = f;
        self
    }
    /// Wrap width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Drive the reveal directly (`0` nothing inked, `1` all of it).
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Width of the soft leading edge as a fraction of the text (default 0.12).
    pub fn softness(mut self, k: f32) -> Self {
        self.softness = k.clamp(0.01, 0.6);
        self
    }
    /// Fully inked text color (default: theme foreground).
    pub fn ink(mut self, c: Color32) -> Self {
        self.ink = Some(c);
        self
    }
    /// Not-yet-inked color (default: a faint foreground).
    pub fn faint(mut self, c: Color32) -> Self {
        self.faint = Some(c);
        self
    }
    /// Accent word color (default: amber on dark themes, the theme accent on light ones).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Highlighter color behind accent words.
    pub fn highlight(mut self, c: Color32) -> Self {
        self.highlight = Some(c);
        self
    }
}

impl Widget for InkReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width());
        // words (keeping their trailing spaces) and which ones are accented
        let mut words: Vec<(String, bool)> = Vec::new();
        for raw in self.text.split_inclusive(' ') {
            let acc = raw.trim().starts_with('*')
                && raw.trim().trim_end_matches(['.', ',', '!', '?', ';', ':']).ends_with('*');
            words.push((raw.replace('*', ""), acc));
        }
        let n = words.len().max(1);
        let ink = self.ink.unwrap_or(t.foreground);
        let faint = self.faint.unwrap_or(with_alpha(t.foreground, if t.dark { 0.16 } else { 0.14 }));
        let accent = self.accent.unwrap_or(if t.dark { hex("#FBBF24") } else { t.accent });
        let hl = self.highlight.unwrap_or(if t.dark {
            with_alpha(hex("#F59E0B"), 0.35)
        } else {
            with_alpha(hex("#FDE047"), 0.75)
        });
        // layout once with placeholder colors to measure
        let layout_painter = ui.painter().clone();
        let build = |colors: &dyn Fn(usize) -> Color32| {
            let mut job = LayoutJob::default();
            job.wrap.max_width = width;
            for (i, (w, _)) in words.iter().enumerate() {
                job.append(
                    w,
                    0.0,
                    TextFormat {
                        font_id: self.font.clone(),
                        color: colors(i),
                        line_height: Some(self.font.size * 1.25),
                        ..Default::default()
                    },
                );
            }
            layout_painter.layout_job(job)
        };
        let measure = build(&|_| ink);
        let (rect, response) = ui.allocate_exact_size(vec2(width, measure.size().y), Sense::hover());
        let p = self.progress.unwrap_or_else(|| {
            let clip = ui.clip_rect();
            let line = clip.min.y + clip.height() * 0.72;
            ((line - rect.min.y) / (rect.height() + clip.height() * 0.3).max(1.0)).clamp(0.0, 1.0)
        });
        let soft = self.softness;
        let amount = |i: usize| {
            let at = i as f32 / n as f32;
            ((p * (1.0 + soft) - at) / soft).clamp(0.0, 1.0)
        };
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        // highlighter sweeps behind accent words
        let mut char_idx = 0usize;
        for (i, (w, acc)) in words.iter().enumerate() {
            let len = w.chars().count();
            if *acc {
                let k = sstep(0.55, 1.0, amount(i));
                if k > 0.0 {
                    let word_len = w.trim_end().chars().count();
                    let a = measure.pos_from_cursor(CCursor { prefer_next_row: true, ..CCursor::new(char_idx) });
                    let b = measure
                        .pos_from_cursor(CCursor { prefer_next_row: false, ..CCursor::new(char_idx + word_len) });
                    if (a.min.y - b.min.y).abs() < 1.0 {
                        let h = a.height();
                        let r = Rect::from_min_max(
                            pos2(a.min.x - 2.0, a.min.y + h * 0.52),
                            pos2(a.min.x + (b.min.x - a.min.x + 4.0) * k, a.max.y - h * 0.08),
                        );
                        painter.rect_filled(r.translate(rect.min.to_vec2()), 3.0, hl);
                    }
                }
            }
            char_idx += len;
        }
        let colored = build(&|i| {
            let k = amount(i);
            let target = if words[i].1 { accent } else { ink };
            lerp_c(faint, target, k * k * (3.0 - 2.0 * k))
        });
        painter.galley(rect.min, colored, ink);
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct TrailPoint {
    pos: Pos2,
    age: f32,
}

#[derive(Clone, Debug, Default)]
struct TrailState {
    pts: Vec<TrailPoint>,
    last: Option<Pos2>,
    t: f32,
}

/// A surface the cursor paints into: a glowing trail of ink that drifts on a slow noise current
/// and fades away. On dark themes it burns like an ember; on light ones it reads as wet ink.
/// While nobody is pointing at it, an invisible hand keeps writing loops so it is never empty.
/// Paint it behind other content with [`Self::paint`] (it never takes clicks).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::InkTrail::new().text("Leave a mark.").life(1.8));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct InkTrail {
    colors: Option<[Color32; 3]>,
    width: f32,
    life: f32,
    drift: f32,
    text: Option<String>,
    size: Option<Vec2>,
    autoplay: bool,
}

impl Default for InkTrail {
    fn default() -> Self {
        Self::new()
    }
}

impl InkTrail {
    /// A trail with theme-aware colors.
    pub fn new() -> Self {
        Self { colors: None, width: 26.0, life: 1.6, drift: 1.0, text: None, size: None, autoplay: true }
    }
    /// Colors: `[core, body, halo]` (hot center to faint outer glow).
    pub fn colors(mut self, c: [Color32; 3]) -> Self {
        self.colors = Some(c);
        self
    }
    /// Stroke width at the head (default 26).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(2.0, 120.0);
        self
    }
    /// Seconds a mark takes to fade (default 1.6).
    pub fn life(mut self, s: f32) -> Self {
        self.life = s.clamp(0.2, 6.0);
        self
    }
    /// Strength of the noise current (default 1).
    pub fn drift(mut self, k: f32) -> Self {
        self.drift = k.clamp(0.0, 4.0);
        self
    }
    /// Text shown over the surface.
    pub fn text(mut self, s: impl Into<String>) -> Self {
        self.text = Some(s.into());
        self
    }
    /// Widget size (default: available width x 340).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Keep drawing loops while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Paint the trail into `rect` (state keyed by `id`), e.g. behind other widgets.
    pub fn paint(&self, ui: &Ui, rect: Rect, id: egui::Id) {
        let t = Theme::of(ui);
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let (life, drift, autoplay) = (self.life, self.drift, self.autoplay && idle > 1.2);
        let st = glowkit_core::with_state(ui, id, TrailState::default, |s| {
            s.t += wdt;
            let tt = s.t;
            let head = match pointer {
                Some(p) if !autoplay || idle <= 1.2 => Some(p),
                _ if autoplay => {
                    // a looping hand: two incommensurate figure-eights
                    let c = rect.center();
                    Some(pos2(
                        c.x + (tt * 0.9).sin() * rect.width() * 0.32 + (tt * 2.3).sin() * 40.0,
                        c.y + (tt * 1.7).sin() * rect.height() * 0.28 + (tt * 3.1).cos() * 18.0,
                    ))
                }
                _ => None,
            };
            for p in &mut s.pts {
                p.age += dt;
                let n = vec2(
                    noise3(p.pos.x * 0.006, p.pos.y * 0.006, tt * 0.2),
                    noise3(p.pos.x * 0.006 + 31.0, p.pos.y * 0.006, tt * 0.2),
                );
                p.pos += n * 90.0 * drift * dt * sstep(0.0, 0.4, p.age);
            }
            s.pts.retain(|p| p.age < life);
            if let Some(h) = head {
                let from = s.last.unwrap_or(h);
                let d = from.distance(h);
                let steps = ((d / 5.0).ceil() as usize).clamp(1, 40);
                for k in 1..=steps {
                    let q = from.lerp(h, k as f32 / steps as f32);
                    s.pts.push(TrailPoint { pos: q, age: dt * (1.0 - k as f32 / steps as f32) });
                }
                if s.pts.len() > 600 {
                    let extra = s.pts.len() - 600;
                    s.pts.drain(0..extra);
                }
                s.last = Some(h);
            } else {
                s.last = None;
            }
            s.clone()
        });
        glowkit_core::animate(ui);
        let painter = ui.painter_at(rect);
        let colors = self.colors.unwrap_or(if t.dark {
            [hex("#FFF1DC"), hex("#FF6A1A"), hex("#8A1C05")]
        } else {
            [hex("#1E1B4B"), hex("#4338CA"), hex("#A5B4FC")]
        });
        if st.pts.len() < 2 {
            return;
        }
        // one ribbon mesh with a soft cross-section: halo | body | core | body | halo
        let mut m = Mesh::default();
        let n = st.pts.len();
        // smooth the drifted centre line so the ribbon edges stay clean
        let mut line: Vec<Pos2> = st.pts.iter().map(|p| p.pos).collect();
        for _ in 0..3 {
            let prev = line.clone();
            for i in 1..n - 1 {
                if prev[i - 1].distance(prev[i + 1]) < 60.0 {
                    line[i] = pos2(
                        (prev[i - 1].x + 2.0 * prev[i].x + prev[i + 1].x) * 0.25,
                        (prev[i - 1].y + 2.0 * prev[i].y + prev[i + 1].y) * 0.25,
                    );
                }
            }
        }
        for i in 0..n {
            let p = TrailPoint { pos: line[i], ..st.pts[i] };
            let prev = line[i.saturating_sub(4)];
            let next = line[(i + 4).min(n - 1)];
            let dir = next - prev;
            let nrm = if dir.length() > 1e-3 { vec2(-dir.y, dir.x).normalized() } else { vec2(0.0, 1.0) };
            let k = 1.0 - p.age / life;
            let w = self.width * (0.25 + 0.75 * k.powf(0.7));
            let fade = sstep(0.0, 0.35, k);
            let core = with_alpha(lerp_c(colors[1], colors[0], k.powf(1.5)), fade);
            let body = with_alpha(lerp_c(colors[2], colors[1], k), 0.85 * fade);
            let halo = with_alpha(colors[2], 0.0);
            let lanes = [(-1.9, halo), (-0.9, body), (0.0, core), (0.9, body), (1.9, halo)];
            for (o, c) in lanes {
                m.colored_vertex(p.pos + nrm * o * w * 0.5, c);
            }
        }
        for i in 0..n as u32 - 1 {
            if st.pts[i as usize].pos.distance(st.pts[i as usize + 1].pos) > 60.0 {
                continue; // a jump (pointer left and came back)
            }
            for l in 0..4u32 {
                let a = i * 5 + l;
                let b = (i + 1) * 5 + l;
                m.add_triangle(a, a + 1, b + 1);
                m.add_triangle(a, b + 1, b);
            }
        }
        painter.add(Shape::mesh(m));
        if let Some(h) = st.pts.last() {
            glowkit_core::paint::glow(
                &painter,
                h.pos,
                self.width * 1.3,
                with_alpha(colors[1], if t.dark { 0.45 } else { 0.25 }),
            );
            painter.circle_filled(h.pos, self.width * 0.42, colors[0]);
        }
    }
}

impl Widget for InkTrail {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 340.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let bg = if t.dark { hex("#0B0B0D") } else { hex("#FAFAF7") };
            ui.painter().rect_filled(rect, t.radius * 1.4, bg);
            self.paint(ui, rect, response.id);
            if let Some(text) = &self.text {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    FontId::proportional(42.0),
                    with_alpha(t.foreground, 0.92),
                );
            }
        }
        response
    }
}
