use super::common::*;
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// One conversation in a [`ThreadListSidebar`].
#[derive(Clone, Debug, PartialEq)]
pub struct ThreadEntry {
    /// Title (renamable inline).
    pub title: String,
    /// Group heading it sits under when not pinned (e.g. "Today", "Yesterday").
    pub group: String,
    /// Pinned threads sit in their own section at the top.
    pub pinned: bool,
    /// Archived threads hide in a collapsible section at the bottom.
    pub archived: bool,
}

impl ThreadEntry {
    /// A thread in `group`.
    pub fn new(title: impl Into<String>, group: impl Into<String>) -> Self {
        Self { title: title.into(), group: group.into(), pinned: false, archived: false }
    }
    /// Pin it.
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
    /// Archive it.
    pub fn archived(mut self) -> Self {
        self.archived = true;
        self
    }
}

/// Caller-owned state of a [`ThreadListSidebar`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThreadListState {
    /// All threads (newest first).
    pub entries: Vec<ThreadEntry>,
    /// The open thread.
    pub selected: Option<usize>,
    /// Search query.
    pub search: String,
    /// Thread being renamed and the edit buffer.
    pub renaming: Option<(usize, String)>,
    /// Archived section expanded.
    pub show_archived: bool,
}

/// What happened in a [`ThreadListSidebar`] this frame.
#[derive(Clone, Debug, Default)]
pub struct ThreadListOutput {
    /// "New thread" was clicked.
    pub new_thread: bool,
    /// A thread was opened.
    pub opened: Option<usize>,
    /// Screen rects of visible rows by entry index (for tours / autoplay).
    pub rows: Vec<(usize, Rect)>,
}

/// A conversation sidebar: a "New thread" button, a search box that filters as you type (matches
/// highlighted), a Pinned section, date groups, and a collapsible Archived section. Hovering a row
/// reveals pin / rename / archive actions; rename edits the title inline (Enter saves, Esc
/// cancels). The selection highlight glides between rows on a spring and rows reflow smoothly
/// when pinned or archived.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ThreadEntry, ThreadListSidebar, ThreadListState};
/// let mut st = ThreadListState { entries: vec![ThreadEntry::new("Q3 summary", "Today")], ..Default::default() };
/// let out = ThreadListSidebar::new(&mut st).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ThreadListSidebar<'a> {
    state: &'a mut ThreadListState,
    width: f32,
    height: f32,
    title: &'a str,
    salt: Option<egui::IdSalt>,
}

