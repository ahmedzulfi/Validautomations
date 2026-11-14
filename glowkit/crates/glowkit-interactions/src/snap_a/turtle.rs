use super::common::{clay, ellipse_pts, idle_time, lerp_c, sstep, wall_dt};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget,
};
use glowkit_core::anim::Spring;
use glowkit_core::color::{shade, with_alpha};
use glowkit_core::{hex, paint, Theme};
use std::f32::consts::PI;

#[derive(Clone, Debug)]
struct TurtleState {
    parts: [Spring; 4],
    met: [f32; 4],
    show: bool,
    auto_owned: bool,
    auto_t: f32,
    auto_i: usize,
    blink: f32,
}

const SCRIPT: &str = "sHr@1gpass";

/// A password field watched by a clay tortoise. Every rule you meet builds its shell up: the
/// dome rises, then knobs pop out of it, spikes line the rim and finally gold studs glint, and
/// once the password is very strong the tortoise smiles. Includes a strength meter, a rule
/// checklist and a show/hide toggle.
///
/// The password is caller-owned. The response reports `changed()` when it changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut password = String::new();
/// ui.add(glowkit_interactions::TurtlePassword::new(&mut password).name("Shelby"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TurtlePassword<'a> {
    text: &'a mut String,
    name: String,
    width: f32,
    min_len: usize,
    bounce: f32,
    autoplay: bool,
}

impl<'a> TurtlePassword<'a> {
    /// A password field editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self { text, name: "Shelby".into(), width: 340.0, min_len: 8, bounce: 1.0, autoplay: true }
    }
    /// The tortoise's name (used in the hint line).
    pub fn name(mut self, s: impl Into<String>) -> Self {
        self.name = s.into();
        self
    }
    /// Card width (default 340).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.clamp(260.0, 520.0);
        self
    }
    /// Minimum length rule (default 8).
    pub fn min_len(mut self, n: usize) -> Self {
        self.min_len = n.max(1);
        self
    }
    /// How springy the shell parts are when they appear (default 1).
    pub fn bounce(mut self, k: f32) -> Self {
        self.bounce = k.clamp(0.2, 2.0);
        self
    }
    /// Type a sample password while idle and the field is empty (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn rules(s: &str, min_len: usize) -> [bool; 4] {
    [
        s.chars().any(|c| c.is_ascii_digit()),
        s.chars().any(|c| c.is_uppercase()),
        s.chars().count() >= min_len,
        s.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace()),
    ]
}

fn paint_scene(p: &egui::Painter, r: Rect, time: f32) {
    paint::shader_grid(p, r, 1, 16, |uv| {
        lerp_c(lerp_c(hex("#E8F0C8"), hex("#A9CF7A"), uv.y * 1.3), hex("#4E8F3A"), sstep(0.55, 1.0, uv.y))
    });
    // bokeh
    for i in 0..9 {
        let h = glowkit_core::rng::hash01(i as u64 * 13 + 1);
        let c = pos2(r.min.x + h * r.width(), r.min.y + glowkit_core::rng::hash01(i as u64 * 7 + 5) * r.height() * 0.5);
        let rr = 10.0 + glowkit_core::rng::hash01(i as u64 * 3 + 2) * 18.0;
        let a = 0.25 + 0.15 * (time * 0.7 + i as f32).sin();
        p.circle_filled(c, rr, with_alpha(Color32::WHITE, a * 0.35));
    }
}

fn grass_blades(p: &egui::Painter, r: Rect, time: f32) {
    for i in 0..70 {
        let h = glowkit_core::rng::hash01(i as u64 * 31 + 3);
        let x = r.min.x + glowkit_core::rng::hash01(i as u64 * 17 + 11) * r.width();
        let len = 8.0 + h * 16.0;
        let sway = (time * 1.4 + x * 0.05).sin() * 2.0;
        let base = pos2(x, r.max.y + 2.0);
        let tip = base + vec2(sway + (h - 0.5) * 6.0, -len);
        let c = lerp_c(hex("#3F7E2C"), hex("#8CC157"), h);
        p.add(Shape::convex_polygon(vec![base - vec2(1.6, 0.0), tip, base + vec2(1.6, 0.0)], c, Stroke::NONE));
    }
}

