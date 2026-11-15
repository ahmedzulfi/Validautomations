use super::common::*;
use egui::emath::TSTransform;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One starter card on a [`WelcomeScreen`].
#[derive(Clone, Debug, PartialEq)]
pub struct WelcomeSuggestion {
    /// Bold first line.
    pub title: String,
    /// Muted second line.
    pub detail: String,
    /// Icon in the tile.
    pub icon: Icon,
}

impl WelcomeSuggestion {
    /// A suggestion card.
    pub fn new(title: impl Into<String>, detail: impl Into<String>, icon: Icon) -> Self {
        Self { title: title.into(), detail: detail.into(), icon }
    }
}

/// The empty state of an assistant: a soft glowing mark, a large greeting and subtitle that rise
/// in, a 2-column grid of starter suggestions that stagger in and lift on hover, and a slot for the
/// composer underneath. Returns the clicked suggestion.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{WelcomeScreen, WelcomeSuggestion};
/// use glowkit_ai::Icon;
/// let s = [WelcomeSuggestion::new("Summarize", "the Q3 report", Icon::File)];
/// if let Some(i) = WelcomeScreen::new("How can I help today?").suggestions(&s).show(ui, |_ui| {}) { /* send s[i] */ }
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct WelcomeScreen<'a> {
    greeting: &'a str,
    subtitle: Option<&'a str>,
    suggestions: &'a [WelcomeSuggestion],
    width: Option<f32>,
    highlight: Option<usize>,
    salt: Option<egui::IdSalt>,
}

impl<'a> WelcomeScreen<'a> {
    /// A welcome screen with `greeting`.
    pub fn new(greeting: &'a str) -> Self {
        Self { greeting, subtitle: None, suggestions: &[], width: None, highlight: None, salt: None }
    }
    /// Line under the greeting.
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    /// Starter suggestions (shown two per row).
    pub fn suggestions(mut self, s: &'a [WelcomeSuggestion]) -> Self {
        self.suggestions = s;
        self
    }
    /// Width (default: available, at most 640).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Draw suggestion `i` as hovered (for autoplay).
    pub fn highlight(mut self, i: Option<usize>) -> Self {
        self.highlight = i;
        self
    }
    /// Stable id salt (also restarts the entrance animation when changed).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the screen; `composer` is laid out under the suggestions.
    pub fn show(self, ui: &mut Ui, composer: impl FnOnce(&mut Ui)) -> Option<usize> {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_welcome");
        let w = self.width.unwrap_or_else(|| ui.available_width().min(640.0));
        let e = since_change(ui, id, hash_str(self.greeting));
        let time = glowkit_core::time(ui);
        glowkit_core::animate(ui);
        let mut clicked = None;
        let (hr, _) = ui.allocate_exact_size(vec2(w, 120.0), Sense::hover());
        let p = ui.painter();
        let c = pos2(hr.center().x, hr.min.y + 26.0);
        let ka = appear_curve(e).min(1.0);
        paint::glow(p, c, 46.0 * ka, with_alpha(t.accent, if t.dark { 0.35 } else { 0.18 }));
        let r = 18.0 * (0.6 + 0.4 * ka);
        let pts = paint::circle_points(c, r, 40);
        let rot = time * 0.4;
        paint::fill_convex_fn(
            p,
            &pts,
            3,
            paint::conic_fn(c, rot, &[t.gradient[0], t.gradient[1], t.gradient[2], t.gradient[0]]),
        );
        paint::sparkle(p, c, r * 0.95, (time * 0.7).sin() * 0.2, Color32::WHITE);
        let k1 = ((e - 0.15) * 3.0).clamp(0.0, 1.0);
        text(
            p,
            pos2(hr.center().x, hr.min.y + 76.0 + (1.0 - k1) * 10.0),
            Align2::CENTER_CENTER,
            self.greeting,
            26.0,
            with_alpha(t.foreground, k1),
        );
        if let Some(s) = self.subtitle {
            let k2 = ((e - 0.3) * 3.0).clamp(0.0, 1.0);
            text(
                p,
                pos2(hr.center().x, hr.min.y + 106.0 + (1.0 - k2) * 8.0),
                Align2::CENTER_CENTER,
                s,
                13.5,
                with_alpha(t.muted_foreground, k2),
            );
        }
        ui.add_space(14.0);
        let cols = 2;
        let gap = 10.0;
        let cw = (w - gap) / cols as f32;
        for (row, chunk) in self.suggestions.chunks(cols).enumerate() {
            let (rr, _) = ui.allocate_exact_size(vec2(w, 62.0), Sense::hover());
            for (k, s) in chunk.iter().enumerate() {
                let i = row * cols + k;
                let r = Rect::from_min_size(pos2(rr.min.x + k as f32 * (cw + gap), rr.min.y), vec2(cw, 62.0));
                let resp = ui.interact(r, id.with(("s", i)), Sense::click());
                let hk = toggle(ui, id.with(("h", i)), resp.hovered() || self.highlight == Some(i));
                let ki = ((e - 0.45 - i as f32 * 0.08) * 4.0).clamp(0.0, 1.0);
                let ki = egui::emath::easing::cubic_out(ki);
                let r = r.translate(vec2(0.0, (1.0 - ki) * 12.0 - hk * 2.0));
                let p = ui.painter();
                paint::soft_shadow(
                    p,
                    r,
                    14.0,
                    8.0 + 10.0 * hk,
                    vec2(0.0, 2.0 + 3.0 * hk),
                    shadow(&t, (0.4 + 0.8 * hk) * ki),
                );
                p.rect_filled(r, cr(14.0), with_alpha(surface(&t), ki));
                p.rect_stroke(r, cr(14.0), Stroke::new(1.0, hairline(&t).color.gamma_multiply(ki)), StrokeKind::Inside);
                let ib = Rect::from_min_size(r.min + vec2(12.0, 14.0), Vec2::splat(34.0));
                p.rect_filled(ib, cr(10.0), with_alpha(mix(well(&t), t.accent, 0.12 * hk), ki));
                icon(p, s.icon, ib.center(), 16.0, with_alpha(mix(t.muted_foreground, t.accent, hk), ki));
                text_bold(
                    p,
                    pos2(ib.max.x + 12.0, r.min.y + 23.0),
                    Align2::LEFT_CENTER,
                    &s.title,
                    13.0,
                    with_alpha(t.foreground, ki),
                );
                text(
                    p,
                    pos2(ib.max.x + 12.0, r.min.y + 41.0),
                    Align2::LEFT_CENTER,
                    &s.detail,
                    12.0,
                    with_alpha(t.muted_foreground, ki),
                );
                icon(
                    p,
                    Icon::ArrowRight,
                    pos2(r.max.x - 18.0 - (1.0 - hk) * 4.0, r.center().y),
                    13.0,
                    with_alpha(t.foreground, hk * ki),
                );
                if resp.clicked() {
                    clicked = Some(i);
                }
            }
            ui.add_space(gap - 8.0);
        }
        ui.add_space(12.0);
        let kc = ((e - 0.8) * 3.0).clamp(0.0, 1.0);
        ui.scope(|ui| {
            ui.multiply_opacity(kc);
            ui.set_max_width(w);
            composer(ui);
        });
        clicked
    }
}

