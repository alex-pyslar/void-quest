# VoidQuest — Terminal MMO RPG

Console MMO RPG written in Rust. Runs entirely in the terminal, supports multiple
simultaneous players over TCP, and is fully customisable through JSON/TOML config files.

## Features

| Feature | Details |
|---------|---------|
| **Classes** | Warrior · Mage · Rogue · Paladin · Ranger (customise in `config/classes.json`) |
| **Monsters** | 9 monsters, each with unique stats and loot tables (`config/monsters.json`) |
| **Items** | Weapons, Armour, Helmets, Rings, Potions (`config/items.json`) |
| **Combat** | Real-time tick-based combat; critical hits, XP, level-up, stat bonuses |
| **World** | Procedurally generated 80×50 tile world with town, forests, rivers, roads |
| **Multiplayer** | Any number of clients over TCP (LAN or internet) |
| **Terminal UI** | Map viewport · Character stats · Equipment · Inventory · Combat log · Chat |
| **WSL ready** | Builds and runs natively in WSL 1 and WSL 2 |

## Quick start

### WSL / Linux
```bash
chmod +x build.sh
./build.sh

# Terminal 1
./target/release/vq-server

# Terminal 2 (same machine)
./target/release/vq-client

# Terminal on another machine
./target/release/vq-client 192.168.1.X:7777
```

### Windows (PowerShell / cmd)
```powershell
cargo run --bin vq-server
# new window:
cargo run --bin vq-client
```

## Controls

| Key | Action |
|-----|--------|
| `W A S D` / arrows | Move |
| `Q E Z C` | Move diagonally |
| `F` | Attack nearest adjacent monster |
| `U` | Use selected inventory item (potion) |
| `G` | Equip selected inventory item |
| `P` | Pick up item on ground |
| `X` | Drop selected inventory item |
| `J` / `K` | Navigate inventory (down/up) |
| `Enter` | Open chat |
| `Esc` | Quit |

## Config files

All game data lives in `config/` and is loaded at server startup.
Edit these files to add classes, monsters, items, or change server settings:

```
config/
  classes.json   — character classes
  monsters.json  — monster definitions
  items.json     — item definitions
  world.toml     — server settings (host, port, tick rate, monster count)
```

The server falls back to built-in defaults if any file is missing.

## Architecture

```
src/
  game.rs         — shared data types (Player, Monster, Item, Map, …)
  protocol.rs     — ClientMsg / ServerMsg (newline-delimited JSON over TCP)
  config.rs       — loads config/ directory at startup
  mapgen.rs       — procedural world generator
  server.rs       — async TCP server, game loop, monster AI
  client/
    app.rs        — client state machine, server message handler
    ui.rs         — ratatui terminal UI (map, stats, inventory, chat)
    mod.rs        — async event loop, TCP connection management
  bin/
    server.rs     — server binary entry point
    client.rs     — client binary entry point
```
