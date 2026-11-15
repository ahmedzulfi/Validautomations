use super::common::*;
use egui::{pos2, vec2, Align2, CornerRadius, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};
use glowkit_ai::{BranchSwitcher, Icon, Markdown};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// One version of a user turn: the prompt, every reply generated for it, and (stashed) the turns
/// that followed it while another version is active.
#[derive(Clone, Debug, Default)]
pub struct TurnVersion {
    /// What the user asked.
    pub prompt: String,
    /// Every reply generated for this prompt (regenerations add more).
    pub replies: Vec<String>,
    /// Which reply is shown.
    pub reply_ix: usize,
    tail: Vec<ThreadTurn>,
}

/// A user turn and its branches (edited prompts).
#[derive(Clone, Debug, Default)]
pub struct ThreadTurn {
    /// Every version of this turn (editing a prompt adds one).
    pub versions: Vec<TurnVersion>,
    /// The active version.
    pub active: usize,
}

impl ThreadTurn {
    fn v(&self) -> &TurnVersion {
        &self.versions[self.active.min(self.versions.len() - 1)]
    }
}

#[derive(Clone, Debug)]
struct Stream {
    turn: usize,
    shown: f32,
    wait: f32,
}

/// The data behind an [`AssistantThread`]: a flat list of turns where every turn can hold several
/// prompt versions (edit-and-resend) and several replies (regenerate). Switching a prompt version
/// swaps in the conversation that followed that version, exactly like a branching chat tree.
///
/// Replies stream: hand the full text to [`ThreadModel::stream_reply`] (or append chunks with
/// [`ThreadModel::append`]) and the thread reveals it at `chars_per_sec`.
#[derive(Clone, Debug)]
pub struct ThreadModel {
    /// The visible turns, oldest first.
    pub turns: Vec<ThreadTurn>,
    /// Reveal speed of streamed replies.
    pub chars_per_sec: f32,
    stream: Option<Stream>,
    editing: Option<(usize, String)>,
}

