//! Hotbar, inventory grid with drag and drop, and the item tooltip card.

use super::common::*;
use egui::{
    pos2, vec2, Align2, Color32, FontId, LayerId, Order, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
};
use glowkit_core::anim::spring;
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::paint;

/// Paint a single item in a slot: icon, count badge and the legendary beam border.
fn paint_item(painter: &Painter, cell: Rect, item: &GameItem, scale: f32, time: f32) {
    let c = cell.center();
    if item.rarity == GameRarity::Legendary {
        let head = (time * 0.35).fract();
        paint::gradient_border(painter, cell.shrink(1.0), 4.0, 2.0, paint::comet_fn(head, 0.45, hex("#FFD27A")));
        paint::glow(
            painter,
            c,
            cell.width() * 0.55,
            with_alpha(GameRarity::Legendary.color(), 0.18 + 0.08 * (time * 3.0).sin()),
        );
    }
    item.icon.paint(painter, c, cell.width() * 0.64 * scale, item.tint());
    if item.count > 1 {
        outlined(
            painter,
            cell.max - vec2(4.0, 2.0),
            Align2::RIGHT_BOTTOM,
            &item.count.to_string(),
            FontId::proportional(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
    }
}

fn rarity_text(style: GameStyle, r: GameRarity) -> Color32 {
    if style == GameStyle::Fantasy {
        shade(r.color(), -0.45)
    } else {
        r.color()
    }
}

/// Paint a tooltip card for `item` at `pos` (its top-left), comparing with `cmp`. Returns its rect.
#[allow(clippy::too_many_arguments)]
fn paint_tooltip(
    painter: &Painter,
    pos: Pos2,
    item: &GameItem,
    cmp: Option<&GameItem>,
    style: GameStyle,
    pal: &GamePalette,
    title: &FontId,
    alpha: f32,
) -> Rect {
    let w = 250.0;
    let flavor_g = (!item.flavor.is_empty())
        .then(|| painter.layout(item.flavor.clone(), FontId::proportional(11.5), with_alpha(pal.dim, alpha), w - 32.0));
    let h = 66.0
        + item.stats.len() as f32 * 21.0
        + flavor_g.as_ref().map(|g| g.size().y + 14.0).unwrap_or(0.0)
        + if cmp.is_some() { 24.0 } else { 6.0 }
        + 8.0;
    let rect = Rect::from_min_size(pos, vec2(w, h));
    let rc = item.rarity.color();
    // panel
    frame(painter, rect, style, pal, 0.0);
    let band = Rect::from_min_max(rect.min + vec2(2.0, 2.0), pos2(rect.max.x - 2.0, rect.min.y + 58.0));
    paint::gradient_rect(painter, band, 4.0, 0.0, &[with_alpha(rc, 0.35 * alpha), with_alpha(rc, 0.0)]);
    painter.line_segment(
        [pos2(rect.min.x + 12.0, band.max.y), pos2(rect.max.x - 12.0, band.max.y)],
        Stroke::new(1.0, with_alpha(rc, 0.6 * alpha)),
    );
    let tc = rarity_text(style, item.rarity);
    painter.text(
        pos2(rect.min.x + 16.0, rect.min.y + 22.0),
        Align2::LEFT_CENTER,
        &item.name,
        title.clone(),
        with_alpha(tc, alpha),
    );
    let sub = if item.kind.is_empty() {
        item.rarity.label().to_string()
    } else {
        format!("{} {}", item.rarity.label(), item.kind)
    };
    painter.text(
        pos2(rect.min.x + 16.0, rect.min.y + 42.0),
        Align2::LEFT_CENTER,
        sub,
        FontId::proportional(11.5),
        with_alpha(pal.dim, alpha),
    );
    let ic = Rect::from_center_size(pos2(rect.max.x - 32.0, rect.min.y + 30.0), Vec2::splat(40.0));
    slot(painter, ic, style, pal, Some(item.rarity), 0.0);
    item.icon.paint(painter, ic.center(), 26.0, item.tint());
    let mut y = rect.min.y + 76.0;
    for (name, v) in &item.stats {
        painter.text(
            pos2(rect.min.x + 16.0, y),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(12.5),
            with_alpha(pal.dim, alpha),
        );
        let vs = if v.fract().abs() < 1e-3 { format!("{}", *v as i64) } else { format!("{v:.1}") };
        let vx = rect.max.x - 72.0;
        painter.text(pos2(vx, y), Align2::RIGHT_CENTER, vs, FontId::monospace(13.0), with_alpha(pal.text, alpha));
        if let Some(other) =
            cmp.and_then(|c| c.stats.iter().find(|(n, _)| n == name)).map(|(_, v)| *v).or(cmp.map(|_| 0.0))
        {
            let d = v - other;
            if d.abs() > 1e-3 {
                let (col, up) = if d > 0.0 { (pal.good, true) } else { (pal.bad, false) };
                let col = if style == GameStyle::Fantasy { shade(col, -0.3) } else { col };
                let ax = vx + 12.0;
                let tri = if up {
                    vec![pos2(ax, y - 5.0), pos2(ax + 5.0, y + 3.0), pos2(ax - 5.0, y + 3.0)]
                } else {
                    vec![pos2(ax - 5.0, y - 3.0), pos2(ax + 5.0, y - 3.0), pos2(ax, y + 5.0)]
                };
                painter.add(egui::Shape::convex_polygon(tri, with_alpha(col, alpha), Stroke::NONE));
                let ds = if d.fract().abs() < 1e-3 { format!("{:+}", d as i64) } else { format!("{d:+.1}") };
                painter.text(
                    pos2(ax + 9.0, y),
                    Align2::LEFT_CENTER,
                    ds,
                    FontId::monospace(11.5),
                    with_alpha(col, alpha),
                );
            }
        }
        y += 21.0;
    }
    if let Some(g) = flavor_g {
        y += 2.0;
        painter.line_segment(
            [pos2(rect.min.x + 16.0, y - 6.0), pos2(rect.max.x - 16.0, y - 6.0)],
            Stroke::new(1.0, with_alpha(pal.dim, 0.3 * alpha)),
        );
        let gh = g.size().y;
        painter.galley(pos2(rect.min.x + 16.0, y), g, pal.dim);
        y += gh + 10.0;
    }
    if let Some(c) = cmp {
        painter.text(
            pos2(rect.min.x + 16.0, y + 4.0),
            Align2::LEFT_CENTER,
            format!("Compared to equipped: {}", c.name),
            FontId::proportional(10.5),
            with_alpha(pal.dim, 0.85 * alpha),
        );
    }
    rect
}

/// A standalone item tooltip card: rarity-tinted header with name, type and icon, a stat list
/// with green / red arrows against the equipped item, and flavor text.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let (item, equipped): (glowkit_interactions::GameItem, glowkit_interactions::GameItem) = todo!();
/// ui.add(glowkit_interactions::GameItemTooltip::new(&item).compare(Some(&equipped)));
/// ```
pub struct GameItemTooltip<'a> {
    item: &'a GameItem,
    cmp: Option<&'a GameItem>,
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameItemTooltip<'a> {
    /// A tooltip for `item`.
    pub fn new(item: &'a GameItem) -> Self {
        Self { item, cmp: None, style: GameStyle::SciFi, palette: None }
    }
    /// The item to compare stats against.
    pub fn compare(mut self, c: Option<&'a GameItem>) -> Self {
        self.cmp = c;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
}

impl egui::Widget for GameItemTooltip<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title = display_font(ui, 16.0);
        let probe = paint_tooltip(
            &ui.painter().with_clip_rect(Rect::NOTHING),
            Pos2::ZERO,
            self.item,
            self.cmp,
            self.style,
            &pal,
            &title,
            1.0,
        );
        let (rect, resp) = ui.allocate_exact_size(probe.size(), Sense::hover());
        paint_tooltip(ui.painter(), rect.min, self.item, self.cmp, self.style, &pal, &title, 1.0);
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Hotbar

/// An action / item hotbar. The selected slot springs up and grows with a glow beneath it, and
/// its item name pops up above for a moment after each change. Click, scroll the mouse wheel over
/// it or press the number keys to select.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let slots: Vec<Option<glowkit_interactions::GameItem>> = vec![];
/// let mut selected = 0usize;
/// ui.add(glowkit_interactions::GameHotbar::new(&mut selected, &slots).keyboard(true));
/// ```
pub struct GameHotbar<'a> {
    selected: &'a mut usize,
    slots: &'a [Option<GameItem>],
    slot: f32,
    keyboard: bool,
    style: GameStyle,
    palette: Option<GamePalette>,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameHotbar<'a> {
    /// A hotbar over `slots`, editing the `selected` index.
    pub fn new(selected: &'a mut usize, slots: &'a [Option<GameItem>]) -> Self {
        Self { selected, slots, slot: 54.0, keyboard: false, style: GameStyle::SciFi, palette: None, cues: None }
    }
    /// Slot size in points.
    pub fn slot_size(mut self, s: f32) -> Self {
        self.slot = s.clamp(28.0, 120.0);
        self
    }
    /// Select with number keys 1-9 (off by default so it doesn't steal text input).
    pub fn keyboard(mut self, on: bool) -> Self {
        self.keyboard = on;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Collect feedback cues ([`GameCue::Tick`] on change).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameHotbar<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let n = self.slots.len().max(1);
        let s = self.slot;
        let gap = 8.0;
        let w = n as f32 * s + (n - 1) as f32 * gap;
        let (rect, resp) = ui.allocate_exact_size(vec2(w + 24.0, s + 54.0), Sense::hover());
        let id = resp.id;
        let mut sel = (*self.selected).min(n - 1);
        // input
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
            let acc = glowkit_core::with_state(
                ui,
                id.with("scroll"),
                || 0.0f32,
                |a| {
                    *a += scroll;
                    let r = *a;
                    if r.abs() > 40.0 {
                        *a = 0.0;
                    }
                    r
                },
            );
            if acc > 40.0 {
                sel = (sel + n - 1) % n;
            } else if acc < -40.0 {
                sel = (sel + 1) % n;
            }
        }
        if self.keyboard {
            let keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ];
            for (i, k) in keys.iter().enumerate().take(n) {
                if ui.input(|inp| inp.key_pressed(*k)) {
                    sel = i;
                }
            }
        }
        let bottom = rect.max.y - 10.0;
        let x0 = rect.center().x - w / 2.0;
        let cells: Vec<Rect> =
            (0..n).map(|i| Rect::from_min_size(pos2(x0 + i as f32 * (s + gap), bottom - s), Vec2::splat(s))).collect();
        for (i, c) in cells.iter().enumerate() {
            let r = ui.interact(*c, id.with(("slot", i)), Sense::click());
            if r.clicked() {
                sel = i;
            }
        }
        if sel != *self.selected {
            *self.selected = sel;
            push_cue(&mut self.cues, GameCue::Tick);
        }
        let (since, _, _) = changed(ui, id.with("sel"), sel as f32);
        let time = glowkit_core::time(ui);
        let painter = ui.painter();
        // bar backing
        let back = Rect::from_min_max(pos2(x0 - 10.0, bottom - s - 10.0), pos2(x0 + w + 10.0, bottom + 8.0));
        match self.style {
            GameStyle::Fantasy => {
                paint::soft_shadow(painter, back, 6.0, 14.0, vec2(0.0, 4.0), Color32::from_black_alpha(150));
                paint::gradient_rect(painter, back, 6.0, 90.0, &[hex("#4A3220"), hex("#2A1B10")]);
                painter.rect_stroke(back, 6.0, Stroke::new(2.0, pal.edge), egui::StrokeKind::Inside);
            }
            GameStyle::SciFi => {
                let pts = chamfer_points(back, 10.0);
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    hex("#030A14B0"),
                    Stroke::new(1.0, with_alpha(pal.edge, 0.35)),
                ));
            }
            GameStyle::Minimal => {
                painter.rect_filled(back, 14.0, hex("#0C0D11B8"));
            }
        }
        let mut sel_rect = cells[sel];
        for (i, cell) in cells.iter().enumerate() {
            let on = i == sel;
            let lift = spring(ui, id.with(("lift", i)), if on { 1.0 } else { 0.0 }, 520.0, 14.0);
            let r =
                Rect::from_center_size(cell.center() - vec2(0.0, 10.0 * lift), Vec2::splat(s * (1.0 + 0.16 * lift)));
            if on {
                sel_rect = r;
                paint::glow_ellipse(
                    painter,
                    pos2(cell.center().x, bottom + 2.0),
                    vec2(s * 0.7, 9.0),
                    with_alpha(pal.accent, 0.55),
                );
            }
            let item = self.slots.get(i).and_then(|o| o.as_ref());
            slot(painter, r, self.style, &pal, item.map(|it| it.rarity), lift);
            if let Some(it) = item {
                paint_item(painter, r, it, 1.0 + 0.08 * lift, time);
            }
            outlined(
                painter,
                r.min + vec2(5.0, 3.0),
                Align2::LEFT_TOP,
                &(i + 1).to_string(),
                FontId::monospace(10.0),
                with_alpha(Color32::WHITE, 0.75),
                Color32::from_black_alpha(200),
            );
        }
        // selection frame
        let sc = pal.accent;
        let b = 10.0;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let p = sel_rect.center() + vec2(sx * (sel_rect.width() / 2.0 + 3.0), sy * (sel_rect.height() / 2.0 + 3.0));
            painter.line_segment([p, p - vec2(sx * b, 0.0)], Stroke::new(2.5, sc));
            painter.line_segment([p, p - vec2(0.0, sy * b)], Stroke::new(2.5, sc));
        }
        // name pop
        if let Some(it) = self.slots.get(sel).and_then(|o| o.as_ref()) {
            let a = if since < 1.6 { 1.0 } else { (1.0 - (since - 1.6) / 0.4).max(0.0) };
            if a > 0.0 {
                ui.ctx().request_repaint();
                let p = pop((since / 0.3).min(1.0));
                let pos = pos2(sel_rect.center().x, sel_rect.min.y - 14.0 - 6.0 * p);
                let f = FontId::proportional(13.0 * (0.7 + 0.3 * p));
                outlined(
                    painter,
                    pos,
                    Align2::CENTER_BOTTOM,
                    &it.name,
                    f,
                    with_alpha(it.rarity.color(), a),
                    Color32::from_black_alpha((220.0 * a) as u8),
                );
            }
        }
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Inventory

