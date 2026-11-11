use super::kit::{self, UiIcon};
use egui::{pos2, vec2, Color32, CornerRadius, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Stroke a closed polyline as dashes, shifted along the path by `offset` points.
pub(crate) fn dashed_closed(painter: &Painter, pts: &[Pos2], dash: f32, gap: f32, offset: f32, stroke: Stroke) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let period = dash + gap;
    let mut along = -offset.rem_euclid(period);
    let mut cur: Vec<Pos2> = Vec::new();
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let len = a.distance(b);
        let mut s = 0.0;
        while s < len {
            let phase = (along + s).rem_euclid(period);
            let in_dash = phase < dash;
            let remain = if in_dash { dash - phase } else { period - phase };
            let step = remain.min(len - s);
            let p0 = a + (b - a) * (s / len);
            let p1 = a + (b - a) * ((s + step) / len);
            if in_dash {
                if cur.is_empty() {
                    cur.push(p0);
                }
                cur.push(p1);
                if (step - remain).abs() < 1e-3 {
                    painter.add(egui::Shape::line(std::mem::take(&mut cur), stroke));
                }
            } else if !cur.is_empty() {
                painter.add(egui::Shape::line(std::mem::take(&mut cur), stroke));
            }
            s += step.max(1e-3);
        }
        along += len;
    }
    if cur.len() >= 2 {
        painter.add(egui::Shape::line(cur, stroke));
    }
}

type DropCallback<'a> = Box<dyn FnMut(&egui::DroppedFileHandle) + 'a>;

/// A drag-and-drop file area with a dashed border that marches while files hover over it, an
/// upload icon that lifts, and a list of received files with remove buttons.
///
/// Dropped file names are appended to the `files` vector; use [`FileDropzone::on_drop`] to read
/// contents, and the returned response's `.clicked()` to open your own file dialog.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut files: Vec<String> = vec![];
/// use glowkit_components::FileDropzone;
/// ui.add(FileDropzone::new(&mut files).hint("PNG, JPG or PDF up to 10 MB"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct FileDropzone<'a> {
    files: &'a mut Vec<String>,
    title: String,
    hint: String,
    height: f32,
    width: Option<f32>,
    list: bool,
    on_drop: Option<DropCallback<'a>>,
    active: Option<bool>,
}

impl<'a> FileDropzone<'a> {
    /// A dropzone collecting dropped file names into `files`.
    pub fn new(files: &'a mut Vec<String>) -> Self {
        Self {
            files,
            title: "Drag & drop files here".into(),
            hint: "or click to browse".into(),
            height: 160.0,
            width: None,
            list: true,
            on_drop: None,
            active: None,
        }
    }
    /// Main line of text.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Muted second line (accepted types, size limit...).
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = h.into();
        self
    }
    /// Height of the drop area.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// Width (default: available width, at most 480).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Show the list of received files under the area (default on).
    pub fn file_list(mut self, l: bool) -> Self {
        self.list = l;
        self
    }
    /// Called with every file dropped on the area (read bytes, upload...).
    pub fn on_drop(mut self, f: impl FnMut(&egui::DroppedFileHandle) + 'a) -> Self {
        self.on_drop = Some(Box::new(f));
        self
    }
    /// Force the drag-hover look on or off (for previews and tests).
    pub fn force_active(mut self, a: bool) -> Self {
        self.active = Some(a);
        self
    }
}

