//! Microphone picker with a live level meter and mute.

use crate::common;
use crate::icons::{paint_icon, Icon};
use crate::media;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Order, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Widget};
use glowkit_core::color::with_alpha;
use glowkit_core::Theme;

/// A microphone picker: a split control with a mute toggle (the mic icon gets a red slash when
/// muted), the current device name with a small live level meter, and a dropdown listing input
/// devices (the selected one shows a full segmented meter) plus a mute switch. The level is fed
/// in by the caller each frame (`0..=1`), so it works with any audio backend or a simulation.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_ai::MicSelector;
/// let devices = ["MacBook Pro Microphone", "Studio USB Mic"];
/// let (mut selected, mut muted) = (0, false);
/// ui.add(MicSelector::new(&devices, &mut selected, &mut muted).level(0.4));
/// ```
#[must_use = "add it with `ui.add(...)`"]
pub struct MicSelector<'a> {
    devices: &'a [&'a str],
    selected: &'a mut usize,
    muted: &'a mut bool,
    level: f32,
    width: f32,
    force_open: Option<bool>,
    id_salt: Option<egui::IdSalt>,
}

impl<'a> MicSelector<'a> {
    /// Choose among `devices`; `selected` indexes into it, `muted` is toggled by the mic button.
    pub fn new(devices: &'a [&'a str], selected: &'a mut usize, muted: &'a mut bool) -> Self {
        Self { devices, selected, muted, level: 0.0, width: 280.0, force_open: None, id_salt: None }
    }
    /// Current input level `0..=1` (ignored while muted).
    pub fn level(mut self, l: f32) -> Self {
        self.level = l.clamp(0.0, 1.0);
        self
    }
    /// Control width (default 280).
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    /// Force the dropdown open or closed (e.g. for previews).
    pub fn force_open(mut self, open: Option<bool>) -> Self {
        self.force_open = open;
        self
    }
    /// Stable id for the open state.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(salt));
        self
    }
}

/// Segmented level meter, `n` segments, green→amber→red at the top end.
fn meter(p: &Painter, r: Rect, level: f32, n: usize, t: &Theme, vertical_bars: bool) {
    let gap = 2.0;
    let seg = (r.width() + gap) / n as f32 - gap;
    for i in 0..n {
        let f = (i as f32 + 0.5) / n as f32;
        let on = f <= level;
        let c = if f > 0.85 {
            t.destructive
        } else if f > 0.65 {
            t.warning
        } else {
            t.success
        };
        let x = r.min.x + i as f32 * (seg + gap);
        let h = if vertical_bars { r.height() * (0.35 + 0.65 * f) } else { r.height() };
        let sr = Rect::from_min_max(pos2(x, r.max.y - h), pos2(x + seg, r.max.y));
        p.rect_filled(sr, CornerRadius::same(2), if on { c } else { with_alpha(t.muted_foreground, 0.18) });
    }
}

