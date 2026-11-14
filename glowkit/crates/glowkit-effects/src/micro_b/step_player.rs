use super::common::{cr, hairline, idle_for, ink, shadow_color, surface};
use egui::{pos2, vec2, Rect, Response, Sense, Shape, Stroke, Ui, Widget};
use glowkit_core::anim::spring;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Debug, Default)]
struct PlayerState {
    playing: bool,
    step: usize,
    progress: f32,
}

/// A compact story-style step indicator with a play button: press play and the current step
/// stretches into a bar that fills over `pace` seconds, then shrinks back to a dot while the
/// next dot springs open. Click a dot to jump to it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::StepPlayer::new(5).pace(3.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StepPlayer {
    steps: usize,
    pace: f32,
    dark: bool,
    stroke: bool,
    bar: f32,
    looping: bool,
    autoplay: bool,
}

impl StepPlayer {
    /// A player with `steps` steps.
    pub fn new(steps: usize) -> Self {
        Self { steps: steps.clamp(1, 12), pace: 3.0, dark: false, stroke: false, bar: 30.0, looping: false, autoplay: true }
    }
    /// Seconds each step takes to fill.
    pub fn pace(mut self, secs: f32) -> Self {
        self.pace = secs.max(0.2);
        self
    }
    /// Dark (ink) pill instead of the light surface.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Draw a hairline outline around the pill and button.
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Length of the active bar in points (the "wake").
    pub fn bar_length(mut self, w: f32) -> Self {
        self.bar = w.clamp(12.0, 120.0);
        self
    }
    /// Start over after the last step instead of stopping.
    pub fn looping(mut self, on: bool) -> Self {
        self.looping = on;
        self
    }
    /// Press play on its own after a few idle seconds (and loop while idle). Default on.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for StepPlayer {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = 30.0;
        let dot = 6.0;
        let gap = 7.0;
        let pad = 12.0;
        let n = self.steps;
        let pill_w = pad * 2.0 + self.bar + (n - 1) as f32 * (dot + gap);
        let btn = 32.0;
        let (rect, response) = ui.allocate_exact_size(vec2(pill_w + 8.0 + btn, btn), Sense::hover());
        let id = response.id;
        let pill = Rect::from_min_size(pos2(rect.min.x, rect.center().y - h / 2.0), vec2(pill_w, h));
        let btn_rect = Rect::from_center_size(pos2(rect.max.x - btn / 2.0, rect.center().y), vec2(btn, btn));
        let b_resp = ui.interact(btn_rect, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let p_resp = ui.interact(pill, id.with("pill"), Sense::click());
        let dt = glowkit_core::dt(ui);
        let idle = idle_for(ui, id, b_resp.hovered() || p_resp.hovered());
        // which dot was clicked (by x position, using last frame's layout is fine: dots are evenly spaced)
        let mut jump = None;
        if p_resp.clicked() {
            if let Some(p) = p_resp.interact_pointer_pos() {
                let st: PlayerState = glowkit_core::state(ui, id, PlayerState::default);
                let mut x = pill.min.x + pad;
                for i in 0..n {
                    let w = if i == st.step { self.bar } else { dot };
                    if p.x < x + w + gap / 2.0 {
                        jump = Some(i);
                        break;
                    }
                    x += w + gap;
                }
            }
        }
        let clicked = b_resp.clicked();
        let auto = self.autoplay && idle > 2.5;
        let (pace, looping) = (self.pace, self.looping || auto);
        let st = glowkit_core::with_state(ui, id, PlayerState::default, |s| {
            if clicked {
                s.playing = !s.playing;
            }
            if let Some(j) = jump {
                s.step = j;
                s.progress = 0.0;
                s.playing = true;
            }
            if auto && !s.playing {
                s.playing = true;
            }
            if s.playing {
                s.progress += dt / pace;
                if s.progress >= 1.0 {
                    s.progress = 0.0;
                    if s.step + 1 < n {
                        s.step += 1;
                    } else {
                        s.step = 0;
                        s.playing = looping;
                    }
                }
            }
            s.step = s.step.min(n - 1);
            *s
        });
        if st.playing {
            glowkit_core::animate(ui);
        }
        let press = spring(ui, id.with("press"), if b_resp.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 500.0, 28.0);
        let play_morph = spring(ui, id.with("morph"), if st.playing { 1.0 } else { 0.0 }, 320.0, 22.0);
        let fill_vis = glowkit_core::anim::follow(ui, id.with("fv"), st.progress, 30.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let (ink_bg, ink_fg) = ink(&t);
        let (bg, fg) = if self.dark { (ink_bg, ink_fg) } else { (surface(&t), t.foreground) };
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        for r in [pill, btn_rect] {
            paint::soft_shadow(painter, r, h / 2.0, 12.0, vec2(0.0, 3.0), shadow_color(&t, 0.6));
        }
        painter.rect(pill, cr(h / 2.0), bg, line, egui::StrokeKind::Inside);
        // steps
        let track = with_alpha(fg, 0.16);
        let done = with_alpha(fg, 0.55);
        let mut x = pill.min.x + pad;
        for i in 0..n {
            let target = if i == st.step { self.bar } else { dot };
            let w = spring(ui, id.with(("w", i)), target, 260.0, 22.0).max(dot * 0.6);
            let r = Rect::from_min_size(pos2(x, pill.center().y - dot / 2.0), vec2(w, dot));
            let col = if i < st.step { done } else { track };
            painter.rect_filled(r, cr(dot / 2.0), col);
            if i == st.step {
                let fw = (r.width() * fill_vis).max(if fill_vis > 0.0 { dot } else { 0.0 }).min(r.width());
                if fw > 0.0 {
                    painter.rect_filled(Rect::from_min_size(r.min, vec2(fw, dot)), cr(dot / 2.0), fg);
                }
            }
            x += w + gap;
        }
        // play button
        let br = btn_rect.shrink(press * 1.5);
        let btn_fill = mix(bg, fg, b_resp.hovered() as u8 as f32 * 0.05);
        painter.circle(br.center(), br.width() / 2.0, btn_fill, line);
        let c = br.center();
        let s = 6.5;
        // play triangle morphing into two pause bars
        let m = play_morph;
        if m < 0.5 {
            let k = 1.0 - m * 2.0;
            let pts = vec![c + vec2(-s * 0.5, -s * 0.95), c + vec2(s * 0.85, 0.0), c + vec2(-s * 0.5, s * 0.95)];
            painter.add(Shape::convex_polygon(pts, with_alpha(fg, k), Stroke::NONE));
        } else {
            let k = (m - 0.5) * 2.0;
            for dx in [-s * 0.42, s * 0.42] {
                let r = Rect::from_center_size(c + vec2(dx, 0.0), vec2(2.8, s * 1.8 * k.max(0.2)));
                painter.rect_filled(r, cr(1.2), with_alpha(fg, k));
            }
        }
        response.union(b_resp)
    }
}
