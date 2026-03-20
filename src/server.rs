use anyhow::Result;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};

use crate::{
    config::GameConfig,
    entity::{Equipment, GroundItem, Item, ItemKind, Monster, Player},
    mapgen,
    protocol::{ClientMsg, ServerMsg},
    world::{CombatEvent, GameMap, MAP_H, MAP_W, Pos},
};

// ─── Account ─────────────────────────────────────────────────────────────────

struct Account {
    password: String,
    player:   Option<Player>,
}

// ─── Shared Game State ───────────────────────────────────────────────────────

struct GameState {
    accounts:     HashMap<String, Account>,
    players:      HashMap<u64, Player>,
    sessions:     HashMap<u64, mpsc::UnboundedSender<String>>,
    monsters:     HashMap<u64, Monster>,
    ground_items: Vec<GroundItem>,
    map:          GameMap,
    next_id:      u64,
    cfg:          GameConfig,
    tick:         u64,
}

impl GameState {
    fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn send(&self, player_id: u64, msg: &ServerMsg) {
        if let Some(tx) = self.sessions.get(&player_id) {
            if let Ok(json) = serde_json::to_string(msg) {
                let _ = tx.send(json + "\n");
            }
        }
    }

    fn broadcast(&self, msg: &ServerMsg) {
        if let Ok(json) = serde_json::to_string(msg) {
            let line = json + "\n";
            for tx in self.sessions.values() {
                let _ = tx.send(line.clone());
            }
        }
    }

    fn broadcast_except(&self, except_id: u64, msg: &ServerMsg) {
        if let Ok(json) = serde_json::to_string(msg) {
            let line = json + "\n";
            for (id, tx) in &self.sessions {
                if *id != except_id {
                    let _ = tx.send(line.clone());
                }
            }
        }
    }

    fn make_item(&mut self, template_id: &str) -> Option<Item> {
        let tmpl = self.cfg.items.get(template_id)?.clone();
        let id = self.new_id();
        Some(Item {
            id,
            template_id: tmpl.id,
            name:        tmpl.name,
            symbol:      tmpl.symbol,
            color:       tmpl.color,
            kind:        tmpl.kind,
        })
    }

    fn spawn_player(
        &mut self,
        name: &str,
        class_id: &str,
        symbol: char,
        color: String,
    ) -> Option<Player> {
        let cls = self.cfg.classes.get(class_id)?.clone();
        let id  = self.new_id();
        let pos = Pos::new(MAP_W / 2, MAP_H / 2);

        let mut inventory  = Vec::new();
        let mut equipment  = Equipment::default();

        if let Some(item_id) = &cls.start_item.clone() {
            if let Some(item) = self.make_item(item_id) {
                let old = equipment.equip_item(item);
                if let Some(o) = old { inventory.push(o); }
            }
        }
        // 3 starter potions
        for _ in 0..3 {
            if let Some(p) = self.make_item("hp_potion") {
                inventory.push(p);
            }
        }

        Some(Player {
            id, name: name.to_string(), class_id: class_id.to_string(),
            symbol, color, pos,
            level: 1, xp: 0, xp_next: Player::xp_for_level(2),
            stats: cls.make_stats(), equipment, inventory,
            stat_points: 0,
        })
    }

    fn spawn_monster_from_template(&mut self, template_id: &str, x: i32, y: i32) -> Option<Monster> {
        let tmpl  = self.cfg.monsters.get(template_id)?.clone();
        let id    = self.new_id();
        let pos   = Pos::new(x, y);
        let stats = tmpl.make_stats();   // compute before any fields are moved
        Some(Monster {
            id, template_id: tmpl.id, name: tmpl.name,
            symbol: tmpl.symbol, color: tmpl.color,
            pos, home: pos, level: tmpl.level,
            stats,
            xp_reward: tmpl.xp_reward,
            loot_table: tmpl.loot_table, target: None,
        })
    }

