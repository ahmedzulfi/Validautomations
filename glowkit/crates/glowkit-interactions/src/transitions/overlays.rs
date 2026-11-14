//! Overlay transitions: a dropdown, a button that morphs into its menu, a modal, a bottom panel,
//! a shared tooltip, a toast and a stack of banners.

use super::common::{
    avatar, bar, chip, chip_button, cr, layer, material, phase, skeleton, span, surface, two_lines, xf_around, MotionCurve,
    Tween, AVATAR_TONES,
};
use egui::emath::TSTransform;
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::Theme;
use glowkit_effects::MicroIcon;

fn wash(t: &Theme) -> Color32 {
    if t.dark {
        with_alpha(Color32::WHITE, 0.06)
    } else {
        Color32::from_black_alpha(9)
    }
}

/// Paint menu rows (icon + label) or, when there are none, three skeleton bars. Returns the
/// rects of the rows.
fn menu_rows(painter: &egui::Painter, t: &Theme, inner: Rect, items: &[(Option<MicroIcon>, String)], hovered: Option<usize>, row_h: f32) -> Vec<Rect> {
    let mut rects = Vec::new();
    if items.is_empty() {
        let widths = [0.78, 0.55, 0.92];
        for (i, w) in widths.iter().enumerate() {
            let y = inner.min.y + 12.0 + i as f32 * 26.0;
            bar(painter, t, Rect::from_min_size(pos2(inner.min.x + 8.0, y), vec2((inner.width() - 16.0) * w, 9.0)));
        }
        return rects;
    }
    for (i, (icon, label)) in items.iter().enumerate() {
        let r = Rect::from_min_size(pos2(inner.min.x, inner.min.y + i as f32 * row_h), vec2(inner.width(), row_h));
        if hovered == Some(i) {
            painter.rect_filled(r, cr(8.0), wash(t));
        }
        let mut x = r.min.x + 10.0;
        if let Some(ic) = icon {
            ic.paint(painter, pos2(x + 7.0, r.center().y), 14.0, t.muted_foreground);
            x += 24.0;
        }
        painter.text(pos2(x, r.center().y), Align2::LEFT_CENTER, label, FontId::proportional(13.0), t.foreground);
        rects.push(r);
    }
    rects
}

// ---------------------------------------------------------------------------------------------

/// A trigger pill with a dropdown that opens from its top edge: it scales up from 97% while
/// fading in on a long smooth curve, and closes quicker, barely shrinking.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = false;
/// let picked = glowkit_interactions::DropdownSurface::new(&mut open)
///     .item(None, "Profile")
///     .item(None, "Settings")
///     .item(None, "Sign out")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct DropdownSurface<'a> {
    open: &'a mut bool,
    label: String,
    items: Vec<(Option<MicroIcon>, String)>,
    width: f32,
    open_dur: f32,
    close_dur: f32,
    from_scale: f32,
    curve: MotionCurve,
}

impl<'a> DropdownSurface<'a> {
    /// A dropdown whose open state lives in `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self {
            open,
            label: "Toggle menu".into(),
            items: Vec::new(),
            width: 180.0,
            open_dur: 0.25,
            close_dur: 0.15,
            from_scale: 0.97,
            curve: MotionCurve::SMOOTH_OUT,
        }
    }
    /// Trigger label (default "Toggle menu").
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// Add a row (without rows the surface shows skeleton bars).
    pub fn item(mut self, icon: Option<MicroIcon>, label: impl Into<String>) -> Self {
        self.items.push((icon, label.into()));
        self
    }
    /// Surface width (default 180).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(60.0);
        self
    }
    /// Open and close durations in seconds (default 0.25 / 0.15).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// Scale the surface opens from (default 0.97).
    pub fn from_scale(mut self, s: f32) -> Self {
        self.from_scale = s.clamp(0.3, 1.0);
        self
    }
    /// Timing curve (default [`MotionCurve::SMOOTH_OUT`]).
    pub fn curve(mut self, c: MotionCurve) -> Self {
        self.curve = c;
        self
    }

    /// Show it; returns the trigger's response and the index of a row picked this frame.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<usize>) {
        let t = Theme::of(ui);
        let row_h = 32.0;
        let body_h = if self.items.is_empty() { 86.0 } else { self.items.len() as f32 * row_h + 12.0 };
        let mut picked = None;
        let inner = ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let trig = chip_button(ui, &self.label);
            if trig.clicked() {
                *self.open = !*self.open;
            }
            let (rect, resp) = ui.allocate_exact_size(vec2(self.width, body_h), Sense::hover());
            (trig, rect, resp.id)
        });
        let (trig, rect, id) = inner.inner;
        let ph = phase(ui, id, *self.open, self.open_dur, self.close_dur, self.curve, self.curve);
        let p = ph.p.clamp(0.0, 1.2);
        if p <= 0.002 {
            return (trig, None);
        }
        let s = if ph.on { self.from_scale + (1.0 - self.from_scale) * p } else { 0.99 + 0.01 * p };
        // hit testing in the un-scaled rect (it is within a few points of the painted one)
        let mut hovered = None;
        let mut rects = Vec::new();
        let inner_r = rect.shrink2(vec2(6.0, 6.0));
        if !self.items.is_empty() && ph.on {
            for i in 0..self.items.len() {
                let r = Rect::from_min_size(pos2(inner_r.min.x, inner_r.min.y + i as f32 * row_h), vec2(inner_r.width(), row_h));
                let resp = ui.interact(r, id.with(("row", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.hovered() {
                    hovered = Some(i);
                }
                if resp.clicked() {
                    picked = Some(i);
                }
                rects.push(r);
            }
        }
        if picked.is_some() {
            *self.open = false;
        }
        let origin = pos2(rect.center().x, rect.min.y);
        layer(ui, xf_around(origin, s, Vec2::ZERO), p.min(1.0), 0.0, |pt| {
            material(pt, &t, rect, 12.0, surface(&t));
            menu_rows(pt, &t, inner_r, &self.items, hovered, row_h);
        });
        (trig, picked)
    }
}

impl Widget for DropdownSurface<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).0
    }
}

