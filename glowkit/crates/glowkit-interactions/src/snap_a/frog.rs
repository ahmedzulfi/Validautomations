use super::common::{clay, ellipse_pts, idle_time, lerp_c, sstep, wall_dt};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
use glowkit_core::anim::Spring;
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::{hex, paint, Theme};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
struct Cookie {
    from: Pos2,
    age: f32,
}

#[derive(Clone, Debug)]
struct FrogState {
    knobs: Vec<Spring>,
    cookies: Vec<Cookie>,
    crown: Spring,
    sad: f32,
    gulp: f32,
    auto_t: f32,
    auto_step: usize,
}

/// A cookie banner with a clay frog. Switch a category on and its cookie pops out of the
/// toggle; the frog's tongue snaps it out of the air and it gulps it down. Allow everything and
/// it smiles and earns a crown; reject everything and it sulks. The eyes follow every cookie.
///
/// `prefs` is caller-owned, one flag per category. The response reports `changed()` on toggles.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut prefs = vec![false; 3];
/// ui.add(glowkit_interactions::CookieFrog::new(&mut prefs).name("Pip"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CookieFrog<'a> {
    prefs: &'a mut Vec<bool>,
    categories: Vec<(String, String)>,
    name: String,
    width: f32,
    tongue_speed: f32,
    autoplay: bool,
}

impl<'a> CookieFrog<'a> {
    /// A cookie banner for three default categories (Analytics, Marketing, Personalisation).
    pub fn new(prefs: &'a mut Vec<bool>) -> Self {
        Self {
            prefs,
            categories: vec![
                ("Analytics".into(), "Shows us what is working".into()),
                ("Marketing".into(), "Fewer, better offers".into()),
                ("Personalisation".into(), "Remembers how you like things".into()),
            ],
            name: "Pip".into(),
            width: 340.0,
            tongue_speed: 1.0,
            autoplay: true,
        }
    }
    /// Replace the categories: `(title, description)` pairs.
    pub fn categories(mut self, c: &[(&str, &str)]) -> Self {
        if !c.is_empty() {
            self.categories = c.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        }
        self
    }
    /// The frog's name (used in the subtitle).
    pub fn name(mut self, s: impl Into<String>) -> Self {
        self.name = s.into();
        self
    }
    /// Card width (default 340).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(280.0, 520.0);
        self
    }
    /// Tongue speed multiplier (default 1).
    pub fn tongue_speed(mut self, k: f32) -> Self {
        self.tongue_speed = k.clamp(0.4, 2.5);
        self
    }
    /// Play with the switches while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const FLY: f32 = 0.55;
const SNAP: f32 = 0.16;
const BACK: f32 = 0.22;

fn paint_cookie(p: &egui::Painter, c: Pos2, r: f32, rot: f32) {
    clay(p, c, Vec2::splat(r), 0.0, hex("#D9A15A"));
    for k in 0..5 {
        let a = rot + k as f32 * 1.37;
        let d = r * (0.25 + 0.4 * ((k * 7 % 5) as f32 / 5.0));
        p.circle_filled(c + vec2(a.cos(), a.sin()) * d, r * 0.16, hex("#5A3416"));
    }
}

/// Mouth position for a frog sitting at `base` with scale `s`.
fn mouth_of(base: Pos2, s: f32) -> Pos2 {
    base + vec2(0.0, -40.0 * s)
}

