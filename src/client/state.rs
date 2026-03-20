// state.rs — Game state machine + key/message handling.
// Pure logic only — zero rendering code.

use std::collections::{HashMap, VecDeque};
use crate::client::bridge::ffi::{Cmd, CmdKind};
use crate::entity::{ClassDef, GroundItem, Monster, Player};
use crate::world::GameMap;
use crate::protocol::ServerMsg;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const LOG_MAX: usize = 50;
pub const SYMBOLS: &[&str] = &["@","$","%","&","?","!","*","~","+","#"];
pub const COLORS:  &[&str] = &["white","red","green","blue","yellow","cyan","magenta","gray"];

// Qt key codes
const QK_RETURN:    i32 = 0x01000004;
const QK_ENTER:     i32 = 0x01000005;
const QK_ESCAPE:    i32 = 0x01000000;
const QK_BACKSPACE: i32 = 0x01000003;
const QK_TAB:       i32 = 0x01000001;
const QK_F1:        i32 = 0x01000030;
const QK_F5:        i32 = 0x01000034;
const QK_UP:        i32 = 0x01000013;
const QK_DOWN:      i32 = 0x01000015;
const QK_LEFT:      i32 = 0x01000012;
const QK_RIGHT:     i32 = 0x01000014;
const QK_A: i32 = 0x41;
const QK_B: i32 = 0x42;
const QK_C: i32 = 0x43;
const QK_D: i32 = 0x44;
const QK_E: i32 = 0x45;
const QK_H: i32 = 0x48;
const QK_I: i32 = 0x49;
const QK_J: i32 = 0x4a;
const QK_K: i32 = 0x4b;
const QK_L: i32 = 0x4c;
const QK_N: i32 = 0x4e;
const QK_Q: i32 = 0x51;
const QK_S: i32 = 0x53;
const QK_T: i32 = 0x54;
const QK_U: i32 = 0x55;
const QK_W: i32 = 0x57;
const QK_Y: i32 = 0x59;
const QK_Z: i32 = 0x5a;

// Zone biome cycle (used in singleplayer Z key)
pub const ZONE_STYLES: &[&str] = &[
    "Wilderness", "Dungeon", "Desert", "Forest", "Swamp", "Town",
];

// ── Screen enum ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Connect,
    Login,
    CharCreate,  // used for both MP and SP (check sp_pending)
    Playing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginMode { Login, Register }

// ── JSON builders ─────────────────────────────────────────────────────────────