impl Default for ThreadModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadModel {
    /// An empty thread.
    pub fn new() -> Self {
        Self { turns: Vec::new(), chars_per_sec: 90.0, stream: None, editing: None }
    }
    /// Add a finished exchange (no streaming), handy for seeding history.
    pub fn with_exchange(mut self, prompt: &str, reply: &str) -> Self {
        self.turns.push(ThreadTurn {
            versions: vec![TurnVersion { prompt: prompt.into(), replies: vec![reply.into()], ..Default::default() }],
            active: 0,
        });
        self
    }
    /// The user sends `prompt`; the thread shows a thinking indicator until a reply arrives.
    pub fn ask(&mut self, prompt: &str) {
        self.finish_stream();
        self.turns.push(ThreadTurn {
            versions: vec![TurnVersion { prompt: prompt.into(), ..Default::default() }],
            active: 0,
        });
        self.stream = Some(Stream { turn: self.turns.len() - 1, shown: 0.0, wait: 0.0 });
    }
    /// Start streaming `text` as a new reply of the last turn (after a short "thinking" pause).
    pub fn stream_reply(&mut self, text: &str) {
        if let Some(turn) = self.turns.len().checked_sub(1) {
            self.regenerate_with(turn, text);
        }
    }
    /// Add `text` as a new reply variant of `turn` and stream it.
    pub fn regenerate_with(&mut self, turn: usize, text: &str) {
        self.finish_stream();
        let Some(tt) = self.turns.get_mut(turn) else { return };
        let v = &mut tt.versions[tt.active];
        v.replies.push(text.into());
        v.reply_ix = v.replies.len() - 1;
        self.stream = Some(Stream { turn, shown: 0.0, wait: 0.55 });
    }
    /// Append a chunk to the reply currently streaming (real backends).
    pub fn append(&mut self, chunk: &str) {
        if let Some(s) = &self.stream {
            let tt = &mut self.turns[s.turn];
            let v = &mut tt.versions[tt.active];
            if v.replies.is_empty() {
                v.replies.push(String::new());
            }
            let ix = v.reply_ix.min(v.replies.len() - 1);
            v.replies[ix].push_str(chunk);
        }
    }
    /// Replace the prompt of `turn` with a new version (the old version and everything after it
    /// stay reachable through the branch picker). Follow with [`ThreadModel::stream_reply`].
    pub fn edit(&mut self, turn: usize, prompt: &str) {
        self.finish_stream();
        if turn >= self.turns.len() {
            return;
        }
        let tail = self.turns.split_off(turn + 1);
        let tt = &mut self.turns[turn];
        let old = tt.active;
        tt.versions[old].tail = tail;
        tt.versions.push(TurnVersion { prompt: prompt.into(), ..Default::default() });
        tt.active = tt.versions.len() - 1;
        self.stream = Some(Stream { turn, shown: 0.0, wait: 0.0 });
    }
    /// Switch `turn` to prompt version `v`, restoring the conversation that followed it.
    pub fn switch_version(&mut self, turn: usize, v: usize) {
        if turn >= self.turns.len() || v >= self.turns[turn].versions.len() || self.turns[turn].active == v {
            return;
        }
        self.finish_stream();
        let tail = self.turns.split_off(turn + 1);
        let tt = &mut self.turns[turn];
        let old = tt.active;
        tt.versions[old].tail = tail;
        let restored = std::mem::take(&mut tt.versions[v].tail);
        tt.active = v;
        self.turns.extend(restored);
    }
    /// True while a reply is being revealed (or awaited).
    pub fn is_streaming(&self) -> bool {
        self.stream.is_some()
    }
    /// Instantly reveal whatever is streaming.
    pub fn finish_stream(&mut self) {
        self.stream = None;
    }
    /// Clear everything.
    pub fn clear(&mut self) {
        self.turns.clear();
        self.stream = None;
        self.editing = None;
    }
    /// Is `turn` being edited inline?
    pub fn is_editing(&self, turn: usize) -> bool {
        self.editing.as_ref().is_some_and(|e| e.0 == turn)
    }
    /// Open the inline editor on `turn` (pre-filled with its prompt).
    pub fn begin_edit(&mut self, turn: usize) {
        if let Some(tt) = self.turns.get(turn) {
            self.editing = Some((turn, tt.v().prompt.clone()));
        }
    }
    /// Text in the inline editor, if open.
    pub fn edit_buffer(&mut self) -> Option<&mut String> {
        self.editing.as_mut().map(|e| &mut e.1)
    }
    /// Submit the inline editor (as if Send was clicked). Returns the edited turn.
    pub fn submit_edit(&mut self) -> Option<usize> {
        let (turn, text) = self.editing.take()?;
        self.edit(turn, &text);
        Some(turn)
    }
}

/// Something the user did in an [`AssistantThread`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadEvent {
    /// Regenerate was clicked on this turn: call [`ThreadModel::regenerate_with`].
    Regenerate(usize),
    /// The prompt of this turn was edited and resent: call [`ThreadModel::stream_reply`].
    Edited(usize),
    /// Copy was clicked on this turn's reply.
    Copied(usize),
    /// Thumbs up (`true`) or down on this turn's reply.
    Feedback(usize, bool),
}

/// What happened in an [`AssistantThread`] this frame.
pub struct ThreadOutput {
    /// Response for the whole thread area.
    pub response: Response,
    /// Clicks and submissions.
    pub events: Vec<ThreadEvent>,
    /// Screen rect of the last reply's regenerate button (for tours and autoplay).
    pub regenerate_rect: Option<Rect>,
    /// Screen rect of the last prompt's edit button.
    pub edit_rect: Option<Rect>,
    /// Screen rect of the inline editor's Send button while editing.
    pub send_edit_rect: Option<Rect>,
}

