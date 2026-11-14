//! An A-to-Z index bar for jumping through a long sectioned list by tapping or scrubbing.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Default)]
struct IndexState {
    sc: Scroller,
    jump: Option<f32>,
    jump_v: f32,
    scrub: bool,
    letter: usize,
    finger_y: f32,
    bubble: f32,
    bubble_v: f32,
    bump: f32,
    bump_v: f32,
    lens: f32,
    lens_v: f32,
    g: Gesture,
    cycle: u32,
}

const LETTERS: [&str; 27] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W",
    "X", "Y", "Z", "#",
];

/// A contacts-style list with sticky section headers and an A-Z index along the trailing edge.
///
/// Tap a letter and the list glides (on a spring) to that section; press and scrub along the
/// bar and the list jumps section by section while a bubble pops out beside your finger showing
/// the current letter, bumping each time it changes. The letters under the finger swell like a
/// magnifying dock. Letters with no entries resolve to the next section that exists. The list
/// also scrolls on its own with flick momentum and rubber-banded ends, and each section header
/// sticks to the top until the next one pushes it away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::SectionIndex;
/// let names = ["Aaron Price", "Abby Chen", "Ben Carter", "Carla Diaz"];
/// ui.add(SectionIndex::new(&names).title("Contacts").magnify(60.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SectionIndex<'a> {
    names: &'a [&'a str],
    title: String,
    size: Option<Vec2>,
    magnify: f32,
    autoplay: bool,
}

impl<'a> SectionIndex<'a> {
    /// An indexed list over `names` (grouped by first letter; sorted for you).
    pub fn new(names: &'a [&'a str]) -> Self {
        Self { names, title: "Contacts".into(), size: None, magnify: 60.0, autoplay: false }
    }
    /// Large title above the list.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// How much letters under the finger swell, 0..100.
    pub fn magnify(mut self, m: f32) -> Self {
        self.magnify = m.clamp(0.0, 100.0);
        self
    }
    /// Scrub and tap the index on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Sample contact names for demos.
pub(crate) const SAMPLE_NAMES: [&str; 44] = [
    "Aaron Price",
    "Abby Chen",
    "Alex Morgan",
    "Amira Haddad",
    "Ben Carter",
    "Bianca Rossi",
    "Carla Diaz",
    "Chris Okafor",
    "Daniel Park",
    "Dana Whitfield",
    "Elena Petrova",
    "Emeka Obi",
    "Farah Khan",
    "Felix Wagner",
    "Grace Liu",
    "Gustavo Lima",
    "Hana Sato",
    "Hugo Martin",
    "Isla Brennan",
    "Jonas Berg",
    "Julia Novak",
    "Kai Nakamura",
    "Kira Volkova",
    "Leo Dubois",
    "Lucia Ferreira",
    "Maya Chen",
    "Mateo Ruiz",
    "Mila Horvat",
    "Nadia Ali",
    "Noah Fischer",
    "Omar Haddad",
    "Priya Raman",
    "Paolo Conti",
    "Rosa Medina",
    "Ryan Walsh",
    "Sara Lindqvist",
    "Sam Taylor",
    "Sofia Rossi",
    "Tom Becker",
    "Tara Singh",
    "Vera Ivanova",
    "Will Turner",
    "Yara Haddad",
    "Zoe Adams",
];

const HEAD_H: f32 = 26.0;
const ROW_H: f32 = 44.0;
const SCRIPT: [Stroke2; 3] =
    [drag(0.8, 2.8, (0.955, 0.16), (0.955, 0.72)), tap(3.9, 0.955, 0.222), drag(5.2, 5.42, (0.45, 0.82), (0.45, 0.42))];

