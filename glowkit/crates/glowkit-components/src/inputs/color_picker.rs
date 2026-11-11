use super::kit::{self, control_radius};
use egui::{pos2, vec2, Align, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, Widget};
use glowkit_core::color::{hex, hsv, with_alpha};
use glowkit_core::paint::{fill_convex_fn, rounded_rect_points, subdivide_closed};
use glowkit_core::Theme;

/// Default swatches (a balanced Tailwind-like spectrum).
pub const DEFAULT_SWATCHES: [Color32; 10] = [
    hex("#EF4444"),
    hex("#F97316"),
    hex("#F59E0B"),
    hex("#22C55E"),
    hex("#14B8A6"),
    hex("#06B6D4"),
    hex("#3B82F6"),
    hex("#8B5CF6"),
    hex("#EC4899"),
    hex("#71717A"),
];

fn to_hsv(c: Color32) -> [f32; 3] {
    let [r, g, b, _] = c.to_srgba_unmultiplied().map(|x| x as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-6 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    [h, s, max]
}

fn to_hex(c: Color32) -> String {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    if !(s.len() == 6 || s.len() == 3) || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(hex(&format!("#{s}")))
}

#[derive(Clone)]
struct PickerState {
    hsv: [f32; 3],
    alpha: f32,
    last: Color32,
    hex_buf: String,
}

/// A color picker in the Kibo UI style: saturation/brightness area, hue and alpha sliders, hex
/// field and preset swatches. Can be shown inline or as a swatch button with a popover.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut color = egui::Color32::from_rgb(139, 92, 246);
/// use glowkit_components::ColorPicker;
/// ui.add(ColorPicker::new(&mut color));
/// ui.add(ColorPicker::new(&mut color).popover(true));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ColorPicker<'a> {
    color: &'a mut Color32,
    width: f32,
    alpha: bool,
    swatches: Vec<Color32>,
    popover: bool,
}

impl<'a> ColorPicker<'a> {
    /// A picker editing `color`.
    pub fn new(color: &'a mut Color32) -> Self {
        Self { color, width: 248.0, alpha: true, swatches: DEFAULT_SWATCHES.to_vec(), popover: false }
    }
    /// Panel width (default 248).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Show the alpha slider (default on).
    pub fn alpha(mut self, a: bool) -> Self {
        self.alpha = a;
        self
    }
    /// Preset swatches (empty to hide the row).
    pub fn swatches(mut self, s: &[Color32]) -> Self {
        self.swatches = s.to_vec();
        self
    }
    /// Render as a compact swatch button that opens the picker in a popover.
    pub fn popover(mut self, p: bool) -> Self {
        self.popover = p;
        self
    }
}

fn checker(painter: &egui::Painter, rect: Rect, cell: f32, t: &Theme) {
    let a = if t.dark { Color32::from_gray(70) } else { Color32::from_gray(255) };
    let b = if t.dark { Color32::from_gray(40) } else { Color32::from_gray(220) };
    painter.rect_filled(rect, CornerRadius::ZERO, a);
    let nx = (rect.width() / cell).ceil() as i32;
    let ny = (rect.height() / cell).ceil() as i32;
    for y in 0..ny {
        for x in 0..nx {
            if (x + y) % 2 == 0 {
                let r = Rect::from_min_size(rect.min + vec2(x as f32 * cell, y as f32 * cell), vec2(cell, cell)).intersect(rect);
                painter.rect_filled(r, CornerRadius::ZERO, b);
            }
        }
    }
}

fn thumb(painter: &egui::Painter, c: Pos2, fill: Color32, r: f32) {
    glowkit_core::paint::soft_shadow(painter, Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)), r, 4.0, vec2(0.0, 1.0), Color32::from_black_alpha(70));
    painter.circle(c, r, fill, Stroke::new(2.5, Color32::WHITE));
    painter.circle_stroke(c, r + 1.25, Stroke::new(0.6, Color32::from_black_alpha(40)));
}