/// A complete assistant thread: right-aligned user bubbles, assistant replies rendered as streaming
/// Markdown, a branch picker (`‹ 2 / 3 ›`) on edited prompts and regenerated replies, inline
/// edit-and-resend, and a calm action bar (copy, regenerate, feedback). New replies show a
/// breathing "thinking" row, then stream in with a caret and a soft fade on the newest tokens.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{AssistantThread, ThreadEvent, ThreadModel};
/// let mut model = ThreadModel::new().with_exchange("Hi", "Hello! How can I help?");
/// let out = AssistantThread::new(&mut model).height(420.0).show(ui);
/// for e in out.events {
///     if let ThreadEvent::Regenerate(turn) = e {
///         model.regenerate_with(turn, "Another take...");
///     }
/// }
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct AssistantThread<'a> {
    model: &'a mut ThreadModel,
    height: f32,
    text_size: f32,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> AssistantThread<'a> {
    /// A thread showing `model`.
    pub fn new(model: &'a mut ThreadModel) -> Self {
        Self { model, height: 420.0, text_size: 14.0, id_salt: None }
    }
    /// Height of the scrolling viewport (default 420).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(120.0);
        self
    }
    /// Body text size (default 14).
    pub fn text_size(mut self, s: f32) -> Self {
        self.text_size = s.clamp(10.0, 22.0);
        self
    }
    /// Stable id salt (several threads on one screen).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the thread.
    pub fn show(self, ui: &mut Ui) -> ThreadOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.id_salt, "asst_thread");
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let model = self.model;
        // advance streaming
        if let Some(s) = &mut model.stream {
            if s.wait > 0.0 {
                s.wait -= dt;
            } else {
                s.shown += dt * model.chars_per_sec;
            }
            let tt = &model.turns[s.turn.min(model.turns.len().saturating_sub(1))];
            let v = tt.v();
            let full = v.replies.get(v.reply_ix).map(|r| r.chars().count()).unwrap_or(0);
            if full > 0 && s.shown >= full as f32 && s.wait <= 0.0 {
                model.stream = None;
            }
            ui.ctx().request_repaint();
        }
        let w = ui.available_width();
        let mut events = Vec::new();
        let mut regen_rect = None;
        let mut edit_rect = None;
        let mut send_edit_rect = None;
        let n = model.turns.len();
        let resp = egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .max_height(self.height)
            .min_scrolled_height(self.height)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(w - 8.0);
                ui.spacing_mut().item_spacing.y = 14.0;
                ui.add_space(6.0);
                let mut switch: Option<(usize, usize)> = None;
                for ti in 0..n {
                    let last = ti + 1 == n;
                    let tid = id.with(("turn", ti));
                    // ---------------- user bubble ----------------
                    let editing = model.editing.as_ref().is_some_and(|e| e.0 == ti);
                    let tt = &model.turns[ti];
                    let nver = tt.versions.len();
                    let active = tt.active;
                    if editing {
                        let (r_send, done, cancel) =
                            edit_box(ui, &t, tid, model.editing.as_mut().map(|e| &mut e.1).unwrap());
                        send_edit_rect = Some(r_send);
                        if done {
                            if let Some(turn) = model.submit_edit() {
                                events.push(ThreadEvent::Edited(turn));
                            }
                        } else if cancel {
                            model.editing = None;
                        }
                    } else {
                        let prompt = model.turns[ti].v().prompt.clone();
                        let (er, edit_clicked, sw) = user_bubble(ui, &t, tid, &prompt, active, nver, self.text_size);
                        if last {
                            edit_rect = Some(er);
                        }
                        if edit_clicked {
                            model.begin_edit(ti);
                        }
                        if let Some(v) = sw {
                            switch = Some((ti, v));
                        }
                    }
                    // ---------------- assistant reply ----------------
                    let streaming_here = model.stream.as_ref().filter(|s| s.turn == ti).map(|s| (s.shown, s.wait));
                    let v = model.turns[ti].v().clone();
                    let reply_full = v.replies.get(v.reply_ix).cloned().unwrap_or_default();
                    let shown: &str = match streaming_here {
                        Some((s, _)) => take_chars(&reply_full, s as usize),
                        None => &reply_full,
                    };
                    let thinking = streaming_here.is_some() && shown.is_empty();
                    if reply_full.is_empty() && streaming_here.is_none() {
                        continue;
                    }
                    let key = hash_str(&reply_full) ^ (v.reply_ix as u64) ^ ((model.turns[ti].active as u64) << 20);
                    let e = since_change(ui, tid.with("reply"), key);
                    keep_alive(ui, e, 0.8);
                    let mut reply_ix = v.reply_ix;
                    let row = ui.horizontal_top(|ui| {
                        let (ar, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                        bot_avatar(ui.painter(), &t, ar.center(), 13.0);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.set_width(w - 60.0);
                            ui.spacing_mut().item_spacing.y = 6.0;
                            ui.add_space(4.0);
                            if thinking {
                                thinking_row(ui, &t, time);
                            } else {
                                let fade = (e * 4.0).clamp(0.0, 1.0);
                                ui.multiply_opacity(if streaming_here.is_some() { 1.0 } else { 0.35 + 0.65 * fade });
                                ui.add(
                                    Markdown::new(shown)
                                        .size(self.text_size)
                                        .caret(streaming_here.is_some())
                                        .fade_tail(if streaming_here.is_some() { 26 } else { 0 }),
                                );
                            }
                            if streaming_here.is_none() {
                                let nrep = v.replies.len();
                                let hover_k = if last { 1.0 } else { 0.0 };
                                let bar = ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if nrep > 1 {
                                        ui.add(BranchSwitcher::new(&mut reply_ix, nrep));
                                    }
                                    let mut rr = None;
                                    for (k, ic) in
                                        [Icon::Copy, Icon::Refresh, Icon::ThumbUp, Icon::ThumbDown].iter().enumerate()
                                    {
                                        let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                                        let b = icon_button(ui, tid.with(("act", k)), r.shrink(1.0), *ic, false);
                                        if k == 1 {
                                            rr = Some(r);
                                        }
                                        if b.clicked() {
                                            events.push(match k {
                                                0 => ThreadEvent::Copied(ti),
                                                1 => ThreadEvent::Regenerate(ti),
                                                2 => ThreadEvent::Feedback(ti, true),
                                                _ => ThreadEvent::Feedback(ti, false),
                                            });
                                            if k == 0 {
                                                ui.ctx().copy_text(reply_full.clone());
                                            }
                                        }
                                    }
                                    rr
                                });
                                let _ = hover_k;
                                if last {
                                    regen_rect = bar.inner;
                                }
                            }
                        });
                    });
                    let _ = row;
                    if reply_ix != v.reply_ix {
                        let tt = &mut model.turns[ti];
                        let a = tt.active;
                        tt.versions[a].reply_ix = reply_ix;
                    }
                }
                if let Some((ti, v)) = switch {
                    model.switch_version(ti, v);
                }
                ui.add_space(8.0);
            });
        let response = ui.interact(resp.inner_rect, id.with("area"), Sense::hover());
        ThreadOutput { response, events, regenerate_rect: regen_rect, edit_rect, send_edit_rect }
    }
}

