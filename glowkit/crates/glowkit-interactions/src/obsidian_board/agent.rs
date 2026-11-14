//! Agent UI pieces: a limits / context-window card, a multi-step progress block, thinking
//! indicators, a composer loading rim and a web-search trail.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};
use glowkit_core::color::{fade, gradient, mix};
use glowkit_core::{hex, paint, Theme};
use std::f32::consts::{PI, TAU};

// =============================================================================================
// Agent limits

/// A token bucket of the context window in [`BoardAgentLimits`].
#[derive(Clone, Debug, PartialEq)]
pub struct BoardBucket {
    /// Bucket name ("Messages").
    pub name: String,
    /// Tokens used.
    pub tokens: f32,
    /// Colour (`None` = BoardUI accent for its position).
    pub color: Option<Color32>,
}

impl BoardBucket {
    /// A bucket `name` holding `tokens`.
    pub fn new(name: impl Into<String>, tokens: f32) -> Self {
        Self { name: name.into(), tokens: tokens.max(0.0), color: None }
    }
}

/// A plan usage limit row in [`BoardAgentLimits`].
#[derive(Clone, Debug, PartialEq)]
pub struct BoardLimit {
    /// Limit name ("5-hour limit").
    pub name: String,
    /// Reset text ("Resets in 2 hr 46 min").
    pub reset: String,
    /// Used fraction `0..1`.
    pub used: f32,
}

impl BoardLimit {
    /// A limit row.
    pub fn new(name: impl Into<String>, reset: impl Into<String>, used: f32) -> Self {
        Self { name: name.into(), reset: reset.into(), used: used.clamp(0.0, 1.0) }
    }
}

/// An agent budget card: a context-window bar segmented by token bucket with a used / max
/// readout, then plan usage limits with reset times and slim progress bars. Clicking the
/// context header grows the whole card (spring height) to reveal the per-bucket breakdown and
/// the free space; segments and bars fill in on entry and glide on change.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardAgentLimits, BoardBucket, BoardLimit};
/// ui.add(BoardAgentLimits::new(1_000_000.0)
///     .buckets(vec![BoardBucket::new("Messages", 520_000.0), BoardBucket::new("Tools", 180_000.0)])
///     .limits(vec![BoardLimit::new("5-hour limit", "Resets in 2 hr 46 min", 0.38)]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardAgentLimits {
    max: f32,
    buckets: Vec<BoardBucket>,
    plan: String,
    limits: Vec<BoardLimit>,
    width: f32,
    autoplay: bool,
}

impl BoardAgentLimits {
    /// A limits card for a context window of `max` tokens, with sample buckets and limits.
    pub fn new(max: f32) -> Self {
        Self {
            max: max.max(1.0),
            buckets: vec![
                BoardBucket::new("Messages", 516_000.0),
                BoardBucket::new("System tools", 96_000.0),
                BoardBucket::new("Memory files", 68_000.0),
                BoardBucket::new("Skills", 52_000.0),
                BoardBucket::new("MCP tools", 38_000.0),
                BoardBucket::new("Custom agents", 22_000.0),
                BoardBucket::new("System prompt", 14_000.0),
            ],
            plan: "Plan usage limits · Max (5x)".into(),
            limits: vec![
                BoardLimit::new("5-hour limit", "Resets in 2 hr 46 min", 0.38),
                BoardLimit::new("Weekly · all models", "Resets Tue 3:00 PM", 0.03),
                BoardLimit::new("Weekly · Pro", "Resets Tue 3:00 PM", 0.05),
            ],
            width: 450.0,
            autoplay: false,
        }
    }
    /// Context buckets (in display order).
    pub fn buckets(mut self, b: Vec<BoardBucket>) -> Self {
        self.buckets = b;
        self
    }
    /// Plan header text.
    pub fn plan(mut self, p: impl Into<String>) -> Self {
        self.plan = p.into();
        self
    }
    /// Usage limit rows.
    pub fn limits(mut self, l: Vec<BoardLimit>) -> Self {
        self.limits = l;
        self
    }
    /// Card width in points (default 450).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Expand and collapse the breakdown by itself while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

impl egui::Widget for BoardAgentLimits {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let cols = [p[BLUE], p[PURPLE], p[PINK], p[YELLOW], p[EMERALD], p[TEAL], p[ORANGE], p[LIME]];
        let id = ui.next_auto_id();
        let open_flag: bool = glowkit_core::state(ui, id.with("open"), || false);
        let last_rect: Rect = glowkit_core::state(ui, id.with("rect"), || Rect::NOTHING);
        let idle = idle_for(ui, id, ui.rect_contains_pointer(last_rect));
        let auto_open = self.autoplay && idle > 2.5 && (((idle - 2.5) / 3.2) as usize % 2 == 0);
        let open = if self.autoplay && idle > 2.5 { auto_open } else { open_flag };
        let ok = glowkit_core::anim::spring(ui, id.with("openk"), if open { 1.0 } else { 0.0 }, 190.0, 23.0);
        let row_h = 26.0;
        let detail_h = (self.buckets.len() + 1) as f32 * row_h + 10.0;
        let base_h = 70.0 + 44.0 + self.limits.len() as f32 * 44.0 + 6.0;
        let h = base_h + detail_h * ok.max(0.0);
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        glowkit_core::set_state(ui, id.with("rect"), rect);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(12.0));
        let el = entry(ui, id);
        let pad = 16.0;
        frame(&painter, &t, rect, 16.0);

