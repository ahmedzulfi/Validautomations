//! Transitions (micro-interactions) and game UI playgrounds.

use super::{center, flag, set_flag, variant, Code, Params, Playground};
use glowkit::color::hex;
use glowkit::core::playground::{lit_f32, lit_str};
use glowkit::egui::{self, vec2, Align, Layout, Rect, Sense, Ui, UiBuilder};
use glowkit::interactions::*;

pub fn all() -> Vec<Playground> {
    vec![
        Playground { id: "tx-number-pop-in", show: pop_number },
        Playground { id: "tx-notification-badge", show: bell_badge },
        Playground { id: "tx-success-check", show: success_check },
        Playground { id: "tx-toggle", show: bounce_switch },
        Playground { id: "tx-matrix-dot-loader", show: dot_loader },
        Playground { id: "tx-icon-swap", show: swap_icon },
        Playground { id: "game-resource-bars", show: resource_bar },
        Playground { id: "game-boss-bar", show: boss_bar },
        Playground { id: "game-cooldown-icons", show: cooldown },
        Playground { id: "game-crosshairs", show: crosshair },
    ]
}

fn pop_number(ui: &mut Ui, p: &mut Params) -> String {
    let base = p.f32("Price", 1299.0, 0.0..=99_999.0, 1.0);
    let ticker = p.bool("Live ticker", true);
    let size = p.f32("Size", 56.0, 16.0..=110.0, 1.0);
    let stagger = p.f32("Stagger (s)", 0.04, 0.0..=0.2, 0.005);
    let duration = p.f32("Duration (s)", 0.45, 0.1..=1.5, 0.05);
    let distance = p.f32("Distance (em)", 0.6, 0.0..=2.0, 0.05);
    let step = (glowkit::core::time(ui) / 1.6) as i64;
    let price = if ticker { base as f64 + (step % 7) as f64 * 3.75 } else { base as f64 };
    if ticker {
        glowkit::core::animate(ui);
    }
    center(ui, |ui| {
        ui.add_space(60.0);
        ui.add(PopNumber::new(format!("${price:.2}")).size(size).stagger(stagger).duration(duration).distance(distance))
    });
    Code::new("glowkit::interactions::PopNumber", "PopNumber::new(format!(\"${price:.2}\"))")
        .pre(format!("let price = {};", lit_f32(base)))
        .f("size", size)
        .f("stagger", stagger)
        .f("duration", duration)
        .f("distance", distance)
        .add()
}

fn bell_badge(ui: &mut Ui, p: &mut Params) -> String {
    let count = p.int("Unread", 3, 0..=120) as u32;
    let size = p.f32("Size", 48.0, 20.0..=96.0, 1.0);
    let ring = p.bool("Ring on change", true);
    let color = p.color("Badge color", hex("#EF4444"));
    center(ui, |ui| {
        ui.add_space(60.0);
        ui.add(BellBadge::new(count).size(size).ring(ring).color(color))
    });
    Code::new("glowkit::interactions::BellBadge", format!("BellBadge::new({count})"))
        .f("size", size)
        .b("ring", ring)
        .c("color", color)
        .add()
}

fn success_check(ui: &mut Ui, p: &mut Params) -> String {
    let shown = p.bool("Shown", true);
    let size = p.f32("Size", 64.0, 20.0..=140.0, 1.0);
    let color = p.color("Color", hex("#22C55E"));
    let duration = p.f32("Duration (s)", 0.5, 0.1..=2.0, 0.05);
    let rise = p.f32("Rise", 8.0, 0.0..=40.0, 0.5);
    let turn = p.f32("Turn (deg)", 0.0, -90.0..=90.0, 1.0);
    center(ui, |ui| {
        ui.add_space(60.0);
        ui.add(SuccessCheck::new(shown).size(size).color(color).duration(duration).rise(rise).turn(turn))
    });
    Code::new("glowkit::interactions::SuccessCheck", format!("SuccessCheck::new({shown})"))
        .f("size", size)
        .c("color", color)
        .f("duration", duration)
        .f("rise", rise)
        .f("turn", turn)
        .add()
}

