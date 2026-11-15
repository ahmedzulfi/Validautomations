//! Motion components: [`Marquee`], [`AnimatedTabs`] and the [`ToastStack`] notification view.

use crate::anim::{approach, ease, Easing, FrameClock, Spring};
use crate::color::{mix, with_alpha};
use crate::text::sub_id;
use crate::theme::{ActiveTheme, Theme};
use gpui::prelude::*;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, px, App, ClickEvent, Context, ElementId, FontWeight, Hsla,
    SharedString, TextRun, Window,
};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Width of a single line of text in the current text style.
pub(crate) fn text_width(window: &Window, text: &SharedString, size: f32, weight: FontWeight) -> f32 {
    let mut font = window.text_style().font();
    font.weight = weight;
    let run = TextRun {
        len: text.len(),
        font,
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(window.text_system().shape_line(text.clone(), px(size), &[run], None).width)
}

// ---------------------------------------------------------------------------------------------

struct MarqueeState {
    offset: f32,
    hovered: bool,
    speed_k: f32,
    width: f32,
    clock: FrameClock,
}

/// An infinitely scrolling strip of tags, with optional pause on hover (it eases to a stop
/// rather than freezing) and faded edges.
#[derive(IntoElement)]
pub struct Marquee {
    id: ElementId,
    items: Vec<SharedString>,
    speed: f32,
    gap: f32,
    reverse: bool,
    pause_on_hover: bool,
    fade: bool,
    fade_color: Option<Hsla>,
    font_size: f32,
}

impl Marquee {
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().map(Into::into).collect(),
            speed: 50.0,
            gap: 12.0,
            reverse: false,
            pause_on_hover: true,
            fade: true,
            fade_color: None,
            font_size: 13.0,
        }
    }
    /// Pixels per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    pub fn reverse(mut self, r: bool) -> Self {
        self.reverse = r;
        self
    }
    pub fn pause_on_hover(mut self, p: bool) -> Self {
        self.pause_on_hover = p;
        self
    }
    /// Fade the left and right edges into `fade_color` (default: the theme background).
    pub fn fade(mut self, f: bool) -> Self {
        self.fade = f;
        self
    }
    pub fn fade_color(mut self, c: Hsla) -> Self {
        self.fade_color = Some(c);
        self
    }
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }
}

impl RenderOnce for Marquee {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let pad = 14.0;
        let dot = 6.0;
        let widths: Vec<f32> = self
            .items
            .iter()
            .map(|s| text_width(window, s, self.font_size, FontWeight::MEDIUM) + pad * 2.0 + dot + 8.0)
            .collect();
        let period: f32 = widths.iter().map(|w| w + self.gap).sum::<f32>().max(1.0);
        let st = window.use_keyed_state(sub_id(&self.id, "marquee"), cx, |_, _| MarqueeState {
            offset: 0.0,
            hovered: false,
            speed_k: 1.0,
            width: 1200.0,
            clock: FrameClock::default(),
        });
        let pause = self.pause_on_hover;
        let (offset, width) = st.update(cx, |s, _| {
            let dt = s.clock.tick();
            s.speed_k = approach(s.speed_k, if pause && s.hovered { 0.0 } else { 1.0 }, 6.0, dt);
            s.offset = (s.offset + self.speed * s.speed_k * dt).rem_euclid(period);
            (s.offset, s.width)
        });
        window.request_animation_frame();
        let copies = (width / period).ceil() as usize + 1;
        let x = if self.reverse { offset - period } else { -offset };
        let h = self.font_size * 2.6;
        let row = div().absolute().top_0().left(px(x)).h_full().flex().items_center().gap(px(self.gap)).children(
            (0..copies).flat_map(|_| self.items.iter().cloned().enumerate()).map(|(i, s)| {
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .h(px(h - 8.0))
                    .px(px(pad))
                    .rounded_full()
                    .bg(t.surface())
                    .border_1()
                    .border_color(t.border)
                    .text_size(px(self.font_size))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.foreground)
                    .whitespace_nowrap()
                    .child(div().size(px(dot)).rounded_full().bg(t.chart[i % t.chart.len()]))
                    .child(s)
            }),
        );
        let fade = self.fade_color.unwrap_or(t.background);
        let (s1, s2) = (st.clone(), st);
        div()
            .id(self.id)
            .relative()
            .w_full()
            .h(px(h))
            .overflow_hidden()
            .on_hover(move |h, _, cx| s1.update(cx, |s, _| s.hovered = *h))
            .child(canvas(move |b, _, cx| s2.update(cx, |s, _| s.width = f32::from(b.size.width)), |_, _, _, _| {}).absolute().inset_0())
            .child(row)
            .when(self.fade, |el| {
                el.child(div().absolute().top_0().left_0().h_full().w(px(64.)).bg(linear_gradient(
                    90.,
                    linear_color_stop(fade, 0.0),
                    linear_color_stop(with_alpha(fade, 0.0), 1.0),
                )))
                .child(div().absolute().top_0().right_0().h_full().w(px(64.)).bg(linear_gradient(
                    270.,
                    linear_color_stop(fade, 0.0),
                    linear_color_stop(with_alpha(fade, 0.0), 1.0),
                )))
            })
    }
}

