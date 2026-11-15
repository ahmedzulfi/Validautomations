use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{hex, mix, with_alpha};
use glowkit_core::{paint, Rng, Theme};

/// A place listed under the map of a [`LocationResult`].
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    /// Place name.
    pub name: String,
    /// Address or short description.
    pub detail: String,
    /// Right-aligned meta (distance, ETA).
    pub meta: String,
}

impl Place {
    /// A place row.
    pub fn new(name: impl Into<String>, detail: impl Into<String>, meta: impl Into<String>) -> Self {
        Self { name: name.into(), detail: detail.into(), meta: meta.into() }
    }
}

/// A location answer with a painted map: soft land, a river, parks and a street grid generated
/// from a seed (no tiles needed), a route that draws itself from "you" to the destination with a
/// travelling dot, and a pin that drops in with a bounce. Below sits a list of places; hovering a
/// row moves the pin to that place along the map.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{LocationResult, Place};
/// let places = [Place::new("Blue Door Coffee", "14 Harbor St", "6 min walk")];
/// let r = LocationResult::new("Coffee near you", &places).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct LocationResult<'a> {
    title: &'a str,
    places: &'a [Place],
    seed: u64,
    selected: Option<usize>,
    width: Option<f32>,
    map_height: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> LocationResult<'a> {
    /// A map titled `title` with a list of `places` (the first is the default destination).
    pub fn new(title: &'a str, places: &'a [Place]) -> Self {
        Self { title, places, seed: 7, selected: None, width: None, map_height: 190.0, salt: None }
    }
    /// Seed of the painted map layout.
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    /// Force the selected place (for autoplay); otherwise hovering rows selects.
    pub fn selected(mut self, i: Option<usize>) -> Self {
        self.selected = i;
        self
    }
    /// Card width (default: available, at most 460).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Map height (default 190).
    pub fn map_height(mut self, h: f32) -> Self {
        self.map_height = h.max(80.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the card. Returns the response and the rects of the place rows.
    pub fn show(self, ui: &mut Ui) -> (Response, Vec<Rect>) {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_location");
        let width = self.width.unwrap_or_else(|| ui.available_width().min(460.0));
        let e = since_change(ui, id, hash_str(self.title) ^ self.seed);
        let time = glowkit_core::time(ui);
        glowkit_core::animate(ui);
        let hovered: Option<usize> = glowkit_core::state(ui, id.with("hov"), || None);
        let sel = self.selected.or(hovered).unwrap_or(0).min(self.places.len().saturating_sub(1));
        let sel_e = since_change(ui, id.with("sel"), sel as u64);
        let places = self.places;
        let seed = self.seed;
        let map_h = self.map_height;
        let mut rows = Vec::new();
        let mut new_hover = None;
        let head = ToolHead { icon: Icon::Globe, title: self.title, subtitle: None, status: Some(("maps.search", t.info)) };
        let rect = tool_card(ui, appear_curve(e), Some(head), width, |ui| {
            let w = ui.available_width();
            let (mr, _) = ui.allocate_exact_size(vec2(w, map_h), Sense::hover());
            let p = ui.painter().with_clip_rect(mr.intersect(ui.clip_rect()));
            let layout = MapLayout::new(seed, mr, places.len());
            layout.paint(&p, &t, mr);
            // route from "you" to the selected place
            let you = layout.you;
            let dest = layout.spots[sel];
            let route = layout.route(you, dest);
            let draw = ((sel_e.min(e - 0.3)) / 0.9).clamp(0.0, 1.0);
            let draw = egui::emath::easing::cubic_out(draw);
            let n = ((route.len() as f32) * draw) as usize;
            if n > 1 {
                p.add(Shape::line(route[..n].to_vec(), Stroke::new(6.0, with_alpha(t.info, 0.18))));
                paint::gradient_polyline(&p, &route[..n], false, 3.2, |u| mix(hex("#60A5FA"), t.info, u));
            }
            if draw >= 1.0 {
                let k = (time * 0.35).fract();
                let i = ((route.len() - 1) as f32 * k) as usize;
                p.circle_filled(route[i], 3.0, Color32::WHITE);
            }
            // you
            let pulse = (time * 1.6).fract();
            p.circle_filled(you, 8.0 + 14.0 * pulse, with_alpha(t.info, 0.25 * (1.0 - pulse)));
            p.circle_filled(you, 7.5, Color32::WHITE);
            p.circle_filled(you, 5.5, t.info);
            // other spots
            for (i, s) in layout.spots.iter().enumerate() {
                if i != sel {
                    p.circle_filled(*s, 5.0, Color32::WHITE);
                    p.circle_filled(*s, 3.5, mix(t.muted_foreground, t.foreground, 0.3));
                }
            }
            // pin drop with bounce
            let drop = sel_e.min(e - 0.2).max(0.0);
            let fall = 1.0 - egui::emath::easing::bounce_out((drop / 0.7).clamp(0.0, 1.0));
            let tip = dest - vec2(0.0, fall * 40.0);
            pin(&p, tip, t.destructive, (drop * 4.0).clamp(0.0, 1.0));
            // label bubble
            if let Some(pl) = places.get(sel) {
                let k = ((drop - 0.4) * 4.0).clamp(0.0, 1.0);
                if k > 0.0 {
                    let g = galley(ui, pl.name.as_str(), 11.5, t.foreground);
                    let br = Rect::from_center_size(tip - vec2(0.0, 44.0), g.size() + vec2(16.0, 8.0));
                    let br = br.translate(vec2((mr.max.x - 6.0 - br.max.x).min(0.0) + (mr.min.x + 6.0 - br.min.x).max(0.0), 0.0));
                    paint::soft_shadow(&p, br, 8.0, 8.0, vec2(0.0, 2.0), shadow(&t, 0.8 * k));
                    p.rect_filled(br, cr(8.0), with_alpha(surface(&t), k));
                    p.galley(br.min + vec2(8.0, 4.0), g, with_alpha(t.foreground, k));
                }
            }
            // zoom buttons
            for (k, ic) in [Icon::Plus, Icon::ChevronDown].iter().enumerate() {
                let b = Rect::from_min_size(pos2(mr.max.x - 34.0, mr.min.y + 10.0 + k as f32 * 28.0), Vec2::splat(24.0));
                p.rect_filled(b, cr(7.0), surface(&t));
                if *ic == Icon::Plus {
                    icon(&p, *ic, b.center(), 12.0, t.muted_foreground);
                } else {
                    p.line_segment([b.center() - vec2(4.0, 0.0), b.center() + vec2(4.0, 0.0)], Stroke::new(1.4, t.muted_foreground));
                }
            }
            ui.painter().rect_stroke(mr, cr(10.0), hairline(&t), egui::StrokeKind::Inside);
            // place rows
            for (i, pl) in places.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 44.0), Sense::click());
                rows.push(r);
                if resp.hovered() {
                    new_hover = Some(i);
                }
                let on = toggle(ui, id.with(("row", i)), i == sel);
                let p = ui.painter();
                if on > 0.01 {
                    p.rect_filled(r, cr(10.0), with_alpha(well(&t), on));
                }
                let c = pos2(r.min.x + 18.0, r.center().y);
                p.circle_filled(c, 12.0, mix(well(&t), with_alpha(t.destructive, 0.14), on));
                text_bold(p, c, Align2::CENTER_CENTER, &((b'A' + i as u8) as char).to_string(), 11.0, mix(t.muted_foreground, t.destructive, on));
                text_bold(p, pos2(c.x + 22.0, r.min.y + 15.0), Align2::LEFT_CENTER, &pl.name, 13.0, t.foreground);
                text(p, pos2(c.x + 22.0, r.min.y + 31.0), Align2::LEFT_CENTER, &pl.detail, 11.5, t.muted_foreground);
                text(p, pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, &pl.meta, 11.5, mix(t.muted_foreground, t.foreground, on));
            }
        });
        if new_hover != hovered && new_hover.is_some() {
            glowkit_core::set_state(ui, id.with("hov"), new_hover);
        }
        (ui.interact(rect, id.with("card"), Sense::hover()), rows)
    }
}