fn bounce_switch(ui: &mut Ui, p: &mut Params) -> String {
    let scale = p.f32("Scale", 1.5, 0.5..=3.0, 0.05);
    let duration = p.f32("Duration (s)", 0.35, 0.1..=1.2, 0.05);
    let overshoot = p.f32("Overshoot", 4.0, 0.0..=16.0, 0.5);
    let color = p.color("On color", hex("#22C55E"));
    let mut on = flag(ui, "on", true);
    center(ui, |ui| {
        ui.add_space(70.0);
        ui.add(BounceSwitch::new(&mut on).scale(scale).duration(duration).overshoot(overshoot).color(color))
    });
    set_flag(ui, "on", on);
    Code::new("glowkit::interactions::BounceSwitch", "BounceSwitch::new(&mut enabled)")
        .f("scale", scale)
        .f("duration", duration)
        .f("overshoot", overshoot)
        .c("color", color)
        .add()
}

const DOTS: [&str; 4] = ["Scan", "Twinkle", "Orbit", "Pulse"];

fn dot_loader(ui: &mut Ui, p: &mut Params) -> String {
    let pattern = p.choice("Pattern", &DOTS, 0);
    let dot = p.f32("Dot size", 8.0, 2.0..=20.0, 0.5);
    let gap = p.f32("Gap", 4.0, 0.0..=12.0, 0.5);
    let cycle = p.f32("Cycle (s)", 1.2, 0.3..=4.0, 0.05);
    let round = p.bool("Round dots", true);
    let pat = [DotPattern::Scan, DotPattern::Twinkle, DotPattern::Orbit, DotPattern::Pulse][pattern];
    center(ui, |ui| {
        ui.add_space(70.0);
        ui.add(DotMatrixLoader::new(pat).dot(dot).gap(gap).cycle(cycle).round(round))
    });
    Code::new(
        "glowkit::interactions::{DotMatrixLoader, DotPattern}",
        format!("DotMatrixLoader::new({})", variant("DotPattern", &DOTS, pattern)),
    )
    .f("dot", dot)
    .f("gap", gap)
    .f("cycle", cycle)
    .b("round", round)
    .add()
}

const PAIRS: [&str; 4] = ["MenuClose", "PlayPause", "CopyCheck", "PlusMinus"];

fn swap_icon(ui: &mut Ui, p: &mut Params) -> String {
    let pair = p.choice("Icon pair", &PAIRS, 0);
    let size = p.f32("Size", 48.0, 16.0..=110.0, 1.0);
    let duration = p.f32("Duration (s)", 0.25, 0.05..=1.0, 0.01);
    let start = p.f32("Start scale", 0.6, 0.0..=1.0, 0.01);
    let framed = p.bool("Framed", true);
    let mut on = flag(ui, "on", false);
    // Flip by itself so the preview always shows the swap.
    let t = glowkit::core::time(ui);
    let auto = ((t / 1.6) as i64) % 2 == 1;
    let auto_id = ui.scope_id().with("auto");
    let prev: bool = ui.data(|d| d.get_temp(auto_id)).unwrap_or(auto);
    if prev != auto {
        on = !on;
    }
    ui.data_mut(|d| d.insert_temp(auto_id, auto));
    glowkit::core::animate(ui);
    let pr = [SwapIconPair::MenuClose, SwapIconPair::PlayPause, SwapIconPair::CopyCheck, SwapIconPair::PlusMinus][pair];
    center(ui, |ui| {
        ui.add_space(70.0);
        ui.add(SwapIcon::new(&mut on, pr).size(size).duration(duration).start_scale(start).framed(framed))
    });
    set_flag(ui, "on", on);
    Code::new(
        "glowkit::interactions::{SwapIcon, SwapIconPair}",
        format!("SwapIcon::new(&mut open, {})", variant("SwapIconPair", &PAIRS, pair)),
    )
    .f("size", size)
    .f("duration", duration)
    .f("start_scale", start)
    .b("framed", framed)
    .add()
}

