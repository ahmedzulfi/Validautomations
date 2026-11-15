//! Sonner-style toast stack.
//!
//! Wrap your app in [`ToastProvider`], then push toasts from anywhere below it:
//!
//! ```ignore
//! let toaster = use_toaster();
//! rsx! { button { onclick: move |_| { toaster.push(Toast::success("Saved").description("Your changes are live.")); }, "Save" } }
//! ```

use crate::anim::sleep_ms;
use dioxus::prelude::*;
use dioxus::core::spawn_forever;
use std::collections::HashMap;

/// Visual flavour of a [`Toast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ToastKind {
    #[default]
    Default,
    Success,
    Error,
    Info,
    Warning,
    /// Spinning loader; never auto-dismisses until updated with [`Toaster::update`].
    Loading,
}

impl ToastKind {
    fn class(self) -> &'static str {
        match self {
            ToastKind::Default => "",
            ToastKind::Success => "gk-toast-success",
            ToastKind::Error => "gk-toast-error",
            ToastKind::Info => "gk-toast-info",
            ToastKind::Warning => "gk-toast-warning",
            ToastKind::Loading => "gk-toast-loading",
        }
    }
    fn icon(self) -> Option<&'static str> {
        // Lucide-style 24x24 stroke icons (path data)
        match self {
            ToastKind::Default | ToastKind::Loading => None,
            ToastKind::Success => Some("M22 11.08V12a10 10 0 1 1-5.93-9.14 M22 4 12 14.01l-3-3"),
            ToastKind::Error => Some("M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20 M15 9l-6 6 M9 9l6 6"),
            ToastKind::Info => Some("M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20 M12 16v-4 M12 8h.01"),
            ToastKind::Warning => Some("M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z M12 9v4 M12 17h.01"),
        }
    }
}

/// Handle to a toast, returned by [`Toaster::push`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(pub u64);

/// A notification for the stack. Mirrors `glowkit_components::Toast`.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub description: Option<String>,
    pub action: Option<String>,
    /// Seconds before auto-dismiss (default 4). `None` keeps it until dismissed.
    pub duration: Option<f32>,
}

impl Toast {
    pub fn new(title: impl Into<String>) -> Self {
        Self { kind: ToastKind::Default, title: title.into(), description: None, action: None, duration: Some(4.0) }
    }
    pub fn success(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Success)
    }
    pub fn error(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Error)
    }
    pub fn info(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Info)
    }
    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Warning)
    }
    pub fn loading(title: impl Into<String>) -> Self {
        Self { duration: None, ..Self::new(title).kind(ToastKind::Loading) }
    }
    pub fn kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
    /// An action button (e.g. "Undo"); clicks are reported through `ToastProvider::onaction`.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }
    pub fn duration(mut self, seconds: Option<f32>) -> Self {
        self.duration = seconds;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    id: ToastId,
    toast: Toast,
    rev: u32,
    leaving: bool,
}

/// Where the stack sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ToastPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

/// Handle to the toast stack (`Copy`). Get it with [`use_toaster`].
#[derive(Clone, Copy, PartialEq)]
pub struct Toaster {
    entries: Signal<Vec<Entry>>,
    next: Signal<u64>,
}

impl Toaster {
    /// Show a toast. Returns its id.
    pub fn push(&self, toast: Toast) -> ToastId {
        let mut next = self.next;
        let mut entries = self.entries;
        next += 1;
        let id = ToastId(*next.peek());
        let dur = toast.duration;
        entries.write().push(Entry { id, toast, rev: 0, leaving: false });
        self.schedule(id, 0, dur);
        id
    }
    fn schedule(&self, id: ToastId, rev: u32, duration: Option<f32>) {
        let Some(d) = duration else { return };
        let this = *self;
        spawn_forever(async move {
            sleep_ms(d as f64 * 1000.0).await;
            let current = this.entries.peek().iter().find(|e| e.id == id).map(|e| e.rev);
            if current == Some(rev) {
                this.dismiss(id);
            }
        });
    }
    /// Replace the content of an existing toast (e.g. loading → success). Restarts its timer.
    pub fn update(&self, id: ToastId, toast: Toast) {
        let mut entries = self.entries;
        let mut rev = None;
        if let Some(e) = entries.write().iter_mut().find(|e| e.id == id) {
            e.rev += 1;
            e.toast = toast.clone();
            rev = Some(e.rev);
        }
        if let Some(r) = rev {
            self.schedule(id, r, toast.duration);
        }
    }
    /// Animate a toast out.
    pub fn dismiss(&self, id: ToastId) {
        let mut entries = self.entries;
        let found = entries.write().iter_mut().find(|e| e.id == id).map(|e| e.leaving = true).is_some();
        if found {
            spawn_forever(async move {
                sleep_ms(260.0).await;
                entries.write().retain(|e| e.id != id);
            });
        }
    }
    /// Dismiss every toast.
    pub fn dismiss_all(&self) {
        let ids: Vec<ToastId> = self.entries.peek().iter().map(|e| e.id).collect();
        for id in ids {
            self.dismiss(id);
        }
    }
    /// Toasts currently alive (including ones animating out).
    pub fn count(&self) -> usize {
        self.entries.read().len()
    }
}

