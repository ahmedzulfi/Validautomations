//! A themed scroll container with a thin overlay scrollbar.

use egui::scroll_area::{DragScroll, ScrollBarVisibility, ScrollSource};
use egui::{pos2, vec2, Color32, CornerRadius, Rect, Sense, Ui, Vec2};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// Persistent per-area state for the overlay bars.
#[derive(Clone, Copy, Debug, Default)]
struct BarState {
    last_offset: Vec2,
    last_activity: f64,
    /// Offset and pointer position at drag start, per axis.
    drag: [Option<(f32, f32)>; 2],
    /// Offset to apply on the next frame (set while dragging a thumb).
    pending: [Option<f32>; 2],
}

/// When the overlay scrollbar is visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollbarMode {
    /// Fade in while hovering or scrolling, fade out after a short delay (default).
    #[default]
    Auto,
    /// Always visible when the content overflows.
    Always,
    /// Never painted (wheel and touch scrolling still work).
    Hidden,
}

/// A themed scroll container: wraps `egui::ScrollArea`, hides the stock bars and paints a thin,
/// rounded overlay thumb that fades in on hover or scroll, widens when the pointer is over the
/// track, and can be dragged. Edges can fade into the background to hint at more content.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_components::StyledScrollArea;
/// StyledScrollArea::vertical().max_height(240.0).show(ui, |ui| {
///     for i in 0..50 {
///         ui.label(format!("Row {i}"));
///     }
/// });
/// ```
#[must_use = "call `.show(ui, |ui| ...)`"]
pub struct StyledScrollArea {
    dirs: [bool; 2],
    max_height: Option<f32>,
    max_width: Option<f32>,
    mode: ScrollbarMode,
    thickness: f32,
    color: Option<Color32>,
    fade_edges: Option<Color32>,
    id_salt: Option<egui::IdSalt>,
    stick_to_bottom: bool,
    drag_to_scroll: bool,
}