// ---------------------------------------------------------------------------------------------

/// How [`AnimatedTabs`] highlights the selected tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabHighlight {
    /// A raised pill inside a muted track.
    #[default]
    Pill,
    /// A line under the label.
    Underline,
    /// A solid accent-colored pill.
    Solid,
}

struct TabsState {
    l: Spring,
    r: Spring,
    last_center: f32,
    moving_right: bool,
    clock: FrameClock,
}

/// Change callback for [`AnimatedTabs`].
pub type TabChange = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A row of tabs whose highlight springs from one tab to the next, stretching as it moves
/// (leading and trailing edges ride springs of different stiffness). Port of
/// `glowkit_effects::AnimatedTabs`.
#[derive(IntoElement)]
pub struct AnimatedTabs {
    id: ElementId,
    labels: Vec<SharedString>,
    selected: usize,
    style: TabHighlight,
    font_size: f32,
    stiffness: f32,
    damping: f32,
    on_change: Option<TabChange>,
}

impl AnimatedTabs {
    pub fn new(id: impl Into<ElementId>, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            selected: 0,
            style: TabHighlight::Pill,
            font_size: 14.0,
            stiffness: 380.0,
            damping: 30.0,
            on_change: None,
        }
    }
    pub fn selected(mut self, i: usize) -> Self {
        self.selected = i;
        self
    }
    pub fn style(mut self, s: TabHighlight) -> Self {
        self.style = s;
        self
    }
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }
    /// Spring stiffness and damping of the highlight.
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }
    pub fn on_change(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for AnimatedTabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let n = self.labels.len();
        let pad_x = 16.0;
        let h = self.font_size + 20.0;
        let inset = if self.style == TabHighlight::Underline { 0.0 } else { 4.0 };
        let widths: Vec<f32> =
            self.labels.iter().map(|l| text_width(window, l, self.font_size, FontWeight::MEDIUM) + pad_x * 2.0).collect();
        let sel = self.selected.min(n.saturating_sub(1));
        let left: f32 = widths[..sel].iter().sum();
        let (tl, tr) = (left, left + widths.get(sel).copied().unwrap_or(0.0));
        let (k, c) = (self.stiffness, self.damping);
        let st = window.use_keyed_state(sub_id(&self.id, "tabs"), cx, |_, _| TabsState {
            l: Spring::new(tl),
            r: Spring::new(tr),
            last_center: (tl + tr) / 2.0,
            moving_right: true,
            clock: FrameClock::default(),
        });
        let (l, r, settled) = st.update(cx, |s, _| {
            let center = (tl + tr) / 2.0;
            if (center - s.last_center).abs() > 0.5 {
                s.moving_right = center > s.last_center;
                s.last_center = center;
            }
            // the trailing edge lags, so the pill stretches in the direction of travel
            let (fast, slow) = ((k, c), (k * 0.55, c * 0.75));
            let (ls, rs) = if s.moving_right { (slow, fast) } else { (fast, slow) };
            s.l = s.l.with(ls.0, ls.1);
            s.r = s.r.with(rs.0, rs.1);
            s.l.target = tl;
            s.r.target = tr;
            let dt = s.clock.tick();
            (s.l.step(dt), s.r.step(dt), s.l.is_settled() && s.r.is_settled())
        });
        if !settled {
            window.request_animation_frame();
        }
        let w = (r - l).max(4.0);
        let highlight = match self.style {
            TabHighlight::Pill => div()
                .absolute()
                .top(px(inset))
                .left(px(inset + l))
                .w(px(w))
                .h(px(h))
                .rounded(px(h / 2.0))
                .bg(if t.dark { mix(t.secondary, t.foreground, 0.06) } else { t.background })
                .shadow(t.card_shadow(0.0)),
            TabHighlight::Solid => {
                div().absolute().top(px(inset)).left(px(inset + l)).w(px(w)).h(px(h)).rounded(px(h / 2.0)).bg(t.accent)
            }
            TabHighlight::Underline => {
                div().absolute().bottom_0().left(px(l + 10.0)).w(px((w - 20.0).max(4.0))).h(px(2.)).rounded_full().bg(t.accent)
            }
        };
        let on_change = self.on_change;
        let style = self.style;
        let track = match style {
            TabHighlight::Underline => div().border_b_1().border_color(t.border),
            _ => div().rounded(px(h / 2.0 + inset)).bg(t.muted).border_1().border_color(t.border),
        };
        track.id(self.id).relative().flex().p(px(inset)).child(highlight).children(
            self.labels.into_iter().zip(widths).enumerate().map(move |(i, (label, tw))| {
                let on = i == sel;
                let fg = match (on, style) {
                    (true, TabHighlight::Solid) => t.accent_foreground,
                    (true, _) => t.foreground,
                    _ => t.muted_foreground,
                };
                let cb = on_change.clone();
                div()
                    .id(i)
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(tw))
                    .h(px(h))
                    .rounded(px(h / 2.0))
                    .text_size(px(self.font_size))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .cursor_pointer()
                    .when(!on, |el| el.hover(|s| s.text_color(t.foreground).bg(with_alpha(t.foreground, 0.04))))
                    .on_click(move |_, w, cx| {
                        if let Some(cb) = &cb {
                            cb(i, w, cx)
                        }
                    })
                    .child(label)
            }),
        )
    }
}

