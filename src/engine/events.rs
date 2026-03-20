use crate::world::CombatEvent;
use crate::entity::{GroundItem, Monster, Player};

/// Minimal log-colour hint — replaces ratatui::style::Color for the engine layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red, Green, Yellow, LightYellow, Cyan, White, Blue, Magenta, Gray, Reset,
}

pub enum SpEvent {
    UpdatePlayer(Player),
    MonsterUpdate(Monster),
    MonsterDied { id: u64, xp: u64 },
    ItemDropped(GroundItem),
    ItemPickedUp { item_id: u64 },
    LevelUp { level: u32, stat_points: u32 },
    Log(String, Color),
    Combat(CombatEvent),
}