/// A docked assistant panel in the style of copilot sidebars: your app on the left, and a panel
/// that slides in from the right edge on a spring, pushing the app over (or overlaying it). The
/// panel has a header with the assistant mark, a title and a close button, and hosts whatever
/// chat UI you put in it. A round launcher button sits at the bottom-right while it is closed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = true;
/// glowkit_agents::CopilotSidebar::new(&mut open).show(ui, |ui| { ui.label("app"); }, |ui| { ui.label("chat"); });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct CopilotSidebar<'a> {
    open: &'a mut bool,
    title: &'a str,
    panel_width: f32,
    height: f32,
    push: bool,
    salt: Option<egui::IdSalt>,
}

impl<'a> CopilotSidebar<'a> {
    /// A sidebar toggled by `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self { open, title: "Assistant", panel_width: 320.0, height: 420.0, push: true, salt: None }
    }
    /// Panel title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Panel width (default 320).
    pub fn panel_width(mut self, w: f32) -> Self {
        self.panel_width = w.max(220.0);
        self
    }
    /// Total height (default 420).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Push the app aside (default) or overlay it.
    pub fn push_content(mut self, on: bool) -> Self {
        self.push = on;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the app (`app`) and the panel (`panel`). Returns the launcher button response.
    pub fn show(self, ui: &mut Ui, app: impl FnOnce(&mut Ui), panel: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_sidebar");
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        let k = glowkit_core::anim::spring(ui, id.with("k"), if *self.open { 1.0 } else { 0.0 }, 210.0, 23.0);
        let pw = self.panel_width.min(w * 0.7);
        let shift = if self.push { pw * k.clamp(0.0, 1.05) } else { 0.0 };
        let p = ui.painter();
        p.rect_filled(rect, cr(16.0), stage(&t));
        p.rect_stroke(rect, cr(16.0), hairline(&t), StrokeKind::Inside);
        let app_r = Rect::from_min_max(rect.min, pos2(rect.max.x - shift, rect.max.y));
        let mut child = ui
            .new_child(UiBuilder::new().max_rect(app_r.shrink(16.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(app_r.intersect(ui.clip_rect()));
        app(&mut child);
        // launcher
        let lk = 1.0 - k.clamp(0.0, 1.0);
        let lc = pos2(rect.max.x - 34.0, rect.max.y - 34.0);
        let lr = Rect::from_center_size(lc, Vec2::splat(44.0));
        let lresp = ui.interact(lr, id.with("launch"), if lk > 0.5 { Sense::click() } else { Sense::hover() });
        if lk > 0.01 {
            let hk = toggle(ui, id.with("lh"), lresp.hovered());
            let p = ui.painter();
            let rad = 22.0 * lk * (1.0 + 0.06 * hk);
            paint::soft_shadow(
                p,
                Rect::from_center_size(lc, Vec2::splat(rad * 2.0)),
                rad,
                14.0,
                vec2(0.0, 4.0),
                shadow(&t, 1.2 * lk),
            );
            p.circle_filled(lc, rad, t.primary);
            paint::sparkle(p, lc, rad * 0.75, 0.0, t.primary_foreground);
        }
        if lresp.clicked() {
            *self.open = true;
        }
        // panel
        if k > 0.002 {
            let x0 = rect.max.x - pw * k.min(1.02);
            let pr = Rect::from_min_size(pos2(x0, rect.min.y), vec2(pw, rect.height()));
            let clip = rect.intersect(ui.clip_rect());
            let p = ui.painter().with_clip_rect(clip);
            if !self.push {
                paint::soft_shadow(&p, pr, 16.0, 30.0, vec2(-6.0, 0.0), shadow(&t, 1.4 * k));
            }
            p.rect_filled(pr, egui::CornerRadius { nw: 0, sw: 0, ne: 16, se: 16 }, surface(&t));
            p.line_segment([pr.left_top(), pr.left_bottom()], hairline(&t));
            let hr = Rect::from_min_size(pr.min, vec2(pw, 52.0));
            bot_avatar(&p, &t, pos2(hr.min.x + 26.0, hr.center().y), 12.0);
            text_bold(&p, pos2(hr.min.x + 46.0, hr.center().y), Align2::LEFT_CENTER, self.title, 13.5, t.foreground);
            p.line_segment([pos2(pr.min.x, hr.max.y), pos2(pr.max.x, hr.max.y)], hairline(&t));
            let xr = Rect::from_center_size(pos2(hr.max.x - 26.0, hr.center().y), Vec2::splat(28.0));
            let mut body = ui.new_child(
                UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(pr.min.x + 12.0, hr.max.y + 10.0), pr.max - vec2(12.0, 12.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            body.set_clip_rect(Rect::from_min_max(pos2(pr.min.x, hr.max.y), pr.max).intersect(clip));
            body.multiply_opacity(((k - 0.3) / 0.7).clamp(0.0, 1.0));
            panel(&mut body);
            if icon_button(ui, id.with("close"), xr, Icon::X, false).clicked() {
                *self.open = false;
            }
        }
        lresp
    }
}

/// A floating assistant in the style of copilot popups: a round launcher at the bottom-right of
/// your app; clicking it springs a chat window out of the button (scaling from the corner) while
/// the launcher's sparkle turns into a close cross. Click again (or the X) to tuck it away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = false;
/// glowkit_agents::CopilotPopup::new(&mut open).show(ui, |ui| { ui.label("app"); }, |ui| { ui.label("chat"); });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct CopilotPopup<'a> {
    open: &'a mut bool,
    title: &'a str,
    window: Vec2,
    height: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> CopilotPopup<'a> {
    /// A popup toggled by `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self { open, title: "Assistant", window: vec2(330.0, 380.0), height: 460.0, salt: None }
    }
    /// Window title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Chat window size (default 330 x 380).
    pub fn window_size(mut self, s: Vec2) -> Self {
        self.window = s;
        self
    }
    /// Height of the host area (default 460).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the host app and, when open, the chat window. Returns the launcher response.
    pub fn show(self, ui: &mut Ui, app: impl FnOnce(&mut Ui), chat: impl FnOnce(&mut Ui)) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_popup");
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        let k = glowkit_core::anim::spring(ui, id.with("k"), if *self.open { 1.0 } else { 0.0 }, 260.0, 21.0);
        let p = ui.painter();
        p.rect_filled(rect, cr(16.0), stage(&t));
        p.rect_stroke(rect, cr(16.0), hairline(&t), StrokeKind::Inside);
        let mut child =
            ui.new_child(UiBuilder::new().max_rect(rect.shrink(16.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        app(&mut child);
        let lc = pos2(rect.max.x - 36.0, rect.max.y - 36.0);
        // window
        if k > 0.01 {
            let ws = vec2(self.window.x.min(w - 32.0), self.window.y.min(self.height - 84.0));
            let wr = Rect::from_min_size(pos2(rect.max.x - 16.0 - ws.x, lc.y - 34.0 - ws.y), ws);
            let pivot = pos2(wr.max.x - 10.0, wr.max.y);
            let s = 0.2 + 0.8 * k;
            let tr = TSTransform::from_translation(pivot.to_vec2())
                * TSTransform::from_scaling(s)
                * TSTransform::from_translation(-pivot.to_vec2());
            let cut = Rect::from_min_max(pos2(wr.min.x, wr.min.y), pos2(wr.max.x, wr.max.y));
            ui.with_visual_transform(tr, |ui| {
                let p = ui.painter();
                let a = k.clamp(0.0, 1.0);
                paint::soft_shadow(p, cut, 18.0, 30.0, vec2(0.0, 10.0), shadow(&t, 1.6 * a));
                p.rect_filled(cut, cr(18.0), surface(&t).gamma_multiply(a));
                p.rect_stroke(
                    cut,
                    cr(18.0),
                    Stroke::new(1.0, hairline(&t).color.gamma_multiply(a)),
                    StrokeKind::Inside,
                );
                let hr = Rect::from_min_size(cut.min, vec2(cut.width(), 50.0));
                bot_avatar(p, &t, pos2(hr.min.x + 24.0, hr.center().y), 12.0);
                text_bold(
                    p,
                    pos2(hr.min.x + 44.0, hr.center().y - 6.0),
                    Align2::LEFT_CENTER,
                    self.title,
                    13.5,
                    t.foreground.gamma_multiply(a),
                );
                p.circle_filled(pos2(hr.min.x + 47.0, hr.center().y + 9.0), 3.0, t.success.gamma_multiply(a));
                text(
                    p,
                    pos2(hr.min.x + 54.0, hr.center().y + 9.0),
                    Align2::LEFT_CENTER,
                    "Online",
                    11.0,
                    t.muted_foreground.gamma_multiply(a),
                );
                p.line_segment(
                    [pos2(cut.min.x, hr.max.y), pos2(cut.max.x, hr.max.y)],
                    Stroke::new(1.0, hairline(&t).color.gamma_multiply(a)),
                );
                let mut body = ui.new_child(
                    UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            pos2(cut.min.x + 12.0, hr.max.y + 10.0),
                            cut.max - vec2(12.0, 12.0),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                body.set_clip_rect(Rect::from_min_max(pos2(cut.min.x, hr.max.y), cut.max));
                body.multiply_opacity(((k - 0.35) / 0.65).clamp(0.0, 1.0));
                chat(&mut body);
            });
        }
        // launcher morphs sparkle -> cross
        let lr = Rect::from_center_size(lc, Vec2::splat(48.0));
        let resp = ui.interact(lr, id.with("launch"), Sense::click());
        let hk = toggle(ui, id.with("lh"), resp.hovered());
        let p = ui.painter();
        let rad = 24.0 * (1.0 + 0.06 * hk);
        paint::soft_shadow(
            p,
            Rect::from_center_size(lc, Vec2::splat(rad * 2.0)),
            rad,
            16.0,
            vec2(0.0, 5.0),
            shadow(&t, 1.3),
        );
        p.circle_filled(lc, rad, t.primary);
        let kk = k.clamp(0.0, 1.0);
        if kk < 0.98 {
            paint::sparkle(p, lc, 17.0 * (1.0 - kk), kk * 1.5, t.primary_foreground.gamma_multiply(1.0 - kk));
        }
        if kk > 0.02 {
            let a = kk * std::f32::consts::FRAC_PI_4 + (1.0 - kk) * 1.2;
            let d = vec2(a.cos(), a.sin()) * 7.5;
            let n = vec2(-d.y, d.x);
            let c = t.primary_foreground.gamma_multiply(kk);
            p.line_segment([lc - d, lc + d], Stroke::new(2.0, c));
            p.line_segment([lc - n, lc + n], Stroke::new(2.0, c));
        }
        if resp.clicked() {
            *self.open = !*self.open;
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}
