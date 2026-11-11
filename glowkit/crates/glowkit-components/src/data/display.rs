use crate::inputs::kit::{self, UiIcon};
use crate::inputs::{Button, ButtonVariant};
use egui::{pos2, vec2, Align, Color32, CornerRadius, FontId, InnerResponse, Layout, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

type UiFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A content card with optional header (title, description, action), body and footer, in the
/// shadcn layout. Hoverable cards lift their shadow and brighten their border.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::Card;
/// Card::new().title("Create project").description("Deploy your new project in one click.").show(ui, |ui| {
///     ui.label("Body");
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct Card<'a> {
    title: Option<String>,
    description: Option<String>,
    width: Option<f32>,
    padding: f32,
    hoverable: bool,
    action: Option<UiFn<'a>>,
    footer: Option<UiFn<'a>>,
    fill: Option<Color32>,
    footer_border: bool,
}

impl Default for Card<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Card<'a> {
    /// An empty card.
    pub fn new() -> Self {
        Self { title: None, description: None, width: None, padding: 22.0, hoverable: false, action: None, footer: None, fill: None, footer_border: false }
    }
    /// Header title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    /// Muted header description.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Outer width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Inner padding (default 22).
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
    /// Lift the shadow and brighten the border on hover.
    pub fn hoverable(mut self, h: bool) -> Self {
        self.hoverable = h;
        self
    }
    /// Content on the right side of the header (e.g. a button or badge).
    pub fn action(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.action = Some(Box::new(f));
        self
    }
    /// Footer content (laid out left to right).
    pub fn footer(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.footer = Some(Box::new(f));
        self
    }
    /// Separate the footer with a hairline and muted background.
    pub fn footer_border(mut self, b: bool) -> Self {
        self.footer_border = b;
        self
    }
    /// Background color (default theme card).
    pub fn fill(mut self, c: Color32) -> Self {
        self.fill = Some(c);
        self
    }

    /// Show the card with `content` as its body.
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let bg = ui.painter().add(Shape::Noop);
        let pad = self.padding;
        let footer_bg = ui.painter().add(Shape::Noop);
        let mut footer_top = None;
        let inner = ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(w);
            ui.add_space(pad);
            let inner_w = w - pad * 2.0;
            let r = ui
                .horizontal(|ui| {
                    ui.add_space(pad);
                    ui.allocate_ui_with_layout(vec2(inner_w, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(inner_w);
                        ui.spacing_mut().item_spacing.y = 6.0;
                        if self.title.is_some() || self.description.is_some() || self.action.is_some() {
                            ui.horizontal_top(|ui| {
                                let action_w = if self.action.is_some() { 120.0 } else { 0.0 };
                                ui.allocate_ui_with_layout(vec2(inner_w - action_w, 0.0), Layout::top_down(Align::Min), |ui| {
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    if let Some(tt) = &self.title {
                                        ui.label(RichText::new(tt).size(16.0).strong().color(t.card_foreground));
                                    }
                                    if let Some(d) = &self.description {
                                        ui.add(egui::Label::new(RichText::new(d).size(13.5).color(t.muted_foreground)).wrap());
                                    }
                                });
                                if let Some(a) = self.action {
                                    ui.with_layout(Layout::right_to_left(Align::Min), a);
                                }
                            });
                            ui.add_space(14.0);
                        }
                        content(ui)
                    })
                    .inner
                })
                .inner;
            if let Some(f) = self.footer {
                ui.add_space(if self.footer_border { pad } else { 18.0 });
                footer_top = Some(ui.cursor().min.y);
                if self.footer_border {
                    ui.add_space(14.0);
                }
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.allocate_ui_with_layout(vec2(inner_w, 0.0), Layout::left_to_right(Align::Center), f);
                });
                ui.add_space(if self.footer_border { 14.0 } else { pad });
            } else {
                ui.add_space(pad);
            }
            r
        });
        let rect = Rect::from_min_size(inner.response.rect.min, vec2(w, inner.response.rect.height()));
        let resp = ui.interact(rect, ui.next_auto_id().with("card"), Sense::hover());
        let hov = if self.hoverable { kit::anim(ui, resp.id.with("h"), ui.rect_contains_pointer(rect), 0.2) } else { 0.0 };
        let cr = t.corner_scaled(1.2);
        let mut shapes = Vec::new();
        let shadow_a = if t.dark { 40.0 + 60.0 * hov } else { 10.0 + 22.0 * hov };
        shapes.push(egui::epaint::Shadow { offset: [0, (2.0 + 6.0 * hov) as i8], blur: (6.0 + 18.0 * hov) as u8, spread: 0, color: Color32::from_black_alpha(shadow_a as u8) }.as_shape(rect, cr).into());
        let border = t.border.lerp_to_gamma(t.muted_foreground, 0.35 * hov);
        shapes.push(egui::epaint::RectShape::new(rect, cr, self.fill.unwrap_or(t.card), Stroke::new(1.0, border), StrokeKind::Inside).into());
        ui.painter().set(bg, Shape::Vec(shapes));
        if let (Some(y), true) = (footer_top, self.footer_border) {
            let fr = Rect::from_min_max(pos2(rect.min.x + 1.0, y), pos2(rect.max.x - 1.0, rect.max.y - 1.0));
            let c = CornerRadius { nw: 0, ne: 0, sw: cr.sw.saturating_sub(1), se: cr.se.saturating_sub(1) };
            ui.painter().set(
                footer_bg,
                Shape::Vec(vec![
                    egui::epaint::RectShape::filled(fr, c, with_alpha(t.muted, if t.dark { 0.5 } else { 0.6 })).into(),
                    Shape::line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.0, border)),
                ]),
            );
        }
        InnerResponse::new(inner.inner, resp)
    }
}