impl egui::Widget for SectionIndex<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 7.6);
        let mut st: IndexState = glowkit_core::state(ui, id.with("index"), IndexState::default);
        if touch.ghost && touch.cycle != st.cycle {
            st = IndexState { cycle: touch.cycle, ..Default::default() };
        }

        // sections
        let mut names: Vec<&str> = self.names.to_vec();
        names.sort_unstable_by_key(|n| n.to_lowercase());
        let mut sections: Vec<(usize, Vec<&str>)> = Vec::new();
        for n in names {
            let c = n.chars().next().unwrap_or('#').to_ascii_uppercase();
            let li = LETTERS.iter().position(|l| l.starts_with(c)).unwrap_or(26);
            match sections.last_mut() {
                Some((l, v)) if *l == li => v.push(n),
                _ => sections.push((li, vec![n])),
            }
        }
        let mut offs = Vec::with_capacity(sections.len());
        let mut acc = 0.0;
        for (_, v) in &sections {
            offs.push(acc);
            acc += HEAD_H + v.len() as f32 * ROW_H;
        }
        let content_h = acc;
        let view = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 66.0), rect.max);
        let max = (content_h - view.height() + 12.0).max(0.0);
        let bar = Rect::from_min_max(pos2(view.max.x - 22.0, view.min.y + 8.0), pos2(view.max.x, view.max.y - 8.0));
        let slot = bar.height() / LETTERS.len() as f32;
        let letter_at = |y: f32| (((y - bar.min.y) / slot).floor().max(0.0) as usize).min(LETTERS.len() - 1);
        let section_for =
            |li: usize| sections.iter().position(|(l, _)| *l >= li).unwrap_or(sections.len().saturating_sub(1));

        let g = st.g.update(&touch, now, dt);
        if g.pressed {
            st.scrub = bar.expand2(vec2(6.0, 0.0)).contains(st.g.start);
            st.jump = None;
            if st.scrub {
                st.letter = letter_at(st.g.start.y);
                st.bubble_v = 4.0;
            }
        }
        if st.g.down && st.scrub {
            let li = letter_at(st.g.last.y);
            st.finger_y = st.g.last.y;
            if li != st.letter || g.pressed {
                if li != st.letter {
                    st.bump_v = 5.0;
                }
                st.letter = li;
                let si = section_for(li);
                st.sc.s = offs.get(si).copied().unwrap_or(0.0).min(max);
                st.sc.v = 0.0;
            }
        } else if st.g.down && !g.pressed {
            st.sc.drag(g.delta.y, 0.0, max);
            st.sc.v = 0.0;
        }
        if g.released {
            if st.scrub && g.tap {
                let si = section_for(letter_at(st.g.last.y));
                st.jump = Some(offs.get(si).copied().unwrap_or(0.0).min(max));
                st.jump_v = 0.0;
            } else if !st.scrub && !g.tap {
                st.sc.v = -st.g.vel.y;
            }
            st.scrub = false;
        }
        let mut moving = st.g.down;
        if let Some(target) = st.jump {
            spring_step(&mut st.sc.s, &mut st.jump_v, target, 140.0, 2.0 * 140f32.sqrt(), dt);
            if settled(st.sc.s, st.jump_v, target) {
                st.jump = None;
            }
            moving = true;
        } else if !st.g.down {
            moving |= st.sc.step(0.0, max, dt);
        }
        let scrubbing = st.g.down && st.scrub;
        let bt = if scrubbing { 1.0 } else { 0.0 };
        spring_step(&mut st.bubble, &mut st.bubble_v, bt, 380.0, 20.0, dt);
        spring_step(&mut st.bump, &mut st.bump_v, 0.0, 600.0, 16.0, dt);
        spring_step(&mut st.lens, &mut st.lens_v, bt, 300.0, 26.0, dt);
        moving |= !settled(st.bubble, st.bubble_v, bt) || st.bump_v.abs() > 0.01 || !settled(st.lens, st.lens_v, bt);
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("index"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        text_bold(
            &painter,
            pos2(rect.min.x + 18.0, rect.min.y + 46.0),
            Align2::LEFT_CENTER,
            &self.title,
            24.0,
            t.foreground,
        );
        let clip = painter.with_clip_rect(view);
        let scroll = s.sc.s;
        let head_bg = if t.dark { mix(t.background, t.foreground, 0.04) } else { hexa(0xF4, 0xF4, 0xF6) };
        let lx = view.min.x + 18.0;
        let rx = bar.min.x - 6.0;
        let mut sticky: Option<(usize, f32)> = None;
        for (k, (li, v)) in sections.iter().enumerate() {
            let y0 = view.min.y + offs[k] - scroll;
            let y1 = y0 + HEAD_H + v.len() as f32 * ROW_H;
            if y1 < view.min.y || y0 > view.max.y {
                continue;
            }
            if y0 <= view.min.y && y1 > view.min.y {
                let push = (y1 - HEAD_H - view.min.y).min(0.0);
                sticky = Some((*li, view.min.y + push));
            } else {
                let hr = Rect::from_min_max(pos2(view.min.x, y0), pos2(view.max.x, y0 + HEAD_H));
                clip.rect_filled(hr, cr(0.0), head_bg);
                text_bold(&clip, pos2(lx, hr.center().y), Align2::LEFT_CENTER, LETTERS[*li], 12.0, t.muted_foreground);
            }
            for (j, name) in v.iter().enumerate() {
                let ry = y0 + HEAD_H + j as f32 * ROW_H;
                if ry > view.max.y || ry + ROW_H < view.min.y {
                    continue;
                }
                let (first, last) = name.split_once(' ').unwrap_or((name, ""));
                let r1 = text(&clip, pos2(lx, ry + ROW_H / 2.0), Align2::LEFT_CENTER, first, 14.5, t.foreground);
                text_bold(&clip, pos2(r1.max.x + 4.0, ry + ROW_H / 2.0), Align2::LEFT_CENTER, last, 14.5, t.foreground);
                if j + 1 < v.len() {
                    clip.line_segment(
                        [pos2(lx, ry + ROW_H - 0.5), pos2(rx, ry + ROW_H - 0.5)],
                        egui::Stroke::new(1.0, hairline(&t)),
                    );
                }
            }
        }
        if let Some((li, y)) = sticky {
            let hr = Rect::from_min_size(pos2(view.min.x, y), vec2(view.width(), HEAD_H));
            clip.rect_filled(hr, cr(0.0), head_bg);
            text_bold(&clip, pos2(lx, hr.center().y), Align2::LEFT_CENTER, LETTERS[li], 12.0, t.muted_foreground);
        }

        // index bar with a dock-like lens around the finger
        let blue = system_blue(&t);
        let lens = s.lens.max(0.0);
        let mag = self.magnify / 100.0;
        if lens > 0.01 {
            painter.rect_filled(bar.expand2(vec2(2.0, 4.0)), cr(11.0), with_alpha(t.foreground, 0.06 * lens.min(1.0)));
        }
        for (i, l) in LETTERS.iter().enumerate() {
            let cy = bar.min.y + (i as f32 + 0.5) * slot;
            let d = ((cy - s.finger_y) / (slot * 3.0)).abs();
            let k = (1.0 - d * d).max(0.0) * lens;
            let scale = 1.0 + k * mag * 1.1;
            let x = bar.center().x - k * mag * 22.0;
            let has = sections.iter().any(|(li, _)| *li == i);
            let col = if has { blue } else { with_alpha(blue, 0.45) };
            painter.text(pos2(x, cy), Align2::CENTER_CENTER, *l, FontId::proportional(10.5 * scale), col);
        }
        // bubble
        let b = s.bubble;
        if b > 0.01 {
            let li = s.letter;
            let si = section_for(li);
            let shown = sections.get(si).map(|(l, _)| *l).unwrap_or(li);
            let c = pos2(
                lerp(bar.min.x - 10.0, bar.min.x - 64.0 - mag * 10.0, b.min(1.2)),
                s.finger_y.clamp(view.min.y + 40.0, view.max.y - 40.0),
            );
            let r = 27.0 * b.clamp(0.0, 1.3) * (1.0 + s.bump * 0.06);
            shadow(&painter, &t, Rect::from_center_size(c, Vec2::splat(r * 2.0)), r, 0.8 * b.min(1.0));
            painter.circle_filled(
                c,
                r,
                with_alpha(if t.dark { hexa(0x3A, 0x3A, 0x40) } else { hexa(0x8E, 0x8E, 0x93) }, b.clamp(0.0, 1.0)),
            );
            painter.text(
                c,
                Align2::CENTER_CENTER,
                LETTERS[shown],
                FontId::proportional(26.0 * b.clamp(0.0, 1.2)),
                with_alpha(Color32::WHITE, b.clamp(0.0, 1.0)),
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
