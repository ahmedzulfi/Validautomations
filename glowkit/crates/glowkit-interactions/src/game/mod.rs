//! # Game UI blocks
//!
//! Bold, animated in-game interface pieces for Rust games: menus, HUD bars, minimaps, ammo and
//! cooldown widgets, hotbars, inventories with drag and drop, skill trees, dialogue, quests,
//! loot reveals, toasts, damage numbers, crosshairs, loading / level-up screens, kill feeds,
//! scoreboards and a character select carousel.
//!
//! Every block is a plain egui widget that **takes your game state as data** (current HP, the
//! inventory `Vec`, quest list, cooldown timers, ...) and only keeps cosmetic animation state
//! (trails, springs, particles) internally. Most blocks come in three skins via [`GameStyle`]:
//! a holographic sci-fi HUD, fantasy parchment and gold, and clean minimal glass. Colors can be
//! overridden with a [`GamePalette`].
//!
//! ## Sound and feedback hooks
//!
//! Interactive blocks accept `.cues(&mut Vec<GameCue>)`. After the widget runs, the vector holds
//! the feedback events of this frame ([`GameCue::Hover`], [`GameCue::Confirm`],
//! [`GameCue::Hit`], [`GameCue::Loot`], ...): map them to sounds, rumble or camera shake.
//!
//! ## Using it from Bevy
//!
//! Bevy games embed egui through the `bevy_egui` plugin. Add the plugin, then draw glowkit
//! blocks inside any egui context, usually a transparent `egui::Area` or a `CentralPanel` with
//! `Frame::NONE` so the game renders underneath:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
//! use glowkit_interactions::{GameResourceBar, GameResourceKind, GameStyle};
//!
//! #[derive(Component)]
//! struct Health { hp: f32, max: f32 }
//!
//! fn hud(mut ctx: EguiContexts, q: Query<&Health>) -> Result {
//!     let ctx = ctx.ctx_mut()?;
//!     let hp = q.single()?;
//!     egui::Area::new("hud".into()).anchor(egui::Align2::LEFT_BOTTOM, [24.0, -24.0]).show(ctx, |ui| {
//!         ui.add(GameResourceBar::new(hp.hp, hp.max).kind(GameResourceKind::Health).style(GameStyle::SciFi).width(320.0));
//!     });
//!     Ok(())
//! }
//!
//! App::new()
//!     .add_plugins((DefaultPlugins, EguiPlugin::default()))
//!     .add_systems(EguiPrimaryContextPass, hud)
//!     .run();
//! ```
//!
//! Tips:
//!
//! * Keep the source of truth in your ECS components / resources and pass values in every
//!   frame; the widgets detect changes themselves (a lower HP plays the damage trail, a lower
//!   magazine count kicks the ammo counter, a new loot item plays the reveal).
//! * Forward [`GameCue`]s to `bevy_audio` (or your audio crate) as events.
//! * The animations call `request_repaint`, which `bevy_egui` honours; in a game that renders
//!   every frame anyway this costs nothing extra.
//! * [`GameBackdrop`] paints a stand-in game scene; in a real game leave it out and let the
//!   3D/2D world show through.
//!
//! **Version note:** glowkit is built on egui 0.37, while `bevy_egui` currently targets egui
//! 0.36. The two egui versions can't share a context, so this works as soon as `bevy_egui`
//! releases its egui 0.37 update (or with a `[patch]` pointing `bevy_egui` at a branch on 0.37).

mod bars;
mod common;
#[cfg(feature = "demos")]
mod demos;
mod dialogue;
mod feed;
mod hud;
mod inventory;
mod menus;
mod popups;
mod quests;
mod screens;
mod skill_tree;

pub use bars::{GameBossBar, GameResourceBar, GameResourceKind};
pub use common::{GameBackdrop, GameCue, GameIcon, GameItem, GamePalette, GameRarity, GameStyle};
pub use dialogue::{GameDialogueBox, GameDialogueLine, GamePortrait};
pub use feed::{
    GameDamageFeed, GameDamageKind, GameDamageNumbers, GameKillEntry, GameKillFeed, GameScoreRow, GameScoreboard,
};
pub use hud::{
    GameAmmoCounter, GameCooldownIcon, GameCrosshair, GameCrosshairShape, GameMapMarker, GameMarkerKind, GameMinimap,
};
pub use inventory::{GameHotbar, GameInventoryGrid, GameItemTooltip};
pub use menus::{GameMainMenu, GamePauseMenu, GameSettings, GameSettingsPanel};
pub use popups::{GameAchievementToast, GameLootPopup};
pub use quests::{GameQuest, GameQuestLog, GameQuestTracker};
pub use screens::{GameCharacter, GameCharacterSelect, GameLevelUpScreen, GameLoadingScreen};
pub use skill_tree::{GameSkillNode, GameSkillTree};

/// Gallery demos for this module.
#[cfg(feature = "demos")]
pub fn demos() -> Vec<glowkit_core::demo::Demo> {
    demos::demos()
}
