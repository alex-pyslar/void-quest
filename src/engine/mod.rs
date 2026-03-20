pub mod events;
mod actions;
mod ai;
mod loot;

use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::{
    config::GameConfig,
    entity::{Equipment, GroundItem, Item, Monster, Player},
    mapgen::{self, MapStyle},
    world::{GameMap, Pos, SP_MAP_W, SP_MAP_H},
};

pub use events::SpEvent;

// ─── Local (offline) game engine ──────────────────────────────────────────────

pub struct LocalGame {
    pub map:          GameMap,
    pub player:       Player,
    pub monsters:     HashMap<u64, Monster>,
    pub ground_items: Vec<GroundItem>,
    pub cfg:          GameConfig,
    pub(crate) next_id: u64,
    pub tick:         u64,
}

impl LocalGame {
    pub fn new(name: &str, class_id: &str, symbol: char, color: String) -> Option<Self> {
        let cfg = GameConfig::load().ok()?;
        let mut next_id = 1u64;
        let mut rng = rand::thread_rng();
        let map = mapgen::generate_large(&mut rng);

        // ── build player ───────────────────────────────────────────────────────
        let cls = cfg.classes.get(class_id)?.clone();
        let mut inventory = Vec::new();
        let mut equipment = Equipment::default();

        if let Some(item_id) = &cls.start_item.clone() {
            if let Some(tmpl) = cfg.items.get(item_id) {
                next_id += 1;
                let item = Item {
                    id: next_id,
                    template_id: tmpl.id.clone(),
                    name:        tmpl.display_name(&cfg.locale.lang).to_string(),
                    symbol:      tmpl.symbol,
                    color:       tmpl.color.clone(),
                    kind:        tmpl.kind.clone(),
                };
                equipment.equip_item(item);
            }
        }
        for _ in 0..3 {
            if let Some(tmpl) = cfg.items.get("hp_potion") {
                next_id += 1;
                inventory.push(Item {
                    id: next_id,
                    template_id: tmpl.id.clone(),
                    name:        tmpl.display_name(&cfg.locale.lang).to_string(),
                    symbol:      tmpl.symbol,
                    color:       tmpl.color.clone(),
                    kind:        tmpl.kind.clone(),
                });
            }
        }

        let player = Player {
            id:          1,
            name:        name.to_string(),
            class_id:    class_id.to_string(),
            symbol, color,
            pos:         Pos::new(SP_MAP_W / 2, SP_MAP_H / 2),
            level:       1,
            xp:          0,
            xp_next:     Player::xp_for_level(2),
            stats:       cls.make_stats(),
            equipment, inventory,
            stat_points: 0,
        };

        // ── spawn monsters ─────────────────────────────────────────────────────
        let mut monsters = HashMap::new();
        let positions = mapgen::monster_spawn_positions(&map);
        if !positions.is_empty() {
            let templates: Vec<String> = cfg.monsters.keys().cloned().collect();
            let count = cfg.world.monster_count;
            for _ in 0..count {
                let tmpl_id = templates[rng.gen_range(0..templates.len())].clone();
                let (x, y)  = positions[rng.gen_range(0..positions.len())];
                if let Some(tmpl) = cfg.monsters.get(&tmpl_id) {
                    next_id += 1;
                    let m = Monster {
                        id:          next_id,
                        template_id: tmpl.id.clone(),
                        name:        tmpl.display_name(&cfg.locale.lang).to_string(),
                        symbol:      tmpl.symbol,
                        color:       tmpl.color.clone(),
                        pos:         Pos::new(x, y),
                        home:        Pos::new(x, y),
                        level:       tmpl.level,
                        stats:       tmpl.make_stats(),
                        xp_reward:   tmpl.xp_reward,
                        loot_table:  tmpl.loot_table.clone(),
                        target:      None,
                    };
                    monsters.insert(m.id, m);
                }
            }
        }

        Some(Self { map, player, monsters, ground_items: Vec::new(), cfg, next_id, tick: 0 })
    }

