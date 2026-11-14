//! An AI work profile page: identity card with cover, contributions heat map, an agents bar
//! chart and a tokens trend line.

use super::common::*;
use super::heat::{contributions_body, contributions_height, sample_counts, BoardStat};
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Response, Sense, Stroke, Ui};
use glowkit_core::color::{fade, mix};
use glowkit_core::{paint, Theme};

/// A GitHub-style profile for AI work. The left card has a cover photo, an avatar overlapping
/// it, name and handle with a PRO badge, Share / Edit buttons, a contributions headline with
/// stat tiles and the activity heat map (Weekly / Monthly / Yearly). The right column holds a
/// 30-day agents bar chart with a month switcher (idle days collapse to grey stubs) and a
/// tokens trend line with sharp joins that draws itself in, with a hover crosshair.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardAiProfile;
/// ui.add(BoardAiProfile::new("Mira Okafor", "mira"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardAiProfile {
    name: String,
    handle: String,
    scene: usize,
    counts: Vec<u32>,
    stats: Vec<BoardStat>,
    months: Vec<(String, Vec<u32>)>,
    tokens: Vec<f32>,
    accent: Option<Color32>,
    width: f32,
    autoplay: bool,
}

impl BoardAiProfile {
    /// A profile for `name` (`@handle`) with sample activity.
    pub fn new(name: impl Into<String>, handle: impl Into<String>) -> Self {
        let mut rng = glowkit_core::Rng::new(5);
        let mut month = |len: usize, idle: f32| -> Vec<u32> {
            (0..len)
                .map(|_| if rng.bool(idle) { 0 } else { (rng.range(1.0, 9.0) + rng.range(0.0, 4.0)).round() as u32 })
                .collect()
        };
        let months = vec![
            ("October".to_string(), month(31, 0.25)),
            ("November".to_string(), month(30, 0.2)),
            ("December".to_string(), month(31, 0.3)),
        ];
        let mut rng = glowkit_core::Rng::new(9);
        let mut v = 40.0_f32;
        let tokens = (0..24)
            .map(|_| {
                v = (v + rng.range(-14.0, 18.0)).clamp(8.0, 120.0);
                v
            })
            .collect();
        Self {
            name: name.into(),
            handle: handle.into(),
            scene: 1,
            counts: sample_counts(23),
            stats: vec![
                BoardStat::new("9B", "Lifetime tokens"),
                BoardStat::new("562.7M", "Peak tokens"),
                BoardStat::new("12h 54m", "Longest task"),
                BoardStat::new("62 days", "Top streak"),
            ],
            months,
            tokens,
            accent: None,
            width: 900.0,
            autoplay: false,
        }
    }
    /// Cover image: one of the painted scenes (`glowkit_effects::PaintedScene`).
    pub fn scene(mut self, i: usize) -> Self {
        self.scene = i;
        self
    }
    /// Daily contribution counts for the year.
    pub fn counts(mut self, c: Vec<u32>) -> Self {
        self.counts = c;
        self
    }
    /// Headline stat tiles.
    pub fn stats(mut self, s: Vec<BoardStat>) -> Self {
        self.stats = s;
        self
    }
    /// Months of daily agent runs for the bar chart (the last is shown first).
    pub fn agent_months(mut self, m: Vec<(String, Vec<u32>)>) -> Self {
        if !m.is_empty() {
            self.months = m;
        }
        self
    }
    /// Token usage series for the trend line (millions).
    pub fn tokens(mut self, t: Vec<f32>) -> Self {
        if t.len() >= 2 {
            self.tokens = t;
        }
        self
    }
    /// Accent colour (default purple).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
    /// Overall width in points (default 900).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(640.0);
        self
    }
    /// Hover the heat map and charts by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

fn button(painter: &egui::Painter, t: &Theme, r: Rect, label: &str, share: bool, hover: f32) {
    painter.rect(r, cr(8.0), mix(tile_bg(t), track(t), hover), Stroke::new(1.0, hairline(t)), egui::StrokeKind::Inside);
    let ic = pos2(r.min.x + 15.0, r.center().y);
    let st = Stroke::new(1.2, t.foreground);
    if share {
        painter.line_segment([ic + vec2(0.0, 3.0), ic + vec2(0.0, -5.0)], st);
        painter.add(egui::Shape::line(vec![ic + vec2(-3.0, -2.0), ic + vec2(0.0, -5.0), ic + vec2(3.0, -2.0)], st));
        painter.add(egui::Shape::line(
            vec![ic + vec2(-5.0, 0.0), ic + vec2(-5.0, 5.0), ic + vec2(5.0, 5.0), ic + vec2(5.0, 0.0)],
            st,
        ));
    } else {
        painter.rect_stroke(Rect::from_center_size(ic, vec2(10.0, 10.0)), cr(2.0), st, egui::StrokeKind::Middle);
        painter.line_segment([ic + vec2(-1.0, 1.0), ic + vec2(5.0, -5.0)], st);
    }
    text(painter, pos2(r.min.x + 27.0, r.center().y), Align2::LEFT_CENTER, label, f(13.0), t.foreground);
}

