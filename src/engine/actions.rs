use rand::Rng;
use crate::engine::events::Color;

use crate::{
    entity::{GroundItem, ItemKind},
    world::{CombatEvent, Pos},
};
use super::{LocalGame, SpEvent};

impl LocalGame {
    pub fn move_player(&mut self, dx: i32, dy: i32) -> Vec<SpEvent> {
        let mut events = Vec::new();
        if !self.player.stats.is_alive() { return events; }
        let new_pos = Pos::new(self.player.pos.x + dx, self.player.pos.y + dy);
        let blocked = !self.map.passable(new_pos.x, new_pos.y)
            || self.monsters.values().any(|m| m.pos == new_pos && m.stats.is_alive());
        if blocked { return events; }
        self.player.pos = new_pos;
        if let Some(gi) = self.ground_items.iter().find(|gi| gi.pos == new_pos) {
            events.push(SpEvent::Log(
                format!("You see '{}' here. Press P to pick it up.", gi.item.name),
                Color::Yellow,
            ));
        }
        events.push(SpEvent::UpdatePlayer(self.player.clone()));
        events
    }

    pub fn attack(&mut self, target_id: u64) -> Vec<SpEvent> {
        let mut events = Vec::new();
        let can = match self.monsters.get(&target_id) {
            Some(m) => m.stats.is_alive() && self.player.pos.adjacent(m.pos),
            None    => false,
        };
        if !can { return events; }

        let mut rng = rand::thread_rng();
        let crit_chance = if self.player.class_id == "rogue" { 0.20 } else { 0.10 };
        let is_crit  = rng.gen_bool(crit_chance);
        let raw_dmg  = if is_crit { self.player.attack() * 2 } else { self.player.attack() };
        let mon_def  = self.monsters[&target_id].stats.vit / 3;
        let mon_name = self.monsters[&target_id].name.clone();
        let actual   = (raw_dmg - mon_def).max(1);

        { self.monsters.get_mut(&target_id).unwrap().stats.hp -= actual; }
        let killed = !self.monsters[&target_id].stats.is_alive();

        events.push(SpEvent::Combat(CombatEvent {
            attacker: self.player.name.clone(),
            target:   mon_name.clone(),
            damage:   actual, is_crit, killed,
        }));

        if killed {
            let (xp_reward, loot_table, mpos) = {
                let m = &self.monsters[&target_id];
                (m.xp_reward, m.loot_table.clone(), m.pos)
            };
            self.monsters.remove(&target_id);
            events.push(SpEvent::MonsterDied { id: target_id, xp: xp_reward });

            let mut rng2 = rand::thread_rng();
            for item_id in &loot_table {
                if rng2.gen_bool(0.35) {
                    if let Some(item) = self.make_item(item_id) {
                        let gi = GroundItem { item, pos: mpos };
                        events.push(SpEvent::ItemDropped(gi.clone()));
                        self.ground_items.push(gi);
                    }
                }
            }
            self.player.xp += xp_reward;
            events.extend(self.try_level_up());
        } else {
            events.push(SpEvent::MonsterUpdate(self.monsters[&target_id].clone()));
        }
        events.push(SpEvent::UpdatePlayer(self.player.clone()));
        events
    }

