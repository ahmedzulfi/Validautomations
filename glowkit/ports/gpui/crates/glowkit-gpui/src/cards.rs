//! Effect cards: [`BorderBeam`], [`SpotlightCard`] and [`TiltCard`]. All three are containers:
//! they implement [`ParentElement`] and [`Styled`], so you size and fill them like a `div`.

use crate::anim::{approach, secs, FrameClock, Spring, Spring2};
use crate::color::{mix, with_alpha};
use crate::paint::{glow, perimeter_point, stroke_outline_fn};
use crate::text::sub_id;
use crate::theme::ActiveTheme;
use gpui::prelude::*;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, px, AnyElement, Animation, AnimationExt, App, Bounds,
    BoxShadow, Div, ElementId, Hsla, Pixels, Point, StyleRefinement, Window,
};

macro_rules! container_impls {
    ($t:ty) => {
        impl ParentElement for $t {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements)
            }
        }
        impl Styled for $t {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }
    };
}

/// Paint-time bounds of an element, captured by an invisible absolutely positioned canvas.
fn bounds_probe<S: 'static>(
    state: gpui::Entity<S>,
    set: impl Fn(&mut S, Bounds<Pixels>) + 'static,
) -> impl IntoElement {
    canvas(move |b, _, cx| state.update(cx, |s, _| set(s, b)), |_, _, _, _| {}).absolute().inset_0()
}

// ---------------------------------------------------------------------------------------------

/// A card with a beam of light travelling around its border, fading from the theme's third
/// gradient color (tail) into the first (head). Port of `glowkit_effects::BorderBeam`.
#[derive(IntoElement)]
pub struct BorderBeam {
    id: ElementId,
    base: Div,
    children: Vec<AnyElement>,
    from: Option<Hsla>,
    to: Option<Hsla>,
    duration: f32,
    length: f32,
    thickness: f32,
    delay: f32,
    reverse: bool,
    radius: Option<f32>,
}
container_impls!(BorderBeam);

impl BorderBeam {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            children: Vec::new(),
            from: None,
            to: None,
            duration: 6.0,
            length: 0.22,
            thickness: 1.6,
            delay: 0.0,
            reverse: false,
            radius: None,
        }
    }
    /// Tail color. Defaults to the theme's third gradient color.
    pub fn from(mut self, c: Hsla) -> Self {
        self.from = Some(c);
        self
    }
    /// Head color. Defaults to the theme's first gradient color.
    pub fn to(mut self, c: Hsla) -> Self {
        self.to = Some(c);
        self
    }
    /// Seconds per lap.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.2);
        self
    }
    /// Beam length as a fraction of the perimeter.
    pub fn length(mut self, l: f32) -> Self {
        self.length = l.clamp(0.02, 0.9);
        self
    }
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t;
        self
    }
    /// Start offset in seconds (stagger several beams).
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s;
        self
    }
    /// Travel counter-clockwise.
    pub fn reverse(mut self, r: bool) -> Self {
        self.reverse = r;
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = Some(r);
        self
    }
}

