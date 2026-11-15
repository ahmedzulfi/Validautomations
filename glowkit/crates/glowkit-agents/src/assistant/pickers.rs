use super::common::*;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use glowkit_ai::Icon;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::{paint, Theme};

/// One mode of an [`AgentModeSwitcher`].
#[derive(Clone, Debug, PartialEq)]
pub struct AgentMode {
    /// Short name ("Ask", "Agent", "Plan").
    pub name: String,
    /// Icon.
    pub icon: Icon,
    /// One-line explanation shown under the switcher.
    pub description: String,
    /// Tint of the selected pill.
    pub color: Color32,
}

impl AgentMode {
    /// A mode.
    pub fn new(name: impl Into<String>, icon: Icon, description: impl Into<String>, color: Color32) -> Self {
        Self { name: name.into(), icon, description: description.into(), color }
    }
}

/// A segmented switch between assistant modes (e.g. Ask / Agent / Plan): a pill glides between
/// segments on a spring and takes on the active mode's tint, the icon of the active mode does a
/// small hop, and the description line underneath slides and cross-fades to the new mode.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{AgentMode, AgentModeSwitcher};
/// use glowkit_ai::Icon;
/// let modes = [AgentMode::new("Ask", Icon::Search, "Answers only", egui::Color32::LIGHT_BLUE)];
/// let mut mode = 0;
/// ui.add(AgentModeSwitcher::new(&modes, &mut mode));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct AgentModeSwitcher<'a> {
    modes: &'a [AgentMode],
    selected: &'a mut usize,
    describe: bool,
    seg_width: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> AgentModeSwitcher<'a> {
    /// A switcher over `modes` editing `selected`.
    pub fn new(modes: &'a [AgentMode], selected: &'a mut usize) -> Self {
        Self { modes, selected, describe: true, seg_width: 96.0, salt: None }
    }
    /// Show the description line (default on).
    pub fn describe(mut self, on: bool) -> Self {
        self.describe = on;
        self
    }
    /// Segment width (default 96).
    pub fn segment_width(mut self, w: f32) -> Self {
        self.seg_width = w.max(56.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }
}

impl egui::Widget for AgentModeSwitcher<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_modes");
        let n = self.modes.len().max(1);
        let seg = self.seg_width;
        let w = seg * n as f32 + 8.0;
        let h = if self.describe { 40.0 + 30.0 } else { 40.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w.max(if self.describe { 300.0 } else { 0.0 }), h), Sense::hover());
        let track = Rect::from_min_size(pos2(rect.center().x - w / 2.0, rect.min.y), vec2(w, 40.0));
        *self.selected = (*self.selected).min(n - 1);
        let x = glowkit_core::anim::spring(ui, id.with("x"), *self.selected as f32, 300.0, 24.0);
        let i0 = x.floor().clamp(0.0, (n - 1) as f32) as usize;
        let i1 = (i0 + 1).min(n - 1);
        let tint = mix(self.modes[i0].color, self.modes[i1].color, x - i0 as f32);
        let p = ui.painter();
        p.rect_filled(track, cr(20.0), well(&t));
        p.rect_stroke(track, cr(20.0), hairline(&t), StrokeKind::Inside);
        let pill = Rect::from_min_size(pos2(track.min.x + 4.0 + x * seg, track.min.y + 4.0), vec2(seg, 32.0));
        paint::soft_shadow(p, pill, 16.0, 8.0, vec2(0.0, 2.0), shadow(&t, 0.8));
        p.rect_filled(pill, cr(16.0), surface(&t));
        p.rect_stroke(pill, cr(16.0), Stroke::new(1.0, with_alpha(tint, 0.35)), StrokeKind::Inside);
        let mut changed = None;
        for (i, m) in self.modes.iter().enumerate() {
            let sr = Rect::from_min_size(pos2(track.min.x + 4.0 + i as f32 * seg, track.min.y + 4.0), vec2(seg, 32.0));
            let b = ui.interact(sr, id.with(("m", i)), Sense::click());
            let on = (1.0 - (x - i as f32).abs()).clamp(0.0, 1.0);
            let hk = toggle(ui, id.with(("h", i)), b.hovered());
            let e = since_change(ui, id.with(("sel", i)), (*self.selected == i) as u64);
            let jump = if *self.selected == i && e < 0.45 { (e / 0.45 * std::f32::consts::PI).sin() * 3.0 } else { 0.0 };
            keep_alive(ui, e, 0.45);
            let col = mix(mix(t.muted_foreground, t.foreground, hk * 0.6), m.color, on);
            let tw = text_w(ui, &m.name, 13.0);
            let x0 = sr.center().x - (tw + 20.0) / 2.0;
            let p = ui.painter();
            icon(p, m.icon, pos2(x0 + 7.0, sr.center().y - jump), 14.0, col);
            if on > 0.5 {
                text_bold(p, pos2(x0 + 20.0, sr.center().y), Align2::LEFT_CENTER, &m.name, 13.0, mix(t.muted_foreground, t.foreground, on));
            } else {
                text(p, pos2(x0 + 20.0, sr.center().y), Align2::LEFT_CENTER, &m.name, 13.0, mix(t.muted_foreground, t.foreground, on.max(hk * 0.6)));
            }
            if b.clicked() {
                changed = Some(i);
            }
        }
        if self.describe {
            let p = ui.painter();
            for (i, m) in self.modes.iter().enumerate() {
                let d = x - i as f32;
                let a = (1.0 - d.abs() * 1.6).clamp(0.0, 1.0);
                if a > 0.01 {
                    text(p, pos2(rect.center().x - d * 30.0, track.max.y + 18.0), Align2::CENTER_CENTER, &m.description, 12.5, with_alpha(t.muted_foreground, a));
                }
            }
        }
        let mut resp = resp;
        if let Some(i) = changed {
            *self.selected = i;
            resp.mark_changed();
        }
        resp
    }
}

