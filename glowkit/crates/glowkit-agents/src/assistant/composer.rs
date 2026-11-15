use super::common::*;
use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};
use std::sync::Arc;

/// Someone (or something) that can be @mentioned in a [`SmartComposer`].
#[derive(Clone, Debug, PartialEq)]
pub struct MentionItem {
    /// Name inserted after the `@`.
    pub name: String,
    /// Small right-aligned role (e.g. "agent", "teammate", "file").
    pub role: String,
    /// Avatar tint.
    pub color: Color32,
}

impl MentionItem {
    /// A mention target.
    pub fn new(name: impl Into<String>, role: impl Into<String>, color: Color32) -> Self {
        Self { name: name.into(), role: role.into(), color }
    }
}

/// A `/command` offered by a [`SmartComposer`].
#[derive(Clone, Debug, PartialEq)]
pub struct SlashCommand {
    /// Command name without the slash.
    pub name: String,
    /// What it does.
    pub description: String,
    /// Icon shown before it.
    pub icon: Icon,
}

impl SlashCommand {
    /// A command.
    pub fn new(name: impl Into<String>, description: impl Into<String>, icon: Icon) -> Self {
        Self { name: name.into(), description: description.into(), icon }
    }
}

/// A file staged in the composer (uploading or ready).
#[derive(Clone, Debug, PartialEq)]
pub struct StagedFile {
    /// File name.
    pub name: String,
    /// Human size, e.g. "128 KB".
    pub size: String,
    /// Upload progress 0..1 (1 = ready). Advance it yourself, or use [`StagedFile::simulated`].
    pub progress: f32,
    simulate: bool,
}

impl StagedFile {
    /// A ready file.
    pub fn new(name: impl Into<String>, size: impl Into<String>) -> Self {
        Self { name: name.into(), size: size.into(), progress: 1.0, simulate: false }
    }
    /// A file whose upload progress the composer animates by itself (for demos and previews).
    pub fn simulated(name: impl Into<String>, size: impl Into<String>) -> Self {
        Self { name: name.into(), size: size.into(), progress: 0.0, simulate: true }
    }
}

/// A piece of context attached to the next message (a selection, an open tab, a doc).
#[derive(Clone, Debug, PartialEq)]
pub struct ContextChip {
    /// Label shown in the chip.
    pub label: String,
    /// Icon before the label.
    pub icon: Icon,
}

impl ContextChip {
    /// A context chip.
    pub fn new(label: impl Into<String>, icon: Icon) -> Self {
        Self { label: label.into(), icon }
    }
}

/// Caller-owned state of a [`SmartComposer`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComposerState {
    /// The draft text.
    pub text: String,
    /// Staged attachments.
    pub files: Vec<StagedFile>,
    /// Context chips attached to the draft.
    pub chips: Vec<ContextChip>,
}

/// What happened in a [`SmartComposer`] this frame.
#[derive(Debug)]
pub struct ComposerOutput {
    /// The submitted text (the draft is cleared).
    pub submitted: Option<String>,
    /// The attach (+) button was clicked.
    pub attach_clicked: bool,
    /// A suggestion menu (mentions or commands) is open.
    pub menu_open: bool,
    /// Screen rect of the composer box.
    pub rect: Rect,
    /// Screen rect of the send button.
    pub send_rect: Rect,
    /// Screen rect of the attach button.
    pub attach_rect: Rect,
}

#[derive(Clone, Copy, Default)]
struct MenuState {
    hi: usize,
    last_query_hash: u64,
}

enum Menu {
    Mentions(String),
    Commands(String),
}

