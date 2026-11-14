//! A navigation trail: breadcrumbs, a back link with history, and an interactive edge-swipe back.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Default)]
struct TrailState {
    stack: Vec<String>,
    depth: usize,
    disp: f32,
    disp_v: f32,
    edge: bool,
    edge_start: f32,
    hold_back: bool,
    menu: f32,
    menu_v: f32,
    menu_open: bool,
    g: Gesture,
    cycle: u32,
}

/// A navigation stack that always shows where you are.
///
/// Each page has a back link naming its parent and, under its large title, a breadcrumb trail of
/// every ancestor (long trails collapse their middle into "…"). Tap a row to push a deeper page:
/// it slides in from the right while the old page parallaxes away under a shadow. Go back by
/// tapping the back link, tapping any crumb, or swiping from the left edge: the page follows
/// your finger and either completes the pop or springs back, depending on distance and speed.
/// Press and hold the back link to open a history menu of all ancestors and jump straight to
/// one. The current path is written to the `Vec<String>` you pass in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::TrailNav;
/// let mut path = vec!["Settings".to_string()];
/// ui.add(TrailNav::new(&mut path).page("Settings", &["Privacy", "Display"]).page("Privacy", &["Camera"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TrailNav<'a> {
    path: &'a mut Vec<String>,
    pages: Vec<(String, Vec<String>)>,
    size: Option<Vec2>,
    parallax: f32,
    bounce: f32,
    autoplay: bool,
}

impl<'a> TrailNav<'a> {
    /// A trail editing `path` (the first entry is the root page).
    pub fn new(path: &'a mut Vec<String>) -> Self {
        Self { path, pages: Vec::new(), size: None, parallax: 30.0, bounce: 15.0, autoplay: false }
    }
    /// Declare a page and its child pages (pages without children show a few options).
    pub fn page(mut self, title: impl Into<String>, children: &[&str]) -> Self {
        self.pages.push((title.into(), children.iter().map(|s| s.to_string()).collect()));
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// How far the covered page drifts during a transition, 0..100 (% of width).
    pub fn parallax(mut self, p: f32) -> Self {
        self.parallax = p.clamp(0.0, 100.0);
        self
    }
    /// Overshoot of page transitions, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Navigate on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn default_pages() -> Vec<(String, Vec<String>)> {
    let p = |t: &str, c: &[&str]| (t.to_string(), c.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    vec![
        p("Settings", &["Privacy & Security", "Notifications", "Display"]),
        p("Privacy & Security", &["Location Services", "Camera", "Contacts"]),
        p("Location Services", &["Maps", "Weather", "Photos"]),
        p("Notifications", &["Mail", "Messages"]),
        p("Display", &["Text Size", "Appearance"]),
    ]
}

const SCRIPT: [Stroke2; 7] = [
    tap(0.8, 0.5, 0.30),
    tap(2.0, 0.5, 0.30),
    tap(3.2, 0.5, 0.39),
    hold_drag(4.4, 0.05, 5.0, (0.03, 0.55), (0.78, 0.56)),
    hold_drag(6.0, 0.6, 6.75, (0.13, 0.087), (0.13, 0.088)),
    tap(7.7, 0.3, 0.174),
    tap(8.9, 0.5, 0.48),
];

const OPTIONS: [&str; 3] = ["Always", "While Using", "Never"];

impl egui::Widget for TrailNav<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let pages = if self.pages.is_empty() { default_pages() } else { self.pages.clone() };
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 10.5);
        let mut st: TrailState = glowkit_core::state(ui, id.with("trail"), TrailState::default);
        if self.path.is_empty() {
            self.path.push(pages.first().map(|p| p.0.clone()).unwrap_or_else(|| "Home".into()));
        }
        if touch.ghost && touch.cycle != st.cycle {
            st.cycle = touch.cycle;
            self.path.truncate(1);
            st.menu_open = false;
        }
        // keep the stack in sync with the caller's path (it may have changed outside)
        if st.stack.len() < self.path.len() || st.stack[..self.path.len()] != self.path[..] {
            st.stack = self.path.clone();
            st.disp = (self.path.len() - 1) as f32;
        }
        st.depth = self.path.len() - 1;
        let w = rect.width();
        let children = |title: &str| pages.iter().find(|p| p.0 == title).map(|p| p.1.clone()).unwrap_or_default();
        let row_h = 48.0;
        let rows_top = rect.min.y + 136.0;
        let back_r = Rect::from_min_size(pos2(rect.min.x + 8.0, rect.min.y + 32.0), vec2(w * 0.55, 28.0));
        let menu_row = |i: usize| {
            Rect::from_min_size(pos2(rect.min.x + 12.0, rect.min.y + 66.0 + i as f32 * 40.0), vec2(200.0, 40.0))
        };

