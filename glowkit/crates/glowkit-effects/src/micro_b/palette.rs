use super::common::{cr, hairline, idle_for, shadow_color, surface};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::{ease, spring, stagger, Easing};
use glowkit_core::color::{hsv, mix, with_alpha};
use glowkit_core::rng::Rng;
use glowkit_core::{paint, Theme};

#[derive(Clone, Debug)]
struct PaletteState {
    from: Vec<Color32>,
    to: Vec<Color32>,
    since: f64,
    seed: u64,
    turns: f32,
}

/// Generate a pleasant `n`-colour palette from `seed`: a random base hue with an analogous,
/// complementary, triadic or monochrome scheme, spread over light-to-dark.
pub(crate) fn harmonious(seed: u64, n: usize) -> Vec<Color32> {
    let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x51);
    let base = r.f32();
    let scheme = r.below(4);
    // hue offsets per scheme, cycled over the swatches
    let offsets: &[f32] = match scheme {
        0 => &[0.0, 0.07, 0.14, 0.21, 0.28, 0.35, 0.42, 0.49],
        1 => &[0.0, 0.5, 0.06, 0.56, 0.12, 0.44, 0.03, 0.53],
        2 => &[0.0, 0.333, 0.667, 0.05, 0.383, 0.717, 0.1, 0.43],
        _ => &[0.0, 0.25, 0.5, 0.75, 0.04, 0.29, 0.54, 0.79],
    };
    let mut cols: Vec<Color32> = (0..n)
        .map(|i| {
            let h = (base + offsets[i % offsets.len()] + r.range(-0.02, 0.02)).rem_euclid(1.0);
            let s = r.range(0.45, 0.85);
            let v = r.range(0.62, 0.97);
            hsv(h * 360.0, s, v)
        })
        .collect();
    // order by brightness for a calm light-to-dark run
    cols.sort_by(|a, b| glowkit_core::color::luminance(*b).total_cmp(&glowkit_core::color::luminance(*a)));
    cols
}

