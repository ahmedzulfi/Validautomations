use super::util::{bold_text, check, idle_time};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::{anim::smoothstep, color::with_alpha, hex, mix, paint, Theme};

/// Default beam colors (cyan → indigo → violet).
const BEAM: [Color32; 3] = [hex("#18CCFC"), hex("#6344F5"), hex("#AE48FF")];

fn paint_beam(ui: &Ui, id: egui::Id, x: f32, top: f32, bottom: f32, progress: f32, colors: &[Color32; 3]) {
    let t = Theme::of(ui);
    let p = ui.painter();
    let prog = glowkit_core::anim::spring(ui, id.with("beam"), progress.clamp(0.0, 1.0), 90.0, 18.0).clamp(0.0, 1.0);
    let dot_c = pos2(x, top + 9.0);
    let track_top = dot_c.y + 10.0;
    let len = (bottom - track_top).max(1.0);
    // track (fades out at the bottom)
    let track: Vec<_> = (0..=24).map(|i| pos2(x, track_top + len * i as f32 / 24.0)).collect();
    paint::gradient_polyline(p, &track, false, 1.5, |u| with_alpha(t.border, (1.0 - smoothstep(0.75, 1.0, u)) * 0.9 + 0.1));
    // beam
    let head = track_top + len * prog;
    if head > track_top + 1.0 {
        let pts: Vec<_> = (0..=32).map(|i| pos2(x, track_top + (head - track_top) * i as f32 / 32.0)).collect();
        paint::gradient_polyline(p, &pts, false, 2.0, |u| {
            let c = glowkit_core::color::gradient(colors, u);
            with_alpha(c, smoothstep(0.0, 0.35, u))
        });
        paint::glow_ellipse(p, pos2(x, head - 18.0), vec2(5.0, 30.0), with_alpha(colors[2], 0.35));
        paint::glow(p, pos2(x, head), 10.0, with_alpha(colors[2], 0.8));
        p.circle_filled(pos2(x, head), 2.0, Color32::WHITE);
    }
    // top dot
    let active = smoothstep(0.0, 0.04, prog);
    paint::soft_shadow(p, Rect::from_center_size(dot_c, Vec2::splat(18.0)), 9.0, 6.0, Vec2::ZERO, Color32::from_black_alpha(40));
    p.circle_filled(dot_c, 9.0, t.card);
    p.circle_stroke(dot_c, 9.0, Stroke::new(1.0, t.border));
    let inner_on = mix(colors[0], colors[1], 0.5);
    p.circle_filled(dot_c, 4.0, t.muted_foreground.lerp_to_gamma(inner_on, active));
    if active > 0.0 {
        paint::glow(p, dot_c, 12.0, with_alpha(inner_on, 0.5 * active));
    }
}

