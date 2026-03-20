use rand::Rng;
use crate::engine::events::Color;

use crate::{
    entity::Monster,
    mapgen,
    world::{CombatEvent, Pos, SP_MAP_W, SP_MAP_H},
};
use super::{LocalGame, SpEvent};

impl LocalGame {
    pub fn tick(&mut self) -> Vec<SpEvent> {
        let mut events = Vec::new();
        self.tick += 1;
        let tick = self.tick;
        let mut rng = rand::thread_rng();

        // monster AI
        enum MAI { Attack(i32, String), Move(Pos), Idle }
        let mids: Vec<u64> = self.monsters.keys().cloned().collect();
        let ppos   = self.player.pos;
        let palive = self.player.stats.is_alive();
        let mut pdamage: Vec<(i32, String)> = Vec::new();
        let mut moved:   Vec<u64>           = Vec::new();

        for mid in &mids {
            let action: MAI = {
                let m = match self.monsters.get(mid) { Some(m) => m, None => continue };
                if !m.stats.is_alive() { continue; }
                if !palive { MAI::Idle } else {
                    let dsq = m.pos.dist_sq(ppos);
                    if dsq <= 2 {
                        MAI::Attack(m.stats.str, m.name.clone())
                    } else if dsq < 100 {
                        MAI::Move(m.pos.step_toward(ppos))
                    } else if dsq > 144 && m.pos != m.home {
                        MAI::Move(m.pos.step_toward(m.home))
                    } else {
                        MAI::Idle
                    }
                }
            };
            match action {
                MAI::Attack(sv, mn) => pdamage.push((sv.saturating_sub(2).max(1), mn)),
                MAI::Move(np) => {
                    let occ = self.player.pos == np
                        || self.monsters.values().filter(|x| x.id != *mid).any(|x| x.pos == np);
                    if self.map.passable(np.x, np.y) && !occ {
                        if let Some(m) = self.monsters.get_mut(mid) {
                            m.pos = np;
                            moved.push(*mid);
                        }
                    }
                }
                MAI::Idle => {}
            }
        }
        for mid in moved {
            if let Some(m) = self.monsters.get(&mid).cloned() {
                events.push(SpEvent::MonsterUpdate(m));
            }
        }

        // apply player damage
        for (dmg, mname) in pdamage {
            let actual = (dmg - self.player.defense() / 2).max(1);
            self.player.stats.hp -= actual;
            let killed = !self.player.stats.is_alive();
            events.push(SpEvent::Combat(CombatEvent {
                attacker: mname, target: self.player.name.clone(),
                damage: actual, is_crit: false, killed,
            }));
            if killed {
                self.player.stats.hp = self.player.stats.max_hp / 4;
                self.player.pos      = Pos::new(SP_MAP_W / 2, SP_MAP_H / 2);
                events.push(SpEvent::Log("You were slain! Respawned in town.".into(), Color::Red));
            }
            events.push(SpEvent::UpdatePlayer(self.player.clone()));
        }

        // HP/MP regen every 3 ticks
        if tick % 3 == 0 && self.player.stats.is_alive() {
            self.player.stats.hp = (self.player.stats.hp + 2 + self.player.stats.vit / 5)
                .min(self.player.stats.max_hp);
            self.player.stats.mp = (self.player.stats.mp + 3 + self.player.stats.int / 5)
                .min(self.player.stats.max_mp);
            events.push(SpEvent::UpdatePlayer(self.player.clone()));
        }

        // monster respawn every 60 ticks
        if tick % 60 == 0 {
            let positions = mapgen::monster_spawn_positions(&self.map);
            if !positions.is_empty() {
                let target = self.cfg.world.monster_count;
                let cur    = self.monsters.len();
                if cur < target {
                    let templates: Vec<String> = self.cfg.monsters.keys().cloned().collect();
                    for _ in 0..(target - cur).min(5) {
                        let tmpl_id = templates[rng.gen_range(0..templates.len())].clone();
                        let (x, y)  = positions[rng.gen_range(0..positions.len())];
                        if let Some(tmpl) = self.cfg.monsters.get(&tmpl_id) {
                            self.next_id += 1;
                            let m = Monster {
                                id:          self.next_id,
                                template_id: tmpl.id.clone(),
                                name:        tmpl.name.clone(),
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
                            events.push(SpEvent::MonsterUpdate(m.clone()));
                            self.monsters.insert(m.id, m);
                        }
                    }
                }
            }
        }
        events
    }
}