// ---------------------------------------------------------------------------------------------

/// Visual flavour of a [`Toast`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    Default,
    Success,
    Info,
    Warning,
    Error,
}

impl ToastKind {
    fn color(self, t: &Theme) -> Hsla {
        match self {
            ToastKind::Default => t.accent,
            ToastKind::Success => t.success,
            ToastKind::Info => t.info,
            ToastKind::Warning => t.warning,
            ToastKind::Error => t.destructive,
        }
    }
    fn glyph(self) -> &'static str {
        match self {
            ToastKind::Default => "\u{2726}", // ✦
            ToastKind::Success => "\u{2713}", // ✓
            ToastKind::Info => "i",
            ToastKind::Warning => "!",
            ToastKind::Error => "\u{2715}", // ✕
        }
    }
}

/// A notification for the [`ToastStack`].
#[derive(Clone, Debug)]
pub struct Toast {
    pub title: SharedString,
    pub description: Option<SharedString>,
    pub kind: ToastKind,
    /// Seconds before it dismisses itself (paused while the stack is hovered).
    pub duration: f32,
}

impl Toast {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), description: None, kind: ToastKind::Default, duration: 4.0 }
    }
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn kind(mut self, k: ToastKind) -> Self {
        self.kind = k;
        self
    }
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s;
        self
    }
}

struct ToastItem {
    id: u64,
    toast: Toast,
    born: Instant,
    shown_for: f32,
    y: Spring,
    inset: Spring,
    leaving: Option<Instant>,
}

const TOAST_H: f32 = 66.0;
const TOAST_W: f32 = 356.0;

/// A Sonner-style toast stack: new toasts spring in from below, older ones tuck behind as a
/// compact deck that fans out while hovered. Create it once with `cx.new(|_| ToastStack::new())`,
/// render it as the last child of a `relative()` root, and call [`ToastStack::push`].
pub struct ToastStack {
    items: Vec<ToastItem>,
    next_id: u64,
    hovered: bool,
    clock: FrameClock,
}

impl Default for ToastStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastStack {
    pub fn new() -> Self {
        Self { items: Vec::new(), next_id: 1, hovered: false, clock: FrameClock::default() }
    }

    /// Show a toast. Returns its id (for [`ToastStack::dismiss`]).
    pub fn push(&mut self, toast: Toast, cx: &mut Context<Self>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.insert(
            0,
            ToastItem {
                id,
                toast,
                born: Instant::now(),
                shown_for: 0.0,
                y: Spring::new(-30.0).with(260.0, 24.0),
                inset: Spring::new(0.0).with(260.0, 26.0),
                leaving: None,
            },
        );
        cx.notify();
        id
    }

