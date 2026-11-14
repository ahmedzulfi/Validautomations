//! A long-press context menu that lifts the pressed item into an enlarged preview.

use super::common::*;
use egui::epaint::RectShape;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Shape, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;
use glowkit_effects::PaintedScene;

#[derive(Clone, Default)]
struct PreviewState {
    phase: u8,
    idx: usize,
    lift: f32,
    lift_v: f32,
    menu: f32,
    menu_v: f32,
    press: f32,
    press_v: f32,
    drag: Vec2,
    hover: Option<usize>,
    flash: Option<(usize, f32)>,
    favs: Vec<bool>,
    g: Gesture,
    cycle: u32,
}

/// A grid of photos with an iOS-style "peek" context menu.
///
/// Touch and hold a photo: it sinks slightly under the finger, then the grid behind it frosts
/// over and the photo springs out of its cell into a large centered preview, its corners
/// rounding as it grows, while a menu of actions unfolds below it item by item. Keep the finger
/// down and slide onto an action to pick it on release (the preview rubber-bands after the
/// finger), or let go and tap one. Tapping anywhere else sends the preview flying back to its
/// slot.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::PreviewMenu;
/// ui.add(PreviewMenu::new().items(9).hold(0.35).actions(&["Copy", "Share", "Favorite", "Delete"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PreviewMenu {
    items: usize,
    hold: f32,
    bounce: f32,
    actions: Vec<String>,
    size: Option<Vec2>,
    title: String,
    autoplay: bool,
}

