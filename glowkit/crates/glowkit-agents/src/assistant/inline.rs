use super::common::*;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Galley, Key, Modifiers, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::{Icon, Markdown};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::sync::Arc;

#[derive(Clone, Copy, Default)]
struct GhostState {
    text_hash: u64,
    changed_at: f64,
    dismissed: u64,
    accepted_at: f64,
    accepted: (usize, usize),
}

/// What happened in an [`InlineSuggestion`] this frame.
pub struct InlineSuggestionOutput {
    /// The text edit's response.
    pub response: Response,
    /// The suggestion was accepted (Tab) this frame.
    pub accepted: bool,
    /// The ghost text is currently visible.
    pub showing: bool,
}

/// Copilot-style ghost-text autocomplete in a multi-line field. After a short pause in typing the
/// suggestion fades in after your text in a muted color with a small `Tab` key hint; Tab accepts it
/// (the accepted words flash with a soft highlight that fades), Esc dismisses it, and typing on
/// simply updates the request. You provide the suggestion for the current text.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut text = String::from("Thanks for the quick turnaround on ");
/// let ghost = Some("the pricing page, it reads much better now.");
/// let out = glowkit_agents::InlineSuggestion::new(&mut text, ghost).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct InlineSuggestion<'a> {
    text: &'a mut String,
    suggestion: Option<&'a str>,
    delay: f32,
    width: Option<f32>,
    rows: usize,
    size: f32,
    accept_now: bool,
    placeholder: &'a str,
    salt: Option<egui::IdSalt>,
}