// ---------------------------------------------------------------------------------------------

/// A round plus button that morphs into the menu it opens: the circle stretches up and to the
/// left into a rounded panel on a springy curve, the plus spins away while the rows slide in from
/// the right, and closing tucks it back with a little anticipatory nudge.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = false;
/// let picked = glowkit_interactions::MorphMenu::new(&mut open)
///     .item(glowkit_effects::MicroIcon::Doc, "New document")
///     .item(glowkit_effects::MicroIcon::Folder, "New folder")
///     .show(ui);
/// ```
#[must_use = "call `.show(ui)` or add it with `ui.add(...)`"]
pub struct MorphMenu<'a> {
    open: &'a mut bool,
    items: Vec<(MicroIcon, String)>,
    size: Vec2,
    button: f32,
    open_dur: f32,
    close_dur: f32,
    shift: f32,
}

impl<'a> MorphMenu<'a> {
    /// A morphing menu whose open state lives in `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self { open, items: Vec::new(), size: vec2(196.0, 172.0), button: 40.0, open_dur: 0.35, close_dur: 0.25, shift: 40.0 }
    }
    /// Add a row.
    pub fn item(mut self, icon: MicroIcon, label: impl Into<String>) -> Self {
        self.items.push((icon, label.into()));
        self
    }
    /// Open panel size (default 196 x 172; the height follows the rows when they need more).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Diameter of the closed button (default 40).
    pub fn button(mut self, d: f32) -> Self {
        self.button = d.max(20.0);
        self
    }
    /// Open and close durations in seconds (default 0.35 / 0.25).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// How far the rows slide in from, in points (default 40).
    pub fn shift(mut self, d: f32) -> Self {
        self.shift = d;
        self
    }

    /// Show it; returns the response and the index of a row picked this frame.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<usize>) {
        let t = Theme::of(ui);
        let row_h = 36.0;
        let size = vec2(self.size.x, self.size.y.max(self.items.len() as f32 * row_h + 16.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let id = response.id;
        let corner = rect.max;
        let closed = Rect::from_min_max(corner - Vec2::splat(self.button), corner);
        let btn = ui.interact(closed, id.with("btn"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let ph0 = phase(ui, id, *self.open, self.open_dur, self.close_dur, MotionCurve::SOFT_BOUNCE, MotionCurve::SMOOTH_OUT);
        let mut picked = None;
        if !*self.open && btn.clicked() {
            *self.open = true;
        } else if *self.open && ph0.age > 0.05 {
            let panel = Rect::from_min_max(corner - size, corner);
            let clicked_outside = ui.input(|i| i.pointer.primary_clicked())
                && ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| !panel.contains(p));
            if clicked_outside {
                *self.open = false;
            }
        }
        let ph = phase(ui, id, *self.open, self.open_dur, self.close_dur, MotionCurve::SOFT_BOUNCE, MotionCurve::SMOOTH_OUT);
        if !ui.is_rect_visible(rect) {
            return (response, None);
        }
        let p = ph.p;
        let w = self.button + (size.x - self.button) * p;
        let h = self.button + (size.y - self.button) * p;
        // closing anticipation: a small nudge toward the corner and back
        let nudge = if !ph.on && ph.age < 0.7 {
            let k = MotionCurve::SMOOTH_OUT.at(ph.age / 0.7);
            let bump = if k < 0.35 { k / 0.35 } else { 1.0 - (k - 0.35) / 0.65 };
            5.0 * bump.max(0.0)
        } else {
            0.0
        };
        let body = Rect::from_min_max(corner - vec2(w, h), corner).translate(Vec2::splat(nudge));
        let r = 20.0f32.min(body.height() / 2.0).min(body.width() / 2.0);
        let painter = ui.painter();
        let fill = surface(&t);
        let hover_btn = btn.hovered() && !*self.open;
        material(painter, &t, body, r, if hover_btn { mix(fill, t.foreground, 0.03) } else { fill });
        // plus: rotates 45 degrees and fades as it opens
        let plus_k = span(p, 0.0, 0.6);
        let pc = body.max - Vec2::splat(self.button / 2.0);
        layer(ui, TSTransform::IDENTITY, 1.0 - plus_k, 2.0 * plus_k, |pt| {
            let ang = plus_k * std::f32::consts::FRAC_PI_4;
            let s = 7.0 * (1.0 - 0.03 * plus_k);
            for a in [ang, ang + std::f32::consts::FRAC_PI_2] {
                let d = vec2(a.cos(), a.sin()) * s;
                pt.line_segment([pc - d, pc + d], Stroke::new(1.75, t.foreground));
            }
        });
        // rows
        let fade = if ph.on { MotionCurve::SMOOTH_OUT.timed(ph.age, 0.06, 0.2) } else { 1.0 - MotionCurve::SMOOTH_OUT.at(ph.age / 0.2) };
        let slide = if ph.on { 1.0 - MotionCurve::SMOOTH_OUT.timed(ph.age, 0.0, 0.35) } else { MotionCurve::SMOOTH_OUT.at(ph.age / 0.35) };
        let alpha = fade.clamp(0.0, 1.0) * span(p, 0.25, 0.7);
        if alpha > 0.0 {
            let inner = Rect::from_min_size(corner - size + vec2(8.0, 8.0), size - vec2(16.0, 16.0)).translate(Vec2::splat(nudge));
            let mut hovered = None;
            if ph.on && ph.t >= 1.0 {
                for i in 0..self.items.len() {
                    let rr = Rect::from_min_size(pos2(inner.min.x, inner.min.y + i as f32 * row_h), vec2(inner.width(), row_h));
                    let resp = ui.interact(rr, id.with(("row", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.hovered() {
                        hovered = Some(i);
                    }
                    if resp.clicked() {
                        picked = Some(i);
                    }
                }
            }
            let clip = body.shrink(1.0).intersect(ui.clip_rect());
            let items: Vec<(Option<MicroIcon>, String)> = self.items.iter().map(|(i, l)| (Some(*i), l.clone())).collect();
            let slid = inner.translate(vec2(self.shift * slide, 0.0));
            layer(ui, TSTransform::IDENTITY, alpha, 2.0 * (1.0 - fade.clamp(0.0, 1.0)), |pt| {
                let pt = pt.with_clip_rect(clip);
                menu_rows(&pt, &t, slid, &items, hovered, row_h);
            });
        }
        if picked.is_some() {
            *self.open = false;
        }
        (response, picked)
    }
}

impl Widget for MorphMenu<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).0
    }
}

// ---------------------------------------------------------------------------------------------

/// A dialog card that pops: it grows in from 96% while fading in on a long smooth curve, and
/// leaves faster than it came. Optionally dims what is behind it.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(
///     glowkit_interactions::PopModal::new(true)
///         .title("Delete project?")
///         .body("This removes the project and its 14 files for everyone.")
///         .actions("Cancel", "Delete"),
/// );
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct PopModal {
    open: bool,
    title: Option<String>,
    body: Option<String>,
    actions: Option<(String, String)>,
    size: Vec2,
    open_dur: f32,
    close_dur: f32,
    scale: f32,
    backdrop: bool,
    accent: Option<Color32>,
}

impl PopModal {
    /// A modal shown while `open` is true.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            title: None,
            body: None,
            actions: None,
            size: vec2(300.0, 168.0),
            open_dur: 0.25,
            close_dur: 0.15,
            scale: 0.96,
            backdrop: false,
            accent: None,
        }
    }
    /// Title (without one the card shows skeleton bars).
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = Some(s.into());
        self
    }
    /// Body text.
    pub fn body(mut self, s: impl Into<String>) -> Self {
        self.body = Some(s.into());
        self
    }
    /// Secondary and primary button labels.
    pub fn actions(mut self, secondary: impl Into<String>, primary: impl Into<String>) -> Self {
        self.actions = Some((secondary.into(), primary.into()));
        self
    }
    /// Card size (default 300 x 168).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Open and close durations in seconds (default 0.25 / 0.15).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// Scale it opens from and closes to (default 0.96).
    pub fn scale(mut self, s: f32) -> Self {
        self.scale = s.clamp(0.3, 1.0);
        self
    }
    /// Dim the widget's area behind the card.
    pub fn backdrop(mut self, on: bool) -> Self {
        self.backdrop = on;
        self
    }
    /// Primary button color (default: the theme primary).
    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = Some(c);
        self
    }
}

