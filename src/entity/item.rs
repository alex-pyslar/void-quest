use serde::{Deserialize, Serialize};
use crate::world::pos::Pos;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemKind {
    Weapon { dmg: i32 },
    Armor  { def: i32 },
    Helmet { def: i32 },
    Ring   { effect: String, value: i32 },
    Potion { hp: i32, mp: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id:          u64,
    pub template_id: String,
    pub name:        String,
    pub symbol:      char,
    pub color:       String,
    pub kind:        ItemKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Equipment {
    pub weapon: Option<Item>,
    pub armor:  Option<Item>,
    pub helmet: Option<Item>,
    pub ring:   Option<Item>,
}

impl Equipment {
    pub fn atk_bonus(&self) -> i32 {
        self.weapon.as_ref().map_or(1, |w| {
            if let ItemKind::Weapon { dmg } = &w.kind { *dmg } else { 1 }
        })
    }
    pub fn def_bonus(&self) -> i32 {
        let a = self.armor.as_ref().map_or(0, |a| {
            if let ItemKind::Armor { def } = &a.kind { *def } else { 0 }
        });
        let h = self.helmet.as_ref().map_or(0, |h| {
            if let ItemKind::Helmet { def } = &h.kind { *def } else { 0 }
        });
        a + h
    }
    pub fn equip_item(&mut self, item: Item) -> Option<Item> {
        match &item.kind {
            ItemKind::Weapon { .. } => self.weapon.replace(item),
            ItemKind::Armor  { .. } => self.armor.replace(item),
            ItemKind::Helmet { .. } => self.helmet.replace(item),
            ItemKind::Ring   { .. } => self.ring.replace(item),
            _                       => None,
        }
    }
    pub fn slot_name(kind: &ItemKind) -> &'static str {
        match kind {
            ItemKind::Weapon { .. } => "Weapon",
            ItemKind::Armor  { .. } => "Armor",
            ItemKind::Helmet { .. } => "Helmet",
            ItemKind::Ring   { .. } => "Ring",
            ItemKind::Potion { .. } => "Consumable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundItem {
    pub item: Item,
    pub pos:  Pos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTemplate {
    pub id:      String,
    pub name:    String,
    pub name_ru: Option<String>,
    pub symbol:  char,
    pub color:   String,
    pub kind:    ItemKind,
}

impl ItemTemplate {
    pub fn display_name<'a>(&'a self, lang: &str) -> &'a str {
        if lang == "ru" {
            if let Some(ref n) = self.name_ru { return n.as_str(); }
        }
        self.name.as_str()
    }
}
