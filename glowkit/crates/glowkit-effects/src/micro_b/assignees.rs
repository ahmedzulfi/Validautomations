use super::common::{avatar, cr, hairline, icon, idle_for, shadow_color, surface, surface_hover, MicroIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Widget};
use glowkit_core::anim::spring;
use glowkit_core::color::with_alpha;
use glowkit_core::{paint, Theme};

/// A person that can be assigned in an [`AssigneePicker`].
#[derive(Clone, Debug, PartialEq)]
pub struct Assignee {
    /// Display name.
    pub name: String,
    /// Role or team, shown under the name.
    pub role: String,
    /// Two-stop gradient for the generated avatar.
    pub colors: [Color32; 2],
}

impl Assignee {
    /// A new person.
    pub fn new(name: impl Into<String>, role: impl Into<String>, colors: [Color32; 2]) -> Self {
        Self { name: name.into(), role: role.into(), colors }
    }
}

/// How the chosen avatars are arranged in the pill of an [`AssigneePicker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AvatarLayout {
    /// Overlapping stack (default).
    #[default]
    Stack,
    /// Side by side.
    Row,
    /// Small avatars in two rows.
    Grid,
}

/// Assign people from a list and watch their faces stack up in the pill above it. Ticking a row
/// pops the avatar into the pill on a spring while the pill widens to make room; unticking
/// shrinks it back out. Press the pill to fold the list away.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_effects::{Assignee, AssigneePicker};
/// let people = vec![Assignee::new("Ada", "Design", [glowkit_core::hex("#F472B6"), glowkit_core::hex("#8B5CF6")])];
/// let mut chosen = vec![true];
/// ui.add(AssigneePicker::new(&people, &mut chosen));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AssigneePicker<'a> {
    people: &'a [Assignee],
    selected: &'a mut Vec<bool>,
    layout: AvatarLayout,
    overlap: f32,
    width: f32,
    corner: f32,
    dark: bool,
    stroke: bool,
    autoplay: bool,
}

