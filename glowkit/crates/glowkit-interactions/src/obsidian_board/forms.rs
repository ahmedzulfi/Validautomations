//! Conversation cards: an agent questionnaire and a notification center.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{fade, mix};
use glowkit_core::{paint, Theme};

// =============================================================================================
// Questionnaire

/// One question of a [`BoardQuestionnaire`].
#[derive(Clone, Debug, PartialEq)]
pub struct BoardQuestion {
    /// The prompt across the top.
    pub prompt: String,
    /// Answer rows: title and a one-line description.
    pub options: Vec<(String, String)>,
    /// Allow several answers (checkboxes) instead of one (number keys).
    pub multi: bool,
    /// Add a free-text "Other" row.
    pub other: bool,
}

impl BoardQuestion {
    /// A single-choice question.
    pub fn new(prompt: impl Into<String>, options: &[(&str, &str)]) -> Self {
        Self {
            prompt: prompt.into(),
            options: options.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            multi: false,
            other: true,
        }
    }
    /// Allow several answers.
    pub fn multi(mut self, on: bool) -> Self {
        self.multi = on;
        self
    }
    /// Show the free-text "Other" row (default on).
    pub fn other(mut self, on: bool) -> Self {
        self.other = on;
        self
    }
}

/// Caller-owned answers of a [`BoardQuestionnaire`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardQuestionnaireState {
    /// Current question index.
    pub step: usize,
    /// Picked options per question (the last slot is the "Other" row).
    pub picks: Vec<Vec<bool>>,
    /// Free text typed into each question's "Other" row.
    pub other: Vec<String>,
    /// Set once the last question was submitted.
    pub done: bool,
}

impl BoardQuestionnaireState {
    fn fit(&mut self, qs: &[BoardQuestion]) {
        self.picks.resize(qs.len(), Vec::new());
        self.other.resize(qs.len(), String::new());
        for (p, q) in self.picks.iter_mut().zip(qs) {
            p.resize(q.options.len() + 1, false);
        }
        self.step = self.step.min(qs.len().saturating_sub(1));
    }
    /// Whether question `i` has at least one answer.
    pub fn answered(&self, i: usize) -> bool {
        self.picks.get(i).is_some_and(|p| p.iter().any(|b| *b))
    }
}

/// The questions an agent asks before it commits to a plan, as a card in the chat thread: one
/// question per step with the prompt across the top, the answers as bordered rows with a
/// checkbox (multi-select) or a number key (single choice), an optional free-text "Other" row,
/// and a footer with a pill per step next to Previous and Next. Questions slide in from the
/// direction of travel while the card springs to the next question's height.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardQuestion, BoardQuestionnaire, BoardQuestionnaireState};
/// let qs = vec![BoardQuestion::new("Which layout?", &[("Grid", "Cards in columns"), ("List", "One per row")])];
/// let mut answers = BoardQuestionnaireState::default();
/// ui.add(BoardQuestionnaire::new(&qs, &mut answers));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardQuestionnaire<'a> {
    questions: &'a [BoardQuestion],
    state: &'a mut BoardQuestionnaireState,
    width: f32,
    autoplay: bool,
}

impl<'a> BoardQuestionnaire<'a> {
    /// A questionnaire over `questions`, storing answers in `state`.
    pub fn new(questions: &'a [BoardQuestion], state: &'a mut BoardQuestionnaireState) -> Self {
        Self { questions, state, width: 620.0, autoplay: false }
    }
    /// Card width in points (default 620).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(320.0);
        self
    }
    /// Pick answers and step through by itself while idle (restarts when done).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

#[derive(Clone, Copy)]
struct Slide {
    step: usize,
    dir: f32,
    at: f32,
}

const ROW_H: f32 = 56.0;
const ROW_GAP: f32 = 8.0;

fn q_height(q: &BoardQuestion) -> f32 {
    let rows = q.options.len() + usize::from(q.other);
    48.0 + rows as f32 * (ROW_H + ROW_GAP) + 50.0
}

