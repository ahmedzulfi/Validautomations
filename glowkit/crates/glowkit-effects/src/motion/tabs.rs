use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::{paint, Theme};

/// How [`AnimatedTabs`] highlights the selected tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TabHighlight {
    /// A raised pill inside a muted track.
    #[default]
    Pill,
    /// A line under the label.
    Underline,
    /// A solid accent-colored pill.
    Solid,
}

/// A row of tabs whose highlight springs from one tab to the next, stretching as it moves;
/// optionally a softer hover highlight follows the pointer.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut selected = 0;
/// ui.add(glowkit_effects::AnimatedTabs::new(&mut selected, &["Overview", "Usage", "API"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AnimatedTabs<'a> {
    selected: &'a mut usize,
    labels: Vec<String>,
    style: TabHighlight,
    hover: bool,
    color: Option<Color32>,
    font_size: f32,
    stiffness: f32,
    damping: f32,
}

impl<'a> AnimatedTabs<'a> {
    /// Tabs over `labels` editing the `selected` index.
    pub fn new<S: AsRef<str>>(selected: &'a mut usize, labels: &[S]) -> Self {
        Self {
            selected,
            labels: labels.iter().map(|s| s.as_ref().to_owned()).collect(),
            style: TabHighlight::Pill,
            hover: true,
            color: None,
            font_size: 14.0,
            stiffness: 380.0,
            damping: 30.0,
        }
    }
    /// Highlight style.
    pub fn style(mut self, s: TabHighlight) -> Self {
        self.style = s;
        self
    }
    /// Show a soft hover highlight that glides between tabs (default on).
    pub fn hover_highlight(mut self, on: bool) -> Self {
        self.hover = on;
        self
    }
    /// Highlight color (pill fill or underline). Defaults depend on the style.
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Label font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.max(6.0);
        self
    }
    /// Spring stiffness and damping of the highlight.
    pub fn spring(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness.max(1.0);
        self.damping = damping.max(0.1);
        self
    }
}