        // header (click to expand)
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), 62.0));
        let hresp = ui.interact(head, id.with("head"), Sense::click());
        if hresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if hresp.clicked() {
            glowkit_core::set_state(ui, id.with("open"), !open);
        }
        let used: f32 = self.buckets.iter().map(|b| b.tokens).sum();
        let used_s = tween(ui, id.with("used"), used, 0.0, 1.0);
        text(
            &painter,
            pos2(rect.min.x + pad, rect.min.y + 24.0),
            Align2::LEFT_CENTER,
            "Context window",
            f(13.5),
            dim(&t),
        );
        let chev_c = pos2(rect.max.x - pad - 5.0, rect.min.y + 24.0);
        // rotate the chevron by drawing up/down mix
        let k = ok.clamp(0.0, 1.0);
        let s = 3.5;
        let pts = vec![
            chev_c + vec2(-s, -s * 0.5 * (1.0 - 2.0 * k)),
            chev_c + vec2(0.0, s * 0.5 * (1.0 - 2.0 * k)),
            chev_c + vec2(s, -s * 0.5 * (1.0 - 2.0 * k)),
        ];
        painter.add(Shape::line(pts, Stroke::new(1.4, dim(&t))));
        let pct = format!("({:.0}%)", used_s / self.max * 100.0);
        let pr =
            text(&painter, pos2(chev_c.x - 14.0, rect.min.y + 24.0), Align2::RIGHT_CENTER, &pct, f(13.5), t.foreground);
        text(
            &painter,
            pos2(pr.min.x - 4.0, rect.min.y + 24.0),
            Align2::RIGHT_CENTER,
            format!("{} / {}", compact(used_s, false), compact(self.max, true)),
            f(13.5),
            dim(&t),
        );
        // segmented bar
        let bar = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + 42.0), vec2(rect.width() - pad * 2.0, 6.0));
        painter.rect_filled(bar, cr(3.0), track(&t));
        let grow = ease_out(entry_k(ui, el, 0.1, 1.0));
        let mut x = bar.min.x;
        let n = self.buckets.len();
        for (i, b) in self.buckets.iter().enumerate() {
            let v = tween(ui, id.with(("b", i)), b.tokens, 0.0, 0.8) * grow;
            let w = bar.width() * v / self.max;
            if w < 0.5 {
                continue;
            }
            let c = b.color.unwrap_or(cols[i % cols.len()]);
            let r = Rect::from_min_size(pos2(x, bar.min.y), vec2((w - 1.5).max(1.0), bar.height()));
            let corner = egui::CornerRadius {
                nw: if i == 0 { 3 } else { 0 },
                sw: if i == 0 { 3 } else { 0 },
                ne: if i + 1 == n { 3 } else { 0 },
                se: if i + 1 == n { 3 } else { 0 },
            };
            painter.rect_filled(r, corner, c);
            x += w;
        }

        // expandable breakdown
        let top = rect.min.y + 62.0;
        if ok > 0.01 {
            let clip = Rect::from_min_max(pos2(rect.min.x, top), pos2(rect.max.x, top + detail_h * ok));
            let mut dp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
            dp.multiply_opacity(ok.clamp(0.0, 1.0));
            let mut rows: Vec<(String, f32, Color32)> = self
                .buckets
                .iter()
                .enumerate()
                .map(|(i, b)| (b.name.clone(), b.tokens, b.color.unwrap_or(cols[i % cols.len()])))
                .collect();
            rows.push(("Free space".into(), (self.max - used).max(0.0), neutral(&t)));
            for (i, (name, v, c)) in rows.iter().enumerate() {
                let y = top + 6.0 + i as f32 * row_h + row_h / 2.0;
                let dy = (1.0 - ok) * 6.0 * (i as f32 * 0.3 + 1.0);
                swatch(&dp, pos2(rect.min.x + pad + 4.5, y - dy), *c);
                text(&dp, pos2(rect.min.x + pad + 16.0, y - dy), Align2::LEFT_CENTER, name, f(13.0), t.foreground);
                text(
                    &dp,
                    pos2(rect.max.x - pad, y - dy),
                    Align2::RIGHT_CENTER,
                    format!("{:.1}%", v / self.max * 100.0),
                    f(12.5),
                    faint(&t),
                );
                text(
                    &dp,
                    pos2(rect.max.x - pad - 54.0, y - dy),
                    Align2::RIGHT_CENTER,
                    compact(*v, false),
                    f(13.0),
                    dim(&t),
                );
            }
        }
        let y0 = top + detail_h * ok.max(0.0);
        painter.line_segment([pos2(rect.min.x + pad, y0), pos2(rect.max.x - pad, y0)], Stroke::new(1.0, hairline(&t)));
        text(&painter, pos2(rect.min.x + pad, y0 + 24.0), Align2::LEFT_CENTER, &self.plan, f(13.5), dim(&t));
        icon_arrow_right(&painter, pos2(rect.max.x - pad - 6.0, y0 + 24.0), 11.0, dim(&t));
        for (i, l) in self.limits.iter().enumerate() {
            let y = y0 + 44.0 + i as f32 * 44.0;
            let a = ease_out(entry_k(ui, el, 0.2 + 0.08 * i as f32, 0.5));
            let pr = text(
                &painter,
                pos2(rect.max.x - pad, y + 12.0),
                Align2::RIGHT_CENTER,
                format!("{:.0}%", l.used * 100.0),
                f(13.5),
                fade(t.foreground, a),
            );
            text(
                &painter,
                pos2(pr.min.x - 12.0, y + 12.0),
                Align2::RIGHT_CENTER,
                &l.reset,
                f(12.5),
                fade(faint(&t), a),
            );
            text(
                &painter,
                pos2(rect.min.x + pad, y + 12.0),
                Align2::LEFT_CENTER,
                &l.name,
                f(13.5),
                fade(t.foreground, a),
            );
            let bar = Rect::from_min_size(pos2(rect.min.x + pad, y + 27.0), vec2(rect.width() - pad * 2.0, 5.0));
            painter.rect_filled(bar, cr(2.5), track(&t));
            let u =
                tween(ui, id.with(("l", i)), l.used, 0.0, 0.9) * ease_out(entry_k(ui, el, 0.3 + 0.1 * i as f32, 0.9));
            if u > 0.0 {
                painter.rect_filled(
                    Rect::from_min_size(bar.min, vec2((bar.width() * u).max(5.0), 5.0)),
                    cr(2.5),
                    p[BLUE],
                );
            }
        }
        response
    }
}

