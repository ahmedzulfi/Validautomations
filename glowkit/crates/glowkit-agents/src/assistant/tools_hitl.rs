use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// The kind of input a [`FormField`] asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// A single-line text box with a placeholder.
    Text(String),
    /// One of a few choices, shown as a segmented control.
    Choice(Vec<String>),
    /// An on/off switch.
    Toggle,
}

/// One field of a [`FormResult`].
#[derive(Clone, Debug, PartialEq)]
pub struct FormField {
    /// Label above (or beside) the input.
    pub label: String,
    /// The input kind.
    pub kind: FieldKind,
}

impl FormField {
    /// A text field.
    pub fn text(label: impl Into<String>, placeholder: impl Into<String>) -> Self {
        Self { label: label.into(), kind: FieldKind::Text(placeholder.into()) }
    }
    /// A segmented choice.
    pub fn choice(label: impl Into<String>, options: &[&str]) -> Self {
        Self { label: label.into(), kind: FieldKind::Choice(options.iter().map(|s| s.to_string()).collect()) }
    }
    /// An on/off switch.
    pub fn toggle(label: impl Into<String>) -> Self {
        Self { label: label.into(), kind: FieldKind::Toggle }
    }
}

/// Caller-owned values of a [`FormResult`]: one string per field (`"true"`/`"false"` for
/// toggles, the chosen option for choices) and whether it was submitted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormState {
    /// Field values, indexed like the fields.
    pub values: Vec<String>,
    /// Set once the user pressed the submit button.
    pub submitted: bool,
    /// Set if the user dismissed the request.
    pub dismissed: bool,
}

/// Human-in-the-loop form: the agent pauses and asks the user to fill a few fields (text, a
/// segmented choice, a switch) before it continues. Focused fields glow softly, the submit button
/// enables once required text is present, and on submit the card folds down into a compact
/// receipt listing what was sent, with a check that draws itself.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{FormField, FormResult, FormState};
/// let fields = [FormField::text("Name", "Ada Lovelace"), FormField::choice("Party", &["2", "4", "6"])];
/// let mut state = FormState::default();
/// FormResult::new("Reserve a table", &fields, &mut state).show(ui);
/// if state.submitted { /* resume the agent with state.values */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FormResult<'a> {
    title: &'a str,
    description: Option<&'a str>,
    fields: &'a [FormField],
    state: &'a mut FormState,
    submit: &'a str,
    width: Option<f32>,
    focus: Option<usize>,
    salt: Option<egui::IdSalt>,
}