/// Visual style of an [`Alert`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlertVariant {
    /// Neutral card.
    #[default]
    Default,
    /// Blue info.
    Info,
    /// Green success.
    Success,
    /// Amber warning.
    Warning,
    /// Red error.
    Destructive,
}

/// A callout box with an icon, title and description.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{Alert, AlertVariant};
/// ui.add(Alert::new("Heads up!").description("You can add components with the CLI.").variant(AlertVariant::Info));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Alert {
    title: String,
    description: Option<String>,
    variant: AlertVariant,
    icon: Option<UiIcon>,
    width: Option<f32>,
    accent_bar: bool,
}

impl Alert {
    /// An alert with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), description: None, variant: AlertVariant::Default, icon: None, width: None, accent_bar: false }
    }
    /// Body text under the title.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Color variant (also picks a default icon).
    pub fn variant(mut self, v: AlertVariant) -> Self {
        self.variant = v;
        self
    }
    /// Override the icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Width (default: available, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Draw a colored bar on the left edge.
    pub fn accent_bar(mut self, a: bool) -> Self {
        self.accent_bar = a;
        self
    }
}

impl Widget for Alert {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let (tint, icon) = match self.variant {
            AlertVariant::Default => (None, UiIcon::Terminal),
            AlertVariant::Info => (Some(t.info), UiIcon::Info),
            AlertVariant::Success => (Some(t.success), UiIcon::CheckCircle),
            AlertVariant::Warning => (Some(t.warning), UiIcon::AlertTriangle),
            AlertVariant::Destructive => (Some(t.destructive), UiIcon::AlertCircle),
        };
        let icon = self.icon.unwrap_or(icon);
        let pad = 14.0;
        let text_x = pad + 28.0;
        let tw = w - text_x - pad;
        let title_col = match tint {
            Some(c) => {
                if t.dark {
                    mix(c, Color32::WHITE, 0.3)
                } else {
                    mix(c, Color32::BLACK, 0.25)
                }
            }
            None => t.foreground,
        };
        let tg = ui.painter().layout(self.title.clone(), FontId::proportional(14.0), title_col, tw);
        let desc_col = match tint {
            Some(c) => mix(t.muted_foreground, title_col, 0.35).lerp_to_gamma(c, 0.0),
            None => t.muted_foreground,
        };
        let dg = self.description.as_ref().map(|d| ui.painter().layout(d.clone(), FontId::proportional(13.5), desc_col, tw));
        let h = pad * 2.0 + tg.size().y + dg.as_ref().map(|g| g.size().y + 4.0).unwrap_or(0.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter();
        let cr = t.corner();
        let (fill, border) = match tint {
            Some(c) => (mix(t.card, c, if t.dark { 0.08 } else { 0.05 }), with_alpha(c, if t.dark { 0.35 } else { 0.3 })),
            None => (t.card, t.border),
        };
        p.rect(rect, cr, fill, Stroke::new(1.0, border), StrokeKind::Inside);
        if self.accent_bar {
            let bar = Rect::from_min_size(rect.min, vec2(3.0, h));
            p.rect_filled(bar, CornerRadius { nw: cr.nw, sw: cr.sw, ne: 0, se: 0 }, tint.unwrap_or(t.foreground));
        }
        icon.paint(p, Rect::from_min_size(rect.min + vec2(pad, pad + 0.5), vec2(17.0, 17.0)), tint.map(|_| title_col).unwrap_or(t.foreground));
        let tgh = tg.size().y;
        p.galley(rect.min + vec2(text_x, pad), tg, title_col);
        if let Some(g) = dg {
            p.galley(rect.min + vec2(text_x, pad + tgh + 4.0), g, desc_col);
        }
        resp
    }
}