impl<'a> InlineSuggestion<'a> {
    /// A field editing `text` that offers `suggestion` as a continuation.
    pub fn new(text: &'a mut String, suggestion: Option<&'a str>) -> Self {
        Self { text, suggestion, delay: 0.45, width: None, rows: 4, size: 15.0, accept_now: false, placeholder: "Start writing…", salt: None }
    }
    /// Pause after typing before the ghost appears (default 0.45 s).
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s.max(0.0);
        self
    }
    /// Width (default: available, at most 600).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Minimum rows (default 4).
    pub fn rows(mut self, r: usize) -> Self {
        self.rows = r.max(1);
        self
    }
    /// Font size (default 15).
    pub fn text_size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Accept the visible suggestion this frame (as if Tab was pressed).
    pub fn accept_now(mut self, on: bool) -> Self {
        self.accept_now = on;
        self
    }
    /// Placeholder text.
    pub fn placeholder(mut self, p: &'a str) -> Self {
        self.placeholder = p;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the field.
    pub fn show(self, ui: &mut Ui) -> InlineSuggestionOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_inline");
        let te_id = id.with("te");
        let now = ui.input(|i| i.time);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(600.0));
        let mut gs: GhostState = glowkit_core::state(ui, id.with("g"), GhostState::default);
        let h = hash_str(self.text);
        if h != gs.text_hash {
            gs.text_hash = h;
            gs.changed_at = now;
        }
        let waited = (now - gs.changed_at) as f32;
        let sugg = self.suggestion.filter(|s| !s.is_empty() && gs.dismissed != h);
        let visible = sugg.is_some() && waited >= self.delay;
        if sugg.is_some() && !visible {
            ui.ctx().request_repaint();
        }
        let focused = ui.memory(|m| m.has_focus(te_id));
        let mut accept = self.accept_now && visible;
        if focused && visible {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::Tab) {
                    accept = true;
                }
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    gs.dismissed = h;
                }
            });
        }
        if accept {
            if let Some(s) = sugg {
                let start = self.text.chars().count();
                self.text.push_str(s);
                gs.accepted = (start, start + s.chars().count());
                gs.accepted_at = now;
                gs.text_hash = hash_str(self.text);
                gs.changed_at = now;
                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), te_id) {
                    let n = self.text.chars().count();
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(n))));
                    st.store(ui.ctx(), te_id);
                }
            }
        }
        let flash = (1.0 - ((now - gs.accepted_at) as f32 / 1.1)).clamp(0.0, 1.0);
        if flash > 0.0 {
            ui.ctx().request_repaint();
        }
        let gk = if visible && !accept { ((waited - self.delay) * 4.0).clamp(0.0, 1.0) } else { 0.0 };
        if gk > 0.0 && gk < 1.0 {
            ui.ctx().request_repaint();
        }
        let size = self.size;
        let fk = toggle(ui, id.with("f"), focused);
        let start = ui.cursor().min;
        let bg = ui.painter().add(egui::Shape::Noop);
        let pad = vec2(18.0, 16.0);
        let inner_w = w - pad.x * 2.0;
        let acc = gs.accepted;
        let tc = t.clone();
        let mut layouter = move |ui: &Ui, buf: &dyn egui::TextBuffer, wrap: f32| -> Arc<Galley> {
            let s = buf.as_str();
            let mut job = LayoutJob::default();
            let base = TextFormat { font_id: FontId::proportional(size), color: tc.foreground, line_height: Some(size * 1.55), ..Default::default() };
            if flash > 0.0 && acc.1 <= s.chars().count() && acc.0 < acc.1 {
                let b0 = s.char_indices().nth(acc.0).map(|x| x.0).unwrap_or(s.len());
                let b1 = s.char_indices().nth(acc.1).map(|x| x.0).unwrap_or(s.len());
                job.append(&s[..b0], 0.0, base.clone());
                let hl = TextFormat { background: with_alpha(tc.accent, 0.18 * flash), color: mix(tc.foreground, tc.accent, 0.6 * flash), ..base.clone() };
                job.append(&s[b0..b1], 0.0, hl);
                job.append(&s[b1..], 0.0, base);
            } else {
                job.append(s, 0.0, base);
            }
            job.wrap.max_width = wrap;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let text = &mut *self.text;
        let placeholder = self.placeholder;
        let rows = self.rows;
        let te = ui
            .scope_builder(
                egui::UiBuilder::new().max_rect(Rect::from_min_size(start + pad, vec2(inner_w, 4000.0))).layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    egui::TextEdit::multiline(text)
                        .id(te_id)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_rows(rows)
                        .desired_width(inner_w)
                        .lock_focus(true)
                        .hint_text(egui::RichText::new(placeholder).size(size).color(with_alpha(t.muted_foreground, 0.7)))
                        .layouter(&mut layouter)
                        .show(ui)
                },
            )
            .inner;
        let rect = Rect::from_min_max(start, pos2(start.x + w, te.response.response.rect.max.y + pad.y));
        ui.allocate_rect(rect, Sense::hover());
        // ghost text
        if gk > 0.0 {
            if let Some(s) = sugg {
                let mut job = LayoutJob::default();
                let invis = TextFormat { font_id: FontId::proportional(size), color: Color32::TRANSPARENT, line_height: Some(size * 1.55), ..Default::default() };
                job.append(self.text.as_str(), 0.0, invis.clone());
                job.append(s, 0.0, TextFormat { color: with_alpha(t.muted_foreground, 0.75 * gk), ..invis });
                job.wrap.max_width = te.galley.job.wrap.max_width;
                let g = ui.painter().layout_job(job);
                let end = g.rows.last().map(|r| te.galley_pos + vec2(r.pos.x + r.row.size.x, r.pos.y + r.row.size.y / 2.0));
                ui.painter().galley(te.galley_pos, g, Color32::TRANSPARENT);
                if let Some(e) = end {
                    let p = ui.painter();
                    let kr = kbd(p, &t, ui, e + vec2(40.0, 0.0), "Tab");
                    let _ = kr;
                }
            }
        }
        let p = ui.painter();
        let mut shapes = vec![];
        let sh = egui::epaint::Shadow { offset: [0, 3], blur: 16, spread: 0, color: shadow(&t, 0.6 + 0.6 * fk) };
        shapes.push(sh.as_shape(rect, cr(16.0)).into());
        shapes.push(egui::Shape::rect_filled(rect, cr(16.0), surface(&t)));
        shapes.push(egui::Shape::rect_stroke(rect, cr(16.0), Stroke::new(1.0, mix(hairline(&t).color, t.ring, 0.5 * fk)), StrokeKind::Inside));
        p.set(bg, egui::Shape::Vec(shapes));
        glowkit_core::set_state(ui, id.with("g"), gs);
        InlineSuggestionOutput { response: te.response.response, accepted: accept, showing: gk > 0.0 }
    }
}

