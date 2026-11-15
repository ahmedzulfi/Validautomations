//! [`AgentSessionList`]: agent sessions across devices, grouped into "Needs you", "Live" and
//! "Recent", with filter tabs, device tiles, pulsing status dots and unread badges. Port of
//! `glowkit_agents::AgentSessionList`.

use crate::anim::secs;
use crate::color::{mix, with_alpha};
use crate::text::sub_id;
use crate::theme::{ActiveTheme, Theme};
use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, Animation, AnimationExt, AnyElement, App, Bounds, ElementId, FontWeight, Hsla,
    PathBuilder, Pixels, SharedString, Window,
};
use std::f32::consts::TAU;
use std::rc::Rc;

/// What an agent session is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RunState {
    /// Waiting for a free worker.
    #[default]
    Queued,
    /// Working right now.
    Running,
    /// Blocked on a person (approval, question, review).
    NeedsInput,
    /// Finished successfully.
    Done,
    /// Finished with an error.
    Failed,
    /// Paused by a person.
    Paused,
    /// Alive but doing nothing.
    Idle,
}

impl RunState {
    /// Short label, e.g. `"Running"`.
    pub fn label(self) -> &'static str {
        match self {
            RunState::Queued => "Queued",
            RunState::Running => "Running",
            RunState::NeedsInput => "Needs you",
            RunState::Done => "Done",
            RunState::Failed => "Failed",
            RunState::Paused => "Paused",
            RunState::Idle => "Idle",
        }
    }
    /// The state's signal color in theme `t`.
    pub fn color(self, t: &Theme) -> Hsla {
        match self {
            RunState::Queued | RunState::Idle => t.muted_foreground,
            RunState::Running => t.info,
            RunState::NeedsInput => t.warning,
            RunState::Done => t.success,
            RunState::Failed => t.destructive,
            RunState::Paused => mix(t.muted_foreground, t.foreground, 0.35),
        }
    }
    fn rank(self) -> u8 {
        match self {
            RunState::NeedsInput => 0,
            RunState::Running => 1,
            _ => 2,
        }
    }
}

/// Where a session runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Device {
    #[default]
    Laptop,
    Cloud,
    Phone,
    Terminal,
}

/// One row of an [`AgentSessionList`].
#[derive(Clone, Debug, Default)]
pub struct AgentSession {
    pub id: u64,
    pub title: SharedString,
    pub repo: SharedString,
    pub device: Device,
    pub device_name: SharedString,
    pub state: RunState,
    pub preview: SharedString,
    pub unread: u32,
    /// Seconds since the last activity.
    pub age: f32,
}

impl AgentSession {
    pub fn new(id: u64, title: impl Into<SharedString>, state: RunState) -> Self {
        Self { id, title: title.into(), state, ..Default::default() }
    }
    pub fn repo(mut self, s: impl Into<SharedString>) -> Self {
        self.repo = s.into();
        self
    }
    pub fn device(mut self, d: Device, name: impl Into<SharedString>) -> Self {
        self.device = d;
        self.device_name = name.into();
        self
    }
    pub fn preview(mut self, s: impl Into<SharedString>) -> Self {
        self.preview = s.into();
        self
    }
    pub fn activity(mut self, unread: u32, age: f32) -> Self {
        self.unread = unread;
        self.age = age;
        self
    }
}

fn age_label(s: f32) -> String {
    let s = s.max(0.0) as u32;
    match s {
        0..=59 => "now".into(),
        60..=3599 => format!("{}m", s / 60),
        3600..=86399 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86400),
    }
}