// =============================================================================================
// Agent progress

/// A collapsible progress block for multi-step agent work. Steps arrive with a soft stagger;
/// finished steps get a filled check and a strikethrough, the active step spins a circular
/// progress ring synced to its fraction, and pending steps wait as hollow circles. The header
/// reads "Working · 2 of 5" with a shimmer (or "All steps completed"), and the minimise button
/// folds the list away while a small ring keeps showing overall progress.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardAgentProgress;
/// let steps = ["Read project files", "Update tokens", "Run the build"];
/// ui.add(BoardAgentProgress::new(&steps).progress(1.4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardAgentProgress {
    steps: Vec<String>,
    progress: f32,
    width: f32,
    collapsed: Option<bool>,
}

impl BoardAgentProgress {
    /// A progress block for `steps`.
    pub fn new(steps: &[&str]) -> Self {
        Self { steps: steps.iter().map(|s| s.to_string()).collect(), progress: 0.0, width: 360.0, collapsed: None }
    }
    /// Progress through the steps: `1.4` = first step done, second 40% through.
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = p.max(0.0);
        self
    }
    /// Width in points (default 360).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Force the minimised state (otherwise the minimise button toggles it).
    pub fn collapsed(mut self, c: bool) -> Self {
        self.collapsed = Some(c);
        self
    }
}

impl egui::Widget for BoardAgentProgress {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let id = ui.next_auto_id();
        let stored: bool = glowkit_core::state(ui, id.with("min"), || false);
        let collapsed = self.collapsed.unwrap_or(stored);
        let ck = glowkit_core::anim::spring(ui, id.with("ck"), if collapsed { 1.0 } else { 0.0 }, 200.0, 24.0);
        let n = self.steps.len();
        let row_h = 38.0;
        let full = 46.0 + n as f32 * row_h + 10.0;
        let h = 46.0 + (full - 46.0) * (1.0 - ck).clamp(0.0, 1.0);
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(16.0));
        let el = entry(ui, id);
        sheet(&painter, &t, rect, 14.0);
        let pad = 14.0;
        let done_all = self.progress >= n as f32 - 1e-3;
        let active = (self.progress.floor() as usize).min(n.saturating_sub(1));
        let now = glowkit_core::time(ui);
        let head_y = rect.min.y + 23.0;
        let label =
            if done_all { "All steps completed".to_string() } else { format!("Working · {} of {}", active + 1, n) };
        if done_all {
            text(&painter, pos2(rect.min.x + pad, head_y), Align2::LEFT_CENTER, &label, f(13.5), dim(&t));
        } else {
            let g = painter.layout_no_wrap(label.clone(), f(13.5), t.foreground);
            shimmer_text(
                &painter,
                pos2(rect.min.x + pad, head_y - g.size().y / 2.0),
                &label,
                f(13.5),
                dim(&t),
                t.foreground,
                now * 0.55,
            );
            glowkit_core::animate(ui);
        }
        // overall mini ring when collapsed
        let overall = tween(ui, id.with("overall"), self.progress / n.max(1) as f32, 0.0, 0.5);
        if ck > 0.02 {
            let c = pos2(rect.max.x - pad - 40.0, head_y);
            let a = ck.clamp(0.0, 1.0);
            painter.circle_stroke(c, 7.0, Stroke::new(2.0, fade(track(&t), a)));
            if overall > 0.0 {
                ring_arc(&painter, c, 7.0, -PI / 2.0, TAU * overall.min(1.0), 2.0, fade(p[BLUE], a));
            }
        }
        // minimise button
        let br = Rect::from_center_size(pos2(rect.max.x - pad - 9.0, head_y), vec2(20.0, 20.0));
        let bresp = ui.interact(br, id.with("minbtn"), Sense::click());
        let bh = flag(ui, id.with("bh"), bresp.hovered(), 14.0);
        painter.rect_filled(br, cr(5.0), mix(track(&t), mix(track(&t), t.foreground, 0.1), bh));
        let k = ck.clamp(0.0, 1.0);
        painter.line_segment(
            [pos2(br.min.x + 6.0, br.center().y), pos2(br.max.x - 6.0, br.center().y)],
            Stroke::new(1.5, dim(&t)),
        );
        if k > 0.01 {
            painter.line_segment(
                [pos2(br.center().x, br.center().y - 4.0 * k), pos2(br.center().x, br.center().y + 4.0 * k)],
                Stroke::new(1.5, dim(&t)),
            );
        }
        if bresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if bresp.clicked() {
            glowkit_core::set_state(ui, id.with("min"), !collapsed);
        }

        // steps
        let clip = Rect::from_min_max(pos2(rect.min.x, rect.min.y + 40.0), rect.max);
        let mut sp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        sp.multiply_opacity((1.0 - ck).clamp(0.0, 1.0));
        for (i, s) in self.steps.iter().enumerate() {
            let a = ease_out(entry_k(ui, el, 0.12 + 0.09 * i as f32, 0.5));
            let y = rect.min.y + 46.0 + i as f32 * row_h + row_h / 2.0 - 6.0 + (1.0 - a) * 8.0;
            let ic = pos2(rect.min.x + pad + 7.0, y);
            let frac = (self.progress - i as f32).clamp(0.0, 1.0);
            let done = flag(ui, id.with(("done", i)), frac >= 1.0, 9.0);
            let is_active = !done_all && i == active;
            let mut rp = sp.clone();
            rp.multiply_opacity(a.max(0.001));
            if done > 0.01 {
                rp.circle_filled(ic, 7.5, fade(neutral(&t), done));
                icon_check(&rp, ic, 8.5 * (0.6 + 0.4 * done), fade(tile_bg(&t), done), 1.6);
            }
            if done < 0.99 {
                let ra = 1.0 - done;
                rp.circle_stroke(ic, 7.0, Stroke::new(1.6, fade(if is_active { track(&t) } else { neutral(&t) }, ra)));
                if is_active {
                    let shown = tween(ui, id.with(("af", i)), frac, 0.0, 0.4);
                    let spin = now * 1.4;
                    if shown > 0.01 {
                        ring_arc(&rp, ic, 7.0, -PI / 2.0, TAU * shown, 1.8, fade(p[BLUE], ra));
                    } else {
                        ring_arc(&rp, ic, 7.0, spin * TAU, 1.4, 1.8, fade(p[BLUE], ra));
                    }
                    glowkit_core::animate(ui);
                }
            }
            let tc = if done > 0.5 {
                faint(&t)
            } else if is_active {
                t.foreground
            } else {
                mix(dim(&t), faint(&t), 0.4)
            };
            let tr = text(&rp, pos2(ic.x + 16.0, y), Align2::LEFT_CENTER, s, f(13.5), tc);
            if done > 0.01 {
                let w = tr.width() * ease_out(done);
                rp.line_segment(
                    [pos2(tr.min.x, tr.center().y + 0.5), pos2(tr.min.x + w, tr.center().y + 0.5)],
                    Stroke::new(1.0, faint(&t)),
                );
            }
        }
        response
    }
}

