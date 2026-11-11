use super::kit;
use super::LineIcon;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Ui, Widget};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::{paint, Theme};

/// What a [`MorphingIsland`] currently shows. Each view has its own size; the pill morphs
/// between them on a bouncy spring while the content cross-fades.
#[derive(Clone, Debug, PartialEq)]
pub enum IslandView {
    /// The small resting pill.
    Idle,
    /// A compact status: leading icon + short text + trailing text.
    Compact {
        /// Leading icon.
        icon: LineIcon,
        /// Icon tint.
        color: Color32,
        /// Text on the left.
        text: String,
        /// Text on the right.
        trailing: String,
    },
    /// An incoming call style row with two round action buttons.
    Call {
        /// Caller name.
        name: String,
        /// Secondary line ("mobile").
        subtitle: String,
    },
    /// Expanded media player with a progress bar and animated equalizer.
    Music {
        /// Track title.
        title: String,
        /// Artist name.
        artist: String,
        /// Playback position `0..=1`.
        progress: f32,
    },
    /// A notification card.
    Notification {
        /// Heading.
        title: String,
        /// Body text.
        body: String,
    },
}

impl IslandView {
    fn size(&self) -> egui::Vec2 {
        match self {
            IslandView::Idle => vec2(126.0, 36.0),
            IslandView::Compact { .. } => vec2(240.0, 36.0),
            IslandView::Call { .. } => vec2(360.0, 72.0),
            IslandView::Music { .. } => vec2(370.0, 150.0),
            IslandView::Notification { .. } => vec2(350.0, 84.0),
        }
    }
    fn key(&self) -> u8 {
        match self {
            IslandView::Idle => 0,
            IslandView::Compact { .. } => 1,
            IslandView::Call { .. } => 2,
            IslandView::Music { .. } => 3,
            IslandView::Notification { .. } => 4,
        }
    }
}

/// A black pill (in the spirit of a phone's "dynamic island") that morphs between an idle
/// pill, compact status, call, media player and notification layouts with springy size
/// changes and cross-faded content. Great for live activity / status surfaces.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::{IslandView, MorphingIsland, LineIcon};
/// ui.add(MorphingIsland::new(IslandView::Compact {
///     icon: LineIcon::Bell, color: egui::Color32::GREEN, text: "Ring".into(), trailing: "On".into(),
/// }));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MorphingIsland {
    view: IslandView,
    color: Option<Color32>,
    id_salt: Option<String>,
}

impl MorphingIsland {
    /// An island showing `view`.
    pub fn new(view: IslandView) -> Self {
        Self { view, color: None, id_salt: None }
    }
    /// Pill color (default near-black).
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
    /// Distinguish several islands in the same `Ui`.
    pub fn id_salt(mut self, salt: impl Into<String>) -> Self {
        self.id_salt = Some(salt.into());
        self
    }
}

