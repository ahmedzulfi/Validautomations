//! The chat composer.

use crate::common;
use crate::icons::{paint_icon, Icon};
use egui::{
    pos2, vec2, CornerRadius, FontId, Key, KeyboardShortcut, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder,
};
use glowkit_core::color::{gradient_cyclic, mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Kind of an [`Attachment`]; picks the chip icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentKind {
    /// A generic file.
    #[default]
    File,
    /// An image.
    Image,
    /// Source code.
    Code,
    /// A web page.
    Link,
}

/// A file attached to a prompt, shown as a removable chip.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attachment {
    /// File name.
    pub name: String,
    /// Kind (icon).
    pub kind: AttachmentKind,
    /// Optional size / detail label, e.g. "24 KB".
    pub detail: String,
}

impl Attachment {
    /// A new attachment.
    pub fn new(name: impl Into<String>, kind: AttachmentKind) -> Self {
        Self { name: name.into(), kind, detail: String::new() }
    }
    /// Size / detail label.
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = d.into();
        self
    }
}

/// What the user did in a [`PromptInput`] this frame.
#[derive(Debug)]
pub struct PromptOutput {
    /// The text edit's response.
    pub response: Response,
    /// The submitted prompt (Enter or the send button). The input is cleared.
    pub submitted: Option<String>,
    /// The stop button was clicked while generating.
    pub stop: bool,
    /// The attach (+) button was clicked.
    pub attach: bool,
    /// The mic button was clicked.
    pub mic: bool,
    /// Index of an attachment removed this frame.
    pub removed: Option<usize>,
}