impl Widget for FileDropzone<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(480.0));
        ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let (rect, mut resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::click());
            resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            let (hovering_files, dropped) = ui.input(|i| (!i.raw.hovered_files.is_empty(), i.raw.dropped_files.clone()));
            let pointer_in = ui.rect_contains_pointer(rect) || ui.ctx().pointer_latest_pos().is_none();
            let drag_on = self.active.unwrap_or(hovering_files && pointer_in);
            if !dropped.is_empty() && (pointer_in || hovering_files) {
                for f in &dropped {
                    let name = f.path().file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| f.path().display().to_string());
                    self.files.push(name);
                    if let Some(cb) = &mut self.on_drop {
                        cb(f);
                    }
                }
                resp.mark_changed();
            }
            let k = kit::anim(ui, resp.id.with("drag"), drag_on, 0.18);
            let hov = kit::anim(ui, resp.id.with("hov"), resp.hovered(), 0.15);
            let painter = ui.painter();
            let r = t.radius + 2.0;
            let fill = with_alpha(t.muted, 0.35 + 0.25 * hov).lerp_to_gamma(with_alpha(t.accent, 0.08), k);
            painter.rect_filled(rect, CornerRadius::same(r as u8), fill);
            let pts = glowkit_core::paint::rounded_rect_points(rect.shrink(0.75), r, 8);
            let time = glowkit_core::time(ui);
            let offset = if k > 0.01 { time * 24.0 } else { 0.0 };
            let col = t.input.lerp_to_gamma(t.muted_foreground, 0.35 * hov + if t.dark { 0.2 } else { 0.1 }).lerp_to_gamma(t.accent, k);
            dashed_closed(painter, &pts, 6.0, 4.0, offset, Stroke::new(1.5, col));
            if k > 0.01 {
                glowkit_core::animate(ui);
            }
            // icon badge
            let lift = 4.0 * k + 2.0 * hov;
            let cy = rect.center().y - 20.0 - lift;
            let badge = Rect::from_center_size(pos2(rect.center().x, cy), vec2(44.0, 44.0));
            glowkit_core::paint::soft_shadow(painter, badge, 22.0, 8.0, vec2(0.0, 2.0 + lift), Color32::from_black_alpha(if t.dark { 60 } else { 18 }));
            painter.circle(badge.center(), 22.0, t.card, Stroke::new(1.0, t.border));
            UiIcon::Upload.paint(painter, Rect::from_center_size(badge.center(), vec2(20.0, 20.0)), t.foreground.lerp_to_gamma(t.accent, k));
            let title = if drag_on { "Drop to upload" } else { self.title.as_str() };
            painter.text(pos2(rect.center().x, cy + 38.0), egui::Align2::CENTER_CENTER, title, FontId::proportional(14.0), t.foreground);
            painter.text(pos2(rect.center().x, cy + 58.0), egui::Align2::CENTER_CENTER, &self.hint, FontId::proportional(12.5), t.muted_foreground);

            if self.list && !self.files.is_empty() {
                let mut remove = None;
                for (i, name) in self.files.iter().enumerate() {
                    let (row, _) = ui.allocate_exact_size(vec2(w, 44.0), Sense::hover());
                    let appear = kit::anim(ui, resp.id.with(("f", name.as_str(), i)), true, 0.25);
                    let row_r = row.translate(vec2(0.0, 6.0 * (1.0 - appear)));
                    let p = ui.painter();
                    p.rect(row_r, CornerRadius::same(8), with_alpha(t.card, appear), Stroke::new(1.0, with_alpha(t.border, appear)), StrokeKind::Inside);
                    let ib = Rect::from_center_size(pos2(row_r.min.x + 22.0, row_r.center().y), vec2(28.0, 28.0));
                    p.rect(ib, CornerRadius::same(6), with_alpha(t.muted, appear), Stroke::new(1.0, with_alpha(t.border, appear)), StrokeKind::Inside);
                    let icon = if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".svg") { UiIcon::Image } else { UiIcon::File };
                    icon.paint(p, ib.shrink(7.0), with_alpha(t.muted_foreground, appear));
                    p.with_clip_rect(row_r.shrink(1.0)).text(pos2(ib.max.x + 10.0, row_r.center().y), egui::Align2::LEFT_CENTER, name, FontId::proportional(13.5), with_alpha(t.foreground, appear));
                    let xr = Rect::from_center_size(pos2(row_r.max.x - 20.0, row_r.center().y), vec2(24.0, 24.0));
                    let xresp = ui.interact(xr, resp.id.with(("x", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    if xresp.hovered() {
                        ui.painter().rect_filled(xr, CornerRadius::same(5), t.secondary);
                    }
                    UiIcon::X.paint(ui.painter(), xr.shrink(5.0), if xresp.hovered() { t.foreground } else { t.muted_foreground });
                    if xresp.clicked() {
                        remove = Some(i);
                    }
                }
                if let Some(i) = remove {
                    self.files.remove(i);
                    resp.mark_changed();
                }
            }
            resp
        })
        .inner
    }
}