impl<'a> FormResult<'a> {
    /// A form titled `title` with `fields`, editing `state`.
    pub fn new(title: &'a str, fields: &'a [FormField], state: &'a mut FormState) -> Self {
        Self { title, description: None, fields, state, submit: "Submit", width: None, focus: None, salt: None }
    }
    /// Muted line under the title explaining why the agent asks.
    pub fn description(mut self, d: &'a str) -> Self {
        self.description = Some(d);
        self
    }
    /// Submit button label (default "Submit").
    pub fn submit_label(mut self, s: &'a str) -> Self {
        self.submit = s;
        self
    }
    /// Card width (default: available, at most 420).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Draw field `i` as focused (for guided tours / autoplay).
    pub fn highlight_field(mut self, i: Option<usize>) -> Self {
        self.focus = i;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the form. Returns the submit button rect (for autoplay) and the card response.
    pub fn show(self, ui: &mut Ui) -> (Response, Rect) {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_form");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(420.0));
        let st = self.state;
        st.values.resize(self.fields.len(), String::new());
        for (i, f) in self.fields.iter().enumerate() {
            if let FieldKind::Choice(o) = &f.kind {
                if st.values[i].is_empty() {
                    st.values[i] = o.first().cloned().unwrap_or_default();
                }
            }
            if f.kind == FieldKind::Toggle && st.values[i].is_empty() {
                st.values[i] = "false".into();
            }
        }
        let e = since_change(ui, id, hash_str(self.title));
        keep_alive(ui, e, 1.0);
        let fold = toggle(ui, id.with("fold"), st.submitted || st.dismissed);
        let mut submit_rect = Rect::NOTHING;
        let status = if st.submitted {
            ("submitted", t.success)
        } else if st.dismissed {
            ("dismissed", t.muted_foreground)
        } else {
            ("needs input", t.warning)
        };
        let head = ToolHead { icon: Icon::Pencil, title: self.title, subtitle: self.description, status: Some(status) };
        let fields = self.fields;
        let focus_hint = self.focus;
        let submit_label = self.submit;
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            if fold > 0.5 {
                // receipt
                let k = ((fold - 0.5) * 2.0).clamp(0.0, 1.0);
                ui.multiply_opacity(k);
                for (i, f) in fields.iter().enumerate() {
                    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
                    let p = ui.painter();
                    text(p, pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, &f.label, 12.0, t.muted_foreground);
                    let v = match (&f.kind, st.values[i].as_str()) {
                        (FieldKind::Toggle, "true") => "Yes",
                        (FieldKind::Toggle, _) => "No",
                        (_, v) if v.is_empty() => "—",
                        (_, v) => v,
                    };
                    text(p, pos2(r.max.x, r.center().y), Align2::RIGHT_CENTER, v, 12.5, t.foreground);
                }
                let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
                let p = ui.painter();
                let ce = since_change(ui, id.with("ck"), st.submitted as u64 + 1);
                keep_alive(ui, ce, 0.8);
                let c = pos2(r.min.x + 10.0, r.center().y + 2.0);
                let col = if st.submitted { t.success } else { t.muted_foreground };
                check_disc(p, c, 9.0, col, Color32::WHITE, (ce * 2.2).clamp(0.0, 1.0));
                let msg = if st.submitted { "Sent to the agent. It will continue with these details." } else { "You dismissed this request." };
                text(p, pos2(c.x + 16.0, c.y), Align2::LEFT_CENTER, msg, 12.0, t.muted_foreground);
                return;
            }
            ui.multiply_opacity(1.0 - fold * 2.0);
            for (i, f) in fields.iter().enumerate() {
                let fid = id.with(("f", i));
                match &f.kind {
                    FieldKind::Text(ph) => {
                        let (lr, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
                        text(ui.painter(), lr.left_center(), Align2::LEFT_CENTER, &f.label, 11.5, t.muted_foreground);
                        let (r, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
                        let te_id = fid.with("te");
                        let has_focus = ui.memory(|m| m.has_focus(te_id)) || focus_hint == Some(i);
                        let fk = toggle(ui, fid.with("fk"), has_focus);
                        let p = ui.painter();
                        if fk > 0.01 {
                            p.rect_stroke(r.expand(2.5), cr(11.0), Stroke::new(3.0, with_alpha(t.ring, 0.18 * fk)), StrokeKind::Inside);
                        }
                        p.rect_filled(r, cr(9.0), well(&t));
                        p.rect_stroke(r, cr(9.0), Stroke::new(1.0, mix(t.border, t.ring, fk)), StrokeKind::Inside);
                        let inner = r.shrink2(vec2(11.0, 0.0));
                        ui.put(
                            inner,
                            egui::TextEdit::singleline(&mut st.values[i])
                                .id(te_id)
                                .hint_text(egui::RichText::new(ph).color(with_alpha(t.muted_foreground, 0.7)))
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::ZERO)
                                .font(FontId::proportional(13.0))
                                .text_color(t.foreground)
                                .align(Align2::LEFT_CENTER)
                                .desired_width(inner.width()),
                        );
                    }
                    FieldKind::Choice(opts) => {
                        let (lr, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
                        text(ui.painter(), lr.left_center(), Align2::LEFT_CENTER, &f.label, 11.5, t.muted_foreground);
                        let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
                        let n = opts.len().max(1);
                        let seg = (r.width() - 6.0) / n as f32;
                        let cur = opts.iter().position(|o| *o == st.values[i]).unwrap_or(0);
                        let x = glowkit_core::anim::spring(ui, fid.with("x"), cur as f32, 380.0, 30.0);
                        let p = ui.painter();
                        p.rect_filled(r, cr(10.0), well(&t));
                        let pill = Rect::from_min_size(pos2(r.min.x + 3.0 + x * seg, r.min.y + 3.0), vec2(seg, r.height() - 6.0));
                        glowkit_core::paint::soft_shadow(p, pill, 8.0, 6.0, vec2(0.0, 1.0), shadow(&t, 0.6));
                        p.rect_filled(pill, cr(8.0), surface(&t));
                        for (k, o) in opts.iter().enumerate() {
                            let sr = Rect::from_min_size(pos2(r.min.x + 3.0 + k as f32 * seg, r.min.y + 3.0), vec2(seg, r.height() - 6.0));
                            let b = ui.interact(sr, fid.with(("o", k)), Sense::click());
                            let on = 1.0 - (x - k as f32).abs().min(1.0);
                            text(ui.painter(), sr.center(), Align2::CENTER_CENTER, o, 12.5, mix(t.muted_foreground, t.foreground, on));
                            if b.clicked() {
                                st.values[i] = o.clone();
                            }
                        }
                    }
                    FieldKind::Toggle => {
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
                        let on = st.values[i] == "true";
                        if resp.clicked() {
                            st.values[i] = (!on).to_string();
                        }
                        let k = glowkit_core::anim::spring(ui, fid.with("sw"), if on { 1.0 } else { 0.0 }, 420.0, 24.0);
                        let p = ui.painter();
                        text(p, pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, &f.label, 12.5, t.foreground);
                        let track = Rect::from_min_size(pos2(r.max.x - 38.0, r.center().y - 11.0), vec2(38.0, 22.0));
                        p.rect_filled(track, cr(11.0), mix(t.input, t.primary, k.clamp(0.0, 1.0)));
                        let kx = track.min.x + 11.0 + k * 16.0;
                        p.circle_filled(pos2(kx, track.center().y), 9.0, if t.dark { mix(t.foreground, t.primary_foreground, k.clamp(0.0, 1.0)) } else { Color32::WHITE });
                    }
                }
            }
            ui.add_space(4.0);
            let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
            let ready = fields.iter().enumerate().all(|(i, f)| !matches!(f.kind, FieldKind::Text(_)) || !st.values[i].trim().is_empty());
            let sw = button_w(ui, submit_label, 32.0, false);
            let sr = Rect::from_min_size(pos2(r.max.x - sw, r.min.y), vec2(sw, 32.0));
            submit_rect = sr;
            let dw = button_w(ui, "Not now", 32.0, false);
            let dr = Rect::from_min_size(pos2(sr.min.x - 6.0 - dw, r.min.y), vec2(dw, 32.0));
            if button(ui, id.with("dismiss"), dr, "Not now", Btn::Ghost, None).clicked() {
                st.dismissed = true;
            }
            let rk = toggle(ui, id.with("ready"), ready);
            ui.multiply_opacity(0.45 + 0.55 * rk);
            if button(ui, id.with("submit"), sr, submit_label, Btn::Primary, None).clicked() && ready {
                st.submitted = true;
            }
        });
        let _ = Vec2::ZERO;
        (ui.interact(rect, id.with("card"), Sense::hover()), submit_rect)
    }
}

/// One choice of an [`OptionListResult`].
#[derive(Clone, Debug, PartialEq)]
pub struct ToolOption {
    /// Title line.
    pub title: String,
    /// Muted detail line.
    pub detail: String,
    /// Optional right-aligned tag (e.g. "Recommended").
    pub tag: Option<String>,
}

impl ToolOption {
    /// An option with a title and detail line.
    pub fn new(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { title: title.into(), detail: detail.into(), tag: None }
    }
    /// A small tag on the right.
    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tag = Some(t.into());
        self
    }
}