impl Widget for PopModal {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let (rect, response) = ui.allocate_exact_size(self.size + vec2(24.0, 24.0), Sense::hover());
        let ph = phase(ui, response.id, self.open, self.open_dur, self.close_dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        let p = ph.p.clamp(0.0, 1.0);
        if p <= 0.002 || !ui.is_rect_visible(rect) {
            return response;
        }
        if self.backdrop {
            ui.painter().rect_filled(rect, cr(16.0), Color32::from_black_alpha((60.0 * p) as u8));
        }
        let card = Rect::from_center_size(rect.center(), self.size);
        let s = self.scale + (1.0 - self.scale) * p;
        layer(ui, xf_around(card.center(), s, Vec2::ZERO), p, 0.0, |pt| {
            material(pt, &t, card, 16.0, surface(&t));
            let pad = 18.0;
            let inner = card.shrink(pad);
            // close dot
            let cd = pos2(card.max.x - 16.0, card.min.y + 16.0);
            pt.circle_filled(cd, 7.0, skeleton(&t));
            if self.title.is_none() {
                bar(pt, &t, Rect::from_min_size(inner.min + vec2(0.0, 6.0), vec2(inner.width() * 0.62, 11.0)));
                for (i, w) in [1.0, 1.0, 0.62].iter().enumerate() {
                    bar(pt, &t, Rect::from_min_size(inner.min + vec2(0.0, 30.0 + i as f32 * 14.0), vec2(inner.width() * w, 7.0)));
                }
            } else {
                let mut y = inner.min.y;
                if let Some(title) = &self.title {
                    let g = pt.layout(title.clone(), FontId::proportional(16.0), t.foreground, inner.width() - 20.0);
                    let h = g.size().y;
                    pt.galley(pos2(inner.min.x, y), g, t.foreground);
                    y += h + 6.0;
                }
                if let Some(body) = &self.body {
                    let g = pt.layout(body.clone(), FontId::proportional(13.0), t.muted_foreground, inner.width());
                    pt.galley(pos2(inner.min.x, y), g, t.muted_foreground);
                }
                // close glyph over the dot
                let d = 2.6;
                pt.line_segment([cd - vec2(d, d), cd + vec2(d, d)], Stroke::new(1.3, t.muted_foreground));
                pt.line_segment([cd + vec2(-d, d), cd + vec2(d, -d)], Stroke::new(1.3, t.muted_foreground));
            }
            // buttons
            let bh = 30.0;
            let by = inner.max.y - bh;
            let labels = self.actions.clone();
            let bw = 78.0;
            let b2 = Rect::from_min_size(pos2(inner.max.x - bw, by), vec2(bw, bh));
            let b1 = Rect::from_min_size(pos2(b2.min.x - bw - 8.0, by), vec2(bw, bh));
            match labels {
                Some((a, b)) => {
                    pt.rect_filled(b1, cr(bh / 2.0), chip(&t));
                    pt.text(b1.center(), Align2::CENTER_CENTER, a, FontId::proportional(13.0), t.foreground);
                    let acc = self.accent.unwrap_or(t.primary);
                    pt.rect_filled(b2, cr(bh / 2.0), acc);
                    pt.text(b2.center(), Align2::CENTER_CENTER, b, FontId::proportional(13.0), glowkit_core::color::contrast_text(acc));
                }
                None => {
                    pt.rect_filled(b1, cr(bh / 2.0), skeleton(&t));
                    pt.rect_filled(b2, cr(bh / 2.0), skeleton(&t));
                }
            }
        });
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A trigger with a panel that rises out of a clipped edge below it: the panel slides up from
/// half its height, un-blurring and fading in, and sinks back the same way.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let mut open = true;
/// ui.add(glowkit_interactions::RevealPanel::new(&mut open).title("Quick actions"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct RevealPanel<'a> {
    open: &'a mut bool,
    label: String,
    title: Option<String>,
    tiles: Vec<(MicroIcon, String)>,
    size: Vec2,
    open_dur: f32,
    close_dur: f32,
    blur: f32,
}

impl<'a> RevealPanel<'a> {
    /// A panel whose open state lives in `open`.
    pub fn new(open: &'a mut bool) -> Self {
        Self {
            open,
            label: "Toggle panel".into(),
            title: None,
            tiles: Vec::new(),
            size: vec2(260.0, 187.0),
            open_dur: 0.4,
            close_dur: 0.35,
            blur: 2.0,
        }
    }
    /// Trigger label (default "Toggle panel").
    pub fn label(mut self, s: impl Into<String>) -> Self {
        self.label = s.into();
        self
    }
    /// Panel heading (without one it shows skeleton bars).
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = Some(s.into());
        self
    }
    /// Add a tile (up to four are shown in a 2 x 2 grid).
    pub fn tile(mut self, icon: MicroIcon, label: impl Into<String>) -> Self {
        self.tiles.push((icon, label.into()));
        self
    }
    /// Panel size (default 260 x 187).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = s;
        self
    }
    /// Open and close durations in seconds (default 0.4 / 0.35).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// Blur at the hidden end (default 2).
    pub fn blur(mut self, b: f32) -> Self {
        self.blur = b.max(0.0);
        self
    }
}

