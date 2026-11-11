use egui::{pos2, vec2, Color32, Id, LayerId, Order, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::{anim::{ease, smoothstep}, color::with_alpha, hex, mix, paint, Easing, Theme};
use std::f32::consts::TAU;

/// Paint a sun (k = 0) that morphs into a crescent moon (k = 1).
fn sun_moon(p: &egui::Painter, c: Pos2, s: f32, k: f32, sun: Color32, moon: Color32, bg: Color32) {
    let rot = -k * 0.7;
    let body_r = s * (0.22 + 0.12 * k);
    let col = sun.lerp_to_gamma(moon, k);
    // rays
    let ray_k = (1.0 - k * 1.6).clamp(0.0, 1.0);
    if ray_k > 0.0 {
        for i in 0..8 {
            let a = i as f32 / 8.0 * TAU + rot + k * 1.2;
            let d = vec2(a.cos(), a.sin());
            let r0 = s * 0.34;
            let r1 = r0 + s * 0.11 * ray_k;
            p.line_segment([c + d * r0, c + d * r1], Stroke::new(s * 0.065, with_alpha(col, ray_k)));
        }
    }
    p.circle_filled(c, body_r, col);
    if k > 0.01 {
        // crescent: a background-colored disc slides over the body
        let da = -0.55 - 0.35 * (1.0 - k);
        let dir = vec2(da.cos(), da.sin());
        let mc = c + dir * body_r * (1.6 - 0.95 * k);
        p.circle_filled(mc, body_r * (0.2 + 0.68 * k.sqrt()), bg);
        // re-stroke a tiny rim to hide aliasing between the two discs
    }
}

#[derive(Clone, Copy, Debug)]
struct RevealState {
    last: bool,
    start: f64,
    to_dark: bool,
}

/// An icon button that toggles between light and dark: a sun whose rays retract as it morphs
/// into a crescent moon. When the value changes, the new theme is revealed through a circle that
/// expands from the button over an optional area, like a view-transition theme switch.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut dark = true;
/// let area = ui.max_rect();
/// ui.add(glowkit_blocks::ThemeToggler::new(&mut dark).reveal(area));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ThemeToggler<'a> {
    dark: &'a mut bool,
    size: f32,
    reveal: Option<Rect>,
    colors: [Color32; 2],
    duration: f32,
    rounding: f32,
}

impl<'a> ThemeToggler<'a> {
    /// Toggle for `dark` (true = dark mode).
    pub fn new(dark: &'a mut bool) -> Self {
        Self { dark, size: 40.0, reveal: None, colors: [Theme::light().background, Theme::dark().background], duration: 0.7, rounding: 0.0 }
    }
    /// Button diameter in points.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Area the circular reveal covers when the value changes.
    pub fn reveal(mut self, area: Rect) -> Self {
        self.reveal = Some(area);
        self
    }
    /// Background colors of the light and dark themes (the old one is held outside the circle).
    pub fn reveal_colors(mut self, light: Color32, dark: Color32) -> Self {
        self.colors = [light, dark];
        self
    }
    /// Corner radius of the reveal area (so the wipe respects rounded containers).
    pub fn reveal_rounding(mut self, r: f32) -> Self {
        self.rounding = r.max(0.0);
        self
    }
    /// Seconds the reveal takes to cover the area.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.05);
        self
    }
}

impl Widget for ThemeToggler<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::click());
        if resp.clicked() {
            *self.dark = !*self.dark;
            resp.mark_changed();
        }
        let dark = *self.dark;
        let now = ui.input(|i| i.time);
        let st = glowkit_core::with_state(ui, resp.id.with("reveal"), || RevealState { last: dark, start: -100.0, to_dark: dark }, |s| {
            if s.last != dark {
                s.last = dark;
                s.start = now;
                s.to_dark = dark;
            }
            *s
        });
        let k = glowkit_core::anim::spring(ui, resp.id.with("k"), if dark { 1.0 } else { 0.0 }, 180.0, 18.0);
        if ui.is_rect_visible(rect) {
            let hover = glowkit_core::anim::follow(ui, resp.id.with("h"), if resp.hovered() { 1.0 } else { 0.0 }, 14.0);
            let p = ui.painter();
            let c = rect.center();
            let bg = mix(t.muted, t.secondary, hover);
            p.circle_filled(c, self.size / 2.0, bg);
            p.circle_stroke(c, self.size / 2.0 - 0.5, Stroke::new(1.0, t.border));
            let sun = if t.dark { hex("#FBBF24") } else { hex("#F59E0B") };
            let moon = if t.dark { hex("#E0E7FF") } else { hex("#4F46E5") };
            sun_moon(p, c, self.size * (1.0 + hover * 0.06), k.clamp(0.0, 1.05), sun, moon, bg);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        // the circular reveal: the area outside a growing circle keeps the old background
        // (fading out at the end) so the new theme appears inside the circle.
        if let Some(area) = self.reveal {
            let e = (now - st.start) as f32;
            let total = self.duration + 0.3;
            if e < total {
                ui.ctx().request_repaint();
                let c = rect.center().clamp(area.min, area.max);
                let far = [area.left_top(), area.right_top(), area.left_bottom(), area.right_bottom()]
                    .iter()
                    .map(|q| q.distance(c))
                    .fold(0.0, f32::max);
                let grow = ease(Easing::InOutCubic, e / self.duration);
                let fade = 1.0 - ((e - self.duration) / 0.3).clamp(0.0, 1.0);
                let old = if st.to_dark { self.colors[0] } else { self.colors[1] };
                let lp = ui.ctx().layer_painter(LayerId::new(Order::Middle, Id::unique("glowkit_theme_reveal").with(resp.id))).with_clip_rect(area);
                let rr = (far * grow).max(self.size * 0.5 + 3.0);
                super::util::rounded_grid(&lp, area, self.rounding, 4.0, |q| {
                    let d = q.distance(c);
                    with_alpha(old, fade * smoothstep(rr - 2.0, rr + 2.0, d))
                });
                if rr > 0.5 && grow < 1.0 {
                    let edge = if st.to_dark { t.gradient[0] } else { t.gradient[2] };
                    lp.circle_stroke(c, rr, Stroke::new(2.0, with_alpha(edge, (1.0 - grow) * 0.8)));
                }
            }
        }
        resp
    }
}

