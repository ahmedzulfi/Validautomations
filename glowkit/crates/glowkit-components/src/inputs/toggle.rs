use super::kit::{self, control_radius, ControlSize, UiIcon};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget,
};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Content of a [`SegmentedControl`] segment: a text label, an icon, or both.
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleItem {
    /// Optional icon shown before the label.
    pub icon: Option<UiIcon>,
    /// Text label (may be empty for icon-only items).
    pub label: String,
}

impl ToggleItem {
    /// Icon and text.
    pub fn new(icon: UiIcon, label: impl Into<String>) -> Self {
        Self { icon: Some(icon), label: label.into() }
    }
}

impl From<&str> for ToggleItem {
    fn from(s: &str) -> Self {
        Self { icon: None, label: s.to_owned() }
    }
}

impl From<String> for ToggleItem {
    fn from(s: String) -> Self {
        Self { icon: None, label: s }
    }
}

impl From<UiIcon> for ToggleItem {
    fn from(i: UiIcon) -> Self {
        Self { icon: Some(i), label: String::new() }
    }
}

fn item_width(ui: &Ui, item: &ToggleItem, size: ControlSize) -> f32 {
    let h = size.height();
    if item.label.is_empty() {
        return h;
    }
    let tw = kit::text_width(ui, &item.label, size.font());
    let pad = if size == ControlSize::Sm { 10.0 } else { 12.0 };
    tw + pad * 2.0 + if item.icon.is_some() { size.icon() + 7.0 } else { 0.0 }
}

fn paint_item(ui: &Ui, rect: Rect, item: &ToggleItem, size: ControlSize, color: Color32) {
    let painter = ui.painter();
    let is = size.icon();
    let galley = painter.layout_no_wrap(item.label.clone(), FontId::proportional(size.font()), color);
    let total =
        galley.size().x + if item.icon.is_some() { is + if item.label.is_empty() { 0.0 } else { 7.0 } } else { 0.0 };
    let mut x = rect.center().x - total / 2.0;
    if let Some(icon) = item.icon {
        icon.paint(painter, Rect::from_center_size(pos2(x + is / 2.0, rect.center().y), vec2(is, is)), color);
        x += is + 7.0;
    }
    if !item.label.is_empty() {
        painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, color);
    }
}

/// An iOS-style segmented control: a muted track with a raised thumb that springs to the
/// selected option.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut view = 0usize;
/// use glowkit_components::SegmentedControl;
/// ui.add(SegmentedControl::new(&mut view, ["Day", "Week", "Month", "Year"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SegmentedControl<'a> {
    selected: &'a mut usize,
    items: Vec<ToggleItem>,
    size: ControlSize,
    equal: bool,
    width: Option<f32>,
}

impl<'a> SegmentedControl<'a> {
    /// A segmented control bound to the selected index.
    pub fn new<I: Into<ToggleItem>>(selected: &'a mut usize, items: impl IntoIterator<Item = I>) -> Self {
        Self {
            selected,
            items: items.into_iter().map(Into::into).collect(),
            size: ControlSize::Md,
            equal: true,
            width: None,
        }
    }
    /// Size variant.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Give every segment the same width (default) or size each to its label.
    pub fn equal_widths(mut self, e: bool) -> Self {
        self.equal = e;
        self
    }
    /// Total width (segments share it equally).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
}

impl Widget for SegmentedControl<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.items.len().max(1);
        let pad = 3.0;
        let mut widths: Vec<f32> = self.items.iter().map(|it| item_width(ui, it, self.size) + 6.0).collect();
        if self.equal || self.width.is_some() {
            let m = match self.width {
                Some(w) => (w - pad * 2.0) / n as f32,
                None => widths.iter().cloned().fold(0.0, f32::max),
            };
            widths.iter_mut().for_each(|w| *w = m);
        }
        let h = self.size.height();
        let total = widths.iter().sum::<f32>() + pad * 2.0;
        let (rect, mut resp) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
        let r = control_radius(&t);
        let inner_r = CornerRadius::same(r.nw.saturating_sub(2));
        ui.painter().rect_filled(rect, r, t.muted);
        let mut xs = Vec::with_capacity(n);
        let mut x = rect.min.x + pad;
        for w in &widths {
            xs.push(x);
            x += w;
        }
        let sel = (*self.selected).min(self.items.len().saturating_sub(1));
        let tx =
            glowkit_core::anim::spring(ui, resp.id.with("x"), xs.get(sel).copied().unwrap_or(rect.min.x), 380.0, 30.0);
        let tw =
            glowkit_core::anim::spring(ui, resp.id.with("w"), widths.get(sel).copied().unwrap_or(0.0), 380.0, 30.0);
        let thumb = Rect::from_min_size(pos2(tx, rect.min.y + pad), vec2(tw, h - pad * 2.0));
        let thumb_fill = if t.dark { mix(t.muted, t.foreground, 0.1) } else { t.background };
        glowkit_core::paint::soft_shadow(
            ui.painter(),
            thumb,
            inner_r.nw as f32,
            3.0,
            vec2(0.0, 1.0),
            Color32::from_black_alpha(if t.dark { 60 } else { 22 }),
        );
        ui.painter().rect(
            thumb,
            inner_r,
            thumb_fill,
            Stroke::new(1.0, if t.dark { with_alpha(t.foreground, 0.06) } else { with_alpha(t.border, 0.6) }),
            StrokeKind::Inside,
        );
        for (i, item) in self.items.iter().enumerate() {
            let ir = Rect::from_min_size(pos2(xs[i], rect.min.y + pad), vec2(widths[i], h - pad * 2.0));
            let iresp = ui.interact(ir, resp.id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if iresp.clicked() && *self.selected != i {
                *self.selected = i;
                resp.mark_changed();
            }
            // text is fully bright where the thumb covers it
            let cover = ((thumb.max.x.min(ir.max.x) - thumb.min.x.max(ir.min.x)) / ir.width()).clamp(0.0, 1.0);
            let hover = kit::anim(ui, resp.id.with(("h", i)), iresp.hovered(), 0.12);
            let col = t.muted_foreground.lerp_to_gamma(t.foreground, cover.max(hover * 0.6));
            paint_item(ui, ir, item, self.size, col);
        }
        resp
    }
}
