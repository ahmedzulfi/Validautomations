use super::field::{field_description, field_error, FieldLabel};
use super::kit::{self, control_radius, ControlSize, UiIcon};
use egui::emath::Numeric;
use egui::{
    pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Label / description / error decoration shared by all fields.
#[derive(Clone, Default)]
struct Decor {
    label: Option<String>,
    required: bool,
    description: Option<String>,
    error: Option<String>,
}

impl Decor {
    fn top(&self, ui: &mut Ui) {
        if let Some(l) = &self.label {
            ui.add(FieldLabel::new(l.clone()).required(self.required));
        }
    }
    fn bottom(&self, ui: &mut Ui) {
        if let Some(e) = &self.error {
            field_error(ui, e);
        } else if let Some(d) = &self.description {
            field_description(ui, d);
        }
    }
}

fn field_width(ui: &Ui, w: Option<f32>) -> f32 {
    match w {
        Some(w) if w.is_finite() => w,
        Some(_) => ui.available_width(),
        None => ui.available_width().min(320.0),
    }
}

/// Small square hit area with an icon; returns the click response.
fn icon_button(ui: &mut Ui, id: Id, center: egui::Pos2, icon: UiIcon, size: f32, t: &Theme) -> Response {
    let r = Rect::from_center_size(center, vec2(size + 8.0, size + 8.0));
    let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let h = kit::anim(ui, id.with("h"), resp.hovered(), 0.1);
    if h > 0.0 {
        ui.painter().rect_filled(r, CornerRadius::same(5), with_alpha(t.muted_foreground, 0.12 * h));
    }
    icon.paint(
        ui.painter(),
        Rect::from_center_size(center, vec2(size, size)),
        t.muted_foreground.lerp_to_gamma(t.foreground, h),
    );
    resp
}

/// A text input with the full shadcn feature set: label, placeholder, leading icon or prefix
/// text, clear button, password reveal, error state and an animated focus ring.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut email = String::new();
/// use glowkit_components::{Input, UiIcon};
/// ui.add(Input::new(&mut email).label("Email").placeholder("you@example.com").icon(UiIcon::Mail).clearable(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Input<'a> {
    text: &'a mut String,
    placeholder: String,
    decor: Decor,
    icon: Option<UiIcon>,
    prefix: Option<String>,
    suffix: Option<String>,
    shortcut: Option<String>,
    clearable: bool,
    password: bool,
    strength: bool,
    size: ControlSize,
    width: Option<f32>,
    disabled: bool,
    id_salt: Option<Id>,
    char_limit: Option<usize>,
    pill: bool,
}

impl<'a> Input<'a> {
    /// An input editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: String::new(),
            decor: Decor::default(),
            icon: None,
            prefix: None,
            suffix: None,
            shortcut: None,
            clearable: false,
            password: false,
            strength: false,
            size: ControlSize::Md,
            width: None,
            disabled: false,
            id_salt: None,
            char_limit: None,
            pill: false,
        }
    }
    /// Placeholder shown while empty.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Label above the field.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.decor.label = Some(l.into());
        self
    }
    /// Show a red asterisk after the label.
    pub fn required(mut self, r: bool) -> Self {
        self.decor.required = r;
        self
    }
    /// Helper text below the field.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.decor.description = Some(d.into());
        self
    }
    /// Error message: turns the border red and replaces the description.
    pub fn error(mut self, e: impl Into<String>) -> Self {
        self.decor.error = Some(e.into());
        self
    }
    /// Error message only when `Some`.
    pub fn error_opt(mut self, e: Option<String>) -> Self {
        self.decor.error = e;
        self
    }
    /// Leading icon inside the field.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Muted inline prefix text (e.g. `"https://"` or `"$"`).
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = Some(p.into());
        self
    }
    /// Muted inline suffix text (e.g. `"USD"` or `".com"`).
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = Some(s.into());
        self
    }
    /// Keyboard shortcut hint drawn as a small key cap on the right (e.g. `"⌘K"`).
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
    /// Show an × button that clears the text.
    pub fn clearable(mut self, c: bool) -> Self {
        self.clearable = c;
        self
    }
    /// Mask the text and show an eye button to reveal it.
    pub fn password(mut self, p: bool) -> Self {
        self.password = p;
        self
    }
    /// Show a 4-segment password strength meter under the field.
    pub fn strength_meter(mut self, s: bool) -> Self {
        self.strength = s;
        self
    }
    /// Size variant.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Width in points (default `min(available, 320)`); `f32::INFINITY` fills the row.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Read-only and dimmed.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Stable id salt (needed when the same input moves around).
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
    /// Maximum number of characters.
    pub fn char_limit(mut self, n: usize) -> Self {
        self.char_limit = Some(n);
        self
    }
    /// Fully rounded ends (search bars).
    pub fn pill(mut self, p: bool) -> Self {
        self.pill = p;
        self
    }
}