        let crumbs = |stack: &[String], depth: usize, painter: &egui::Painter| -> Vec<(usize, Rect, String)> {
            let mut out = Vec::new();
            let mut x = rect.min.x + 18.0;
            let y = rect.min.y + 112.0;
            let items: Vec<usize> =
                if depth >= 3 { vec![0, usize::MAX, depth - 1, depth] } else { (0..=depth).collect() };
            for k in items {
                let label = if k == usize::MAX { "…".to_string() } else { stack[k].clone() };
                let tw = text_width(painter, &label, 12.0);
                out.push((k, Rect::from_min_size(pos2(x, y - 10.0), vec2(tw, 20.0)), label));
                x += tw + 18.0;
            }
            out
        };
        let painter = ui.painter_at(rect);

        let g = st.g.update(&touch, now, dt);
        if g.pressed {
            st.edge = st.depth > 0 && st.g.start.x < rect.min.x + 26.0 && !st.menu_open;
            st.edge_start = st.disp;
            st.hold_back = st.depth > 0 && back_r.contains(st.g.start) && !st.menu_open;
        }
        if st.g.down && st.edge {
            let dx = (st.g.last.x - st.g.start.x).max(0.0);
            st.disp = (st.depth as f32 - dx / w).max(st.depth as f32 - 1.0);
            st.disp_v = 0.0;
        }
        if st.g.down && st.hold_back && st.g.moved < 10.0 && now - st.g.t_down > 0.45 && !st.menu_open {
            st.menu_open = true;
            st.hold_back = false;
            st.menu_v = 3.0;
        }
        let mut new_depth: Option<usize> = None;
        let mut push: Option<String> = None;
        if g.released {
            let p = st.g.last;
            if st.edge {
                let dx = (p.x - st.g.start.x).max(0.0);
                if dx > w * 0.4 || st.g.vel.x > 500.0 {
                    new_depth = Some(st.depth - 1);
                }
                st.disp_v = -st.g.vel.x / w;
            } else if st.menu_open {
                if g.tap || !st.hold_back {
                    let anc = st.depth;
                    if let Some(i) = (0..anc).find(|&i| menu_row(i).contains(p)) {
                        new_depth = Some(anc - 1 - i);
                    }
                    if g.tap {
                        st.menu_open = false;
                    }
                }
            } else if g.tap {
                if st.hold_back && back_r.contains(p) {
                    new_depth = Some(st.depth - 1);
                } else if let Some((k, _, _)) = crumbs(&st.stack, st.depth, &painter)
                    .into_iter()
                    .find(|(k, r, _)| r.expand(4.0).contains(p) && *k != usize::MAX)
                {
                    if k < st.depth {
                        new_depth = Some(k);
                    }
                } else if p.y > rows_top {
                    let i = ((p.y - rows_top) / row_h) as usize;
                    let kids = children(&st.stack[st.depth]);
                    if let Some(c) = kids.get(i) {
                        push = Some(c.clone());
                    } else if kids.is_empty() && i < OPTIONS.len() {
                        let key = id.with(("opt", st.stack[st.depth].clone()));
                        glowkit_core::set_state(ui, key, i);
                    }
                }
            }
            st.edge = false;
            st.hold_back = false;
        }
        if let Some(c) = push {
            st.stack.truncate(st.depth + 1);
            st.stack.push(c);
            st.depth += 1;
        }
        if let Some(d) = new_depth {
            st.depth = d;
        }
        if self.path.len() != st.depth + 1 || self.path[..] != st.stack[..=st.depth] {
            *self.path = st.stack[..=st.depth].to_vec();
            response.mark_changed();
        }
        if !(st.g.down && st.edge) {
            spring_step(&mut st.disp, &mut st.disp_v, st.depth as f32, 220.0, damping_for(self.bounce, 220.0), dt);
        }
        let mt = if st.menu_open { 1.0 } else { 0.0 };
        spring_step(&mut st.menu, &mut st.menu_v, mt, 360.0, damping_for(40.0, 360.0), dt);
        let moving =
            st.g.down || !settled(st.disp, st.disp_v * 100.0, st.depth as f32) || !settled(st.menu, st.menu_v, mt);
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("trail"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint pages: the two around the displayed depth
        let bg = if t.dark { t.background } else { hexa(0xF4, 0xF4, 0xF6) };
        let blue = system_blue(&t);
        let lo = s.disp.floor().max(0.0) as usize;
        let hi = (lo + 1).min(s.stack.len() - 1);
        let draw_page = |d: usize| {
            let rel = d as f32 - s.disp;
            let x = if rel >= 0.0 { rel * w } else { rel * w * self.parallax / 100.0 };
            let page = rect.translate(vec2(x, 0.0));
            if !page.intersects(rect) {
                return;
            }
            if rel > 0.0 {
                shadow(&painter, &t, page, 0.0, 0.8 * (1.0 - rel).clamp(0.0, 1.0));
            }
            painter.rect_filled(page, cr(screen_radius(ui)), bg);
            let title = &s.stack[d];
            if d > 0 {
                glyph(&painter, Glyph::ChevronRight, pos2(page.min.x + 18.0, page.min.y + 46.0), -12.0, blue);
                text(
                    &painter,
                    pos2(page.min.x + 28.0, page.min.y + 46.0),
                    Align2::LEFT_CENTER,
                    &s.stack[d - 1],
                    14.5,
                    blue,
                );
            }
            text_bold(
                &painter,
                pos2(page.min.x + 18.0, page.min.y + 84.0),
                Align2::LEFT_CENTER,
                title,
                22.0,
                t.foreground,
            );
            for (k, r, label) in crumbs(&s.stack, d, &painter) {
                let r = r.translate(vec2(x, 0.0));
                let current = k == d;
                text(
                    &painter,
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    &label,
                    12.0,
                    if current {
                        t.foreground
                    } else if k == usize::MAX {
                        t.muted_foreground
                    } else {
                        blue
                    },
                );
                if !current {
                    text(
                        &painter,
                        pos2(r.max.x + 9.0, r.center().y),
                        Align2::CENTER_CENTER,
                        "›",
                        12.0,
                        t.muted_foreground,
                    );
                }
            }
            let kids = children(title);
            let opts: Vec<String> =
                if kids.is_empty() { OPTIONS.iter().map(|s| s.to_string()).collect() } else { kids.clone() };
            let list =
                Rect::from_min_size(pos2(page.min.x + 12.0, rows_top), vec2(w - 24.0, row_h * opts.len() as f32));
            card(&painter, &t, list, 12.0, 0.3, surface(&t));
            let sel: usize = glowkit_core::state(ui, id.with(("opt", title.clone())), || 1);
            for (i, o) in opts.iter().enumerate() {
                let y = rows_top + i as f32 * row_h + row_h / 2.0;
                text(&painter, pos2(list.min.x + 16.0, y), Align2::LEFT_CENTER, o, 14.5, t.foreground);
                if kids.is_empty() {
                    if i == sel {
                        glyph(&painter, Glyph::Check, pos2(list.max.x - 22.0, y), 13.0, blue);
                    }
                } else {
                    glyph(&painter, Glyph::ChevronRight, pos2(list.max.x - 18.0, y), 10.0, t.muted_foreground);
                }
                if i + 1 < opts.len() {
                    painter.line_segment(
                        [pos2(list.min.x + 16.0, y + row_h / 2.0), pos2(list.max.x, y + row_h / 2.0)],
                        egui::Stroke::new(1.0, hairline(&t)),
                    );
                }
            }
            if rel < 0.0 {
                painter.rect_filled(page, cr(screen_radius(ui)), with_alpha(Color32::BLACK, 0.12 * (-rel).min(1.0)));
            }
        };
        draw_page(lo);
        if hi != lo && (s.disp - lo as f32) > 0.001 {
            draw_page(hi);
        }
        // history menu
        if s.menu > 0.01 {
            let anc = s.depth;
            let k = s.menu.clamp(0.0, 1.2);
            let full = Rect::from_min_size(menu_row(0).min, vec2(200.0, 40.0 * anc as f32));
            let r = Rect::from_min_size(full.min, vec2(full.width() * (0.7 + 0.3 * k.min(1.0)), full.height() * k));
            let fill = if t.dark { hexa(0x2C, 0x2C, 0x31) } else { Color32::WHITE };
            card(&painter, &t, r, 12.0, k.min(1.0), with_alpha(fill, k.min(1.0)));
            let clip = painter.with_clip_rect(r);
            for i in 0..anc {
                let mr = menu_row(i);
                let a = ((k - i as f32 * 0.12) * 1.5).clamp(0.0, 1.0);
                let label = &s.stack[anc - 1 - i];
                text(
                    &clip,
                    pos2(mr.min.x + 14.0, mr.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    14.0,
                    with_alpha(t.foreground, a),
                );
                if i + 1 < anc {
                    clip.line_segment(
                        [pos2(mr.min.x, mr.max.y), pos2(mr.max.x, mr.max.y)],
                        egui::Stroke::new(1.0, with_alpha(mix(fill, t.foreground, 0.1), a)),
                    );
                }
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
