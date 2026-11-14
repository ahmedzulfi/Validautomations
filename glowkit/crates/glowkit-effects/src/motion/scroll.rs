use egui::emath::TSTransform;
use egui::{pos2, vec2, Color32, CursorIcon, Rect, Response, Sense, Ui, UiBuilder};
use glowkit_core::anim::{ease, Easing, Spring};
use glowkit_core::{paint, Theme};

#[derive(Clone, Copy, Default)]
struct ScrollState {
    offset: f32,
    vel: f32,
}

/// An endless vertical list that wraps around: scroll it with the wheel or by dragging (with
/// inertia), or let it drift on its own. Edges fade out.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// glowkit_effects::InfiniteScroll::new(8).autoplay(30.0).show(ui, |ui, i| {
///     ui.label(format!("Item {i}"));
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index| ...)`"]
pub struct InfiniteScroll {
    count: usize,
    item_height: f32,
    gap: f32,
    height: f32,
    width: Option<f32>,
    speed: f32,
    pause_on_hover: bool,
    fade: f32,
    fade_color: Option<Color32>,
    salt: u64,
}

impl InfiniteScroll {
    /// A wrapping list of `count` items.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            item_height: 72.0,
            gap: 12.0,
            height: 360.0,
            width: None,
            speed: 0.0,
            pause_on_hover: true,
            fade: 70.0,
            fade_color: None,
            salt: 0,
        }
    }
    /// Height of every item.
    pub fn item_height(mut self, h: f32) -> Self {
        self.item_height = h.max(8.0);
        self
    }
    /// Gap between items.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Viewport height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(20.0);
        self
    }
    /// Viewport width (defaults to all available).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w.max(20.0));
        self
    }
    /// Drift speed in points per second (negative scrolls down). 0 = no autoplay.
    pub fn autoplay(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    /// Stop drifting while hovered (default on).
    pub fn pause_on_hover(mut self, on: bool) -> Self {
        self.pause_on_hover = on;
        self
    }
    /// Height of the faded edges (0 = none).
    pub fn fade(mut self, h: f32) -> Self {
        self.fade = h.max(0.0);
        self
    }
    /// Color the edges fade into. Defaults to the theme card color.
    pub fn fade_color(mut self, c: Color32) -> Self {
        self.fade_color = Some(c);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the list; `item(ui, index)` lays out one item inside its slot.
    pub fn show(self, ui: &mut Ui, mut item: impl FnMut(&mut Ui, usize)) -> Response {
        let w = self.width.unwrap_or(ui.available_width()).min(ui.available_width());
        let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), vec2(w, self.height));
        let response = ui.interact(rect, ui.scope_id().with(("infinite_scroll", self.salt)), Sense::drag());
        let id = response.id;
        if self.count == 0 {
            return response;
        }
        let dt = glowkit_core::dt(ui);
        let step = self.item_height + self.gap;
        let period = step * self.count as f32;
        let hovered = response.hovered();
        let scroll = if hovered { ui.input(|i| i.smooth_scroll_delta().y) } else { 0.0 };
        let factor =
            glowkit_core::anim::follow(ui, id.with("f"), if self.pause_on_hover && hovered { 0.0 } else { 1.0 }, 4.0);
        let mut st: ScrollState = glowkit_core::state(ui, id.with("st"), ScrollState::default);
        if response.dragged() {
            let dy = response.drag_delta().y;
            st.offset -= dy;
            st.vel = if dt > 0.0 { -dy / dt } else { 0.0 };
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else {
            st.offset -= scroll;
            st.vel *= (-3.5 * dt).exp();
            st.offset += (st.vel + self.speed * factor) * dt;
            if hovered {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        st.offset = st.offset.rem_euclid(period);
        glowkit_core::set_state(ui, id.with("st"), st);
        if self.speed != 0.0 || st.vel.abs() > 0.5 {
            glowkit_core::animate(ui);
        }
        let clip = rect.intersect(ui.clip_rect());
        let first = (st.offset / step).floor() as i64 - 1;
        let visible = (self.height / step).ceil() as i64 + 2;
        for k in first..first + visible {
            let y = rect.min.y + k as f32 * step - st.offset;
            let index = k.rem_euclid(self.count as i64) as usize;
            let r = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), self.item_height));
            if r.max.y < rect.min.y || r.min.y > rect.max.y {
                continue;
            }
            let mut child = ui.new_child(UiBuilder::new().max_rect(r).id_salt(("inf_item", k)));
            child.set_clip_rect(clip);
            item(&mut child, index);
        }
        if self.fade > 0.0 {
            let bg = self.fade_color.unwrap_or_else(|| Theme::of(ui).card);
            paint::fade_edges_vertical(&ui.painter_at(rect), rect, self.fade.min(self.height / 3.0), bg);
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Tile {
    x: Spring,
    y: Spring,
    w: Spring,
    born: f32,
}

/// A masonry grid of variable-height tiles: columns adapt to the width, and when the layout
/// changes (resize, reorder, column count) every tile glides to its new spot on a spring.
/// Tiles rise in with a stagger the first time they appear.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let heights = [180.0, 120.0, 240.0, 160.0];
/// glowkit_effects::Masonry::new(&heights).show(ui, |ui, i, rect| {
///     ui.painter().rect_filled(rect, 8, egui::Color32::DARK_GRAY);
/// });
/// ```
#[must_use = "call `.show(ui, |ui, index, rect| ...)`"]
pub struct Masonry<'a> {
    heights: &'a [f32],
    order: Option<&'a [usize]>,
    columns: Option<usize>,
    min_column_width: f32,
    gap: f32,
    stagger: f32,
    hover_scale: f32,
    salt: u64,
}

