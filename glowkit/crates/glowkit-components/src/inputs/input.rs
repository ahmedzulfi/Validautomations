use super::field::{FieldDescription, FieldError, FieldLabel};
use super::kit::{self, control_radius, ControlSize, UiIcon};
use egui::emath::Numeric;
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Widget};
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
            ui.add(FieldError::new(e.clone()));
        } else if let Some(d) = &self.description {
            ui.add(FieldDescription::new(d.clone()));
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
    icon.paint(ui.painter(), Rect::from_center_size(center, vec2(size, size)), t.muted_foreground.lerp_to_gamma(t.foreground, h));
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

/// Constructor for an [`Input`] preconfigured as a search box (magnifier icon, clear button).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut q = String::new();
/// ui.add(glowkit_components::SearchInput::new(&mut q).shortcut("⌘K"));
/// ```
pub struct SearchInput;

impl SearchInput {
    /// A search [`Input`].
    #[allow(clippy::new_ret_no_self)]
    pub fn new(text: &mut String) -> Input<'_> {
        Input::new(text).icon(UiIcon::Search).clearable(true).placeholder("Search…")
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
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
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
            let r = kit::text_at(ui.painter(), pos2(if any_right { xr - 4.0 } else { rect.max.x - pad }, cy), egui::Align2::RIGHT_CENTER, s, fs, muted);
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
            .scope_builder(UiBuilder::new().max_rect(text_rect).layout(Layout::left_to_right(Align::Center)), |ui| ui.add(te))
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

/// A multi-line text area with label, auto-growing height and an optional character counter.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut bio = String::new();
/// use glowkit_components::Textarea;
/// ui.add(Textarea::new(&mut bio).label("Bio").placeholder("Tell us about yourself").max_len(280));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Textarea<'a> {
    text: &'a mut String,
    placeholder: String,
    decor: Decor,
    rows: usize,
    max_len: Option<usize>,
    width: Option<f32>,
    disabled: bool,
    id_salt: Option<Id>,
}

impl<'a> Textarea<'a> {
    /// A text area editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self { text, placeholder: String::new(), decor: Decor::default(), rows: 3, max_len: None, width: None, disabled: false, id_salt: None }
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
    /// Helper text below the field.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.decor.description = Some(d.into());
        self
    }
    /// Error message (red border).
    pub fn error(mut self, e: impl Into<String>) -> Self {
        self.decor.error = Some(e.into());
        self
    }
    /// Minimum visible rows (the area grows with its content).
    pub fn rows(mut self, r: usize) -> Self {
        self.rows = r.max(1);
        self
    }
    /// Character limit; also shows a `12/280` counter.
    pub fn max_len(mut self, n: usize) -> Self {
        self.max_len = Some(n);
        self
    }
    /// Width in points (default `min(available, 420)`).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Read-only and dimmed.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

impl Widget for Textarea<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = match self.width {
            Some(w) if w.is_finite() => w,
            Some(_) => ui.available_width(),
            None => ui.available_width().min(420.0),
        };
        let decor = self.decor.clone();
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            decor.top(ui);
            let base_id = match self.id_salt {
                Some(s) => ui.make_persistent_id(s),
                None => {
                    let id = ui.next_auto_id();
                    ui.skip_ahead_auto_ids(1);
                    id
                }
            };
            let te_id = base_id.with("te");
            let chrome_idx = ui.painter().add(Shape::Noop);
            let focused = ui.memory(|m| m.has_focus(te_id));
            let focus = kit::anim(ui, base_id.with("f"), focused, 0.16);
            let text = self.text;
            let mut te = TextEdit::multiline(&mut *text)
                .id(te_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(FontId::proportional(14.0))
                .text_color(t.foreground)
                .hint_text(RichText::new(self.placeholder.clone()).color(with_alpha(t.muted_foreground, 0.85)).size(14.0))
                .desired_rows(self.rows)
                .desired_width(width - 24.0)
                .interactive(!self.disabled);
            if let Some(n) = self.max_len {
                te = te.char_limit(n);
            }
            let inner = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 9)).show(ui, |ui| {
                ui.set_width(width - 24.0);
                ui.add(te)
            });
            let rect = inner.response.rect;
            let bg = ui.interact(rect, base_id.with("bg"), Sense::click()).on_hover_cursor(CursorIcon::Text);
            if bg.clicked() {
                ui.memory_mut(|m| m.request_focus(te_id));
            }
            let hover = kit::anim(ui, base_id.with("h"), ui.rect_contains_pointer(rect), 0.12);
            let error = decor.error.is_some();
            let painter = ui.painter().clone();
            let shapes = chrome_shapes(rect, &t, hover, focus, error, self.disabled, control_radius(&t));
            painter.set(chrome_idx, Shape::Vec(shapes));
            let resp = inner.inner;
            // footer: description / error on the left, counter on the right
            if let Some(max) = self.max_len {
                let (row, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
                let n = text.chars().count();
                let near = n as f32 / max as f32;
                let col = if near >= 1.0 { t.destructive } else if near > 0.9 { t.warning } else { t.muted_foreground };
                let p = ui.painter();
                let cr = p.text(row.right_center(), egui::Align2::RIGHT_CENTER, format!("{n}/{max}"), FontId::proportional(12.0), col);
                let (msg, mcol) = match (&decor.error, &decor.description) {
                    (Some(e), _) => (Some(e.clone()), t.destructive),
                    (None, Some(d)) => (Some(d.clone()), t.muted_foreground),
                    _ => (None, t.muted_foreground),
                };
                if let Some(m) = msg {
                    p.with_clip_rect(Rect::from_min_max(row.min, pos2(cr.min.x - 8.0, row.max.y))).text(row.left_center(), egui::Align2::LEFT_CENTER, m, FontId::proportional(12.5), mcol);
                }
            } else {
                decor.bottom(ui);
            }
            resp
        })
        .inner
    }
}