fn paint_tortoise(p: &egui::Painter, base: Pos2, s: f32, parts: [f32; 4], time: f32, blink: f32, happy: f32) {
    let skin = hex("#86B25A");
    let skin_dark = hex("#5F8A3D");
    let shell = hex("#C98F55");
    let rx = 64.0 * s;
    let dome = (24.0 + 30.0 * parts[0].clamp(0.0, 1.3)) * s;
    let breathe = (time * 2.0).sin() * 0.8 * s;
    // ground shadow
    super::common::contact_shadow(p, base + vec2(6.0 * s, 4.0 * s), vec2(rx * 1.35, 12.0 * s), 0.3);
    // back legs + tail
    clay(p, base + vec2(-rx * 0.62, -2.0 * s), vec2(13.0, 10.0) * s, 0.2, skin_dark);
    clay(p, base + vec2(-rx * 1.05, -6.0 * s), vec2(8.0, 4.0) * s, -0.3, skin_dark);
    clay(p, base + vec2(rx * 0.45, -2.0 * s), vec2(13.0, 10.0) * s, -0.2, skin_dark);
    // neck + head (bobs gently)
    let bob = (time * 1.3).sin() * 2.0 * s;
    let head = base + vec2(rx * 1.18, -22.0 * s + bob);
    clay(p, base + vec2(rx * 0.95, -12.0 * s + bob * 0.5), vec2(17.0, 10.0) * s, -0.5, skin);
    clay(p, head, vec2(21.0, 16.0) * s, 0.1, skin);
    // eye
    let eye = head + vec2(5.0 * s, -5.0 * s);
    p.circle_filled(eye, 4.6 * s, hex("#FBFBF2"));
    let open = 1.0 - blink;
    p.add(Shape::convex_polygon(
        ellipse_pts(eye + vec2(1.0 * s, 0.4 * s), vec2(2.6 * s, 2.8 * s * open.max(0.08)), 0.0, 16),
        hex("#1C1A16"),
        Stroke::NONE,
    ));
    p.circle_filled(eye + vec2(0.2, -1.4) * s, 0.9 * s * open, Color32::WHITE);
    // mouth
    let m0 = head + vec2(8.0 * s, 6.0 * s);
    let pts: Vec<Pos2> = (0..=8)
        .map(|i| {
            let t = i as f32 / 8.0;
            m0 + vec2(t * 10.0 * s, (t - 0.5).powi(2) * -10.0 * s * happy + 1.5 * s * (1.0 - happy) * (t - 0.5))
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.3 * s, hex("#3C5626"))));
    p.circle_filled(head + vec2(-4.0 * s, 5.0 * s), 3.2 * s, with_alpha(hex("#F28B7C"), 0.35 * happy));
    // front legs
    clay(p, base + vec2(-rx * 0.35, 4.0 * s), vec2(13.0, 11.0) * s, 0.1, skin);
    clay(p, base + vec2(rx * 0.72, 4.0 * s), vec2(13.0, 11.0) * s, -0.1, skin);
    // rim spikes
    let spikes = parts[2];
    if spikes > 0.01 {
        for i in 0..11 {
            let u = -1.0 + i as f32 / 10.0 * 2.0;
            let a = PI + (u * 0.5 + 0.5) * PI;
            let at = base + vec2(a.cos() * rx * 1.02, a.sin() * dome * 0.35 - 6.0 * s - breathe);
            let out = vec2(a.cos(), a.sin() * 0.6 - 0.25).normalized();
            let len = 11.0 * s * spikes;
            let side = out.rot90() * 4.0 * s;
            p.add(Shape::convex_polygon(vec![at - side, at + out * len, at + side], shade(shell, -0.12), Stroke::NONE));
        }
    }
    // dome
    let n = 40;
    let mut outline: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = PI + i as f32 / n as f32 * PI;
            base + vec2(a.cos() * rx, a.sin() * dome - 6.0 * s - breathe)
        })
        .collect();
    outline.push(base + vec2(rx * 0.9, -2.0 * s));
    outline.push(base + vec2(-rx * 0.9, -2.0 * s));
    let light = base + vec2(-rx * 0.35, -dome * 0.95);
    let stops = [shade(shell, 0.16), shell, shade(shell, -0.2)];
    paint::fill_convex_fn(p, &outline, 6, paint::radial_fn(light, rx * 1.6, &stops));
    // rim band
    p.add(Shape::convex_polygon(
        ellipse_pts(base + vec2(0.0, -5.0 * s), vec2(rx * 1.02, 7.0 * s), 0.0, 40),
        shade(shell, -0.12),
        Stroke::NONE,
    ));
    // scute pattern (projected onto the dome)
    let on_dome = |u: f32, h: f32| {
        // u -1..1 across, h 0..1 up the dome
        let a = PI + (u * 0.5 + 0.5) * PI;
        let squeeze = (1.0 - h * h).sqrt();
        base + vec2(a.cos() * rx * squeeze, -dome * h * 0.98 - 6.0 * s - breathe + a.sin() * 0.0)
    };
    let line_c = with_alpha(shade(shell, -0.32), 0.55);
    for u in [-0.55f32, 0.0, 0.55] {
        let pts: Vec<Pos2> =
            (0..=10).map(|k| on_dome(u + (k as f32 / 10.0) * 0.0, 0.12 + k as f32 / 10.0 * 0.62)).collect();
        p.add(Shape::line(pts, Stroke::new(1.2 * s, line_c)));
    }
    let ring: Vec<Pos2> = (0..=20).map(|k| on_dome(-0.95 + k as f32 / 20.0 * 1.9, 0.42)).collect();
    p.add(Shape::line(ring, Stroke::new(1.2 * s, line_c)));
    // knobs
    let knobs = parts[1];
    if knobs > 0.01 {
        for (i, u) in [-0.72f32, -0.28, 0.28, 0.72, 0.0].iter().enumerate() {
            let h = if i == 4 { 0.86 } else { 0.62 };
            let c = on_dome(*u, h);
            clay(p, c, vec2(6.5, 5.0) * s * knobs, 0.0, shade(shell, 0.04));
        }
    }
    // gold studs
    let studs = parts[3];
    if studs > 0.01 {
        let gold = hex("#E9B949");
        for (k, (u, h)) in
            [(-0.55f32, 0.25f32), (0.0, 0.25), (0.55, 0.25), (-0.3, 0.42), (0.3, 0.42)].iter().enumerate()
        {
            let c = on_dome(*u, *h);
            clay(p, c, Vec2::splat(3.6 * s * studs), 0.0, gold);
            let tw = ((time * 3.0 + k as f32 * 1.3).sin() * 0.5 + 0.5).powi(6) * studs;
            if tw > 0.05 {
                paint::sparkle(p, c + vec2(-1.2, -1.2) * s, 6.0 * s * tw, time, with_alpha(Color32::WHITE, tw));
            }
        }
    }
    // specular sheen on the dome
    p.add(Shape::line(
        (0..=12).map(|k| on_dome(-0.7 + k as f32 / 12.0 * 0.5, 0.78)).collect(),
        Stroke::new(2.4 * s, with_alpha(Color32::WHITE, 0.22)),
    ));
}

