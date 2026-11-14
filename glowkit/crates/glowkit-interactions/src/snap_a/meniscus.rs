use super::common::{ghost_cursor, idle_time, iso_fill, lerp_c, sd_round_rect, smin, sstep, wall_dt};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::with_alpha;
use glowkit_core::hex;
use glowkit_core::noise::noise3;

/// What lies under the [`Meniscus`] glass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeniscusBackdrop {
    /// A tiled pool floor with moving caustics.
    #[default]
    Pool,
    /// Warm diagonal stripes, like a printed poster.
    Poster,
}

#[derive(Clone, Copy, Debug)]
struct DropState {
    x: Spring,
    y: Spring,
    grab: Option<Vec2>,
    frost: f32,
    frosted: bool,
    ghost: f32,
    init: bool,
}

/// A drop of liquid glass that pulls free of its dock and merges with whatever it touches. The
/// glass is an implicit surface (two rounded slabs melted together with a smooth union), so it
/// necks and snaps apart like a meniscus; everything behind it is refracted through the curved
/// rim with a little colour fringing. Tap the drop to frost the glass.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::Meniscus;
/// ui.add(Meniscus::new("Lido, four o'clock", "Pool floor, lane four, clear glass"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Meniscus {
    title: String,
    subtitle: String,
    backdrop: MeniscusBackdrop,
    size: Option<Vec2>,
    melt: f32,
    refraction: f32,
    autoplay: bool,
}

impl Meniscus {
    /// A glass card with `title` / `subtitle` and a draggable glass dock.
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
            backdrop: MeniscusBackdrop::Pool,
            size: None,
            melt: 1.0,
            refraction: 1.0,
            autoplay: true,
        }
    }
    /// What is behind the glass (default the pool).
    pub fn backdrop(mut self, b: MeniscusBackdrop) -> Self {
        self.backdrop = b;
        self
    }
    /// Widget size (default: available width x 400).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// How eagerly the two pieces of glass melt together (default 1).
    pub fn melt(mut self, k: f32) -> Self {
        self.melt = k.clamp(0.2, 3.0);
        self
    }
    /// Strength of the refraction at the rim (default 1).
    pub fn refraction(mut self, k: f32) -> Self {
        self.refraction = k.clamp(0.0, 3.0);
        self
    }
    /// Drag the drop around with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn pool(p: Pos2, time: f32) -> [f32; 3] {
    let tile = 34.0;
    let gx = (p.x / tile).fract().abs();
    let gy = (p.y / tile).fract().abs();
    let grout = (1.0 - (gx.min(1.0 - gx).min(gy.min(1.0 - gy)) / 0.05).clamp(0.0, 1.0)) * 0.5;
    // caustics: two layers of domain-warped sine waves, sharpened into a bright web
    let web = |s: f32, t: f32, o: f32| {
        let (x, y) = (p.x * s + o, p.y * s - o);
        let a = (x * 1.3 + (y * 1.7 + t * 0.6).sin() * 1.6 + t * 0.4).sin();
        let b = (y * 1.1 + (x * 1.9 - t * 0.5).sin() * 1.6 - t * 0.3).sin();
        (1.0 - (a + b).abs() * 0.5).powi(7)
    };
    let n = noise3(p.x * 0.004, p.y * 0.004, time * 0.1) * 0.5;
    let caustic = web(0.03, time, 0.0) + 0.55 * web(0.047, time * 1.2, 3.0) + n * 0.05;
    let base = [0.52, 0.79, 0.81];
    let mut c = [
        base[0] + caustic * 0.42 + grout * 0.22,
        base[1] + caustic * 0.22 + grout * 0.12,
        base[2] + caustic * 0.18 + grout * 0.1,
    ];
    c.iter_mut().for_each(|x| *x = x.clamp(0.0, 1.0));
    c
}

fn poster(p: Pos2, _time: f32) -> [f32; 3] {
    let s = ((p.x + p.y) * 0.035).sin() * 0.5 + 0.5;
    let k = ((p.x - p.y * 0.4) * 0.011).sin() * 0.5 + 0.5;
    [0.95 - 0.1 * k, 0.55 + 0.3 * s, 0.35 + 0.25 * s * k]
}

