use super::common::{phone, phone_bezel, status_bar};
use super::*;
use egui::{pos2, vec2, Align, Color32, Layout, Rect, Sense, Ui, UiBuilder, Widget};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::Theme;
use glowkit_effects::ControlSlider;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

/// Parameter values for a demo, persisted per demo.
fn params<const N: usize>(ui: &Ui, key: &str, init: [f32; N]) -> [f32; N] {
    mem(ui, key, || init)
}

/// Run `f` inside `rect`, vertically centered using last frame's measured height.
fn centered(ui: &mut Ui, rect: Rect, key: &str, f: impl FnOnce(&mut Ui)) {
    let mid = ui.scope_id().with(("centered", key));
    let measured: f32 = glowkit_core::state(ui, mid, || rect.height() * 0.5);
    let top = ((rect.height() - measured) / 2.0).max(0.0);
    let child = Rect::from_min_max(rect.min + vec2(0.0, top), rect.max);
    let h = ui
        .scope_builder(UiBuilder::new().max_rect(child).layout(Layout::top_down(Align::Center)), |ui| {
            f(ui);
            ui.min_rect().height()
        })
        .inner;
    glowkit_core::set_state(ui, mid, h);
}

/// The live component on the left, parameter sliders on the right (stacked on narrow screens).
fn bench<P>(
    ui: &mut Ui,
    height: f32,
    p: &mut P,
    component: impl FnOnce(&mut Ui, &mut P),
    controls: impl FnOnce(&mut Ui, &mut P),
) {
    let w = ui.available_width();
    if w > 620.0 {
        let panel_w = 230.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        let left = Rect::from_min_size(rect.min, vec2(w - panel_w - 28.0, height));
        let right = Rect::from_min_max(pos2(rect.max.x - panel_w, rect.min.y), rect.max);
        centered(ui, left, "stage", |ui| component(ui, p));
        centered(ui, right, "panel", |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            controls(ui, p);
        });
    } else {
        let (rect, _) = ui.allocate_exact_size(vec2(w, height), Sense::hover());
        centered(ui, rect, "stage", |ui| component(ui, p));
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        controls(ui, p);
    }
}

/// Paint a phone and place `widget` on its screen.
fn on_phone(ui: &mut Ui, w: f32, h: f32, screen: Option<Color32>, widget: impl Widget) {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let fill = screen.unwrap_or(if t.dark { t.background } else { Color32::WHITE });
    let scr = phone(ui.painter(), &t, rect, fill);
    let dark_screen = glowkit_core::color::luminance(fill) < 0.4;
    ui.put(scr, widget);
    status_bar(ui.painter(), scr, if dark_screen { Color32::WHITE } else { t.foreground });
    phone_bezel(ui.painter(), &t, rect);
}

const PH_W: f32 = 272.0;
const PH_H: f32 = 540.0;
const PHONE_BENCH: f32 = 560.0;

/// Place `widget` at the top-left of `rect` (unlike `ui.put`, which centers it).
fn put_top(ui: &mut Ui, rect: Rect, widget: impl Widget) {
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)), |ui| {
        ui.add(widget);
    });
}

/// A small pill button painted in the theme (used inside phone screens).
fn chip(ui: &mut Ui, rect: Rect, label: &str, key: &str) -> bool {
    let t = Theme::of(ui);
    let resp = ui.interact(rect, ui.scope_id().with(("chip", key)), Sense::click());
    let hov = resp.hovered();
    let fill = glowkit_core::color::mix(
        if t.dark { t.muted } else { glowkit_core::hex("#F1F1F3") },
        t.foreground,
        if hov { 0.06 } else { 0.0 },
    );
    ui.painter().rect_filled(rect, super::common::cr(rect.height() / 2.0), fill);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.5), t.foreground);
    if hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// A phone frame: `f` paints the screen content (status bar already painted), then the bezel.
