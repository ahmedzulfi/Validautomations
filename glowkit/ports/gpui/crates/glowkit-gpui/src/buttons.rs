//! Animated buttons: [`ShimmerButton`] and [`MagneticButton`].

use crate::anim::{secs, FrameClock, Spring2};
use crate::color::{hex, with_alpha};
use crate::paint::{glow, perimeter_point, stroke_outline_fn, sweep_profile};
use crate::text::sub_id;
use crate::theme::ActiveTheme;
use gpui::prelude::*;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, px, Animation, AnimationExt, App, Bounds, BoxShadow,
    ClickEvent, ElementId, FontWeight, Hsla, Pixels, SharedString, Window,
};
use std::rc::Rc;

/// Click callback shared by glowkit buttons.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A dark pill with a spark of light racing around its border and a soft inner highlight that
/// rises on hover. Port of `glowkit_effects::ShimmerButton`.
#[derive(IntoElement)]
pub struct ShimmerButton {
    id: ElementId,
    label: SharedString,
    size: f32,
    background: Option<Hsla>,
    shimmer: Option<Hsla>,
    text_color: Option<Hsla>,
    duration: f32,
    on_click: Option<ClickHandler>,
}

impl ShimmerButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            size: 15.0,
            background: None,
            shimmer: None,
            text_color: None,
            duration: 3.0,
            on_click: None,
        }
    }
    /// Font size (the button scales with it).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    /// Fill color. Defaults to near-black in dark themes, the primary color in light ones.
    pub fn background(mut self, c: Hsla) -> Self {
        self.background = Some(c);
        self
    }
    /// Color of the travelling spark. Defaults to white.
    pub fn shimmer_color(mut self, c: Hsla) -> Self {
        self.shimmer = Some(c);
        self
    }
    pub fn text_color(mut self, c: Hsla) -> Self {
        self.text_color = Some(c);
        self
    }
    /// Seconds per lap of the spark.
    pub fn duration(mut self, s: f32) -> Self {
        self.duration = s.max(0.2);
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ShimmerButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme();
        let h = (self.size * 2.8).round();
        let radius = h / 2.0;
        let bg = self.background.unwrap_or(if t.dark { hex("#050507") } else { t.primary });
        let spark = self.shimmer.unwrap_or(gpui::white());
        let text = self.text_color.unwrap_or(gpui::white());
        let shadow_a = if t.dark { 0.5 } else { 0.25 };
        let group: SharedString = "glowkit-shimmer".into();
        let mut el = div()
            .id(self.id.clone())
            .group(group.clone())
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .h(px(h))
            .px(px(self.size * 1.6))
            .rounded(px(radius))
            .overflow_hidden()
            .bg(bg)
            .text_color(text)
            .text_size(px(self.size))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .shadow(vec![BoxShadow {
                color: with_alpha(gpui::black(), shadow_a),
                offset: point(px(0.), px(6.)),
                blur_radius: px(18.),
                spread_radius: px(0.),
            }])
            .active(|s| s.opacity(0.9))
            // inner highlight rising from the bottom, stronger on hover
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(with_alpha(spark, 0.0), 0.45),
                        linear_color_stop(with_alpha(spark, 0.26), 1.0),
                    ))
                    .opacity(0.38)
                    .group_hover(group, |s| s.opacity(1.0)),
            )
            // the spark racing around the hairline, with light spilling inward from it
            .child(div().absolute().inset_0().with_animation(
                sub_id(&self.id, "spark"),
                Animation::new(secs(self.duration)).repeat(),
                move |el, head| {
                    el.child(
                        canvas(
                            |_, _, _| (),
                            move |b: Bounds<Pixels>, _, window, _| {
                                let hp = perimeter_point(b, radius, head);
                                glow(window, hp, 6.0, f32::from(b.size.height) * 0.6, with_alpha(spark, 0.22));
                                stroke_outline_fn(window, b, radius, 1.4, |s, _| {
                                    with_alpha(spark, 0.12 + 0.88 * sweep_profile(head, s, 0.035, 0.16))
                                });
                            },
                        )
                        .size_full(),
                    )
                },
            ))
            .child(div().relative().child(self.label));
        if let Some(f) = self.on_click {
            el = el.on_click(move |e, w, cx| f(e, w, cx));
        }
        el
    }
}

// ---------------------------------------------------------------------------------------------

struct MagnetState {
    spring: Spring2,
    clock: FrameClock,
    bounds: Bounds<Pixels>,
}

/// A button that leans toward the pointer when it comes close and springs back (with a little
/// overshoot) when it leaves. The label moves a bit further than the body for depth.
#[derive(IntoElement)]
pub struct MagneticButton {
    id: ElementId,
    label: SharedString,
    strength: f32,
    reach: f32,
    size: f32,
    on_click: Option<ClickHandler>,
}

impl MagneticButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), strength: 0.35, reach: 28.0, size: 15.0, on_click: None }
    }
    /// How far the button follows the pointer (fraction of the pointer offset).
    pub fn strength(mut self, s: f32) -> Self {
        self.strength = s;
        self
    }
    /// Extra pointer-sensitive margin around the button, in pixels.
    pub fn reach(mut self, r: f32) -> Self {
        self.reach = r;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for MagneticButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme().clone();
        let st = window.use_keyed_state(sub_id(&self.id, "magnet"), cx, |_, _| MagnetState {
            spring: Spring2::new(300.0, 12.0), // glowkit's "bouncy" spring
            clock: FrameClock::default(),
            bounds: Bounds::default(),
        });
        let (dx, dy, settled) = st.update(cx, |s, _| {
            let dt = s.clock.tick();
            let (x, y) = s.spring.step(dt);
            (x, y, s.spring.is_settled())
        });
        if !settled {
            window.request_animation_frame();
        }
        let strength = self.strength;
        let h = (self.size * 2.8).round();
        let (st_move, st_hover, st_bounds) = (st.clone(), st.clone(), st);
        let mut button = div()
            .id(sub_id(&self.id, "body"))
            .relative()
            .left(px(dx))
            .top(px(dy))
            .flex()
            .items_center()
            .justify_center()
            .h(px(h))
            .px(px(self.size * 1.7))
            .rounded(px(h / 2.0))
            .bg(t.primary)
            .text_color(t.primary_foreground)
            .text_size(px(self.size))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .shadow(t.card_shadow(((dx.abs() + dy.abs()) / 12.0).min(1.0)))
            .active(|s| s.opacity(0.9))
            .child(div().relative().left(px(dx * 0.35)).top(px(dy * 0.35)).child(self.label));
        if let Some(f) = self.on_click {
            button = button.on_click(move |e, w, cx| f(e, w, cx));
        }
        div()
            .id(self.id)
            .relative()
            .p(px(self.reach))
            .on_mouse_move(move |e, _, cx| {
                st_move.update(cx, |s, cx| {
                    let c = s.bounds.center();
                    s.spring.set_target(f32::from(e.position.x - c.x) * strength, f32::from(e.position.y - c.y) * strength);
                    cx.notify();
                })
            })
            .on_hover(move |hovered, _, cx| {
                if !hovered {
                    st_hover.update(cx, |s, cx| {
                        s.spring.set_target(0.0, 0.0);
                        cx.notify();
                    })
                }
            })
            .child(
                canvas(move |b, _, cx| st_bounds.update(cx, |s, _| s.bounds = b), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
            .child(button)
    }
}
