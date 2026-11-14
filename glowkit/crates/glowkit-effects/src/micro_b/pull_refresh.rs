use super::common::{cr, ghost_cursor, hairline, idle_for, ink, shadow_color, surface};
use egui::{pos2, vec2, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2};
use glowkit_core::anim::{smoothstep, Spring};
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
struct PullState {
    pull: Spring,
    grab: bool,
    refreshing: Option<f64>,
    ghost: f32,
}

/// What [`PullToRefresh::show`] returns.
pub struct PullOutput<R> {
    /// The card's response.
    pub response: Response,
    /// `true` on the frame a refresh finishes: reload your data now.
    pub refreshed: bool,
    /// Whatever the content closure returned.
    pub inner: R,
}

/// A card you pull down to refresh. Dragging stretches the card downward with rubber-band
/// resistance while a ring of dots fills in one by one in the gap above the content. Let go past
/// the line and the dots spin while the card holds open, then everything springs back and
/// [`PullOutput::refreshed`] fires. Let go early and it just snaps back.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let out = glowkit_effects::PullToRefresh::new().show(ui, |ui, rect| {
///     ui.label("Pull me");
/// });
/// if out.refreshed { /* reload */ }
/// ```
#[must_use = "call `.show(ui, ...)`"]
pub struct PullToRefresh {
    size: Vec2,
    corner: f32,
    dots: usize,
    spin: f32,
    dark: bool,
    stroke: bool,
    max_pull: f32,
    autoplay: bool,
}

impl Default for PullToRefresh {
    fn default() -> Self {
        Self::new()
    }
}

