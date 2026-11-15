use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2};
use glowkit_ai::{Icon, VoiceOrb, VoiceState};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// What happened in a [`VoiceModeOverlay`] this frame.
pub struct VoiceModeOutput {
    /// Response of the host area.
    pub response: Response,
    /// The mute button was clicked.
    pub mute_clicked: bool,
    /// The end-call button was clicked (the overlay closes itself).
    pub ended: bool,
    /// Screen rect of the launcher (mic) the overlay grows from — place your mic button there.
    pub origin: Pos2,
}

/// A full-bleed voice conversation mode for an assistant: opening it grows a frosted backdrop out
/// of the mic button in a circular reveal; a living orb (glowkit-ai's voice orb) reacts to the
/// state and audio level, a shimmering status line reads "Listening", "Thinking" or "Speaking",
/// the live caption fades in word by word, and a control bar offers mute, captions and a red end
/// button. Closing reverses the reveal back into the button.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::VoiceModeOverlay;
/// use glowkit_ai::VoiceState;
/// let mut open = true;
/// VoiceModeOverlay::new(&mut open).state(VoiceState::Listening).level(0.4).caption("What's on my calendar").show(ui, |ui| { ui.label("chat"); });
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct VoiceModeOverlay<'a> {
    open: &'a mut bool,
    state: VoiceState,
    level: f32,
    caption: &'a str,
    muted: bool,
    height: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> VoiceModeOverlay<'a> {
    /// An overlay toggled by `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self { open, state: VoiceState::Listening, level: 0.0, caption: "", muted: false, height: 440.0, salt: None }
    }
    /// What the agent is doing.
    pub fn state(mut self, s: VoiceState) -> Self {
        self.state = s;
        self
    }
    /// Current audio level 0..1.
    pub fn level(mut self, l: f32) -> Self {
        self.level = l.clamp(0.0, 1.0);
        self
    }
    /// Live caption (the words revealed so far).
    pub fn caption(mut self, c: &'a str) -> Self {
        self.caption = c;
        self
    }
    /// Show the mic as muted.
    pub fn muted(mut self, m: bool) -> Self {
        self.muted = m;
        self
    }
    /// Height of the host area (default 440).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(260.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the host content (`host`) with the overlay on top when open.
    pub fn show(self, ui: &mut Ui, host: impl FnOnce(&mut Ui)) -> VoiceModeOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_voice");
        let w = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        let k = glowkit_core::anim::spring(ui, id.with("k"), if *self.open { 1.0 } else { 0.0 }, 120.0, 20.0)
            .clamp(0.0, 1.0);
        let time = glowkit_core::time(ui);
        let p = ui.painter();
        p.rect_filled(rect, cr(18.0), stage(&t));
        p.rect_stroke(rect, cr(18.0), hairline(&t), egui::StrokeKind::Inside);
        let mut child =
            ui.new_child(UiBuilder::new().max_rect(rect.shrink(18.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        host(&mut child);
        let origin = pos2(rect.max.x - 62.0, rect.max.y - 40.0);
        let mut out = VoiceModeOutput { response, mute_clicked: false, ended: false, origin };
        if k < 0.002 {
            return out;
        }
        glowkit_core::animate(ui);
        // circular reveal from the mic button
        let far = [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()]
            .iter()
            .map(|c| (*c - origin).length())
            .fold(0.0, f32::max);
        let rad = far * egui::emath::easing::cubic_in_out(k);
        let clip = rect.intersect(ui.clip_rect());
        let p = ui.painter().with_clip_rect(clip);
        let bg = if t.dark { mix(t.background, t.accent, 0.08) } else { mix(Color32::WHITE, t.accent, 0.035) };
        if rad < far * 0.98 {
            p.circle_filled(origin, rad, bg);
        } else {
            p.rect_filled(rect, cr(18.0), bg);
        }
        let ck = ((k - 0.75) / 0.25).clamp(0.0, 1.0);
        if ck <= 0.0 {
            return out;
        }
        // ambient glows
        paint::glow(&p, rect.center() - vec2(0.0, 30.0), rect.height() * 0.45, with_alpha(t.gradient[0], 0.10 * ck));
        paint::glow(&p, rect.center() + vec2(120.0, 40.0), rect.height() * 0.3, with_alpha(t.gradient[1], 0.07 * ck));
        // top bar
        bot_avatar(&p, &t, pos2(rect.min.x + 30.0, rect.min.y + 30.0), 11.0);
        p.text(
            pos2(rect.min.x + 48.0, rect.min.y + 30.0),
            Align2::LEFT_CENTER,
            "Voice mode",
            FontId::proportional(13.0),
            t.foreground.gamma_multiply(ck),
        );
        let secs = (time * 1.0) as u32 % 3600;
        mono(
            &p,
            pos2(rect.max.x - 24.0, rect.min.y + 30.0),
            Align2::RIGHT_CENTER,
            &format!("{:02}:{:02}", secs / 60, secs % 60),
            11.5,
            t.muted_foreground.gamma_multiply(ck),
        );
        // orb
        let orb_s = (rect.height() * 0.3).min(150.0);
        let orb_c = pos2(rect.center().x, rect.min.y + 64.0 + orb_s / 2.0);
        let orb_r = Rect::from_center_size(orb_c, Vec2::splat(orb_s));
        let mut oc = ui.new_child(UiBuilder::new().max_rect(orb_r));
        oc.set_clip_rect(clip);
        oc.multiply_opacity(ck);
        oc.add(
            VoiceOrb::new()
                .state(self.state)
                .amplitude(self.level)
                .size(orb_s * (0.85 + 0.15 * ck))
                .id_salt(id.with("orb")),
        );
        // status
        let status = match self.state {
            VoiceState::Idle => "Tap to talk",
            VoiceState::Listening => "Listening",
            VoiceState::Thinking => "Thinking",
            VoiceState::Speaking => "Speaking",
        };
        let sy = orb_r.max.y + 62.0;
        let g = p.layout_no_wrap(status.to_owned(), FontId::proportional(13.0), Color32::WHITE);
        let x0 = rect.center().x - g.size().x / 2.0;
        let sweep = (time * 0.7).fract() * 1.6 - 0.3;
        paint::galley_with(&p, pos2(x0, sy - g.size().y / 2.0), &g, |q| {
            let u = (q.x - x0) / g.size().x;
            let kk = (1.0 - ((u - sweep) * 4.0).abs()).clamp(0.0, 1.0);
            mix(t.muted_foreground, t.foreground, kk).gamma_multiply(ck)
        });
        // caption, newest words fade in
        if !self.caption.is_empty() {
            let words: Vec<&str> = self.caption.split_whitespace().collect();
            let n = words.len();
            let mut job = egui::text::LayoutJob::default();
            for (i, wd) in words.iter().enumerate() {
                let age = (n - i) as f32;
                let a = (age / 3.0).clamp(0.35, 1.0);
                job.append(
                    &format!("{wd} "),
                    0.0,
                    egui::text::TextFormat {
                        font_id: FontId::proportional(16.0),
                        color: t.foreground.gamma_multiply(a * ck),
                        ..Default::default()
                    },
                );
            }
            job.wrap.max_width = (w * 0.7).min(460.0);
            job.halign = egui::Align::Center;
            let g = p.layout_job(job);
            p.galley(pos2(rect.center().x, sy + 18.0), g, t.foreground);
        }
        // controls
        let cy = rect.max.y - 40.0;
        let specs = [(Icon::Mic, -64.0), (Icon::File, 0.0), (Icon::X, 64.0)];
        for (i, (ic, dx)) in specs.iter().enumerate() {
            let c = pos2(rect.center().x + dx, cy + (1.0 - ck) * 16.0);
            let r = Rect::from_center_size(c, Vec2::splat(46.0));
            let b = ui.interact(r, id.with(("c", i)), Sense::click());
            let hk = toggle(ui, id.with(("ch", i)), b.hovered());
            let p = ui.painter().with_clip_rect(clip);
            let (fill, fg) = if i == 2 {
                (mix(t.destructive, Color32::BLACK, 0.1 * hk), Color32::WHITE)
            } else if i == 0 && self.muted {
                (t.foreground, t.background)
            } else {
                (mix(surface(&t), well(&t), hk), t.foreground)
            };
            paint::soft_shadow(&p, r, 23.0, 10.0, vec2(0.0, 3.0), shadow(&t, 0.8 * ck));
            p.circle_filled(c, 23.0 * (1.0 + 0.04 * hk), fill.gamma_multiply(ck));
            if i != 2 {
                p.circle_stroke(c, 23.0, Stroke::new(1.0, hairline(&t).color.gamma_multiply(ck)));
            }
            if i == 1 {
                // captions glyph: "CC"
                p.text(c, Align2::CENTER_CENTER, "CC", FontId::proportional(12.0), fg.gamma_multiply(ck));
            } else {
                icon(&p, *ic, c, 18.0, fg.gamma_multiply(ck));
                if i == 0 && self.muted {
                    p.line_segment([c + vec2(-8.0, -8.0), c + vec2(8.0, 8.0)], Stroke::new(2.0, fg.gamma_multiply(ck)));
                }
            }
            if b.clicked() {
                match i {
                    0 => out.mute_clicked = true,
                    2 => {
                        out.ended = true;
                        *self.open = false;
                    }
                    _ => {}
                }
            }
        }
        out
    }
}