/// The full agent composer: an auto-growing rounded input with `@` mentions and `/` commands that
/// open a floating, filterable menu above the box (arrow keys + Enter / Tab, or click), mention and
/// command tokens highlighted inline, a tray of staged files with upload rings, context chips, a
/// drag-and-drop overlay ("Drop to attach") when files hover the window, and a toolbar with attach,
/// a model label and a send button that wakes up when there is something to send.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ComposerState, MentionItem, SlashCommand, SmartComposer};
/// use glowkit_ai::Icon;
/// let mut state = ComposerState::default();
/// let people = [MentionItem::new("Mara", "teammate", egui::Color32::LIGHT_BLUE)];
/// let cmds = [SlashCommand::new("review", "Review the current diff", Icon::Eye)];
/// let out = SmartComposer::new(&mut state).mentions(&people).commands(&cmds).show(ui);
/// if let Some(msg) = out.submitted { /* send */ }
/// ```
#[must_use = "call `.show(ui)`"]
pub struct SmartComposer<'a> {
    state: &'a mut ComposerState,
    mentions: &'a [MentionItem],
    commands: &'a [SlashCommand],
    placeholder: &'a str,
    model_label: Option<&'a str>,
    width: Option<f32>,
    drag_preview: bool,
    focus_ring: bool,
    salt: Option<egui::IdSalt>,
}