impl Default for PreviewMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewMenu {
    /// Nine photos with Copy / Share / Favorite / Delete actions.
    pub fn new() -> Self {
        Self {
            items: 9,
            hold: 0.35,
            bounce: 40.0,
            actions: ["Copy", "Share", "Favorite", "Delete"].iter().map(|s| s.to_string()).collect(),
            size: None,
            title: "Library".into(),
            autoplay: false,
        }
    }
    /// Number of photos in the grid (3..=12).
    pub fn items(mut self, n: usize) -> Self {
        self.items = n.clamp(3, 12);
        self
    }
    /// Seconds to hold before the preview lifts.
    pub fn hold(mut self, s: f32) -> Self {
        self.hold = s.clamp(0.1, 1.5);
        self
    }
    /// Overshoot of the lift, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Menu action labels (the last one is shown as destructive). "Favorite" toggles a heart.
    pub fn actions(mut self, a: &[&str]) -> Self {
        if !a.is_empty() {
            self.actions = a.iter().map(|s| s.to_string()).collect();
        }
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Heading above the grid.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Play a scripted long-press while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SCRIPT: [Stroke2; 3] = [
    hold_drag(0.8, 1.2, 2.4, (0.5, 0.352), (0.48, 0.58)),
    hold_drag(4.0, 0.9, 4.9, (0.195, 0.202), (0.2, 0.21)),
    tap(6.5, 0.5, 0.93),
];

fn glyph_for(label: &str) -> Glyph {
    match label.to_ascii_lowercase().as_str() {
        "copy" => Glyph::Copy,
        "share" => Glyph::Share,
        "favorite" | "like" => Glyph::Heart,
        "delete" | "remove" => Glyph::Trash,
        "edit" => Glyph::Edit,
        _ => Glyph::Info,
    }
}

impl egui::Widget for PreviewMenu {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 8.5);
        let mut st: PreviewState = glowkit_core::state(ui, id.with("pm"), PreviewState::default);
        if touch.ghost && touch.cycle != st.cycle {
            st = PreviewState { cycle: touch.cycle, ..Default::default() };
        }
        let n = self.items;
        st.favs.resize(n, false);

        let pad = 14.0;
        let gap = 6.0;
        let cw = (rect.width() - pad * 2.0 - gap * 2.0) / 3.0;
        let top = rect.min.y + 70.0;
        let cell = |i: usize| {
            Rect::from_min_size(
                pos2(rect.min.x + pad + (i % 3) as f32 * (cw + gap), top + (i / 3) as f32 * (cw + gap)),
                vec2(cw, cw),
            )
        };
        let pw = rect.width() - 48.0;
        let ph = pw / PaintedScene::ASPECT;
        let preview = Rect::from_min_size(pos2(rect.center().x - pw / 2.0, rect.min.y + 64.0), vec2(pw, ph));
        let item_h = 40.0;
        let menu_w = 200.0_f32.min(pw);
        let menu_top = preview.max.y + 12.0;
        let item_rect = |k: usize| {
            Rect::from_min_size(pos2(preview.min.x, menu_top + k as f32 * item_h), vec2(menu_w, item_h))
        };
        let menu_rect = Rect::from_min_size(pos2(preview.min.x, menu_top), vec2(menu_w, item_h * self.actions.len() as f32));

        let g = st.g.update(&touch, now, dt);
        let mut select: Option<usize> = None;
        match st.phase {
            0 => {
                if g.pressed {
                    if let Some(i) = (0..n).find(|&i| cell(i).contains(st.g.start)) {
                        st.phase = 1;
                        st.idx = i;
                    }
                }
            }
            1 => {
                if !st.g.down || st.g.moved > 10.0 {
                    st.phase = 0;
                } else if now - st.g.t_down > self.hold {
                    st.phase = 2;
                    st.hover = None;
                    st.lift_v = 2.0;
                }
            }
            _ => {
                if st.g.down {
                    let p = st.g.last;
                    let off = st.drag * 0.3;
                    st.hover = (0..self.actions.len()).find(|&k| item_rect(k).translate(off).contains(p));
                    if st.g.moved > 10.0 && st.hover.is_none() && !menu_rect.contains(st.g.start) {
                        let d = p - st.g.start;
                        st.drag = vec2(rubber(d.x, 30.0), rubber(d.y, 40.0));
                    }
                }
                if g.released {
                    let p = st.g.last;
                    if let Some(k) = (0..self.actions.len()).find(|&k| item_rect(k).translate(st.drag * 0.3).contains(p)) {
                        select = Some(k);
                    } else if g.tap && !preview.contains(p) {
                        st.phase = 0;
                    }
                    st.hover = None;
                }
            }
        }
        if let Some(k) = select {
            st.flash = Some((k, now));
            if self.actions[k].eq_ignore_ascii_case("favorite") {
                let i = st.idx;
                st.favs[i] = !st.favs[i];
            }
        }
        if let Some((_, t0)) = st.flash {
            if now - t0 > 0.18 && st.phase == 2 {
                st.phase = 0;
            }
            if now - t0 > 1.0 {
                st.flash = None;
            }
        }
        if !st.g.down {
            st.drag *= (1.0 - dt * 14.0).max(0.0);
        }

        let open = st.phase == 2;
        let stiff = 240.0;
        spring_step(&mut st.lift, &mut st.lift_v, if open { 1.0 } else { 0.0 }, stiff, damping_for(self.bounce, stiff), dt);
        spring_step(&mut st.menu, &mut st.menu_v, if open { 1.0 } else { 0.0 }, 300.0, damping_for(self.bounce, 300.0), dt);
        let press_t = if st.phase == 1 { 1.0 } else { 0.0 };
        spring_step(&mut st.press, &mut st.press_v, press_t, 200.0, 20.0, dt);
        let moving = st.phase != 0
            || st.lift > 0.002
            || st.menu.abs() > 0.002
            || st.press > 0.002
            || st.flash.is_some()
            || st.drag.length() > 0.2;
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("pm"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        text_bold(&painter, pos2(rect.min.x + pad, rect.min.y + 44.0), Align2::LEFT_CENTER, &self.title, 22.0, t.foreground);
        let tex = |i: usize| PaintedScene::texture(ui.ctx(), (i * 5 + 2) % PaintedScene::COUNT);
        let lift = s.lift.max(0.0);
        let blur = lift.clamp(0.0, 1.0);
        let bg = if t.dark { t.background } else { Color32::WHITE };
        for i in 0..n {
            let r = cell(i);
            if lift > 0.001 && i == s.idx {
                continue;
            }
            let r = if i == s.idx { Rect::from_center_size(r.center(), r.size() * (1.0 - 0.06 * s.press)) } else { r };
            image_rect(&painter, r, 10.0, tex(i), cover_uv(PaintedScene::ASPECT, 1.0), with_alpha(Color32::WHITE, 1.0 - 0.8 * blur));
            if s.favs.get(i).copied().unwrap_or(false) {
                let c = r.left_bottom() + vec2(14.0, -14.0);
                painter.circle_filled(c, 10.0, with_alpha(Color32::BLACK, 0.35));
                glyph(&painter, Glyph::Heart, c, 11.0, Color32::WHITE);
            }
        }
        // frosted backdrop: blurred color blobs of each tile plus a veil
        if blur > 0.01 {
            let veil = with_alpha(bg, if t.dark { 0.45 } else { 0.5 } * blur);
            for i in 0..n {
                let c = PaintedScene::accent((i * 5 + 2) % PaintedScene::COUNT);
                painter.add(Shape::Rect(
                    RectShape::filled(cell(i).shrink(2.0), cr(14.0), with_alpha(mix(c, bg, 0.25), 0.9 * blur))
                        .with_blur_width(30.0 * blur),
                ));
            }
            painter.rect_filled(rect, cr(screen_radius(ui)), veil);
        }
        if lift > 0.001 {
            let a = cell(s.idx);
            let k = lift;
            let center = a.center().lerp(preview.center(), k) + s.drag;
            let sz = vec2(lerp(a.width(), preview.width(), k), lerp(a.height(), preview.height(), k));
            let r = Rect::from_center_size(center, sz);
            let cu = cover_uv(PaintedScene::ASPECT, 1.0);
            let kk = k.clamp(0.0, 1.0);
            let uv = Rect::from_min_max(
                pos2(lerp(cu.min.x, 0.0, kk), 0.0),
                pos2(lerp(cu.max.x, 1.0, kk), 1.0),
            );
            shadow(&painter, &t, r, 16.0, 1.4 * kk);
            image_rect(&painter, r, lerp(10.0, 16.0, kk), tex(s.idx), uv, Color32::WHITE);
            if s.favs.get(s.idx).copied().unwrap_or(false) {
                let c = r.left_bottom() + vec2(18.0, -18.0);
                painter.circle_filled(c, 12.0, with_alpha(Color32::BLACK, 0.35));
                glyph(&painter, Glyph::Heart, c, 13.0, Color32::WHITE);
            }
        }
        // menu
        let m = s.menu;
        if m > 0.01 {
            let mfill = if t.dark { hexa(0x2A, 0x2A, 0x2E) } else { hexa(0xF2, 0xF2, 0xF5) };
            let grow = m.clamp(0.0, 1.2);
            let h = menu_rect.height() * grow.min(1.0) + (grow - 1.0).max(0.0) * 10.0;
            let w = menu_rect.width() * (0.6 + 0.4 * grow);
            let mr = Rect::from_min_size(menu_rect.min + s.drag * 0.3, vec2(w, h.max(1.0)));
            shadow(&painter, &t, mr, 14.0, (m * 2.0).clamp(0.0, 1.0));
            painter.rect_filled(mr, cr(14.0), with_alpha(mfill, (m * 3.0).clamp(0.0, 1.0)));
            let clip = painter.with_clip_rect(mr);
            for (k, label) in self.actions.iter().enumerate() {
                let ir = item_rect(k).translate(s.drag * 0.3);
                let appear = ramp(m * self.actions.len() as f32 - k as f32 * 0.6, 0.0, 1.0);
                let destructive = k + 1 == self.actions.len();
                let col = if destructive {
                    if t.dark {
                        hexa(0xFF, 0x45, 0x3A)
                    } else {
                        hexa(0xE5, 0x30, 0x2A)
                    }
                } else {
                    t.foreground
                };
                let flash = s.flash.is_some_and(|(f, _)| f == k);
                if s.hover == Some(k) || flash {
                    let hr = ir.shrink2(vec2(0.0, 0.0));
                    clip.rect_filled(hr, cr(if k == 0 { 14.0 } else { 0.0 }), mix(mfill, t.foreground, 0.1));
                }
                if k > 0 {
                    clip.line_segment(
                        [pos2(ir.min.x, ir.min.y), pos2(ir.max.x, ir.min.y)],
                        egui::Stroke::new(1.0, with_alpha(t.foreground, 0.08)),
                    );
                }
                let dy = (1.0 - appear) * 6.0;
                text(&clip, pos2(ir.min.x + 16.0, ir.center().y + dy), Align2::LEFT_CENTER, label, 14.0, with_alpha(col, appear));
                glyph(&clip, glyph_for(label), pos2(ir.max.x - 22.0, ir.center().y + dy), 15.0, with_alpha(col, appear));
            }
        }
        if s.phase == 0 && lift < 0.01 {
            text(
                &painter,
                pos2(rect.min.x + pad, cell(n - 1).max.y + 22.0),
                Align2::LEFT_CENTER,
                "Touch and hold a photo.",
                12.0,
                t.muted_foreground,
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