/// A quick action offered by a [`SelectionAskBar`]: its label and the answer it streams.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionAction {
    /// Button label.
    pub label: String,
    /// Icon.
    pub icon: Icon,
    /// The reply shown (streamed) when chosen.
    pub answer: String,
}

impl SelectionAction {
    /// An action that answers with `answer`.
    pub fn new(label: impl Into<String>, icon: Icon, answer: impl Into<String>) -> Self {
        Self { label: label.into(), icon, answer: answer.into() }
    }
}

#[derive(Clone, Copy, Default)]
struct SelState {
    sel: Option<(usize, usize)>,
    anchor: Option<usize>,
    action: Option<usize>,
    action_at: f64,
    shown_at: f64,
}

/// What happened in a [`SelectionAskBar`] this frame.
pub struct SelectionOutput {
    /// Response of the text area.
    pub response: Response,
    /// Selected character range, if any.
    pub selection: Option<(usize, usize)>,
    /// Rects of the toolbar buttons (for autoplay), empty when hidden.
    pub buttons: Vec<Rect>,
    /// Map a character index to a screen position (top-left of that glyph).
    pub char_pos: Vec<Pos2>,
}

/// "Ask AI" on selected text: drag across a paragraph to select it (soft accent highlight) and a
/// floating toolbar pops up above the selection with a gradient "Ask AI" button and quick actions
/// (Explain, Improve, Translate...). Choosing one opens a small answer card under the selection
/// whose reply streams in, with Replace / Copy buttons.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{SelectionAction, SelectionAskBar};
/// use glowkit_ai::Icon;
/// let actions = [SelectionAction::new("Explain", Icon::Lightbulb, "It means...")];
/// SelectionAskBar::new("Some paragraph of text to select.", &actions).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SelectionAskBar<'a> {
    text: &'a str,
    actions: &'a [SelectionAction],
    width: Option<f32>,
    size: f32,
    select: Option<Option<(usize, usize)>>,
    trigger: Option<usize>,
    salt: Option<egui::IdSalt>,
}