impl Widget for AnimatedTabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.labels.len();
        let font = FontId::proportional(self.font_size);
        let pad_x = 16.0;
        let h = self.font_size + 20.0;
        let inset = if self.style == TabHighlight::Underline { 0.0 } else { 4.0 };
        let widths: Vec<f32> = self.labels.iter().map(|l| ui.painter().layout_no_wrap(l.clone(), font.clone(), Color32::WHITE).size().x + pad_x * 2.0).collect();
        let total = widths.iter().sum::<f32>() + inset * 2.0;
        let (rect, mut response) = ui.allocate_exact_size(vec2(total, h + inset * 2.0), Sense::click());
        let id = response.id;
        if n == 0 {
            return response;
        }
        let mut tabs = Vec::with_capacity(n);
        let mut x = rect.min.x + inset;
        for w in &widths {
            tabs.push(Rect::from_min_size(pos2(x, rect.min.y + inset), vec2(*w, h)));
            x += w;
        }
        let hovered = response.hover_pos().and_then(|p| tabs.iter().position(|r| r.contains(p)));
        if response.clicked() {
            if let Some(i) = hovered {
                if *self.selected != i {
                    *self.selected = i;
                    response.mark_changed();
                }
            }
        }
        if hovered.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let sel = (*self.selected).min(n - 1);
        // leading and trailing edges use different springs so the pill stretches in motion
        let target = tabs[sel];
        let (fast, slow) = ((self.stiffness, self.damping), (self.stiffness * 0.55, self.damping * 0.75));
        let moving_right = glowkit_core::with_state(ui, id.with("dir"), || (target.center().x, true), |(cx, right)| {
            if (target.center().x - *cx).abs() > 0.5 {
                *right = target.center().x > *cx;
                *cx = target.center().x;
            }
            *right
        });
        let (ls, rs) = if moving_right { (slow, fast) } else { (fast, slow) };
        let l = glowkit_core::anim::spring(ui, id.with("l"), target.min.x, ls.0, ls.1);
        let r = glowkit_core::anim::spring(ui, id.with("r"), target.max.x, rs.0, rs.1);
        let pill = Rect::from_min_max(pos2(l, target.min.y), pos2(r.max(l + 4.0), target.max.y));
        let hv_alpha = glowkit_core::anim::follow(ui, id.with("hva"), if self.hover && hovered.is_some() { 1.0 } else { 0.0 }, 12.0);
        let hv_t = hovered.map(|i| tabs[i]).unwrap_or(pill);
        let hl = glowkit_core::anim::spring(ui, id.with("hl"), hv_t.min.x, 420.0, 34.0);
        let hr = glowkit_core::anim::spring(ui, id.with("hr"), hv_t.max.x, 420.0, 34.0);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let radius = CornerRadius::same(((h + inset * 2.0) / 2.0).round() as u8);
        let inner_r = CornerRadius::same((h / 2.0).round() as u8);
        match self.style {
            TabHighlight::Pill | TabHighlight::Solid => {
                painter.rect(rect, radius, t.muted, Stroke::new(1.0, t.border), StrokeKind::Inside);
            }
            TabHighlight::Underline => {
                painter.line_segment([pos2(rect.min.x, rect.max.y - 0.5), pos2(rect.max.x, rect.max.y - 0.5)], Stroke::new(1.0, t.border));
            }
        }
        if hv_alpha > 0.01 {
            let hr_rect = Rect::from_min_max(pos2(hl, pill.min.y), pos2(hr, pill.max.y));
            let hcol = with_alpha(t.muted_foreground, 0.12 * hv_alpha);
            let hr_rect = if self.style == TabHighlight::Underline { hr_rect.shrink2(vec2(4.0, 4.0)) } else { hr_rect };
            painter.rect_filled(hr_rect, inner_r, hcol);
        }
        let pill_fill = match self.style {
            TabHighlight::Pill => self.color.unwrap_or(if t.dark { shade(t.secondary, 0.06) } else { t.background }),
            TabHighlight::Solid => self.color.unwrap_or(t.accent),
            TabHighlight::Underline => self.color.unwrap_or(t.accent),
        };
        match self.style {
            TabHighlight::Pill => {
                paint::soft_shadow(painter, pill, h / 2.0, 6.0, vec2(0.0, 1.0), with_alpha(Color32::BLACK, if t.dark { 0.4 } else { 0.1 }));
                painter.rect(pill, inner_r, pill_fill, Stroke::new(1.0, with_alpha(t.border, 0.8)), StrokeKind::Inside);
            }
            TabHighlight::Solid => {
                paint::soft_shadow(painter, pill, h / 2.0, 12.0, vec2(0.0, 3.0), with_alpha(pill_fill, 0.35));
                painter.rect_filled(pill, inner_r, pill_fill);
            }
            TabHighlight::Underline => {
                let u = Rect::from_min_max(pos2(pill.min.x + 8.0, rect.max.y - 2.5), pos2(pill.max.x - 8.0, rect.max.y + 0.5));
                painter.rect_filled(u, CornerRadius::same(2), pill_fill);
            }
        }
        let on_col = match self.style {
            TabHighlight::Solid => glowkit_core::color::contrast_text(pill_fill),
            _ => t.foreground,
        };
        for (i, tab) in tabs.iter().enumerate() {
            // label color follows how much of the pill covers this tab
            let overlap = (pill.max.x.min(tab.max.x) - pill.min.x.max(tab.min.x)).max(0.0) / tab.width();
            let base = if hovered == Some(i) { mix(t.muted_foreground, t.foreground, 0.6) } else { t.muted_foreground };
            let col = mix(base, on_col, overlap.clamp(0.0, 1.0));
            painter.text(tab.center(), Align2::CENTER_CENTER, &self.labels[i], font.clone(), col);
        }
        response
    }
}
