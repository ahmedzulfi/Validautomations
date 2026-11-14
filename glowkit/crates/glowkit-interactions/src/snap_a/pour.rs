use super::common::{ghost_cursor, idle_time, iso_fill, sd_round_rect, smin, sstep, wall_dt};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, Theme};

#[derive(Clone, Debug)]
struct PourState {
    selected: Vec<bool>,
    order: Vec<usize>,
    x: Vec<Spring>,
    fill: Vec<f32>,
    count: Spring,
    ghost: f32,
    script: usize,
}

/// A filter bar of liquid ink. Tap a tag and the result-count pill pours into it: a tongue of
/// ink runs across, floods the chip and swallows it into the pill while the count rolls down.
/// Tap it again and the ink drains back, necks and snaps free. The ink is a real implicit
/// surface (smooth-unioned rounded rects, marched into a mesh), so merges and splits are fluid.
///
/// `selected` is caller-owned (one flag per tag). The response reports `changed()` on toggles.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut on = vec![false; 4];
/// use glowkit_interactions::PourFilter;
/// ui.add(PourFilter::new(&["Sauna", "Lakeside", "Dogs OK", "Hot tub"], &mut on).total(86).noun("cabins"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PourFilter<'a> {
    tags: Vec<String>,
    selected: &'a mut Vec<bool>,
    total: u32,
    noun: String,
    ink: Option<Color32>,
    viscosity: f32,
    height: f32,
    autoplay: bool,
}

impl<'a> PourFilter<'a> {
    /// A filter bar for `tags`, with toggle state in `selected`.
    pub fn new(tags: &[&str], selected: &'a mut Vec<bool>) -> Self {
        Self {
            tags: tags.iter().map(|s| s.to_string()).collect(),
            selected,
            total: 86,
            noun: "results".into(),
            ink: None,
            viscosity: 1.0,
            height: 120.0,
            autoplay: true,
        }
    }
    /// Unfiltered result count (default 86).
    pub fn total(mut self, n: u32) -> Self {
        self.total = n;
        self
    }
    /// Noun after the count, e.g. `"cabins"`.
    pub fn noun(mut self, s: impl Into<String>) -> Self {
        self.noun = s.into();
        self
    }
    /// Ink color (default: a deep ultramarine, lighter on dark themes).
    pub fn ink(mut self, c: Color32) -> Self {
        self.ink = Some(c);
        self
    }
    /// How thick the ink is: higher melts shapes together from further away (default 1).
    pub fn viscosity(mut self, k: f32) -> Self {
        self.viscosity = k.clamp(0.3, 2.5);
        self
    }
    /// Widget height (default 120).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(60.0);
        self
    }
    /// Demonstrate with a ghost cursor while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// A deterministic "result count" for a set of filters.
fn count_for(total: u32, sel: &[bool]) -> f32 {
    let mut c = total as f32;
    for (i, on) in sel.iter().enumerate() {
        if *on {
            c *= [0.6, 0.48, 0.55, 0.4, 0.66, 0.5][i % 6];
        }
    }
    c.round().max(1.0)
}

/// Odometer-style number: each digit rolls vertically toward its target.
fn rolling_number(painter: &egui::Painter, right: Pos2, value: f32, font: FontId, color: Color32, clip: Rect) {
    let target = value.round().max(0.0) as u32;
    let digits = format!("{target}").len().max(format!("{}", value.ceil() as u32).len());
    let dw = painter.layout_no_wrap("0".into(), font.clone(), color).size();
    let p = painter.with_clip_rect(clip.intersect(Rect::from_center_size(
        right - vec2(dw.x * digits as f32 * 0.5, 0.0),
        vec2(dw.x * digits as f32 + 4.0, dw.y),
    )));
    for d in 0..digits {
        let place = 10f32.powi(d as i32);
        let v = value / place;
        let whole = v.floor();
        let roll = if d == 0 {
            sstep(0.0, 1.0, value.fract())
        } else {
            (value.rem_euclid(place) - (place - 1.0)).clamp(0.0, 1.0)
        };
        let a = (whole as u32) % 10;
        let b = (a + 1) % 10;
        let x = right.x - (d as f32 + 0.5) * dw.x;
        if v < 1.0 && d > 0 && a == 0 && roll < 0.01 {
            continue;
        }
        p.text(pos2(x, right.y - roll * dw.y), Align2::CENTER_CENTER, a.to_string(), font.clone(), color);
        p.text(pos2(x, right.y + (1.0 - roll) * dw.y), Align2::CENTER_CENTER, b.to_string(), font.clone(), color);
    }
}

