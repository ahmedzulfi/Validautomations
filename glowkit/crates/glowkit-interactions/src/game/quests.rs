//! Quest tracker (HUD) and quest log (journal panel).

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use glowkit_core::anim::{follow, spring};
use glowkit_core::color::{hex, mix, shade, with_alpha};
use glowkit_core::paint;

/// A quest: title, description, objectives with progress and rewards.
#[derive(Clone, Debug, PartialEq)]
pub struct GameQuest {
    /// Title.
    pub title: String,
    /// Journal description.
    pub desc: String,
    /// Who gave the quest.
    pub giver: String,
    /// Main story quest (gold) or side quest (silver).
    pub main: bool,
    /// Objectives: (text, current, target).
    pub objectives: Vec<(String, u32, u32)>,
    /// Experience reward.
    pub xp: u32,
    /// Gold reward.
    pub gold: u32,
    /// Item rewards.
    pub items: Vec<GameItem>,
}

impl GameQuest {
    /// A quest titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            desc: String::new(),
            giver: String::new(),
            main: false,
            objectives: Vec::new(),
            xp: 0,
            gold: 0,
            items: Vec::new(),
        }
    }
    /// Description.
    pub fn desc(mut self, d: impl Into<String>) -> Self {
        self.desc = d.into();
        self
    }
    /// Quest giver.
    pub fn giver(mut self, g: impl Into<String>) -> Self {
        self.giver = g.into();
        self
    }
    /// Mark as a main story quest.
    pub fn main(mut self, m: bool) -> Self {
        self.main = m;
        self
    }
    /// Add an objective with progress `current` / `target`.
    pub fn objective(mut self, text: impl Into<String>, current: u32, target: u32) -> Self {
        self.objectives.push((text.into(), current, target.max(1)));
        self
    }
    /// Rewards.
    pub fn rewards(mut self, xp: u32, gold: u32, items: Vec<GameItem>) -> Self {
        self.xp = xp;
        self.gold = gold;
        self.items = items;
        self
    }
    /// All objectives complete.
    pub fn complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|(_, c, t)| c >= t)
    }
}

fn quest_mark(painter: &Painter, c: Pos2, main: bool, s: f32) {
    let col = if main { hex("#FFC94A") } else { hex("#C9D3E0") };
    painter.add(Shape::convex_polygon(
        vec![c + vec2(0.0, -s), c + vec2(s * 0.7, 0.0), c + vec2(0.0, s), c + vec2(-s * 0.7, 0.0)],
        col,
        Stroke::new(1.0, Color32::from_black_alpha(200)),
    ));
    painter.add(Shape::convex_polygon(
        vec![c + vec2(0.0, -s), c + vec2(s * 0.7, 0.0), c],
        with_alpha(Color32::WHITE, 0.45),
        Stroke::NONE,
    ));
}

fn check_box(painter: &Painter, c: Pos2, done: bool, k: f32, accent: Color32, line: Color32) {
    let r = Rect::from_center_size(
        c,
        Vec2::splat(12.0 * (1.0 + 0.4 * (k * std::f32::consts::PI).sin() * (k < 1.0) as i32 as f32)),
    );
    painter.rect_stroke(r, 2.0, Stroke::new(1.5, line), egui::StrokeKind::Inside);
    if done {
        painter.rect_filled(r.shrink(1.0), 2.0, accent);
        let e = (k * 1.5).min(1.0);
        let a = r.left_center() + vec2(2.5, 0.0);
        let b = r.center_bottom() + vec2(-1.0, -3.0);
        let cc = r.right_top() + vec2(-2.0, 2.5);
        let pts = if e < 0.4 { vec![a, a + (b - a) * (e / 0.4)] } else { vec![a, b, b + (cc - b) * ((e - 0.4) / 0.6)] };
        painter.add(Shape::line(pts, Stroke::new(2.0, Color32::from_black_alpha(230))));
    }
}

/// A compact HUD quest tracker. New quests slide in with a "NEW" flash; when an objective's
/// count goes up the number pops and its bar fills; finishing an objective ticks its box with a
/// burst while a strike-through sweeps across, and a finished quest lights up with a
/// QUEST COMPLETE banner.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{GameQuest, GameQuestTracker};
/// let quests = vec![GameQuest::new("Wolves at the Gate").main(true).objective("Slay dire wolves", 3, 5)];
/// ui.add(GameQuestTracker::new(&quests));
/// ```
pub struct GameQuestTracker<'a> {
    quests: &'a [GameQuest],
    width: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
}