// =============================================================================================
// Agent thinking

/// Indicator style for [`BoardAgentThinking`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoardThinkingStyle {
    /// A 3x3 dot grid crossed by a diagonal wave.
    #[default]
    DotWave,
    /// A 3x3 dot grid with a bright head orbiting the rim.
    DotSpin,
    /// Three twinkling four-point stars.
    Stars,
    /// A comet running an infinity loop.
    Infinity,
}

impl BoardThinkingStyle {
    /// All styles in order.
    pub const ALL: [BoardThinkingStyle; 4] = [Self::DotWave, Self::DotSpin, Self::Stars, Self::Infinity];
    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::DotWave => "Dot wave",
            Self::DotSpin => "Dot spin",
            Self::Stars => "Stars",
            Self::Infinity => "Infinity",
        }
    }
}

/// The "thinking" row that sits above a chat composer while a response is being worked on: an
/// animated indicator (dot wave, dot spin, stars or an infinity comet), a label with a soft
/// shimmer sweeping through it, and an elapsed timer in monospace.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardAgentThinking, BoardThinkingStyle};
/// ui.add(BoardAgentThinking::new("Thinking").style(BoardThinkingStyle::Infinity));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardAgentThinking {
    label: String,
    style: BoardThinkingStyle,
    elapsed: Option<f32>,
    speed: f32,
    color: Option<Color32>,
}

impl BoardAgentThinking {
    /// A thinking row with `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), style: BoardThinkingStyle::DotWave, elapsed: None, speed: 1.0, color: None }
    }
    /// Indicator style.
    pub fn style(mut self, s: BoardThinkingStyle) -> Self {
        self.style = s;
        self
    }
    /// Seconds to show in the timer (default: time since the row appeared).
    pub fn elapsed(mut self, secs: f32) -> Self {
        self.elapsed = Some(secs.max(0.0));
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.clamp(0.1, 4.0);
        self
    }
    /// Indicator colour (default: muted foreground).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
}

/// Paint a thinking indicator in a box of `size` centered at `c`.
pub(crate) fn paint_indicator(
    painter: &Painter,
    style: BoardThinkingStyle,
    c: Pos2,
    size: f32,
    ph: f32,
    col: Color32,
    faint_c: Color32,
) {
    let s = size / 2.0;
    match style {
        BoardThinkingStyle::DotWave => {
            for r in 0..3 {
                for q in 0..3 {
                    let p = c + vec2((q as f32 - 1.0) * s * 0.72, (r as f32 - 1.0) * s * 0.72);
                    let w = ((ph * 1.2 - (r + q) as f32 * 0.16).rem_euclid(1.0) * TAU).sin().max(0.0);
                    painter.circle_filled(p, s * 0.17 * (0.85 + 0.25 * w), mix(faint_c, col, w));
                }
            }
        }
        BoardThinkingStyle::DotSpin => {
            let ring = [(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (1, 2), (0, 2), (0, 1)];
            let head = (ph * 1.3 * 8.0).rem_euclid(8.0);
            painter.circle_filled(c, s * 0.15, faint_c);
            for (k, (q, r)) in ring.iter().enumerate() {
                let p = c + vec2((*q as f32 - 1.0) * s * 0.72, (*r as f32 - 1.0) * s * 0.72);
                let d = (head - k as f32).rem_euclid(8.0);
                let w = (1.0 - d / 4.0).max(0.0);
                painter.circle_filled(p, s * 0.17 * (0.85 + 0.3 * w), mix(faint_c, col, w));
            }
        }
        BoardThinkingStyle::Stars => {
            for (k, (dx, dy, sz)) in [(-0.3, 0.25, 0.75), (0.45, -0.35, 0.45), (-0.45, -0.5, 0.32)].iter().enumerate() {
                let tw = ((ph * 1.1 + k as f32 * 0.33).rem_euclid(1.0) * TAU).sin() * 0.5 + 0.5;
                paint::sparkle(
                    painter,
                    c + vec2(dx * s, dy * s),
                    s * sz * (0.55 + 0.45 * tw),
                    tw * 0.5,
                    mix(faint_c, col, 0.35 + 0.65 * tw),
                );
            }
        }
        BoardThinkingStyle::Infinity => {
            let n = 64;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = TAU * i as f32 / n as f32;
                    let d = 1.0 + a.sin().powi(2);
                    c + vec2(s * 0.95 * a.cos() / d, s * 0.95 * a.sin() * a.cos() / d)
                })
                .collect();
            paint::gradient_polyline(painter, &pts, false, (s * 0.16).max(1.2), |_| faint_c);
            let head = (ph * 0.8).rem_euclid(1.0);
            paint::gradient_polyline(painter, &pts, false, (s * 0.2).max(1.4), paint::comet_fn(head, 0.42, col));
        }
    }
}