/// Stroke a device glyph (the same shapes glowkit draws) centered in `b`.
fn paint_device(window: &mut Window, b: Bounds<Pixels>, d: Device, col: Hsla) {
    let c = b.center();
    let (cx, cy) = (f32::from(c.x), f32::from(c.y));
    let s = 20.0;
    let p = |x: f32, y: f32| point(px(cx + x * s), px(cy + y * s));
    let mut pb = PathBuilder::stroke(px(1.3));
    match d {
        Device::Laptop => {
            pb.add_polygon(&[p(-0.36, -0.32), p(0.36, -0.32), p(0.36, 0.16), p(-0.36, 0.16)], true);
            pb.move_to(p(-0.48, 0.28));
            pb.line_to(p(0.48, 0.28));
        }
        Device::Phone => {
            pb.add_polygon(&[p(-0.22, -0.37), p(0.22, -0.37), p(0.22, 0.37), p(-0.22, 0.37)], true);
            pb.move_to(p(-0.06, 0.25));
            pb.line_to(p(0.06, 0.25));
        }
        Device::Cloud => {
            // union of three circles, sampled radially (same construction as glowkit)
            let circles = [(-0.2f32, 0.06f32, 0.17f32), (0.04, -0.06, 0.24), (0.26, 0.07, 0.16)];
            let pts: Vec<_> = (0..48)
                .map(|i| {
                    let a = i as f32 / 48.0 * TAU;
                    let (dx, dy) = (a.cos(), a.sin());
                    let mut r: f32 = 0.0;
                    for (x, y, rr) in circles {
                        let bdot = x * dx + y * dy;
                        let disc = bdot * bdot - (x * x + y * y - rr * rr);
                        if disc >= 0.0 {
                            r = r.max(bdot + disc.sqrt());
                        }
                    }
                    p(dx * r, (dy * r).min(0.22))
                })
                .collect();
            pb.add_polygon(&pts, true);
        }
        Device::Terminal => {
            pb.move_to(p(-0.3, -0.2));
            pb.line_to(p(-0.08, 0.0));
            pb.line_to(p(-0.3, 0.2));
            pb.move_to(p(0.0, 0.22));
            pb.line_to(p(0.3, 0.22));
        }
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, col);
    }
}

/// Selection callback for [`AgentSessionList`].
pub type SelectHandler = Rc<dyn Fn(u64, &mut Window, &mut App)>;

/// A session manager list: filter tabs (All / Needs input / Live) with counts, rows grouped into
/// sections and sorted by urgency then recency.
#[derive(IntoElement)]
pub struct AgentSessionList {
    id: ElementId,
    sessions: Vec<AgentSession>,
    selected: Option<u64>,
    on_select: Option<SelectHandler>,
}

