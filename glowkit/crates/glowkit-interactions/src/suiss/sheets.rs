//! Bottom sheets: a confirmation prompt and an action bar that folds into a drawer on phones.

use super::common::*;
use egui::{pos2, vec2, Align2, Color32, Rect, Response, Sense, Ui, Vec2};
use glowkit_core::anim::lerp;
use glowkit_core::color::{mix, with_alpha};
use glowkit_core::Theme;

/// How [`ConfirmSheet`] presents itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ConfirmStyle {
    /// A sheet on narrow areas, an alert dialog on wide ones.
    #[default]
    Auto,
    /// Always an action sheet sliding up from the bottom.
    Sheet,
    /// Always a centered alert dialog.
    Alert,
}

#[derive(Clone, Default)]
struct SheetState {
    open: bool,
    pos: f32,
    pos_v: f32,
    drag: f32,
    grabbed: bool,
    press: f32,
    press_v: f32,
    pressed_btn: Option<u8>,
    result: Option<(String, f32)>,
    g: Gesture,
    cycle: u32,
}

/// Paint a pill button with a press squish; returns its rect.
fn pill(
    painter: &egui::Painter,
    t: &Theme,
    c: egui::Pos2,
    label: &str,
    squish: f32,
    fill: Color32,
    fg: Color32,
) -> Rect {
    let w = text_width(painter, label, 14.0) + 36.0;
    let r = Rect::from_center_size(c, vec2(w, 38.0) * (1.0 - 0.05 * squish));
    painter.rect_filled(r, cr(19.0), fill);
    if !t.dark && fill == Color32::WHITE {
        painter.rect_stroke(r, cr(19.0), egui::Stroke::new(1.0, hairline(t)), egui::StrokeKind::Inside);
    }
    text(painter, r.center(), Align2::CENTER_CENTER, label, 14.0, fg);
    r
}

/// A confirmation prompt that rises as an action sheet on phones and pops as an alert on wide
/// screens.
///
/// Press the trigger and the screen dims while the sheet springs up from the bottom: the title,
/// description and destructive action share one rounded group, with **Cancel** on its own card
/// below. The sheet is physical: drag it down and it follows (let go past a threshold, or flick,
/// to dismiss), push it up and it resists like rubber. On wide areas it shows as an alert that
/// scales in from slightly large. Buttons squish when pressed.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::{ConfirmSheet, ConfirmStyle};
/// ui.add(ConfirmSheet::new("Delete order", "Delete this order?").description("This can't be undone.").style(ConfirmStyle::Sheet));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ConfirmSheet {
    trigger: String,
    title: String,
    description: String,
    action: String,
    size: Option<Vec2>,
    style: ConfirmStyle,
    bounce: f32,
    autoplay: bool,
}

impl ConfirmSheet {
    /// A prompt opened by a button labelled `trigger`, asking `title`.
    pub fn new(trigger: impl Into<String>, title: impl Into<String>) -> Self {
        let trigger = trigger.into();
        Self {
            action: trigger.clone(),
            trigger,
            title: title.into(),
            description: String::new(),
            size: None,
            style: ConfirmStyle::Auto,
            bounce: 25.0,
            autoplay: false,
        }
    }
    /// Explanatory text under the title.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    /// Label of the destructive action (default: the trigger label).
    pub fn action(mut self, a: impl Into<String>) -> Self {
        self.action = a.into();
        self
    }
    /// Area the sheet lives in (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Sheet, alert, or pick by width.
    pub fn style(mut self, s: ConfirmStyle) -> Self {
        self.style = s;
        self
    }
    /// Overshoot of the sheet, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Open, drag and confirm on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const CONFIRM_SCRIPT: [Stroke2; 5] = [
    tap(0.8, 0.5, 0.45),
    drag(2.2, 2.6, (0.5, 0.76), (0.5, 0.6)),
    drag(3.5, 3.9, (0.5, 0.76), (0.5, 0.99)),
    tap(5.0, 0.5, 0.45),
    tap(6.4, 0.5, 0.818),
];