// --------------------------------------------------------------------------------------------
// Game UI
// --------------------------------------------------------------------------------------------

const STYLES: [&str; 3] = ["SciFi", "Fantasy", "Minimal"];

/// Dark game backdrop; returns its rect.
fn scene(ui: &mut Ui, h: f32, style: GameStyle) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    GameBackdrop::new(style).paint(ui, rect);
    rect
}

/// Run `f` centered inside `r`.
fn inside<R>(ui: &mut Ui, r: Rect, f: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child =
        ui.new_child(UiBuilder::new().id_salt("gk_pg_inside").max_rect(r).layout(Layout::top_down(Align::Center)));
    f(&mut child)
}

fn resource_bar(ui: &mut Ui, p: &mut Params) -> String {
    let kind = p.choice("Kind", &["Health", "Mana", "Stamina", "Shield"], 0);
    let style = p.choice("Style", &STYLES, 0);
    let current = p.f32("Current", 640.0, 0.0..=1000.0, 1.0);
    let segments = p.count("Segments", 10, 0..=20);
    let width = p.f32("Width", 360.0, 160.0..=600.0, 5.0);
    let numbers = p.bool("Numbers", true);
    let st = GameStyle::ALL[style];
    let k =
        [GameResourceKind::Health, GameResourceKind::Mana, GameResourceKind::Stamina, GameResourceKind::Shield][kind];
    let rect = scene(ui, 300.0, st);
    inside(ui, Rect::from_center_size(rect.center(), vec2(width + 60.0, 80.0)), |ui| {
        ui.add(GameResourceBar::new(current, 1000.0).kind(k).style(st).segments(segments).width(width).numbers(numbers))
    });
    Code::new(
        "glowkit::interactions::{GameResourceBar, GameResourceKind, GameStyle}",
        format!("GameResourceBar::new({}, 1000.0)", lit_f32(current)),
    )
    .call("kind", variant("GameResourceKind", &["Health", "Mana", "Stamina", "Shield"], kind))
    .call("style", variant("GameStyle", &STYLES, style))
    .call("segments", segments)
    .f("width", width)
    .b("numbers", numbers)
    .add()
}

fn boss_bar(ui: &mut Ui, p: &mut Params) -> String {
    let name = p.text("Name", "Vorrhak");
    let title = p.text("Title", "The Ashen Tyrant");
    let hp = p.f32("Health %", 58.0, 0.0..=100.0, 0.5);
    let armor = p.f32("Armor", 0.3, 0.0..=1.0, 0.01);
    let style = p.choice("Style", &STYLES, 1);
    let phases = p.bool("Phase notches", true);
    let st = GameStyle::ALL[style];
    let current = 80_000.0 * hp / 100.0;
    let rect = scene(ui, 300.0, st);
    let ph: &[f32] = if phases { &[0.66, 0.33] } else { &[] };
    inside(ui, Rect::from_center_size(rect.center(), vec2((rect.width() - 60.0).min(700.0), 110.0)), |ui| {
        ui.add(
            GameBossBar::new(&name, current, 80_000.0)
                .title(&title)
                .armor(armor)
                .phases(ph)
                .style(st)
                .width((rect.width() - 100.0).min(640.0)),
        )
    });
    Code::new(
        "glowkit::interactions::{GameBossBar, GameStyle}",
        format!("GameBossBar::new({}, {}, 80_000.0)", lit_str(&name), lit_f32(current)),
    )
    .s("title", &title)
    .f("armor", armor)
    .call("phases", if phases { "&[0.66, 0.33]" } else { "&[]" })
    .call("style", variant("GameStyle", &STYLES, style))
    .add()
}