#[derive(Clone, Debug, Default)]
struct InvState {
    drag: Option<usize>,
    landing: Option<(usize, Pos2, f64)>,
    ghost_phase: i64,
}

/// An inventory grid with drag and drop. Items sit in slots tinted by rarity (rare and up glow,
/// legendary items have a light beam running around their border). Grab an item and it lifts
/// out with a lagging tilt, hovered targets light up, and on release the two slots swap with a
/// springy landing. Hovering shows a tooltip card with stats compared to the equipped item of the
/// same type.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut bag: Vec<Option<glowkit_interactions::GameItem>> = vec![None; 24];
/// # let equipped: Vec<glowkit_interactions::GameItem> = vec![];
/// ui.add(glowkit_interactions::GameInventoryGrid::new(&mut bag, 6).title("Backpack").equipped(&equipped));
/// ```
pub struct GameInventoryGrid<'a> {
    items: &'a mut Vec<Option<GameItem>>,
    cols: usize,
    cell: f32,
    title: &'a str,
    equipped: &'a [GameItem],
    style: GameStyle,
    palette: Option<GamePalette>,
    autoplay: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameInventoryGrid<'a> {
    /// A grid over `items` with `cols` columns (empty slots are `None`).
    pub fn new(items: &'a mut Vec<Option<GameItem>>, cols: usize) -> Self {
        Self {
            items,
            cols: cols.max(1),
            cell: 54.0,
            title: "",
            equipped: &[],
            style: GameStyle::SciFi,
            palette: None,
            autoplay: false,
            cues: None,
        }
    }
    /// Cell size in points.
    pub fn cell_size(mut self, s: f32) -> Self {
        self.cell = s.clamp(28.0, 120.0);
        self
    }
    /// Panel title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Equipped items; tooltips compare against the one with the same `kind`.
    pub fn equipped(mut self, e: &'a [GameItem]) -> Self {
        self.equipped = e;
        self
    }
    /// Visual style.
    pub fn style(mut self, s: GameStyle) -> Self {
        self.style = s;
        self
    }
    /// Override the palette.
    pub fn palette(mut self, p: GamePalette) -> Self {
        self.palette = Some(p);
        self
    }
    /// Demo mode: when idle, a ghost cursor inspects and rearranges items (modifies `items`).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Equip`] on pick up and drop, [`GameCue::Hover`]).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameInventoryGrid<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title_f = display_font(ui, 16.0);
        let tip_f = display_font(ui, 16.0);
        let n = self.items.len();
        let rows = n.div_ceil(self.cols).max(1);
        let s = self.cell;
        let gap = 6.0;
        let pad = 16.0;
        let head = if self.title.is_empty() { 0.0 } else { 36.0 };
        let gw = self.cols as f32 * s + (self.cols - 1) as f32 * gap;
        let gh = rows as f32 * s + (rows - 1) as f32 * gap;
        let (rect, resp) = ui.allocate_exact_size(vec2(gw + pad * 2.0, gh + pad * 2.0 + head), Sense::hover());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let time = now as f32;
        let origin = rect.min + vec2(pad, pad + head);
        let cell_at = |i: usize| {
            Rect::from_min_size(
                origin + vec2((i % self.cols) as f32 * (s + gap), (i / self.cols) as f32 * (s + gap)),
                Vec2::splat(s),
            )
        };
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let mut st: InvState = glowkit_core::state(ui, id.with("inv"), InvState::default);
        // real interaction
        let mut hovered: Option<usize> = None;
        let mut drop_now: Option<(usize, usize, Pos2)> = None;
        let mut any_active = false;
        for i in 0..n {
            let r = ui.interact(cell_at(i), id.with(("cell", i)), Sense::click_and_drag());
            if r.hovered() || r.dragged() {
                any_active = true;
            }
            if r.hovered() {
                hovered = Some(i);
            }
            if r.drag_started() && self.items[i].is_some() {
                st.drag = Some(i);
                push_cue(&mut self.cues, GameCue::Equip);
            }
            if r.drag_stopped() && st.drag == Some(i) {
                let p = ptr.unwrap_or(cell_at(i).center());
                let dst = (0..n).find(|j| cell_at(*j).expand(gap / 2.0).contains(p)).unwrap_or(i);
                drop_now = Some((i, dst, p));
            }
        }
        let idle = idle_for(ui, id, any_active || st.drag.is_some() && !self.autoplay);
        // ghost autoplay: inspect a legendary, then drag one item onto another slot
        let mut ghost: Option<(Pos2, bool)> = None;
        if self.autoplay && idle > 2.5 && st.drag.is_none_or(|_| st.ghost_phase != 0) {
            glowkit_core::animate(ui);
            let cyc = 7.0;
            let ph = (idle - 2.5) % cyc;
            let k = ((idle - 2.5) / cyc).floor() as i64;
            let filled: Vec<usize> = (0..n).filter(|i| self.items[*i].is_some()).collect();
            let empty: Vec<usize> = (0..n).filter(|i| self.items[*i].is_none()).collect();
            if !filled.is_empty() {
                let best = *filled
                    .iter()
                    .max_by_key(|i| self.items[**i].as_ref().map(|it| it.rarity).unwrap_or_default())
                    .unwrap();
                let src = filled[(k as usize * 3 + 1) % filled.len()];
                let dst = if empty.is_empty() {
                    filled[(k as usize * 5 + 2) % filled.len()]
                } else {
                    empty[(k as usize * 7) % empty.len()]
                };
                let rest = pos2(rect.max.x - 30.0, rect.max.y - 20.0);
                let lerp = |a: Pos2, b: Pos2, t: f32| {
                    a + (b - a) * glowkit_core::ease(glowkit_core::Easing::InOutCubic, t.clamp(0.0, 1.0))
                };
                let (p, pressed) = if ph < 0.8 {
                    (lerp(rest, cell_at(best).center(), ph / 0.8), false)
                } else if ph < 3.0 {
                    (cell_at(best).center() + vec2(4.0, 6.0), false)
                } else if ph < 3.7 {
                    (lerp(cell_at(best).center(), cell_at(src).center(), (ph - 3.0) / 0.7), false)
                } else if ph < 5.2 {
                    if st.ghost_phase != k + 1 && st.drag.is_none() {
                        st.drag = Some(src);
                        st.ghost_phase = k + 1;
                    }
                    (lerp(cell_at(src).center(), cell_at(dst).center(), (ph - 3.9) / 1.2), ph > 3.8)
                } else {
                    if st.ghost_phase == k + 1 {
                        if let Some(sidx) = st.drag {
                            drop_now = Some((sidx, dst, cell_at(dst).center()));
                        }
                        st.ghost_phase = -(k + 1);
                    }
                    (lerp(cell_at(dst).center(), rest, (ph - 5.2) / 1.2), false)
                };
                ghost = Some((p, pressed));
                if (0.8..3.0).contains(&ph) {
                    hovered = Some(best);
                }
            }
        }
        let drag_pos = if let Some((g, _)) = ghost { Some(g) } else { ptr };
        if let Some((src, dst, p)) = drop_now {
            if src != dst {
                self.items.swap(src, dst);
            }
            st.landing = Some((dst, p, now));
            st.drag = None;
            push_cue(&mut self.cues, GameCue::Equip);
        }
        // drag follow spring
        let target = drag_pos.unwrap_or(rect.center());
        let follow = spring(ui, id.with("fx"), target.x, 600.0, 32.0);
        let follow_y = spring(ui, id.with("fy"), target.y, 600.0, 32.0);
        let fpos = pos2(follow, follow_y);
        let tilt = ((target.x - follow) * 0.012).clamp(-0.35, 0.35);
        glowkit_core::set_state(ui, id.with("inv"), st.clone());
        if st.drag.is_some() || st.landing.is_some_and(|l| now - l.2 < 0.4) {
            glowkit_core::animate(ui);
        }
        let painter = ui.painter();
        frame(painter, rect, self.style, &pal, 0.0);
        if !self.title.is_empty() {
            let tc = pal.text;
            painter.text(
                pos2(rect.min.x + pad, rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                self.title.to_uppercase(),
                title_f,
                tc,
            );
            let used = self.items.iter().filter(|i| i.is_some()).count();
            painter.text(
                pos2(rect.max.x - pad, rect.min.y + 24.0),
                Align2::RIGHT_CENTER,
                format!("{used} / {n}"),
                FontId::monospace(12.0),
                pal.dim,
            );
            painter.line_segment(
                [pos2(rect.min.x + pad, rect.min.y + 40.0), pos2(rect.max.x - pad, rect.min.y + 40.0)],
                Stroke::new(1.0, with_alpha(pal.dim, 0.3)),
            );
        }
        let drop_target =
            st.drag.and(drag_pos).and_then(|p| (0..n).find(|j| cell_at(*j).expand(gap / 2.0).contains(p)));
        for i in 0..n {
            let cell = cell_at(i);
            let item = self.items[i].as_ref();
            let is_src = st.drag == Some(i);
            let lit = if drop_target == Some(i) && !is_src {
                1.0
            } else if hovered == Some(i) && st.drag.is_none() {
                0.6
            } else {
                0.0
            };
            slot(painter, cell, self.style, &pal, item.filter(|_| !is_src).map(|it| it.rarity), lit);
            if let Some(it) = item {
                if is_src {
                    it.icon.paint(painter, cell.center(), s * 0.6, with_alpha(it.tint(), 0.18));
                    continue;
                }
                let mut c = cell;
                if let Some((li, from, at)) = st.landing {
                    let k = ((now - at) as f32 / 0.32).min(1.0);
                    if li == i && k < 1.0 {
                        let e = pop(k);
                        c = cell.translate((from - cell.center()) * (1.0 - e));
                    }
                }
                paint_item(painter, c, it, 1.0, time);
            }
        }
        // dragged item on top
        let fg = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, id.with("drag_layer")));
        if let Some(src) = st.drag {
            if let Some(it) = self.items.get(src).and_then(|o| o.as_ref()) {
                let r = Rect::from_center_size(fpos, Vec2::splat(s * 1.12));
                paint::soft_shadow(
                    &fg,
                    r.translate(vec2(0.0, 10.0)),
                    8.0,
                    18.0,
                    Vec2::ZERO,
                    Color32::from_black_alpha(140),
                );
                paint::glow(&fg, r.center(), s, with_alpha(it.rarity.color(), 0.35));
                let (sn, cs) = tilt.sin_cos();
                let rot = |p: Pos2| {
                    r.center()
                        + vec2(
                            (p.x - r.center().x) * cs - (p.y - r.center().y) * sn,
                            (p.x - r.center().x) * sn + (p.y - r.center().y) * cs,
                        )
                };
                let pts: Vec<Pos2> =
                    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().map(rot).collect();
                fg.add(egui::Shape::convex_polygon(
                    pts.clone(),
                    with_alpha(mix(hex("#0E1220"), it.rarity.color(), 0.25), 0.95),
                    Stroke::new(2.0, it.rarity.color()),
                ));
                it.icon.paint(&fg, r.center(), s * 0.74, it.tint());
            }
        }
        // tooltip
        let show_tip = if st.drag.is_none() { hovered.filter(|i| self.items[*i].is_some()) } else { None };
        let tip_a = glowkit_core::anim::follow(ui, id.with("tip_a"), if show_tip.is_some() { 1.0 } else { 0.0 }, 14.0);
        let last_tip: usize = if let Some(t) = show_tip {
            glowkit_core::set_state(ui, id.with("tip_i"), t);
            t
        } else {
            glowkit_core::state(ui, id.with("tip_i"), || 0usize)
        };
        if tip_a > 0.02 {
            if let Some(it) = self.items.get(last_tip).and_then(|o| o.as_ref()) {
                let cmp = self.equipped.iter().find(|e| !it.kind.is_empty() && e.kind == it.kind && e.name != it.name);
                let cell = cell_at(last_tip);
                let screen = ui.ctx().content_rect();
                let mut pos = pos2(cell.max.x + 12.0, cell.min.y - 8.0 + (1.0 - tip_a) * 8.0);
                if pos.x + 250.0 > screen.max.x - 8.0 {
                    pos.x = cell.min.x - 262.0;
                }
                let tp = fg.clone().with_clip_rect(screen);
                let mut tp = tp;
                tp.set_opacity(tip_a);
                let r = paint_tooltip(&tp.with_clip_rect(Rect::NOTHING), pos, it, cmp, self.style, &pal, &tip_f, 1.0);
                if r.max.y > screen.max.y - 8.0 {
                    pos.y -= r.max.y - (screen.max.y - 8.0);
                }
                paint_tooltip(&tp, pos, it, cmp, self.style, &pal, &tip_f, 1.0);
            }
        }
        if let Some((g, pressed)) = ghost {
            ghost_cursor(&fg, g, 1.0, pressed);
        }
        resp
    }
}
