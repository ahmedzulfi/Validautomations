//! [`DiffReview`]: review an agent's file changes hunk by hunk.

use super::common::*;
use egui::{pos2, vec2, Align2, CursorIcon, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// Kind of a line in a [`DiffHunk`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiffLineKind {
    /// Unchanged.
    Context,
    /// Added.
    Add,
    /// Removed.
    Del,
}

/// A hunk of changes with a pending accept / reject decision.
#[derive(Clone, Debug)]
pub struct DiffHunk {
    /// The `@@` context text, e.g. `"\"dependencies\""`.
    pub header: String,
    /// First old line number.
    pub old_start: u32,
    /// First new line number.
    pub new_start: u32,
    /// Lines with their kind.
    pub lines: Vec<(DiffLineKind, String)>,
    /// `Some(true)` accepted, `Some(false)` rejected, `None` not reviewed.
    pub decision: Option<bool>,
}

impl DiffHunk {
    /// Parse a hunk from unified-diff body lines (each starting with `' '`, `'+'` or `'-'`).
    pub fn parse(old_start: u32, new_start: u32, header: impl Into<String>, body: &str) -> Self {
        let lines = body
            .lines()
            .map(|l| {
                let (k, rest) = match l.chars().next() {
                    Some('+') => (DiffLineKind::Add, &l[1..]),
                    Some('-') => (DiffLineKind::Del, &l[1..]),
                    Some(' ') => (DiffLineKind::Context, &l[1..]),
                    _ => (DiffLineKind::Context, l),
                };
                (k, rest.to_owned())
            })
            .collect();
        Self { header: header.into(), old_start, new_start, lines, decision: None }
    }
    fn counts(&self) -> (usize, usize) {
        let a = self.lines.iter().filter(|l| l.0 == DiffLineKind::Add).count();
        let d = self.lines.iter().filter(|l| l.0 == DiffLineKind::Del).count();
        (a, d)
    }
    fn old_len(&self) -> usize {
        self.lines.iter().filter(|l| l.0 != DiffLineKind::Add).count()
    }
    fn new_len(&self) -> usize {
        self.lines.iter().filter(|l| l.0 != DiffLineKind::Del).count()
    }
    /// Rows for side-by-side view: (old line, new line) pairs.
    fn split_rows(&self) -> Vec<(SideCell<'_>, SideCell<'_>)> {
        let mut out = Vec::new();
        let (mut o, mut n) = (self.old_start, self.new_start);
        let mut i = 0;
        while i < self.lines.len() {
            let (k, s) = &self.lines[i];
            if *k == DiffLineKind::Context {
                out.push((Some((o, s.as_str(), *k)), Some((n, s.as_str(), *k))));
                o += 1;
                n += 1;
                i += 1;
                continue;
            }
            let mut dels = Vec::new();
            let mut adds = Vec::new();
            while i < self.lines.len() && self.lines[i].0 == DiffLineKind::Del {
                dels.push(self.lines[i].1.as_str());
                i += 1;
            }
            while i < self.lines.len() && self.lines[i].0 == DiffLineKind::Add {
                adds.push(self.lines[i].1.as_str());
                i += 1;
            }
            for r in 0..dels.len().max(adds.len()) {
                let l = dels.get(r).map(|s| {
                    o += 1;
                    (o - 1, *s, DiffLineKind::Del)
                });
                let rr = adds.get(r).map(|s| {
                    n += 1;
                    (n - 1, *s, DiffLineKind::Add)
                });
                out.push((l, rr));
            }
        }
        out
    }
}

type SideCell<'a> = Option<(u32, &'a str, DiffLineKind)>;

/// One changed file in a [`DiffReview`].
#[derive(Clone, Debug)]
pub struct DiffFile {
    /// Path, e.g. `"web/package.json"`.
    pub path: String,
    /// Why the agent changed it.
    pub note: String,
    /// Its hunks.
    pub hunks: Vec<DiffHunk>,
}

impl DiffFile {
    /// A file at `path`.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), note: String::new(), hunks: Vec::new() }
    }
    /// Why the agent changed it.
    pub fn note(mut self, s: impl Into<String>) -> Self {
        self.note = s.into();
        self
    }
    /// Add a hunk.
    pub fn hunk(mut self, h: DiffHunk) -> Self {
        self.hunks.push(h);
        self
    }
}

#[derive(Clone, Copy, Default)]
struct DiffState {
    split: bool,
    focus: usize,
}

