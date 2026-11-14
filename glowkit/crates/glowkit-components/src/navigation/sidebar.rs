use crate::overlays::kit;
use crate::overlays::{LineIcon, Placement, Tooltip};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One navigation row in a [`Sidebar`].
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarItem {
    /// Icon (always visible, also in the collapsed rail).
    pub icon: LineIcon,
    /// Text.
    pub label: String,
    /// Optional count / badge text on the right.
    pub badge: Option<String>,
}

impl SidebarItem {
    /// An item with an icon and label.
    pub fn new(icon: LineIcon, label: impl Into<String>) -> Self {
        Self { icon, label: label.into(), badge: None }
    }
    /// A badge on the right ("12", "New").
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }
}

/// A collapsible application sidebar: brand header, titled sections of icon items with
/// badges, a sliding active indicator, and a user footer. Collapsing animates the width down to
/// an icon rail (labels fade out; hovering an icon shows a tooltip).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (mut selected, mut collapsed) = (0, false);
/// use glowkit_components::{LineIcon, Sidebar, SidebarItem};
/// Sidebar::new(&mut selected, &mut collapsed)
///     .brand("Acme Inc", "Enterprise")
///     .section("Platform", vec![
///         SidebarItem::new(LineIcon::Home, "Dashboard"),
///         SidebarItem::new(LineIcon::Inbox, "Inbox").badge("12"),
///     ])
///     .user("shadcn", "m@example.com")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct Sidebar<'a> {
    selected: &'a mut usize,
    collapsed: &'a mut bool,
    brand: Option<(String, String)>,
    sections: Vec<(String, Vec<SidebarItem>)>,
    user: Option<(String, String)>,
    width: f32,
    rail_width: f32,
    height: Option<f32>,
}

