//! A scrolling list with a search field tucked above it that appears when you pull down.

use super::common::*;
use super::reorder::TouchRow;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

#[derive(Clone, Default)]
struct SearchState {
    sc: Scroller,
    snap_v: f32,
    focused: bool,
    query: String,
    heights: Vec<(f32, f32)>,
    g: Gesture,
    cycle: u32,
    init: bool,
}

/// A notes list with the search field hidden above it, the way phone lists do it.
///
/// The list starts scrolled so the field is just out of sight under the title. Pull the list
/// down and the field slides out, fading and unfolding as it comes; stop halfway and it snaps
/// either fully open or fully shut. The list itself scrolls with flick momentum and
/// rubber-bands at both ends. Tap the field to search (type on the keyboard): rows that don't
/// match fold away on springs, and the field stays put while it holds a query.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{SearchReveal, TouchRow};
/// let rows = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
/// ui.add(SearchReveal::new(&rows).title("Notes"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SearchReveal<'a> {
    rows: &'a [TouchRow],
    title: String,
    placeholder: String,
    size: Option<Vec2>,
    snap: f32,
    autoplay: bool,
}

impl<'a> SearchReveal<'a> {
    /// A searchable list over `rows`.
    pub fn new(rows: &'a [TouchRow]) -> Self {
        Self { rows, title: "Notes".into(), placeholder: "Search".into(), size: None, snap: 50.0, autoplay: false }
    }
    /// Large title above the list.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Placeholder text of the field.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// Widget size (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Snap stiffness when a half-revealed field settles, 0..100.
    pub fn snap(mut self, s: f32) -> Self {
        self.snap = s.clamp(0.0, 100.0);
        self
    }
    /// Pull, search and flick on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

/// Sample notes for demos.
pub(crate) fn sample_long_notes() -> Vec<TouchRow> {
    vec![
        TouchRow::new("Groceries", "Oat milk, eggs, sourdough, coffee"),
        TouchRow::new("Trip ideas", "Lisbon, Porto and the coast road"),
        TouchRow::new("Book list", "Three novels for the summer"),
        TouchRow::new("Meeting notes", "Ship the beta on Friday"),
        TouchRow::new("Gift ideas", "Headphones, a plant, a scarf"),
        TouchRow::new("Recipes", "Lemon pasta and a quick curry"),
        TouchRow::new("Garden", "Tomatoes need a trellis"),
        TouchRow::new("Workout", "Intervals, then a long stretch"),
        TouchRow::new("Podcasts", "Two episodes on typography"),
        TouchRow::new("Weekend", "Market, then a long walk"),
    ]
}

const BAND: f32 = 52.0;
const ROW_H: f32 = 58.0;
const SCRIPT: [Stroke2; 5] = [
    drag(0.8, 1.5, (0.5, 0.4), (0.5, 0.47)),
    tap(2.4, 0.4, 0.174),
    tap(5.0, 0.87, 0.174),
    drag(6.0, 6.22, (0.5, 0.78), (0.5, 0.36)),
    drag(7.8, 8.02, (0.5, 0.3), (0.5, 0.74)),
];

impl egui::Widget for SearchReveal<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let touch = touch(ui, &response, rect, self.autoplay, &SCRIPT, 10.0);
        let mut st: SearchState = glowkit_core::state(ui, id.with("search"), SearchState::default);
        if !st.init || (touch.ghost && touch.cycle != st.cycle) {
            st = SearchState { init: true, cycle: touch.cycle, sc: Scroller { s: BAND, v: 0.0 }, ..Default::default() };
        }
        let n = self.rows.len();
        st.heights.resize(n, (1.0, 0.0));

