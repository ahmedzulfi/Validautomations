//! AI-agent interface components: [`ChatMessage`] (with a tiny Markdown subset and a live
//! streaming caret), [`PromptInput`] (a focusable, keyboard-driven composer view) and
//! [`ApprovalCard`] (human-in-the-loop tool approval).

use crate::anim::{approach, secs, FrameClock};
use crate::buttons::ClickHandler;
use crate::color::{mix, with_alpha};
use crate::text::sub_id;
use crate::theme::{ActiveTheme, Theme};
use gpui::prelude::*;
use gpui::{
    div, linear_color_stop, linear_gradient, point, px, AnyElement, Animation, AnimationExt, App, BoxShadow,
    ClickEvent, Context, ElementId, EventEmitter, FocusHandle, Focusable, FontWeight, HighlightStyle, Hsla,
    KeyDownEvent, MouseButton, SharedString, StyledText, Window,
};
use std::ops::Range;
use std::rc::Rc;

/// Who wrote a [`ChatMessage`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    User,
    #[default]
    Assistant,
    System,
}

const CARET: &str = "\u{258D}";

/// Inline Markdown: `**bold**` and `` `code` `` spans. Returns the plain text and highlights.
fn inline_md(src: &str, t: &Theme) -> (String, Vec<(Range<usize>, HighlightStyle)>) {
    let mut out = String::with_capacity(src.len());
    let mut hl = Vec::new();
    let mut rest = src;
    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let (pos, marker) = match (next_bold, next_code) {
            (Some(b), Some(c)) if c < b => (c, "`"),
            (Some(b), _) => (b, "**"),
            (None, Some(c)) => (c, "`"),
            (None, None) => {
                out.push_str(rest);
                break;
            }
        };
        out.push_str(&rest[..pos]);
        let after = &rest[pos + marker.len()..];
        match after.find(marker) {
            Some(end) => {
                let start = out.len();
                out.push_str(&after[..end]);
                let style = if marker == "`" {
                    HighlightStyle {
                        background_color: Some(with_alpha(t.foreground, if t.dark { 0.09 } else { 0.06 })),
                        color: Some(mix(t.foreground, t.accent, 0.35)),
                        ..Default::default()
                    }
                } else {
                    HighlightStyle { font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() }
                };
                hl.push((start..out.len(), style));
                rest = &after[end + marker.len()..];
            }
            None => {
                out.push_str(&rest[pos..]);
                break;
            }
        }
    }
    (out, hl)
}

/// A paragraph of inline Markdown, optionally ending in a pulsing caret.
fn md_line(id: ElementId, src: &str, t: &Theme, caret: bool) -> AnyElement {
    let (mut text, hl) = inline_md(src, t);
    if !caret {
        return StyledText::new(text).with_highlights(hl).into_any_element();
    }
    let start = text.len();
    text.push_str(CARET);
    let caret_range = start..text.len();
    let accent = t.accent;
    let text: SharedString = text.into();
    div()
        .with_animation(
            id,
            Animation::new(secs(0.9)).repeat().with_easing(gpui::pulsating_between(0.2, 1.0)),
            move |el, k| {
                let mut h = hl.clone();
                h.push((caret_range.clone(), HighlightStyle { color: Some(with_alpha(accent, k)), ..Default::default() }));
                el.child(StyledText::new(text.clone()).with_highlights(h))
            },
        )
        .into_any_element()
}