/// Caller-owned state of an [`OptionListResult`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionState {
    /// Which options are selected.
    pub selected: Vec<bool>,
    /// Set once the user confirmed.
    pub confirmed: bool,
}

/// The agent asks the user to pick: a list of option rows (single choice with radio dots, or
/// multiple with checkboxes), each with a title, detail and optional tag. Selected rows tint and
/// their indicator springs in; Confirm folds the list down to just the chosen rows as a receipt.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{OptionListResult, OptionState, ToolOption};
/// let opts = [ToolOption::new("Rebase", "Keep history linear"), ToolOption::new("Merge", "Keep both histories")];
/// let mut state = OptionState::default();
/// OptionListResult::new("How should I integrate main?", &opts, &mut state).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct OptionListResult<'a> {
    question: &'a str,
    options: &'a [ToolOption],
    state: &'a mut OptionState,
    multi: bool,
    width: Option<f32>,
    salt: Option<egui::IdSalt>,
}

impl<'a> OptionListResult<'a> {
    /// Ask `question` with `options`.
    pub fn new(question: &'a str, options: &'a [ToolOption], state: &'a mut OptionState) -> Self {
        Self { question, options, state, multi: false, width: None, salt: None }
    }
    /// Allow several selections (checkboxes).
    pub fn multiple(mut self, on: bool) -> Self {
        self.multi = on;
        self
    }
    /// Card width (default: available, at most 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the list. Returns the card response, each row's rect and the confirm button rect.
    pub fn show(self, ui: &mut Ui) -> (Response, Vec<Rect>, Rect) {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_options");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(440.0));
        let st = self.state;
        st.selected.resize(self.options.len(), false);
        let e = since_change(ui, id, hash_str(self.question));
        keep_alive(ui, e, 1.2);
        let done = toggle(ui, id.with("done"), st.confirmed);
        let any = st.selected.iter().any(|s| *s);
        let status = if st.confirmed { ("answered", t.success) } else { ("waiting for you", t.warning) };
        let sub = if self.multi { "Pick any that apply" } else { "Pick one" };
        let head = ToolHead { icon: Icon::Check, title: self.question, subtitle: Some(sub), status: Some(status) };
        let options = self.options;
        let multi = self.multi;
        let mut rows = Vec::new();
        let mut confirm_rect = Rect::NOTHING;
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            for (i, o) in options.iter().enumerate() {
                let sel = st.selected[i];
                // unselected rows collapse away once confirmed
                let hide = if sel { 0.0 } else { done };
                let h = 52.0 * (1.0 - hide);
                if h < 0.5 {
                    rows.push(Rect::NOTHING);
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(vec2(w, h), if st.confirmed { Sense::hover() } else { Sense::click() });
                rows.push(r);
                let rid = id.with(("row", i));
                let hk = toggle(ui, rid.with("h"), resp.hovered() && !st.confirmed);
                let sk = glowkit_core::anim::spring(ui, rid.with("s"), if sel { 1.0 } else { 0.0 }, 380.0, 20.0);
                let k_in = ((e - 0.2 - i as f32 * 0.07) * 5.0).clamp(0.0, 1.0);
                if resp.clicked() && !st.confirmed {
                    if multi {
                        st.selected[i] = !sel;
                    } else {
                        for (j, s) in st.selected.iter_mut().enumerate() {
                            *s = j == i;
                        }
                    }
                }
                let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
                let a = k_in * (1.0 - hide);
                let box_r = Rect::from_min_size(r.min + vec2(0.0, (1.0 - k_in) * 8.0), vec2(w, 46.0));
                p.rect_filled(box_r, cr(11.0), with_alpha(mix(surface(&t), well(&t), hk.max(sk * 0.6)), a));
                p.rect_stroke(box_r, cr(11.0), Stroke::new(1.0 + sk * 0.5, with_alpha(mix(t.border, t.foreground, sk * 0.8), a)), StrokeKind::Inside);
                let ic = pos2(box_r.min.x + 20.0, box_r.center().y);
                if multi {
                    let b = Rect::from_center_size(ic, Vec2::splat(16.0));
                    p.rect_stroke(b, cr(4.5), Stroke::new(1.3, with_alpha(mix(t.muted_foreground, t.primary, sk.clamp(0.0, 1.0)), a)), StrokeKind::Inside);
                    if sk > 0.02 {
                        p.rect_filled(Rect::from_center_size(ic, Vec2::splat(16.0 * sk.clamp(0.0, 1.1))), cr(4.5), with_alpha(t.primary, a));
                        check_disc(&p, ic, 7.5, Color32::TRANSPARENT, with_alpha(t.primary_foreground, a), sk.clamp(0.0, 1.0));
                    }
                } else {
                    p.circle_stroke(ic, 8.0, Stroke::new(1.3, with_alpha(mix(t.muted_foreground, t.primary, sk.clamp(0.0, 1.0)), a)));
                    if sk > 0.02 {
                        p.circle_filled(ic, 4.2 * sk, with_alpha(t.primary, a));
                    }
                }
                let tx = box_r.min.x + 40.0;
                text_bold(&p, pos2(tx, box_r.min.y + 15.0), Align2::LEFT_CENTER, &o.title, 13.0, with_alpha(t.foreground, a));
                text(&p, pos2(tx, box_r.min.y + 32.0), Align2::LEFT_CENTER, &o.detail, 11.5, with_alpha(t.muted_foreground, a));
                if let Some(tag) = &o.tag {
                    let g = galley(ui, tag.as_str(), 10.5, t.accent);
                    let tr = Rect::from_min_size(pos2(box_r.max.x - 12.0 - g.size().x - 14.0, box_r.center().y - 10.0), vec2(g.size().x + 14.0, 20.0));
                    p.rect_filled(tr, cr(10.0), with_alpha(t.accent, 0.1 * a));
                    text(&p, tr.center(), Align2::CENTER_CENTER, tag, 10.5, with_alpha(t.accent, a));
                }
            }
            let fh = 34.0 * (1.0 - done);
            if fh > 0.5 {
                let (r, _) = ui.allocate_exact_size(vec2(w, fh), Sense::hover());
                ui.multiply_opacity(1.0 - done);
                let n = st.selected.iter().filter(|s| **s).count();
                text(ui.painter(), pos2(r.min.x + 2.0, r.center().y), Align2::LEFT_CENTER, &format!("{n} selected"), 11.5, t.muted_foreground);
                let bw = button_w(ui, "Confirm", 32.0, false);
                let br = Rect::from_min_size(pos2(r.max.x - bw, r.center().y - 16.0), vec2(bw, 32.0));
                confirm_rect = br;
                let rk = toggle(ui, id.with("any"), any);
                ui.multiply_opacity(0.4 + 0.6 * rk);
                if button(ui, id.with("confirm"), br, "Confirm", Btn::Primary, None).clicked() && any {
                    st.confirmed = true;
                }
            }
        });
        (ui.interact(rect, id.with("card"), Sense::hover()), rows, confirm_rect)
    }
}