impl Widget for RevealPanel<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size;
        let inner = ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let trig = chip_button(ui, &self.label);
            let (rect, resp) = ui.allocate_exact_size(vec2(size.x + 40.0, size.y + 30.0), Sense::hover());
            (trig, rect, resp.id)
        });
        let (trig, rect, id) = inner.inner;
        if trig.clicked() {
            *self.open = !*self.open;
        }
        let ph = phase(ui, id, *self.open, self.open_dur, self.close_dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        let p = ph.p.clamp(0.0, 1.0);
        if p <= 0.002 || !ui.is_rect_visible(rect) {
            return trig;
        }
        let panel = Rect::from_min_size(pos2(rect.center().x - size.x / 2.0, rect.max.y - size.y), size);
        let dy = size.y * 0.5 * (1.0 - p);
        let clip = rect.intersect(ui.clip_rect());
        let panel = panel.translate(vec2(0.0, dy));
        let tiles = self.tiles.clone();
        let title = self.title.clone();
        let blur = self.blur * (1.0 - p);
        layer(ui, TSTransform::IDENTITY, p, blur, |pt| {
            let pt = pt.with_clip_rect(clip);
            let fill = surface(&t);
            let r = 18.0;
            // a sheet: rounded top, flat bottom cut by the clip
            let sheet = panel.with_max_y(panel.max.y + r);
            if t.dark {
                pt.rect(sheet, cr(r), fill, Stroke::new(1.0, with_alpha(Color32::WHITE, 0.07)), egui::StrokeKind::Inside);
            } else {
                glowkit_core::paint::soft_shadow(&pt, sheet, r, 40.0, vec2(0.0, -2.0), Color32::from_black_alpha(16));
                pt.rect(sheet, cr(r), fill, Stroke::new(1.0, Color32::from_black_alpha(14)), egui::StrokeKind::Outside);
            }
            let inner = panel.shrink2(vec2(20.0, 18.0));
            let mut y = inner.min.y;
            match &title {
                Some(s) => {
                    pt.text(pos2(inner.min.x, y), Align2::LEFT_TOP, s, FontId::proportional(14.0), t.foreground);
                    pt.text(
                        pos2(inner.min.x, y + 19.0),
                        Align2::LEFT_TOP,
                        "Pick up where you left off",
                        FontId::proportional(12.0),
                        t.muted_foreground,
                    );
                    y += 44.0;
                }
                None => {
                    bar(&pt, &t, Rect::from_min_size(pos2(inner.min.x + inner.width() * 0.1, y + 2.0), vec2(inner.width() * 0.8, 7.0)));
                    bar(&pt, &t, Rect::from_min_size(pos2(inner.min.x + inner.width() * 0.1, y + 14.0), vec2(inner.width() * 0.6, 7.0)));
                    y += 34.0;
                }
            }
            let gap = 10.0;
            let tw = (inner.width() - gap) / 2.0;
            let th = 44.0;
            for i in 0..4 {
                let r = Rect::from_min_size(
                    pos2(inner.min.x + (i % 2) as f32 * (tw + gap), y + (i / 2) as f32 * (th + gap)),
                    vec2(tw, th),
                );
                let well = if t.dark { with_alpha(Color32::WHITE, 0.05) } else { mix(t.muted, t.border, 0.2) };
                pt.rect_filled(r, cr(10.0), well);
                if let Some((ic, label)) = tiles.get(i) {
                    ic.paint(&pt, pos2(r.min.x + 18.0, r.center().y), 14.0, t.muted_foreground);
                    pt.text(pos2(r.min.x + 32.0, r.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.5), t.foreground);
                }
            }
        });
        trig
    }
}

// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct TipState {
    x: Tween,
    w: Tween,
    shown: bool,
    since: f64,
    vis_from: f32,
    last_vis: f32,
    active: Option<usize>,
}

/// A row of pill buttons sharing one tooltip: hover one and, after a short delay, the tooltip
/// fades and grows in above it; slide to a neighbour and the same tooltip glides over and
/// resizes to the new label instead of blinking out and in.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::TooltipBar::new(&[("Copy", "Copy link"), ("Share", "Share with team")]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct TooltipBar {
    items: Vec<(String, String)>,
    force: Option<Option<usize>>,
    delay: f32,
    in_dur: f32,
    out_dur: f32,
    move_dur: f32,
}

impl TooltipBar {
    /// Buttons as `(label, tooltip)` pairs.
    pub fn new(items: &[(&str, &str)]) -> Self {
        Self {
            items: items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            force: None,
            delay: 0.08,
            in_dur: 0.15,
            out_dur: 0.05,
            move_dur: 0.16,
        }
    }
    /// Drive the hovered button yourself (for demos and walkthroughs); overrides the pointer.
    pub fn hovered(mut self, i: Option<usize>) -> Self {
        self.force = Some(i);
        self
    }
    /// Delay before the tooltip shows, in seconds (default 0.08).
    pub fn delay(mut self, s: f32) -> Self {
        self.delay = s.max(0.0);
        self
    }
    /// Fade-in and fade-out durations in seconds (default 0.15 / 0.05).
    pub fn durations(mut self, show: f32, hide: f32) -> Self {
        self.in_dur = show.max(0.01);
        self.out_dur = hide.max(0.01);
        self
    }
    /// Seconds the tooltip takes to glide between buttons (default 0.16).
    pub fn move_duration(mut self, s: f32) -> Self {
        self.move_dur = s.max(0.01);
        self
    }
}

