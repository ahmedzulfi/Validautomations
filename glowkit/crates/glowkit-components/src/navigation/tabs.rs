use crate::overlays::kit;
use crate::overlays::LineIcon;
use egui::{pos2, vec2, Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// Look of a [`Tabs`] strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabsStyle {
    /// Muted track with a raised "thumb" behind the active tab (shadcn default).
    #[default]
    Segmented,
    /// Text tabs over a hairline with a sliding accent underline.
    Underline,
    /// Floating pill behind the active tab.
    Pill,
}

/// A tab strip whose active indicator glides between tabs on a spring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut tab = 0;
/// use glowkit_components::{Tabs, TabsStyle};
/// ui.add(Tabs::new(&mut tab, &["Account", "Password", "Settings"]).style(TabsStyle::Underline));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Tabs<'a> {
    selected: &'a mut usize,
    labels: Vec<String>,
    icons: Vec<Option<LineIcon>>,
    style: TabsStyle,
    full_width: bool,
    height: f32,
    accent: Option<Color32>,
}

impl<'a> Tabs<'a> {
    /// Tabs with the given labels; `selected` is the active index.
    pub fn new(selected: &'a mut usize, labels: &[&str]) -> Self {
        Self {
            selected,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            icons: vec![None; labels.len()],
            style: TabsStyle::Segmented,
            full_width: false,
            height: 36.0,
            accent: None,
        }
    }
    /// Leading icons, one per tab.
    pub fn icons(mut self, icons: &[LineIcon]) -> Self {
        self.icons = (0..self.labels.len()).map(|i| icons.get(i).copied()).collect();
        self
    }
    /// Visual style.
    pub fn style(mut self, style: TabsStyle) -> Self {
        self.style = style;
        self
    }
    /// Stretch tabs to fill the available width.
    pub fn full_width(mut self, yes: bool) -> Self {
        self.full_width = yes;
        self
    }
    /// Strip height (default 36).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Indicator color for the underline / pill styles (default: accent / primary).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl Widget for Tabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.labels.len().max(1);
        let font = FontId::proportional(13.5);
        let pad_x = if self.style == TabsStyle::Underline { 14.0 } else { 16.0 };
        let widths: Vec<f32> = self
            .labels
            .iter()
            .zip(&self.icons)
            .map(|(l, i)| {
                ui.painter().layout_no_wrap(l.clone(), font.clone(), Color32::WHITE).size().x
                    + pad_x * 2.0
                    + if i.is_some() { 22.0 } else { 0.0 }
            })
            .collect();
        let inset = if self.style == TabsStyle::Segmented { 4.0 } else { 0.0 };
        let natural: f32 = widths.iter().sum::<f32>() + inset * 2.0;
        let total_w = if self.full_width { ui.available_width() } else { natural };
        let scale = if self.full_width { (total_w - inset * 2.0) / (natural - inset * 2.0) } else { 1.0 };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(total_w, self.height), Sense::hover());
        let id = ui.scope_id().with(("glowkit_tabs", &self.labels));
        let mut xs = Vec::with_capacity(n);
        let mut x = rect.min.x + inset;
        for w in &widths {
            let w = w * scale;
            xs.push((x, w));
            x += w;
        }
        let sel = (*self.selected).min(n - 1);
        // input
        for (i, &(x, w)) in xs.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, rect.min.y + inset), vec2(w, self.height - inset * 2.0));
            let tr = ui.interact(r, id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if tr.clicked() && *self.selected != i {
                *self.selected = i;
                resp.mark_changed();
            }
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let sel = (*self.selected).min(n - 1).max(sel.min(n - 1));
        let (tx, tw) = xs[sel];
        let ix = glowkit_core::anim::spring(ui, id.with("ix"), tx, 380.0, 32.0);
        let iw = glowkit_core::anim::spring(ui, id.with("iw"), tw, 380.0, 32.0);
        let p = ui.painter();
        let accent = self.accent.unwrap_or(match self.style {
            TabsStyle::Underline => t.foreground,
            _ => t.primary,
        });
        let r = t.radius * 0.8;
        match self.style {
            TabsStyle::Segmented => {
                p.rect_filled(rect, r, t.muted);
                let thumb = Rect::from_min_size(pos2(ix, rect.min.y + inset), vec2(iw, self.height - inset * 2.0));
                paint::soft_shadow(
                    p,
                    thumb,
                    r - 2.0,
                    6.0,
                    vec2(0.0, 1.0),
                    Color32::from_black_alpha(if t.dark { 70 } else { 22 }),
                );
                let thumb_fill = if t.dark { mix(t.background, t.foreground, 0.06) } else { t.background };
                p.rect_filled(thumb, r - 2.0, thumb_fill);
                if t.dark {
                    p.rect_stroke(thumb, r - 2.0, Stroke::new(1.0, with_alpha(t.foreground, 0.06)), StrokeKind::Inside);
                }
            }
            TabsStyle::Underline => {
                p.line_segment(
                    [rect.left_bottom() - vec2(0.0, 0.5), rect.right_bottom() - vec2(0.0, 0.5)],
                    Stroke::new(1.0, t.border),
                );
                let bar = Rect::from_min_size(pos2(ix + 6.0, rect.max.y - 2.0), vec2((iw - 12.0).max(4.0), 2.0));
                p.rect_filled(bar, 1.0, accent);
            }
            TabsStyle::Pill => {
                let pill = Rect::from_min_size(pos2(ix, rect.min.y), vec2(iw, self.height));
                p.rect_filled(pill, self.height / 2.0, accent);
            }
        }
        for (i, &(x, w)) in xs.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, rect.min.y), vec2(w, self.height));
            let hovered = ui.rect_contains_pointer(r);
            let h = kit::hover_anim(ui, id.with(("h", i)), hovered);
            // how much of the indicator covers this tab (for smooth color handoff)
            let cover = ((ix + iw).min(x + w) - ix.max(x)).max(0.0) / w.max(1.0);
            let on = match self.style {
                TabsStyle::Pill => t.primary_foreground,
                _ => t.foreground,
            };
            let off = t.muted_foreground.lerp_to_gamma(t.foreground, h * 0.6);
            let col = off.lerp_to_gamma(on, cover.clamp(0.0, 1.0));
            if self.style == TabsStyle::Pill && h > 0.01 && cover < 0.5 {
                p.rect_filled(r, self.height / 2.0, with_alpha(t.foreground, 0.06 * h));
            }
            let g = p.layout_no_wrap(self.labels[i].clone(), font.clone(), col);
            let icon_w = if self.icons[i].is_some() { 22.0 } else { 0.0 };
            let content_w = g.size().x + icon_w;
            let cx = r.center().x - content_w / 2.0;
            if let Some(ic) = self.icons[i] {
                ic.paint(p, Rect::from_center_size(pos2(cx + 8.0, r.center().y), vec2(15.0, 15.0)), col);
            }
            p.galley(pos2(cx + icon_w, r.center().y - g.size().y / 2.0), g, col);
        }
        resp
    }
}