impl RenderOnce for BorderBeam {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme();
        let radius = self.radius.unwrap_or(t.radius * 1.4);
        let from = self.from.unwrap_or(t.gradient[2]);
        let to = self.to.unwrap_or(t.gradient[0]);
        let (len, width, rev) = (self.length, self.thickness, self.reverse);
        let offset = self.delay / self.duration;
        let dir = if rev { -1.0 } else { 1.0 };
        self.base
            .id(self.id.clone())
            .relative()
            .rounded(px(radius))
            .bg(t.surface())
            .border_1()
            .border_color(t.border)
            .shadow(t.card_shadow(0.0))
            .children(self.children)
            .child(div().absolute().inset_0().rounded(px(radius)).overflow_hidden().with_animation(
                sub_id(&self.id, "beam"),
                Animation::new(secs(self.duration)).repeat(),
                move |el, phase| {
                    let mut head = (phase + offset).rem_euclid(1.0);
                    if rev {
                        head = 1.0 - head;
                    }
                    el.child(
                        canvas(
                            |_, _, _| (),
                            move |b, _, window, _| {
                                let hp = perimeter_point(b, radius, head);
                                glow(window, hp, 4.0, 40.0, with_alpha(to, 0.14));
                                stroke_outline_fn(window, b, radius, width, |s, _| {
                                    let mut d = ((head - s) * dir).rem_euclid(1.0);
                                    if d > 0.5 + len / 2.0 {
                                        d -= 1.0;
                                    }
                                    // soft rounded head, long fading tail: no hard ends
                                    let front = (-(d / 0.012).powi(2)).exp();
                                    let k = if (0.0..len).contains(&d) {
                                        let u = 1.0 - d / len;
                                        u * u * (3.0 - 2.0 * u)
                                    } else {
                                        0.0
                                    };
                                    let k = k.max(if d < 0.0 { front } else { 0.0 });
                                    if k < 0.01 {
                                        return with_alpha(to, 0.0);
                                    }
                                    with_alpha(mix(from, to, k), k)
                                });
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                },
            ))
    }
}

// ---------------------------------------------------------------------------------------------

struct SpotState {
    bounds: Bounds<Pixels>,
    pointer: Option<Point<Pixels>>,
    hovered: bool,
    alpha: f32,
    clock: FrameClock,
}

/// A card lit by a soft spotlight that follows the pointer, also catching the nearby border.
#[derive(IntoElement)]
pub struct SpotlightCard {
    id: ElementId,
    base: Div,
    children: Vec<AnyElement>,
    color: Option<Hsla>,
    size: f32,
}
container_impls!(SpotlightCard);

impl SpotlightCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), base: div(), children: Vec::new(), color: None, size: 90.0 }
    }
    /// Spotlight color. Defaults to a light tint of the accent.
    pub fn color(mut self, c: Hsla) -> Self {
        self.color = Some(c);
        self
    }
    /// Spotlight radius in pixels.
    pub fn spot_size(mut self, r: f32) -> Self {
        self.size = r;
        self
    }
}

impl RenderOnce for SpotlightCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let st = window.use_keyed_state(sub_id(&self.id, "spot"), cx, |_, _| SpotState {
            bounds: Bounds::default(),
            pointer: None,
            hovered: false,
            alpha: 0.0,
            clock: FrameClock::default(),
        });
        let (alpha, pointer, moving) = st.update(cx, |s, _| {
            let dt = s.clock.tick();
            let target = if s.hovered { 1.0 } else { 0.0 };
            s.alpha = approach(s.alpha, target, 10.0, dt);
            (s.alpha, s.pointer, (s.alpha - target).abs() > 1e-3)
        });
        if moving {
            window.request_animation_frame();
        }
        let spot = self.color.unwrap_or(mix(t.accent, gpui::white(), 0.35));
        let radius = t.radius * 1.4;
        let size = self.size;
        let border = t.border;
        let (s1, s2) = (st.clone(), st.clone());
        self.base
            .id(self.id)
            .relative()
            .rounded(px(radius))
            .bg(t.surface())
            .border_1()
            .border_color(t.border)
            .shadow(t.card_shadow(alpha * 0.6))
            .on_mouse_move(move |e, _, cx| {
                s1.update(cx, |s, cx| {
                    s.pointer = Some(e.position);
                    s.hovered = true;
                    cx.notify();
                })
            })
            .on_hover(move |h, _, cx| {
                s2.update(cx, |s, cx| {
                    s.hovered = *h;
                    cx.notify();
                })
            })
            .child(bounds_probe(st, |s, b| s.bounds = b))
            .child(
                div().absolute().inset_0().rounded(px(radius)).overflow_hidden().child(
                    canvas(
                        |_, _, _| (),
                        move |b, _, window, _| {
                            let Some(p) = pointer else { return };
                            if alpha < 0.005 {
                                return;
                            }
                            glow(window, p, size * 0.35, size, with_alpha(spot, 0.20 * alpha));
                            stroke_outline_fn(window, b, radius, 1.0, |_, q| {
                                let (dx, dy) = (f32::from(q.x - p.x), f32::from(q.y - p.y));
                                let k = (1.0 - (dx * dx + dy * dy).sqrt() / (size * 1.6)).clamp(0.0, 1.0);
                                with_alpha(mix(border, spot, k), (k * k * alpha).min(1.0))
                            });
                        },
                    )
                    .absolute()
                    .inset_0(),
                ),
            )
            .child(div().relative().children(self.children))
    }
}

