use anyhow::Result;
use rand::Rng;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;

use crate::{
    entity::{Equipment, GroundItem, ItemKind},
    protocol::{ClientMsg, ServerMsg},
    world::Pos,
    mapgen,
};
use super::GameState;

pub async fn handle_client(stream: TcpStream, state: Arc<Mutex<GameState>>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

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
            ClientMsg::Register { username, password } => {
                let mut gs = state.lock().await;
                if gs.accounts.contains_key(&username) {
                    send_raw!(&ServerMsg::Err { msg: "Username already taken.".into() });
                } else {
                    gs.accounts.insert(username.clone(), super::Account { password, player: None });
                    auth_username = Some(username);
                    send_raw!(&ServerMsg::RegisterOk);
                }
            }

            ClientMsg::Login { username, password } => {
                let mut gs = state.lock().await;
                let ok = gs.accounts.get(&username)
                    .map_or(false, |a| a.password == password);
                if ok {
                    auth_username = Some(username.clone());
                    let has_char = gs.accounts[&username].player.is_some();
                    if has_char {
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

            ClientMsg::Attack { target_id } => {
                let pid = match player_id { Some(p) => p, None => continue };
                let mut gs = state.lock().await;

                let can_attack = match (gs.players.get(&pid), gs.monsters.get(&target_id)) {
                    (Some(p), Some(m)) => p.stats.is_alive() && p.pos.adjacent(m.pos),
                    _ => false,
                };
                if !can_attack { continue; }

                let mut rng = rand::thread_rng();
                let is_crit = {
                    let p = gs.players.get(&pid).unwrap();
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

                let killed = {
                    let m = gs.monsters.get_mut(&target_id).unwrap();
                    m.stats.hp -= actual_dmg;
                    !m.stats.is_alive()
                };

                use crate::world::CombatEvent;
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

                    for item_id in &loot_table {
                        if rng.gen_bool(0.35) {
                            if let Some(item) = gs.make_item(item_id) {
                                let gi = GroundItem { item, pos: monster_pos };
                                gs.broadcast(&ServerMsg::ItemDropped(gi.clone()));
                                gs.ground_items.push(gi);
                            }
                        }
                    }

                    if let Some(p) = gs.players.get_mut(&pid) {
                        p.xp += xp_reward;
                    }
                    gs.try_level_up(pid);

                    let pu = gs.players.get(&pid).cloned().map(ServerMsg::PlayerUpdate);
                    if let Some(msg) = pu { gs.broadcast(&msg); }

                    let state2 = state.clone();
                    let tmpl_id = gs.monsters.get(&target_id)
                        .map(|m| m.template_id.clone())
                        .unwrap_or_else(|| {
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
                if is_crit {
                    gs.send(pid, &ServerMsg::System(format!("CRITICAL HIT on {}!", monster_name)));
                }
            }

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

    // Clean up disconnected player
    if let Some(pid) = player_id {
        let mut gs = state.lock().await;
        if let Some(p) = gs.players.remove(&pid) {
            let name = p.name.clone();
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