impl<'a> ThreadListSidebar<'a> {
    /// A sidebar over `state`.
    pub fn new(state: &'a mut ThreadListState) -> Self {
        Self { state, width: 260.0, height: 420.0, title: "Threads", salt: None }
    }
    /// Width (default 260).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(180.0);
        self
    }
    /// Height (default 420).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h.max(200.0);
        self
    }
    /// Header title.
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the sidebar.
    pub fn show(self, ui: &mut Ui) -> ThreadListOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_threadlist");
        let st = self.state;
        let mut out = ThreadListOutput::default();
        let (rect, _) = ui.allocate_exact_size(vec2(self.width, self.height), Sense::hover());
        let bg = if t.dark { mix(t.background, t.muted, 0.6) } else { mix(t.card, t.muted, 0.55) };
        ui.painter().rect_filled(rect, cr(16.0), bg);
        ui.painter().rect_stroke(rect, cr(16.0), hairline(&t), StrokeKind::Inside);
        let inner = rect.shrink(10.0);
        // header
        let p = ui.painter();
        bot_avatar(p, &t, pos2(inner.min.x + 11.0, inner.min.y + 13.0), 10.0);
        text_bold(p, pos2(inner.min.x + 28.0, inner.min.y + 13.0), Align2::LEFT_CENTER, self.title, 13.5, t.foreground);
        // new thread
        let nr = Rect::from_min_size(pos2(inner.min.x, inner.min.y + 34.0), vec2(inner.width(), 34.0));
        if button(ui, id.with("new"), nr, "New thread", Btn::Outline, Some(Icon::Plus)).clicked() {
            out.new_thread = true;
        }
        // search
        let sr = Rect::from_min_size(pos2(inner.min.x, nr.max.y + 8.0), vec2(inner.width(), 32.0));
        let sid = id.with("search");
        let sfocus = ui.memory(|m| m.has_focus(sid));
        let sk = toggle(ui, id.with("sk"), sfocus);
        let p = ui.painter();
        p.rect_filled(sr, cr(10.0), mix(bg, surface(&t), 0.5 + 0.5 * sk));
        p.rect_stroke(
            sr,
            cr(10.0),
            Stroke::new(1.0, mix(with_alpha(t.border, 0.0), t.ring, sk * 0.6)),
            StrokeKind::Inside,
        );
        icon(p, Icon::Search, pos2(sr.min.x + 15.0, sr.center().y), 13.0, t.muted_foreground);
        let te_r = Rect::from_min_max(pos2(sr.min.x + 28.0, sr.min.y), pos2(sr.max.x - 8.0, sr.max.y));
        ui.put(
            te_r,
            egui::TextEdit::singleline(&mut st.search)
                .id(sid)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .hint_text(egui::RichText::new("Search threads").color(with_alpha(t.muted_foreground, 0.8)))
                .font(FontId::proportional(13.0))
                .align(Align2::LEFT_CENTER)
                .desired_width(te_r.width()),
        );
        let q = st.search.trim().to_lowercase();
        let list = Rect::from_min_max(pos2(inner.min.x, sr.max.y + 8.0), inner.max);
        // build sections
        let matches = |e: &ThreadEntry| q.is_empty() || e.title.to_lowercase().contains(&q);
        let mut sections: Vec<(String, Vec<usize>)> = Vec::new();
        let pinned: Vec<usize> = (0..st.entries.len())
            .filter(|&i| st.entries[i].pinned && !st.entries[i].archived && matches(&st.entries[i]))
            .collect();
        if !pinned.is_empty() {
            sections.push(("Pinned".into(), pinned));
        }
        for i in 0..st.entries.len() {
            let e = &st.entries[i];
            if e.pinned || e.archived || !matches(e) {
                continue;
            }
            match sections.iter_mut().find(|s| s.0 == e.group) {
                Some(s) => s.1.push(i),
                None => sections.push((e.group.clone(), vec![i])),
            }
        }
        let archived: Vec<usize> =
            (0..st.entries.len()).filter(|&i| st.entries[i].archived && matches(&st.entries[i])).collect();
        let mut action: Option<(usize, u8)> = None;
        let mut toggle_arch = false;
        let mut child =
            ui.new_child(egui::UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(list.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .max_height(list.height())
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let w = ui.available_width();
                let draw_rows = |ui: &mut Ui,
                                 ix: &[usize],
                                 st: &mut ThreadListState,
                                 out: &mut ThreadListOutput,
                                 action: &mut Option<(usize, u8)>| {
                    for &i in ix {
                        let rid = id.with(("row", hash_str(&st.entries[i].title) ^ i as u64));
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
                        out.rows.push((i, r));
                        let e = since_change(
                            ui,
                            rid,
                            (st.entries[i].pinned as u64) | ((st.entries[i].archived as u64) << 1),
                        );
                        keep_alive(ui, e, 0.5);
                        let a = (e * 4.0).clamp(0.0, 1.0);
                        let selected = st.selected == Some(i);
                        let hov = toggle(ui, rid.with("h"), resp.hovered() || resp.contains_pointer());
                        let sel = toggle(ui, rid.with("s"), selected);
                        let p = ui.painter();
                        let fill = mix(with_alpha(surface(&t), 0.0), surface(&t), sel.max(hov * 0.6));
                        p.rect_filled(r, cr(9.0), fill);
                        if sel > 0.01 {
                            p.rect_stroke(
                                r,
                                cr(9.0),
                                Stroke::new(1.0, hairline(&t).color.gamma_multiply(sel)),
                                StrokeKind::Inside,
                            );
                        }
                        let renaming = st.renaming.as_ref().is_some_and(|x| x.0 == i);
                        if renaming {
                            let tr = r.shrink2(vec2(10.0, 4.0));
                            p.rect_stroke(
                                tr.expand2(vec2(4.0, 0.0)),
                                cr(7.0),
                                Stroke::new(1.5, t.ring),
                                StrokeKind::Inside,
                            );
                            let tid = rid.with("te");
                            let buf = &mut st.renaming.as_mut().unwrap().1;
                            let resp = ui.put(
                                tr,
                                egui::TextEdit::singleline(buf)
                                    .id(tid)
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::ZERO)
                                    .font(FontId::proportional(13.0))
                                    .align(Align2::LEFT_CENTER)
                                    .desired_width(tr.width()),
                            );
                            if !resp.has_focus() && !resp.lost_focus() {
                                ui.memory_mut(|m| m.request_focus(tid));
                            }
                            let (enter, esc) =
                                ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
                            if resp.lost_focus() || enter || esc {
                                if let Some((ix, s)) = st.renaming.take() {
                                    if !esc && !s.trim().is_empty() {
                                        st.entries[ix].title = s.trim().to_owned();
                                    }
                                }
                            }
                            continue;
                        }
                        let title = st.entries[i].title.clone();
                        let col = with_alpha(
                            if selected {
                                t.foreground
                            } else {
                                mix(t.muted_foreground, t.foreground, 0.55 + 0.45 * hov)
                            },
                            a,
                        );
                        let clip = Rect::from_min_max(r.min, pos2(r.max.x - 8.0 - 76.0 * hov, r.max.y));
                        let pc = p.with_clip_rect(clip.intersect(ui.clip_rect()));
                        let x0 = r.min.x + 10.0 + (1.0 - a) * 6.0;
                        // highlight the search match
                        if !q.is_empty() {
                            if let Some(pos) = title.to_lowercase().find(&q) {
                                let pre = text_w(ui, &title[..pos], 13.0);
                                let mw = text_w(ui, &title[pos..pos + q.len()], 13.0);
                                pc.rect_filled(
                                    Rect::from_min_size(pos2(x0 + pre - 1.0, r.center().y - 9.0), vec2(mw + 2.0, 18.0)),
                                    cr(4.0),
                                    with_alpha(t.warning, 0.22),
                                );
                            }
                        }
                        pc.text(pos2(x0, r.center().y), Align2::LEFT_CENTER, &title, FontId::proportional(13.0), col);
                        if hov > 0.02 {
                            let icons = [
                                Icon::Bookmark,
                                Icon::Pencil,
                                if st.entries[i].archived { Icon::History } else { Icon::Download },
                            ];
                            for (k, ic) in icons.iter().enumerate() {
                                let br = Rect::from_center_size(
                                    pos2(r.max.x - 14.0 - (2 - k) as f32 * 24.0, r.center().y),
                                    Vec2::splat(22.0),
                                );
                                let b = ui.interact(br, rid.with(("a", k)), Sense::click());
                                let bh = toggle(ui, rid.with(("ah", k)), b.hovered());
                                let p = ui.painter();
                                if bh > 0.01 {
                                    p.rect_filled(br, cr(6.0), with_alpha(well(&t), bh));
                                }
                                let on = k == 0 && st.entries[i].pinned;
                                icon(
                                    p,
                                    *ic,
                                    br.center(),
                                    12.5,
                                    with_alpha(
                                        if on { t.accent } else { mix(t.muted_foreground, t.foreground, bh) },
                                        hov,
                                    ),
                                );
                                if b.clicked() {
                                    *action = Some((i, k as u8));
                                }
                            }
                        }
                        if resp.clicked() {
                            st.selected = Some(i);
                            out.opened = Some(i);
                        }
                        if resp.double_clicked() {
                            *action = Some((i, 1));
                        }
                    }
                };
                for (name, ix) in &sections {
                    ui.add_space(6.0);
                    let (hr, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                    let p = ui.painter();
                    if name == "Pinned" {
                        icon(p, Icon::Bookmark, pos2(hr.min.x + 15.0, hr.center().y), 10.0, t.muted_foreground);
                        text(
                            p,
                            pos2(hr.min.x + 24.0, hr.center().y),
                            Align2::LEFT_CENTER,
                            name,
                            11.0,
                            t.muted_foreground,
                        );
                    } else {
                        text(
                            p,
                            pos2(hr.min.x + 10.0, hr.center().y),
                            Align2::LEFT_CENTER,
                            name,
                            11.0,
                            t.muted_foreground,
                        );
                    }
                    draw_rows(ui, ix, st, &mut out, &mut action);
                }
                if sections.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
                    text(ui.painter(), r.center(), Align2::CENTER_CENTER, "No threads match", 12.5, t.muted_foreground);
                }
                if !archived.is_empty() {
                    ui.add_space(8.0);
                    let (hr, hresp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
                    let ak = toggle(ui, id.with("arch_k"), st.show_archived);
                    let hk = toggle(ui, id.with("arch_h"), hresp.hovered());
                    let p = ui.painter();
                    p.rect_filled(hr, cr(8.0), with_alpha(surface(&t), hk * 0.6));
                    let c = pos2(hr.min.x + 15.0, hr.center().y);
                    let ang = ak * std::f32::consts::FRAC_PI_2;
                    let d = vec2(ang.cos(), ang.sin());
                    let n = vec2(-d.y, d.x);
                    p.line_segment([c - d * 2.0 - n * 3.5, c + d * 2.0], Stroke::new(1.4, t.muted_foreground));
                    p.line_segment([c - d * 2.0 + n * 3.5, c + d * 2.0], Stroke::new(1.4, t.muted_foreground));
                    text(
                        p,
                        pos2(hr.min.x + 26.0, hr.center().y),
                        Align2::LEFT_CENTER,
                        &format!("Archived  ·  {}", archived.len()),
                        11.5,
                        t.muted_foreground,
                    );
                    if hresp.clicked() {
                        toggle_arch = true;
                    }
                    if ak > 0.02 {
                        ui.multiply_opacity(ak);
                        draw_rows(ui, &archived, st, &mut out, &mut action);
                    }
                }
            });
        if toggle_arch {
            st.show_archived = !st.show_archived;
        }
        if let Some((i, k)) = action {
            match k {
                0 => st.entries[i].pinned = !st.entries[i].pinned,
                1 => st.renaming = Some((i, st.entries[i].title.clone())),
                _ => {
                    st.entries[i].archived = !st.entries[i].archived;
                    st.entries[i].pinned = false;
                }
            }
        }
        out
    }
}