impl egui::Widget for BoardAgentThinking {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = f(14.0);
        let id = ui.next_auto_id();
        let el = entry(ui, id);
        let secs = self.elapsed.unwrap_or(el);
        let timer = format!("{secs:.1}s");
        let lw = ui.painter().layout_no_wrap(self.label.clone(), font.clone(), t.foreground).size();
        let tw = ui.painter().layout_no_wrap(timer.clone(), mono(12.5), t.foreground).size().x;
        let (rect, response) = ui.allocate_exact_size(vec2(18.0 + 10.0 + lw.x + 10.0 + tw + 4.0, 24.0), Sense::hover());
        glowkit_core::animate(ui);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        let now = glowkit_core::time(ui) * self.speed;
        let col = self.color.unwrap_or(mix(t.muted_foreground, t.foreground, 0.25));
        let faint_c = mix(track(&t), t.muted_foreground, 0.25);
        let ic = pos2(rect.min.x + 9.0, rect.center().y);
        paint_indicator(painter, self.style, ic, 16.0, now, col, faint_c);
        let lx = rect.min.x + 28.0;
        shimmer_text(
            painter,
            pos2(lx, rect.center().y - lw.y / 2.0),
            &self.label,
            font,
            mix(t.muted_foreground, frame_bg(&t), 0.2),
            t.foreground,
            now * 0.5,
        );
        text(painter, pos2(lx + lw.x + 10.0, rect.center().y + 0.5), Align2::LEFT_CENTER, timer, mono(12.5), faint(&t));
        response
    }
}

// =============================================================================================
// Composer loader

/// A chat composer pill whose rim lights up while the agent works: an iridescent band travels
/// around the rim at constant speed (by arc length), led by a crisp bright line, with a soft
/// bloom bleeding inward behind it. Toggling the loading flag (or pressing send) fades the
/// whole effect in and out so the composer reads as lighting up rather than switching.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::BoardComposerLoader;
/// let mut loading = true;
/// ui.add(BoardComposerLoader::new(&mut loading).placeholder("Ask me anything"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardComposerLoader<'a> {
    loading: &'a mut bool,
    placeholder: String,
    model: String,
    colors: Vec<Color32>,
    speed: f32,
    tail: f32,
    width: f32,
}

impl<'a> BoardComposerLoader<'a> {
    /// A composer whose loading state is `loading` (the send button toggles it).
    pub fn new(loading: &'a mut bool) -> Self {
        Self {
            loading,
            placeholder: "Ask me anything".into(),
            model: "Model 5".into(),
            colors: vec![hex("#5EEAD4"), hex("#60A5FA"), hex("#A78BFA"), hex("#F472B6"), hex("#FBBF24")],
            speed: 260.0,
            tail: 0.5,
            width: 540.0,
        }
    }
    /// Placeholder text.
    pub fn placeholder(mut self, s: impl Into<String>) -> Self {
        self.placeholder = s.into();
        self
    }
    /// Model selector label.
    pub fn model(mut self, s: impl Into<String>) -> Self {
        self.model = s.into();
        self
    }
    /// Colours of the travelling band, head first (at least one).
    pub fn colors(mut self, c: Vec<Color32>) -> Self {
        if !c.is_empty() {
            self.colors = c;
        }
        self
    }
    /// Travel speed in points per second (default 260).
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s.clamp(20.0, 2000.0);
        self
    }
    /// Band length as a fraction of the rim (default 0.5).
    pub fn tail(mut self, t: f32) -> Self {
        self.tail = t.clamp(0.05, 0.95);
        self
    }
    /// Width in points (default 540).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(280.0);
        self
    }
}

