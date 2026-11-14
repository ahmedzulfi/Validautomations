//! Chat messages, message actions and branch switching.

use crate::common::{self, AvatarKind};
use crate::icons::{paint_icon, Icon};
use crate::markdown::Markdown;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder,
    Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Who wrote a [`ChatMessage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The human: right-aligned bubble.
    User,
    /// The model: left-aligned, avatar + Markdown, no bubble.
    Assistant,
    /// A system notice: centered, muted.
    System,
}

/// An action button under a message, see [`MessageActions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageAction {
    /// Copy the message text.
    Copy,
    /// Ask the model to answer again.
    Regenerate,
    /// Thumbs up.
    Like,
    /// Thumbs down.
    Dislike,
    /// Edit the message.
    Edit,
    /// Read the message aloud.
    Speak,
    /// Overflow menu.
    More,
}

impl MessageAction {
    fn icon(self) -> Icon {
        match self {
            MessageAction::Copy => Icon::Copy,
            MessageAction::Regenerate => Icon::Refresh,
            MessageAction::Like => Icon::ThumbUp,
            MessageAction::Dislike => Icon::ThumbDown,
            MessageAction::Edit => Icon::Pencil,
            MessageAction::Speak => Icon::Mic,
            MessageAction::More => Icon::More,
        }
    }
    fn label(self) -> &'static str {
        match self {
            MessageAction::Copy => "Copy",
            MessageAction::Regenerate => "Regenerate",
            MessageAction::Like => "Good response",
            MessageAction::Dislike => "Bad response",
            MessageAction::Edit => "Edit",
            MessageAction::Speak => "Read aloud",
            MessageAction::More => "More",
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ActionsState {
    feedback: Option<bool>,
    copied_at: f64,
}

/// A row of small ghost icon buttons under a message: copy (turns into a check), regenerate,
/// thumbs up / down (remembered), edit... Can reveal itself only while the message is hovered.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let text = "";
/// use glowkit_ai::{MessageAction, MessageActions};
/// if let Some(action) = MessageActions::new().copy_text(text).show(ui) {
///     if action == MessageAction::Regenerate { /* ask again */ }
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MessageActions<'a> {
    actions: Vec<MessageAction>,
    copy_text: Option<&'a str>,
    opacity: f32,
    size: f32,
    id_salt: Option<egui::IdSalt>,
    fixed_id: Option<Id>,
}

impl Default for MessageActions<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MessageActions<'a> {
    /// Copy, regenerate, thumbs up and thumbs down.
    pub fn new() -> Self {
        Self {
            actions: vec![MessageAction::Copy, MessageAction::Regenerate, MessageAction::Like, MessageAction::Dislike],
            copy_text: None,
            opacity: 1.0,
            size: 28.0,
            id_salt: None,
            fixed_id: None,
        }
    }
    /// Which buttons to show, in order.
    pub fn actions(mut self, actions: &[MessageAction]) -> Self {
        self.actions = actions.to_vec();
        self
    }
    /// Text put on the clipboard by the copy button.
    pub fn copy_text(mut self, text: &'a str) -> Self {
        self.copy_text = Some(text);
        self
    }
    /// Overall opacity (used for hover reveal).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    /// Button size in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Stable id for the remembered thumbs state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    pub(crate) fn id(mut self, id: Id) -> Self {
        self.fixed_id = Some(id);
        self
    }

    /// Show the row; returns the action clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<MessageAction> {
        let t = Theme::of(ui);
        let id = self.fixed_id.unwrap_or_else(|| common::widget_id(ui, self.id_salt)).with("actions");
        let mut st: ActionsState = glowkit_core::state(ui, id, ActionsState::default);
        let now = ui.input(|i| i.time);
        let mut clicked = None;
        ui.scope(|ui| {
            ui.multiply_opacity(self.opacity);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for &a in &self.actions {
                    let (icon, active) = match a {
                        MessageAction::Copy if now - st.copied_at < 1.5 => (Icon::Check, Some(t.success)),
                        MessageAction::Like if st.feedback == Some(true) => (a.icon(), Some(t.foreground)),
                        MessageAction::Dislike if st.feedback == Some(false) => (a.icon(), Some(t.foreground)),
                        _ => (a.icon(), None),
                    };
                    let tip = if a == MessageAction::Copy && now - st.copied_at < 1.5 { "Copied!" } else { a.label() };
                    if common::icon_button(ui, icon, self.size, active, tip).clicked() && self.opacity > 0.3 {
                        clicked = Some(a);
                        match a {
                            MessageAction::Copy => {
                                if let Some(text) = self.copy_text {
                                    ui.ctx().copy_text(text.to_owned());
                                }
                                st.copied_at = now;
                            }
                            MessageAction::Like => {
                                st.feedback = if st.feedback == Some(true) { None } else { Some(true) }
                            }
                            MessageAction::Dislike => {
                                st.feedback = if st.feedback == Some(false) { None } else { Some(false) }
                            }
                            _ => {}
                        }
                    }
                }
            });
        });
        if now - st.copied_at < 1.6 {
            ui.ctx().request_repaint();
        }
        glowkit_core::set_state(ui, id, st);
        clicked
    }
}