impl Widget for AssistantThread<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// Right-aligned user bubble with an edit button and a version picker. Returns (edit button
/// rect, edit clicked, switch-to version).
fn user_bubble(
    ui: &mut Ui,
    t: &Theme,
    id: Id,
    prompt: &str,
    active: usize,
    nver: usize,
    size: f32,
) -> (Rect, bool, Option<usize>) {
    let w = ui.available_width();
    let max_w = (w * 0.72).max(140.0);
    let pad = vec2(15.0, 10.0);
    let g = ui.painter().layout(prompt.to_owned(), FontId::proportional(size), t.foreground, max_w - pad.x * 2.0);
    let bs = g.size() + pad * 2.0;
    let foot = if nver > 1 { 28.0 } else { 0.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(w, bs.y + foot), Sense::hover());
    let bubble = Rect::from_min_size(pos2(rect.max.x - bs.x, rect.min.y), bs);
    let hovered = ui.rect_contains_pointer(rect);
    let hk = toggle(ui, id.with("uh"), hovered);
    let p = ui.painter();
    let fill = if t.dark { t.secondary } else { well(t) };
    let rad = CornerRadius { nw: 18, ne: 18, sw: 18, se: 6 };
    p.rect_filled(bubble, rad, fill);
    p.galley(bubble.min + pad, g, t.foreground);
    // edit button to the left of the bubble
    let er = Rect::from_center_size(pos2(bubble.min.x - 20.0, bubble.min.y + 18.0), Vec2::splat(28.0));
    let mut clicked = false;
    {
        let b = ui.interact(er, id.with("edit"), Sense::click());
        let bh = toggle(ui, id.with("eh"), b.hovered());
        let p = ui.painter();
        let op = (0.25 + 0.75 * hk.max(bh)).clamp(0.0, 1.0);
        if bh > 0.01 {
            p.rect_filled(er, egui::CornerRadius::same(14), with_alpha(well(t), bh));
        }
        icon(p, Icon::Pencil, er.center(), 14.0, with_alpha(mix(t.muted_foreground, t.foreground, bh), op));
        if b.clicked() {
            clicked = true;
        }
    }
    let mut sw = None;
    if nver > 1 {
        let mut cur = active;
        let bw = 24.0 * 2.0 + text_w(ui, &format!("{} / {}", active + 1, nver), 12.0) + 8.0;
        let r = Rect::from_min_size(pos2(bubble.max.x - bw, bubble.max.y + 1.0), vec2(bw, 28.0));
        ui.put(r, BranchSwitcher::new(&mut cur, nver));
        if cur != active {
            sw = Some(cur);
        }
    }
    (er, clicked, sw)
}

