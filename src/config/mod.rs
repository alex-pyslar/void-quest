pub mod locale;
pub mod defaults;

use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, fs};

use crate::entity::{ClassDef, ItemTemplate, MonsterTemplate, Spell};

pub use locale::LocaleConfig;
pub use defaults::{default_classes, default_monsters, default_items, default_spells};

// ─── 3D Model / Texture configuration ────────────────────────────────────────

/// Visual texture definition for a map tile in 3D view.
#[derive(Debug, Clone, Deserialize)]
pub struct TileModel {
    /// Chars sampled by wall-hit X position (wraps if shorter than screen width).
    pub texture:     String,
    pub r_near: u8, pub g_near: u8, pub b_near: u8,
    pub r_far:  u8, pub g_far:  u8, pub b_far:  u8,
    /// Fraction of wall height from the bottom that uses trunk color (trees).
    #[serde(default)]
    pub trunk_ratio: f64,
    #[serde(default)]
    pub trunk_r: u8,
    #[serde(default)]
    pub trunk_g: u8,
    #[serde(default)]
    pub trunk_b: u8,
}

impl Default for TileModel {
    fn default() -> Self {
        Self {
            texture: "#".into(),
            r_near: 160, g_near: 160, b_near: 160,
            r_far: 40,   g_far: 40,   b_far: 40,
            trunk_ratio: 0.0, trunk_r: 0, trunk_g: 0, trunk_b: 0,
        }
    }
}

/// Visual sprite definition for a monster in 3D view.
#[derive(Debug, Clone, Deserialize)]
pub struct MonsterModel {
    pub id:      String,
    /// Multi-row ASCII sprite. Each string is one row; chars are sampled by
    /// horizontal position within the billboard. Spaces are transparent.
    /// Falls back to head/body/feet if empty.
    #[serde(default)]
    pub sprite:  Vec<String>,
    /// Legacy single-char fallbacks (used when sprite is empty).
    #[serde(default = "default_head")]
    pub head:    String,
    #[serde(default = "default_body")]
    pub body:    String,
    #[serde(default = "default_feet")]
    pub feet:    String,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
}

fn default_head() -> String { "^".into() }
fn default_body() -> String { "|".into() }
fn default_feet() -> String { "v".into() }

#[derive(Debug, Clone, Deserialize)]
struct ModelsFile {
    #[serde(default)] wall:    Option<TileModel>,
    #[serde(default)] floor:   Option<TileModel>,
    #[serde(default)] tree:    Option<TileModel>,
    #[serde(default)] water:   Option<TileModel>,
    #[serde(default)] sand:    Option<TileModel>,
    #[serde(default)] road:    Option<TileModel>,
    #[serde(default)] lava:    Option<TileModel>,
    #[serde(default)] ice:     Option<TileModel>,
    #[serde(default)] pillar:  Option<TileModel>,
    #[serde(default)] bramble: Option<TileModel>,
    #[serde(default)] ruins:   Option<TileModel>,
    #[serde(default)] mud:     Option<TileModel>,
    #[serde(default)] monsters: Vec<MonsterModel>,
}

#[derive(Debug, Clone)]
pub struct ModelsConfig {
    pub wall:    TileModel,
    pub floor:   TileModel,
    pub tree:    TileModel,
    pub water:   TileModel,
    pub sand:    TileModel,
    pub road:    TileModel,
    pub lava:    TileModel,
    pub ice:     TileModel,
    pub pillar:  TileModel,
    pub bramble: TileModel,
    pub ruins:   TileModel,
    pub mud:     TileModel,
    pub monsters: Vec<MonsterModel>,
}

