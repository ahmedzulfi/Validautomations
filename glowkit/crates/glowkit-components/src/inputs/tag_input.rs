use super::field::FieldLabel;
use super::kit::{self, control_radius, UiIcon};
use egui::{
    pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Layout, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, TextEdit, Ui, Widget,
};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A field holding a list of tags shown as removable chips. Type and press Enter or comma to add,
/// Backspace on an empty field removes the last tag.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut tags = vec!["rust".to_string(), "egui".to_string()];
/// use glowkit_components::TagInput;
/// ui.add(TagInput::new(&mut tags).placeholder("Add a tag…").max_tags(8));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TagInput<'a> {
    tags: &'a mut Vec<String>,
    placeholder: String,
    label: Option<String>,
    max_tags: Option<usize>,
    width: Option<f32>,
    allow_duplicates: bool,
}

impl<'a> TagInput<'a> {
    /// A tag input editing `tags`.
    pub fn new(tags: &'a mut Vec<String>) -> Self {
        Self {
            tags, placeholder: "Add tag…".into(), label: None, max_tags: None, width: None, allow_duplicates: false
        }
    }
    /// Placeholder for the text field.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Label above the field.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Maximum number of tags.
    pub fn max_tags(mut self, n: usize) -> Self {
        self.max_tags = Some(n);
        self
    }
    /// Width (default `min(available, 420)`).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Allow the same tag twice.
    pub fn allow_duplicates(mut self, a: bool) -> Self {
        self.allow_duplicates = a;
        self
    }
}