fn to_c(c: [f32; 3]) -> Color32 {
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

impl Widget for Meniscus {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = glowkit_core::desired_size(ui, self.size, 400.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, rect);
        let card = Rect::from_center_size(rect.center() + vec2(-60.0, -70.0), vec2(270.0, 66.0));
        let dock_size = vec2(196.0, 46.0);
        let home = pos2(rect.center().x - 140.0, rect.max.y - 70.0);
        let st0: DropState = glowkit_core::state(ui, id, || DropState {
            x: Spring::new(home.x).with(140.0, 16.0),
            y: Spring::new(home.y).with(140.0, 16.0),
            grab: None,
            frost: 0.0,
            frosted: false,
            ghost: 0.0,
            init: true,
        });
        let dock_rect = Rect::from_center_size(pos2(st0.x.value, st0.y.value), dock_size);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let on_dock = pointer.is_some_and(|p| dock_rect.contains(p));
        if on_dock {
            ui.ctx().set_cursor_icon(if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let started = response.drag_started() && on_dock;
        let dragging = response.dragged();
        let stopped = response.drag_stopped();
        let tapped = response.clicked() && on_dock;
        let autoplay = self.autoplay && idle > 2.0;
        let mut ghost_draw: Option<(Pos2, bool)> = None;
        // autoplay path: lift off the dock, merge with the card, slide along it, pull free, return
        let path = |g: f32| -> Pos2 {
            let keys = [
                (0.0, home),
                (1.2, home),
                (2.4, pos2(card.min.x + 70.0, card.max.y + 26.0)),
                (3.6, pos2(card.max.x - 40.0, card.max.y + 22.0)),
                (4.6, pos2(card.max.x + 90.0, card.center().y + 70.0)),
                (5.8, home + vec2(240.0, 0.0)),
                (7.0, home),
            ];
            let g = g % 8.0;
            for w in keys.windows(2) {
                if g >= w[0].0 && g <= w[1].0 {
                    let k = sstep(w[0].0, w[1].0, g);
                    return w[0].1.lerp(w[1].1, k);
                }
            }
            home
        };
        let st = glowkit_core::with_state(ui, id, || st0, |s| {
            if s.init {
                s.init = false;
            }
            if started {
                if let Some(p) = pointer {
                    s.grab = Some(pos2(s.x.value, s.y.value) - p);
                }
            }
            if let (true, Some(g), Some(p)) = (dragging, s.grab, pointer) {
                s.x.target = p.x + g.x;
                s.y.target = p.y + g.y;
                s.ghost = 0.0;
            } else if autoplay {
                s.ghost += wdt;
                let p = path(s.ghost);
                s.x.target = p.x;
                s.y.target = p.y;
                ghost_draw = Some((pos2(s.x.value, s.y.value) + vec2(30.0, 4.0), (1.2..7.0).contains(&(s.ghost % 8.0))));
            }
            if stopped {
                s.grab = None;
            }
            if tapped {
                s.frosted = !s.frosted;
            }
            s.x.target = s.x.target.clamp(rect.min.x + dock_size.x * 0.5, rect.max.x - dock_size.x * 0.5);
            s.y.target = s.y.target.clamp(rect.min.y + dock_size.y * 0.5, rect.max.y - dock_size.y * 0.5);
            s.x.step(dt);
            s.y.step(dt);
            s.frost += ((if s.frosted { 1.0 } else { 0.0 }) - s.frost) * (1.0 - (-6.0 * dt).exp());
            *s
        });
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let bg_fn: fn(Pos2, f32) -> [f32; 3] = match self.backdrop {
            MeniscusBackdrop::Pool => pool,
            MeniscusBackdrop::Poster => poster,
        };
        // ---- backdrop
        let cell = 7.0;
        let (nx, ny) = ((rect.width() / cell).ceil() as usize, (rect.height() / cell).ceil() as usize);
        let mut m = Mesh::default();
        for j in 0..=ny {
            for i in 0..=nx {
                let p = pos2(rect.min.x + i as f32 * cell, rect.min.y + j as f32 * cell);
                m.colored_vertex(p, to_c(bg_fn(p, time)));
            }
        }
        for j in 0..ny as u32 {
            for i in 0..nx as u32 {
                let a = j * (nx as u32 + 1) + i;
                m.add_triangle(a, a + 1, a + nx as u32 + 2);
                m.add_triangle(a, a + nx as u32 + 2, a + nx as u32 + 1);
            }
        }
        painter.add(Shape::mesh(m));
        if self.backdrop == MeniscusBackdrop::Pool {
            // lane marking
            let lane = with_alpha(hex("#1D3A6E"), 0.82);
            painter.rect_filled(Rect::from_min_size(pos2(rect.center().x + 70.0, rect.min.y), vec2(22.0, rect.height())), 0.0, lane);
            painter.rect_filled(Rect::from_center_size(pos2(rect.center().x + 81.0, rect.min.y + 40.0), vec2(90.0, 20.0)), 0.0, lane);
        }
        // ---- glass field
        let dock = Rect::from_center_size(pos2(st.x.value, st.y.value), dock_size);
        let k = 26.0 * self.melt;
        let field = |p: Pos2| -> f32 {
            let a = sd_round_rect(p, card, 18.0);
            let b = sd_round_rect(p, dock, dock_size.y * 0.5);
            smin(a, b, k)
        };
        let area = card.union(dock).expand(k + 4.0).intersect(rect);
        // soft shadow under the glass
        iso_fill(&painter, area.translate(vec2(0.0, 10.0)), 6.0, |p| -(field(p - vec2(0.0, 10.0)) - 4.0), |_| Color32::from_black_alpha(14));
        iso_fill(&painter, area.translate(vec2(0.0, 6.0)), 6.0, |p| -field(p - vec2(0.0, 6.0)), |_| Color32::from_black_alpha(16));
        let refr = 16.0 * self.refraction;
        let frost = st.frost;
        let pond = self.backdrop == MeniscusBackdrop::Pool;
        let segs = iso_fill(&painter, area, 3.5, |p| -field(p), |p| {
            let d = -field(p);
            let e = 1.0 - (d / 16.0).clamp(0.0, 1.0);
            let h = 1.5;
            let g = vec2(field(p + vec2(h, 0.0)) - field(p - vec2(h, 0.0)), field(p + vec2(0.0, h)) - field(p - vec2(0.0, h)));
            let n = if g.length() > 1e-4 { g.normalized() } else { Vec2::ZERO };
            // the rim bends light inward; split channels for a slight fringe
            let off = n * refr * e * e;
            let mut c = if e > 0.05 {
                let r = bg_fn(p + off * 1.12, time)[0];
                let gch = bg_fn(p + off, time)[1];
                let b = bg_fn(p + off * 0.88, time)[2];
                [r, gch, b]
            } else {
                bg_fn(p, time)
            };
            if frost > 0.01 {
                let blur = [bg_fn(p + vec2(9.0, 4.0), time), bg_fn(p + vec2(-7.0, -6.0), time), bg_fn(p + vec2(3.0, 9.0), time)];
                for ch in 0..3 {
                    let avg = (blur[0][ch] + blur[1][ch] + blur[2][ch] + c[ch]) / 4.0;
                    c[ch] = c[ch] + (avg * 0.85 + 0.15 - c[ch]) * frost;
                }
            }
            // glass body: a touch brighter, cooler, with a lit upper rim
            let top = (-n.y).max(0.0) * e;
            let tint = if pond { [0.96, 1.0, 1.02] } else { [1.0, 1.0, 1.0] };
            let lift = 0.06 + 0.35 * top * e;
            to_c([
                (c[0] * tint[0] + lift).min(1.0),
                (c[1] * tint[1] + lift).min(1.0),
                (c[2] * tint[2] + lift).min(1.0),
            ])
        });
        for (a, b) in segs {
            painter.line_segment([a, b], Stroke::new(1.2, with_alpha(Color32::WHITE, 0.7)));
        }
        // ---- content on the glass
        let ink = hex("#0E2A33");
        painter.text(card.min + vec2(18.0, 14.0), Align2::LEFT_TOP, &self.title, FontId::proportional(17.0), ink);
        painter.text(card.min + vec2(18.0, 38.0), Align2::LEFT_TOP, &self.subtitle, FontId::proportional(11.0), with_alpha(ink, 0.65));
        painter.text(card.right_center() - vec2(18.0, 0.0), Align2::RIGHT_CENTER, "01", FontId::monospace(10.0), with_alpha(ink, 0.6));
        // dock icons
        let icons = 4;
        for i in 0..icons {
            let c = pos2(dock.min.x + 28.0 + i as f32 * (dock.width() - 56.0) / (icons - 1) as f32, dock.center().y);
            let s = Stroke::new(1.5, ink);
            match i {
                0 => {
                    for dy in [-4.0, 0.0, 4.0] {
                        painter.line_segment([c + vec2(-7.0, dy), c + vec2(7.0, dy)], s);
                    }
                }
                1 => {
                    painter.line_segment([c + vec2(-6.0, 6.0), c + vec2(5.0, -5.0)], s);
                    painter.line_segment([c + vec2(-6.0, 6.0), c + vec2(-6.5, 2.5)], s);
                }
                2 => {
                    painter.rect_stroke(Rect::from_center_size(c, vec2(11.0, 14.0)), 2, s, egui::StrokeKind::Middle);
                    painter.line_segment([c + vec2(-3.0, -1.0), c + vec2(3.0, -1.0)], s);
                    painter.line_segment([c + vec2(-3.0, 3.0), c + vec2(3.0, 3.0)], s);
                }
                _ => {
                    // a water drop: a round belly that narrows to a point
                    let mut pts: Vec<Pos2> = (0..=16)
                        .map(|k| {
                            let a = -0.35 + k as f32 / 16.0 * (std::f32::consts::PI + 0.7);
                            c + vec2(a.cos() * 5.0, 2.0 + a.sin() * 5.0)
                        })
                        .collect();
                    pts.push(c + vec2(0.0, -8.0));
                    painter.add(Shape::closed_line(pts, s));
                }
            }
        }
        // caption
        painter.text(
            pos2(rect.center().x, rect.max.y - 14.0),
            Align2::CENTER_BOTTOM,
            "Pull the drop out of the dock, or tap it to frost the glass",
            FontId::proportional(11.0),
            lerp_c(ink, Color32::WHITE, 0.25),
        );
        if let Some((p, pressed)) = ghost_draw {
            ghost_cursor(&painter, false, p, 0.9, pressed);
        }
        response
    }
}