impl ModelsConfig {
    pub fn load() -> Self {
        let file: Option<ModelsFile> = fs::read_to_string("config/models.toml")
            .ok()
            .and_then(|s| toml::from_str(&s).ok());

        let f = file.unwrap_or(ModelsFile {
            wall: None, floor: None, tree: None,
            water: None, sand: None, road: None,
            lava: None, ice: None, pillar: None,
            bramble: None, ruins: None, mud: None,
            monsters: vec![],
        });

        Self {
            wall:    f.wall.unwrap_or_default(),
            floor:   f.floor.unwrap_or_default(),
            tree:    f.tree.unwrap_or(TileModel {
                texture: "T|".into(), r_near: 32, g_near: 168, b_near: 32,
                r_far: 12, g_far: 80, b_far: 12,
                trunk_ratio: 0.35, trunk_r: 110, trunk_g: 65, trunk_b: 22,
            }),
            water:   f.water.unwrap_or(TileModel {
                texture: "~≈".into(), r_near: 22, g_near: 90, b_near: 230,
                r_far: 8, g_far: 38, b_far: 115,
                ..Default::default()
            }),
            sand:    f.sand.unwrap_or(TileModel {
                texture: ".,".into(), r_near: 200, g_near: 170, b_near: 90,
                r_far: 80, g_far: 68, b_far: 36,
                ..Default::default()
            }),
            road:    f.road.unwrap_or(TileModel {
                texture: "+-".into(), r_near: 140, g_near: 120, b_near: 80,
                r_far: 55, g_far: 48, b_far: 32,
                ..Default::default()
            }),
            lava:    f.lava.unwrap_or(TileModel {
                texture: "^~^~".into(), r_near: 255, g_near: 80, b_near: 0,
                r_far: 140, g_far: 30, b_far: 0,
                ..Default::default()
            }),
            ice:     f.ice.unwrap_or(TileModel {
                texture: "___-".into(), r_near: 180, g_near: 220, b_near: 255,
                r_far: 80, g_far: 120, b_far: 180,
                ..Default::default()
            }),
            pillar:  f.pillar.unwrap_or(TileModel {
                texture: "O|O|".into(), r_near: 160, g_near: 155, b_near: 145,
                r_far: 55, g_far: 52, b_far: 48,
                ..Default::default()
            }),
            bramble: f.bramble.unwrap_or(TileModel {
                texture: "*+*+".into(), r_near: 80, g_near: 130, b_near: 40,
                r_far: 30, g_far: 55, b_far: 15,
                ..Default::default()
            }),
            ruins:   f.ruins.unwrap_or(TileModel {
                texture: "::.".into(), r_near: 110, g_near: 105, b_near: 95,
                r_far: 42, g_far: 40, b_far: 36,
                ..Default::default()
            }),
            mud:     f.mud.unwrap_or(TileModel {
                texture: ";,;,".into(), r_near: 90, g_near: 70, b_near: 40,
                r_far: 35, g_far: 28, b_far: 16,
                ..Default::default()
            }),
            monsters: f.monsters,
        }
    }

    pub fn monster(&self, id: &str) -> Option<&MonsterModel> {
        self.monsters.iter().find(|m| m.id == id)
    }
}

// ─── World / server configuration ────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct WorldConfig {
    pub host:          String,
    pub port:          u16,
    pub tick_ms:       u64,
    pub monster_count: usize,
    pub view_radius:   i32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            host:          "127.0.0.1".into(),
            port:          7777,
            tick_ms:       600,
            monster_count: 40,
            view_radius:   18,
        }
    }
}

// ─── Full game configuration ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub world:    WorldConfig,
    pub classes:  HashMap<String, ClassDef>,
    pub monsters: HashMap<String, MonsterTemplate>,
    pub items:    HashMap<String, ItemTemplate>,
    pub spells:   HashMap<String, Spell>,
    pub locale:   LocaleConfig,
    pub models:   ModelsConfig,
}

impl GameConfig {
    pub fn load() -> Result<Self> {
        let world = fs::read_to_string("config/world.toml")
            .ok()
            .and_then(|s| toml::from_str::<WorldConfig>(&s).ok())
            .unwrap_or_default();

        let classes = fs::read_to_string("config/classes.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<ClassDef>>(&s).ok())
            .unwrap_or_else(default_classes)
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect();

        let monsters = fs::read_to_string("config/monsters.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<MonsterTemplate>>(&s).ok())
            .unwrap_or_else(default_monsters)
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();

        let items = fs::read_to_string("config/items.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<ItemTemplate>>(&s).ok())
            .unwrap_or_else(default_items)
            .into_iter()
            .map(|i| (i.id.clone(), i))
            .collect();

        let spells = fs::read_to_string("config/spells.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<Spell>>(&s).ok())
            .unwrap_or_else(default_spells)
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect();

        let locale = LocaleConfig::load();
        let models = ModelsConfig::load();

        Ok(Self { world, classes, monsters, items, spells, locale, models })
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.world.host, self.world.port)
    }
}