impl<'a> Masonry<'a> {
    /// A grid with one tile per entry of `heights` (tile heights in points).
    pub fn new(heights: &'a [f32]) -> Self {
        Self {
            heights,
            order: None,
            columns: None,
            min_column_width: 170.0,
            gap: 14.0,
            stagger: 0.05,
            hover_scale: 1.03,
            salt: 0,
        }
    }
    /// Display order (a permutation of tile indices). Changing it animates the reflow.
    pub fn order(mut self, order: &'a [usize]) -> Self {
        self.order = Some(order);
        self
    }
    /// Fixed column count (default: as many as fit [`Self::min_column_width`]).
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = Some(n.max(1));
        self
    }
    /// Minimum column width when the column count is automatic.
    pub fn min_column_width(mut self, w: f32) -> Self {
        self.min_column_width = w.max(20.0);
        self
    }
    /// Gap between tiles.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g.max(0.0);
        self
    }
    /// Seconds between consecutive tiles rising in.
    pub fn stagger(mut self, s: f32) -> Self {
        self.stagger = s.max(0.0);
        self
    }
    /// Scale of a hovered tile (1 = none).
    pub fn hover_scale(mut self, s: f32) -> Self {
        self.hover_scale = s.clamp(1.0, 1.3);
        self
    }
    /// Distinguish several instances inside the same parent `Ui`.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.salt = egui::epaint::util::hash(salt);
        self
    }

    /// Show the grid; `tile(ui, index, rect)` draws one tile into its current (animated) rect.
    pub fn show(self, ui: &mut Ui, mut tile: impl FnMut(&mut Ui, usize, Rect)) -> Response {
        let n = self.heights.len();
        let width = ui.available_width();
        let cols = self
            .columns
            .unwrap_or(((width + self.gap) / (self.min_column_width + self.gap)).floor().max(1.0) as usize)
            .max(1);
        let col_w = (width - self.gap * (cols as f32 - 1.0)) / cols as f32;
        let default_order: Vec<usize> = (0..n).collect();
        let order = self.order.filter(|o| o.len() == n).unwrap_or(&default_order);
        // shortest-column-first packing
        let mut col_h = vec![0.0f32; cols];
        let mut targets = vec![Rect::NOTHING; n];
        for &i in order {
            let c = (0..cols).min_by(|a, b| col_h[*a].total_cmp(&col_h[*b])).unwrap_or(0);
            let x = c as f32 * (col_w + self.gap);
            targets[i] = Rect::from_min_size(pos2(x, col_h[c]), vec2(col_w, self.heights[i]));
            col_h[c] += self.heights[i] + self.gap;
        }
        let total_h = col_h.iter().cloned().fold(0.0, f32::max) - self.gap;
        let id = ui.scope_id().with(("masonry", self.salt));
        let h = glowkit_core::anim::spring(ui, id.with("h"), total_h.max(0.0), 200.0, 26.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let stagger = self.stagger;
        let (tiles, settled) = glowkit_core::with_state(ui, id.with("tiles"), Vec::<Tile>::new, |tiles| {
            if tiles.len() != n {
                *tiles = (0..n)
                    .map(|i| {
                        let r = targets[i];
                        let k = order.iter().position(|o| *o == i).unwrap_or(i);
                        Tile {
                            x: Spring::new(r.min.x).with(240.0, 26.0),
                            y: Spring::new(r.min.y).with(240.0, 26.0),
                            w: Spring::new(r.width()).with(240.0, 26.0),
                            born: now + k as f32 * stagger,
                        }
                    })
                    .collect();
            }
            let mut settled = true;
            for (i, t) in tiles.iter_mut().enumerate() {
                let r = targets[i];
                t.x.target = r.min.x;
                t.y.target = r.min.y;
                t.w.target = r.width();
                if now < t.born + 0.3 {
                    // not shown yet: jump straight to the slot (first-frame layout may differ)
                    t.x.value = r.min.x;
                    t.y.value = r.min.y;
                    t.w.value = r.width();
                }
                t.x.step(dt);
                t.y.step(dt);
                t.w.step(dt);
                settled &= t.x.is_settled() && t.y.is_settled() && t.w.is_settled() && now - t.born > 0.8;
            }
            (tiles.clone(), settled)
        });
        if !settled {
            ui.ctx().request_repaint();
        }
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|_| ui.rect_contains_pointer(rect));
        for &i in order {
            let tl = tiles[i];
            let appear = ease(Easing::OutCubic, ((now - tl.born) / 0.7).clamp(0.0, 1.0));
            if appear <= 0.0 {
                continue;
            }
            let r = Rect::from_min_size(
                rect.min + vec2(tl.x.value, tl.y.value + (1.0 - appear) * 60.0),
                vec2(tl.w.value, self.heights[i]),
            );
            let hov = pointer.map(|p| r.contains(p)).unwrap_or(false);
            let s = glowkit_core::anim::spring(
                ui,
                id.with(("hs", i)),
                if hov { self.hover_scale } else { 1.0 },
                300.0,
                20.0,
            );
            let c = r.center();
            let tf = TSTransform::from_translation(c.to_vec2())
                * TSTransform::from_scaling(s)
                * TSTransform::from_translation(-c.to_vec2());
            ui.with_visual_transform(tf, |ui| {
                let mut child = ui.new_child(UiBuilder::new().max_rect(r).id_salt(("masonry_tile", i)));
                child.multiply_opacity(appear);
                tile(&mut child, i, r);
            });
        }
        response
    }
}
