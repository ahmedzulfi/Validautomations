use super::util::{bold_text, idle_time};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_core::{
    anim::{ease, smoothstep},
    color::with_alpha,
    hex, mix, paint,
    rng::hash01,
    Easing, Theme,
};

/// The default Gemini-like palette.
const GEMINI: [Color32; 5] = [hex("#FFB7C5"), hex("#FFDDB7"), hex("#B1C5FF"), hex("#4FABFF"), hex("#076EFF")];

/// Five flowing colored curves that draw themselves in, sweeping from the left edge into a
/// central pill button and fanning out to the right. Bind `progress` to scroll, or let it loop.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::GeminiEffect::new().title("Build with glowkit").button("glowkit.rs"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct GeminiEffect {
    progress: Option<f32>,
    size: Option<Vec2>,
    title: String,
    subtitle: String,
    button: String,
    colors: [Color32; 5],
}

impl Default for GeminiEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiEffect {
    /// The effect with default palette, looping on its own.
    pub fn new() -> Self {
        Self {
            progress: None,
            size: None,
            title: "Build with glowkit".into(),
            subtitle: "Scroll this component and see the bottom SVG come to life.".into(),
            button: "glowkit.rs".into(),
            colors: GEMINI,
        }
    }
    /// Drive the drawing yourself (0..1), e.g. from a scroll offset. Disables the loop.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = Some(p.clamp(0.0, 1.0));
        self
    }
    /// Size (defaults to available width x 380).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some(vec2(w, h));
        self
    }
    /// Heading above the curves.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Sub heading.
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = s.into();
        self
    }
    /// Label of the central pill.
    pub fn button(mut self, s: impl Into<String>) -> Self {
        self.button = s.into();
        self
    }
    /// The five curve colors.
    pub fn colors(mut self, c: [Color32; 5]) -> Self {
        self.colors = c;
        self
    }
}

impl Widget for GeminiEffect {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = glowkit_core::desired_size(ui, self.size, 380.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let prog = match self.progress {
            Some(p) => p,
            None => {
                glowkit_core::animate(ui);
                let e = glowkit_core::time(ui) % 6.0;
                ease(Easing::InOutCubic, (e / 3.6).min(1.0)) * (1.0 - smoothstep(5.4, 6.0, e))
            }
        };
        let p = ui.painter_at(rect);
        bold_text(&p, pos2(rect.center().x, rect.min.y + 36.0), Align2::CENTER_CENTER, &self.title, FontId::proportional(32.0), t.foreground);
        p.text(pos2(rect.center().x, rect.min.y + 72.0), Align2::CENTER_CENTER, &self.subtitle, FontId::proportional(14.0), t.muted_foreground);
        let hub = pos2(rect.center().x, rect.min.y + rect.height() * 0.68);
        let time = glowkit_core::time(ui);
        for (i, col) in self.colors.iter().enumerate() {
            let fi = i as f32;
            let y0 = rect.min.y + rect.height() * (0.30 + 0.09 * fi);
            let y1 = rect.min.y + rect.height() * (0.95 - 0.06 * fi);
            let a = pos2(rect.min.x - 10.0, y0);
            let c1 = pos2(rect.min.x + rect.width() * (0.18 + 0.03 * fi), y0 + (fi - 2.0) * 22.0 + 40.0);
            let c2 = pos2(rect.min.x + rect.width() * (0.32 - 0.02 * fi), hub.y + (fi - 2.0) * 18.0);
            let mut pts = paint::cubic_bezier(a, c1, c2, hub + vec2(-30.0, (fi - 2.0) * 3.0), 70);
            let d1 = pos2(rect.max.x - rect.width() * (0.32 - 0.02 * fi), hub.y + (fi - 2.0) * 18.0);
            let d2 = pos2(rect.max.x - rect.width() * (0.18 + 0.03 * fi), y1 - 30.0);
            pts.extend(paint::cubic_bezier(hub + vec2(30.0, (fi - 2.0) * 3.0), d1, d2, pos2(rect.max.x + 10.0, y1 - 60.0 - fi * 10.0), 70));
            // gentle living wave
            for (k, q) in pts.iter_mut().enumerate() {
                let u = k as f32 / 140.0;
                q.y += (u * 9.0 + time * 1.2 + fi).sin() * 4.0 * (1.0 - (u * 2.0 - 1.0).abs() * 0.5);
            }
            // faint full track
            paint::gradient_polyline(&p, &pts, false, 1.5, |_| with_alpha(*col, if t.dark { 0.10 } else { 0.18 }));
            let n = ((pts.len() as f32) * prog) as usize;
            if n >= 2 {
                let drawn = &pts[..n];
                paint::gradient_polyline(&p, drawn, false, 9.0, |u| with_alpha(*col, 0.10 * u.powf(0.5)));
                paint::gradient_polyline(&p, drawn, false, 2.2, |u| with_alpha(*col, 0.4 + 0.6 * u));
                let head = drawn[n - 1];
                paint::glow(&p, head, 12.0, with_alpha(*col, 0.7));
            }
        }
        // the hub button
        let g = p.layout_no_wrap(self.button.clone(), FontId::proportional(15.0), Color32::WHITE);
        let pill = Rect::from_center_size(hub, g.size() + vec2(36.0, 18.0));
        paint::glow(&p, hub, 90.0, with_alpha(self.colors[3], 0.25 * prog));
        paint::soft_shadow(&p, pill, pill.height() / 2.0, 18.0, vec2(0.0, 4.0), Color32::from_black_alpha(90));
        let (bg, fg) = if t.dark { (Color32::WHITE, hex("#09090B")) } else { (hex("#09090B"), Color32::WHITE) };
        p.rect_filled(pill, pill.height() / 2.0, bg);
        p.galley(pill.center() - g.size() / 2.0 + vec2(0.4, 0.0), g.clone(), fg);
        p.galley(pill.center() - g.size() / 2.0, g, fg);
        resp
    }
}