/// Render a small Markdown subset: paragraphs, `- ` bullets, fenced code blocks, `**bold**` and
/// `` `code` ``. `caret` puts a live caret after the last block (for streaming replies).
pub fn markdown(id: &ElementId, src: &str, t: &Theme, caret: bool) -> impl IntoElement {
    let mut blocks: Vec<AnyElement> = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    let last_nonempty = lines.iter().rposition(|l| !l.trim().is_empty()).unwrap_or(0);
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("```") {
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push(lines[i]);
                i += 1;
            }
            i += 1;
            blocks.push(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(t.radius * 0.8))
                    .bg(if t.dark { mix(t.background, t.muted, 0.7) } else { t.muted })
                    .border_1()
                    .border_color(t.border)
                    .font_family(t.mono_family)
                    .text_size(px(12.5))
                    .text_color(mix(t.foreground, t.muted_foreground, 0.2))
                    .children(code.into_iter().map(|l| div().child(l.to_string())))
                    .into_any_element(),
            );
            continue;
        }
        let caret_here = caret && i == last_nonempty;
        let sub = ElementId::NamedChild(Box::new(id.clone()), format!("md{i}").into());
        if let Some(item) = line.trim_start().strip_prefix("- ") {
            blocks.push(
                div()
                    .flex()
                    .gap(px(8.))
                    .pl(px(4.))
                    .child(div().flex_none().text_color(t.accent).child("\u{2022}"))
                    .child(div().flex_1().child(md_line(sub, item, t, caret_here)))
                    .into_any_element(),
            );
        } else if !line.trim().is_empty() {
            blocks.push(div().child(md_line(sub, line, t, caret_here)).into_any_element());
        } else if caret_here {
            blocks.push(md_line(sub, "", t, true));
        }
        i += 1;
    }
    if blocks.is_empty() && caret {
        blocks.push(md_line(sub_id(id, "md-empty"), "", t, true));
    }
    div().flex().flex_col().gap(px(8.)).children(blocks)
}

/// One chat turn. User messages sit on the right in a tinted bubble; assistant messages sit on
/// the left next to a gradient avatar, with a hover action row.
#[derive(IntoElement)]
pub struct ChatMessage {
    id: ElementId,
    role: Role,
    content: SharedString,
    name: Option<SharedString>,
    timestamp: Option<SharedString>,
    streaming: bool,
    actions: bool,
}

impl ChatMessage {
    pub fn new(id: impl Into<ElementId>, role: Role, content: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            name: None,
            timestamp: None,
            streaming: false,
            actions: true,
        }
    }
    pub fn name(mut self, n: impl Into<SharedString>) -> Self {
        self.name = Some(n.into());
        self
    }
    pub fn timestamp(mut self, ts: impl Into<SharedString>) -> Self {
        self.timestamp = Some(ts.into());
        self
    }
    /// The reply is still arriving: show a live caret and hide the action row.
    pub fn streaming(mut self, s: bool) -> Self {
        self.streaming = s;
        self
    }
    /// Show the hover action row on assistant messages (default on).
    pub fn actions(mut self, a: bool) -> Self {
        self.actions = a;
        self
    }
}