#[allow(clippy::too_many_arguments)]
fn paint_frog(
    p: &egui::Painter,
    base: Pos2,
    s: f32,
    look: Vec2,
    happy: f32,
    sad: f32,
    gulp: f32,
    time: f32,
    crown: f32,
) {
    let green = hex("#6CBF4A");
    let dark = hex("#4E9A34");
    let breathe = (time * 2.2).sin() * 0.8 * s;
    // back legs
    clay(p, base + vec2(-34.0, -6.0) * s, vec2(20.0, 13.0) * s, 0.3, dark);
    clay(p, base + vec2(34.0, -6.0) * s, vec2(20.0, 13.0) * s, -0.3, dark);
    // body
    clay(p, base + vec2(0.0, -26.0 * s - breathe), vec2(40.0, 33.0) * s, 0.0, green);
    clay(p, base + vec2(0.0, -16.0 * s - breathe * 0.5), vec2(25.0, 18.0) * s, 0.0, hex("#F1EBC7"));
    // spots
    for (dx, dy, r) in [(-24.0, -38.0, 3.5), (22.0, -42.0, 3.0), (-14.0, -50.0, 2.4), (28.0, -28.0, 2.6)] {
        p.circle_filled(base + vec2(dx, dy - breathe / s) * s, r * s, with_alpha(dark, 0.7));
    }
    // front feet
    clay(p, base + vec2(-15.0, -3.0) * s, vec2(9.0, 5.0) * s, 0.0, dark);
    clay(p, base + vec2(15.0, -3.0) * s, vec2(9.0, 5.0) * s, 0.0, dark);
    // cheeks puff when gulping
    let puff = (gulp * PI).sin().max(0.0);
    if puff > 0.01 {
        clay(p, base + vec2(-22.0, -36.0) * s, Vec2::splat(9.0 * s * puff), 0.0, shade(green, 0.04));
        clay(p, base + vec2(22.0, -36.0) * s, Vec2::splat(9.0 * s * puff), 0.0, shade(green, 0.04));
    }
    // eyes
    for side in [-1.0f32, 1.0] {
        let ec = base + vec2(side * 17.0, -58.0 - breathe / s) * s;
        clay(p, ec, Vec2::splat(14.0 * s), 0.0, green);
        p.circle_filled(ec + vec2(0.0, -1.0) * s, 10.5 * s, hex("#FDFCF3"));
        let pup = ec + vec2(0.0, -1.0) * s + look * 3.2 * s;
        p.circle_filled(pup, 7.0 * s, hex("#E5A92D"));
        p.add(Shape::convex_polygon(ellipse_pts(pup, vec2(2.4, 5.2) * s, 0.0, 16), hex("#1A140C"), Stroke::NONE));
        p.circle_filled(pup + vec2(-2.0, -2.5) * s, 1.4 * s, Color32::WHITE);
        // lids: droop when sad (a clipped disc of skin over the top of the eyeball)
        let lid = 0.18 + sad * 0.42;
        let top = ec + vec2(0.0, -1.0) * s;
        let lid_y = top.y - 11.0 * s + lid * 22.0 * s;
        let lp = p.with_clip_rect(
            p.clip_rect()
                .intersect(Rect::from_min_max(pos2(top.x - 13.0 * s, top.y - 13.0 * s), pos2(top.x + 13.0 * s, lid_y))),
        );
        lp.circle_filled(top, 11.2 * s, shade(green, 0.03));
        p.line_segment(
            [
                pos2(top.x - (11.0 * s).min(((11.0 * s).powi(2) - (lid_y - top.y).powi(2)).max(0.0).sqrt()), lid_y),
                pos2(top.x + ((11.0 * s).powi(2) - (lid_y - top.y).powi(2)).max(0.0).sqrt(), lid_y),
            ],
            Stroke::new(1.0 * s, dark),
        );
    }
    // mouth
    let m = mouth_of(base, s) - vec2(0.0, breathe);
    let curve = happy * 6.0 - sad * 5.0;
    let pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0 * 2.0 - 1.0;
            m + vec2(t * 22.0, curve * (1.0 - t * t) - 2.0 * t * t) * s
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.8 * s, hex("#2F5E1F"))));
    p.circle_filled(base + vec2(-26.0, -40.0) * s, 4.5 * s, with_alpha(hex("#F58B8B"), 0.3 + 0.3 * happy));
    p.circle_filled(base + vec2(26.0, -40.0) * s, 4.5 * s, with_alpha(hex("#F58B8B"), 0.3 + 0.3 * happy));
    // crown
    if crown > 0.01 {
        let c = base + vec2(0.0, -78.0 - (1.0 - crown.min(1.0)) * 50.0) * s;
        let w = 15.0 * s;
        let pts = vec![
            c + vec2(-w, 8.0 * s),
            c + vec2(-w, -6.0 * s),
            c + vec2(-w * 0.5, 1.0 * s),
            c + vec2(0.0, -10.0 * s),
            c + vec2(w * 0.5, 1.0 * s),
            c + vec2(w, -6.0 * s),
            c + vec2(w, 8.0 * s),
        ];
        let gold = hex("#F1C232");
        let a = crown.clamp(0.0, 1.0);
        super::common::fan(p, c + vec2(0.0, 3.0 * s), &pts, with_alpha(gold, a));
        p.add(Shape::closed_line(pts, Stroke::new(1.0, with_alpha(shade(gold, -0.2), a))));
        p.circle_filled(c + vec2(0.0, 3.0 * s), 2.2 * s, with_alpha(hex("#E2455A"), a));
        p.circle_filled(c + vec2(-w * 0.6, 4.0 * s), 1.6 * s, with_alpha(hex("#3C8DDE"), a));
        p.circle_filled(c + vec2(w * 0.6, 4.0 * s), 1.6 * s, with_alpha(hex("#3C8DDE"), a));
    }
}