/// The chat composer: a rounded, auto-growing multiline input with placeholder, attachment chips,
/// a toolbar (attach, model pill or any custom widgets, character count, mic) and a send button
/// that turns into a stop button while the model is generating. Enter sends, Shift+Enter adds a
/// newline. When focused, a soft animated gradient glow wraps the box.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut text = String::new();
/// let mut generating = false;
/// let out = glowkit_ai::PromptInput::new(&mut text)
///     .placeholder("Ask anything…")
///     .generating(generating)
///     .show(ui, |_toolbar| {});
/// if let Some(prompt) = out.submitted {
///     generating = true; // send `prompt` to the model
/// }
/// ```
#[must_use = "call `.show(ui, ..)`"]
pub struct PromptInput<'a> {
    text: &'a mut String,
    placeholder: &'a str,
    attachments: Option<&'a mut Vec<Attachment>>,
    generating: bool,
    max_chars: Option<usize>,
    max_height: f32,
    min_rows: usize,
    glow: bool,
    attach_button: bool,
    mic_button: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> PromptInput<'a> {
    /// A composer editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: "Ask anything…",
            attachments: None,
            generating: false,
            max_chars: None,
            max_height: 200.0,
            min_rows: 1,
            glow: true,
            attach_button: true,
            mic_button: false,
            id_salt: None,
        }
    }
    /// Placeholder text.
    pub fn placeholder(mut self, p: &'a str) -> Self {
        self.placeholder = p;
        self
    }
    /// Attachment chips shown above the text (removable).
    pub fn attachments(mut self, a: &'a mut Vec<Attachment>) -> Self {
        self.attachments = Some(a);
        self
    }
    /// The model is generating: the send button becomes a stop button.
    pub fn generating(mut self, on: bool) -> Self {
        self.generating = on;
        self
    }
    /// Show "n / max" and block sending beyond it.
    pub fn max_chars(mut self, n: usize) -> Self {
        self.max_chars = Some(n);
        self
    }
    /// Height after which the text scrolls instead of growing.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = h;
        self
    }
    /// Minimum number of text rows.
    pub fn min_rows(mut self, rows: usize) -> Self {
        self.min_rows = rows.max(1);
        self
    }
    /// Animated gradient glow while focused (default on).
    pub fn glow(mut self, on: bool) -> Self {
        self.glow = on;
        self
    }
    /// Show the attach (+) button (default on).
    pub fn attach_button(mut self, on: bool) -> Self {
        self.attach_button = on;
        self
    }
    /// Show a mic button next to send.
    pub fn mic_button(mut self, on: bool) -> Self {
        self.mic_button = on;
        self
    }
    /// Stable id.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the composer. `toolbar` adds extra widgets (e.g. a [`crate::ModelSelector`]) to the
    /// left part of the bottom toolbar.
    pub fn show(mut self, ui: &mut Ui, toolbar: impl FnOnce(&mut Ui)) -> PromptOutput {
        let t = Theme::of(ui);
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.scope_id().with("prompt_input"));
        let te_id = id.with("te");
        let width = ui.available_width();
        let top = ui.cursor().min;
        let cr = CornerRadius::same(22);
        let focused_prev = ui.memory(|m| m.has_focus(te_id));
        // Enter sends; Shift+Enter is the TextEdit's newline key.
        let enter = focused_prev && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let glow_idx = ui.painter().add(egui::Shape::Noop);
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let mut out = PromptOutput {
            response: ui.allocate_response(vec2(0.0, 0.0), Sense::hover()),
            submitted: None,
            stop: false,
            attach: false,
            mic: false,
            removed: None,
        };
        let pad = 14.0;
        let inner = Rect::from_min_max(top + vec2(pad, 12.0), pos2(top.x + width - pad, f32::INFINITY));
        let r =
            ui.scope_builder(UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                // attachments
                if let Some(atts) = self.attachments.as_deref_mut() {
                    if !atts.is_empty() {
                        let mut remove = None;
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                            for (i, a) in atts.iter().enumerate() {
                                if chip(ui, a, id.with(("att", i))) {
                                    remove = Some(i);
                                }
                            }
                        });
                        if let Some(i) = remove {
                            atts.remove(i);
                            out.removed = Some(i);
                        }
                    }
                }
                // text
                let te_w = ui.available_width();
                let font = FontId::proportional(15.0);
                let te = egui::ScrollArea::vertical()
                    .id_salt(id.with("te_scroll"))
                    .max_height(self.max_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::TextEdit::multiline(self.text)
                            .id(te_id)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::symmetric(4, 4))
                            .font(font)
                            .text_color(t.foreground)
                            .hint_text(
                                egui::RichText::new(self.placeholder)
                                    .color(with_alpha(t.muted_foreground, 0.8))
                                    .size(15.0),
                            )
                            .desired_rows(self.min_rows)
                            .desired_width(te_w)
                            .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
                            .show(ui)
                    });
                let te = te.inner;
                // toolbar
                let mut submit_click = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.set_min_height(34.0);
                    if self.attach_button && common::icon_button(ui, Icon::Plus, 32.0, None, "Add attachment").clicked()
                    {
                        out.attach = true;
                    }
                    toolbar(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        // send / stop
                        let can_send = !self.text.trim().is_empty()
                            && self.max_chars.is_none_or(|m| self.text.chars().count() <= m);
                        let (br, bresp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                        let bresp = bresp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(if self.generating { "Stop generating" } else { "Send (Enter)" });
                        let gen_t = common::anim_bool(ui, id.with("gen"), self.generating, 0.2);
                        let active = common::anim_bool(ui, id.with("can"), can_send || self.generating, 0.15);
                        let h = common::anim_bool(ui, id.with("bh"), bresp.hovered(), 0.12);
                        let p = ui.painter();
                        let c = br.center();
                        let off = if t.dark { with_alpha(t.foreground, 0.1) } else { with_alpha(t.foreground, 0.08) };
                        let fill = mix(off, t.primary, active);
                        let fill = if h > 0.0 && active > 0.5 { mix(fill, t.muted_foreground, h * 0.15) } else { fill };
                        let press = if bresp.is_pointer_button_down_on() { 0.92 } else { 1.0 };
                        p.circle_filled(c, 17.0 * press, fill);
                        let icol = mix(with_alpha(t.muted_foreground, 0.7), t.primary_foreground, active);
                        if gen_t > 0.5 {
                            paint_icon(p, Icon::Stop, Rect::from_center_size(c, vec2(15.0, 15.0)), icol);
                            let time = glowkit_core::time(ui);
                            paint::arc(
                                p,
                                c,
                                20.0,
                                time * 4.0,
                                time * 4.0 + 1.6,
                                1.6,
                                with_alpha(t.primary, 0.6 * gen_t),
                            );
                            glowkit_core::animate(ui);
                        } else {
                            paint_icon(p, Icon::ArrowUp, Rect::from_center_size(c, vec2(16.0, 16.0)), icol);
                        }
                        if bresp.clicked() {
                            if self.generating {
                                out.stop = true;
                            } else if can_send {
                                submit_click = true;
                            }
                        }
                        if self.mic_button && common::icon_button(ui, Icon::Mic, 32.0, None, "Voice input").clicked() {
                            out.mic = true;
                        }
                        if let Some(m) = self.max_chars {
                            let n = self.text.chars().count();
                            let col = if n > m {
                                t.destructive
                            } else if n as f32 > m as f32 * 0.9 {
                                t.warning
                            } else {
                                with_alpha(t.muted_foreground, 0.8)
                            };
                            ui.label(egui::RichText::new(format!("{n} / {m}")).size(11.5).color(col));
                        }
                    });
                });
                (te, submit_click)
            });
        let (te, submit_click) = r.inner;
        let focused = te.response.response.has_focus();
        let rect = Rect::from_min_max(top, pos2(top.x + width, r.response.rect.max.y + 10.0));
        ui.advance_cursor_after_rect(rect);
        // background + border + glow
        let f = common::anim_bool(ui, id.with("focus"), focused, 0.25);
        let p = ui.painter();
        p.set(bg_idx, egui::Shape::rect_filled(rect, cr, t.card));
        let border = mix(t.border, common::border_strong(&t), f * 0.6);
        p.rect_stroke(rect, cr, Stroke::new(1.0, border), StrokeKind::Inside);
        if f < 1.0 {
            let shadow = egui::Shadow {
                offset: [0, 2],
                blur: 10,
                spread: 0,
                color: egui::Color32::from_black_alpha(((if t.dark { 50.0 } else { 12.0 }) * (1.0 - f)) as u8),
            };
            p.set(glow_idx, shadow.as_shape(rect, cr));
        }
        if self.glow && f > 0.0 {
            let time = glowkit_core::time(ui);
            let stops = [t.gradient[0], t.gradient[1], t.gradient[2], t.gradient[1]];
            // soft outer glow (behind the box)
            let shadow = egui::Shadow {
                offset: [0, 2],
                blur: 26,
                spread: 0,
                color: with_alpha(t.gradient[0], (if t.dark { 0.30 } else { 0.20 }) * f),
            };
            p.set(glow_idx, shadow.as_shape(rect, cr));
            // rotating gradient ring
            paint::gradient_border(p, rect.expand(0.5), 22.5, 1.5, |u| {
                with_alpha(gradient_cyclic(&stops, u + time * 0.12), 0.9 * f)
            });
            glowkit_core::animate(ui);
        }
        // clicking anywhere on the box focuses the text
        let box_resp = ui.interact(rect, id.with("box"), Sense::click());
        if box_resp.clicked() {
            ui.memory_mut(|m| m.request_focus(te_id));
        }
        let can_send = !self.text.trim().is_empty() && self.max_chars.is_none_or(|m| self.text.chars().count() <= m);
        if (enter || submit_click) && can_send && !self.generating {
            out.submitted = Some(std::mem::take(self.text).trim().to_owned());
            if let Some(a) = self.attachments {
                a.clear();
            }
            ui.memory_mut(|m| m.request_focus(te_id));
        }
        out.response = te.response.response;
        out
    }
}