impl<'a> Sidebar<'a> {
    /// `selected` is the flat index of the active item across all sections.
    pub fn new(selected: &'a mut usize, collapsed: &'a mut bool) -> Self {
        Self {
            selected,
            collapsed,
            brand: None,
            sections: Vec::new(),
            user: None,
            width: 248.0,
            rail_width: 60.0,
            height: None,
        }
    }
    /// Brand name and a sub-line (e.g. plan) in the header.
    pub fn brand(mut self, name: impl Into<String>, sub: impl Into<String>) -> Self {
        self.brand = Some((name.into(), sub.into()));
        self
    }
    /// Add a titled section of items.
    pub fn section(mut self, title: impl Into<String>, items: Vec<SidebarItem>) -> Self {
        self.sections.push((title.into(), items));
        self
    }
    /// A user card in the footer (name, email).
    pub fn user(mut self, name: impl Into<String>, email: impl Into<String>) -> Self {
        self.user = Some((name.into(), email.into()));
        self
    }
    /// Expanded width (default 248).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Collapsed rail width (default 60).
    pub fn rail_width(mut self, w: f32) -> Self {
        self.rail_width = w;
        self
    }
    /// Fixed height (defaults to all available height).
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }

    /// Show the sidebar. Returns the area response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id().with("glowkit_sidebar");
        let c =
            glowkit_core::anim::spring(ui, id.with("collapse"), if *self.collapsed { 1.0 } else { 0.0 }, 260.0, 28.0)
                .clamp(0.0, 1.0);
        let w = self.width + (self.rail_width - self.width) * c;
        let h = self.height.unwrap_or_else(|| ui.available_height());
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let clip = rect.intersect(ui.clip_rect());
        let p = ui.painter().with_clip_rect(clip);
        let bg = if t.dark { mix(t.background, t.foreground, 0.03) } else { mix(t.background, t.foreground, 0.025) };
        p.rect_filled(rect, 0.0, bg);
        p.line_segment([rect.right_top(), rect.right_bottom()], Stroke::new(1.0, t.border));
        let label_a = (1.0 - c * 1.8).clamp(0.0, 1.0);
        let mut label_p = p.clone();
        label_p.multiply_opacity(label_a);
        let x0 = rect.min.x + 12.0;
        let inner_w = rect.width() - 24.0;
        let mut y = rect.min.y + 12.0;

        // header
        if let Some((name, sub)) = &self.brand {
            let logo = Rect::from_min_size(pos2(x0 + (self.rail_width - 24.0 - 34.0) / 2.0 * c, y), vec2(34.0, 34.0));
            paint::gradient_rect(&p, logo, 9.0, 45.0, &t.gradient);
            LineIcon::Zap.paint(&p, Rect::from_center_size(logo.center(), vec2(17.0, 17.0)), Color32::WHITE);
            label_p.text(
                pos2(logo.max.x + 10.0, logo.min.y + 9.0),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(14.0),
                t.foreground,
            );
            label_p.text(
                pos2(logo.max.x + 10.0, logo.min.y + 26.0),
                Align2::LEFT_CENTER,
                sub,
                FontId::proportional(12.0),
                t.muted_foreground,
            );
            y += 34.0 + 14.0;
        }
        // collapse toggle
        {
            let tr = if c < 0.5 {
                Rect::from_min_size(pos2(rect.max.x - 12.0 - 28.0, rect.min.y + 15.0), vec2(28.0, 28.0))
            } else {
                Rect::from_min_size(pos2(rect.center().x - 14.0, y - 4.0), vec2(28.0, 28.0))
            };
            if c >= 0.5 {
                y += 30.0;
            }
            let r = kit::icon_button(ui, tr, id.with("toggle"), LineIcon::Sidebar, t.muted_foreground);
            if r.clicked() {
                *self.collapsed = !*self.collapsed;
            }
        }
        p.line_segment(
            [pos2(rect.min.x + 12.0, y - 6.0), pos2(rect.max.x - 12.0, y - 6.0)],
            Stroke::new(1.0, with_alpha(t.border, 0.8)),
        );

        // items
        let mut flat = 0usize;
        let mut active_rect = None;
        let mut rows = Vec::new();
        for (title, items) in &self.sections {
            let th = 28.0 * (1.0 - c) + 8.0 * c;
            if label_a > 0.0 {
                label_p.text(
                    pos2(x0 + 8.0, y + th / 2.0 + 2.0),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(11.5),
                    t.muted_foreground,
                );
            }
            y += th;
            for item in items {
                let r = Rect::from_min_size(pos2(x0, y), vec2(inner_w, 34.0));
                if flat == *self.selected {
                    active_rect = Some(r);
                }
                rows.push((flat, r, item));
                y += 36.0;
                flat += 1;
            }
            y += 6.0;
        }
        if let Some(ar) = active_rect {
            let ay = glowkit_core::anim::spring(ui, id.with("ay"), ar.min.y, 380.0, 30.0);
            let r = Rect::from_min_size(pos2(ar.min.x, ay), ar.size());
            p.rect_filled(
                r,
                t.radius * 0.7,
                if t.dark { mix(bg, t.foreground, 0.08) } else { mix(bg, t.foreground, 0.06) },
            );
            let bar = Rect::from_min_size(pos2(rect.min.x + 1.0, ay + 8.0), vec2(3.0, 18.0));
            p.rect_filled(bar, 2.0, t.accent);
        }
        let mut new_sel = None;
        for (i, r, item) in rows {
            let resp = ui.interact(r, id.with(("item", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with(("item", i)), resp.hovered());
            let active = i == *self.selected;
            if !active && hv > 0.01 {
                p.rect_filled(r, t.radius * 0.7, with_alpha(t.foreground, 0.045 * hv));
            }
            let col = if active { t.foreground } else { t.muted_foreground.lerp_to_gamma(t.foreground, hv) };
            let icon_x = r.min.x + 10.0 + ((self.rail_width - 24.0) / 2.0 - 18.0) * c;
            item.icon.paint(
                &p,
                Rect::from_center_size(pos2(icon_x + 8.0, r.center().y), vec2(17.0, 17.0)),
                if active { t.accent } else { col },
            );
            label_p.text(
                pos2(r.min.x + 38.0, r.center().y),
                Align2::LEFT_CENTER,
                &item.label,
                FontId::proportional(13.5),
                col,
            );
            if let Some(b) = &item.badge {
                let g = label_p.layout_no_wrap(b.clone(), FontId::proportional(11.0), t.foreground);
                let br = Rect::from_min_size(
                    pos2(r.max.x - 10.0 - g.size().x - 12.0, r.center().y - 10.0),
                    vec2(g.size().x + 12.0, 20.0),
                );
                label_p.rect_filled(br, 10.0, if active { with_alpha(t.accent, 0.2) } else { t.muted });
                label_p.galley(br.center() - g.size() / 2.0, g, if active { t.accent } else { t.muted_foreground });
                // collapsed: a dot on the icon instead
                if c > 0.5 {
                    p.circle_filled(pos2(icon_x + 16.0, r.center().y - 8.0), 3.5, t.accent);
                }
            }
            if c > 0.6 {
                Tooltip::new(item.label.clone()).placement(Placement::Right).delay(0.1).show(&resp);
            }
            if resp.clicked() {
                new_sel = Some(i);
            }
        }
        if let Some(s) = new_sel {
            *self.selected = s;
        }

        // footer
        if let Some((name, email)) = &self.user {
            let fr = Rect::from_min_size(pos2(x0, rect.max.y - 12.0 - 48.0), vec2(inner_w, 48.0));
            let resp = ui.interact(fr, id.with("user"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let hv = kit::hover_anim(ui, id.with("user"), resp.hovered());
            p.rect_filled(fr, t.radius * 0.7, with_alpha(t.foreground, 0.05 * hv));
            p.line_segment(
                [pos2(rect.min.x + 12.0, fr.min.y - 8.0), pos2(rect.max.x - 12.0, fr.min.y - 8.0)],
                Stroke::new(1.0, with_alpha(t.border, 0.8)),
            );
            let av_c = pos2(fr.min.x + 8.0 + 16.0 + ((self.rail_width - 24.0) / 2.0 - 24.0) * c, fr.center().y);
            paint::gradient_circle(&p, av_c, 16.0, &[t.chart[1], t.chart[0]]);
            p.text(
                av_c,
                Align2::CENTER_CENTER,
                name.chars().next().unwrap_or('?').to_uppercase().to_string(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            label_p.text(
                pos2(av_c.x + 26.0, fr.center().y - 8.0),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(13.5),
                t.foreground,
            );
            label_p.text(
                pos2(av_c.x + 26.0, fr.center().y + 9.0),
                Align2::LEFT_CENTER,
                email,
                FontId::proportional(12.0),
                t.muted_foreground,
            );
            LineIcon::ChevronUp.paint(
                &label_p,
                Rect::from_center_size(pos2(fr.max.x - 14.0, fr.center().y), vec2(14.0, 14.0)),
                t.muted_foreground,
            );
        }
        resp
    }
}
