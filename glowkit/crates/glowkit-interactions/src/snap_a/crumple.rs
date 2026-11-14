use super::common::{
    cached_ink, contact_shadow, idle_time, lerp_c, photo, sstep, wall_dt, Cam, Crumple, Ink, Sheet, Soup, PAPER,
    PAPER_BACK,
};
use super::posters::{poster_bg, poster_ink, POSTERS};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, paint, Theme};

/// What is printed on one sheet of a [`CrumpleReveal`].
#[derive(Clone, Debug)]
pub enum CrumpleItem {
    /// A typed letter on headed paper with a signature and a red "received" stamp.
    Letter {
        /// Letterhead.
        heading: String,
        /// Body paragraphs.
        body: Vec<String>,
        /// Signature (painted in blue ink).
        signature: String,
    },
    /// A photo print with a white border and a handwritten caption.
    Photo {
        /// Which painted scene (0 sunset sea, 1 mountain lake, 2 meadow, 3 night city).
        scene: usize,
        /// Caption under the picture.
        caption: String,
    },
    /// A short handwritten note.
    Note {
        /// Lines of the note.
        lines: Vec<String>,
        /// Signature.
        signature: String,
    },
}

impl CrumpleItem {
    /// A typed letter.
    pub fn letter(heading: impl Into<String>, body: &[&str], signature: impl Into<String>) -> Self {
        Self::Letter {
            heading: heading.into(),
            body: body.iter().map(|s| s.to_string()).collect(),
            signature: signature.into(),
        }
    }
    /// A photo print of painted scene `scene`.
    pub fn photo(scene: usize, caption: impl Into<String>) -> Self {
        Self::Photo { scene, caption: caption.into() }
    }
    /// A handwritten note.
    pub fn note(lines: &[&str], signature: impl Into<String>) -> Self {
        Self::Note { lines: lines.iter().map(|s| s.to_string()).collect(), signature: signature.into() }
    }

    fn defaults() -> Vec<Self> {
        vec![
            Self::note(
                &[
                    "Dear Rowan,",
                    "",
                    "I kept your map folded in my",
                    "coat all winter, and every",
                    "street on it is still there.",
                    "",
                    "The bakery on the corner",
                    "finally reopened. I bought",
                    "two of everything for you.",
                ],
                "Mira",
            ),
            Self::photo(0, "harbour, first light"),
            Self::letter(
                "ATELIER NORD",
                &[
                    "Dear Mr. Lind,",
                    "",
                    "Thank you for your letter and",
                    "the sketches. We have read",
                    "them twice and would love to",
                    "see the full series in person.",
                    "",
                    "Warmly,",
                ],
                "Ingrid Vale",
            ),
        ]
    }

    fn front(&self, u: f32, v: f32) -> Color32 {
        match self {
            CrumpleItem::Photo { scene, .. } => {
                let (bx, bt, bb) = (0.07, 0.06, 0.24);
                if u > bx && u < 1.0 - bx && v > bt && v < 1.0 - bb {
                    photo(*scene, (u - bx) / (1.0 - 2.0 * bx), (v - bt) / (1.0 - bt - bb))
                } else {
                    PAPER
                }
            }
            CrumpleItem::Note { .. } => lerp_c(hex("#F3EBD7"), hex("#EADFC4"), v),
            CrumpleItem::Letter { .. } => PAPER,
        }
    }

