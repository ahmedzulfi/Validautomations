use crate::overlays::kit::{self, Btn};
use crate::overlays::LineIcon;
use egui::{
    pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// What was clicked in a [`Navbar`] this frame.
#[derive(Debug)]
pub struct NavbarResponse {
    /// The bar's area.
    pub response: Response,
    /// Index of a clicked link.
    pub link_clicked: Option<usize>,
    /// Index of a clicked action button.
    pub action_clicked: Option<usize>,
}

fn brand_mark(p: &Painter, t: &Theme, center: egui::Pos2, size: f32) {
    let r = Rect::from_center_size(center, vec2(size, size));
    paint::gradient_rect(p, r, size * 0.28, 45.0, &t.gradient);
    LineIcon::Zap.paint(p, Rect::from_center_size(center, vec2(size * 0.52, size * 0.52)), Color32::WHITE);
}

/// Paint a row of links with a hover pill that glides to the hovered link and a small active
/// underline. Returns the clicked index.
#[allow(clippy::too_many_arguments)]
fn link_row(
    ui: &mut Ui,
    p: &Painter,
    id: Id,
    links: &[String],
    icons: Option<&[LineIcon]>,
    active: usize,
    x: f32,
    cy: f32,
    h: f32,
    font: f32,
    underline: bool,
) -> (Option<usize>, f32) {
    let t = Theme::of(ui);
    let fid = FontId::proportional(font);
    let mut rects = Vec::with_capacity(links.len());
    let mut cx = x;
    for (i, l) in links.iter().enumerate() {
        let tw = p.layout_no_wrap(l.clone(), fid.clone(), Color32::WHITE).size().x;
        let iw = if icons.is_some_and(|ic| i < ic.len()) { 20.0 } else { 0.0 };
        let w = tw + iw + 26.0;
        rects.push(Rect::from_min_size(pos2(cx, cy - h / 2.0), vec2(w, h)));
        cx += w + 2.0;
    }
    let mut hovered = None;
    let mut clicked = None;
    for (i, r) in rects.iter().enumerate() {
        let resp = ui.interact(*r, id.with(("link", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        if resp.hovered() {
            hovered = Some(i);
        }
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    let last_hover: usize = glowkit_core::state(ui, id.with("last_hover"), || active);
    let target_i = hovered.unwrap_or(last_hover);
    if let Some(hh) = hovered {
        glowkit_core::set_state(ui, id.with("last_hover"), hh);
    }
    let tr = rects[target_i.min(rects.len() - 1)];
    let hx = glowkit_core::anim::spring(ui, id.with("hx"), tr.min.x, 420.0, 34.0);
    let hw = glowkit_core::anim::spring(ui, id.with("hw"), tr.width(), 420.0, 34.0);
    let ha = kit::hover_anim(ui, id.with("ha"), hovered.is_some());
    if ha > 0.01 {
        let pill = Rect::from_min_size(pos2(hx, tr.min.y), vec2(hw, tr.height()));
        p.rect_filled(pill, h / 2.0, with_alpha(t.foreground, 0.07 * ha));
    }
    for (i, r) in rects.iter().enumerate() {
        let on = i == active;
        let hv = kit::hover_anim(ui, id.with(("lh", i)), hovered == Some(i));
        let col = if on { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
        let mut tx = r.min.x + 13.0;
        if let Some(ic) = icons.and_then(|ic| ic.get(i)) {
            ic.paint(p, Rect::from_center_size(pos2(tx + 7.0, r.center().y), vec2(15.0, 15.0)), col);
            tx += 20.0;
        }
        p.text(pos2(tx, r.center().y), Align2::LEFT_CENTER, &links[i], fid.clone(), col);
    }
    if underline && active < rects.len() {
        let ar = rects[active];
        let ux = glowkit_core::anim::spring(ui, id.with("ux"), ar.center().x, 380.0, 30.0);
        let uw = glowkit_core::anim::spring(ui, id.with("uw"), ar.width() - 26.0, 380.0, 30.0);
        p.rect_filled(Rect::from_center_size(pos2(ux, ar.max.y + 1.0), vec2(uw, 2.0)), 1.0, t.accent);
    }
    (clicked, cx)
}

/// A top application bar: brand on the left, links whose hover highlight glides between them
/// with an accent underline under the active page, and action buttons on the right.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut page = 0;
/// use glowkit_components::Navbar;
/// let r = Navbar::new(&mut page, &["Home", "Features", "Pricing", "Docs"])
///     .brand("glowkit")
///     .actions(&["Log in", "Get started"])
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Navbar<'a> {
    active: &'a mut usize,
    links: Vec<String>,
    brand: Option<String>,
    actions: Vec<String>,
    height: f32,
    border: bool,
}

impl<'a> Navbar<'a> {
    /// A navbar with the given links; `active` is the current page.
    pub fn new(active: &'a mut usize, links: &[&str]) -> Self {
        Self {
            active,
            links: links.iter().map(|s| s.to_string()).collect(),
            brand: None,
            actions: Vec::new(),
            height: 60.0,
            border: true,
        }
    }
    /// Brand name (with a gradient logo mark).
    pub fn brand(mut self, name: impl Into<String>) -> Self {
        self.brand = Some(name.into());
        self
    }
    /// Right-side buttons; the last one is styled as the primary action.
    pub fn actions(mut self, labels: &[&str]) -> Self {
        self.actions = labels.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Bar height (default 60).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Draw the bottom border (default true).
    pub fn border(mut self, yes: bool) -> Self {
        self.border = yes;
        self
    }
    /// Show the bar using the full available width.
    pub fn show(self, ui: &mut Ui) -> NavbarResponse {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("glowkit_navbar", &self.links));
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let mut out = NavbarResponse { response, link_clicked: None, action_clicked: None };
        let p = ui.painter().with_clip_rect(rect.expand(4.0).intersect(ui.clip_rect()));
        if self.border {
            p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t.border));
        }
        let cy = rect.center().y;
        let mut x = rect.min.x + 18.0;
        if let Some(b) = &self.brand {
            brand_mark(&p, &t, pos2(x + 15.0, cy), 30.0);
            let r = p.text(pos2(x + 40.0, cy), Align2::LEFT_CENTER, b, FontId::proportional(16.5), t.foreground);
            x = r.max.x + 28.0;
        }
        let (clicked, _) = link_row(ui, &p, id, &self.links, None, *self.active, x, cy, 34.0, 14.0, true);
        if let Some(c) = clicked {
            *self.active = c;
            out.link_clicked = Some(c);
        }
        let mut rx = rect.max.x - 18.0;
        for (i, a) in self.actions.iter().enumerate().rev() {
            let primary = i + 1 == self.actions.len();
            let w = kit::button_width(ui, a, 13.5);
            let br = Rect::from_min_max(pos2(rx - w, cy - 17.0), pos2(rx, cy + 17.0));
            if kit::button_at(ui, br, id.with(("act", i)), a, if primary { Btn::Primary } else { Btn::Ghost }).clicked()
            {
                out.action_clicked = Some(i);
            }
            rx -= w + 8.0;
        }
        out
    }
}

/// Hides when scrolling down, reappears when scrolling up (or near the top).
/// Feed it the scroll offset of your content every frame; it returns whether a
/// [`FloatingNavbar`] should be visible.
pub fn auto_hide_on_scroll(ui: &Ui, id_salt: &str, scroll_y: f32) -> bool {
    let id = ui.scope_id().with(("glowkit_autohide", id_salt));
    let (last, mut visible): (f32, bool) = glowkit_core::state(ui, id, || (scroll_y, true));
    let d = scroll_y - last;
    if scroll_y < 40.0 {
        visible = true;
    } else if d > 0.5 {
        visible = false;
    } else if d < -0.5 {
        visible = true;
    }
    glowkit_core::set_state(ui, id, (scroll_y, visible));
    visible
}

/// A floating pill navbar that sits over the top of the content and slides away when hidden
/// (pair with [`auto_hide_on_scroll`] to hide on scroll-down and return on scroll-up).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (mut active, scroll_y, content_rect) = (0, 0.0, egui::Rect::NOTHING);
/// use glowkit_components::{auto_hide_on_scroll, FloatingNavbar, LineIcon};
/// let visible = auto_hide_on_scroll(ui, "nav", scroll_y);
/// FloatingNavbar::new(&mut active, &[(LineIcon::Home, "Home"), (LineIcon::User, "About")])
///     .cta("Login")
///     .visible(visible)
///     .show(ui, content_rect);
/// ```
#[must_use = "call `.show(ui, rect)`"]
pub struct FloatingNavbar<'a> {
    active: &'a mut usize,
    items: Vec<(LineIcon, String)>,
    cta: Option<String>,
    visible: bool,
}

impl<'a> FloatingNavbar<'a> {
    /// A floating navbar with icon + label items.
    pub fn new(active: &'a mut usize, items: &[(LineIcon, &str)]) -> Self {
        Self { active, items: items.iter().map(|(i, s)| (*i, s.to_string())).collect(), cta: None, visible: true }
    }
    /// A call-to-action pill at the end (e.g. "Login").
    pub fn cta(mut self, label: impl Into<String>) -> Self {
        self.cta = Some(label.into());
        self
    }
    /// Whether the bar is shown (animated).
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    /// Paint at the top-center of `area`, over whatever is already there. Returns a clicked item
    /// index (the CTA reports `items.len()`).
    pub fn show(self, ui: &mut Ui, area: Rect) -> Option<usize> {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_floating_nav");
        let v = glowkit_core::anim::spring(ui, id.with("v"), if self.visible { 1.0 } else { 0.0 }, 300.0, 26.0);
        if v < 0.01 && !self.visible {
            return None;
        }
        let font = FontId::proportional(13.5);
        let labels: Vec<String> = self.items.iter().map(|(_, s)| s.clone()).collect();
        let icons: Vec<LineIcon> = self.items.iter().map(|(i, _)| *i).collect();
        let links_w: f32 = labels
            .iter()
            .map(|l| ui.painter().layout_no_wrap(l.clone(), font.clone(), Color32::WHITE).size().x + 20.0 + 26.0 + 2.0)
            .sum();
        let cta_w = self
            .cta
            .as_ref()
            .map(|c| ui.painter().layout_no_wrap(c.clone(), font.clone(), Color32::WHITE).size().x + 36.0 + 10.0)
            .unwrap_or(0.0);
        let w = links_w + cta_w + 16.0;
        let h = 50.0;
        let top = area.min.y + 18.0 - (1.0 - v) * 90.0;
        let rect = Rect::from_min_size(pos2(area.center().x - w / 2.0, top), vec2(w, h));
        let mut p = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
        p.multiply_opacity(v.clamp(0.0, 1.0));
        paint::soft_shadow(&p, rect, h / 2.0, 22.0, vec2(0.0, 8.0), kit::shadow_color(&t));
        let fill = if t.dark { mix(t.background, t.foreground, 0.04) } else { t.background };
        p.rect_filled(rect, h / 2.0, fill);
        p.rect_stroke(rect, h / 2.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
        ui.interact(rect, id.with("block"), Sense::hover());
        let (clicked, _) = link_row(
            ui,
            &p,
            id,
            &labels,
            Some(&icons),
            *self.active,
            rect.min.x + 8.0,
            rect.center().y,
            34.0,
            13.5,
            false,
        );
        let mut out = None;
        if let Some(c) = clicked {
            *self.active = c;
            out = Some(c);
        }
        if let Some(c) = &self.cta {
            let br = Rect::from_min_max(
                pos2(rect.max.x - 8.0 - (cta_w - 10.0), rect.center().y - 17.0),
                pos2(rect.max.x - 8.0, rect.center().y + 17.0),
            );
            let resp = ui.interact(br, id.with("cta"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with("cta"), resp.hovered());
            p.rect_filled(br, 17.0, with_alpha(t.foreground, 0.04 + 0.05 * hv));
            p.rect_stroke(br, 17.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
            // glowing gradient line along the bottom edge
            let ly = br.max.y - 0.5;
            let lw = br.width() * (0.5 + 0.2 * hv);
            paint::gradient_polyline(
                &p,
                &[pos2(br.center().x - lw / 2.0, ly), pos2(br.center().x + lw / 2.0, ly)],
                false,
                1.2,
                |u| with_alpha(t.accent, (1.0 - (u * 2.0 - 1.0).abs()).powf(0.7)),
            );
            p.text(br.center(), Align2::CENTER_CENTER, c, font.clone(), t.foreground);
            if resp.clicked() {
                out = Some(self.items.len());
            }
        }
        out
    }
}

/// A full-width header that, once the page scrolls, shrinks into a narrower floating, rounded
/// bar with a border and shadow (and back when you return to the top).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (mut active, scroll_y, content_rect) = (0, 0.0, egui::Rect::NOTHING);
/// use glowkit_components::ResizableNavbar;
/// ResizableNavbar::new(&mut active, &["Features", "Pricing", "Contact"])
///     .brand("glowkit")
///     .cta("Book a call")
///     .show(ui, content_rect, scroll_y);
/// ```
#[must_use = "call `.show(ui, rect, scroll_y)`"]
pub struct ResizableNavbar<'a> {
    active: &'a mut usize,
    links: Vec<String>,
    brand: String,
    cta: Option<String>,
    threshold: f32,
    shrunk_width: f32,
}

impl<'a> ResizableNavbar<'a> {
    /// A resizable navbar with the given links.
    pub fn new(active: &'a mut usize, links: &[&str]) -> Self {
        Self {
            active,
            links: links.iter().map(|s| s.to_string()).collect(),
            brand: "Brand".into(),
            cta: None,
            threshold: 40.0,
            shrunk_width: 680.0,
        }
    }
    /// Brand name.
    pub fn brand(mut self, name: impl Into<String>) -> Self {
        self.brand = name.into();
        self
    }
    /// Primary action button.
    pub fn cta(mut self, label: impl Into<String>) -> Self {
        self.cta = Some(label.into());
        self
    }
    /// Scroll distance after which it shrinks (default 40).
    pub fn threshold(mut self, px: f32) -> Self {
        self.threshold = px;
        self
    }
    /// Width when shrunk (default 680).
    pub fn shrunk_width(mut self, w: f32) -> Self {
        self.shrunk_width = w;
        self
    }
    /// Paint at the top of `area` given the content's scroll offset. Returns a clicked link index
    /// (the CTA reports `links.len()`).
    pub fn show(self, ui: &mut Ui, area: Rect, scroll_y: f32) -> Option<usize> {
        let t = Theme::of(ui);
        let id = ui.scope_id().with("glowkit_resizable_nav");
        let s = glowkit_core::anim::spring(
            ui,
            id.with("s"),
            if scroll_y > self.threshold { 1.0 } else { 0.0 },
            220.0,
            24.0,
        );
        let sc = s.clamp(0.0, 1.0);
        let full = area.width();
        let w = full + (self.shrunk_width.min(full - 24.0) - full) * s;
        let h = 62.0 - 10.0 * sc;
        let top = area.min.y + 12.0 * sc;
        let rect = Rect::from_min_size(pos2(area.center().x - w / 2.0, top), vec2(w, h));
        let r = (h / 2.0) * sc + t.radius * (1.0 - sc) * 0.0;
        let p = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
        if sc > 0.01 {
            paint::soft_shadow(&p, rect, r, 24.0 * sc, vec2(0.0, 8.0 * sc), with_alpha(kit::shadow_color(&t), sc));
        }
        let fill_a = 0.86 + 0.14 * sc;
        let fill = if t.dark { mix(t.background, t.foreground, 0.04 * sc) } else { t.background };
        p.rect_filled(rect, r, with_alpha(fill, fill_a));
        if sc > 0.01 {
            p.rect_stroke(rect, r, Stroke::new(1.0, with_alpha(t.border, sc)), StrokeKind::Inside);
        }
        p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, with_alpha(t.border, 1.0 - sc)));
        ui.interact(rect, id.with("block"), Sense::hover());
        let cy = rect.center().y;
        let x = rect.min.x + 16.0 + 4.0 * sc;
        brand_mark(&p, &t, pos2(x + 14.0, cy), 28.0);
        let br = p.text(pos2(x + 36.0, cy), Align2::LEFT_CENTER, &self.brand, FontId::proportional(15.5), t.foreground);
        // center the links
        let fid = FontId::proportional(13.5);
        let links_w: f32 =
            self.links.iter().map(|l| p.layout_no_wrap(l.clone(), fid.clone(), Color32::WHITE).size().x + 28.0).sum();
        let lx = (rect.center().x - links_w / 2.0).max(br.max.x + 16.0);
        let (clicked, _) = link_row(ui, &p, id, &self.links, None, *self.active, lx, cy, 34.0, 13.5, false);
        let mut out = None;
        if let Some(c) = clicked {
            *self.active = c;
            out = Some(c);
        }
        if let Some(c) = &self.cta {
            let bw = kit::button_width(ui, c, 13.5);
            let b = Rect::from_min_max(pos2(rect.max.x - 12.0 - bw, cy - 17.0), pos2(rect.max.x - 12.0, cy + 17.0));
            if kit::button_at(ui, b, id.with("cta"), c, Btn::Primary).clicked() {
                out = Some(self.links.len());
            }
        }
        out
    }
}