    /// Cast a spell by ID. Finds nearest monster for damage spells, heals player for heal spells.
    pub fn cast_spell(&mut self, spell_id: &str) -> Vec<SpEvent> {
        let mut events = Vec::new();

        let spell = match self.cfg.spells.get(spell_id).cloned() {
            Some(s) => s,
            None => {
                events.push(SpEvent::Log(format!("Unknown spell: {}", spell_id), Color::Red));
                return events;
            }
        };

        if self.player.stats.mp < spell.mp_cost {
            events.push(SpEvent::Log(
                format!("Not enough MP to cast {}! ({}/{})", spell.name, self.player.stats.mp, spell.mp_cost),
                Color::Red,
            ));
            return events;
        }

        self.player.stats.mp -= spell.mp_cost;

        if spell.heal > 0 {
            self.player.stats.hp = (self.player.stats.hp + spell.heal).min(self.player.stats.max_hp);
            events.push(SpEvent::Log(
                format!("✨ {} heals {} HP! (MP: {})", spell.name, spell.heal, self.player.stats.mp),
                Color::Green,
            ));
        }

        if spell.damage > 0 {
            let target_id = self.nearest_monster_id()
                .or_else(|| self.nearest_monster_id_any());

            if let Some(tid) = target_id {
                let mon_name = self.monsters[&tid].name.clone();
                let mpos     = self.monsters[&tid].pos;
                let mon_def  = self.monsters[&tid].stats.vit / 4;
                let actual   = (spell.damage - mon_def).max(1);

                self.monsters.get_mut(&tid).unwrap().stats.hp -= actual;
                let killed = !self.monsters[&tid].stats.is_alive();

                events.push(SpEvent::Combat(CombatEvent {
                    attacker: format!("{} ({})", self.player.name, spell.name),
                    target:   mon_name.clone(),
                    damage:   actual,
                    is_crit:  false,
                    killed,
                }));

                if killed {
                    let xp_reward = self.monsters[&tid].xp_reward;
                    let loot_table = self.monsters[&tid].loot_table.clone();
                    self.monsters.remove(&tid);
                    events.push(SpEvent::MonsterDied { id: tid, xp: xp_reward });

                    let mut rng2 = rand::thread_rng();
                    for item_id in &loot_table {
                        if rng2.gen_bool(0.35) {
                            if let Some(item) = self.make_item(item_id) {
                                let gi = GroundItem { item, pos: mpos };
                                events.push(SpEvent::ItemDropped(gi.clone()));
                                self.ground_items.push(gi);
                            }
                        }
                    }
                    self.player.xp += xp_reward;
                    events.extend(self.try_level_up());
                } else {
                    events.push(SpEvent::MonsterUpdate(self.monsters[&tid].clone()));
                }
            } else {
                events.push(SpEvent::Log(
                    format!("✨ {} — no target in range!", spell.name),
                    Color::Yellow,
                ));
            }
        }

        events.push(SpEvent::UpdatePlayer(self.player.clone()));
        events
    }

    pub fn use_item(&mut self, item_id: u64) -> Vec<SpEvent> {
        let mut events = Vec::new();
        if let Some(idx) = self.player.inventory.iter().position(|i| i.id == item_id) {
            let item = self.player.inventory[idx].clone();
            if let ItemKind::Potion { hp, mp } = item.kind {
                self.player.inventory.remove(idx);
                self.player.stats.hp = (self.player.stats.hp + hp).min(self.player.stats.max_hp);
                self.player.stats.mp = (self.player.stats.mp + mp).min(self.player.stats.max_mp);
                events.push(SpEvent::Log(
                    format!("Used {}. HP +{} MP +{}", item.name, hp, mp), Color::Green,
                ));
                events.push(SpEvent::UpdatePlayer(self.player.clone()));
            } else {
                events.push(SpEvent::Log("That item is not a consumable.".into(), Color::Red));
            }
        }
        events
    }

    pub fn equip_item(&mut self, item_id: u64) -> Vec<SpEvent> {
        use crate::entity::Equipment;
        let mut events = Vec::new();
        if let Some(idx) = self.player.inventory.iter().position(|i| i.id == item_id) {
            let item = self.player.inventory.remove(idx);
            let slot = Equipment::slot_name(&item.kind).to_string();
            let old  = self.player.equipment.equip_item(item.clone());
            if let Some(o) = old { self.player.inventory.push(o); }
            events.push(SpEvent::Log(
                format!("Equipped {} in {} slot.", item.name, slot), Color::Cyan,
            ));
            events.push(SpEvent::UpdatePlayer(self.player.clone()));
        }
        events
    }

    pub fn pickup(&mut self) -> Vec<SpEvent> {
        let mut events = Vec::new();
        let pos = self.player.pos;
        if let Some(idx) = self.ground_items.iter().position(|gi| gi.pos == pos) {
            let gi = self.ground_items.remove(idx);
            let item_id = gi.item.id;
            let name    = gi.item.name.clone();
            self.player.inventory.push(gi.item);
            events.push(SpEvent::Log(format!("Picked up {}.", name), Color::Yellow));
            events.push(SpEvent::ItemPickedUp { item_id });
            events.push(SpEvent::UpdatePlayer(self.player.clone()));
        } else {
            events.push(SpEvent::Log("Nothing to pick up here.".into(), Color::Yellow));
        }
        events
    }

    pub fn drop_item(&mut self, item_id: u64) -> Vec<SpEvent> {
        let mut events = Vec::new();
        let pos = self.player.pos;
        if let Some(idx) = self.player.inventory.iter().position(|i| i.id == item_id) {
            let item = self.player.inventory.remove(idx);
            let name = item.name.clone();
            let gi   = GroundItem { item, pos };
            events.push(SpEvent::Log(format!("Dropped {}.", name), Color::Yellow));
            events.push(SpEvent::ItemDropped(gi.clone()));
            self.ground_items.push(gi);
            events.push(SpEvent::UpdatePlayer(self.player.clone()));
        }
        events
    }
}
