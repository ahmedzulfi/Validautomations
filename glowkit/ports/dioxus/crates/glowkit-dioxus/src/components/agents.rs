//! Agent supervision: the approval card (human in the loop for a tool call).

use crate::anim::sleep_ms;
use crate::components::ai::{Icon, IconSvg};
use dioxus::prelude::*;

/// How dangerous a requested action is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Risk {
    /// Read-only or easily undone.
    #[default]
    Low,
    /// Changes something, but recoverable.
    Medium,
    /// Destructive, external or expensive.
    High,
}

impl Risk {
    /// Label, e.g. `"High risk"`.
    pub fn label(self) -> &'static str {
        match self {
            Risk::Low => "Low risk",
            Risk::Medium => "Medium",
            Risk::High => "High risk",
        }
    }
    fn class(self) -> &'static str {
        match self {
            Risk::Low => "gk-risk-low",
            Risk::Medium => "gk-risk-medium",
            Risk::High => "gk-risk-high",
        }
    }
}

/// A person's decision on a tool call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Decision {
    Approved,
    Denied,
}

fn fmt_wait(s: u64) -> String {
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    }
}

/// One pending tool call an agent wants to make: a risk badge, the requesting agent, the tool
/// and the exact command, why it is wanted and how long it has waited, with Edit, Deny and
/// Approve. Editing opens the command inline; once decided the card morphs into a resolved
/// line.
///
/// The single-request counterpart of `glowkit_agents::ApprovalQueue` rows (same fields as
/// `ApprovalRequest`: `agent`, `tool`, `command`, `reason`, `risk`, `waiting`, `decision`).
/// `decision` may be controlled by the parent; otherwise the card remembers the click.
#[component]
pub fn ApprovalCard(
    #[props(into)] agent: String,
    #[props(into)] tool: String,
    #[props(into)] command: String,
    #[props(into, default)] reason: String,
    #[props(default)] risk: Risk,
    #[props(default)] waiting: f32,
    decision: Option<Decision>,
    #[props(default = 460.0)] width: f32,
    ondecide: Option<EventHandler<(Decision, String)>>,
) -> Element {
    let mut local = use_signal(|| None::<Decision>);
    let mut editing = use_signal(|| false);
    let mut cmd = use_signal(|| command.clone());
    let mut edited = use_signal(|| false);
    let mut secs = use_signal(|| waiting.max(0.0) as u64);
    use_future(move || async move {
        loop {
            sleep_ms(1000.0).await;
            secs += 1;
        }
    });
    let decided = decision.or(local());
    let mut decide = move |d: Decision| {
        local.set(Some(d));
        editing.set(false);
        if let Some(h) = ondecide {
            h.call((d, cmd.peek().clone()));
        }
    };
    let state_class = match decided {
        Some(Decision::Approved) => "gk-approval gk-approved",
        Some(Decision::Denied) => "gk-approval gk-denied",
        None => "gk-approval",
    };
    rsx! {
        div { class: "{state_class} {risk.class()}", style: "width:{width}px; max-width:100%;",
            div { class: "gk-approval-resolved",
                span { class: "gk-approval-check",
                    IconSvg { icon: if decided == Some(Decision::Denied) { Icon::X } else { Icon::Check }, size: 14.0, stroke: 2.6 }
                }
                span { class: "gk-approval-resolved-text",
                    strong { if decided == Some(Decision::Denied) { "Denied" } else { "Approved" } }
                    " · {tool} · "
                    code { "{cmd}" }
                }
                if decision.is_none() {
                    button { class: "gk-link-btn", onclick: move |_| local.set(None), "Undo" }
                }
            }
            div { class: "gk-approval-body",
                div { class: "gk-approval-inner",
                    div { class: "gk-approval-head",
                        span { class: "gk-risk-badge", span { class: "gk-risk-dot" } "{risk.label()}" }
                        span { class: "gk-approval-agent", "{agent}" }
                        span { class: "gk-approval-wait", IconSvg { icon: Icon::Clock, size: 13.0 } "{fmt_wait(secs())}" }
                    }
                    div { class: "gk-approval-tool",
                        IconSvg { icon: Icon::Terminal, size: 14.0 }
                        span { "{tool}" }
                        if edited() { span { class: "gk-edited", "edited" } }
                    }
                    if editing() {
                        textarea {
                            class: "gk-approval-edit",
                            value: "{cmd}",
                            rows: "2",
                            oninput: move |e| {
                                cmd.set(e.value());
                                edited.set(true);
                            },
                        }
                    } else {
                        pre { class: "gk-approval-cmd", "{cmd}" }
                    }
                    if !reason.is_empty() {
                        p { class: "gk-approval-reason", "{reason}" }
                    }
                    div { class: "gk-approval-actions",
                        button { class: "gk-btn-ghost", onclick: move |_| editing.toggle(),
                            IconSvg { icon: Icon::Pencil, size: 14.0 }
                            if editing() { "Done" } else { "Edit" }
                        }
                        div { class: "gk-grow" }
                        button { class: "gk-btn-outline", onclick: move |_| decide(Decision::Denied), "Deny" }
                        button { class: "gk-btn-solid", onclick: move |_| decide(Decision::Approved),
                            IconSvg { icon: Icon::Check, size: 14.0, stroke: 2.6 }
                            "Approve"
                        }
                    }
                }
            }
        }
    }
}
