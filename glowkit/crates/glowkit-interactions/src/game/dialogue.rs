//! Dialogue box with a painted talking portrait, typewriter text and choices.

use super::common::*;
use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::paint;
use std::f32::consts::{PI, TAU};

/// A painted character bust used by [`GameDialogueBox`] and [`super::GameCharacterSelect`]:
/// head, hair or headgear, shoulders in the character's outfit, blinking eyes and a mouth that
/// moves while talking. Use the presets or set the colors yourself.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(120.0, 120.0));
/// glowkit_interactions::GamePortrait::mage().paint(ui.painter(), rect, 0.0, false);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamePortrait {
    /// Skin tone.
    pub skin: Color32,
    /// Hair / hood color.
    pub hair: Color32,
    /// Outfit color.
    pub outfit: Color32,
    /// Trim / accent color (collar, visor, eyes glow).
    pub trim: Color32,
    /// Background gradient (inner, outer).
    pub bg: [Color32; 2],
    /// 0 short hair, 1 long hair, 2 hood, 3 helmet with visor.
    pub headgear: u8,
    /// Draw a beard.
    pub beard: bool,
}

impl GamePortrait {
    /// A sword-and-board hero.
    pub fn hero() -> Self {
        Self {
            skin: hex("#E8B48F"),
            hair: hex("#6B3A1E"),
            outfit: hex("#3D6FB6"),
            trim: hex("#E8B64A"),
            bg: [hex("#4C7BD9"), hex("#1A2752")],
            headgear: 0,
            beard: false,
        }
    }
    /// A hooded mage.
    pub fn mage() -> Self {
        Self {
            skin: hex("#D9A27E"),
            hair: hex("#5B2A86"),
            outfit: hex("#3B1F5C"),
            trim: hex("#B97CFF"),
            bg: [hex("#9D5CFF"), hex("#251038")],
            headgear: 2,
            beard: true,
        }
    }
    /// A long-haired ranger / rogue.
    pub fn rogue() -> Self {
        Self {
            skin: hex("#C68A62"),
            hair: hex("#1E1A1A"),
            outfit: hex("#2F5A3A"),
            trim: hex("#7EE08A"),
            bg: [hex("#3FAE6A"), hex("#0F2A1A")],
            headgear: 1,
            beard: false,
        }
    }
    /// A helmeted soldier / android.
    pub fn trooper() -> Self {
        Self {
            skin: hex("#BFC7D2"),
            hair: hex("#5D6B7E"),
            outfit: hex("#36404F"),
            trim: hex("#3FE0FF"),
            bg: [hex("#2BB5D8"), hex("#0A1A2A")],
            headgear: 3,
            beard: false,
        }
    }
    /// A grizzled blacksmith / quest giver.
    pub fn elder() -> Self {
        Self {
            skin: hex("#E2A983"),
            hair: hex("#D8D2C8"),
            outfit: hex("#7A4A28"),
            trim: hex("#E8B64A"),
            bg: [hex("#E0873A"), hex("#3A1A0A")],
            headgear: 0,
            beard: true,
        }
    }