/// Constructor for an [`Input`] preconfigured as a password field (masked, with reveal button).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut pw = String::new();
/// ui.add(glowkit_components::PasswordInput::new(&mut pw).label("Password").strength_meter(true));
/// ```
pub struct PasswordInput;

impl PasswordInput {
    /// A masked [`Input`] with a lock icon and show/hide toggle.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(text: &mut String) -> Input<'_> {
        Input::new(text).password(true).icon(UiIcon::Lock).placeholder("Enter password")
    }
}

/// Password strength 0..=4 by a few simple rules.
fn strength(s: &str) -> usize {
    if s.is_empty() {
        return 0;
    }
    let mut n = 0;
    if s.len() >= 8 {
        n += 1;
    }
    if s.chars().any(|c| c.is_ascii_digit()) {
        n += 1;
    }
    if s.chars().any(|c| c.is_uppercase()) && s.chars().any(|c| c.is_lowercase()) {
        n += 1;
    }
    if s.chars().any(|c| !c.is_alphanumeric()) {
        n += 1;
    }
    n.max(1)
}

impl Widget for Input<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = field_width(ui, self.width);
        let decor = self.decor.clone();
        ui.allocate_ui_with_layout(vec2(width, self.size.height()), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            decor.top(ui);
            let r = self.field(ui, width);
            decor.bottom(ui);
            r
        })
        .inner
    }
}

impl Input<'_> {
    fn field(self, ui: &mut Ui, width: f32) -> Response {
        let t = Theme::of(ui);
        let h = self.size.height();
        let fs = self.size.font();
        let base_id = match self.id_salt {
            Some(s) => ui.make_persistent_id(s),
            None => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        };
        let te_id = base_id.with("te");
        let (rect, bg) = ui.allocate_exact_size(vec2(width, h), Sense::click());
        if bg.clicked() && !self.disabled {
            ui.memory_mut(|m| m.request_focus(te_id));
        }
        let _ = bg.on_hover_cursor(CursorIcon::Text);
        let focused = ui.memory(|m| m.has_focus(te_id));
        let hover = kit::anim(ui, base_id.with("h"), ui.rect_contains_pointer(rect) && !self.disabled, 0.12);
        let focus = kit::anim(ui, base_id.with("f"), focused, 0.16);
        let error = self.decor.error.is_some();
        let chrome_idx = ui.painter().add(Shape::Noop);

        let pad = if self.pill { 14.0 } else { 12.0 };
        let is = self.size.icon();
        let mut x = rect.min.x + pad;
        let cy = rect.center().y;
        let muted = t.muted_foreground;
        if let Some(icon) = self.icon {
            let col = muted.lerp_to_gamma(t.foreground, focus * 0.6);
            icon.paint(ui.painter(), Rect::from_center_size(pos2(x + is / 2.0, cy), vec2(is, is)), col);
            x += is + 8.0;
        }
        if let Some(p) = &self.prefix {
            let r = kit::text_at(ui.painter(), pos2(x, cy), egui::Align2::LEFT_CENTER, p, fs, muted);
            x = r.max.x + 2.0;
        }
        // right-side adornments, laid out right-to-left
        let mut xr = rect.max.x - 6.0;
        let reveal_id = base_id.with("reveal");
        let mut revealed: bool = glowkit_core::state(ui, reveal_id, || false);
        let text = self.text;
        let mut cleared = false;
        let mut any_right = false;
        if self.password {
            let c = pos2(xr - 12.0, cy);
            let icon = if revealed { UiIcon::EyeOff } else { UiIcon::Eye };
            if icon_button(ui, base_id.with("eye"), c, icon, is, &t).clicked() {
                revealed = !revealed;
                glowkit_core::set_state(ui, reveal_id, revealed);
            }
            xr -= 26.0;
            any_right = true;
        }
        if self.clearable && !text.is_empty() && !self.disabled {
            let c = pos2(xr - 12.0, cy);
            if icon_button(ui, base_id.with("clear"), c, UiIcon::X, is - 2.0, &t).clicked() {
                cleared = true;
                text.clear();
                ui.memory_mut(|m| m.request_focus(te_id));
            }
            xr -= 26.0;
            any_right = true;
        }
        if let Some(s) = &self.shortcut {
            if text.is_empty() {
                let w = kit::text_width(ui, s, 11.5) + 10.0;
                let kr = Rect::from_center_size(pos2(xr - w / 2.0 - 2.0, cy), vec2(w, 20.0));
                ui.painter().rect(kr, CornerRadius::same(4), t.muted, Stroke::new(1.0, t.border), StrokeKind::Inside);
                kit::text_at(ui.painter(), kr.center(), egui::Align2::CENTER_CENTER, s, 11.5, muted);
                xr = kr.min.x - 4.0;
                any_right = true;
            }
        }
        if let Some(s) = &self.suffix {
            let r = kit::text_at(
                ui.painter(),
                pos2(if any_right { xr - 4.0 } else { rect.max.x - pad }, cy),
                egui::Align2::RIGHT_CENTER,
                s,
                fs,
                muted,
            );
            xr = r.min.x - 6.0;
        } else if !any_right {
            xr = rect.max.x - pad;
        }

        let text_rect = Rect::from_min_max(pos2(x, rect.min.y), pos2(xr.max(x + 10.0), rect.max.y));
        let mut te = TextEdit::singleline(&mut *text)
            .id(te_id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(FontId::proportional(fs))
            .text_color(if self.disabled { muted } else { t.foreground })
            .hint_text(RichText::new(self.placeholder.clone()).color(with_alpha(muted, 0.85)).size(fs))
            .desired_width(text_rect.width())
            .password(self.password && !revealed)
            .interactive(!self.disabled)
            .align(egui::Align2::LEFT_CENTER);
        if let Some(n) = self.char_limit {
            te = te.char_limit(n);
        }
        let mut resp = ui
            .scope_builder(UiBuilder::new().max_rect(text_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.add(te)
            })
            .inner;
        if cleared {
            resp.mark_changed();
        }
        let r = if self.pill { CornerRadius::same((h / 2.0) as u8) } else { control_radius(&t) };
        let shapes = chrome_shapes(rect, &t, hover, focus, error, self.disabled, r);
        ui.painter().set(chrome_idx, Shape::Vec(shapes));
        if self.strength {
            strength_meter(ui, text, width, base_id);
        }
        resp
    }
}