/// Text where one phrase gets a rectangle drawn around it while a pointer cursor glides to the
/// rectangle's corner, as if someone just highlighted it. Loops.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::PointerHighlight::new("The best way to grow is to ", "collaborate", "."));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PointerHighlight {
    before: String,
    phrase: String,
    after: String,
    size: f32,
    color: Option<Color32>,
    looping: bool,
}

impl PointerHighlight {
    /// `before`, the highlighted `phrase`, and `after` (laid out on one line).
    pub fn new(before: impl Into<String>, phrase: impl Into<String>, after: impl Into<String>) -> Self {
        Self { before: before.into(), phrase: phrase.into(), after: after.into(), size: 34.0, color: None, looping: true }
    }
    /// Font size.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Rectangle / pointer color (defaults to the accent).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Replay forever (otherwise plays once when it appears).
    pub fn looping(mut self, b: bool) -> Self {
        self.looping = b;
        self
    }
}

fn cursor_shape(tip: Pos2, s: f32) -> Vec<Pos2> {
    [(0.0, 0.0), (0.0, 1.0), (0.27, 0.76), (0.45, 1.12), (0.58, 1.06), (0.41, 0.70), (0.75, 0.70)].iter().map(|(x, y)| tip + vec2(*x, *y) * s).collect()
}

/// Paint a mouse-pointer arrow with its tip at `tip`.
pub(crate) fn paint_cursor(p: &egui::Painter, tip: Pos2, s: f32, fill: Color32, outline: Color32) {
    let pts = cursor_shape(tip, s);
    // the arrow is concave; fan from a point inside the arrow head
    super::util::fill_star_shaped(p, tip + vec2(0.25, 0.55) * s, &pts, fill);
    p.add(Shape::closed_line(pts, Stroke::new(1.2, outline)));
}

impl Widget for PointerHighlight {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let p0 = ui.painter().clone();
        let gb = p0.layout_no_wrap(self.before.clone(), font.clone(), t.foreground);
        let gp = p0.layout_no_wrap(self.phrase.clone(), font.clone(), t.foreground);
        let ga = p0.layout_no_wrap(self.after.clone(), font.clone(), t.foreground);
        let w = gb.size().x + gp.size().x + ga.size().x;
        let h = gp.size().y;
        let (rect, resp) = ui.allocate_exact_size(vec2(w + 30.0, h + 30.0), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let trig = if self.looping { glowkit_core::Trigger::Loop { pause_ms: 600 } } else { glowkit_core::Trigger::OnAppear };
        let e = glowkit_core::Clock::triggered(ui, resp.id, trig, 4.4, resp.hovered(), false);
        let o = pos2(rect.min.x + 15.0, rect.min.y + 15.0);
        let col = self.color.unwrap_or(t.accent);
        let p = ui.painter_at(rect.expand(30.0));
        let fg = t.foreground;
        p.galley(o + vec2(0.4, 0.0), gb.clone(), fg);
        p.galley(o, gb.clone(), fg);
        let px = o.x + gb.size().x;
        let hr = Rect::from_min_size(pos2(px, o.y), gp.size()).expand2(vec2(6.0, 2.0));
        let grow = ease(Easing::InOutCubic, smoothstep(0.3, 1.3, e));
        let fade = 1.0 - smoothstep(3.8, 4.3, e);
        let box_r = Rect::from_min_size(hr.min, vec2(hr.width() * grow, hr.height()));
        if grow > 0.0 {
            p.rect_filled(box_r, 3.0, with_alpha(col, 0.10 * fade));
            p.rect_stroke(box_r, 3.0, Stroke::new(1.5, with_alpha(col, fade)), StrokeKind::Middle);
        }
        p.galley(pos2(px + 0.4, o.y), gp.clone(), fg);
        p.galley(pos2(px, o.y), gp.clone(), fg);
        p.galley(pos2(px + gp.size().x + 0.4, o.y), ga.clone(), fg);
        p.galley(pos2(px + gp.size().x, o.y), ga.clone(), fg);
        // pointer: enters from below right, follows the growing edge, then rests at the corner
        let appear = smoothstep(0.0, 0.3, e) * fade;
        if appear > 0.0 {
            let start = box_r.right_bottom() + vec2(40.0, 34.0);
            let k = smoothstep(0.0, 0.3, e);
            let corner = box_r.right_bottom() + vec2(2.0, 2.0);
            let bob = (glowkit_core::time(ui) * 3.0).sin() * 1.5 * smoothstep(1.3, 1.6, e);
            let tip = start.lerp(corner, k) + vec2(bob, bob * 0.5);
            paint_cursor(&p, tip, 18.0, with_alpha(col, appear), with_alpha(Color32::WHITE, appear));
        }
        resp
    }
}