impl PullToRefresh {
    /// A pull-to-refresh card.
    pub fn new() -> Self {
        Self {
            size: vec2(300.0, 196.0),
            corner: 20.0,
            dots: 6,
            spin: 0.5,
            dark: false,
            stroke: true,
            max_pull: 120.0,
            autoplay: true,
        }
    }
    /// Resting card size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.max(vec2(80.0, 60.0));
        self
    }
    /// Corner radius.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Number of dots in the spinner.
    pub fn dots(mut self, n: usize) -> Self {
        self.dots = n.clamp(3, 16);
        self
    }
    /// Spinner speed (0..1).
    pub fn spin(mut self, k: f32) -> Self {
        self.spin = k.clamp(0.0, 1.0);
        self
    }
    /// Dark (ink) card.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outline (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Maximum stretch in points.
    pub fn max_pull(mut self, px: f32) -> Self {
        self.max_pull = px.max(30.0);
        self
    }
    /// Demonstrate the pull with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Paint the card and `add` its content (given the content rect, which moves down as the
    /// card is pulled). The stage reserves room for the stretch below the card.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui, Rect) -> R) -> PullOutput<R> {
        let t = Theme::of(ui);
        let total = vec2(self.size.x, self.size.y + self.max_pull);
        let (outer, _) = ui.allocate_exact_size(total, Sense::hover());
        let base = Rect::from_min_size(outer.min, self.size);
        let id = ui.scope_id().with(("glowkit_pull", outer.min.x as i32));
        let st0: PullState = glowkit_core::state(ui, id, || PullState {
            pull: Spring::new(0.0),
            grab: false,
            refreshing: None,
            ghost: -1.0,
        });
        let card = Rect::from_min_size(base.min, vec2(base.width(), base.height() + st0.pull.value.max(0.0)));
        let response = ui.interact(card, id.with("card"), Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let now = ui.input(|i| i.time);
        let dt = glowkit_core::dt(ui);
        let idle = idle_for(ui, id, response.hovered() || response.dragged());
        let drag = response.drag_delta().y;
        let (started, dragging, released) = (response.drag_started(), response.dragged(), response.drag_stopped());
        let line = self.max_pull * 0.55;
        let max = self.max_pull;
        let auto = self.autoplay && idle > 3.0;
        let mut refreshed = false;
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if started && s.refreshing.is_none() {
                    s.grab = true;
                    s.ghost = -1.0;
                    s.pull.target = -max * (1.0 - (s.pull.value.max(0.0) / max).min(0.99)).ln();
                }
                if s.grab && dragging {
                    // rubber band: track the raw pull distance in `target`, show a resisted value
                    s.pull.target += drag;
                    let raw = s.pull.target.max(0.0);
                    s.pull.value = max * (1.0 - (-raw / max).exp());
                    s.pull.velocity = 0.0;
                } else if s.refreshing.is_none() && s.ghost < 0.0 && auto {
                    s.ghost = 0.0;
                }
                if s.ghost >= 0.0 && !s.grab {
                    s.ghost += dt;
                    let g = s.ghost;
                    if (0.6..1.6).contains(&g) {
                        let k = smoothstep(0.6, 1.6, g);
                        s.pull.value = max * 0.72 * k;
                        s.pull.target = s.pull.value;
                        s.pull.velocity = 0.0;
                    } else if g >= 1.6 && s.refreshing.is_none() && g < 1.7 {
                        s.refreshing = Some(now);
                    }
                    if g > 4.6 {
                        s.ghost = -1.0;
                    }
                }
                if released && s.grab {
                    s.grab = false;
                    if s.pull.value > line {
                        s.refreshing = Some(now);
                    }
                }
                if let Some(at) = s.refreshing {
                    if now - at > 1.3 {
                        s.refreshing = None;
                        refreshed = true;
                    }
                }
                let ghost_holding = (0.6..1.6).contains(&s.ghost);
                if !s.grab && !ghost_holding {
                    s.pull.target = if s.refreshing.is_some() { line * 0.62 } else { 0.0 };
                    s.pull.stiffness = 260.0;
                    s.pull.damping = if s.refreshing.is_some() { 20.0 } else { 15.0 };
                    s.pull.step(dt);
                }
                *s
            },
        );
        if !st.pull.is_settled() || st.refreshing.is_some() || st.ghost >= 0.0 || st.grab || auto {
            glowkit_core::animate(ui);
        }
        let pull = st.pull.value.max(0.0);
        let card = Rect::from_min_size(base.min, vec2(base.width(), base.height() + pull));
        let painter = ui.painter().clone();
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, fg) = if self.dark { (ink_bg, ink_fg) } else { (surface(&t), t.foreground) };
        paint::soft_shadow(&painter, card, self.corner, 20.0, vec2(0.0, 5.0), shadow_color(&t, 0.7));
        painter.rect(
            card,
            cr(self.corner),
            bg,
            if self.stroke { hairline(&t) } else { Stroke::NONE },
            egui::StrokeKind::Inside,
        );
        // spinner of dots in the opening gap
        let progress = (pull / line).clamp(0.0, 1.0);
        let spin_c = pos2(card.center().x, card.min.y + 8.0 + pull * 0.5);
        let time = glowkit_core::time(ui);
        let refreshing = st.refreshing.is_some();
        let rot = if refreshing { time * (2.0 + 8.0 * self.spin) } else { pull * 0.02 };
        let n = self.dots;
        let r = 9.0 * (0.6 + 0.4 * progress);
        for i in 0..n {
            let appear = ((progress * n as f32) - i as f32).clamp(0.0, 1.0);
            if appear <= 0.0 {
                continue;
            }
            let a = rot + i as f32 / n as f32 * TAU - TAU / 4.0;
            let lead = if refreshing { 0.35 + 0.65 * ((i as f32 / n as f32 + time * 1.2) % 1.0) } else { 0.85 };
            let p = spin_c + vec2(a.cos(), a.sin()) * r;
            painter.circle_filled(p, 1.9 * appear, with_alpha(fg, lead * appear * 0.8));
        }
        // content follows the stretch
        let content = Rect::from_min_size(base.min + vec2(0.0, pull), base.size());
        let clip = card.shrink(1.0);
        let inner = ui
            .scope_builder(UiBuilder::new().max_rect(content), |ui| {
                ui.set_clip_rect(clip.intersect(ui.clip_rect()));
                add(ui, content)
            })
            .inner;
        if st.ghost >= 0.0 {
            let g = st.ghost;
            let a = smoothstep(0.2, 0.5, g) * (1.0 - smoothstep(1.7, 2.1, g));
            let tip = pos2(card.center().x + 40.0, base.min.y + 40.0 + pull);
            ghost_cursor(&painter, &t, tip, a, (0.55..1.6).contains(&g));
        }
        PullOutput { response, refreshed, inner }
    }
}