/// An empty-state block: a fanned stack of icon tiles, a title, a description and an optional
/// call-to-action. The returned response's `.clicked()` reports the primary action.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{EmptyState, UiIcon};
/// if ui.add(EmptyState::new(UiIcon::Folder, "No projects yet").description("Create your first project to get started.").action("Create project")).clicked() {}
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct EmptyState {
    icons: [UiIcon; 3],
    title: String,
    description: Option<String>,
    action: Option<String>,
    secondary: Option<String>,
    width: Option<f32>,
    dashed: bool,
}

impl EmptyState {
    /// An empty state with a main icon and title.
    pub fn new(icon: UiIcon, title: impl Into<String>) -> Self {
        Self { icons: [UiIcon::File, icon, UiIcon::Image], title: title.into(), description: None, action: None, secondary: None, width: None, dashed: true }
    }
    /// The two side icons of the fanned stack.
    pub fn side_icons(mut self, left: UiIcon, right: UiIcon) -> Self {
        self.icons[0] = left;
        self.icons[2] = right;
        self
    }
    /// Muted explanation.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    /// Primary button label.
    pub fn action(mut self, a: impl Into<String>) -> Self {
        self.action = Some(a.into());
        self
    }
    /// Secondary (outline) button label.
    pub fn secondary_action(mut self, a: impl Into<String>) -> Self {
        self.secondary = Some(a.into());
        self
    }
    /// Width (default: available, at most 520).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Dashed border around the block (default on).
    pub fn dashed(mut self, d: bool) -> Self {
        self.dashed = d;
        self
    }
}

fn rotated_rect(c: Pos2, size: f32, angle: f32, radius: f32) -> Vec<Pos2> {
    let pts = glowkit_core::paint::rounded_rect_points(Rect::from_center_size(Pos2::ZERO, vec2(size, size)), radius, 6);
    let (s, co) = angle.sin_cos();
    pts.into_iter().map(|p| c + vec2(p.x * co - p.y * s, p.x * s + p.y * co)).collect()
}