impl<'a> GameQuestTracker<'a> {
    /// A tracker listing `quests`.
    pub fn new(quests: &'a [GameQuest]) -> Self {
        Self { quests, width: 300.0, style: GameStyle::SciFi, palette: None }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(180.0);
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

fn str_id(id: Id, s: &str) -> Id {
    id.with(s)
}

impl egui::Widget for GameQuestTracker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let title_f = display_font(ui, 14.0);
        let obj_f = semi_font(ui, 12.5);
        let line_h = 22.0;
        let total_h: f32 =
            self.quests.iter().map(|q| 30.0 + q.objectives.len() as f32 * line_h + 12.0).sum::<f32>() + 30.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(self.width, total_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let painter = ui.painter();
        let hud = pal.hud;
        let shadow = Color32::from_black_alpha(190);
        outlined(
            painter,
            pos2(rect.max.x, rect.min.y + 10.0),
            Align2::RIGHT_CENTER,
            "QUESTS",
            FontId::monospace(11.0),
            with_alpha(hud, 0.7),
            shadow,
        );
        painter.line_segment(
            [pos2(rect.max.x - 120.0, rect.min.y + 20.0), pos2(rect.max.x, rect.min.y + 20.0)],
            Stroke::new(1.0, with_alpha(hud, 0.3)),
        );
        let mut y = rect.min.y + 30.0;
        let mut animating = false;
        for q in self.quests {
            let qid = str_id(id, &q.title);
            let first: f64 = glowkit_core::state(ui, qid.with("first"), || now);
            let age = (now - first) as f32;
            let k = glowkit_core::ease(glowkit_core::Easing::OutCubic, (age / 0.5).min(1.0));
            let x_off = (1.0 - k) * 60.0;
            let a = k;
            if age < 2.5 {
                animating = true;
            }
            let qh = 30.0 + q.objectives.len() as f32 * line_h;
            let block = Rect::from_min_max(pos2(rect.min.x, y - 4.0), pos2(rect.max.x, y + qh));
            let complete = q.complete();
            let (since_c, _, _) = changed(ui, qid.with("done"), complete as i32 as f32);
            if complete && since_c < 3.0 {
                animating = true;
            }
            // backing gradient
            paint::gradient_rect(
                painter,
                block.translate(vec2(x_off, 0.0)),
                6.0,
                0.0,
                &[Color32::TRANSPARENT, with_alpha(Color32::BLACK, 0.45 * a)],
            );
            if age < 1.2 {
                let f = 1.0 - age / 1.2;
                paint::gradient_rect(
                    painter,
                    block.translate(vec2(x_off, 0.0)),
                    6.0,
                    0.0,
                    &[Color32::TRANSPARENT, with_alpha(pal.accent, 0.5 * f)],
                );
                outlined(
                    painter,
                    pos2(block.min.x + x_off + 6.0, y + 8.0),
                    Align2::LEFT_CENTER,
                    "NEW",
                    FontId::monospace(10.0),
                    with_alpha(pal.warn, f.min(1.0) * a),
                    shadow,
                );
            }
            let tx = rect.max.x - 10.0 + x_off;
            quest_mark(painter, pos2(tx - 2.0, y + 8.0), q.main, 7.0);
            let tc = if q.main { hex("#FFD87A") } else { hud };
            outlined(
                painter,
                pos2(tx - 16.0, y + 8.0),
                Align2::RIGHT_CENTER,
                &q.title,
                title_f.clone(),
                with_alpha(tc, a),
                with_alpha(shadow, a),
            );
            let mut oy = y + 30.0;
            for (oi, (text, cur, target)) in q.objectives.iter().enumerate() {
                let oid = qid.with(("obj", oi));
                let (since, prev, _) = changed(ui, oid, *cur as f32);
                let done = cur >= target;
                let just_done = done && prev < *target as f32 && since < 1.2;
                if since < 1.2 {
                    animating = true;
                }
                let ck = if just_done { since / 0.4 } else { 1.0 };
                let col = if done { with_alpha(hud, 0.55 * a) } else { with_alpha(hud, 0.92 * a) };
                let cnt = format!("{}/{}", cur.min(target), target);
                let pop_k = if since < 0.35 && *cur as f32 > prev { 1.0 - since / 0.35 } else { 0.0 };
                let cf = FontId::monospace(12.0 * (1.0 + 0.45 * pop_k));
                let cw = outlined(
                    painter,
                    pos2(tx, oy),
                    Align2::RIGHT_CENTER,
                    &cnt,
                    cf,
                    mix(col, pal.warn, pop_k),
                    with_alpha(shadow, a),
                )
                .width();
                let txt_r = outlined(
                    painter,
                    pos2(tx - cw - 10.0, oy),
                    Align2::RIGHT_CENTER,
                    text,
                    obj_f.clone(),
                    col,
                    with_alpha(shadow, a),
                );
                let cb = pos2(txt_r.min.x - 12.0, oy);
                check_box(painter, cb, done, ck.min(1.0), pal.good, with_alpha(hud, 0.8 * a));
                if done {
                    let sk = if just_done { ((since - 0.15) / 0.35).clamp(0.0, 1.0) } else { 1.0 };
                    painter.line_segment(
                        [pos2(txt_r.min.x - 2.0, oy), pos2(txt_r.min.x - 2.0 + (txt_r.width() + 4.0) * sk, oy)],
                        Stroke::new(1.5, with_alpha(hud, 0.6 * a)),
                    );
                }
                if just_done {
                    burst(painter, cb, since, 10, oi as u64 + 3, 140.0, 40.0, pal.good, 2.2);
                    ring(painter, cb, since, 0.5, 6.0, 22.0, pal.good, 2.0);
                }
                if !done && *target > 1 {
                    let pw = 60.0;
                    let pr = Rect::from_min_size(pos2(tx - pw, oy + 8.0), vec2(pw, 2.0));
                    let fill = follow(ui, oid.with("fill"), *cur as f32 / *target as f32, 6.0);
                    painter.rect_filled(pr, 1.0, with_alpha(hud, 0.18 * a));
                    painter.rect_filled(
                        Rect::from_min_size(pr.min, vec2(pw * fill, 2.0)),
                        1.0,
                        with_alpha(pal.accent, a),
                    );
                }
                oy += line_h;
            }
            if complete && since_c < 3.0 {
                let f = if since_c < 2.4 { 1.0 } else { 1.0 - (since_c - 2.4) / 0.6 };
                let s = pop((since_c / 0.4).min(1.0));
                let c = block.center() + vec2(x_off, 0.0);
                let banner = Rect::from_center_size(c, vec2(block.width() * 0.9 * s, 26.0));
                paint::glow_ellipse(painter, c, vec2(block.width() * 0.6, 30.0), with_alpha(hex("#FFC94A"), 0.4 * f));
                paint::gradient_rect(
                    painter,
                    banner,
                    3.0,
                    0.0,
                    &[
                        with_alpha(hex("#FFC94A"), 0.0),
                        with_alpha(hex("#FFC94A"), 0.95 * f),
                        with_alpha(hex("#FFC94A"), 0.0),
                    ],
                );
                let tf = FontId::new(13.0 * s.max(0.1), title_f.family.clone());
                painter.text(c, Align2::CENTER_CENTER, "QUEST COMPLETE", tf, with_alpha(hex("#2A1800"), f));
                burst(painter, c, since_c, 24, 99, 300.0, 80.0, hex("#FFC94A"), 2.8);
            }
            y += qh + 12.0;
        }
        if animating {
            ui.ctx().request_repaint();
        }
        resp
    }
}

/// A quest journal: quests grouped into Main and Side on the left (the selection highlight
/// glides between them), and the selected quest's giver, description, objectives with progress
/// bars and rewards on the right. Switching quests slides the details in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let quests: Vec<glowkit_interactions::GameQuest> = vec![];
/// let mut sel = 0usize;
/// ui.add(glowkit_interactions::GameQuestLog::new(&quests, &mut sel));
/// ```
pub struct GameQuestLog<'a> {
    quests: &'a [GameQuest],
    selected: &'a mut usize,
    height: f32,
    style: GameStyle,
    palette: Option<GamePalette>,
    autoplay: bool,
    cues: Option<&'a mut Vec<GameCue>>,
}

