use super::*;
use egui::{pos2, vec2, Align, Align2, Color32, FontId, Layout, Rect, Sense, Ui, UiBuilder, Vec2};
use glowkit_core::color::{hex, with_alpha};
use glowkit_core::demo::{Category, Demo};
use glowkit_core::rng::hash01;
use glowkit_core::{paint, Theme};
use glowkit_effects::ControlSlider;

const C: Category = Category::Games;

fn mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, init: impl FnOnce() -> T) -> T {
    glowkit_core::state(ui, ui.scope_id().with(key), init)
}

fn set_mem<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, v: T) {
    glowkit_core::set_state(ui, ui.scope_id().with(key), v);
}

fn now(ui: &Ui) -> f64 {
    ui.input(|i| i.time)
}

/// A row of three style pills; returns the chosen style.
fn style_tabs(ui: &mut Ui, default: usize) -> GameStyle {
    let t = Theme::of(ui);
    let mut sel: usize = mem(ui, "style_tab", || default);
    let w = 3.0 * 92.0 + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    let x0 = rect.center().x - w / 2.0;
    let bg = Rect::from_min_size(pos2(x0 - 4.0, rect.min.y + 2.0), vec2(w + 8.0, 30.0));
    ui.painter().rect_filled(bg, 15.0, t.muted);
    let hx = glowkit_core::anim::spring(ui, ui.scope_id().with("style_hx"), sel as f32, 400.0, 28.0);
    let hl = Rect::from_min_size(pos2(x0 + hx * 92.0 + 2.0, rect.min.y + 5.0), vec2(88.0, 24.0));
    ui.painter().rect_filled(hl, 12.0, t.foreground);
    for (i, s) in GameStyle::ALL.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x0 + i as f32 * 92.0 + 2.0, rect.min.y + 5.0), vec2(88.0, 24.0));
        let resp = ui.interact(r, ui.scope_id().with(("style_pill", i)), Sense::click());
        if resp.clicked() {
            sel = i;
        }
        let on = (hx - i as f32).abs() < 0.5;
        ui.painter().text(r.center(), Align2::CENTER_CENTER, s.label(), FontId::proportional(12.5), if on { t.background } else { t.muted_foreground });
    }
    set_mem(ui, "style_tab", sel);
    ui.add_space(6.0);
    GameStyle::ALL[sel]
}

/// Allocate a full-width scene and paint the game viewport behind.
fn scene(ui: &mut Ui, h: f32, style: GameStyle) -> Rect {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    GameBackdrop::new(style).paint(ui, rect);
    rect
}

/// Run `f` in a child ui laid out top-down and centered within `r`.
fn inside(ui: &mut Ui, r: Rect, f: impl FnOnce(&mut Ui)) {
    ui.scope_builder(UiBuilder::new().max_rect(r).layout(Layout::top_down(Align::Center)), f);
}

fn controls(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(14.0, 8.0);
        f(ui);
    });
}

// ---------------------------------------------------------------------------------------------
// Sample data

fn items() -> Vec<GameItem> {
    vec![
        GameItem::new("Dawnbreaker", GameIcon::Sword, GameRarity::Legendary).kind("Sword").stat("Damage", 148.0).stat("Crit chance", 12.0).stat("Fire damage", 40.0).flavor("Forged in the last light of a dying sun."),
        GameItem::new("Warden's Aegis", GameIcon::Shield, GameRarity::Epic).kind("Shield").stat("Armor", 96.0).stat("Block", 24.0).flavor("Still warm from the siege of Halveth."),
        GameItem::new("Healing Draught", GameIcon::Potion, GameRarity::Common).kind("Potion").count(12).stat("Restores", 250.0),
        GameItem::new("Arcane Tonic", GameIcon::ManaPotion, GameRarity::Uncommon).kind("Potion").count(5).stat("Restores", 180.0),
        GameItem::new("Stormstring", GameIcon::Bow, GameRarity::Rare).kind("Bow").stat("Damage", 92.0).stat("Range", 40.0),
        GameItem::new("Iron Helm", GameIcon::Helmet, GameRarity::Common).kind("Helmet").stat("Armor", 22.0),
        GameItem::new("Voidcut Gem", GameIcon::Gem, GameRarity::Epic).kind("Gem").count(3),
        GameItem::new("Rusty Blade", GameIcon::Sword, GameRarity::Common).kind("Sword").stat("Damage", 31.0).stat("Crit chance", 2.0),
        GameItem::new("Gold", GameIcon::Coin, GameRarity::Common).count(240),
        GameItem::new("Ember Rune", GameIcon::Fire, GameRarity::Rare).kind("Rune").stat("Fire damage", 18.0),
        GameItem::new("Vault Key", GameIcon::Key, GameRarity::Uncommon).kind("Quest item"),
        GameItem::new("Bloodstone Ring", GameIcon::Ring, GameRarity::Rare).kind("Ring").stat("Life steal", 4.0),
        GameItem::new("Old Map", GameIcon::Map, GameRarity::Uncommon).kind("Quest item"),
        GameItem::new("Black Powder Bomb", GameIcon::Bomb, GameRarity::Common).kind("Throwable").count(4).stat("Damage", 120.0),
        GameItem::new("Trailstriders", GameIcon::Boots, GameRarity::Uncommon).kind("Boots").stat("Armor", 14.0).stat("Move speed", 6.0),
    ]
}

fn equipped() -> Vec<GameItem> {
    vec![
        GameItem::new("Knight's Longsword", GameIcon::Sword, GameRarity::Rare).kind("Sword").stat("Damage", 112.0).stat("Crit chance", 8.0).stat("Fire damage", 0.0),
        GameItem::new("Oak Buckler", GameIcon::Shield, GameRarity::Uncommon).kind("Shield").stat("Armor", 61.0).stat("Block", 30.0),
        GameItem::new("Hunter's Bow", GameIcon::Bow, GameRarity::Uncommon).kind("Bow").stat("Damage", 70.0).stat("Range", 45.0),
    ]
}

fn hotbar_slots() -> Vec<Option<GameItem>> {
    let it = items();
    vec![Some(it[0].clone()), Some(it[4].clone()), Some(it[2].clone()), Some(it[3].clone()), Some(it[13].clone()), None, Some(it[9].clone()), Some(it[10].clone())]
}

fn skill_nodes() -> Vec<GameSkillNode> {
    vec![
        GameSkillNode::new("Kindle", GameIcon::Fire, vec2(0.5, 0.92)).desc("Your attacks ignite foes for 3s.").unlocked(true),
        GameSkillNode::new("Fireball", GameIcon::Fire, vec2(0.22, 0.6)).desc("Hurl a fireball that explodes on impact.").requires(&[0]),
        GameSkillNode::new("Frost Nova", GameIcon::Frost, vec2(0.5, 0.58)).desc("Freeze enemies around you for 2s.").requires(&[0]),
        GameSkillNode::new("Chain Bolt", GameIcon::Bolt, vec2(0.78, 0.6)).desc("Lightning arcs between up to 4 targets.").requires(&[0]),
        GameSkillNode::new("Inferno", GameIcon::Fire, vec2(0.12, 0.22)).desc("Fireballs leave burning ground behind.").requires(&[1]).cost(2),
        GameSkillNode::new("Mend", GameIcon::Heal, vec2(0.36, 0.24)).desc("Kills restore 4% health.").requires(&[1, 2]),
        GameSkillNode::new("Blink", GameIcon::Dash, vec2(0.64, 0.24)).desc("Teleport a short distance. 8s cooldown.").requires(&[2, 3]),
        GameSkillNode::new("Tempest", GameIcon::Bolt, vec2(0.88, 0.22)).desc("Chain Bolt strikes twice.").requires(&[3]).cost(2),
        GameSkillNode::new("Archmage", GameIcon::Star, vec2(0.5, 0.02)).desc("All spells cost 20% less mana.").requires(&[5, 6]).cost(3),
    ]
}

