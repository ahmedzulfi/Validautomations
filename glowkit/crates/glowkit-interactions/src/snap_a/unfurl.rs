use super::common::{clay, contact_shadow, idle_time, lerp_c, sstep, wall_dt};
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::{ease, Easing};
use glowkit_core::color::with_alpha;
use glowkit_core::noise::fbm3;
use glowkit_core::{hex, Theme};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default)]
struct UnfurlState {
    t: f32,
    open: bool,
    auto_t: f32,
}

/// A parchment button that unrolls into a charter. Click the little rolled scroll: it
/// levitates, tips onto its edge, the bottom roller drops and unrolls the parchment (rollers
/// turning, paper shaded and stained), the text inks itself in line by line and a wax seal
/// stamps it shut. Click again and it rolls back up into a button.
///
/// The response reports `changed()` when it opens or closes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::UnfurlScroll;
/// ui.add(UnfurlScroll::new("Unfurl").title("The Charter of", "Small Delights"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct UnfurlScroll {
    label: String,
    title: (String, String),
    lines: Vec<String>,
    signature: String,
    height: f32,
    duration: f32,
    paper_width: f32,
    autoplay: bool,
}

impl UnfurlScroll {
    /// A rolled-up scroll labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            title: ("The Charter of".into(), "Small Delights".into()),
            lines: [
                "e it known to all who wander here,",
                "that a button is no mere trigger but",
                "a small machine, a folded thing that",
                "keeps its promise close and opens",
                "only for a patient hand.",
                "",
                "Read slowly, for the scribe is slower",
                "still. Press again and all of it rolls",
                "back to the quiet shape it came from.",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            signature: "Sealed by the Guild of Small Machines".into(),
            height: 420.0,
            duration: 2.8,
            paper_width: 190.0,
            autoplay: true,
        }
    }
    /// Two-line heading.
    pub fn title(mut self, a: impl Into<String>, b: impl Into<String>) -> Self {
        self.title = (a.into(), b.into());
        self
    }
    /// Body lines (the first line gets a red drop cap "B").
    pub fn lines(mut self, lines: &[&str]) -> Self {
        self.lines = lines.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Signature line under the text.
    pub fn signature(mut self, s: impl Into<String>) -> Self {
        self.signature = s.into();
        self
    }
    /// Widget height (default 420).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Opening duration in seconds (default 2.8).
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.clamp(0.6, 8.0);
        self
    }
    /// Parchment width in points (default 190).
    pub fn paper_width(mut self, w: f32) -> Self {
        self.paper_width = w.clamp(120.0, 400.0);
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// A horizontal wooden roller with brass knobs; `spin` scrolls the grain.
fn roller(p: &egui::Painter, c: Pos2, len: f32, r: f32, spin: f32, alpha: f32) {
    let rect = Rect::from_center_size(c, vec2(len, r * 2.0));
    let rows = 8;
    let wood = [hex("#5B3518"), hex("#B07A45"), hex("#E2B47A"), hex("#9A6533"), hex("#4A2A12")];
    let mut m = Mesh::default();
    for j in 0..=rows {
        let v = j as f32 / rows as f32;
        let col = glowkit_core::color::gradient(&wood, v);
        let col = with_alpha(col, alpha);
        m.colored_vertex(pos2(rect.min.x, rect.min.y + v * rect.height()), col);
        m.colored_vertex(pos2(rect.max.x, rect.min.y + v * rect.height()), col);
    }
    for j in 0..rows as u32 {
        let k = j * 2;
        m.add_triangle(k, k + 1, k + 3);
        m.add_triangle(k, k + 3, k + 2);
    }
    p.add(Shape::mesh(m));
    // grain lines that turn with the roller
    for g in 0..3 {
        let a = (spin + g as f32 * 2.1).rem_euclid(PI * 2.0);
        if a < PI {
            let y = c.y - a.cos() * r;
            p.line_segment(
                [pos2(rect.min.x + 2.0, y), pos2(rect.max.x - 2.0, y)],
                Stroke::new(0.7, with_alpha(hex("#3B200B"), 0.35 * alpha * a.sin())),
            );
        }
    }
    // knobs
    for side in [-1.0f32, 1.0] {
        let k = pos2(c.x + side * (len * 0.5 + r * 0.9), c.y);
        clay(
            p,
            pos2(c.x + side * (len * 0.5 + r * 0.2), c.y),
            vec2(r * 0.5, r * 0.9),
            0.0,
            with_alpha(hex("#B88A35"), alpha),
        );
        clay(p, k, vec2(r * 0.95, r * 1.15), 0.0, with_alpha(hex("#C9A04A"), alpha));
        p.add(Shape::convex_polygon(
            vec![k + vec2(side * r * 0.8, -r * 0.5), k + vec2(side * r * 2.0, 0.0), k + vec2(side * r * 0.8, r * 0.5)],
            with_alpha(hex("#A8822F"), alpha),
            Stroke::NONE,
        ));
    }
}

fn parchment(p: &egui::Painter, rect: Rect, seed: f32, alpha: f32) {
    let (cols, rows) = (10usize, ((rect.height() / 12.0) as usize).clamp(2, 40));
    let mut m = Mesh::default();
    let base = hex("#EED9AE");
    let stain = hex("#C9A46A");
    let edge = hex("#B88A50");
    for j in 0..=rows {
        for i in 0..=cols {
            let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
            let y = rect.min.y + v * rect.height();
            let wave = ((y * 0.05 + seed).sin() * 1.2 + (y * 0.13).sin() * 0.6)
                * if i == 0 {
                    1.0
                } else if i == cols {
                    -1.0
                } else {
                    0.0
                };
            let x = rect.min.x + u * rect.width() + wave;
            let n = fbm3(u * 3.0 + seed, y * 0.012, 0.5, 3) * 0.5 + 0.5;
            let side = (1.0 - (u - 0.5).abs() * 2.0).clamp(0.0, 1.0);
            let mut c = lerp_c(base, stain, (n - 0.45).max(0.0) * 1.6);
            c = lerp_c(edge, c, sstep(0.0, 0.18, side));
            m.colored_vertex(pos2(x, y), with_alpha(c, alpha));
        }
    }
    let w = cols + 1;
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * w + i) as u32;
            m.add_triangle(a, a + 1, a + w as u32 + 1);
            m.add_triangle(a, a + w as u32 + 1, a + w as u32);
        }
    }
    p.add(Shape::mesh(m));
}

