use serde::{Deserialize, Serialize};
use crate::world::Stats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDef {
    pub id:             String,
    pub name:           String,
    pub name_ru:        Option<String>,
    pub description:    String,
    pub description_ru: Option<String>,
    pub symbol:         char,
    pub color:          String,
    pub base_hp:        i32,
    pub base_mp:        i32,
    pub base_str:       i32,
    pub base_dex:       i32,
    pub base_int:       i32,
    pub base_vit:       i32,
    pub hp_per_level:   i32,
    pub mp_per_level:   i32,
    pub start_item:     Option<String>,
    #[serde(default)]
    pub spells:         Vec<String>,
}

impl ClassDef {
    pub fn make_stats(&self) -> Stats {
        Stats {
            hp: self.base_hp, max_hp: self.base_hp,
            mp: self.base_mp, max_mp: self.base_mp,
            str: self.base_str, dex: self.base_dex,
            int: self.base_int, vit: self.base_vit,
        }
    }

    pub fn display_name<'a>(&'a self, lang: &str) -> &'a str {
        if lang == "ru" {
            if let Some(ref n) = self.name_ru { return n.as_str(); }
        }
        self.name.as_str()
    }

    pub fn display_desc<'a>(&'a self, lang: &str) -> &'a str {
        if lang == "ru" {
            if let Some(ref d) = self.description_ru { return d.as_str(); }
        }
        self.description.as_str()
    }
}