impl egui::Widget for BoardComposerLoader<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let p = pal(&t);
        let (outer, response) = ui.allocate_exact_size(vec2(self.width, 76.0), Sense::hover());
        let id = response.id;
        let rect = Rect::from_center_size(outer.center(), vec2(self.width - 24.0, 52.0));
        let on = flag(ui, id.with("on"), *self.loading, 3.5);
        // send button
        let send = Rect::from_center_size(pos2(rect.max.x - 26.0, rect.center().y), vec2(36.0, 36.0));
        let sresp = ui.interact(send, id.with("send"), Sense::click());
        if sresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let mut response = response;
        if sresp.clicked() {
            *self.loading = !*self.loading;
            response.mark_changed();
        }
        if !ui.is_rect_visible(outer) {
            return response;
        }
        let painter = ui.painter_at(outer.expand(30.0));
        let radius = rect.height() / 2.0;
        let now = glowkit_core::time(ui);
        // ambient bloom around the pill
        if on > 0.01 {
            let c0 = self.colors[0];
            let c1 = self.colors[self.colors.len() / 2];
            paint::glow_ellipse(
                &painter,
                rect.center(),
                vec2(rect.width() * 0.62, 52.0),
                fade(mix(c0, c1, 0.5), 0.1 * on),
            );
            glowkit_core::animate(ui);
        }
        let sc = if t.dark { Color32::from_black_alpha(80) } else { Color32::from_rgba_unmultiplied(20, 20, 40, 16) };
        paint::soft_shadow(&painter, rect, radius, 18.0, vec2(0.0, 4.0), sc);
        painter.rect_filled(rect, cr(radius), tile_bg(&t));

        // rim geometry by arc length
        let rim = paint::subdivide_closed(&paint::rounded_rect_points(rect.shrink(0.5), radius - 0.5, 18), 3.0);
        let mut lens = Vec::with_capacity(rim.len() + 1);
        let mut total = 0.0;
        for i in 0..rim.len() {
            lens.push(total);
            total += rim[i].distance(rim[(i + 1) % rim.len()]);
        }
        let head = (now * self.speed / total.max(1.0)).rem_euclid(1.0);
        let tail = self.tail;
        let stops = self.colors.clone();
        let band = |u: f32| -> (f32, Color32) {
            let d = (head - u).rem_euclid(1.0);
            if d > tail {
                return (0.0, Color32::TRANSPARENT);
            }
            let k = 1.0 - d / tail;
            (k, gradient(&stops, d / tail))
        };
        // base hairline
        painter.rect_stroke(
            rect,
            cr(radius),
            Stroke::new(1.0, fade(hairline(&t), 1.0 - 0.6 * on)),
            egui::StrokeKind::Inside,
        );
        if on > 0.01 {
            // inward bloom: glows along the band, clipped to the pill
            let inner = painter.with_clip_rect(rect.shrink(1.0).intersect(painter.clip_rect()));
            let samples = 36;
            for i in 0..samples {
                let d = i as f32 / samples as f32 * tail;
                let u = (head - d).rem_euclid(1.0);
                let target = u * total;
                let idx = lens.partition_point(|l| *l < target).min(rim.len() - 1);
                let pt = rim[idx];
                let (k, c) = band(u);
                let toward = (rect.center() - pt).normalized();
                let ctr = pt + vec2(toward.x * 6.0, toward.y * 10.0);
                paint::glow_ellipse(&inner, ctr, vec2(30.0, 22.0) * (0.55 + 0.45 * k), fade(c, 0.09 * k * k * on));
            }
            // the band itself
            let colored: Vec<Color32> = lens
                .iter()
                .map(|l| {
                    let (k, c) = band(l / total);
                    fade(c, k.powf(1.3) * on)
                })
                .collect();
            stroke_colored(&painter, &rim, &colored, 2.4);
            // crisp refraction line at the head
            let lead: Vec<Pos2> = (0..10)
                .map(|i| {
                    let u = (head - i as f32 * 0.004).rem_euclid(1.0);
                    let idx = lens.partition_point(|l| *l < u * total).min(rim.len() - 1);
                    let q = rim[idx];
                    q + (rect.center() - q).normalized() * 1.2
                })
                .collect();
            paint::gradient_polyline(&painter, &lead, false, 1.3, |s| fade(Color32::WHITE, (1.0 - s) * 0.95 * on));
            paint::glow(&painter, lead[0], 7.0, fade(Color32::WHITE, 0.5 * on));
        }

        // contents
        let plus = pos2(rect.min.x + 26.0, rect.center().y);
        painter.circle_filled(plus, 16.0, frame_bg(&t));
        icon_plus(&painter, plus, 11.0, t.foreground);
        text(
            &painter,
            pos2(rect.min.x + 52.0, rect.center().y),
            Align2::LEFT_CENTER,
            &self.placeholder,
            f(14.0),
            faint(&t),
        );
        let mic = pos2(send.min.x - 26.0, rect.center().y);
        painter.circle_stroke(mic, 16.0, Stroke::new(1.0, hairline(&t)));
        icon_mic(&painter, mic, 15.0, t.foreground);
        let mr = text(
            &painter,
            pos2(mic.x - 46.0, rect.center().y),
            Align2::RIGHT_CENTER,
            &self.model,
            f(13.5),
            t.foreground,
        );
        chevron(&painter, pos2(mr.max.x + 12.0, rect.center().y), 7.0, Dir::Down, dim(&t));
        let sh = flag(ui, id.with("sh"), sresp.hovered(), 14.0);
        painter.circle_filled(send.center(), 18.0, mix(p[BLUE], egui::Color32::from_rgb(20, 90, 230), sh * 0.6));
        if on > 0.5 {
            painter.rect_filled(Rect::from_center_size(send.center(), vec2(10.0, 10.0)), cr(2.5), Color32::WHITE);
        } else {
            icon_arrow_up(&painter, send.center(), 14.0, Color32::WHITE);
        }
        response
    }
}