fn quests(step: usize) -> Vec<GameQuest> {
    let s = step as u32;
    let mut v = vec![
        GameQuest::new("The Sunken Vault")
            .main(true)
            .giver("Maren the Smith")
            .desc("Maren believes the old vault beneath the harbor still holds her father's forge-hammer. Find a way past the flooded halls.")
            .objective("Find the vault entrance", 1, 1)
            .objective("Recover the rune keys", (s / 2).min(3), 3)
            .objective("Open the vault door", if s >= 8 { 1 } else { 0 }, 1)
            .rewards(4200, 350, vec![items()[0].clone(), items()[6].clone()]),
        GameQuest::new("Wolves at the Gate")
            .giver("Captain Hollis")
            .desc("Dire wolves have been raiding the east farms at night. Thin the pack before the harvest.")
            .objective("Slay dire wolves", s.min(5), 5)
            .objective("Report to Captain Hollis", if s >= 6 { 1 } else { 0 }, 1)
            .rewards(900, 120, vec![items()[14].clone()]),
    ];
    if s >= 3 {
        v.push(
            GameQuest::new("A Letter for Ysolde")
                .giver("Courier")
                .desc("Deliver a sealed letter to Ysolde at the lighthouse.")
                .objective("Deliver the letter", if s >= 9 { 1 } else { 0 }, 1)
                .rewards(300, 40, vec![]),
        );
    }
    v
}

fn roster() -> Vec<GameCharacter> {
    vec![
        GameCharacter::new("Aria", "Vanguard", GamePortrait::hero()).bio("A shieldbearer who never takes a step back.").stat("Power", 0.65).stat("Defense", 0.9).stat("Speed", 0.45).stat("Magic", 0.2),
        GameCharacter::new("Thessaly", "Arcanist", GamePortrait::mage()).bio("Reads storms like scripture and writes them back.").stat("Power", 0.85).stat("Defense", 0.3).stat("Speed", 0.5).stat("Magic", 1.0),
        GameCharacter::new("Kestrel", "Ranger", GamePortrait::rogue()).bio("Silent, patient, and never misses twice.").stat("Power", 0.6).stat("Defense", 0.4).stat("Speed", 0.95).stat("Magic", 0.3),
        GameCharacter::new("Unit 7", "Sentinel", GamePortrait::trooper()).bio("A decommissioned guardian frame with a new purpose.").stat("Power", 0.75).stat("Defense", 0.75).stat("Speed", 0.6).stat("Magic", 0.1),
        GameCharacter::new("Old Bram", "Smith", GamePortrait::elder()).bio("Swings a hammer heavier than most doors.").stat("Power", 1.0).stat("Defense", 0.7).stat("Speed", 0.25).stat("Magic", 0.15),
    ]
}

fn dialogue() -> Vec<GameDialogueLine> {
    vec![
        GameDialogueLine::new("Maren", GamePortrait::elder(), "You came back. Good. The tide is low tonight, which means the vault doors are above water for the first time in nine years."),
        GameDialogueLine::new("Maren", GamePortrait::elder(), "My father sealed his forge-hammer down there. I need it back before the Guild does. Will you help me?")
            .choice("I'll find your hammer.")
            .choice("What's in it for me?")
            .choice("Tell me more about the Guild."),
        GameDialogueLine::new("Kestrel", GamePortrait::rogue(), "Careful. I counted six Guild sentries on the pier, and they are not here for the fishing."),
        GameDialogueLine::new("Unit 7", GamePortrait::trooper(), "Scan complete. Structural integrity of the vault: 41 percent. Recommend haste.")
            .choice("Lead the way.")
            .choice("We wait for nightfall."),
    ]
}

// ---------------------------------------------------------------------------------------------
// Simulations

#[derive(Clone, Debug)]
struct BarSim {
    hp: f32,
    mp: f32,
    sp: f32,
    next: f64,
    n: u64,
}

fn step_bars(ui: &Ui, s: &mut BarSim, dmg: f32) {
    let t = now(ui);
    if t >= s.next {
        s.n += 1;
        let r = hash01(s.n * 7);
        if s.hp < 260.0 || r < 0.22 {
            s.hp = (s.hp + 180.0 + 200.0 * hash01(s.n * 3)).min(1000.0);
            s.mp = (s.mp + 120.0).min(600.0);
        } else {
            s.hp = (s.hp - dmg * (0.4 + hash01(s.n * 5) * 1.2)).max(0.0);
            s.mp = (s.mp - 40.0 - 80.0 * hash01(s.n * 11)).max(0.0);
            s.sp = (s.sp - 30.0).max(0.0);
        }
        s.next = t + 0.9 + 0.8 * hash01(s.n * 13) as f64;
    }
    s.sp = (s.sp + glowkit_core::dt(ui) * 25.0).min(200.0);
    glowkit_core::animate(ui);
}

#[derive(Clone, Debug)]
struct GunSim {
    mag: u32,
    reserve: u32,
    reload_at: Option<f64>,
    next_shot: f64,
    burst: u32,
}

