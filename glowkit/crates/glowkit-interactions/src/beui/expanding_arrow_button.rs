use super::common::{cr, idle_for, lerp, text_width};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Ui};
use glowkit_core::anim::spring;
use glowkit_core::color::{contrast_text, hex, shade, with_alpha};
use glowkit_core::Theme;

/// A call-to-action whose arrow swallows the button. At rest it is a dark pill with a bright
/// square holding a dotted arrow; hover and the square morphs (width and corner radius on a
/// spring) until it fills the whole pill, the label slides away, and a train of dotted chevrons
/// marches across in a travelling wave.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ExpandingArrowButton;
/// if ui.add(ExpandingArrowButton::new("Book a demo")).clicked() {
///     // go
/// }
/// ```
pub struct ExpandingArrowButton {
    label: String,
    accent: Color32,
    body: Option<Color32>,
    height: f32,
    width: Option<f32>,
    autoplay: bool,
}

impl ExpandingArrowButton {
    /// A button labelled `label` with a lime arrow block.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), accent: hex("#C4F135"), body: None, height: 60.0, width: None, autoplay: true }
    }
    /// Color of the arrow block (and of the whole button when hovered).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = c;
        self
    }
    /// Color of the pill (default: near-black ink).
    pub fn body(mut self, c: Color32) -> Self {
        self.body = Some(c);
        self
    }
    /// Height of the button.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.clamp(32.0, 96.0);
        self
    }
    /// Fixed width (default: fits the label).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Expand on its own now and then while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// A chevron drawn as five dots, centered on `c`.
fn dot_chevron(painter: &Painter, c: Pos2, s: f32, color: Color32) {
    for (x, y) in [(-0.5, -0.8), (0.0, -0.4), (0.5, 0.0), (0.0, 0.4), (-0.5, 0.8)] {
        painter.circle_filled(c + vec2(x, y) * s, s * 0.16 + 0.6, color);
    }
}

impl egui::Widget for ExpandingArrowButton {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let h = self.height;
        let fs = h * 0.29;
        let w = self.width.unwrap_or_else(|| text_width(ui.painter(), &self.label, fs) + h * 2.2);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let id = resp.id;
        let idle = idle_for(ui, id, resp.hovered());
        let mut hot = resp.hovered();
        if self.autoplay && idle > 1.5 {
            let ph = (idle - 1.5 + (id.value() % 7) as f32 * 0.3) % 3.6;
            hot = ph > 1.2 && ph < 3.0;
            glowkit_core::animate(ui);
        }
        let press = spring(ui, id.with("press"), if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 600.0, 26.0);
        let k = spring(ui, id.with("k"), if hot { 1.0 } else { 0.0 }, 260.0, 21.0);
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = ui.painter().clone();
        let rect = rect.shrink(press * 2.0);
        let body = self.body.unwrap_or(if t.dark { shade(t.muted, 0.09) } else { hex("#0B0B0F") });
        let pad = h * 0.08;
        painter.rect_filled(rect, cr(h * 0.28), body);
        // the block: a square that morphs into the whole inner pill
        let sq = h - pad * 2.0;
        let inner_w = rect.width() - pad * 2.0;
        let bw = lerp(sq, inner_w, k.max(0.0));
        let block = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + pad), vec2(bw.max(0.0), sq));
        let br = lerp(h * 0.2, h * 0.22, k);
        painter.rect_filled(block, cr(br), self.accent);
        // label slides right and fades out as the block arrives
        let label_x = rect.min.x + pad + sq + h * 0.4 + k * 30.0;
        let la = (1.0 - k * 1.8).clamp(0.0, 1.0);
        if la > 0.0 {
            let clip = painter.with_clip_rect(Rect::from_min_max(pos2(block.max.x, rect.min.y), rect.max));
            clip.text(
                pos2(label_x, rect.center().y),
                Align2::LEFT_CENTER,
                &self.label,
                FontId::proportional(fs),
                with_alpha(Color32::WHITE, la),
            );
        }
        // the dotted chevrons: one at rest, a marching train when expanded
        let ink = contrast_text(self.accent);
        let time = glowkit_core::time(ui);
        let s = sq * 0.22;
        let clip = painter.with_clip_rect(block);
        if k < 0.98 {
            dot_chevron(&clip, block.min + vec2(sq / 2.0 + k * 6.0, sq / 2.0), s, with_alpha(ink, 1.0 - k));
        }
        if k > 0.02 {
            let spacing = sq * 0.62;
            let count = (inner_w / spacing).floor() as usize;
            let x0 = block.min.x + sq * 0.5;
            for i in 0..count {
                let x = x0 + i as f32 * spacing;
                if x > block.max.x - s {
                    break;
                }
                // a wave of brightness running left to right
                let wave = ((time * 3.0 - i as f32 * 0.55).sin() * 0.5 + 0.5).powf(1.5);
                let a = k * (0.25 + 0.75 * wave) * (1.0 - i as f32 / count.max(1) as f32 * 0.5);
                dot_chevron(&clip, pos2(x, block.center().y), s, with_alpha(ink, a));
            }
            glowkit_core::animate(ui);
        }
        resp
    }
}