/// An attachment chip; returns true when its remove button was clicked.
fn chip(ui: &mut Ui, a: &Attachment, id: egui::Id) -> bool {
    let t = Theme::of(ui);
    let g = common::galley(ui, &a.name, 12.5, t.foreground);
    let dg = (!a.detail.is_empty()).then(|| common::galley(ui, &a.detail, 11.0, t.muted_foreground));
    let tw = g.size().x.max(dg.as_ref().map(|d| d.size().x).unwrap_or(0.0));
    let h = if dg.is_some() { 44.0 } else { 32.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(tw + 36.0 + 30.0 + 8.0, h), Sense::hover());
    let hv = common::anim_bool(ui, id.with("h"), resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(10), common::raised(&t));
    p.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, mix(t.border, common::border_strong(&t), hv)),
        StrokeKind::Inside,
    );
    let ib =
        Rect::from_center_size(pos2(rect.min.x + 6.0 + (h - 12.0) / 2.0, rect.center().y), vec2(h - 12.0, h - 12.0));
    let (icon, col) = match a.kind {
        AttachmentKind::File => (Icon::File, t.info),
        AttachmentKind::Image => (Icon::Image, t.chart[2]),
        AttachmentKind::Code => (Icon::Code, t.success),
        AttachmentKind::Link => (Icon::Globe, t.accent),
    };
    p.rect_filled(ib, CornerRadius::same(7), with_alpha(col, if t.dark { 0.18 } else { 0.12 }));
    paint_icon(p, icon, ib.shrink(ib.width() * 0.25), col);
    let tx = ib.max.x + 8.0;
    match dg {
        Some(dg) => {
            p.galley(pos2(tx, rect.center().y - g.size().y), g, t.foreground);
            p.galley(pos2(tx, rect.center().y + 1.0), dg, t.muted_foreground);
        }
        None => {
            p.galley(pos2(tx, rect.center().y - g.size().y / 2.0), g, t.foreground);
        }
    }
    // remove button
    let xr = Rect::from_center_size(pos2(rect.max.x - 15.0, rect.center().y), vec2(20.0, 20.0));
    let xresp = ui
        .interact(xr, id.with("x"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Remove");
    let xh = common::anim_bool(ui, id.with("xh"), xresp.hovered(), 0.1);
    let p = ui.painter();
    if xh > 0.0 {
        p.circle_filled(xr.center(), 10.0, common::hover_fill(&t).gamma_multiply(xh * 1.5));
    }
    paint_icon(p, Icon::X, xr.shrink(5.5), t.muted_foreground.lerp_to_gamma(t.foreground, xh));
    xresp.clicked()
}
