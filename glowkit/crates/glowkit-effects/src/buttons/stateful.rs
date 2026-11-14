use super::shimmer::pressed_rect;
use super::util::{self, cr};
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::color::{contrast_text, shade, with_alpha};
use glowkit_core::{paint, Rng, Theme};

// ---------------------------------------------------------------------------------------------
// Stateful

/// The state shown by a [`StatefulButton`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StatefulButtonState {
    /// Waiting for a click.
    #[default]
    Idle,
    /// Work in progress: shows a spinner.
    Loading,
    /// Done: shows a check mark (and returns to idle after a while).
    Success,
}

#[derive(Clone, Copy)]
struct Phase {
    state: StatefulButtonState,
    since: f32,
}

/// A button that walks through idle, loading (spinner) and success (drawn check mark), smoothly
/// resizing between them.
///
/// Clicking an idle button switches it to `Loading`; set `Success` yourself when the work is
/// done (or use [`Self::simulate`]).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{StatefulButton, StatefulButtonState};
/// let mut state = StatefulButtonState::Idle; // keep this in your app state
/// if ui.add(StatefulButton::new("Send message", &mut state)).clicked() {
///     // start the work; later: state = StatefulButtonState::Success;
/// }
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StatefulButton<'a> {
    label: String,
    state: &'a mut StatefulButtonState,
    loading_label: Option<String>,
    success_label: Option<String>,
    size: f32,
    color: Option<Color32>,
    success_color: Option<Color32>,
    reset_after: Option<f32>,
    simulate: Option<f32>,
}

impl<'a> StatefulButton<'a> {
    /// A stateful button with a label, bound to caller-owned state.
    pub fn new(label: impl Into<String>, state: &'a mut StatefulButtonState) -> Self {
        Self {
            label: label.into(),
            state,
            loading_label: None,
            success_label: None,
            size: 15.0,
            color: None,
            success_color: None,
            reset_after: Some(2.0),
            simulate: None,
        }
    }
    /// Label while loading. Defaults to the idle label.
    pub fn loading_label(mut self, label: impl Into<String>) -> Self {
        self.loading_label = Some(label.into());
        self
    }
    /// Label on success. Defaults to the idle label.
    pub fn success_label(mut self, label: impl Into<String>) -> Self {
        self.success_label = Some(label.into());
        self
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Fill color. Defaults to the accent.
        color: Color32);
    opt_setter!(/// Fill color on success. Defaults to the theme's success color.
        success_color: Color32);
    setter!(/// Seconds after success before returning to idle (`None` = stay).
        reset_after: Option<f32>);
    opt_setter!(/// Pretend the work takes this many seconds: loading turns into success automatically.
        simulate: f32);
}