fn mk_register(u: &str, p: &str) -> String {
    serde_json::json!({"t":"Register","d":{"username":u,"password":p}}).to_string()
}
fn mk_login(u: &str, p: &str) -> String {
    serde_json::json!({"t":"Login","d":{"username":u,"password":p}}).to_string()
}
fn mk_create_char(name: &str, class_id: &str, symbol: &str, color: &str) -> String {
    serde_json::json!({"t":"CreateChar","d":{"name":name,"class_id":class_id,"symbol":symbol,"color":color}}).to_string()
}
fn mk_move(dx: i32, dy: i32) -> String {
    serde_json::json!({"t":"Move","d":{"dx":dx,"dy":dy}}).to_string()
}
fn mk_pickup()           -> String { r#"{"t":"Pickup"}"#.to_string() }
fn mk_equip(id: u64)     -> String { serde_json::json!({"t":"Equip","d":{"item_id":id}}).to_string() }
fn mk_use_item(id: u64)  -> String { serde_json::json!({"t":"UseItem","d":{"item_id":id}}).to_string() }
fn mk_drop_item(id: u64) -> String { serde_json::json!({"t":"DropItem","d":{"item_id":id}}).to_string() }
fn mk_chat(msg: &str)    -> String { serde_json::json!({"t":"Chat","d":{"msg":msg}}).to_string() }
fn mk_attack(id: u64)    -> String { serde_json::json!({"t":"Attack","d":{"target_id":id}}).to_string() }

// ── GameState ─────────────────────────────────────────────────────────────────

pub struct GameState {
    pub screen: Screen,

    // MainMenu — 0=Play(MP), 1=Singleplayer, 2=Quit
    pub menu_sel: usize,
    pub sp_pending: bool,   // creating character for singleplayer

    // Connect
    pub conn_host:   String,
    pub conn_port:   String,
    pub conn_cursor: usize,

    // Login
    pub login_mode:   LoginMode,
    pub login_user:   String,
    pub login_pass:   String,
    pub login_cursor: usize,

    // CharCreate
    pub char_name:       String,
    pub char_class_idx:  usize,
    pub char_symbol_idx: usize,
    pub char_color_idx:  usize,
    pub char_cursor:     usize,
    pub avail_classes:   Vec<ClassDef>,

    // Playing
    pub my_id:        u64,
    pub world:        GameMap,
    pub players:      HashMap<u64, Player>,
    pub monsters:     HashMap<u64, Monster>,
    pub ground_items: Vec<GroundItem>,
    pub log:          VecDeque<String>,

    pub inv_open:    bool,
    pub equip_open:  bool,
    pub inv_sel:     usize,
    pub chat_active: bool,
    pub chat_buf:    String,

    // Camera mode: 0=TopDown 1=ThirdPerson 2=FirstPerson 3=2D
    pub camera_mode: u8,
    // Last movement direction (for first-person camera)
    pub player_face_dx: i32,
    pub player_face_dy: i32,
    // Current zone biome index (singleplayer)
    pub zone_idx: usize,

    // Animation
    pub anim_tick: u64,

    // Error overlay
    pub error_msg:        String,
    pub error_clear_tick: u64,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            screen:       Screen::MainMenu,
            menu_sel:     0,
            sp_pending:   false,
            conn_host:    "127.0.0.1".to_string(),
            conn_port:    "7777".to_string(),
            conn_cursor:  0,
            login_mode:   LoginMode::Login,
            login_user:   String::new(),
            login_pass:   String::new(),
            login_cursor: 0,
            char_name:       String::new(),
            char_class_idx:  0,
            char_symbol_idx: 0,
            char_color_idx:  0,
            char_cursor:     0,
            avail_classes:   Vec::new(),
            my_id:        0,
            world:        GameMap::new(0, 0),
            players:      HashMap::new(),
            monsters:     HashMap::new(),
            ground_items: Vec::new(),
            log:          VecDeque::new(),
            inv_open:     false,
            equip_open:   false,
            inv_sel:      0,
            chat_active:  false,
            chat_buf:     String::new(),
            camera_mode:  0,
            player_face_dx: 0,
            player_face_dy: 1,  // default facing south
            zone_idx:     0,
            anim_tick:    0,
            error_msg:    String::new(),
            error_clear_tick: 0,
        }
    }
}

impl GameState {
    pub fn my_player(&self) -> Option<&Player> { self.players.get(&self.my_id) }

    pub fn add_log(&mut self, line: impl Into<String>) {
        self.log.push_back(line.into());
        while self.log.len() > LOG_MAX { self.log.pop_front(); }
    }

    // ── Server message handler ────────────────────────────────────────────────

