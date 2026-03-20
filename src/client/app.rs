// app.rs — GameApp: opaque Rust object owned by C++ VqWidget3D.
//
// Implements the extern "Rust" bridge API declared in bridge.rs:
//   tick, on_key, on_resize, do_connect, do_send, do_disconnect,
//   get_map_info, get_tile, get_entities, get_hud.
//
// No terminal / TerminalBuffer dependency — all rendering is in C++.

use crate::client::bridge::ffi::{Cmd, CmdKind, EntityInfo, HudData, MapInfo};
use crate::client::net::NetHandle;
use crate::client::state::{GameState, LoginMode, Screen, COLORS, SYMBOLS, ZONE_STYLES};
use crate::config::GameConfig;
use crate::engine::{LocalGame, SpEvent};
use crate::mapgen::MapStyle;
use crate::protocol::ServerMsg;

// ── GameApp ───────────────────────────────────────────────────────────────────

pub struct GameApp {
    state:       GameState,
    view_w:      i32,
    view_h:      i32,
    net:         Option<NetHandle>,
    local_game:  Option<LocalGame>,
    sp_ai_timer: u32,
}

impl GameApp {
    pub fn new() -> Self {
        Self {
            state:       GameState::default(),
            view_w:      24,
            view_h:      16,
            net:         None,
            local_game:  None,
            sp_ai_timer: 0,
        }
    }

    // ── Tick (called ~30 FPS) ─────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.state.anim_tick += 1;

        // Clear timed error overlay
        if !self.state.error_msg.is_empty()
            && self.state.anim_tick >= self.state.error_clear_tick
        {
            self.state.error_msg.clear();
        }

        // Singleplayer AI tick (~18 fps)
        if self.local_game.is_some() {
            self.sp_ai_timer += 1;
            if self.sp_ai_timer >= 2 {
                self.sp_ai_timer = 0;
                let events = self.local_game.as_mut().unwrap().tick();
                self.apply_sp_events(events);
            }
        }

