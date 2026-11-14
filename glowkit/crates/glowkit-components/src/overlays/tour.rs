use super::kit::{self, Btn};
use super::popover::place;
use super::{LineIcon, Placement};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Key, Order, Pos2, Rect, Sense, Shape, Stroke, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// One stop of a [`SpotlightTour`].
#[derive(Clone, Debug)]
pub struct TourStep {
    /// The screen rect to highlight (e.g. a widget's `response.rect`).
    pub target: Rect,
    /// Card heading.
    pub title: String,
    /// Card body.
    pub body: String,
    /// Which side of the target the card prefers.
    pub placement: Placement,
}

impl TourStep {
    /// A step highlighting `target`.
    pub fn new(target: Rect, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { target, title: title.into(), body: body.into(), placement: Placement::Bottom }
    }
    /// Preferred card side.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
}

/// A guided product tour / coachmark: everything except the current target dims, a glowing
/// cut-out glides between targets on a spring, and a step card with Back / Next / progress dots
/// sits next to it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (search, new_btn): (egui::Response, egui::Response) = todo!();
/// # let mut step = Some(0);
/// use glowkit_components::{SpotlightTour, TourStep};
/// SpotlightTour::new(&mut step, vec![
///     TourStep::new(search.rect, "Search", "Find anything with ⌘K."),
///     TourStep::new(new_btn.rect, "Create", "Start a new project here."),
/// ]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SpotlightTour<'a> {
    step: &'a mut Option<usize>,
    steps: Vec<TourStep>,
    within: Option<Rect>,
    padding: f32,
}

/// Fill the area between `outer` and a rounded-rect hole with `color` (one seamless mesh).
fn dim_with_hole(painter: &egui::Painter, outer: Rect, hole: Rect, radius: f32, color: Color32) {
    let hole = hole.intersect(outer);
    let r = radius.min(hole.width() / 2.0).min(hole.height() / 2.0).max(0.0);
    let mut mesh = Mesh::default();
    let mut quad = |a: Pos2, b: Pos2| {
        if b.x <= a.x || b.y <= a.y {
            return;
        }
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(a, color);
        mesh.colored_vertex(pos2(b.x, a.y), color);
        mesh.colored_vertex(b, color);
        mesh.colored_vertex(pos2(a.x, b.y), color);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    };
    quad(outer.min, pos2(outer.max.x, hole.min.y));
    quad(pos2(outer.min.x, hole.max.y), outer.max);
    quad(pos2(outer.min.x, hole.min.y), pos2(hole.min.x, hole.max.y));
    quad(pos2(hole.max.x, hole.min.y), pos2(outer.max.x, hole.max.y));
    // rounded corners of the hole
    let corners = [
        (hole.left_top(), pos2(hole.min.x + r, hole.min.y + r), std::f32::consts::PI),
        (hole.right_top(), pos2(hole.max.x - r, hole.min.y + r), 1.5 * std::f32::consts::PI),
        (hole.right_bottom(), pos2(hole.max.x - r, hole.max.y - r), 0.0),
        (hole.left_bottom(), pos2(hole.min.x + r, hole.max.y - r), 0.5 * std::f32::consts::PI),
    ];
    for (corner, center, a0) in corners {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(corner, color);
        let seg = 10;
        for k in 0..=seg {
            let a = a0 + std::f32::consts::FRAC_PI_2 * k as f32 / seg as f32;
            mesh.colored_vertex(center + vec2(a.cos(), a.sin()) * r, color);
        }
        for k in 0..seg as u32 {
            mesh.add_triangle(base, base + 1 + k, base + 2 + k);
        }
    }
    painter.add(Shape::mesh(mesh));
}