impl StyledScrollArea {
    /// Scroll vertically.
    pub fn vertical() -> Self {
        Self::new([false, true])
    }
    /// Scroll horizontally.
    pub fn horizontal() -> Self {
        Self::new([true, false])
    }
    /// Scroll both ways.
    pub fn both() -> Self {
        Self::new([true, true])
    }
    fn new(dirs: [bool; 2]) -> Self {
        Self {
            dirs,
            max_height: None,
            max_width: None,
            mode: ScrollbarMode::Auto,
            thickness: 6.0,
            color: None,
            fade_edges: None,
            id_salt: None,
            stick_to_bottom: false,
            drag_to_scroll: true,
        }
    }
    /// Maximum viewport height. Taller content scrolls, and the viewport keeps this height even in
    /// tight layouts.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
    /// Maximum viewport width (kept even in tight layouts when the content is wider).
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }
    /// When the scrollbar shows.
    pub fn scrollbar(mut self, m: ScrollbarMode) -> Self {
        self.mode = m;
        self
    }
    /// Thumb thickness at rest (grows by 50% on hover; default 6).
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t;
        self
    }
    /// Thumb color (default: theme muted foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    /// Fade the content into `bg` at the scrollable edges.
    pub fn fade_edges(mut self, bg: Color32) -> Self {
        self.fade_edges = Some(bg);
        self
    }
    /// Stable id (needed when several areas share a parent).
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
    /// Keep scrolled to the bottom as content grows (chat logs).
    pub fn stick_to_bottom(mut self, s: bool) -> Self {
        self.stick_to_bottom = s;
        self
    }
    /// Allow dragging the content itself to scroll on touch screens (default on).
    pub fn drag_to_scroll(mut self, d: bool) -> Self {
        self.drag_to_scroll = d;
        self
    }

    /// Show the area. Returns the egui scroll output (offset, content size, inner rect).
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        let t = Theme::of(ui);
        let now = ui.input(|i| i.time);
        let base_id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.next_auto_id());
        let sid = base_id.with("glowkit_styled_scroll");
        let st: BarState = glowkit_core::state(ui, sid, BarState::default);

        let mut sa = egui::ScrollArea::new(self.dirs)
            .id_salt(sid.with("inner"))
            .auto_shrink([false, true])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .stick_to_bottom(self.stick_to_bottom);
        if !self.drag_to_scroll {
            sa = sa.scroll_source(ScrollSource { drag: DragScroll::Never, ..Default::default() });
        }
        if let Some(h) = self.max_height {
            sa = sa.max_height(h).min_scrolled_height(h);
        }
        if let Some(w) = self.max_width {
            sa = sa.max_width(w).min_scrolled_width(w);
        }
        if let Some(o) = st.pending[0] {
            sa = sa.horizontal_scroll_offset(o);
        }
        if let Some(o) = st.pending[1] {
            sa = sa.vertical_scroll_offset(o);
        }
        let scoped = ui.scope(|ui| sa.show(ui, add));
        let out = scoped.inner;
        // the rect egui actually allocated (out.inner_rect is the pre-shrink estimate)
        let view = scoped.response.rect;
        let content = out.content_size;
        let offset = out.state.clamped_offset();
        let mut st = st;
        st.pending = [None, None];
        if (offset - st.last_offset).length_sq() > 0.01 {
            st.last_activity = now;
        }
        st.last_offset = offset;
        let hovered = ui.rect_contains_pointer(view);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let mut any_drag = false;
        let mut requests_repaint = false;

        for axis in 0..2 {
            if !self.dirs[axis] || self.mode == ScrollbarMode::Hidden {
                continue;
            }
            let (view_len, content_len) =
                if axis == 1 { (view.height(), content.y) } else { (view.width(), content.x) };
            if content_len <= view_len + 0.5 {
                continue;
            }
            let max_off = content_len - view_len;
            let off = if axis == 1 { offset.y } else { offset.x };
            let margin = 3.0;
            let grow = self.thickness * 1.6;
            let track = if axis == 1 {
                Rect::from_min_max(
                    pos2(view.max.x - grow - margin, view.min.y + margin),
                    pos2(view.max.x - margin, view.max.y - margin),
                )
            } else {
                Rect::from_min_max(
                    pos2(view.min.x + margin, view.max.y - grow - margin),
                    pos2(view.max.x - margin, view.max.y - margin),
                )
            };
            let track_len = if axis == 1 { track.height() } else { track.width() };
            let thumb_len = (view_len / content_len * track_len).clamp(24.0, track_len);
            let thumb_pos = (off / max_off).clamp(0.0, 1.0) * (track_len - thumb_len);

            let hit = ui.interact(track.expand(2.0), sid.with(("bar", axis)), Sense::click_and_drag());
            let near = pointer.is_some_and(|p| track.expand(6.0).contains(p));
            let pos_along = |p: egui::Pos2| if axis == 1 { p.y } else { p.x };
            if hit.drag_started() {
                if let Some(p) = hit.interact_pointer_pos() {
                    let along = pos_along(p) - if axis == 1 { track.min.y } else { track.min.x };
                    // click on the track (outside the thumb) jumps there first
                    let start_off = if along < thumb_pos || along > thumb_pos + thumb_len {
                        ((along - thumb_len / 2.0) / (track_len - thumb_len)).clamp(0.0, 1.0) * max_off
                    } else {
                        off
                    };
                    st.drag[axis] = Some((start_off, pos_along(p)));
                    st.pending[axis] = Some(start_off);
                }
            }
            if hit.dragged() {
                if let (Some((o0, p0)), Some(p)) = (st.drag[axis], hit.interact_pointer_pos()) {
                    let d = (pos_along(p) - p0) / (track_len - thumb_len).max(1.0) * max_off;
                    st.pending[axis] = Some((o0 + d).clamp(0.0, max_off));
                    st.last_activity = now;
                    any_drag = true;
                }
            } else {
                st.drag[axis] = None;
            }

            let id = sid.with(("vis", axis));
            let since = (now - st.last_activity) as f32;
            let want = match self.mode {
                ScrollbarMode::Always => true,
                _ => hovered || near || any_drag || since < 0.9,
            };
            let vis = ui.ctx().animate_bool_with_time(id, want, if want { 0.15 } else { 0.35 });
            let wide = ui.ctx().animate_bool_with_time(id.with("w"), near || any_drag, 0.12);
            if since < 1.0 {
                requests_repaint = true;
            }
            if vis <= 0.001 {
                continue;
            }
            let thick = self.thickness + (grow - self.thickness) * wide;
            let p = ui.painter().with_clip_rect(ui.clip_rect());
            let thumb = if axis == 1 {
                Rect::from_min_size(pos2(track.max.x - thick, track.min.y + thumb_pos), vec2(thick, thumb_len))
            } else {
                Rect::from_min_size(pos2(track.min.x + thumb_pos, track.max.y - thick), vec2(thumb_len, thick))
            };
            if wide > 0.0 {
                let tr = if axis == 1 {
                    Rect::from_min_max(pos2(track.max.x - thick, track.min.y), track.max)
                } else {
                    Rect::from_min_max(pos2(track.min.x, track.max.y - thick), track.max)
                };
                p.rect_filled(tr, CornerRadius::same(255), with_alpha(t.muted_foreground, 0.10 * wide * vis));
            }
            let base = self.color.unwrap_or(t.muted_foreground);
            let a = (if t.dark { 0.45 } else { 0.5 }) + 0.3 * wide;
            p.rect_filled(thumb, CornerRadius::same(255), with_alpha(base, a * vis));
        }

        if let Some(bg) = self.fade_edges {
            let fade = 18.0;
            let p = ui.painter().with_clip_rect(view);
            if self.dirs[1] {
                let top = (offset.y / fade).clamp(0.0, 1.0);
                let bottom = ((content.y - view.height() - offset.y) / fade).clamp(0.0, 1.0);
                edge_fade(&p, view, fade, bg, top, bottom, true);
            }
            if self.dirs[0] {
                let left = (offset.x / fade).clamp(0.0, 1.0);
                let right = ((content.x - view.width() - offset.x) / fade).clamp(0.0, 1.0);
                edge_fade(&p, view, fade, bg, left, right, false);
            }
        }

        if any_drag || st.pending.iter().any(Option::is_some) || requests_repaint {
            ui.ctx().request_repaint();
        }
        glowkit_core::set_state(ui, sid, st);
        out
    }
}

