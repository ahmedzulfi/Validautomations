//! Motion components: marquee, dock and animated tabs.

use crate::anim::{next_frame, now_ms, sleep_ms, use_springs, SpringConfig};
use crate::pointer::use_element_rect;
use dioxus::prelude::*;
use std::f32::consts::PI;
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Marquee
// ---------------------------------------------------------------------------------------------

/// An infinitely scrolling strip of any content (logos, testimonials, tags), horizontal or
/// vertical, with optional pause on hover and faded edges.
///
/// The content comes from `render`, called once per copy (twice) with the copy index, because a
/// seamless loop needs two identical copies of the content:
///
/// ```ignore
/// Marquee { speed: 50.0, render: move |_| rsx! { for n in ["Rust", "Dioxus"] { span { "{n}" } } } }
/// ```
///
/// Mirrors `glowkit_effects::Marquee`: `speed` (points per second), `reverse`, `vertical`,
/// `pause_on_hover`, `gap`, `fade` (edge fade length), `length` (visible height when vertical).
#[component]
pub fn Marquee(
    render: Callback<usize, Element>,
    #[props(default = 40.0)] speed: f32,
    #[props(default)] reverse: bool,
    #[props(default)] vertical: bool,
    #[props(default = true)] pause_on_hover: bool,
    #[props(default = 24.0)] gap: f32,
    #[props(default = 64.0)] fade: f32,
    length: Option<f32>,
    #[props(into, default)] class: String,
) -> Element {
    let copy = use_element_rect();
    let b = copy.bounds();
    let extent = if vertical { b.height } else { b.width } as f32;
    let period = extent + gap;
    let dur = if extent > 0.0 && speed > 0.0 { period / speed } else { 30.0 };
    let mut classes = vec!["gk-marquee"];
    if vertical {
        classes.push("gk-marquee-v");
    }
    if pause_on_hover {
        classes.push("gk-marquee-pause");
    }
    let dir = if reverse { "reverse" } else { "normal" };
    let len = length.map(|l| format!("height:{l}px;")).unwrap_or_default();
    let play = if speed > 0.0 { "running" } else { "paused" };
    rsx! {
        div {
            class: "{classes.join(\" \")} {class}",
            style: "--gk-gap:{gap}px; --gk-fade:{fade}px; --gk-dur:{dur}s; --gk-dir:{dir}; --gk-play:{play}; {len}",
            onmouseenter: move |_| copy.refresh(),
            div { class: "gk-marquee-track",
                div { class: "gk-marquee-copy", onmounted: move |e| copy.mounted(e), {render.call(0)} }
                div { class: "gk-marquee-copy", "aria-hidden": "true", {render.call(1)} }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Dock
// ---------------------------------------------------------------------------------------------

/// One icon in a [`Dock`].
#[derive(Clone, Debug, PartialEq)]
pub struct DockItem {
    /// Glyph shown on the icon (emoji or a short string).
    pub icon: String,
    /// Tooltip label.
    pub label: String,
    /// Tile gradient (two CSS colors); `None` uses the theme chart palette.
    pub colors: Option<(String, String)>,
    /// Show a "running" dot under the icon.
    pub active: bool,
}

impl DockItem {
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self { icon: icon.into(), label: label.into(), colors: None, active: false }
    }
    /// Tile gradient colors.
    pub fn colors(mut self, a: impl Into<String>, b: impl Into<String>) -> Self {
        self.colors = Some((a.into(), b.into()));
        self
    }
    /// Show the "running" indicator dot.
    pub fn active(mut self, on: bool) -> Self {
        self.active = on;
        self
    }
}

/// Visual style of dock icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockStyle {
    /// Colorful gradient app tiles (macOS-like).
    #[default]
    Tiles,
    /// Muted circles with a foreground glyph.
    Minimal,
}

/// A macOS-style dock: icons magnify smoothly around the pointer, show a tooltip label and
/// bounce when clicked.
///
/// Mirrors `glowkit_effects::Dock`: `items`, `size`, `magnification`, `distance`, `gap`,
/// `style`, `labels`, `autoplay`; `onselect` reports the clicked index.
#[component]
pub fn Dock(
    items: Vec<DockItem>,
    #[props(default = 46.0)] size: f32,
    #[props(default = 1.75)] magnification: f32,
    #[props(default = 150.0)] distance: f32,
    #[props(default = 10.0)] gap: f32,
    #[props(default)] style: DockStyle,
    #[props(default = true)] labels: bool,
    #[props(default)] autoplay: bool,
    onselect: Option<EventHandler<usize>>,
) -> Element {
    let n = items.len();
    let magnification = magnification.clamp(1.0, 3.0);
    let pad = 10.0f32;
    let big = size * magnification;
    let base_w = n as f32 * size + (n as f32 - 1.0).max(0.0) * gap + pad * 2.0;
    let zone_w = base_w + 40.0;
    let height = big + pad * 2.0 + 44.0;
    let zone = use_element_rect();
    let sizes = use_springs(vec![size; n], SpringConfig::new(320.0, 24.0));
    let mut pointer_x = use_signal(|| None::<f32>);
    let mut real = use_signal(|| false);
    let mut bounce = use_signal(|| vec![0u64; n]);
    if bounce.peek().len() != n {
        bounce.set(vec![0; n]);
    }
    let base_center = move |i: usize| 20.0 + pad + size / 2.0 + i as f32 * (size + gap);
    let mut cfg = use_signal(|| (size, big, distance, n, autoplay, base_w));
    let cur = (size, big, distance, n, autoplay, base_w);
    if *cfg.peek() != cur {
        cfg.set(cur);
    }
    // retarget whenever the pointer moves
    use_effect(move || {
        let px = pointer_x();
        let (size, big, distance, n, _, _) = cfg();
        let targets: Vec<f32> = (0..n)
            .map(|i| match px {
                Some(x) => {
                    let d = ((base_center(i) - x).abs() / distance).min(1.0);
                    size + (big - size) * ((PI * d).cos() + 1.0) / 2.0
                }
                None => size,
            })
            .collect();
        sizes.set_all(&targets);
    });
    // autoplay: sweep a simulated pointer across the dock, "clicking" now and then
    use_future(move || async move {
        let t0 = now_ms();
        let mut last_k = -1.0f32;
        loop {
            let (_, _, _, n, auto, base_w) = *cfg.peek();
            if !auto || *real.peek() || n == 0 {
                sleep_ms(200.0).await;
                continue;
            }
            next_frame().await;
            let now = ((now_ms() - t0) / 1000.0) as f32;
            let phase = now * 0.55;
            let x = if (phase / PI).floor() as i64 % 3 != 2 { Some(zone_w / 2.0 + phase.sin() * base_w * 0.5) } else { None };
            pointer_x.set(x);
            let k = (now / 2.6).floor();
            if k != last_k {
                if let (Some(x), true) = (x, last_k >= 0.0) {
                    let i = (0..n).min_by(|a, b| (base_center(*a) - x).abs().total_cmp(&(base_center(*b) - x).abs()));
                    if let Some(i) = i {
                        if let Some(v) = bounce.write().get_mut(i) {
                            *v += 1;
                        }
                    }
                }
                last_k = k;
            }
        }
    });
    let px = pointer_x();
    let hovered = px.and_then(|x| {
        (0..n)
            .filter(|i| (base_center(*i) - x).abs() < (size + gap) / 2.0 + 4.0)
            .min_by(|a, b| (base_center(*a) - x).abs().total_cmp(&(base_center(*b) - x).abs()))
    });
    let vals = sizes.values();
    let bounces = bounce();
    let style_class = if style == DockStyle::Minimal { "gk-dock gk-dock-minimal" } else { "gk-dock" };
    rsx! {
        div { class: "gk-dock-row", style: "height:{height}px;",
            div {
                class: style_class,
                style: "width:{zone_w}px; height:{height}px; --gk-dock-gap:{gap}px; --gk-dock-pad:{pad}px; --gk-dock-size:{size}px;",
                onmounted: move |e| zone.mounted(e),
                onmouseenter: move |_| {
                    real.set(true);
                    zone.refresh();
                },
                onmousemove: move |e| {
                    let (x, y) = zone.local(&e);
                    let in_zone = y as f32 > height - big - pad - 8.0 - 2.0;
                    pointer_x.set(if in_zone { Some(x as f32) } else { None });
                },
                onmouseleave: move |_| {
                    real.set(false);
                    pointer_x.set(None);
                },
                div { class: "gk-dock-panel",
                    for (i, item) in items.iter().enumerate() {
                        {
                            let s = vals.get(i).copied().unwrap_or(size);
                            let bg = match &item.colors {
                                Some((a, b)) => format!("linear-gradient(160deg, {a}, {b})"),
                                None => {
                                    let c = i % 6 + 1;
                                    format!("linear-gradient(160deg, color-mix(in oklab, var(--gk-chart-{c}), white 28%), var(--gk-chart-{c}))")
                                }
                            };
                            let show_label = labels && hovered == Some(i);
                            let key = bounces.get(i).copied().unwrap_or(0);
                            rsx! {
                                div {
                                    key: "{i}",
                                    class: "gk-dock-item",
                                    style: "width:{s}px; height:{s}px;",
                                    onclick: move |_| {
                                        if let Some(v) = bounce.write().get_mut(i) {
                                            *v += 1;
                                        }
                                        if let Some(h) = onselect {
                                            h.call(i);
                                        }
                                    },
                                    if show_label {
                                        div { class: "gk-dock-label", "{item.label}" }
                                    }
                                    div { key: "{key}", class: if key > 0 { "gk-dock-icon gk-dock-bounce" } else { "gk-dock-icon" },
                                        style: "--gk-tile:{bg}; font-size:{s * 0.5}px;",
                                        span { "{item.icon}" }
                                    }
                                    if item.active { div { class: "gk-dock-dot" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// AnimatedTabs
// ---------------------------------------------------------------------------------------------

/// How [`AnimatedTabs`] highlights the selected tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabHighlight {
    /// A raised pill inside a muted track.
    #[default]
    Pill,
    /// A line under the label.
    Underline,
    /// A solid accent-colored pill.
    Solid,
}

/// A row of tabs whose highlight springs from one tab to the next, stretching as it moves; a
/// softer hover highlight glides after the pointer.
///
/// Mirrors `glowkit_effects::AnimatedTabs`: `selected` (the `&mut usize`, as a signal),
/// `labels`, `style`, `hover_highlight`, `color`, `font_size`, `stiffness`, `damping`.
#[component]
pub fn AnimatedTabs(
    selected: Signal<usize>,
    labels: Vec<String>,
    #[props(default)] style: TabHighlight,
    #[props(default = true)] hover_highlight: bool,
    #[props(into)] color: Option<String>,
    #[props(default = 14.0)] font_size: f32,
    #[props(default = 380.0)] stiffness: f32,
    #[props(default = 30.0)] damping: f32,
) -> Element {
    let n = labels.len();
    let container = use_element_rect();
    let mut tabs = use_signal(Vec::<Option<Rc<MountedData>>>::new);
    if tabs.peek().len() != n {
        tabs.set(vec![None; n]);
    }
    // measured (left, width) of every tab, relative to the container
    let mut layout = use_signal(Vec::<(f32, f32)>::new);
    // springs: selected center, selected width, hover center, hover width, hover opacity
    let sp = use_springs(vec![0.0; 5], SpringConfig::new(stiffness.max(1.0), damping.max(0.1)));
    let mut hover = use_signal(|| None::<usize>);
    let mut measured_once = use_signal(|| false);

    let measure = move || {
        let els = tabs.peek().clone();
        let Some(root) = container.element() else { return };
        spawn(async move {
            let Ok(rr) = root.get_client_rect().await else { return };
            let mut out = Vec::with_capacity(els.len());
            for el in els.iter() {
                let r = match el {
                    Some(el) => el.get_client_rect().await.ok(),
                    None => None,
                };
                out.push(r.map(|r| ((r.origin.x - rr.origin.x) as f32, r.size.width as f32)).unwrap_or((0.0, 0.0)));
            }
            layout.set(out);
        });
    };

    // move the highlight when the selection or the layout changes
    use_effect(move || {
        let sel = selected();
        let lay = layout();
        if let Some(&(x, w)) = lay.get(sel) {
            let c = x + w / 2.0;
            if !*measured_once.peek() && w > 0.0 {
                sp.snap(0, c);
                sp.snap(1, w);
                sp.snap(2, c);
                sp.snap(3, w);
                measured_once.set(true);
            } else {
                let dx = (c - sp.target(0)).abs();
                sp.set(0, c);
                sp.set(1, w);
                // stretch: kick the width so the pill elongates in flight, then settles
                if dx > 1.0 {
                    sp.impulse(1, dx * 7.0);
                }
            }
        }
    });

    let (sc, sw, hc, hw, ho) = (sp.get(0), sp.get(1), sp.get(2), sp.get(3), sp.get(4));
    let sel = selected();
    let style_class = match style {
        TabHighlight::Pill => "gk-tabs gk-tabs-pill",
        TabHighlight::Underline => "gk-tabs gk-tabs-underline",
        TabHighlight::Solid => "gk-tabs gk-tabs-solid",
    };
    let color_var = color.map(|c| format!("--gk-tab-color:{c};")).unwrap_or_default();
    let ready = *measured_once.read();
    rsx! {
        div {
            class: style_class,
            style: "font-size:{font_size}px; {color_var}",
            role: "tablist",
            onmounted: move |e| {
                container.mounted(e);
                measure();
            },
            onmouseenter: move |_| measure(),
            onmouseleave: move |_| {
                hover.set(None);
                sp.set(4, 0.0);
            },
            if hover_highlight {
                div { class: "gk-tab-hover", style: "left:{hc - hw / 2.0}px; width:{hw}px; opacity:{ho};" }
            }
            if ready {
                div { class: "gk-tab-indicator", style: "left:{sc - sw / 2.0}px; width:{sw}px;" }
            }
            for (i, label) in labels.iter().enumerate() {
                button {
                    key: "{i}",
                    class: if i == sel { "gk-tab gk-tab-active" } else { "gk-tab" },
                    role: "tab",
                    "aria-selected": if i == sel { "true" } else { "false" },
                    onmounted: move |e| {
                        if let Some(slot) = tabs.write().get_mut(i) {
                            *slot = Some(e.data());
                        }
                        measure();
                    },
                    onmouseenter: move |_| {
                        let lay = layout.peek().clone();
                        if let Some(&(x, w)) = lay.get(i) {
                            if hover.peek().is_none() && sp.get(4) < 0.05 {
                                sp.snap(2, x + w / 2.0);
                                sp.snap(3, w);
                            }
                            sp.set(2, x + w / 2.0);
                            sp.set(3, w);
                            sp.set(4, if i == *selected.peek() { 0.0 } else { 1.0 });
                        }
                        hover.set(Some(i));
                    },
                    onclick: move |_| {
                        selected.set(i);
                        sp.set(4, 0.0);
                    },
                    "{label}"
                }
            }
        }
    }
}