    pub fn handle_message(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::RegisterOk => {
                self.add_log("Registration successful — please log in.");
                self.login_mode = LoginMode::Login;
            }
            ServerMsg::LoginOk  => {}
            ServerMsg::CharOk   => { self.add_log("Character created!"); }
            ServerMsg::Pong     => {}
            ServerMsg::Err { msg } => {
                self.add_log(format!("[ERROR] {}", msg));
                self.error_msg        = msg;
                self.error_clear_tick = self.anim_tick + 120;
            }
            ServerMsg::NeedChar { classes } => {
                self.avail_classes   = classes;
                self.char_class_idx  = 0;
                self.char_symbol_idx = 0;
                self.char_color_idx  = 0;
                self.char_cursor     = 0;
                self.char_name.clear();
                self.sp_pending = false;
                self.screen = Screen::CharCreate;
            }
            ServerMsg::WorldInit { player_id, map, players, monsters, items } => {
                self.my_id = player_id;
                self.world = map;
                self.players.clear(); self.monsters.clear(); self.ground_items.clear();
                for p in players  { self.players.insert(p.id, p); }
                for m in monsters { self.monsters.insert(m.id, m); }
                self.ground_items = items;
                self.screen = Screen::Playing;
                self.add_log(format!("Welcome to {}!", self.world.name));
            }
            ServerMsg::PlayerUpdate(p) => { self.players.insert(p.id, p); }
            ServerMsg::PlayerLeft(id)  => { self.players.remove(&id); }
            ServerMsg::MonsterUpdate(m)=> { self.monsters.insert(m.id, m); }
            ServerMsg::MonsterDied { id, xp } => {
                self.monsters.remove(&id);
                if xp > 0 { self.add_log(format!("You gain {} XP.", xp)); }
            }
            ServerMsg::ItemDropped(gi)   => { self.ground_items.push(gi); }
            ServerMsg::ItemPickedUp { item_id, by } => {
                self.ground_items.retain(|gi| gi.item.id != item_id);
                self.add_log(format!("{} picked up an item.", by));
            }
            ServerMsg::Combat(ce) => {
                self.add_log(format!("{} hits {} for {}{}{}",
                    ce.attacker, ce.target, ce.damage,
                    if ce.is_crit { " [CRIT]" } else { "" },
                    if ce.killed  { " — killed!" } else { "" }));
            }
            ServerMsg::Chat { from, msg }  => { self.add_log(format!("<{}> {}", from, msg)); }
            ServerMsg::System(s)           => { self.add_log(format!("* {}", s)); }
            ServerMsg::LevelUp { level, stat_points } => {
                self.add_log(format!("*** LEVEL UP! Level {} (+{} stat points) ***",
                             level, stat_points));
            }
        }
    }

    // ── Key handler ───────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, qt_key: i32, _mods: u32, text: &str) -> Cmd {
        let none = Cmd { kind: CmdKind::None, host: String::new(), port: 0, json: String::new() };

        if !self.error_msg.is_empty() && self.anim_tick >= self.error_clear_tick {
            self.error_msg.clear();
        }

        // F5 always toggles camera mode (in any screen, for instant feedback)
        if qt_key == QK_F5 {
            self.camera_mode = (self.camera_mode + 1) % 4;
            return none;
        }

        match self.screen {

            // ─── Main Menu ────────────────────────────────────────────────────
            Screen::MainMenu => {
                match qt_key {
                    k if k == QK_UP   || k == QK_K => { if self.menu_sel > 0 { self.menu_sel -= 1; } }
                    k if k == QK_DOWN || k == QK_J => { self.menu_sel = (self.menu_sel + 1).min(2); }
                    k if k == QK_RETURN || k == QK_ENTER => {
                        match self.menu_sel {
                            0 => { self.screen = Screen::Connect; }
                            1 => { // Singleplayer
                                self.sp_pending = true;
                                // Load classes from config for char creation
                                // (will be populated by app.rs when starting SP)
                                self.char_name.clear();
                                self.char_class_idx  = 0;
                                self.char_symbol_idx = 0;
                                self.char_color_idx  = 0;
                                self.char_cursor     = 0;
                                self.screen = Screen::CharCreate;
                                return Cmd { kind: CmdKind::StartSP, json: "load_classes".to_string(), ..none };
                            }
                            _ => { return Cmd { kind: CmdKind::Quit, ..none }; }
                        }
                    }
                    k if k == QK_Q || k == QK_ESCAPE => {
                        return Cmd { kind: CmdKind::Quit, ..none };
                    }
                    _ => {}
                }
                none
            }

            // ─── Connect ──────────────────────────────────────────────────────
            Screen::Connect => {
                let field = if self.conn_cursor == 0 { &mut self.conn_host } else { &mut self.conn_port };
                match qt_key {
                    k if k == QK_TAB || k == QK_DOWN => { self.conn_cursor ^= 1; }
                    k if k == QK_UP  => { self.conn_cursor ^= 1; }
                    k if k == QK_BACKSPACE => { field.pop(); }
                    k if k == QK_ESCAPE => { self.screen = Screen::MainMenu; }
                    k if k == QK_RETURN || k == QK_ENTER => {
                        let host = self.conn_host.clone();
                        let port = self.conn_port.parse::<u16>().unwrap_or(7777);
                        self.screen = Screen::Login;
                        return Cmd { kind: CmdKind::Connect, host, port, json: String::new() };
                    }
                    _ => { if let Some(ch) = text.chars().next() {
                        if ch.is_ascii_graphic() || ch == '.' { field.push(ch); }
                    }}
                }
                none
            }

            // ─── Login ────────────────────────────────────────────────────────
            Screen::Login => {
                let field = if self.login_cursor == 0
                    { &mut self.login_user } else { &mut self.login_pass };
                match qt_key {
                    k if k == QK_TAB || k == QK_DOWN => { self.login_cursor ^= 1; }
                    k if k == QK_UP  => { self.login_cursor ^= 1; }
                    k if k == QK_BACKSPACE => { field.pop(); }
                    k if k == QK_ESCAPE => { self.screen = Screen::Connect; }
                    k if k == QK_F1 => {
                        self.login_mode = if self.login_mode == LoginMode::Login
                            { LoginMode::Register } else { LoginMode::Login };
                    }
                    k if k == QK_RETURN || k == QK_ENTER => {
                        let json = match self.login_mode {
                            LoginMode::Login    => mk_login(&self.login_user, &self.login_pass),
                            LoginMode::Register => mk_register(&self.login_user, &self.login_pass),
                        };
                        self.login_pass.clear();
                        return Cmd { kind: CmdKind::SendMsg, json, ..none };
                    }
                    _ => { if let Some(ch) = text.chars().next() {
                        if ch.is_ascii_graphic() { field.push(ch); }
                    }}
                }
                none
            }

            // ─── CharCreate (MP and SP) ───────────────────────────────────────
            Screen::CharCreate => {
                let n_cls = self.avail_classes.len();
                match qt_key {
                    k if k == QK_TAB || k == QK_DOWN => { self.char_cursor = (self.char_cursor+1)%4; }
                    k if k == QK_UP  => { self.char_cursor = (self.char_cursor+3)%4; }
                    k if k == QK_LEFT => {
                        match self.char_cursor {
                            1 if n_cls>0 => { self.char_class_idx  = (self.char_class_idx  + n_cls-1) % n_cls; }
                            2 => { self.char_symbol_idx = (self.char_symbol_idx + SYMBOLS.len()-1) % SYMBOLS.len(); }
                            3 => { self.char_color_idx  = (self.char_color_idx  + COLORS.len()-1)  % COLORS.len(); }
                            _ => {}
                        }
                    }
                    k if k == QK_RIGHT => {
                        match self.char_cursor {
                            1 if n_cls>0 => { self.char_class_idx  = (self.char_class_idx  + 1) % n_cls; }
                            2 => { self.char_symbol_idx = (self.char_symbol_idx + 1) % SYMBOLS.len(); }
                            3 => { self.char_color_idx  = (self.char_color_idx  + 1) % COLORS.len(); }
                            _ => {}
                        }
                    }
                    k if k == QK_BACKSPACE => {
                        if self.char_cursor == 0 { self.char_name.pop(); }
                    }
                    k if k == QK_ESCAPE => {
                        self.sp_pending = false;
                        self.screen = Screen::MainMenu;
                    }
                    k if k == QK_RETURN || k == QK_ENTER => {
                        if !self.char_name.is_empty() && n_cls > 0 {
                            let cls    = &self.avail_classes[self.char_class_idx];
                            let symbol = SYMBOLS[self.char_symbol_idx];
                            let color  = COLORS[self.char_color_idx];
                            if self.sp_pending {
                                // Start singleplayer game
                                let json = serde_json::json!({
                                    "name":     self.char_name,
                                    "class_id": cls.id,
                                    "symbol":   symbol,
                                    "color":    color,
                                }).to_string();
                                return Cmd { kind: CmdKind::StartSP, json, ..none };
                            } else {
                                let json = mk_create_char(&self.char_name, &cls.id, symbol, color);
                                return Cmd { kind: CmdKind::SendMsg, json, ..none };
                            }
                        }
                    }
                    _ => {
                        if self.char_cursor == 0 && self.char_name.len() < 16 {
                            if let Some(ch) = text.chars().next() {
                                if ch.is_alphanumeric() { self.char_name.push(ch); }
                            }
                        }
                    }
                }
                none
            }

            // ─── Playing ──────────────────────────────────────────────────────
            Screen::Playing => {
                // Chat mode
                if self.chat_active {
                    match qt_key {
                        k if k == QK_RETURN || k == QK_ENTER => {
                            if !self.chat_buf.is_empty() {
                                let json = mk_chat(&self.chat_buf);
                                self.chat_buf.clear();
                                self.chat_active = false;
                                return Cmd { kind: CmdKind::SendMsg, json, ..none };
                            }
                            self.chat_active = false;
                        }
                        k if k == QK_ESCAPE    => { self.chat_buf.clear(); self.chat_active = false; }
                        k if k == QK_BACKSPACE => { self.chat_buf.pop(); }
                        _ => {
                            if let Some(ch) = text.chars().next() {
                                if !ch.is_control() { self.chat_buf.push(ch); }
                            }
                        }
                    }
                    return none;
                }

                // Inventory panel
                if self.inv_open {
                    let inv_len = self.my_player().map_or(0, |p| p.inventory.len());
                    match qt_key {
                        k if k == QK_UP || k == QK_K => {
                            if inv_len > 0 { self.inv_sel = (self.inv_sel + inv_len - 1) % inv_len; }
                        }
                        k if k == QK_DOWN || k == QK_J => {
                            if inv_len > 0 { self.inv_sel = (self.inv_sel + 1) % inv_len; }
                        }
                        k if k == QK_E || k == QK_RETURN => {
                            if let Some(id) = self.my_player()
                                .and_then(|p| p.inventory.get(self.inv_sel)).map(|it| it.id) {
                                return Cmd { kind: CmdKind::SendMsg, json: mk_equip(id), ..none };
                            }
                        }
                        k if k == QK_U => {
                            if let Some(id) = self.my_player()
                                .and_then(|p| p.inventory.get(self.inv_sel)).map(|it| it.id) {
                                return Cmd { kind: CmdKind::SendMsg, json: mk_use_item(id), ..none };
                            }
                        }
                        k if k == QK_D => {
                            if let Some(id) = self.my_player()
                                .and_then(|p| p.inventory.get(self.inv_sel)).map(|it| it.id) {
                                self.inv_open = false;
                                return Cmd { kind: CmdKind::SendMsg, json: mk_drop_item(id), ..none };
                            }
                        }
                        k if k == QK_I || k == QK_ESCAPE => { self.inv_open = false; }
                        _ => {}
                    }
                    return none;
                }

                // Normal gameplay
                // Global non-movement keys (same in all camera modes)
                match qt_key {
                    k if k == QK_I => { self.inv_open = !self.inv_open; self.inv_sel = 0; return none; }
                    k if k == QK_C => { self.equip_open = !self.equip_open; return none; }
                    k if k == QK_T => { self.chat_active = true; return none; }
                    k if k == QK_Z => {
                        return Cmd { kind: CmdKind::SendMsg,
                                     json: r#"{"t":"ZoneTravel"}"#.to_string(), ..none };
                    }
                    k if k == QK_ESCAPE => {
                        if self.equip_open { self.equip_open = false; }
                        else if self.inv_open { self.inv_open = false; }
                        else { return Cmd { kind: CmdKind::Disconnect, ..none }; }
                        return none;
                    }
                    _ => {}
                }

                // Movement — first-person: WASD relative to facing, arrows=turn
                //           other modes:   WASD/arrows/hjkl in world axes
                let json: Option<String> = if self.camera_mode == 2 {
                    let (fdx, fdy) = (self.player_face_dx, self.player_face_dy);
                    let (slx, sly) = (fdy, -fdx); // strafe-left = rotate CCW
                    match qt_key {
                        k if k == QK_W || k == QK_UP   || k == QK_K => Some(mk_move(fdx,  fdy)),
                        k if k == QK_S || k == QK_DOWN  || k == QK_J => Some(mk_move(-fdx, -fdy)),
                        k if k == QK_A => Some(mk_move(slx, sly)),
                        k if k == QK_D => Some(mk_move(-slx, -sly)),
                        // Turn left (CCW)
                        k if k == QK_LEFT  || k == QK_H => {
                            self.player_face_dx = fdy; self.player_face_dy = -fdx; None
                        }
                        // Turn right (CW)
                        k if k == QK_RIGHT || k == QK_L => {
                            self.player_face_dx = -fdy; self.player_face_dy = fdx; None
                        }
                        k if k == QK_E => Some(mk_pickup()),
                        _ => None,
                    }
                } else {
                    match qt_key {
                        k if k == QK_UP    || k == QK_K || k == QK_W => {
                            self.player_face_dx = 0; self.player_face_dy = -1;
                            Some(mk_move(0, -1))
                        }
                        k if k == QK_DOWN  || k == QK_J || k == QK_S => {
                            self.player_face_dx = 0; self.player_face_dy = 1;
                            Some(mk_move(0, 1))
                        }
                        k if k == QK_LEFT  || k == QK_H || k == QK_A => {
                            self.player_face_dx = -1; self.player_face_dy = 0;
                            Some(mk_move(-1, 0))
                        }
                        k if k == QK_RIGHT || k == QK_L || k == QK_D => {
                            self.player_face_dx = 1; self.player_face_dy = 0;
                            Some(mk_move(1, 0))
                        }
                        k if k == QK_Y => { self.player_face_dx = -1; self.player_face_dy = -1; Some(mk_move(-1,-1)) }
                        k if k == QK_U => { self.player_face_dx =  1; self.player_face_dy = -1; Some(mk_move( 1,-1)) }
                        k if k == QK_B => { self.player_face_dx = -1; self.player_face_dy =  1; Some(mk_move(-1, 1)) }
                        k if k == QK_N => { self.player_face_dx =  1; self.player_face_dy =  1; Some(mk_move( 1, 1)) }
                        k if k == QK_E => Some(mk_pickup()),
                        _ => None,
                    }
                };
                if let Some(json) = json {
                    return Cmd { kind: CmdKind::SendMsg, json, ..none };
                }
                none
            }
        }
    }
}