/// The nearest [`ToastProvider`]'s handle.
pub fn use_toaster() -> Toaster {
    use_context::<Toaster>()
}

/// Provides a [`Toaster`] to its children and renders the animated stack: newest in front,
/// older ones tucked behind as a deck that fans out on hover.
///
/// Mirrors `glowkit_components::Toaster`: `position`, `width` (356), `visible_toasts` (3),
/// `expand` (always fanned out), `rich_colors`, `close_button`, plus `onaction`.
#[component]
pub fn ToastProvider(
    #[props(default)] position: ToastPosition,
    #[props(default = 356.0)] width: f32,
    #[props(default = 3)] visible_toasts: usize,
    #[props(default)] expand: bool,
    #[props(default)] rich_colors: bool,
    #[props(default = true)] close_button: bool,
    /// Contain the stack inside the nearest positioned ancestor instead of the viewport.
    #[props(default)]
    contained: bool,
    onaction: Option<EventHandler<ToastId>>,
    children: Element,
) -> Element {
    let toaster = use_context_provider(|| Toaster { entries: Signal::new(Vec::new()), next: Signal::new(0) });
    let mut heights = use_signal(HashMap::<u64, f64>::new);
    let mut hovered = use_signal(|| false);
    let entries = toaster.entries.read().clone();
    let fanned = expand || hovered();
    let bottom = matches!(position, ToastPosition::BottomLeft | ToastPosition::BottomCenter | ToastPosition::BottomRight);
    let pos_class = match position {
        ToastPosition::TopLeft => "gk-toasts-tl",
        ToastPosition::TopCenter => "gk-toasts-tc",
        ToastPosition::TopRight => "gk-toasts-tr",
        ToastPosition::BottomLeft => "gk-toasts-bl",
        ToastPosition::BottomCenter => "gk-toasts-bc",
        ToastPosition::BottomRight => "gk-toasts-br",
    };
    let fixed = if contained { "gk-toasts-contained" } else { "" };
    let gap = 14.0;
    let sign = if bottom { -1.0 } else { 1.0 };
    let hs = heights();
    let front_h = entries.iter().rev().find(|e| !e.leaving).and_then(|e| hs.get(&e.id.0)).copied().unwrap_or(64.0);
    // order: newest first
    let alive: Vec<Entry> = entries.iter().rev().cloned().collect();
    let mut offset = 0.0;
    let mut layout = Vec::new();
    let mut depth = 0usize;
    for e in &alive {
        let h = hs.get(&e.id.0).copied().unwrap_or(64.0);
        let d = depth;
        let (ty, scale, opacity) = if fanned {
            (sign * offset, 1.0, 1.0)
        } else {
            (sign * d as f64 * 14.0, 1.0 - d as f64 * 0.05, if d < visible_toasts { 1.0 } else { 0.0 })
        };
        layout.push((e.clone(), ty, scale, opacity, d, if fanned { h } else { front_h }));
        if !e.leaving {
            offset += h + gap;
            depth += 1;
        }
    }
    let stack_h = if fanned { offset } else { front_h + (depth.min(visible_toasts).saturating_sub(1)) as f64 * 14.0 };
    rsx! {
        {children}
        div {
            class: "gk-toasts {pos_class} {fixed}",
            style: "width:{width}px; height:{stack_h.max(0.0)}px;",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            for (e, ty, scale, opacity, d, clip_h) in layout {
                {
                    let id = e.id;
                    let kind = e.toast.kind;
                    let rich = if rich_colors { "gk-toast-rich" } else { "" };
                    let leaving = if e.leaving { "gk-toast-leaving" } else { "" };
                    let z = 1000 - d as i32;
                    let collapsed_h = if fanned || d == 0 { String::new() } else { format!("height:{clip_h}px; overflow:hidden;") };
                    rsx! {
                        div {
                            key: "{id.0}",
                            class: "gk-toast-slot {leaving}",
                            style: "z-index:{z}; transform: translateY({ty}px) scale({scale}); opacity:{opacity}; {collapsed_h}",
                            div {
                                class: "gk-toast {kind.class()} {rich}",
                                onmounted: move |ev| async move {
                                    if let Ok(r) = ev.get_client_rect().await {
                                        heights.write().insert(id.0, r.size.height);
                                    }
                                },
                                if kind == ToastKind::Loading {
                                    span { class: "gk-toast-icon gk-spinner" }
                                } else if let Some(d) = kind.icon() {
                                    svg { class: "gk-toast-icon", view_box: "0 0 24 24", fill: "none", stroke: "currentColor",
                                        stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                        path { d }
                                    }
                                }
                                div { class: "gk-toast-body",
                                    div { class: "gk-toast-title", "{e.toast.title}" }
                                    if let Some(desc) = &e.toast.description {
                                        div { class: "gk-toast-desc", "{desc}" }
                                    }
                                }
                                if let Some(a) = &e.toast.action {
                                    button { class: "gk-toast-action",
                                        onclick: move |_| {
                                            if let Some(h) = onaction { h.call(id); }
                                            toaster.dismiss(id);
                                        },
                                        "{a}"
                                    }
                                }
                                if close_button {
                                    button { class: "gk-toast-close", "aria-label": "Close",
                                        onclick: move |_| toaster.dismiss(id),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