impl<'a> SmartComposer<'a> {
    /// A composer editing `state`.
    pub fn new(state: &'a mut ComposerState) -> Self {
        Self {
            state,
            mentions: &[],
            commands: &[],
            placeholder: "Ask anything…  @ to mention, / for commands",
            model_label: None,
            width: None,
            drag_preview: false,
            focus_ring: false,
            salt: None,
        }
    }
    /// Targets offered after `@`.
    pub fn mentions(mut self, m: &'a [MentionItem]) -> Self {
        self.mentions = m;
        self
    }
    /// Commands offered after a leading `/`.
    pub fn commands(mut self, c: &'a [SlashCommand]) -> Self {
        self.commands = c;
        self
    }
    /// Placeholder text.
    pub fn placeholder(mut self, p: &'a str) -> Self {
        self.placeholder = p;
        self
    }
    /// Model name shown in the toolbar.
    pub fn model_label(mut self, m: &'a str) -> Self {
        self.model_label = Some(m);
        self
    }
    /// Width (default: available, at most 640).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Show the drop overlay as if files were being dragged over (for previews).
    pub fn drag_preview(mut self, on: bool) -> Self {
        self.drag_preview = on;
        self
    }
    /// Draw the focus ring even when unfocused (for previews).
    pub fn focus_ring(mut self, on: bool) -> Self {
        self.focus_ring = on;
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the composer.
    pub fn show(self, ui: &mut Ui) -> ComposerOutput {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_composer");
        let te_id = id.with("te");
        let st = self.state;
        let dt = glowkit_core::dt(ui);
        let time = glowkit_core::time(ui);
        let w = self.width.unwrap_or_else(|| ui.available_width().min(640.0));
        let mut out = ComposerOutput {
            submitted: None,
            attach_clicked: false,
            menu_open: false,
            rect: Rect::NOTHING,
            send_rect: Rect::NOTHING,
            attach_rect: Rect::NOTHING,
        };

        // simulated uploads
        let mut uploading = false;
        for f in st.files.iter_mut() {
            if f.simulate && f.progress < 1.0 {
                f.progress = (f.progress + dt * 0.55).min(1.0);
                uploading = true;
            }
        }
        if uploading {
            ui.ctx().request_repaint();
        }
        // dropped files become staged files
        let (hovering_files, dropped): (bool, Vec<String>) = ui.input(|i| {
            (
                !i.raw.hovered_files.is_empty(),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| {
                        f.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".into())
                    })
                    .collect(),
            )
        });
        for name in dropped {
            st.files.push(StagedFile::simulated(name, "—"));
        }

        // which menu (if any)?
        let menu = detect_menu(&st.text, !self.mentions.is_empty(), !self.commands.is_empty());
        let items: Vec<usize> = match &menu {
            Some(Menu::Mentions(q)) => (0..self.mentions.len())
                .filter(|&i| self.mentions[i].name.to_lowercase().starts_with(&q.to_lowercase()))
                .collect(),
            Some(Menu::Commands(q)) => (0..self.commands.len())
                .filter(|&i| self.commands[i].name.to_lowercase().starts_with(&q.to_lowercase()))
                .collect(),
            None => Vec::new(),
        };
        let menu_open = menu.is_some() && !items.is_empty();
        let qh = match &menu {
            Some(Menu::Mentions(q)) | Some(Menu::Commands(q)) => hash_str(q),
            None => 0,
        };
        let mut ms: MenuState = glowkit_core::state(ui, id.with("menu"), MenuState::default);
        if ms.last_query_hash != qh {
            ms = MenuState { hi: 0, last_query_hash: qh };
        }
        let focused = ui.memory(|m| m.has_focus(te_id));
        let mut accept: Option<usize> = None;
        let mut submit = false;
        if focused {
            ui.input_mut(|i| {
                if menu_open {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        ms.hi = (ms.hi + 1) % items.len();
                    }
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        ms.hi = (ms.hi + items.len() - 1) % items.len();
                    }
                    if i.consume_key(Modifiers::NONE, Key::Enter) || i.consume_key(Modifiers::NONE, Key::Tab) {
                        accept = Some(items[ms.hi.min(items.len() - 1)]);
                    }
                } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                    submit = true;
                }
            });
        }

        // ------------------------------------------------------------------ layout
        let start = ui.cursor().min;
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let pad = 14.0;
        let inner_w = w - pad * 2.0;
        let mut remove_file = None;
        let mut remove_chip = None;
        let inner = ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(start + vec2(pad, 12.0), vec2(inner_w, 2000.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                if !st.files.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for (i, f) in st.files.iter().enumerate() {
                            if file_tile(ui, &t, id.with(("file", i)), f, time) {
                                remove_file = Some(i);
                            }
                        }
                    });
                }
                if !st.chips.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for (i, c) in st.chips.iter().enumerate() {
                            if context_chip(ui, &t, id.with(("chip", i)), c) {
                                remove_chip = Some(i);
                            }
                        }
                    });
                }
                // text with inline highlighting of tokens
                let mention_names: Vec<String> = self.mentions.iter().map(|m| m.name.clone()).collect();
                let cmd_names: Vec<String> = self.commands.iter().map(|c| c.name.clone()).collect();
                let tc = t.clone();
                let mut layouter = move |ui: &Ui, buf: &dyn egui::TextBuffer, wrap: f32| -> Arc<egui::Galley> {
                    let mut job = highlight_job(buf.as_str(), &tc, &mention_names, &cmd_names);
                    job.wrap.max_width = wrap;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let te = egui::TextEdit::multiline(&mut st.text)
                    .id(te_id)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_rows(1)
                    .desired_width(inner_w)
                    .hint_text(
                        egui::RichText::new(self.placeholder).color(with_alpha(t.muted_foreground, 0.8)).size(14.5),
                    )
                    .layouter(&mut layouter)
                    .show(ui);
                let caret_end = te.galley.rows.last().map(|r| te.galley_pos + r.rect().right_center().to_vec2());
                ui.add_space(2.0);
                let (bar, _) = ui.allocate_exact_size(vec2(inner_w, 32.0), Sense::hover());
                (te.response.response, bar, caret_end)
            },
        );
        let (te_resp, bar, _caret) = inner.inner;
        let rect = Rect::from_min_max(start, pos2(start.x + w, inner.response.rect.max.y + 10.0));
        ui.allocate_rect(rect, Sense::hover());
        out.rect = rect;
        if let Some(i) = remove_file {
            st.files.remove(i);
        }
        if let Some(i) = remove_chip {
            st.chips.remove(i);
        }

        // toolbar
        let ar = Rect::from_min_size(pos2(bar.min.x - 4.0, bar.min.y), Vec2::splat(32.0));
        out.attach_rect = ar;
        if icon_button(ui, id.with("attach"), ar, Icon::Plus, false).clicked() {
            out.attach_clicked = true;
        }
        if let Some(m) = self.model_label {
            let mw = text_w(ui, m, 12.5) + 30.0;
            let mr = Rect::from_min_size(pos2(ar.max.x + 4.0, bar.center().y - 14.0), vec2(mw, 28.0));
            let r = ui.interact(mr, id.with("model"), Sense::click());
            let hk = toggle(ui, id.with("mh"), r.hovered());
            let p = ui.painter();
            p.rect_filled(mr, cr(14.0), with_alpha(well(&t), hk));
            text(p, pos2(mr.min.x + 10.0, mr.center().y), Align2::LEFT_CENTER, m, 12.5, t.muted_foreground);
            icon(p, Icon::ChevronDown, pos2(mr.max.x - 11.0, mr.center().y), 10.0, t.muted_foreground);
        }
        let can_send = !st.text.trim().is_empty() && st.files.iter().all(|f| f.progress >= 1.0);
        let sk = toggle(ui, id.with("send_k"), can_send);
        let sr = Rect::from_min_size(pos2(bar.max.x - 32.0 + 4.0, bar.min.y), Vec2::splat(32.0));
        out.send_rect = sr;
        let sresp = ui.interact(sr, id.with("send"), Sense::click());
        {
            let p = ui.painter();
            let fill = mix(well(&t), t.primary, sk);
            p.circle_filled(sr.center(), 15.0 + sk * 1.0, fill);
            icon(p, Icon::ArrowUp, sr.center(), 15.0, mix(t.muted_foreground, t.primary_foreground, sk));
        }
        let mic = Rect::from_min_size(pos2(sr.min.x - 36.0, bar.min.y), Vec2::splat(32.0));
        let _ = icon_button(ui, id.with("mic"), mic, Icon::Mic, false);
        if (sresp.clicked() || submit) && can_send {
            out.submitted = Some(std::mem::take(&mut st.text));
            st.files.clear();
            st.chips.clear();
        }

        // background, focus ring, drop overlay
        let fk = toggle(ui, id.with("focus"), te_resp.has_focus() || self.focus_ring);
        let drag = toggle(ui, id.with("drag"), hovering_files || self.drag_preview);
        let mut shapes = Vec::new();
        let sh = egui::epaint::Shadow { offset: [0, 3], blur: 16, spread: 0, color: shadow(&t, 0.7 + 0.5 * fk) };
        shapes.push(sh.as_shape(rect, cr(22.0)).into());
        if fk > 0.01 {
            shapes.push(egui::Shape::rect_stroke(
                rect.expand(3.0),
                cr(25.0),
                Stroke::new(4.0, with_alpha(t.ring, 0.13 * fk)),
                StrokeKind::Inside,
            ));
        }
        shapes.push(egui::Shape::rect_filled(rect, cr(22.0), surface(&t)));
        shapes.push(egui::Shape::rect_stroke(
            rect,
            cr(22.0),
            Stroke::new(1.0, mix(hairline(&t).color, t.ring, 0.55 * fk)),
            StrokeKind::Inside,
        ));
        ui.painter().set(bg_idx, egui::Shape::Vec(shapes));
        if drag > 0.01 {
            let p = ui.painter();
            let r = rect.shrink(3.0);
            p.rect_filled(r, cr(19.0), with_alpha(mix(surface(&t), t.accent, 0.06), 0.96 * drag));
            let dash = rect_dashes(r.shrink(4.0), 16.0, 8.0, 5.0, time * 18.0);
            for d in dash {
                p.line_segment(d, Stroke::new(1.5, with_alpha(t.accent, 0.8 * drag)));
            }
            let bob = (time * 3.0).sin() * 3.0;
            icon(p, Icon::Download, r.center() - vec2(0.0, 12.0 - bob * drag), 22.0, with_alpha(t.accent, drag));
            text(
                p,
                r.center() + vec2(0.0, 16.0),
                Align2::CENTER_CENTER,
                "Drop files to attach",
                13.0,
                with_alpha(t.foreground, drag),
            );
            ui.ctx().request_repaint();
        }

        // ------------------------------------------------------------------ menu
        let open_k = toggle(ui, id.with("menu_k"), menu_open);
        if accept.is_none() && menu_open && open_k > 0.01 {
            let rows = items.len().min(6);
            let mw = (w * 0.62).max(260.0).min(w);
            let mh = rows as f32 * 38.0 + 32.0;
            let pos = pos2(rect.min.x + 6.0, rect.min.y - 8.0 - mh + (1.0 - open_k) * 8.0);
            let area =
                egui::Area::new(id.with("menu_area")).order(egui::Order::Foreground).fixed_pos(pos).interactable(true);
            let mention_list = self.mentions;
            let cmd_list = self.commands;
            let header = match &menu {
                Some(Menu::Mentions(_)) => "Mention",
                _ => "Commands",
            };
            area.show(ui.ctx(), |ui| {
                ui.multiply_opacity(open_k.clamp(0.0, 1.0));
                let (mr, _) = ui.allocate_exact_size(vec2(mw, mh), Sense::hover());
                let p = ui.painter();
                paint::soft_shadow(p, mr, 14.0, 22.0, vec2(0.0, 6.0), shadow(&t, 1.3));
                p.rect_filled(mr, cr(14.0), t.popover);
                p.rect_stroke(mr, cr(14.0), hairline(&t), StrokeKind::Inside);
                text(p, pos2(mr.min.x + 14.0, mr.min.y + 16.0), Align2::LEFT_CENTER, header, 11.0, t.muted_foreground);
                let hi_y = glowkit_core::anim::spring(ui, id.with("hi_y"), ms.hi.min(rows - 1) as f32, 520.0, 34.0);
                let hr =
                    Rect::from_min_size(pos2(mr.min.x + 6.0, mr.min.y + 28.0 + hi_y * 38.0), vec2(mw - 12.0, 36.0));
                ui.painter().rect_filled(hr, cr(10.0), well(&t));
                for (k, &ix) in items.iter().take(rows).enumerate() {
                    let r = Rect::from_min_size(
                        pos2(mr.min.x + 6.0, mr.min.y + 28.0 + k as f32 * 38.0),
                        vec2(mw - 12.0, 36.0),
                    );
                    let resp = ui.interact(r, id.with(("mi", k)), Sense::click());
                    if resp.hovered() {
                        ms.hi = k;
                    }
                    if resp.clicked() {
                        accept = Some(ix);
                    }
                    let p = ui.painter();
                    match &menu {
                        Some(Menu::Mentions(_)) => {
                            let m = &mention_list[ix];
                            let c = pos2(r.min.x + 18.0, r.center().y);
                            initials_avatar(p, &t, c, 11.0, &m.name[..1], m.color);
                            text(p, pos2(c.x + 18.0, c.y), Align2::LEFT_CENTER, &m.name, 13.5, t.foreground);
                            mono(p, pos2(r.max.x - 12.0, c.y), Align2::RIGHT_CENTER, &m.role, 10.5, t.muted_foreground);
                        }
                        _ => {
                            let c = &cmd_list[ix];
                            let ic = pos2(r.min.x + 18.0, r.center().y);
                            icon(p, c.icon, ic, 14.0, t.muted_foreground);
                            let nr = text(
                                p,
                                pos2(ic.x + 16.0, ic.y),
                                Align2::LEFT_CENTER,
                                &format!("/{}", c.name),
                                13.5,
                                t.foreground,
                            );
                            text(
                                p,
                                pos2(nr.max.x + 10.0, ic.y),
                                Align2::LEFT_CENTER,
                                &c.description,
                                12.0,
                                t.muted_foreground,
                            );
                            if k == ms.hi {
                                kbd(p, &t, ui, pos2(r.max.x - 8.0, ic.y), "Enter");
                            }
                        }
                    }
                }
            });
        }
        if let Some(ix) = accept {
            match &menu {
                Some(Menu::Mentions(q)) => {
                    let cut = st.text.len() - q.len() - 1;
                    st.text.truncate(cut);
                    st.text.push('@');
                    st.text.push_str(&self.mentions[ix].name);
                    st.text.push(' ');
                }
                Some(Menu::Commands(_)) => {
                    st.text = format!("/{} ", self.commands[ix].name);
                }
                None => {}
            }
            set_cursor_end(ui, te_id, &st.text);
        }
        glowkit_core::set_state(ui, id.with("menu"), ms);
        out.menu_open = menu_open;
        out
    }
}