/// Text in a box that, on hover, turns into hyperspace: streaking light beams, twinkling
/// sparkles, glowing corner dots, and jittering text. Toggles on a timer when idle.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_blocks::Cover::new("warp speed").size(40.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Cover {
    text: String,
    size: f32,
    beam: Option<Color32>,
    autoplay: bool,
}

impl Cover {
    /// Covered `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), size: 36.0, beam: None, autoplay: true }
    }
    /// Font size.
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Beam / sparkle color (defaults to a bright sky blue).
    pub fn beam_color(mut self, c: Color32) -> Self {
        self.beam = Some(c);
        self
    }
    /// Toggle the effect on a timer when idle.
    pub fn autoplay(mut self, b: bool) -> Self {
        self.autoplay = b;
        self
    }
}

impl Widget for Cover {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(self.size);
        let g = ui.painter().layout_no_wrap(self.text.clone(), font, Color32::WHITE);
        let pad = vec2(self.size * 0.45, self.size * 0.22);
        let (rect, resp) = ui.allocate_exact_size(g.size() + pad * 2.0, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let idle = idle_time(ui, resp.id, resp.hovered());
        let time = glowkit_core::time(ui);
        let target = if resp.hovered() || (self.autoplay && idle > 1.0 && (time / 3.5).fract() < 0.6) { 1.0 } else { 0.0 };
        let k = glowkit_core::anim::follow(ui, resp.id.with("k"), target, 6.0);
        glowkit_core::animate(ui);
        let beam = self.beam.unwrap_or(hex("#38BDF8"));
        let p = ui.painter_at(rect.expand(16.0));
        let box_bg = if t.dark { hex("#111114") } else { hex("#F4F4F5") };
        let hover_bg = hex("#0A0A0F");
        let bg = box_bg.lerp_to_gamma(hover_bg, k);
        p.rect_filled(rect, 4.0, bg);
        p.rect_stroke(rect, 4.0, Stroke::new(1.0, t.border.lerp_to_gamma(with_alpha(beam, 0.5), k)), StrokeKind::Inside);
        let clip = p.with_clip_rect(rect.intersect(p.clip_rect()));
        // beams
        let beams = 14;
        for i in 0..beams as u64 {
            let y = rect.min.y + rect.height() * (0.08 + 0.84 * hash01(i * 11 + 3));
            let len = rect.width() * (0.25 + 0.35 * hash01(i * 7 + 1));
            let speed = 0.6 + 0.9 * hash01(i * 5 + 9);
            let span = rect.width() + len;
            let head = rect.min.x + ((time * speed * (1.0 + 2.0 * k) + hash01(i * 3)) % 1.0) * span;
            let a = (0.22 + 0.78 * k) * (0.4 + 0.6 * hash01(i * 13));
            let line: Vec<Pos2> = (0..=12).map(|j| pos2(head - len + len * j as f32 / 12.0, y)).collect();
            let c = if i % 3 == 0 { hex("#A78BFA") } else { beam };
            paint::gradient_polyline(&clip, &line, false, 1.2, |u| with_alpha(c, a * u * u));
        }
        // sparkles
        if k > 0.01 {
            for i in 0..18u64 {
                let sp = pos2(rect.min.x + rect.width() * hash01(i * 17 + 2), rect.min.y + rect.height() * hash01(i * 19 + 4));
                let tw = ((time * (1.5 + hash01(i)) + hash01(i * 23) * 6.0).sin() * 0.5 + 0.5).powf(3.0);
                paint::sparkle(&clip, sp, 2.0 + 3.0 * tw, 0.0, with_alpha(Color32::WHITE, k * tw));
            }
        }
        // corner dots
        for cpt in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
            p.circle_filled(cpt, 2.5, with_alpha(beam, 0.35 + 0.65 * k));
            if k > 0.0 {
                paint::glow(&p, cpt, 9.0, with_alpha(beam, 0.5 * k));
            }
        }
        // text with jitter
        let tick = (time * 30.0) as u64;
        let j = vec2(hash01(tick * 3) - 0.5, hash01(tick * 7 + 1) - 0.5) * 2.6 * k;
        let base_c = t.foreground.lerp_to_gamma(Color32::WHITE, k);
        let tp = rect.min + pad + j;
        p.galley(tp + vec2(0.5, 0.0), g.clone(), base_c);
        p.galley(tp, g.clone(), base_c);
        if k > 0.05 {
            // chromatic ghost
            p.galley(tp + vec2(-1.5 * k, 0.0), g.clone(), with_alpha(mix(beam, Color32::WHITE, 0.2), 0.25 * k));
        }
        resp
    }
}