impl Widget for TagInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width().min(420.0));
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if let Some(l) = &self.label {
                ui.add(FieldLabel::new(l.clone()));
            }
            let id = ui.next_auto_id();
            ui.skip_ahead_auto_ids(1);
            let te_id = id.with("te");
            let buf_id = id.with("buf");
            let mut buf: String = glowkit_core::state(ui, buf_id, String::new);
            let chrome = ui.painter().add(Shape::Noop);
            let focused = ui.memory(|m| m.has_focus(te_id));
            let mut remove: Option<usize> = None;
            let mut changed = false;
            let full = self.max_tags.map(|m| self.tags.len() >= m).unwrap_or(false);
            let inner =
                egui::Frame::NONE.inner_margin(egui::Margin { left: 6, right: 6, top: 5, bottom: 5 }).show(ui, |ui| {
                    ui.set_width(width - 12.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                        for (i, tag) in self.tags.iter().enumerate() {
                            let g = ui.painter().layout_no_wrap(
                                tag.clone(),
                                FontId::proportional(13.0),
                                t.secondary_foreground,
                            );
                            let size = vec2(g.size().x + 10.0 + 20.0, 24.0);
                            let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                            let appear = kit::anim(ui, id.with(("tag", tag.as_str())), true, 0.2);
                            let rr = Rect::from_center_size(r.center(), r.size() * (0.85 + 0.15 * appear));
                            ui.painter().rect(
                                rr,
                                CornerRadius::same(5),
                                with_alpha(t.secondary, appear),
                                Stroke::new(1.0, with_alpha(t.border, appear)),
                                StrokeKind::Inside,
                            );
                            ui.painter().galley(
                                pos2(rr.min.x + 9.0, rr.center().y - g.size().y / 2.0),
                                g,
                                with_alpha(t.secondary_foreground, appear),
                            );
                            let xr = Rect::from_center_size(pos2(rr.max.x - 11.0, rr.center().y), vec2(16.0, 16.0));
                            let x = ui
                                .interact(xr, id.with(("x", i)), Sense::click())
                                .on_hover_cursor(CursorIcon::PointingHand);
                            if x.hovered() {
                                ui.painter().rect_filled(
                                    xr,
                                    CornerRadius::same(3),
                                    with_alpha(t.muted_foreground, 0.18),
                                );
                            }
                            UiIcon::X.paint(
                                ui.painter(),
                                xr.shrink(2.5),
                                if x.hovered() { t.foreground } else { t.muted_foreground },
                            );
                            if x.clicked() {
                                remove = Some(i);
                            }
                        }
                        let hint = if full { String::new() } else { self.placeholder.clone() };
                        let te = TextEdit::singleline(&mut buf)
                            .id(te_id)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::symmetric(4, 0))
                            .font(FontId::proportional(14.0))
                            .hint_text(RichText::new(hint).color(t.muted_foreground))
                            .desired_width(ui.available_width().clamp(80.0, 160.0))
                            .interactive(!full)
                            .min_size(vec2(0.0, 24.0))
                            .align(egui::Align2::LEFT_CENTER);
                        let r = ui.add(te);
                        let (enter, back) =
                            ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Backspace)));
                        let mut commit = |text: &str, tags: &mut Vec<String>| {
                            let s = text.trim().trim_end_matches(',').trim().to_owned();
                            if !s.is_empty() && (self.allow_duplicates || !tags.contains(&s)) && !full {
                                tags.push(s);
                                changed = true;
                            }
                        };
                        if buf.contains(',') {
                            let parts: Vec<String> = buf.split(',').map(|s| s.to_owned()).collect();
                            for p in &parts[..parts.len() - 1] {
                                commit(p, self.tags);
                            }
                            buf = parts.last().cloned().unwrap_or_default();
                        }
                        if r.lost_focus() && enter {
                            let b = std::mem::take(&mut buf);
                            commit(&b, self.tags);
                            r.request_focus();
                        } else if r.has_focus() && back && buf.is_empty() && !self.tags.is_empty() && !r.changed() {
                            remove = Some(self.tags.len() - 1);
                        }
                        r
                    })
                    .inner
                });
            if let Some(i) = remove {
                self.tags.remove(i);
                changed = true;
            }
            glowkit_core::set_state(ui, buf_id, buf);
            let rect = inner.response.rect;
            let bg = ui.interact(rect, id.with("bg"), Sense::click());
            if bg.clicked() {
                ui.memory_mut(|m| m.request_focus(te_id));
            }
            let hover = kit::anim(ui, id.with("h"), ui.rect_contains_pointer(rect), 0.12);
            let focus = kit::anim(ui, id.with("f"), focused, 0.16);
            let r = control_radius(&t);
            let border = t.input.lerp_to_gamma(t.muted_foreground, 0.25 * hover).lerp_to_gamma(t.ring, focus);
            let mut shapes = vec![];
            if !t.dark {
                shapes.push(
                    egui::epaint::Shadow { offset: [0, 1], blur: 2, spread: 0, color: Color32::from_black_alpha(10) }
                        .as_shape(rect, r)
                        .into(),
                );
            }
            shapes.push(
                egui::epaint::RectShape::new(
                    rect,
                    r,
                    kit::field_fill(&t),
                    Stroke::new(1.0, border),
                    StrokeKind::Inside,
                )
                .into(),
            );
            if focus > 0.01 {
                shapes.push(
                    egui::epaint::RectShape::stroke(
                        rect,
                        r,
                        Stroke::new(1.0 + 2.0 * focus, with_alpha(t.ring, 0.32 * focus)),
                        StrokeKind::Outside,
                    )
                    .into(),
                );
            }
            ui.painter().set(chrome, Shape::Vec(shapes));
            let mut resp = inner.inner;
            if changed {
                resp.mark_changed();
            }
            if let Some(m) = self.max_tags {
                ui.label(RichText::new(format!("{} / {m} tags", self.tags.len())).size(12.0).color(t.muted_foreground));
            }
            resp
        })
        .inner
    }
}

/// A star rating with hover preview, optional half stars and a springy pop on the hovered star.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut stars = 3.5;
/// use glowkit_components::Rating;
/// ui.add(Rating::new(&mut stars).half(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Rating<'a> {
    value: &'a mut f32,
    max: usize,
    half: bool,
    size: f32,
    color: Option<Color32>,
    read_only: bool,
    show_value: bool,
}