    fn initial_monster_spawn(&mut self) {
        let mut rng  = rand::thread_rng();
        let positions = mapgen::monster_spawn_positions(&self.map);
        if positions.is_empty() { return; }

        let templates: Vec<String> = self.cfg.monsters.keys().cloned().collect();
        let count = self.cfg.world.monster_count;

        for _ in 0..count {
            let tmpl = templates[rng.gen_range(0..templates.len())].clone();
            let (x, y) = positions[rng.gen_range(0..positions.len())];
            if let Some(m) = self.spawn_monster_from_template(&tmpl, x, y) {
                self.monsters.insert(m.id, m);
            }
        }
    }

    /// Handle player levelling up (modifies player in-place, returns true if levelled).
    fn try_level_up(&mut self, player_id: u64) -> bool {
        let (levelled, new_level, sp) = {
            let p = match self.players.get_mut(&player_id) { Some(p) => p, None => return false };
            if p.xp < p.xp_next { return false; }
            p.xp     -= p.xp_next;
            p.level  += 1;
            p.xp_next = Player::xp_for_level(p.level + 1);
            p.stat_points += 3;
            p.stats.str += 1;
            p.stats.vit += 1;
            (true, p.level, p.stat_points)
        };

        if levelled {
            let cls_id = self.players.get(&player_id).map(|p| p.class_id.clone()).unwrap_or_default();
            if let Some(cls) = self.cfg.classes.get(&cls_id).cloned() {
                if let Some(p) = self.players.get_mut(&player_id) {
                    p.stats.max_hp += cls.hp_per_level;
                    p.stats.hp      = p.stats.max_hp;
                    p.stats.max_mp += cls.mp_per_level;
                    p.stats.mp      = p.stats.max_mp;
                }
            }
            self.send(player_id, &ServerMsg::LevelUp { level: new_level, stat_points: sp });
            self.send(player_id, &ServerMsg::System(
                format!("*** LEVEL UP! You are now level {}! +3 stat points ***", new_level)
            ));
        }
        levelled
    }
}

// ─── Server entry point ───────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    let cfg  = GameConfig::load()?;
    let addr = cfg.addr();

    println!("VoidQuest Server v0.1");
    println!("Loading config…  classes={}, monsters={}, items={}",
        cfg.classes.len(), cfg.monsters.len(), cfg.items.len());

    let mut rng = rand::thread_rng();
    let map = mapgen::generate(&mut rng);

    let mut initial_state = GameState {
        accounts: HashMap::new(),
        players:  HashMap::new(),
        sessions: HashMap::new(),
        monsters: HashMap::new(),
        ground_items: Vec::new(),
        map, next_id: 0, cfg, tick: 0,
    };
    initial_state.initial_monster_spawn();
    println!("World generated. {} monsters spawned.", initial_state.monsters.len());

    let state: Arc<Mutex<GameState>> = Arc::new(Mutex::new(initial_state));

    // ── game tick task ──────────────────────────────────────────────────────
    let tick_state = state.clone();
    tokio::spawn(async move {
        let tick_ms = {
            let gs = tick_state.lock().await;
            gs.cfg.world.tick_ms
        };
        let mut ticker = interval(Duration::from_millis(tick_ms));
        loop {
            ticker.tick().await;
            game_tick(&tick_state).await;
        }
    });

    // ── TCP listener ─────────────────────────────────────────────────────────
    let listener: TcpListener = TcpListener::bind(&addr).await?;
    println!("Listening on {} — ready!", addr);
    println!("Run:  cargo run --bin vq-client  to connect.");

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("[server] new connection from {}", peer);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, state).await {
                eprintln!("[server] client {} disconnected: {}", peer, e);
            }
        });
    }
}

// ─── Per-client handler ───────────────────────────────────────────────────────

