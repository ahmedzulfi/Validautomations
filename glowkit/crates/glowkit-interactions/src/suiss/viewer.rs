//! A full-screen photo viewer with swipe paging, double-tap zoom and swipe-down to close.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;
use glowkit_effects::PaintedScene;

#[derive(Clone, Copy, Default)]
struct ViewerState {
    open: bool,
    from: usize,
    page: f32,
    page_v: f32,
    target: usize,
    hero: f32,
    hero_v: f32,
    dismiss: Vec2,
    dismiss_v: Vec2,
    zoom: f32,
    zoom_v: f32,
    zoom_target: f32,
    focus: Vec2,
    pan: Vec2,
    pan_v: Vec2,
    axis: u8,
    g: Gesture,
    cycle: u32,
}

/// A photo grid that opens into a full-screen viewer, the way phone galleries do.
///
/// Tap a thumbnail and it grows into a black, edge-to-edge viewer (a spring "hero" transition).
/// Swipe sideways to page between photos, with rubber-band resistance at the first and last one
/// and a springy snap that carries your flick's velocity. Double-tap zooms to 2.5x around the
/// tapped point (drag to pan while zoomed), and dragging a photo down shrinks it and fades the
/// backdrop; let go far enough and it flies home to its thumbnail. A counter pill and a close
/// button sit at the top.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::SwipeImageViewer;
/// ui.add(SwipeImageViewer::new().images(6).bounce(40.0).size(egui::vec2(280.0, 520.0)));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SwipeImageViewer {
    images: Vec<egui::TextureId>,
    count: usize,
    size: Option<Vec2>,
    bounce: f32,
    zoom: f32,
    title: String,
    autoplay: bool,
}

impl Default for SwipeImageViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl SwipeImageViewer {
    /// A viewer over six painted placeholder photos.
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            count: 6,
            size: None,
            bounce: 35.0,
            zoom: 2.5,
            title: "Photos".into(),
            autoplay: false,
        }
    }
    /// Use your own textures (aspect 1.6 looks best). Overrides [`Self::images`].
    pub fn textures(mut self, t: Vec<egui::TextureId>) -> Self {
        self.images = t;
        self
    }
    /// Number of painted placeholder photos (2..=9) when no textures are given.
    pub fn images(mut self, n: usize) -> Self {
        self.count = n.clamp(2, 9);
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Overshoot of the page snap and hero spring, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Double-tap zoom factor (1.5..=4).
    pub fn zoom(mut self, z: f32) -> Self {
        self.zoom = z.clamp(1.5, 4.0);
        self
    }
    /// Title above the grid.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Play a scripted demo gesture while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const SCRIPT: [Stroke2; 11] = [
    tap(0.7, 0.5, 0.2),
    drag(2.0, 2.3, (0.8, 0.5), (0.22, 0.5)),
    drag(3.1, 3.4, (0.8, 0.5), (0.25, 0.52)),
    drag(4.2, 4.5, (0.8, 0.5), (0.25, 0.5)),
    drag(5.3, 5.9, (0.25, 0.5), (0.95, 0.52)),
    tap(7.0, 0.42, 0.46),
    tap(7.25, 0.42, 0.46),
    drag(8.1, 8.8, (0.4, 0.5), (0.62, 0.56)),
    tap(9.5, 0.5, 0.5),
    tap(9.75, 0.5, 0.5),
    drag(10.7, 11.1, (0.5, 0.45), (0.56, 0.8)),
];