impl Widget for StatefulButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let time = glowkit_core::time(ui);
        let label = match *self.state {
            StatefulButtonState::Idle => self.label.clone(),
            StatefulButtonState::Loading => self.loading_label.clone().unwrap_or_else(|| self.label.clone()),
            StatefulButtonState::Success => self.success_label.clone().unwrap_or_else(|| self.label.clone()),
        };
        let g = util::text_galley(ui, &label, self.size, Color32::WHITE);
        let h = (self.size * 2.8).round();
        let icon = self.size * 1.1;
        let has_icon = *self.state != StatefulButtonState::Idle;
        let target_w = g.size().x + self.size * 3.2 + if has_icon { icon + 8.0 } else { 0.0 };
        let id = ui.next_auto_id();
        let w = glowkit_core::anim::spring(ui, id.with("w"), target_w, 260.0, 24.0);
        // allocate the widest of current and target so layout doesn't jitter
        let (rect, resp) = util::alloc_button(ui, vec2(target_w.max(w).ceil(), h), &label);
        let key = resp.id.with("phase");
        let mut phase: Phase = glowkit_core::state(ui, key, || Phase { state: *self.state, since: time });
        if resp.clicked() && *self.state == StatefulButtonState::Idle {
            *self.state = StatefulButtonState::Loading;
        }
        if phase.state != *self.state {
            phase = Phase { state: *self.state, since: time };
        }
        let age = time - phase.since;
        match *self.state {
            StatefulButtonState::Loading => {
                if let Some(s) = self.simulate {
                    if age >= s {
                        *self.state = StatefulButtonState::Success;
                        phase = Phase { state: *self.state, since: time };
                    }
                }
            }
            StatefulButtonState::Success => {
                if let Some(r) = self.reset_after {
                    if age >= r {
                        *self.state = StatefulButtonState::Idle;
                        phase = Phase { state: *self.state, since: time };
                    }
                }
            }
            StatefulButtonState::Idle => {}
        }
        glowkit_core::set_state(ui, key, phase);
        if *self.state != StatefulButtonState::Idle {
            glowkit_core::animate(ui);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let age = time - phase.since;
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let body = Rect::from_center_size(rect.center(), vec2(w, h));
        let body = pressed_rect(body, press);
        let base = self.color.unwrap_or(t.accent);
        let ok = self.success_color.unwrap_or(t.success);
        let sk = glowkit_core::anim::follow(
            ui,
            resp.id.with("ok"),
            if *self.state == StatefulButtonState::Success { 1.0 } else { 0.0 },
            8.0,
        );
        let fill = base.lerp_to_gamma(ok, sk);
        let fill = shade(fill, 0.04 * hover);
        let text = contrast_text(base);
        let radius = h / 2.0;
        let p = ui.painter();
        paint::soft_shadow(p, body, radius, 14.0, vec2(0.0, 4.0), with_alpha(fill, if t.dark { 0.35 } else { 0.3 }));
        p.rect_filled(body, cr(radius), fill);
        let label_g = util::text_galley(ui, &label, self.size, text);
        let icon_k = glowkit_core::anim::follow(ui, resp.id.with("icon"), if has_icon { 1.0 } else { 0.0 }, 14.0);
        let content_w = label_g.size().x + (icon + 8.0) * icon_k;
        let left = body.center().x - content_w / 2.0;
        let ic = pos2(left + icon / 2.0, body.center().y);
        let clip = p.with_clip_rect(body);
        match phase.state {
            StatefulButtonState::Loading => {
                util::spinner(&clip, ic, icon * 0.42, 2.0, text.gamma_multiply(icon_k), time)
            }
            StatefulButtonState::Success => {
                let k = (age / 0.35).clamp(0.0, 1.0);
                clip.circle_filled(
                    ic,
                    icon * 0.5 * glowkit_core::ease(glowkit_core::Easing::OutBack, k),
                    with_alpha(text, 0.22),
                );
                util::check_mark(&clip, ic, icon * 0.6, 2.0, text, ((age - 0.1) / 0.3).clamp(0.0, 1.0));
            }
            StatefulButtonState::Idle => {}
        }
        let lx = left + (icon + 8.0) * icon_k;
        clip.galley_with_override_text_color(pos2(lx, body.center().y - label_g.size().y / 2.0), label_g, text);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Subscribe

/// A toggle button that flips between two labels with a vertical slide, gaining a check mark
/// and a new color when active.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut subscribed = false; // keep this in your app state
/// ui.add(glowkit_effects::SubscribeButton::new(&mut subscribed));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SubscribeButton<'a> {
    on: &'a mut bool,
    label: String,
    active_label: String,
    size: f32,
    color: Option<Color32>,
    active_color: Option<Color32>,
    min_width: Option<f32>,
}

