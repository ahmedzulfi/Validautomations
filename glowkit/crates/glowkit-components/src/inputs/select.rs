use super::field::FieldLabel;
use super::kit::{self, control_radius, ControlSize, UiIcon};
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Rect, Response, RichText, Sense, Shape, Stroke, TextEdit, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use std::f32::consts::PI;

/// One entry of a [`Select`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    /// Display text (also what search matches against).
    pub label: String,
    /// Optional icon before the label.
    pub icon: Option<UiIcon>,
    /// Optional muted text on the right (a shortcut, a count, an email...).
    pub detail: Option<String>,
    /// Optional colored dot before the label (status pickers).
    pub dot: Option<Color32>,
}

impl SelectOption {
    /// A plain option.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), icon: None, detail: None, dot: None }
    }
    /// Add an icon.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Add muted right-aligned detail text.
    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }
    /// Add a colored status dot.
    pub fn dot(mut self, c: Color32) -> Self {
        self.dot = Some(c);
        self
    }
}

impl From<&str> for SelectOption {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SelectOption {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Default)]
struct SelState {
    query: String,
    highlight: usize,
    was_open: bool,
}

/// A dropdown select. With [`Select::searchable`] it becomes a combobox with a filter field and
/// keyboard navigation (↑ ↓ Enter).
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut fw: Option<usize> = None;
/// use glowkit_components::Select;
/// ui.add(Select::new(&mut fw, ["Next.js", "SvelteKit", "Nuxt", "Remix", "Astro"]).placeholder("Select framework…").searchable(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct Select<'a> {
    selected: &'a mut Option<usize>,
    options: Vec<SelectOption>,
    placeholder: String,
    label: Option<String>,
    searchable: bool,
    size: ControlSize,
    width: Option<f32>,
    max_height: f32,
    id_salt: Option<Id>,
    icon: Option<UiIcon>,
}

impl<'a> Select<'a> {
    /// A select bound to the selected index (`None` shows the placeholder).
    pub fn new<O: Into<SelectOption>>(selected: &'a mut Option<usize>, options: impl IntoIterator<Item = O>) -> Self {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            placeholder: "Select…".into(),
            label: None,
            searchable: false,
            size: ControlSize::Md,
            width: None,
            max_height: 260.0,
            id_salt: None,
            icon: None,
        }
    }
    /// Text shown when nothing is selected.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Label above the trigger.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Add a search field at the top of the list (combobox).
    pub fn searchable(mut self, s: bool) -> Self {
        self.searchable = s;
        self
    }
    /// Size variant of the trigger.
    pub fn size(mut self, s: ControlSize) -> Self {
        self.size = s;
        self
    }
    /// Trigger width (default `min(available, 240)`).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Maximum list height before scrolling.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = h;
        self
    }
    /// Leading icon in the trigger.
    pub fn icon(mut self, i: UiIcon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

/// Constructor for a searchable [`Select`] (shadcn's "Combobox").
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut v: Option<usize> = None;
/// ui.add(glowkit_components::Combobox::new(&mut v, ["Apple", "Banana", "Cherry"]));
/// ```
pub struct Combobox;

impl Combobox {
    /// A [`Select`] with search enabled.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<O: Into<SelectOption>>(selected: &mut Option<usize>, options: impl IntoIterator<Item = O>) -> Select<'_> {
        Select::new(selected, options).searchable(true).placeholder("Search…")
    }
}

fn paint_option_content(ui: &Ui, rect: Rect, opt: &SelectOption, color: Color32, fs: f32) -> f32 {
    let t = Theme::of(ui);
    let painter = ui.painter();
    let mut x = rect.min.x;
    let cy = rect.center().y;
    if let Some(d) = opt.dot {
        painter.circle_filled(pos2(x + 4.0, cy), 4.0, d);
        x += 16.0;
    }
    if let Some(i) = opt.icon {
        i.paint(painter, Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 16.0)), t.muted_foreground);
        x += 24.0;
    }
    let r = painter.text(pos2(x, cy), egui::Align2::LEFT_CENTER, &opt.label, FontId::proportional(fs), color);
    r.max.x
}