/// Review an agent's changes before they land. Each file shows its path, why it changed and its
/// `+/-` counts; each hunk has a header with Accept and Reject and its lines in a unified or
/// side-by-side view (switch at the top) with old / new line numbers and tinted additions and
/// removals. Accepted hunks fold to a green header, rejected ones to a red one. `J` / `K` move
/// the focus ring between hunks, `Y` / `N` decide; the footer counts progress and offers
/// Accept all / Reject all.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{DiffFile, DiffHunk, DiffReview};
/// let mut files = vec![DiffFile::new("web/package.json").hunk(DiffHunk::parse(8, 8, "\"dependencies\"",
///     "   \"react\": \"^19.0.0\",\n+  \"zod\": \"^3.23.8\",\n   \"react-dom\": \"^19.0.0\""))];
/// ui.add(DiffReview::new(&mut files));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct DiffReview<'a> {
    files: &'a mut [DiffFile],
    width: f32,
    split: Option<bool>,
    focus: Option<usize>,
}

impl<'a> DiffReview<'a> {
    /// Review over `files` (decisions are written into their hunks).
    pub fn new(files: &'a mut [DiffFile]) -> Self {
        Self { files, width: 780.0, split: None, focus: None }
    }
    /// Width in points.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(420.0);
        self
    }
    /// Force the side-by-side (`true`) or unified (`false`) view.
    pub fn split(mut self, on: bool) -> Self {
        self.split = Some(on);
        self
    }
    /// Force the focused hunk (global index across files).
    pub fn focus(mut self, i: usize) -> Self {
        self.focus = Some(i);
        self
    }
}

const LINE_H: f32 = 19.0;