async fn handle_client(stream: TcpStream, state: Arc<Mutex<GameState>>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // channel: game state → this client's TCP socket
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // writer task: drain the channel and write to TCP
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.write_all(msg.as_bytes()).await.is_err() { break; }
        }
    });

    let mut auth_username: Option<String> = None;
    let mut player_id:     Option<u64>    = None;

    macro_rules! send_raw {
        ($msg:expr) => {{
            if let Ok(json) = serde_json::to_string($msg) {
                let _ = tx.send(json + "\n");
            }
        }};
    }

    while let Ok(Some(line)) = lines.next_line().await {
        let msg: ClientMsg = match serde_json::from_str(&line) {
            Ok(m)  => m,
            Err(e) => {
                send_raw!(&ServerMsg::Err { msg: format!("parse error: {}", e) });
                continue;
            }
        };

        match msg {
            // ── Registration ────────────────────────────────────────────────
            ClientMsg::Register { username, password } => {
                let mut gs = state.lock().await;
                if gs.accounts.contains_key(&username) {
                    send_raw!(&ServerMsg::Err { msg: "Username already taken.".into() });
                } else {
                    gs.accounts.insert(username.clone(), Account { password, player: None });
                    auth_username = Some(username);
                    send_raw!(&ServerMsg::RegisterOk);
                }
            }

            // ── Login ───────────────────────────────────────────────────────
            ClientMsg::Login { username, password } => {
                let mut gs = state.lock().await;
                let ok = gs.accounts.get(&username)
                    .map_or(false, |a| a.password == password);
                if ok {
                    auth_username = Some(username.clone());
                    let has_char = gs.accounts[&username].player.is_some();
                    if has_char {
                        // restore player
                        let p = gs.accounts[&username].player.clone().unwrap();
                        let pid = p.id;
                        player_id = Some(pid);
                        gs.players.insert(pid, p.clone());
                        gs.sessions.insert(pid, tx.clone());

                        let map      = gs.map.clone();
                        let players  = gs.players.values().cloned().collect();
                        let monsters = gs.monsters.values().cloned().collect();
                        let items    = gs.ground_items.clone();

                        send_raw!(&ServerMsg::LoginOk);
                        send_raw!(&ServerMsg::WorldInit { player_id: pid, map, players, monsters, items });
                        gs.broadcast_except(pid, &ServerMsg::PlayerUpdate(p));
                    } else {
                        let classes = gs.cfg.classes.values().cloned().collect();
                        send_raw!(&ServerMsg::LoginOk);
                        send_raw!(&ServerMsg::NeedChar { classes });
                    }
                } else {
                    send_raw!(&ServerMsg::Err { msg: "Invalid username or password.".into() });
                }
            }

            // ── Character creation ──────────────────────────────────────────
            ClientMsg::CreateChar { name, class_id, symbol, color } => {
                let username = match &auth_username {
                    Some(u) => u.clone(),
                    None => {
                        send_raw!(&ServerMsg::Err { msg: "Not logged in.".into() });
                        continue;
                    }
                };
                let mut gs = state.lock().await;
                if !gs.cfg.classes.contains_key(&class_id) {
                    send_raw!(&ServerMsg::Err { msg: format!("Unknown class: {}", class_id) });
                    continue;
                }
                // Check name uniqueness
                let name_taken = gs.players.values().any(|p| p.name == name)
                    || gs.accounts.values().any(|a| a.player.as_ref().map_or(false, |p| p.name == name));
                if name_taken {
                    send_raw!(&ServerMsg::Err { msg: "Character name already in use.".into() });
                    continue;
                }

                if let Some(player) = gs.spawn_player(&name, &class_id, symbol, color) {
                    let pid = player.id;
                    player_id = Some(pid);
                    gs.accounts.get_mut(&username).unwrap().player = Some(player.clone());
                    gs.players.insert(pid, player.clone());
                    gs.sessions.insert(pid, tx.clone());

                    let map      = gs.map.clone();
                    let players  = gs.players.values().cloned().collect();
                    let monsters = gs.monsters.values().cloned().collect();
                    let items    = gs.ground_items.clone();

                    send_raw!(&ServerMsg::CharOk);
                    send_raw!(&ServerMsg::WorldInit { player_id: pid, map, players, monsters, items });
                    gs.broadcast_except(pid, &ServerMsg::PlayerUpdate(player));
                    gs.send(pid, &ServerMsg::System("Welcome to VoidQuest! WASD: move  F: attack  U: use item  E: equip  P: pickup  Enter: chat".into()));
                } else {
                    send_raw!(&ServerMsg::Err { msg: "Failed to create character.".into() });
                }
            }

            // ── Move ────────────────────────────────────────────────────────
            ClientMsg::Move { dx, dy } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                let new_pos = {
                    let p = match gs.players.get(&pid) { Some(p) => p, None => continue };
                    if !p.stats.is_alive() { continue; }
                    Pos::new(p.pos.x + dx, p.pos.y + dy)
                };

                let blocked = !gs.map.passable(new_pos.x, new_pos.y)
                    || gs.monsters.values().any(|m| m.pos == new_pos)
                    || gs.players.values().any(|p| p.id != pid && p.pos == new_pos);

                if blocked { continue; }

                if let Some(p) = gs.players.get_mut(&pid) {
                    p.pos = new_pos;
                }

                // Check if player stepped on a ground item
                let item_here = gs.ground_items.iter().position(|gi| gi.pos == new_pos);
                if let Some(idx) = item_here {
                    let gi = gs.ground_items[idx].item.clone();
                    gs.send(pid, &ServerMsg::System(
                        format!("You see '{}' here. Press P to pick it up.", gi.name)
                    ));
                }

                let p = gs.players.get(&pid).cloned().unwrap();
                gs.broadcast(&ServerMsg::PlayerUpdate(p));
            }

            // ── Attack ──────────────────────────────────────────────────────
            ClientMsg::Attack { target_id } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                // Validate adjacency
                let can_attack = match (gs.players.get(&pid), gs.monsters.get(&target_id)) {
                    (Some(p), Some(m)) => p.stats.is_alive() && p.pos.adjacent(m.pos),
                    _ => false,
                };
                if !can_attack { continue; }

                let mut rng = rand::thread_rng();
                let is_crit = {
                    let p = gs.players.get(&pid).unwrap();
                    // rogues crit more often
                    let crit_chance = if p.class_id == "rogue" { 0.20 } else { 0.10 };
                    rng.gen_bool(crit_chance)
                };

                let raw_dmg = {
                    let p = gs.players.get(&pid).unwrap();
                    if is_crit { p.attack() * 2 } else { p.attack() }
                };
                let (monster_def, monster_name, _monster_hp_before) = {
                    let m = gs.monsters.get(&target_id).unwrap();
                    (m.stats.vit / 3, m.name.clone(), m.stats.hp)
                };
                let actual_dmg = (raw_dmg - monster_def).max(1);
                let player_name = gs.players.get(&pid).map(|p| p.name.clone()).unwrap_or_default();

                // Apply damage
                let killed = {
                    let m = gs.monsters.get_mut(&target_id).unwrap();
                    m.stats.hp -= actual_dmg;
                    !m.stats.is_alive()
                };

                let event = CombatEvent {
                    attacker: player_name.clone(),
                    target:   monster_name.clone(),
                    damage:   actual_dmg,
                    is_crit,
                    killed,
                };
                gs.broadcast(&ServerMsg::Combat(event));

                if killed {
                    let (xp_reward, loot_table, monster_pos) = {
                        let m = gs.monsters.get(&target_id).unwrap();
                        (m.xp_reward, m.loot_table.clone(), m.pos)
                    };
                    gs.monsters.remove(&target_id);
                    gs.broadcast(&ServerMsg::MonsterDied { id: target_id, xp: xp_reward });

                    // Drop loot
                    for item_id in &loot_table {
                        if rng.gen_bool(0.35) {
                            if let Some(item) = gs.make_item(item_id) {
                                let gi = GroundItem { item, pos: monster_pos };
                                gs.broadcast(&ServerMsg::ItemDropped(gi.clone()));
                                gs.ground_items.push(gi);
                            }
                        }
                    }

                    // Award XP
                    if let Some(p) = gs.players.get_mut(&pid) {
                        p.xp += xp_reward;
                    }
                    gs.try_level_up(pid);

                    let pu = gs.players.get(&pid).cloned().map(ServerMsg::PlayerUpdate);
                    if let Some(msg) = pu { gs.broadcast(&msg); }

                    // Respawn monster after delay
                    let state2 = state.clone();
                    let tmpl_id = gs.monsters.get(&target_id)
                        .map(|m| m.template_id.clone())
                        .unwrap_or_else(|| {
                            // pick a random template
                            let templates: Vec<String> = gs.cfg.monsters.keys().cloned().collect();
                            templates[rng.gen_range(0..templates.len())].clone()
                        });
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        let mut gs = state2.lock().await;
                        let positions = mapgen::monster_spawn_positions(&gs.map);
                        if positions.is_empty() { return; }
                        let mut rng = rand::thread_rng();
                        let (x, y) = positions[rng.gen_range(0..positions.len())];
                        if let Some(m) = gs.spawn_monster_from_template(&tmpl_id, x, y) {
                            gs.broadcast(&ServerMsg::MonsterUpdate(m.clone()));
                            gs.monsters.insert(m.id, m);
                        }
                    });
                } else {
                    let mu = gs.monsters.get(&target_id).cloned().map(ServerMsg::MonsterUpdate);
                    if let Some(msg) = mu { gs.broadcast(&msg); }
                }
                // Send log of the hit
                if is_crit {
                    gs.send(pid, &ServerMsg::System(format!("CRITICAL HIT on {}!", monster_name)));
                }
            }

            // ── Use Item (potion) ────────────────────────────────────────────
            ClientMsg::UseItem { item_id } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                if let Some(player) = gs.players.get_mut(&pid) {
                    if let Some(idx) = player.inventory.iter().position(|i| i.id == item_id) {
                        let item = player.inventory[idx].clone();
                        if let ItemKind::Potion { hp, mp } = item.kind {
                            player.inventory.remove(idx);
                            player.stats.hp = (player.stats.hp + hp).min(player.stats.max_hp);
                            player.stats.mp = (player.stats.mp + mp).min(player.stats.max_mp);
                            let msg = ServerMsg::System(
                                format!("Used {}. HP +{} MP +{}", item.name, hp, mp)
                            );
                            let pu = ServerMsg::PlayerUpdate(player.clone());
                            gs.send(pid, &msg);
                            gs.send(pid, &pu);
                        } else {
                            gs.send(pid, &ServerMsg::Err { msg: "That item is not a consumable.".into() });
                        }
                    }
                }
            }

            // ── Equip Item ───────────────────────────────────────────────────
            ClientMsg::Equip { item_id } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                if let Some(player) = gs.players.get_mut(&pid) {
                    if let Some(idx) = player.inventory.iter().position(|i| i.id == item_id) {
                        let item = player.inventory.remove(idx);
                        let slot = Equipment::slot_name(&item.kind).to_string();
                        let old  = player.equipment.equip_item(item.clone());
                        if let Some(unequipped) = old {
                            player.inventory.push(unequipped);
                        }
                        let msg = ServerMsg::System(format!("Equipped {} in {} slot.", item.name, slot));
                        let pu  = ServerMsg::PlayerUpdate(player.clone());
                        gs.send(pid, &msg);
                        gs.send(pid, &pu);
                    }
                }
            }

            // ── Pickup ground item ───────────────────────────────────────────
            ClientMsg::Pickup => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                let player_pos = gs.players.get(&pid).map(|p| p.pos);
                if let Some(pos) = player_pos {
                    if let Some(idx) = gs.ground_items.iter().position(|gi| gi.pos == pos) {
                        let gi = gs.ground_items.remove(idx);
                        let item_id = gi.item.id;
                        let item_name = gi.item.name.clone();
                        let player_name = gs.players.get(&pid).map(|p| p.name.clone()).unwrap_or_default();

                        if let Some(p) = gs.players.get_mut(&pid) {
                            p.inventory.push(gi.item);
                        }
                        gs.broadcast(&ServerMsg::ItemPickedUp { item_id, by: player_name });
                        gs.send(pid, &ServerMsg::System(format!("Picked up {}.", item_name)));
                        let pu = gs.players.get(&pid).cloned().map(ServerMsg::PlayerUpdate);
                        if let Some(msg) = pu { gs.send(pid, &msg); }
                    } else {
                        gs.send(pid, &ServerMsg::Err { msg: "Nothing to pick up here.".into() });
                    }
                }
            }

            // ── Drop Item ────────────────────────────────────────────────────
            ClientMsg::DropItem { item_id } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                let player_pos = gs.players.get(&pid).map(|p| p.pos);
                if let Some(pos) = player_pos {
                    if let Some(player) = gs.players.get_mut(&pid) {
                        if let Some(idx) = player.inventory.iter().position(|i| i.id == item_id) {
                            let item = player.inventory.remove(idx);
                            let name = item.name.clone();
                            let gi   = GroundItem { item, pos };
                            gs.broadcast(&ServerMsg::ItemDropped(gi.clone()));
                            gs.ground_items.push(gi);
                            gs.send(pid, &ServerMsg::System(format!("Dropped {}.", name)));
                            let pu = gs.players.get(&pid).cloned().map(ServerMsg::PlayerUpdate);
                            if let Some(msg) = pu { gs.send(pid, &msg); }
                        }
                    }
                }
            }

            // ── Chat ─────────────────────────────────────────────────────────
            ClientMsg::Chat { msg } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let gs = state.lock().await;
                if let Some(p) = gs.players.get(&pid) {
                    let from = p.name.clone();
                    gs.broadcast(&ServerMsg::Chat { from, msg });
                }
            }

            ClientMsg::Ping => {
                send_raw!(&ServerMsg::Pong);
            }

            ClientMsg::Quit => {
                break;
            }
        }
    }

    // ── Clean up disconnected player ─────────────────────────────────────────
    if let Some(pid) = player_id {
        let mut gs = state.lock().await;
        // Save player state back to account
        if let Some(p) = gs.players.remove(&pid) {
            let name = p.name.clone();
            // find account
            for acc in gs.accounts.values_mut() {
                if acc.player.as_ref().map_or(false, |ap| ap.id == pid) {
                    acc.player = Some(p);
                    break;
                }
            }
            gs.sessions.remove(&pid);
            gs.broadcast(&ServerMsg::PlayerLeft(pid));
            println!("[server] {} disconnected", name);
        }
    }

    Ok(())
}