/// "‹ 2 / 3 ›": switch between alternative versions (branches) of a message.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut branch = 1; // zero-based
/// ui.add(glowkit_ai::BranchSwitcher::new(&mut branch, 3));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BranchSwitcher<'a> {
    current: &'a mut usize,
    total: usize,
}

impl<'a> BranchSwitcher<'a> {
    /// `current` is zero-based and clamped to `0..total`.
    pub fn new(current: &'a mut usize, total: usize) -> Self {
        Self { current, total }
    }
}

impl Widget for BranchSwitcher<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let total = self.total.max(1);
        *self.current = (*self.current).min(total - 1);
        let label = format!("{} / {}", *self.current + 1, total);
        let g = common::galley(ui, label, 12.0, t.muted_foreground);
        let w = 24.0 * 2.0 + g.size().x + 8.0;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
        let left = Rect::from_min_size(rect.min + vec2(0.0, 2.0), vec2(24.0, 24.0));
        let right = Rect::from_min_size(pos2(rect.max.x - 24.0, rect.min.y + 2.0), vec2(24.0, 24.0));
        for (r, icon, enabled, delta) in [
            (left, Icon::ChevronLeft, *self.current > 0, -1i32),
            (right, Icon::ChevronRight, *self.current + 1 < total, 1),
        ] {
            let b = ui.interact(r, resp.id.with(delta), if enabled { Sense::click() } else { Sense::hover() });
            let h = common::anim_bool(ui, b.id.with("h"), b.hovered() && enabled, 0.12);
            if h > 0.0 {
                ui.painter().rect_filled(r, CornerRadius::same(6), common::hover_fill(&t).gamma_multiply(h));
            }
            let col = if enabled {
                t.muted_foreground.lerp_to_gamma(t.foreground, h)
            } else {
                with_alpha(t.muted_foreground, 0.35)
            };
            paint_icon(ui.painter(), icon, r.shrink(6.0), col);
            if b.clicked() {
                *self.current = (*self.current as i32 + delta).clamp(0, total as i32 - 1) as usize;
                resp.mark_changed();
            }
        }
        ui.painter().galley(
            pos2(rect.center().x - g.size().x / 2.0, rect.center().y - g.size().y / 2.0),
            g,
            t.muted_foreground,
        );
        resp
    }
}

/// What happened in a [`ChatMessage`] this frame.
pub struct MessageOutput {
    /// Response covering the whole message row.
    pub response: Response,
    /// Action button clicked, if any.
    pub action: Option<MessageAction>,
}

