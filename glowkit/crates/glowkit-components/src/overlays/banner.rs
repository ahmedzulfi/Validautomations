use super::kit;
use super::LineIcon;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Look of a [`NoticeBanner`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BannerStyle {
    /// Brand gradient fill with a light sweep (default).
    #[default]
    Gradient,
    /// Muted card with a 1px border.
    Subtle,
    /// Solid accent color.
    Accent,
    /// Warning tint.
    Warning,
}

/// What happened to a [`NoticeBanner`] this frame.
#[derive(Clone, Debug)]
pub struct BannerResponse {
    /// The banner's area.
    pub response: egui::Response,
    /// The action link was clicked.
    pub action_clicked: bool,
    /// The close button was clicked.
    pub dismissed: bool,
}

/// A dismissible announcement bar for the top of a page ("🎉 v2.0 is out → Learn more").
/// Collapses smoothly when closed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut visible = true;
/// use glowkit_components::NoticeBanner;
/// NoticeBanner::new(&mut visible, "glowkit 1.0 is here, with 120+ components")
///     .badge("New")
///     .action("Read the changelog")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct NoticeBanner<'a> {
    visible: &'a mut bool,
    text: String,
    badge: Option<String>,
    action: Option<String>,
    icon: Option<LineIcon>,
    style: BannerStyle,
    height: f32,
    dismissable: bool,
}