    fn print(&self, ink: &mut Ink, size: Vec2) {
        let s = size.x / 170.0;
        let typed = hex("#3A3A40");
        let blue = hex("#2B3A8F");
        match self {
            CrumpleItem::Letter { heading, body, signature } => {
                ink.text(pos2(size.x * 0.5, 14.0 * s), Align2::CENTER_TOP, heading, FontId::monospace(10.0 * s), hex("#22222A"));
                ink.text(pos2(size.x * 0.5, 27.0 * s), Align2::CENTER_TOP, "Harbour Road 12", FontId::proportional(5.5 * s), typed);
                ink.rect(Rect::from_min_size(pos2(16.0 * s, 36.0 * s), vec2(size.x - 32.0 * s, 0.6 * s)), 0.0, with_alpha(typed, 0.4));
                ink.text(pos2(size.x - 16.0 * s, 42.0 * s), Align2::RIGHT_TOP, "14 November", FontId::monospace(5.0 * s), typed);
                let mut y = 54.0 * s;
                for line in body {
                    ink.text(pos2(16.0 * s, y), Align2::LEFT_TOP, line, FontId::monospace(5.4 * s), typed);
                    y += 8.0 * s;
                }
                ink.text(pos2(16.0 * s, y + 4.0 * s), Align2::LEFT_TOP, signature, FontId::proportional(12.0 * s), blue);
                // stamp
                let st = Rect::from_min_size(pos2(size.x - 66.0 * s, size.y - 40.0 * s), vec2(50.0 * s, 17.0 * s));
                let red = hex("#C8343C");
                ink.shapes.push(egui::Shape::rect_stroke(st, 2.0 * s, egui::Stroke::new(1.3 * s, with_alpha(red, 0.8)), egui::StrokeKind::Inside));
                ink.text(st.center(), Align2::CENTER_CENTER, "RECEIVED", FontId::monospace(6.4 * s), with_alpha(red, 0.85));
            }
            CrumpleItem::Photo { caption, .. } => {
                ink.text(pos2(size.x * 0.12, size.y * 0.84), Align2::LEFT_TOP, caption, FontId::proportional(10.0 * s), hex("#2E3550"));
            }
            CrumpleItem::Note { lines, signature } => {
                ink.text(pos2(size.x - 14.0 * s, 12.0 * s), Align2::RIGHT_TOP, "3 March", FontId::proportional(6.0 * s), blue);
                let mut y = 24.0 * s;
                for line in lines {
                    ink.text(pos2(14.0 * s, y), Align2::LEFT_TOP, line, FontId::proportional(8.0 * s), blue);
                    y += 11.0 * s;
                }
                ink.text(pos2(size.x - 18.0 * s, y + 8.0 * s), Align2::RIGHT_TOP, "with love,", FontId::proportional(6.5 * s), blue);
                ink.text(pos2(size.x - 18.0 * s, y + 17.0 * s), Align2::RIGHT_TOP, signature, FontId::proportional(13.0 * s), blue);
            }
        }
    }
}

#[derive(Clone)]
struct RevealState {
    c: Vec<Spring>,
    lift: Vec<f32>,
    open: Option<usize>,
    auto_t: f32,
    auto_step: usize,
}

/// Balls of screwed-up paper on a desk. Click one and it smooths out into the letter, note or
/// photograph it was (still faintly creased), while the one that was open crumples back into a
/// ball. Every sheet is a real facet-shaded paper mesh: the ink bends and hides with the folds.
///
/// Drive it yourself with [`Self::open`], or leave the built-in autoplay on and it takes turns
/// opening each ball while nobody is touching it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{CrumpleItem, CrumpleReveal};
/// ui.add(CrumpleReveal::new(vec![
///     CrumpleItem::photo(1, "lake, june"),
///     CrumpleItem::note(&["Back at six.", "Feed the cat!"], "J."),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CrumpleReveal {
    items: Vec<CrumpleItem>,
    size: Option<Vec2>,
    sheet: Vec2,
    desk: Option<Color32>,
    residual: f32,
    speed: f32,
    autoplay: bool,
    open: Option<Option<usize>>,
}

impl Default for CrumpleReveal {
    fn default() -> Self {
        Self::new(CrumpleItem::defaults())
    }
}

