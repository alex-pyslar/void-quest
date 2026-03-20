use serde::{Deserialize, Serialize};
use crate::world::{Pos, Stats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monster {
    pub id:          u64,
    pub template_id: String,
    pub name:        String,
    pub symbol:      char,
    pub color:       String,
    pub pos:         Pos,
    pub home:        Pos,
    pub level:       u32,
    pub stats:       Stats,
    pub xp_reward:   u64,
    pub loot_table:  Vec<String>,
    pub target:      Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterTemplate {
    pub id:         String,
    pub name:       String,
    pub name_ru:    Option<String>,
    pub symbol:     char,
    pub color:      String,
    pub level:      u32,
    pub hp:         i32,
    pub str:        i32,
    pub vit:        i32,
    pub xp_reward:  u64,
    pub loot_table: Vec<String>,
}

impl MonsterTemplate {
    pub fn make_stats(&self) -> Stats {
        Stats {
            hp: self.hp, max_hp: self.hp,
            mp: 0, max_mp: 0,
            str: self.str, dex: 5, int: 2, vit: self.vit,
        }
    }

    pub fn display_name<'a>(&'a self, lang: &str) -> &'a str {
        if lang == "ru" {
            if let Some(ref n) = self.name_ru { return n.as_str(); }
        }
        self.name.as_str()
    }
}