impl egui::Widget for SwipeImageViewer {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let id = response.id;
        let n = if self.images.is_empty() { self.count } else { self.images.len() };
        let tex = |i: usize| {
            if self.images.is_empty() {
                PaintedScene::texture(ui.ctx(), (i * 3 + 1) % PaintedScene::COUNT)
            } else {
                self.images[i % self.images.len()]
            }
        };
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 14.0);
        let mut st: ViewerState = glowkit_core::state(ui, id.with("viewer"), || ViewerState {
            zoom: 1.0,
            zoom_target: 1.0,
            ..Default::default()
        });
        if touch.ghost && touch.cycle != st.cycle {
            st = ViewerState { zoom: 1.0, zoom_target: 1.0, cycle: touch.cycle, ..Default::default() };
        }

        // grid geometry
        let pad = 14.0;
        let gap = 6.0;
        let cols = 3;
        let cw = (rect.width() - pad * 2.0 - gap * (cols as f32 - 1.0)) / cols as f32;
        let grid_top = rect.min.y + 66.0;
        let thumb = |i: usize| {
            let (c, r) = (i % cols, i / cols);
            Rect::from_min_size(
                pos2(rect.min.x + pad + c as f32 * (cw + gap), grid_top + r as f32 * (cw + gap)),
                vec2(cw, cw),
            )
        };
        let pw = rect.width() + 16.0;
        let fit_h = rect.width() / PaintedScene::ASPECT;
        let fitted = Rect::from_center_size(rect.center(), vec2(rect.width(), fit_h));
        let close_c = pos2(rect.max.x - 26.0, rect.min.y + 44.0);

        let g = st.g.update(&touch, now, dt);
        if !st.open {
            if g.tap {
                if let Some(i) = (0..n).find(|&i| thumb(i).contains(st.g.last)) {
                    st.open = true;
                    st.from = i;
                    st.target = i;
                    st.page = i as f32;
                    st.page_v = 0.0;
                    st.dismiss = Vec2::ZERO;
                    st.zoom = 1.0;
                    st.zoom_target = 1.0;
                    st.pan = Vec2::ZERO;
                }
            }
        } else if st.hero > 0.6 {
            if g.pressed {
                st.axis = 0;
            }
            if st.g.down {
                let total = st.g.last - st.g.start;
                if st.axis == 0 && total.length() > 6.0 {
                    st.axis = if st.zoom_target > 1.01 {
                        3
                    } else if total.y.abs() > total.x.abs() * 1.2 {
                        2
                    } else {
                        1
                    };
                }
                match st.axis {
                    1 => st.page -= g.delta.x / pw,
                    2 => st.dismiss += g.delta,
                    3 => st.pan += g.delta,
                    _ => {}
                }
            }
            if g.released {
                match st.axis {
                    1 => {
                        let disp = rubber_clamp(st.page * pw, 0.0, (n - 1) as f32 * pw, pw * 0.4) / pw;
                        st.page = disp;
                        let proj = st.page - project(st.g.vel.x, 0.994) / pw;
                        let cur = st.target as f32;
                        let tgt = proj.round().clamp(cur - 1.0, cur + 1.0).clamp(0.0, (n - 1) as f32);
                        st.target = tgt as usize;
                        st.page_v = -st.g.vel.x / pw;
                    }
                    2 => {
                        if st.dismiss.y.abs() > 90.0 || st.g.vel.y.abs() > 900.0 {
                            st.open = false;
                            st.from = st.target;
                            st.hero_v = -2.0;
                        }
                        st.dismiss_v = st.g.vel;
                    }
                    _ => {}
                }
                if g.double_tap {
                    if st.zoom_target > 1.01 {
                        st.zoom_target = 1.0;
                    } else {
                        st.zoom_target = self.zoom;
                        st.focus = st.g.last - rect.center();
                        st.pan = Vec2::ZERO;
                    }
                } else if g.tap && (st.g.last - close_c).length() < 18.0 {
                    st.open = false;
                    st.from = st.target;
                }
            }
        }

        // springs
        let stiff = 260.0;
        let damp = damping_for(self.bounce, stiff);
        let hero_target = if st.open { 1.0 } else { 0.0 };
        spring_step(&mut st.hero, &mut st.hero_v, hero_target, 190.0, damping_for(self.bounce * 0.6, 190.0), dt);
        if !(st.g.down && st.axis == 1) {
            spring_step(&mut st.page, &mut st.page_v, st.target as f32, stiff, damp, dt);
        }
        if !(st.g.down && st.axis == 2) && st.open {
            spring_step(&mut st.dismiss.x, &mut st.dismiss_v.x, 0.0, stiff, damp, dt);
            spring_step(&mut st.dismiss.y, &mut st.dismiss_v.y, 0.0, stiff, damp, dt);
        }
        spring_step(&mut st.zoom, &mut st.zoom_v, st.zoom_target, 300.0, damping_for(self.bounce, 300.0), dt);
        // pan limits while zoomed
        let z = st.zoom.max(1.0);
        let lim = vec2((rect.width() * (z - 1.0) / 2.0).max(0.0), ((fit_h * z - rect.height()) / 2.0).max(0.0));
        let base = -st.focus * (z - 1.0);
        let base = vec2(base.x.clamp(-lim.x, lim.x), base.y.clamp(-lim.y, lim.y));
        let pan_total = base + st.pan;
        if !(st.g.down && st.axis == 3) {
            let want = vec2(pan_total.x.clamp(-lim.x, lim.x), pan_total.y.clamp(-lim.y, lim.y)) - base;
            spring_step(&mut st.pan.x, &mut st.pan_v.x, want.x, 300.0, 30.0, dt);
            spring_step(&mut st.pan.y, &mut st.pan_v.y, want.y, 300.0, 30.0, dt);
        }
        if st.zoom_target <= 1.01 {
            st.pan *= 0.0_f32.max(1.0 - dt * 12.0);
        }
        let moving = !settled(st.hero, st.hero_v, hero_target)
            || !settled(st.page, st.page_v * 100.0, st.target as f32)
            || st.dismiss.length() > 0.3
            || !settled(st.zoom, st.zoom_v, st.zoom_target)
            || st.g.down
            || st.pan_v.length() > 0.5;
        glowkit_core::set_state(ui, id.with("viewer"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }

        // ---- paint
        let painter = ui.painter_at(rect);
        text_bold(&painter, pos2(rect.min.x + pad, rect.min.y + 44.0), Align2::LEFT_CENTER, &self.title, 22.0, t.foreground);
        text(
            &painter,
            pos2(rect.max.x - pad, rect.min.y + 46.0),
            Align2::RIGHT_CENTER,
            format!("{n} items"),
            12.0,
            t.muted_foreground,
        );
        let hero = st.hero;
        for i in 0..n {
            let r = thumb(i);
            if hero > 0.001 && i == st.from {
                painter.rect_filled(r, cr(8.0), recess(&t));
                continue;
            }
            image_rect(&painter, r, 8.0, tex(i), cover_uv(PaintedScene::ASPECT, 1.0), Color32::WHITE);
        }
        if hero <= 0.001 {
            ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
            return response;
        }
        let drag_k = (st.dismiss.y.abs() / (rect.height() * 0.5)).clamp(0.0, 1.0);
        let back_a = hero.clamp(0.0, 1.0) * (1.0 - drag_k * 0.85);
        painter.rect_filled(rect, cr(screen_radius(ui)), with_alpha(Color32::BLACK, back_a));

        let disp_page = rubber_clamp(st.page * pw, 0.0, (n - 1) as f32 * pw, pw * 0.4) / pw;
        let shrink = 1.0 - drag_k * 0.35;
        let photo = |i: usize| -> Rect {
            let dx = (i as f32 - disp_page) * pw;
            let c = rect.center() + vec2(dx, 0.0);
            if i == st.target {
                let sz = fitted.size() * z * shrink;
                Rect::from_center_size(c + st.dismiss + pan_total * shrink, sz)
            } else {
                Rect::from_center_size(c, fitted.size())
            }
        };
        let in_transition = (hero - 1.0).abs() > 0.02 || !st.open;
        for i in 0..n {
            let mut r = photo(i);
            if !r.intersects(rect) {
                continue;
            }
            let mut uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            let mut radius = 0.0;
            if in_transition && i == st.from {
                let k = hero.clamp(0.0, 1.05);
                let a = thumb(i);
                r = Rect::from_min_max(
                    pos2(lerp(a.min.x, r.min.x, k), lerp(a.min.y, r.min.y, k)),
                    pos2(lerp(a.max.x, r.max.x, k), lerp(a.max.y, r.max.y, k)),
                );
                let cu = cover_uv(PaintedScene::ASPECT, 1.0);
                let kk = k.clamp(0.0, 1.0);
                uv = Rect::from_min_max(
                    pos2(lerp(cu.min.x, 0.0, kk), lerp(cu.min.y, 0.0, kk)),
                    pos2(lerp(cu.max.x, 1.0, kk), lerp(cu.max.y, 1.0, kk)),
                );
                radius = lerp(8.0, 0.0, kk) + drag_k * 14.0;
            } else if in_transition {
                continue;
            } else if i == st.target {
                radius = drag_k * 18.0;
            }
            image_rect(&painter, r, radius, tex(i), uv, Color32::WHITE);
        }
        // chrome
        let chrome = (hero * (1.0 - drag_k * 3.0)).clamp(0.0, 1.0) * if st.open { 1.0 } else { 0.0 };
        if chrome > 0.01 {
            let label = format!("{} of {}", st.target + 1, n);
            let w = text_width(&painter, &label, 11.5) + 18.0;
            let pill = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 44.0), vec2(w, 22.0));
            painter.rect_filled(pill, cr(11.0), with_alpha(hexa(60, 60, 64), 0.85 * chrome));
            text(&painter, pill.center(), Align2::CENTER_CENTER, label, 11.5, with_alpha(Color32::WHITE, chrome));
            painter.circle_filled(close_c, 13.0, with_alpha(hexa(60, 60, 64), 0.85 * chrome));
            glyph(&painter, Glyph::Close, close_c, 10.0, with_alpha(Color32::WHITE, chrome));
            // page dots
            let dy = rect.max.y - 30.0;
            for i in 0..n {
                let x = rect.center().x + (i as f32 - (n - 1) as f32 / 2.0) * 12.0;
                let near = (1.0 - (i as f32 - disp_page).abs()).clamp(0.0, 1.0);
                painter.circle_filled(
                    pos2(x, dy),
                    2.6 + near * 1.2,
                    with_alpha(Color32::WHITE, (0.35 + 0.65 * near) * chrome),
                );
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