fn edge_fade(p: &egui::Painter, view: Rect, len: f32, bg: Color32, a0: f32, a1: f32, vertical: bool) {
    let mut mesh = egui::Mesh::default();
    let mut quad = |r: Rect, c0: Color32, c1: Color32| {
        let i = mesh.vertices.len() as u32;
        let (tl, tr, bl, br) = if vertical { (c0, c0, c1, c1) } else { (c0, c1, c0, c1) };
        mesh.colored_vertex(r.left_top(), tl);
        mesh.colored_vertex(r.right_top(), tr);
        mesh.colored_vertex(r.left_bottom(), bl);
        mesh.colored_vertex(r.right_bottom(), br);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i + 2, i + 1, i + 3);
    };
    let clear = with_alpha(bg, 0.0);
    if vertical {
        if a0 > 0.0 {
            quad(Rect::from_min_size(view.min, vec2(view.width(), len)), with_alpha(bg, a0), clear);
        }
        if a1 > 0.0 {
            quad(Rect::from_min_max(pos2(view.min.x, view.max.y - len), view.max), clear, with_alpha(bg, a1));
        }
    } else {
        if a0 > 0.0 {
            quad(Rect::from_min_size(view.min, vec2(len, view.height())), with_alpha(bg, a0), clear);
        }
        if a1 > 0.0 {
            quad(Rect::from_min_max(pos2(view.max.x - len, view.min.y), view.max), clear, with_alpha(bg, a1));
        }
    }
    p.add(egui::Shape::mesh(mesh));
}
