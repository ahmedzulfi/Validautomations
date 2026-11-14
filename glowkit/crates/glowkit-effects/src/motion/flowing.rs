use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, InnerResponse, Rect, Sense, Stroke, Ui};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A large text menu where hovering a row floods it with a contrasting band (entering from
/// the edge the pointer came from) carrying a scrolling marquee of the label.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let r = glowkit_effects::FlowingMenu::new(&[("🌊", "Mojave"), ("🌲", "Sonoma")]).show(ui);
/// if let Some(i) = r.inner { /* row i clicked */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct FlowingMenu {
    items: Vec<(String, String)>,
    row_height: f32,
    font_size: f32,
    speed: f32,
    band: Option<Color32>,
    band_text: Option<Color32>,
    autoplay: bool,
}

impl FlowingMenu {
    /// Rows of `(icon, label)`.
    pub fn new<A: AsRef<str>, B: AsRef<str>>(items: &[(A, B)]) -> Self {
        Self {
            items: items.iter().map(|(a, b)| (a.as_ref().to_owned(), b.as_ref().to_owned())).collect(),
            row_height: 84.0,
            font_size: 30.0,
            speed: 70.0,
            band: None,
            band_text: None,
            autoplay: false,
        }
    }
    /// Height of each row.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h.max(24.0);
        self
    }
    /// Label font size.
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s.max(8.0);
        self
    }
    /// Marquee speed in points per second.
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    /// Band and band-text colors. Default: inverted theme foreground / background.
    pub fn colors(mut self, band: Color32, text: Color32) -> Self {
        self.band = Some(band);
        self.band_text = Some(text);
        self
    }
    /// Hover rows on its own while the pointer is away.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the menu. `inner` is the row clicked this frame.
    pub fn show(self, ui: &mut Ui) -> InnerResponse<Option<usize>> {
        let t = Theme::of(ui);
        let n = self.items.len();
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), self.row_height * n as f32), Sense::click());
        let id = response.id;
        let rows: Vec<Rect> = (0..n)
            .map(|i| {
                Rect::from_min_size(
                    rect.min + vec2(0.0, i as f32 * self.row_height),
                    vec2(rect.width(), self.row_height),
                )
            })
            .collect();
        let pointer = response.hover_pos();
        let mut hovered = pointer.and_then(|p| rows.iter().position(|r| r.contains(p)));
        let mut sim_y = None;
        if hovered.is_none() && self.autoplay && pointer.is_none() && n > 0 {
            let now = glowkit_core::time(ui);
            let slot = (now / 1.5).floor() as usize;
            if slot % 4 != 3 {
                let i = (slot * 3 / 2) % n;
                hovered = Some(i);
                sim_y = Some(if slot % 2 == 0 { rows[i].min.y } else { rows[i].max.y });
            }
            glowkit_core::animate(ui);
        }
        if hovered.is_some() && pointer.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let clicked = if response.clicked() { hovered } else { None };
        let painter = ui.painter_at(rect);
        let band = self.band.unwrap_or(t.foreground);
        let band_text = self.band_text.unwrap_or(t.background);
        let font = FontId::proportional(self.font_size);
        let now = glowkit_core::time(ui);
        for (i, r) in rows.iter().enumerate() {
            let on = hovered == Some(i);
            // remember which edge the pointer entered from
            let from_top = glowkit_core::with_state(
                ui,
                id.with(("edge", i)),
                || (false, true),
                |(was, top)| {
                    if on && !*was {
                        let y = pointer.map(|p| p.y).or(sim_y).unwrap_or(r.center().y);
                        *top = y < r.center().y;
                    }
                    if !on && *was {
                        let y = pointer.map(|p| p.y).unwrap_or(r.center().y);
                        *top = y < r.center().y;
                    }
                    *was = on;
                    *top
                },
            );
            let reveal = glowkit_core::anim::spring(ui, id.with(("rv", i)), if on { 1.0 } else { 0.0 }, 260.0, 30.0)
                .clamp(0.0, 1.0);
            if i > 0 {
                painter.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, t.border));
            }
            let (icon, label) = &self.items[i];
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                with_alpha(t.foreground, (1.0 - reveal * 3.0).max(0.0)),
            );
            if reveal > 0.001 {
                let h = r.height() * reveal;
                let bandr = if from_top {
                    Rect::from_min_size(r.min, vec2(r.width(), h))
                } else {
                    Rect::from_min_max(pos2(r.min.x, r.max.y - h), r.max)
                };
                let clip = painter.with_clip_rect(bandr);
                clip.rect_filled(bandr, CornerRadius::ZERO, band);
                // marquee of label + icon pills
                let unit_text = label.clone();
                let g = clip.layout_no_wrap(unit_text.clone(), font.clone(), band_text);
                let pill_w = r.height() * 0.9;
                let unit = g.size().x + 28.0 + pill_w + 28.0;
                let off = (now * self.speed).rem_euclid(unit);
                let copies = (r.width() / unit).ceil() as usize + 2;
                // slide the content in with the band so it doesn't look pasted on
                let dy = if from_top { -(1.0 - reveal) * r.height() * 0.5 } else { (1.0 - reveal) * r.height() * 0.5 };
                let cy = r.center().y + dy;
                for k in 0..copies {
                    let x = r.min.x - off + k as f32 * unit;
                    clip.galley(pos2(x, cy - g.size().y / 2.0), g.clone(), band_text);
                    let pill = Rect::from_center_size(
                        pos2(x + g.size().x + 28.0 + pill_w / 2.0, cy),
                        vec2(pill_w, r.height() * 0.56),
                    );
                    let pc = t.chart[(i + k) % t.chart.len()];
                    clip.rect_filled(pill, CornerRadius::same(99), pc);
                    clip.text(
                        pill.center(),
                        Align2::CENTER_CENTER,
                        icon,
                        FontId::proportional(r.height() * 0.3),
                        Color32::WHITE,
                    );
                }
                ui.ctx().request_repaint();
            }
        }
        painter.line_segment(
            [rect.left_bottom() - vec2(0.0, 0.5), rect.right_bottom() - vec2(0.0, 0.5)],
            Stroke::new(1.0, t.border),
        );
        painter.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, t.border));
        InnerResponse::new(clicked, response)
    }
}