/// Field chrome shapes, so they can be inserted beneath already-painted content.
fn chrome_shapes(rect: Rect, t: &Theme, hover: f32, focus: f32, error: bool, disabled: bool, r: CornerRadius) -> Vec<Shape> {
    let mut fill = kit::field_fill(t);
    if disabled {
        fill = if t.dark { with_alpha(t.input, 0.15) } else { t.muted };
    }
    let base = t.input.lerp_to_gamma(t.muted_foreground, 0.25 * hover);
    let accent = if error { t.destructive } else { t.ring };
    let border = if error { t.destructive } else { base.lerp_to_gamma(accent, focus) };
    let mut shapes: Vec<Shape> = Vec::new();
    if !t.dark && !disabled {
        shapes.push(egui::epaint::Shadow { offset: [0, 1], blur: 2, spread: 0, color: Color32::from_black_alpha(10) }.as_shape(rect, r).into());
    }
    shapes.push(egui::epaint::RectShape::new(rect, r, fill, Stroke::new(1.0, border), StrokeKind::Inside).into());
    let k = if error { focus.max(0.4) } else { focus };
    if k > 0.01 {
        shapes.push(egui::epaint::RectShape::stroke(rect, r, Stroke::new(1.0 + 2.0 * k, with_alpha(accent, 0.32 * k)), StrokeKind::Outside).into());
    }
    shapes
}

#[derive(Clone)]
struct NumState {
    editing: bool,
    buf: String,
    prev: String,
    shown: f64,
    dir: f32,
    changed_at: f64,
    edit_started: f64,
}

/// A numeric field with − / + stepper buttons; the value slides up or down when it changes.
/// Click the number to type a value.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut qty = 1u32;
/// use glowkit_components::NumberInput;
/// ui.add(NumberInput::new(&mut qty).range(0.0..=99.0).label("Quantity"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct NumberInput<'a, N: Numeric> {
    value: &'a mut N,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    prefix: String,
    suffix: String,
    decor: Decor,
    size: ControlSize,
    width: f32,
}

impl<'a, N: Numeric> NumberInput<'a, N> {
    /// A stepper bound to `value`.
    pub fn new(value: &'a mut N) -> Self {
        Self {
            value,
            min: N::MIN.to_f64(),
            max: N::MAX.to_f64(),
            step: 1.0,
            decimals: if N::INTEGRAL { 0 } else { 2 },
            prefix: String::new(),
            suffix: String::new(),
            decor: Decor::default(),
            size: ControlSize::Md,
            width: 150.0,
        }
    }
    /// Allowed range.
    pub fn range(mut self, r: std::ops::RangeInclusive<f64>) -> Self {
        self.min = *r.start();
        self.max = *r.end();
        self
    }
    /// Increment per click.
    pub fn step(mut self, s: f64) -> Self {
        self.step = s;
        self
    }
    /// Digits after the decimal point.
    pub fn decimals(mut self, d: usize) -> Self {
        self.decimals = d;
        self
    }
    /// Text before the number (e.g. `"$"`).
    pub fn prefix(mut self, p: impl Into<String>) -> Self {
        self.prefix = p.into();
        self
    }
    /// Text after the number (e.g. `" kg"`).
    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = s.into();
        self
    }
    /// Label above the field.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.decor.label = Some(l.into());
        self
    }
    /// Helper text below the field.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.decor.description = Some(d.into());
        self
    }
    /// Size variant.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Total width in points (default 150).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
}

impl<N: Numeric> Widget for NumberInput<'_, N> {
    fn ui(self, ui: &mut Ui) -> Response {
        let decor = self.decor.clone();
        let width = self.width;
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            decor.top(ui);
            let r = self.field(ui);
            decor.bottom(ui);
            r
        })
        .inner
    }
}