impl<'a> SelectionAskBar<'a> {
    /// A selectable paragraph with quick `actions`.
    pub fn new(text: &'a str, actions: &'a [SelectionAction]) -> Self {
        Self { text, actions, width: None, size: 15.0, select: None, trigger: None, salt: None }
    }
    /// Width (default: available, at most 620).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Font size (default 15).
    pub fn text_size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Set the selection programmatically this frame (for autoplay).
    pub fn select(mut self, range: Option<(usize, usize)>) -> Self {
        self.select = Some(range);
        self
    }
    /// Run action `i` this frame (for autoplay).
    pub fn trigger(mut self, i: Option<usize>) -> Self {
        self.trigger = i;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the paragraph, toolbar and answer card.
    pub fn show(self, ui: &mut Ui) -> SelectionOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_selection");
        let now = ui.input(|i| i.time);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(620.0));
        let g = ui.painter().layout(self.text.to_owned(), FontId::proportional(self.size), t.foreground, w);
        let extra = 190.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, g.size().y + extra), Sense::click_and_drag());
        let text_rect = Rect::from_min_size(rect.min + vec2(0.0, 56.0), g.size());
        let mut st: SelState = glowkit_core::state(ui, id.with("s"), SelState::default);
        // glyph positions (char index -> rect)
        let mut glyphs: Vec<Rect> = Vec::new();
        for row in &g.rows {
            for gl in &row.row.glyphs {
                glyphs.push(gl.logical_rect().translate(row.pos.to_vec2() + text_rect.min.to_vec2()));
            }
            if row.ends_with_newline {
                glyphs.push(Rect::from_min_size(text_rect.min + row.pos.to_vec2() + vec2(row.row.size.x, 0.0), vec2(0.0, row.row.size.y)));
            }
        }
        let char_at = |p: Pos2| -> usize { g.cursor_from_pos(p - text_rect.min).index.0 };
        if let Some(s) = self.select {
            if s != st.sel {
                st.sel = s;
                st.action = None;
                st.shown_at = now;
            }
        }
        let ptr = ui.input(|i| i.pointer.interact_pos());
        if resp.drag_started() {
            if let Some(p) = ptr {
                if text_rect.expand(6.0).contains(p) {
                    let c = char_at(p);
                    st.anchor = Some(c);
                    st.sel = None;
                    st.action = None;
                }
            }
        }
        if resp.dragged() {
            if let (Some(a), Some(p)) = (st.anchor, ptr) {
                let c = char_at(p);
                if c != a {
                    st.sel = Some((a.min(c), a.max(c)));
                    st.shown_at = now;
                }
            }
        }
        if resp.drag_stopped() {
            st.anchor = None;
        }
        if resp.clicked() {
            st.sel = None;
            st.action = None;
        }
        if let Some(i) = self.trigger {
            if st.action != Some(i) && st.sel.is_some() {
                st.action = Some(i);
                st.action_at = now;
            }
        }
        let p = ui.painter();
        // selection highlight
        let mut sel_rects: Vec<Rect> = Vec::new();
        if let Some((a, b)) = st.sel {
            for gr in glyphs.iter().take(b.min(glyphs.len())).skip(a) {
                match sel_rects.last_mut() {
                    Some(l) if (l.center().y - gr.center().y).abs() < 2.0 => *l = l.union(*gr),
                    _ => sel_rects.push(*gr),
                }
            }
            let k = ((now - st.shown_at) as f32 * 8.0).clamp(0.0, 1.0);
            for r in &sel_rects {
                p.rect_filled(r.expand2(vec2(1.0, 0.0)), cr(3.0), with_alpha(t.accent, (if t.dark { 0.32 } else { 0.18 }) * k));
            }
        }
        p.galley(text_rect.min, g.clone(), t.foreground);
        let mut buttons = Vec::new();
        let tb_k = toggle(ui, id.with("tb"), st.sel.is_some() && st.anchor.is_none());
        if let (Some(first), true) = (sel_rects.first().copied(), tb_k > 0.01) {
            // toolbar above the selection
            let labels: Vec<(String, Icon)> = std::iter::once(("Ask AI".to_owned(), Icon::Sparkle)).chain(self.actions.iter().map(|a| (a.label.clone(), a.icon))).collect();
            let widths: Vec<f32> = labels.iter().map(|(l, _)| text_w(ui, l, 12.5) + 36.0).collect();
            let tw: f32 = widths.iter().sum::<f32>() + 8.0;
            let x = (first.min.x - 6.0).clamp(rect.min.x, rect.max.x - tw);
            let tbr = Rect::from_min_size(pos2(x, first.min.y - 46.0 + (1.0 - tb_k) * 6.0), vec2(tw, 36.0));
            let p = ui.painter();
            paint::soft_shadow(p, tbr, 12.0, 18.0, vec2(0.0, 6.0), shadow(&t, 1.3 * tb_k));
            p.rect_filled(tbr, cr(12.0), t.popover.gamma_multiply(tb_k));
            p.rect_stroke(tbr, cr(12.0), Stroke::new(1.0, hairline(&t).color.gamma_multiply(tb_k)), StrokeKind::Inside);
            let mut bx = tbr.min.x + 4.0;
            for (k, (l, ic)) in labels.iter().enumerate() {
                let br = Rect::from_min_size(pos2(bx, tbr.min.y + 4.0), vec2(widths[k], 28.0));
                bx += widths[k];
                buttons.push(br);
                let b = ui.interact(br, id.with(("b", k)), Sense::click());
                let hk = toggle(ui, id.with(("bh", k)), b.hovered() || st.action.map(|a| a + 1) == Some(k));
                let p = ui.painter();
                if k == 0 {
                    let fill = [t.gradient[0], t.gradient[1]];
                    paint::gradient_rect(p, br, 9.0, 0.0, &[with_alpha(fill[0], 0.12 + 0.1 * hk), with_alpha(fill[1], 0.12 + 0.1 * hk)]);
                    icon(p, *ic, pos2(br.min.x + 15.0, br.center().y), 13.0, t.accent.gamma_multiply(tb_k));
                    text_bold(p, pos2(br.min.x + 26.0, br.center().y), Align2::LEFT_CENTER, l, 12.5, mix(t.foreground, t.accent, 0.5).gamma_multiply(tb_k));
                } else {
                    if hk > 0.01 {
                        p.rect_filled(br, cr(9.0), with_alpha(well(&t), hk * tb_k));
                    }
                    icon(p, *ic, pos2(br.min.x + 15.0, br.center().y), 13.0, t.muted_foreground.gamma_multiply(tb_k));
                    text(p, pos2(br.min.x + 26.0, br.center().y), Align2::LEFT_CENTER, l, 12.5, t.foreground.gamma_multiply(tb_k));
                }
                if b.clicked() && !self.actions.is_empty() {
                    st.action = Some(if k == 0 { 0 } else { k - 1 });
                    st.action_at = now;
                }
            }
        }
        // answer card under the selection
        let ak = toggle(ui, id.with("ans"), st.action.is_some() && st.sel.is_some());
        if ak > 0.01 {
            if let (Some(a), Some(last)) = (st.action, sel_rects.last().copied()) {
                let act = &self.actions[a.min(self.actions.len() - 1)];
                let cw = (w * 0.78).min(440.0);
                let x = (last.min.x - 40.0).clamp(rect.min.x, rect.max.x - cw);
                let el = (now - st.action_at) as f32;
                let n = ((el - 0.5).max(0.0) * 70.0) as usize;
                let shown = take_chars(&act.answer, n);
                let streaming = shown.len() < act.answer.len();
                if streaming || el < 0.6 {
                    ui.ctx().request_repaint();
                }
                let card = Rect::from_min_size(pos2(x, text_rect.max.y + 12.0 + (1.0 - ak) * 8.0), vec2(cw, 140.0));
                let p = ui.painter();
                paint::soft_shadow(p, card, 14.0, 22.0, vec2(0.0, 8.0), shadow(&t, 1.4 * ak));
                p.rect_filled(card, cr(14.0), t.popover.gamma_multiply(ak));
                p.rect_stroke(card, cr(14.0), Stroke::new(1.0, hairline(&t).color.gamma_multiply(ak)), StrokeKind::Inside);
                bot_avatar(p, &t, pos2(card.min.x + 20.0, card.min.y + 20.0), 9.0);
                text_bold(p, pos2(card.min.x + 36.0, card.min.y + 20.0), Align2::LEFT_CENTER, &act.label, 12.5, t.foreground.gamma_multiply(ak));
                let body = Rect::from_min_max(card.min + vec2(14.0, 36.0), card.max - vec2(14.0, 40.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
                child.set_clip_rect(body.intersect(ui.clip_rect()));
                child.multiply_opacity(ak);
                if shown.is_empty() {
                    super::thread::thinking_row(&mut child, &t, glowkit_core::time(ui));
                } else {
                    child.add(Markdown::new(shown).size(13.0).caret(streaming).fade_tail(if streaming { 20 } else { 0 }));
                }
                if !streaming {
                    let by = card.max.y - 22.0;
                    let rw = button_w(ui, "Replace", 26.0, false);
                    let rr = Rect::from_min_size(pos2(card.max.x - 12.0 - rw, by - 13.0), vec2(rw, 26.0));
                    if button(ui, id.with("replace"), rr, "Replace", Btn::Primary, None).clicked() {
                        st.action = None;
                        st.sel = None;
                    }
                    let cw2 = button_w(ui, "Copy", 26.0, false);
                    let cr2 = Rect::from_min_size(pos2(rr.min.x - 6.0 - cw2, by - 13.0), vec2(cw2, 26.0));
                    if button(ui, id.with("copy"), cr2, "Copy", Btn::Ghost, None).clicked() {
                        ui.ctx().copy_text(act.answer.clone());
                    }
                }
            }
        }
        let sel = st.sel;
        glowkit_core::set_state(ui, id.with("s"), st);
        let char_pos = glyphs.iter().map(|r| r.left_center()).collect();
        let _ = Vec2::ZERO;
        SelectionOutput { response: resp, selection: sel, buttons, char_pos }
    }
}