impl egui::Widget for DiffReview<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = ui.next_auto_id().with("diff_review");
        let mut st: DiffState = glowkit_core::state(ui, id.with("st"), DiffState::default);
        if let Some(s) = self.split {
            st.split = s;
        }
        if let Some(f) = self.focus {
            st.focus = f;
        }
        let total_hunks: usize = self.files.iter().map(|f| f.hunks.len()).sum();
        let splitk = glowkit_core::anim::spring(ui, id.with("splitk"), if st.split { 1.0 } else { 0.0 }, 360.0, 30.0);
        // heights
        let mut g = 0;
        let mut opens = Vec::new();
        let mut body = 0.0;
        for f in self.files.iter() {
            body += if f.note.is_empty() { 40.0 } else { 58.0 };
            for h in &f.hunks {
                let o = glowkit_core::anim::spring(ui, id.with(("open", g)), if h.decision.is_none() { 1.0 } else { 0.0 }, 300.0, 30.0);
                let rows = if st.split { h.split_rows().len() } else { h.lines.len() };
                body += 38.0 + (rows as f32 * LINE_H + 10.0) * o.max(0.0) + 10.0;
                opens.push(o);
                g += 1;
            }
            body += 6.0;
        }
        let header = 56.0;
        let footer = 56.0;
        let h = header + body + footer;
        let (rect, resp) = alloc_centered(ui, self.width, h, Sense::hover());
        let p = ui.painter_at(rect.expand(14.0));
        panel(&p, &t, rect, 14.0);
        let pad = 16.0;
        // header: title + counts + view switch
        let reviewed = self.files.iter().flat_map(|f| f.hunks.iter()).filter(|h| h.decision.is_some()).count();
        let (adds, dels) = self.files.iter().flat_map(|f| f.hunks.iter()).fold((0, 0), |(a, d), h| {
            let (x, y) = h.counts();
            (a + x, d + y)
        });
        let hy = rect.min.y + header / 2.0;
        let tr = text(&p, pos2(rect.min.x + pad, hy), Align2::LEFT_CENTER, "Files changed", font(14.0), t.foreground, 200.0);
        chip(&p, &t, pos2(tr.max.x + 8.0, hy), Align2::LEFT_CENTER, &format!("{}", self.files.len()), false);
        let ar = text(&p, pos2(tr.max.x + 44.0, hy), Align2::LEFT_CENTER, &format!("+{adds}"), mono(12.0), ink(&t, t.success), 60.0);
        text(&p, pos2(ar.max.x + 6.0, hy), Align2::LEFT_CENTER, &format!("-{dels}"), mono(12.0), ink(&t, t.destructive), 60.0);
        // segmented control
        let seg = Rect::from_min_size(pos2(rect.max.x - pad - 156.0, hy - 15.0), vec2(156.0, 30.0));
        p.rect_filled(seg, cr(9.0), track(&t));
        let half = seg.width() / 2.0;
        let knob = Rect::from_min_size(pos2(seg.min.x + 3.0 + splitk * (half - 3.0), seg.min.y + 3.0), vec2(half - 3.0, 24.0));
        p.rect_filled(knob, cr(7.0), surface(&t));
        for (k, label) in ["Unified", "Split"].iter().enumerate() {
            let r = Rect::from_min_size(pos2(seg.min.x + k as f32 * half, seg.min.y), vec2(half, seg.height()));
            let resp = ui.interact(r, id.with(("seg", k)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if resp.clicked() {
                st.split = k == 1;
            }
            let on = (k == 1) == st.split;
            p.text(r.center(), Align2::CENTER_CENTER, *label, font(12.0), if on { t.foreground } else { dim(&t) });
        }
        hline(&p, &t, rect.min.x, rect.max.x, rect.min.y + header);
        // files
        let mut y = rect.min.y + header + 12.0;
        let mut g = 0usize;
        let mut decide: Vec<(usize, usize, bool)> = Vec::new();
        for (fi, f) in self.files.iter().enumerate() {
            let pr = chip(&p, &t, pos2(rect.min.x + pad, y + 10.0), Align2::LEFT_CENTER, &f.path, true);
            let _ = pr;
            let (a, d) = f.hunks.iter().fold((0, 0), |(a, d), h| {
                let (x, y) = h.counts();
                (a + x, d + y)
            });
            let r = text(&p, pos2(rect.max.x - pad, y + 10.0), Align2::RIGHT_CENTER, &format!("-{d}"), mono(11.5), ink(&t, t.destructive), 60.0);
            text(&p, pos2(r.min.x - 6.0, y + 10.0), Align2::RIGHT_CENTER, &format!("+{a}"), mono(11.5), ink(&t, t.success), 60.0);
            y += 28.0;
            if !f.note.is_empty() {
                text(&p, pos2(rect.min.x + pad, y + 6.0), Align2::LEFT_CENTER, &f.note, font(12.0), dim(&t), rect.width() - pad * 2.0);
                y += 18.0;
            }
            y += 2.0;
            for (hi, h) in f.hunks.iter().enumerate() {
                let o = opens[g].max(0.0);
                let rows = if st.split { h.split_rows().len() } else { h.lines.len() };
                let box_h = 38.0 + (rows as f32 * LINE_H + 10.0) * o;
                let br = Rect::from_min_size(pos2(rect.min.x + pad, y), vec2(rect.width() - pad * 2.0, box_h));
                let focused = st.focus == g;
                let dc = match h.decision {
                    Some(true) => Some(t.success),
                    Some(false) => Some(t.destructive),
                    None => None,
                };
                let stroke = if let Some(c) = dc {
                    Stroke::new(1.0, with_alpha(c, 0.5))
                } else if focused {
                    Stroke::new(1.5, mix(hair(&t), t.ring, 0.8))
                } else {
                    Stroke::new(1.0, hair(&t))
                };
                p.rect(br, cr(10.0), surface(&t), stroke, StrokeKind::Inside);
                let hr = Rect::from_min_size(br.min, vec2(br.width(), 38.0));
                if let Some(c) = dc {
                    p.rect_filled(hr.shrink(1.0), cr(9.0), tint(&t, c));
                }
                let cy = hr.center().y;
                let lab = text(&p, pos2(hr.min.x + 12.0, cy), Align2::LEFT_CENTER, &format!("Hunk {} of {}", hi + 1, f.hunks.len()), font(12.0), t.foreground, 120.0);
                let at = format!("@@ -{},{} +{},{} @@ {}", h.old_start, h.old_len(), h.new_start, h.new_len(), h.header);
                let actions_w = 190.0;
                text(&p, pos2(lab.max.x + 10.0, cy), Align2::LEFT_CENTER, &at, mono(11.0), dim(&t), hr.max.x - actions_w - lab.max.x - 10.0);
                let hit = ui.interact(hr, id.with(("hh", g)), Sense::click());
                if hit.clicked() {
                    st.focus = g;
                }
                match h.decision {
                    None => {
                        let ab = Rect::from_min_size(pos2(hr.max.x - 10.0 - 84.0, cy - 13.0), vec2(84.0, 26.0));
                        if button(ui, ab, id.with(("acc", g)), "Accept", Some(Icon::Check), Btn::Secondary).clicked() {
                            decide.push((fi, hi, true));
                        }
                        let rb = Rect::from_min_size(pos2(ab.min.x - 88.0, cy - 13.0), vec2(82.0, 26.0));
                        if button(ui, rb, id.with(("rej", g)), "Reject", Some(Icon::X), Btn::Secondary).clicked() {
                            decide.push((fi, hi, false));
                        }
                    }
                    Some(ok) => {
                        let c = if ok { t.success } else { t.destructive };
                        let br2 = badge(&p, &t, pos2(hr.max.x - 64.0, cy), Align2::RIGHT_CENTER, if ok { "Accepted" } else { "Rejected" }, c, true);
                        let _ = br2;
                        let ub = Rect::from_min_size(pos2(hr.max.x - 58.0, cy - 12.0), vec2(50.0, 24.0));
                        if button(ui, ub, id.with(("undo", g)), "Undo", None, Btn::Ghost).clicked() {
                            decide.push((fi, hi, ok));
                        }
                    }
                }
                if o > 0.01 {
                    let body_r = Rect::from_min_max(pos2(br.min.x + 1.0, hr.max.y), pos2(br.max.x - 1.0, br.max.y - 1.0));
                    let pc = faded(&p.with_clip_rect(body_r.intersect(p.clip_rect())), o.min(1.0));
                    hline(&pc, &t, body_r.min.x, body_r.max.x, body_r.min.y);
                    let ly0 = body_r.min.y + 5.0;
                    let num_w = 34.0;
                    let add_bg = with_alpha(t.success, if t.dark { 0.14 } else { 0.09 });
                    let del_bg = with_alpha(t.destructive, if t.dark { 0.14 } else { 0.08 });
                    let fg = t.foreground;
                    if splitk < 0.5 {
                        let a = 1.0 - splitk * 2.0;
                        let pu = faded(&pc, a);
                        let (mut on, mut nn) = (h.old_start, h.new_start);
                        for (k, (kind, s)) in h.lines.iter().enumerate() {
                            let ly = ly0 + k as f32 * LINE_H;
                            let lr = Rect::from_min_size(pos2(body_r.min.x, ly), vec2(body_r.width(), LINE_H));
                            let (bg, sign, sc) = match kind {
                                DiffLineKind::Add => (Some(add_bg), "+", ink(&t, t.success)),
                                DiffLineKind::Del => (Some(del_bg), "-", ink(&t, t.destructive)),
                                DiffLineKind::Context => (None, " ", faint(&t)),
                            };
                            if let Some(bg) = bg {
                                pu.rect_filled(lr, cr(0.0), bg);
                            }
                            let (ol, nl) = match kind {
                                DiffLineKind::Context => {
                                    on += 1;
                                    nn += 1;
                                    (Some(on - 1), Some(nn - 1))
                                }
                                DiffLineKind::Del => {
                                    on += 1;
                                    (Some(on - 1), None)
                                }
                                DiffLineKind::Add => {
                                    nn += 1;
                                    (None, Some(nn - 1))
                                }
                            };
                            if let Some(v) = ol {
                                pu.text(pos2(lr.min.x + num_w - 6.0, lr.center().y), Align2::RIGHT_CENTER, v.to_string(), mono(10.5), faint(&t));
                            }
                            if let Some(v) = nl {
                                pu.text(pos2(lr.min.x + num_w * 2.0 - 6.0, lr.center().y), Align2::RIGHT_CENTER, v.to_string(), mono(10.5), faint(&t));
                            }
                            pu.text(pos2(lr.min.x + num_w * 2.0 + 6.0, lr.center().y), Align2::LEFT_CENTER, sign, mono(11.0), sc);
                            let c = if h.decision == Some(false) && *kind == DiffLineKind::Add { faint(&t) } else { fg };
                            text(&pu, pos2(lr.min.x + num_w * 2.0 + 20.0, lr.center().y), Align2::LEFT_CENTER, s, mono(11.0), c, lr.width() - num_w * 2.0 - 30.0);
                        }
                    } else {
                        let a = (splitk - 0.5) * 2.0;
                        let ps = faded(&pc, a);
                        let half = body_r.width() / 2.0;
                        for (k, (l, r)) in h.split_rows().iter().enumerate() {
                            let ly = ly0 + k as f32 * LINE_H;
                            for (side, cell) in [(0.0, l), (1.0, r)] {
                                let cr_ = Rect::from_min_size(pos2(body_r.min.x + side * half, ly), vec2(half, LINE_H));
                                if let Some((num, s, kind)) = cell {
                                    match kind {
                                        DiffLineKind::Add => {
                                            ps.rect_filled(cr_, cr(0.0), add_bg);
                                        }
                                        DiffLineKind::Del => {
                                            ps.rect_filled(cr_, cr(0.0), del_bg);
                                        }
                                        _ => {}
                                    }
                                    ps.text(pos2(cr_.min.x + num_w - 6.0, cr_.center().y), Align2::RIGHT_CENTER, num.to_string(), mono(10.5), faint(&t));
                                    text(&ps, pos2(cr_.min.x + num_w + 6.0, cr_.center().y), Align2::LEFT_CENTER, s, mono(11.0), fg, half - num_w - 14.0);
                                } else {
                                    ps.rect_filled(cr_, cr(0.0), with_alpha(well(&t), 0.8));
                                }
                            }
                        }
                        ps.line_segment([pos2(body_r.center().x, ly0 - 5.0), pos2(body_r.center().x, body_r.max.y)], Stroke::new(1.0, hair(&t)));
                    }
                }
                y += box_h + 10.0;
                g += 1;
            }
            y += 6.0;
        }
        // footer
        let fy = rect.max.y - footer / 2.0;
        hline(&p, &t, rect.min.x, rect.max.x, rect.max.y - footer);
        let r = text(&p, pos2(rect.min.x + pad, fy), Align2::LEFT_CENTER, &format!("{reviewed} of {total_hunks} reviewed"), font(12.0), t.foreground, 160.0);
        let mut kx = r.max.x + 16.0;
        for (key, what) in [("J", "next"), ("K", "prev"), ("Y", "accept"), ("N", "reject")] {
            if kx > rect.max.x - 330.0 {
                break;
            }
            let kr = Rect::from_min_size(pos2(kx, fy - 10.0), vec2(20.0, 20.0));
            p.rect(kr, cr(5.0), well(&t), Stroke::new(1.0, hair(&t)), StrokeKind::Inside);
            p.text(kr.center(), Align2::CENTER_CENTER, key, mono(10.5), dim(&t));
            let r = text(&p, pos2(kr.max.x + 5.0, fy), Align2::LEFT_CENTER, what, font(11.0), faint(&t), 60.0);
            kx = r.max.x + 12.0;
        }
        let accepted = self.files.iter().flat_map(|f| f.hunks.iter()).filter(|h| h.decision == Some(true)).count();
        let apply_label = format!("Apply {accepted} of {total_hunks}");
        let aw = button_w(&p, &apply_label, false);
        let ab = Rect::from_min_size(pos2(rect.max.x - pad - aw, fy - 14.0), vec2(aw, 28.0));
        let _ = button(ui, ab, id.with("apply"), &apply_label, None, if reviewed == total_hunks { Btn::Primary } else { Btn::Secondary });
        let acc = Rect::from_min_size(pos2(ab.min.x - 90.0, fy - 14.0), vec2(84.0, 28.0));
        let accept_all = button(ui, acc, id.with("accall"), "Accept all", None, Btn::Ghost).clicked();
        let rej = Rect::from_min_size(pos2(acc.min.x - 86.0, fy - 14.0), vec2(80.0, 28.0));
        let reject_all = button(ui, rej, id.with("rejall"), "Reject all", None, Btn::Ghost).clicked();
        // keyboard
        let mut key_dec: Option<bool> = None;
        if resp.hovered() || ui.rect_contains_pointer(rect) {
            ui.input(|i| {
                if i.key_pressed(Key::J) {
                    st.focus = (st.focus + 1).min(total_hunks.saturating_sub(1));
                }
                if i.key_pressed(Key::K) {
                    st.focus = st.focus.saturating_sub(1);
                }
                if i.key_pressed(Key::Y) {
                    key_dec = Some(true);
                }
                if i.key_pressed(Key::N) {
                    key_dec = Some(false);
                }
            });
        }
        // apply decisions
        for (fi, hi, ok) in decide {
            let h = &mut self.files[fi].hunks[hi];
            h.decision = if h.decision.is_some() { None } else { Some(ok) };
        }
        let mut k = 0;
        for f in self.files.iter_mut() {
            for h in f.hunks.iter_mut() {
                if accept_all && h.decision.is_none() {
                    h.decision = Some(true);
                }
                if reject_all && h.decision.is_none() {
                    h.decision = Some(false);
                }
                if let Some(d) = key_dec {
                    if k == st.focus {
                        h.decision = Some(d);
                    }
                }
                k += 1;
            }
        }
        if key_dec.is_some() {
            st.focus = (st.focus + 1).min(total_hunks.saturating_sub(1));
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        resp
    }
}
