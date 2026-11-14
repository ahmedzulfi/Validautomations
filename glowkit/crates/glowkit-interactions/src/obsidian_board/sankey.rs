//! A true sankey diagram in a dashboard card: node columns sized by flow, smooth ribbons whose
//! thickness is proportional to each flow, and hover isolation.

use super::common::*;
use egui::epaint::Mesh;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::color::{fade, mix};
use glowkit_core::Theme;

/// One flow of a [`BoardSankey`]: `value` units travel from the source node `from` to the
/// target node `to`. Nodes are created from the names in order of first appearance.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardFlow {
    /// Source node name (left column).
    pub from: String,
    /// Target node name (right column).
    pub to: String,
    /// Amount carried; ribbon thickness is proportional to it.
    pub value: f32,
}

impl BoardFlow {
    /// A flow of `value` from `from` to `to`.
    pub fn new(from: impl Into<String>, to: impl Into<String>, value: f32) -> Self {
        Self { from: from.into(), to: to.into(), value: value.max(0.0) }
    }
}

/// A sankey flow chart in a calm dashboard card. Sources stack as coloured pill nodes on the
/// left (labelled with name and value), sinks as neutral nodes on the right (labelled with their
/// share), and every flow is a smooth ribbon whose thickness is proportional to its value,
/// tinted with its source colour so each origin reads as one family. Hover a node or a ribbon
/// to isolate it while the headline counts to its value; ribbons sweep in from the left on
/// entry and re-flow when the data changes.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardFlow, BoardSankey};
/// ui.add(BoardSankey::new("Tracked time").unit("h").flows(vec![
///     BoardFlow::new("Focus", "Writing", 12.0),
///     BoardFlow::new("Focus", "Browsing", 8.0),
///     BoardFlow::new("Meetings", "Video calls", 10.0),
/// ]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardSankey {
    title: String,
    flows: Vec<BoardFlow>,
    unit: String,
    size: Vec2,
    colors: Option<Vec<Color32>>,
    opacity: f32,
    gap: f32,
    period: Option<String>,
    captions: (String, String),
    autoplay: bool,
}

impl BoardSankey {
    /// A sankey card titled `title`, filled with a sample "tracked time" week.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            flows: sample_flows(),
            unit: "h".into(),
            size: vec2(660.0, 440.0),
            colors: None,
            opacity: 0.42,
            gap: 14.0,
            period: Some("This week".into()),
            captions: ("Tracked time".into(), "Share of tracked time".into()),
            autoplay: false,
        }
    }
    /// The flows to draw (replaces the sample data).
    pub fn flows(mut self, flows: Vec<BoardFlow>) -> Self {
        self.flows = flows;
        self
    }
    /// Unit appended to values, e.g. `"h"` or `" GB"`.
    pub fn unit(mut self, u: impl Into<String>) -> Self {
        self.unit = u.into();
        self
    }
    /// Card size in points (default 660 x 440).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = vec2(s.x.max(380.0), s.y.max(280.0));
        self
    }
    /// Source node colours, cycled (default: the BoardUI accent family).
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = Some(c);
        }
        self
    }
    /// Ribbon opacity `0..1` (default 0.42).
    pub fn opacity(mut self, a: f32) -> Self {
        self.opacity = a.clamp(0.05, 1.0);
        self
    }
    /// Vertical gap between nodes in points (default 14).
    pub fn node_gap(mut self, g: f32) -> Self {
        self.gap = g.clamp(0.0, 40.0);
        self
    }
    /// Label of the period pill in the corner (`None` hides it).
    pub fn period(mut self, p: Option<String>) -> Self {
        self.period = p;
        self
    }
    /// Footer captions under the left and right columns.
    pub fn captions(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.captions = (left.into(), right.into());
        self
    }
    /// Walk the hover highlight through nodes and ribbons by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn sample_flows() -> Vec<BoardFlow> {
    let rows: [(&str, &[(&str, f32)]); 5] = [
        ("Deep work", &[("Writing", 9.0), ("Design", 8.0), ("Browsing", 6.0), ("Messaging", 3.0), ("Other", 4.0)]),
        ("Meetings", &[("Video calls", 10.0), ("Messaging", 3.0), ("Writing", 2.0), ("Browsing", 3.0)]),
        ("Breaks", &[("Browsing", 6.0), ("Other", 4.0), ("Messaging", 2.0)]),
        ("Admin", &[("Email", 6.0), ("Messaging", 3.0), ("Design", 2.0), ("Browsing", 3.0)]),
        ("Learning", &[("Browsing", 4.0), ("Writing", 3.0), ("Video calls", 1.0), ("Other", 2.0)]),
    ];
    let mut v = Vec::new();
    for (from, outs) in rows {
        for (to, val) in outs {
            v.push(BoardFlow::new(from, *to, *val));
        }
    }
    v
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Hit {
    Left(usize),
    Right(usize),
    Link(usize),
}