impl RenderOnce for ChatMessage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        match self.role {
            Role::User => div().w_full().flex().justify_end().child(
                div()
                    .max_w(gpui::relative(0.78))
                    .px(px(14.))
                    .py(px(9.))
                    .rounded(px(18.))
                    .rounded_br(px(6.))
                    .bg(if t.dark { mix(t.secondary, t.accent, 0.10) } else { mix(t.secondary, t.accent, 0.06) })
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(t.foreground)
                    .child(markdown(&self.id, &self.content, &t, false)),
            ),
            Role::System => div().w_full().flex().justify_center().child(
                div()
                    .px(px(10.))
                    .py(px(3.))
                    .rounded_full()
                    .bg(t.muted)
                    .text_size(px(11.5))
                    .text_color(t.muted_foreground)
                    .child(self.content),
            ),
            Role::Assistant => {
                let group: SharedString = "glowkit-chat-msg".into();
                div()
                    .group(group.clone())
                    .w_full()
                    .flex()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_none()
                            .size(px(28.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(linear_gradient(
                                135.,
                                linear_color_stop(t.gradient[0], 0.0),
                                linear_color_stop(t.gradient[1], 1.0),
                            ))
                            .shadow(vec![BoxShadow {
                                color: with_alpha(t.gradient[0], 0.35),
                                offset: point(px(0.), px(2.)),
                                blur_radius: px(10.),
                                spread_radius: px(0.),
                            }])
                            .text_color(gpui::white())
                            .text_size(px(13.))
                            .child("\u{2726}"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(t.foreground)
                                            .child(self.name.unwrap_or_else(|| "Assistant".into())),
                                    )
                                    .children(self.timestamp.map(|ts| {
                                        div().text_size(px(11.5)).text_color(t.muted_foreground).child(ts)
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(23.))
                                    .text_color(mix(t.foreground, t.muted_foreground, 0.12))
                                    .child(markdown(&self.id, &self.content, &t, self.streaming)),
                            )
                            .when(self.actions && !self.streaming, |el| {
                                el.child(
                                    div()
                                        .flex()
                                        .gap(px(4.))
                                        .pt(px(2.))
                                        .opacity(0.0)
                                        .group_hover(group, |s| s.opacity(1.0))
                                        .children(["Copy", "Regenerate", "Good reply"].into_iter().map(|a| {
                                            div()
                                                .px(px(8.))
                                                .py(px(2.))
                                                .rounded(px(6.))
                                                .text_size(px(11.5))
                                                .text_color(t.muted_foreground)
                                                .hover(|s| s.bg(with_alpha(t.foreground, 0.06)).text_color(t.foreground))
                                                .child(a)
                                        })),
                                )
                            }),
                    )
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------

/// Emitted by [`PromptInput`] when the user presses Enter (or the send button) with text.
#[derive(Clone, Debug)]
pub struct PromptSubmit(pub String);

/// Emitted by [`PromptInput`] when the stop button is pressed while busy.
#[derive(Clone, Debug)]
pub struct PromptStop;

/// A chat composer: a focusable view with a growing text area, an animated focus ring and a send
/// / stop button. Typing is handled from key-down events (printable keys, Backspace, Enter to
/// send, Shift+Enter for a newline, Escape to blur); it does not implement IME composition.
pub struct PromptInput {
    focus: FocusHandle,
    text: String,
    placeholder: SharedString,
    model: SharedString,
    busy: bool,
    ring: f32,
    clock: FrameClock,
}

impl EventEmitter<PromptSubmit> for PromptInput {}
impl EventEmitter<PromptStop> for PromptInput {}

impl Focusable for PromptInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PromptInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            text: String::new(),
            placeholder: "Ask anything\u{2026}".into(),
            model: "glow-1".into(),
            busy: false,
            ring: 0.0,
            clock: FrameClock::default(),
        }
    }
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Label of the model chip.
    pub fn model(mut self, m: impl Into<SharedString>) -> Self {
        self.model = m.into();
        self
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.text = text.into();
        cx.notify();
    }
    /// While busy the send button turns into a pulsing stop button and Enter does nothing.
    pub fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        cx.notify();
    }
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            cx.emit(PromptStop);
            return;
        }
        let text = self.text.trim().to_string();
        if !text.is_empty() {
            self.text.clear();
            cx.emit(PromptSubmit(text));
            cx.notify();
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "enter" if !ks.modifiers.shift => self.submit(cx),
            "enter" => self.text.push('\n'),
            "backspace" => {
                self.text.pop();
            }
            "escape" => window.blur(),
            _ => {
                if ks.modifiers.control || ks.modifiers.platform || ks.modifiers.alt {
                    return;
                }
                match &ks.key_char {
                    Some(ch) => self.text.push_str(ch),
                    None => return,
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for PromptInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme().clone();
        let focused = self.focus.is_focused(window);
        let dt = self.clock.tick();
        let target = if focused { 1.0 } else { 0.0 };
        self.ring = approach(self.ring, target, 12.0, dt);
        if (self.ring - target).abs() > 1e-3 {
            window.request_animation_frame();
        }
        let ring = self.ring;
        let empty = self.text.is_empty();
        let accent = t.ring;
        let body: AnyElement = if empty && !focused {
            div().text_color(t.muted_foreground).child(self.placeholder.clone()).into_any_element()
        } else {
            let shown: SharedString = format!("{}{CARET}", self.text).into();
            let caret = self.text.len()..shown.len();
            let placeholder = (empty).then(|| self.placeholder.clone());
            let fg = t.muted_foreground;
            div()
                .relative()
                .when_some(placeholder, |el, p| {
                    el.child(div().absolute().left(px(10.)).top_0().text_color(fg).child(p))
                })
                .with_animation(
                    "prompt-caret",
                    Animation::new(secs(1.0)).repeat().with_easing(|k| if k < 0.5 { 1.0 } else { 0.0 }),
                    move |el, k| {
                        el.child(StyledText::new(shown.clone()).with_highlights([(
                            caret.clone(),
                            HighlightStyle { color: Some(with_alpha(accent, k * ring)), ..Default::default() },
                        )]))
                    },
                )
                .into_any_element()
        };
        let can_send = !empty || self.busy;
        let send_bg = if can_send { t.foreground } else { with_alpha(t.foreground, 0.12) };
        let send_fg = if can_send { t.background } else { t.muted_foreground };
        let chip = |label: SharedString| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .h(px(28.))
                .px(px(10.))
                .rounded_full()
                .border_1()
                .border_color(t.border)
                .text_size(px(12.))
                .text_color(t.muted_foreground)
                .hover(|s| s.bg(with_alpha(t.foreground, 0.05)).text_color(t.foreground))
                .child(label)
        };
        div()
            .id("glowkit-prompt-input")
            .track_focus(&self.focus)
            .key_context("PromptInput")
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, _| {
                    window.focus(&this.focus);
                }),
            )
            .w_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(12.))
            .rounded(px(t.radius * 1.6))
            .bg(t.surface())
            .border_1()
            .border_color(mix(t.border, t.ring, 0.75 * ring))
            .shadow(vec![
                BoxShadow {
                    color: with_alpha(t.ring, 0.22 * ring),
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(3.0 * ring),
                },
                BoxShadow {
                    color: with_alpha(t.ring, 0.25 * ring),
                    offset: point(px(0.), px(4.)),
                    blur_radius: px(24.),
                    spread_radius: px(0.),
                },
            ])
            .cursor_text()
            .child(
                div()
                    .min_h(px(44.))
                    .px(px(4.))
                    .text_size(px(14.5))
                    .line_height(px(22.))
                    .text_color(t.foreground)
                    .child(body),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(chip("+  Attach".into()))
                            .child(chip(format!("{}  \u{25BE}", self.model).into())),
                    )
                    .child(
                        div()
                            .id("glowkit-prompt-send")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(32.))
                            .rounded_full()
                            .bg(send_bg)
                            .text_color(send_fg)
                            .text_size(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .when(can_send, |el| el.cursor_pointer().hover(|s| s.opacity(0.88)))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit(cx)))
                            .map(|el| {
                                if self.busy {
                                    el.child(div().size(px(10.)).rounded(px(2.)).bg(send_fg)).with_animation(
                                        "prompt-busy",
                                        Animation::new(secs(1.2)).repeat().with_easing(gpui::pulsating_between(0.55, 1.0)),
                                        |el, k| el.opacity(k),
                                    )
                                    .into_any_element()
                                } else {
                                    el.child("\u{2191}").into_any_element()
                                }
                            }),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------------------------