impl Widget for TooltipBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let font = FontId::proportional(13.0);
        let tip_font = FontId::proportional(12.5);
        let widths: Vec<f32> = self.items.iter().map(|(l, _)| super::common::text_width(ui.painter(), l, &font) + 26.0).collect();
        let gap = 6.0;
        let total = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
        let (rect, response) = ui.allocate_exact_size(vec2(total + 20.0, 32.0 + 46.0), Sense::hover());
        let row_y = rect.max.y - 32.0;
        let mut x = rect.center().x - total / 2.0;
        let mut btns = Vec::new();
        let mut real_hover = None;
        for (i, w) in widths.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x, row_y), vec2(*w, 32.0));
            let resp = ui.interact(r, response.id.with(("tip", i)), Sense::click());
            if resp.hovered() {
                real_hover = Some(i);
            }
            btns.push(r);
            x += w + gap;
        }
        let active = if real_hover.is_some() { real_hover } else { self.force.unwrap_or(None) };
        let now = ui.input(|i| i.time);
        let tip_w: Vec<f32> = self.items.iter().map(|(_, tip)| super::common::text_width(ui.painter(), tip, &tip_font) + 20.0).collect();
        let (md, delay, ind, outd) = (self.move_dur, self.delay, self.in_dur, self.out_dur);
        let st = glowkit_core::with_state(
            ui,
            response.id.with("tipstate"),
            || TipState { x: Tween::new(0.0), w: Tween::new(0.0), shown: false, since: -1e6, vis_from: 0.0, last_vis: 0.0, active: None },
            |s| {
                if let Some(i) = active {
                    let cx = btns[i].center().x;
                    if !s.shown || s.last_vis < 0.05 {
                        s.x.snap(cx);
                        s.w.snap(tip_w[i]);
                    } else {
                        s.x.retarget(cx, now, md, MotionCurve::SMOOTH_OUT);
                        s.w.retarget(tip_w[i], now, md, MotionCurve::SMOOTH_OUT);
                    }
                    s.active = Some(i);
                }
                let want = active.is_some();
                if want != s.shown {
                    s.shown = want;
                    s.since = now;
                    s.vis_from = s.last_vis;
                }
                let age = (now - s.since) as f32;
                let vis = if s.shown {
                    let k = MotionCurve::EASE_OUT.at((age - if s.vis_from > 0.05 { 0.0 } else { delay }) / ind);
                    s.vis_from + (1.0 - s.vis_from) * k
                } else {
                    s.vis_from * (1.0 - MotionCurve::EASE_OUT.at(age / outd))
                };
                s.last_vis = vis;
                *s
            },
        );
        let age = (now - st.since) as f32;
        if age < delay + ind + md + 0.05 || st.x.moving(now, md) {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter();
        for (i, r) in btns.iter().enumerate() {
            let on = active == Some(i);
            let fill = if on { shade(chip(&t), if t.dark { 0.05 } else { -0.05 }) } else { chip(&t) };
            painter.rect_filled(*r, cr(16.0), fill);
            painter.text(r.center(), Align2::CENTER_CENTER, &self.items[i].0, font.clone(), t.foreground);
        }
        let vis = st.last_vis;
        if vis > 0.003 {
            if let Some(i) = st.active {
                let cx = st.x.value(now, md, MotionCurve::SMOOTH_OUT);
                let w = st.w.value(now, md, MotionCurve::SMOOTH_OUT);
                let tip = Rect::from_center_size(pos2(cx, row_y - 22.0), vec2(w, 26.0));
                let s = 0.98 + 0.02 * vis;
                let label = self.items[i].1.clone();
                let tf = tip_font.clone();
                layer(ui, xf_around(pos2(tip.center().x, tip.max.y), s, Vec2::ZERO), vis, 0.0, |pt| {
                    let bg = if t.dark { Color32::from_gray(34) } else { Color32::WHITE };
                    material(pt, &t, tip, 8.0, bg);
                    let clip = pt.with_clip_rect(tip.shrink(2.0));
                    clip.text(tip.center(), Align2::CENTER_CENTER, label, tf, t.foreground);
                });
            }
        }
        response
    }
}

// ---------------------------------------------------------------------------------------------

/// A notification toast that rises into place: up 16 points from below, growing from 97% and
/// un-blurring on a long smooth curve; it leaves the way it came, a little quicker.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// ui.add(glowkit_interactions::SlideToast::new(true).title("Mara shared a file").message("Q3 roadmap.pdf"));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct SlideToast {
    open: bool,
    title: String,
    message: String,
    tone: usize,
    width: f32,
    open_dur: f32,
    close_dur: f32,
    distance: f32,
    from_top: bool,
}