fn wax_seal(p: &egui::Painter, c: Pos2, r: f32, alpha: f32) {
    let red = with_alpha(hex("#A3171F"), alpha);
    let pts: Vec<Pos2> = (0..32)
        .map(|i| {
            let a = i as f32 / 32.0 * PI * 2.0;
            let rr = r * (1.0 + 0.09 * (a * 5.0).sin() + 0.05 * (a * 9.0 + 1.0).sin());
            c + vec2(a.cos(), a.sin()) * rr
        })
        .collect();
    super::common::fan(p, c, &pts, red);
    clay(p, c, Vec2::splat(r * 0.72), 0.0, with_alpha(hex("#B8222B"), alpha));
    p.circle_stroke(c, r * 0.58, Stroke::new(1.0, with_alpha(hex("#7A0F15"), alpha)));
    p.text(c, Align2::CENTER_CENTER, "S", FontId::proportional(r * 0.8), with_alpha(hex("#6E0C12"), alpha));
}

impl Widget for UnfurlScroll {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, rect);
        let clicked = response.clicked();
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let dur = self.duration;
        let autoplay = self.autoplay && idle > 2.0;
        let mut toggled = false;
        let st = glowkit_core::with_state(ui, id, UnfurlState::default, |s| {
            if clicked {
                s.open = !s.open;
                s.auto_t = 0.0;
                toggled = true;
            } else if autoplay {
                s.auto_t += wdt;
                let wait = if s.open { 4.5 + dur } else { 1.6 };
                if s.auto_t > wait {
                    s.open = !s.open;
                    s.auto_t = 0.0;
                }
            }
            let rate = if s.open { 1.0 / dur } else { -1.8 / dur };
            s.t = (s.t + rate * dt).clamp(0.0, 1.0);
            *s
        });
        if toggled {
            response.mark_changed();
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let tt = st.t;
        let lift = sstep(0.0, 0.14, tt);
        let tip = sstep(0.1, 0.3, tt);
        let drop = ease(Easing::OutBack, sstep(0.28, 0.7, tt).powf(0.9));
        let write = sstep(0.68, 0.94, tt);
        let seal = sstep(0.92, 1.0, tt);
        let paper_w = self.paper_width;
        let full_h = (rect.height() - 70.0).max(80.0);
        let center = rect.center();
        let float = lift * (1.0 - drop) * (8.0 + (time * 2.0).sin() * 2.0);
        // the top roller travels from the button's place to the top of the open scroll
        let top_y = center.y - float - (full_h * 0.5) * drop;
        let rod_len = paper_w + 6.0;
        let roll_r = 9.0;
        let paper_len = full_h * drop;
        let shadow_k = 1.0 - drop;
        // shadow
        contact_shadow(
            &painter,
            pos2(center.x, center.y + 24.0 + paper_len * 0.5),
            vec2(rod_len * 0.55 + float * 2.0, 8.0 + paper_len * 0.08),
            (0.28 - float * 0.01) * (0.6 + 0.4 * shadow_k),
        );
        if tip < 0.999 {
            // closed button: one fat roll, tipping into two thin ones
            let a = 1.0 - tip;
            let w = 150.0 + (rod_len - 150.0) * tip;
            let r = 17.0 - 8.0 * tip;
            let c = pos2(center.x, top_y);
            let body = Rect::from_center_size(c, vec2(w, r * 2.0));
            let rows = 10;
            let mut m = Mesh::default();
            for j in 0..=rows {
                let v = j as f32 / rows as f32;
                let k = (v * PI).sin();
                let col = lerp_c(hex("#8E6A3A"), hex("#F2DDB0"), k.powf(0.7));
                let col = with_alpha(col, a.max(0.3));
                m.colored_vertex(pos2(body.min.x, body.min.y + v * body.height()), col);
                m.colored_vertex(pos2(body.max.x, body.min.y + v * body.height()), col);
            }
            for j in 0..rows as u32 {
                let k = j * 2;
                m.add_triangle(k, k + 1, k + 3);
                m.add_triangle(k, k + 3, k + 2);
            }
            painter.add(Shape::mesh(m));
            for side in [-1.0f32, 1.0] {
                clay(&painter, pos2(c.x + side * (w * 0.5 + 4.0), c.y), vec2(6.0, r * 0.9), 0.0, hex("#C9A04A"));
            }
            painter.line_segment(
                [body.left_bottom() + vec2(6.0, -r * 0.55), body.right_bottom() + vec2(-6.0, -r * 0.55)],
                Stroke::new(0.8, with_alpha(hex("#7B5527"), 0.35 * a)),
            );
            painter.text(
                c,
                Align2::CENTER_CENTER,
                spaced(&self.label),
                FontId::proportional(11.0),
                with_alpha(hex("#5A3A16"), a),
            );
        }
        if tip > 0.001 {
            let a = tip;
            // parchment between the rollers
            if paper_len > 1.0 {
                let pr = Rect::from_min_size(pos2(center.x - paper_w * 0.5, top_y), vec2(paper_w, paper_len));
                parchment(&painter, pr, 3.7, a);
                // curl shading just above the bottom roller
                let curl = Rect::from_min_max(pos2(pr.min.x, pr.max.y - 14.0), pr.max);
                glowkit_core::paint::shader_grid(&painter, curl, 1, 4, |uv| {
                    Color32::from_black_alpha((uv.y * uv.y * 60.0) as u8)
                });
                // ink
                let ink = hex("#4A2E14");
                let red = hex("#9B1C1F");
                let lines_total = self.lines.len() as f32 + 4.0;
                let reveal = |i: f32| ((write * lines_total) - i).clamp(0.0, 1.0);
                let clip = painter.with_clip_rect(pr.shrink2(vec2(0.0, 6.0)));
                let x0 = pr.min.x + 18.0;
                let mut y = pr.min.y + 26.0;
                let k0 = reveal(0.0);
                clip.text(
                    pos2(center.x, y),
                    Align2::CENTER_TOP,
                    "~ BY ORDER OF THE GUILD ~",
                    FontId::proportional(6.5),
                    with_alpha(ink, 0.6 * k0),
                );
                y += 14.0;
                let k1 = reveal(1.0);
                clip.text(
                    pos2(center.x, y),
                    Align2::CENTER_TOP,
                    self.title.0.to_uppercase(),
                    FontId::proportional(12.0),
                    with_alpha(ink, k1),
                );
                y += 16.0;
                clip.text(
                    pos2(center.x, y),
                    Align2::CENTER_TOP,
                    self.title.1.to_uppercase(),
                    FontId::proportional(12.0),
                    with_alpha(ink, k1),
                );
                y += 20.0;
                let k2 = reveal(2.0);
                clip.line_segment(
                    [pos2(center.x - 30.0 * k2, y), pos2(center.x + 30.0 * k2, y)],
                    Stroke::new(0.8, with_alpha(ink, 0.6)),
                );
                clip.circle_filled(pos2(center.x, y), 2.0 * k2, with_alpha(red, k2));
                y += 12.0;
                let k3 = reveal(3.0);
                let cap = clip.text(
                    pos2(x0, y - 2.0),
                    Align2::LEFT_TOP,
                    "B",
                    FontId::proportional(26.0),
                    with_alpha(red, k3),
                );
                for (i, line) in self.lines.iter().enumerate() {
                    let k = reveal(3.0 + i as f32);
                    if k <= 0.0 {
                        break;
                    }
                    let lx = if i < 2 { cap.max.x + 3.0 } else { x0 };
                    let g = clip.layout_no_wrap(line.clone(), FontId::proportional(8.6), ink);
                    let lr = Rect::from_min_size(pos2(lx, y), g.size());
                    // the quill writes left to right
                    let lc = clip.with_clip_rect(clip.clip_rect().intersect(Rect::from_min_max(
                        lr.min - vec2(2.0, 2.0),
                        pos2(lr.min.x + lr.width() * k + 2.0, lr.max.y + 2.0),
                    )));
                    lc.galley(lr.min, g, ink);
                    y += 12.5;
                }
                y += 8.0;
                let ks = reveal(lines_total - 1.0);
                clip.text(
                    pos2(x0, y),
                    Align2::LEFT_TOP,
                    &self.signature,
                    FontId::proportional(7.5),
                    with_alpha(ink, 0.75 * ks),
                );
                if seal > 0.0 {
                    let sc = pos2(center.x, (y + 44.0).min(pr.max.y - 40.0));
                    let k = ease(Easing::OutBack, seal);
                    let r = 15.0 * (1.6 - 0.6 * k);
                    if seal < 1.0 {
                        contact_shadow(&painter, sc + vec2(4.0, 6.0) * (2.0 - k), Vec2::splat(r * 1.1), 0.2 * seal);
                    }
                    wax_seal(&painter, sc, r, seal);
                }
            }
            roller(&painter, pos2(center.x, top_y), rod_len, roll_r, 0.0, a);
            roller(
                &painter,
                pos2(center.x, top_y + paper_len + roll_r * (0.2 + tip * 0.8)),
                rod_len,
                roll_r,
                paper_len / roll_r,
                a,
            );
        }
        // caption
        let hint =
            if st.open { "Click the scroll to roll it back up" } else { "Press the button to unfurl the charter" };
        painter.text(
            pos2(center.x, rect.max.y - 10.0),
            Align2::CENTER_BOTTOM,
            hint,
            FontId::proportional(11.5),
            t.muted_foreground,
        );
        response
    }
}

fn spaced(s: &str) -> String {
    s.to_uppercase().chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
}
