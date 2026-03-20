use std::collections::HashMap;
use std::fs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct LocaleFile {
    lang: String,
    #[serde(default)]
    en: HashMap<String, String>,
    #[serde(default)]
    ru: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LocaleConfig {
    pub lang:    String,
    strings:     HashMap<String, String>,
}

impl LocaleConfig {
    pub fn load() -> Self {
        let raw = fs::read_to_string("config/locale.toml")
            .ok()
            .and_then(|s| toml::from_str::<LocaleFile>(&s).ok());

        match raw {
            Some(file) => {
                // Start with full code defaults (guarantees all keys exist),
                // then overlay any overrides from the TOML file.
                let mut base = match file.lang.as_str() {
                    "ru" => Self::default_ru(),
                    _    => Self::default_en(),
                };
                let overrides = match file.lang.as_str() {
                    "ru" => file.ru,
                    _    => file.en,
                };
                for (k, v) in overrides {
                    base.strings.insert(k, v);
                }
                base.lang = file.lang;
                base
            }
            None => Self::default_en(),
        }
    }

    /// Toggle between English and Russian.
    pub fn switch_lang(&mut self) {
        if self.lang == "en" {
            *self = Self::default_ru();
        } else {
            *self = Self::default_en();
        }
    }

    pub fn default_en() -> Self {
        let mut s = HashMap::new();
        // Window titles
        s.insert("title_map".into(),       " Map ".into());
        s.insert("title_log".into(),       " Log ".into());
        s.insert("title_stats".into(),     " Character ".into());
        s.insert("title_equipment".into(), " Equipment ".into());
        s.insert("title_inventory".into(), " Inventory ".into());
        s.insert("title_spells".into(),    " Spells ".into());
        s.insert("title_help".into(),      " Help (?) ".into());
        s.insert("title_settings".into(),  " Settings (F10) ".into());
        // Playing hints
        s.insert("hint_playing_sp".into(),
            "   WASD:move  F:attack  1-5:spell  U:use  G:equip  P:pickup  ?:help  Esc:menu".into());
        s.insert("hint_playing_mp".into(),
            "   Enter:chat   WASD:move  F:attack  U:use  G:equip  ?:help  Esc:quit".into());
        s.insert("hint_inv".into(),        "J/K: nav  U: use  G: equip  X: drop".into());
        s.insert("hint_inv_empty".into(),  "(empty)".into());
        s.insert("hint_windows".into(),    "m:Map l:Log c:Stats i:Inv n:Spells ?:Help F10:Settings Ctrl+R:reset".into());
        // Badges
        s.insert("badge_solo".into(),      " SOLO".into());
        s.insert("badge_online".into(),    " ONLINE".into());
        s.insert("badge_sp_map".into(),    " SOLO".into());
        // Log messages
        s.insert("log_welcome_sp".into(),  "Welcome to Voidlands! [SINGLEPLAYER]".into());
        s.insert("log_welcome_mp".into(),  "Welcome to Voidlands!".into());
        s.insert("log_controls_sp".into(),
            "WASD:move  F:attack  U:use  G:equip  P:pickup  J/K:inv  ?:help  Esc:menu".into());
        s.insert("log_controls_mp".into(),
            "WASD:move  F:attack  U:use  G:equip  P:pickup  J/K:inv  Enter:chat  Esc:quit".into());
        // Settings window
        s.insert("settings_lang_label".into(), "Language:".into());
        s.insert("settings_lang_val".into(),   "English (en)".into());
        s.insert("settings_lang_hint".into(),  "Tab/Enter: switch language".into());
        s.insert("settings_3d_label".into(),   "Default view:".into());
        s.insert("settings_3d_on".into(),      "3D First-Person".into());
        s.insert("settings_3d_off".into(),     "2D Top-Down".into());
        // Help window content (lines joined with |)
        s.insert("help_content".into(), [
            "=== VOIDQUEST — HELP ===",
            "",
            "── MOVEMENT ──────────────────────────",
            "  WASD / Arrow keys : move / walk",
            "  Q / E             : diagonal move",
            "  Z                 : diagonal move (down-left)",
            "  (in 3D) A/D       : turn left/right",
            "  (in 3D) Q/E       : strafe left/right",
            "  Mouse             : look around (3D mode)",
            "",
            "── COMBAT ────────────────────────────",
            "  F                 : attack nearest monster",
            "  1-5               : cast spell (singleplayer)",
            "",
            "── ITEMS ─────────────────────────────",
            "  J / PageDown      : next inventory slot",
            "  K / PageUp        : previous inventory slot",
            "  U                 : use selected item",
            "  G                 : equip selected item",
            "  X                 : drop selected item",
            "  P                 : pick up nearby items",
            "",
            "── VIEW ──────────────────────────────",
            "  F3                : toggle 3D / 2D view",
            "  PageUp/PageDown   : look up/down (3D)",
            "  R                 : level horizon (3D)",
            "  Mouse Y           : pitch (3D)",
            "",
            "── WINDOWS ───────────────────────────",
            "  M                 : toggle Map",
            "  L                 : toggle Log",
            "  C                 : toggle Stats",
            "  I                 : toggle Inventory",
            "  N                 : toggle Spells",
            "  ?                 : toggle Help",
            "  F10               : toggle Settings",
            "  Ctrl+R            : reset window layout",
            "  Drag title bar    : move window",
            "  Drag corner [\\]   : resize window",
            "",
            "── WORLD ─────────────────────────────",
            "  Global Map        : navigate between zones",
            "  Arrow keys        : move cursor on world map",
            "  Enter             : enter selected zone",
            "  Esc (in zone)     : return to world map",
            "",
            "── MULTIPLAYER ───────────────────────",
            "  Enter             : open chat",
            "  Esc               : disconnect and quit",
            "",
            "── ABOUT ─────────────────────────────",
            "  VoidQuest — terminal roguelike RPG",
            "  Explore procedurally generated zones,",
            "  fight monsters, collect loot, level up.",
            "  Each zone has unique terrain and enemies.",
        ].join("\n"));
        // Global map
        s.insert("global_map_title".into(), "=== VOIDLANDS WORLD MAP ===".into());
        s.insert("global_map_hint".into(),  "Arrows:move  Enter:enter zone  Esc:main menu".into());
        s.insert("global_map_unvisited".into(), "(unexplored)".into());
        Self { lang: "en".into(), strings: s }
    }

    pub fn default_ru() -> Self {
        let mut s = HashMap::new();
        // Заголовки окон
        s.insert("title_map".into(),       " Карта ".into());
        s.insert("title_log".into(),       " Журнал ".into());
        s.insert("title_stats".into(),     " Персонаж ".into());
        s.insert("title_equipment".into(), " Снаряжение ".into());
        s.insert("title_inventory".into(), " Инвентарь ".into());
        s.insert("title_spells".into(),    " Заклинания ".into());
        s.insert("title_help".into(),      " Справка (?) ".into());
        s.insert("title_settings".into(),  " Настройки (F10) ".into());
        // Подсказки
        s.insert("hint_playing_sp".into(),
            "   WASD:движение  F:атака  1-5:заклинание  U:использовать  G:надеть  ?:справка  Esc:меню".into());
        s.insert("hint_playing_mp".into(),
            "   Enter:чат   WASD:движение  F:атака  U:использ  G:надеть  ?:справка  Esc:выйти".into());
        s.insert("hint_inv".into(),        "J/K:выбор  U:использ  G:надеть  X:выбросить".into());
        s.insert("hint_inv_empty".into(),  "(пусто)".into());
        s.insert("hint_windows".into(),    "m:Карта l:Журнал c:Стат i:Инв n:Заклинания ?:Справка F10:Настройки Ctrl+R:сброс".into());
        // Знаки
        s.insert("badge_solo".into(),      " СОЛО".into());
        s.insert("badge_online".into(),    " ОНЛАЙН".into());
        s.insert("badge_sp_map".into(),    " СОЛО".into());
        // Лог
        s.insert("log_welcome_sp".into(),  "Добро пожаловать в Войдлэнд! [ОДИНОЧНАЯ]".into());
        s.insert("log_welcome_mp".into(),  "Добро пожаловать в Войдлэнд!".into());
        s.insert("log_controls_sp".into(),
            "WASD:движение  F:атака  U:использ  G:надеть  P:подобрать  J/K:инв  ?:справка  Esc:меню".into());
        s.insert("log_controls_mp".into(),
            "WASD:движение  F:атака  U:использ  G:надеть  P:подобрать  J/K:инв  Enter:чат  Esc:выйти".into());
        // Настройки
        s.insert("settings_lang_label".into(), "Язык:".into());
        s.insert("settings_lang_val".into(),   "Русский (ru)".into());
        s.insert("settings_lang_hint".into(),  "Tab/Enter: переключить язык".into());
        s.insert("settings_3d_label".into(),   "Вид по умолчанию:".into());
        s.insert("settings_3d_on".into(),      "3D Вид от первого лица".into());
        s.insert("settings_3d_off".into(),     "2D Вид сверху".into());
        // Справка
        s.insert("help_content".into(), [
            "=== VOIDQUEST — СПРАВКА ===",
            "",
            "── ДВИЖЕНИЕ ──────────────────────────",
            "  WASD / стрелки  : перемещение",
            "  Q / E           : по диагонали",
            "  Z               : по диагонали (вниз-влево)",
            "  (3D) A/D        : поворот влево/вправо",
            "  (3D) Q/E        : страфить влево/вправо",
            "  Мышь            : осмотреться (режим 3D)",
            "",
            "── БОЙ ───────────────────────────────",
            "  F               : атаковать ближайшего монстра",
            "  1-5             : заклинание (одиночная игра)",
            "",
            "── ПРЕДМЕТЫ ──────────────────────────",
            "  J / PageDown    : следующий слот инвентаря",
            "  K / PageUp      : предыдущий слот",
            "  U               : использовать предмет",
            "  G               : надеть предмет",
            "  X               : выбросить предмет",
            "  P               : подобрать предметы рядом",
            "",
            "── ВИД ───────────────────────────────",
            "  F3              : переключить 3D / 2D вид",
            "  PageUp/PageDown : смотреть вверх/вниз (3D)",
            "  R               : выровнять горизонт (3D)",
            "  Мышь по Y       : наклон камеры (3D)",
            "",
            "── ОКНА ──────────────────────────────",
            "  M               : переключить Карту",
            "  L               : переключить Журнал",
            "  C               : переключить Статы",
            "  I               : переключить Инвентарь",
            "  N               : переключить Заклинания",
            "  ?               : переключить Справку",
            "  F10             : переключить Настройки",
            "  Ctrl+R          : сбросить расположение окон",
            "  Тянуть заголовок: переместить окно",
            "  Тянуть угол [\\] : изменить размер",
            "",
            "── МИР ───────────────────────────────",
            "  Глобальная карта: перемещение между зонами",
            "  Стрелки         : курсор на карте мира",
            "  Enter           : войти в выбранную зону",
            "  Esc (в зоне)    : вернуться на карту мира",
            "",
            "── МУЛЬТИПЛЕЕР ───────────────────────",
            "  Enter           : открыть чат",
            "  Esc             : отключиться и выйти",
            "",
            "── ОБ ИГРЕ ───────────────────────────",
            "  VoidQuest — терминальный roguelike RPG",
            "  Исследуй процедурно генерируемые зоны,",
            "  сражайся с монстрами, собирай лут, расти.",
        ].join("\n"));
        // Глобальная карта
        s.insert("global_map_title".into(), "=== КАРТА МИРА ВОЙДЛЭНД ===".into());
        s.insert("global_map_hint".into(),  "Стрелки:курсор  Enter:войти в зону  Esc:главное меню".into());
        s.insert("global_map_unvisited".into(), "(не исследована)".into());
        Self { lang: "ru".into(), strings: s }
    }

    /// Look up a locale string by key. Returns the value or key itself if not found.
    pub fn t(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

impl Default for LocaleConfig {
    fn default() -> Self { Self::default_en() }
}