const ICONS: [&str; 8] = ["Fire", "Frost", "Bolt", "Heal", "Dash", "Bomb", "Sword", "Shield"];

fn cooldown(ui: &mut Ui, p: &mut Params) -> String {
    let icon = p.choice("Icon", &ICONS, 0);
    let total = p.f32("Cooldown (s)", 6.0, 1.0..=20.0, 0.5);
    let key = p.text("Key", "Q");
    let charges = p.int("Charges", 2, 0..=5) as u32;
    let size = p.f32("Size", 72.0, 32.0..=120.0, 1.0);
    let style = p.choice("Style", &STYLES, 0);
    let st = GameStyle::ALL[style];
    let gi = [
        GameIcon::Fire,
        GameIcon::Frost,
        GameIcon::Bolt,
        GameIcon::Heal,
        GameIcon::Dash,
        GameIcon::Bomb,
        GameIcon::Sword,
        GameIcon::Shield,
    ][icon];
    let t = glowkit::core::time(ui);
    let period = total + 1.5;
    let ph = (t % period) - 1.5;
    let remaining = if ph < 0.0 { 0.0 } else { total - ph };
    glowkit::core::animate(ui);
    let rect = scene(ui, 300.0, st);
    inside(ui, Rect::from_center_size(rect.center(), vec2(size + 40.0, size + 40.0)), |ui| {
        let mut w = GameCooldownIcon::new(gi, remaining.max(0.0), total).key(&key).size(size).style(st);
        if charges > 0 {
            w = w.charges(charges);
        }
        ui.add(w)
    });
    Code::new(
        "glowkit::interactions::{GameCooldownIcon, GameIcon, GameStyle}",
        format!("GameCooldownIcon::new({}, remaining, {})", variant("GameIcon", &ICONS, icon), lit_f32(total)),
    )
    .s("key", &key)
    .call_if(charges > 0, "charges", charges)
    .f("size", size)
    .call("style", variant("GameStyle", &STYLES, style))
    .add()
}

const SHAPES: [&str; 6] = ["Cross", "Dot", "Circle", "Chevron", "Brackets", "Scope"];

fn crosshair(ui: &mut Ui, p: &mut Params) -> String {
    let shape = p.choice("Shape", &SHAPES, 0);
    let spread = p.f32("Spread", 0.3, 0.0..=1.5, 0.01);
    let color = p.color("Color", hex("#7CFFB2"));
    let size = p.f32("Size", 120.0, 40.0..=220.0, 1.0);
    let thickness = p.f32("Thickness", 2.0, 1.0..=5.0, 0.1);
    let hits = p.bool("Hit markers", true);
    let t = glowkit::core::time(ui);
    let hit_age = (t * 1.25).fract() / 1.25;
    let kill = ((t * 1.25) as u64) % 4 == 3;
    glowkit::core::animate(ui);
    let rect = scene(ui, 320.0, GameStyle::SciFi);
    let sh = GameCrosshairShape::ALL[shape];
    inside(ui, Rect::from_center_size(rect.center(), vec2(size + 20.0, size + 20.0)), |ui| {
        ui.add(
            GameCrosshair::new(sh)
                .spread(spread)
                .hit(if hits { Some(hit_age) } else { None }, kill)
                .color(color)
                .size(size)
                .thickness(thickness),
        )
    });
    let _ = egui::Id::NULL;
    Code::new(
        "glowkit::interactions::{GameCrosshair, GameCrosshairShape}",
        format!("GameCrosshair::new({})", variant("GameCrosshairShape", &SHAPES, shape)),
    )
    .f("spread", spread)
    .call_if(hits, "hit", "Some(secs_since_hit), was_kill")
    .c("color", color)
    .f("size", size)
    .f("thickness", thickness)
    .add()
}
