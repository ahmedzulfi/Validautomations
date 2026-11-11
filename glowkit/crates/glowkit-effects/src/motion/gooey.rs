use super::common::{fill_field, sd_round_rect, smin};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Response, Sense, Ui, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::{contrast_text, mix};
use glowkit_core::{Rng, Theme};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy)]
struct Particle {
    dir: egui::Vec2,
    dist: f32,
    radius: f32,
    color: Color32,
    delay: f32,
}

#[derive(Clone, Default)]
struct GooState {
    burst_at: f32,
    center: Pos2,
    particles: Vec<Particle>,
    seed: u64,
}

/// A navigation bar whose active pill slides to the clicked item while a burst of colorful
/// particles shoots out and melts back into it, gooey style.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut active = 0;
/// ui.add(glowkit_effects::GooeyNav::new(&mut active, &["Home", "About", "Contact"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GooeyNav<'a> {
    selected: &'a mut usize,
    labels: Vec<String>,
    particles: usize,
    colors: Option<Vec<Color32>>,
    pill: Option<Color32>,
    duration: f32,
    autoplay: Option<f32>,
}

impl<'a> GooeyNav<'a> {
    /// A gooey nav over `labels` editing the `selected` index.
    pub fn new<S: AsRef<str>>(selected: &'a mut usize, labels: &[S]) -> Self {
        Self { selected, labels: labels.iter().map(|s| s.as_ref().to_owned()).collect(), particles: 12, colors: None, pill: None, duration: 0.9, autoplay: None }
    }
    /// Particles per burst.
    pub fn particles(mut self, n: usize) -> Self {
        self.particles = n.min(60);
        self
    }
    /// Particle colors. Defaults to the theme chart palette.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }
    /// Active pill color. Defaults to the theme foreground.
    pub fn pill_color(mut self, c: Color32) -> Self {
        self.pill = Some(c);
        self
    }
    /// Burst duration in seconds.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.clamp(0.2, 3.0);
        self
    }
    /// Advance to the next item every `seconds` while the pointer is away.
    pub fn autoplay(mut self, seconds: f32) -> Self {
        self.autoplay = Some(seconds.max(0.5));
        self
    }
}