impl egui::Widget for BoardAiProfile {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let accent = self.accent.unwrap_or(p[PURPLE]);
        let gap = 16.0;
        let lw = (self.width - gap) * 0.58;
        let cover_h = 120.0;
        let ident_h = 104.0;
        let contrib_h = contributions_height(lw, true);
        let h = cover_h + ident_h + contrib_h;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        let id = response.id;
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(30.0));
        let el = entry(ui, id);
        let left = Rect::from_min_size(rect.min, vec2(lw, h));
        let right = Rect::from_min_max(pos2(left.max.x + gap, rect.min.y), rect.max);

        // ---- identity card
        sheet(&painter, &t, left, 18.0);
        let cover = Rect::from_min_size(left.min + vec2(1.0, 1.0), vec2(lw - 2.0, cover_h));
        let tex = glowkit_effects::PaintedScene::texture(ui.ctx(), self.scene);
        // crop the scene to the cover's aspect
        let aspect = cover.width() / cover.height();
        let uv_h = (glowkit_effects::PaintedScene::ASPECT / aspect).min(1.0);
        let pan = 0.5 + 0.08 * (glowkit_core::time(ui) * 0.05).sin();
        let uv = Rect::from_min_max(pos2(0.0, (1.0 - uv_h) * pan), pos2(1.0, (1.0 - uv_h) * pan + uv_h));
        egui::Image::new((tex, cover.size()))
            .uv(uv)
            .corner_radius(egui::CornerRadius { nw: 17, ne: 17, sw: 0, se: 0 })
            .paint_at(ui, cover);
        if self.autoplay {
            glowkit_core::animate(ui);
        }
        let av = pos2(left.min.x + 52.0, cover.max.y);
        painter.circle_filled(av, 36.0, tile_bg(&t));
        paint::gradient_circle(
            &painter,
            av,
            32.0,
            &[mix(frame_bg(&t), t.foreground, 0.04), mix(frame_bg(&t), t.foreground, 0.1)],
        );
        let initial: String = self.name.chars().next().map(|c| c.to_string()).unwrap_or_default();
        text(&painter, av, Align2::CENTER_CENTER, initial, f(24.0), dim(&t));
        let by = cover.max.y + 24.0;
        let mut bx = left.max.x - 16.0;
        for (label, share) in [("Edit", false), ("Share", true)] {
            let w = text_w(&painter, label, f(13.0)) + 38.0;
            let r = Rect::from_min_size(pos2(bx - w, by - 15.0), vec2(w, 30.0));
            let resp = ui.interact(r, id.with(label), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            button(&painter, &t, r, label, share, flag(ui, id.with(("bh", label)), resp.hovered(), 14.0));
            bx -= w + 8.0;
        }
        let ny = cover.max.y + 54.0;
        text(&painter, pos2(left.min.x + 18.0, ny), Align2::LEFT_CENTER, &self.name, f(18.0), t.foreground);
        let hr = text(
            &painter,
            pos2(left.min.x + 18.0, ny + 24.0),
            Align2::LEFT_CENTER,
            format!("@{}", self.handle),
            f(13.0),
            dim(&t),
        );
        let pro = Rect::from_min_size(pos2(hr.max.x + 6.0, hr.center().y - 8.0), vec2(28.0, 16.0));
        painter.rect_filled(pro, cr(4.0), track(&t));
        text(&painter, pro.center(), Align2::CENTER_CENTER, "PRO", f(9.5), dim(&t));
        let body = Rect::from_min_max(pos2(left.min.x, cover.max.y + ident_h - 14.0), left.max);
        let total: u32 = self.counts.iter().sum();
        let money = tween(ui, id.with("money"), total as f32 * 7.9, 0.0, 1.2);
        contributions_body(
            ui,
            &painter,
            id.with("contrib"),
            body,
            "Contributions this year",
            &format!("${}", thousands(money as f64)),
            &self.counts,
            3,
            &self.stats,
            "+14.8%",
            accent,
            self.autoplay,
            response.hovered() && ui.rect_contains_pointer(body),
            true,
        );

        // ---- agents bar chart
        let ah = (h - gap) * 0.52;
        let agents = Rect::from_min_size(right.min, vec2(right.width(), ah));
        frame(&painter, &t, agents, 16.0);
        let pad = 16.0;
        let last = self.months.len() - 1;
        let mut mi: usize = glowkit_core::state(ui, id.with("month"), || last);
        let pr = Rect::from_min_size(pos2(agents.max.x - pad - 130.0, agents.min.y + 14.0), vec2(130.0, 30.0));
        let lr = Rect::from_min_size(pr.min, vec2(26.0, 30.0));
        let rr = Rect::from_min_size(pos2(pr.max.x - 26.0, pr.min.y), vec2(26.0, 30.0));
        let l = ui.interact(lr, id.with("pm"), Sense::click());
        let r = ui.interact(rr, id.with("nm"), Sense::click());
        if l.clicked() && mi > 0 {
            mi -= 1;
        }
        if r.clicked() && mi < last {
            mi += 1;
        }
        glowkit_core::set_state(ui, id.with("month"), mi);
        let hovered_right = response.hovered() && ui.rect_contains_pointer(right);
        let idle = idle_for(ui, id.with("right"), hovered_right || l.hovered() || r.hovered());
        let shown_m =
            if self.autoplay && idle > 3.0 { (mi + ((idle - 3.0) / 4.0) as usize) % self.months.len() } else { mi };
        let (mname, days) = &self.months[shown_m];
        let hl = [flag(ui, id.with("lh"), l.hovered(), 14.0), flag(ui, id.with("rh"), r.hovered(), 14.0)];
        range_pill(&painter, &t, pr, mname, hl);
        let active_days = days.iter().filter(|d| **d > 0).count();
        let distinct: u32 = days.iter().sum::<u32>() / 3;
        text(&painter, pos2(agents.min.x + pad, agents.min.y + 26.0), Align2::LEFT_CENTER, "Agents", f(13.0), dim(&t));
        let av_n = tween(ui, id.with("agents_n"), distinct as f32, 0.0, 0.9);
        let hr = text(
            &painter,
            pos2(agents.min.x + pad, agents.min.y + 52.0),
            Align2::LEFT_CENTER,
            format!("{} agents", av_n.round()),
            f(20.0),
            t.foreground,
        );
        text(
            &painter,
            pos2(hr.max.x + 8.0, hr.center().y + 2.0),
            Align2::LEFT_CENTER,
            format!("{active_days} active days"),
            f(12.0),
            faint(&t),
        );
        let plot = Rect::from_min_max(
            pos2(agents.min.x + pad, agents.min.y + 80.0),
            pos2(agents.max.x - pad, agents.max.y - 30.0),
        );
        let n = days.len().max(1);
        let slot = plot.width() / n as f32;
        let bw = (slot * 0.62).max(2.0);
        let mx = days.iter().copied().max().unwrap_or(1).max(1) as f32;
        let ptr = response.hover_pos().filter(|q| plot.expand2(vec2(0.0, 20.0)).contains(*q));
        let mut hov = ptr.map(|q| (((q.x - plot.min.x) / slot) as usize).min(n - 1));
        if hov.is_none() && self.autoplay && idle > 1.5 {
            hov = Some(((idle * 1.7) as usize * 7) % n);
        }
        for (i, d) in days.iter().enumerate() {
            let cx = plot.min.x + slot * (i as f32 + 0.5);
            let g = ease_out(entry_k(ui, el, 0.2 + 0.012 * i as f32, 0.6));
            let target = if *d == 0 { 4.0 } else { (*d as f32 / mx) * plot.height() };
            let hh = glowkit_core::anim::spring(ui, id.with(("ab", i)), target, 170.0, 19.0).max(2.0) * g;
            let br = Rect::from_min_max(pos2(cx - bw / 2.0, plot.max.y - hh), pos2(cx + bw / 2.0, plot.max.y));
            let is_h = hov == Some(i);
            let c = if *d == 0 {
                neutral(&t)
            } else if is_h {
                glowkit_core::color::shade(accent, -0.08)
            } else {
                accent
            };
            let a = if hov.is_some() && !is_h { 0.55 } else { 1.0 };
            painter.rect_filled(br, cr((bw * 0.35).min(3.0)), fade(c, a));
            if i % 5 == 0 {
                text(&painter, pos2(cx, plot.max.y + 14.0), Align2::CENTER_CENTER, i + 1, f(11.0), faint(&t));
            }
        }
        if let Some(i) = hov.filter(|i| *i < days.len()) {
            let cx = plot.min.x + slot * (i as f32 + 0.5);
            let short: String = mname.chars().take(3).collect();
            let s = if days[i] == 0 {
                format!("{short} {} · idle", i + 1)
            } else {
                format!("{short} {} · {} runs", i + 1, days[i])
            };
            tooltip(&painter, &t, pos2(cx, plot.min.y + 28.0), &s, agents);
        }

        // ---- tokens trend
        let tk = Rect::from_min_max(pos2(right.min.x, agents.max.y + gap), right.max);
        frame(&painter, &t, tk, 16.0);
        text(&painter, pos2(tk.min.x + pad, tk.min.y + 26.0), Align2::LEFT_CENTER, "Tokens", f(13.0), dim(&t));
        let total_t: f32 = self.tokens.iter().sum();
        let tt = tween(ui, id.with("tok"), total_t, 0.0, 1.1);
        let hr = text(
            &painter,
            pos2(tk.min.x + pad, tk.min.y + 52.0),
            Align2::LEFT_CENTER,
            format!("{:.1}M", tt),
            f(20.0),
            t.foreground,
        );
        chip(&painter, &t, pos2(hr.max.x + 8.0, hr.center().y + 1.0), "+9.2%", true);
        let plot = Rect::from_min_max(pos2(tk.min.x + pad, tk.min.y + 78.0), pos2(tk.max.x - pad, tk.max.y - 16.0));
        let n = self.tokens.len();
        let tmax = self.tokens.iter().copied().fold(1.0, f32::max) * 1.1;
        let pts: Vec<Pos2> = self
            .tokens
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let vv = tween(ui, id.with(("tk", i)), *v, *v, 0.6);
                pos2(plot.min.x + plot.width() * i as f32 / (n - 1) as f32, plot.max.y - plot.height() * vv / tmax)
            })
            .collect();
        for k in 0..3 {
            let y = plot.min.y + plot.height() * k as f32 / 2.0;
            painter.line_segment([pos2(plot.min.x, y), pos2(plot.max.x, y)], Stroke::new(1.0, fade(hairline(&t), 0.6)));
        }
        let draw = ease_out(entry_k(ui, el, 0.4, 1.2));
        let clip = Rect::from_min_max(
            pos2(plot.min.x - 4.0, plot.min.y - 10.0),
            pos2(plot.min.x + (plot.width() + 8.0) * draw, plot.max.y + 2.0),
        );
        let lp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        paint::area_fill(&lp, &pts, plot.max.y, fade(accent, 0.22), fade(accent, 0.0));
        lp.add(egui::Shape::line(pts.clone(), Stroke::new(2.0, accent)));
        let ptr = response.hover_pos().filter(|q| plot.expand(8.0).contains(*q));
        let mut hi = ptr.map(|q| (((q.x - plot.min.x) / plot.width() * (n - 1) as f32).round() as usize).min(n - 1));
        if hi.is_none() && self.autoplay && idle > 1.5 && draw >= 1.0 {
            hi = Some(((idle * 0.6) as usize * 5 + 3) % n);
        }
        if let Some(i) = hi {
            let q = pts[i];
            painter.line_segment(
                [pos2(q.x, plot.min.y), pos2(q.x, plot.max.y)],
                Stroke::new(1.0, fade(t.muted_foreground, 0.4)),
            );
            painter.circle_filled(q, 5.0, tile_bg(&t));
            painter.circle_stroke(q, 4.0, Stroke::new(2.0, accent));
            tooltip(&painter, &t, pos2(q.x, q.y - 8.0), &format!("Week {} · {:.1}M", i + 1, self.tokens[i]), tk);
        }
        response
    }
}

fn tooltip(painter: &egui::Painter, t: &Theme, anchor: Pos2, s: &str, bounds: Rect) {
    let w = text_w(painter, s, f(12.0)) + 18.0;
    let x = (anchor.x - w / 2.0).clamp(bounds.min.x + 6.0, bounds.max.x - 6.0 - w);
    let r = Rect::from_min_size(pos2(x, anchor.y - 30.0), vec2(w, 24.0));
    let bg = if t.dark { mix(t.muted, t.foreground, 0.12) } else { Color32::from_rgb(24, 24, 27) };
    paint::soft_shadow(painter, r, 7.0, 10.0, vec2(0.0, 3.0), Color32::from_black_alpha(40));
    painter.rect_filled(r, cr(7.0), bg);
    text(painter, r.center(), Align2::CENTER_CENTER, s, f(12.0), Color32::WHITE);
}