/// One model in a [`ModelPickerRich`].
#[derive(Clone, Debug, PartialEq)]
pub struct ModelSpec {
    /// Display name.
    pub name: String,
    /// Provider / family label.
    pub provider: String,
    /// Short description.
    pub description: String,
    /// Speed 1..=3 (bolts).
    pub speed: u8,
    /// Cost 1..=3 (dollar signs).
    pub cost: u8,
    /// Context window label, e.g. "200K".
    pub context: String,
    /// Capability tags ("Reasoning", "Vision"...).
    pub tags: Vec<String>,
    /// Provider color.
    pub color: Color32,
}

impl ModelSpec {
    /// A model entry.
    pub fn new(name: impl Into<String>, provider: impl Into<String>, color: Color32) -> Self {
        Self { name: name.into(), provider: provider.into(), description: String::new(), speed: 2, cost: 2, context: "128K".into(), tags: Vec::new(), color }
    }
    /// Description line.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    /// Speed (1..=3) and cost (1..=3).
    pub fn speed_cost(mut self, speed: u8, cost: u8) -> Self {
        self.speed = speed.clamp(1, 3);
        self.cost = cost.clamp(1, 3);
        self
    }
    /// Context window label.
    pub fn context(mut self, c: impl Into<String>) -> Self {
        self.context = c.into();
        self
    }
    /// Capability tags.
    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }
}

#[derive(Clone, Default)]
struct PickerState {
    open: bool,
    query: String,
    hover: usize,
}

/// A rich model picker: a trigger pill with the provider mark and model name opens a popover with
/// a search field and one row per model showing provider mark, name and description, speed bolts,
/// cost, context-window chip and capability tags, with a check on the current model. The hover
/// highlight glides between rows; the popover springs open from the trigger.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_agents::{ModelPickerRich, ModelSpec};
/// let models = [ModelSpec::new("Swift", "Acme", egui::Color32::LIGHT_BLUE).speed_cost(3, 1)];
/// let mut sel = 0;
/// ModelPickerRich::new(&models, &mut sel).show(ui);
/// ```
#[must_use = "call `.show(ui)`"]
pub struct ModelPickerRich<'a> {
    models: &'a [ModelSpec],
    selected: &'a mut usize,
    open: Option<bool>,
    hover: Option<usize>,
    width: f32,
    salt: Option<egui::IdSalt>,
}