/// A vertical gradient beam beside your content that fills as the reader scrolls: a track line,
/// a dot at the top that lights up, and a glowing head that follows the scroll position with a spring.
///
/// Use [`TracingBeam::show`] to wrap content in a scroll area (the beam tracks its offset), or add
/// it as a widget with an explicit [`TracingBeam::progress`].
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_blocks::TracingBeam::new().height(360.0).show(ui, |ui| {
///     for i in 0..20 { ui.label(format!("Paragraph {i}")); }
/// });
/// ```
#[must_use = "call `.show(ui, ...)` or add it with `ui.add(...)`"]
pub struct TracingBeam {
    progress: f32,
    height: f32,
    colors: [Color32; 3],
    autoplay: bool,
}

impl Default for TracingBeam {
    fn default() -> Self {
        Self::new()
    }
}

impl TracingBeam {
    /// A beam with the default cyan / indigo / violet gradient.
    pub fn new() -> Self {
        Self { progress: 0.0, height: 360.0, colors: BEAM, autoplay: true }
    }
    /// Fill amount (0..1) when used as a plain widget.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = p;
        self
    }
    /// Height in points (of the beam, or of the scroll viewport with `show`).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Beam gradient stops (top → head).
    pub fn colors(mut self, a: Color32, b: Color32, c: Color32) -> Self {
        self.colors = [a, b, c];
        self
    }
    /// With `show`: slowly scroll the content by itself while the pointer is away.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
    /// Wrap `add` in a vertical scroll area of `height` with the beam in a left gutter.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        let id = ui.scope_id().with("tracing_beam");
        let gutter = 44.0;
        let avail = ui.available_width();
        let start = ui.cursor().min;
        let area_rect = Rect::from_min_size(start, vec2(avail, self.height));
        let hovered = ui.rect_contains_pointer(area_rect);
        let idle = idle_time(ui, id, hovered);
        // (offset, max offset) from last frame
        let (last_off, max_off): (f32, f32) = glowkit_core::state(ui, id.with("off"), || (0.0, 1.0));
        let mut sa = egui::ScrollArea::vertical().id_salt(id).max_height(self.height).min_scrolled_height(self.height).auto_shrink([false, false]);
        if self.autoplay && idle > 2.0 && max_off > 1.0 {
            let tt = glowkit_core::time(ui) * 0.12;
            let tri = 1.0 - (tt.fract() * 2.0 - 1.0).abs();
            let target = smoothstep(0.0, 1.0, tri) * max_off;
            let k = ((idle - 2.0) / 2.0).clamp(0.0, 1.0) * 0.06;
            sa = sa.vertical_scroll_offset(last_off + (target - last_off) * k);
            glowkit_core::animate(ui);
        }
        let mut out = None;
        ui.horizontal_top(|ui| {
            ui.add_space(gutter);
            let o = sa.show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(avail - gutter - 16.0);
                    add(ui)
                })
                .inner
            });
            out = Some(o);
        });
        let out = out.expect("scroll area shown");
        let max = (out.content_size.y - out.inner_rect.height()).max(1.0);
        glowkit_core::set_state(ui, id.with("off"), (out.state.clamped_offset().y, max));
        let prog = (out.state.clamped_offset().y / max).clamp(0.0, 1.0);
        let r = out.inner_rect;
        paint_beam(ui, id, start.x + gutter * 0.45, r.min.y, r.max.y, prog * 0.92 + 0.08, &self.colors);
        out
    }
}

impl Widget for TracingBeam {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, self.height), Sense::hover());
        if ui.is_rect_visible(rect) {
            paint_beam(ui, resp.id, rect.center().x, rect.min.y, rect.max.y, self.progress, &self.colors);
        }
        resp
    }
}

/// An overlay card listing the steps of a long operation. Steps check off one by one; the
/// current one is highlighted and the list glides so it stays centered, with the others fading
/// by distance.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::MultiStepLoader::new(&["Buying a condo", "Travelling in a flight", "Meeting Tyler Durden"])
///     .step_duration(1.5));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MultiStepLoader {
    steps: Vec<String>,
    step_duration: f32,
    looping: bool,
    size: Option<Vec2>,
    current: Option<usize>,
    backdrop: bool,
    accent: Option<Color32>,
}