/// Inline editor replacing a user bubble. Returns (send rect, send clicked, cancel clicked).
fn edit_box(ui: &mut Ui, t: &Theme, id: Id, buf: &mut String) -> (Rect, bool, bool) {
    let w = ui.available_width();
    let bw = (w * 0.78).max(220.0);
    let (rect, _) = ui.allocate_exact_size(vec2(w, 98.0), Sense::hover());
    let r = Rect::from_min_size(pos2(rect.max.x - bw, rect.min.y), vec2(bw, 98.0));
    let k = toggle(ui, id.with("open"), true);
    let p = ui.painter();
    p.rect_filled(r, egui::CornerRadius::same(16), surface(t));
    p.rect_stroke(
        r,
        egui::CornerRadius::same(16),
        Stroke::new(1.0 + 0.5 * k, mix(t.border, t.ring, 0.6 * k)),
        StrokeKind::Inside,
    );
    let te = Rect::from_min_max(r.min + vec2(14.0, 10.0), pos2(r.max.x - 14.0, r.max.y - 42.0));
    let resp = ui.put(
        te,
        egui::TextEdit::multiline(buf)
            .id(id.with("te"))
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(FontId::proportional(14.0))
            .text_color(t.foreground)
            .desired_rows(2)
            .desired_width(te.width()),
    );
    let enter = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
    let sw = button_w(ui, "Send", 28.0, false);
    let send = Rect::from_min_size(pos2(r.max.x - 10.0 - sw, r.max.y - 38.0), vec2(sw, 28.0));
    let cw = button_w(ui, "Cancel", 28.0, false);
    let cancel = Rect::from_min_size(pos2(send.min.x - 6.0 - cw, send.min.y), vec2(cw, 28.0));
    let c = button(ui, id.with("cancel"), cancel, "Cancel", Btn::Ghost, None).clicked();
    let s = button(ui, id.with("send"), send, "Send", Btn::Primary, None).clicked();
    (send, s || enter, c)
}

/// "Thinking" row: three breathing dots and a shimmering label.
pub(crate) fn thinking_row(ui: &mut Ui, t: &Theme, time: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(160.0, 22.0), Sense::hover());
    let p = ui.painter();
    for i in 0..3 {
        let ph = (time * 5.0 - i as f32 * 0.7).sin() * 0.5 + 0.5;
        p.circle_filled(
            pos2(r.min.x + 5.0 + i as f32 * 10.0, r.center().y - ph * 2.5),
            3.0,
            with_alpha(t.muted_foreground, 0.35 + 0.55 * ph),
        );
    }
    let g = galley(ui, "Thinking", 13.0, t.muted_foreground);
    let x0 = r.min.x + 36.0;
    let sweep = (time * 0.8).fract() * 1.6 - 0.3;
    glowkit_core::paint::galley_with(p, pos2(x0, r.center().y - g.size().y / 2.0), &g, |q| {
        let u = (q.x - x0) / g.size().x;
        let k = (1.0 - ((u - sweep) * 5.0).abs()).clamp(0.0, 1.0);
        mix(t.muted_foreground, t.foreground, k)
    });
    ui.ctx().request_repaint();
    let _ = Align2::LEFT_CENTER;
}