// ---------------------------------------------------------------------------------------------

struct TiltState {
    bounds: Bounds<Pixels>,
    tilt: Spring2,
    lift: Spring,
    clock: FrameClock,
}

/// A card that tilts toward the pointer.
///
/// GPUI has no 3D transforms for elements, so the tilt is emulated in 2.5D: the card and its
/// content shift with parallax toward the pointer, a glare sweeps across from the pointer side,
/// and the shadow counter-shifts and deepens as the card "lifts".
#[derive(IntoElement)]
pub struct TiltCard {
    id: ElementId,
    base: Div,
    children: Vec<AnyElement>,
    max_shift: f32,
    glare: bool,
}
container_impls!(TiltCard);

impl TiltCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), base: div(), children: Vec::new(), max_shift: 7.0, glare: true }
    }
    /// Maximum parallax shift in pixels.
    pub fn max_shift(mut self, px: f32) -> Self {
        self.max_shift = px;
        self
    }
    pub fn glare(mut self, on: bool) -> Self {
        self.glare = on;
        self
    }
}

impl RenderOnce for TiltCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let st = window.use_keyed_state(sub_id(&self.id, "tilt"), cx, |_, _| TiltState {
            bounds: Bounds::default(),
            tilt: Spring2::new(170.0, 18.0),
            lift: Spring::new(0.0).with(170.0, 22.0),
            clock: FrameClock::default(),
        });
        let (tx, ty, lift, settled) = st.update(cx, |s, _| {
            let dt = s.clock.tick();
            let (x, y) = s.tilt.step(dt);
            let l = s.lift.step(dt);
            (x, y, l, s.tilt.is_settled() && s.lift.is_settled())
        });
        if !settled {
            window.request_animation_frame();
        }
        let m = self.max_shift;
        let radius = t.radius * 1.4;
        let shadow_a = if t.dark { 0.55 } else { 0.14 };
        // gradient pointing away from the pointer, so the light sits on the pointer's side
        let angle = (-tx).atan2(ty).to_degrees().rem_euclid(360.0);
        let glare = with_alpha(gpui::white(), (if t.dark { 0.16 } else { 0.32 }) * lift.clamp(0.0, 1.0));
        let (s1, s2) = (st.clone(), st.clone());
        let card = self
            .base
            .relative()
            .left(px(tx * m))
            .top(px(ty * m))
            .rounded(px(radius))
            .bg(t.surface())
            .border_1()
            .border_color(mix(t.border, t.muted_foreground, 0.25 * lift))
            .shadow(vec![BoxShadow {
                color: with_alpha(gpui::black(), shadow_a * (0.4 + 0.6 * lift)),
                offset: point(px(-tx * m * 1.6), px(6.0 - ty * m * 1.6 + 8.0 * lift)),
                blur_radius: px(16.0 + 18.0 * lift),
                spread_radius: px(0.),
            }])
            .child(div().relative().left(px(tx * m * 0.5)).top(px(ty * m * 0.5)).children(self.children))
            .when(self.glare, |el| {
                el.child(div().absolute().inset_0().rounded(px(radius)).bg(linear_gradient(
                    angle,
                    linear_color_stop(glare, 0.0),
                    linear_color_stop(with_alpha(glare, 0.0), 0.6),
                )))
            });
        div()
            .id(self.id)
            .relative()
            .on_mouse_move(move |e, _, cx| {
                s1.update(cx, |s, cx| {
                    let c = s.bounds.center();
                    let hw = f32::from(s.bounds.size.width).max(1.0) / 2.0;
                    let hh = f32::from(s.bounds.size.height).max(1.0) / 2.0;
                    let nx = (f32::from(e.position.x - c.x) / hw).clamp(-1.0, 1.0);
                    let ny = (f32::from(e.position.y - c.y) / hh).clamp(-1.0, 1.0);
                    s.tilt.set_target(nx, ny);
                    s.lift.target = 1.0;
                    cx.notify();
                })
            })
            .on_hover(move |h, _, cx| {
                if !h {
                    s2.update(cx, |s, cx| {
                        s.tilt.set_target(0.0, 0.0);
                        s.lift.target = 0.0;
                        cx.notify();
                    })
                }
            })
            .child(bounds_probe(st, |s, b| s.bounds = b))
            .child(card)
    }
}
