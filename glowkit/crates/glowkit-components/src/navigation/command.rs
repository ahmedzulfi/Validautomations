use crate::overlays::kit;
use crate::overlays::LineIcon;
use egui::{pos2, vec2, Align2, CursorIcon, FontId, Key, LayerId, Modifiers, Order, Rect, Sense, Stroke, TextEdit, Ui, UiBuilder};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// One result row in a [`CommandPalette`].
#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    /// Text (also what the search matches against).
    pub label: String,
    /// Leading icon.
    pub icon: Option<LineIcon>,
    /// Right-aligned shortcut hint.
    pub shortcut: Option<String>,
    /// Extra search keywords.
    pub keywords: String,
}

impl CommandItem {
    /// A command with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), icon: None, shortcut: None, keywords: String::new() }
    }
    /// Leading icon.
    pub fn icon(mut self, icon: LineIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Shortcut hint.
    pub fn shortcut(mut self, s: impl Into<String>) -> Self {
        self.shortcut = Some(s.into());
        self
    }
    /// Extra words that should match this command.
    pub fn keywords(mut self, k: impl Into<String>) -> Self {
        self.keywords = k.into();
        self
    }
}

/// Case-insensitive subsequence match; returns a score (higher is better) or `None`.
fn fuzzy(query: &str, text: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let q: Vec<char> = query.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let tl = text.to_lowercase();
    if tl.contains(&query.to_lowercase()) {
        return Some(1000 - tl.find(&query.to_lowercase()).unwrap_or(0) as i32);
    }
    let mut qi = 0;
    let mut score = 0;
    let mut last = None;
    for (i, c) in t.iter().enumerate() {
        if qi < q.len() && *c == q[qi] {
            score += if last == Some(i.wrapping_sub(1)) { 8 } else { 1 };
            last = Some(i);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(score)
}

/// A ⌘K / Ctrl+K command palette: a modal search box with grouped, fuzzy-filtered results,
/// keyboard navigation (↑ ↓ Enter Esc) and a highlight that glides between rows.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// # let mut open = false;
/// use glowkit_components::{CommandItem, CommandPalette, LineIcon};
/// CommandPalette::toggle_on_shortcut(ui.ctx(), &mut open);
/// if let Some(cmd) = CommandPalette::new(&mut open)
///     .group("Suggestions", vec![CommandItem::new("Calendar").icon(LineIcon::Calendar)])
///     .group("Settings", vec![CommandItem::new("Profile").icon(LineIcon::User).shortcut("Ctrl+P")])
///     .show(ui)
/// {
///     println!("run {cmd}");
/// }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct CommandPalette<'a> {
    open: &'a mut bool,
    groups: Vec<(String, Vec<CommandItem>)>,
    placeholder: String,
    within: Option<Rect>,
    width: f32,
    max_height: f32,
}

impl<'a> CommandPalette<'a> {
    /// A palette controlled by `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self { open, groups: Vec::new(), placeholder: "Type a command or search…".into(), within: None, width: 560.0, max_height: 340.0 }
    }
    /// Add a titled group of commands.
    pub fn group(mut self, title: impl Into<String>, items: Vec<CommandItem>) -> Self {
        self.groups.push((title.into(), items));
        self
    }
    /// Search box placeholder.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }
    /// Confine to `rect` instead of the window.
    pub fn within(mut self, rect: Rect) -> Self {
        self.within = Some(rect);
        self
    }
    /// Panel width (default 560).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Maximum height of the result list (default 340).
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = h;
        self
    }
    /// Flip `open` when Ctrl+K / ⌘K is pressed. Call once per frame.
    pub fn toggle_on_shortcut(ctx: &egui::Context, open: &mut bool) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            *open = !*open;
        }
    }

    /// Show the palette. Returns the label of the chosen command (the palette then closes).
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        let t = Theme::of(ui);
        let ctx = ui.ctx().clone();
        let id = ui.scope_id().with("glowkit_command");
        let was_open: bool = glowkit_core::state(ui, id.with("was_open"), || false);
        let just_opened = *self.open && !was_open;
        glowkit_core::set_state(ui, id.with("was_open"), *self.open);
        let mut query: String = glowkit_core::state(ui, id.with("q"), String::new);
        let mut sel: usize = glowkit_core::state(ui, id.with("sel"), || 0);
        if just_opened {
            query.clear();
            sel = 0;
        }
        let p = kit::presence(ui, id, *self.open, 16.0);
        let pop = kit::pop_spring(ui, id, *self.open);
        if p < 0.01 && !*self.open {
            return None;
        }
        // filter
        let mut flat: Vec<(usize, &CommandItem)> = Vec::new();
        let mut groups: Vec<(&str, Vec<(usize, &CommandItem)>)> = Vec::new();
        for (gi, (title, items)) in self.groups.iter().enumerate() {
            let mut matched: Vec<(i32, &CommandItem)> = items
                .iter()
                .filter_map(|it| fuzzy(&query, &format!("{} {}", it.label, it.keywords)).map(|s| (s, it)))
                .collect();
            if !query.is_empty() {
                matched.sort_by_key(|m| std::cmp::Reverse(m.0));
            }
            if !matched.is_empty() {
                let mut g = Vec::new();
                for (_, it) in matched {
                    g.push((flat.len(), it));
                    flat.push((gi, it));
                }
                groups.push((title.as_str(), g));
            }
        }
        let n = flat.len();
        let mut chosen: Option<String> = None;
        if *self.open {
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) && n > 0 {
                    sel = (sel + 1) % n;
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) && n > 0 {
                    sel = (sel + n - 1) % n;
                }
                if i.consume_key(Modifiers::NONE, Key::Enter) && n > 0 {
                    chosen = Some(flat[sel.min(n - 1)].1.label.clone());
                }
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    *self.open = false;
                }
            });
        }
        if n > 0 {
            sel = sel.min(n - 1);
        }

        let area = kit::screen(&ctx, self.within);
        let clip = if self.within.is_some() { area.intersect(ui.clip_rect()) } else { area };
        // backdrop
        let bid = id.with("backdrop");
        let bclick = kit::float_layer(&ctx, bid, Order::Foreground, area, clip, 1.0, area.center(), p, |ui| {
            let r = ui.interact(area, bid.with("hit"), Sense::click());
            ui.painter().rect_filled(area, if self.within.is_some() { t.radius } else { 0.0 }, kit::backdrop_color(&t).gamma_multiply(0.8));
            r.clicked()
        });
        ctx.move_to_top(LayerId::new(Order::Foreground, bid));
        if bclick.inner {
            *self.open = false;
        }

        // layout heights
        let row_h = 38.0;
        let head_h = 28.0;
        let list_h: f32 = if n == 0 { 64.0 } else { groups.iter().map(|(_, g)| head_h + g.len() as f32 * row_h + 6.0).sum::<f32>() + 6.0 };
        let list_h = list_h.min(self.max_height);
        let input_h = 50.0;
        let footer_h = 36.0;
        let w = self.width.min(area.width() - 32.0);
        let h = input_h + list_h + footer_h;
        let rect = Rect::from_min_size(pos2(area.center().x - w / 2.0, area.min.y + (area.height() * 0.14).max(16.0) + (1.0 - p) * -8.0), vec2(w, h));
        let lid = id.with("panel");
        let placeholder = self.placeholder.clone();
        let res = kit::float_layer(&ctx, lid, Order::Foreground, rect.expand(16.0), clip, 0.96 + 0.04 * pop, rect.center(), p, |ui| {
            ui.interact(rect, lid.with("block"), Sense::click());
            let pnt = ui.painter().clone();
            kit::panel(&pnt, rect, &t, t.radius * 1.1);
            // search row
            let sr = Rect::from_min_size(rect.min, vec2(w, input_h));
            LineIcon::Search.paint(&pnt, Rect::from_center_size(pos2(sr.min.x + 22.0, sr.center().y), vec2(17.0, 17.0)), t.muted_foreground);
            pnt.line_segment([sr.left_bottom(), sr.right_bottom()], Stroke::new(1.0, t.border));
            let er = Rect::from_min_max(pos2(sr.min.x + 40.0, sr.center().y - 15.0), pos2(sr.max.x - 60.0, sr.center().y + 15.0));
            let mut child = ui.new_child(UiBuilder::new().max_rect(er));
            let te = child.add(
                TextEdit::singleline(&mut query)
                    .id(lid.with("input"))
                    .hint_text(placeholder.as_str())
                    .frame(egui::Frame::NONE)
                    .font(FontId::proportional(15.0))
                    .desired_width(er.width())
                    .margin(vec2(0.0, 6.0)),
            );
            if just_opened || (te.lost_focus() && !ui.input(|i| i.key_pressed(Key::Escape))) {
                te.request_focus();
            }
            if te.changed() {
                sel = 0;
            }
            kit::kbd(&pnt, pos2(sr.max.x - 16.0, sr.center().y), "Esc", &t);
            // list
            let lr = Rect::from_min_size(pos2(rect.min.x, sr.max.y), vec2(w, list_h));
            let lp = pnt.with_clip_rect(lr.intersect(clip));
            let mut picked = None;
            if n == 0 {
                lp.text(lr.center(), Align2::CENTER_CENTER, "No results found.", FontId::proportional(14.0), t.muted_foreground);
            } else {
                // keep the selection in view
                let mut sel_y = 0.0;
                {
                    let mut yy = 6.0;
                    for (_, g) in &groups {
                        yy += head_h;
                        for (fi, _) in g {
                            if *fi == sel {
                                sel_y = yy;
                            }
                            yy += row_h;
                        }
                        yy += 6.0;
                    }
                }
                let mut scroll: f32 = glowkit_core::state(ui, lid.with("scroll"), || 0.0);
                if sel_y - scroll > list_h - row_h - 6.0 {
                    scroll = sel_y - list_h + row_h + 6.0;
                }
                if sel_y - scroll < head_h {
                    scroll = (sel_y - head_h).max(0.0);
                }
                let scroll_s = kit::follow_ctx(&ctx, lid.with("scroll_s"), scroll, 20.0);
                glowkit_core::set_state(ui, lid.with("scroll"), scroll);
                let mut y = lr.min.y + 6.0 - scroll_s;
                let hl_target = lr.min.y + sel_y - scroll_s;
                let hl = kit::follow_ctx(&ctx, lid.with("hl"), hl_target, 28.0);
                let hr = Rect::from_min_size(pos2(lr.min.x + 6.0, hl), vec2(w - 12.0, row_h));
                lp.rect_filled(hr, t.radius * 0.6, kit::row_highlight(&t));
                for (title, g) in &groups {
                    lp.text(pos2(lr.min.x + 16.0, y + head_h / 2.0 + 2.0), Align2::LEFT_CENTER, *title, FontId::proportional(12.0), t.muted_foreground);
                    y += head_h;
                    for (fi, it) in g {
                        let rr = Rect::from_min_size(pos2(lr.min.x + 6.0, y), vec2(w - 12.0, row_h));
                        if lr.intersects(rr) {
                            let resp = ui.interact(rr.intersect(lr), lid.with(("row", *fi)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                            if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                sel = *fi;
                            }
                            if resp.clicked() {
                                picked = Some(it.label.clone());
                            }
                            let on = *fi == sel;
                            let col = if on { t.foreground } else { t.foreground.gamma_multiply(0.92) };
                            let mut tx = rr.min.x + 12.0;
                            if let Some(ic) = it.icon {
                                ic.paint(&lp, Rect::from_center_size(pos2(tx + 8.0, rr.center().y), vec2(16.0, 16.0)), if on { t.foreground } else { t.muted_foreground });
                                tx += 28.0;
                            }
                            lp.text(pos2(tx, rr.center().y), Align2::LEFT_CENTER, &it.label, FontId::proportional(14.0), col);
                            if let Some(s) = &it.shortcut {
                                kit::shortcut_text(&lp, pos2(rr.max.x - 12.0, rr.center().y), s, &t);
                            }
                        }
                        y += row_h;
                    }
                    y += 6.0;
                }
            }
            // footer
            let fr = Rect::from_min_max(pos2(rect.min.x, rect.max.y - footer_h), rect.max);
            pnt.line_segment([fr.left_top(), fr.right_top()], Stroke::new(1.0, t.border));
            pnt.rect_filled(fr.shrink(1.0), egui::CornerRadius { nw: 0, ne: 0, sw: (t.radius * 1.1) as u8, se: (t.radius * 1.1) as u8 }, with_alpha(t.muted, 0.5));
            let mut fx = fr.min.x + 14.0;
            for (k, label) in [("↑↓", "to navigate"), ("↵", "to select"), ("esc", "to close")] {
                let kw = pnt.layout_no_wrap(k.to_owned(), FontId::proportional(11.0), t.muted_foreground).size().x + 10.0;
                let kr2 = kit::kbd(&pnt, pos2(fx + kw, fr.center().y), k, &t);
                let tr = pnt.text(pos2(kr2.max.x + 6.0, fr.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.0), t.muted_foreground);
                fx = tr.max.x + 16.0;
            }
            picked
        });
        ctx.move_to_top(LayerId::new(Order::Foreground, lid));
        if let Some(c) = res.inner {
            chosen = Some(c);
        }
        if chosen.is_some() {
            *self.open = false;
        }
        glowkit_core::set_state(ui, id.with("q"), query);
        glowkit_core::set_state(ui, id.with("sel"), sel);
        chosen
    }
}