/// A row of colour swatches that go together, with a round refresh button. Press it (or any
/// swatch) and a new palette washes across from left to right: each swatch blends to its new
/// colour with a little squash while the refresh arrows spin round.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::PaletteSwatches::new().colors(4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PaletteSwatches {
    n: usize,
    swatch: egui::Vec2,
    corner: f32,
    morph: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl Default for PaletteSwatches {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteSwatches {
    /// A four-colour palette.
    pub fn new() -> Self {
        Self { n: 4, swatch: vec2(54.0, 58.0), corner: 14.0, morph: 0.5, dark: false, stroke: true, autoplay: true }
    }
    /// Number of colours.
    pub fn colors(mut self, n: usize) -> Self {
        self.n = n.clamp(2, 8);
        self
    }
    /// Size of each swatch.
    pub fn swatch_size(mut self, size: egui::Vec2) -> Self {
        self.swatch = size.max(vec2(16.0, 16.0));
        self
    }
    /// Corner radius of the swatches.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// How slowly the colours blend into the next palette (0..1).
    pub fn morph(mut self, k: f32) -> Self {
        self.morph = k.clamp(0.0, 1.0);
        self
    }
    /// Dark tray.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outlines (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Shuffle on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for PaletteSwatches {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.n;
        let pad = 6.0;
        let gap = 5.0;
        let tray_w = pad * 2.0 + n as f32 * self.swatch.x + (n - 1) as f32 * gap;
        let tray_h = pad * 2.0 + self.swatch.y;
        let btn_d = 44.0;
        let (rect, mut response) = ui.allocate_exact_size(vec2(tray_w + 8.0 + btn_d, tray_h), Sense::hover());
        let id = response.id;
        let tray = Rect::from_min_size(rect.min, vec2(tray_w, tray_h));
        let btn = Rect::from_center_size(pos2(rect.max.x - btn_d / 2.0, rect.center().y), vec2(btn_d, btn_d));
        let b_resp = ui.interact(btn, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let t_resp = ui.interact(tray, id.with("tray"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let now = ui.input(|i| i.time);
        let idle = idle_for(ui, id, b_resp.hovered() || t_resp.hovered());
        let auto = self.autoplay && idle > 1.5;
        let dur = 0.25 + 0.6 * self.morph;
        let seed0 = id.value() % 1000 + 3;
        let mut shuffled = b_resp.clicked() || t_resp.clicked();
        let st = glowkit_core::with_state(
            ui,
            id,
            || PaletteState {
                from: harmonious(seed0, n),
                to: harmonious(seed0, n),
                since: -10.0,
                seed: seed0,
                turns: 0.0,
            },
            |s| {
                if s.to.len() != n {
                    s.to = harmonious(s.seed, n);
                    s.from = s.to.clone();
                }
                if auto && now - s.since > 2.6 {
                    shuffled = true;
                }
                if shuffled {
                    // continue from whatever is on screen right now
                    let e = (now - s.since) as f32;
                    s.from = (0..n)
                        .map(|i| mix(s.from[i], s.to[i], ease(Easing::InOutCubic, stagger(e, i, 0.07, dur))))
                        .collect();
                    s.seed = s.seed.wrapping_add(1);
                    s.to = harmonious(s.seed, n);
                    s.since = now;
                    s.turns += 1.0;
                }
                s.clone()
            },
        );
        if shuffled {
            response.mark_changed();
        }
        let e = (now - st.since) as f32;
        if auto || e < dur + n as f32 * 0.07 + 0.1 {
            glowkit_core::animate(ui);
        }
        let spin = spring(ui, id.with("spin"), st.turns, 120.0, 14.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let bg = if self.dark { t.primary } else { surface(&t) };
        let fg = if self.dark { t.primary_foreground } else { t.foreground };
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        paint::soft_shadow(painter, tray, self.corner + pad, 16.0, vec2(0.0, 4.0), shadow_color(&t, 0.6));
        painter.rect(tray, cr(self.corner + pad), bg, line, egui::StrokeKind::Inside);
        let hover = ui.input(|i| i.pointer.hover_pos());
        for i in 0..n {
            let k = ease(Easing::InOutCubic, stagger(e, i, 0.07, dur));
            let c = mix(st.from[i], st.to[i], k);
            let bump = (k * std::f32::consts::PI).sin() * if st.since > 0.0 { 1.0 } else { 0.0 };
            let base = Rect::from_min_size(tray.min + vec2(pad + i as f32 * (self.swatch.x + gap), pad), self.swatch);
            let hov = hover.is_some_and(|p| base.contains(p));
            let lift = spring(ui, id.with(("lift", i)), if hov { 1.0 } else { 0.0 }, 400.0, 24.0);
            let sz = vec2(self.swatch.x * (1.0 + 0.04 * bump), self.swatch.y * (1.0 - 0.08 * bump));
            let r = Rect::from_center_size(base.center() - vec2(0.0, 3.0 * lift), sz);
            painter.rect_filled(r, cr(self.corner), c);
            // glossy top edge
            painter.rect_filled(
                Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.5)),
                cr(self.corner),
                with_alpha(Color32::WHITE, 0.06),
            );
            if lift > 0.05 {
                let [rr, gg, bb, _] = st.to[i].to_srgba_unmultiplied();
                let txt = format!("#{rr:02X}{gg:02X}{bb:02X}");
                painter.text(
                    pos2(r.center().x, r.max.y - 9.0),
                    Align2::CENTER_CENTER,
                    txt,
                    FontId::monospace(8.5),
                    with_alpha(glowkit_core::color::contrast_text(c), 0.85 * lift),
                );
            }
        }
        let press = if b_resp.is_pointer_button_down_on() { 0.93 } else { 1.0 };
        paint::soft_shadow(painter, btn, btn_d / 2.0, 14.0, vec2(0.0, 3.0), shadow_color(&t, 0.6));
        painter.circle(btn.center(), btn_d / 2.0 * press, bg, line);
        super::common::refresh_icon(painter, btn.center(), 15.0, fg, 1.6, spin * std::f32::consts::TAU);
        response.union(b_resp).union(t_resp)
    }
}
