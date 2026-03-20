use crate::entity::{ClassDef, ItemKind, ItemTemplate, MonsterTemplate, Spell};

pub fn default_classes() -> Vec<ClassDef> {
    vec![
        ClassDef {
            id: "warrior".into(), name: "Warrior".into(),
            name_ru: None,
            description: "Sturdy melee fighter. High HP and strength.".into(),
            description_ru: None,
            symbol: '@', color: "red".into(),
            base_hp: 130, base_mp: 20, base_str: 18, base_dex: 10, base_int: 5, base_vit: 15,
            hp_per_level: 15, mp_per_level: 2, start_item: Some("iron_sword".into()),
            spells: vec!["battle_cry".into(), "whirlwind".into()],
        },
        ClassDef {
            id: "mage".into(), name: "Mage".into(),
            name_ru: None,
            description: "Frail but devastating arcane caster. High INT.".into(),
            description_ru: None,
            symbol: '@', color: "cyan".into(),
            base_hp: 65, base_mp: 130, base_str: 6, base_dex: 10, base_int: 22, base_vit: 7,
            hp_per_level: 7, mp_per_level: 15, start_item: Some("staff".into()),
            spells: vec!["fireball".into(), "ice_lance".into(), "thunder_bolt".into()],
        },
        ClassDef {
            id: "rogue".into(), name: "Rogue".into(),
            name_ru: None,
            description: "Fast and precise. High DEX, critical strikes.".into(),
            description_ru: None,
            symbol: '@', color: "green".into(),
            base_hp: 90, base_mp: 45, base_str: 12, base_dex: 22, base_int: 8, base_vit: 10,
            hp_per_level: 10, mp_per_level: 5, start_item: Some("dagger".into()),
            spells: vec!["shadow_step".into(), "poison_blade".into()],
        },
        ClassDef {
            id: "paladin".into(), name: "Paladin".into(),
            name_ru: None,
            description: "Holy warrior. Balanced stats, self-healing.".into(),
            description_ru: None,
            symbol: '@', color: "yellow".into(),
            base_hp: 110, base_mp: 80, base_str: 14, base_dex: 9, base_int: 12, base_vit: 13,
            hp_per_level: 12, mp_per_level: 8, start_item: Some("mace".into()),
            spells: vec!["holy_light".into(), "divine_smite".into()],
        },
    ]
}

pub fn default_monsters() -> Vec<MonsterTemplate> {
    vec![
        MonsterTemplate { id: "goblin".into(),   name: "Goblin".into(),   name_ru: None, symbol: 'g', color: "green".into(),   level: 1, hp: 22,  str: 5,  vit: 3,  xp_reward: 12,  loot_table: vec!["hp_potion".into()] },
        MonsterTemplate { id: "wolf".into(),     name: "Wolf".into(),     name_ru: None, symbol: 'w', color: "white".into(),  level: 2, hp: 38,  str: 8,  vit: 4,  xp_reward: 22,  loot_table: vec![] },
        MonsterTemplate { id: "orc".into(),      name: "Orc".into(),      name_ru: None, symbol: 'O', color: "yellow".into(), level: 4, hp: 75,  str: 14, vit: 8,  xp_reward: 50,  loot_table: vec!["iron_sword".into(), "hp_potion".into()] },
        MonsterTemplate { id: "skeleton".into(), name: "Skeleton".into(), name_ru: None, symbol: 'S', color: "white".into(),  level: 4, hp: 65,  str: 12, vit: 7,  xp_reward: 55,  loot_table: vec!["iron_sword".into()] },
        MonsterTemplate { id: "troll".into(),    name: "Troll".into(),    name_ru: None, symbol: 'T', color: "magenta".into(),level: 5, hp: 130, str: 20, vit: 14, xp_reward: 105, loot_table: vec!["leather_armor".into(), "hp_potion".into()] },
        MonsterTemplate { id: "vampire".into(),  name: "Vampire".into(),  name_ru: None, symbol: 'V', color: "red".into(),    level: 6, hp: 100, str: 18, vit: 10, xp_reward: 130, loot_table: vec!["full_potion".into()] },
        MonsterTemplate { id: "dragon".into(),   name: "Cave Dragon".into(), name_ru: None, symbol: 'D', color: "red".into(), level: 9, hp: 280, str: 32, vit: 22, xp_reward: 450, loot_table: vec!["plate_armor".into(), "full_potion".into()] },
    ]
}