    /// Paint into `rect` (best square). `time` drives blinking; `talking` animates the mouth.
    pub fn paint(&self, painter: &Painter, rect: Rect, time: f32, talking: bool) {
        let s = rect.width().min(rect.height());
        let c = rect.center();
        let u = |x: f32, y: f32| c + vec2(x, y) * s;
        let bg_pts = paint::rounded_rect_points(rect, s * 0.08, 4);
        paint::fill_convex_fn(painter, &bg_pts, 4, paint::radial_fn(u(0.0, -0.1), s * 0.75, &self.bg));
        // shoulders
        let sh: Vec<Pos2> = (0..=20)
            .map(|i| {
                let a = PI + PI * i as f32 / 20.0;
                u(a.cos() * 0.46, 0.5 + a.sin() * 0.26)
            })
            .collect();
        let mut sh2 = sh.clone();
        sh2.push(u(0.46, 0.5));
        sh2.push(u(-0.46, 0.5));
        let clip_bottom = rect.max.y;
        let sh2: Vec<Pos2> =
            sh2.into_iter().map(|p| pos2(p.x.clamp(rect.min.x, rect.max.x), p.y.min(clip_bottom))).collect();
        painter.add(Shape::convex_polygon(sh2, self.outfit, Stroke::NONE));
        painter.add(Shape::convex_polygon(
            vec![u(-0.12, 0.25), u(0.12, 0.25), u(0.0, 0.42)],
            shade(self.outfit, -0.3),
            Stroke::NONE,
        ));
        painter.add(Shape::line(vec![u(-0.2, 0.27), u(0.0, 0.42), u(0.2, 0.27)], Stroke::new(s * 0.025, self.trim)));
        // long hair behind
        if self.headgear == 1 {
            painter.add(Shape::convex_polygon(
                vec![u(-0.22, -0.15), u(0.22, -0.15), u(0.26, 0.3), u(-0.26, 0.3)],
                self.hair,
                Stroke::NONE,
            ));
        }
        if self.headgear == 2 {
            let hood: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = PI * 0.85 + PI * 1.3 * i as f32 / 24.0;
                    u(a.cos() * 0.27, -0.06 + a.sin() * 0.3)
                })
                .collect();
            let mut hp = hood;
            hp.push(u(0.3, 0.3));
            hp.push(u(-0.3, 0.3));
            painter.add(Shape::convex_polygon(hp, self.hair, Stroke::NONE));
        }
        // neck + head
        painter.rect_filled(Rect::from_min_max(u(-0.07, 0.05), u(0.07, 0.28)), 0.0, shade(self.skin, -0.12));
        let head: Vec<Pos2> = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * TAU;
                let squash = if a.sin() > 0.0 { 1.12 } else { 1.0 };
                u(a.cos() * 0.17, -0.06 + a.sin() * 0.21 * squash)
            })
            .collect();
        painter.add(Shape::convex_polygon(head, self.skin, Stroke::NONE));
        // ears
        painter.circle_filled(u(-0.17, -0.04), s * 0.035, shade(self.skin, -0.08));
        painter.circle_filled(u(0.17, -0.04), s * 0.035, shade(self.skin, -0.08));
        // hair / headgear
        match self.headgear {
            0 | 1 => {
                let cap: Vec<Pos2> = (0..=16)
                    .map(|i| {
                        let a = PI + PI * i as f32 / 16.0;
                        u(a.cos() * 0.185, -0.1 + a.sin() * 0.2)
                    })
                    .collect();
                let mut cp = cap;
                cp.push(u(0.16, -0.04));
                cp.push(u(-0.16, -0.04));
                painter.add(Shape::convex_polygon(cp, self.hair, Stroke::NONE));
                painter.add(Shape::convex_polygon(
                    vec![u(-0.12, -0.22), u(0.1, -0.24), u(0.02, -0.1)],
                    shade(self.hair, 0.12),
                    Stroke::NONE,
                ));
            }
            2 => {
                painter.add(Shape::convex_polygon(
                    vec![u(-0.2, -0.2), u(0.0, -0.33), u(0.2, -0.2), u(0.15, -0.12), u(-0.15, -0.12)],
                    self.hair,
                    Stroke::NONE,
                ));
                painter.add(Shape::line(
                    vec![u(-0.21, 0.1), u(-0.2, -0.18), u(0.0, -0.31), u(0.2, -0.18), u(0.21, 0.1)],
                    Stroke::new(s * 0.02, self.trim),
                ));
            }
            _ => {
                let dome: Vec<Pos2> = (0..=20)
                    .map(|i| {
                        let a = PI * 0.92 + PI * 1.16 * i as f32 / 20.0;
                        u(a.cos() * 0.2, -0.05 + a.sin() * 0.25)
                    })
                    .collect();
                let mut dp = dome;
                dp.push(u(0.16, 0.12));
                dp.push(u(-0.16, 0.12));
                painter.add(Shape::convex_polygon(dp, self.hair, Stroke::NONE));
                let visor = Rect::from_min_max(u(-0.16, -0.08), u(0.16, 0.0));
                painter.rect_filled(visor, s * 0.02, hex("#05080D"));
                let blink = 0.6 + 0.4 * (time * 3.0).sin().abs();
                paint::glow_ellipse(
                    painter,
                    visor.center(),
                    vec2(visor.width() * 0.6, visor.height()),
                    with_alpha(self.trim, 0.6 * blink),
                );
                painter.line_segment(
                    [visor.left_center() + vec2(s * 0.02, 0.0), visor.right_center() - vec2(s * 0.02, 0.0)],
                    Stroke::new(s * 0.015, mix(self.trim, Color32::WHITE, 0.4)),
                );
                painter.line_segment([u(-0.2, -0.14), u(0.2, -0.14)], Stroke::new(s * 0.012, shade(self.hair, 0.2)));
                if talking {
                    let k = (time * 18.0).sin().abs();
                    for j in 0..5 {
                        let x = -0.08 + j as f32 * 0.04;
                        let h = 0.01 + 0.03 * k * (1.0 - (j as f32 - 2.0).abs() / 3.0);
                        painter.line_segment([u(x, 0.07 - h), u(x, 0.07 + h)], Stroke::new(s * 0.012, self.trim));
                    }
                }
                return;
            }
        }
        // face
        let blink_phase = (time / 3.7).fract();
        let closed = blink_phase > 0.96;
        for sx in [-1.0, 1.0] {
            let e = u(sx * 0.07, -0.03);
            if closed {
                painter.line_segment(
                    [e - vec2(s * 0.025, 0.0), e + vec2(s * 0.025, 0.0)],
                    Stroke::new(s * 0.012, hex("#2A1A12")),
                );
            } else {
                painter.circle_filled(e, s * 0.022, hex("#1E1410"));
                painter.circle_filled(e + vec2(-s * 0.007, -s * 0.008), s * 0.007, Color32::WHITE);
            }
            painter.line_segment(
                [u(sx * 0.1, -0.08), u(sx * 0.04, -0.09)],
                Stroke::new(s * 0.014, shade(self.hair, -0.1)),
            );
        }
        if self.beard {
            painter.add(Shape::convex_polygon(
                vec![u(-0.16, 0.0), u(0.16, 0.0), u(0.12, 0.13), u(0.0, 0.2), u(-0.12, 0.13)],
                self.hair,
                Stroke::NONE,
            ));
        }
        let open = if talking { (time * 16.0).sin().abs() * 0.035 + 0.005 } else { 0.0 };
        let mouth = u(0.0, 0.07);
        if open > 0.004 {
            painter.add(Shape::ellipse_filled(mouth, vec2(s * 0.035, s * open), hex("#4A1A18")));
        } else {
            painter.line_segment(
                [mouth - vec2(s * 0.03, 0.0), mouth + vec2(s * 0.03, 0.0)],
                Stroke::new(s * 0.012, shade(self.skin, -0.45)),
            );
        }
    }
}