impl Widget for TurtlePassword<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.min(ui.available_width());
        let h = 520.0;
        let (outer, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
        let id = response.id;
        let card = Rect::from_center_size(outer.center(), vec2(w, h - 8.0));
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let time = glowkit_core::time(ui);
        let idle = idle_time(ui, id, outer);
        let panel = Rect::from_min_size(card.min + vec2(8.0, 8.0), vec2(w - 16.0, 168.0));
        let x0 = card.min.x + 20.0;
        let inner_w = w - 40.0;
        let field = Rect::from_min_size(pos2(x0, panel.max.y + 70.0), vec2(inner_w, 38.0));
        let eye = Rect::from_center_size(pos2(field.max.x - 18.0, field.center().y), vec2(26.0, 26.0));
        let st0: TurtleState = glowkit_core::state(ui, id, || TurtleState {
            parts: [Spring::new(0.0); 4],
            met: [0.0; 4],
            show: false,
            auto_owned: false,
            auto_t: 0.0,
            auto_i: 0,
            blink: 0.0,
        });
        // card
        paint::soft_shadow(
            ui.painter(),
            card,
            16.0,
            24.0,
            vec2(0.0, 10.0),
            Color32::from_black_alpha(if t.dark { 110 } else { 28 }),
        );
        ui.painter().rect_filled(card, 16.0, if t.dark { shade(t.card, 0.03) } else { Color32::WHITE });
        ui.painter().rect_stroke(card, 16, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        // text field
        let edit_id = id.with("pw");
        let focused = ui.memory(|m| m.has_focus(edit_id));
        let before = self.text.clone();
        // autoplay typing
        let mut st_auto = (st0.auto_owned, st0.auto_t, st0.auto_i);
        if focused {
            st_auto.0 = false;
        } else if self.autoplay && idle > 2.5 && (self.text.is_empty() || st_auto.0) {
            st_auto.0 = true;
            st_auto.1 += wdt;
            let typed = st_auto.2;
            let n = SCRIPT.chars().count();
            if typed < n && st_auto.1 > 0.26 {
                st_auto.1 = 0.0;
                st_auto.2 += 1;
                *self.text = SCRIPT.chars().take(st_auto.2).collect();
            } else if typed >= n && st_auto.1 > 3.2 {
                st_auto = (true, 0.0, 0);
                self.text.clear();
            }
        }
        ui.painter().rect_filled(field, 10.0, if t.dark { t.muted } else { hex("#FBFBFA") });
        ui.painter().rect_stroke(
            field,
            10,
            Stroke::new(if focused { 1.6 } else { 1.0 }, if focused { t.foreground } else { t.border }),
            egui::StrokeKind::Inside,
        );
        let edit_rect = Rect::from_min_max(field.min + vec2(12.0, 9.0), pos2(eye.min.x - 4.0, field.max.y - 7.0));
        let te = egui::TextEdit::singleline(self.text)
            .id(edit_id)
            .password(!st0.show)
            .frame(egui::Frame::NONE)
            .hint_text("Enter a password")
            .font(FontId::proportional(14.0))
            .desired_width(edit_rect.width());
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(edit_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let r = child.add(te);
        let eye_r = ui.interact(eye, id.with("eye"), Sense::click());
        if eye_r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let toggle_show = eye_r.clicked();
        if r.changed() || *self.text != before {
            response.mark_changed();
        }
        let met = rules(self.text, self.min_len);
        let count = met.iter().filter(|m| **m).count();
        let bounce = self.bounce;
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0.clone(),
            |s| {
                (s.auto_owned, s.auto_t, s.auto_i) = st_auto;
                if toggle_show {
                    s.show = !s.show;
                }
                for k in 0..4 {
                    s.parts[k].target = if count > k { 1.0 } else { 0.0 };
                    s.parts[k].stiffness = 220.0;
                    s.parts[k].damping = 9.0 / bounce;
                    s.parts[k].step(dt);
                    s.met[k] += ((if met[k] { 1.0 } else { 0.0 }) - s.met[k]) * (1.0 - (-12.0 * dt).exp());
                }
                s.blink = {
                    let phase = (time % 3.7) / 3.7;
                    if phase > 0.95 {
                        ((phase - 0.95) / 0.05 * PI).sin()
                    } else {
                        0.0
                    }
                };
                s.clone()
            },
        );
        glowkit_core::animate(ui);
        let painter = ui.painter_at(outer);
        // eye icon
        let ec = eye.center();
        let open = st.show;
        let lid: Vec<Pos2> = (0..=16)
            .map(|i| {
                let a = i as f32 / 16.0 * PI;
                ec + vec2(-a.cos() * 8.0, -a.sin() * 4.5)
            })
            .collect();
        let lid2: Vec<Pos2> = (0..=16)
            .map(|i| {
                let a = i as f32 / 16.0 * PI;
                ec + vec2(-a.cos() * 8.0, a.sin() * 4.5)
            })
            .collect();
        painter.add(Shape::line(lid, Stroke::new(1.3, t.muted_foreground)));
        painter.add(Shape::line(lid2, Stroke::new(1.3, t.muted_foreground)));
        painter.circle_stroke(ec, 2.4, Stroke::new(1.3, t.muted_foreground));
        if !open {
            painter.line_segment([ec + vec2(-7.0, 6.0), ec + vec2(7.0, -6.0)], Stroke::new(1.3, t.muted_foreground));
        }
        // scene
        let scene_p = painter.with_clip_rect(panel);
        paint_scene(&scene_p, panel, time);
        let parts = std::array::from_fn(|k| st.parts[k].value.max(0.0));
        let base = pos2(panel.center().x - 18.0, panel.max.y - 34.0);
        paint_tortoise(
            &scene_p,
            base,
            (panel.width() / 330.0).clamp(0.75, 1.2),
            parts,
            time,
            st.blink,
            sstep(3.0, 4.0, count as f32 + st.parts[3].value - 1.0),
        );
        grass_blades(&scene_p, panel, time);
        painter.rect_stroke(
            panel,
            12,
            Stroke::new(6.0, if t.dark { shade(t.card, 0.03) } else { Color32::WHITE }),
            egui::StrokeKind::Outside,
        );
        // texts
        painter.text(
            pos2(x0, panel.max.y + 18.0),
            Align2::LEFT_TOP,
            "Create a password",
            FontId::proportional(19.0),
            t.foreground,
        );
        painter.text(
            pos2(x0, panel.max.y + 50.0),
            Align2::LEFT_TOP,
            "Password",
            FontId::proportional(12.0),
            t.foreground,
        );
        // strength
        let labels = ["Too short", "Weak", "Fair", "Strong", "Very strong"];
        let colors = [t.muted_foreground, hex("#E5484D"), hex("#F59E0B"), hex("#EAB308"), hex("#22A06B")];
        let seg_w = (inner_w - 18.0) / 4.0;
        let sy = field.max.y + 12.0;
        for k in 0..4 {
            let r = Rect::from_min_size(pos2(x0 + k as f32 * (seg_w + 6.0), sy), vec2(seg_w, 3.5));
            painter.rect_filled(r, 2.0, with_alpha(t.muted_foreground, 0.18));
            let f = (st.parts[k].value).clamp(0.0, 1.0);
            if f > 0.0 {
                painter.rect_filled(
                    Rect::from_min_size(r.min, vec2(r.width() * f, r.height())),
                    2.0,
                    colors[count.max(1)],
                );
            }
        }
        if self.text.is_empty() {
            painter.text(
                pos2(x0, sy + 12.0),
                Align2::LEFT_TOP,
                format!("{}'s shell grows stronger with your password.", self.name),
                FontId::proportional(11.0),
                t.muted_foreground,
            );
        } else {
            let r = painter.text(
                pos2(x0, sy + 12.0),
                Align2::LEFT_TOP,
                "Strength",
                FontId::proportional(11.0),
                t.muted_foreground,
            );
            painter.text(
                pos2(r.max.x + 6.0, sy + 12.0),
                Align2::LEFT_TOP,
                labels[count],
                FontId::proportional(11.0),
                colors[count],
            );
        }
        // rules
        let names = [
            "A number".to_string(),
            "An uppercase letter".to_string(),
            format!("{} or more characters", self.min_len),
            "A symbol".to_string(),
        ];
        for k in 0..4 {
            let y = sy + 40.0 + k as f32 * 24.0;
            let m = st.met[k];
            let row = Rect::from_min_size(pos2(x0 - 6.0, y - 3.0), vec2(inner_w + 12.0, 22.0));
            painter.rect_filled(
                row,
                6.0,
                with_alpha(
                    hex("#22A06B"),
                    0.1 * m * (1.0 - sstep(0.0, 1.0, st.parts[k].velocity.abs().min(1.0)) * 0.0),
                ),
            );
            let c = pos2(x0 + 7.0, y + 8.0);
            painter.circle_stroke(c, 7.0, Stroke::new(1.0, with_alpha(t.muted_foreground, 0.5 * (1.0 - m))));
            if m > 0.01 {
                painter.circle_filled(c, 7.0 * (0.6 + 0.4 * m), with_alpha(hex("#22A06B"), m));
                let ck = [c + vec2(-3.0, 0.2), c + vec2(-0.8, 2.6), c + vec2(3.4, -2.4)];
                painter.add(Shape::line(ck.to_vec(), Stroke::new(1.6, with_alpha(Color32::WHITE, m))));
            }
            painter.text(
                pos2(x0 + 22.0, y + 8.0),
                Align2::LEFT_CENTER,
                &names[k],
                FontId::proportional(12.5),
                lerp_c(t.muted_foreground, t.foreground, m),
            );
        }
        // continue
        let all = count == 4;
        let btn = Rect::from_min_size(pos2(x0, card.max.y - 56.0), vec2(inner_w, 40.0));
        let bk = st.parts[3].value.clamp(0.0, 1.0);
        painter.rect_filled(
            btn,
            CornerRadius::same(10),
            lerp_c(with_alpha(t.muted_foreground, 0.18), t.foreground, bk),
        );
        painter.text(
            btn.center(),
            Align2::CENTER_CENTER,
            "Continue",
            FontId::proportional(14.0),
            if all { t.background } else { t.muted_foreground },
        );
        response
    }
}
