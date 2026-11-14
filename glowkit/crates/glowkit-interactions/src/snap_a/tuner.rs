use super::common::{idle_time, sstep, wall_dt};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::{hex, paint, Theme};

#[derive(Clone, Copy, Debug, Default)]
struct TunerState {
    pos: f32,
    vel: f32,
    shown: f32,
    auto_t: f32,
    target: Option<f32>,
    lock: f32,
    init: bool,
}

/// A ruler picker styled as an FM dial. Fling the tick tape and it coasts, then settles onto a
/// channel; the digits roll inside the vermilion needle's pill like an odometer, and preset
/// stations pull the dial in magnetically and show their name.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut freq = 94.7_f32;
/// use glowkit_interactions::TunerDial;
/// ui.add(TunerDial::new(&mut freq).station(91.2, "Harbour Jazz").station(99.9, "Night Owl"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TunerDial<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    step: f32,
    spacing: f32,
    friction: f32,
    stations: Vec<(f32, String)>,
    needle: Option<Color32>,
    height: f32,
    autoplay: bool,
}

impl<'a> TunerDial<'a> {
    /// An FM dial (87.5 – 108.0 MHz in 0.1 steps) editing `value`.
    pub fn new(value: &'a mut f32) -> Self {
        Self {
            value,
            min: 87.5,
            max: 108.0,
            step: 0.1,
            spacing: 8.0,
            friction: 1.0,
            stations: Vec::new(),
            needle: None,
            height: 230.0,
            autoplay: true,
        }
    }
    /// Value range.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min.min(max);
        self.max = max.max(min);
        self
    }
    /// Distance in points between two small ticks (default 8).
    pub fn spacing(mut self, px: f32) -> Self {
        self.spacing = px.clamp(4.0, 24.0);
        self
    }
    /// How quickly a fling slows down (default 1; lower coasts further).
    pub fn friction(mut self, k: f32) -> Self {
        self.friction = k.clamp(0.2, 4.0);
        self
    }
    /// Add a preset station that the dial locks onto.
    pub fn station(mut self, freq: f32, name: impl Into<String>) -> Self {
        self.stations.push((freq, name.into()));
        self
    }
    /// Needle color (default vermilion).
    pub fn needle(mut self, c: Color32) -> Self {
        self.needle = Some(c);
        self
    }
    /// Widget height (default 230).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(140.0);
        self
    }
    /// Fling to random stations while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for TunerDial<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let wdt = wall_dt(ui);
        let idle = idle_time(ui, id, rect);
        let stations = if self.stations.is_empty() {
            vec![
                (89.1, "Harbour Jazz".to_string()),
                (93.4, "Paper Lantern".into()),
                (97.9, "Night Owl".into()),
                (101.5, "Static Garden".into()),
                (104.8, "Coastline FM".into()),
            ]
        } else {
            self.stations.clone()
        };
        let units = |v: f32| (v - self.min) / self.step;
        let (lo, hi) = (0.0, units(self.max));
        let px = self.spacing;
        let drag = if response.dragged() { response.drag_delta().x } else { 0.0 };
        let dragging = response.dragged();
        let released = response.drag_stopped();
        let wheel =
            if response.hovered() { ui.input(|i| i.smooth_scroll_delta.x + i.smooth_scroll_delta.y) } else { 0.0 };
        if response.hovered() {
            ui.ctx().set_cursor_icon(if dragging { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let ext = units(*self.value);
        let friction = self.friction;
        let autoplay = self.autoplay && idle > 2.0;
        let station_units: Vec<f32> = stations.iter().map(|(f, _)| units(*f)).collect();
        let seed = (glowkit_core::time(ui) * 7.3) as u64;
        let st = glowkit_core::with_state(ui, id, TunerState::default, |s| {
            if !s.init {
                s.init = true;
                s.pos = ext;
                s.shown = ext;
                s.auto_t = 1.0;
            }
            if (ext - s.pos).abs() > 0.5 && !dragging && s.vel.abs() < 0.01 && s.target.is_none() {
                s.pos = ext; // changed from outside
            }
            if dragging {
                s.pos -= drag / px;
                s.vel = -drag / px / dt.max(1e-3);
                s.target = None;
                s.auto_t = 3.0;
            } else {
                s.pos -= wheel / px;
                if released {
                    s.vel = s.vel.clamp(-160.0, 160.0);
                }
                if autoplay && s.target.is_none() && s.vel.abs() < 0.5 {
                    s.auto_t -= wdt;
                    if s.auto_t <= 0.0 {
                        // fling: either to a station or somewhere in between
                        let pick = glowkit_core::rng::hash01(seed) * (station_units.len() as f32 + 1.5);
                        let goal = if (pick as usize) < station_units.len() {
                            station_units[pick as usize]
                        } else {
                            (glowkit_core::rng::hash01(seed + 9) * hi).round()
                        };
                        s.target = Some(goal);
                        s.auto_t = 3.2;
                    }
                }
                if let Some(g) = s.target {
                    let acc = -14.0 * (s.pos - g) - 7.0 * s.vel;
                    s.vel += acc * dt;
                    s.pos += s.vel * dt;
                    if (s.pos - g).abs() < 0.02 && s.vel.abs() < 0.05 {
                        s.pos = g;
                        s.vel = 0.0;
                        s.target = None;
                    }
                } else {
                    // coast, then snap to the nearest channel (stations pull harder)
                    s.vel *= (-2.6 * friction * dt).exp();
                    s.pos += s.vel * dt;
                    if s.vel.abs() < 18.0 {
                        let mut goal = s.pos.round();
                        let mut pull = 60.0;
                        for su in &station_units {
                            if (s.pos - su).abs() < 3.5 {
                                goal = *su;
                                pull = 90.0;
                            }
                        }
                        let acc = -pull * (s.pos - goal) - 2.0 * pull.sqrt() * s.vel;
                        s.vel += acc * dt;
                    }
                }
            }
            if s.pos < lo {
                s.pos += (lo - s.pos) * 0.3;
                s.vel *= 0.5;
            }
            if s.pos > hi {
                s.pos -= (s.pos - hi) * 0.3;
                s.vel *= 0.5;
            }
            s.shown += (s.pos - s.shown) * (1.0 - (-20.0 * dt).exp());
            let on_station = station_units.iter().any(|su| (s.pos - su).abs() < 0.05) && s.vel.abs() < 1.0;
            s.lock += ((if on_station { 1.0 } else { 0.0 }) - s.lock) * (1.0 - (-8.0 * dt).exp());
            *s
        });
        let new_value = (self.min + st.pos.round().clamp(lo, hi) * self.step).clamp(self.min, self.max);
        if (new_value - *self.value).abs() > 1e-4 {
            *self.value = new_value;
            response.mark_changed();
        }
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        let needle = self.needle.unwrap_or(hex("#E8492F"));
        let cx = rect.center().x;
        let base_y = rect.min.y + rect.height() * 0.52;
        // soft spotlight behind the dial
        paint::glow_ellipse(
            &painter,
            pos2(cx, base_y - 10.0),
            vec2(rect.width() * 0.32, 90.0),
            with_alpha(if t.dark { Color32::WHITE } else { hex("#FFF8EC") }, if t.dark { 0.03 } else { 0.6 }),
        );
        // ticks
        let half = rect.width() * 0.5;
        let first = (st.shown - half / px).floor() as i32 - 1;
        let last = (st.shown + half / px).ceil() as i32 + 1;
        let ink = t.foreground;
        for i in first..=last {
            if (i as f32) < lo - 0.01 || (i as f32) > hi + 0.01 {
                continue;
            }
            let x = cx + (i as f32 - st.shown) * px;
            let d = ((x - cx) / half).abs();
            let bell = (1.0 - d * d).max(0.0);
            let major = (i as f32 * self.step + self.min).fract().abs() < 1e-3
                || (i as f32 * self.step + self.min).fract() > 0.999;
            let mid = !major && i % 5 == 0;
            let h = (if major {
                34.0
            } else if mid {
                24.0
            } else {
                16.0
            }) * (0.45 + 0.55 * bell);
            let a = (if major {
                0.9
            } else if mid {
                0.55
            } else {
                0.32
            }) * sstep(1.0, 0.55, d);
            let w = if major { 1.6 } else { 1.0 };
            painter.line_segment([pos2(x, base_y - h), pos2(x, base_y)], Stroke::new(w, with_alpha(ink, a)));
            if major {
                let f = (self.min + i as f32 * self.step).round();
                painter.text(
                    pos2(x, base_y + 14.0),
                    Align2::CENTER_CENTER,
                    format!("{f:.0}"),
                    FontId::monospace(9.0),
                    with_alpha(t.muted_foreground, a),
                );
            }
        }
        // station markers
        for su in &station_units {
            let x = cx + (su - st.shown) * px;
            if (x - cx).abs() < half {
                let d = ((x - cx) / half).abs();
                painter.circle_filled(pos2(x, base_y - 44.0), 2.2, with_alpha(needle, 0.7 * sstep(1.0, 0.5, d)));
            }
        }
        // needle
        let pill_c = pos2(cx, base_y - 74.0);
        let pill = Rect::from_center_size(pill_c, vec2(64.0, 30.0));
        let wob = (st.vel * 0.002).clamp(-0.12, 0.12);
        let needle_bottom = pos2(cx - wob * 60.0, base_y + 22.0);
        paint::soft_shadow(&painter, pill, 15.0, 10.0, egui::Vec2::new(0.0, 4.0), with_alpha(needle, 0.35));
        painter.line_segment([pill.center_bottom(), needle_bottom], Stroke::new(2.6, needle));
        painter.circle_filled(needle_bottom, 2.6, needle);
        painter.rect_filled(pill, 15.0, needle);
        painter.rect_filled(
            Rect::from_min_size(pill.min + vec2(6.0, 3.0), vec2(pill.width() - 12.0, 10.0)),
            5.0,
            with_alpha(Color32::WHITE, 0.12),
        );
        // odometer digits: value*10 as an integer, with a decimal point before the last digit
        let v10 = self.min * 10.0 + st.shown * self.step * 10.0;
        let font = FontId::monospace(16.0);
        let dw = painter.layout_no_wrap("0".into(), font.clone(), Color32::WHITE).size();
        let clip = painter.with_clip_rect(pill.shrink2(vec2(6.0, 4.0)));
        let digits = if v10 >= 999.5 { 4 } else { 3 };
        let total_w = dw.x * (digits as f32 + 0.5);
        let mut x = pill_c.x + total_w * 0.5;
        for d in 0..digits {
            if d == 1 {
                x -= dw.x * 0.5;
                clip.text(pos2(x + dw.x * 0.25, pill_c.y), Align2::CENTER_CENTER, ".", font.clone(), Color32::WHITE);
            }
            x -= dw.x;
            let place = 10f32.powi(d);
            let whole = (v10 / place).floor();
            // a digit only turns while everything below it rolls over from 9 to 0
            let roll = if d == 0 { v10 - v10.floor() } else { (v10.rem_euclid(place) - (place - 1.0)).clamp(0.0, 1.0) };
            let roll = sstep(0.0, 1.0, roll);
            let a = (whole as i64).rem_euclid(10);
            let b = (a + 1) % 10;
            let cxd = x + dw.x * 0.5;
            clip.text(
                pos2(cxd, pill_c.y - roll * dw.y),
                Align2::CENTER_CENTER,
                a.to_string(),
                font.clone(),
                Color32::WHITE,
            );
            clip.text(
                pos2(cxd, pill_c.y + (1.0 - roll) * dw.y),
                Align2::CENTER_CENTER,
                b.to_string(),
                font.clone(),
                Color32::WHITE,
            );
        }
        // readout under the dial
        let info_y = base_y + 46.0;
        let freq = self.min + st.pos.round().clamp(lo, hi) * self.step;
        let chip = Rect::from_center_size(pos2(cx - 34.0, info_y), vec2(22.0, 13.0));
        painter.rect_filled(chip, 3.0, with_alpha(ink, 0.8));
        painter.text(chip.center(), Align2::CENTER_CENTER, "FM", FontId::monospace(8.0), t.background);
        painter.text(
            pos2(chip.max.x + 6.0, info_y),
            Align2::LEFT_CENTER,
            format!("{freq:.1} MHz"),
            FontId::monospace(10.0),
            t.muted_foreground,
        );
        for b in 0..4 {
            let lit = (b as f32) < 1.0 + st.lock * 3.0;
            let r = Rect::from_min_size(
                pos2(cx + 52.0 + b as f32 * 4.0, info_y + 4.0 - (b + 1) as f32 * 2.5),
                vec2(2.5, (b + 1) as f32 * 2.5),
            );
            painter.rect_filled(r, 0.5, with_alpha(ink, if lit { 0.8 } else { 0.2 }));
        }
        let name = stations.iter().find(|(f, _)| (f - freq).abs() < 0.05).map(|(_, n)| n.as_str());
        let lock = st.lock;
        let (title, sub, col) = match name {
            Some(n) if lock > 0.3 => (n, "Locked in · stereo", with_alpha(ink, lock)),
            _ => ("Between stations", "Turn to a marked channel", with_alpha(t.muted_foreground, 0.9)),
        };
        painter.text(pos2(cx, info_y + 24.0), Align2::CENTER_CENTER, title, FontId::proportional(17.0), col);
        painter.text(
            pos2(cx, info_y + 43.0),
            Align2::CENTER_CENTER,
            sub,
            FontId::proportional(10.5),
            with_alpha(t.muted_foreground, 0.7),
        );
        response
    }
}
