//! Animated backgrounds: Aurora and DotGrid.
//!
//! On the web both paint into a `<canvas>` with the exact math of the egui versions (Aurora is
//! the same per-vertex "shader grid", upscaled by the browser's bilinear filter; DotGrid runs
//! the same per-dot spring field). On other renderers they fall back to a static CSS rendering.

use crate::theme::use_theme;
use dioxus::prelude::*;
use glowkit_tokens::color::{gradient, mix, Color};
use glowkit_tokens::noise::{fbm3, noise3};
use glowkit_tokens::{smoothstep, Theme};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rgb(f32, f32, f32);

impl Rgb {
    fn of(c: Color) -> Self {
        Self(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0)
    }
    fn lerp(self, o: Self, t: f32) -> Self {
        Self(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t, self.2 + (o.2 - self.2) * t)
    }
    fn scale(self, k: f32) -> Self {
        Self(self.0 * k, self.1 * k, self.2 * k)
    }
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    fn bytes(self) -> [u8; 3] {
        let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [f(self.0), f(self.1), f(self.2)]
    }
}

/// Gradient look-up table (OKLab-interpolated), same as the egui backgrounds' `Lut`.
struct Lut(Vec<Rgb>);

impl Lut {
    fn new(stops: &[Color]) -> Self {
        let n = 96;
        Self((0..n).map(|i| Rgb::of(gradient(stops, i as f32 / (n - 1) as f32))).collect())
    }
    fn sample(&self, t: f32) -> Rgb {
        let n = self.0.len();
        let x = t.clamp(0.0, 1.0) * (n - 1) as f32;
        let i = x.floor() as usize;
        self.0[i.min(n - 1)].lerp(self.0[(i + 1).min(n - 1)], x - i as f32)
    }
}

/// Parameters of one aurora frame.
#[derive(Clone, Debug, PartialEq)]
pub struct AuroraParams {
    pub colors: Vec<Color>,
    pub background: Color,
    pub dark: bool,
    pub speed: f32,
    pub amplitude: f32,
    pub blend: f32,
    pub height: f32,
}

/// Grid resolution of the aurora field (vertices = cols+1 x rows+1).
pub const AURORA_GRID: (usize, usize) = (96, 56);