impl Widget for Select<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let width = self.width.unwrap_or_else(|| ui.available_width().min(240.0));
        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if let Some(l) = &self.label {
                ui.add(FieldLabel::new(l.clone()));
            }
            let h = self.size.height();
            let fs = self.size.font();
            let (rect, mut resp) = ui.allocate_exact_size(vec2(width, h), Sense::click());
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            let popup_id = match self.id_salt {
                Some(s) => ui.make_persistent_id(s),
                None => resp.id,
            }
            .with("select_popup");
            let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
            let hover = kit::anim(ui, resp.id.with("h"), resp.hovered(), 0.12);
            let focus = kit::anim(ui, resp.id.with("f"), open || resp.has_focus(), 0.16);
            kit::field_chrome(ui.painter(), rect, &t, hover, focus, false, false);
            let inner = rect.shrink2(vec2(12.0, 0.0));
            let mut x = inner.min.x;
            if let Some(i) = self.icon {
                i.paint(ui.painter(), Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(16.0, 16.0)), t.muted_foreground);
                x += 24.0;
            }
            let content = Rect::from_min_max(pos2(x, rect.min.y), pos2(inner.max.x - 22.0, rect.max.y));
            {
                let clip = ui.painter().with_clip_rect(content);
                match self.selected.and_then(|i| self.options.get(i)) {
                    Some(opt) => {
                        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(content));
                        let _ = clip;
                        sub.set_clip_rect(content);
                        paint_option_content(&sub, content, opt, t.foreground, fs);
                    }
                    None => {
                        clip.text(content.left_center(), egui::Align2::LEFT_CENTER, &self.placeholder, FontId::proportional(fs), t.muted_foreground);
                    }
                }
            }
            // chevron that flips when open
            let k = kit::anim(ui, resp.id.with("chev"), open, 0.2);
            let c = pos2(inner.max.x - 7.0, rect.center().y);
            let ang = PI * k;
            let rot = |p: egui::Vec2| c + vec2(p.x * ang.cos() - p.y * ang.sin(), p.x * ang.sin() + p.y * ang.cos());
            ui.painter().add(Shape::line(vec![rot(vec2(-4.0, -2.0)), rot(vec2(0.0, 2.0)), rot(vec2(4.0, -2.0))], Stroke::new(1.4, t.muted_foreground)));

            let sid = popup_id.with("state");
            let mut st: SelState = glowkit_core::state(ui, sid, SelState::default);
            if resp.clicked() {
                egui::Popup::toggle_id(ui.ctx(), popup_id);
            }
            let mut changed = false;
            let popup = egui::Popup::new(popup_id, ui.ctx().clone(), rect, ui.layer_id())
                .open_memory(None)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .gap(4.0)
                .width(width)
                .frame(
                    egui::Frame::new()
                        .fill(t.popover)
                        .stroke(t.border_stroke())
                        .corner_radius(control_radius(&t))
                        .inner_margin(egui::Margin::same(4))
                        .shadow(egui::epaint::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(if t.dark { 90 } else { 28 }) }),
                );
            let shown = popup.show(|ui| {
                ui.set_min_width(width - 8.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                if !st.was_open {
                    st.query.clear();
                    st.highlight = self.selected.unwrap_or(0);
                }
                let q = st.query.to_lowercase();
                let visible: Vec<usize> = (0..self.options.len()).filter(|&i| q.is_empty() || self.options[i].label.to_lowercase().contains(&q)).collect();
                if self.searchable {
                    let (sr, _) = ui.allocate_exact_size(vec2(width - 8.0, 34.0), Sense::hover());
                    UiIcon::Search.paint(ui.painter(), Rect::from_center_size(pos2(sr.min.x + 16.0, sr.center().y), vec2(15.0, 15.0)), t.muted_foreground);
                    let te_rect = Rect::from_min_max(pos2(sr.min.x + 32.0, sr.min.y), pos2(sr.max.x - 8.0, sr.max.y));
                    let te_id = popup_id.with("q");
                    let r = ui
                        .scope_builder(egui::UiBuilder::new().max_rect(te_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
                            ui.add(
                                TextEdit::singleline(&mut st.query)
                                    .id(te_id)
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::ZERO)
                                    .font(FontId::proportional(14.0))
                                    .hint_text(RichText::new("Search…").color(t.muted_foreground))
                                    .desired_width(te_rect.width())
                                    .align(egui::Align2::LEFT_CENTER),
                            )
                        })
                        .inner;
                    if !st.was_open {
                        r.request_focus();
                    }
                    if r.changed() {
                        st.highlight = 0;
                    }
                    ui.painter().line_segment([pos2(sr.min.x - 4.0, sr.max.y), pos2(sr.max.x + 4.0, sr.max.y)], t.border_stroke());
                    ui.add_space(4.0);
                }
                // keyboard navigation
                let (down, up, enter) = ui.input(|i| (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::Enter)));
                if !visible.is_empty() {
                    let pos = visible.iter().position(|&i| i == st.highlight).unwrap_or(0);
                    if down {
                        st.highlight = visible[(pos + 1).min(visible.len() - 1)];
                    } else if up {
                        st.highlight = visible[pos.saturating_sub(1)];
                    } else if !visible.contains(&st.highlight) {
                        st.highlight = visible[0];
                    }
                    if enter {
                        *self.selected = Some(st.highlight);
                        changed = true;
                        ui.close();
                    }
                }
                if visible.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(width - 8.0, 56.0), Sense::hover());
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "No results found.", FontId::proportional(13.5), t.muted_foreground);
                }
                egui::ScrollArea::vertical().max_height(self.max_height).auto_shrink([false, true]).show(ui, |ui| {
                    for &i in &visible {
                        let opt = &self.options[i];
                        let (r, row) = ui.allocate_exact_size(vec2(width - 8.0, 32.0), Sense::click());
                        let row = row.on_hover_cursor(CursorIcon::PointingHand);
                        if row.hovered() && ui.input(|inp| inp.pointer.delta().length_sq() > 0.0) {
                            st.highlight = i;
                        }
                        let hl = kit::anim(ui, popup_id.with(("hl", i)), st.highlight == i, 0.08);
                        if hl > 0.0 {
                            let c = if t.dark { t.secondary } else { t.muted };
                            ui.painter().rect_filled(r, CornerRadius::same(control_radius(&t).nw.saturating_sub(3)), with_alpha(c, hl));
                        }
                        let content = r.shrink2(vec2(8.0, 0.0));
                        paint_option_content(ui, content, opt, t.foreground, 14.0);
                        let mut right = content.max.x;
                        if *self.selected == Some(i) {
                            UiIcon::Check.paint(ui.painter(), Rect::from_center_size(pos2(right - 8.0, r.center().y), vec2(15.0, 15.0)), t.foreground);
                            right -= 24.0;
                        }
                        if let Some(d) = &opt.detail {
                            ui.painter().text(pos2(right, r.center().y), egui::Align2::RIGHT_CENTER, d, FontId::proportional(12.5), t.muted_foreground);
                        }
                        if row.clicked() {
                            *self.selected = Some(i);
                            changed = true;
                            ui.close();
                        }
                    }
                });
            });
            st.was_open = shown.is_some() && egui::Popup::is_id_open(ui.ctx(), popup_id);
            glowkit_core::set_state(ui, sid, st);
            if changed {
                resp.mark_changed();
            }
            resp
        })
        .inner
    }
}
