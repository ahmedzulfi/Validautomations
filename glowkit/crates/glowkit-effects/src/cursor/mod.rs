//! Cursor & interaction effects: click sparks, ripples, pointer trails, custom cursors,
//! crosshairs, magnets, confetti and fireworks.
//!
//! Every effect is bound to an area. Either paint it over a rect you already have with
//! `Effect::new().show(ui, rect)`, or let it allocate an area with `ui.add(Effect::new())`.
//! Each effect can `autoplay`: when the real pointer is not over the area it follows a smooth
//! simulated path (and clicks now and then), which is handy for demos and hero sections.

mod blob;
mod confetti;
mod crosshair;
mod magnet;
mod spark;
mod trail;

pub use blob::{BlobCursor, FollowingPointer, SmoothCursor};
pub use confetti::{ConfettiBurst, Fireworks};
pub use crosshair::{Crosshair, TargetCursor};
pub use magnet::Magnet;
pub use spark::{ClickRipple, ClickSpark};
pub use trail::{CardTrail, PixelTrail, PointerTrail, RibbonTrail};

#[cfg(feature = "demos")]
mod demos;

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::all()
}

use egui::{pos2, Id, Pos2, Rect, Ui};

/// Pointer state for an effect area this frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct AreaInput {
    /// Pointer (real or simulated) position, if any.
    pub pos: Option<Pos2>,
    /// Click positions this frame.
    pub clicks: Vec<Pos2>,
    /// `true` when `pos` comes from the real pointer.
    pub real: bool,
}

/// A smooth, never-repeating-looking path inside `rect` (a Lissajous curve), used by autoplay.
pub(crate) fn lissajous(rect: Rect, t: f32) -> Pos2 {
    let c = rect.center();
    let rx = rect.width() * 0.36;
    let ry = rect.height() * 0.32;
    pos2(
        c.x + rx * (t * 0.9).sin() + rx * 0.12 * (t * 2.3).sin(),
        c.y + ry * (t * 1.37 + 0.6).sin() + ry * 0.15 * (t * 3.1).cos(),
    )
}

/// Read the pointer for an effect area without stealing clicks from widgets inside it.
///
/// With `autoplay` set and no real pointer over the area, a simulated pointer moves along
/// [`lissajous`] and "clicks" every `click_every` seconds (`0` = never).
pub(crate) fn area_input(ui: &Ui, rect: Rect, id: Id, autoplay: bool, click_every: f32) -> AreaInput {
    let hovering = ui.rect_contains_pointer(rect);
    if hovering {
        let (pos, pressed) = ui.input(|i| (i.pointer.hover_pos(), i.pointer.primary_pressed()));
        let mut clicks = Vec::new();
        if pressed {
            if let Some(p) = pos {
                clicks.push(p);
            }
        }
        glowkit_core::set_state(ui, id.with("auto_t"), -1.0f32);
        return AreaInput { pos, clicks, real: true };
    }
    if !autoplay {
        return AreaInput::default();
    }
    glowkit_core::animate(ui);
    let now = glowkit_core::time(ui);
    let pos = lissajous(rect, now * 1.8);
    let mut clicks = Vec::new();
    if click_every > 0.0 {
        let prev: f32 = glowkit_core::state(ui, id.with("auto_t"), || now);
        if prev >= 0.0 && (prev / click_every).floor() != (now / click_every).floor() {
            clicks.push(pos);
        }
        glowkit_core::set_state(ui, id.with("auto_t"), now);
    }
    AreaInput { pos: Some(pos), clicks, real: false }
}
