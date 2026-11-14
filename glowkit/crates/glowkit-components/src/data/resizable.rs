//! Resizable split panels with draggable handles.

use crate::inputs::kit;
use egui::{pos2, vec2, CornerRadius, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Split direction of a [`ResizablePanels`] group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SplitDirection {
    /// Panels side by side, handles are vertical bars.
    #[default]
    Horizontal,
    /// Panels stacked, handles are horizontal bars.
    Vertical,
}

/// A group of panels separated by draggable handles, like shadcn's "Resizable".
///
/// Sizes are caller-owned fractions (`&mut [f32]`, one per panel, summing to 1). Dragging a handle
/// moves space between its two neighbours while respecting per-panel minimum and maximum fractions.
/// Groups nest: call another `ResizablePanels` inside a panel. Double-click a handle to reset the
/// pair to an even split.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::ResizablePanels;
/// let mut sizes = [0.3, 0.7];
/// ResizablePanels::horizontal(&mut sizes).height(240.0).min(0.15).show(ui, |ui, i| {
///     ui.label(format!("Panel {i}"));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct ResizablePanels<'a> {
    sizes: &'a mut [f32],
    direction: SplitDirection,
    height: Option<f32>,
    width: Option<f32>,
    min: f32,
    max: f32,
    mins: Option<&'a [f32]>,
    maxs: Option<&'a [f32]>,
    grip: bool,
    frame: bool,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> ResizablePanels<'a> {
    /// Side-by-side panels.
    pub fn horizontal(sizes: &'a mut [f32]) -> Self {
        Self::new(sizes, SplitDirection::Horizontal)
    }
    /// Stacked panels.
    pub fn vertical(sizes: &'a mut [f32]) -> Self {
        Self::new(sizes, SplitDirection::Vertical)
    }
    /// Panels split in `direction`.
    pub fn new(sizes: &'a mut [f32], direction: SplitDirection) -> Self {
        Self {
            sizes,
            direction,
            height: None,
            width: None,
            min: 0.08,
            max: 1.0,
            mins: None,
            maxs: None,
            grip: true,
            frame: true,
            id_salt: None,
        }
    }
    /// Total height (default: available height, at least 120).
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
    /// Total width (default: available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Minimum fraction for every panel (default 0.08).
    pub fn min(mut self, f: f32) -> Self {
        self.min = f;
        self
    }
    /// Maximum fraction for every panel (default 1).
    pub fn max(mut self, f: f32) -> Self {
        self.max = f;
        self
    }
    /// Per-panel minimum fractions (overrides [`ResizablePanels::min`]).
    pub fn min_sizes(mut self, m: &'a [f32]) -> Self {
        self.mins = Some(m);
        self
    }
    /// Per-panel maximum fractions (overrides [`ResizablePanels::max`]).
    pub fn max_sizes(mut self, m: &'a [f32]) -> Self {
        self.maxs = Some(m);
        self
    }
    /// Show a small grip pill on each handle (default on).
    pub fn grip(mut self, g: bool) -> Self {
        self.grip = g;
        self
    }
    /// Draw the rounded outer frame (default on). Turn off for nested groups.
    pub fn frame(mut self, f: bool) -> Self {
        self.frame = f;
        self
    }
    /// Stable id (needed when several groups share a parent).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }

    fn lo(&self, i: usize) -> f32 {
        self.mins.and_then(|m| m.get(i).copied()).unwrap_or(self.min)
    }
    fn hi(&self, i: usize) -> f32 {
        self.maxs.and_then(|m| m.get(i).copied()).unwrap_or(self.max)
    }

    /// Show the group; `panel(ui, index)` fills each panel. Returns the response of the whole group
    /// (marked changed while a handle is dragged).
    pub fn show(self, ui: &mut Ui, mut panel: impl FnMut(&mut Ui, usize)) -> Response {
        let t = Theme::of(ui);
        let n = self.sizes.len();
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let h = self.height.unwrap_or_else(|| ui.available_height().max(120.0));
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        if n == 0 {
            return resp;
        }
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or(resp.id).with("resizable");
        // normalise
        let sum: f32 = self.sizes.iter().map(|s| s.max(0.0)).sum();
        if sum <= 0.0 {
            self.sizes.iter_mut().for_each(|s| *s = 1.0 / n as f32);
        } else if (sum - 1.0).abs() > 1e-4 {
            self.sizes.iter_mut().for_each(|s| *s = s.max(0.0) / sum);
        }
        let horiz = self.direction == SplitDirection::Horizontal;
        let total = if horiz { rect.width() } else { rect.height() };
        let cr = t.corner();
        if self.frame && ui.is_rect_visible(rect) {
            ui.painter().rect(rect, cr, t.card, Stroke::NONE, StrokeKind::Inside);
        }
        // handles first (interaction), using this frame's sizes
        let mut edges = Vec::with_capacity(n + 1);
        let mut acc = 0.0;
        edges.push(0.0);
        for s in self.sizes.iter() {
            acc += s;
            edges.push(acc);
        }
        let mut handle_vis = Vec::new();
        #[allow(clippy::needless_range_loop)]
        for k in 1..n {
            let pos = edges[k] * total;
            let hit = if horiz {
                Rect::from_center_size(pos2(rect.min.x + pos, rect.center().y), vec2(10.0, rect.height()))
            } else {
                Rect::from_center_size(pos2(rect.center().x, rect.min.y + pos), vec2(rect.width(), 10.0))
            };
            let hr = ui.interact(hit, id.with(("handle", k)), Sense::click_and_drag()).on_hover_cursor(if horiz {
                CursorIcon::ResizeColumn
            } else {
                CursorIcon::ResizeRow
            });
            let delta = if horiz { hr.drag_delta().x } else { hr.drag_delta().y } / total.max(1.0);
            let (a, b) = (k - 1, k);
            if hr.double_clicked() {
                let pair = self.sizes[a] + self.sizes[b];
                self.sizes[a] = pair / 2.0;
                self.sizes[b] = pair / 2.0;
                resp.mark_changed();
            } else if delta != 0.0 {
                let pair = self.sizes[a] + self.sizes[b];
                let lo = self.lo(a).max(pair - self.hi(b));
                let hi = self.hi(a).min(pair - self.lo(b));
                let na = (self.sizes[a] + delta).clamp(lo.min(hi), hi.max(lo));
                self.sizes[a] = na;
                self.sizes[b] = pair - na;
                resp.mark_changed();
            }
            let active = hr.dragged();
            let k_h = kit::anim(ui, id.with(("hh", k)), hr.hovered() || active, 0.15);
            handle_vis.push((k, k_h, active));
        }
        // recompute edges after drag
        let mut acc = 0.0;
        for (i, s) in self.sizes.iter().enumerate() {
            acc += s;
            edges[i + 1] = acc;
        }
        // panels
        for i in 0..n {
            let (a, b) = (edges[i] * total, edges[i + 1] * total);
            let pr = if horiz {
                Rect::from_min_max(pos2(rect.min.x + a, rect.min.y), pos2(rect.min.x + b, rect.max.y))
            } else {
                Rect::from_min_max(pos2(rect.min.x, rect.min.y + a), pos2(rect.max.x, rect.min.y + b))
            };
            let inner = pr.shrink2(if horiz { vec2(0.5, 0.0) } else { vec2(0.0, 0.5) });
            let mut child = ui.new_child(
                UiBuilder::new()
                    .id_salt(id.with(("panel", i)))
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(inner.intersect(ui.clip_rect()));
            panel(&mut child, i);
        }
        // handle visuals on top
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            for (k, kh, active) in handle_vis {
                let pos = edges[k] * total;
                let line_c = t.border.lerp_to_gamma(t.ring, kh);
                let (a, b) = if horiz {
                    (pos2(rect.min.x + pos, rect.min.y), pos2(rect.min.x + pos, rect.max.y))
                } else {
                    (pos2(rect.min.x, rect.min.y + pos), pos2(rect.max.x, rect.min.y + pos))
                };
                if kh > 0.0 {
                    p.line_segment([a, b], Stroke::new(1.0 + 2.0 * kh, with_alpha(t.ring, 0.35 * kh)));
                }
                p.line_segment([a, b], Stroke::new(1.0, line_c));
                if self.grip {
                    let c = a.lerp(b, 0.5);
                    let gs = if horiz { vec2(12.0, 22.0) } else { vec2(22.0, 12.0) };
                    let gr = Rect::from_center_size(c, gs);
                    let grc = CornerRadius::same(4);
                    p.rect(
                        gr,
                        grc,
                        if active { t.secondary } else { t.background.lerp_to_gamma(t.secondary, 0.6 + 0.4 * kh) },
                        Stroke::new(1.0, t.border.lerp_to_gamma(t.ring, kh * 0.8)),
                        StrokeKind::Inside,
                    );
                    let dot = t.muted_foreground.lerp_to_gamma(t.foreground, kh);
                    for j in -1..=1 {
                        for m in [-1.0f32, 1.0] {
                            let off = if horiz { vec2(m * 2.0, j as f32 * 4.5) } else { vec2(j as f32 * 4.5, m * 2.0) };
                            p.circle_filled(c + off, 1.0, dot);
                        }
                    }
                }
            }
            if self.frame {
                p.rect_stroke(rect, cr, Stroke::new(1.0, t.border), StrokeKind::Inside);
            }
        }
        resp
    }
}