impl Widget for EmptyState {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(520.0));
        let start = ui.cursor().min;
        let mut clicked = None;
        let inner = ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Center), |ui| {
            ui.set_width(w);
            ui.add_space(30.0);
            let (ir, _) = ui.allocate_exact_size(vec2(140.0, 64.0), Sense::hover());
            let time = glowkit_core::time(ui);
            let hov = kit::anim(ui, ui.next_auto_id().with("es"), ui.rect_contains_pointer(Rect::from_min_size(start, vec2(w, 260.0))), 0.3);
            let p = ui.painter();
            for (i, (dx, ang)) in [(-30.0f32, -0.21f32), (30.0, 0.21)].iter().enumerate() {
                let spread = 1.0 + 0.25 * hov;
                let c = ir.center() + vec2(dx * spread, 6.0 - 2.0 * hov);
                let pts = rotated_rect(c, 46.0, ang * spread, 10.0);
                p.add(Shape::convex_polygon(pts.clone(), t.card, Stroke::new(1.0, t.border)));
                let icon_r = Rect::from_center_size(c, vec2(20.0, 20.0));
                self.icons[if i == 0 { 0 } else { 2 }].paint(p, icon_r, t.muted_foreground);
            }
            let bob = (time * 1.6).sin() * 1.5 * hov;
            let main = Rect::from_center_size(ir.center() - vec2(0.0, 4.0 + bob), vec2(52.0, 52.0));
            glowkit_core::paint::soft_shadow(p, main, 12.0, 10.0, vec2(0.0, 3.0), Color32::from_black_alpha(if t.dark { 70 } else { 22 }));
            p.rect(main, CornerRadius::same(12), t.card, Stroke::new(1.0, t.border), StrokeKind::Inside);
            self.icons[1].paint(p, Rect::from_center_size(main.center(), vec2(24.0, 24.0)), t.foreground);
            if hov > 0.01 && hov < 0.99 {
                ui.ctx().request_repaint();
            } else if hov >= 0.99 {
                glowkit_core::animate(ui);
            }
            ui.add_space(14.0);
            ui.label(RichText::new(&self.title).size(16.5).strong().color(t.foreground));
            if let Some(d) = &self.description {
                ui.add_space(-2.0);
                ui.allocate_ui_with_layout(vec2(w.min(360.0), 0.0), Layout::top_down(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(d).size(13.5).color(t.muted_foreground)).wrap().halign(Align::Center));
                });
            }
            if self.action.is_some() || self.secondary.is_some() {
                ui.add_space(10.0);
                let bw = self.action.as_ref().map(|a| kit::text_width(ui, a, 14.0) + 32.0 + 24.0).unwrap_or(0.0) + self.secondary.as_ref().map(|a| kit::text_width(ui, a, 14.0) + 32.0 + 8.0).unwrap_or(0.0);
                ui.allocate_ui_with_layout(vec2(bw, 36.0), Layout::left_to_right(Align::Center), |ui| {
                    if let Some(s) = &self.secondary {
                        ui.add(Button::new(s.clone()).variant(ButtonVariant::Outline));
                    }
                    if let Some(a) = &self.action {
                        let r = ui.add(Button::new(a.clone()).icon(UiIcon::Plus));
                        if r.clicked() {
                            clicked = Some(r);
                        }
                    }
                });
            }
            ui.add_space(30.0);
        });
        let rect = Rect::from_min_size(start, vec2(w, inner.response.rect.height()));
        if self.dashed {
            let pts = glowkit_core::paint::rounded_rect_points(rect.shrink(0.75), t.radius + 2.0, 8);
            let painter = ui.painter().clone();
            crate::inputs::dashed_closed(&painter, &pts, 5.0, 4.0, 0.0, Stroke::new(1.0, t.input.lerp_to_gamma(t.muted_foreground, 0.3)));
        }
        clicked.unwrap_or(inner.response)
    }
}

/// A KPI card: label, big value that counts up on appear, a colored delta pill and a sparkline
/// that draws itself in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{StatCard, UiIcon};
/// ui.add(StatCard::new("Total revenue", 45231.89).prefix("$").decimals(2).delta(20.1).icon(UiIcon::Dollar)
///     .sparkline(&[3.0, 4.0, 3.5, 5.0, 4.8, 6.2, 7.0]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct StatCard {
    label: String,
    value: f64,
    prefix: String,
    suffix: String,
    decimals: usize,
    delta: Option<f32>,
    delta_note: String,
    icon: Option<UiIcon>,
    spark: Vec<f32>,
    width: f32,
    color: Option<Color32>,
    invert: bool,
}