// ─── Game Tick (monster AI, HP/MP regen) ─────────────────────────────────────

enum MonsterAI {
    Attack(u64, i32, String),   // target_pid, str_val, name
    MoveTo(Pos),
    Idle,
}

async fn game_tick(state: &Arc<Mutex<GameState>>) {
    let mut gs = state.lock().await;
    gs.tick += 1;
    let tick = gs.tick;

    // ── monster AI ───────────────────────────────────────────────────────────
    let monster_ids: Vec<u64> = gs.monsters.keys().cloned().collect();
    let player_snap: Vec<(u64, Pos, bool)> = gs.players.values()
        .map(|p| (p.id, p.pos, p.stats.is_alive()))
        .collect();

    let mut player_damage:  Vec<(u64, i32, String)> = Vec::new();
    let mut moved_monsters: Vec<u64>                = Vec::new();

    for mid in &monster_ids {
        // Phase 1: compute AI decision with immutable borrow (dropped at end of block)
        let action: MonsterAI = {
            let m = match gs.monsters.get(mid) { Some(m) => m, None => continue };
            if !m.stats.is_alive() { continue; }

            let nearest = player_snap.iter()
                .filter(|(_, _, alive)| *alive)
                .min_by_key(|(_, pos, _)| m.pos.dist_sq(*pos));

            if let Some((pid, ppos, _)) = nearest {
                let dsq = m.pos.dist_sq(*ppos);
                if dsq <= 2 {
                    MonsterAI::Attack(*pid, m.stats.str, m.name.clone())
                } else if dsq < 100 {
                    MonsterAI::MoveTo(m.pos.step_toward(*ppos))
                } else if dsq > 144 && m.pos != m.home {
                    MonsterAI::MoveTo(m.pos.step_toward(m.home))
                } else {
                    MonsterAI::Idle
                }
            } else {
                MonsterAI::Idle
            }
        }; // immutable borrow of m ends here

        // Phase 2: apply decision (can now freely borrow gs)
        match action {
            MonsterAI::Attack(pid, str_val, mname) => {
                let dmg = str_val.saturating_sub(2).max(1);
                player_damage.push((pid, dmg, mname));
                if let Some(m) = gs.monsters.get_mut(mid) {
                    m.target = Some(pid);
                }
            }
            MonsterAI::MoveTo(new_pos) => {
                let occupied = gs.players.values().any(|p| p.pos == new_pos)
                    || gs.monsters.values().filter(|x| x.id != *mid).any(|x| x.pos == new_pos);
                if gs.map.passable(new_pos.x, new_pos.y) && !occupied {
                    if let Some(m) = gs.monsters.get_mut(mid) {
                        m.pos = new_pos;
                        moved_monsters.push(*mid);
                    }
                }
            }
            MonsterAI::Idle => {}
        }
    }

    // send monster movement updates
    for mid in moved_monsters {
        if let Some(m) = gs.monsters.get(&mid).cloned() {
            gs.broadcast(&ServerMsg::MonsterUpdate(m));
        }
    }

    // apply player damage from monsters
    let mut player_updates: Vec<u64> = Vec::new();
    for (pid, dmg, mname) in player_damage {
        // Phase 1: compute damage result (mutable borrow ends at closing })
        let result = {
            let p = match gs.players.get_mut(&pid) { Some(p) => p, None => continue };
            let actual = (dmg - p.defense() / 2).max(1);
            p.stats.hp -= actual;
            let killed = !p.stats.is_alive();
            let event  = CombatEvent {
                attacker: mname, target: p.name.clone(),
                damage: actual, is_crit: false, killed,
            };
            if killed {
                p.stats.hp = p.stats.max_hp / 4;
                p.pos      = Pos::new(MAP_W / 2, MAP_H / 2);
            }
            (event, killed)
        }; // mutable borrow of p ends here

        // Phase 2: broadcast (no conflicting borrows)
        gs.broadcast(&ServerMsg::Combat(result.0));
        if result.1 {
            gs.send(pid, &ServerMsg::System("You were slain! Respawned in town.".into()));
        }
        player_updates.push(pid);
    }
    for pid in player_updates {
        if let Some(p) = gs.players.get(&pid).cloned() {
            gs.broadcast(&ServerMsg::PlayerUpdate(p));
        }
    }

    // ── HP/MP regen every 3 ticks ─────────────────────────────────────────────
    if tick % 3 == 0 {
        let pids: Vec<u64> = gs.players.keys().cloned().collect();
        for pid in pids {
            let pu = {
                let p = match gs.players.get_mut(&pid) { Some(p) => p, None => continue };
                if !p.stats.is_alive() { continue; }
                p.stats.hp = (p.stats.hp + 2 + p.stats.vit / 5).min(p.stats.max_hp);
                p.stats.mp = (p.stats.mp + 3 + p.stats.int / 5).min(p.stats.max_mp);
                ServerMsg::PlayerUpdate(p.clone())
            };
            gs.send(pid, &pu);
        }
    }
}
