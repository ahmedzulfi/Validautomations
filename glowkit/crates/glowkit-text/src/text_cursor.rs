use crate::util;
use egui::{pos2, vec2, Color32, Id, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::{ease, Easing, Rng, Theme};

#[derive(Clone, Copy, Debug)]
struct Stamp {
    pos: Pos2,
    vel: Vec2,
    born: f32,
    angle: f32,
    spin: f32,
    scale: f32,
    color: usize,
}

#[derive(Clone, Debug, Default)]
struct TrailState {
    stamps: Vec<Stamp>,
    last: Option<Pos2>,
    seed: u64,
}

/// A trail of little text stamps (an emoji, a word, a symbol) that pops out behind the pointer,
/// wobbles, drifts and fades away. When nobody is pointing at it, a ghost pointer draws loops
/// so the area is never empty.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_text::TextCursor::new("✦").size(28.0));
/// // or over your own content:
/// # let rect = egui::Rect::NOTHING;
/// glowkit_text::TextCursor::new("❤").paint(ui, rect);
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
pub struct TextCursor {
    text: String,
    size: f32,
    spacing: f32,
    max_points: usize,
    life: f32,
    colors: Vec<Color32>,
    random_float: bool,
    autoplay: bool,
    area: Option<Vec2>,
    salt: Option<Id>,
}

impl TextCursor {
    /// A trail of `text` (an emoji or short string).
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 30.0,
            spacing: 28.0,
            max_points: 32,
            life: 1.6,
            colors: Vec::new(),
            random_float: true,
            autoplay: true,
            area: None,
            salt: None,
        }
    }
    /// Font size of each stamp in points (default 30).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Distance the pointer travels between stamps, in points (default 28).
    pub fn spacing(mut self, pts: f32) -> Self {
        self.spacing = pts.max(4.0);
        self
    }
    /// Maximum stamps alive at once (default 32).
    pub fn max_points(mut self, n: usize) -> Self {
        self.max_points = n.max(1);
        self
    }
    /// Seconds each stamp lives (default 1.6).
    pub fn life(mut self, seconds: f32) -> Self {
        self.life = seconds.max(0.1);
        self
    }
    /// Stamp colors, cycled (default: the theme chart palette).
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    /// Let stamps drift and spin randomly as they fade (default true).
    pub fn random_float(mut self, on: bool) -> Self {
        self.random_float = on;
        self
    }
    /// Draw with a ghost pointer while the real one is away (default true).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Size when added as a widget (default: available width x 300).
    pub fn area(mut self, size: Vec2) -> Self {
        self.area = Some(size);
        self
    }
    /// Salt for the state id; needed when several trails are painted in one `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.salt = Some(Id::NULL.with(salt));
        self
    }

    /// Paint the trail over `rect`, following the pointer while it is inside.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let id = match self.salt {
            Some(s) => ui.scope_id().with(("glowkit_text_cursor", s)),
            None => ui.scope_id().with("glowkit_text_cursor"),
        };
        self.paint_with_id(ui, rect, id);
    }

    fn paint_with_id(&self, ui: &Ui, rect: Rect, id: Id) {
        let theme = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let hover = if ui.rect_contains_pointer(rect) {
            ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p))
        } else {
            None
        };
        let ghost = || {
            // a lazy figure-eight across the area
            let a = time * 0.9;
            let c = rect.center();
            pos2(c.x + rect.width() * 0.36 * a.sin(), c.y + rect.height() * 0.28 * (2.0 * a).sin())
        };
        let target = hover.or_else(|| self.autoplay.then(ghost));
        let (spacing, max, life, size, float) =
            (self.spacing, self.max_points, self.life, self.size, self.random_float);
        let ncolors = if self.colors.is_empty() { theme.chart.len() } else { self.colors.len() };
        let st = glowkit_core::with_state(ui, id, TrailState::default, |s| {
            s.stamps.retain(|p| time - p.born < life);
            for p in &mut s.stamps {
                p.pos += p.vel * dt;
                p.vel *= (1.0 - 1.6 * dt).max(0.0);
                p.angle += p.spin * dt;
            }
            match (target, s.last) {
                (Some(t), Some(last)) => {
                    let d = t.distance(last);
                    if d >= spacing {
                        let steps = ((d / spacing) as usize).min(12);
                        let dir = (t - last) / d;
                        for k in 1..=steps {
                            s.seed = s.seed.wrapping_add(1);
                            let mut r = Rng::new(s.seed);
                            let p = last + dir * spacing * k as f32;
                            let drift =
                                if float { vec2(r.range(-30.0, 30.0), r.range(-55.0, -15.0)) } else { Vec2::ZERO };
                            s.stamps.push(Stamp {
                                pos: p,
                                vel: drift,
                                born: time,
                                angle: if float { r.range(-0.5, 0.5) } else { 0.0 },
                                spin: if float { r.range(-1.5, 1.5) } else { 0.0 },
                                scale: if float { r.range(0.75, 1.15) } else { 1.0 },
                                color: s.seed as usize % ncolors,
                            });
                        }
                        s.last = Some(last + dir * spacing * steps as f32);
                        if steps == 12 {
                            s.last = Some(t);
                        }
                    }
                }
                (Some(t), None) => s.last = Some(t),
                (None, _) => s.last = None,
            }
            if s.stamps.len() > max {
                let extra = s.stamps.len() - max;
                s.stamps.drain(..extra);
            }
            s.clone()
        });
        if target.is_some() || !st.stamps.is_empty() {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let painter = ui.painter_at(rect);
        let font = util::font(size, false);
        let galley = util::layout(&painter, self.text.as_str(), &font);
        for p in &st.stamps {
            let age = ((time - p.born) / life).clamp(0.0, 1.0);
            let pop = ease(Easing::OutBack, (age / 0.18).min(1.0));
            let out = ease(Easing::InCubic, ((age - 0.45) / 0.55).clamp(0.0, 1.0));
            let scale = p.scale * pop * (1.0 - 0.6 * out);
            let color = if self.colors.is_empty() { theme.chart[p.color] } else { self.colors[p.color] };
            util::paint_galley_xf(&painter, &galley, p.pos, scale, p.angle, color.gamma_multiply(1.0 - out), 0.0);
        }
    }
}

impl Widget for TextCursor {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.area, 300.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        let id = match self.salt {
            Some(s) => resp.id.with(s),
            None => resp.id,
        };
        self.paint_with_id(ui, rect, id);
        resp
    }
}