/// Stroke a closed polyline with a per-vertex colour list.
fn stroke_colored(painter: &Painter, pts: &[Pos2], cols: &[Color32], width: f32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let mut m = egui::epaint::Mesh::default();
    let hw = width / 2.0;
    for i in 0..=n {
        let a = pts[(i + n - 1) % n];
        let b = pts[i % n];
        let c = pts[(i + 1) % n];
        let d = ((c - b).normalized() + (b - a).normalized()).normalized();
        let nrm = vec2(-d.y, d.x);
        let col = cols[i % n];
        m.colored_vertex(b + nrm * (hw + 0.6), Color32::TRANSPARENT);
        m.colored_vertex(b + nrm * hw, col);
        m.colored_vertex(b - nrm * hw, col);
        m.colored_vertex(b - nrm * (hw + 0.6), Color32::TRANSPARENT);
    }
    for i in 0..n as u32 {
        let k = i * 4;
        for j in 0..3 {
            m.add_triangle(k + j, k + j + 1, k + j + 4);
            m.add_triangle(k + j + 1, k + j + 5, k + j + 4);
        }
    }
    painter.add(Shape::mesh(m));
}

// =============================================================================================
// Web search

/// One step of a [`BoardWebSearch`] trail: a query the agent ran or a page it opened.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardSearchStep {
    /// Query text or page title.
    pub text: String,
    /// `true` for a query, `false` for an opened page.
    pub query: bool,
    /// Result count shown at the end of a query row (0 hides it).
    pub results: u32,
    /// Sites found, drawn as overlapping marks (first letter + colour).
    pub sites: Vec<String>,
}

impl BoardSearchStep {
    /// A query row with `results` results.
    pub fn query(text: impl Into<String>, results: u32) -> Self {
        Self { text: text.into(), query: true, results, sites: Vec::new() }
    }
    /// An opened page on `site`.
    pub fn page(title: impl Into<String>, site: impl Into<String>) -> Self {
        Self { text: title.into(), query: false, results: 0, sites: vec![site.into()] }
    }
    /// Sites this step found (drawn as overlapping marks).
    pub fn sites(mut self, s: &[&str]) -> Self {
        self.sites = s.iter().map(|x| x.to_string()).collect();
        self
    }
}

/// The research trail of an agent: a "Ran 3 searches" header with a magnifier, then each query
/// it ran (bulb) or page it opened, hung off a curved guide line, with result counts and
/// overlapping site marks for what each step found. Steps stream in as `progress` advances
/// (rows grow open behind a clipping edge) and a shimmering "Working" row with a timer trails
/// the list until the search is done.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{BoardSearchStep, BoardWebSearch};
/// ui.add(BoardWebSearch::new(vec![
///     BoardSearchStep::query("component library pricing", 5).sites(&["reddit", "github"]),
///     BoardSearchStep::page("Pricing · Example", "example.com"),
/// ]).progress(1.5));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BoardWebSearch {
    steps: Vec<BoardSearchStep>,
    progress: f32,
    elapsed: Option<f32>,
    width: f32,
}

impl BoardWebSearch {
    /// A trail of `steps` (all shown unless [`Self::progress`] is set).
    pub fn new(steps: Vec<BoardSearchStep>) -> Self {
        let n = steps.len() as f32;
        Self { steps, progress: n + 1.0, elapsed: None, width: 560.0 }
    }
    /// Steps revealed so far (fractional = the next row is opening). Past the last step the
    /// "Working" row resolves to "Done".
    pub fn progress(mut self, p: f32) -> Self {
        self.progress = p.max(0.0);
        self
    }
    /// Seconds shown in the timer (default: time since the trail appeared).
    pub fn elapsed(mut self, s: f32) -> Self {
        self.elapsed = Some(s.max(0.0));
        self
    }
    /// Width in points (default 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(260.0);
        self
    }
}

fn site_color(t: &Theme, s: &str) -> Color32 {
    let p = pal(t);
    let h = s.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    [p[ORANGE], p[BLUE], p[EMERALD], p[PURPLE], p[PINK], p[INDIGO], p[TEAL], p[RED]][(h % 8) as usize]
}

/// Overlapping round site marks; returns the width used. `x` is the right edge.
fn site_marks(painter: &Painter, t: &Theme, right: Pos2, sites: &[String], a: f32) -> f32 {
    let r = 8.0;
    let step = 13.5;
    let n = sites.len().min(5);
    let mut x = right.x - r;
    for s in sites.iter().take(n).rev() {
        let c = pos2(x, right.y);
        painter.circle_filled(c, r + 1.5, fade(tile_bg(t), a));
        painter.circle_filled(c, r, fade(site_color(t, s), a));
        let ch: String = s.chars().next().map(|c| c.to_ascii_uppercase().to_string()).unwrap_or_default();
        text(painter, c, Align2::CENTER_CENTER, ch, f(9.5), fade(Color32::WHITE, a));
        x -= step;
    }
    if n == 0 {
        0.0
    } else {
        (n - 1) as f32 * step + r * 2.0
    }
}