impl egui::Widget for BoardQuestionnaire<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let blue = p[BLUE];
        let st = self.state;
        if self.questions.is_empty() {
            return ui.allocate_response(vec2(self.width, 1.0), Sense::hover());
        }
        st.fit(self.questions);
        let id = ui.next_auto_id();
        let now = glowkit_core::time(ui);
        let target_h = if st.done { 140.0 } else { q_height(&self.questions[st.step]) };
        let h = glowkit_core::anim::spring(ui, id.with("h"), target_h, 220.0, 26.0);
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let hovered_any = ui.rect_contains_pointer(rect);
        let idle = idle_for(ui, id, hovered_any);

        // autoplay: a ghost answers and advances
        if self.autoplay && idle > 1.6 {
            let tick = ((idle - 1.6) / 1.1) as u64;
            let last_tick: u64 = glowkit_core::state(ui, id.with("tick"), || u64::MAX);
            if tick != last_tick {
                glowkit_core::set_state(ui, id.with("tick"), tick);
                if st.done {
                    if tick % 3 == 2 {
                        *st = BoardQuestionnaireState::default();
                        st.fit(self.questions);
                    }
                } else {
                    let q = &self.questions[st.step];
                    if !st.answered(st.step) {
                        let k = (st.step * 2 + 1) % q.options.len().max(1);
                        st.picks[st.step][k] = true;
                    } else if q.multi && st.picks[st.step].iter().filter(|b| **b).count() < 2 {
                        let k = (st.step * 2 + 2) % q.options.len().max(1);
                        st.picks[st.step][k] = true;
                    } else if st.step + 1 < self.questions.len() {
                        st.step += 1;
                    } else {
                        st.done = true;
                    }
                }
            }
            glowkit_core::animate(ui);
        }

        let slide = {
            let prev: Slide =
                glowkit_core::state(ui, id.with("slide"), || Slide { step: st.step, dir: 0.0, at: now - 5.0 });
            let s = if prev.step != st.step {
                Slide { step: st.step, dir: if st.step > prev.step { 1.0 } else { -1.0 }, at: now }
            } else {
                prev
            };
            glowkit_core::set_state(ui, id.with("slide"), s);
            s
        };
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(24.0));
        sheet(&painter, &t, rect, 16.0);
        let pad = 14.0;
        let clip = rect.shrink(1.0);
        let k = ease_out(((now - slide.at) / 0.38).clamp(0.0, 1.0));
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let ox = slide.dir * (1.0 - k) * 46.0;
        let mut cp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        cp.multiply_opacity(0.2 + 0.8 * k);

        if st.done {
            let c = pos2(rect.center().x, rect.min.y + 52.0);
            let dk = flag(ui, id.with("donek"), true, 6.0);
            painter.circle_filled(c, 18.0 * (0.6 + 0.4 * ease_back(dk)), fade(p[EMERALD], 0.16));
            icon_check(&painter, c, 16.0, p[EMERALD], 2.0);
            text(&painter, pos2(c.x, c.y + 36.0), Align2::CENTER_CENTER, "Answers sent", f(14.5), t.foreground);
            let n: usize = st.picks.iter().map(|v| v.iter().filter(|b| **b).count()).sum();
            text(
                &painter,
                pos2(c.x, c.y + 56.0),
                Align2::CENTER_CENTER,
                format!("{n} choices across {} questions", self.questions.len()),
                f(12.5),
                dim(&t),
            );
            let rr = Rect::from_center_size(pos2(rect.max.x - pad - 9.0, rect.min.y + 22.0), vec2(20.0, 20.0));
            let r = ui.interact(rr, id.with("again"), Sense::click());
            painter.circle_filled(rr.center(), 10.0, track(&t));
            icon_close(&painter, rr.center(), 7.0, dim(&t));
            if r.clicked() {
                *st = BoardQuestionnaireState::default();
                st.fit(self.questions);
            }
            return response;
        }

        let qi = st.step;
        let q = &self.questions[qi];
        // header
        text(
            &cp,
            pos2(rect.min.x + pad + ox, rect.min.y + 24.0),
            Align2::LEFT_CENTER,
            &q.prompt,
            f(14.0),
            t.foreground,
        );
        let xr = Rect::from_center_size(pos2(rect.max.x - pad - 9.0, rect.min.y + 24.0), vec2(20.0, 20.0));
        let xresp = ui.interact(xr, id.with("x"), Sense::click());
        let xh = flag(ui, id.with("xh"), xresp.hovered(), 14.0);
        painter.circle_filled(xr.center(), 10.0, mix(track(&t), neutral(&t), xh));
        icon_close(&painter, xr.center(), 6.5, dim(&t));
        if xresp.clicked() {
            st.done = true;
        }

        // rows
        let rows = q.options.len() + usize::from(q.other);
        for i in 0..rows {
            let y = rect.min.y + 44.0 + i as f32 * (ROW_H + ROW_GAP);
            let rr = Rect::from_min_size(pos2(rect.min.x + pad + ox, y), vec2(rect.width() - pad * 2.0, ROW_H));
            let is_other = i >= q.options.len();
            let resp = ui.interact(rr, id.with(("row", qi, i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                if q.multi {
                    st.picks[qi][i] = !st.picks[qi][i];
                } else {
                    let was = st.picks[qi][i];
                    st.picks[qi].iter_mut().for_each(|b| *b = false);
                    st.picks[qi][i] = !was;
                }
            }
            let sel = st.picks[qi][i];
            let sk = flag(ui, id.with(("sel", qi, i)), sel, 14.0);
            let hk = flag(ui, id.with(("hov", qi, i)), resp.hovered(), 14.0);
            let border = mix(mix(hairline(&t), neutral(&t), hk * 0.6), blue, sk);
            let fill = mix(tile_bg(&t), mix(tile_bg(&t), blue, if t.dark { 0.12 } else { 0.05 }), sk);
            cp.rect(rr, cr(10.0), fill, Stroke::new(1.0 + 0.4 * sk, border), egui::StrokeKind::Inside);
            let (title, desc) = if is_other { ("Other".to_string(), String::new()) } else { q.options[i].clone() };
            text(&cp, pos2(rr.min.x + 12.0, rr.min.y + 18.0), Align2::LEFT_CENTER, &title, f(13.5), t.foreground);
            if is_other {
                let er = Rect::from_min_size(pos2(rr.min.x + 10.0, rr.min.y + 28.0), vec2(rr.width() - 60.0, 22.0));
                let te = egui::TextEdit::singleline(&mut st.other[qi])
                    .frame(egui::Frame::NONE)
                    .font(f(13.0))
                    .text_color(t.foreground)
                    .hint_text(egui::RichText::new("Enter your custom answer here").color(faint(&t)).size(13.0));
                let r = ui.put(er, te);
                if r.changed() {
                    st.picks[qi][i] = !st.other[qi].trim().is_empty();
                }
            } else {
                text(&cp, pos2(rr.min.x + 12.0, rr.min.y + 38.0), Align2::LEFT_CENTER, &desc, f(12.5), dim(&t));
            }
            // marker
            let mc = pos2(rr.max.x - 26.0, rr.center().y);
            if q.multi || is_other {
                let b = Rect::from_center_size(mc, vec2(16.0, 16.0));
                cp.rect(
                    b,
                    cr(4.0),
                    mix(tile_bg(&t), blue, sk),
                    Stroke::new(1.0, mix(neutral(&t), blue, sk)),
                    egui::StrokeKind::Inside,
                );
                if sk > 0.02 {
                    icon_check(&cp, mc, 10.0 * (0.5 + 0.5 * ease_back(sk)), fade(Color32::WHITE, sk), 1.7);
                }
            } else {
                let b = Rect::from_center_size(mc, vec2(20.0, 20.0));
                cp.rect(b, cr(5.0), mix(track(&t), blue, sk), Stroke::NONE, egui::StrokeKind::Inside);
                text(&cp, mc, Align2::CENTER_CENTER, i + 1, f(12.0), mix(dim(&t), Color32::WHITE, sk));
            }
        }

        // footer
        let fy = rect.max.y - 26.0;
        let mut x = rect.min.x + pad;
        for i in 0..self.questions.len() {
            let label = format!("Step {}", i + 1);
            let w = text_w(&painter, &label, f(13.0)) + 18.0;
            let r = Rect::from_min_size(pos2(x, fy - 14.0), vec2(w, 28.0));
            let resp = ui.interact(r, id.with(("pill", i)), Sense::click());
            if resp.clicked() {
                st.step = i;
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let a = flag(ui, id.with(("pa", i)), i == qi, 12.0);
            let done_k = if st.answered(i) && i != qi { 1.0 } else { 0.0 };
            painter.rect_filled(r, cr(8.0), fade(mix(tile_bg(&t), blue, if t.dark { 0.2 } else { 0.1 }), a));
            let c = mix(mix(dim(&t), t.foreground, done_k * 0.6), blue, a);
            text(&painter, r.center(), Align2::CENTER_CENTER, label, f(13.0), c);
            x += w + 4.0;
        }
        let last = qi + 1 == self.questions.len();
        let next_l = if last { "Submit" } else { "Next" };
        let nw = text_w(&painter, next_l, f(13.5)) + 26.0;
        let nr = Rect::from_min_size(pos2(rect.max.x - pad - nw, fy - 15.0), vec2(nw, 30.0));
        let pw = text_w(&painter, "Previous", f(13.5)) + 26.0;
        let pr = Rect::from_min_size(pos2(nr.min.x - 8.0 - pw, fy - 15.0), vec2(pw, 30.0));
        let nresp = ui.interact(nr, id.with("next"), Sense::click());
        let presp = ui.interact(pr, id.with("prev"), Sense::click());
        let can_prev = qi > 0;
        let nh = flag(ui, id.with("nh"), nresp.hovered(), 14.0);
        painter.rect_filled(nr, cr(8.0), mix(blue, glowkit_core::color::shade(blue, -0.08), nh));
        text(&painter, nr.center(), Align2::CENTER_CENTER, next_l, f(13.5), Color32::WHITE);
        painter.rect(
            pr,
            cr(8.0),
            if can_prev { tile_bg(&t) } else { track(&t) },
            Stroke::new(1.0, hairline(&t)),
            egui::StrokeKind::Inside,
        );
        text(
            &painter,
            pr.center(),
            Align2::CENTER_CENTER,
            "Previous",
            f(13.5),
            if can_prev { t.foreground } else { faint(&t) },
        );
        for r in [&nresp, &presp] {
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        if nresp.clicked() {
            if last {
                st.done = true;
            } else {
                st.step += 1;
            }
        }
        if presp.clicked() && can_prev {
            st.step -= 1;
        }
        response
    }
}

// =============================================================================================
// Notification center

/// The badge drawn for a system notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoardNoticeIcon {
    /// A cloud with a down arrow (downloads, backups).
    #[default]
    Download,
    /// A person with a plus (joins, invites).
    Join,
    /// A cog (settings, system).
    Settings,
    /// An at sign (mentions).
    Mention,
}

/// One entry of a [`BoardNotifications`] inbox.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardNotice {
    /// Bold first line.
    pub title: String,
    /// Wrapped body text.
    pub body: String,
    /// Relative time ("2m").
    pub time: String,
    /// Timeline group heading ("Today").
    pub group: String,
    /// `true` for a mention (person avatar), `false` for a system notice (icon badge).
    pub mention: bool,
    /// Initials for the avatar of a mention.
    pub initials: String,
    /// Icon for a system notice.
    pub icon: BoardNoticeIcon,
    /// Inline action buttons (the first is primary).
    pub actions: Vec<String>,
    /// Unread flag (blue dot).
    pub unread: bool,
}