/// A sliding day / night switch: a sky-blue track with clouds and a sun knob that slides into a
/// starry night track with a cratered moon.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut night = false;
/// ui.add(glowkit_blocks::DayNightSwitch::new(&mut night).width(84.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DayNightSwitch<'a> {
    night: &'a mut bool,
    width: f32,
}

impl<'a> DayNightSwitch<'a> {
    /// Switch bound to `night` (true = night).
    pub fn new(night: &'a mut bool) -> Self {
        Self { night, width: 76.0 }
    }
    /// Track width in points (height is ~44% of it).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl Widget for DayNightSwitch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(self.width, self.width * 0.44);
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
        if resp.clicked() {
            *self.night = !*self.night;
            resp.mark_changed();
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let k = glowkit_core::anim::spring(ui, resp.id.with("k"), if *self.night { 1.0 } else { 0.0 }, 220.0, 20.0);
        let kc = k.clamp(0.0, 1.0);
        let time = glowkit_core::time(ui);
        let p = ui.painter_at(rect.expand(6.0));
        let h = rect.height();
        let top = hex("#60A5FA").lerp_to_gamma(hex("#0B1026"), kc);
        let bot = hex("#BAE6FD").lerp_to_gamma(hex("#312E81"), kc);
        paint::gradient_rect(&p, rect, h / 2.0, 90.0, &[top, bot]);
        let clip = p.with_clip_rect(rect.shrink(1.0));
        // stars
        for i in 0..9 {
            let r = glowkit_core::rng::hash01(i * 7 + 1);
            let r2 = glowkit_core::rng::hash01(i * 13 + 5);
            let sp = pos2(rect.min.x + h * 0.3 + r * (self.width - h * 1.15), rect.min.y + h * (0.2 + r2 * 0.6) - (1.0 - kc) * h * 0.5);
            let tw = 0.55 + 0.45 * (time * 2.0 + i as f32).sin();
            if i % 3 == 0 {
                paint::sparkle(&clip, sp, h * (0.06 + r2 * 0.05), 0.0, with_alpha(Color32::WHITE, kc * tw));
            } else {
                clip.circle_filled(sp, h * 0.025 + 0.4, with_alpha(Color32::WHITE, kc * tw));
            }
        }
        // clouds
        let cloud_off = kc * h * 0.9;
        let cc = Color32::from_white_alpha(((1.0 - kc) * 235.0) as u8);
        for (dx, dy, rr) in [(0.78, 0.78, 0.26), (0.62, 0.86, 0.22), (0.92, 0.7, 0.24), (0.86, 0.95, 0.2), (0.5, 0.98, 0.16)] {
            let pts: Vec<Pos2> = paint::circle_points(pos2(rect.min.x + self.width * dx, rect.min.y + h * dy + cloud_off), h * rr, 32)
                .into_iter()
                .map(|q| clamp_to_stadium(q, rect))
                .collect();
            clip.add(egui::Shape::convex_polygon(pts, cc, Stroke::NONE));
        }
        // knob
        let pad = h * 0.1;
        let kr = h / 2.0 - pad;
        let x = rect.min.x + pad + kr + (self.width - 2.0 * (pad + kr)) * k;
        let kcen = pos2(x, rect.center().y);
        // halo rings
        for i in 1..=3 {
            let pts: Vec<Pos2> = paint::circle_points(kcen, kr + h * 0.16 * i as f32, 48).into_iter().map(|q| clamp_to_stadium(q, rect)).collect();
            clip.add(egui::Shape::convex_polygon(pts, Color32::from_white_alpha((14.0 - i as f32 * 3.0) as u8), Stroke::NONE));
        }
        paint::soft_shadow(&p, Rect::from_center_size(kcen, Vec2::splat(kr * 2.0)), kr, 6.0, vec2(0.0, 2.0), Color32::from_black_alpha(70));
        let sun_c = hex("#FCD34D");
        let moon_c = hex("#E5E7EB");
        let kcol = sun_c.lerp_to_gamma(moon_c, kc);
        paint::gradient_circle(&p, kcen, kr, &[mix(kcol, Color32::WHITE, 0.35), kcol, mix(kcol, Color32::BLACK, 0.1)]);
        // craters
        if kc > 0.0 {
            let crc = with_alpha(hex("#9CA3AF"), kc);
            p.circle_filled(kcen + vec2(-kr * 0.3, -kr * 0.25), kr * 0.22, crc);
            p.circle_filled(kcen + vec2(kr * 0.35, kr * 0.05), kr * 0.15, crc);
            p.circle_filled(kcen + vec2(-kr * 0.05, kr * 0.42), kr * 0.12, crc);
        }
        resp
    }
}

/// Project `q` into the pill (stadium) shape inscribed in `r`.
fn clamp_to_stadium(q: Pos2, r: Rect) -> Pos2 {
    let rad = r.height() / 2.0 - 0.5;
    let cy = r.center().y;
    let cx = q.x.clamp(r.min.x + rad + 0.5, r.max.x - rad - 0.5);
    let c = pos2(cx, cy);
    let d = q - c;
    if d.length() > rad {
        c + d.normalized() * rad
    } else {
        q
    }
}