fn step_gun(ui: &Ui, g: &mut GunSim, size: u32, seed: u64) -> Option<f32> {
    let t = now(ui);
    glowkit_core::animate(ui);
    if let Some(r) = g.reload_at {
        let p = ((t - r) / 1.6) as f32;
        if p >= 1.0 {
            let need = size - g.mag;
            if g.reserve < need {
                g.reserve += 120;
            }
            g.reserve -= need;
            g.mag = size;
            g.reload_at = None;
            g.next_shot = t + 0.8;
        } else {
            return Some(p);
        }
    } else if t >= g.next_shot {
        if g.mag == 0 {
            g.reload_at = Some(t + 0.5);
        } else {
            g.mag -= 1;
            g.burst += 1;
            let pause = if g.burst % 6 == 0 { 0.9 + hash01(g.burst as u64 + seed) as f64 } else { 0.11 };
            g.next_shot = t + pause;
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "game-hud",
            name: "Game HUD",
            category: C,
            description: "A full shooter / RPG HUD over a live scene: health and mana with damage trails, ability cooldowns, ammo, minimap, kill feed and a dynamic crosshair, all driven by simulated gameplay.",
            covers: &[],
            code: r##"// every block takes plain game state each frame
ui.add(GameResourceBar::new(hp, 1000.0).kind(GameResourceKind::Health));
ui.add(GameCooldownIcon::new(GameIcon::Fire, cd, 6.0).key("Q"));
ui.add(GameAmmoCounter::new(mag, 30, reserve).reloading(reload));
ui.add(GameMinimap::new(player, heading).markers(&markers));
ui.add(GameKillFeed::new(&kills));
ui.add(GameCrosshair::new(GameCrosshairShape::Cross).spread(spread));"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let rect = scene(ui, 440.0, style);
                let t = now(ui);
                let mut bars = mem(ui, "bars", || BarSim { hp: 820.0, mp: 420.0, sp: 160.0, next: 0.0, n: 0 });
                step_bars(ui, &mut bars, 160.0);
                set_mem(ui, "bars", bars.clone());
                let mut gun = mem(ui, "gun", || GunSim { mag: 30, reserve: 120, reload_at: None, next_shot: 0.0, burst: 0 });
                let reload = step_gun(ui, &mut gun, 30, 3);
                set_mem(ui, "gun", gun.clone());
                // HP/MP bottom-left
                let bl = Rect::from_min_size(pos2(rect.min.x + 18.0, rect.max.y - 104.0), vec2(330.0, 90.0));
                inside(ui, bl, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add(GameResourceBar::new(bars.hp, 1000.0).style(style).width(250.0));
                    ui.add(GameResourceBar::new(bars.mp, 600.0).kind(GameResourceKind::Mana).style(style).width(210.0).height(12.0).numbers(false));
                });
                // cooldowns bottom-center
                let cds = [(GameIcon::Fire, 6.0, "Q"), (GameIcon::Frost, 9.0, "E"), (GameIcon::Dash, 4.0, "⇧"), (GameIcon::Heal, 14.0, "R")];
                let cw = 4.0 * 60.0;
                let cr = Rect::from_min_size(pos2(rect.center().x - cw / 2.0, rect.max.y - 74.0), vec2(cw, 64.0));
                inside(ui, cr, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for (i, (icon, total, key)) in cds.iter().enumerate() {
                            let period = total + 1.5 + i as f32;
                            let ph = ((t as f32 + i as f32 * 1.7) % period) - 1.5;
                            let rem = if ph < 0.0 { 0.0 } else { total - ph };
                            ui.add(GameCooldownIcon::new(*icon, rem.max(0.0), *total).key(key).size(52.0).style(style));
                        }
                    });
                });
                // ammo bottom-right
                ui.put(Rect::from_min_size(pos2(rect.max.x - 280.0, rect.max.y - 118.0), vec2(270.0, 108.0)), GameAmmoCounter::new(gun.mag, 30, gun.reserve).reloading(reload).weapon("AR-7 Vanguard").style(style));
                // minimap top-right
                let path = |k: f64| vec2((k * 0.3).cos() as f32 * 160.0 + (k * 0.11).sin() as f32 * 60.0, (k * 0.23).sin() as f32 * 140.0);
                let player = path(t);
                let ahead = path(t + 0.05);
                let heading = (ahead.x - player.x).atan2(-(ahead.y - player.y));
                let markers = [
                    GameMapMarker::new(GameMarkerKind::Objective, vec2(320.0, -260.0)),
                    GameMapMarker::new(GameMarkerKind::Enemy, player + vec2(60.0 * (t as f32 * 0.7).sin(), -70.0)),
                    GameMapMarker::new(GameMarkerKind::Enemy, player + vec2(-90.0, 40.0 * (t as f32).cos())),
                    GameMapMarker::new(GameMarkerKind::Ally, player + vec2(30.0, 50.0)),
                    GameMapMarker::new(GameMarkerKind::Ping, player + vec2(-40.0, -90.0)).age((t as f32) % 4.5),
                ];
                ui.put(Rect::from_min_size(pos2(rect.max.x - 170.0, rect.min.y + 10.0), vec2(160.0, 160.0)), GameMinimap::new(player, heading).markers(&markers).size(140.0).style(style).zoom(260.0));
                // kill feed top-left
                let names = ["Vex", "Halcyon", "Nyx", "Brannoc", "Quill", "Sable", "Orrin", "Talon"];
                let weapons = [GameIcon::Rifle, GameIcon::Pistol, GameIcon::Bomb, GameIcon::Sword];
                let k0 = (t / 1.7) as u64;
                let kills: Vec<GameKillEntry> = (0..4u64)
                    .rev()
                    .map(|j| {
                        let k = k0.saturating_sub(j);
                        let age = (t - k as f64 * 1.7) as f32;
                        GameKillEntry::new(names[(k * 3 % 8) as usize], names[(k * 5 + 1) as usize % 8], weapons[(k % 4) as usize], age)
                            .headshot(k % 3 == 0)
                            .ally(k % 2 == 0)
                            .own(k % 5 == 0)
                    })
                    .collect();
                ui.put(Rect::from_min_size(rect.min + vec2(14.0, 12.0), vec2(330.0, 130.0)), GameKillFeed::new(&kills).width(330.0).max_rows(4).life(6.0));
                // crosshair
                let firing = reload.is_none() && (t - (gun.next_shot - 0.11)).abs() < 0.3;
                let hit_age = ((t * 1.3).fract() / 1.3) as f32;
                ui.put(Rect::from_center_size(rect.center(), Vec2::splat(96.0)), GameCrosshair::new(GameCrosshairShape::Cross).spread(if firing { 0.7 } else { 0.1 }).hit(Some(hit_age), (t as u64) % 4 == 0));
            },
        },
        Demo {
            id: "game-main-menu",
            name: "Main Menu",
            category: C,
            description: "A title screen over a parallax scene: the title drops in letter by letter and breathes with a glow, entries stagger in and slide on hover, and every hover / confirm is reported as a sound cue.",
            covers: &[],
            code: r##"let mut cues = Vec::new();
ui.add(GameMainMenu::new(&mut sel, &["Continue", "New Game", "Settings", "Credits", "Quit"])
    .title("Astral Drift")
    .subtitle("Chapter III · The Sunken Vault")
    .style(GameStyle::SciFi)
    .activated(&mut chosen)
    .cues(&mut cues));
for cue in cues { audio.play(cue); }"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let mut sel: usize = mem(ui, "sel", || 0);
                let mut cues = Vec::new();
                let title = match style {
                    GameStyle::SciFi => "Astral Drift",
                    GameStyle::Fantasy => "Emberfall",
                    GameStyle::Minimal => "Lumen",
                };
                let r = ui.add(
                    GameMainMenu::new(&mut sel, &["Continue", "New Game", "Settings", "Credits", "Quit"])
                        .title(title)
                        .subtitle("Chapter III · The Sunken Vault")
                        .version("v0.9.2")
                        .style(style)
                        .autoplay(true)
                        .cues(&mut cues),
                );
                set_mem(ui, "sel", sel);
                // cue indicator
                let t = now(ui);
                let mut last: (String, f64) = mem(ui, "cue", || (String::new(), -10.0));
                if let Some(c) = cues.last() {
                    last = (format!("{c:?}"), t);
                    set_mem(ui, "cue", last.clone());
                }
                let age = (t - last.1) as f32;
                if age < 1.2 && !last.0.is_empty() {
                    let a = 1.0 - age / 1.2;
                    let p = r.rect.right_top() + vec2(-16.0, 16.0);
                    let label = format!("♪ cue: {}", last.0);
                    let g = ui.painter().layout_no_wrap(label, FontId::monospace(11.0), with_alpha(Color32::WHITE, a));
                    let rr = Align2::RIGHT_TOP.anchor_size(p, g.size()).expand2(vec2(10.0, 5.0));
                    ui.painter().rect_filled(rr, 10.0, Color32::from_black_alpha((160.0 * a) as u8));
                    ui.painter().galley(rr.min + vec2(10.0, 5.0), g, Color32::WHITE);
                }
            },
        },
        Demo {
            id: "game-pause-menu",
            name: "Pause Menu",
            category: C,
            description: "A pause overlay: the running scene slows to a stop and dims into a vignette while the panel springs in with play info and staggered menu entries.",
            covers: &[],
            code: r##"ui.add(GamePauseMenu::new(paused, &mut sel, &["Resume", "Settings", "Load Game", "Quit to Title"])
    .info(&[("Chapter", "III · The Sunken Vault"), ("Play time", "12h 04m")])
    .activated(&mut chosen));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let mut sel: usize = mem(ui, "sel", || 0);
                let t = now(ui);
                let open = (t % 7.0) > 1.6;
                let mut chosen = None;
                ui.add(
                    GamePauseMenu::new(open, &mut sel, &["Resume", "Settings", "Load Game", "Quit to Title"])
                        .info(&[("Chapter", "III · The Sunken Vault"), ("Play time", "12h 04m"), ("Difficulty", "Veteran")])
                        .style(style)
                        .height(420.0)
                        .backdrop(true)
                        .activated(&mut chosen),
                );
                set_mem(ui, "sel", sel);
            },
        },
        Demo {
            id: "game-settings-panel",
            name: "Settings Panel",
            category: C,
            description: "Tabbed game settings for graphics, audio and controls: segmented sliders, springy toggles, arrow selectors and key rebind rows that capture the next key you press.",
            covers: &[],
            code: r##"let mut settings = GameSettings::default();
ui.add(GameSettingsPanel::new(&mut settings).style(GameStyle::SciFi).cues(&mut cues));"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let rect = scene(ui, 470.0, style);
                let mut s: GameSettings = mem(ui, "s", GameSettings::default);
                inside(ui, rect.shrink2(vec2(20.0, 12.0)), |ui| {
                    ui.add(GameSettingsPanel::new(&mut s).style(style).width(600.0).autoplay(true));
                });
                set_mem(ui, "s", s);
            },
        },
        Demo {
            id: "game-resource-bars",
            name: "Health & Mana Bars",
            category: C,
            description: "Health, mana and stamina bars in three skins: damage cuts the fill and leaves a delayed trail that drains after a beat (with sparks and shake), heals pour in with a green ghost and rising pluses, low health pulses red.",
            covers: &[],
            code: r##"ui.add(GameResourceBar::new(hp, 1000.0).kind(GameResourceKind::Health).style(GameStyle::SciFi).segments(10));
ui.add(GameResourceBar::new(mp, 600.0).kind(GameResourceKind::Mana).style(GameStyle::SciFi));"##,
            show: |ui| {
                let mut p: [f32; 2] = mem(ui, "p", || [160.0, 4.0]);
                let rect = scene(ui, 360.0, GameStyle::Minimal);
                let colw = (rect.width() - 40.0) / 3.0;
                for (i, style) in GameStyle::ALL.iter().enumerate() {
                    let key = format!("sim{i}");
                    let mut sim = mem(ui, &key, || BarSim { hp: 900.0 - 150.0 * i as f32, mp: 500.0, sp: 150.0, next: i as f64 * 0.4, n: i as u64 * 100 });
                    step_bars(ui, &mut sim, p[0]);
                    set_mem(ui, &key, sim.clone());
                    let col = Rect::from_min_size(pos2(rect.min.x + 20.0 + i as f32 * colw, rect.min.y + 30.0), vec2(colw, rect.height() - 40.0));
                    inside(ui, col, |ui| {
                        ui.label(egui::RichText::new(style.label().to_uppercase()).monospace().size(11.0).color(with_alpha(Color32::WHITE, 0.6)));
                        ui.add_space(14.0);
                        let w = (colw - 70.0).min(230.0);
                        ui.add(GameResourceBar::new(sim.hp, 1000.0).style(*style).width(w).shake(p[1]));
                        ui.add_space(8.0);
                        ui.add(GameResourceBar::new(sim.mp, 600.0).kind(GameResourceKind::Mana).style(*style).width(w).shake(0.0));
                        ui.add_space(8.0);
                        ui.add(GameResourceBar::new(sim.sp, 200.0).kind(GameResourceKind::Stamina).style(*style).width(w).height(12.0).segments(4).numbers(false).shake(0.0));
                        ui.add_space(8.0);
                        ui.add(GameResourceBar::new(sim.hp * 0.3, 300.0).kind(GameResourceKind::Shield).style(*style).width(w).height(10.0).segments(0).numbers(false).shake(0.0));
                    });
                }
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Hit size", &mut p[0], 40.0..=400.0).width(220.0));
                    ui.add(ControlSlider::new("Shake", &mut p[1], 0.0..=12.0).width(220.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-boss-bar",
            name: "Boss Bar",
            category: C,
            description: "A wide boss health bar with the name glowing above, phase notches that fire a flash, ring and sparks when crossed, an armor layer and an enraged pulse in the final phase.",
            covers: &[],
            code: r##"ui.add(GameBossBar::new("Vorrhak", hp, 80_000.0)
    .title("The Ashen Tyrant")
    .phases(&[0.66, 0.33])
    .armor(armor));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let mut p: [f32; 1] = mem(ui, "p", || [3000.0]);
                let rect = scene(ui, 300.0, style);
                let t = now(ui);
                let mut st: (f32, f64, u64) = mem(ui, "boss", || (80_000.0, 0.0, 0));
                if t >= st.1 {
                    st.2 += 1;
                    if st.0 <= 0.0 {
                        st.0 = 80_000.0;
                        st.1 = t + 1.5;
                    } else {
                        st.0 = (st.0 - p[0] * (0.5 + hash01(st.2) * 1.5)).max(0.0);
                        st.1 = t + if st.0 <= 0.0 { 2.5 } else { 0.5 + 0.5 * hash01(st.2 * 3) as f64 };
                    }
                }
                set_mem(ui, "boss", st);
                glowkit_core::animate(ui);
                let armor = ((st.0 - 60_000.0) / 20_000.0).clamp(0.0, 1.0) * 0.6;
                ui.put(Rect::from_min_size(rect.min + vec2(20.0, 26.0), vec2(rect.width() - 40.0, 90.0)), GameBossBar::new("Vorrhak", st.0, 80_000.0).title("The Ashen Tyrant").phases(&[0.66, 0.33]).armor(armor).style(style).width((rect.width() - 80.0).min(640.0)));
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Hit size", &mut p[0], 500.0..=12000.0).width(260.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-minimap",
            name: "Minimap",
            category: C,
            description: "Round and square minimaps with procedural terrain, fog of war that clears along the explored trail, a player arrow with view cone, pulsing enemies, rippling pings and objectives pinned to the rim.",
            covers: &[],
            code: r##"ui.add(GameMinimap::new(player, heading)
    .markers(&markers)
    .explored(&trail, 70.0)
    .round(true)
    .rotate(true)
    .size(180.0));"##,
            show: |ui| {
                let mut p: [f32; 1] = mem(ui, "p", || [220.0]);
                let rect = scene(ui, 290.0, GameStyle::Minimal);
                let t = now(ui);
                let path = |k: f64| vec2((k * 0.25).cos() as f32 * 220.0 + (k * 0.09).sin() as f32 * 90.0, (k * 0.19).sin() as f32 * 200.0);
                let player = path(t);
                let ahead = path(t + 0.05);
                let heading = (ahead.x - player.x).atan2(-(ahead.y - player.y));
                let mut trail: Vec<Vec2> = mem(ui, "trail", Vec::new);
                if trail.last().is_none_or(|l| (*l - player).length() > 30.0) {
                    trail.push(player);
                    if trail.len() > 80 {
                        trail.remove(0);
                    }
                    set_mem(ui, "trail", trail.clone());
                }
                let markers = [
                    GameMapMarker::new(GameMarkerKind::Objective, vec2(420.0, -300.0)),
                    GameMapMarker::new(GameMarkerKind::Enemy, player + vec2(80.0 * (t as f32 * 0.6).sin(), -60.0)),
                    GameMapMarker::new(GameMarkerKind::Enemy, player + vec2(-70.0, 70.0 * (t as f32 * 0.8).cos())),
                    GameMapMarker::new(GameMarkerKind::Ally, player + vec2(40.0, 30.0)),
                    GameMapMarker::new(GameMarkerKind::Loot, player + vec2(-30.0, -110.0)),
                    GameMapMarker::new(GameMarkerKind::Ping, player + vec2(100.0, 60.0)).age((t as f32) % 5.0),
                ];
                let cw = rect.width() / 3.0;
                let configs = [(GameStyle::SciFi, true, true), (GameStyle::Fantasy, false, false), (GameStyle::Minimal, true, false)];
                for (i, (style, round, rotate)) in configs.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(rect.min.x + i as f32 * cw, rect.min.y), vec2(cw, rect.height()));
                    ui.put(r, GameMinimap::new(player, heading).markers(&markers).explored(&trail, 80.0).round(*round).rotate(*rotate).style(*style).zoom(p[0]).size(190.0f32.min(cw - 30.0)));
                }
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Zoom", &mut p[0], 100.0..=500.0).suffix(" m").width(240.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-ammo-counter",
            name: "Ammo Counter",
            category: C,
            description: "Magazine and reserve counters: each shot kicks the number and ejects a spinning casing, low ammo pulses amber, an empty magazine flashes RELOAD and the reload sweeps rounds back in.",
            covers: &[],
            code: r##"ui.add(GameAmmoCounter::new(mag, 30, reserve)
    .reloading(reload_progress)
    .weapon("AR-7 Vanguard")
    .cues(&mut cues));"##,
            show: |ui| {
                let rect = scene(ui, 250.0, GameStyle::Minimal);
                let cfg = [(GameStyle::SciFi, 30u32, GameIcon::Rifle, "AR-7 Vanguard"), (GameStyle::Fantasy, 8, GameIcon::Bow, "Stormstring"), (GameStyle::Minimal, 12, GameIcon::Pistol, "P-12 Wasp")];
                let cw = rect.width() / 3.0;
                for (i, (style, size, icon, name)) in cfg.iter().enumerate() {
                    let key = format!("gun{i}");
                    let mut g = mem(ui, &key, || GunSim { mag: *size, reserve: 90, reload_at: None, next_shot: i as f64 * 0.5, burst: 0 });
                    let reload = step_gun(ui, &mut g, *size, i as u64 * 7);
                    set_mem(ui, &key, g.clone());
                    let r = Rect::from_min_size(pos2(rect.min.x + i as f32 * cw, rect.min.y), vec2(cw, rect.height()));
                    ui.put(r, GameAmmoCounter::new(g.mag, *size, g.reserve).reloading(reload).icon(*icon).weapon(name).style(*style));
                }
            },
        },
        Demo {
            id: "game-cooldown-icons",
            name: "Cooldown Icons",
            category: C,
            description: "Ability slots with a radial cooldown sweep, a bright hand on the edge and seconds left; coming off cooldown flashes white, pops on a spring and throws a ring of sparks.",
            covers: &[],
            code: r##"ui.add(GameCooldownIcon::new(GameIcon::Fire, remaining, 8.0).key("Q").charges(2));"##,
            show: |ui| {
                let mut p: [f32; 1] = mem(ui, "p", || [60.0]);
                let rect = scene(ui, 300.0, GameStyle::Minimal);
                let t = now(ui) as f32;
                let abil = [(GameIcon::Fire, 5.0, "Q"), (GameIcon::Frost, 8.0, "E"), (GameIcon::Bolt, 3.5, "R"), (GameIcon::Dash, 2.5, "⇧"), (GameIcon::Heal, 11.0, "F"), (GameIcon::Bomb, 6.5, "G")];
                let rows = rect.height() / 3.0;
                for (si, style) in GameStyle::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(rect.min.x, rect.min.y + si as f32 * rows), vec2(rect.width(), rows));
                    inside(ui, r.shrink2(vec2(0.0, (rows - p[0] - 10.0).max(0.0) / 2.0)), |ui| {
                        ui.horizontal(|ui| {
                            let total_w = abil.len() as f32 * (p[0] + 14.0);
                            ui.add_space((ui.available_width() - total_w).max(0.0) / 2.0);
                            for (i, (icon, total, key)) in abil.iter().enumerate() {
                                let period = total + 1.2 + i as f32 * 0.4;
                                let ph = ((t + i as f32 * 0.9 + si as f32 * 0.5) % period) - 1.2;
                                let rem = if ph < 0.0 { 0.0 } else { total - ph };
                                let mut w = GameCooldownIcon::new(*icon, rem.max(0.0), *total).key(key).size(p[0]).style(*style);
                                if i == 3 {
                                    w = w.charges(2);
                                }
                                if i == 4 && si == 2 {
                                    w = w.disabled(true);
                                }
                                ui.add(w);
                            }
                        });
                    });
                }
                glowkit_core::animate(ui);
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Size", &mut p[0], 40.0..=80.0).width(220.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-hotbar",
            name: "Hotbar",
            category: C,
            description: "Item hotbars where the selected slot springs up and grows with a glow beneath, its name popping up above; select by click, scroll wheel or number keys.",
            covers: &[],
            code: r##"ui.add(GameHotbar::new(&mut selected, &slots).keyboard(true).style(GameStyle::Fantasy));"##,
            show: |ui| {
                let rect = scene(ui, 360.0, GameStyle::Minimal);
                let slots = hotbar_slots();
                let t = now(ui);
                let rows = rect.height() / 3.0;
                for (i, style) in GameStyle::ALL.iter().enumerate() {
                    let key = format!("sel{i}");
                    let auto = ((t / 1.3 + i as f64 * 0.7) as usize) % slots.len();
                    let mut sel: usize = mem(ui, &key, || 0);
                    let r = Rect::from_min_size(pos2(rect.min.x, rect.min.y + i as f32 * rows), vec2(rect.width(), rows));
                    let hovered = ui.rect_contains_pointer(r);
                    if !hovered {
                        sel = auto;
                    }
                    ui.put(r, GameHotbar::new(&mut sel, &slots).slot_size(48.0).style(*style));
                    set_mem(ui, &key, sel);
                }
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "game-inventory",
            name: "Inventory Grid",
            category: C,
            description: "Drag-and-drop inventory: items glow by rarity (legendaries run a light beam around their slot), lift out with a lagging tilt, swap with a springy landing, and show a tooltip card comparing stats with your equipped gear.",
            covers: &[],
            code: r##"ui.add(GameInventoryGrid::new(&mut bag, 6)
    .title("Backpack")
    .equipped(&equipped)
    .style(GameStyle::Fantasy));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let rect = scene(ui, 400.0, style);
                let mut bag: Vec<Option<GameItem>> = mem(ui, "bag", || {
                    let it = items();
                    let mut v: Vec<Option<GameItem>> = vec![None; 24];
                    for (i, slot) in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14].iter().zip([0usize, 2, 3, 7, 8, 9, 12, 13, 14, 16, 19, 20, 21, 23]) {
                        v[slot] = Some(it[*i].clone());
                    }
                    v
                });
                let eq = equipped();
                let r = Rect::from_min_size(rect.min + vec2(20.0, 0.0), vec2(rect.width() * 0.62, rect.height()));
                ui.put(r, GameInventoryGrid::new(&mut bag, 6).title("Backpack").equipped(&eq).style(style).autoplay(true).cell_size(50.0));
                set_mem(ui, "bag", bag);
            },
        },
        Demo {
            id: "game-item-tooltip",
            name: "Item Tooltip",
            category: C,
            description: "Item tooltip cards with a rarity-tinted header, stats compared to the equipped item (green and red arrows with deltas) and flavor text, in all three skins.",
            covers: &[],
            code: r##"ui.add(GameItemTooltip::new(&item).compare(Some(&equipped_sword)).style(GameStyle::SciFi));"##,
            show: |ui| {
                let rect = scene(ui, 300.0, GameStyle::Minimal);
                let it = items();
                let eq = equipped();
                let pairs = [(&it[0], &eq[0]), (&it[1], &eq[1]), (&it[4], &eq[2])];
                let cw = rect.width() / 3.0;
                for (i, style) in GameStyle::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(rect.min.x + i as f32 * cw, rect.min.y), vec2(cw, rect.height()));
                    ui.put(r, GameItemTooltip::new(pairs[i].0).compare(Some(pairs[i].1)).style(*style));
                }
            },
        },
        Demo {
            id: "game-skill-tree",
            name: "Skill Tree",
            category: C,
            description: "A skill tree where unlocking sends a surge of energy down the link before the node pops, rings and sparkles alight; available links shimmer, locked ones stay faint, and hovering shows a card.",
            covers: &[],
            code: r##"ui.add(GameSkillTree::new(&mut nodes, &mut points).title("Pyromancy").cues(&mut cues));"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let rect = scene(ui, 420.0, style);
                let mut nodes: Vec<GameSkillNode> = mem(ui, "nodes", skill_nodes);
                let mut pts: u32 = mem(ui, "pts", || 6);
                inside(ui, rect.shrink2(vec2(20.0, 10.0)), |ui| {
                    ui.add(GameSkillTree::new(&mut nodes, &mut pts).title("Elementalist").height(400.0).style(style).autoplay(true));
                });
                set_mem(ui, "nodes", nodes);
                set_mem(ui, "pts", pts);
            },
        },
        Demo {
            id: "game-dialogue-box",
            name: "Dialogue Box",
            category: C,
            description: "An RPG dialogue box with a painted portrait that blinks and talks, a name plate, typewriter text with a glowing caret, and player choices that stagger in when the line finishes.",
            covers: &[],
            code: r##"ui.add(GameDialogueBox::new(&line)
    .chosen(&mut choice)
    .advance(&mut next)
    .cues(&mut cues)); // GameCue::Type per letter for blips"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let rect = scene(ui, 340.0, style);
                let lines = dialogue();
                let t = now(ui);
                let mut st: (usize, f64) = mem(ui, "line", || (0, t));
                let line = &lines[st.0 % lines.len()];
                let dur = line.text.chars().count() as f64 / 38.0 + if line.choices.is_empty() { 2.0 } else { 4.0 };
                let mut next = false;
                let mut chosen = None;
                inside(ui, Rect::from_min_max(pos2(rect.min.x + 20.0, rect.min.y + 20.0), rect.max - vec2(20.0, 10.0)), |ui| {
                    ui.add(GameDialogueBox::new(line).style(style).autoplay(true).advance(&mut next).chosen(&mut chosen));
                });
                if next || chosen.is_some() || t - st.1 > dur {
                    st = (st.0 + 1, t);
                }
                set_mem(ui, "line", st);
            },
        },
        Demo {
            id: "game-quest-tracker",
            name: "Quest Tracker",
            category: C,
            description: "A HUD quest tracker: new quests slide in flagged NEW, counts pop as they go up, finished objectives tick with a burst and a sweeping strike-through, and completed quests light up with a banner.",
            covers: &[],
            code: r##"ui.add(GameQuestTracker::new(&quests).width(320.0));"##,
            show: |ui| {
                let rect = scene(ui, 340.0, GameStyle::Fantasy);
                let t = now(ui);
                let step = ((t / 1.4) as usize) % 13;
                let q = quests(step);
                ui.put(Rect::from_min_size(pos2(rect.max.x - 360.0, rect.min.y + 10.0), vec2(340.0, rect.height() - 20.0)), GameQuestTracker::new(&q).width(330.0).style(GameStyle::Fantasy));
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "game-quest-log",
            name: "Quest Log",
            category: C,
            description: "A quest journal: main and side quests on the left with a gliding highlight, the selected quest's story, objectives with filling progress bars and rewards on the right.",
            covers: &[],
            code: r##"ui.add(GameQuestLog::new(&quests, &mut selected).style(GameStyle::Fantasy));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let rect = scene(ui, 420.0, style);
                let q = quests(5);
                let mut sel: usize = mem(ui, "sel", || 0);
                inside(ui, rect.shrink2(vec2(20.0, 14.0)), |ui| {
                    ui.add(GameQuestLog::new(&q, &mut sel).style(style).height(390.0).autoplay(true));
                });
                set_mem(ui, "sel", sel);
            },
        },
        Demo {
            id: "game-loot-popup",
            name: "Loot Popup",
            category: C,
            description: "A loot reveal: an orb in the rarity color charges and trembles, then bursts into a flash, shock rings, particles and god rays as the item card springs out with its stats.",
            covers: &[],
            code: r##"ui.add(GameLootPopup::new(&item).cues(&mut cues)); // GameCue::Loot(rarity) at the burst"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let rect = scene(ui, 360.0, style);
                let it = items();
                let order = [4usize, 1, 0, 14, 11];
                let t = now(ui);
                let k = ((t / 4.0) as usize) % order.len();
                ui.put(rect, GameLootPopup::new(&it[order[k]]).style(style).height(rect.height()));
            },
        },
        Demo {
            id: "game-achievement-toast",
            name: "Achievement Toast",
            category: C,
            description: "Achievement toasts that drop in on a spring, pop their badge with a ring and sparks, sweep a sheen across the card and count down before sliding away.",
            covers: &[],
            code: r##"ui.add(GameAchievementToast::new("Dragonslayer", "Defeat Vorrhak on Veteran", GameIcon::Trophy)
    .points(50)
    .rarity(GameRarity::Legendary)
    .percent(2.4)
    .age(seconds_since_unlock));"##,
            show: |ui| {
                let rect = scene(ui, 340.0, GameStyle::Minimal);
                let t = now(ui) as f32;
                let list = [
                    ("Dragonslayer", "Defeat Vorrhak on Veteran", GameIcon::Trophy, GameRarity::Legendary, 50, 2.4),
                    ("Hoarder", "Fill every inventory slot", GameIcon::Coin, GameRarity::Rare, 20, 18.1),
                    ("Sharpshooter", "Land 10 headshots in a row", GameIcon::Target, GameRarity::Epic, 30, 6.7),
                ];
                for (i, style) in GameStyle::ALL.iter().enumerate() {
                    let (title, desc, icon, rar, pts, pct) = list[i];
                    let age = (t + 6.5 - i as f32 * 0.6) % 6.5;
                    let r = Rect::from_min_size(pos2(rect.center().x - 200.0, rect.min.y + 12.0 + i as f32 * 106.0), vec2(400.0, 104.0));
                    ui.put(r, GameAchievementToast::new(title, desc, icon).points(pts).rarity(rar).percent(pct).age(age).duration(5.0).style(*style).width(400.0));
                }
            },
        },
        Demo {
            id: "game-damage-numbers",
            name: "Damage Numbers",
            category: C,
            description: "Floating combat text over a training target: normal hits pop and arc away, crits slam in golden with a starburst and shake, heals rise green, misses slide off. Click the scene to land crits.",
            covers: &[],
            code: r##"feed.spawn(ui, hit_pos, 1284, GameDamageKind::Crit);
feed.prune(ui);
GameDamageNumbers::new(&feed).paint(ui, viewport_rect);"##,
            show: |ui| {
                let mut p: [f32; 2] = mem(ui, "p", || [25.0, 1.0]);
                let rect = scene(ui, 340.0, GameStyle::Fantasy);
                let resp = ui.interact(rect, ui.scope_id().with("dmg_click"), Sense::click());
                let t = now(ui);
                let mut feed: GameDamageFeed = mem(ui, "feed", GameDamageFeed::new);
                let mut st: (f64, u64, f64) = mem(ui, "st", || (0.0, 0, -10.0));
                let target = pos2(rect.center().x, rect.min.y + rect.height() * 0.62);
                let local = target - rect.min;
                if t >= st.0 {
                    st.1 += 1;
                    let r = hash01(st.1 * 3);
                    let jitter = vec2(hash01(st.1 * 5) * 60.0 - 30.0, -40.0 - hash01(st.1 * 7) * 30.0);
                    let pos = (local + jitter).to_pos2();
                    let crit = r * 100.0 < p[0];
                    let kind = if crit { GameDamageKind::Crit } else if r > 0.93 { GameDamageKind::Heal } else if r > 0.86 { GameDamageKind::Miss } else if r > 0.78 { GameDamageKind::Poison } else { GameDamageKind::Normal };
                    let amount = match kind {
                        GameDamageKind::Crit => 900 + (hash01(st.1) * 900.0) as i64,
                        GameDamageKind::Heal => 120 + (hash01(st.1) * 100.0) as i64,
                        GameDamageKind::Poison => 12 + (hash01(st.1) * 10.0) as i64,
                        _ => 180 + (hash01(st.1) * 260.0) as i64,
                    };
                    feed.spawn(ui, pos, amount, kind);
                    if kind != GameDamageKind::Miss && kind != GameDamageKind::Heal {
                        st.2 = t;
                    }
                    st.0 = t + (0.35 + 0.45 * hash01(st.1 * 11) as f64) / p[1] as f64;
                }
                if resp.clicked() {
                    if let Some(pp) = resp.interact_pointer_pos() {
                        feed.spawn(ui, (pp - rect.min).to_pos2(), 2400, GameDamageKind::Crit);
                        st.2 = t;
                    }
                }
                feed.prune(ui);
                // training slime
                let hit = (t - st.2) as f32;
                let squash = if hit < 0.3 { (hit / 0.3 * std::f32::consts::PI).sin() * 0.18 } else { 0.0 };
                let bob = (t as f32 * 2.0).sin() * 3.0;
                let c = target + vec2(0.0, bob);
                let painter = ui.painter();
                paint::glow_ellipse(painter, target + vec2(0.0, 44.0), vec2(70.0, 12.0), Color32::from_black_alpha(140));
                let body = Rect::from_center_size(c, vec2(110.0 * (1.0 + squash), 86.0 * (1.0 - squash)));
                let pts = paint::rounded_rect_points(body, 40.0, 8);
                let flash = if hit < 0.12 { 1.0 - hit / 0.12 } else { 0.0 };
                let top = glowkit_core::color::mix(hex("#7CF07A"), Color32::WHITE, flash);
                paint::fill_convex_fn(painter, &pts, 4, paint::linear_fn(body, 90.0, &[top, hex("#2E9B4A")]));
                painter.circle_filled(c + vec2(-20.0, -8.0), 8.0, Color32::WHITE);
                painter.circle_filled(c + vec2(20.0, -8.0), 8.0, Color32::WHITE);
                let look = if hit < 0.4 { vec2(0.0, -2.0) } else { vec2(2.0, 1.0) };
                painter.circle_filled(c + vec2(-20.0, -8.0) + look, 4.0, hex("#14301C"));
                painter.circle_filled(c + vec2(20.0, -8.0) + look, 4.0, hex("#14301C"));
                painter.circle_filled(c + vec2(-30.0, -26.0), 6.0, with_alpha(Color32::WHITE, 0.5));
                GameDamageNumbers::new(&feed).paint(ui, rect);
                set_mem(ui, "feed", feed);
                set_mem(ui, "st", st);
                glowkit_core::animate(ui);
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Crit chance", &mut p[0], 0.0..=100.0).suffix("%").width(220.0));
                    ui.add(ControlSlider::new("Fire rate", &mut p[1], 0.3..=3.0).decimals(1).suffix("x").width(220.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-crosshairs",
            name: "Crosshairs",
            category: C,
            description: "A set of dynamic crosshairs (cross, dot, circle, chevron, brackets, scope) that open on a spring as spread grows, with white hit markers and red kill markers.",
            covers: &[],
            code: r##"ui.add(GameCrosshair::new(GameCrosshairShape::Cross)
    .spread(weapon_spread)
    .hit(Some(secs_since_hit), was_kill)
    .color(egui::Color32::from_rgb(124, 255, 178)));"##,
            show: |ui| {
                let mut p: [f32; 2] = mem(ui, "p", || [0.0, 2.0]);
                let rect = scene(ui, 300.0, GameStyle::SciFi);
                let t = now(ui) as f32;
                let firing = ((t * 0.5).fract() < 0.45) as i32 as f32;
                let spread = (p[0] / 100.0 + firing * 0.6 * (0.6 + 0.4 * (t * 12.0).sin().abs())).min(1.5);
                let hit_age = (t * 1.25).fract() / 1.25;
                let kill = ((t * 1.25) as u64) % 4 == 3;
                let colors = [hex("#7CFFB2"), hex("#FFFFFF"), hex("#FF5AD1"), hex("#FFD24A"), hex("#5AD1FF"), hex("#FFFFFF")];
                let cw = rect.width() / 3.0;
                let rh = rect.height() / 2.0;
                for (i, shape) in GameCrosshairShape::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(rect.min.x + (i % 3) as f32 * cw, rect.min.y + (i / 3) as f32 * rh), vec2(cw, rh));
                    ui.put(r, GameCrosshair::new(*shape).spread(spread).hit(if firing > 0.0 { Some(hit_age) } else { None }, kill).color(colors[i]).thickness(p[1]).size(rh - 16.0));
                }
                glowkit_core::animate(ui);
                controls(ui, |ui| {
                    ui.add(ControlSlider::new("Base spread", &mut p[0], 0.0..=100.0).suffix("%").width(220.0));
                    ui.add(ControlSlider::new("Thickness", &mut p[1], 1.0..=5.0).decimals(1).width(220.0));
                });
                set_mem(ui, "p", p);
            },
        },
        Demo {
            id: "game-loading-screen",
            name: "Loading Screen",
            category: C,
            description: "A loading screen with drifting level art, the level name in the corner, rotating gameplay tips, a progress bar with a moving sheen, a styled spinner and a pulsing prompt at 100%.",
            covers: &[],
            code: r##"ui.add(GameLoadingScreen::new(progress)
    .level("The Sunken Vault", "Chapter III")
    .tips(&TIPS));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let t = now(ui) as f32;
                let cyc = t % 11.0;
                let progress = if cyc < 8.0 { (cyc / 8.0).powf(0.8) * (0.85 + 0.15 * ((cyc * 3.0).sin() * 0.5 + 0.5)).min(1.0) } else { 1.0 };
                ui.add(
                    GameLoadingScreen::new(progress)
                        .level("The Sunken Vault", "Chapter III")
                        .tips(&["Parry just before a blow lands to stagger heavy enemies.", "Torches scare off dire wolves, but draw Guild patrols.", "Hold Shift while swimming to dive under vault grates."])
                        .style(style)
                        .height(380.0),
                );
            },
        },
        Demo {
            id: "game-level-up",
            name: "Level Up Screen",
            category: C,
            description: "A level-up moment: rays wheel open, LEVEL UP slams in letter by letter with a shake, the level number rolls over, confetti bursts and stat gains pop in one by one.",
            covers: &[],
            code: r##"ui.add(GameLevelUpScreen::new(level)
    .stats(&[("Strength", 31.0, 34.0), ("Vitality", 22.0, 25.0), ("Agility", 18.0, 19.0)])
    .note("+1 Skill Point"));"##,
            show: |ui| {
                let style = style_tabs(ui, 1);
                let t = now(ui);
                let lvl = 23 + ((t / 6.0) as u32) % 20;
                ui.add(GameLevelUpScreen::new(lvl).stats(&[("Strength", 31.0, 34.0), ("Vitality", 22.0, 25.0), ("Agility", 18.0, 19.0), ("Intellect", 12.0, 14.0)]).style(style).height(380.0));
            },
        },
        Demo {
            id: "game-kill-feed",
            name: "Kill Feed",
            category: C,
            description: "A kill feed where new kills punch in from the right with a flash, old ones fade and the rest glide up; headshots get a skull and your own kills a bright frame.",
            covers: &[],
            code: r##"ui.add(GameKillFeed::new(&entries).max_rows(6));"##,
            show: |ui| {
                let rect = scene(ui, 260.0, GameStyle::SciFi);
                let t = now(ui);
                let names = ["Vex", "Halcyon", "Nyx", "Brannoc", "Quill", "Sable", "Orrin", "Talon", "Wren", "Mako"];
                let weapons = [GameIcon::Rifle, GameIcon::Pistol, GameIcon::Bomb, GameIcon::Sword, GameIcon::Bow];
                let k0 = (t / 1.3) as u64;
                let kills: Vec<GameKillEntry> = (0..7u64)
                    .rev()
                    .map(|j| {
                        let k = k0.saturating_sub(j);
                        let age = (t - k as f64 * 1.3) as f32;
                        GameKillEntry::new(names[(k * 3 % 10) as usize], names[(k * 7 + 1) as usize % 10], weapons[(k % 5) as usize], age)
                            .headshot(k % 3 == 0)
                            .ally(k % 2 == 0)
                            .own(k % 4 == 1)
                    })
                    .collect();
                ui.put(Rect::from_min_size(pos2(rect.max.x - 400.0, rect.min.y + 14.0), vec2(380.0, 230.0)), GameKillFeed::new(&kills).width(380.0).max_rows(7).life(7.0));
            },
        },
        Demo {
            id: "game-scoreboard",
            name: "Scoreboard",
            category: C,
            description: "A two-team scoreboard: rows stagger in, re-sort with springy moves as scores change (the new score pops), the local player glows, dead players dim with a skull and ping shows as signal bars.",
            covers: &[],
            code: r##"ui.add(GameScoreboard::new(&rows).teams(["Aurora", "Obsidian"]));"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let rect = scene(ui, 300.0, style);
                let t = now(ui);
                let tick = (t / 1.5) as u64;
                let base = [("Vex", 0), ("Halcyon", 0), ("Nyx", 0), ("Brannoc", 0), ("Quill", 0), ("Sable", 1), ("Orrin", 1), ("Talon", 1), ("Wren", 1), ("Mako", 1)];
                let rows: Vec<GameScoreRow> = base
                    .iter()
                    .enumerate()
                    .map(|(i, (n, team))| {
                        let mut score = 900 + (hash01(i as u64 * 17) * 1600.0) as u32;
                        let mut kills = 2 + (hash01(i as u64 * 3) * 8.0) as u32;
                        for k in 0..tick % 40 {
                            if (hash01(k * 31 + 5) * 10.0) as usize == i {
                                score += 150;
                                kills += 1;
                            }
                        }
                        GameScoreRow::new(*n, *team)
                            .kda(kills, 1 + (hash01(i as u64 * 5) * 7.0) as u32, (hash01(i as u64 * 9) * 6.0) as u32)
                            .score(score)
                            .ping(20 + (hash01(i as u64 * 23) * 180.0) as u32)
                            .local(i == 2)
                            .dead((tick + i as u64) % 7 == 0)
                    })
                    .collect();
                inside(ui, rect.shrink2(vec2(16.0, 6.0)), |ui| {
                    ui.add(GameScoreboard::new(&rows).teams(["Aurora", "Obsidian"]).style(style));
                });
                glowkit_core::animate(ui);
            },
        },
        Demo {
            id: "game-character-select",
            name: "Character Select",
            category: C,
            description: "A hero select carousel: cards ride a spring along an arc with the chosen one tall and glowing, the scene tints to the hero's color, stat bars refill and the confirm button breathes.",
            covers: &[],
            code: r##"ui.add(GameCharacterSelect::new(&roster, &mut selected).confirmed(&mut ready));"##,
            show: |ui| {
                let style = style_tabs(ui, 0);
                let r = roster();
                let mut sel: usize = mem(ui, "sel", || 0);
                ui.add(GameCharacterSelect::new(&r, &mut sel).style(style).autoplay(true).height(410.0));
                set_mem(ui, "sel", sel);
            },
        },
    ]
}
