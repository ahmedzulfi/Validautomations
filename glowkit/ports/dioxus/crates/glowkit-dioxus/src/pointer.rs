//! Element measuring and pointer-to-local coordinate helpers.

use dioxus::prelude::*;
use std::rc::Rc;

/// Bounding box of an element in client (viewport) coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Tracks an element's bounds so pointer events can be turned into element-local coordinates.
///
/// Wire `onmounted: move |e| rect.mounted(e)` on the element, then call [`ElementRect::local`]
/// from mouse handlers. Bounds are re-measured on every call to [`ElementRect::refresh`]
/// (do it on `mouseenter`/`mousemove` so scrolling never desyncs them).
#[derive(Clone, Copy, PartialEq)]
pub struct ElementRect {
    el: Signal<Option<Rc<MountedData>>>,
    bounds: Signal<Bounds>,
}

impl ElementRect {
    /// Store the mounted element and measure it.
    pub fn mounted(&self, e: MountedEvent) {
        let mut el = self.el;
        el.set(Some(e.data()));
        self.refresh();
    }
    /// Re-measure asynchronously.
    pub fn refresh(&self) {
        let Some(el) = self.el.peek().clone() else { return };
        let mut bounds = self.bounds;
        spawn(async move {
            if let Ok(r) = el.get_client_rect().await {
                let b = Bounds { x: r.origin.x, y: r.origin.y, width: r.size.width, height: r.size.height };
                if *bounds.peek() != b {
                    bounds.set(b);
                }
            }
        });
    }
    /// Last measured bounds (reactive).
    pub fn bounds(&self) -> Bounds {
        (self.bounds)()
    }
    /// Last measured bounds without subscribing.
    pub fn peek(&self) -> Bounds {
        *self.bounds.peek()
    }
    /// Pointer position relative to the element's top-left corner.
    pub fn local(&self, e: &MouseEvent) -> (f64, f64) {
        let p = e.client_coordinates();
        let b = self.peek();
        (p.x - b.x, p.y - b.y)
    }
    /// The mounted element, if any.
    pub fn element(&self) -> Option<Rc<MountedData>> {
        self.el.peek().clone()
    }
}

/// See [`ElementRect`].
pub fn use_element_rect() -> ElementRect {
    ElementRect { el: use_signal(|| None), bounds: use_signal(Bounds::default) }
}