impl StatCard {
    /// A stat card for a numeric value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self { label: label.into(), value, prefix: String::new(), suffix: String::new(), decimals: 0, delta: None, delta_note: "from last month".into(), icon: None, spark: Vec::new(), width: 260.0, color: None, invert: false }
    }
    /// Text before the number (e.g. `"$"`).
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = p.into();
        self
    }
    /// Text after the number (e.g. `"%"`).
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Digits after the decimal point.
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Percent change; positive is green with an up arrow, negative red with a down arrow.
    pub fn delta(mut self, pct: f32) -> Self {
        self.delta = Some(pct);
        self
    }
    /// Text after the delta (default "from last month").
    pub fn delta_note(mut self, n: impl Into<String>) -> Self {
        self.delta_note = n.into();
        self
    }
    /// Treat decreases as good (e.g. churn, latency).
    pub fn invert_delta(mut self, i: bool) -> Self {
        self.invert = i;
        self
    }
    /// Icon in the top-right corner.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Values for the sparkline at the bottom.
    pub fn sparkline(mut self, values: &[f32]) -> Self {
        self.spark = values.to_vec();
        self
    }
    /// Card width (default 260).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Sparkline color (default: delta color, or the chart palette).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// Format with thousands separators.
pub(crate) fn group_thousands(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (s.clone(), None),
    };
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&f);
    }
    if v < 0.0 {
        out.insert(0, '-');
    }
    out
}

impl Widget for StatCard {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = if self.spark.len() >= 2 { 168.0 } else { 120.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let hov = kit::anim(ui, id.with("h"), resp.hovered(), 0.2);
        let p = ui.painter();
        let cr = t.corner_scaled(1.2);
        glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 6.0 + 10.0 * hov, vec2(0.0, 2.0 + 3.0 * hov), Color32::from_black_alpha(if t.dark { (40.0 + 40.0 * hov) as u8 } else { (10.0 + 14.0 * hov) as u8 }));
        p.rect(rect, cr, t.card, Stroke::new(1.0, t.border.lerp_to_gamma(t.muted_foreground, 0.3 * hov)), StrokeKind::Inside);
        let pad = 20.0;
        p.text(rect.min + vec2(pad, pad + 8.0), egui::Align2::LEFT_CENTER, &self.label, FontId::proportional(13.5), t.muted_foreground);
        if let Some(i) = self.icon {
            i.paint(p, Rect::from_center_size(pos2(rect.max.x - pad - 8.0, rect.min.y + pad + 8.0), vec2(16.0, 16.0)), t.muted_foreground);
        }
        let el = glowkit_core::Clock::elapsed(ui, id.with("clock"));
        let k = glowkit_core::ease(glowkit_core::Easing::OutExpo, el / 1.4);
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let shown = self.value * k as f64;
        let txt = format!("{}{}{}", self.prefix, group_thousands(shown, self.decimals), self.suffix);
        p.text(rect.min + vec2(pad, pad + 40.0), egui::Align2::LEFT_CENTER, txt, FontId::proportional(28.0), t.foreground);
        let good_col = |d: f32| if (d >= 0.0) != self.invert { t.success } else { t.destructive };
        if let Some(d) = self.delta {
            let col = good_col(d);
            let s = format!("{}{:.1}%", if d >= 0.0 { "+" } else { "" }, d);
            let g = p.layout_no_wrap(s, FontId::proportional(12.0), col);
            let pill = Rect::from_min_size(rect.min + vec2(pad, pad + 64.0), vec2(g.size().x + 26.0, 22.0));
            p.rect_filled(pill, CornerRadius::same(11), with_alpha(col, if t.dark { 0.15 } else { 0.1 }));
            (if d >= 0.0 { UiIcon::ArrowUp } else { UiIcon::ArrowDown }).paint(p, Rect::from_center_size(pos2(pill.min.x + 11.0, pill.center().y), vec2(12.0, 12.0)), col);
            p.galley(pos2(pill.min.x + 19.0, pill.center().y - g.size().y / 2.0), g, col);
            p.text(pos2(pill.max.x + 8.0, pill.center().y), egui::Align2::LEFT_CENTER, &self.delta_note, FontId::proportional(12.5), t.muted_foreground);
        }
        if self.spark.len() >= 2 {
            let col = self.color.unwrap_or_else(|| self.delta.map(good_col).unwrap_or(t.chart[0]));
            let area = Rect::from_min_max(pos2(rect.min.x + 1.0, rect.max.y - 54.0), pos2(rect.max.x - 1.0, rect.max.y - 8.0));
            let (lo, hi) = self.spark.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
            let n = self.spark.len();
            let pts: Vec<Pos2> = self
                .spark
                .iter()
                .enumerate()
                .map(|(i, &v)| pos2(area.min.x + area.width() * i as f32 / (n - 1) as f32, area.max.y - area.height() * glowkit_core::anim::unlerp(lo, hi, v)))
                .collect();
            let smooth = glowkit_core::paint::monotone(&pts, 8);
            let draw = glowkit_core::ease(glowkit_core::Easing::InOutCubic, el / 1.2);
            let clip = Rect::from_min_max(area.min - vec2(2.0, 6.0), pos2(area.min.x + area.width() * draw + 2.0, rect.max.y));
            let cp = p.with_clip_rect(clip.intersect(p.clip_rect()));
            glowkit_core::paint::area_fill(&cp, &smooth, rect.max.y - 2.0, with_alpha(col, if t.dark { 0.28 } else { 0.18 }), with_alpha(col, 0.0));
            glowkit_core::paint::gradient_polyline(&cp, &smooth, false, 2.0, |_| col);
            if draw >= 1.0 {
                let last = *pts.last().unwrap();
                p.circle_filled(last, 3.5, col);
                p.circle_stroke(last, 6.0, Stroke::new(1.0, with_alpha(col, 0.35)));
            }
        }
        resp
    }
}