impl Widget for MorphingIsland {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.scope_id().with(("glowkit_island", self.id_salt.as_deref().unwrap_or("")));
        let target = self.view.size();
        let w = glowkit_core::anim::spring(ui, id.with("w"), target.x, 260.0, 19.0);
        let h = glowkit_core::anim::spring(ui, id.with("h"), target.y, 260.0, 19.0);
        let max_h = 150.0;
        let (outer, resp) = ui.allocate_exact_size(vec2(ui.available_width(), max_h + 8.0), Sense::click());
        if !ui.is_rect_visible(outer) {
            return resp;
        }
        let rect = Rect::from_min_size(pos2(outer.center().x - w / 2.0, outer.min.y + 4.0), vec2(w, h));
        let bg = self.color.unwrap_or(hex("#050506"));
        let radius = (h / 2.0).min(36.0);
        let p = ui.painter();
        paint::soft_shadow(p, rect, radius, 24.0, vec2(0.0, 10.0), Color32::from_black_alpha(if t.dark { 120 } else { 60 }));
        p.rect_filled(rect, radius, bg);
        if t.dark {
            p.rect_stroke(rect, radius, egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.07)), egui::StrokeKind::Inside);
        }

        // cross-fade: content alpha rises once the size is close to the target
        let key = self.view.key();
        let prev_key: u8 = glowkit_core::state(ui, id.with("key"), || key);
        if prev_key != key {
            glowkit_core::set_state(ui, id.with("key"), key);
            glowkit_core::set_state(ui, id.with("fade").with("kit_follow"), 0.0f32);
        }
        let closeness = 1.0 - ((w - target.x).abs() / 60.0 + (h - target.y).abs() / 30.0).min(1.0);
        let fade = kit::follow_ctx(ui.ctx(), id.with("fade"), 1.0, 7.0).min(closeness.max(0.0));
        let mut pnt = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        pnt.multiply_opacity(fade);
        let white = Color32::WHITE;
        let dim = with_alpha(Color32::WHITE, 0.55);
        let time = glowkit_core::time(ui);
        let pad = 14.0;
        match &self.view {
            IslandView::Idle => {
                // camera dot
                pnt.circle_filled(pos2(rect.max.x - 22.0, rect.center().y), 5.0, hex("#111827"));
                pnt.circle_filled(pos2(rect.max.x - 22.0, rect.center().y), 2.0, hex("#1E3A8A"));
            }
            IslandView::Compact { icon, color, text, trailing } => {
                let cy = rect.center().y;
                icon.paint(&pnt, Rect::from_center_size(pos2(rect.min.x + 22.0, cy), vec2(16.0, 16.0)), *color);
                pnt.text(pos2(rect.min.x + 38.0, cy), Align2::LEFT_CENTER, text, FontId::proportional(13.0), white);
                pnt.text(pos2(rect.max.x - 16.0, cy), Align2::RIGHT_CENTER, trailing, FontId::proportional(13.0), *color);
            }
            IslandView::Call { name, subtitle } => {
                let cy = rect.center().y;
                let av = pos2(rect.min.x + pad + 22.0, cy);
                paint::gradient_circle(&pnt, av, 22.0, &[t.gradient[0], t.gradient[1]]);
                pnt.text(av, Align2::CENTER_CENTER, name.chars().next().unwrap_or('?').to_string(), FontId::proportional(18.0), white);
                pnt.text(pos2(av.x + 34.0, cy - 9.0), Align2::LEFT_CENTER, subtitle, FontId::proportional(12.0), dim);
                pnt.text(pos2(av.x + 34.0, cy + 9.0), Align2::LEFT_CENTER, name, FontId::proportional(15.5), white);
                let decline = pos2(rect.max.x - pad - 70.0, cy);
                let accept = pos2(rect.max.x - pad - 22.0, cy);
                pnt.circle_filled(decline, 21.0, hex("#EF4444"));
                LineIcon::X.paint_with(&pnt, Rect::from_center_size(decline, vec2(16.0, 16.0)), white, 2.0);
                let pulse = 1.0 + 0.06 * (time * 5.0).sin();
                pnt.circle_filled(accept, 21.0 * pulse, hex("#22C55E"));
                LineIcon::Phone.paint_with(&pnt, Rect::from_center_size(accept, vec2(16.0, 16.0)), white, 1.8);
                glowkit_core::animate(ui);
            }
            IslandView::Music { title, artist, progress } => {
                let art = Rect::from_min_size(rect.min + vec2(pad + 4.0, pad + 4.0), vec2(52.0, 52.0));
                paint::gradient_rect(&pnt, art, 12.0, 45.0, &t.gradient);
                LineIcon::Music.paint(&pnt, Rect::from_center_size(art.center(), vec2(22.0, 22.0)), white);
                pnt.text(pos2(art.max.x + 14.0, art.min.y + 14.0), Align2::LEFT_CENTER, title, FontId::proportional(15.5), white);
                pnt.text(pos2(art.max.x + 14.0, art.min.y + 36.0), Align2::LEFT_CENTER, artist, FontId::proportional(13.0), dim);
                // equalizer
                for i in 0..4 {
                    let hh = 6.0 + 14.0 * (0.5 + 0.5 * (time * (5.0 + i as f32 * 1.7) + i as f32).sin());
                    let x = rect.max.x - pad - 34.0 + i as f32 * 7.0;
                    let r = Rect::from_min_max(pos2(x, art.center().y + 10.0 - hh), pos2(x + 4.0, art.center().y + 10.0));
                    pnt.rect_filled(r, 2.0, t.gradient[1]);
                }
                // progress
                let py = art.max.y + 20.0;
                let bar = Rect::from_min_max(pos2(rect.min.x + pad + 4.0, py), pos2(rect.max.x - pad - 4.0, py + 4.0));
                pnt.rect_filled(bar, 2.0, with_alpha(white, 0.2));
                let filled = Rect::from_min_max(bar.min, pos2(bar.min.x + bar.width() * progress.clamp(0.0, 1.0), bar.max.y));
                pnt.rect_filled(filled, 2.0, white);
                let total = 214.0;
                let cur = total * progress;
                pnt.text(pos2(bar.min.x, py + 14.0), Align2::LEFT_CENTER, format!("{}:{:02}", cur as u32 / 60, cur as u32 % 60), FontId::proportional(11.0), dim);
                pnt.text(pos2(bar.max.x, py + 14.0), Align2::RIGHT_CENTER, format!("-{}:{:02}", (total - cur) as u32 / 60, (total - cur) as u32 % 60), FontId::proportional(11.0), dim);
                // controls
                let cy = py + 30.0;
                let cx = rect.center().x;
                LineIcon::ChevronLeft.paint_with(&pnt, Rect::from_center_size(pos2(cx - 50.0, cy), vec2(18.0, 18.0)), white, 2.2);
                pnt.rect_filled(Rect::from_center_size(pos2(cx - 4.5, cy), vec2(4.0, 16.0)), 1.5, white);
                pnt.rect_filled(Rect::from_center_size(pos2(cx + 4.5, cy), vec2(4.0, 16.0)), 1.5, white);
                LineIcon::ChevronRight.paint_with(&pnt, Rect::from_center_size(pos2(cx + 50.0, cy), vec2(18.0, 18.0)), white, 2.2);
                glowkit_core::animate(ui);
            }
            IslandView::Notification { title, body } => {
                let ic = Rect::from_min_size(rect.min + vec2(pad + 2.0, pad + 4.0), vec2(40.0, 40.0));
                pnt.rect_filled(ic, 10.0, t.accent);
                LineIcon::Mail.paint(&pnt, Rect::from_center_size(ic.center(), vec2(20.0, 20.0)), t.accent_foreground);
                pnt.text(pos2(ic.max.x + 12.0, ic.min.y + 10.0), Align2::LEFT_CENTER, title, FontId::proportional(14.5), white);
                let g = pnt.layout(body.clone(), FontId::proportional(12.5), dim, rect.max.x - ic.max.x - 12.0 - pad - 30.0);
                pnt.galley(pos2(ic.max.x + 12.0, ic.min.y + 20.0), g, dim);
                pnt.text(pos2(rect.max.x - pad - 4.0, ic.min.y + 10.0), Align2::RIGHT_CENTER, "now", FontId::proportional(11.5), dim);
            }
        }
        resp
    }
}