impl MultiStepLoader {
    /// A loader for these step labels.
    pub fn new(steps: &[&str]) -> Self {
        Self {
            steps: steps.iter().map(|s| s.to_string()).collect(),
            step_duration: 1.6,
            looping: true,
            size: None,
            current: None,
            backdrop: true,
            accent: None,
        }
    }
    /// Seconds per step when running on its own timer.
    pub fn step_duration(mut self, s: f32) -> Self {
        self.step_duration = s.max(0.1);
        self
    }
    /// Restart from the first step after finishing.
    pub fn looping(mut self, b: bool) -> Self {
        self.looping = b;
        self
    }
    /// Control the current step yourself (disables the timer). `steps.len()` = all done.
    pub fn current(mut self, step: usize) -> Self {
        self.current = Some(step);
        self
    }
    /// Overlay size (defaults to the available width x 360).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Dim whatever is behind the card.
    pub fn backdrop(mut self, b: bool) -> Self {
        self.backdrop = b;
        self
    }
    /// Check color (defaults to a lime / success green).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl Widget for MultiStepLoader {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 360.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let n = self.steps.len().max(1);
        let (cur, local) = match self.current {
            Some(c) => (c.min(n), 1.0),
            None => {
                let el = glowkit_core::Clock::elapsed(ui, resp.id);
                let cycle = (n as f32 + 1.2) * self.step_duration;
                let e = if self.looping { el % cycle } else { el };
                glowkit_core::animate(ui);
                let i = (e / self.step_duration) as usize;
                (i.min(n), (e / self.step_duration).fract())
            }
        };
        let accent = self.accent.unwrap_or(if t.dark { hex("#A3E635") } else { hex("#65A30D") });
        let p = ui.painter_at(rect);
        if self.backdrop {
            p.rect_filled(rect, t.radius, with_alpha(t.background, 0.72));
        }
        let center_y = rect.center().y;
        let scroll = glowkit_core::anim::spring(ui, resp.id.with("scroll"), cur.min(n - 1) as f32, 120.0, 20.0);
        let row_h = 42.0;
        let col_w = 320.0f32.min(rect.width() - 40.0);
        let x0 = rect.center().x - col_w / 2.0;
        let time = glowkit_core::time(ui);
        for (i, label) in self.steps.iter().enumerate() {
            let d = i as f32 - scroll;
            let y = center_y + d * row_h;
            let fade = (1.0 - (d.abs() / 3.2)).clamp(0.0, 1.0);
            if fade <= 0.0 {
                continue;
            }
            let done = i < cur;
            let is_cur = i == cur;
            let c = pos2(x0 + 12.0, y);
            if done {
                p.circle_filled(c, 10.0, with_alpha(accent, fade));
                check(&p, c, 13.0, Stroke::new(2.2, with_alpha(glowkit_core::color::contrast_text(accent), fade)), 1.0);
            } else if is_cur {
                // the current step: ring that fills, then a check pops in near the end
                paint::glow(&p, c, 22.0, with_alpha(accent, 0.25));
                p.circle_stroke(c, 10.0, Stroke::new(1.5, with_alpha(t.border, fade)));
                paint::arc(&p, c, 10.0, -std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2 + local * std::f32::consts::TAU, 2.0, accent);
                let spin = time * 6.0;
                p.circle_filled(c + vec2(spin.cos(), spin.sin()) * 4.0, 1.6, accent);
            } else {
                p.circle_stroke(c, 10.0, Stroke::new(1.5, with_alpha(t.muted_foreground, 0.6 * fade)));
            }
            let col = if is_cur {
                t.foreground
            } else if done {
                with_alpha(accent, 0.85 * fade)
            } else {
                with_alpha(t.muted_foreground, fade)
            };
            let font = FontId::proportional(if is_cur { 17.0 } else { 15.0 });
            if is_cur {
                bold_text(&p, pos2(x0 + 34.0, y), Align2::LEFT_CENTER, label, font, col);
            } else {
                p.text(pos2(x0 + 34.0, y), Align2::LEFT_CENTER, label, font, col);
            }
        }
        // progress bar under the list
        let frac = (cur as f32 + if cur < n { local } else { 0.0 }) / n as f32;
        let bar = Rect::from_min_size(pos2(x0, rect.max.y - 34.0), vec2(col_w, 4.0));
        p.rect_filled(bar, 2.0, with_alpha(t.muted_foreground, 0.2));
        let fill = Rect::from_min_size(bar.min, vec2(col_w * frac.clamp(0.0, 1.0), 4.0));
        paint::gradient_rect(&p, fill, 2.0, 0.0, &[mix(accent, t.background, 0.4), accent]);
        p.text(pos2(bar.max.x, bar.min.y - 10.0), Align2::RIGHT_CENTER, format!("{}/{}", cur.min(n), n), FontId::proportional(11.0), t.muted_foreground);
        resp
    }
}