impl<'a> GameQuestLog<'a> {
    /// A journal over `quests`, editing the `selected` index.
    pub fn new(quests: &'a [GameQuest], selected: &'a mut usize) -> Self {
        Self { quests, selected, height: 380.0, style: GameStyle::Fantasy, palette: None, autoplay: false, cues: None }
    }
    /// Height in points.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(240.0);
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
    /// Demo mode: page through quests while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
    /// Collect feedback cues ([`GameCue::Tick`] on selection).
    pub fn cues(mut self, c: &'a mut Vec<GameCue>) -> Self {
        self.cues = Some(c);
        self
    }
}

impl egui::Widget for GameQuestLog<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let pal = self.palette.unwrap_or_else(|| self.style.palette());
        let head_f = display_font(ui, 20.0);
        let item_f = semi_font(ui, 13.0);
        let sec_f = display_font(ui, 12.0);
        let w = ui.available_width().min(760.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height), Sense::hover());
        if !ui.is_rect_visible(rect) || self.quests.is_empty() {
            return resp;
        }
        let id = resp.id;
        let n = self.quests.len();
        let mut sel = (*self.selected).min(n - 1);
        // order: main first, then side
        let mut order: Vec<usize> = (0..n).filter(|i| self.quests[*i].main).collect();
        let n_main = order.len();
        order.extend((0..n).filter(|i| !self.quests[*i].main));
        let list_w = (w * 0.36).clamp(170.0, 260.0);
        let list = Rect::from_min_max(rect.min + vec2(16.0, 52.0), pos2(rect.min.x + 16.0 + list_w, rect.max.y - 16.0));
        let row_h = 30.0;
        let mut row_y = Vec::with_capacity(n);
        let mut y = list.min.y;
        let mut headers = Vec::new();
        for (k, qi) in order.iter().enumerate() {
            if k == 0 && n_main > 0 {
                headers.push((y, "MAIN"));
                y += 22.0;
            }
            if k == n_main {
                headers.push((y + 4.0, "SIDE"));
                y += 26.0;
            }
            row_y.push((*qi, y));
            y += row_h;
        }
        let mut any = false;
        for (qi, ry) in &row_y {
            let r = Rect::from_min_size(pos2(list.min.x, *ry), vec2(list_w, row_h - 2.0));
            let rr = ui.interact(r, id.with(("q", *qi)), Sense::click());
            any |= rr.hovered();
            if rr.clicked() {
                sel = *qi;
            }
        }
        let idle = idle_for(ui, id, any);
        if self.autoplay && idle > 2.5 {
            ui.ctx().request_repaint();
            let k = ((idle - 2.5) / 2.6) as usize;
            sel = order[(k + 1) % n];
        }
        if sel != *self.selected {
            *self.selected = sel;
            push_cue(&mut self.cues, GameCue::Tick);
        }
        let (since, _, _) = changed(ui, id.with("sel"), sel as f32);
        if since < 0.5 {
            ui.ctx().request_repaint();
        }
        let painter = ui.painter();
        frame(painter, rect, self.style, &pal, 0.0);
        painter.text(
            pos2(rect.min.x + 22.0, rect.min.y + 26.0),
            Align2::LEFT_CENTER,
            "JOURNAL",
            head_f.clone(),
            pal.text,
        );
        let done_n = self.quests.iter().filter(|q| q.complete()).count();
        painter.text(
            pos2(rect.max.x - 22.0, rect.min.y + 26.0),
            Align2::RIGHT_CENTER,
            format!("{done_n} / {n} COMPLETE"),
            FontId::monospace(11.0),
            pal.dim,
        );
        for (hy, label) in headers {
            painter.text(pos2(list.min.x + 4.0, hy + 9.0), Align2::LEFT_CENTER, label, sec_f.clone(), pal.dim);
            painter.line_segment(
                [pos2(list.min.x + 44.0, hy + 9.0), pos2(list.max.x, hy + 9.0)],
                Stroke::new(1.0, with_alpha(pal.dim, 0.3)),
            );
        }
        let sel_y = row_y.iter().find(|(q, _)| *q == sel).map(|(_, y)| *y).unwrap_or(list.min.y);
        let hy = spring(ui, id.with("hy"), sel_y, 360.0, 26.0);
        let hl = Rect::from_min_size(pos2(list.min.x, hy), vec2(list_w, row_h - 2.0));
        match self.style {
            GameStyle::Fantasy => {
                paint::gradient_rect(
                    painter,
                    hl,
                    3.0,
                    0.0,
                    &[with_alpha(hex("#8E2F25"), 0.9), with_alpha(hex("#8E2F25"), 0.25)],
                );
            }
            _ => {
                paint::gradient_rect(
                    painter,
                    hl,
                    3.0,
                    0.0,
                    &[with_alpha(pal.accent, 0.35), with_alpha(pal.accent, 0.05)],
                );
                painter.rect_filled(Rect::from_min_size(hl.min, vec2(3.0, hl.height())), 0.0, pal.accent);
            }
        }
        for (qi, ry) in &row_y {
            let q = &self.quests[*qi];
            let on = *qi == sel;
            let c = pos2(list.min.x + 14.0, ry + row_h / 2.0 - 1.0);
            quest_mark(painter, c, q.main, 6.0);
            let tc = if on {
                if self.style == GameStyle::Fantasy {
                    hex("#FBEFD2")
                } else {
                    pal.text
                }
            } else {
                pal.text
            };
            let tc = if q.complete() && !on { with_alpha(tc, 0.45) } else { tc };
            painter.text(pos2(list.min.x + 28.0, c.y), Align2::LEFT_CENTER, &q.title, item_f.clone(), tc);
            if q.complete() {
                painter.text(
                    pos2(list.max.x - 8.0, c.y),
                    Align2::RIGHT_CENTER,
                    "✔",
                    FontId::proportional(12.0),
                    if self.style == GameStyle::Fantasy { shade(pal.good, -0.3) } else { pal.good },
                );
            }
        }
        // divider
        let dx = list.max.x + 16.0;
        painter.line_segment([pos2(dx, list.min.y), pos2(dx, list.max.y)], Stroke::new(1.0, with_alpha(pal.dim, 0.35)));
        // detail
        let q = &self.quests[sel];
        let e = glowkit_core::ease(glowkit_core::Easing::OutCubic, (since / 0.35).min(1.0));
        let mut p2 = painter.clone();
        p2.set_opacity(e);
        let dx0 = dx + 18.0 + (1.0 - e) * 24.0;
        let dw = rect.max.x - 22.0 - dx0;
        let mut y = list.min.y + 4.0;
        p2.text(pos2(dx0, y + 10.0), Align2::LEFT_CENTER, &q.title, head_f, pal.text);
        y += 28.0;
        if !q.giver.is_empty() {
            p2.text(
                pos2(dx0, y),
                Align2::LEFT_CENTER,
                format!("Given by {}", q.giver),
                FontId::proportional(11.5),
                pal.dim,
            );
            y += 18.0;
        }
        let g = p2.layout(q.desc.clone(), FontId::proportional(12.5), pal.text, dw);
        let gh = g.size().y;
        p2.galley(pos2(dx0, y), g, pal.text);
        y += gh + 14.0;
        p2.text(pos2(dx0, y), Align2::LEFT_CENTER, "OBJECTIVES", sec_f.clone(), pal.dim);
        y += 18.0;
        for (oi, (text, cur, target)) in q.objectives.iter().enumerate() {
            let done = cur >= target;
            check_box(
                &p2,
                pos2(dx0 + 6.0, y),
                done,
                1.0,
                if self.style == GameStyle::Fantasy { shade(pal.good, -0.2) } else { pal.good },
                with_alpha(pal.text, 0.7),
            );
            p2.text(
                pos2(dx0 + 20.0, y),
                Align2::LEFT_CENTER,
                text,
                item_f.clone(),
                if done { with_alpha(pal.text, 0.5) } else { pal.text },
            );
            p2.text(
                pos2(dx0 + dw, y),
                Align2::RIGHT_CENTER,
                format!("{}/{}", cur.min(target), target),
                FontId::monospace(11.5),
                pal.dim,
            );
            let bar = Rect::from_min_size(pos2(dx0 + 20.0, y + 9.0), vec2(dw - 20.0, 3.0));
            let k = glowkit_core::anim::stagger(since, oi, 0.08, 0.5);
            let f = (*cur as f32 / *target as f32).min(1.0) * glowkit_core::ease(glowkit_core::Easing::OutCubic, k);
            p2.rect_filled(bar, 1.5, with_alpha(pal.dim, 0.2));
            p2.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * f, 3.0)),
                1.5,
                if done { pal.good } else { pal.accent },
            );
            y += 26.0;
        }
        // rewards
        let ry = rect.max.y - 62.0;
        p2.text(pos2(dx0, ry), Align2::LEFT_CENTER, "REWARDS", sec_f, pal.dim);
        let mut x = dx0;
        let cy = ry + 28.0;
        for (icon, label) in
            [(GameIcon::Star, format!("{} XP", thousands(q.xp as i64))), (GameIcon::Coin, thousands(q.gold as i64))]
        {
            icon.paint(&p2, pos2(x + 10.0, cy), 18.0, icon.tint());
            p2.text(pos2(x + 24.0, cy), Align2::LEFT_CENTER, &label, FontId::proportional(12.5), pal.text);
            x += 34.0 + text_w(&p2, &label, &FontId::proportional(12.5));
        }
        for (k, it) in q.items.iter().enumerate() {
            let s = pop(glowkit_core::anim::stagger(since, k, 0.1, 0.4));
            let r = Rect::from_center_size(pos2(x + 18.0, cy), Vec2::splat(36.0 * s.max(0.01)));
            slot(&p2, r, self.style, &pal, Some(it.rarity), 0.0);
            it.icon.paint(&p2, r.center(), 24.0 * s, it.tint());
            x += 44.0;
        }
        resp
    }
}
