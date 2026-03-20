use serde::{Deserialize, Serialize};
use crate::entity::{ClassDef, GroundItem, Monster, Player};
use crate::world::{CombatEvent, GameMap};

/// Messages sent from client → server (newline-delimited JSON over TCP)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum ClientMsg {
    Register { username: String, password: String },
    Login    { username: String, password: String },
    CreateChar {
        name:     String,
        class_id: String,
        symbol:   char,
        color:    String,
    },
    Move     { dx: i32, dy: i32 },
    Attack   { target_id: u64 },
    UseItem  { item_id: u64 },
    Equip    { item_id: u64 },
    Pickup,
    DropItem { item_id: u64 },
    Chat     { msg: String },
    Ping,
    Quit,
}

/// Messages sent from server → client (newline-delimited JSON over TCP)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum ServerMsg {
    // ── auth / registration ──
    RegisterOk,
    LoginOk,
    NeedChar { classes: Vec<ClassDef> },
    CharOk,
    Err { msg: String },

    // ── initial world snapshot ──
    WorldInit {
        player_id: u64,
        map:       GameMap,
        players:   Vec<Player>,
        monsters:  Vec<Monster>,
        items:     Vec<GroundItem>,
    },

    // ── incremental world updates ──
    PlayerUpdate  (Player),
    PlayerLeft    (u64),
    MonsterUpdate (Monster),
    MonsterDied   { id: u64, xp: u64 },
    ItemDropped   (GroundItem),
    ItemPickedUp  { item_id: u64, by: String },

    // ── game events ──
    Combat  (CombatEvent),
    Chat    { from: String, msg: String },
    System  (String),
    LevelUp { level: u32, stat_points: u32 },

    Pong,
}