/// How risky an [`ApprovalCard`] action is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Risk {
    #[default]
    Low,
    Medium,
    High,
}

impl Risk {
    fn color(self, t: &Theme) -> Hsla {
        match self {
            Risk::Low => t.success,
            Risk::Medium => t.warning,
            Risk::High => t.destructive,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Risk::Low => "Low risk",
            Risk::Medium => "Medium risk",
            Risk::High => "High risk",
        }
    }
}

/// The outcome of an approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Approved,
    Denied,
}

/// An agent asking a person to approve a tool call: who, which tool, the exact command, why,
/// how risky, and how long it has been waiting. While pending its border breathes in the risk
/// color.
#[derive(IntoElement)]
pub struct ApprovalCard {
    id: ElementId,
    agent: SharedString,
    tool: SharedString,
    command: SharedString,
    reason: Option<SharedString>,
    risk: Risk,
    waiting: f32,
    decision: Option<Decision>,
    on_approve: Option<ClickHandler>,
    on_deny: Option<ClickHandler>,
}

impl ApprovalCard {
    pub fn new(
        id: impl Into<ElementId>,
        agent: impl Into<SharedString>,
        tool: impl Into<SharedString>,
        command: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            tool: tool.into(),
            command: command.into(),
            reason: None,
            risk: Risk::Low,
            waiting: 0.0,
            decision: None,
            on_approve: None,
            on_deny: None,
        }
    }
    pub fn reason(mut self, r: impl Into<SharedString>) -> Self {
        self.reason = Some(r.into());
        self
    }
    pub fn risk(mut self, r: Risk) -> Self {
        self.risk = r;
        self
    }
    /// Seconds the agent has been waiting.
    pub fn waiting(mut self, s: f32) -> Self {
        self.waiting = s;
        self
    }
    pub fn decision(mut self, d: Option<Decision>) -> Self {
        self.decision = d;
        self
    }
    pub fn on_approve(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_approve = Some(Rc::new(f));
        self
    }
    pub fn on_deny(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_deny = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ApprovalCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let rc = self.risk.color(&t);
        let radius = t.radius * 1.4;
        let pending = self.decision.is_none();
        let w = self.waiting.max(0.0) as u32;
        let waiting = format!("Waiting {}:{:02}", w / 60, w % 60);
        let button = |id: &'static str, label: &'static str, primary: bool, cb: Option<ClickHandler>| {
            div()
                .id(id)
                .flex()
                .items_center()
                .h(px(32.))
                .px(px(14.))
                .rounded(px(t.radius * 0.8))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .map(|el| {
                    if primary {
                        el.bg(t.primary).text_color(t.primary_foreground).hover(|s| s.opacity(0.88))
                    } else {
                        el.border_1()
                            .border_color(t.border)
                            .text_color(t.foreground)
                            .hover(|s| s.bg(with_alpha(t.foreground, 0.06)))
                    }
                })
                .active(|s| s.opacity(0.75))
                .when_some(cb, |el, cb| el.on_click(move |e, w, cx| cb(e, w, cx)))
                .child(label)
        };
        let footer: AnyElement = match self.decision {
            None => div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(t.muted_foreground)
                        .child(div().size(px(6.)).rounded_full().bg(rc).with_animation(
                            sub_id(&self.id, "dot"),
                            Animation::new(secs(1.4)).repeat().with_easing(gpui::pulsating_between(0.3, 1.0)),
                            |el, k| el.opacity(k),
                        ))
                        .child(waiting),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(button("deny", "Deny", false, self.on_deny))
                        .child(button("approve", "Approve", true, self.on_approve)),
                )
                .into_any_element(),
            Some(d) => {
                let (c, label) = match d {
                    Decision::Approved => (t.success, "\u{2713}  Approved"),
                    Decision::Denied => (t.destructive, "\u{2715}  Denied"),
                };
                div()
                    .flex()
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(4.))
                            .rounded_full()
                            .bg(with_alpha(c, 0.14))
                            .text_color(c)
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(label),
                    )
                    .into_any_element()
            }
        };
        div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p(px(16.))
            .rounded(px(radius))
            .bg(t.surface())
            .border_1()
            .border_color(t.border)
            .shadow(t.card_shadow(0.0))
            .when(!pending, |el| el.opacity(0.85))
            .when(pending, |el| {
                el.child(div().absolute().inset_0().rounded(px(radius)).border_1().with_animation(
                    sub_id(&self.id, "ring"),
                    Animation::new(secs(2.0)).repeat().with_easing(gpui::pulsating_between(0.15, 0.7)),
                    move |el, k| el.border_color(with_alpha(rc, k)),
                ))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_none()
                            .size(px(30.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(with_alpha(rc, 0.14))
                            .text_color(rc)
                            .font_weight(FontWeight::BOLD)
                            .text_size(px(14.))
                            .child("!"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.5))
                            .text_color(t.foreground)
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.agent))
                            .child(div().text_color(t.muted_foreground).child("wants to run"))
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(1.))
                                    .rounded(px(5.))
                                    .bg(with_alpha(t.foreground, 0.07))
                                    .font_family(t.mono_family)
                                    .text_size(px(12.))
                                    .child(self.tool),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded_full()
                            .border_1()
                            .border_color(with_alpha(rc, 0.4))
                            .bg(with_alpha(rc, 0.10))
                            .text_color(rc)
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.risk.label()),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(t.radius * 0.8))
                    .bg(if t.dark { mix(t.background, t.muted, 0.6) } else { t.muted })
                    .border_1()
                    .border_color(t.border)
                    .font_family(t.mono_family)
                    .text_size(px(12.5))
                    .text_color(t.foreground)
                    .child(
                        StyledText::new(format!("$ {}", self.command)).with_highlights([(
                            0..1,
                            HighlightStyle { color: Some(t.muted_foreground), ..Default::default() },
                        )]),
                    ),
            )
            .children(
                self.reason.map(|r| div().text_size(px(12.5)).line_height(px(19.)).text_color(t.muted_foreground).child(r)),
            )
            .child(footer)
    }
}
