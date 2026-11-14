use super::common::{card, cr, hairline, hover_fill, icon, idle_for, lerp, morph_axes, progress, text_width, BeIcon};
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::{spring, stagger};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Copy, Debug)]
struct BloomState {
    open: bool,
    since: f64,
}

/// A "Create +" pill that blooms into a grid of actions. The pill's own box springs open into
/// the panel (width, height, position and corner radius all morph), its label glides up into
/// the panel's title, the plus turns into the close cross, and the tiles pop in one after
/// another. Closing plays the same morph backwards.
///
/// [`BloomMenu::show`] returns the index of the tile picked this frame.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BeIcon, BloomMenu};
/// let picked = BloomMenu::new("Create")
///     .item(BeIcon::Doc, "Doc")
///     .item(BeIcon::Grid, "Board")
///     .item(BeIcon::Table, "Table")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct BloomMenu {
    label: String,
    items: Vec<(BeIcon, String)>,
    columns: usize,
    tile: egui::Vec2,
    corner: f32,
    bounce: f32,
    autoplay: bool,
}

impl BloomMenu {
    /// A pill labelled `label` with no items yet.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
            columns: 3,
            tile: vec2(96.0, 76.0),
            corner: 18.0,
            bounce: 0.5,
            autoplay: true,
        }
    }
    /// Add a tile.
    pub fn item(mut self, icon: BeIcon, label: impl Into<String>) -> Self {
        self.items.push((icon, label.into()));
        self
    }
    /// Tiles per row (default 3).
    pub fn columns(mut self, n: usize) -> Self {
        self.columns = n.clamp(1, 6);
        self
    }
    /// Size of one tile.
    pub fn tile_size(mut self, s: egui::Vec2) -> Self {
        self.tile = s.max(vec2(48.0, 48.0));
        self
    }
    /// Corner radius of the open panel.
    pub fn corner(mut self, r: f32) -> Self {
        self.corner = r.max(0.0);
        self
    }
    /// Springiness of the bloom, 0 (no overshoot) to 1 (very bouncy).
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 1.0);
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the pill / panel. Returns the tile picked this frame, if any.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = Theme::of(ui);
        let painter = ui.painter().clone();
        let n = self.items.len();
        let cols = self.columns.min(n.max(1));
        let rows = n.div_ceil(cols).max(1);
        let head = 40.0;
        let open_size = vec2(self.tile.x * cols as f32, head + self.tile.y * rows as f32);
        let lw = text_width(&painter, &self.label, 14.0);
        let pill_size = vec2(lw + 66.0, 42.0);
        let area = open_size.max(pill_size);
        let (outer, resp) = ui.allocate_exact_size(area, Sense::click());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let top = outer.min.y;
        let pill_rect = Rect::from_center_size(pos2(outer.center().x, top + pill_size.y / 2.0), pill_size);
        let open_rect = Rect::from_min_size(pos2(outer.center().x - open_size.x / 2.0, top), open_size);
        let st0: BloomState = glowkit_core::state(ui, id, || BloomState { open: false, since: now });
        let hit = if st0.open { open_rect } else { pill_rect };
        let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| hit.contains(*p));
        let idle = idle_for(ui, id, ptr.is_some());
        let click_pos = if resp.clicked() { resp.interact_pointer_pos() } else { None };
        let tile_at = |p: egui::Pos2| -> Option<usize> {
            if p.y < open_rect.min.y + head {
                return None;
            }
            let c = ((p.x - open_rect.min.x) / self.tile.x).floor() as i32;
            let r = ((p.y - open_rect.min.y - head) / self.tile.y).floor() as i32;
            if c < 0 || r < 0 || c as usize >= cols {
                return None;
            }
            let i = r as usize * cols + c as usize;
            (i < n).then_some(i)
        };
        let close_btn =
            Rect::from_center_size(pos2(open_rect.max.x - 22.0, open_rect.min.y + head / 2.0), vec2(26.0, 26.0));
        let mut picked = None;
        let mut toggle = false;
        if let Some(p) = click_pos {
            if !st0.open && pill_rect.contains(p) {
                toggle = true;
            } else if st0.open {
                if close_btn.contains(p) || !open_rect.contains(p) {
                    toggle = true;
                } else if let Some(i) = tile_at(p) {
                    picked = Some(i);
                    toggle = true;
                }
            }
        }
        let outside =
            st0.open && ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(open_rect) && !resp.clicked();
        let auto = self.autoplay && idle > 2.2;
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0,
            |s| {
                if toggle || outside {
                    s.open = !s.open;
                    s.since = now;
                }
                if auto {
                    let held = now - s.since;
                    if (s.open && held > 2.6) || (!s.open && held > 1.4) {
                        s.open = !s.open;
                        s.since = now;
                    }
                }
                *s
            },
        );
        if auto {
            glowkit_core::animate(ui);
        }
        let open = st.open;
        let damping = lerp(30.0, 15.0, self.bounce);
        let (rect, radius) = morph_axes(
            ui,
            id.with("box"),
            if open { open_rect } else { pill_rect },
            if open { self.corner } else { pill_size.y / 2.0 },
            (if open { 300.0 } else { 380.0 }, damping),
            (if open { 260.0 } else { 420.0 }, damping + 2.0),
        );
        let k = progress(rect.height(), pill_size.y, open_size.y);
        if !ui.is_rect_visible(outer) {
            return picked;
        }
        card(&painter, &t, rect, radius, 0.35 + 0.65 * k.min(1.0));
        let clip = painter.with_clip_rect(rect.shrink(0.5));
        // the label travels from the middle of the pill to the header's corner
        let kk = k.min(1.0);
        let label_from = pos2(pill_rect.center().x - (lw + 22.0) / 2.0, pill_rect.center().y);
        let label_to = pos2(rect.min.x + 16.0, rect.min.y + head / 2.0);
        let lp = label_from.lerp(label_to, kk);
        let size = lerp(14.0, 13.0, kk);
        clip.text(
            lp,
            Align2::LEFT_CENTER,
            &self.label,
            FontId::proportional(size),
            mix(t.foreground, t.muted_foreground, kk),
        );
        // plus turns into the close cross and slides to the corner
        let plus_from = pos2(label_from.x + lw + 22.0 + 1.0, pill_rect.center().y);
        let plus_to = close_btn.center();
        let pc = plus_from.lerp(plus_to, kk);
        let hov_close = open && ptr.is_some_and(|p| close_btn.contains(p));
        if hov_close {
            clip.circle_filled(pc, 12.0, hover_fill(&t));
        }
        let rot = kk * std::f32::consts::FRAC_PI_4;
        let s = 5.5;
        let (sn, cs) = rot.sin_cos();
        let col = mix(t.foreground, t.muted_foreground, kk * 0.6);
        for d in [vec2(1.0, 0.0), vec2(0.0, 1.0)] {
            let v = vec2(d.x * cs - d.y * sn, d.x * sn + d.y * cs) * s;
            clip.line_segment([pc - v, pc + v], Stroke::new(1.6, col));
        }
        // header divider and tiles
        let ga = ((k - 0.35) / 0.65).clamp(0.0, 1.0);
        if ga > 0.0 {
            let y = rect.min.y + head;
            clip.line_segment(
                [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                Stroke::new(1.0, with_alpha(hairline(&t).color, ga)),
            );
            let since = (now - st.since) as f32;
            let hov_tile = ptr.and_then(tile_at).filter(|_| open);
            let hx = spring(ui, id.with("hx"), hov_tile.map(|i| (i % cols) as f32).unwrap_or(0.0), 420.0, 32.0);
            let hy = spring(ui, id.with("hy"), hov_tile.map(|i| (i / cols) as f32).unwrap_or(0.0), 420.0, 32.0);
            let ha = spring(ui, id.with("ha"), if hov_tile.is_some() { 1.0 } else { 0.0 }, 300.0, 26.0);
            if ha > 0.01 {
                let hr = Rect::from_min_size(pos2(rect.min.x + hx * self.tile.x, y + hy * self.tile.y), self.tile)
                    .shrink(4.0);
                clip.rect_filled(hr, cr(12.0), with_alpha(hover_fill(&t), ha * ga));
            }
            for (i, (ic, label)) in self.items.iter().enumerate() {
                let c = (i % cols) as f32;
                let r = (i / cols) as f32;
                let e = if open { stagger(since - 0.05, i, 0.035, 0.32) } else { 1.0 };
                let e = glowkit_core::ease(glowkit_core::Easing::OutBack, e);
                let a = ga * e.clamp(0.0, 1.0);
                if a <= 0.01 {
                    continue;
                }
                let cell = Rect::from_min_size(pos2(rect.min.x + c * self.tile.x, y + r * self.tile.y), self.tile);
                if c > 0.0 {
                    clip.line_segment(
                        [cell.left_top(), cell.left_bottom()],
                        Stroke::new(1.0, with_alpha(hairline(&t).color, ga)),
                    );
                }
                if r > 0.0 {
                    clip.line_segment(
                        [cell.left_top(), cell.right_top()],
                        Stroke::new(1.0, with_alpha(hairline(&t).color, ga)),
                    );
                }
                let sc = 0.7 + 0.3 * e;
                let mid = cell.center() + vec2(0.0, (1.0 - e) * 8.0);
                let hot = hov_tile == Some(i);
                let fg = if hot { t.foreground } else { t.muted_foreground };
                icon(&clip, *ic, mid - vec2(0.0, 10.0 * sc), 16.0 * sc, with_alpha(fg, a), 1.4);
                clip.text(
                    mid + vec2(0.0, 13.0 * sc),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(12.0 * sc),
                    with_alpha(fg, a),
                );
            }
        }
        if ptr.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        picked
    }
}