/// A map pin (teardrop) with its tip at `tip`.
fn pin(p: &egui::Painter, tip: Pos2, color: Color32, a: f32) {
    if a <= 0.0 {
        return;
    }
    p.add(Shape::ellipse_filled(tip, vec2(6.0, 2.5), Color32::from_black_alpha((50.0 * a) as u8)));
    let c = tip - vec2(0.0, 20.0);
    let r = 10.0;
    let pts = vec![tip, c + vec2(-r * 0.86, r * 0.5), c + vec2(r * 0.86, r * 0.5)];
    p.add(Shape::convex_polygon(pts, with_alpha(color, a), Stroke::NONE));
    p.circle_filled(c, r, with_alpha(color, a));
    p.circle_filled(c, 3.8, with_alpha(Color32::WHITE, a));
}

struct MapLayout {
    roads_h: Vec<f32>,
    roads_v: Vec<f32>,
    river: Vec<Pos2>,
    parks: Vec<Rect>,
    you: Pos2,
    spots: Vec<Pos2>,
}

impl MapLayout {
    fn new(seed: u64, r: Rect, n: usize) -> Self {
        let mut rng = Rng::new(seed * 131 + 9);
        let mut roads_h = Vec::new();
        let mut y = r.min.y + rng.range(18.0, 34.0);
        while y < r.max.y {
            roads_h.push(y);
            y += rng.range(34.0, 58.0);
        }
        let mut roads_v = Vec::new();
        let mut x = r.min.x + rng.range(20.0, 40.0);
        while x < r.max.x {
            roads_v.push(x);
            x += rng.range(48.0, 86.0);
        }
        let river: Vec<Pos2> = (0..=24)
            .map(|i| {
                let u = i as f32 / 24.0;
                pos2(
                    r.min.x + r.width() * (0.68 + 0.12 * (u * 4.0 + seed as f32).sin()),
                    r.min.y - 10.0 + (r.height() + 20.0) * u,
                )
            })
            .collect();
        let mut parks = Vec::new();
        for _ in 0..2 {
            let i = rng.below(roads_v.len().saturating_sub(1).max(1));
            let j = rng.below(roads_h.len().saturating_sub(1).max(1));
            if i + 1 < roads_v.len() && j + 1 < roads_h.len() {
                parks.push(Rect::from_min_max(pos2(roads_v[i] + 5.0, roads_h[j] + 5.0), pos2(roads_v[i + 1] - 5.0, roads_h[j + 1] - 5.0)));
            }
        }
        let snap = |rng: &mut Rng| -> Pos2 {
            let x = roads_v[rng.below(roads_v.len())];
            let y = roads_h[rng.below(roads_h.len())];
            pos2(x, y)
        };
        let you = pos2(roads_v[roads_v.len() / 4], roads_h[roads_h.len() * 3 / 4]);
        let mut spots = Vec::new();
        for _ in 0..n.max(1) {
            let mut s = snap(&mut rng);
            for _ in 0..8 {
                if (s - you).length() > 60.0 && spots.iter().all(|q: &Pos2| (*q - s).length() > 40.0) && s.x < r.max.x - 40.0 && s.y > r.min.y + 50.0 {
                    break;
                }
                s = snap(&mut rng);
            }
            spots.push(s);
        }
        Self { roads_h, roads_v, river, parks, you, spots }
    }