impl<'a> AssigneePicker<'a> {
    /// A picker over `people`; `selected[i]` says whether person `i` is assigned.
    pub fn new(people: &'a [Assignee], selected: &'a mut Vec<bool>) -> Self {
        Self { people, selected, layout: AvatarLayout::Stack, overlap: 10.0, width: 236.0, corner: 20.0, dark: false, stroke: true, autoplay: false }
    }
    /// Avatar arrangement in the pill.
    pub fn layout(mut self, l: AvatarLayout) -> Self {
        self.layout = l;
        self
    }
    /// How much stacked avatars overlap, in points.
    pub fn overlap(mut self, px: f32) -> Self {
        self.overlap = px.clamp(0.0, 20.0);
        self
    }
    /// List width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(160.0);
        self
    }
    /// Corner radius of the list.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Dark surfaces.
    pub fn dark(mut self, on: bool) -> Self {
        self.dark = on;
        self
    }
    /// Hairline outlines (default on).
    pub fn stroke(mut self, on: bool) -> Self {
        self.stroke = on;
        self
    }
    /// Toggle people on its own while idle (for showcases; off by default).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl Widget for AssigneePicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let n = self.people.len();
        self.selected.resize(n, false);
        let row_h = 46.0;
        let pad = 6.0;
        let list_h = pad * 2.0 + row_h * n as f32;
        let pill_h = 36.0;
        let size = vec2(self.width, pill_h + 10.0 + list_h);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id;
        let now = ui.input(|i| i.time);
        let open: bool = glowkit_core::state(ui, id.with("open"), || true);
        let openk = spring(ui, id.with("openk"), if open { 1.0 } else { 0.0 }, 260.0, 24.0);
        // avatar slots in the pill
        let av_r = if self.layout == AvatarLayout::Grid { 7.5 } else { 11.0 };
        let step = match self.layout {
            AvatarLayout::Stack => av_r * 2.0 - self.overlap,
            AvatarLayout::Row => av_r * 2.0 + 4.0,
            AvatarLayout::Grid => av_r * 2.0 + 2.0,
        };
        let ks: Vec<f32> = (0..n).map(|i| spring(ui, id.with(("av", i)), if self.selected[i] { 1.0 } else { 0.0 }, 380.0, 19.0)).collect();
        let count: f32 = ks.iter().map(|k| k.clamp(0.0, 1.0)).sum();
        let slots = if self.layout == AvatarLayout::Grid { (count / 2.0).ceil() } else { count };
        let content_w = if count < 0.01 { 0.0 } else { (slots - 1.0).max(0.0) * step + av_r * 2.0 };
        let pill_w = 12.0 + content_w + if count > 0.01 { 8.0 } else { 0.0 } + 22.0 + 6.0;
        let pill = Rect::from_min_size(rect.min, vec2(pill_w.max(40.0), pill_h));
        let p_resp = ui.interact(pill, id.with("pill"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if p_resp.clicked() {
            glowkit_core::set_state(ui, id.with("open"), !open);
        }
        let list = Rect::from_min_size(pos2(rect.min.x, pill.max.y + 10.0), vec2(self.width, list_h));
        let mut hovered_row = None;
        let idle = idle_for(ui, id, ui.rect_contains_pointer(rect));
        for i in 0..n {
            let rr = Rect::from_min_size(pos2(list.min.x + pad, list.min.y + pad + i as f32 * row_h), vec2(list.width() - pad * 2.0, row_h));
            if openk < 0.9 {
                break;
            }
            let r = ui.interact(rr, id.with(("row", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.hovered() {
                hovered_row = Some(i);
            }
            if r.clicked() {
                self.selected[i] = !self.selected[i];
                response.mark_changed();
            }
        }
        if self.autoplay && idle > 2.0 {
            glowkit_core::animate(ui);
            let last: f64 = glowkit_core::state(ui, id.with("auto"), || now);
            if now - last > 1.4 && n > 0 {
                glowkit_core::set_state(ui, id.with("auto"), now);
                let i = ((now * 3.7) as usize * 7 + 3) % n;
                self.selected[i] = !self.selected[i];
                response.mark_changed();
            }
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let (bg, fg, sub) = if self.dark {
            (t.primary, t.primary_foreground, with_alpha(t.primary_foreground, 0.55))
        } else {
            (surface(&t), t.foreground, t.muted_foreground)
        };
        let line = if self.stroke { hairline(&t) } else { Stroke::NONE };
        // pill
        paint::soft_shadow(painter, pill, pill_h / 2.0, 12.0, vec2(0.0, 3.0), shadow_color(&t, 0.5));
        painter.rect(pill, cr(pill_h / 2.0), if p_resp.hovered() && !self.dark { surface_hover(&t).lerp_to_gamma(bg, 0.5) } else { bg }, line, egui::StrokeKind::Inside);
        let mut slot = 0.0;
        let mut j = 0usize;
        let x0 = pill.min.x + 12.0 + av_r;
        for (i, p) in self.people.iter().enumerate() {
            let k = ks[i];
            if k <= 0.01 {
                continue;
            }
            let c = if self.layout == AvatarLayout::Grid {
                let (col, row) = ((j / 2) as f32, (j % 2) as f32);
                pos2(x0 + col * step, pill.center().y + (row - 0.5) * (av_r * 2.0 + 1.0))
            } else {
                pos2(x0 + slot * step, pill.center().y)
            };
            avatar(painter, c, av_r * k.max(0.0), p.colors, Some(bg));
            slot += k.clamp(0.0, 1.0);
            j += 1;
        }
        let chev_x = pill.max.x - 17.0;
        let flip = 1.0 - 2.0 * openk;
        let cc = pos2(chev_x, pill.center().y);
        let s = 4.0;
        painter.add(egui::Shape::line(
            vec![cc + vec2(-s, s * 0.5 * flip), cc + vec2(0.0, -s * 0.5 * flip), cc + vec2(s, s * 0.5 * flip)],
            Stroke::new(1.4, sub),
        ));
        // list folds down from under the pill
        if openk > 0.01 {
            let h = list.height() * openk.clamp(0.0, 1.03);
            let lr = Rect::from_min_size(list.min, vec2(list.width(), h));
            let a = openk.clamp(0.0, 1.0);
            paint::soft_shadow(painter, lr, self.corner, 18.0, vec2(0.0, 5.0), shadow_color(&t, 0.6 * a));
            painter.rect(lr, cr(self.corner.min(h / 2.0)), with_alpha(bg, a), line, egui::StrokeKind::Inside);
            let clip = painter.with_clip_rect(lr.shrink(1.0));
            let hy = spring(ui, id.with("hy"), hovered_row.unwrap_or(0) as f32, 500.0, 34.0);
            let ha = spring(ui, id.with("ha"), if hovered_row.is_some() { 1.0 } else { 0.0 }, 400.0, 30.0);
            if ha > 0.01 {
                let hr = Rect::from_min_size(pos2(list.min.x + pad, list.min.y + pad + hy * row_h), vec2(list.width() - pad * 2.0, row_h));
                clip.rect_filled(hr, cr((self.corner - pad).max(8.0)), with_alpha(if self.dark { with_alpha(fg, 0.1) } else { surface_hover(&t) }, ha));
            }
            for (i, p) in self.people.iter().enumerate() {
                let y = list.min.y + pad + (i as f32 + 0.5) * row_h;
                let x = list.min.x + pad + 10.0;
                avatar(&clip, pos2(x + 13.0, y), 13.0, p.colors, None);
                clip.text(pos2(x + 34.0, y - 8.0), Align2::LEFT_CENTER, &p.name, FontId::proportional(13.5), fg);
                clip.text(pos2(x + 34.0, y + 9.0), Align2::LEFT_CENTER, &p.role, FontId::proportional(11.5), sub);
                let k = ks[i].clamp(0.0, 1.0);
                let b = Rect::from_center_size(pos2(list.max.x - pad - 22.0, y), vec2(17.0, 17.0));
                clip.rect(b, cr(5.0), with_alpha(fg, k), Stroke::new(1.2, with_alpha(sub, 0.6 * (1.0 - k))), egui::StrokeKind::Inside);
                if k > 0.05 {
                    icon(&clip, MicroIcon::Check, b.center(), 10.0 * (0.6 + 0.4 * k), with_alpha(bg, k), 1.6);
                }
            }
        }
        if (0..n).any(|i| (ks[i] - self.selected[i] as u8 as f32).abs() > 0.001) {
            glowkit_core::animate(ui);
        }
        response.union(p_resp)
    }
}