impl<'a> SubscribeButton<'a> {
    /// A subscribe toggle bound to caller-owned state.
    pub fn new(on: &'a mut bool) -> Self {
        Self {
            on,
            label: "Subscribe".into(),
            active_label: "Subscribed".into(),
            size: 15.0,
            color: None,
            active_color: None,
            min_width: None,
        }
    }
    /// The two labels (inactive, active).
    pub fn labels(mut self, off: impl Into<String>, on: impl Into<String>) -> Self {
        self.label = off.into();
        self.active_label = on.into();
        self
    }
    setter!(/// Font size in points.
        size: f32);
    opt_setter!(/// Inactive fill. Defaults to the theme's primary color.
        color: Color32);
    opt_setter!(/// Active fill. Defaults to the theme's secondary color.
        active_color: Color32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for SubscribeButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let g_off = util::text_galley(ui, &self.label, self.size, Color32::WHITE);
        let g_on = util::text_galley(ui, &self.active_label, self.size, Color32::WHITE);
        let icon = self.size;
        let w = (g_off.size().x.max(g_on.size().x + icon + 8.0) + self.size * 3.2).max(self.min_width.unwrap_or(0.0));
        let (rect, mut resp) = util::alloc_button(ui, vec2(w.round(), (self.size * 2.8).round()), &self.label);
        if resp.clicked() {
            *self.on = !*self.on;
            resp.mark_changed();
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let k = glowkit_core::anim::spring(ui, resp.id.with("k"), if *self.on { 1.0 } else { 0.0 }, 240.0, 22.0);
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let radius = t.radius;
        let off_c = self.color.unwrap_or(t.primary);
        let on_c = self.active_color.unwrap_or(t.secondary);
        let fill = off_c.lerp_to_gamma(on_c, k.clamp(0.0, 1.0));
        let p = ui.painter();
        p.rect_filled(rect, cr(radius), shade(fill, if t.dark { -0.03 } else { 0.03 } * hover));
        if k > 0.01 {
            p.rect_stroke(
                rect,
                cr(radius),
                Stroke::new(1.0, with_alpha(t.border, k.clamp(0.0, 1.0))),
                StrokeKind::Inside,
            );
        }
        let clip = p.with_clip_rect(rect.shrink(1.0));
        let dy = rect.height();
        // inactive label slides up and out; active slides in from below
        let off_text = contrast_text(off_c);
        let on_text = contrast_text(on_c);
        let c1 = rect.center() - vec2(0.0, dy * k);
        let g1 = util::text_galley(ui, &self.label, self.size, off_text);
        util::galley_centered(&clip, &g1, c1, off_text);
        let c2 = rect.center() + vec2(0.0, dy * (1.0 - k));
        let g2 = util::text_galley(ui, &self.active_label, self.size, on_text);
        let total = g2.size().x + icon + 8.0;
        let ic = pos2(c2.x - total / 2.0 + icon / 2.0, c2.y);
        util::check_mark(&clip, ic, icon * 0.8, 2.0, t.success, k.clamp(0.0, 1.0));
        util::galley_centered(&clip, &g2, pos2(ic.x + icon / 2.0 + 8.0 + g2.size().x / 2.0, c2.y), on_text);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Copy

/// A copy-to-clipboard button whose icon morphs into a check mark after copying.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::CopyButton::new("cargo add glowkit").label("Copy"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct CopyButton {
    text: String,
    label: Option<String>,
    copied_label: Option<String>,
    size: f32,
    color: Option<Color32>,
    duration: f32,
    demo_loop: bool,
}

impl CopyButton {
    /// A button that copies `text` to the clipboard when clicked.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            label: None,
            copied_label: None,
            size: 15.0,
            color: None,
            duration: 1.8,
            demo_loop: false,
        }
    }
    /// Show a text label next to the icon (default: icon only).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Label after copying. Defaults to "Copied!" when a label is set.
    pub fn copied_label(mut self, label: impl Into<String>) -> Self {
        self.copied_label = Some(label.into());
        self
    }
    setter!(/// Font / icon size in points.
        size: f32);
    opt_setter!(/// Check-mark color. Defaults to the theme's success color.
        color: Color32);
    setter!(/// Seconds the check mark stays before morphing back.
        duration: f32);
    setter!(/// Loop the copy animation on its own (for showcases). Does not touch the clipboard.
        demo_loop: bool);
}

fn copy_icon(painter: &egui::Painter, c: Pos2, s: f32, color: Color32, bg: Color32, scale: f32, angle_k: f32) {
    if scale <= 0.01 {
        return;
    }
    let s = s * scale;
    let sq = s * 0.58;
    let stroke = Stroke::new(1.6, color);
    let r = cr(s * 0.12);
    let off = vec2(s, s) * (0.13 + 0.03 * angle_k);
    let back = Rect::from_center_size(c - off, vec2(sq, sq));
    let front = Rect::from_center_size(c + off, vec2(sq, sq));
    painter.rect_stroke(back, r, stroke, StrokeKind::Middle);
    painter.rect_filled(front, r, bg);
    painter.rect_stroke(front, r, stroke, StrokeKind::Middle);
}