impl ColorPicker<'_> {
    fn panel(self, ui: &mut Ui, id: Id) -> Response {
        let t = Theme::of(ui);
        let mut st: PickerState = glowkit_core::state(ui, id, || PickerState { hsv: to_hsv(*self.color), alpha: self.color.a() as f32 / 255.0, last: *self.color, hex_buf: to_hex(*self.color) });
        if st.last != *self.color {
            let new = to_hsv(*self.color);
            st.hsv = [if new[1] < 1e-3 || new[2] < 1e-3 { st.hsv[0] } else { new[0] }, new[1], new[2]];
            st.alpha = self.color.to_srgba_unmultiplied()[3] as f32 / 255.0;
            st.hex_buf = to_hex(*self.color);
        }
        let w = self.width;
        let mut changed = false;
        let resp = ui
            .allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                // saturation / value area
                let (area, ar) = ui.allocate_exact_size(vec2(w, w * 0.62), Sense::click_and_drag());
                let ar = ar.on_hover_cursor(CursorIcon::Crosshair);
                if let Some(p) = ar.interact_pointer_pos() {
                    if ar.dragged() || ar.clicked() {
                        st.hsv[1] = ((p.x - area.min.x) / area.width()).clamp(0.0, 1.0);
                        st.hsv[2] = 1.0 - ((p.y - area.min.y) / area.height()).clamp(0.0, 1.0);
                        changed = true;
                    }
                }
                let painter = ui.painter();
                let rad = control_radius(&t).nw as f32;
                let hue = st.hsv[0];
                let outline = subdivide_closed(&rounded_rect_points(area, rad, 6), 8.0);
                fill_convex_fn(painter, &outline, 10, |p| {
                    let s = ((p.x - area.min.x) / area.width()).clamp(0.0, 1.0);
                    let v = 1.0 - ((p.y - area.min.y) / area.height()).clamp(0.0, 1.0);
                    hsv(hue, s, v)
                });
                painter.rect_stroke(area, CornerRadius::same(rad as u8), Stroke::new(1.0, with_alpha(Color32::BLACK, 0.08)), StrokeKind::Inside);
                let c = pos2(area.min.x + st.hsv[1] * area.width(), area.min.y + (1.0 - st.hsv[2]) * area.height());
                let grab = glowkit_core::anim::spring(ui, id.with("grab"), if ar.dragged() { 1.0 } else { 0.0 }, 500.0, 24.0);
                thumb(painter, c, hsv(hue, st.hsv[1], st.hsv[2]), 7.0 + 2.0 * grab);

                // preview + sliders
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let rows: f32 = if self.alpha { 2.0 } else { 1.0 };
                    let sh = 12.0 * rows + 10.0 * (rows - 1.0);
                    let pv = sh.max(28.0);
                    let (prev, _) = ui.allocate_exact_size(vec2(pv, pv), Sense::hover());
                    let cur = hsv(st.hsv[0], st.hsv[1], st.hsv[2]);
                    let pr = CornerRadius::same(6);
                    {
                        let p = ui.painter().with_clip_rect(prev);
                        checker(&p, prev, 5.0, &t);
                    }
                    ui.painter().rect(prev, pr, with_alpha(cur, st.alpha), Stroke::new(1.0, t.border), StrokeKind::Inside);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        let sw = w - pv - 12.0;
                        // hue
                        let (hr, hresp) = ui.allocate_exact_size(vec2(sw, 12.0), Sense::click_and_drag());
                        let hresp = hresp.on_hover_cursor(CursorIcon::PointingHand);
                        if let Some(p) = hresp.interact_pointer_pos() {
                            if hresp.dragged() || hresp.clicked() {
                                st.hsv[0] = ((p.x - hr.min.x - 6.0) / (hr.width() - 12.0)).clamp(0.0, 1.0) * 359.9;
                                changed = true;
                            }
                        }
                        let pts = subdivide_closed(&rounded_rect_points(hr, 6.0, 6), 4.0);
                        fill_convex_fn(ui.painter(), &pts, 3, |p| hsv(((p.x - hr.min.x - 6.0) / (hr.width() - 12.0)).clamp(0.0, 1.0) * 360.0, 1.0, 1.0));
                        thumb(ui.painter(), pos2(hr.min.x + 6.0 + st.hsv[0] / 359.9 * (hr.width() - 12.0), hr.center().y), hsv(st.hsv[0], 1.0, 1.0), 7.0);
                        if self.alpha {
                            let (arr, aresp) = ui.allocate_exact_size(vec2(sw, 12.0), Sense::click_and_drag());
                            let aresp = aresp.on_hover_cursor(CursorIcon::PointingHand);
                            if let Some(p) = aresp.interact_pointer_pos() {
                                if aresp.dragged() || aresp.clicked() {
                                    st.alpha = ((p.x - arr.min.x - 6.0) / (arr.width() - 12.0)).clamp(0.0, 1.0);
                                    changed = true;
                                }
                            }
                            let pts = subdivide_closed(&rounded_rect_points(arr, 6.0, 6), 4.0);
                            let base = if t.dark { Color32::from_gray(70) } else { Color32::from_gray(255) };
                            fill_convex_fn(ui.painter(), &pts, 2, |_| base);
                            {
                                let inner = arr.shrink2(vec2(6.0, 0.0));
                                let p = ui.painter().with_clip_rect(inner);
                                checker(&p, inner, 4.0, &t);
                            }
                            fill_convex_fn(ui.painter(), &pts, 3, |p| with_alpha(cur, ((p.x - arr.min.x - 6.0) / (arr.width() - 12.0)).clamp(0.0, 1.0)));
                            thumb(ui.painter(), pos2(arr.min.x + 6.0 + st.alpha * (arr.width() - 12.0), arr.center().y), with_alpha(cur, st.alpha), 7.0);
                        }
                    });
                });

                // hex + alpha fields
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let fw = if self.alpha { w - 72.0 - 8.0 } else { w };
                    let (fr, _) = ui.allocate_exact_size(vec2(fw, 32.0), Sense::hover());
                    let te_id = id.with("hex");
                    let focused = ui.memory(|m| m.has_focus(te_id));
                    let fk = kit::anim(ui, te_id.with("f"), focused, 0.15);
                    kit::field_chrome(ui.painter(), fr, &t, 0.0, fk, false, false);
                    ui.painter().text(pos2(fr.min.x + 10.0, fr.center().y), egui::Align2::LEFT_CENTER, "HEX", FontId::proportional(11.5), t.muted_foreground);
                    if !focused {
                        st.hex_buf = to_hex(hsv(st.hsv[0], st.hsv[1], st.hsv[2]));
                    }
                    let ter = Rect::from_min_max(pos2(fr.min.x + 44.0, fr.min.y), pos2(fr.max.x - 8.0, fr.max.y));
                    let r = ui
                        .scope_builder(egui::UiBuilder::new().max_rect(ter).layout(Layout::left_to_right(Align::Center)), |ui| {
                            ui.add(
                                TextEdit::singleline(&mut st.hex_buf)
                                    .id(te_id)
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::ZERO)
                                    .font(FontId::monospace(13.0))
                                    .text_color(t.foreground)
                                    .desired_width(ter.width())
                                    .char_limit(7)
                                    .align(egui::Align2::LEFT_CENTER),
                            )
                        })
                        .inner;
                    if r.changed() {
                        if let Some(c) = parse_hex(&st.hex_buf) {
                            let h = to_hsv(c);
                            st.hsv = [if h[1] < 1e-3 { st.hsv[0] } else { h[0] }, h[1], h[2]];
                            changed = true;
                        }
                    }
                    if self.alpha {
                        let (ar, _) = ui.allocate_exact_size(vec2(72.0, 32.0), Sense::hover());
                        kit::field_chrome(ui.painter(), ar, &t, 0.0, 0.0, false, false);
                        ui.painter().text(ar.center(), egui::Align2::CENTER_CENTER, format!("{:.0}%", st.alpha * 100.0), FontId::proportional(13.0), t.foreground);
                    }
                });

                if !self.swatches.is_empty() {
                    let n = self.swatches.len();
                    let gap = 6.0;
                    let s = ((w - gap * (n - 1) as f32) / n as f32).min(24.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        let cur = hsv(st.hsv[0], st.hsv[1], st.hsv[2]);
                        for (i, &c) in self.swatches.iter().enumerate() {
                            let (r, sr) = ui.allocate_exact_size(vec2(s, s), Sense::click());
                            let sr = sr.on_hover_cursor(CursorIcon::PointingHand);
                            let hov = glowkit_core::anim::spring(ui, id.with(("sw", i)), if sr.hovered() { 1.0 } else { 0.0 }, 500.0, 20.0);
                            let rr = Rect::from_center_size(r.center(), r.size() * (1.0 + 0.12 * hov));
                            let selected = to_hex(c) == to_hex(cur);
                            ui.painter().rect_filled(rr, CornerRadius::same(6), c);
                            if selected {
                                ui.painter().rect_stroke(rr.expand(2.0), CornerRadius::same(8), Stroke::new(1.5, t.foreground), StrokeKind::Inside);
                                ui.painter().rect_stroke(rr, CornerRadius::same(6), Stroke::new(1.0, t.background), StrokeKind::Outside);
                            }
                            if sr.clicked() {
                                let h = to_hsv(c);
                                st.hsv = [if h[1] < 1e-3 { st.hsv[0] } else { h[0] }, h[1], h[2]];
                                changed = true;
                            }
                        }
                    });
                }
            })
            .response;
        if changed {
            *self.color = with_alpha(hsv(st.hsv[0], st.hsv[1], st.hsv[2]), st.alpha);
        }
        st.last = *self.color;
        glowkit_core::set_state(ui, id, st);
        let mut resp = resp;
        if changed {
            resp.mark_changed();
        }
        resp
    }
}