fn phone_frame(ui: &mut Ui, w: f32, h: f32, screen: Option<Color32>, f: impl FnOnce(&mut Ui, Rect)) {
    let t = Theme::of(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let fill = screen.unwrap_or(if t.dark { t.background } else { Color32::WHITE });
    let scr = phone(ui.painter(), &t, rect, fill);
    let dark_screen = glowkit_core::color::luminance(fill) < 0.4;
    status_bar(ui.painter(), scr, if dark_screen { Color32::WHITE } else { t.foreground });
    f(ui, scr);
    phone_bezel(ui.painter(), &t, rect);
}

/// The grouped-list background color of a phone screen.
fn stage_color(ui: &Ui) -> Color32 {
    super::common::stage(&Theme::of(ui))
}

/// A large bold screen title.
fn header(ui: &Ui, scr: Rect, title: &str) {
    let t = Theme::of(ui);
    super::common::text_bold(
        ui.painter(),
        pos2(scr.min.x + 18.0, scr.min.y + 46.0),
        egui::Align2::LEFT_CENTER,
        title,
        24.0,
        t.foreground,
    );
}

pub(super) fn all() -> Vec<Demo> {
    vec![Demo {
        id: "live-island",
        name: "Live Island",
        category: Category::MicroInteractions,
        description: "A black pill at the top of the screen that springs open into a live activity card and stretches to show notices.",
        covers: &["suiss/ui: Island"],
        code: r##"ui.add(LiveIsland::new("island").bounce(45.0));
if ui.button("Ping").clicked() {
    LiveIsland::notify(ui.ctx(), "island", IslandNotice::new("Maya", "Running 5 min late"));
}"##,
        show: |ui| {
            let mut p = params(ui, "p", [45.0, 2.6]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    phone_frame(ui, PH_W, PH_H, None, |ui, scr| {
                    let t = Theme::of(ui);
                    let key = ui.scope_id().with("island");
                    ui.painter().text(
                        pos2(scr.center().x, scr.center().y - 30.0),
                        egui::Align2::CENTER_CENTER,
                        "Tap or pull the pill.",
                        egui::FontId::proportional(12.5),
                        t.muted_foreground,
                    );
                    let b1 = Rect::from_center_size(pos2(scr.center().x - 58.0, scr.center().y + 4.0), vec2(104.0, 30.0));
                    let b2 = Rect::from_center_size(pos2(scr.center().x + 58.0, scr.center().y + 4.0), vec2(104.0, 30.0));
                    if chip(ui, b1, "New message", "m") {
                        LiveIsland::notify(ui.ctx(), key, IslandNotice::new("Maya Chen", "On my way, 5 min").badge("MC"));
                    }
                    if chip(ui, b2, "Reminder", "r") {
                        LiveIsland::notify(
                            ui.ctx(),
                            key,
                            IslandNotice::new("Stand-up", "Starts in 10 minutes").badge("10").accent(glowkit_core::hex("#AF52DE")),
                        );
                    }
                    let area = Rect::from_min_size(scr.min, vec2(scr.width(), 220.0));
                    ui.put(area, LiveIsland::new(key).bounce(p[0]).notice_duration(p[1]).autoplay(true));
                    });
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Notice", &mut p[1], 1.0..=6.0).decimals(1).suffix("s"));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "otp-autofill",
        name: "OTP Autofill",
        category: Category::Inputs,
        description: "A one-time-code field: a keypad springs up with the incoming code, the digits pop in one by one, and a wrong code shakes itself clear.",
        covers: &["suiss/ui: OTP Autofill"],
        code: r##"let mut code = String::new();
ui.add(OtpAutofill::new(&mut code).expected("482913").suggestion("482913").shake(60.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [60.0, 6.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut code: String = mem(ui, "code", String::new);
                    on_phone(
                        ui,
                        PH_W,
                        PH_H,
                        None,
                        OtpAutofill::new(&mut code)
                            .digits(p[1] as usize)
                            .expected(&"482913507"[..p[1] as usize])
                            .suggestion(&"482913507"[..p[1] as usize])
                            .shake(p[0])
                            .autoplay(true),
                    );
                    set_mem(ui, "code", code);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Shake", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Digits", &mut p[1], 4.0..=8.0).step(2.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "preview-menu",
        name: "Preview Menu",
        category: Category::MicroInteractions,
        description: "Touch and hold a photo: the grid frosts over, the photo springs into a large preview and a menu unfolds; slide onto an action and let go.",
        covers: &["suiss/ui: Preview Menu"],
        code: r##"ui.add(PreviewMenu::new().items(9).hold(0.35).bounce(40.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [40.0, 0.35, 9.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    on_phone(ui, PH_W, PH_H, None, PreviewMenu::new().bounce(p[0]).hold(p[1]).items(p[2] as usize).autoplay(true));
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Hold", &mut p[1], 0.1..=1.0).decimals(2).suffix("s"));
                    ui.add(ControlSlider::new("Photos", &mut p[2], 3.0..=12.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "reorder-list",
        name: "Reorder List",
        category: Category::MicroInteractions,
        description: "Drag a row by its grip: it lifts with a shadow, follows the finger with rubber-banded ends, and the other rows spring out of its way.",
        covers: &["suiss/ui: Reorder List"],
        code: r##"let mut rows = vec![TouchRow::new("Inbox", "12 messages"), TouchRow::new("Today", "")];
ui.add(ReorderList::new(&mut rows).bounce(40.0).lift(50.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [40.0, 50.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut rows: Vec<TouchRow> = mem(ui, "rows", super::reorder::sample_lists);
                    let stage = stage_color(ui);
                    phone_frame(ui, PH_W, PH_H, Some(stage), |ui, scr| {
                        header(ui, scr, "Lists");
                        let area = Rect::from_min_size(scr.min + vec2(12.0, 64.0), vec2(scr.width() - 24.0, 340.0));
                        put_top(ui, area, ReorderList::new(&mut rows).bounce(p[0]).lift(p[1]).autoplay(true));
                    });
                    set_mem(ui, "rows", rows);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Lift", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "swipe-actions",
        name: "Swipe Actions",
        category: Category::MicroInteractions,
        description: "Mail rows that slide to reveal Flag, Delete and Pin; over-drag rubber-bands, and a long swipe stretches Delete across the row and collapses it.",
        covers: &["suiss/ui: Swipe Actions"],
        code: r##"let mut rows = vec![TouchRow::new("Maya Chen", "Dinner on Friday?")];
ui.add(SwipeActions::new(&mut rows).full_swipe(true).resistance(50.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [50.0, 30.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut rows: Vec<TouchRow> = mem(ui, "rows", super::swipe::sample_messages);
                    let stage = stage_color(ui);
                    phone_frame(ui, PH_W, PH_H, Some(stage), |ui, scr| {
                        header(ui, scr, "Inbox");
                        let area = Rect::from_min_size(scr.min + vec2(12.0, 66.0), vec2(scr.width() - 24.0, 400.0));
                        put_top(ui, area, SwipeActions::new(&mut rows).resistance(p[0]).bounce(p[1]).autoplay(true));
                        let reset = Rect::from_center_size(pos2(scr.center().x, scr.max.y - 40.0), vec2(80.0, 28.0));
                        if rows.len() < 3 || chip(ui, reset, "Reset", "reset") {
                            rows = super::swipe::sample_messages();
                        }
                    });
                    set_mem(ui, "rows", rows);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Resistance", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Bounce", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "selection-list",
        name: "Selection List",
        category: Category::MicroInteractions,
        description: "Tap Select: rows slide aside in a cascade for round checkboxes, a floating toolbar rises, and a sweep down the checkbox column selects a run.",
        covers: &["suiss/ui: Selection List"],
        code: r##"let mut notes = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
ui.add(SelectionList::new(&mut notes).title("Notes").cascade(40.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [30.0, 40.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut rows: Vec<TouchRow> = mem(ui, "rows", super::select::sample_notes);
                    if rows.len() < 4 {
                        rows = super::select::sample_notes();
                    }
                    on_phone(ui, PH_W, PH_H, Some(stage_color(ui)), SelectionList::new(&mut rows).bounce(p[0]).cascade(p[1]).autoplay(true));
                    set_mem(ui, "rows", rows);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Cascade", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "search-reveal",
        name: "Search Reveal",
        category: Category::MicroInteractions,
        description: "A search field tucked above a list: pull down and it unfolds, stop halfway and it snaps; the list flicks with momentum and rubber-bands at both ends.",
        covers: &["suiss/ui: Search Reveal"],
        code: r##"let rows = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
ui.add(SearchReveal::new(&rows).title("Notes").snap(50.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [50.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let rows = super::search::sample_long_notes();
                    on_phone(ui, PH_W, PH_H, Some(stage_color(ui)), SearchReveal::new(&rows).snap(p[0]).autoplay(true));
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Snap", &mut p[0], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "section-index",
        name: "Section Index",
        category: Category::Navigation,
        description: "Contacts with sticky headers and an A-Z bar: scrub it and the letters swell like a dock while a bubble pops out beside the finger.",
        covers: &["suiss/ui: Section Index"],
        code: r##"let names = ["Aaron Price", "Abby Chen", "Ben Carter", "Carla Diaz"];
ui.add(SectionIndex::new(&names).title("Contacts").magnify(60.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [60.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    on_phone(ui, PH_W, PH_H, None, SectionIndex::new(&super::index::SAMPLE_NAMES).magnify(p[0]).autoplay(true));
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Magnify", &mut p[0], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "coach-tip",
        name: "Coach Tip",
        category: Category::Overlays,
        description: "Feature tips that pop out of their arrow on a spring; drag one and it rubber-bands, fling it and it flies off; then the next tip in the group appears.",
        covers: &["suiss/ui: Tip"],
        code: r##"ui.add(CoachTip::new("new-note", "Start a note fast", "Tap here any time to begin a new note.").anchor(anchor));
if CoachTip::dismissed(ui.ctx(), "new-note") {
    ui.add(CoachTip::new("share", "Share in one tap", "Send any note to a friend.").anchor(share_anchor));
}"##,
        show: |ui| {
            let mut p = params(ui, "p", [45.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let stage = stage_color(ui);
                    phone_frame(ui, PH_W, PH_H, Some(stage), |ui, scr| {
                        let t = Theme::of(ui);
                        header(ui, scr, "Notes");
                        let edit = pos2(scr.max.x - 26.0, scr.min.y + 46.0);
                        let share = pos2(scr.max.x - 60.0, scr.min.y + 46.0);
                        super::common::glyph(ui.painter(), super::common::Glyph::Edit, edit, 17.0, super::common::system_blue(&t));
                        super::common::glyph(ui.painter(), super::common::Glyph::Share, share, 17.0, super::common::system_blue(&t));
                        let salt = ui.scope_id();
                        let k1 = salt.with("tip-new");
                        let k2 = salt.with("tip-share");
                        let k3 = salt.with("tip-pin");
                        let d1 = CoachTip::dismissed(ui.ctx(), k1);
                        let d2 = CoachTip::dismissed(ui.ctx(), k2);
                        let d3 = CoachTip::dismissed(ui.ctx(), k3);
                        let list_area = Rect::from_min_max(scr.min + vec2(12.0, 70.0), scr.max - vec2(12.0, 60.0));
                        let mut child = ui.new_child(UiBuilder::new().max_rect(list_area).layout(Layout::top_down(Align::Min)));
                        if d1 && d2 {
                            child.add(
                                CoachTip::new(k3, "Pin what matters", "Swipe right on a note to keep it at the top.")
                                    .inline(true)
                                    .icon(glowkit_effects::MicroIcon::Heart)
                                    .accent(glowkit_core::hex("#FF2D55"))
                                    .bounce(p[0])
                                    .autoplay(true),
                            );
                        }
                        let rows = super::select::sample_notes();
                        let top = child.cursor().min.y;
                        let list = Rect::from_min_size(pos2(list_area.min.x, top), vec2(list_area.width(), 58.0 * 4.0));
                        super::common::card(child.painter(), &t, list, 14.0, 0.4, super::common::surface(&t));
                        for (i, r) in rows.iter().take(4).enumerate() {
                            let y = top + 58.0 * i as f32;
                            super::common::text_bold(child.painter(), pos2(list.min.x + 16.0, y + 20.0), egui::Align2::LEFT_CENTER, &r.title, 14.0, t.foreground);
                            super::common::text(child.painter(), pos2(list.min.x + 16.0, y + 39.0), egui::Align2::LEFT_CENTER, &r.detail, 12.0, t.muted_foreground);
                            if i < 3 {
                                child.painter().line_segment(
                                    [pos2(list.min.x + 16.0, y + 58.0), pos2(list.max.x, y + 58.0)],
                                    egui::Stroke::new(1.0, super::common::hairline(&t)),
                                );
                            }
                        }
                        if !d1 {
                            ui.add(CoachTip::new(k1, "Start a note fast", "Tap here any time to begin a new note.").anchor(edit + vec2(0.0, 12.0)).bounds(scr).bounce(p[0]).autoplay(true));
                        } else if !d2 {
                            ui.add(
                                CoachTip::new(k2, "Share in one tap", "Send any note to a friend, or collaborate live.")
                                    .anchor(share + vec2(0.0, 12.0))
                                    .bounds(scr)
                                    .icon(glowkit_effects::MicroIcon::Link)
                                    .bounce(p[0])
                                    .autoplay(true),
                            );
                        }
                        let again = Rect::from_center_size(pos2(scr.center().x, scr.max.y - 36.0), vec2(124.0, 28.0));
                        let all_done = d1 && d2 && d3;
                        let since: f32 = {
                            let now = glowkit_core::time(ui);
                            let key = salt.with("tips-done-at");
                            let at = glowkit_core::state(ui, key, || now);
                            if !all_done {
                                glowkit_core::set_state(ui, key, now);
                            }
                            now - at
                        };
                        if chip(ui, again, "Show tips again", "again") || (all_done && since > 1.5) {
                            CoachTip::reset(ui.ctx(), k1);
                            CoachTip::reset(ui.ctx(), k2);
                            CoachTip::reset(ui.ctx(), k3);
                        }
                        if all_done {
                            ui.ctx().request_repaint();
                        }
                    });
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "top-banner",
        name: "Top Banner",
        category: Category::Overlays,
        description: "Banners drop in from the top on a spring and queue one at a time; grab one to pause it, stretch it down against rubber, or flick it up to dismiss.",
        covers: &["suiss/ui: Top Banner"],
        code: r##"ui.add(TopBanner::new("banners").duration(4.0));
if ui.button("Copy link").clicked() {
    TopBanner::show(ui.ctx(), "banners", BannerSpec::new("Link copied").description("Anyone with the link can view."));
}"##,
        show: |ui| {
            let mut p = params(ui, "p", [4.0, 35.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    phone_frame(ui, PH_W, PH_H, None, |ui, scr| {
                        let key = ui.scope_id().with("banners");
                        let cx = scr.center().x;
                        let labels = ["Show banner", "With icon and action", "Queue three", "Dismiss current"];
                        for (i, l) in labels.iter().enumerate() {
                            let r = Rect::from_center_size(pos2(cx, scr.center().y - 40.0 + i as f32 * 40.0), vec2(scr.width() - 60.0, 30.0));
                            if chip(ui, r, l, l) {
                                use glowkit_effects::MicroIcon as I;
                                let blue = glowkit_core::hex("#0A84FF");
                                match i {
                                    0 => TopBanner::show(ui.ctx(), key, BannerSpec::new("Link copied")),
                                    1 => TopBanner::show(
                                        ui.ctx(),
                                        key,
                                        BannerSpec::new("Photo saved").description("Added to Recents.").icon(I::Folder, glowkit_core::hex("#FF9F0A")).action("View"),
                                    ),
                                    2 => {
                                        for (title, icon) in [("Uploading 3 files", I::ArrowUp), ("Upload complete", I::Check), ("Shared with Maya", I::Link)] {
                                            TopBanner::show(ui.ctx(), key, BannerSpec::new(title).icon(icon, blue));
                                        }
                                    }
                                    _ => TopBanner::dismiss(ui.ctx(), key),
                                }
                            }
                        }
                        ui.put(scr, TopBanner::new(key).duration(p[0]).bounce(p[1]).autoplay(true));
                    });
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Duration", &mut p[0], 1.0..=8.0).decimals(1).suffix("s"));
                    ui.add(ControlSlider::new("Bounce", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "undo-bar",
        name: "Undo Bar",
        category: Category::Overlays,
        description: "Delete a note and it folds away while a pill with a countdown ring rises; Undo unfolds it back, swipe the pill down to commit.",
        covers: &["suiss/ui: Undo Bar"],
        code: r##"let mut notes = vec![TouchRow::new("Groceries", "Milk, eggs, bread")];
ui.add(UndoBar::new(&mut notes).timeout(5.0).bounce(35.0));"##,
        show: |ui| {
            let mut p = params(ui, "p", [5.0, 35.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut rows: Vec<TouchRow> = mem(ui, "rows", super::select::sample_notes);
                    if rows.len() < 3 {
                        rows = super::select::sample_notes();
                    }
                    on_phone(ui, PH_W, PH_H, Some(stage_color(ui)), UndoBar::new(&mut rows).timeout(p[0]).bounce(p[1]).autoplay(true));
                    set_mem(ui, "rows", rows);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Timeout", &mut p[0], 1.0..=10.0).decimals(1).suffix("s"));
                    ui.add(ControlSlider::new("Bounce", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "confirm-sheet",
        name: "Confirm Sheet",
        category: Category::Overlays,
        description: "A confirmation that rises as an action sheet on phones (drag it down to dismiss, push it up against rubber) and pops as an alert on wide screens.",
        covers: &["suiss/ui: Confirm"],
        code: r##"ui.add(ConfirmSheet::new("Delete order", "Delete this order?")
    .description("The order and its receipt will be removed. This can't be undone.")
    .style(ConfirmStyle::Auto));"##,
        show: |ui| {
            let mut p = params(ui, "p", [25.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    on_phone(
                        ui,
                        PH_W,
                        PH_H,
                        None,
                        ConfirmSheet::new("Delete order", "Delete this order?")
                            .description("The order and its receipt will be removed. This can't be undone.")
                            .bounce(p[0])
                            .autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "action-bar-sheet",
        name: "Action Bar Sheet",
        category: Category::Navigation,
        description: "Command menus that collapse into a ⋯ drawer on phones: rows slide sideways into each menu's items, and the drawer drags and flicks away.",
        covers: &["suiss/ui: Action Bar"],
        code: r##"ui.add(ActionBarSheet::new()
    .menu("File", &["New", "Open…", "Duplicate"])
    .menu("Edit", &["Undo", "Copy", "Paste"])
    .menu("View", &["Zoom In", "Zoom Out"]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [25.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    on_phone(ui, PH_W, PH_H, None, ActionBarSheet::new().bounce(p[0]).mobile(true).autoplay(true));
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    let w = ui.available_width();
                    let (r, _) = ui.allocate_exact_size(vec2(w, 200.0), Sense::hover());
                    ui.put(r, ActionBarSheet::new().bounce(p[0]).mobile(false).title("Wide"));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "trail-nav",
        name: "Trail Nav",
        category: Category::Navigation,
        description: "A navigation stack with breadcrumbs and a back link: pages slide with parallax, swipe from the edge to pop interactively, hold Back for the history.",
        covers: &["suiss/ui: Trail"],
        code: r##"let mut path = vec!["Settings".to_string()];
ui.add(TrailNav::new(&mut path)
    .page("Settings", &["Privacy & Security", "Display"])
    .page("Privacy & Security", &["Location Services", "Camera"]));"##,
        show: |ui| {
            let mut p = params(ui, "p", [30.0, 15.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    let mut path: Vec<String> = mem(ui, "path", Vec::new);
                    on_phone(ui, PH_W, PH_H, Some(stage_color(ui)), TrailNav::new(&mut path).parallax(p[0]).bounce(p[1]).autoplay(true));
                    set_mem(ui, "path", path);
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Parallax", &mut p[0], 0.0..=100.0).suffix("%"));
                    ui.add(ControlSlider::new("Bounce", &mut p[1], 0.0..=100.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }, Demo {
        id: "swipe-image-viewer",
        name: "Swipe Image Viewer",
        category: Category::MicroInteractions,
        description: "A photo grid that grows into a full-screen viewer: swipe to page with rubber-banded edges, double-tap to zoom, drag down to fling it home.",
        covers: &["suiss/ui: Image Viewer"],
        code: r##"ui.add(SwipeImageViewer::new().images(6).bounce(35.0).zoom(2.5));"##,
        show: |ui| {
            let mut p = params(ui, "p", [35.0, 2.5, 6.0]);
            bench(
                ui,
                PHONE_BENCH,
                &mut p,
                |ui, p| {
                    on_phone(
                        ui,
                        PH_W,
                        PH_H,
                        None,
                        SwipeImageViewer::new().bounce(p[0]).zoom(p[1]).images(p[2] as usize).autoplay(true),
                    );
                },
                |ui, p| {
                    ui.add(ControlSlider::new("Bounce", &mut p[0], 0.0..=100.0));
                    ui.add(ControlSlider::new("Zoom", &mut p[1], 1.5..=4.0).decimals(1).suffix("x"));
                    ui.add(ControlSlider::new("Photos", &mut p[2], 2.0..=9.0));
                },
            );
            set_mem(ui, "p", p);
        },
    }]
}