/// One line of dialogue: who speaks, what they say, and optional player choices.
#[derive(Clone, Debug, PartialEq)]
pub struct GameDialogueLine {
    /// Speaker name.
    pub speaker: String,
    /// The line (typed out).
    pub text: String,
    /// Speaker portrait.
    pub portrait: GamePortrait,
    /// Player responses shown when the text finishes (empty = click to continue).
    pub choices: Vec<String>,
}

impl GameDialogueLine {
    /// A line spoken by `speaker`.
    pub fn new(speaker: impl Into<String>, portrait: GamePortrait, text: impl Into<String>) -> Self {
        Self { speaker: speaker.into(), text: text.into(), portrait, choices: Vec::new() }
    }
    /// Add a player choice.
    pub fn choice(mut self, c: impl Into<String>) -> Self {
        self.choices.push(c.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
struct DlgState {
    key: u64,
    start: f64,
    skip: bool,
    shown: usize,
    sel: usize,
}

/// An RPG dialogue box. The speaker's portrait sits in a frame breaking out of the top-left with
/// a name plate; the text types itself out with a glowing caret while the portrait's mouth moves.
/// Click once to finish the line, again to continue. When the line has choices they stagger in
/// once the text is done; pick one with the mouse or arrow keys + Enter.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameDialogueBox, GameDialogueLine, GamePortrait};
/// let line = GameDialogueLine::new("Maren", GamePortrait::elder(), "The vault door won't open for just anyone.")
///     .choice("Then I'll make it.")
///     .choice("Who can open it?");
/// let mut chosen = None;
/// let mut next = false;
/// ui.add(GameDialogueBox::new(&line).chosen(&mut chosen).advance(&mut next));
/// ```
pub struct GameDialogueBox<'a> {
    line: &'a GameDialogueLine,
    chosen: Option<&'a mut Option<usize>>,
    advance: Option<&'a mut bool>,
    cps: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    autoplay: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameDialogueBox<'a> {
    /// A box showing `line`. A new line (different text) restarts the typewriter.
    pub fn new(line: &'a GameDialogueLine) -> Self {
        Self {
            line,
            chosen: None,
            advance: None,
            cps: 38.0,
            style: GameStyle::Fantasy,
            palette: None,
            autoplay: false,
            cues: None,
        }
    }
    /// Receives the picked choice index.
    pub fn chosen(mut self, c: &'a mut Option<usize>) -> Self {
        self.chosen = Some(c);
        self
    }
    /// Set to true when the player clicks past a finished line without choices.
    pub fn advance(mut self, a: &'a mut bool) -> Self {
        self.advance = Some(a);
        self
    }
    /// Typing speed in characters per second.
    pub fn speed(mut self, cps: f32) -> Self {
        self.cps = cps.max(1.0);
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Demo mode: hover through the choices while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Type`] while typing, [`GameCue::Confirm`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

impl egui::Widget for GameDialogueBox<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let name_f = display_font(ui, 16.0);
        let body_f = semi_font(ui, 16.0);
        let choice_f = semi_font(ui, 14.0);
        let w = ui.available_width().min(780.0);
        let nchoice = self.line.choices.len();
        let ch_h = 38.0;
        let box_h = 150.0;
        let top_pad = 28.0;
        let h = top_pad + box_h + if nchoice > 0 { nchoice as f32 * ch_h + 14.0 } else { 0.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let time = now as f32;
        let key = hash_str(&self.line.text) ^ hash_str(&self.line.speaker);
        let total = self.line.text.chars().count();
        let mut st: DlgState = glowkit_core::state(ui, id.with("dlg"), DlgState::default);
        if st.key != key {
            st = DlgState { key, start: now, skip: false, shown: 0, sel: 0 };
        }
        let typed = if st.skip { total } else { (((now - st.start) as f32 * self.cps) as usize).min(total) };
        let done = typed >= total;
        if typed > st.shown {
            let new_char = self.line.text.chars().nth(typed - 1).unwrap_or(' ');
            if !new_char.is_whitespace() {
                push_cue(&mut self.cues, GameCue::Type);
            }
            st.shown = typed;
        }
        let done_at = if st.skip { now } else { st.start + total as f64 / self.cps as f64 };
        let since_done = if done { (now - done_at) as f32 } else { 0.0 };
        if !done || since_done < 1.0 || nchoice > 0 {
            glowkit_core::animate(ui);
        }
        let box_rect = Rect::from_min_size(pos2(rect.min.x, rect.min.y + top_pad), vec2(w, box_h));
        // choices
        let ch_w = (w * 0.5).max(240.0).min(w);
        let mut hovered_choice = None;
        let mut clicked_choice = None;
        if done && nchoice > 0 {
            for i in 0..nchoice {
                let r = Rect::from_min_size(
                    pos2(rect.max.x - ch_w, box_rect.max.y + 10.0 + i as f32 * ch_h),
                    vec2(ch_w, ch_h - 6.0),
                );
                let cr = ui.interact(r, id.with(("choice", i)), Sense::click());
                if cr.hovered() {
                    hovered_choice = Some(i);
                }
                if cr.clicked() {
                    clicked_choice = Some(i);
                }
            }
            let (up, down, enter) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::Enter),
                )
            });
            if up {
                st.sel = (st.sel + nchoice - 1) % nchoice;
            }
            if down {
                st.sel = (st.sel + 1) % nchoice;
            }
            if enter {
                clicked_choice = Some(st.sel);
            }
        }
        let idle = idle_for(ui, id, hovered_choice.is_some());
        if let Some(hc) = hovered_choice {
            if hc != st.sel {
                push_cue(&mut self.cues, GameCue::Hover);
            }
            st.sel = hc;
        } else if self.autoplay && done && nchoice > 0 && idle > 1.0 {
            st.sel = ((since_done / 1.2) as usize) % nchoice;
        }
        if resp.clicked() && clicked_choice.is_none() {
            if !done {
                st.skip = true;
            } else if nchoice == 0 {
                push_cue(&mut self.cues, GameCue::Confirm);
                if let Some(a) = self.advance.as_deref_mut() {
                    *a = true;
                }
            }
        }
        if let Some(c) = clicked_choice {
            push_cue(&mut self.cues, GameCue::Confirm);
            if let Some(ch) = self.chosen.as_deref_mut() {
                *ch = Some(c);
            }
        }
        glowkit_core::set_state(ui, id.with("dlg"), st.clone());
        let painter = ui.painter();
        // box
        frame(painter, box_rect, self.style, &pal, 0.0);
        let por_s = 132.0;
        let por =
            Rect::from_min_size(pos2(box_rect.min.x + 16.0, box_rect.max.y - por_s - 12.0 - 16.0), Vec2::splat(por_s));
        let bob = if !done { (time * 9.0).sin() * 0.8 } else { 0.0 };
        let por = por.translate(vec2(0.0, bob));
        paint::soft_shadow(painter, por, 8.0, 14.0, vec2(0.0, 4.0), Color32::from_black_alpha(140));
        self.line.portrait.paint(painter, por, time, !done);
        match self.style {
            GameStyle::Fantasy => {
                painter.rect_stroke(por, 10.0, Stroke::new(3.0, pal.edge), egui::StrokeKind::Outside);
            }
            GameStyle::SciFi => {
                painter.rect_stroke(por, 10.0, Stroke::new(1.5, pal.accent), egui::StrokeKind::Outside);
                let y = por.min.y + por.height() * ((time * 0.6).fract());
                painter.line_segment(
                    [pos2(por.min.x + 2.0, y), pos2(por.max.x - 2.0, y)],
                    Stroke::new(1.0, with_alpha(pal.accent, 0.35)),
                );
            }
            GameStyle::Minimal => {
                painter.rect_stroke(por, 10.0, Stroke::new(1.0, hex("#FFFFFF30")), egui::StrokeKind::Outside);
            }
        }
        // name plate
        let nw = text_w(painter, &self.line.speaker, &name_f) + 32.0;
        let plate = Rect::from_min_size(pos2(por.max.x + 14.0, box_rect.min.y - 14.0), vec2(nw, 28.0));
        match self.style {
            GameStyle::Fantasy => {
                paint::soft_shadow(painter, plate, 4.0, 8.0, vec2(0.0, 2.0), Color32::from_black_alpha(120));
                paint::gradient_rect(painter, plate, 4.0, 90.0, &[hex("#A33A2C"), hex("#6E2219")]);
                painter.rect_stroke(plate, 4.0, Stroke::new(1.5, pal.edge), egui::StrokeKind::Inside);
                painter.text(plate.center(), Align2::CENTER_CENTER, &self.line.speaker, name_f, hex("#FBEFD2"));
            }
            GameStyle::SciFi => {
                let pts = chamfer_points(plate, 8.0);
                painter.add(Shape::convex_polygon(pts, pal.accent, Stroke::NONE));
                painter.text(
                    plate.center(),
                    Align2::CENTER_CENTER,
                    self.line.speaker.to_uppercase(),
                    name_f,
                    hex("#03121C"),
                );
            }
            GameStyle::Minimal => {
                painter.rect_filled(plate, 14.0, Color32::WHITE);
                painter.text(plate.center(), Align2::CENTER_CENTER, &self.line.speaker, name_f, hex("#111318"));
            }
        }
        // text with stable wrapping: hidden part is transparent
        let tx = por.max.x + 20.0;
        let wrap = box_rect.max.x - tx - 24.0;
        let split = self.line.text.char_indices().nth(typed).map(|(b, _)| b).unwrap_or(self.line.text.len());
        let mut job = LayoutJob::default();
        let tc = pal.text;
        job.append(
            &self.line.text[..split],
            0.0,
            TextFormat { font_id: body_f.clone(), color: tc, line_height: Some(24.0), ..Default::default() },
        );
        job.append(
            &self.line.text[split..],
            0.0,
            TextFormat {
                font_id: body_f.clone(),
                color: Color32::TRANSPARENT,
                line_height: Some(24.0),
                ..Default::default()
            },
        );
        job.wrap.max_width = wrap;
        let galley = painter.layout_job(job);
        let tpos = pos2(tx, box_rect.min.y + 26.0);
        let caret = galley.pos_from_cursor(CCursor::new(typed));
        painter.galley(tpos, galley, tc);
        if !done {
            let cp = tpos + caret.center().to_vec2() + vec2(3.0, 0.0);
            paint::glow(painter, cp, 10.0, with_alpha(pal.accent, 0.6));
            paint::sparkle(painter, cp, 5.0, time * 4.0, mix(pal.accent, Color32::WHITE, 0.5));
        } else if nchoice == 0 {
            let b = (time * 5.0).sin() * 3.0;
            let p = pos2(box_rect.max.x - 26.0, box_rect.max.y - 22.0 + b);
            painter.add(Shape::convex_polygon(
                vec![p + vec2(-7.0, -4.0), p + vec2(7.0, -4.0), p + vec2(0.0, 5.0)],
                pal.accent,
                Stroke::NONE,
            ));
        }
        // choices
        if done {
            for (i, text) in self.line.choices.iter().enumerate() {
                let k = glowkit_core::anim::stagger(since_done, i, 0.08, 0.3);
                if k <= 0.0 {
                    continue;
                }
                let e = glowkit_core::ease(glowkit_core::Easing::OutBack, k);
                let on = st.sel == i;
                let slide = spring(ui, id.with(("cs", i)), if on { 1.0 } else { 0.0 }, 420.0, 22.0);
                let r = Rect::from_min_size(
                    pos2(rect.max.x - ch_w + (1.0 - e) * 50.0 - slide * 10.0, box_rect.max.y + 10.0 + i as f32 * ch_h),
                    vec2(ch_w, ch_h - 6.0),
                );
                let mut p2 = painter.clone();
                p2.set_opacity(k);
                frame(&p2, r, self.style, &pal, slide);
                let num = format!("{}", i + 1);
                let badge = pos2(r.min.x + 20.0, r.center().y);
                p2.circle_filled(badge, 10.0, if on { pal.accent } else { with_alpha(pal.dim, 0.35) });
                p2.text(
                    badge,
                    Align2::CENTER_CENTER,
                    num,
                    FontId::proportional(11.0),
                    if on { hex("#111111") } else { pal.text },
                );
                p2.text(
                    pos2(r.min.x + 40.0, r.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    choice_f.clone(),
                    if on { pal.text } else { with_alpha(pal.text, 0.75) },
                );
            }
        }
        resp
    }
}