impl<'a> SpotlightTour<'a> {
    /// A tour; `step` is the current index (`None` = not running).
    pub fn new(step: &'a mut Option<usize>, steps: Vec<TourStep>) -> Self {
        Self { step, steps, within: None, padding: 6.0 }
    }
    /// Dim only `rect` instead of the whole window.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Space between the target and the cut-out edge (default 6).
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Show the tour (call every frame). Returns true on the frame the tour finishes or is skipped.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = Theme::of(ui);
        let ctx = ui.ctx().clone();
        let id = ui.scope_id().with("glowkit_tour");
        let n = self.steps.len();
        if let Some(s) = *self.step {
            if s >= n {
                *self.step = None;
            }
        }
        let open = self.step.is_some();
        let p = kit::presence(ui, id, open, 10.0);
        if p < 0.01 && !open {
            return false;
        }
        let shown: usize = glowkit_core::state(ui, id.with("shown"), || 0);
        let cur = self.step.unwrap_or(shown).min(n.saturating_sub(1));
        glowkit_core::set_state(ui, id.with("shown"), cur);
        if n == 0 {
            return false;
        }
        let step = &self.steps[cur];
        let target = step.target.expand(self.padding);
        let sp = |k: &str, v: f32| glowkit_core::anim::spring(ui, id.with(k), v, 210.0, 24.0);
        let hole = Rect::from_min_max(
            pos2(sp("x0", target.min.x), sp("y0", target.min.y)),
            pos2(sp("x1", target.max.x), sp("y1", target.max.y)),
        );
        let area = kit::screen(&ctx, self.within);
        let clip = if self.within.is_some() { area.intersect(ui.clip_rect()) } else { area };
        let mut finished = false;
        let mut next = *self.step;
        if open && ctx.input(|i| i.key_pressed(Key::Escape)) {
            next = None;
            finished = true;
        }
        if open && ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            next = if cur + 1 < n { Some(cur + 1) } else { None };
            finished = next.is_none();
        }
        if open && cur > 0 && ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            next = Some(cur - 1);
        }

        let lid = id.with("layer");
        let time = glowkit_core::time(ui);
        // dim layer
        kit::float_layer(&ctx, lid, Order::Foreground, area, clip, 1.0, area.center(), p, |ui| {
            ui.interact(area, lid.with("block"), Sense::click());
            let dim = with_alpha(Color32::BLACK, if t.dark { 0.62 } else { 0.45 });
            // paint the outer region generously (the mesh covers up to the rect edges)
            let big = area;
            dim_with_hole(ui.painter(), big, hole, 10.0, dim);
            let pulse = 0.5 + 0.5 * (time * 2.4).sin();
            let ring = with_alpha(t.accent, 0.55 + 0.35 * pulse);
            ui.painter().rect_stroke(hole, 10.0, Stroke::new(1.5, ring), egui::StrokeKind::Outside);
            ui.painter().rect_stroke(
                hole.expand(3.0 + 3.0 * pulse),
                13.0,
                Stroke::new(1.0, with_alpha(t.accent, 0.25 * (1.0 - pulse))),
                egui::StrokeKind::Outside,
            );
        });
        ctx.move_to_top(egui::LayerId::new(Order::Foreground, lid));
        glowkit_core::animate(ui);

        // step card
        let cw = 300.0;
        let tg = kit::wrap(ui, &step.title, 15.5, t.foreground, cw - 64.0);
        let bg = kit::wrap(ui, &step.body, 13.0, t.muted_foreground, cw - 36.0);
        let ch = 18.0 + tg.size().y + 6.0 + bg.size().y + 18.0 + 32.0 + 16.0;
        let (rect, used) = place(hole, vec2(cw, ch), step.placement, 14.0, area);
        let rx = glowkit_core::anim::spring(ui, id.with("cx"), rect.min.x, 210.0, 24.0);
        let ry = glowkit_core::anim::spring(ui, id.with("cy"), rect.min.y, 210.0, 24.0);
        let rect = Rect::from_min_size(pos2(rx, ry), rect.size());
        let cid = id.with("card");
        let res = kit::float_layer(
            &ctx,
            cid,
            Order::Foreground,
            rect.expand(12.0),
            clip,
            0.96 + 0.04 * p,
            rect.center() - used.dir() * rect.size() * 0.5,
            p,
            |ui| {
                ui.interact(rect, cid.with("block"), Sense::click());
                kit::panel(ui.painter(), rect, &t, t.radius);
                let x = rect.min.x + 18.0;
                let mut y = rect.min.y + 18.0;
                ui.painter().galley(pos2(x, y), tg.clone(), t.foreground);
                let closer = Rect::from_min_size(pos2(rect.max.x - 38.0, rect.min.y + 12.0), vec2(24.0, 24.0));
                let close = kit::icon_button(ui, closer, cid.with("x"), LineIcon::X, t.muted_foreground).clicked();
                y += tg.size().y + 6.0;
                ui.painter().galley(pos2(x, y), bg.clone(), t.muted_foreground);
                // dots
                let by = rect.max.y - 16.0;
                for i in 0..n {
                    let active = i == cur;
                    let w = if active { 16.0 } else { 6.0 };
                    let dx: f32 = (0..i).map(|j| if j == cur { 16.0 } else { 6.0 } + 5.0).sum();
                    let r = Rect::from_min_size(pos2(x + dx, by - 16.0 - 3.0), vec2(w, 6.0));
                    ui.painter().rect_filled(
                        r,
                        3.0,
                        if active { t.accent } else { with_alpha(t.muted_foreground, 0.35) },
                    );
                }
                let mut nx = None;
                let mut fin = false;
                let last = cur + 1 == n;
                let next_label = if last { "Finish" } else { "Next" };
                let w = kit::button_width(ui, next_label, 13.5);
                let nr = Rect::from_min_max(pos2(rect.max.x - 18.0 - w, by - 32.0), pos2(rect.max.x - 18.0, by));
                if kit::button_at(ui, nr, cid.with("next"), next_label, Btn::Primary).clicked() {
                    if last {
                        fin = true;
                    } else {
                        nx = Some(cur + 1);
                    }
                }
                if cur > 0 {
                    let bw = kit::button_width(ui, "Back", 13.5);
                    let br = Rect::from_min_max(pos2(nr.min.x - 8.0 - bw, by - 32.0), pos2(nr.min.x - 8.0, by));
                    if kit::button_at(ui, br, cid.with("back"), "Back", Btn::Ghost).clicked() {
                        nx = Some(cur - 1);
                    }
                }
                (close, nx, fin)
            },
        );
        ctx.move_to_top(egui::LayerId::new(Order::Foreground, cid));
        let (close, nx, fin) = res.inner;
        if close || fin {
            next = None;
            finished = true;
        } else if let Some(s) = nx {
            next = Some(s);
        }
        if open {
            *self.step = next;
        }
        finished
    }
}
