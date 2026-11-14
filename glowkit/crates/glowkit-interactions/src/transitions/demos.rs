use super::common::{chip, cr, idle_for, text_width};
use super::*;
use egui::{pos2, vec2, Align, Align2, Color32, FontId, Layout, Pos2, Rect, Sense, Shape, Stroke, Ui, UiBuilder, Vec2};
use glowkit_core::color::{mix, shade, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::{Rng, Theme};
use glowkit_effects::{ControlSlider, MicroIcon};

const MICRO: Category = Category::MicroInteractions;
const TEXT: Category = Category::TextAnimations;
const MOTION: Category = Category::Motion;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

// ---------------------------------------------------------------------------------------------
// The loop clock: every demo replays its script on a loop; "Replay" restarts it from zero.

#[derive(Clone, Copy)]
struct ClockState {
    start: f64,
    prev: f32,
    restart: bool,
}

/// Where a demo's looping script is.
#[derive(Clone, Copy)]
struct Script {
    /// Seconds into the current loop.
    t: f32,
    /// `t` on the previous frame (negative right after a (re)start).
    prev: f32,
    /// Loop length.
    period: f32,
    /// How many loops (or replays) have started.
    cycle: u64,
}

impl Script {
    /// Whether the script passed `mark` seconds this frame.
    fn hit(&self, mark: f32) -> bool {
        if self.t >= self.prev {
            self.prev < mark && mark <= self.t
        } else {
            mark > self.prev || mark <= self.t
        }
    }
    /// Whether `t` is within `[a, b)`.
    fn within(&self, a: f32, b: f32) -> bool {
        self.t >= a && self.t < b
    }
}

fn script(ui: &Ui, period: f32) -> Script {
    let now = ui.input(|i| i.time);
    let id = ui.scope_id().with("tx_clock");
    let (t, prev, cycle) = glowkit_core::with_state(
        ui,
        id,
        || ClockState { start: now, prev: -1.0, restart: false },
        |c| {
            if c.restart {
                c.restart = false;
                c.start = now;
                c.prev = -1.0;
            }
            let e = (now - c.start) as f32;
            let t = e.rem_euclid(period);
            let cycle = (e / period).floor() as u64;
            let prev = c.prev;
            c.prev = t;
            (t, prev, cycle)
        },
    );
    let restarts: u64 = glowkit_core::state(ui, id.with("restarts"), || 0);
    glowkit_core::animate(ui);
    Script { t, prev, period, cycle: cycle + restarts * 1000 }
}

fn request_replay(ui: &Ui) {
    let id = ui.scope_id().with("tx_clock");
    let n: u64 = glowkit_core::state(ui, id.with("restarts"), || 0);
    glowkit_core::set_state(ui, id.with("restarts"), n + 1);
    let now = ui.input(|i| i.time);
    glowkit_core::with_state(ui, id, || ClockState { start: now, prev: -1.0, restart: true }, |c| c.restart = true);
}

/// The stage: a soft panel with the content centered in it, a loop progress hairline and a
/// "Replay" button in the corner.
fn stage(ui: &mut Ui, key: &str, height: f32, sc: &Script, add: impl FnOnce(&mut Ui)) {
    let t = Theme::of(ui);
    let id = ui.scope_id().with(("tx_stage", key));
    let measured: Vec2 = glowkit_core::state(ui, id, || Vec2::ZERO);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, height.max(measured.y + 40.0)), Sense::hover());
    let bg = if t.dark { mix(t.background, t.muted, 0.55) } else { mix(t.background, t.muted, 0.6) };
    ui.painter().rect(
        rect,
        cr(18.0),
        bg,
        Stroke::new(1.0, if t.dark { with_alpha(Color32::WHITE, 0.05) } else { with_alpha(t.border, 0.7) }),
        egui::StrokeKind::Inside,
    );
    // loop progress
    let k = sc.t / sc.period;
    let track = Rect::from_min_size(pos2(rect.min.x + 18.0, rect.max.y - 3.0), vec2((rect.width() - 36.0) * k, 1.5));
    ui.painter().rect_filled(track, cr(1.0), with_alpha(t.foreground, 0.12));
    let inner = Rect::from_center_size(rect.center() - vec2(0.0, 6.0), measured);
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(("tx_stage_child", key))
            .max_rect(Rect::from_min_size(inner.min, vec2(rect.max.x - inner.min.x, measured.y.max(1.0))))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing = vec2(36.0, 18.0);
    add(&mut child);
    let got = child.min_rect().size();
    if (got - measured).length() > 0.5 {
        glowkit_core::set_state(ui, id, got);
        ui.ctx().request_repaint();
    }
    // replay
    let font = FontId::proportional(12.0);
    let bw = text_width(ui.painter(), "Replay", &font) + 34.0;
    let br = Rect::from_min_size(pos2(rect.max.x - bw - 12.0, rect.max.y - 40.0), vec2(bw, 28.0));
    let resp = ui.interact(br, id.with("replay"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let fill = if resp.hovered() { shade(chip(&t), if t.dark { 0.05 } else { -0.04 }) } else { chip(&t) };
    let br2 = if resp.is_pointer_button_down_on() { br.shrink(1.0) } else { br };
    ui.painter().rect_filled(br2, cr(14.0), fill);
    MicroIcon::Refresh.paint(ui.painter(), pos2(br2.min.x + 15.0, br2.center().y), 12.0, t.foreground);
    ui.painter().text(pos2(br2.min.x + 26.0, br2.center().y), Align2::LEFT_CENTER, "Replay", font, t.foreground);
    if resp.clicked() {
        request_replay(ui);
    }
}

/// A centered row of parameter sliders under the stage.
fn controls(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.add_space(10.0);
    let id = ui.scope_id().with("tx_controls");
    let w: f32 = glowkit_core::state(ui, id, || 0.0);
    let avail = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(avail, 36.0), Sense::hover());
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("tx_controls_child")
            .max_rect(Rect::from_min_size(pos2(rect.center().x - w / 2.0, rect.min.y), vec2(avail, 36.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 12.0;
    let base = ui.scope_id();
    child.data_mut(|d| d.insert_temp(egui::Id::new("tx_knob_base"), base));
    add(&mut child);
    let got = child.min_rect().width();
    if (got - w).abs() > 0.5 {
        glowkit_core::set_state(ui, id, got);
        ui.ctx().request_repaint();
    }
}

/// A slider row bound to a remembered value.
fn knob(ui: &mut Ui, key: &str, label: &str, default: f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> f32 {
    knob_with(ui, key, label, default, range, suffix, None)
}

fn knob_with(ui: &mut Ui, key: &str, label: &str, default: f32, range: std::ops::RangeInclusive<f32>, suffix: &str, decimals: Option<usize>) -> f32 {
    let base: egui::Id = ui.data(|d| d.get_temp(egui::Id::new("tx_knob_base"))).unwrap_or(ui.scope_id());
    let id = base.with(key);
    let mut v: f32 = glowkit_core::state(ui, id, || default);
    let s = ControlSlider::new(label, &mut v, range).suffix(suffix).width(210.0);
    ui.add(match decimals {
        Some(n) => s.decimals(n),
        None => s,
    });
    glowkit_core::set_state(ui, id, v);
    v
}

/// Same as [`knob`] with decimals.
fn knob_f(ui: &mut Ui, key: &str, label: &str, default: f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> f32 {
    knob_with(ui, key, label, default, range, suffix, Some(2))
}

/// A small arrow cursor for the scripted hover demos.
fn ghost(ui: &Ui, tip: Pos2, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let t = Theme::of(ui);
    let painter = ui.painter();
    let pts: Vec<Pos2> = [(0.0, 0.0), (0.0, 16.0), (4.2, 12.3), (7.0, 18.2), (9.6, 17.0), (6.9, 11.3), (12.0, 11.3)]
        .iter()
        .map(|(x, y)| tip + vec2(*x, *y))
        .collect();
    let (fill, line) = if t.dark { (Color32::WHITE, Color32::BLACK) } else { (Color32::BLACK, Color32::WHITE) };
    let c = tip + vec2(3.5, 9.0);
    for i in 0..pts.len() {
        painter.add(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % pts.len()]], with_alpha(fill, alpha), Stroke::NONE));
    }
    let mut closed = pts.clone();
    closed.push(pts[0]);
    painter.add(Shape::line(closed, Stroke::new(1.2, with_alpha(line, alpha))));
}

/// Smoothly move a remembered ghost position toward `target`.
fn ghost_follow(ui: &Ui, key: &str, target: Pos2) -> Pos2 {
    let dt = glowkit_core::dt(ui);
    glowkit_core::with_state(ui, ui.scope_id().with(key), || target, |p| {
        let k = 1.0 - (-14.0 * dt).exp();
        *p += (target - *p) * k;
        *p
    })
}

const REASONING: &str = "The list stutters when it scrolls, so start with what changes per frame. Each row re-measures its text on every paint, which is wasted work once the width is known.\n\nCaching the layout per row and keying it on width should remove most of that. The images are the other suspect: they decode on the main thread the first time they come into view.\n\nIf decoding moves off the scroll path and the rows keep their measured heights, the frame time should drop well under budget. Worth checking the long rows with emoji, since they take the slow shaping path.";

const STREAM: &str = "Springs feel right because they never stop abruptly: the value carries its velocity into the target, overshoots a hair and settles, so the eye reads weight instead of a timer. Keep exits shorter than entrances, and let blur and opacity finish just before position does.";

pub(crate) fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "tx-card-resize",
            name: "Card Resize",
            category: MOTION,
            description: "A card glides between two sizes on a long smooth curve, its skeleton bars and wrapped text reflowing live as it moves.",
            covers: &["Transitions.dev: Card Resize"],
            code: r##"ui.add(ResizeCard::new(expanded));
ui.add(ResizeCard::new(expanded).text("Release notes", "Sync is faster and...").duration(0.3));"##,
            show: |ui| {
                let sc = script(ui, 2.6);
                let dur = mem(ui, "dur", || 300.0f32);
                let expanded = sc.within(0.3, 1.6);
                stage(ui, "cards", 300.0, &sc, |ui| {
                    ui.add(ResizeCard::new(expanded).duration(dur / 1000.0));
                    ui.add(
                        ResizeCard::new(expanded)
                            .sizes(vec2(250.0, 110.0), vec2(170.0, 150.0))
                            .text("Release notes", "Sync is faster, conflicts resolve on their own and the sidebar remembers where you were.")
                            .duration(dur / 1000.0),
                    );
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 300.0, 100.0..=1200.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-number-pop-in",
            name: "Number Pop In",
            category: TEXT,
            description: "Old digits sink away one by one; new ones rise from below, small and soft, overshoot the baseline and spring into place, staggered left to right.",
            covers: &["Transitions.dev: Number Pop In"],
            code: r##"ui.add(PopNumber::new(format!("{price:.2}")).size(48.0).stagger(0.04));"##,
            show: |ui| {
                let sc = script(ui, 2.2);
                let mut v: (u32, u32) = mem(ui, "v", || (6578, 1284));
                if sc.hit(0.25) {
                    let mut rng = Rng::new(sc.cycle.wrapping_mul(7919) + 11);
                    v = (1000 + rng.below(8999) as u32, 100 + rng.below(9899) as u32);
                    set_mem(ui, "v", v);
                }
                let dur = mem(ui, "dur", || 600.0f32);
                let stagger = mem(ui, "stag", || 40.0f32);
                stage(ui, "nums", 300.0, &sc, |ui| {
                    ui.add(PopNumber::new(format!("{}.{:02}", v.0 / 100, v.0 % 100)).size(56.0).duration(dur / 1000.0).stagger(stagger / 1000.0));
                    ui.add(
                        PopNumber::new(format!("${}", super::numbers::group_thousands(&v.1.to_string())))
                            .size(30.0)
                            .duration(dur / 1000.0)
                            .stagger(stagger / 1000.0)
                            .color(Theme::of(ui).muted_foreground),
                    );
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 600.0, 200.0..=1500.0, "ms");
                    knob(ui, "stag", "Stagger", 40.0, 0.0..=150.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-number-subtle-change",
            name: "Number Subtle Change",
            category: TEXT,
            description: "A counter that changes quietly: the old value drifts a few points away and blurs out as the new one drifts in, up for increases and down for decreases.",
            covers: &["Transitions.dev: Number Subtle Change"],
            code: r##"ui.add(SubtleNumber::new(value).size(44.0));
ui.add(SubtleNumber::new(balance).prefix("$").decimals(2).per_digit(true));"##,
            show: |ui| {
                let sc = script(ui, 1.6);
                let mut v: (f64, f64) = mem(ui, "v", || (1284.0, 2405.5));
                if sc.hit(0.2) {
                    let mut rng = Rng::new(sc.cycle.wrapping_mul(31) + 5);
                    let mut d = 0i64;
                    while d == 0 {
                        d = rng.below(81) as i64 - 40;
                    }
                    v.0 = (v.0 + d as f64).max(0.0);
                    v.1 = (v.1 + rng.range(-30.0, 30.0) as f64).max(0.0);
                    set_mem(ui, "v", v);
                }
                let dur = mem(ui, "dur", || 455.0f32);
                let dist = mem(ui, "dist", || 0.14f32);
                stage(ui, "nums", 300.0, &sc, |ui| {
                    ui.add(SubtleNumber::new(v.0).size(56.0).duration(dur / 1000.0).distance(dist));
                    ui.add(SubtleNumber::new(v.1).prefix("$").decimals(2).per_digit(true).size(32.0).duration(dur / 1000.0).distance(dist));
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 455.0, 100.0..=1200.0, "ms");
                    knob_f(ui, "dist", "Distance", 0.14, 0.0..=0.6, "em");
                });
            },
        },
        Demo {
            id: "tx-notification-badge",
            name: "Notification Badge",
            category: MICRO,
            description: "A count badge slides out of the bell's shoulder, swells past full size on a springy curve and sharpens; it ducks away quickly when cleared.",
            covers: &["Transitions.dev: Notification Badge"],
            code: r##"ui.add(BellBadge::new(unread));
ui.add(BellBadge::new(unread).ring(true).size(48.0));"##,
            show: |ui| {
                let sc = script(ui, 3.2);
                let count = if sc.within(0.4, 1.5) {
                    1
                } else if sc.within(1.5, 2.4) {
                    3
                } else {
                    0
                };
                let dur = mem(ui, "dur", || 500.0f32);
                stage(ui, "bells", 300.0, &sc, |ui| {
                    ui.add(BellBadge::new(count).durations(dur / 1000.0, 0.18));
                    ui.add(BellBadge::new(count).ring(true).size(52.0).durations(dur / 1000.0, 0.18));
                    ui.add(BellBadge::new(if count > 0 { count + 11 } else { 0 }).size(44.0).color(Theme::of(ui).info).durations(dur / 1000.0, 0.18));
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Pop duration", 500.0, 150.0..=1200.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-text-states-swap",
            name: "Text States Swap",
            category: TEXT,
            description: "A status line swaps its words with a tiny lift: the old text rises and blurs out, the new text rises into place from below.",
            covers: &["Transitions.dev: Text States Swap"],
            code: r##"ui.add(StateText::new(status).duration(0.15).distance(4.0));"##,
            show: |ui| {
                let sc = script(ui, 4.2);
                let states = ["Transaction processing...", "Confirming on chain...", "Payment complete"];
                let i = ((sc.t / 1.4) as usize).min(2);
                let dur = mem(ui, "dur", || 150.0f32);
                let dist = mem(ui, "dist", || 4.0f32);
                stage(ui, "swap", 300.0, &sc, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 24.0;
                        ui.add(StateText::new(states[i]).size(20.0).duration(dur / 1000.0).distance(dist));
                        let small = ["Uploading 3 files", "Processing", "Done"];
                        ui.add(StateText::new(small[i]).size(14.0).color(Theme::of(ui).muted_foreground).duration(dur / 1000.0).distance(dist));
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 150.0, 50.0..=600.0, "ms");
                    knob(ui, "dist", "Distance", 4.0, 0.0..=16.0, "px");
                });
            },
        },
        Demo {
            id: "tx-menu-dropdown",
            name: "Menu Dropdown",
            category: MICRO,
            description: "A dropdown opens from its top edge, scaling up from 97% while fading in on a long smooth curve, and closes faster than it opened.",
            covers: &["Transitions.dev: Menu Dropdown"],
            code: r##"let picked = DropdownSurface::new(&mut open)
    .item(Some(MicroIcon::Doc), "New file")
    .item(Some(MicroIcon::Folder), "New folder")
    .show(ui).1;"##,
            show: |ui| {
                let sc = script(ui, 2.8);
                let mut open: (bool, bool) = mem(ui, "open", || (false, false));
                if sc.hit(0.3) {
                    open = (true, true);
                }
                if sc.hit(2.0) {
                    open = (false, false);
                }
                let od = mem(ui, "od", || 250.0f32);
                let cd = mem(ui, "cd", || 150.0f32);
                stage(ui, "menus", 300.0, &sc, |ui| {
                    ui.allocate_ui(vec2(200.0, 160.0), |ui| {
                        let _ = DropdownSurface::new(&mut open.0).width(170.0).durations(od / 1000.0, cd / 1000.0).show(ui);
                    });
                    ui.allocate_ui(vec2(200.0, 160.0), |ui| {
                        ui.push_id("b", |ui| {
                            let _ = DropdownSurface::new(&mut open.1)
                                .label("Create")
                                .item(Some(MicroIcon::Doc), "New document")
                                .item(Some(MicroIcon::Folder), "New folder")
                                .item(Some(MicroIcon::Link), "Share link")
                                .durations(od / 1000.0, cd / 1000.0)
                                .show(ui);
                        });
                    });
                });
                set_mem(ui, "open", open);
                controls(ui, |ui| {
                    knob(ui, "od", "Open", 250.0, 50.0..=800.0, "ms");
                    knob(ui, "cd", "Close", 150.0, 50.0..=800.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-dropdown-menu-morph",
            name: "Dropdown Menu Morph",
            category: MICRO,
            description: "A round plus button stretches into the menu it opens on a springy curve; the plus spins away as the rows slide in, and closing tucks it back with a small nudge.",
            covers: &["Transitions.dev: Dropdown Menu Morph"],
            code: r##"let picked = MorphMenu::new(&mut open)
    .item(MicroIcon::Doc, "New document")
    .item(MicroIcon::Folder, "New folder")
    .item(MicroIcon::Link, "Copy link")
    .item(MicroIcon::Sliders, "Settings")
    .show(ui).1;"##,
            show: |ui| {
                let sc = script(ui, 3.2);
                let mut open: bool = mem(ui, "open", || false);
                if sc.hit(0.4) {
                    open = true;
                }
                if sc.hit(2.2) {
                    open = false;
                }
                let od = mem(ui, "od", || 350.0f32);
                let shift = mem(ui, "shift", || 40.0f32);
                stage(ui, "morph", 300.0, &sc, |ui| {
                    let _ = MorphMenu::new(&mut open)
                        .item(MicroIcon::Doc, "New document")
                        .item(MicroIcon::Folder, "New folder")
                        .item(MicroIcon::Link, "Copy link")
                        .item(MicroIcon::Sliders, "Settings")
                        .durations(od / 1000.0, 0.25)
                        .shift(shift)
                        .show(ui);
                });
                set_mem(ui, "open", open);
                controls(ui, |ui| {
                    knob(ui, "od", "Open", 350.0, 100.0..=1000.0, "ms");
                    knob(ui, "shift", "Row slide", 40.0, 0.0..=100.0, "px");
                });
            },
        },
        Demo {
            id: "tx-modal-open-close",
            name: "Modal Open Close",
            category: MICRO,
            description: "A dialog pops in from 96% while fading on a long smooth curve, and leaves in about half the time.",
            covers: &["Transitions.dev: Modal Open Close"],
            code: r##"ui.add(PopModal::new(open)
    .title("Delete project?")
    .body("This removes the project and its files for everyone.")
    .actions("Cancel", "Delete"));"##,
            show: |ui| {
                let sc = script(ui, 2.6);
                let open = sc.within(0.3, 1.8);
                let od = mem(ui, "od", || 250.0f32);
                let scale = mem(ui, "scale", || 0.96f32);
                stage(ui, "modals", 300.0, &sc, |ui| {
                    ui.add(PopModal::new(open).size(vec2(250.0, 150.0)).durations(od / 1000.0, 0.15).scale(scale));
                    ui.add(
                        PopModal::new(open)
                            .title("Delete project?")
                            .body("This removes the project and its 14 files for everyone on the team.")
                            .actions("Cancel", "Delete")
                            .accent(Theme::of(ui).destructive)
                            .durations(od / 1000.0, 0.15)
                            .scale(scale),
                    );
                });
                controls(ui, |ui| {
                    knob(ui, "od", "Open", 250.0, 50.0..=900.0, "ms");
                    knob_f(ui, "scale", "From scale", 0.96, 0.6..=1.0, "");
                });
            },
        },
        Demo {
            id: "tx-panel-reveal",
            name: "Panel Reveal",
            category: MICRO,
            description: "A panel rises out of a clipped edge below its trigger, sliding up from half its height while it sharpens and fades in.",
            covers: &["Transitions.dev: Panel Reveal"],
            code: r##"ui.add(RevealPanel::new(&mut open)
    .title("Quick actions")
    .tile(MicroIcon::Doc, "Notes")
    .tile(MicroIcon::Folder, "Files"));"##,
            show: |ui| {
                let sc = script(ui, 3.0);
                let mut open: (bool, bool) = mem(ui, "open", || (false, false));
                if sc.hit(0.3) {
                    open = (true, true);
                }
                if sc.hit(2.1) {
                    open = (false, false);
                }
                let od = mem(ui, "od", || 400.0f32);
                stage(ui, "panels", 330.0, &sc, |ui| {
                    ui.allocate_ui(vec2(300.0, 290.0), |ui| {
                        ui.add(RevealPanel::new(&mut open.0).size(vec2(240.0, 170.0)).durations(od / 1000.0, od * 0.875 / 1000.0));
                    });
                    ui.allocate_ui(vec2(300.0, 290.0), |ui| {
                        ui.push_id("b", |ui| {
                            ui.add(
                                RevealPanel::new(&mut open.1)
                                    .label("Shortcuts")
                                    .title("Quick actions")
                                    .tile(MicroIcon::Doc, "Notes")
                                    .tile(MicroIcon::Folder, "Files")
                                    .tile(MicroIcon::Link, "Links")
                                    .tile(MicroIcon::Board, "Boards")
                                    .size(vec2(250.0, 170.0))
                                    .durations(od / 1000.0, od * 0.875 / 1000.0),
                            );
                        });
                    });
                });
                set_mem(ui, "open", open);
                controls(ui, |ui| {
                    knob(ui, "od", "Duration", 400.0, 100.0..=1200.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-page-side-by-side",
            name: "Page Side By Side",
            category: MOTION,
            description: "A two-page card: picking a token drifts the list left and blurs it out while the amount page drifts in from the right, and the card eases to its new height.",
            covers: &["Transitions.dev: Page Side By Side"],
            code: r##"SidePager::new(page).show(ui, |ui, p| match p {
    0 => { if ui.button("Ethereum").clicked() { page = 1; } }
    _ => { if ui.button("Back").clicked() { page = 0; } }
});"##,
            show: |ui| {
                let sc = script(ui, 3.6);
                let mut page: usize = mem(ui, "page", || 0);
                let mut picked: usize = mem(ui, "picked", || 0);
                if sc.hit(0.6) {
                    page = 1;
                    picked = (sc.cycle % 3) as usize;
                }
                if sc.hit(2.4) {
                    page = 0;
                }
                let dur = mem(ui, "dur", || 250.0f32);
                let dist = mem(ui, "dist", || 8.0f32);
                stage(ui, "pager", 330.0, &sc, |ui| {
                    let t = Theme::of(ui);
                    let tokens = [("Ethereum", "ETH", "Ξ", "#627EEA"), ("Avalanche", "AVAX", "▲", "#E84142"), ("BNB", "BNB", "◆", "#F0B90B")];
                    let mut next = page;
                    let mut next_pick = picked;
                    SidePager::new(page).width(270.0).duration(dur / 1000.0).distance(dist).show(ui, |ui, p| {
                        let painter = ui.painter().clone();
                        if p == 0 {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 4.0;
                                let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
                                painter.text(hr.center(), Align2::CENTER_CENTER, "Select token", FontId::proportional(14.0), t.foreground);
                                for (i, (name, tick, glyph, col)) in tokens.iter().enumerate() {
                                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if resp.hovered() {
                                        painter.rect_filled(r, cr(10.0), if t.dark { with_alpha(Color32::WHITE, 0.05) } else { Color32::from_black_alpha(8) });
                                    }
                                    let c = pos2(r.min.x + 22.0, r.center().y);
                                    let colr = glowkit_core::hex(col);
                                    painter.circle_filled(c, 15.0, with_alpha(colr, 0.14));
                                    painter.text(c, Align2::CENTER_CENTER, *glyph, FontId::proportional(13.0), colr);
                                    painter.text(pos2(r.min.x + 46.0, r.center().y - 7.0), Align2::LEFT_CENTER, *name, FontId::proportional(13.0), t.foreground);
                                    painter.text(pos2(r.min.x + 46.0, r.center().y + 8.0), Align2::LEFT_CENTER, *tick, FontId::proportional(11.5), t.muted_foreground);
                                    if resp.clicked() {
                                        next = 1;
                                        next_pick = i;
                                    }
                                }
                            });
                        } else {
                            ui.vertical(|ui| {
                                let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
                                let back = Rect::from_center_size(pos2(hr.min.x + 13.0, hr.center().y), Vec2::splat(26.0));
                                let br = ui.interact(back, ui.scope_id().with("back"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                                painter.circle_filled(back.center(), 13.0, chip(&t));
                                let bc = back.center();
                                painter.add(Shape::line(vec![bc + vec2(2.0, -4.5), bc + vec2(-2.5, 0.0), bc + vec2(2.0, 4.5)], Stroke::new(1.5, t.foreground)));
                                if br.clicked() {
                                    next = 0;
                                }
                                let (_, tick, _, _) = tokens[picked.min(2)];
                                painter.text(hr.center(), Align2::CENTER_CENTER, format!("Send {tick}"), FontId::proportional(14.0), t.foreground);
                                ui.add_space(10.0);
                                let (ar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
                                painter.text(ar.center(), Align2::CENTER_CENTER, "$10", FontId::proportional(44.0), t.foreground);
                                ui.add_space(6.0);
                                let (cr_, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                                let labels = ["25%", "50%", "Max"];
                                let cw = 52.0;
                                let x0 = cr_.center().x - (cw * 3.0 + 12.0) / 2.0;
                                for (i, l) in labels.iter().enumerate() {
                                    let r = Rect::from_min_size(pos2(x0 + i as f32 * (cw + 6.0), cr_.min.y), vec2(cw, 28.0));
                                    painter.rect_filled(r, cr(14.0), chip(&t));
                                    painter.text(r.center(), Align2::CENTER_CENTER, *l, FontId::proportional(12.0), t.muted_foreground);
                                }
                                ui.add_space(4.0);
                                let (lr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
                                painter.text(lr.center(), Align2::CENTER_CENTER, "$66.11 available", FontId::proportional(12.5), t.foreground);
                            });
                        }
                    });
                    page = next;
                    picked = next_pick;
                });
                set_mem(ui, "page", page);
                set_mem(ui, "picked", picked);
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 250.0, 80.0..=900.0, "ms");
                    knob(ui, "dist", "Drift", 8.0, 0.0..=60.0, "px");
                });
            },
        },
        Demo {
            id: "tx-icon-swap",
            name: "Icon Swap",
            category: MICRO,
            description: "An icon button swaps icons in place: the old one shrinks to a quarter, blurs and fades while the new one grows out of the same spot.",
            covers: &["Transitions.dev: Icon Swap"],
            code: r##"ui.add(SwapIcon::new(&mut open, SwapIconPair::MenuClose));
ui.add(SwapIcon::new(&mut playing, SwapIconPair::PlayPause).duration(0.25));"##,
            show: |ui| {
                let sc = script(ui, 2.4);
                let mut on: [bool; 4] = mem(ui, "on", || [false; 4]);
                for (i, v) in on.iter_mut().enumerate() {
                    let a = 0.3 + i as f32 * 0.08;
                    if sc.hit(a) {
                        *v = true;
                    }
                    if sc.hit(a + 1.2) {
                        *v = false;
                    }
                }
                let dur = mem(ui, "dur", || 250.0f32);
                let ss = mem(ui, "ss", || 0.25f32);
                stage(ui, "icons", 300.0, &sc, |ui| {
                    let pairs = [SwapIconPair::MenuClose, SwapIconPair::PlayPause, SwapIconPair::CopyCheck, SwapIconPair::PlusMinus];
                    for (i, p) in pairs.iter().enumerate() {
                        ui.add(SwapIcon::new(&mut on[i], *p).duration(dur / 1000.0).start_scale(ss));
                    }
                });
                set_mem(ui, "on", on);
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 250.0, 60.0..=800.0, "ms");
                    knob_f(ui, "ss", "Start scale", 0.25, 0.0..=1.0, "");
                });
            },
        },
        Demo {
            id: "tx-success-check",
            name: "Success Check",
            category: MICRO,
            description: "A green disc bobs up on a springy curve, spinning back from an angle as it sharpens out of a heavy blur, and its check draws itself a beat later.",
            covers: &["Transitions.dev: Success Check"],
            code: r##"ui.add(SuccessCheck::new(done).size(56.0));"##,
            show: |ui| {
                let sc = script(ui, 2.6);
                let shown = sc.within(0.2, 1.9);
                let dur = mem(ui, "dur", || 500.0f32);
                let rise = mem(ui, "rise", || 40.0f32);
                stage(ui, "checks", 300.0, &sc, |ui| {
                    ui.add(SuccessCheck::new(shown).size(56.0).duration(dur / 1000.0).rise(rise));
                    ui.add(SuccessCheck::new(shown).size(36.0).color(Theme::of(ui).info).duration(dur / 1000.0).rise(rise * 0.7));
                });
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 500.0, 150.0..=1500.0, "ms");
                    knob(ui, "rise", "Rise", 40.0, 0.0..=100.0, "px");
                });
            },
        },
        Demo {
            id: "tx-avatar-group-hover",
            name: "Avatar Group Hover",
            category: MICRO,
            description: "Overlapping avatars lift toward the pointer: the hovered face rises and grows, its neighbours rise less and less, and they drop back with a quick springy bounce.",
            covers: &["Transitions.dev: Avatar Group Hover"],
            code: r##"ui.add(AvatarStack::new(6).more(2).lift(4.0).falloff(0.45));"##,
            show: |ui| {
                let sc = script(ui, 4.4);
                let lift = mem(ui, "lift", || 6.0f32);
                let falloff = mem(ui, "fall", || 0.45f32);
                let (n, r, overlap) = (6usize, 22.0f32, 0.28f32);
                let idx = if sc.within(0.4, 3.0) { Some((((sc.t - 0.4) / 0.36) as usize).min(n)) } else { None };
                let mut rect_seen = Rect::NOTHING;
                let mut user = false;
                stage(ui, "faces", 300.0, &sc, |ui| {
                    let resp = ui.add(AvatarStack::new(n).more(2).radius(r).overlap(overlap).lift(lift).falloff(falloff).hovered(idx));
                    user = resp.hovered();
                    rect_seen = resp.rect;
                });
                let idle = idle_for(ui, ui.scope_id().with("idle"), user);
                if idle > 0.5 && rect_seen.is_positive() {
                    let step = r * 2.0 * (1.0 - overlap);
                    let x0 = rect_seen.min.x + 8.0 + r;
                    let cy = rect_seen.center().y;
                    let target = match idx {
                        Some(i) => pos2(x0 + i as f32 * step, cy + 2.0),
                        None => pos2(if sc.t < 0.4 { x0 - 40.0 } else { x0 + (n as f32 + 1.0) * step + 30.0 }, cy + 34.0),
                    };
                    let p = ghost_follow(ui, "ghost", target);
                    ghost(ui, p, 0.9);
                }
                controls(ui, |ui| {
                    knob(ui, "lift", "Lift", 6.0, 0.0..=16.0, "px");
                    knob_f(ui, "fall", "Falloff", 0.45, 0.0..=1.0, "");
                });
            },
        },
        Demo {
            id: "tx-error-state-shake",
            name: "Error State Shake",
            category: MICRO,
            description: "On a failed submit the field shakes with decaying swings, its outline turns red and a message fades in, then both ease back to neutral.",
            covers: &["Transitions.dev: Error State Shake"],
            code: r##"ui.add(ShakeField::new(&mut email).error("Please enter a valid email.").attempt(attempts));"##,
            show: |ui| {
                let sc = script(ui, 4.4);
                let mut attempt: u32 = mem(ui, "att", || 0);
                if sc.hit(0.4) {
                    attempt += 1;
                }
                let mut text: String = mem(ui, "text", || "John".to_string());
                let dist = mem(ui, "dist", || 6.0f32);
                let hold = mem(ui, "hold", || 3.0f32);
                stage(ui, "field", 300.0, &sc, |ui| {
                    ui.vertical_centered(|ui| {
                        let field = ShakeField::new(&mut text).width(260.0).distance(dist).hold(hold).attempt(attempt);
                        let field = if attempt > 0 { field.error("Please enter a valid email.") } else { field };
                        ui.add(field);
                        ui.add_space(6.0);
                        if super::common::chip_button(ui, "Submit").clicked() {
                            attempt += 1;
                        }
                    });
                });
                set_mem(ui, "att", attempt);
                set_mem(ui, "text", text);
                controls(ui, |ui| {
                    knob(ui, "dist", "Distance", 6.0, 0.0..=20.0, "px");
                    knob_f(ui, "hold", "Hold", 3.0, 0.5..=6.0, "s");
                });
            },
        },
        Demo {
            id: "tx-input-clear-with-dissolve",
            name: "Input Clear With Dissolve",
            category: MICRO,
            description: "Pressing clear dissolves the query: the characters drift down and break into dust from the button outward, a soft haze blooms and the placeholder fades back.",
            covers: &["Transitions.dev: Input Clear With Dissolve"],
            code: r##"ui.add(DissolveSearch::new(&mut query).placeholder("Search"));"##,
            show: |ui| {
                let sc = script(ui, 3.4);
                let mut q: String = mem(ui, "q", || "How do transitions work?".to_string());
                if sc.hit(0.05) && q.is_empty() {
                    q = "How do transitions work?".into();
                }
                let fire = sc.hit(1.2);
                let dur = mem(ui, "dur", || 1000.0f32);
                let fly = mem(ui, "fly", || 12.0f32);
                stage(ui, "search", 300.0, &sc, |ui| {
                    ui.add(DissolveSearch::new(&mut q).width(300.0).duration(dur / 1000.0).fly(fly).clear_if(fire));
                });
                set_mem(ui, "q", q);
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 1000.0, 300.0..=2500.0, "ms");
                    knob(ui, "fly", "Drift", 12.0, 0.0..=40.0, "px");
                });
            },
        },
        Demo {
            id: "tx-skeleton-loader-and-reveal",
            name: "Skeleton Loader And Reveal",
            category: MICRO,
            description: "A skeleton row breathes while it waits, then the real avatar and text fade up out of a slight blur as the skeleton melts away.",
            covers: &["Transitions.dev: Skeleton Loader And Reveal"],
            code: r##"ui.add(SkeletonReveal::new(loaded).name("Jane Cooper").detail("jane.cooper@example.com"));"##,
            show: |ui| {
                let sc = script(ui, 3.6);
                let rd = mem(ui, "rd", || 400.0f32);
                let pulse = mem(ui, "pulse", || 1000.0f32);
                stage(ui, "rows", 300.0, &sc, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        let people = [("Jane Cooper", "jane.cooper@example.com"), ("Theo Marsh", "theo@studio.design"), ("Ada Lindqvist", "ada.l@northwind.io")];
                        for (i, (n, d)) in people.iter().enumerate() {
                            let loaded = sc.within(1.0 + i as f32 * 0.18, 3.0);
                            ui.add(SkeletonReveal::new(loaded).name(*n).detail(*d).tone(i * 3).width(280.0).reveal(rd / 1000.0).pulse(pulse / 1000.0));
                        }
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "rd", "Reveal", 400.0, 100.0..=1200.0, "ms");
                    knob(ui, "pulse", "Breath", 1000.0, 300.0..=2500.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-texts-reveal",
            name: "Texts Reveal",
            category: TEXT,
            description: "Lines (or words) rise 12 points, un-blur and fade in on a long smooth curve, each a beat after the last; hiding is a quick fade.",
            covers: &["Transitions.dev: Texts Reveal"],
            code: r##"ui.add(StaggerReveal::new(shown)
    .line("Pull request opened")
    .secondary("Review requested from 3 teammates"));"##,
            show: |ui| {
                let sc = script(ui, 3.0);
                let shown = sc.within(0.25, 2.5);
                let stagger = mem(ui, "stag", || 40.0f32);
                let dist = mem(ui, "dist", || 12.0f32);
                stage(ui, "reveal", 300.0, &sc, |ui| {
                    ui.add(
                        StaggerReveal::new(shown)
                            .line("Pull request opened")
                            .secondary("Review requested from 3 teammates")
                            .size(18.0)
                            .stagger(stagger / 1000.0)
                            .distance(dist),
                    );
                    ui.add(
                        StaggerReveal::new(shown)
                            .line("Ship it on Friday")
                            .secondary("Everything is green and the notes are written")
                            .by_words(true)
                            .size(18.0)
                            .stagger(stagger / 1000.0)
                            .distance(dist),
                    );
                });
                controls(ui, |ui| {
                    knob(ui, "stag", "Stagger", 40.0, 0.0..=200.0, "ms");
                    knob(ui, "dist", "Distance", 12.0, 0.0..=40.0, "px");
                });
            },
        },
        Demo {
            id: "tx-tooltip-open-close",
            name: "Tooltip Open Close",
            category: MICRO,
            description: "One tooltip shared by a row of buttons: it fades and grows in after a short delay, then glides and resizes between neighbours instead of blinking.",
            covers: &["Transitions.dev: Tooltip Open Close"],
            code: r##"ui.add(TooltipBar::new(&[("Copy", "Copy link"), ("Share", "Share with team"), ("More", "More options")]));"##,
            show: |ui| {
                let sc = script(ui, 4.2);
                let idx = if sc.within(0.4, 1.5) {
                    Some(0)
                } else if sc.within(1.5, 2.3) {
                    Some(1)
                } else if sc.within(2.3, 3.2) {
                    Some(2)
                } else {
                    None
                };
                let md = mem(ui, "md", || 160.0f32);
                let delay = mem(ui, "delay", || 80.0f32);
                let items = [("Copy", "Copy link"), ("Share", "Share with team"), ("More", "More options")];
                let mut seen = Rect::NOTHING;
                let mut user = false;
                stage(ui, "tips", 300.0, &sc, |ui| {
                    let r = ui.add(TooltipBar::new(&items).hovered(idx).move_duration(md / 1000.0).delay(delay / 1000.0));
                    user = r.hovered();
                    seen = r.rect;
                });
                let idle = idle_for(ui, ui.scope_id().with("idle"), user);
                if idle > 0.5 && seen.is_positive() {
                    let font = FontId::proportional(13.0);
                    let widths: Vec<f32> = items.iter().map(|(l, _)| text_width(ui.painter(), l, &font) + 26.0).collect();
                    let total = widths.iter().sum::<f32>() + 12.0;
                    let mut x = seen.center().x - total / 2.0;
                    let mut centers = Vec::new();
                    for w in &widths {
                        centers.push(x + w / 2.0);
                        x += w + 6.0;
                    }
                    let y = seen.max.y - 14.0;
                    let target = match idx {
                        Some(i) => pos2(centers[i] + 4.0, y),
                        None => pos2(seen.max.x + 20.0, y + 30.0),
                    };
                    let p = ghost_follow(ui, "ghost", target);
                    ghost(ui, p, 0.9);
                }
                controls(ui, |ui| {
                    knob(ui, "md", "Glide", 160.0, 40.0..=600.0, "ms");
                    knob(ui, "delay", "Delay", 80.0, 0.0..=500.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-toast-open-close",
            name: "Toast Open Close",
            category: MICRO,
            description: "A toast rises 16 points into place, growing from 97% and un-blurring on a long smooth curve; it leaves the way it came, a little quicker.",
            covers: &["Transitions.dev: Toast Open Close"],
            code: r##"ui.add(SlideToast::new(open).title("Mara shared a file").message("Q3 roadmap.pdf"));"##,
            show: |ui| {
                let sc = script(ui, 2.8);
                let open = sc.within(0.3, 1.9);
                let od = mem(ui, "od", || 350.0f32);
                let dist = mem(ui, "dist", || 16.0f32);
                stage(ui, "toasts", 300.0, &sc, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.add(SlideToast::new(open).title("Mara shared a file").message("Q3 roadmap.pdf · 2.4 MB").tone(1).durations(od / 1000.0, 0.25).distance(dist));
                        ui.add(
                            SlideToast::new(sc.within(0.45, 2.05))
                                .title("Deploy finished")
                                .message("main → production in 42s")
                                .tone(3)
                                .from_top(true)
                                .durations(od / 1000.0, 0.25)
                                .distance(dist),
                        );
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "od", "Open", 350.0, 80.0..=1000.0, "ms");
                    knob(ui, "dist", "Distance", 16.0, 0.0..=60.0, "px");
                });
            },
        },
        Demo {
            id: "tx-learn-more-hover",
            name: "Learn More Hover",
            category: MICRO,
            description: "On hover the chevron leans forward: it nudges right while its arms splay open, optionally growing a shaft into an arrow.",
            covers: &["Transitions.dev: Learn More Hover"],
            code: r##"ui.add(LearnMoreLink::new("Learn more"));
ui.add(LearnMoreLink::new("Read the guide").arrow(true));"##,
            show: |ui| {
                let sc = script(ui, 3.0);
                let a = sc.within(0.4, 1.4);
                let b = sc.within(1.6, 2.6);
                let shift = mem(ui, "shift", || 2.0f32);
                let spread = mem(ui, "spread", || 8.0f32);
                let mut rects = [Rect::NOTHING; 2];
                let mut user = false;
                stage(ui, "links", 300.0, &sc, |ui| {
                    let r1 = ui.add(LearnMoreLink::new("Learn more").shift(shift).spread(spread).hovered(a));
                    let r2 = ui.add(LearnMoreLink::new("Read the guide").arrow(true).shift(shift * 2.0).spread(spread).hovered(b));
                    user = r1.hovered() || r2.hovered();
                    rects = [r1.rect, r2.rect];
                });
                let idle = idle_for(ui, ui.scope_id().with("idle"), user);
                if idle > 0.5 && rects[0].is_positive() {
                    let target = if a {
                        rects[0].center() + vec2(10.0, 4.0)
                    } else if b {
                        rects[1].center() + vec2(10.0, 4.0)
                    } else {
                        pos2((rects[0].max.x + rects[1].min.x) / 2.0, rects[0].max.y + 30.0)
                    };
                    let p = ghost_follow(ui, "ghost", target);
                    ghost(ui, p, 0.9);
                }
                controls(ui, |ui| {
                    knob(ui, "shift", "Nudge", 2.0, 0.0..=10.0, "px");
                    knob(ui, "spread", "Splay", 8.0, 0.0..=30.0, "°");
                });
            },
        },
        Demo {
            id: "tx-banner-stacking",
            name: "Banner Stacking",
            category: MICRO,
            description: "New banners rise in at the front while the others step back, smaller, dimmer and softer, peeking out above; the stack fans out into a list on hover.",
            covers: &["Transitions.dev: Banner Stacking"],
            code: r##"banners.push(StackBanner::new(next_id, "Deploy finished", "main · 42s"));
ui.add(BannerStack::new(&banners));"##,
            show: |ui| {
                let sc = script(ui, 5.6);
                let mut list: Vec<StackBanner> = mem(ui, "list", Vec::new);
                let mut next: u64 = mem(ui, "next", || 0);
                let msgs = [
                    ("Deploy finished", "main → production in 42s"),
                    ("Mara commented", "\"Love the new empty state\""),
                    ("Invoice paid", "$1,240.00 from Northwind"),
                    ("Backup complete", "128 files · 3.2 GB"),
                    ("New sign-in", "Safari on macOS · Lisbon"),
                ];
                if sc.prev < 0.0 {
                    list.clear();
                }
                for k in 0..4 {
                    if sc.hit(0.3 + k as f32 * 0.9) {
                        let (a, b) = msgs[(next as usize) % msgs.len()];
                        list.push(StackBanner::new(next, a, b));
                        next += 1;
                        if list.len() > 4 {
                            list.remove(0);
                        }
                    }
                }
                let spread = sc.within(4.0, 5.2);
                let od = mem(ui, "od", || 350.0f32);
                let peek = mem(ui, "peek", || 12.0f32);
                stage(ui, "stack", 300.0, &sc, |ui| {
                    let mut b = BannerStack::new(&list).durations(od / 1000.0, 0.25).peek(peek);
                    if spread {
                        b = b.spread(true);
                    }
                    ui.add(b);
                });
                set_mem(ui, "list", list);
                set_mem(ui, "next", next);
                controls(ui, |ui| {
                    knob(ui, "od", "Enter", 350.0, 80.0..=1000.0, "ms");
                    knob(ui, "peek", "Peek", 12.0, 0.0..=30.0, "px");
                });
            },
        },
        Demo {
            id: "tx-checkbox-check",
            name: "Checkbox Check",
            category: MICRO,
            description: "The box fills in quickly, then the tick draws itself end to end on a long smooth curve; unchecking erases it fast.",
            covers: &["Transitions.dev: Checkbox Check"],
            code: r##"ui.add(DrawnCheckbox::new(&mut notify, "Notify me"));"##,
            show: |ui| {
                let sc = script(ui, 2.6);
                let mut on: [bool; 3] = mem(ui, "on", || [false; 3]);
                for (i, v) in on.iter_mut().enumerate() {
                    let a = 0.3 + i as f32 * 0.14;
                    if sc.hit(a) {
                        *v = true;
                    }
                    if sc.hit(a + 1.3) {
                        *v = false;
                    }
                }
                let draw = mem(ui, "draw", || 350.0f32);
                stage(ui, "checks", 300.0, &sc, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 14.0;
                        let labels = ["Notify me", "Weekly digest", "Mentions only"];
                        for (i, l) in labels.iter().enumerate() {
                            ui.add(DrawnCheckbox::new(&mut on[i], *l).size(20.0).durations(0.15, draw / 1000.0, 0.15));
                        }
                    });
                });
                set_mem(ui, "on", on);
                controls(ui, |ui| {
                    knob(ui, "draw", "Draw", 350.0, 80.0..=1200.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-spinning-counter",
            name: "Spinning Counter",
            category: TEXT,
            description: "Digits on little drums roll to the new value column by column, neighbours ghosting past; each lands with a pull-back and a springy snap past full size.",
            covers: &["Transitions.dev: Spinning Counter"],
            code: r##"ui.add(DrumCounter::new(visitors).size(56.0).step(0.13).stagger(0.09));"##,
            show: |ui| {
                let sc = script(ui, 3.4);
                let mut v: i64 = mem(ui, "v", || 100);
                if sc.hit(0.3) {
                    let mut rng = Rng::new(sc.cycle.wrapping_mul(977) + 3);
                    let mut n = v;
                    while n == v {
                        n = 100 + rng.below(900) as i64;
                    }
                    v = n;
                    set_mem(ui, "v", v);
                }
                let step = mem(ui, "step", || 130.0f32);
                let stagger = mem(ui, "stag", || 90.0f32);
                stage(ui, "drum", 300.0, &sc, |ui| {
                    ui.add(DrumCounter::new(v).size(64.0).step(step / 1000.0).stagger(stagger / 1000.0));
                    ui.add(DrumCounter::new(v * 37 + 1200).size(32.0).step(step / 1000.0).stagger(stagger / 1000.0).color(Theme::of(ui).muted_foreground));
                });
                controls(ui, |ui| {
                    knob(ui, "step", "Step", 130.0, 30.0..=400.0, "ms");
                    knob(ui, "stag", "Stagger", 90.0, 0.0..=300.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-matrix-dot-loader",
            name: "Matrix Dot Loader",
            category: MICRO,
            description: "Tiny 4 x 4 dot matrices that flash dot by dot in scan, twinkle, orbit and pulse patterns.",
            covers: &["Transitions.dev: Matrix Dot Loader"],
            code: r##"ui.add(DotMatrixLoader::new(DotPattern::Scan));
ui.add(DotMatrixLoader::new(DotPattern::Orbit).dot(6.0).cycle(1.2));"##,
            show: |ui| {
                let sc = script(ui, 2.4);
                let cycle = mem(ui, "cycle", || 1200.0f32);
                let dot = mem(ui, "dot", || 9.0f32);
                stage(ui, "dots", 300.0, &sc, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 26.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 44.0;
                            for p in [DotPattern::Scan, DotPattern::Twinkle, DotPattern::Orbit, DotPattern::Pulse] {
                                ui.add(DotMatrixLoader::new(p).dot(dot).gap(dot * 0.45).cycle(cycle / 1000.0));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let t = Theme::of(ui);
                            ui.add(DotMatrixLoader::new(DotPattern::Orbit).dot(3.0).gap(1.5).cycle(cycle / 1000.0));
                            ui.label(egui::RichText::new("Thinking").size(13.0).color(t.muted_foreground));
                        });
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "cycle", "Cycle", 1200.0, 400.0..=3000.0, "ms");
                    knob(ui, "dot", "Dot size", 9.0, 3.0..=16.0, "px");
                });
            },
        },
        Demo {
            id: "tx-thinking-states",
            name: "Thinking States",
            category: TEXT,
            description: "An AI status label with a soft highlight sweeping across muted text; each new state lifts the old words away and rises in.",
            covers: &["Transitions.dev: Thinking States"],
            code: r##"ui.add(ThinkingText::new(status));
ui.add(ThinkingText::cycle(&["Planning", "Searching", "Writing"], 2.0));"##,
            show: |ui| {
                let sc = script(ui, 8.0);
                let states = ["Setting up a workspace", "Reading the codebase", "Planning the change", "Writing a patch"];
                let i = ((sc.t / 2.0) as usize).min(3);
                let sweep = mem(ui, "sweep", || 2.0f32);
                stage(ui, "think", 300.0, &sc, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spacing_mut().item_spacing.y = 18.0;
                        ui.add(ThinkingText::new(states[i]).size(22.0).sweep(sweep));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.add(DotMatrixLoader::new(DotPattern::Orbit).dot(3.0).gap(1.5));
                            ui.add(ThinkingText::new(format!("Step {} of 4", i + 1)).size(13.0).sweep(sweep));
                        });
                    });
                });
                controls(ui, |ui| {
                    knob_f(ui, "sweep", "Sweep", 2.0, 0.5..=5.0, "s");
                });
            },
        },
        Demo {
            id: "tx-reasoning-stream",
            name: "Reasoning Stream",
            category: TEXT,
            description: "A card of agent reasoning that scrolls itself: every beat the text glides up a couple of lines on an ease-out, fading at the top and bottom, looping seamlessly.",
            covers: &["Transitions.dev: Reasoning Stream"],
            code: r##"ui.add(ReasoningScroll::new(reasoning).size(egui::vec2(320.0, 150.0)).lines(2.0));"##,
            show: |ui| {
                let sc = script(ui, 12.0);
                let hold = mem(ui, "hold", || 840.0f32);
                let lines = mem(ui, "lines", || 2.0f32);
                stage(ui, "reason", 300.0, &sc, |ui| {
                    ui.push_id(sc.cycle, |ui| {
                        ui.add(ReasoningScroll::new(REASONING).size(vec2(320.0, 150.0)).hold(hold / 1000.0).lines(lines));
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "hold", "Hold", 840.0, 100.0..=3000.0, "ms");
                    knob(ui, "lines", "Lines per step", 2.0, 1.0..=5.0, "");
                });
            },
        },
        Demo {
            id: "tx-streaming-text",
            name: "Streaming Text",
            category: TEXT,
            description: "A reply streams in word by word, each word fading up out of a soft blur a beat after the last, with a caret riding the end.",
            covers: &["Transitions.dev: Streaming Text"],
            code: r##"// feed it the text so far; only new words animate
ui.add(StreamText::new(&reply).width(380.0));"##,
            show: |ui| {
                let sc = script(ui, 6.0);
                let gap = mem(ui, "gap", || 60.0f32);
                let fade = mem(ui, "fade", || 320.0f32);
                stage(ui, "stream", 300.0, &sc, |ui| {
                    ui.push_id(sc.cycle, |ui| {
                        ui.add(StreamText::new(STREAM).width(400.0).size(15.5).word_gap(gap / 1000.0).fade(fade / 1000.0));
                    });
                });
                controls(ui, |ui| {
                    knob(ui, "gap", "Word gap", 60.0, 10.0..=200.0, "ms");
                    knob(ui, "fade", "Fade", 320.0, 50.0..=1000.0, "ms");
                });
            },
        },
        Demo {
            id: "tx-toggle",
            name: "Toggle",
            category: MICRO,
            description: "A switch whose pill thumb double-bounces: it overshoots the far end, swings back a hair and settles while the track flips color instantly.",
            covers: &["Transitions.dev: Toggle"],
            code: r##"ui.add(BounceSwitch::new(&mut enabled));
ui.add(BounceSwitch::new(&mut wifi).scale(1.5).color(theme.success));"##,
            show: |ui| {
                let sc = script(ui, 2.2);
                let mut on: [bool; 3] = mem(ui, "on", || [false; 3]);
                for (i, v) in on.iter_mut().enumerate() {
                    let a = 0.3 + i as f32 * 0.1;
                    if sc.hit(a) {
                        *v = true;
                    }
                    if sc.hit(a + 1.0) {
                        *v = false;
                    }
                }
                let dur = mem(ui, "dur", || 350.0f32);
                let ov = mem(ui, "ov", || 1.0f32);
                stage(ui, "switches", 300.0, &sc, |ui| {
                    let t = Theme::of(ui);
                    ui.add(BounceSwitch::new(&mut on[0]).duration(dur / 1000.0).overshoot(ov));
                    ui.add(BounceSwitch::new(&mut on[1]).scale(1.6).duration(dur / 1000.0).overshoot(ov));
                    ui.add(BounceSwitch::new(&mut on[2]).scale(1.6).color(t.success).duration(dur / 1000.0).overshoot(ov));
                });
                set_mem(ui, "on", on);
                controls(ui, |ui| {
                    knob(ui, "dur", "Duration", 350.0, 100.0..=1200.0, "ms");
                    knob_f(ui, "ov", "Overshoot", 1.0, 0.0..=4.0, "px");
                });
            },
        },
    ]
}