impl SlideToast {
    /// A toast shown while `open` is true.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            title: "New message".into(),
            message: "You have a new message".into(),
            tone: 0,
            width: 300.0,
            open_dur: 0.35,
            close_dur: 0.25,
            distance: 16.0,
            from_top: false,
        }
    }
    /// Title line.
    pub fn title(mut self, s: impl Into<String>) -> Self {
        self.title = s.into();
        self
    }
    /// Message line.
    pub fn message(mut self, s: impl Into<String>) -> Self {
        self.message = s.into();
        self
    }
    /// Avatar color index.
    pub fn tone(mut self, i: usize) -> Self {
        self.tone = i;
        self
    }
    /// Width (default 300).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// Open and close durations in seconds (default 0.35 / 0.25).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// Travel distance in points (default 16).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// Drop in from above instead of rising from below.
    pub fn from_top(mut self, on: bool) -> Self {
        self.from_top = on;
        self
    }
}

impl Widget for SlideToast {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = vec2(self.width, 60.0);
        let (rect, response) = ui.allocate_exact_size(size + vec2(0.0, self.distance.abs() + 8.0), Sense::hover());
        let ph = phase(ui, response.id, self.open, self.open_dur, self.close_dur, MotionCurve::SMOOTH_OUT, MotionCurve::SMOOTH_OUT);
        let p = ph.p.clamp(0.0, 1.0);
        if p <= 0.002 || !ui.is_rect_visible(rect) {
            return response;
        }
        let card = Rect::from_center_size(rect.center(), size);
        let dir = if self.from_top { -1.0 } else { 1.0 };
        let xf = xf_around(card.center(), 0.97 + 0.03 * p, vec2(0.0, dir * self.distance * (1.0 - p)));
        layer(ui, xf, p, 2.0 * (1.0 - p), |pt| {
            paint_banner(pt, &t, card, &self.title, &self.message, self.tone, 1.0);
        });
        response
    }
}

fn paint_banner(pt: &egui::Painter, t: &Theme, card: Rect, title: &str, msg: &str, tone: usize, content: f32) {
    let r = card.height() / 2.0;
    material(pt, t, card, r.min(18.0), surface(t));
    let mut p2 = pt.clone();
    p2.multiply_opacity(content);
    let ac = pos2(card.min.x + 30.0, card.center().y);
    avatar(&p2, ac, 16.0, AVATAR_TONES[tone % AVATAR_TONES.len()], None);
    two_lines(&p2, t, card.min.x + 56.0, card.center().y, title, msg, card.width() - 90.0);
    p2.text(pos2(card.max.x - 18.0, card.min.y + 17.0), Align2::RIGHT_CENTER, "now", FontId::proportional(11.0), t.muted_foreground);
}

// ---------------------------------------------------------------------------------------------

/// One banner in a [`BannerStack`].
#[derive(Clone, Debug, PartialEq)]
pub struct StackBanner {
    /// Unique id (new ids animate in).
    pub id: u64,
    /// Title line.
    pub title: String,
    /// Message line.
    pub message: String,
}

impl StackBanner {
    /// A banner with a unique `id`.
    pub fn new(id: u64, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { id, title: title.into(), message: message.into() }
    }
}

#[derive(Clone, Copy)]
struct BannerAnim {
    id: u64,
    depth: Tween,
    born: f64,
    gone: Option<f64>,
}

#[derive(Clone, Default)]
struct StackState {
    anims: Vec<BannerAnim>,
    cache: Vec<StackBanner>,
    spread: Option<Tween>,
}

/// Notification banners that stack: a new one rises in at the front and the others step back,
/// each a little smaller, dimmer and softer, peeking out above. Hover the stack and it fans out
/// into a list.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// let banners = vec![glowkit_interactions::StackBanner::new(1, "Deploy finished", "main · 42s")];
/// ui.add(glowkit_interactions::BannerStack::new(&banners));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct BannerStack<'a> {
    banners: &'a [StackBanner],
    width: f32,
    visible: usize,
    open_dur: f32,
    close_dur: f32,
    distance: f32,
    peek: f32,
    spread: Option<bool>,
}

impl<'a> BannerStack<'a> {
    /// Stack `banners` (last = newest, shown in front).
    pub fn new(banners: &'a [StackBanner]) -> Self {
        Self { banners, width: 300.0, visible: 3, open_dur: 0.35, close_dur: 0.25, distance: 60.0, peek: 12.0, spread: None }
    }
    /// Banner width (default 300).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w.max(120.0);
        self
    }
    /// How many banners stay visible (default 3).
    pub fn visible(mut self, n: usize) -> Self {
        self.visible = n.max(1);
        self
    }
    /// Enter and leave durations in seconds (default 0.35 / 0.25).
    pub fn durations(mut self, open: f32, close: f32) -> Self {
        self.open_dur = open.max(0.01);
        self.close_dur = close.max(0.01);
        self
    }
    /// How far a new banner rises from (default 60).
    pub fn distance(mut self, d: f32) -> Self {
        self.distance = d;
        self
    }
    /// How much each banner behind peeks out (default 12).
    pub fn peek(mut self, d: f32) -> Self {
        self.peek = d;
        self
    }
    /// Force the fanned-out list on or off (default: follows hover).
    pub fn spread(mut self, on: bool) -> Self {
        self.spread = Some(on);
        self
    }
}