/// One message in a conversation. User messages are right-aligned rounded bubbles; assistant
/// messages sit on the left next to an avatar with full-width Markdown and an action row that
/// fades in on hover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::{ChatMessage, Role};
/// ui.add(ChatMessage::new(Role::User, "How do lifetimes work?").timestamp("2:14 PM"));
/// let out = ChatMessage::new(Role::Assistant, "Lifetimes are **names for scopes**...")
///     .actions(true)
///     .show(ui, |_ui| {}); // extra content (reasoning, tool calls) goes in the closure
/// ```
#[must_use = "add it with `ui.add(...)` or call `.show(ui, ..)`"]
pub struct ChatMessage<'a> {
    role: Role,
    content: &'a str,
    name: Option<&'a str>,
    timestamp: Option<&'a str>,
    avatar: bool,
    user_avatar: Option<&'a str>,
    branch: Option<(&'a mut usize, usize)>,
    actions: bool,
    actions_always: bool,
    streaming: bool,
    size: f32,
    max_bubble: f32,
    id_salt: Option<egui::IdSalt>,
    bubble: Option<Color32>,
}

impl<'a> ChatMessage<'a> {
    /// A message from `role` with Markdown `content`.
    pub fn new(role: Role, content: &'a str) -> Self {
        Self {
            role,
            content,
            name: None,
            timestamp: None,
            avatar: true,
            user_avatar: None,
            branch: None,
            actions: false,
            actions_always: false,
            streaming: false,
            size: 14.0,
            max_bubble: 0.78,
            id_salt: None,
            bubble: None,
        }
    }
    /// Author name shown above the content (assistant) .
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }
    /// Small muted time label.
    pub fn timestamp(mut self, ts: &'a str) -> Self {
        self.timestamp = Some(ts);
        self
    }
    /// Show the assistant avatar (gradient sparkle). Default on.
    pub fn avatar(mut self, on: bool) -> Self {
        self.avatar = on;
        self
    }
    /// Show an initials avatar next to user bubbles.
    pub fn user_avatar(mut self, initials: &'a str) -> Self {
        self.user_avatar = Some(initials);
        self
    }
    /// Show a [`BranchSwitcher`] (`current` is zero-based) at the start of the action row.
    pub fn branch(mut self, current: &'a mut usize, total: usize) -> Self {
        self.branch = Some((current, total));
        self
    }
    /// Show the [`MessageActions`] row (revealed on hover).
    pub fn actions(mut self, on: bool) -> Self {
        self.actions = on;
        self
    }
    /// Keep the action row visible without hover (typical for the latest answer).
    pub fn actions_always_visible(mut self, on: bool) -> Self {
        self.actions_always = on;
        self
    }
    /// The content is still streaming: shows a caret and fades in new characters.
    pub fn streaming(mut self, on: bool) -> Self {
        self.streaming = on;
        self
    }
    /// Body font size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Maximum bubble width as a fraction of the available width (user messages).
    pub fn max_bubble_width(mut self, fraction: f32) -> Self {
        self.max_bubble = fraction.clamp(0.2, 1.0);
        self
    }
    /// Override the user bubble color.
    pub fn bubble_color(mut self, color: Color32) -> Self {
        self.bubble = Some(color);
        self
    }
    /// Stable id (keeps hover / feedback state when messages are inserted above).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the message with extra content (reasoning, tool calls, sources...) placed before the
    /// Markdown body of an assistant message.
    pub fn show(self, ui: &mut Ui, extra: impl FnOnce(&mut Ui)) -> MessageOutput {
        let t = Theme::of(ui);
        let id = common::widget_id(ui, self.id_salt);
        match self.role {
            Role::User => self.user(ui, &t, id, extra),
            Role::Assistant => self.assistant(ui, &t, id, extra),
            Role::System => {
                let w = ui.available_width();
                let g = common::galley_wrapped(ui, self.content, 12.5, t.muted_foreground, w * 0.7);
                let (rect, response) = ui.allocate_exact_size(vec2(w, g.size().y + 8.0), Sense::hover());
                let p = ui.painter();
                let gx = rect.center().x - g.size().x / 2.0;
                let y = rect.center().y;
                p.line_segment([pos2(rect.min.x, y), pos2(gx - 12.0, y)], Stroke::new(1.0, t.border));
                p.line_segment([pos2(gx + g.size().x + 12.0, y), pos2(rect.max.x, y)], Stroke::new(1.0, t.border));
                p.galley(pos2(gx, rect.min.y + 4.0), g, t.muted_foreground);
                MessageOutput { response, action: None }
            }
        }
    }

    fn user(self, ui: &mut Ui, t: &Theme, id: Id, extra: impl FnOnce(&mut Ui)) -> MessageOutput {
        let avail = ui.available_width();
        let av = if self.user_avatar.is_some() { 40.0 } else { 0.0 };
        let max_w = (avail * self.max_bubble).max(120.0);
        let pad = vec2(16.0, 10.0);
        let g = ui.painter().layout(
            self.content.to_owned(),
            FontId::proportional(self.size),
            t.foreground,
            max_w - pad.x * 2.0,
        );
        let bubble_size = g.size() + pad * 2.0;
        let ts_h = if self.timestamp.is_some() || self.actions { 26.0 } else { 0.0 };
        let (rect, response) = ui.allocate_exact_size(vec2(avail, bubble_size.y + ts_h), Sense::hover());
        let hovered = ui.rect_contains_pointer(rect);
        let bubble = Rect::from_min_size(pos2(rect.max.x - bubble_size.x - av, rect.min.y), bubble_size);
        let fill = self.bubble.unwrap_or(if t.dark { t.secondary } else { t.muted });
        let p = ui.painter();
        p.rect_filled(bubble, CornerRadius { nw: 18, ne: 18, sw: 18, se: 6 }, fill);
        if !t.dark {
            p.rect_stroke(
                bubble,
                CornerRadius { nw: 18, ne: 18, sw: 18, se: 6 },
                Stroke::new(1.0, with_alpha(t.border, 0.6)),
                StrokeKind::Inside,
            );
        }
        p.galley(bubble.min + pad, g, t.foreground);
        if let Some(ini) = self.user_avatar {
            common::avatar(
                ui.painter(),
                pos2(rect.max.x - 15.0, rect.min.y + 15.0),
                15.0,
                t,
                AvatarKind::Initials(ini),
            );
        }
        let mut action = None;
        if ts_h > 0.0 {
            let row = Rect::from_min_max(pos2(rect.min.x, bubble.max.y + 2.0), pos2(bubble.max.x, rect.max.y));
            let mut x = row.max.x - 4.0;
            if let Some(ts) = self.timestamp {
                let r = common::text(
                    ui.painter(),
                    pos2(x, row.center().y),
                    Align2::RIGHT_CENTER,
                    ts,
                    11.0,
                    with_alpha(t.muted_foreground, 0.8),
                );
                x = r.min.x - 6.0;
            }
            if self.actions {
                let op = common::anim_bool(ui, id.with("hover"), hovered || self.actions_always, 0.15);
                let acts = [MessageAction::Edit, MessageAction::Copy];
                let w = acts.len() as f32 * 26.0;
                let r = Rect::from_min_max(pos2(x - w, row.min.y), pos2(x, row.max.y));
                let mut child =
                    ui.new_child(UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)));
                action = MessageActions::new()
                    .actions(&acts)
                    .copy_text(self.content)
                    .size(24.0)
                    .opacity(op)
                    .id(id)
                    .show(&mut child);
            }
        }
        extra(ui);
        MessageOutput { response, action }
    }

    fn assistant(self, ui: &mut Ui, t: &Theme, id: Id, extra: impl FnOnce(&mut Ui)) -> MessageOutput {
        let avail = ui.available_rect_before_wrap();
        let av_r = 15.0;
        let indent = if self.avatar { av_r * 2.0 + 14.0 } else { 0.0 };
        let prev_rect: Rect = glowkit_core::state(ui, id.with("rect"), || Rect::NOTHING);
        let hovered = ui.rect_contains_pointer(prev_rect);
        let col_rect = Rect::from_min_max(pos2(avail.min.x + indent, avail.min.y), pos2(avail.max.x, avail.max.y));
        let mut action = None;
        let inner = ui.scope_builder(
            UiBuilder::new().max_rect(col_rect).layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                if self.name.is_some() || self.timestamp.is_some() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if let Some(n) = self.name {
                            let g = common::galley(ui, n, 13.0, t.foreground);
                            let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 1.0, av_r * 2.0), Sense::hover());
                            common::paint_galley_fx(
                                ui.painter(),
                                pos2(r.min.x, r.center().y - g.size().y / 2.0),
                                &g,
                                |_| common::Fx { bold: true, ..Default::default() },
                            );
                        }
                        if let Some(ts) = self.timestamp {
                            let g = common::galley(ui, ts, 11.5, with_alpha(t.muted_foreground, 0.85));
                            let (r, _) = ui.allocate_exact_size(vec2(g.size().x, av_r * 2.0), Sense::hover());
                            ui.painter().galley(
                                pos2(r.min.x, r.center().y - g.size().y / 2.0 + 0.5),
                                g,
                                t.muted_foreground,
                            );
                        }
                    });
                } else if self.avatar {
                    // align the first line of text with the avatar's center
                    ui.add_space(3.0);
                }
                extra(ui);
                if !self.content.is_empty() || self.streaming {
                    ui.add(
                        Markdown::new(self.content)
                            .size(self.size)
                            .caret(self.streaming)
                            .fade_tail(if self.streaming { 28 } else { 0 }),
                    );
                }
                if (self.actions || self.branch.is_some()) && !self.streaming {
                    let op = common::anim_bool(ui, id.with("hover"), hovered || self.actions_always, 0.18);
                    ui.add_space(-4.0);
                    let content = self.content;
                    let actions = self.actions;
                    let branch = self.branch;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if let Some((cur, total)) = branch {
                            ui.add(BranchSwitcher::new(cur, total));
                        }
                        if actions {
                            action = MessageActions::new().copy_text(content).opacity(op).id(id).show(ui);
                        }
                    });
                }
            },
        );
        let rect = Rect::from_min_max(avail.min, pos2(avail.max.x, inner.response.rect.max.y));
        glowkit_core::set_state(ui, id.with("rect"), rect);
        if self.avatar {
            common::avatar(ui.painter(), pos2(avail.min.x + av_r, avail.min.y + av_r), av_r, t, AvatarKind::Assistant);
        }
        let response = ui.interact(rect, id.with("msg"), Sense::hover());
        MessageOutput { response, action }
    }
}

impl Widget for ChatMessage<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| {}).response
    }
}

/// Paint a user/assistant avatar (useful for custom layouts).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_ai::Avatar::assistant(16.0));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Avatar<'a> {
    radius: f32,
    initials: Option<&'a str>,
}

impl<'a> Avatar<'a> {
    /// The assistant avatar: a gradient circle with a sparkle.
    pub fn assistant(radius: f32) -> Self {
        Self { radius, initials: None }
    }
    /// A neutral circle with initials.
    pub fn initials(initials: &'a str, radius: f32) -> Self {
        Self { radius, initials: Some(initials) }
    }
}

impl Widget for Avatar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(self.radius * 2.0, self.radius * 2.0), Sense::hover());
        let kind = match self.initials {
            Some(s) => AvatarKind::Initials(s),
            None => AvatarKind::Assistant,
        };
        common::avatar(ui.painter(), rect.center(), self.radius, &t, kind);
        resp
    }
}