/// A list of label / value rows separated by hairlines (a "description list").
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::DescriptionList;
/// ui.add(DescriptionList::new(&[("Full name", "Ada Lovelace"), ("Email", "ada@lovelace.dev")]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DescriptionList {
    rows: Vec<(String, String)>,
    key_width: f32,
    width: Option<f32>,
    stacked: bool,
}

/// Alias: key/value list.
pub type KeyValue = DescriptionList;

impl DescriptionList {
    /// Rows of `(label, value)`.
    pub fn new(rows: &[(&str, &str)]) -> Self {
        Self { rows: rows.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), key_width: 150.0, width: None, stacked: false }
    }
    /// Width of the label column (default 150).
    pub fn key_width(mut self, w: f32) -> Self {
        self.key_width = w;
        self
    }
    /// Total width (default: available, at most 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Put the label above the value instead of beside it.
    pub fn stacked(mut self, s: bool) -> Self {
        self.stacked = s;
        self
    }
}

impl Widget for DescriptionList {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(560.0));
        let vw = if self.stacked { w } else { w - self.key_width };
        let galleys: Vec<_> = self.rows.iter().map(|(k, v)| (ui.painter().layout(k.clone(), FontId::proportional(13.5), t.muted_foreground, self.key_width - 12.0), ui.painter().layout(v.clone(), FontId::proportional(13.5), t.foreground, vw))).collect();
        let row_h: Vec<f32> = galleys.iter().map(|(k, v)| if self.stacked { k.size().y + v.size().y + 4.0 + 22.0 } else { k.size().y.max(v.size().y) + 24.0 }).collect();
        let total: f32 = row_h.iter().sum();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, total), Sense::hover());
        let p = ui.painter();
        let mut y = rect.min.y;
        for (i, (k, v)) in galleys.into_iter().enumerate() {
            let h = row_h[i];
            if i > 0 {
                let yy = kit::snap(ui, y) + 0.5;
                p.line_segment([pos2(rect.min.x, yy), pos2(rect.max.x, yy)], Stroke::new(1.0, t.border));
            }
            if self.stacked {
                let kh = k.size().y;
                p.galley(pos2(rect.min.x, y + 11.0), k, t.muted_foreground);
                p.galley(pos2(rect.min.x, y + 15.0 + kh), v, t.foreground);
            } else {
                p.galley(pos2(rect.min.x, y + 12.0), k, t.muted_foreground);
                p.galley(pos2(rect.min.x + self.key_width, y + 12.0), v, t.foreground);
            }
            y += h;
        }
        resp
    }
}