impl Widget for GooeyNav<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.labels.len();
        let font = FontId::proportional(15.0);
        let h = 42.0;
        let pad_x = 20.0;
        let widths: Vec<f32> = self.labels.iter().map(|l| ui.painter().layout_no_wrap(l.clone(), font.clone(), Color32::WHITE).size().x + pad_x * 2.0).collect();
        let total: f32 = widths.iter().sum();
        let margin = 70.0;
        let (rect, mut response) = ui.allocate_exact_size(vec2(total + margin * 2.0, h + margin * 2.0), Sense::click());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let mut items = Vec::with_capacity(n);
        let mut x = rect.min.x + margin;
        for w in &widths {
            items.push(Rect::from_min_size(pos2(x, rect.min.y + margin), vec2(*w, h)));
            x += w;
        }
        let hovered = response.hover_pos().and_then(|p| items.iter().position(|r| r.contains(p)));
        if hovered.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let now = glowkit_core::time(ui);
        let mut changed_to = None;
        if response.clicked() {
            if let Some(i) = hovered {
                if i != *self.selected {
                    changed_to = Some(i);
                }
            }
        }
        if let Some(every) = self.autoplay {
            if !response.hovered() {
                let last: f32 = glowkit_core::state(ui, id.with("auto"), || now);
                if now - last >= every {
                    changed_to = Some((*self.selected + 1) % n);
                    glowkit_core::set_state(ui, id.with("auto"), now);
                }
                glowkit_core::animate(ui);
            } else {
                glowkit_core::set_state(ui, id.with("auto"), now);
            }
        }
        let colors = self.colors.clone().unwrap_or_else(|| t.chart.to_vec());
        if let Some(i) = changed_to {
            *self.selected = i;
            response.mark_changed();
            let count = self.particles;
            glowkit_core::with_state(ui, id.with("goo"), GooState::default, |g| {
                g.seed += 1;
                let mut rng = Rng::new(g.seed * 7919 + 3);
                g.burst_at = now;
                g.center = items[i].center();
                g.particles = (0..count)
                    .map(|k| {
                        let a = k as f32 / count as f32 * TAU + rng.range(-0.2, 0.2);
                        Particle {
                            dir: vec2(a.cos(), a.sin() * 0.7),
                            dist: rng.range(55.0, 95.0),
                            radius: rng.range(3.5, 7.0),
                            color: colors[rng.below(colors.len())],
                            delay: rng.range(0.0, 0.15),
                        }
                    })
                    .collect();
            });
        }
        let sel = (*self.selected).min(n - 1);
        let target = items[sel];
        let l = glowkit_core::anim::spring(ui, id.with("l"), target.min.x, 300.0, 26.0);
        let r = glowkit_core::anim::spring(ui, id.with("r"), target.max.x, 300.0, 26.0);
        let pill = Rect::from_min_max(pos2(l, target.min.y), pos2(r.max(l + 4.0), target.max.y));
        let goo: GooState = glowkit_core::state(ui, id.with("goo"), GooState::default);
        let age = now - goo.burst_at;
        let active = !goo.particles.is_empty() && age < self.duration + 0.2;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let pill_col = self.pill.unwrap_or(t.foreground);
        // pill "pops" when it lands
        let pop = if active { 1.0 + 0.08 * (age / self.duration * PI).sin().max(0.0) * (1.0 - age / self.duration).max(0.0) } else { 1.0 };
        let pill_draw = Rect::from_center_size(pill.center(), pill.size() * pop);
        if active {
            ui.ctx().request_repaint();
            let dur = self.duration;
            let ps: Vec<(Pos2, f32, Color32)> = goo
                .particles
                .iter()
                .filter_map(|p| {
                    let lt = ((age - p.delay) / (dur - p.delay)).clamp(0.0, 1.0);
                    if lt <= 0.0 || lt >= 1.0 {
                        return None;
                    }
                    // out, hang, and fall back in
                    let d = (lt * PI).sin().powf(0.8) * p.dist;
                    let rad = p.radius * (1.0 - ease(Easing::InCubic, lt) * 0.7);
                    Some((goo.center + p.dir * d, rad, p.color))
                })
                .collect();
            let bounds = pill_draw.expand(80.0).intersect(rect);
            let k = 9.0;
            let pr = pill_draw.height() / 2.0;
            let field = |q: Pos2| {
                let mut d = sd_round_rect(q, pill_draw, pr);
                for (c, rad, _) in &ps {
                    d = smin(d, q.distance(*c) - rad, k);
                }
                d
            };
            let color = |q: Pos2| {
                let mut best = pill_col;
                let mut bd = sd_round_rect(q, pill_draw, pr).max(0.0) + 6.0;
                for (c, rad, col) in &ps {
                    let d = q.distance(*c) - rad;
                    if d < bd {
                        bd = d;
                        best = *col;
                    }
                }
                best
            };
            fill_field(&painter, bounds, 2.5, field, color);
        }
        // the pill itself is always drawn crisp on top of the goo
        painter.rect_filled(pill_draw, CornerRadius::same((pill_draw.height() / 2.0) as u8), pill_col);
        let on_col = contrast_text(pill_col);
        for (i, it) in items.iter().enumerate() {
            let overlap = (pill.max.x.min(it.max.x) - pill.min.x.max(it.min.x)).max(0.0) / it.width();
            let base = if hovered == Some(i) { t.foreground } else { t.muted_foreground };
            painter.text(it.center(), Align2::CENTER_CENTER, &self.labels[i], font.clone(), mix(base, on_col, overlap.clamp(0.0, 1.0)));
        }
        response
    }
}