impl Widget for MicSelector<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::of(ui);
        let id = self.id_salt.map(|s| ui.scope_id().with_salt(s)).unwrap_or_else(|| ui.scope_id().with("glowkit_mic_selector"));
        if !self.devices.is_empty() {
            *self.selected = (*self.selected).min(self.devices.len() - 1);
        }
        // smooth the level for a meter-like ballistics feel
        let level = glowkit_core::anim::follow(ui, id.with("lvl"), if *self.muted { 0.0 } else { self.level }, 18.0);
        let (rect, mut resp) = ui.allocate_exact_size(vec2(self.width, 38.0), Sense::hover());
        let cr = CornerRadius::same((t.radius * 0.8) as u8);
        let p = ui.painter();
        if !t.dark {
            glowkit_core::paint::soft_shadow(p, rect, cr.nw as f32, 2.0, vec2(0.0, 1.0), Color32::from_black_alpha(10));
        }
        p.rect(rect, cr, t.card, Stroke::new(1.0, common::border_strong(&t)), StrokeKind::Inside);
        // mute half
        let mr = Rect::from_min_size(rect.min, vec2(40.0, rect.height()));
        let mresp = ui.interact(mr, id.with("mute"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let mresp = mresp.on_hover_text(if *self.muted { "Unmute" } else { "Mute" });
        if mresp.clicked() {
            *self.muted = !*self.muted;
            resp.mark_changed();
        }
        let mh = common::anim_bool(ui, id.with("mh"), mresp.hovered(), 0.1);
        let p = ui.painter();
        let mcr = CornerRadius { nw: cr.nw, sw: cr.sw, ne: 0, se: 0 };
        if *self.muted {
            p.rect_filled(mr.shrink(1.0), mcr, with_alpha(t.destructive, if t.dark { 0.14 } else { 0.08 }));
        }
        if mh > 0.0 {
            p.rect_filled(mr.shrink(1.0), mcr, common::hover_fill(&t).gamma_multiply(mh));
        }
        let ir = Rect::from_center_size(mr.center(), vec2(17.0, 17.0));
        if *self.muted {
            paint_icon(p, Icon::Mic, ir, t.destructive);
            media::slash(p, ir, t.destructive, t.card);
        } else {
            // glow behind the mic with the level
            if level > 0.02 {
                p.circle_filled(mr.center(), 9.0 + 7.0 * level, with_alpha(t.success, 0.18 * level));
            }
            paint_icon(p, Icon::Mic, ir, t.foreground);
        }
        p.line_segment([pos2(mr.max.x, rect.min.y + 1.0), pos2(mr.max.x, rect.max.y - 1.0)], Stroke::new(1.0, t.border));
        // device half
        let dr = Rect::from_min_max(pos2(mr.max.x, rect.min.y), rect.max);
        let dresp = ui.interact(dr, id.with("dev"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut open: bool = glowkit_core::state(ui, id.with("open"), || false);
        let toggled = dresp.clicked();
        if toggled {
            open = !open;
        }
        if let Some(f) = self.force_open {
            open = f;
        }
        let dh = common::anim_bool(ui, id.with("dh"), dresp.hovered() || open, 0.1);
        let p = ui.painter();
        if dh > 0.0 {
            p.rect_filled(dr.shrink(1.0), CornerRadius { nw: 0, sw: 0, ne: cr.ne, se: cr.se }, common::hover_fill(&t).gamma_multiply(dh));
        }
        let name = self.devices.get(*self.selected).copied().unwrap_or("No microphone");
        let meter_w = 26.0;
        let max_text = dr.width() - 12.0 - meter_w - 36.0;
        let mut label: String = name.to_owned();
        if common::galley(ui, label.clone(), 13.0, t.foreground).size().x > max_text {
            let chars: Vec<char> = name.chars().collect();
            let mut keep = chars.len();
            while keep > 1 {
                keep -= 1;
                label = chars[..keep].iter().collect::<String>().trim_end().to_owned() + "…";
                if common::galley(ui, label.clone(), 13.0, t.foreground).size().x <= max_text {
                    break;
                }
            }
        }
        common::text(p, pos2(dr.min.x + 12.0, dr.center().y), Align2::LEFT_CENTER, label, 13.0, if *self.muted { t.muted_foreground } else { t.foreground });
        let mrect = Rect::from_min_size(pos2(dr.max.x - 32.0 - meter_w, dr.center().y - 7.0), vec2(meter_w, 14.0));
        meter(p, mrect, level, 5, &t, true);
        let ot = common::anim_bool(ui, id.with("o"), open, 0.15);
        let cc = pos2(dr.max.x - 15.0, dr.center().y);
        let flip = 1.0 - 2.0 * ot;
        p.add(egui::Shape::line(vec![cc + vec2(-4.0, -2.0 * flip), cc + vec2(0.0, 2.0 * flip), cc + vec2(4.0, -2.0 * flip)], Stroke::new(1.5, t.muted_foreground)));

        if open || ot > 0.01 {
            let width = self.width.max(300.0);
            let slide = (1.0 - ot) * 6.0;
            let area = egui::Area::new(id.with("menu"))
                .order(Order::Foreground)
                .fade_in(false)
                .fixed_pos(pos2(rect.min.x, rect.max.y + 6.0 - slide))
                .interactable(open)
                .show(ui.ctx(), |ui| {
                    ui.multiply_opacity(ot);
                    media::popover_frame(&t).show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let (hr, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
                        common::text(ui.painter(), pos2(hr.min.x + 10.0, hr.center().y + 1.0), Align2::LEFT_CENTER, "Input device", 11.5, t.muted_foreground);
                        for (i, d) in self.devices.iter().enumerate() {
                            let sel = i == *self.selected;
                            let h = if sel { 52.0 } else { 36.0 };
                            let (r, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
                            let rr = ui.interact(r, id.with(("row", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                            let hh = common::anim_bool(ui, id.with(("rh", i)), rr.hovered(), 0.08);
                            let p = ui.painter();
                            if hh > 0.0 {
                                p.rect_filled(r, CornerRadius::same(8), common::hover_fill(&t).gamma_multiply(hh));
                            }
                            let cy = r.min.y + 18.0;
                            paint_icon(p, Icon::Mic, Rect::from_center_size(pos2(r.min.x + 20.0, cy), vec2(15.0, 15.0)), if sel { t.foreground } else { t.muted_foreground });
                            common::text(p, pos2(r.min.x + 38.0, cy), Align2::LEFT_CENTER, *d, 13.0, t.foreground);
                            if sel {
                                paint_icon(p, Icon::Check, Rect::from_center_size(pos2(r.max.x - 18.0, cy), vec2(14.0, 14.0)), t.foreground);
                                let mr = Rect::from_min_size(pos2(r.min.x + 38.0, r.min.y + 32.0), vec2(width - 76.0, 6.0));
                                meter(p, mr, level, 24, &t, false);
                            }
                            if rr.clicked() {
                                *self.selected = i;
                                open = false;
                                resp.mark_changed();
                            }
                        }
                        let (sr, _) = ui.allocate_exact_size(vec2(width, 9.0), Sense::hover());
                        ui.painter().line_segment([pos2(sr.min.x - 6.0, sr.center().y), pos2(sr.max.x + 6.0, sr.center().y)], Stroke::new(1.0, t.border));
                        // mute switch row
                        let (r, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
                        let rr = ui.interact(r, id.with("mute_row"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                        let hh = common::anim_bool(ui, id.with("mrh"), rr.hovered(), 0.08);
                        let p = ui.painter();
                        if hh > 0.0 {
                            p.rect_filled(r, CornerRadius::same(8), common::hover_fill(&t).gamma_multiply(hh));
                        }
                        common::text(p, pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, "Mute microphone", 13.0, t.foreground);
                        let on = common::anim_bool(ui, id.with("sw"), *self.muted, 0.16);
                        let sw = Rect::from_center_size(pos2(r.max.x - 28.0, r.center().y), vec2(32.0, 18.0));
                        let p = ui.painter();
                        p.rect_filled(sw, CornerRadius::same(9), with_alpha(t.muted_foreground, 0.3).lerp_to_gamma(t.destructive, on));
                        p.circle_filled(pos2(sw.min.x + 9.0 + 14.0 * on, sw.center().y), 7.0, Color32::WHITE);
                        if rr.clicked() {
                            *self.muted = !*self.muted;
                            resp.mark_changed();
                        }
                    });
                });
            open = media::close_on_outside(ui, open, toggled, &[area.response.rect, dr]);
        }
        if self.force_open.is_none() {
            glowkit_core::set_state(ui, id.with("open"), open);
        }
        if !*self.muted && self.level > 0.0 {
            glowkit_core::animate(ui);
        }
        resp
    }
}