    /// Create a new zone instance with a GIVEN player (for zone travel).
    /// Generates a fresh smaller map using the zone seed and biome style.
    pub fn new_zone(mut player: Player, zone_seed: u64, style: MapStyle) -> Option<Self> {
        let cfg = GameConfig::load().ok()?;
        let mut rng = StdRng::seed_from_u64(zone_seed);
        let map = mapgen::generate_zone(&mut rng, style);

        // Find spawn: prefer passable tiles away from center (avoid town walls)
        // Scan in a spiral from a point slightly offset from center
        let cx = map.width / 2;
        let cy = map.height / 2;
        let mut spawned = false;
        'search: for r in 2..30i32 {
            for dy in -r..=r { for dx in -r..=r {
                // Only check perimeter of each shell
                if dx.abs() != r && dy.abs() != r { continue; }
                let x = cx + dx; let y = cy + dy;
                if x > 0 && y > 0 && x < map.width-1 && y < map.height-1
                    && map.passable(x, y)
                {
                    player.pos = Pos::new(x, y);
                    spawned = true;
                    break 'search;
                }
            }}
        }
        // Fallback: any passable tile
        if !spawned {
            for y in 1..map.height-1 { for x in 1..map.width-1 {
                if map.passable(x, y) { player.pos = Pos::new(x, y); spawned = true; break; }
            } if spawned { break; }}
        }

        // Spawn monsters
        let mut next_id = 1000u64;
        let mut monsters = HashMap::new();
        let positions = mapgen::monster_spawn_positions(&map);
        if !positions.is_empty() {
            let templates: Vec<String> = cfg.monsters.keys().cloned().collect();
            let count = (cfg.world.monster_count / 3).max(4);
            for _ in 0..count {
                let tmpl_id = templates[rng.gen_range(0..templates.len())].clone();
                let (x, y)  = positions[rng.gen_range(0..positions.len())];
                if let Some(tmpl) = cfg.monsters.get(&tmpl_id) {
                    next_id += 1;
                    let m = Monster {
                        id:          next_id,
                        template_id: tmpl.id.clone(),
                        name:        tmpl.display_name(&cfg.locale.lang).to_string(),
                        symbol:      tmpl.symbol,
                        color:       tmpl.color.clone(),
                        pos:         Pos::new(x, y),
                        home:        Pos::new(x, y),
                        level:       tmpl.level,
                        stats:       tmpl.make_stats(),
                        xp_reward:   tmpl.xp_reward,
                        loot_table:  tmpl.loot_table.clone(),
                        target:      None,
                    };
                    monsters.insert(m.id, m);
                }
            }
        }

        Some(Self { map, player, monsters, ground_items: Vec::new(), cfg, next_id, tick: 0 })
    }

    /// Return the list of spell IDs available to the given class.
    pub fn cfg_spells_for_class(&self, class_id: &str) -> Vec<String> {
        self.cfg.classes.get(class_id)
            .map(|c| c.spells.clone())
            .unwrap_or_default()
    }

    pub fn nearest_monster_id(&self) -> Option<u64> {
        self.monsters.values()
            .filter(|m| m.stats.is_alive() && self.player.pos.adjacent(m.pos))
            .min_by_key(|m| self.player.pos.dist_sq(m.pos))
            .map(|m| m.id)
    }

    /// Find the nearest alive monster anywhere on the map (for spells).
    pub(crate) fn nearest_monster_id_any(&self) -> Option<u64> {
        self.monsters.values()
            .filter(|m| m.stats.is_alive())
            .min_by_key(|m| self.player.pos.dist_sq(m.pos))
            .map(|m| m.id)
    }

    pub(crate) fn try_level_up(&mut self) -> Vec<SpEvent> {
        use crate::engine::events::Color;
        let mut events = Vec::new();
        while self.player.xp >= self.player.xp_next {
            self.player.xp          -= self.player.xp_next;
            self.player.level       += 1;
            self.player.xp_next      = Player::xp_for_level(self.player.level + 1);
            self.player.stat_points += 3;
            self.player.stats.str   += 1;
            self.player.stats.vit   += 1;
            if let Some(cls) = self.cfg.classes.get(&self.player.class_id).cloned() {
                self.player.stats.max_hp += cls.hp_per_level;
                self.player.stats.hp      = self.player.stats.max_hp;
                self.player.stats.max_mp += cls.mp_per_level;
                self.player.stats.mp      = self.player.stats.max_mp;
            }
            events.push(SpEvent::LevelUp {
                level:       self.player.level,
                stat_points: self.player.stat_points,
            });
            events.push(SpEvent::Log(
                format!("★★★ LEVEL UP! Level {}! (+3 stat points) ★★★", self.player.level),
                Color::LightYellow,
            ));
        }
        events
    }
}
