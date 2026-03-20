// bridge.rs — cxx FFI bridge: Rust (GameApp) ↔ C++ (VqWidget3D).

use crate::client::app::GameApp;

#[cxx::bridge(namespace = "vq")]
pub mod ffi {

    // ── Entity visible in 3D world ────────────────────────────────────────────

    #[derive(Clone)]
    pub struct EntityInfo {
        pub x:    i32,
        pub y:    i32,
        /// 0=player-self, 1=other-player, 2=monster, 3=ground-item
        pub kind: u8,
        pub r: u8, pub g: u8, pub b: u8,
        pub anim: u32,
    }

    // ── Camera / map info ─────────────────────────────────────────────────────

    pub struct MapInfo {
        pub map_w:    i32,
        pub map_h:    i32,
        pub cam_x:    i32,
        pub cam_y:    i32,
        pub view_w:   i32,
        pub view_h:   i32,
        pub player_x: i32,
        pub player_y: i32,
    }

    // ── Full HUD snapshot ─────────────────────────────────────────────────────

    pub struct HudData {
        /// 0=MainMenu 1=Connect 2=Login 3=CharCreate 4=Playing
        pub screen:    u8,
        pub anim_tick: u64,

        // ── Playing: player stats ─────────────────────────────────────────
        pub player_name:  String,
        pub player_class: String,
        pub player_level: i32,
        pub hp: i32, pub max_hp: i32,
        pub mp: i32, pub max_mp: i32,
        pub xp: i64, pub xp_next: i64,
        pub stat_str: i32, pub stat_dex: i32,
        pub stat_int: i32, pub stat_vit: i32,
        pub atk: i32, pub def_: i32,
        pub stat_points: i32,
        pub world_name: String,

        // ── Equipment ────────────────────────────────────────────────────
        pub eq_weapon: String, pub eq_armor: String,
        pub eq_helmet: String, pub eq_ring:  String,
        pub eq_weapon_r: u8, pub eq_weapon_g: u8, pub eq_weapon_b: u8,
        pub eq_armor_r:  u8, pub eq_armor_g:  u8, pub eq_armor_b:  u8,
        pub eq_helmet_r: u8, pub eq_helmet_g: u8, pub eq_helmet_b: u8,
        pub eq_ring_r:   u8, pub eq_ring_g:   u8, pub eq_ring_b:   u8,

        // ── Playing: UI state ─────────────────────────────────────────────
        pub chat_active: bool,
        pub chat_buf: String,
        pub inv_open:   bool,
        pub equip_open: bool,
        pub inv_sel:    i32,

        // JSON blobs
        pub inventory_json: String,
        pub log_json:       String,
        pub nearby_json:    String,

        // ── Camera & mode ─────────────────────────────────────────────────
        /// 0=TopDown 1=ThirdPerson 2=FirstPerson 3=2D-pixel
        pub camera_mode:   u8,
        pub is_singleplayer: bool,
        /// Last movement direction (for first-person look)
        pub player_face_dx: i32,
        pub player_face_dy: i32,
        /// Current biome/zone name shown in HUD
        pub zone_style: String,

        // ── MainMenu ──────────────────────────────────────────────────────
        pub menu_sel: i32,

        // ── Connect ───────────────────────────────────────────────────────
        pub conn_host:   String,
        pub conn_port:   String,
        pub conn_cursor: i32,

        // ── Login ─────────────────────────────────────────────────────────
        pub login_user:        String,
        pub login_pass_len:    i32,
        pub login_cursor:      i32,
        pub login_is_register: bool,

        // ── CharCreate (MP and SP) ────────────────────────────────────────
        pub char_name:       String,
        pub char_class_name: String,
        pub char_class_desc: String,
        pub char_symbol:     String,
        pub char_col_r: u8, pub char_col_g: u8, pub char_col_b: u8,
        pub char_cursor:    i32,
        pub char_n_classes: i32,
        pub char_is_sp:     bool,   // true = creating for singleplayer

        // ── Error overlay ─────────────────────────────────────────────────
        pub error_msg: String,
    }

    // ── Commands returned from key events ─────────────────────────────────────

    #[derive(Debug)]
    pub enum CmdKind {
        None       = 0,
        Connect    = 1,
        SendMsg    = 2,
        Disconnect = 3,
        Quit       = 4,
        StartSP    = 5,  // start singleplayer; json = {name,class_id,symbol,color}
    }

    #[derive(Debug)]
    pub struct Cmd {
        pub kind: CmdKind,
        pub host: String,
        pub port: u16,
        pub json: String,
    }

    // ── Rust API (called by C++) ───────────────────────────────────────────────

    extern "Rust" {
        type GameApp;

        fn tick(self: &mut GameApp);
        fn on_key(self: &mut GameApp, qt_key: i32, mods: u32, text: &str) -> Cmd;
        fn on_resize(self: &mut GameApp, view_w: i32, view_h: i32);
        fn do_connect(self: &mut GameApp, host: &str, port: u16);
        fn do_send(self: &mut GameApp, json: &str);
        fn do_disconnect(self: &mut GameApp);

        fn get_map_info(self: &GameApp) -> MapInfo;
        fn get_tile(self: &GameApp, x: i32, y: i32) -> u8;
        fn get_entities(self: &GameApp) -> Vec<EntityInfo>;
        fn get_hud(self: &GameApp) -> HudData;
    }

    unsafe extern "C++" {
        include!("vqwidget.h");
        fn run_app(game: Box<GameApp>);
    }
}