impl CrumpleReveal {
    /// A desk of crumpled `items` (an empty list uses three built-in sheets).
    pub fn new(items: Vec<CrumpleItem>) -> Self {
        let items = if items.is_empty() { CrumpleItem::defaults() } else { items };
        Self {
            items,
            size: None,
            sheet: vec2(172.0, 222.0),
            desk: None,
            residual: 1.0,
            speed: 1.0,
            autoplay: true,
            open: None,
        }
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Size of one flattened sheet in points (default 172 x 222).
    pub fn sheet_size(mut self, s: Vec2) -> Self {
        self.sheet = s.max(vec2(60.0, 60.0));
        self
    }
    /// Desk color (default: a warm paper-grey for light themes, a dark felt for dark ones).
    pub fn desk(mut self, c: Color32) -> Self {
        self.desk = Some(c);
        self
    }
    /// How visible the creases stay once a sheet is smoothed out (0 pristine .. 2 well worn).
    pub fn creases(mut self, k: f32) -> Self {
        self.residual = k.clamp(0.0, 2.0);
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, k: f32) -> Self {
        self.speed = k.clamp(0.2, 3.0);
        self
    }
    /// Take turns opening the sheets while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Force which sheet is open (`None` = all crumpled). Overrides clicks and autoplay.
    pub fn open(mut self, index: Option<usize>) -> Self {
        self.open = Some(index);
        self
    }
}

pub(crate) fn desk_color(t: &Theme) -> Color32 {
    if t.dark {
        hex("#1E1D1B")
    } else {
        hex("#E8E3D9")
    }
}

pub(crate) fn paint_desk(painter: &egui::Painter, rect: Rect, base: Color32, radius: f32) {
    let light = glowkit_core::color::shade(base, 0.04);
    let dark = glowkit_core::color::shade(base, -0.05);
    let pts = paint::rounded_rect_points(rect, radius, 8);
    let c = rect.center() - vec2(0.0, rect.height() * 0.1);
    paint::fill_convex_fn(painter, &pts, 8, paint::radial_fn(c, rect.width().max(rect.height()) * 0.62, &[light, base, dark]));
}

impl Widget for CrumpleReveal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let id = response.id;
        let n = self.items.len();
        let dt = glowkit_core::dt(ui) * self.speed;
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let spacing = (rect.width() / n as f32).min(self.sheet.x * 1.35);
        let slot = |i: usize| pos2(rect.center().x + (i as f32 - (n as f32 - 1.0) / 2.0) * spacing, rect.center().y);
        let ball_r = (self.sheet.x + self.sheet.y) * 0.5 * 0.24;
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let clicked = response.clicked();
        let st0: RevealState = glowkit_core::state(ui, id, || RevealState {
            c: vec![Spring::new(1.0).with(70.0, 12.0); n],
            lift: vec![0.0; n],
            open: None,
            auto_t: 1.2,
            auto_step: 0,
        });
        // which sheet is under the pointer (nearest, within its current footprint)
        let hit = pointer.and_then(|p| {
            (0..n)
                .filter_map(|i| {
                    let c = st0.c.get(i).map_or(1.0, |s| s.value).clamp(0.0, 1.0);
                    let r = ball_r * 1.1 + (self.sheet.y * 0.5 - ball_r) * (1.0 - c);
                    let d = p.distance(slot(i));
                    (d < r).then_some((i, d))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|x| x.0)
        });
        if hit.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let (forced, autoplay) = (self.open, self.autoplay);
        let st = glowkit_core::with_state(ui, id, || st0.clone(), |s| {
            s.c.resize(n, Spring::new(1.0).with(70.0, 12.0));
            s.lift.resize(n, 0.0);
            if clicked {
                s.open = match hit {
                    Some(i) if s.open != Some(i) => Some(i),
                    _ => None,
                };
                s.auto_t = 0.0;
            }
            if let Some(f) = forced {
                s.open = f.filter(|i| *i < n);
            } else if autoplay && idle > 2.5 {
                s.auto_t -= wdt;
                if s.auto_t <= 0.0 {
                    s.auto_step = (s.auto_step + 1) % (n + 1);
                    s.open = if s.auto_step == n { None } else { Some(s.auto_step) };
                    s.auto_t = if s.open.is_some() { 2.6 } else { 1.2 };
                }
            }
            for i in 0..n {
                s.c[i].target = if s.open == Some(i) { 0.0 } else { 1.0 };
                s.c[i].step(dt);
                let target = if hit == Some(i) { 1.0 } else { 0.0 };
                s.lift[i] = glowkit_core::anim::approach(s.lift[i], target, 10.0, dt);
            }
            s.clone()
        });
        if st.c.iter().any(|s| !s.is_settled()) || st.lift.iter().any(|l| *l > 0.001 && *l < 0.999) {
            glowkit_core::animate(ui);
        }
        if autoplay {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let desk = self.desk.unwrap_or_else(|| desk_color(&t));
        paint_desk(&painter, rect, desk, t.radius * 1.4);
        let mut order: Vec<usize> = (0..n).collect();
        // the sheet being opened / open goes on top
        order.sort_by(|a, b| st.c[*b].value.total_cmp(&st.c[*a].value));
        for &i in &order {
            let c = st.c[i].value.clamp(-0.05, 1.08);
            let cc = c.clamp(0.0, 1.0);
            let item = &self.items[i];
            let center = slot(i);
            let fly = (std::f32::consts::PI * cc).sin();
            let lift = st.lift[i] * cc;
            let z = fly * 60.0 + lift * 18.0;
            // shadows: a soft sheet shadow fades into a round ball shadow
            let sh_off = vec2(6.0, 10.0) + vec2(1.0, 1.4) * z * 0.25;
            let flat_sh = (1.0 - cc * 1.6).clamp(0.0, 1.0);
            if flat_sh > 0.0 {
                let r = Rect::from_center_size(center + sh_off * 0.6, self.sheet * 0.98);
                paint::soft_shadow(&painter, r, 4.0, 18.0, Vec2::ZERO, Color32::from_black_alpha((60.0 * flat_sh) as u8));
            }
            let ball_sh = sstep(0.3, 0.9, cc);
            if ball_sh > 0.0 {
                let rad = vec2(ball_r * 1.2, ball_r * 0.7) * (1.0 + z * 0.004);
                contact_shadow(&painter, center + vec2(10.0, ball_r * 0.78) + sh_off * 0.3, rad, 0.35 * ball_sh);
            }
            let crumple = Crumple::new(i as u64 * 977 + 13);
            let sheet_size = self.sheet;
            let residual = self.residual;
            let deform = |u: f32, v: f32| crumple.pos(u, v, sheet_size, cc, residual);
            let front = |u: f32, v: f32| item.front(u, v);
            let ink = cached_ink(ui, id.with(("crumple_item", i)), 7.0, |ink| item.print(ink, sheet_size));
            let mut cam = Cam::new(center);
            let tilt = (crate::snap_a::common::h01(i as u64 + 4) - 0.5) * 0.12;
            cam.roll = tilt * (1.0 - cc) + cc * 0.4 * (i as f32 - 1.0);
            cam.pitch = -0.1 * cc;
            cam.offset.z = z;
            cam.focal = 900.0;
            let mut soup = Soup::new();
            let fine = matches!(item, CrumpleItem::Photo { .. });
            let sheet = Sheet {
                size: sheet_size,
                cols: if fine { 34 } else { 22 },
                rows: if fine { 42 } else { 28 },
                deform: &deform,
                front: &front,
                back: PAPER_BACK,
                ambient: 0.5,
                gloss: 0.1,
                bias: 0.0,
                alpha: 1.0,
                ink_bias: 300.0 * (1.0 - sstep(0.25, 0.6, cc)) + 0.8,
            };
            sheet.emit(&mut soup, &cam, Some(&ink));
            soup.paint(&painter);
            if cc < 0.12 {
                let edge = with_alpha(lerp_c(PAPER, desk, 0.25), 1.0 - cc / 0.12);
                painter.add(egui::Shape::closed_line(sheet.outline(&cam, 12), egui::Stroke::new(1.0, edge)));
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct CarouselState {
    pos: f32,
    vel: f32,
    target: Option<f32>,
    auto_t: f32,
}

/// A row of posters where only the one in focus lies flat: its neighbours crinkle and the outer
/// ones are screwed up into balls of paper, smoothing out as they travel to the middle.
///
/// Drag or scroll the row, click a poster to bring it to the middle, or link it to a page
/// scroll with [`Self::progress`] (`0..=1` across the whole row). Idle, it drifts poster to
/// poster on its own.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::CrumpleCarousel::new().count(8));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CrumpleCarousel {
    count: usize,
    size: Option<Vec2>,
    poster: Vec2,
    spread: f32,
    falloff: f32,
    progress: Option<f32>,
    autoplay: bool,
    background: Option<Color32>,
}

impl Default for CrumpleCarousel {
    fn default() -> Self {
        Self::new()
    }
}

impl CrumpleCarousel {
    /// A carousel of the built-in generated posters.
    pub fn new() -> Self {
        Self {
            count: POSTERS,
            size: None,
            poster: vec2(190.0, 250.0),
            spread: 1.0,
            falloff: 1.0,
            progress: None,
            autoplay: true,
            background: None,
        }
    }
    /// Number of posters (the designs repeat; default 8).
    pub fn count(mut self, n: usize) -> Self {
        self.count = n.max(1);
        self
    }
    /// Widget size (default: available width x 380).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Size of a flat poster in points (default 190 x 250).
    pub fn poster_size(mut self, s: Vec2) -> Self {
        self.poster = s.max(vec2(60.0, 80.0));
        self
    }
    /// Horizontal spacing multiplier (default 1).
    pub fn spread(mut self, k: f32) -> Self {
        self.spread = k.clamp(0.5, 2.0);
        self
    }
    /// How quickly posters crumple away from the middle (default 1; higher = tighter).
    pub fn falloff(mut self, k: f32) -> Self {
        self.falloff = k.clamp(0.3, 3.0);
        self
    }
    /// Link to an outside scroll position: `0` shows the first poster, `1` the last.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Drift from poster to poster while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Background color (default: from the theme).
    pub fn background(mut self, c: Color32) -> Self {
        self.background = Some(c);
        self
    }
}

impl Widget for CrumpleCarousel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let n = self.count;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let step = self.poster.x * 0.62 * self.spread;
        let drag = if response.dragged() { response.drag_delta().x } else { 0.0 };
        let released = response.drag_stopped();
        let wheel = if response.hovered() { ui.input(|i| i.smooth_scroll_delta.x + i.smooth_scroll_delta.y) } else { 0.0 };
        let clicked = response.clicked().then(|| response.interact_pointer_pos()).flatten();
        let (progress, autoplay) = (self.progress, self.autoplay);
        let max = (n - 1) as f32;
        let nf = n as f32;
        let st = glowkit_core::with_state(ui, id, CarouselState::default, |s| {
            if let Some(p) = progress {
                s.pos = p * max;
                s.vel = 0.0;
                return *s;
            }
            if drag != 0.0 {
                s.pos -= drag / step;
                s.vel = -drag / step / dt.max(1e-3);
                s.target = None;
            } else if wheel != 0.0 {
                s.pos -= wheel / step * 0.6;
                s.target = Some((s.pos - wheel.signum() * 0.3).round());
            } else {
                if released {
                    s.target = Some((s.pos + s.vel * 0.25).round());
                }
                if let Some(p) = clicked {
                    let d = ((p.x - rect.center().x) / step).round();
                    s.target = Some(s.pos.round() + d);
                    s.auto_t = 3.0;
                }
                if autoplay && idle > 2.0 {
                    s.auto_t -= wdt;
                    if s.auto_t <= 0.0 {
                        s.target = Some(s.pos.round() + 1.0);
                        s.auto_t = 2.4;
                    }
                }
                let tgt = s.target.unwrap_or_else(|| s.pos.round());
                // critically damped glide to the snapped poster
                let k = 60.0;
                let acc = -k * (s.pos - tgt) - 2.0 * k.sqrt() * s.vel;
                s.vel += acc * dt;
                s.pos += s.vel * dt;
            }
            // keep the endless row's position bounded
            if s.pos > nf * 4.0 || s.pos < -nf * 4.0 {
                let shift = (s.pos / nf).floor() * nf;
                s.pos -= shift;
                s.target = s.target.map(|t| t - shift);
            }
            *s
        });
        if autoplay || st.vel.abs() > 1e-3 || response.dragged() {
            glowkit_core::animate(ui);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let bg = self.background.unwrap_or(if t.dark { hex("#141416") } else { hex("#E6E6E8") });
        paint_desk(&painter, rect, bg, t.radius * 1.4);
        // endless: every poster sits at its nearest wrapped distance from the focus
        let mut items: Vec<(usize, f32)> = (0..n)
            .map(|i| (i, (i as f32 - st.pos + nf * 0.5).rem_euclid(nf) - nf * 0.5))
            .filter(|(_, d)| d.abs() < 6.0)
            .collect();
        // far ones first
        items.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
        let poster = self.poster;
        let ball = (poster.x + poster.y) * 0.5 * 0.2;
        let mut soup = Soup::new();
        let falloff = self.falloff;
        let crumple_at = |ad: f32| sstep(0.18, 1.9 / falloff, ad);
        // each item takes up its own width: flat posters wide, balls narrow
        let half_w = |ad: f32| {
            let c = crumple_at(ad);
            poster.x * 0.5 * (1.0 - c) + ball * 1.08 * c
        };
        let x_of = |d: f32| {
            let ad = d.abs();
            let steps = 14;
            let dx = ad / steps as f32;
            let x: f32 = (0..steps).map(|k| 2.0 * half_w((k as f32 + 0.5) * dx) * dx).sum();
            x * d.signum() * 0.94
        };
        let mut flat_outline: Vec<(Vec<egui::Pos2>, Color32, f32)> = Vec::new();
        for (i, d) in items {
            let ad = d.abs();
            let c = crumple_at(ad);
            let x = x_of(d) * self.spread;
            let center = pos2(rect.center().x + x, rect.center().y - 6.0);
            if (center.x - rect.center().x).abs() > rect.width() * 0.5 + poster.x {
                continue;
            }
            contact_shadow(
                &painter,
                pos2(center.x + 6.0, rect.center().y + poster.y * 0.5 * (1.0 - c) + ball * c + 16.0),
                vec2(poster.x * 0.45 * (1.0 - c) + ball * c, 9.0),
                0.22,
            );
            let k = i % POSTERS;
            let crumple = Crumple::new(i as u64 * 31 + 7);
            let deform = |u: f32, v: f32| crumple.pos(u, v, poster, c, 0.35);
            let front = |u: f32, v: f32| poster_bg(k, u, v);
            let ink = cached_ink(ui, id.with(("poster", k)), 8.0, |ink| poster_ink(ink, k, poster));
            let mut cam = Cam::new(center);
            cam.yaw = -d.clamp(-1.0, 1.0) * 0.32 * (1.0 - c * 0.6);
            cam.roll = c * 0.6 * d.signum();
            cam.focal = 1100.0;
            cam.offset.z = -ad * 30.0;
            let back = lerp_c(PAPER_BACK, poster_bg(k, 0.5, 0.5), 0.15);
            let sheet = Sheet {
                size: poster,
                cols: 18,
                rows: 24,
                deform: &deform,
                front: &front,
                back,
                ambient: 0.48,
                gloss: 0.16,
                bias: -ad * 1000.0,
                alpha: 1.0,
                ink_bias: 300.0 * (1.0 - sstep(0.25, 0.6, c)) + 0.8,
            };
            sheet.emit(&mut soup, &cam, Some(&ink));
            if c < 0.1 {
                flat_outline.push((sheet.outline(&cam, 10), poster_bg(k, 0.5, 0.5), 1.0 - c / 0.1));
            }
        }
        soup.paint(&painter);
        for (pts, col, a) in flat_outline {
            painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.0, with_alpha(col, a))));
        }
        // index
        let idx = (st.pos.round().rem_euclid(nf)) as usize % n;
        painter.text(
            pos2(rect.right() - 18.0, rect.bottom() - 16.0),
            Align2::RIGHT_BOTTOM,
            format!("{:02} / {:02}", idx + 1, n),
            FontId::monospace(11.0),
            t.muted_foreground,
        );
        painter.text(
            pos2(rect.left() + 18.0, rect.bottom() - 16.0),
            Align2::LEFT_BOTTOM,
            super::posters::poster_title(idx),
            FontId::proportional(13.0),
            t.foreground,
        );
        response
    }
}