struct Link {
    s: usize,
    d: usize,
    v: f32,
    y0: f32,
    y1: f32,
    w: f32,
}

fn bez_x(x0: f32, x1: f32, s: f32) -> f32 {
    let xm = (x0 + x1) * 0.5;
    let u = 1.0 - s;
    x0 * u * u * u + 3.0 * xm * u * u * s + 3.0 * xm * u * s * s + x1 * s * s * s
}

fn smooth(s: f32) -> f32 {
    s * s * (3.0 - 2.0 * s)
}

#[allow(clippy::too_many_arguments)]
fn ribbon(painter: &egui::Painter, x0: f32, x1: f32, y0: f32, y1: f32, w: f32, c0: Color32, c1: Color32) {
    let n = 36;
    let mut m = Mesh::default();
    for i in 0..=n {
        let s = i as f32 / n as f32;
        let x = bez_x(x0, x1, s);
        let y = y0 + (y1 - y0) * smooth(s);
        let c = mix(c0, c1, s);
        m.colored_vertex(pos2(x, y), c);
        m.colored_vertex(pos2(x, y + w), c);
    }
    for i in 0..n as u32 {
        let k = i * 2;
        m.add_triangle(k, k + 1, k + 2);
        m.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(egui::Shape::mesh(m));
}

fn fmt_val(v: f32, unit: &str) -> String {
    if (v - v.round()).abs() < 0.05 || v >= 100.0 {
        format!("{}{unit}", thousands(v as f64))
    } else {
        format!("{v:.1}{unit}")
    }
}

impl egui::Widget for BoardSankey {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(24.0));
        let el = entry(ui, id);

        // ---- nodes
        let mut left: Vec<(String, f32)> = Vec::new();
        let mut right: Vec<(String, f32)> = Vec::new();
        let mut raw = Vec::new();
        for fl in &self.flows {
            let s = match left.iter().position(|n| n.0 == fl.from) {
                Some(i) => i,
                None => {
                    left.push((fl.from.clone(), 0.0));
                    left.len() - 1
                }
            };
            let d = match right.iter().position(|n| n.0 == fl.to) {
                Some(i) => i,
                None => {
                    right.push((fl.to.clone(), 0.0));
                    right.len() - 1
                }
            };
            left[s].1 += fl.value;
            right[d].1 += fl.value;
            raw.push((s, d, fl.value));
        }
        let total_l: f32 = left.iter().map(|n| n.1).sum();
        let total_r: f32 = right.iter().map(|n| n.1).sum();
        let total = total_l.max(total_r).max(1e-3);
        let colors: Vec<Color32> = self
            .colors
            .clone()
            .unwrap_or_else(|| vec![p[EMERALD], p[PURPLE], p[YELLOW], p[BLUE], p[PINK], p[ORANGE], p[TEAL]]);
        let src_col = |i: usize| colors[i % colors.len()];
        let sink_col = mix(t.muted_foreground, frame_bg(&t), if t.dark { 0.15 } else { 0.25 });

        // ---- layout
        frame(&painter, &t, rect, 16.0);
        let pad = 18.0;
        let chart =
            Rect::from_min_max(pos2(rect.min.x + pad, rect.min.y + 96.0), pos2(rect.max.x - pad, rect.max.y - 40.0));
        let lw = left.iter().map(|n| text_w(&painter, &n.0, f(13.0))).fold(40.0, f32::max) + 12.0;
        let rw = right
            .iter()
            .map(|n| text_w(&painter, &n.0, f(13.0)) + text_w(&painter, " · 100%", f(12.0)))
            .fold(60.0, f32::max)
            + 12.0;
        let node_w = 8.0;
        let xl = chart.min.x + lw;
        let xr = chart.max.x - rw - node_w;
        let h = chart.height();
        let scale_l = (h - self.gap * (left.len().max(1) - 1) as f32) / total;
        let scale_r = (h - self.gap * (right.len().max(1) - 1) as f32) / total;
        let scale = scale_l.min(scale_r).max(0.0);
        let col_h =
            |n: &[(String, f32)]| n.iter().map(|x| x.1 * scale).sum::<f32>() + self.gap * (n.len().max(1) - 1) as f32;
        let mut ly = Vec::new();
        let mut y = chart.center().y - col_h(&left) / 2.0;
        for n in &left {
            ly.push(y);
            y += n.1 * scale + self.gap;
        }
        let mut ry = Vec::new();
        let mut y = chart.center().y - col_h(&right) / 2.0;
        for n in &right {
            ry.push(y);
            y += n.1 * scale + self.gap;
        }
        // link ends: at the source ordered by target, at the target ordered by source
        let mut links: Vec<Link> =
            raw.iter().map(|&(s, d, v)| Link { s, d, v, y0: 0.0, y1: 0.0, w: v * scale }).collect();
        let mut order: Vec<usize> = (0..links.len()).collect();
        order.sort_by_key(|&k| (links[k].s, links[k].d));
        let mut cur = ly.clone();
        for &k in &order {
            links[k].y0 = cur[links[k].s];
            cur[links[k].s] += links[k].w;
        }
        order.sort_by_key(|&k| (links[k].d, links[k].s));
        let mut cur = ry.clone();
        for &k in &order {
            links[k].y1 = cur[links[k].d];
            cur[links[k].d] += links[k].w;
        }
        // animate ribbon positions so data changes re-flow
        for (k, l) in links.iter_mut().enumerate() {
            l.y0 = glowkit_core::anim::spring(ui, id.with(("y0", k)), l.y0, 220.0, 26.0);
            l.y1 = glowkit_core::anim::spring(ui, id.with(("y1", k)), l.y1, 220.0, 26.0);
            l.w = glowkit_core::anim::spring(ui, id.with(("w", k)), l.w, 220.0, 26.0).max(0.0);
        }

        // ---- hover
        let x0 = xl + node_w;
        let x1 = xr;
        let mut hit = None;
        if let Some(mp) = response.hover_pos() {
            for (i, n) in left.iter().enumerate() {
                let r = Rect::from_min_size(pos2(chart.min.x, ly[i]), vec2(lw + node_w, n.1 * scale))
                    .expand2(vec2(0.0, 4.0));
                if r.contains(mp) {
                    hit = Some(Hit::Left(i));
                }
            }
            for (i, n) in right.iter().enumerate() {
                let r =
                    Rect::from_min_max(pos2(xr, ry[i]), pos2(chart.max.x, ry[i] + n.1 * scale)).expand2(vec2(0.0, 4.0));
                if r.contains(mp) {
                    hit = Some(Hit::Right(i));
                }
            }
            if hit.is_none() && mp.x > x0 && mp.x < x1 {
                // invert the bezier x(s) by bisection, then test the band
                let (mut a, mut b) = (0.0_f32, 1.0_f32);
                for _ in 0..18 {
                    let m = (a + b) * 0.5;
                    if bez_x(x0, x1, m) < mp.x {
                        a = m;
                    } else {
                        b = m;
                    }
                }
                let s = smooth((a + b) * 0.5);
                for (k, l) in links.iter().enumerate().rev() {
                    let yt = l.y0 + (l.y1 - l.y0) * s;
                    if mp.y >= yt && mp.y <= yt + l.w.max(2.0) {
                        hit = Some(Hit::Link(k));
                        break;
                    }
                }
            }
        }
        let idle = idle_for(ui, id, response.hovered());
        if hit.is_none() && self.autoplay && idle > 2.2 && el > 2.0 {
            let seq = left.len() + 3;
            let step = ((idle - 2.2) / 1.9) as usize % (seq + 1);
            hit = if step < left.len() {
                Some(Hit::Left(step))
            } else if step < left.len() + 2 {
                let k = (step * 7 + 3) % links.len().max(1);
                if links.is_empty() {
                    None
                } else {
                    Some(Hit::Link(k))
                }
            } else if step == left.len() + 2 && !right.is_empty() {
                Some(Hit::Right(0))
            } else {
                None
            };
            glowkit_core::animate(ui);
        }
        let sticky: Option<Hit> = {
            let prev: Option<Hit> = glowkit_core::state(ui, id.with("sticky"), || None);
            let v = hit.or(prev);
            glowkit_core::set_state(ui, id.with("sticky"), v);
            v
        };
        let focus = flag(ui, id.with("focus"), hit.is_some(), 10.0);
        let lit_link = |k: usize, l: &Link| match sticky {
            Some(Hit::Left(i)) => l.s == i,
            Some(Hit::Right(j)) => l.d == j,
            Some(Hit::Link(m)) => m == k,
            None => true,
        };
        let lit_left = |i: usize| match sticky {
            Some(Hit::Left(a)) => a == i,
            Some(Hit::Right(j)) => links.iter().any(|l| l.s == i && l.d == j),
            Some(Hit::Link(m)) => links.get(m).is_some_and(|l| l.s == i),
            None => true,
        };
        let lit_right = |j: usize| match sticky {
            Some(Hit::Right(a)) => a == j,
            Some(Hit::Left(i)) => links.iter().any(|l| l.s == i && l.d == j),
            Some(Hit::Link(m)) => links.get(m).is_some_and(|l| l.d == j),
            None => true,
        };
        let dimk = |lit: bool| if lit { 1.0 } else { 1.0 - 0.82 * focus };

        // ---- header
        let (title, value) = match hit {
            Some(Hit::Left(i)) => (left[i].0.clone(), left[i].1),
            Some(Hit::Right(j)) => (right[j].0.clone(), right[j].1),
            Some(Hit::Link(k)) => (format!("{} → {}", left[links[k].s].0, right[links[k].d].0), links[k].v),
            None => (self.title.clone(), total_l),
        };
        let shown = tween(ui, id.with("head"), value, 0.0, 0.9);
        text(&painter, pos2(rect.min.x + pad, rect.min.y + 26.0), Align2::LEFT_CENTER, &title, f(13.0), dim(&t));
        text(
            &painter,
            pos2(rect.min.x + pad, rect.min.y + 56.0),
            Align2::LEFT_CENTER,
            fmt_val(shown.round(), &self.unit),
            f(26.0),
            t.foreground,
        );
        if let Some(pl) = &self.period {
            let w = pill_w(&painter, pl, true);
            let r = Rect::from_min_size(pos2(rect.max.x - pad - w, rect.min.y + 16.0), vec2(w, 30.0));
            let hov = ui.rect_contains_pointer(r);
            pill(&painter, &t, r, pl, true, flag(ui, id.with("pillh"), hov, 12.0));
        }

        // ---- ribbons (sweep in from the left)
        let sweep = ease_out(entry_k(ui, el, 0.25, 1.1));
        let clip = Rect::from_min_max(pos2(x0 - 1.0, rect.min.y), pos2(x0 + (x1 - x0 + 2.0) * sweep, rect.max.y));
        let rp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        let base_a = self.opacity * if t.dark { 0.85 } else { 1.0 };
        let draw = |lit_pass: bool| {
            for (k, l) in links.iter().enumerate() {
                let lit = lit_link(k, l);
                if lit != lit_pass && focus > 0.01 {
                    continue;
                }
                if !lit_pass && focus <= 0.01 {
                    continue;
                }
                let boost = if lit { 1.0 + 0.55 * focus } else { 1.0 };
                let a = (base_a * boost * dimk(lit)).min(0.95);
                let c0 = fade(src_col(l.s), a);
                let c1 = fade(mix(src_col(l.s), sink_col, 0.18), a);
                ribbon(&rp, x0, x1, l.y0, l.y1, l.w, c0, c1);
            }
        };
        draw(false);
        draw(true);

        // ---- nodes and labels
        for (i, n) in left.iter().enumerate() {
            let k = ease_back(entry_k(ui, el, 0.05 * i as f32, 0.55));
            let hgt = n.1 * scale;
            let r = Rect::from_center_size(pos2(xl + node_w / 2.0, ly[i] + hgt / 2.0), vec2(node_w, hgt * k));
            let a = dimk(lit_left(i));
            painter.rect_filled(r, cr(3.0), fade(src_col(i), a));
            let cy = ly[i] + hgt / 2.0;
            let la = a * ease_out(entry_k(ui, el, 0.1 + 0.05 * i as f32, 0.5));
            text(&painter, pos2(xl - 10.0, cy - 8.0), Align2::RIGHT_CENTER, &n.0, f(13.0), fade(t.foreground, la));
            text(
                &painter,
                pos2(xl - 10.0, cy + 8.0),
                Align2::RIGHT_CENTER,
                fmt_val(n.1, &self.unit),
                f(11.5),
                fade(faint(&t), la),
            );
        }
        for (j, n) in right.iter().enumerate() {
            let k = ease_back(entry_k(ui, el, 0.9 + 0.05 * j as f32, 0.55));
            let hgt = n.1 * scale;
            let r = Rect::from_center_size(pos2(xr + node_w / 2.0, ry[j] + hgt / 2.0), vec2(node_w, hgt * k));
            let a = dimk(lit_right(j));
            painter.rect_filled(r, cr(3.0), fade(sink_col, a));
            let cy = ry[j] + hgt / 2.0;
            let la = a * ease_out(entry_k(ui, el, 1.0 + 0.05 * j as f32, 0.5));
            let nr = text(
                &painter,
                pos2(xr + node_w + 10.0, cy),
                Align2::LEFT_CENTER,
                &n.0,
                f(13.0),
                fade(t.foreground, la),
            );
            let share = format!(" · {:.0}%", n.1 / total_r.max(1e-3) * 100.0);
            text(&painter, pos2(nr.max.x, cy), Align2::LEFT_CENTER, share, f(12.0), fade(faint(&t), la));
        }

        // ---- footer captions
        let fy = rect.max.y - 20.0;
        text(&painter, pos2(rect.min.x + pad, fy), Align2::LEFT_CENTER, &self.captions.0, f(12.0), faint(&t));
        text(&painter, pos2(rect.max.x - pad, fy), Align2::RIGHT_CENTER, &self.captions.1, f(12.0), faint(&t));
        if hit.is_some() && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        response
    }
}