impl egui::Widget for ConfirmSheet {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let alert = match self.style {
            ConfirmStyle::Alert => true,
            ConfirmStyle::Sheet => false,
            ConfirmStyle::Auto => rect.width() > 480.0,
        };
        let touch = touch(ui, &response, rect, self.autoplay && !alert, &CONFIRM_SCRIPT, 9.0);
        let mut st: SheetState = glowkit_core::state(ui, id.with("confirm"), SheetState::default);
        if touch.ghost && touch.cycle != st.cycle {
            st = SheetState { cycle: touch.cycle, ..Default::default() };
        }
        let painter = ui.painter_at(rect);
        let trig_c = pos2(rect.center().x, rect.min.y + rect.height() * 0.45);
        let trig = Rect::from_center_size(trig_c, vec2(text_width(&painter, &self.trigger, 14.0) + 36.0, 38.0));

        // sheet geometry
        let m = 10.0;
        let cancel_h = 52.0;
        let wrap = rect.width() - 2.0 * m - 32.0;
        let desc_g =
            painter.layout(self.description.clone(), egui::FontId::proportional(12.5), t.muted_foreground, wrap);
        let head_h = 30.0 + 22.0 + if self.description.is_empty() { 0.0 } else { desc_g.size().y + 4.0 };
        let group_h = head_h + 52.0;
        let sheet_h = group_h + 8.0 + cancel_h + m;
        let cancel = Rect::from_min_size(
            pos2(rect.min.x + m, rect.max.y - m - cancel_h),
            vec2(rect.width() - 2.0 * m, cancel_h),
        );
        let group = Rect::from_min_size(
            pos2(rect.min.x + m, cancel.min.y - 8.0 - group_h),
            vec2(rect.width() - 2.0 * m, group_h),
        );
        let action_r = Rect::from_min_max(pos2(group.min.x, group.max.y - 52.0), group.max);
        let alert_r = Rect::from_center_size(rect.center(), vec2(280.0_f32.min(rect.width() - 40.0), head_h + 56.0));
        let alert_btns = |k: u8| {
            let half = alert_r.width() / 2.0;
            Rect::from_min_size(pos2(alert_r.min.x + half * k as f32, alert_r.max.y - 46.0), vec2(half, 46.0))
        };
        let hidden = sheet_h + 20.0;
        let off = (1.0 - st.pos) * hidden + st.drag;

        let g = st.g.update(&touch, now, dt);
        if g.pressed {
            let p = st.g.start;
            st.pressed_btn = None;
            if !st.open && trig.contains(p) {
                st.pressed_btn = Some(0);
            } else if st.open {
                st.grabbed = !alert && p.y > group.min.y + off - 10.0;
                if alert {
                    if alert_btns(0).contains(p) {
                        st.pressed_btn = Some(2);
                    } else if alert_btns(1).contains(p) {
                        st.pressed_btn = Some(1);
                    }
                } else if action_r.translate(vec2(0.0, off)).contains(p) {
                    st.pressed_btn = Some(1);
                } else if cancel.translate(vec2(0.0, off)).contains(p) {
                    st.pressed_btn = Some(2);
                }
            }
        }
        if st.grabbed && st.g.down && !g.pressed {
            let d = st.g.last.y - st.g.start.y;
            st.drag = if d > 0.0 { d } else { rubber(d, 24.0) };
            if st.g.moved > 8.0 {
                st.pressed_btn = None;
            }
        }
        if g.released {
            if st.grabbed && (st.drag > 80.0 || st.g.vel.y > 650.0) {
                st.open = false;
                st.pos -= st.drag / hidden;
                st.pos_v = -st.g.vel.y.max(0.0) / hidden;
                st.drag = 0.0;
            } else if g.tap {
                let p = st.g.last;
                match st.pressed_btn {
                    Some(0) => {
                        st.open = true;
                        st.result = None;
                    }
                    Some(1) => {
                        st.open = false;
                        st.result = Some((format!("{} confirmed", self.action), now));
                        response.mark_changed();
                    }
                    Some(2) => {
                        st.open = false;
                        st.result = Some(("Cancelled".into(), now));
                    }
                    _ => {
                        let inside = if alert { alert_r.contains(p) } else { p.y > group.min.y + off };
                        if st.open && !inside {
                            st.open = false;
                        }
                    }
                }
            }
            st.grabbed = false;
            st.pressed_btn = None;
        }
        let target = if st.open { 1.0 } else { 0.0 };
        let stiff = if alert { 420.0 } else { 260.0 };
        spring_step(
            &mut st.pos,
            &mut st.pos_v,
            target,
            stiff,
            damping_for(if st.open { self.bounce } else { 0.0 }, stiff),
            dt,
        );
        if !st.grabbed {
            spring_step(&mut st.drag, &mut st.press_v, 0.0, 300.0, damping_for(self.bounce, 300.0), dt);
        }
        st.press =
            if st.pressed_btn.is_some() { (st.press + dt * 10.0).min(1.0) } else { (st.press - dt * 8.0).max(0.0) };
        let moving = st.g.down || !settled(st.pos, st.pos_v, target) || st.drag.abs() > 0.2 || st.press > 0.0;
        let s = st.clone();
        glowkit_core::set_state(ui, id.with("confirm"), st);
        if moving || touch.ghost {
            ui.ctx().request_repaint();
        }

