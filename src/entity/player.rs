use serde::{Deserialize, Serialize};
use crate::world::{Pos, Stats};
use super::item::{Equipment, Item};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id:          u64,
    pub name:        String,
    pub class_id:    String,
    pub symbol:      char,
    pub color:       String,
    pub pos:         Pos,
    pub level:       u32,
    pub xp:          u64,
    pub xp_next:     u64,
    pub stats:       Stats,
    pub equipment:   Equipment,
    pub inventory:   Vec<Item>,
    pub stat_points: u32,
}

impl Player {
    pub fn attack(&self)  -> i32 { self.stats.str + self.equipment.atk_bonus() }
    pub fn defense(&self) -> i32 { self.stats.vit / 2 + self.equipment.def_bonus() }
    pub fn xp_for_level(lv: u32) -> u64 { (lv as u64).pow(2) * 150 }
}