impl Widget for CopyButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = (self.size * 2.5).round();
        let copied_label = self.copied_label.clone().or_else(|| self.label.as_ref().map(|_| "Copied!".to_string()));
        let label_w = match (&self.label, &copied_label) {
            (Some(a), Some(b)) => {
                let ga = util::text_galley(ui, a, self.size * 0.93, Color32::WHITE);
                let gb = util::text_galley(ui, b, self.size * 0.93, Color32::WHITE);
                ga.size().x.max(gb.size().x) + 8.0
            }
            _ => 0.0,
        };
        let w = if label_w > 0.0 { h * 0.8 + label_w + self.size } else { h };
        let (rect, resp) = util::alloc_button(ui, vec2(w.round(), h), self.label.as_deref().unwrap_or("Copy"));
        let time = glowkit_core::time(ui);
        let key = resp.id.with("copied_at");
        let mut at: f32 = glowkit_core::state(ui, key, || -100.0);
        if resp.clicked() {
            ui.ctx().copy_text(self.text.clone());
            at = time;
            glowkit_core::set_state(ui, key, at);
        }
        if self.demo_loop {
            let cycle = self.duration + 1.6;
            at = at.max(time - time % cycle);
            glowkit_core::animate(ui);
        }
        let copied = time - at < self.duration;
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let k = glowkit_core::anim::spring(ui, resp.id.with("k"), if copied { 1.0 } else { 0.0 }, 260.0, 18.0);
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let rect = pressed_rect(rect, press);
        let ok = self.color.unwrap_or(t.success);
        let radius = t.radius * 0.8;
        let p = ui.painter();
        let fill = shade(t.card, if t.dark { 0.03 } else { -0.02 } * hover).lerp_to_gamma(ok, 0.08 * k.clamp(0.0, 1.0));
        p.rect_filled(rect, cr(radius), fill);
        p.rect_stroke(
            rect,
            cr(radius),
            Stroke::new(1.0, t.border.lerp_to_gamma(ok, 0.5 * k.clamp(0.0, 1.0))),
            StrokeKind::Inside,
        );
        let ic = if label_w > 0.0 { pos2(rect.min.x + h * 0.5, rect.center().y) } else { rect.center() };
        let icon_s = self.size * 1.25;
        let kc = k.clamp(0.0, 1.2);
        copy_icon(
            p,
            ic,
            icon_s,
            t.muted_foreground.lerp_to_gamma(t.foreground, hover),
            fill,
            (1.0 - kc).max(0.0),
            hover,
        );
        if k > 0.01 {
            util::check_mark(p, ic, icon_s * 0.55 * k.max(0.0), 2.0, ok, k.clamp(0.0, 1.0));
        }
        if let (Some(l), Some(cl)) = (&self.label, &copied_label) {
            let clip = p.with_clip_rect(rect.shrink(1.0));
            let x = ic.x + h * 0.38;
            let kk = k.clamp(0.0, 1.0);
            let g1 = util::text_galley(ui, l, self.size * 0.93, t.foreground);
            let g2 = util::text_galley(ui, cl, self.size * 0.93, ok);
            let y = rect.center().y;
            clip.galley_with_override_text_color(
                pos2(x, y - g1.size().y / 2.0 - kk * h * 0.8),
                g1,
                t.foreground.gamma_multiply(1.0 - kk),
            );
            clip.galley_with_override_text_color(
                pos2(x, y - g2.size().y / 2.0 + (1.0 - kk) * h * 0.8),
                g2,
                ok.gamma_multiply(kk),
            );
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Particle burst

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    size: f32,
    color: Color32,
    rot: f32,
    spin: f32,
    shape: u8,
}

/// A button that bursts into a shower of confetti-like particles when clicked (and keeps
/// spraying while held).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_effects::ParticleBurstButton::new("Cool mode"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ParticleBurstButton {
    label: String,
    size: f32,
    colors: Vec<Color32>,
    background: Option<Color32>,
    count: usize,
    auto_burst: Option<f32>,
    min_width: Option<f32>,
}

impl ParticleBurstButton {
    /// A particle-burst button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: 15.0,
            colors: Vec::new(),
            background: None,
            count: 22,
            auto_burst: None,
            min_width: None,
        }
    }
    setter!(/// Font size in points.
        size: f32);
    /// Particle colors. Defaults to the theme's chart palette.
    pub fn colors(mut self, colors: &[Color32]) -> Self {
        self.colors = colors.to_vec();
        self
    }
    opt_setter!(/// Fill color. Defaults to the theme's primary color.
        background: Color32);
    setter!(/// Particles per burst.
        count: usize);
    opt_setter!(/// Also burst on its own every this many seconds (for showcases or celebrations).
        auto_burst: f32);
    opt_setter!(/// Minimum width in points.
        min_width: f32);
}