        // paint
        let red = if t.dark { hexa(0xFF, 0x45, 0x3A) } else { hexa(0xE5, 0x30, 0x2A) };
        let trig_fill = if t.dark { mix(t.muted, t.foreground, 0.04) } else { Color32::WHITE };
        pill(
            &painter,
            &t,
            trig_c,
            &self.trigger,
            if s.pressed_btn == Some(0) { s.press } else { 0.0 },
            trig_fill,
            t.foreground,
        );
        if let Some((msg, t0)) = &s.result {
            let a = ramp(now - t0, 0.2, 0.5) * (1.0 - ramp(now - t0, 2.5, 3.0));
            text(
                &painter,
                trig_c + vec2(0.0, 36.0),
                Align2::CENTER_CENTER,
                msg,
                12.0,
                with_alpha(t.muted_foreground, a),
            );
        }
        let k = s.pos.max(0.0);
        if k > 0.001 {
            let dim = (k.min(1.0) * (1.0 - (s.drag.max(0.0) / hidden).min(1.0))) * 0.35;
            painter.rect_filled(rect, cr(screen_radius(ui)), with_alpha(Color32::BLACK, dim));
            let sheet_fill = if t.dark { hexa(0x2C, 0x2C, 0x31) } else { Color32::WHITE };
            let sep = with_alpha(t.foreground, 0.1);
            if alert {
                let sc = lerp(1.12, 1.0, k.min(1.0));
                let r = Rect::from_center_size(alert_r.center(), alert_r.size() * sc);
                let a = k.min(1.0);
                shadow(&painter, &t, r, 16.0, a);
                painter.rect_filled(r, cr(16.0), with_alpha(sheet_fill, a));
                text_bold(
                    &painter,
                    pos2(r.center().x, r.min.y + 26.0),
                    Align2::CENTER_CENTER,
                    &self.title,
                    15.0,
                    with_alpha(t.foreground, a),
                );
                let g2 = painter.layout(
                    self.description.clone(),
                    egui::FontId::proportional(12.5),
                    with_alpha(t.muted_foreground, a),
                    r.width() - 32.0,
                );
                painter.galley(pos2(r.center().x - g2.size().x / 2.0, r.min.y + 42.0), g2, t.muted_foreground);
                let by = r.max.y - 46.0 * sc;
                painter.line_segment([pos2(r.min.x, by), pos2(r.max.x, by)], egui::Stroke::new(1.0, sep));
                painter
                    .line_segment([pos2(r.center().x, by), pos2(r.center().x, r.max.y)], egui::Stroke::new(1.0, sep));
                text(
                    &painter,
                    pos2(r.min.x + r.width() * 0.25, by + 23.0 * sc),
                    Align2::CENTER_CENTER,
                    "Cancel",
                    14.0,
                    with_alpha(system_blue(&t), a),
                );
                text_bold(
                    &painter,
                    pos2(r.min.x + r.width() * 0.75, by + 23.0 * sc),
                    Align2::CENTER_CENTER,
                    &self.action,
                    14.0,
                    with_alpha(red, a),
                );
            } else {
                let off = (1.0 - s.pos) * hidden + s.drag;
                let stretch = (-s.drag).max(0.0);
                let gr = Rect::from_min_max(group.min + vec2(0.0, off - stretch), group.max + vec2(0.0, off));
                let cr_ = cancel.translate(vec2(0.0, off));
                shadow(&painter, &t, gr, 14.0, 0.8);
                painter.rect_filled(gr, cr(14.0), sheet_fill);
                text_bold(
                    &painter,
                    pos2(gr.center().x, gr.min.y + 24.0),
                    Align2::CENTER_CENTER,
                    &self.title,
                    13.5,
                    t.muted_foreground,
                );
                if !self.description.is_empty() {
                    let g2 = painter.layout(
                        self.description.clone(),
                        egui::FontId::proportional(12.5),
                        t.muted_foreground,
                        wrap,
                    );
                    painter.galley(pos2(gr.center().x - g2.size().x / 2.0, gr.min.y + 40.0), g2, t.muted_foreground);
                }
                let ar = Rect::from_min_max(pos2(gr.min.x, gr.max.y - 52.0), gr.max);
                if s.pressed_btn == Some(1) {
                    painter.rect_filled(
                        ar,
                        egui::CornerRadius { nw: 0, ne: 0, sw: 14, se: 14 },
                        mix(sheet_fill, t.foreground, 0.08 * s.press),
                    );
                }
                painter.line_segment([pos2(ar.min.x, ar.min.y), pos2(ar.max.x, ar.min.y)], egui::Stroke::new(1.0, sep));
                text(&painter, ar.center(), Align2::CENTER_CENTER, &self.action, 17.0, red);
                let cfill =
                    if s.pressed_btn == Some(2) { mix(sheet_fill, t.foreground, 0.08 * s.press) } else { sheet_fill };
                shadow(&painter, &t, cr_, 14.0, 0.6);
                painter.rect_filled(cr_, cr(14.0), cfill);
                text_bold(&painter, cr_.center(), Align2::CENTER_CENTER, "Cancel", 16.5, system_blue(&t));
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}

// ---------------------------------------------------------------------------------------------
// Action bar

#[derive(Clone, Default)]
struct BarState {
    open: bool,
    pos: f32,
    pos_v: f32,
    drag: f32,
    drag_v: f32,
    grabbed: bool,
    menu: Option<usize>,
    last_menu: usize,
    page: f32,
    page_v: f32,
    hover_menu: Option<usize>,
    drop: f32,
    drop_v: f32,
    drop_x: f32,
    drop_xv: f32,
    flash: Option<(String, f32)>,
    g: Gesture,
    cycle: u32,
}

/// A row of command menus that is a menubar on wide areas and a single "more" drawer on phones.
///
/// Wide: File / Edit / View sit in a bar; click one to drop its menu (it unfolds on a spring),
/// and while a menu is open, hovering another trigger slides the menu across to it. Narrow: the
/// bar collapses into a round "⋯" button that pulls up a drawer with a grabber and one row per
/// menu. Choosing a row slides the drawer sideways into that menu's items (with a back row),
/// and the drawer can be dragged down and flicked away, rubber-banding when pushed up.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_interactions::ActionBarSheet;
/// ui.add(ActionBarSheet::new()
///     .menu("File", &["New", "Open…", "Save"])
///     .menu("Edit", &["Undo", "Copy", "Paste"]));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct ActionBarSheet {
    menus: Vec<(String, Vec<String>)>,
    size: Option<Vec2>,
    title: String,
    bounce: f32,
    force_mobile: Option<bool>,
    autoplay: bool,
}

impl Default for ActionBarSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionBarSheet {
    /// An action bar with no menus yet (add them with [`Self::menu`]; empty shows File/Edit/View).
    pub fn new() -> Self {
        Self {
            menus: Vec::new(),
            size: None,
            title: "Untitled draft".into(),
            bounce: 25.0,
            force_mobile: None,
            autoplay: false,
        }
    }
    /// Add a menu with its items.
    pub fn menu(mut self, name: impl Into<String>, items: &[&str]) -> Self {
        self.menus.push((name.into(), items.iter().map(|s| s.to_string()).collect()));
        self
    }
    /// Area the bar lives in (default: all available space).
    pub fn size(mut self, s: Vec2) -> Self {
        self.size = Some(s);
        self
    }
    /// Document title shown next to the bar.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// Overshoot of the drawer and menus, 0..100.
    pub fn bounce(mut self, b: f32) -> Self {
        self.bounce = b.clamp(0.0, 100.0);
        self
    }
    /// Force the phone drawer (`true`) or the menubar (`false`) instead of picking by width.
    pub fn mobile(mut self, on: bool) -> Self {
        self.force_mobile = Some(on);
        self
    }
    /// Open the drawer and browse menus on its own while idle.
    pub fn autoplay(mut self, on: bool) -> Self {
        self.autoplay = on;
        self
    }
}

const BAR_SCRIPT: [Stroke2; 5] =
    [tap(0.8, 0.9, 0.087), tap(2.0, 0.5, 0.614), tap(3.3, 0.5, 0.523), tap(4.4, 0.5, 0.705), tap(5.6, 0.5, 0.705)];

impl egui::Widget for ActionBarSheet {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let size = self.size.unwrap_or(ui.available_size());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let id = response.id;
        let dt = glowkit_core::dt(ui);
        let now = glowkit_core::time(ui);
        let menus: Vec<(String, Vec<String>)> = if self.menus.is_empty() {
            vec![
                ("File".into(), ["New", "Open…", "Duplicate", "Export PDF"].iter().map(|s| s.to_string()).collect()),
                ("Edit".into(), ["Undo", "Cut", "Copy", "Paste"].iter().map(|s| s.to_string()).collect()),
                ("View".into(), ["Zoom In", "Zoom Out", "Show Outline"].iter().map(|s| s.to_string()).collect()),
            ]
        } else {
            self.menus.clone()
        };
        let mobile = self.force_mobile.unwrap_or(rect.width() < 480.0);
        let touch = touch(ui, &response, rect, self.autoplay && mobile, &BAR_SCRIPT, 7.5);
        let mut st: BarState = glowkit_core::state(ui, id.with("abar"), BarState::default);
        if touch.ghost && touch.cycle != st.cycle {
            st = BarState { cycle: touch.cycle, ..Default::default() };
        }
        let painter = ui.painter_at(rect);
        let g = st.g.update(&touch, now, dt);
        let stiff = 280.0;
        let damp = damping_for(self.bounce, stiff);
        let shortcut = |name: &str| -> &'static str {
            match name {
                "New" => "⌘N",
                "Open…" => "⌘O",
                "Undo" => "⌘Z",
                "Cut" => "⌘X",
                "Copy" => "⌘C",
                "Paste" => "⌘V",
                "Zoom In" => "⌘+",
                "Zoom Out" => "⌘-",
                _ => "",
            }
        };
        let red = if t.dark { hexa(0xFF, 0x45, 0x3A) } else { hexa(0xE5, 0x30, 0x2A) };
        if mobile {
            let dots = pos2(rect.max.x - 26.0, rect.min.y + 46.0);
            let row_h = 48.0;
            let rows_n = menus.iter().map(|m| m.1.len() + 1).max().unwrap_or(1).max(menus.len());
            let sheet_h = 56.0 + rows_n as f32 * row_h + 20.0;
            let sheet = Rect::from_min_max(pos2(rect.min.x, rect.max.y - sheet_h), rect.max);
            let hidden = sheet_h + 10.0;
            let off = (1.0 - st.pos) * hidden + st.drag;
            let sh = sheet.translate(vec2(0.0, off));
            let row_r = |i: usize| {
                Rect::from_min_size(
                    pos2(sh.min.x + 12.0, sh.min.y + 40.0 + i as f32 * row_h),
                    vec2(sh.width() - 24.0, row_h),
                )
            };
            if g.pressed {
                st.grabbed = st.open && sh.contains(st.g.start);
            }
            if st.grabbed && st.g.down && !g.pressed {
                let d = st.g.last.y - st.g.start.y;
                st.drag = if d > 0.0 { d } else { rubber(d, 26.0) };
            }
            if g.released {
                if st.grabbed && (st.drag > 70.0 || st.g.vel.y > 600.0) {
                    st.open = false;
                    st.pos -= st.drag / hidden;
                    st.pos_v = -st.g.vel.y.max(0.0) / hidden;
                    st.drag = 0.0;
                } else if g.tap {
                    let p = st.g.last;
                    if !st.open && (p - dots).length() < 20.0 {
                        st.open = true;
                        st.menu = None;
                        st.page = 0.0;
                    } else if st.open && !sh.contains(p) {
                        st.open = false;
                    } else if st.open {
                        if let Some(i) = (0..rows_n).find(|&i| row_r(i).contains(p)) {
                            match st.menu {
                                None if i < menus.len() => {
                                    st.menu = Some(i);
                                    st.last_menu = i;
                                }
                                Some(_) if i == 0 => st.menu = None,
                                Some(m) if i <= menus[m].1.len() => {
                                    st.flash = Some((menus[m].1[i - 1].clone(), now));
                                    st.open = false;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                st.grabbed = false;
            }
            let target = if st.open { 1.0 } else { 0.0 };
            spring_step(
                &mut st.pos,
                &mut st.pos_v,
                target,
                stiff,
                damping_for(if st.open { self.bounce } else { 0.0 }, stiff),
                dt,
            );
            if !st.grabbed {
                spring_step(&mut st.drag, &mut st.drag_v, 0.0, 300.0, damp, dt);
            }
            let pt = if st.menu.is_some() { 1.0 } else { 0.0 };
            spring_step(&mut st.page, &mut st.page_v, pt, 320.0, damping_for(self.bounce * 0.6, 320.0), dt);
            let moving = st.g.down
                || !settled(st.pos, st.pos_v, target)
                || st.drag.abs() > 0.2
                || !settled(st.page, st.page_v, pt);
            let s = st.clone();
            glowkit_core::set_state(ui, id.with("abar"), st);
            if moving || touch.ghost || s.flash.is_some() {
                ui.ctx().request_repaint();
            }

            // document behind
            text_bold(
                &painter,
                pos2(rect.min.x + 18.0, rect.min.y + 46.0),
                Align2::LEFT_CENTER,
                &self.title,
                20.0,
                t.foreground,
            );
            painter.circle_filled(dots, 16.0, recess(&t));
            for dx in [-6.0, 0.0, 6.0] {
                painter.circle_filled(dots + vec2(dx, 0.0), 1.8, t.foreground);
            }
            for i in 0..9 {
                let w = [0.92, 0.85, 0.95, 0.6, 0.9, 0.88, 0.97, 0.7, 0.4][i];
                let y = rect.min.y + 84.0 + i as f32 * 20.0 + if i > 3 { 16.0 } else { 0.0 };
                painter.rect_filled(
                    Rect::from_min_size(pos2(rect.min.x + 18.0, y), vec2((rect.width() - 36.0) * w, 8.0)),
                    cr(4.0),
                    recess(&t),
                );
            }
            if let Some((item, t0)) = &s.flash {
                let a = ramp(now - t0, 0.2, 0.4) * (1.0 - ramp(now - t0, 1.6, 2.0));
                if a > 0.0 {
                    let r = Rect::from_center_size(
                        pos2(rect.center().x, rect.min.y + 300.0),
                        vec2(text_width(&painter, item, 13.0) + 60.0, 34.0),
                    );
                    painter.rect_filled(
                        r,
                        cr(17.0),
                        with_alpha(if t.dark { hexa(0x3A, 0x3A, 0x40) } else { hexa(0x1C, 0x1C, 0x1F) }, a),
                    );
                    text(
                        &painter,
                        r.center(),
                        Align2::CENTER_CENTER,
                        format!("✓  {item}"),
                        13.0,
                        with_alpha(Color32::WHITE, a),
                    );
                }
            }
            let k = s.pos.max(0.0);
            if k > 0.001 {
                painter.rect_filled(
                    rect,
                    cr(screen_radius(ui)),
                    with_alpha(Color32::BLACK, 0.3 * k.min(1.0) * (1.0 - (s.drag.max(0.0) / hidden).min(1.0))),
                );
                let off = (1.0 - s.pos) * hidden + s.drag;
                let stretch = (-s.drag).max(0.0);
                let sh = Rect::from_min_max(sheet.min + vec2(0.0, off - stretch), sheet.max + vec2(0.0, 40.0));
                let fill = if t.dark { hexa(0x24, 0x24, 0x29) } else { Color32::WHITE };
                shadow(&painter, &t, sh, 22.0, 1.0);
                painter.rect_filled(sh, egui::CornerRadius { nw: 22, ne: 22, sw: 0, se: 0 }, fill);
                painter.rect_filled(
                    Rect::from_center_size(pos2(sh.center().x, sh.min.y + 10.0), vec2(36.0, 5.0)),
                    cr(2.5),
                    with_alpha(t.muted_foreground, 0.4),
                );
                let clip = painter.with_clip_rect(sh);
                let page = s.page;
                let w = sh.width();
                let row_at = |i: usize, dx: f32| {
                    Rect::from_min_size(
                        pos2(sh.min.x + 12.0 + dx, sh.min.y + 40.0 + i as f32 * row_h),
                        vec2(w - 24.0, row_h),
                    )
                };
                // page 0: menus
                let dx0 = -page * w * 0.3;
                let a0 = (1.0 - page).clamp(0.0, 1.0);
                text_bold(
                    &clip,
                    pos2(sh.center().x + dx0, sh.min.y + 26.0),
                    Align2::CENTER_CENTER,
                    "More actions",
                    13.0,
                    with_alpha(t.muted_foreground, a0),
                );
                for (i, (name, items)) in menus.iter().enumerate() {
                    let r = row_at(i, dx0);
                    text(
                        &clip,
                        pos2(r.min.x + 8.0, r.center().y),
                        Align2::LEFT_CENTER,
                        name,
                        15.5,
                        with_alpha(t.foreground, a0),
                    );
                    text(
                        &clip,
                        pos2(r.max.x - 30.0, r.center().y),
                        Align2::RIGHT_CENTER,
                        format!("{}", items.len()),
                        12.5,
                        with_alpha(t.muted_foreground, a0),
                    );
                    glyph(
                        &clip,
                        Glyph::ChevronRight,
                        pos2(r.max.x - 12.0, r.center().y),
                        10.0,
                        with_alpha(t.muted_foreground, a0),
                    );
                    if i + 1 < menus.len() {
                        clip.line_segment(
                            [pos2(r.min.x + 8.0, r.max.y), pos2(r.max.x, r.max.y)],
                            egui::Stroke::new(1.0, with_alpha(hairline(&t), a0)),
                        );
                    }
                }
                // page 1: the menu's items
                if page > 0.01 {
                    let m = s.last_menu.min(menus.len() - 1);
                    let dx1 = (1.0 - page) * w;
                    let back = row_at(0, dx1);
                    glyph(&clip, Glyph::ChevronRight, pos2(back.min.x + 10.0, back.center().y), -10.0, system_blue(&t));
                    text(
                        &clip,
                        pos2(back.min.x + 22.0, back.center().y),
                        Align2::LEFT_CENTER,
                        "Menus",
                        15.0,
                        system_blue(&t),
                    );
                    text_bold(
                        &clip,
                        pos2(sh.center().x + dx1, sh.min.y + 26.0),
                        Align2::CENTER_CENTER,
                        &menus[m].0,
                        13.0,
                        t.muted_foreground,
                    );
                    for (j, item) in menus[m].1.iter().enumerate() {
                        let r = row_at(j + 1, dx1);
                        clip.line_segment(
                            [pos2(r.min.x + 8.0, r.min.y), pos2(r.max.x, r.min.y)],
                            egui::Stroke::new(1.0, hairline(&t)),
                        );
                        let destructive = item.starts_with("Delete");
                        text(
                            &clip,
                            pos2(r.min.x + 8.0, r.center().y),
                            Align2::LEFT_CENTER,
                            item,
                            15.5,
                            if destructive { red } else { t.foreground },
                        );
                        text(
                            &clip,
                            pos2(r.max.x - 8.0, r.center().y),
                            Align2::RIGHT_CENTER,
                            shortcut(item),
                            12.5,
                            t.muted_foreground,
                        );
                    }
                }
            }
        } else {
            // menubar
            let bar = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(rect.width() - 32.0, 36.0));
            let mut x = bar.min.x + 8.0;
            let mut triggers = Vec::new();
            for (name, _) in &menus {
                let w = text_width(&painter, name, 13.5) + 22.0;
                triggers.push(Rect::from_min_size(pos2(x, bar.min.y + 4.0), vec2(w, 28.0)));
                x += w + 2.0;
            }
            let hover = response.hover_pos();
            let hit = hover.and_then(|p| triggers.iter().position(|r| r.contains(p)));
            if response.clicked() {
                if let Some(i) = hit {
                    st.menu = if st.menu == Some(i) { None } else { Some(i) };
                } else if let (Some(m), Some(p)) = (st.menu, response.interact_pointer_pos()) {
                    let n = menus[m].1.len();
                    let drop =
                        Rect::from_min_size(pos2(st.drop_x, bar.max.y + 6.0), vec2(200.0, 12.0 + n as f32 * 32.0));
                    if let Some(j) = (0..n).find(|&j| {
                        Rect::from_min_size(
                            pos2(drop.min.x + 6.0, drop.min.y + 6.0 + j as f32 * 32.0),
                            vec2(188.0, 32.0),
                        )
                        .contains(p)
                    }) {
                        st.flash = Some((menus[m].1[j].clone(), now));
                    }
                    st.menu = None;
                }
            }
            if let (Some(_), Some(i)) = (st.menu, hit) {
                st.menu = Some(i);
            }
            st.hover_menu = hit;
            let dt_ = if st.menu.is_some() { 1.0 } else { 0.0 };
            spring_step(&mut st.drop, &mut st.drop_v, dt_, 380.0, damping_for(self.bounce, 380.0), dt);
            if let Some(m) = st.menu {
                st.last_menu = m;
                let tx = triggers[m].min.x;
                if st.drop < 0.05 {
                    st.drop_x = tx;
                }
                spring_step(&mut st.drop_x, &mut st.drop_xv, tx, 420.0, damping_for(self.bounce, 420.0), dt);
            }
            let moving = !settled(st.drop, st.drop_v, dt_) || st.drop_xv.abs() > 0.5;
            let s = st.clone();
            glowkit_core::set_state(ui, id.with("abar"), st);
            if moving || s.flash.is_some() {
                ui.ctx().request_repaint();
            }
            let fill = if t.dark { hexa(0x24, 0x24, 0x29) } else { Color32::WHITE };
            card(&painter, &t, bar, 10.0, 0.5, fill);
            for (i, r) in triggers.iter().enumerate() {
                if s.menu == Some(i) || s.hover_menu == Some(i) {
                    painter.rect_filled(*r, cr(7.0), recess(&t));
                }
                text(&painter, r.center(), Align2::CENTER_CENTER, &menus[i].0, 13.5, t.foreground);
            }
            text(
                &painter,
                pos2(bar.max.x - 12.0, bar.center().y),
                Align2::RIGHT_CENTER,
                &self.title,
                12.5,
                t.muted_foreground,
            );
            if s.drop > 0.01 {
                let m = s.last_menu.min(menus.len() - 1);
                let n = menus[m].1.len();
                let full = Rect::from_min_size(pos2(s.drop_x, bar.max.y + 6.0), vec2(200.0, 12.0 + n as f32 * 32.0));
                let k = s.drop.clamp(0.0, 1.1);
                let r = Rect::from_min_size(
                    full.min,
                    vec2(full.width(), full.height() * k.min(1.0) + (k - 1.0).max(0.0) * 20.0),
                );
                card(&painter, &t, r, 10.0, 1.0, fill);
                let clip = painter.with_clip_rect(r);
                for (j, item) in menus[m].1.iter().enumerate() {
                    let ir =
                        Rect::from_min_size(pos2(r.min.x + 6.0, r.min.y + 6.0 + j as f32 * 32.0), vec2(188.0, 32.0));
                    if hover.is_some_and(|p| ir.contains(p)) {
                        clip.rect_filled(ir, cr(6.0), system_blue(&t));
                        text(
                            &clip,
                            pos2(ir.min.x + 10.0, ir.center().y),
                            Align2::LEFT_CENTER,
                            item,
                            13.0,
                            Color32::WHITE,
                        );
                        text(
                            &clip,
                            pos2(ir.max.x - 10.0, ir.center().y),
                            Align2::RIGHT_CENTER,
                            shortcut(item),
                            12.0,
                            Color32::WHITE,
                        );
                    } else {
                        text(
                            &clip,
                            pos2(ir.min.x + 10.0, ir.center().y),
                            Align2::LEFT_CENTER,
                            item,
                            13.0,
                            t.foreground,
                        );
                        text(
                            &clip,
                            pos2(ir.max.x - 10.0, ir.center().y),
                            Align2::RIGHT_CENTER,
                            shortcut(item),
                            12.0,
                            t.muted_foreground,
                        );
                    }
                }
            }
        }
        ghost_touch(&painter, touch.pos.unwrap_or(rect.center()), if touch.down { 1.0 } else { 0.0 }, touch.alpha);
        response
    }
}