    /// Start the exit animation of a toast.
    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.leaving.get_or_insert_with(Instant::now);
            cx.notify();
        }
    }

    /// Number of toasts currently on screen (including ones animating out).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Render for ToastStack {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme().clone();
        let dt = self.clock.tick();
        let hovered = self.hovered;
        // age toasts (time is frozen while the user reads the expanded stack)
        for it in &mut self.items {
            if !hovered && it.leaving.is_none() {
                it.shown_for += dt;
                if it.shown_for >= it.toast.duration {
                    it.leaving = Some(Instant::now());
                }
            }
        }
        self.items.retain(|it| it.leaving.is_none_or(|l| l.elapsed() < Duration::from_millis(260)));
        let live: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].leaving.is_none()).collect();
        let gap = 10.0;
        for (i, it) in self.items.iter_mut().enumerate() {
            let depth = live.iter().position(|&j| j == i).unwrap_or(0) as f32;
            let (y, inset) = if hovered { (depth * (TOAST_H + gap), 0.0) } else { (depth * 14.0, depth * 10.0) };
            it.y.target = y;
            it.inset.target = inset.min(30.0);
            it.y.step(dt);
            it.inset.step(dt);
        }
        if !self.items.is_empty() {
            window.request_animation_frame();
        }
        let n_live = live.len() as f32;
        let height = if hovered { n_live * (TOAST_H + gap) } else { TOAST_H + (n_live - 1.0).max(0.0) * 14.0 };
        let count = self.items.len();
        div()
            .id("glowkit-toast-stack")
            .absolute()
            .bottom(px(20.))
            .right(px(20.))
            .w(px(TOAST_W))
            .h(px(height.max(1.0)))
            .on_hover(cx.listener(|this, h: &bool, _, cx| {
                this.hovered = *h;
                cx.notify();
            }))
            .children(self.items.iter().enumerate().rev().map(|(i, it)| {
                let depth = live.iter().position(|&j| j == i).unwrap_or(0);
                let enter = ease(Easing::OutCubic, it.born.elapsed().as_secs_f32() / 0.35);
                let exit = it.leaving.map(|l| (l.elapsed().as_secs_f32() / 0.26).min(1.0)).unwrap_or(0.0);
                let hidden = !hovered && depth >= 3;
                let mut alpha = enter * (1.0 - exit);
                if !hovered {
                    alpha *= (1.0 - depth as f32 * 0.18).max(0.0);
                }
                if hidden {
                    alpha = 0.0;
                }
                let kc = it.toast.kind.color(&t);
                let id = it.id;
                let progress = (1.0 - it.shown_for / it.toast.duration.max(0.1)).clamp(0.0, 1.0);
                let bottom = it.y.value - (1.0 - enter) * 24.0 + exit * 12.0;
                let inset = it.inset.value + exit * 12.0;
                div()
                    .id(("toast", id))
                    .absolute()
                    .bottom(px(bottom))
                    .left(px(inset))
                    .right(px(inset))
                    .h(px(TOAST_H))
                    .opacity(alpha)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(14.))
                    .rounded(px(t.radius * 1.2))
                    .bg(t.popover)
                    .border_1()
                    .border_color(t.border)
                    .shadow(t.card_shadow(1.0))
                    .overflow_hidden()
                    .when(depth == 0 || hovered || count == 1, |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .size(px(26.))
                                .rounded_full()
                                .bg(with_alpha(kc, 0.16))
                                .text_color(kc)
                                .text_size(px(13.))
                                .font_weight(FontWeight::BOLD)
                                .child(it.toast.kind.glyph()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(13.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(t.foreground)
                                        .truncate()
                                        .child(it.toast.title.clone()),
                                )
                                .children(it.toast.description.clone().map(|d| {
                                    div().text_size(px(12.5)).text_color(t.muted_foreground).truncate().child(d)
                                })),
                        )
                        .child(
                            div()
                                .id(("toast-close", id))
                                .flex_none()
                                .size(px(22.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .text_size(px(12.))
                                .text_color(t.muted_foreground)
                                .cursor_pointer()
                                .hover(|s| s.bg(with_alpha(t.foreground, 0.08)).text_color(t.foreground))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.dismiss(id, cx)))
                                .child("\u{2715}"),
                        )
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .bottom_0()
                                .h(px(2.))
                                .w(gpui::relative(progress))
                                .bg(with_alpha(kc, 0.7)),
                        )
                    })
            }))
    }
}