pub fn default_items() -> Vec<ItemTemplate> {
    vec![
        ItemTemplate { id: "iron_sword".into(),    name: "Iron Sword".into(),    name_ru: None, symbol: '/', color: "white".into(),   kind: ItemKind::Weapon { dmg: 8 } },
        ItemTemplate { id: "dagger".into(),        name: "Dagger".into(),        name_ru: None, symbol: '!', color: "white".into(),   kind: ItemKind::Weapon { dmg: 5 } },
        ItemTemplate { id: "staff".into(),         name: "Magic Staff".into(),   name_ru: None, symbol: '|', color: "cyan".into(),    kind: ItemKind::Weapon { dmg: 7 } },
        ItemTemplate { id: "mace".into(),          name: "Holy Mace".into(),     name_ru: None, symbol: '\\',color: "yellow".into(),  kind: ItemKind::Weapon { dmg: 9 } },
        ItemTemplate { id: "great_sword".into(),   name: "Greatsword".into(),    name_ru: None, symbol: '/', color: "blue".into(),    kind: ItemKind::Weapon { dmg: 14 } },
        ItemTemplate { id: "leather_armor".into(), name: "Leather Armor".into(), name_ru: None, symbol: ']', color: "yellow".into(),  kind: ItemKind::Armor  { def: 5 } },
        ItemTemplate { id: "chain_mail".into(),    name: "Chain Mail".into(),    name_ru: None, symbol: ']', color: "white".into(),   kind: ItemKind::Armor  { def: 9 } },
        ItemTemplate { id: "plate_armor".into(),   name: "Plate Armor".into(),   name_ru: None, symbol: ']', color: "blue".into(),    kind: ItemKind::Armor  { def: 15 } },
        ItemTemplate { id: "leather_helm".into(),  name: "Leather Helm".into(),  name_ru: None, symbol: '^', color: "yellow".into(),  kind: ItemKind::Helmet { def: 3 } },
        ItemTemplate { id: "iron_helm".into(),     name: "Iron Helm".into(),     name_ru: None, symbol: '^', color: "white".into(),   kind: ItemKind::Helmet { def: 6 } },
        ItemTemplate { id: "hp_potion".into(),     name: "HP Potion".into(),     name_ru: None, symbol: '*', color: "red".into(),     kind: ItemKind::Potion { hp: 40, mp: 0 } },
        ItemTemplate { id: "mp_potion".into(),     name: "MP Potion".into(),     name_ru: None, symbol: '*', color: "blue".into(),    kind: ItemKind::Potion { hp: 0,  mp: 30 } },
        ItemTemplate { id: "full_potion".into(),   name: "Elixir".into(),        name_ru: None, symbol: '*', color: "magenta".into(), kind: ItemKind::Potion { hp: 60, mp: 45 } },
        ItemTemplate { id: "power_ring".into(),    name: "Ring of Power".into(), name_ru: None, symbol: '=', color: "yellow".into(),  kind: ItemKind::Ring   { effect: "str".into(), value: 4 } },
    ]
}

pub fn default_spells() -> Vec<Spell> {
    vec![
        Spell { id: "fireball".into(),    name: "Fireball".into(),    name_ru: None, icon: '*', color: "red".into(),    mp_cost: 20, damage: 35, heal: 0,  effect: "damage".into(), classes: vec!["mage".into()] },
        Spell { id: "holy_light".into(),  name: "Holy Light".into(),  name_ru: None, icon: '+', color: "yellow".into(), mp_cost: 20, damage: 0,  heal: 40, effect: "heal".into(),   classes: vec!["paladin".into()] },
        Spell { id: "battle_cry".into(),  name: "Battle Cry".into(),  name_ru: None, icon: '!', color: "red".into(),    mp_cost: 15, damage: 25, heal: 10, effect: "damage".into(), classes: vec!["warrior".into()] },
        Spell { id: "shadow_step".into(), name: "Shadow Step".into(), name_ru: None, icon: '~', color: "magenta".into(), mp_cost: 15, damage: 30, heal: 0, effect: "damage".into(), classes: vec!["rogue".into()] },
    ]
}