/// Render one aurora frame at `time` seconds into an RGBA buffer of
/// `(AURORA_GRID.0 + 1) x (AURORA_GRID.1 + 1)` pixels. This is the egui Aurora shader verbatim.
pub fn aurora_frame(p: &AuroraParams, time: f32, out: &mut Vec<u8>) {
    let (cols, rows) = AURORA_GRID;
    let lut = Lut::new(&p.colors);
    let bg = Rgb::of(p.background);
    let time = time * p.speed;
    let (amp, blend, hgt) = (p.amplitude, p.blend, p.height);
    let layers: Vec<[(f32, f32, f32); 3]> = (0..=cols)
        .map(|c| {
            let x = c as f32 / cols as f32;
            let mut out = [(0.0, 0.0, 0.0); 3];
            for (k, o) in out.iter_mut().enumerate() {
                let kf = k as f32;
                let edge = hgt
                    + 0.07 * (kf - 1.0)
                    + amp
                        * (0.16 * fbm3(x * 2.1 + kf * 3.7 + time * 0.03, kf * 5.3, time * 0.12, 3)
                            + 0.035 * (x * 9.0 + time * 0.7 + kf * 2.1).sin());
                let reach = 0.18 + 0.32 * (0.5 + 0.5 * noise3(x * 1.6 + kf * 7.0, time * 0.09, kf + 3.0));
                let rays =
                    0.55 + 0.45 * (0.5 + 0.5 * noise3(x * 13.0 + kf * 13.0 + time * 0.1, time * 0.25, kf)).powf(1.5);
                *o = (edge, reach, rays);
            }
            out
        })
        .collect();
    out.clear();
    out.reserve((cols + 1) * (rows + 1) * 4);
    for r in 0..=rows {
        let y = r as f32 / rows as f32;
        for (ci, layer) in layers.iter().enumerate() {
            let x = ci as f32 / cols as f32;
            let mut glow = 0.0;
            let mut hue = 0.0;
            for (k, &(edge, reach, rays)) in layer.iter().enumerate() {
                let d = edge - y;
                let rim = (-(d / 0.07).powi(2)).exp();
                let up = (-d.max(0.0) / reach).exp() * rays * smoothstep(-0.09, 0.03, d);
                let v = rim * 0.55 + up * 0.75;
                let w = [1.0, 0.75, 0.55][k];
                glow += v * w;
                hue += v * w * (k as f32 * 0.1 - 0.1);
            }
            let ct = (x * 0.9 + 0.05 + hue * 0.35 + 0.06 * (time * 0.15 + x * 3.0).sin()).clamp(0.0, 1.0);
            let c = lut.sample(ct);
            let i = (glow * blend * 0.85).min(1.6);
            let px = if p.dark {
                let lit = bg.add(c.scale(i.min(1.0)));
                lit.lerp(Rgb(1.0, 1.0, 1.0), ((i - 0.9) * 0.3).clamp(0.0, 0.18))
            } else {
                bg.lerp(c, (i * 0.7).min(1.0))
            };
            let [r8, g8, b8] = px.bytes();
            out.extend_from_slice(&[r8, g8, b8, 255]);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::{Clamped, JsCast};
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

    pub fn canvas_of(e: &dioxus::prelude::MountedData) -> Option<HtmlCanvasElement> {
        e.downcast::<web_sys::Element>().and_then(|el| el.clone().dyn_into::<HtmlCanvasElement>().ok())
    }

    pub fn ctx2d(c: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
        c.get_context("2d").ok().flatten().and_then(|x| x.dyn_into::<CanvasRenderingContext2d>().ok())
    }

    pub fn put_rgba(c: &HtmlCanvasElement, w: u32, h: u32, data: &[u8]) {
        if c.width() != w {
            c.set_width(w);
        }
        if c.height() != h {
            c.set_height(h);
        }
        if let (Some(ctx), Ok(img)) =
            (ctx2d(c), web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(data), w, h))
        {
            let _ = ctx.put_image_data(&img, 0.0, 0.0);
        }
    }

    /// Whether the element is at least partly inside the viewport (skip painting otherwise).
    pub fn on_screen(c: &HtmlCanvasElement) -> bool {
        let r = c.get_bounding_client_rect();
        let vh = web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(1e9);
        r.bottom() > 0.0 && r.top() < vh && r.width() > 0.0
    }
}

fn aurora_fallback_css(p: &AuroraParams) -> String {
    let c: Vec<String> = p.colors.iter().map(|c| c.alpha(0.55).to_css()).collect();
    let g = |i: usize| c.get(i % c.len().max(1)).cloned().unwrap_or_default();
    format!(
        "background: radial-gradient(60% 45% at 20% {h}%, {a}, transparent 70%), \
         radial-gradient(55% 40% at 55% {h}%, {b}, transparent 70%), \
         radial-gradient(55% 45% at 85% {h}%, {cc}, transparent 70%), {bg};",
        h = p.height * 100.0,
        a = g(0),
        b = g(1),
        cc = g(2),
        bg = p.background.to_css()
    )
}

/// Flowing curtains of northern light: wavy, ray-streaked bands of color that drift and shimmer
/// and fade softly into the background. Children are layered on top.
///
/// Mirrors `glowkit_backgrounds::Aurora`: `colors` (default: theme gradient[0], chart[1],
/// gradient[1]), `speed`, `amplitude`, `blend` (0..1.5), `height` (lower edge as a fraction).
#[component]
pub fn Aurora(
    #[props(default)] colors: Vec<Color>,
    background: Option<Color>,
    #[props(default = 1.0)] speed: f32,
    #[props(default = 1.0)] amplitude: f32,
    #[props(default = 0.9)] blend: f32,
    #[props(default = 0.5)] height: f32,
    #[props(into, default)] class: String,
    #[props(into, default)] style: String,
    children: Element,
) -> Element {
    let theme = use_theme();
    let params = aurora_params(&theme, colors, background, speed, amplitude, blend, height);
    let mut shared = use_signal(|| params.clone());
    if *shared.peek() != params {
        shared.set(params.clone());
    }
    let fallback = aurora_fallback_css(&params);
    #[cfg(target_arch = "wasm32")]
    {
        let mut canvas = use_signal(|| None::<web_sys::HtmlCanvasElement>);
        use_future(move || async move {
            let start = crate::anim::now_ms();
            let mut buf = Vec::new();
            loop {
                let now = crate::anim::next_frame().await;
                let Some(c) = canvas.peek().clone() else { continue };
                if !web::on_screen(&c) {
                    continue;
                }
                let p = shared.peek().clone();
                aurora_frame(&p, ((now - start) / 1000.0) as f32, &mut buf);
                let (w, h) = (AURORA_GRID.0 as u32 + 1, AURORA_GRID.1 as u32 + 1);
                web::put_rgba(&c, w, h, &buf);
            }
        });
        rsx! {
            div { class: "gk-bg gk-aurora {class}", style: "{fallback} {style}",
                canvas {
                    class: "gk-bg-canvas",
                    onmounted: move |e| canvas.set(web::canvas_of(&e.data())),
                }
                div { class: "gk-bg-content", {children} }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = &mut shared;
        rsx! {
            div { class: "gk-bg gk-aurora {class}", style: "{fallback} {style}",
                div { class: "gk-bg-content", {children} }
            }
        }
    }
}

fn aurora_params(
    t: &Theme,
    colors: Vec<Color>,
    background: Option<Color>,
    speed: f32,
    amplitude: f32,
    blend: f32,
    height: f32,
) -> AuroraParams {
    let colors = if colors.is_empty() { vec![t.gradient[0], t.chart[1], t.gradient[1]] } else { colors };
    let bg = background.unwrap_or(t.background);
    AuroraParams {
        colors,
        background: bg,
        dark: glowkit_tokens::color::luminance(bg) < 0.35,
        speed,
        amplitude,
        blend: blend.clamp(0.0, 1.5),
        height: height.clamp(0.1, 0.95),
    }
}

// ---------------------------------------------------------------------------------------------

/// Parameters of the dot field.
#[derive(Clone, Debug, PartialEq)]
struct DotParams {
    gap: f32,
    dot: f32,
    base: Color,
    active: Color,
    background: Color,
    proximity: f32,
    strength: f32,
    auto_hover: bool,
    dark: bool,
}

/// A grid of dots that light up and get pushed away around the pointer, springing back into
/// place; click to send a shockwave. Children are layered on top.
///
/// Mirrors `glowkit_backgrounds::DotGrid`: `gap`, `dot_size`, `color`, `active_color`,
/// `proximity`, `strength`, `auto_hover`.
#[component]
pub fn DotGrid(
    #[props(default = 28.0)] gap: f32,
    #[props(default = 3.5)] dot_size: f32,
    color: Option<Color>,
    active_color: Option<Color>,
    background: Option<Color>,
    #[props(default = 140.0)] proximity: f32,
    #[props(default = 1.0)] strength: f32,
    #[props(default)] auto_hover: bool,
    #[props(into, default)] class: String,
    #[props(into, default)] style: String,
    children: Element,
) -> Element {
    let t = use_theme();
    let bg = background.unwrap_or(t.background);
    let base = color.unwrap_or_else(|| mix(bg, t.foreground, if t.dark { 0.2 } else { 0.22 }));
    let params = DotParams {
        gap: gap.max(6.0),
        dot: dot_size.max(0.5),
        base,
        active: active_color.unwrap_or(t.accent),
        background: bg,
        proximity: proximity.max(10.0),
        strength,
        auto_hover,
        dark: t.dark,
    };
    let mut shared = use_signal(|| params.clone());
    if *shared.peek() != params {
        shared.set(params.clone());
    }
    let fallback = format!(
        "background-color:{}; background-image: radial-gradient(circle, {} {}px, transparent {}px); \
         background-size:{g}px {g}px; background-position:center;",
        bg.to_css(),
        base.to_css(),
        params.dot * 0.5,
        params.dot * 0.5 + 0.6,
        g = params.gap
    );
    #[cfg(target_arch = "wasm32")]
    {
        let mut canvas = use_signal(|| None::<web_sys::HtmlCanvasElement>);
        // pointer in client coordinates + pending click
        let mut pointer = use_hook(|| CopyValue::new((None::<(f64, f64)>, false)));
        use_future(move || async move { dot_loop(canvas, shared, pointer).await });
        rsx! {
            div { class: "gk-bg gk-dotgrid {class}", style: "{fallback} {style}",
                onmousemove: move |e| {
                    let c = e.client_coordinates();
                    pointer.write().0 = Some((c.x, c.y));
                },
                onmouseleave: move |_| pointer.write().0 = None,
                onmousedown: move |_| pointer.write().1 = true,
                canvas {
                    class: "gk-bg-canvas gk-dotgrid-canvas",
                    onmounted: move |e| canvas.set(web::canvas_of(&e.data())),
                }
                div { class: "gk-bg-content", {children} }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = &mut shared;
        rsx! {
            div { class: "gk-bg gk-dotgrid {class}", style: "{fallback} {style}",
                div { class: "gk-bg-content", {children} }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn dot_loop(
    canvas: Signal<Option<web_sys::HtmlCanvasElement>>,
    shared: Signal<DotParams>,
    mut pointer: CopyValue<(Option<(f64, f64)>, bool)>,
) {
    use crate::anim::{next_frame, now_ms};
    let start = now_ms();
    let mut last = start;
    let mut off: Vec<(f32, f32)> = Vec::new();
    let mut vel: Vec<(f32, f32)> = Vec::new();
    let mut dims = (0usize, 0usize);
    let mut last_hp: Option<(f32, f32)> = None;
    loop {
        let now = next_frame().await;
        let dt = (((now - last) / 1000.0) as f32).clamp(0.0, 1.0 / 20.0);
        last = now;
        let Some(c) = canvas.peek().clone() else { continue };
        if !web::on_screen(&c) {
            continue;
        }
        let p = shared.peek().clone();
        let rect = c.get_bounding_client_rect();
        let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
        let (w, h) = (rect.width() as f32, rect.height() as f32);
        let (bw, bh) = ((w as f64 * dpr).round() as u32, (h as f64 * dpr).round() as u32);
        if c.width() != bw {
            c.set_width(bw);
        }
        if c.height() != bh {
            c.set_height(bh);
        }
        let Some(ctx) = web::ctx2d(&c) else { continue };
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        ctx.clear_rect(0.0, 0.0, w as f64, h as f64);

        let gap = p.gap;
        let cols = (w / gap).floor() as usize + 1;
        let rows = (h / gap).floor() as usize + 1;
        let origin = (w / 2.0 - (cols - 1) as f32 * gap * 0.5, h / 2.0 - (rows - 1) as f32 * gap * 0.5);
        let (ptr, clicked) = {
            let mut pw = pointer.write();
            let v = *pw;
            pw.1 = false;
            v
        };
        let mut hp = ptr.map(|(x, y)| ((x - rect.left()) as f32, (y - rect.top()) as f32));
        if hp.is_none() && p.auto_hover {
            let t = ((now - start) / 1000.0) as f32;
            hp = Some((w * (0.5 + 0.36 * (t * 0.43).sin()), h * (0.5 + 0.32 * (t * 0.71 + 1.0).cos())));
        }
        if dims != (cols, rows) {
            off = vec![(0.0, 0.0); cols * rows];
            vel = vec![(0.0, 0.0); cols * rows];
            dims = (cols, rows);
        }
        let pv = match (hp, last_hp) {
            (Some(a), Some(b)) if dt > 0.0 => ((a.0 - b.0) / dt, (a.1 - b.1) / dt),
            _ => (0.0, 0.0),
        };
        last_hp = hp;
        let (prox, strength) = (p.proximity, p.strength);
        let steps = 2;
        let hstep = dt / steps as f32;
        for _ in 0..steps {
            for j in 0..rows {
                for i in 0..cols {
                    let k = j * cols + i;
                    let home = (origin.0 + i as f32 * gap, origin.1 + j as f32 * gap);
                    let mut target = (0.0, 0.0);
                    if let Some(pp) = hp {
                        let d = (home.0 - pp.0, home.1 - pp.1);
                        let dist = (d.0 * d.0 + d.1 * d.1).sqrt();
                        if dist < prox && dist > 0.01 {
                            let fall = 1.0 - dist / prox;
                            let s = fall * fall * gap * 0.75 * strength / dist;
                            target = (d.0 * s, d.1 * s);
                            let pk = fall * fall * 0.9 * hstep * strength;
                            vel[k].0 += pv.0 * pk;
                            vel[k].1 += pv.1 * pk;
                            if clicked {
                                let ck = fall * 900.0 * strength / dist;
                                vel[k].0 += d.0 * ck;
                                vel[k].1 += d.1 * ck;
                            }
                        }
                    }
                    let acc = ((target.0 - off[k].0) * 140.0 - vel[k].0 * 9.0, (target.1 - off[k].1) * 140.0 - vel[k].1 * 9.0);
                    vel[k].0 += acc.0 * hstep;
                    vel[k].1 += acc.1 * hstep;
                    off[k].0 += vel[k].0 * hstep;
                    off[k].1 += vel[k].1 * hstep;
                }
            }
        }
        let base = Rgb::of(p.base);
        let active = Rgb::of(p.active);
        let tau = std::f64::consts::TAU;
        for j in 0..rows {
            for i in 0..cols {
                let home = (origin.0 + i as f32 * gap, origin.1 + j as f32 * gap);
                let o = off[j * cols + i];
                let pos = (home.0 + o.0, home.1 + o.1);
                let k = hp.map_or(0.0, |pp| {
                    let d = ((home.0 - pp.0).powi(2) + (home.1 - pp.1).powi(2)).sqrt();
                    (1.0 - d / prox).clamp(0.0, 1.0)
                });
                let k = k * k * (3.0 - 2.0 * k);
                let r = p.dot * 0.5 * (1.0 + 0.35 * k);
                if k > 0.3 {
                    let [a, b, cc] = active.bytes();
                    let alpha = ((k - 0.3) * 0.25 * if p.dark { 1.0 } else { 0.8 }).clamp(0.0, 1.0);
                    ctx.set_fill_style_str(&format!("rgba({a},{b},{cc},{alpha})"));
                    ctx.begin_path();
                    let _ = ctx.arc(pos.0 as f64, pos.1 as f64, (r * 3.0) as f64, 0.0, tau);
                    ctx.fill();
                }
                let [a, b, cc] = base.lerp(active, k).bytes();
                ctx.set_fill_style_str(&format!("rgb({a},{b},{cc})"));
                ctx.begin_path();
                let _ = ctx.arc(pos.0 as f64, pos.1 as f64, r as f64, 0.0, tau);
                ctx.fill();
            }
        }
    }
}