impl Widget for ParticleBurstButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = super::shimmer::label_size(ui, &self.label, self.size, 8.0, self.min_width);
        let (rect, resp) = util::alloc_button(ui, size, &self.label);
        let colors = if self.colors.is_empty() { t.chart.to_vec() } else { self.colors.clone() };
        let time = glowkit_core::time(ui);
        let dt = glowkit_core::dt(ui);
        let key = resp.id.with("particles");
        let mut parts: Vec<Particle> = glowkit_core::state(ui, key, Vec::new);
        let mut rng = Rng::new((time * 1000.0) as u64 ^ resp.id.value());
        let spawn = |parts: &mut Vec<Particle>, at: Vec2, n: usize, rng: &mut Rng| {
            for _ in 0..n {
                let a = rng.range(-2.6, -0.55);
                let sp = rng.range(140.0, 380.0);
                parts.push(Particle {
                    pos: at,
                    vel: vec2(a.cos(), a.sin()) * sp,
                    age: 0.0,
                    life: rng.range(0.9, 1.6),
                    size: rng.range(3.0, 7.0),
                    color: *rng.pick(&colors),
                    rot: rng.range(0.0, std::f32::consts::TAU),
                    spin: rng.range(-8.0, 8.0),
                    shape: rng.below(3) as u8,
                });
            }
        };
        let local = |p: Pos2| p - rect.min;
        if resp.is_pointer_button_down_on() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if ui.input(|i| i.pointer.any_pressed()) {
                    spawn(&mut parts, local(pos), self.count, &mut rng);
                } else if rng.f32() < dt * 25.0 {
                    spawn(&mut parts, local(pos), 1, &mut rng);
                }
            }
        }
        if let Some(every) = self.auto_burst {
            let last: f32 = glowkit_core::state(ui, key.with("auto"), || time - every + 0.4);
            if time - last >= every.max(0.3) {
                let x = rect.width() * rng.range(0.3, 0.7);
                spawn(&mut parts, vec2(x, rect.height() * 0.4), self.count, &mut rng);
                glowkit_core::set_state(ui, key.with("auto"), time);
            }
            glowkit_core::animate(ui);
        }
        for p in &mut parts {
            p.age += dt;
            p.vel.y += 620.0 * dt;
            p.vel *= (-1.2 * dt).exp();
            p.pos += p.vel * dt;
            p.rot += p.spin * dt;
        }
        parts.retain(|p| p.age < p.life);
        if parts.len() > 400 {
            let n = parts.len() - 400;
            parts.drain(0..n);
        }
        if !parts.is_empty() {
            glowkit_core::animate(ui);
        }
        glowkit_core::set_state(ui, key, parts.clone());
        if !ui.is_rect_visible(rect.expand(200.0)) {
            return resp;
        }
        let hover = util::hover_t(ui, resp.id, resp.hovered(), 10.0);
        let press = util::press_t(ui, resp.id, resp.is_pointer_button_down_on());
        let body = pressed_rect(rect, press * 2.0);
        let bg = self.background.unwrap_or(t.primary);
        let text = contrast_text(bg);
        let radius = body.height() / 2.0;
        let pnt = ui.painter();
        paint::soft_shadow(
            pnt,
            body,
            radius,
            14.0,
            vec2(0.0, 4.0),
            with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.15 }),
        );
        pnt.rect_filled(body, cr(radius), shade(bg, if t.dark { -0.04 } else { 0.06 } * hover));
        let g = util::text_galley(ui, &self.label, self.size, text);
        util::galley_centered(pnt, &g, body.center(), text);
        // particles on top
        for p in &parts {
            let k = p.age / p.life;
            let a = (1.0 - k).powf(0.6);
            let c = p.color.gamma_multiply(a);
            let at = rect.min + p.pos;
            match p.shape {
                0 => {
                    pnt.circle_filled(at, p.size * 0.6, c);
                }
                1 => paint::sparkle(pnt, at, p.size * 1.1, p.rot, c),
                _ => {
                    let (s, co) = p.rot.sin_cos();
                    let hx = vec2(co, s) * p.size * 0.75;
                    let hy = vec2(-s, co) * p.size * 0.35;
                    paint::quad(pnt, [at - hx - hy, at + hx - hy, at + hx + hy, at - hx + hy], [c; 4]);
                }
            }
        }
        resp
    }
}