        let view = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 66.0), rect.max);
        let field_rel = |s: f32| {
            Rect::from_min_size(pos2(view.min.x + 14.0, view.min.y + 8.0 - s), vec2(view.width() - 28.0, 36.0))
        };
        let content_h = BAND + st.heights.iter().map(|h| h.0.clamp(0.0, 1.0) * ROW_H).sum::<f32>() + 16.0;
        let max = (content_h - view.height()).max(BAND);

        let g = st.g.update(&touch, now, dt);
        if st.g.down {
            if !g.pressed {
                st.sc.drag(g.delta.y, 0.0, max);
            }
            st.sc.v = 0.0;
            st.snap_v = 0.0;
        }
        if g.released && !g.tap {
            st.sc.v = -st.g.vel.y;
        }
        let field = field_rel(st.sc.s);
        let clear_c = pos2(field.max.x - 16.0, field.center().y);
        if g.tap {
            let p = st.g.last;
            if !st.query.is_empty() && (p - clear_c).length() < 16.0 {
                st.query.clear();
                st.focused = false;
            } else {
                st.focused = field.contains(p) && st.sc.s < BAND * 0.5;
            }
        }
        if touch.ghost {
            st.focused = (2.45..5.0).contains(&touch.t);
            st.query = if (2.9..3.25).contains(&touch.t) {
                "t".into()
            } else if (3.25..5.0).contains(&touch.t) {
                "tr".into()
            } else {
                String::new()
            };
        } else if st.focused {
            for e in ui.input(|i| i.events.clone()) {
                match e {
                    egui::Event::Text(s) => st.query.push_str(&s),
                    egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                        st.query.pop();
                    }
                    egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                        st.query.clear();
                        st.focused = false;
                    }
                    _ => {}
                }
            }
        }
        // physics
        let mut moving = st.g.down;
        if !st.g.down {
            moving |= st.sc.step(0.0, max, dt);
            let s = st.sc.s;
            let pinned = !st.query.is_empty() || st.focused;
            if st.sc.v.abs() < 80.0 && s > 0.0 && s < BAND && (pinned || s > 0.3 && s < BAND - 0.3) {
                let target = if pinned || s < BAND * 0.5 { 0.0 } else { BAND };
                let k = 120.0 + self.snap * 3.0;
                spring_step(&mut st.sc.s, &mut st.snap_v, target, k, 2.0 * k.sqrt() * 0.9, dt);
                st.sc.v = 0.0;
                moving = true;
            }
        }
        let q = st.query.to_lowercase();
        for (i, r) in self.rows.iter().enumerate() {
            let show = q.is_empty() || r.title.to_lowercase().contains(&q) || r.detail.to_lowercase().contains(&q);
            let h = &mut st.heights[i];
            let target = if show { 1.0 } else { 0.0 };
            spring_step(&mut h.0, &mut h.1, target, 300.0, 30.0, dt);
            moving |= !settled(h.0 * 100.0, h.1 * 100.0, target * 100.0);
        }
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("search"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        } else if s.focused {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(400));
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        // paint
        let painter = ui.painter_at(rect);
        text_bold(
            &painter,
            pos2(rect.min.x + 18.0, rect.min.y + 46.0),
            Align2::LEFT_CENTER,
            &self.title,
            24.0,
            t.foreground,
        );
        let clip = painter.with_clip_rect(view);
        let scroll = s.sc.s;
        // field: unfolds as it comes into view
        let reveal = (1.0 - scroll / BAND).clamp(0.0, 1.0);
        let over = (-scroll).max(0.0);
        let fh = 36.0 * (0.55 + 0.45 * reveal);
        let field = Rect::from_min_size(
            pos2(view.min.x + 14.0, view.min.y + 8.0 - scroll.min(BAND) * 0.5 + over * 0.5 - (36.0 - fh) * 0.0),
            vec2(view.width() - 28.0, fh),
        );
        let fa = ((reveal - 0.25) / 0.75).clamp(0.0, 1.0);
        if fa > 0.0 {
            let ff = mix(recess(&t), t.foreground, if t.dark { 0.02 } else { 0.0 });
            clip.rect_filled(field, cr(10.0), with_alpha(ff, fa));
            if s.focused {
                clip.rect_stroke(
                    field,
                    cr(10.0),
                    egui::Stroke::new(1.5, with_alpha(system_blue(&t), fa)),
                    egui::StrokeKind::Inside,
                );
            }
            let gc = pos2(field.min.x + 16.0, field.center().y);
            glyph(&clip, Glyph::Search, gc, 13.0 * (0.7 + 0.3 * fa), with_alpha(t.muted_foreground, fa));
            let tx = field.min.x + 30.0;
            if s.query.is_empty() {
                text(
                    &clip,
                    pos2(tx, field.center().y),
                    Align2::LEFT_CENTER,
                    &self.placeholder,
                    14.0,
                    with_alpha(t.muted_foreground, fa),
                );
            } else {
                let r = text(
                    &clip,
                    pos2(tx, field.center().y),
                    Align2::LEFT_CENTER,
                    &s.query,
                    14.0,
                    with_alpha(t.foreground, fa),
                );
                if s.focused && (now * 2.0).fract() < 0.55 {
                    clip.line_segment(
                        [pos2(r.max.x + 2.0, r.min.y), pos2(r.max.x + 2.0, r.max.y)],
                        egui::Stroke::new(1.6, system_blue(&t)),
                    );
                }
                let cc = pos2(field.max.x - 16.0, field.center().y);
                clip.circle_filled(cc, 8.0, with_alpha(t.muted_foreground, 0.6 * fa));
                glyph(&clip, Glyph::Close, cc, 6.0, with_alpha(surface(&t), fa));
            }
            if s.focused && s.query.is_empty() && (now * 2.0).fract() < 0.55 {
                clip.line_segment(
                    [pos2(tx - 1.0, field.center().y - 9.0), pos2(tx - 1.0, field.center().y + 9.0)],
                    egui::Stroke::new(1.6, system_blue(&t)),
                );
            }
        }
        // list card
        let top = view.min.y + BAND - scroll;
        let total: f32 = s.heights.iter().map(|h| h.0.clamp(0.0, 1.0) * ROW_H).sum();
        let list = Rect::from_min_size(pos2(view.min.x + 12.0, top), vec2(view.width() - 24.0, total.max(0.0)));
        if total > 1.0 {
            card(&clip, &t, list, 14.0, 0.4, surface(&t));
        }
        let list_clip = clip.with_clip_rect(list.intersect(view));
        let mut y = top;
        let shown: Vec<usize> = (0..n).filter(|&i| s.heights[i].0 > 0.01).collect();
        for (k, &i) in shown.iter().enumerate() {
            let h = s.heights[i].0.clamp(0.0, 1.0) * ROW_H;
            let row = Rect::from_min_size(pos2(list.min.x, y), vec2(list.width(), h));
            y += h;
            if !row.intersects(view) {
                continue;
            }
            let a = s.heights[i].0.clamp(0.0, 1.0);
            let rc = list_clip.with_clip_rect(row.intersect(list).intersect(view));
            let r = &self.rows[i];
            let cy = row.min.y + ROW_H / 2.0 - (1.0 - a) * ROW_H / 2.0;
            text_bold(
                &rc,
                pos2(row.min.x + 16.0, cy - 9.0),
                Align2::LEFT_CENTER,
                &r.title,
                14.0,
                with_alpha(t.foreground, a),
            );
            text(
                &rc,
                pos2(row.min.x + 16.0, cy + 10.0),
                Align2::LEFT_CENTER,
                &r.detail,
                12.0,
                with_alpha(t.muted_foreground, a),
            );
            if k + 1 < shown.len() {
                rc.line_segment(
                    [pos2(row.min.x + 16.0, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)],
                    egui::Stroke::new(1.0, hairline(&t)),
                );
            }
        }
        if total < 1.0 && !s.query.is_empty() {
            text(
                &clip,
                pos2(view.center().x, top + 40.0),
                Align2::CENTER_CENTER,
                format!("No results for “{}”", s.query),
                13.0,
                t.muted_foreground,
            );
        }
        // a slim scroll indicator while moving
        if moving && max > BAND {
            let frac = view.height() / content_h;
            let bh = (view.height() * frac).max(30.0);
            let pos = ((scroll - 0.0) / max).clamp(0.0, 1.0);
            let by = view.min.y + 4.0 + pos * (view.height() - bh - 8.0);
            painter.rect_filled(
                Rect::from_min_size(pos2(view.max.x - 5.0, by), vec2(3.0, bh)),
                cr(1.5),
                with_alpha(mix(t.muted_foreground, Color32::GRAY, 0.5), 0.5),
            );
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