impl<N: Numeric> NumberInput<'_, N> {
    fn fmt(&self, v: f64) -> String {
        format!("{}{:.*}{}", self.prefix, self.decimals, v, self.suffix)
    }

    fn field(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let h = self.size.height();
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        let te_id = id.with("te");
        let (rect, mut resp) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let now = ui.input(|i| i.time);
        let cur = self.value.to_f64();
        let mut st: NumState = glowkit_core::state(ui, id, || NumState { editing: false, buf: String::new(), prev: String::new(), shown: cur, dir: 0.0, changed_at: -10.0, edit_started: 0.0 });
        let focused = ui.memory(|m| m.has_focus(te_id));
        let hover = kit::anim(ui, id.with("h"), ui.rect_contains_pointer(rect), 0.12);
        let focus = kit::anim(ui, id.with("f"), focused, 0.16);
        kit::field_chrome(ui.painter(), rect, &t, hover, focus, false, false);
        let bw = h;
        let minus_r = Rect::from_min_size(rect.min, vec2(bw, h));
        let plus_r = Rect::from_min_size(pos2(rect.max.x - bw, rect.min.y), vec2(bw, h));
        let mut new = cur;
        for (r, icon, delta, key) in [(minus_r, UiIcon::Minus, -self.step, "m"), (plus_r, UiIcon::Plus, self.step, "p")] {
            let enabled = if delta < 0.0 { cur > self.min } else { cur < self.max };
            let b = ui.interact(r.shrink(1.0), id.with(key), if enabled { Sense::click() } else { Sense::hover() });
            let b = if enabled { b.on_hover_cursor(CursorIcon::PointingHand) } else { b };
            let bh = kit::anim(ui, id.with((key, "h")), b.hovered() && enabled, 0.1);
            let corner = control_radius(&t).nw.saturating_sub(1);
            let c = if delta < 0.0 { CornerRadius { nw: corner, sw: corner, ne: 0, se: 0 } } else { CornerRadius { nw: 0, sw: 0, ne: corner, se: corner } };
            if bh > 0.0 {
                ui.painter().rect_filled(r.shrink(1.0), c, with_alpha(t.muted_foreground, 0.1 * bh));
            }
            let col = if enabled { t.muted_foreground.lerp_to_gamma(t.foreground, bh) } else { with_alpha(t.muted_foreground, 0.35) };
            icon.paint(ui.painter(), Rect::from_center_size(r.center(), vec2(15.0, 15.0)), col);
            let x = if delta < 0.0 { r.max.x - 0.5 } else { r.min.x + 0.5 };
            ui.painter().line_segment([pos2(x, rect.min.y + 1.0), pos2(x, rect.max.y - 1.0)], Stroke::new(1.0, t.input));
            if b.clicked() {
                new = (cur + delta).clamp(self.min, self.max);
            }
        }
        let mid = Rect::from_min_max(pos2(minus_r.max.x + 1.0, rect.min.y), pos2(plus_r.min.x - 1.0, rect.max.y));
        let fs = self.size.font();
        if st.editing {
            let te = TextEdit::singleline(&mut st.buf)
                .id(te_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(FontId::proportional(fs))
                .text_color(t.foreground)
                .align(egui::Align2::CENTER_CENTER)
                .desired_width(mid.width() - 8.0);
            let r = ui.scope_builder(UiBuilder::new().max_rect(mid.shrink2(vec2(4.0, 0.0))).layout(Layout::left_to_right(Align::Center)), |ui| ui.add(te)).inner;
            if r.lost_focus() || (!r.has_focus() && now - st.edit_started > 0.3) {
                let digits: String = st.buf.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
                if let Ok(v) = digits.parse::<f64>() {
                    new = v.clamp(self.min, self.max);
                }
                st.editing = false;
            }
        } else {
            let click = ui.interact(mid, id.with("mid"), Sense::click()).on_hover_cursor(CursorIcon::Text);
            if click.clicked() {
                st.editing = true;
                st.edit_started = now;
                st.buf = format!("{:.*}", self.decimals, cur);
                ui.memory_mut(|m| m.request_focus(te_id));
            }
            let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, ((now - st.changed_at) / 0.28) as f32);
            if k < 1.0 {
                ui.ctx().request_repaint();
            }
            let painter = ui.painter().with_clip_rect(mid);
            let off = 12.0 * st.dir;
            let cur_s = self.fmt(cur);
            painter.text(mid.center() + vec2(0.0, off * (1.0 - k)), egui::Align2::CENTER_CENTER, &cur_s, FontId::proportional(fs), with_alpha(t.foreground, k.max(0.0)));
            if k < 1.0 {
                painter.text(mid.center() - vec2(0.0, off * k), egui::Align2::CENTER_CENTER, &st.prev, FontId::proportional(fs), with_alpha(t.foreground, 1.0 - k));
            }
        }
        if (new - cur).abs() > 1e-12 {
            st.prev = self.fmt(cur);
            st.dir = if new > cur { -1.0 } else { 1.0 };
            st.changed_at = now;
            st.shown = new;
            *self.value = N::from_f64(new);
            resp.mark_changed();
        }
        glowkit_core::set_state(ui, id, st);
        resp
    }
}