    fn paint(&self, p: &egui::Painter, t: &Theme, r: Rect) {
        let land = if t.dark { mix(t.muted, hex("#1E293B"), 0.35) } else { hex("#F1EFE9") };
        let road = if t.dark { mix(t.muted, Color32::WHITE, 0.12) } else { Color32::WHITE };
        let park = if t.dark { with_alpha(hex("#22C55E"), 0.18) } else { hex("#DCEFD5") };
        let water = if t.dark { with_alpha(hex("#3B82F6"), 0.28) } else { hex("#CFE3F5") };
        p.rect_filled(r, cr(10.0), land);
        for pk in &self.parks {
            p.rect_filled(*pk, cr(6.0), park);
        }
        for y in &self.roads_h {
            p.line_segment([pos2(r.min.x, *y), pos2(r.max.x, *y)], Stroke::new(5.0, road));
        }
        for x in &self.roads_v {
            p.line_segment([pos2(*x, r.min.y), pos2(*x, r.max.y)], Stroke::new(5.0, road));
        }
        // one diagonal avenue
        p.line_segment([pos2(r.min.x, r.max.y - 10.0), pos2(r.min.x + r.width() * 0.6, r.min.y)], Stroke::new(7.0, if t.dark { road } else { hex("#FDF3D7") }));
        let river = paint::catmull_rom(&self.river, 6);
        p.add(Shape::line(river, Stroke::new(16.0, water)));
    }

    /// A Manhattan route along the grid from `a` to `b`.
    fn route(&self, a: Pos2, b: Pos2) -> Vec<Pos2> {
        let corner = pos2(a.x, b.y);
        let mut v = Vec::new();
        let seg = |v: &mut Vec<Pos2>, p: Pos2, q: Pos2| {
            let n = ((q - p).length() / 4.0).ceil().max(1.0) as usize;
            for i in 0..n {
                v.push(p.lerp(q, i as f32 / n as f32));
            }
        };
        seg(&mut v, a, corner);
        seg(&mut v, corner, b);
        v.push(b);
        v
    }
}