impl egui::Widget for BoardWebSearch {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id();
        let row_h = 30.0;
        let n = self.steps.len();
        let shown =
            glowkit_core::anim::spring(ui, id.with("shown"), self.progress.min(n as f32 + 1.0), 120.0, 18.0).max(0.0);
        let rows_vis = shown.min(n as f32);
        let done = shown > n as f32 + 0.5;
        let h = 30.0 + rows_vis * row_h + 32.0;
        let (rect, response) = ui.allocate_exact_size(vec2(self.width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect.expand(8.0));
        let el = entry(ui, id);
        let now = glowkit_core::time(ui);
        let x0 = rect.min.x + 4.0;
        let queries = self.steps.iter().take(rows_vis.ceil() as usize).filter(|s| s.query).count();
        let head_y = rect.min.y + 14.0;
        icon_search(&painter, pos2(x0 + 7.0, head_y), 13.0, dim(&t));
        let title = if queries == 1 { "Ran 1 search".to_string() } else { format!("Ran {queries} searches") };
        text(&painter, pos2(x0 + 22.0, head_y), Align2::LEFT_CENTER, title, f(13.5), dim(&t));
        // guide line
        let gx = x0 + 7.0;
        let guide_c = hairline(&t);
        let st = Stroke::new(1.2, mix(guide_c, t.muted_foreground, 0.25));
        if rows_vis > 0.01 {
            let last_y = rect.min.y + 30.0 + (rows_vis - 0.5).max(0.0) * row_h;
            let r = 7.0;
            painter.line_segment([pos2(gx, head_y + 9.0), pos2(gx, last_y - r)], st);
            // elbow into every visible row
            for i in 0..rows_vis.ceil() as usize {
                let y = rect.min.y + 30.0 + (i as f32 + 0.5) * row_h;
                if y > last_y + 0.5 {
                    break;
                }
                let arc: Vec<Pos2> = (0..=8)
                    .map(|k| {
                        let a = PI - (PI / 2.0) * k as f32 / 8.0;
                        pos2(gx + r + r * a.cos(), y - r + r * a.sin())
                    })
                    .collect();
                if i + 1 < rows_vis.ceil() as usize {
                    painter.line_segment([pos2(gx, y - r), pos2(gx, y + r)], st);
                }
                painter.add(Shape::line(arc, st));
                painter.line_segment([pos2(gx + r, y), pos2(gx + 14.0, y)], st);
            }
        }
        // rows
        let clip = Rect::from_min_max(rect.min, pos2(rect.max.x + 8.0, rect.min.y + 30.0 + rows_vis * row_h));
        let rp = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        for (i, s) in self.steps.iter().enumerate() {
            let k = (rows_vis - i as f32).clamp(0.0, 1.0);
            if k <= 0.0 {
                break;
            }
            let y = rect.min.y + 30.0 + (i as f32 + 0.5) * row_h + (1.0 - ease_out(k)) * 6.0;
            let a = ease_out(k);
            let ix = gx + 26.0;
            if s.query {
                icon_bulb(&rp, pos2(ix, y), 13.0, fade(dim(&t), a));
            } else {
                rp.circle_stroke(pos2(ix, y), 5.5, Stroke::new(1.2, fade(dim(&t), a)));
                rp.line_segment([pos2(ix - 5.5, y), pos2(ix + 5.5, y)], Stroke::new(1.0, fade(dim(&t), a)));
                rp.line_segment([pos2(ix, y - 5.5), pos2(ix, y + 5.5)], Stroke::new(1.0, fade(dim(&t), a)));
            }
            let mut right = rect.max.x - 4.0;
            if !s.sites.is_empty() {
                right -= site_marks(&rp, &t, pos2(right, y), &s.sites, a) + 10.0;
            }
            if s.results > 0 {
                let rr = text(
                    &rp,
                    pos2(right, y),
                    Align2::RIGHT_CENTER,
                    format!("{} results", s.results),
                    f(12.5),
                    fade(faint(&t), a),
                );
                right = rr.min.x - 14.0;
            }
            // elide long text
            let maxw = right - (ix + 14.0);
            let mut label = s.text.clone();
            if text_w(&rp, &label, f(13.5)) > maxw {
                let mut chars: Vec<char> = label.chars().collect();
                while chars.len() > 3 && text_w(&rp, &format!("{}…", chars.iter().collect::<String>()), f(13.5)) > maxw
                {
                    chars.pop();
                }
                label = format!("{}…", chars.iter().collect::<String>().trim_end());
            }
            // the newest row shimmers while it's the active one
            let active = !done && i + 1 == rows_vis.ceil() as usize;
            if active {
                let g = rp.layout_no_wrap(label.clone(), f(13.5), Color32::WHITE);
                shimmer_text(
                    &rp,
                    pos2(ix + 14.0, y - g.size().y / 2.0),
                    &label,
                    f(13.5),
                    fade(dim(&t), a),
                    fade(t.foreground, a),
                    now * 0.6,
                );
            } else {
                text(
                    &rp,
                    pos2(ix + 14.0, y),
                    Align2::LEFT_CENTER,
                    label,
                    f(13.5),
                    fade(mix(dim(&t), t.foreground, 0.55), a),
                );
            }
        }
        // working / done row
        let wy = rect.min.y + 30.0 + rows_vis * row_h + 16.0;
        let secs = self.elapsed.unwrap_or(el);
        if done {
            icon_check(&painter, pos2(x0 + 7.0, wy), 11.0, faint(&t), 1.4);
            text(&painter, pos2(x0 + 22.0, wy), Align2::LEFT_CENTER, format!("Done in {secs:.1}s"), f(13.5), faint(&t));
        } else {
            icon_sparkles(&painter, pos2(x0 + 7.0, wy), 14.0, faint(&t), now * 0.7);
            let g = painter.layout_no_wrap("Working".into(), f(13.5), Color32::WHITE);
            shimmer_text(
                &painter,
                pos2(x0 + 22.0, wy - g.size().y / 2.0),
                "Working",
                f(13.5),
                faint(&t),
                dim(&t),
                now * 0.5,
            );
            text(
                &painter,
                pos2(x0 + 22.0 + g.size().x + 8.0, wy + 0.5),
                Align2::LEFT_CENTER,
                format!("{secs:.1}s"),
                mono(12.0),
                faint(&t),
            );
            glowkit_core::animate(ui);
        }
        response
    }
}