impl Widget for ColorPicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        if !self.popover {
            return self.panel(ui, id);
        }
        let t = Theme::of(ui);
        let text = to_hex(*self.color);
        let tw = kit::text_width(ui, &text, 13.5);
        let (rect, mut resp) = ui.allocate_exact_size(vec2(tw + 58.0, 36.0), Sense::click());
        resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        let popup_id = id.with("popup");
        let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let hover = kit::anim(ui, id.with("h"), resp.hovered(), 0.12);
        let focus = kit::anim(ui, id.with("f"), open, 0.16);
        kit::field_chrome(ui.painter(), rect, &t, hover, focus, false, false);
        let sw = Rect::from_center_size(pos2(rect.min.x + 20.0, rect.center().y), vec2(20.0, 20.0));
        ui.painter().rect(sw, CornerRadius::same(5), *self.color, Stroke::new(1.0, with_alpha(t.foreground, 0.12)), StrokeKind::Inside);
        ui.painter().text(pos2(sw.max.x + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, text, FontId::monospace(13.0), t.foreground);
        if resp.clicked() {
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }
        let mut changed = false;
        let width = self.width;
        egui::Popup::new(popup_id, ui.ctx().clone(), rect, ui.layer_id())
            .open_memory(None::<egui::SetOpenCommand>)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(6.0)
            .frame(
                egui::Frame::new()
                    .fill(t.popover)
                    .stroke(t.border_stroke())
                    .corner_radius(t.corner())
                    .inner_margin(egui::Margin::same(12))
                    .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if t.dark { 100 } else { 30 }) }),
            )
            .show(|ui| {
                ui.set_width(width);
                changed = self.panel(ui, id.with("panel")).changed();
            });
        if changed {
            resp.mark_changed();
        }
        resp
    }
}