impl Widget for PourFilter<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.tags.len();
        self.selected.resize(n, false);
        let (rect, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let ink = self.ink.unwrap_or(if t.dark { hex("#4F5BFF") } else { hex("#2025C9") });
        let font = FontId::proportional(13.0);
        let chip_h = 32.0;
        let pad = 14.0;
        let gap = 8.0;
        let widths: Vec<f32> = self
            .tags
            .iter()
            .map(|s| ui.painter().layout_no_wrap(s.clone(), font.clone(), Color32::WHITE).size().x + pad * 2.0)
            .collect();
        let count_w =
            50.0 + ui.painter().layout_no_wrap(self.noun.clone(), font.clone(), Color32::WHITE).size().x + pad;
        let total_w = count_w + widths.iter().sum::<f32>() + gap * n as f32;
        let x0 = rect.center().x - total_w * 0.5;
        let cy = rect.center().y;
        let sel_now = self.selected.clone();
        let st0: PourState = glowkit_core::state(ui, id, || PourState {
            selected: sel_now.clone(),
            order: (0..n).collect(),
            x: vec![Spring::new(0.0); n],
            fill: sel_now.iter().map(|s| if *s { 1.0 } else { 0.0 }).collect(),
            count: Spring::new(0.0),
            ghost: -1.0,
            script: 0,
        });
        // target layout: count pill, selected chips (in order), then the rest
        let mut layout = vec![0.0; n];
        {
            let mut x = x0 + count_w;
            for &i in st0.order.iter().filter(|i| sel_now[**i]) {
                layout[i] = x;
                x += widths[i];
            }
            x += gap * 1.6;
            for i in (0..n).filter(|i| !sel_now[*i]) {
                layout[i] = x;
                x += widths[i] + gap;
            }
        }
        let chip_rect = |i: usize, x: f32| Rect::from_min_size(pos2(x, cy - chip_h * 0.5), vec2(widths[i], chip_h));
        // clicks
        let mut toggled = None;
        for i in 0..n {
            let x =
                st0.x.get(i).map_or(layout[i], |s| if s.value == 0.0 && s.target == 0.0 { layout[i] } else { s.value });
            let r = ui.interact(chip_rect(i, x), id.with(("chip", i)), Sense::click());
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                toggled = Some(i);
            }
        }
        // ghost cursor script
        let autoplay = self.autoplay && idle > 2.5;
        let script = [0usize, 1, 3, 1, 0, 3];
        let mut ghost_draw = None;
        let (gx, gstep) = {
            let s = &st0;
            let step = script[s.script % script.len()] % n.max(1);
            (s.x.get(step).map_or(layout[step], |sp| sp.value) + widths[step] * 0.5, step)
        };
        let st_ghost = st0.ghost;
        if toggled.is_none() && (autoplay || st_ghost >= 0.0) && n > 0 {
            let g = st_ghost.max(0.0);
            let from = pos2(gx + 40.0, cy + 46.0);
            let to = pos2(gx, cy + 4.0);
            let tip = from.lerp(to, sstep(0.0, 0.7, g));
            let pressed = (0.75..0.95).contains(&g);
            ghost_draw = Some((tip, pressed, sstep(0.0, 0.25, g) * (1.0 - sstep(1.3, 1.6, g))));
            if g < 0.85 && g + wdt >= 0.85 {
                toggled = Some(gstep);
            }
        }
        if let Some(i) = toggled {
            self.selected[i] = !self.selected[i];
            response.mark_changed();
        }
        let sel = self.selected.clone();
        // recompute layout targets after a toggle
        let mut layout = vec![0.0; n];
        let mut order = st0.order.clone();
        if let Some(i) = toggled {
            order.retain(|k| *k != i);
            order.push(i);
        }
        {
            let mut x = x0 + count_w;
            for &i in order.iter().filter(|i| sel[**i]) {
                layout[i] = x;
                x += widths[i];
            }
            x += gap * 1.6;
            for i in (0..n).filter(|i| !sel[*i]) {
                layout[i] = x;
                x += widths[i] + gap;
            }
        }
        let visc = self.viscosity;
        let target_count = count_for(self.total, &sel);
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0.clone(),
            |s| {
                s.order = order.clone();
                s.selected = sel.clone();
                s.x.resize(n, Spring::new(0.0));
                s.fill.resize(n, 0.0);
                for i in 0..n {
                    if s.x[i].value == 0.0 && s.x[i].target == 0.0 {
                        s.x[i] = Spring::new(layout[i]).with(260.0, 22.0);
                    }
                    s.x[i].target = layout[i];
                    s.x[i].step(dt);
                    // ink floods the chip once it has nearly arrived; drains straight away
                    let near = (s.x[i].value - layout[i]).abs() < widths[i] * 0.6;
                    let goal = if sel[i] && near { 1.0 } else { 0.0 };
                    let rate = if goal > s.fill[i] { 3.2 / visc } else { 4.5 };
                    s.fill[i] += (goal - s.fill[i]).clamp(-rate * dt, rate * dt);
                }
                if s.count.value == 0.0 {
                    s.count = Spring::new(target_count).with(60.0, 16.0);
                }
                s.count.target = target_count;
                s.count.step(dt);
                if toggled.is_some() && s.ghost < 0.0 {
                    s.ghost = -1.0;
                }
                if autoplay || s.ghost >= 0.0 {
                    s.ghost += wdt;
                    if s.ghost < 0.0 {
                        s.ghost = 0.0;
                    }
                    if s.ghost > 2.3 {
                        s.ghost = -1.0;
                        s.script += 1;
                    }
                }
                s.clone()
            },
        );
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let count_rect = Rect::from_min_size(pos2(x0, cy - chip_h * 0.5), vec2(count_w, chip_h));
        // ---- the ink surface
        let k = 9.0 * visc;
        let inks: Vec<(Rect, f32)> = (0..n)
            .filter(|i| st.fill[*i] > 0.001)
            .map(|i| {
                let r = chip_rect(i, st.x[i].value);
                let f = st.fill[i];
                // flood from the left (where the pill is), as a rounded tongue
                let w = (r.width() * f).max(4.0);
                let h = chip_h * (0.55 + 0.45 * sstep(0.0, 0.35, f));
                (Rect::from_min_size(pos2(r.min.x - 4.0 * (1.0 - f), cy - h * 0.5), vec2(w + 4.0 * (1.0 - f), h)), f)
            })
            .collect();
        let field = |p: Pos2| {
            let mut d = sd_round_rect(p, count_rect, chip_h * 0.5);
            for (r, _) in &inks {
                d = smin(d, sd_round_rect(p, *r, r.height() * 0.5), k);
            }
            -d
        };
        let area = Rect::from_min_max(pos2(x0 - 12.0, cy - chip_h), pos2(rect.max.x, cy + chip_h));
        let ink_hi = glowkit_core::color::shade(ink, 0.08);
        let segs = iso_fill(&painter, area, 2.5, field, |p| {
            super::common::lerp_c(ink_hi, ink, sstep(cy - chip_h * 0.5, cy + 2.0, p.y))
        });
        for (a, b) in segs {
            painter.line_segment([a, b], Stroke::new(1.0, ink));
        }
        // ---- chips + labels
        let on_ink = Color32::WHITE;
        for i in 0..n {
            let r = chip_rect(i, st.x[i].value);
            let f = st.fill[i];
            if f < 0.98 {
                painter.rect_stroke(
                    r.shrink(0.5),
                    chip_h * 0.5,
                    Stroke::new(1.1, with_alpha(ink, 1.0 - f)),
                    egui::StrokeKind::Inside,
                );
            }
            let col = glowkit_core::color::mix(ink, on_ink, sstep(0.35, 0.8, f));
            painter.text(r.center(), Align2::CENTER_CENTER, &self.tags[i], font.clone(), col);
        }
        // count + divider
        let num_right = pos2(count_rect.min.x + 14.0 + 18.0, cy);
        rolling_number(&painter, num_right, st.count.value, FontId::proportional(14.0), on_ink, area);
        painter.text(
            num_right + vec2(6.0, 0.0),
            Align2::LEFT_CENTER,
            &self.noun,
            font.clone(),
            with_alpha(on_ink, 0.75),
        );
        if st.fill.iter().any(|f| *f > 0.5) {
            painter.line_segment(
                [pos2(count_rect.max.x - 1.0, cy - 8.0), pos2(count_rect.max.x - 1.0, cy + 8.0)],
                Stroke::new(1.0, with_alpha(on_ink, 0.3)),
            );
        }
        if let Some((tip, pressed, a)) = ghost_draw {
            ghost_cursor(&painter, t.dark, tip, a, pressed);
        }
        response
    }
}