/// Paint the 4-segment strength meter (used by the password demo through [`Input::strength_meter`]).
fn strength_meter(ui: &mut Ui, text: &str, width: f32, id: Id) {
    let t = Theme::of(ui);
    let s = strength(text);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 4.0), Sense::hover());
    let gap = 4.0;
    let seg = (width - gap * 3.0) / 4.0;
    let col = match s {
        0 | 1 => t.destructive,
        2 => t.warning,
        3 => glowkit_core::color::mix(t.warning, t.success, 0.5),
        _ => t.success,
    };
    for i in 0..4 {
        let r = Rect::from_min_size(pos2(rect.min.x + i as f32 * (seg + gap), rect.min.y), vec2(seg, 4.0));
        let k = kit::anim(ui, id.with(("seg", i)), i < s, 0.25);
        ui.painter().rect_filled(r, CornerRadius::same(2), t.muted);
        if k > 0.0 {
            ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(seg * k, 4.0)), CornerRadius::same(2), col);
        }
    }
    let label = ["Enter a password", "Weak password", "Medium password", "Strong password", "Very strong password"][s];
    ui.label(RichText::new(label).size(12.0).color(if s == 0 { t.muted_foreground } else { col }));
}

/// Field chrome shapes, so they can be inserted beneath already-painted content.
fn chrome_shapes(
    rect: Rect,
    t: &Theme,
    hover: f32,
    focus: f32,
    error: bool,
    disabled: bool,
    r: CornerRadius,
) -> Vec<Shape> {
    let mut fill = kit::field_fill(t);
    if disabled {
        fill = if t.dark { with_alpha(t.input, 0.15) } else { t.muted };
    }
    let base = t.input.lerp_to_gamma(t.muted_foreground, 0.25 * hover);
    let accent = if error { t.destructive } else { t.ring };
    let border = if error { t.destructive } else { base.lerp_to_gamma(accent, focus) };
    let mut shapes: Vec<Shape> = Vec::new();
    if !t.dark && !disabled {
        shapes.push(
            egui::epaint::Shadow { offset: [0, 1], blur: 2, spread: 0, color: Color32::from_black_alpha(10) }
                .as_shape(rect, r)
                .into(),
        );
    }
    shapes.push(egui::epaint::RectShape::new(rect, r, fill, Stroke::new(1.0, border), StrokeKind::Inside).into());
    let k = if error { focus.max(0.4) } else { focus };
    if k > 0.01 {
        shapes.push(
            egui::epaint::RectShape::stroke(
                rect,
                r,
                Stroke::new(1.0 + 2.0 * k, with_alpha(accent, 0.32 * k)),
                StrokeKind::Outside,
            )
            .into(),
        );
    }
    shapes
}