impl BoardNotice {
    /// A mention from a person with `initials`.
    pub fn mention(initials: &str, title: impl Into<String>, body: impl Into<String>, time: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            time: time.into(),
            group: "Today".into(),
            mention: true,
            initials: initials.into(),
            icon: BoardNoticeIcon::Mention,
            actions: Vec::new(),
            unread: true,
        }
    }
    /// A system notice with an icon badge.
    pub fn system(
        icon: BoardNoticeIcon,
        title: impl Into<String>,
        body: impl Into<String>,
        time: impl Into<String>,
    ) -> Self {
        Self { mention: false, icon, initials: String::new(), ..Self::mention("", title, body, time) }
    }
    /// Inline action buttons.
    pub fn actions(mut self, a: &[&str]) -> Self {
        self.actions = a.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Timeline group heading.
    pub fn group(mut self, g: impl Into<String>) -> Self {
        self.group = g.into();
        self
    }
    /// Unread flag.
    pub fn unread(mut self, u: bool) -> Self {
        self.unread = u;
        self
    }
}

/// An activity inbox: a title with the unread count, a "Mark all read" button, three filter
/// tabs (All, Mentions, System) with counts on a sliding segmented control, and grouped
/// timeline cards with avatars or status badges, unread dots, wrapped bodies and inline action
/// buttons. Switching tabs re-staggers the list, clicking a card or an action marks it read,
/// the list scrolls with the wheel, and an empty filter shows a composed empty state.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardNotice, BoardNotifications};
/// let mut inbox = vec![BoardNotice::mention("LK", "Livia mentioned you", "Can you review this?", "2m").actions(&["Reply"])];
/// ui.add(BoardNotifications::new(&mut inbox));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardNotifications<'a> {
    items: &'a mut [BoardNotice],
    size: egui::Vec2,
    autoplay: bool,
}

