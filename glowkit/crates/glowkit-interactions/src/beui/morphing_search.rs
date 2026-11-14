use super::common::{cr, hover_fill, icon, idle_for, morph_axes, progress, shadow_color, surface, text_width, BeIcon};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
use glowkit_core::anim::{spring, stagger};
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

#[derive(Clone, Debug)]
struct SearchState {
    recents: Vec<String>,
    open: bool,
    since: f64,
    auto_open: bool,
}

/// A frosted search bar that morphs upward into a panel of recent searches when focused. The bar
/// itself is the panel: its top edge rises on a spring while the corners relax from a pill into
/// a card, the list rows stagger in, and a highlight glides under the pointer. Picking a row
/// fills the query; Enter turns the query into a removable tag chip.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::MorphingSearch;
/// let mut query = String::new();
/// let mut tags = vec!["onboarding".to_string()];
/// MorphingSearch::new(&mut query).tags(&mut tags).recent(["dashboards", "empty states"]).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct MorphingSearch<'a> {
    query: &'a mut String,
    tags: Option<&'a mut Vec<String>>,
    recent: Vec<String>,
    width: f32,
    hint: String,
    autoplay: bool,
}

impl<'a> MorphingSearch<'a> {
    /// A search bar editing `query`.
    pub fn new(query: &'a mut String) -> Self {
        Self { query, tags: None, recent: Vec::new(), width: 420.0, hint: "Search inspiration".into(), autoplay: true }
    }
    /// Tag chips shown before the input (Enter adds one, the cross removes one).
    pub fn tags(mut self, tags: &'a mut Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
    /// Initial recent searches.
    pub fn recent(mut self, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.recent = items.into_iter().map(Into::into).collect();
        self
    }
    /// Width of the bar.
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(220.0);
        self
    }
    /// Placeholder text.
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = h.into();
        self
    }
    /// Open and close on its own while idle (default on).
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }

    /// Show the bar; the panel grows upward over whatever was painted above it.
    pub fn show(mut self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui);
        let bar_h = 50.0;
        let row_h = 38.0;
        let (bar, resp) = ui.allocate_exact_size(vec2(self.width, bar_h), Sense::click());
        let id = resp.id;
        let now = ui.input(|i| i.time);
        let recent0 = self.recent.clone();
        let st0: SearchState =
            glowkit_core::state(ui, id, || SearchState { recents: recent0, open: false, since: now, auto_open: false });
        let q = self.query.to_lowercase();
        let rows: Vec<String> =
            st0.recents.iter().filter(|r| q.is_empty() || r.to_lowercase().contains(&q)).cloned().collect();
        let nrows = rows.len().max(1);
        let panel_h = bar_h + 34.0 + nrows as f32 * row_h + 6.0;
        let open_rect = Rect::from_min_max(pos2(bar.min.x, bar.max.y - panel_h), bar.max);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let over = pointer.is_some_and(|p| (if st0.open { open_rect } else { bar }).contains(p));
        let idle = idle_for(ui, id, over || st0.open && !st0.auto_open);
        let auto = self.autoplay && idle > 2.5;

        // chips
        let painter = ui.painter().clone();
        let mut chip_x = bar.min.x + 44.0;
        let mut remove_tag = None;
        let mut chip_rects = Vec::new();
        if let Some(tags) = &self.tags {
            for (i, tag) in tags.iter().enumerate() {
                let w = text_width(&painter, tag, 13.0) + 38.0;
                let r = Rect::from_min_size(pos2(chip_x, bar.center().y - 15.0), vec2(w, 30.0));
                let x = Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(18.0, 18.0));
                if ui.interact(x, id.with(("chipx", i)), Sense::click()).clicked() {
                    remove_tag = Some(i);
                }
                chip_rects.push((r, tag.clone()));
                chip_x += w + 6.0;
            }
        }
        // text input
        let input_rect =
            Rect::from_min_max(pos2(chip_x + 6.0, bar.min.y + 8.0), pos2(bar.max.x - 16.0, bar.max.y - 8.0));
        let te_id = id.with("te");
        let focused = ui.memory(|m| m.has_focus(te_id));
        // rows
        let rows_top = open_rect.min.y + 34.0;
        let row_at = |p: egui::Pos2| -> Option<usize> {
            if !open_rect.contains(p) || p.y >= bar.min.y {
                return None;
            }
            let i = ((p.y - rows_top) / row_h).floor();
            (i >= 0.0 && (i as usize) < rows.len()).then_some(i as usize)
        };
        let clear_r = Rect::from_center_size(pos2(open_rect.max.x - 30.0, open_rect.min.y + 18.0), vec2(48.0, 20.0));
        let mut picked: Option<String> = None;
        let mut removed: Option<String> = None;
        let mut clear = false;
        let click = ui.input(|i| if i.pointer.primary_clicked() { i.pointer.interact_pos() } else { None });
        if st0.open {
            if let Some(p) = click {
                if clear_r.contains(p) {
                    clear = true;
                } else if let Some(i) = row_at(p) {
                    if p.x > open_rect.max.x - 40.0 {
                        removed = Some(rows[i].clone());
                    } else {
                        picked = Some(rows[i].clone());
                    }
                }
            }
        }
        if let Some(p) = &picked {
            *self.query = p.clone();
        }
        if let (Some(i), Some(tags)) = (remove_tag, self.tags.as_mut()) {
            tags.remove(i);
        }
        let outside = click.is_some_and(|p| !open_rect.contains(p) && !bar.contains(p));
        let st = glowkit_core::with_state(
            ui,
            id,
            || st0.clone(),
            |s| {
                if let Some(r) = &removed {
                    s.recents.retain(|x| x != r);
                }
                if clear {
                    s.recents.clear();
                }
                let want_real = focused;
                if want_real && !s.open {
                    s.open = true;
                    s.since = now;
                    s.auto_open = false;
                } else if !want_real && s.open && !s.auto_open && (outside || picked.is_some() || !over) {
                    s.open = false;
                    s.since = now;
                }
                if auto && !focused {
                    let held = now - s.since;
                    if s.open && held > 2.8 {
                        s.open = false;
                        s.auto_open = false;
                        s.since = now;
                    } else if !s.open && held > 1.6 {
                        s.open = true;
                        s.auto_open = true;
                        s.since = now;
                    }
                }
                s.clone()
            },
        );
        if auto {
            glowkit_core::animate(ui);
        }
        let open = st.open;
        let (rect, radius) = morph_axes(
            ui,
            id.with("box"),
            if open { open_rect } else { bar },
            if open { 22.0 } else { bar_h / 2.0 },
            (380.0, 30.0),
            (260.0, 20.0),
        );
        let k = progress(rect.height(), bar_h, panel_h);
        // paint (above earlier content)
        let fg_painter = painter.clone();
        let glass = with_alpha(surface(&t), if t.dark { 0.82 } else { 0.86 });
        paint::soft_shadow(&fg_painter, rect, radius, 22.0, vec2(0.0, 8.0), shadow_color(&t, 0.55 + 0.4 * k.min(1.0)));
        fg_painter.rect(
            rect,
            cr(radius),
            glass,
            Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.1) } else { with_alpha(Color32::WHITE, 0.9) }),
            egui::StrokeKind::Inside,
        );
        // the input line divider rises with the panel
        let clip = fg_painter.with_clip_rect(rect.shrink(1.0));
        let ka = ((k - 0.3) / 0.7).clamp(0.0, 1.0);
        if ka > 0.0 {
            clip.text(
                pos2(rect.min.x + 18.0, rect.min.y + 18.0),
                Align2::LEFT_CENTER,
                if q.is_empty() { "Recent" } else { "Matches" },
                FontId::proportional(11.5),
                with_alpha(t.muted_foreground, ka),
            );
            let hov_clear = pointer.is_some_and(|p| clear_r.contains(p));
            clip.text(
                pos2(rect.max.x - 18.0, rect.min.y + 18.0),
                Align2::RIGHT_CENTER,
                "Clear",
                FontId::proportional(11.5),
                with_alpha(if hov_clear { t.foreground } else { t.muted_foreground }, ka),
            );
            let since = (now - st.since) as f32;
            let auto_row = if st.auto_open { Some(((since * 1.2) as usize) % rows.len().max(1)) } else { None };
            let hov = pointer.and_then(row_at).or(auto_row);
            let hy = spring(ui, id.with("hy"), hov.unwrap_or(0) as f32, 420.0, 32.0);
            let ha = spring(ui, id.with("ha"), if hov.is_some() && open { 1.0 } else { 0.0 }, 300.0, 26.0);
            let top = rect.min.y + 34.0;
            if ha > 0.01 && !rows.is_empty() {
                let hr =
                    Rect::from_min_size(pos2(rect.min.x + 6.0, top + hy * row_h), vec2(rect.width() - 12.0, row_h));
                clip.rect_filled(hr, cr(12.0), with_alpha(hover_fill(&t), ha * ka));
            }
            if rows.is_empty() {
                clip.text(
                    pos2(rect.min.x + 18.0, top + row_h / 2.0),
                    Align2::LEFT_CENTER,
                    "Nothing yet",
                    FontId::proportional(13.0),
                    with_alpha(t.muted_foreground, ka),
                );
            }
            for (i, r) in rows.iter().enumerate() {
                let e = if open { stagger(since - 0.08, i, 0.04, 0.3) } else { 1.0 };
                let a = ka * e;
                let y = top + row_h * (i as f32 + 0.5) + (1.0 - e) * 10.0;
                let col = if hov == Some(i) { t.foreground } else { mix(t.foreground, t.muted_foreground, 0.2) };
                icon(&clip, BeIcon::Clock, pos2(rect.min.x + 26.0, y), 13.0, with_alpha(t.muted_foreground, a), 1.2);
                clip.text(
                    pos2(rect.min.x + 46.0, y),
                    Align2::LEFT_CENTER,
                    r,
                    FontId::proportional(13.5),
                    with_alpha(col, a),
                );
                icon(
                    &clip,
                    BeIcon::Close,
                    pos2(rect.max.x - 24.0, y),
                    9.0,
                    with_alpha(t.muted_foreground, a * 0.8),
                    1.2,
                );
            }
            if open {
                if let Some(p) = pointer {
                    if row_at(p).is_some() || clear_r.contains(p) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }
            }
        }
        // bar contents
        let by = bar.center().y;
        if k > 0.15 {
            let y = bar.min.y;
            clip.line_segment(
                [pos2(rect.min.x + 14.0, y), pos2(rect.max.x - 14.0, y)],
                Stroke::new(1.0, with_alpha(t.border, ((k - 0.15) * 2.0).clamp(0.0, 1.0) * 0.8)),
            );
        }
        icon(&fg_painter, BeIcon::Search, pos2(bar.min.x + 24.0, by), 15.0, t.muted_foreground, 1.5);
        for (r, tag) in &chip_rects {
            fg_painter.rect_filled(*r, cr(15.0), hover_fill(&t));
            fg_painter.text(
                pos2(r.min.x + 12.0, r.center().y),
                Align2::LEFT_CENTER,
                tag,
                FontId::proportional(13.0),
                t.foreground,
            );
            icon(&fg_painter, BeIcon::Close, pos2(r.max.x - 14.0, r.center().y), 8.0, t.muted_foreground, 1.2);
        }
        let ghost_n = if st.auto_open && self.query.is_empty() && !focused {
            let since = (now - st.since) as f32;
            (((since - 0.4) / 0.07).max(0.0) as usize).min(13)
        } else {
            0
        };
        let hint = if ghost_n > 0 { String::new() } else { self.hint.clone() };
        let te = ui.put(
            input_rect,
            egui::TextEdit::singleline(self.query)
                .id(te_id)
                .frame(egui::Frame::NONE)
                .font(FontId::proportional(14.5))
                .text_color(t.foreground)
                .hint_text(egui::RichText::new(hint).color(t.muted_foreground)),
        );
        if resp.clicked() {
            te.request_focus();
        }
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && !self.query.trim().is_empty() {
            let qq = self.query.trim().to_string();
            if let Some(tags) = self.tags.as_mut() {
                if !tags.contains(&qq) {
                    tags.push(qq.clone());
                }
            }
            self.query.clear();
            glowkit_core::with_state(
                ui,
                id,
                || st0.clone(),
                |s| {
                    s.recents.retain(|r| r != &qq);
                    s.recents.insert(0, qq.clone());
                    s.recents.truncate(5);
                },
            );
        }
        if st.auto_open && self.query.is_empty() && !focused {
            // ghost typing while the panel demonstrates itself
            let since = (now - st.since) as f32;
            let word = "pricing pages";
            let n = ghost_n.min(word.len());
            if n > 0 {
                let r = fg_painter.text(
                    pos2(input_rect.min.x + 2.0, by),
                    Align2::LEFT_CENTER,
                    &word[..n],
                    FontId::proportional(14.5),
                    t.foreground,
                );
                if (since * 2.0) as i32 % 2 == 0 {
                    fg_painter.line_segment(
                        [pos2(r.max.x + 1.0, by - 8.0), pos2(r.max.x + 1.0, by + 8.0)],
                        Stroke::new(1.2, t.accent),
                    );
                }
            }
        }
        resp
    }
}