impl AgentSessionList {
    pub fn new(id: impl Into<ElementId>, sessions: Vec<AgentSession>) -> Self {
        Self { id: id.into(), sessions, selected: None, on_select: None }
    }
    pub fn selected(mut self, id: Option<u64>) -> Self {
        self.selected = id;
        self
    }
    pub fn on_select(mut self, f: impl Fn(u64, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for AgentSessionList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let tab_state = window.use_keyed_state(sub_id(&self.id, "tab"), cx, |_, _| 0usize);
        let tab = *tab_state.read(cx);
        let counts = [
            self.sessions.len(),
            self.sessions.iter().filter(|s| s.state == RunState::NeedsInput).count(),
            self.sessions.iter().filter(|s| s.state == RunState::Running).count(),
        ];
        let tabs = div().flex().gap(px(4.)).px(px(10.)).pb(px(10.)).children(
            ["All", "Needs input", "Live"].into_iter().enumerate().map(|(k, label)| {
                let on = k == tab;
                let st = tab_state.clone();
                let count_c = if k == 1 && counts[1] > 0 { t.warning } else { mix(t.muted_foreground, t.background, 0.3) };
                div()
                    .id(("session-tab", k))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(26.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(on, |el| el.bg(with_alpha(t.foreground, if t.dark { 0.08 } else { 0.06 })))
                    .when(!on, |el| el.hover(|s| s.bg(with_alpha(t.foreground, 0.04))))
                    .on_click(move |_, _, cx| {
                        st.update(cx, |v, cx| {
                            *v = k;
                            cx.notify();
                        })
                    })
                    .child(div().text_color(if on { t.foreground } else { t.muted_foreground }).child(label))
                    .child(div().font_family(t.mono_family).text_size(px(10.5)).text_color(count_c).child(counts[k].to_string()))
            }),
        );
        let mut items: Vec<AgentSession> = self
            .sessions
            .into_iter()
            .filter(|s| match tab {
                1 => s.state == RunState::NeedsInput,
                2 => s.state == RunState::Running,
                _ => true,
            })
            .collect();
        items.sort_by(|a, b| a.state.rank().cmp(&b.state.rank()).then(a.age.total_cmp(&b.age)));
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut last_sec = None;
        for s in items {
            let sec = s.state.rank();
            if last_sec != Some(sec) {
                rows.push(
                    div()
                        .px(px(16.))
                        .pt(px(10.))
                        .pb(px(4.))
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(mix(t.muted_foreground, t.background, 0.2))
                        .child(["NEEDS YOU", "LIVE", "RECENT"][sec as usize])
                        .into_any_element(),
                );
                last_sec = Some(sec);
            }
            rows.push(session_row(&self.id, &t, s, self.selected, self.on_select.clone()));
        }
        div()
            .id(self.id)
            .w_full()
            .flex()
            .flex_col()
            .pt(px(10.))
            .rounded(px(t.radius * 1.4))
            .bg(t.surface())
            .border_1()
            .border_color(t.border)
            .shadow(t.card_shadow(0.0))
            .overflow_hidden()
            .child(tabs)
            .child(div().h(px(1.)).bg(t.border))
            .child(div().flex().flex_col().pb(px(6.)).children(rows))
    }
}

fn session_row(
    list_id: &ElementId,
    t: &Theme,
    s: AgentSession,
    selected: Option<u64>,
    on_select: Option<SelectHandler>,
) -> AnyElement {
    let sel = selected == Some(s.id);
    let col = s.state.color(t);
    let dim = mix(t.muted_foreground, t.foreground, 0.15);
    let device = s.device;
    let live = matches!(s.state, RunState::Running | RunState::NeedsInput);
    let id = s.id;
    let well = if t.dark { mix(t.background, t.muted, 0.6) } else { t.muted };
    let surface = t.surface();
    let pulse_id = ElementId::NamedChild(Box::new(list_id.clone()), format!("pulse{id}").into());
    let status = div()
        .absolute()
        .right(px(-3.))
        .bottom(px(-3.))
        .size(px(11.))
        .rounded_full()
        .bg(surface)
        .flex()
        .items_center()
        .justify_center()
        .child({
            let dot = div().size(px(7.)).rounded_full().bg(col);
            if live {
                dot.with_animation(
                    pulse_id,
                    Animation::new(secs(1.4)).repeat().with_easing(gpui::pulsating_between(0.35, 1.0)),
                    |el, k| el.opacity(k),
                )
                .into_any_element()
            } else {
                dot.into_any_element()
            }
        });
    div()
        .id(("session-row", id))
        .mx(px(6.))
        .px(px(10.))
        .py(px(10.))
        .flex()
        .gap(px(12.))
        .rounded(px(11.))
        .cursor_pointer()
        .when(sel, |el| {
            el.bg(mix(surface, t.foreground, if t.dark { 0.07 } else { 0.045 })).border_1().border_color(t.border)
        })
        .when(!sel, |el| el.border_1().border_color(gpui::transparent_black()).hover(|st| st.bg(with_alpha(t.foreground, 0.03))))
        .when_some(on_select, |el, cb| el.on_click(move |_, w, cx| cb(id, w, cx)))
        .child(
            div()
                .relative()
                .flex_none()
                .size(px(34.))
                .rounded(px(9.))
                .bg(well)
                .border_1()
                .border_color(t.border)
                .child(canvas(|_, _, _| (), move |b, _, w, _| paint_device(w, b, device, dim)).absolute().inset_0())
                .child(status),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.5))
                                .font_weight(if s.unread > 0 { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                                .text_color(t.foreground)
                                .child(s.title),
                        )
                        .when(s.unread > 0, |el| {
                            el.child(
                                div()
                                    .flex_none()
                                    .min_w(px(18.))
                                    .h(px(18.))
                                    .px(px(5.))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(t.accent)
                                    .text_color(t.accent_foreground)
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child(s.unread.to_string()),
                            )
                        })
                        .child(
                            div()
                                .flex_none()
                                .font_family(t.mono_family)
                                .text_size(px(10.5))
                                .text_color(t.muted_foreground)
                                .child(age_label(s.age)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .text_size(px(11.5))
                        .text_color(t.muted_foreground)
                        .child(div().text_color(col).child(s.state.label()))
                        .child("\u{00B7}")
                        .child(div().font_family(t.mono_family).text_size(px(11.)).child(s.repo))
                        .child("\u{00B7}")
                        .child(s.device_name),
                )
                .child(div().truncate().text_size(px(12.)).text_color(dim).child(s.preview)),
        )
        .into_any_element()
}