impl<'a> ModelPickerRich<'a> {
    /// A picker over `models` editing `selected`.
    pub fn new(models: &'a [ModelSpec], selected: &'a mut usize) -> Self {
        Self { models, selected, open: None, hover: None, width: 440.0, salt: None }
    }
    /// Force the popover open or closed this frame.
    pub fn open(mut self, on: bool) -> Self {
        self.open = Some(on);
        self
    }
    /// Force the highlighted row (for autoplay).
    pub fn hover_row(mut self, i: Option<usize>) -> Self {
        self.hover = i;
        self
    }
    /// Popover width (default 440).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(300.0);
        self
    }
    /// Stable id salt.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.salt = Some(egui::IdSalt::new(salt));
        self
    }

    /// Show the trigger (and popover). Returns the trigger response and the row rects.
    pub fn show(self, ui: &mut Ui) -> (Response, Vec<Rect>) {
        let t = Theme::of(ui);
        let id = wid(ui, self.salt, "asst_modelpicker");
        let mut st: PickerState = glowkit_core::state(ui, id.with("st"), PickerState::default);
        if let Some(o) = self.open {
            st.open = o;
        }
        let n = self.models.len();
        *self.selected = (*self.selected).min(n.saturating_sub(1));
        let cur = &self.models[*self.selected];
        let tw = text_w(ui, &cur.name, 13.5) + 64.0;
        let (tr, resp) = ui.allocate_exact_size(vec2(tw, 36.0), Sense::click());
        let hk = toggle(ui, id.with("th"), resp.hovered() || st.open);
        let p = ui.painter();
        p.rect_filled(tr, cr(18.0), mix(surface(&t), well(&t), hk));
        p.rect_stroke(tr, cr(18.0), hairline(&t), StrokeKind::Inside);
        provider_mark(p, pos2(tr.min.x + 18.0, tr.center().y), 8.0, cur);
        text_bold(p, pos2(tr.min.x + 32.0, tr.center().y), Align2::LEFT_CENTER, &cur.name, 13.5, t.foreground);
        let ok = toggle(ui, id.with("ok"), st.open);
        let ang = ok * std::f32::consts::PI;
        let c = pos2(tr.max.x - 17.0, tr.center().y);
        let d = vec2(4.0, 2.2);
        let rot = |v: Vec2| vec2(v.x, v.y * ang.cos());
        p.line_segment([c + rot(vec2(-d.x, -d.y)), c + rot(vec2(0.0, d.y))], Stroke::new(1.5, t.muted_foreground));
        p.line_segment([c + rot(vec2(0.0, d.y)), c + rot(vec2(d.x, -d.y))], Stroke::new(1.5, t.muted_foreground));
        if resp.clicked() {
            st.open = !st.open;
        }
        let mut rows = Vec::new();
        if ok > 0.01 {
            let q = st.query.to_lowercase();
            let items: Vec<usize> = (0..n).filter(|&i| q.is_empty() || self.models[i].name.to_lowercase().contains(&q) || self.models[i].provider.to_lowercase().contains(&q)).collect();
            let row_h = 58.0;
            let ph = 52.0 + items.len().max(1) as f32 * row_h + 10.0;
            let pos = pos2(tr.min.x, tr.max.y + 8.0 - (1.0 - ok) * 8.0);
            let models = self.models;
            let selected = &mut *self.selected;
            let force_hover = self.hover;
            let pw = self.width;
            let mut close = false;
            egui::Area::new(id.with("pop")).order(egui::Order::Foreground).fixed_pos(pos).show(ui.ctx(), |ui| {
                ui.multiply_opacity(ok);
                let (r, _) = ui.allocate_exact_size(vec2(pw, ph), Sense::hover());
                let p = ui.painter();
                paint::soft_shadow(p, r, 16.0, 28.0, vec2(0.0, 10.0), shadow(&t, 1.5));
                p.rect_filled(r, cr(16.0), t.popover);
                p.rect_stroke(r, cr(16.0), hairline(&t), StrokeKind::Inside);
                let sr = Rect::from_min_size(r.min + vec2(8.0, 8.0), vec2(pw - 16.0, 36.0));
                p.rect_filled(sr, cr(10.0), well(&t));
                icon(p, Icon::Search, pos2(sr.min.x + 16.0, sr.center().y), 13.0, t.muted_foreground);
                let te = Rect::from_min_max(pos2(sr.min.x + 30.0, sr.min.y), sr.max - vec2(10.0, 0.0));
                ui.put(te, egui::TextEdit::singleline(&mut st.query).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(FontId::proportional(13.0)).align(Align2::LEFT_CENTER).hint_text("Search models").desired_width(te.width()));
                let list_top = sr.max.y + 6.0;
                for (k, &i) in items.iter().enumerate() {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, list_top + k as f32 * row_h), vec2(pw - 12.0, row_h - 2.0));
                    rows.push(rr);
                    let b = ui.interact(rr, id.with(("row", i)), Sense::click());
                    if b.hovered() {
                        st.hover = k;
                    }
                    if b.clicked() {
                        *selected = i;
                        close = true;
                    }
                }
                let hv = force_hover.unwrap_or(st.hover).min(items.len().saturating_sub(1));
                let hy = glowkit_core::anim::spring(ui, id.with("hy"), hv as f32, 480.0, 34.0);
                if !items.is_empty() {
                    let hr = Rect::from_min_size(pos2(r.min.x + 6.0, list_top + hy * row_h), vec2(pw - 12.0, row_h - 2.0));
                    ui.painter().rect_filled(hr, cr(11.0), well(&t));
                }
                for (k, &i) in items.iter().enumerate() {
                    let m = &models[i];
                    let rr = rows[k];
                    let p = ui.painter();
                    provider_mark(p, pos2(rr.min.x + 22.0, rr.center().y), 13.0, m);
                    let tx = rr.min.x + 44.0;
                    let nr = text_bold(p, pos2(tx, rr.min.y + 19.0), Align2::LEFT_CENTER, &m.name, 13.5, t.foreground);
                    let mut x = nr.max.x + 8.0;
                    for tag in &m.tags {
                        let r = chip(p, &t, ui, pos2(x, rr.min.y + 11.0), tag, 10.0, Some(if tag == "Reasoning" { t.accent } else { t.info }));
                        x = r.max.x + 4.0;
                    }
                    text(p, pos2(tx, rr.min.y + 39.0), Align2::LEFT_CENTER, &m.description, 11.5, t.muted_foreground);
                    // badges on the right: speed, cost, context
                    let rx = rr.max.x - 34.0;
                    let ctx_w = text_w(ui, &m.context, 10.5) + 12.0;
                    let cr_ = Rect::from_min_size(pos2(rx - ctx_w, rr.min.y + 10.0), vec2(ctx_w, 18.0));
                    p.rect_stroke(cr_, cr(5.0), hairline(&t), StrokeKind::Inside);
                    mono(p, cr_.center(), Align2::CENTER_CENTER, &m.context, 10.0, t.muted_foreground);
                    for b in 0..3 {
                        let on = b < m.speed as usize;
                        let c = pos2(rx - 32.0 + b as f32 * 11.0, rr.min.y + 40.0);
                        bolt(p, c, 4.5, if on { t.warning } else { with_alpha(t.muted_foreground, 0.25) });
                    }
                    for b in 0..3 {
                        let on = b < m.cost as usize;
                        text(p, pos2(rx - 64.0 + b as f32 * 7.0, rr.min.y + 40.0), Align2::CENTER_CENTER, "$", 11.0, if on { t.foreground } else { with_alpha(t.muted_foreground, 0.3) });
                    }
                    if i == *selected {
                        check_disc(p, pos2(rr.max.x - 16.0, rr.center().y), 8.0, t.primary, t.primary_foreground, 1.0);
                    }
                }
            });
            if close {
                st.open = false;
            }
        }
        glowkit_core::set_state(ui, id.with("st"), st);
        (resp, rows)
    }
}

fn provider_mark(p: &egui::Painter, c: egui::Pos2, r: f32, m: &ModelSpec) {
    p.circle_filled(c, r, m.color);
    let ch = m.provider.chars().next().unwrap_or('?').to_uppercase().to_string();
    p.text(c, Align2::CENTER_CENTER, ch, FontId::proportional(r * 1.05), Color32::WHITE);
}

fn bolt(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32) {
    let pts = [c + vec2(0.3, -1.2) * s, c + vec2(-0.6, 0.15) * s, c + vec2(0.0, 0.15) * s, c + vec2(-0.3, 1.2) * s, c + vec2(0.6, -0.15) * s, c + vec2(0.0, -0.15) * s];
    p.add(egui::Shape::convex_polygon(vec![pts[0], pts[1], pts[2]], col, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![pts[2], pts[5], pts[0]], col, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![pts[3], pts[4], pts[5]], col, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![pts[5], pts[2], pts[3]], col, Stroke::NONE));
}