impl<'a> BoardNotifications<'a> {
    /// An inbox over `items`.
    pub fn new(items: &'a mut [BoardNotice]) -> Self {
        Self { items, size: vec2(430.0, 560.0), autoplay: false }
    }
    /// Panel size in points (default 430 x 560).
    pub fn size(mut self, s: egui::Vec2) -> Self {
        self.size = vec2(s.x.max(300.0), s.y.max(260.0));
        self
    }
    /// Cycle the filter tabs by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn avatar_color(t: &Theme, s: &str) -> Color32 {
    let p = pal(t);
    let h = s.bytes().fold(3u32, |a, b| a.wrapping_mul(33).wrapping_add(b as u32));
    [p[ORANGE], p[PURPLE], p[TEAL], p[PINK], p[INDIGO], p[EMERALD]][(h % 6) as usize]
}

impl egui::Widget for BoardNotifications<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(24.0));
        let el = entry(ui, id);
        sheet(&painter, &t, rect, 18.0);
        let pad = 16.0;
        let unread = self.items.iter().filter(|n| n.unread).count();
        text(
            &painter,
            pos2(rect.min.x + pad, rect.min.y + 28.0),
            Align2::LEFT_CENTER,
            "Notifications",
            f(17.0),
            t.foreground,
        );
        let ur = tween(ui, id.with("unread"), unread as f32, unread as f32, 0.4);
        text(
            &painter,
            pos2(rect.min.x + pad, rect.min.y + 52.0),
            Align2::LEFT_CENTER,
            format!("{} unread", ur.round()),
            f(13.0),
            dim(&t),
        );
        let mw = text_w(&painter, "Mark all read", f(13.5)) + 22.0;
        let mr = Rect::from_min_size(pos2(rect.max.x - pad - mw, rect.min.y + 16.0), vec2(mw, 30.0));
        let mresp = ui.interact(mr, id.with("markall"), Sense::click());
        let mh = flag(ui, id.with("mh"), mresp.hovered(), 14.0);
        let any = flag(ui, id.with("any"), unread > 0, 8.0);
        let blue = p[BLUE];
        painter.rect_filled(
            mr,
            cr(8.0),
            fade(mix(tile_bg(&t), blue, if t.dark { 0.2 + 0.1 * mh } else { 0.12 + 0.06 * mh }), 0.4 + 0.6 * any),
        );
        text(&painter, mr.center(), Align2::CENTER_CENTER, "Mark all read", f(13.5), fade(blue, 0.45 + 0.55 * any));
        if mresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if mresp.clicked() {
            self.items.iter_mut().for_each(|n| n.unread = false);
        }

        // tabs
        let seg = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + 70.0), vec2(rect.width() - pad * 2.0, 36.0));
        painter.rect_filled(seg, cr(10.0), track(&t));
        let counts = [
            self.items.len(),
            self.items.iter().filter(|n| n.mention).count(),
            self.items.iter().filter(|n| !n.mention).count(),
        ];
        let names = ["All", "Mentions", "System"];
        let tw = (seg.width() - 6.0) / 3.0;
        let mut tab: usize = glowkit_core::state(ui, id.with("tab"), || 0);
        let mut over_tabs = false;
        for i in 0..3 {
            let r = Rect::from_min_size(
                pos2(seg.min.x + 3.0 + i as f32 * tw, seg.min.y + 3.0),
                vec2(tw, seg.height() - 6.0),
            );
            let resp = ui.interact(r, id.with(("tab", i)), Sense::click());
            over_tabs |= resp.hovered();
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                tab = i;
            }
        }
        glowkit_core::set_state(ui, id.with("tab"), tab);
        let idle = idle_for(ui, id.with("idle"), response.hovered() || over_tabs);
        let shown_tab = if self.autoplay && idle > 2.5 { (tab + ((idle - 2.5) / 2.4) as usize) % 3 } else { tab };
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        let kx = glowkit_core::anim::spring(ui, id.with("kx"), seg.min.x + 3.0 + shown_tab as f32 * tw, 380.0, 30.0);
        let knob = Rect::from_min_size(pos2(kx, seg.min.y + 3.0), vec2(tw, seg.height() - 6.0));
        if !t.dark {
            paint::soft_shadow(&painter, knob, 8.0, 4.0, vec2(0.0, 1.0), Color32::from_rgba_unmultiplied(0, 0, 0, 16));
        }
        painter.rect_filled(knob, cr(8.0), if t.dark { glowkit_core::color::shade(t.muted, 0.12) } else { t.card });
        for i in 0..3 {
            let c = pos2(seg.min.x + 3.0 + (i as f32 + 0.5) * tw, seg.center().y);
            let lw = text_w(&painter, names[i], f(13.5));
            let cs = counts[i].to_string();
            let cw = text_w(&painter, &cs, f(11.5)) + 10.0;
            let x0 = c.x - (lw + 6.0 + cw) / 2.0;
            let a = flag(ui, id.with(("ta", i)), i == shown_tab, 12.0);
            text(&painter, pos2(x0, c.y), Align2::LEFT_CENTER, names[i], f(13.5), mix(dim(&t), t.foreground, a));
            let br = Rect::from_min_size(pos2(x0 + lw + 6.0, c.y - 9.0), vec2(cw, 18.0));
            painter.rect_filled(br, cr(5.0), mix(neutral(&t), track(&t), 0.35));
            text(&painter, br.center(), Align2::CENTER_CENTER, cs, f(11.5), dim(&t));
        }

        // list
        let list =
            Rect::from_min_max(pos2(rect.min.x + 8.0, seg.max.y + 10.0), pos2(rect.max.x - 8.0, rect.max.y - 8.0));
        painter.rect_filled(list, cr(14.0), frame_bg(&t));
        let lp = painter.with_clip_rect(list.shrink(1.0).intersect(painter.clip_rect()));
        let vis: Vec<usize> = (0..self.items.len())
            .filter(|&i| match shown_tab {
                1 => self.items[i].mention,
                2 => !self.items[i].mention,
                _ => true,
            })
            .collect();
        // re-stagger on tab change
        let now = glowkit_core::time(ui);
        let (tab_at, last_tab): (f32, usize) = glowkit_core::state(ui, id.with("tab_at"), || (now - el, shown_tab));
        let tab_at = if last_tab != shown_tab { now } else { tab_at };
        glowkit_core::set_state(ui, id.with("tab_at"), (tab_at, shown_tab));
        let since = now - tab_at;
        let inner_w = list.width() - 16.0;
        let text_x = 56.0;
        let body_w = inner_w - text_x - 14.0;
        // measure
        let mut cards = Vec::new();
        let mut y = 8.0;
        let mut last_group = String::new();
        for &i in &vis {
            let n = &self.items[i];
            if n.group != last_group {
                cards.push((None, y, 26.0));
                y += 26.0;
                last_group = n.group.clone();
            }
            let g = painter.layout(n.body.clone(), f(13.0), dim(&t), body_w);
            let hgt = 14.0 + 20.0 + g.size().y + if n.actions.is_empty() { 0.0 } else { 40.0 } + 14.0;
            cards.push((Some((i, g)), y, hgt));
            y += hgt + 8.0;
        }
        let content_h = y;
        let max_scroll = (content_h - list.height()).max(0.0);
        let mut scroll: f32 = glowkit_core::state(ui, id.with(("scroll", shown_tab)), || 0.0);
        if ui.rect_contains_pointer(list) {
            scroll = (scroll - ui.input(|i| i.smooth_scroll_delta.y)).clamp(0.0, max_scroll);
        }
        glowkit_core::set_state(ui, id.with(("scroll", shown_tab)), scroll);
        let scroll_s = glowkit_core::anim::spring(ui, id.with("scroll_s"), scroll, 300.0, 32.0);
        let mut k_i = 0;
        let mut groups_seen = String::new();
        for (entry_c, cy, ch) in cards {
            let ky = list.min.y + cy - scroll_s;
            if ky > list.max.y || ky + ch < list.min.y {
                k_i += 1;
                continue;
            }
            let a = ease_out(((since.min(el) - 0.05 * k_i as f32) / 0.4).clamp(0.0, 1.0));
            if a < 1.0 {
                ui.ctx().request_repaint();
            }
            let dy = (1.0 - a) * 10.0;
            match entry_c {
                None => {
                    // group heading
                    let g = vis
                        .iter()
                        .map(|i| &self.items[*i].group)
                        .find(|g| **g != groups_seen)
                        .cloned()
                        .unwrap_or_default();
                    groups_seen = g.clone();
                    text(
                        &lp,
                        pos2(list.min.x + 12.0, ky + 14.0 + dy),
                        Align2::LEFT_CENTER,
                        g,
                        f(12.0),
                        fade(faint(&t), a),
                    );
                }
                Some((i, g)) => {
                    let r = Rect::from_min_size(pos2(list.min.x + 8.0, ky + dy), vec2(inner_w, ch));
                    let mut cp = lp.clone();
                    cp.multiply_opacity(a.max(0.001));
                    let resp = ui.interact(r.intersect(list), id.with(("card", i)), Sense::click());
                    let hk = flag(ui, id.with(("ch", i)), resp.hovered(), 14.0);
                    cp.rect_filled(r, cr(12.0), mix(tile_bg(&t), mix(tile_bg(&t), t.foreground, 0.02), hk));
                    let n = &self.items[i];
                    let av = pos2(r.min.x + 30.0, r.min.y + 30.0);
                    if n.mention {
                        let c = avatar_color(&t, &n.initials);
                        paint::gradient_circle(
                            &cp,
                            av,
                            19.0,
                            &[glowkit_core::color::shade(c, 0.08), glowkit_core::color::shade(c, -0.06)],
                        );
                        text(&cp, av, Align2::CENTER_CENTER, &n.initials, f(13.0), Color32::WHITE);
                    } else {
                        let (c, gl) = match n.icon {
                            BoardNoticeIcon::Download => (p[LIME], Glyph::Cloud),
                            BoardNoticeIcon::Join => (p[BLUE], Glyph::UserPlus),
                            BoardNoticeIcon::Settings => (p[PURPLE], Glyph::Gear),
                            BoardNoticeIcon::Mention => (p[PINK], Glyph::At),
                        };
                        cp.circle_filled(av, 19.0, fade(c, if t.dark { 0.2 } else { 0.18 }));
                        glyph(&cp, gl, av, 17.0, glowkit_core::color::shade(c, if t.dark { 0.05 } else { -0.18 }));
                    }
                    let tx = r.min.x + text_x;
                    let dot = flag(ui, id.with(("dot", i)), n.unread, 8.0);
                    let time_x = r.max.x - 14.0 - 14.0 * dot;
                    let tr = text(&cp, pos2(time_x, r.min.y + 24.0), Align2::RIGHT_CENTER, &n.time, f(12.0), faint(&t));
                    if dot > 0.01 {
                        cp.circle_filled(pos2(r.max.x - 16.0, r.min.y + 24.0), 3.6 * dot, blue);
                    }
                    let mut title = n.title.clone();
                    let maxw = tr.min.x - tx - 8.0;
                    if text_w(&cp, &title, f(14.0)) > maxw {
                        let mut ch: Vec<char> = title.chars().collect();
                        while ch.len() > 3
                            && text_w(&cp, &format!("{}…", ch.iter().collect::<String>()), f(14.0)) > maxw
                        {
                            ch.pop();
                        }
                        title = format!("{}…", ch.iter().collect::<String>());
                    }
                    text(&cp, pos2(tx, r.min.y + 24.0), Align2::LEFT_CENTER, title, f(14.0), t.foreground);
                    cp.galley(pos2(tx, r.min.y + 36.0), g.clone(), dim(&t));
                    let mut clicked_any = resp.clicked();
                    if !n.actions.is_empty() {
                        let mut bx = tx;
                        let by = r.min.y + 36.0 + g.size().y + 8.0;
                        for (k, a_label) in n.actions.iter().enumerate() {
                            let bw = text_w(&cp, a_label, f(13.5)) + 22.0;
                            let br = Rect::from_min_size(pos2(bx, by), vec2(bw, 30.0));
                            let bresp = ui.interact(br.intersect(list), id.with(("act", i, k)), Sense::click());
                            let bh = flag(ui, id.with(("bh", i, k)), bresp.hovered(), 14.0);
                            if k == 0 && n.mention {
                                cp.rect_filled(br, cr(8.0), mix(blue, glowkit_core::color::shade(blue, -0.08), bh));
                                text(&cp, br.center(), Align2::CENTER_CENTER, a_label, f(13.5), Color32::WHITE);
                            } else {
                                cp.rect(
                                    br,
                                    cr(8.0),
                                    mix(tile_bg(&t), track(&t), bh),
                                    Stroke::new(1.0, hairline(&t)),
                                    egui::StrokeKind::Inside,
                                );
                                text(&cp, br.center(), Align2::CENTER_CENTER, a_label, f(13.5), t.foreground);
                            }
                            if bresp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            clicked_any |= bresp.clicked();
                            bx += bw + 6.0;
                        }
                    }
                    if clicked_any {
                        self.items[i].unread = false;
                    }
                }
            }
            k_i += 1;
        }
        if vis.is_empty() {
            let c = list.center() - vec2(0.0, 20.0);
            let a = ease_out((since / 0.4).clamp(0.0, 1.0));
            painter.circle_filled(c, 28.0, fade(tile_bg(&t), a));
            // bell
            let bell: Vec<egui::Pos2> = (0..=16)
                .map(|k| {
                    let u = k as f32 / 16.0;
                    let ang = std::f32::consts::PI * (1.0 + u);
                    c + vec2(ang.cos() * 8.0, ang.sin() * 9.0 - 1.0)
                })
                .collect();
            let st = Stroke::new(1.5, fade(dim(&t), a));
            painter.add(egui::Shape::line(bell, st));
            painter.line_segment([c + vec2(-8.0, -1.0), c + vec2(-9.5, 7.0)], st);
            painter.line_segment([c + vec2(8.0, -1.0), c + vec2(9.5, 7.0)], st);
            painter.line_segment([c + vec2(-11.0, 7.0), c + vec2(11.0, 7.0)], st);
            painter.circle_filled(c + vec2(0.0, 10.5), 2.0, fade(dim(&t), a));
            text(
                &painter,
                c + vec2(0.0, 46.0),
                Align2::CENTER_CENTER,
                "You're all caught up",
                f(14.0),
                fade(t.foreground, a),
            );
            text(
                &painter,
                c + vec2(0.0, 66.0),
                Align2::CENTER_CENTER,
                "New activity will show up here.",
                f(12.5),
                fade(dim(&t), a),
            );
        }
        // fade the scroll edges
        if scroll_s > 1.0 || scroll_s < max_scroll - 1.0 {
            let bg = frame_bg(&t);
            let r =
                Rect::from_min_max(pos2(list.min.x + 8.0, list.max.y - 22.0), pos2(list.max.x - 8.0, list.max.y - 1.0));
            if scroll_s < max_scroll - 1.0 {
                paint::quad(
                    &painter,
                    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()],
                    [Color32::TRANSPARENT, Color32::TRANSPARENT, bg, bg],
                );
            }
        }
        response
    }
}