impl<'a> Rating<'a> {
    /// A rating bound to `value` (0..=max).
    pub fn new(value: &'a mut f32) -> Self {
        Self { value, max: 5, half: false, size: 22.0, color: None, read_only: false, show_value: false }
    }
    /// Number of stars (default 5).
    pub fn max(mut self, m: usize) -> Self {
        self.max = m.max(1);
        self
    }
    /// Allow half-star values.
    pub fn half(mut self, h: bool) -> Self {
        self.half = h;
        self
    }
    /// Star size in points (default 22).
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    /// Fill color (default: theme warning / amber).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Display only.
    pub fn read_only(mut self, r: bool) -> Self {
        self.read_only = r;
        self
    }
    /// Show the numeric value after the stars.
    pub fn show_value(mut self, s: bool) -> Self {
        self.show_value = s;
        self
    }
}

impl Widget for Rating<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let gap = self.size * 0.18;
        let stars_w = self.max as f32 * self.size + (self.max - 1) as f32 * gap;
        let extra = if self.show_value { 36.0 } else { 0.0 };
        let (rect, mut resp) = ui.allocate_exact_size(
            vec2(stars_w + extra, self.size),
            if self.read_only { Sense::hover() } else { Sense::click() },
        );
        if !self.read_only {
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        }
        let value_at = |x: f32| -> f32 {
            let rel = ((x - rect.min.x) / (self.size + gap)).max(0.0);
            let i = rel.floor();
            let frac = ((rel - i) * (self.size + gap) / self.size).clamp(0.0, 1.0);
            let v = if self.half && frac < 0.5 { i + 0.5 } else { i + 1.0 };
            v.min(self.max as f32)
        };
        let hover_v = if self.read_only {
            None
        } else {
            resp.hover_pos().filter(|p| p.x <= rect.min.x + stars_w).map(|p| value_at(p.x))
        };
        if let (true, Some(v)) = (resp.clicked(), hover_v) {
            *self.value = if (*self.value - v).abs() < 1e-3 { 0.0 } else { v };
            resp.mark_changed();
        }
        let shown = hover_v.unwrap_or(*self.value);
        let fill = self.color.unwrap_or(t.warning);
        let empty = if t.dark { t.secondary } else { t.muted.lerp_to_gamma(t.input, 0.7) };
        let painter = ui.painter();
        for i in 0..self.max {
            let c = pos2(rect.min.x + i as f32 * (self.size + gap) + self.size / 2.0, rect.center().y);
            let is_hovered = hover_v.map(|v| (v.ceil() as usize).saturating_sub(1) == i).unwrap_or(false);
            let s = glowkit_core::anim::spring(
                ui,
                resp.id.with(("s", i)),
                if is_hovered { 1.18 } else { 1.0 },
                500.0,
                14.0,
            );
            let outer = self.size * 0.5 * s;
            let inner = outer * 0.45;
            star_aa(painter, c, outer, inner, empty);
            let f = (shown - i as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                let clip = Rect::from_min_max(
                    pos2(c.x - outer, c.y - outer),
                    pos2(c.x - outer + 2.0 * outer * f, c.y + outer),
                );
                let col = if hover_v.is_some() { with_alpha(fill, 0.85) } else { fill };
                star_aa(&painter.with_clip_rect(clip.intersect(painter.clip_rect())), c, outer, inner, col);
            }
        }
        if self.show_value {
            painter.text(
                pos2(rect.min.x + stars_w + 10.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{shown:.1}"),
                FontId::proportional(14.0),
                t.muted_foreground,
            );
        }
        resp
    }
}

/// A filled star with an anti-aliased edge (rounded-ish tips).
pub(crate) fn star_aa(painter: &egui::Painter, c: egui::Pos2, outer: f32, inner: f32, color: Color32) {
    glowkit_core::paint::star(painter, c, outer - 0.5, inner - 0.3, 5, 0.0, color);
    let pts: Vec<egui::Pos2> = (0..10)
        .map(|i| {
            let a = i as f32 / 10.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let r = if i % 2 == 0 { outer - 0.5 } else { inner - 0.3 };
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    painter.add(Shape::closed_line(pts, Stroke::new(1.0, color)));
}
