//! The scrollable page container every page template renders into.

use crate::sections::SectionResponse;
use egui::{vec2, Align, Id, Layout, ScrollArea, Sense, Ui, UiBuilder};
use glowkit_core::Theme;

/// Collects clicks and submissions from the sections of a page.
pub(crate) struct PageCtx {
    pub t: Theme,
    /// Visible height of the page viewport.
    pub height: f32,
    pub clicked: Option<String>,
    pub submitted: Option<String>,
}

impl PageCtx {
    /// Merge a section's response into the page's.
    pub fn take(&mut self, r: SectionResponse) {
        if r.clicked.is_some() {
            self.clicked = r.clicked;
        }
        if r.submitted.is_some() {
            self.submitted = r.submitted;
        }
    }
}

/// Shared page options (height and scrolling) used by every page builder.
#[derive(Clone, Copy)]
pub(crate) struct PageOpts {
    pub height: Option<f32>,
    pub scroll: bool,
}

impl Default for PageOpts {
    fn default() -> Self {
        Self { height: None, scroll: true }
    }
}

/// Fill a viewport with the theme background and lay `add` out inside a vertical scroll area.
pub(crate) fn page(ui: &mut Ui, id: Id, opts: PageOpts, add: impl FnOnce(&mut Ui, &mut PageCtx)) -> SectionResponse {
    let t = Theme::of(ui);
    let w = ui.available_width();
    if !opts.scroll {
        // flow into the parent (e.g. an outer scroll area); paint the background behind it
        let bg = ui.painter().add(egui::Shape::Noop);
        let mut ctx = PageCtx { t: t.clone(), height: opts.height.unwrap_or(800.0), clicked: None, submitted: None };
        let inner = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.set_width(w);
            add(ui, &mut ctx);
        });
        ui.painter().set(bg, egui::Shape::rect_filled(inner.response.rect, 0.0, t.background));
        return SectionResponse { response: inner.response, clicked: ctx.clicked, submitted: ctx.submitted };
    }
    let h = opts.height.unwrap_or_else(|| ui.available_height()).max(240.0);
    let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, t.background);
    let mut ctx = PageCtx { t, height: h, clicked: None, submitted: None };
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)).id_salt(id));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    ScrollArea::vertical().id_salt(id.with("scroll")).auto_shrink([false, false]).max_height(h).show(
        &mut child,
        |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            add(ui, &mut ctx);
        },
    );
    SectionResponse { response, clicked: ctx.clicked, submitted: ctx.submitted }
}

/// Builder setters every page shares.
macro_rules! page_setters {
    () => {
        /// Fixed viewport height (default: all available height).
        pub fn height(mut self, h: f32) -> Self {
            self.opts.height = Some(h);
            self
        }
        /// Wrap the page in its own vertical scroll area (default on). Turn off when the
        /// page already lives inside a scroll area.
        pub fn scroll(mut self, on: bool) -> Self {
            self.opts.scroll = on;
            self
        }
        /// Distinguish several pages in one `Ui`.
        pub fn id_salt(mut self, s: impl Into<String>) -> Self {
            self.salt = Some(s.into());
            self
        }
    };
}
pub(crate) use page_setters;