fn set_cursor_end(ui: &Ui, te_id: Id, text: &str) {
    if let Some(mut s) = egui::TextEdit::load_state(ui.ctx(), te_id) {
        let n = text.chars().count();
        s.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(n))));
        s.store(ui.ctx(), te_id);
    }
    ui.memory_mut(|m| m.request_focus(te_id));
}

fn detect_menu(text: &str, mentions: bool, commands: bool) -> Option<Menu> {
    if commands && text.starts_with('/') && !text.contains(char::is_whitespace) {
        return Some(Menu::Commands(text[1..].to_owned()));
    }
    if mentions {
        let last = text.rsplit(char::is_whitespace).next().unwrap_or("");
        if let Some(q) = last.strip_prefix('@') {
            if !q.contains('@') {
                return Some(Menu::Mentions(q.to_owned()));
            }
        }
    }
    None
}

/// Color `@Name` mentions and a leading `/command` inside the draft.
fn highlight_job(s: &str, t: &Theme, mentions: &[String], cmds: &[String]) -> LayoutJob {
    let font = FontId::proportional(14.5);
    let base = TextFormat { font_id: font.clone(), color: t.foreground, line_height: Some(21.0), ..Default::default() };
    let tok = TextFormat {
        font_id: font,
        color: if t.dark { mix(t.accent, egui::Color32::WHITE, 0.35) } else { t.accent },
        background: with_alpha(t.accent, if t.dark { 0.2 } else { 0.1 }),
        line_height: Some(21.0),
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    let mut i = 0;
    let bytes = s.as_bytes();
    let mut plain_start = 0;
    while i < s.len() {
        let mut matched = 0;
        if bytes[i] == b'@' {
            for m in mentions {
                if s[i + 1..].starts_with(m.as_str()) {
                    matched = 1 + m.len();
                    break;
                }
            }
        } else if bytes[i] == b'/' && i == 0 {
            for c in cmds {
                let end = 1 + c.len();
                if s[1..].starts_with(c.as_str()) && (s.len() == end || s[end..].starts_with(' ')) {
                    matched = end;
                    break;
                }
            }
        }
        if matched > 0 {
            if plain_start < i {
                job.append(&s[plain_start..i], 0.0, base.clone());
            }
            job.append(&s[i..i + matched], 0.0, tok.clone());
            i += matched;
            plain_start = i;
        } else {
            i += s[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    if plain_start < s.len() {
        job.append(&s[plain_start..], 0.0, base);
    }
    job
}

/// A staged file tile; returns true when its remove button was clicked.
fn file_tile(ui: &mut Ui, t: &Theme, id: Id, f: &StagedFile, time: f32) -> bool {
    let name_w = text_w(ui, &f.name, 12.5).min(150.0);
    let (r, resp) = ui.allocate_exact_size(vec2(name_w + 76.0, 46.0), Sense::hover());
    let e = since_change(ui, id, hash_str(&f.name));
    keep_alive(ui, e, 0.6);
    let a = appear_curve(e).min(1.0);
    let r = Rect::from_center_size(r.center(), r.size() * (0.9 + 0.1 * a));
    let p = ui.painter();
    p.rect_filled(r, cr(12.0), with_alpha(well(t), a));
    let ib = Rect::from_min_size(r.min + vec2(7.0, 7.0), Vec2::splat(32.0));
    p.rect_filled(ib, cr(8.0), with_alpha(surface(t), a));
    let is_img = f.name.ends_with(".png") || f.name.ends_with(".jpg");
    icon(p, if is_img { Icon::Image } else { Icon::File }, ib.center(), 15.0, with_alpha(t.muted_foreground, a));
    if f.progress < 1.0 {
        // upload ring over the icon
        p.rect_filled(ib, cr(8.0), with_alpha(surface(t), 0.75 * a));
        p.circle_stroke(ib.center(), 9.0, Stroke::new(2.0, with_alpha(t.border, a)));
        let a1 = -std::f32::consts::FRAC_PI_2;
        paint::arc(p, ib.center(), 9.0, a1, a1 + f.progress * std::f32::consts::TAU, 2.0, t.accent);
        let _ = time;
    }
    let tx = ib.max.x + 8.0;
    let clip = Rect::from_min_max(pos2(tx, r.min.y), pos2(tx + name_w, r.max.y));
    p.with_clip_rect(clip).text(
        pos2(tx, r.min.y + 16.0),
        Align2::LEFT_CENTER,
        &f.name,
        FontId::proportional(12.5),
        with_alpha(t.foreground, a),
    );
    let sub = if f.progress < 1.0 { format!("Uploading {}%", (f.progress * 100.0) as u32) } else { f.size.clone() };
    text(p, pos2(tx, r.min.y + 32.0), Align2::LEFT_CENTER, &sub, 10.5, with_alpha(t.muted_foreground, a));
    let xr = Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), Vec2::splat(20.0));
    let x = ui.interact(xr, id.with("x"), Sense::click());
    let hk = toggle(ui, id.with("xh"), x.hovered() || resp.hovered());
    let p = ui.painter();
    if x.hovered() {
        p.circle_filled(xr.center(), 9.0, surface(t));
    }
    icon(p, Icon::X, xr.center(), 10.0, with_alpha(t.muted_foreground, (0.4 + 0.6 * hk) * a));
    x.clicked()
}

/// A context chip; returns true when removed.
fn context_chip(ui: &mut Ui, t: &Theme, id: Id, c: &ContextChip) -> bool {
    let w = text_w(ui, &c.label, 12.0) + 52.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, cr(13.0), surface(t));
    p.rect_stroke(r, cr(13.0), hairline(t), StrokeKind::Inside);
    icon(p, c.icon, pos2(r.min.x + 14.0, r.center().y), 12.0, t.accent);
    text(p, pos2(r.min.x + 25.0, r.center().y), Align2::LEFT_CENTER, &c.label, 12.0, t.foreground);
    let xr = Rect::from_center_size(pos2(r.max.x - 13.0, r.center().y), Vec2::splat(18.0));
    let x = ui.interact(xr, id.with("x"), Sense::click());
    let hk = toggle(ui, id.with("h"), x.hovered());
    icon(ui.painter(), Icon::X, xr.center(), 9.0, mix(t.muted_foreground, t.foreground, hk));
    x.clicked()
}

/// Dashes around a rounded rect, marching with `offset`.
fn rect_dashes(r: Rect, dash: f32, gap: f32, _inset: f32, offset: f32) -> Vec<[egui::Pos2; 2]> {
    let mut pts = paint::rounded_rect_points(r, 16.0, 6);
    if let Some(f) = pts.first().copied() {
        pts.push(f);
    }
    let mut cum = vec![0.0f32];
    for w in pts.windows(2) {
        cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).length());
    }
    let total = cum.last().copied().unwrap_or(0.0);
    let at = |d: f32| -> egui::Pos2 {
        let d = d.clamp(0.0, total);
        let i = cum.partition_point(|c| *c < d).clamp(1, pts.len() - 1);
        let seg = (cum[i] - cum[i - 1]).max(1e-4);
        pts[i - 1].lerp(pts[i], (d - cum[i - 1]) / seg)
    };
    let period = dash + gap;
    let mut out = Vec::new();
    let mut s = offset.rem_euclid(period) - period;
    while s < total && out.len() < 400 {
        let a = s.max(0.0);
        let b = (s + dash).min(total);
        if b > a {
            out.push([at(a), at(b)]);
        }
        s += period;
    }
    out
}