impl<'a> NoticeBanner<'a> {
    /// A banner controlled by `visible`.
    pub fn new(visible: &'a mut bool, text: impl Into<String>) -> Self {
        Self { visible, text: text.into(), badge: None, action: None, icon: None, style: BannerStyle::Gradient, height: 44.0, dismissable: true }
    }
    /// A small pill before the text (e.g. "New").
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }
    /// A link-style action after the text (with an arrow).
    pub fn action(mut self, text: impl Into<String>) -> Self {
        self.action = Some(text.into());
        self
    }
    /// A leading icon.
    pub fn icon(mut self, icon: LineIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Visual style.
    pub fn style(mut self, style: BannerStyle) -> Self {
        self.style = style;
        self
    }
    /// Bar height (default 44).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
    /// Show the close button (default true).
    pub fn dismissable(mut self, yes: bool) -> Self {
        self.dismissable = yes;
        self
    }

    /// Show the banner.
    pub fn show(self, ui: &mut Ui) -> BannerResponse {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("glowkit_banner", &self.text));
        let p = kit::presence(ui, id, *self.visible, 12.0);
        let h = self.height * glowkit_core::ease(glowkit_core::Easing::OutCubic, p);
        let (outer, response) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
        let mut out = BannerResponse { response, action_clicked: false, dismissed: false };
        if h < 0.5 || !ui.is_rect_visible(outer) {
            return out;
        }
        let rect = Rect::from_min_size(outer.min, vec2(outer.width(), self.height));
        let mut pnt = ui.painter().with_clip_rect(outer.intersect(ui.clip_rect()));
        pnt.multiply_opacity(p);
        let r = t.radius * 0.8;
        let (fg, sub) = match self.style {
            BannerStyle::Gradient => {
                paint::gradient_rect(&pnt, rect, r, 0.0, &t.gradient);
                // slow light sweep
                let time = glowkit_core::time(ui);
                let x = rect.min.x + ((time * 0.18).fract() * 1.6 - 0.3) * rect.width();
                paint::glow_ellipse(&pnt, pos2(x, rect.center().y), vec2(140.0, self.height * 0.9), with_alpha(Color32::WHITE, 0.16));
                glowkit_core::animate(ui);
                (Color32::WHITE, with_alpha(Color32::WHITE, 0.85))
            }
            BannerStyle::Subtle => {
                pnt.rect_filled(rect, r, t.muted);
                pnt.rect_stroke(rect, r, Stroke::new(1.0, t.border), StrokeKind::Inside);
                (t.foreground, t.muted_foreground)
            }
            BannerStyle::Accent => {
                pnt.rect_filled(rect, r, t.accent);
                (t.accent_foreground, with_alpha(t.accent_foreground, 0.85))
            }
            BannerStyle::Warning => {
                pnt.rect_filled(rect, r, mix(t.background, t.warning, if t.dark { 0.16 } else { 0.12 }));
                pnt.rect_stroke(rect, r, Stroke::new(1.0, mix(t.border, t.warning, 0.4)), StrokeKind::Inside);
                (t.foreground, t.muted_foreground)
            }
        };

        // measure content to center it
        let font = FontId::proportional(13.5);
        let text_g = pnt.layout_no_wrap(self.text.clone(), font.clone(), fg);
        let badge_g = self.badge.as_ref().map(|b| pnt.layout_no_wrap(b.clone(), FontId::proportional(11.5), fg));
        let action_g = self.action.as_ref().map(|a| pnt.layout_no_wrap(a.clone(), font.clone(), fg));
        let mut total = text_g.size().x;
        if let Some(b) = &badge_g {
            total += b.size().x + 16.0 + 10.0;
        }
        if self.icon.is_some() {
            total += 24.0;
        }
        if let Some(a) = &action_g {
            total += 18.0 + a.size().x + 18.0;
        }
        let mut x = (rect.center().x - total / 2.0).max(rect.min.x + 14.0);
        let cy = rect.center().y;
        if let Some(ic) = self.icon {
            ic.paint(&pnt, Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 16.0)), fg);
            x += 24.0;
        }
        if let Some(b) = badge_g {
            let br = Rect::from_min_size(pos2(x, cy - 10.0), vec2(b.size().x + 16.0, 20.0));
            let bfill = match self.style {
                BannerStyle::Gradient | BannerStyle::Accent => with_alpha(Color32::WHITE, 0.2),
                _ => t.accent,
            };
            let bfg = match self.style {
                BannerStyle::Gradient | BannerStyle::Accent => fg,
                _ => t.accent_foreground,
            };
            pnt.rect_filled(br, 10.0, bfill);
            pnt.galley(br.center() - b.size() / 2.0, b, bfg);
            x = br.max.x + 10.0;
        }
        pnt.galley(pos2(x, cy - text_g.size().y / 2.0), text_g.clone(), fg);
        x += text_g.size().x;
        if let Some(a) = action_g {
            x += 18.0;
            let sep_col = with_alpha(sub, 0.4);
            pnt.line_segment([pos2(x - 9.0, cy - 7.0), pos2(x - 9.0, cy + 7.0)], Stroke::new(1.0, sep_col));
            let ar = Rect::from_min_size(pos2(x, cy - 12.0), vec2(a.size().x + 20.0, 24.0));
            let ar_resp = ui.interact(ar, id.with("action"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with("action"), ar_resp.hovered());
            pnt.galley(pos2(x, cy - a.size().y / 2.0), a.clone(), fg);
            let ul_w = a.size().x * hv;
            pnt.line_segment([pos2(x, cy + 9.0), pos2(x + ul_w, cy + 9.0)], Stroke::new(1.0, fg));
            LineIcon::ArrowRight.paint(&pnt, Rect::from_center_size(pos2(x + a.size().x + 10.0 + hv * 3.0, cy), vec2(13.0, 13.0)), fg);
            out.action_clicked = ar_resp.clicked();
        }
        if self.dismissable {
            let cr = Rect::from_center_size(pos2(rect.max.x - 22.0, cy), vec2(26.0, 26.0));
            let resp = ui.interact(cr, id.with("close"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with("close"), resp.hovered());
            pnt.rect_filled(cr, 6.0, with_alpha(fg, 0.14 * hv));
            LineIcon::X.paint(&pnt, Rect::from_center_size(cr.center(), vec2(14.0, 14.0)), sub);
            if resp.clicked() {
                *self.visible = false;
                out.dismissed = true;
            }
        }
        let _ = Align2::CENTER_CENTER;
        out
    }
}