        // Multiplayer: drain network messages
        if let Some(net) = &self.net {
            let msgs: Vec<String> = net.rx.try_iter().collect();
            for raw in msgs {
                match serde_json::from_str::<ServerMsg>(&raw) {
                    Ok(msg) => self.state.handle_message(msg),
                    Err(e)  => eprintln!("[client] parse error: {e}"),
                }
            }
        }
    }

    // ── Viewport resize ───────────────────────────────────────────────────────

    pub fn on_resize(&mut self, view_w: i32, view_h: i32) {
        self.view_w = view_w.max(8);
        self.view_h = view_h.max(6);
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn on_key(&mut self, qt_key: i32, qt_mods: u32, text: &str) -> Cmd {
        let cmd = self.state.handle_key(qt_key, qt_mods, text);

        match cmd.kind {
            CmdKind::StartSP => {
                self.handle_start_sp(&cmd.json);
                return Cmd { kind: CmdKind::None, host: String::new(), port: 0, json: String::new() };
            }
            CmdKind::SendMsg => {
                if self.local_game.is_some() {
                    // Singleplayer: intercept and dispatch locally
                    self.dispatch_sp_action(&cmd.json.clone());
                } else {
                    self.do_send(&cmd.json.clone());
                }
            }
            CmdKind::Connect    => { self.do_connect(&cmd.host.clone(), cmd.port); }
            CmdKind::Disconnect => {
                self.do_disconnect();
                // Reset to main menu
                self.state.screen = Screen::MainMenu;
            }
            CmdKind::Quit       => {
                self.do_disconnect();
                return cmd;
            }
            _ => {}
        }

        Cmd { kind: CmdKind::None, host: String::new(), port: 0, json: String::new() }
    }

    // ── Singleplayer: start / class loading ───────────────────────────────────

    fn handle_start_sp(&mut self, json: &str) {
        if json == "load_classes" {
            // Just loading classes for the char creation screen
            if let Ok(cfg) = GameConfig::load() {
                let mut classes: Vec<_> = cfg.classes.values().cloned().collect();
                classes.sort_by(|a, b| a.id.cmp(&b.id));
                self.state.avail_classes = classes;
                self.state.char_class_idx  = 0;
                self.state.char_cursor     = 0;
                self.state.char_name.clear();
            }
            // Screen::CharCreate already set by state.rs
            return;
        }

        // Full start: json = {name, class_id, symbol, color}
        let v: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[sp] bad StartSP json: {e}");
                return;
            }
        };
        let name     = v["name"].as_str().unwrap_or("Hero").to_string();
        let class_id = v["class_id"].as_str().unwrap_or("warrior").to_string();
        let symbol   = v["symbol"].as_str().unwrap_or("@")
                           .chars().next().unwrap_or('@');
        let color    = v["color"].as_str().unwrap_or("white").to_string();

        match LocalGame::new(&name, &class_id, symbol, color) {
            Some(lg) => {
                // Sync initial state into GameState
                self.state.my_id = lg.player.id;
                self.state.world = lg.map.clone();
                self.state.players.clear();
                self.state.players.insert(lg.player.id, lg.player.clone());
                self.state.monsters.clear();
                for (id, m) in &lg.monsters { self.state.monsters.insert(*id, m.clone()); }
                self.state.ground_items = lg.ground_items.clone();
                self.state.screen = Screen::Playing;
                self.state.sp_pending = false;
                self.state.add_log(format!("Singleplayer started — welcome, {}!", name));
                self.local_game = Some(lg);
                self.sp_ai_timer = 0;
            }
            None => {
                self.state.error_msg = "Failed to start singleplayer game.".to_string();
                self.state.error_clear_tick = self.state.anim_tick + 180;
                self.state.screen = Screen::MainMenu;
            }
        }
    }

    // ── Singleplayer: action dispatch ─────────────────────────────────────────

    fn dispatch_sp_action(&mut self, json: &str) {
        let v: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => return,
        };
        let t = v["t"].as_str().unwrap_or("");
        let events: Vec<SpEvent> = match t {
            "Move" => {
                let dx = v["d"]["dx"].as_i64().unwrap_or(0) as i32;
                let dy = v["d"]["dy"].as_i64().unwrap_or(0) as i32;
                let lg = self.local_game.as_mut().unwrap();
                let evs = lg.move_player(dx, dy);
                if evs.is_empty() {
                    // Bump-attack: if a monster is at the destination, attack it
                    use crate::world::Pos;
                    let dest = Pos::new(lg.player.pos.x + dx, lg.player.pos.y + dy);
                    if let Some(mid) = lg.monsters.values()
                        .find(|m| m.pos == dest && m.stats.is_alive())
                        .map(|m| m.id)
                    {
                        lg.attack(mid)
                    } else {
                        evs
                    }
                } else {
                    evs
                }
            }
            "Pickup" => {
                self.local_game.as_mut().unwrap().pickup()
            }
            "Equip" => {
                let id = v["d"]["item_id"].as_u64().unwrap_or(0);
                self.local_game.as_mut().unwrap().equip_item(id)
            }
            "UseItem" => {
                let id = v["d"]["item_id"].as_u64().unwrap_or(0);
                self.local_game.as_mut().unwrap().use_item(id)
            }
            "DropItem" => {
                let id = v["d"]["item_id"].as_u64().unwrap_or(0);
                self.local_game.as_mut().unwrap().drop_item(id)
            }
            "ZoneTravel" => {
                self.travel_zone();
                return;
            }
            "Chat" => {
                self.state.add_log("(Chat is multiplayer only.)".to_string());
                return;
            }
            _ => return,
        };
        self.apply_sp_events(events);
    }

    fn travel_zone(&mut self) {
        self.state.zone_idx = (self.state.zone_idx + 1) % ZONE_STYLES.len();
        let style = match self.state.zone_idx {
            0 => MapStyle::Wilderness,
            1 => MapStyle::Dungeon,
            2 => MapStyle::Desert,
            3 => MapStyle::Forest,
            4 => MapStyle::Swamp,
            _ => MapStyle::Town,
        };
        let player = self.local_game.as_ref().unwrap().player.clone();
        let seed   = self.state.anim_tick.wrapping_mul(0x9e3779b97f4a7c15);
        match LocalGame::new_zone(player, seed, style) {
            Some(lg) => {
                self.state.world = lg.map.clone();
                self.state.players.clear();
                self.state.players.insert(lg.player.id, lg.player.clone());
                self.state.monsters.clear();
                for (id, m) in &lg.monsters { self.state.monsters.insert(*id, m.clone()); }
                self.state.ground_items = lg.ground_items.clone();
                self.state.add_log(format!("Entered {}.", ZONE_STYLES[self.state.zone_idx]));
                self.local_game = Some(lg);
            }
            None => {
                self.state.add_log("Zone travel failed.".to_string());
            }
        }
    }

    fn apply_sp_events(&mut self, events: Vec<SpEvent>) {
        for ev in events {
            match ev {
                SpEvent::UpdatePlayer(p) => {
                    self.state.my_id = p.id;
                    self.state.players.insert(p.id, p.clone());
                    // Keep local_game player in sync
                    if let Some(lg) = &mut self.local_game { lg.player = p; }
                }
                SpEvent::MonsterUpdate(m) => { self.state.monsters.insert(m.id, m); }
                SpEvent::MonsterDied { id, xp } => {
                    self.state.monsters.remove(&id);
                    if xp > 0 { self.state.add_log(format!("You gain {} XP.", xp)); }
                }
                SpEvent::ItemDropped(gi)        => { self.state.ground_items.push(gi); }
                SpEvent::ItemPickedUp { item_id } => {
                    self.state.ground_items.retain(|gi| gi.item.id != item_id);
                }
                SpEvent::LevelUp { level, stat_points } => {
                    self.state.add_log(format!(
                        "*** LEVEL UP! Level {} (+{} stat points) ***", level, stat_points));
                }
                SpEvent::Log(msg, _color) => { self.state.add_log(msg); }
                SpEvent::Combat(ce) => {
                    self.state.add_log(format!("{} hits {} for {}{}{}",
                        ce.attacker, ce.target, ce.damage,
                        if ce.is_crit { " [CRIT]" } else { "" },
                        if ce.killed  { " — killed!" } else { "" }));
                }
            }
        }
    }

    // ── Network ───────────────────────────────────────────────────────────────

    pub fn do_connect(&mut self, host: &str, port: u16) {
        self.do_disconnect();
        self.local_game = None;
        match NetHandle::connect(host, port) {
            Ok(h) => {
                self.net = Some(h);
                self.state.add_log(format!("Connecting to {}:{}…", host, port));
            }
            Err(e) => {
                self.state.add_log(format!("[NET] Connection failed: {}", e));
                self.state.error_msg        = format!("Cannot connect: {}", e);
                self.state.error_clear_tick = self.state.anim_tick + 180;
                self.state.screen           = Screen::MainMenu;
            }
        }
    }

    pub fn do_send(&mut self, json: &str) {
        if let Some(net) = &self.net { net.send(json); }
    }

    pub fn do_disconnect(&mut self) {
        if self.net.take().is_some() { self.state.add_log("Disconnected."); }
        self.local_game = None;
    }

    // ── 3D scene data ─────────────────────────────────────────────────────────

    pub fn get_map_info(&self) -> MapInfo {
        let (cam_x, cam_y, px, py) = if let Some(p) = self.state.my_player() {
            (p.pos.x - self.view_w / 2, p.pos.y - self.view_h / 2, p.pos.x, p.pos.y)
        } else { (0, 0, 0, 0) };

        MapInfo {
            map_w: self.state.world.width,
            map_h: self.state.world.height,
            cam_x, cam_y,
            view_w: self.view_w, view_h: self.view_h,
            player_x: px, player_y: py,
        }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> u8 {
        self.state.world.get(x, y) as u8
    }

    pub fn get_entities(&self) -> Vec<EntityInfo> {
        let mut out = Vec::new();
        let info  = self.get_map_info();
        let cam_x = info.cam_x;
        let cam_y = info.cam_y;
        let vw    = self.view_w;
        let vh    = self.view_h;

        let visible = |x: i32, y: i32| -> bool {
            let vc = x - cam_x; let vr = y - cam_y;
            vc >= 0 && vc < vw && vr >= 0 && vr < vh
        };

        // Ground items
        for gi in &self.state.ground_items {
            if visible(gi.pos.x, gi.pos.y) {
                let (r, g, b) = named_color(&gi.item.color);
                out.push(EntityInfo { x: gi.pos.x, y: gi.pos.y, kind: 3,
                    r, g, b, anim: self.state.anim_tick as u32 });
            }
        }
        // Other players
        for (&id, pl) in &self.state.players {
            if id == self.state.my_id { continue; }
            if visible(pl.pos.x, pl.pos.y) {
                let (r, g, b) = named_color(&pl.color);
                out.push(EntityInfo { x: pl.pos.x, y: pl.pos.y, kind: 1, r, g, b, anim: 0 });
            }
        }
        // Monsters
        for mob in self.state.monsters.values() {
            if visible(mob.pos.x, mob.pos.y) {
                let (r, g, b) = named_color(&mob.color);
                out.push(EntityInfo { x: mob.pos.x, y: mob.pos.y, kind: 2, r, g, b, anim: 0 });
            }
        }
        // Self (drawn last = on top)
        if let Some(p) = self.state.my_player() {
            if visible(p.pos.x, p.pos.y) {
                let (r, g, b) = named_color(&p.color);
                out.push(EntityInfo { x: p.pos.x, y: p.pos.y, kind: 0,
                    r, g, b, anim: self.state.anim_tick as u32 });
            }
        }
        out
    }

    pub fn get_hud(&self) -> HudData {
        let s = &self.state;

        let screen = match s.screen {
            Screen::MainMenu   => 0u8,
            Screen::Connect    => 1,
            Screen::Login      => 2,
            Screen::CharCreate => 3,
            Screen::Playing    => 4,
        };

        // ── Log (last 20 lines) ───────────────────────────────────────────────
        let log_lines: Vec<&String> = {
            let v: Vec<_> = s.log.iter().rev().take(20).collect();
            v.into_iter().rev().collect()
        };
        let log_json = serde_json::to_string(&log_lines).unwrap_or_default();

        // ── CharCreate class info ─────────────────────────────────────────────
        let (char_class_name, char_class_desc, char_col_r, char_col_g, char_col_b) =
            if let Some(cls) = s.avail_classes.get(s.char_class_idx) {
                let (r, g, b) = named_color(COLORS[s.char_color_idx]);
                (cls.display_name("en").to_owned().to_string(), cls.display_desc("en").to_owned().to_string(), r, g, b)
            } else {
                (String::new(), String::new(), 200u8, 200u8, 200u8)
            };

        // ── Build HudData ─────────────────────────────────────────────────────
        let mut hud = HudData {
            screen,
            anim_tick: s.anim_tick,

            player_name:  String::new(),
            player_class: String::new(),
            player_level: 0,
            hp: 0, max_hp: 0,
            mp: 0, max_mp: 0,
            xp: 0, xp_next: 0,
            stat_str: 0, stat_dex: 0, stat_int: 0, stat_vit: 0,
            atk: 0, def_: 0,
            stat_points: 0,
            world_name: s.world.name.clone(),

            eq_weapon: String::new(), eq_armor: String::new(),
            eq_helmet: String::new(), eq_ring:  String::new(),
            eq_weapon_r: 90, eq_weapon_g: 90, eq_weapon_b: 90,
            eq_armor_r:  90, eq_armor_g:  90, eq_armor_b:  90,
            eq_helmet_r: 90, eq_helmet_g: 90, eq_helmet_b: 90,
            eq_ring_r:   90, eq_ring_g:   90, eq_ring_b:   90,

            chat_active: s.chat_active,
            chat_buf:    s.chat_buf.clone(),
            inv_open:    s.inv_open,
            equip_open:  s.equip_open,
            inv_sel:     s.inv_sel as i32,

            inventory_json: String::new(),
            log_json,
            nearby_json: String::new(),

            camera_mode:    s.camera_mode,
            is_singleplayer: self.local_game.is_some(),
            player_face_dx: s.player_face_dx,
            player_face_dy: s.player_face_dy,
            zone_style:     ZONE_STYLES[s.zone_idx].to_string(),

            menu_sel:    s.menu_sel as i32,
            conn_host:   s.conn_host.clone(),
            conn_port:   s.conn_port.clone(),
            conn_cursor: s.conn_cursor as i32,

            login_user:        s.login_user.clone(),
            login_pass_len:    s.login_pass.len() as i32,
            login_cursor:      s.login_cursor as i32,
            login_is_register: s.login_mode == LoginMode::Register,

            char_name:       s.char_name.clone(),
            char_class_name, char_class_desc,
            char_symbol:     SYMBOLS[s.char_symbol_idx].to_string(),
            char_col_r, char_col_g, char_col_b,
            char_cursor:    s.char_cursor as i32,
            char_n_classes: s.avail_classes.len() as i32,
            char_is_sp:     s.sp_pending,

            error_msg: s.error_msg.clone(),
        };

        // ── Fill player-specific fields when in-game ──────────────────────────
        if let Some(p) = s.my_player() {
            hud.player_name  = p.name.clone();
            hud.player_class = p.class_id.clone();
            hud.player_level = p.level as i32;
            hud.hp     = p.stats.hp;
            hud.max_hp = p.stats.max_hp;
            hud.mp     = p.stats.mp;
            hud.max_mp = p.stats.max_mp;
            hud.xp     = p.xp as i64;
            hud.xp_next = p.xp_next as i64;
            hud.stat_str = p.stats.str;
            hud.stat_dex = p.stats.dex;
            hud.stat_int = p.stats.int;
            hud.stat_vit = p.stats.vit;
            hud.atk        = p.attack();
            hud.def_       = p.defense();
            hud.stat_points = p.stat_points as i32;

            // Equipment
            let eq = &p.equipment;
            let slot_name = |opt: &Option<crate::entity::Item>| -> String {
                opt.as_ref().map_or(String::new(), |i| i.name.clone())
            };
            let slot_col = |opt: &Option<crate::entity::Item>| -> (u8,u8,u8) {
                opt.as_ref().map_or((90,90,90), |i| named_color(&i.color))
            };
            hud.eq_weapon = slot_name(&eq.weapon);
            hud.eq_armor  = slot_name(&eq.armor);
            hud.eq_helmet = slot_name(&eq.helmet);
            hud.eq_ring   = slot_name(&eq.ring);
            let (r,g,b) = slot_col(&eq.weapon);
            hud.eq_weapon_r = r; hud.eq_weapon_g = g; hud.eq_weapon_b = b;
            let (r,g,b) = slot_col(&eq.armor);
            hud.eq_armor_r  = r; hud.eq_armor_g  = g; hud.eq_armor_b  = b;
            let (r,g,b) = slot_col(&eq.helmet);
            hud.eq_helmet_r = r; hud.eq_helmet_g = g; hud.eq_helmet_b = b;
            let (r,g,b) = slot_col(&eq.ring);
            hud.eq_ring_r   = r; hud.eq_ring_g   = g; hud.eq_ring_b   = b;

            hud.inventory_json = serde_json::to_string(&p.inventory).unwrap_or_default();

            // Nearby monsters JSON
            let nearby: Vec<_> = s.monsters.values()
                .filter(|m| (m.pos.x - p.pos.x).abs() <= 12
                         && (m.pos.y - p.pos.y).abs() <= 12)
                .take(6)
                .map(|m| serde_json::json!({
                    "name":   m.name,
                    "symbol": m.symbol.to_string(),
                    "color":  m.color,
                    "hp":     m.stats.hp,
                    "max_hp": m.stats.max_hp,
                }))
                .collect();
            hud.nearby_json = serde_json::to_string(&nearby).unwrap_or_default();
        }

        hud
    }
}

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Map a named game color to RGB bytes.
pub fn named_color(name: &str) -> (u8, u8, u8) {
    match name {
        "white"   => (220, 220, 220),
        "red"     => (220,  60,  60),
        "green"   => ( 60, 200,  60),
        "blue"    => ( 60, 100, 220),
        "yellow"  => (220, 200,  60),
        "cyan"    => ( 60, 200, 200),
        "magenta" => (180,  60, 180),
        "gray"    => (140, 140, 140),
        "orange"  => (220, 140,  40),
        "purple"  => (140,  40, 200),
        _         => (180, 180, 180),
    }
}