fn paint_toggle(p: &egui::Painter, r: Rect, k: f32, t: &Theme) {
    let on = hex("#2FB45A");
    let track = lerp_c(with_alpha(t.muted_foreground, 0.25), on, k.clamp(0.0, 1.0));
    p.rect_filled(r, r.height() * 0.5, track);
    let d = r.height() - 4.0;
    let x = r.min.x + 2.0 + d * 0.5 + (r.width() - 4.0 - d) * k;
    p.circle_filled(pos2(x, r.center().y + 1.0), d * 0.5, Color32::from_black_alpha(30));
    p.circle_filled(pos2(x, r.center().y), d * 0.5, Color32::WHITE);
}

impl Widget for CookieFrog<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.categories.len();
        self.prefs.resize(n, false);
        let w = self.width.min(ui.available_width());
        let h = 196.0 + 90.0 + n as f32 * 48.0;
        let (outer, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), h + 8.0), Sense::hover());
        let id = response.id;
        let card = Rect::from_center_size(outer.center(), vec2(w, h));
        let panel = Rect::from_min_size(card.min + vec2(8.0, 8.0), vec2(w - 16.0, 150.0));
        let x0 = card.min.x + 20.0;
        let inner_w = w - 40.0;
        let dt = glowkit_core::dt(ui) * self.tongue_speed;
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, outer);
        let s = (panel.width() / 330.0).clamp(0.8, 1.2);
        let frog_base = pos2(panel.center().x, panel.max.y - 26.0);
        let mouth = mouth_of(frog_base, s);
        let rows_top = panel.max.y + 64.0;
        let toggle_rect = |i: usize| {
            Rect::from_min_size(pos2(x0 + inner_w - 40.0, rows_top + i as f32 * 48.0 + 8.0), vec2(40.0, 22.0))
        };
        let btn_y = rows_top + n as f32 * 48.0 + 10.0;
        let allow = Rect::from_min_size(pos2(x0, btn_y), vec2(inner_w * 0.5 - 5.0, 40.0));
        let reject = Rect::from_min_size(pos2(x0 + inner_w * 0.5 + 5.0, btn_y), vec2(inner_w * 0.5 - 5.0, 40.0));
        let mut flips: Vec<usize> = Vec::new();
        for i in 0..n {
            let r = ui.interact(toggle_rect(i).expand(4.0), id.with(("tg", i)), Sense::click());
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                flips.push(i);
            }
        }
        let allow_r = ui.interact(allow, id.with("allow"), Sense::click());
        let reject_r = ui.interact(reject, id.with("reject"), Sense::click());
        for r in [&allow_r, &reject_r] {
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        let allow_all = allow_r.clicked();
        let mut reject_all = reject_r.clicked();
        // autoplay script: switch on one by one, bask, reject, repeat
        let st0: FrogState = glowkit_core::state(ui, id, || FrogState {
            knobs: vec![Spring::new(0.0).with(320.0, 20.0); n],
            cookies: Vec::new(),
            crown: Spring::new(0.0).with(180.0, 9.0),
            sad: 0.0,
            gulp: 0.0,
            auto_t: 1.5,
            auto_step: 0,
        });
        let mut auto_step = st0.auto_step;
        let mut auto_t = st0.auto_t;
        if self.autoplay && idle > 2.5 && flips.is_empty() && !allow_all && !reject_all {
            auto_t -= wdt;
            if auto_t <= 0.0 {
                let step = auto_step % (n + 2);
                if step < n {
                    if !self.prefs[step] {
                        flips.push(step);
                    }
                    auto_t = 1.4;
                } else if step == n {
                    auto_t = 2.6;
                } else {
                    reject_all = true;
                    auto_t = 2.0;
                }
                auto_step += 1;
            }
        }
        let mut launched: Vec<usize> = Vec::new();
        for i in flips {
            self.prefs[i] = !self.prefs[i];
            if self.prefs[i] {
                launched.push(i);
            }
            response.mark_changed();
        }
        if allow_all {
            for i in 0..n {
                if !self.prefs[i] {
                    self.prefs[i] = true;
                    launched.push(i);
                }
            }
            response.mark_changed();
        }
        if reject_all {
            for i in 0..n {
                self.prefs[i] = false;
            }
            response.mark_changed();
        }
        let prefs = self.prefs.clone();
        let all_on = prefs.iter().all(|p| *p);
        let any_on = prefs.iter().any(|p| *p);
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0.clone(),
            |st| {
                st.auto_step = auto_step;
                st.auto_t = auto_t;
                st.knobs.resize(n, Spring::new(0.0).with(320.0, 20.0));
                for (k, i) in launched.iter().enumerate() {
                    let r = toggle_rect(*i);
                    st.cookies.push(Cookie { from: pos2(r.max.x - 11.0, r.center().y), age: -(k as f32) * 0.35 });
                }
                for i in 0..n {
                    st.knobs[i].target = if prefs[i] { 1.0 } else { 0.0 };
                    st.knobs[i].step(dt);
                }
                for c in &mut st.cookies {
                    let before = c.age;
                    c.age += dt;
                    if before < FLY + SNAP + BACK && c.age >= FLY + SNAP + BACK {
                        st.gulp = 1.0;
                    }
                }
                st.cookies.retain(|c| c.age < FLY + SNAP + BACK);
                st.gulp = (st.gulp - dt * 2.2).max(0.0);
                st.crown.target = if all_on { 1.0 } else { 0.0 };
                st.crown.step(dt);
                if reject_all {
                    st.sad = 1.0;
                }
                st.sad = if any_on { (st.sad - dt * 3.0).max(0.0) } else { (st.sad - dt * 0.25).max(0.0) };
                st.clone()
            },
        );
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(outer) {
            return response;
        }
        let painter = ui.painter_at(outer);
        // card
        paint::soft_shadow(
            &painter,
            card,
            16.0,
            24.0,
            vec2(0.0, 10.0),
            Color32::from_black_alpha(if t.dark { 110 } else { 26 }),
        );
        let card_fill = if t.dark { shade(t.card, 0.03) } else { Color32::WHITE };
        painter.rect_filled(card, 16.0, card_fill);
        painter.rect_stroke(card, 16, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        // pond
        let pp = painter.with_clip_rect(panel);
        paint::shader_grid(&pp, panel, 1, 12, |uv| lerp_c(hex("#CFF0E2"), hex("#8FD3BC"), uv.y));
        let pad_c = frog_base + vec2(0.0, 4.0);
        for k in 0..3 {
            let ph = (time * 0.35 + k as f32 / 3.0).fract();
            let r = vec2(70.0, 16.0) * s * (1.0 + ph * 1.2);
            pp.add(Shape::closed_line(
                ellipse_pts(pad_c, r, 0.0, 48),
                Stroke::new(1.0, with_alpha(Color32::WHITE, 0.45 * (1.0 - ph))),
            ));
        }
        clay(&pp, pad_c, vec2(66.0, 15.0) * s, 0.0, hex("#55A83A"));
        pp.add(Shape::convex_polygon(
            vec![pad_c + vec2(10.0, 15.0) * s, pad_c + vec2(26.0, 16.0) * s, pad_c + vec2(4.0, 2.0) * s],
            hex("#9BD7C3"),
            Stroke::NONE,
        ));
        // where to look: the nearest cookie in flight
        let cookie_pos = |c: &Cookie| -> Option<(Pos2, f32)> {
            if c.age < 0.0 {
                return None;
            }
            let catch = pos2(frog_base.x + 48.0 * s, panel.min.y + 52.0);
            if c.age < FLY {
                let k = c.age / FLY;
                let e = 1.0 - (1.0 - k).powi(3);
                let mut p = c.from.lerp(catch, e);
                p.y -= (k * PI).sin() * 18.0;
                Some((p, 0.0))
            } else if c.age < FLY + SNAP {
                Some((catch, (c.age - FLY) / SNAP))
            } else {
                let k = (c.age - FLY - SNAP) / BACK;
                Some((catch.lerp(mouth, sstep(0.0, 1.0, k)), 1.0))
            }
        };
        let look = st
            .cookies
            .iter()
            .filter_map(|c| cookie_pos(c).map(|(p, _)| p))
            .next()
            .map(|p| ((p - (frog_base + vec2(0.0, -58.0 * s))) / 60.0).clamp(vec2(-1.0, -1.0), vec2(1.0, 1.0)))
            .unwrap_or(vec2((time * 0.4).sin() * 0.3, 0.1));
        let happy = (prefs.iter().filter(|p| **p).count() as f32 / n as f32 * 0.6 + st.crown.value * 0.4).min(1.0);
        paint_frog(&pp, frog_base, s, look, happy, st.sad, st.gulp, time, st.crown.value);
        // tongues
        for c in &st.cookies {
            if let Some((p, tongue)) = cookie_pos(c) {
                if c.age >= FLY {
                    let tip = mouth.lerp(p, if c.age < FLY + SNAP { sstep(0.0, 1.0, tongue) } else { 1.0 });
                    pp.line_segment([mouth, tip], Stroke::new(5.0 * s, hex("#E8697A")));
                    pp.circle_filled(tip, 4.2 * s, hex("#E8697A"));
                }
            }
        }
        painter.rect_stroke(panel, 12, Stroke::new(6.0, card_fill), egui::StrokeKind::Outside);
        // copy + rows
        painter.text(
            pos2(x0, panel.max.y + 16.0),
            Align2::LEFT_TOP,
            "Cookie preferences",
            FontId::proportional(17.0),
            t.foreground,
        );
        painter.text(
            pos2(x0, panel.max.y + 39.0),
            Align2::LEFT_TOP,
            format!("Every cookie you allow goes to {}.", self.name),
            FontId::proportional(12.0),
            t.muted_foreground,
        );
        painter
            .line_segment([pos2(x0, rows_top - 4.0), pos2(x0 + inner_w, rows_top - 4.0)], Stroke::new(1.0, t.border));
        for i in 0..n {
            let y = rows_top + i as f32 * 48.0;
            painter.text(
                pos2(x0, y + 8.0),
                Align2::LEFT_TOP,
                &self.categories[i].0,
                FontId::proportional(13.0),
                t.foreground,
            );
            painter.text(
                pos2(x0, y + 26.0),
                Align2::LEFT_TOP,
                &self.categories[i].1,
                FontId::proportional(11.0),
                t.muted_foreground,
            );
            paint_toggle(&painter, toggle_rect(i), st.knobs[i].value, &t);
        }
        painter.rect_filled(allow, 20.0, t.foreground);
        painter.text(allow.center(), Align2::CENTER_CENTER, "Allow all", FontId::proportional(13.0), t.background);
        painter.rect_stroke(reject, 20, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        painter.text(reject.center(), Align2::CENTER_CENTER, "Reject all", FontId::proportional(13.0), t.foreground);
        // cookies in flight (drawn over everything)
        for c in &st.cookies {
            if let Some((p, _)) = cookie_pos(c) {
                let k = (c.age / FLY).clamp(0.0, 1.0);
                let r = 4.0 + 6.0 * sstep(0.0, 0.25, k);
                let shrink = if c.age > FLY + SNAP { 1.0 - sstep(0.6, 1.0, (c.age - FLY - SNAP) / BACK) } else { 1.0 };
                paint_cookie(&painter, p, r * s * shrink, c.age * 6.0);
            }
        }
        response
    }
}