impl Widget for BannerStack<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let bh = 60.0;
        let gap = 8.0;
        let vis = self.visible;
        let size = vec2(self.width + 24.0, bh * vis as f32 + gap * (vis - 1) as f32 + 24.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let now = ui.input(|i| i.time);
        let front = Rect::from_min_size(pos2(rect.center().x - self.width / 2.0, rect.max.y - 12.0 - bh), vec2(self.width, bh));
        let stack_rect = Rect::from_min_max(pos2(front.min.x, front.min.y - self.peek * 2.0), front.max);
        let hovering = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| stack_rect.contains(p));
        let spread_on = self.spread.unwrap_or(hovering);
        let n = self.banners.len();
        let (od, cd) = (self.open_dur, self.close_dur);
        let banners = self.banners.to_vec();
        let st = glowkit_core::with_state(ui, response.id.with("stack"), StackState::default, |s| {
            for (i, b) in banners.iter().enumerate() {
                let depth = (n - 1 - i) as f32;
                if let Some(a) = s.anims.iter_mut().find(|a| a.id == b.id) {
                    a.depth.retarget(depth, now, od, MotionCurve::SMOOTH_OUT);
                    a.gone = None;
                } else {
                    let fresh = s.cache.is_empty() && s.anims.is_empty();
                    let mut d = Tween::new(depth);
                    if fresh {
                        d.since = -1e6;
                    }
                    s.anims.push(BannerAnim { id: b.id, depth: d, born: if fresh { -1e6 } else { now }, gone: None });
                }
            }
            for a in s.anims.iter_mut() {
                if !banners.iter().any(|b| b.id == a.id) && a.gone.is_none() {
                    a.gone = Some(now);
                }
            }
            s.anims.retain(|a| a.gone.is_none_or(|g| now - g < cd as f64 + 0.05));
            for b in &banners {
                if !s.cache.iter().any(|c| c.id == b.id) {
                    s.cache.push(b.clone());
                }
            }
            let ids: Vec<u64> = s.anims.iter().map(|a| a.id).collect();
            s.cache.retain(|c| ids.contains(&c.id));
            let sp = s.spread.get_or_insert(Tween::new(spread_on as u8 as f32));
            sp.retarget(spread_on as u8 as f32, now, od, MotionCurve::SMOOTH_OUT);
            s.clone()
        });
        let busy = st.anims.iter().any(|a| a.depth.moving(now, od) || now - a.born < od as f64 || a.gone.is_some())
            || st.spread.is_some_and(|s| s.moving(now, od));
        if busy {
            ui.ctx().request_repaint();
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let spread = st.spread.map_or(0.0, |s| s.value(now, od, MotionCurve::SMOOTH_OUT));
        // paint back to front
        let mut order: Vec<&BannerAnim> = st.anims.iter().collect();
        order.sort_by(|a, b| {
            b.depth.value(now, od, MotionCurve::SMOOTH_OUT).partial_cmp(&a.depth.value(now, od, MotionCurve::SMOOTH_OUT)).unwrap_or(std::cmp::Ordering::Equal)
        });
        for a in order {
            let Some(b) = st.cache.iter().find(|c| c.id == a.id) else { continue };
            let d = a.depth.value(now, od, MotionCurve::SMOOTH_OUT);
            let enter = MotionCurve::SMOOTH_OUT.at(((now - a.born) as f32) / od);
            let leave = a.gone.map_or(0.0, |g| MotionCurve::SMOOTH_OUT.at(((now - g) as f32) / cd));
            let over = (d - (vis as f32 - 1.0)).max(0.0); // beyond the visible stack
            let stack_y = -self.peek * d.min(vis as f32);
            let spread_y = -(bh + gap) * d;
            let y = super::common::lerp(stack_y, spread_y, spread) + self.distance * (1.0 - enter);
            let scale_stack = 1.0 - 0.06 * d.min(vis as f32);
            let scale = super::common::lerp(scale_stack, 1.0, spread) * (0.97 + 0.03 * enter);
            let fade_stack = (1.0 - 0.25 * d).max(0.0);
            let alpha = super::common::lerp(fade_stack, 1.0, spread) * enter * (1.0 - leave) * (1.0 - over.min(1.0));
            let content = super::common::lerp((1.0 - 0.4 * d).max(0.0), 1.0, spread);
            let blur = super::common::lerp(d.min(2.0), 0.0, spread) + 2.0 * (1.0 - enter);
            let card = front;
            let tone = (b.id as usize) % AVATAR_TONES.len();
            layer(ui, xf_around(pos2(card.center().x, card.min.y), scale, vec2(0.0, y)), alpha, blur, |pt| {
                paint_banner(pt, &t, card, &b.title, &b.message, tone, content);
            });
        }
        response
    }
}
