mod helpers;

use rand::Rng;
use crate::world::{GameMap, TileKind, MAP_W, MAP_H, SP_MAP_W, SP_MAP_H};
use helpers::{draw_path, place_lake, place_walled_area};

/// Visual/biome style for zone generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapStyle {
    Wilderness,
    Town,
    Dungeon,
    Desert,
    Forest,
    Swamp,
}

/// Generate a zone map for the given style and seed.
pub fn generate_zone(rng: &mut impl Rng, style: MapStyle) -> GameMap {
    match style {
        MapStyle::Dungeon    => generate_dungeon(rng),
        MapStyle::Desert     => generate_desert(rng),
        MapStyle::Forest     => generate_forest(rng),
        MapStyle::Swamp      => generate_swamp(rng),
        MapStyle::Town       => { let mut m = generate(rng); m.name = "Town".into(); m }
        MapStyle::Wilderness => { let mut m = generate(rng); m.name = "Wilderness".into(); m }
    }
}

// ─── Dungeon ──────────────────────────────────────────────────────────────────

fn generate_dungeon(rng: &mut impl Rng) -> GameMap {
    let w = MAP_W;
    let h = MAP_H;
    let mut map = GameMap::new(w, h);

    // Fill entirely with wall
    for y in 0..h { for x in 0..w { map.set(x, y, TileKind::Wall); } }

    // Carve rooms
    let room_count = rng.gen_range(8usize..14);
    let mut rooms: Vec<(i32, i32, i32, i32)> = Vec::new(); // (x, y, w, h)
    for _ in 0..room_count * 5 {
        if rooms.len() >= room_count { break; }
        let rw = rng.gen_range(4i32..10);
        let rh = rng.gen_range(3i32..7);
        let rx = rng.gen_range(1..w - rw - 1);
        let ry = rng.gen_range(1..h - rh - 1);
        // No overlap check (overlapping rooms are fine — creates passages)
        for dy in 0..rh { for dx in 0..rw { map.set(rx+dx, ry+dy, TileKind::Floor); } }
        rooms.push((rx, ry, rw, rh));
    }

    // Connect rooms with corridors
    for i in 1..rooms.len() {
        let (ax, ay, aw, ah) = rooms[i - 1];
        let (bx, by, bw, bh) = rooms[i];
        let cx0 = ax + aw / 2;
        let cy0 = ay + ah / 2;
        let cx1 = bx + bw / 2;
        let cy1 = by + bh / 2;
        // Horizontal then vertical
        let sx = if cx0 < cx1 { 1 } else { -1 };
        let mut x = cx0;
        while x != cx1 { map.set(x, cy0, TileKind::Floor); x += sx; }
        let sy = if cy0 < cy1 { 1 } else { -1 };
        let mut y = cy0;
        while y != cy1 { map.set(cx1, y, TileKind::Floor); y += sy; }
    }

    // Stone pillars scattered in rooms
    for _ in 0..rng.gen_range(6usize..16) {
        let px = rng.gen_range(3..w - 3);
        let py = rng.gen_range(3..h - 3);
        if map.get(px, py) == TileKind::Floor {
            map.set(px, py, TileKind::Pillar);
        }
    }

    // Ruined sections (passable crumbled stone)
    for _ in 0..rng.gen_range(4usize..10) {
        let rx = rng.gen_range(2..w - 2);
        let ry = rng.gen_range(2..h - 2);
        for dy in -1i32..=1 { for dx in -1i32..=1 {
            if map.get(rx+dx, ry+dy) == TileKind::Floor && rng.gen_bool(0.6) {
                map.set(rx+dx, ry+dy, TileKind::Ruins);
            }
        }}
    }

    // Some water pools (underground lakes)
    for _ in 0..rng.gen_range(1usize..4) {
        let lx = rng.gen_range(3..w - 3);
        let ly = rng.gen_range(3..h - 3);
        let lr = rng.gen_range(1i32..3);
        place_lake(&mut map, rng, lx, ly, lr, w, h);
    }

    map.name = "Dungeon".into();
    map
}

// ─── Desert ───────────────────────────────────────────────────────────────────

fn generate_desert(rng: &mut impl Rng) -> GameMap {
    let w = MAP_W;
    let h = MAP_H;
    let mut map = GameMap::new(w, h);

    // Fill with sand
    for y in 0..h { for x in 0..w { map.set(x, y, TileKind::Sand); } }

    // Border walls
    for x in 0..w { map.set(x, 0, TileKind::Wall); map.set(x, h-1, TileKind::Wall); }
    for y in 0..h { map.set(0, y, TileKind::Wall); map.set(w-1, y, TileKind::Wall); }

    // Rock formations (wall clusters)
    for _ in 0..rng.gen_range(8usize..16) {
        let rx = rng.gen_range(3..w-3);
        let ry = rng.gen_range(3..h-3);
        let rr = rng.gen_range(1i32..4);
        for dy in -rr..=rr { for dx in -rr..=rr {
            if dx*dx + dy*dy <= rr*rr && rng.gen_bool(0.7) {
                map.set(rx+dx, ry+dy, TileKind::Wall);
            }
        }}
    }

    // Oasis (water + grass)
    let num_oasis = rng.gen_range(1usize..4);
    for _ in 0..num_oasis {
        let ox = rng.gen_range(5..w-5);
        let oy = rng.gen_range(5..h-5);
        let or_ = rng.gen_range(2i32..5);
        place_lake(&mut map, rng, ox, oy, or_, w, h);
        // Grass ring around oasis
        for dy in -(or_+2)..=(or_+2) { for dx in -(or_+2)..=(or_+2) {
            let x = ox+dx; let y = oy+dy;
            if x > 0 && y > 0 && x < w-1 && y < h-1 && map.get(x,y) == TileKind::Sand {
                if dx*dx + dy*dy <= (or_+2)*(or_+2) { map.set(x, y, TileKind::Grass); }
            }
        }}
    }

    // Lava vents (volcanic cracks)
    for _ in 0..rng.gen_range(3usize..7) {
        let lx = rng.gen_range(3..w-3);
        let ly = rng.gen_range(3..h-3);
        for dy in -1i32..=1 { for dx in -2i32..=2 {
            let x = lx+dx; let y = ly+dy;
            if x > 0 && y > 0 && x < w-1 && y < h-1
                && map.get(x, y) == TileKind::Sand && rng.gen_bool(0.55) {
                map.set(x, y, TileKind::Lava);
            }
        }}
    }

    // Desert outpost (small walled settlement)
    let cx = w/2; let cy = h/2;
    place_walled_area(&mut map, cx, cy, 5);
    // Road heading East
    for x in cx+6..w-1 { if map.get(x, cy) == TileKind::Sand { map.set(x, cy, TileKind::Road); } }

    map.name = "Desert".into();
    map
}

// ─── Forest ───────────────────────────────────────────────────────────────────

fn generate_forest(rng: &mut impl Rng) -> GameMap {
    let w = MAP_W;
    let h = MAP_H;
    let mut map = GameMap::new(w, h);

    // Fill with grass
    for y in 0..h { for x in 0..w { map.set(x, y, TileKind::Grass); } }

    // Border walls
    for x in 0..w { map.set(x, 0, TileKind::Wall); map.set(x, h-1, TileKind::Wall); }
    for y in 0..h { map.set(0, y, TileKind::Wall); map.set(w-1, y, TileKind::Wall); }

    // Very dense trees (80% coverage except clearing areas)
    let clearing_count = rng.gen_range(3usize..7);
    let mut clearings: Vec<(i32, i32, i32)> = Vec::new();
    for _ in 0..clearing_count {
        clearings.push((rng.gen_range(8..w-8), rng.gen_range(6..h-6), rng.gen_range(4i32..8)));
    }

    for y in 1..h-1 { for x in 1..w-1 {
        let in_clearing = clearings.iter().any(|&(cx, cy, cr)| {
            let d = (x-cx)*(x-cx) + (y-cy)*(y-cy);
            d <= cr*cr
        });
        if !in_clearing && rng.gen_bool(0.72) {
            map.set(x, y, TileKind::Tree);
        }
    }}

    // Streams through clearings
    if let Some(&(_cx, cy, _)) = clearings.first() {
        let mut rx = 2i32;
        for y in 1..cy {
            map.set(rx, y, TileKind::Water);
            if rng.gen_bool(0.3) { rx = (rx + rng.gen_range(-1i32..=1)).clamp(2, w-3); }
        }
    }

    // A small ranger outpost in centre clearing
    if let Some(&(cx, cy, _)) = clearings.get(clearings.len() / 2) {
        place_walled_area(&mut map, cx, cy, 3);
    }

    // Bramble patches between trees
    for _ in 0..rng.gen_range(10usize..20) {
        let bx = rng.gen_range(2..w-2);
        let by = rng.gen_range(2..h-2);
        for dy in -1i32..=1 { for dx in -1i32..=1 {
            let x = bx+dx; let y = by+dy;
            if x > 0 && y > 0 && x < w-1 && y < h-1
                && map.get(x, y) == TileKind::Grass && rng.gen_bool(0.55) {
                map.set(x, y, TileKind::Bramble);
            }
        }}
    }

    // Paths between clearings
    for i in 1..clearings.len() {
        let (ax, ay, _) = clearings[i-1];
        let (bx, by, _) = clearings[i];
        draw_path(&mut map, ax, ay, bx, by);
    }

    map.name = "Forest".into();
    map
}

// ─── Swamp ────────────────────────────────────────────────────────────────────

fn generate_swamp(rng: &mut impl Rng) -> GameMap {
    let w = MAP_W;
    let h = MAP_H;
    let mut map = GameMap::new(w, h);

    // Mix of grass, water, sand (mud)
    for y in 0..h { for x in 0..w {
        let t = match rng.gen_range(0u8..10) {
            0..=3 => TileKind::Grass,
            4..=5 => TileKind::Water,
            6..=7 => TileKind::Sand,
            _     => TileKind::Grass,
        };
        map.set(x, y, t);
    }}

    // Border walls
    for x in 0..w { map.set(x, 0, TileKind::Wall); map.set(x, h-1, TileKind::Wall); }
    for y in 0..h { map.set(0, y, TileKind::Wall); map.set(w-1, y, TileKind::Wall); }

    // Extra water pools
    for _ in 0..rng.gen_range(5usize..10) {
        let lx = rng.gen_range(3..w-3);
        let ly = rng.gen_range(3..h-3);
        let lr = rng.gen_range(2i32..6);
        place_lake(&mut map, rng, lx, ly, lr, w, h);
    }

    // Scattered dead trees
    for _ in 0..rng.gen_range(30usize..60) {
        let tx = rng.gen_range(2..w-2);
        let ty = rng.gen_range(2..h-2);
        if map.get(tx, ty) == TileKind::Grass { map.set(tx, ty, TileKind::Tree); }
    }

    // Mud patches (passable, murky ground)
    for _ in 0..rng.gen_range(12usize..24) {
        let mx2 = rng.gen_range(2..w-2);
        let my2 = rng.gen_range(2..h-2);
        for dy in -2i32..=2 { for dx in -2i32..=2 {
            let x = mx2+dx; let y = my2+dy;
            if x > 0 && y > 0 && x < w-1 && y < h-1
                && map.get(x, y) == TileKind::Grass && rng.gen_bool(0.5) {
                map.set(x, y, TileKind::Mud);
            }
        }}
    }

    // Bramble thickets
    for _ in 0..rng.gen_range(8usize..18) {
        let bx = rng.gen_range(2..w-2);
        let by = rng.gen_range(2..h-2);
        for dy in -1i32..=1 { for dx in -1i32..=1 {
            let x = bx+dx; let y = by+dy;
            if x > 0 && y > 0 && x < w-1 && y < h-1
                && matches!(map.get(x, y), TileKind::Grass | TileKind::Mud) && rng.gen_bool(0.6) {
                map.set(x, y, TileKind::Bramble);
            }
        }}
    }

    // A ruined outpost
    let cx = w/2; let cy = h/2;
    place_walled_area(&mut map, cx, cy, 4);

    map.name = "Swamp".into();
    map
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Generate a procedural world map with winding river, lakes, forests,
/// random outposts, and a central walled town.
pub fn generate(rng: &mut impl Rng) -> GameMap {
    let mut map = GameMap::new(MAP_W, MAP_H);

    // ── fill with grass ──────────────────────────────────────────────────────
    for y in 0..MAP_H {
        for x in 0..MAP_W { map.set(x, y, TileKind::Grass); }
    }

    // ── border walls ─────────────────────────────────────────────────────────
    for x in 0..MAP_W {
        map.set(x, 0,         TileKind::Wall);
        map.set(x, MAP_H - 1, TileKind::Wall);
    }
    for y in 0..MAP_H {
        map.set(0,         y, TileKind::Wall);
        map.set(MAP_W - 1, y, TileKind::Wall);
    }

    // ── winding river ─────────────────────────────────────────────────────────
    let start_x = rng.gen_range(MAP_W / 6..MAP_W / 3);
    let mut rx  = start_x;
    let cx = MAP_W / 2;
    let cy = MAP_H / 2;

    // store river x per row to know where the bridge goes
    let mut river_xs = vec![0i32; MAP_H as usize];
    for y in 1..MAP_H - 1 {
        river_xs[y as usize] = rx;
        map.set(rx,     y, TileKind::Water);
        map.set(rx + 1, y, TileKind::Water);
        // sandy banks
        if map.get(rx - 1, y) == TileKind::Grass { map.set(rx - 1, y, TileKind::Sand); }
        if map.get(rx + 2, y) == TileKind::Grass { map.set(rx + 2, y, TileKind::Sand); }
        // drift
        if rng.gen_bool(0.35) {
            rx = (rx + rng.gen_range(-1i32..=1)).clamp(3, MAP_W - 5);
        }
    }
    let bridge_rx = river_xs[cy as usize];

    // ── optional lake in one quadrant ─────────────────────────────────────────
    if rng.gen_bool(0.65) {
        let quadrant = rng.gen_range(0..4usize);
        let (lx, ly) = match quadrant {
            0 => (rng.gen_range(cx + 12..MAP_W - 8), rng.gen_range(3..cy - 6)),
            1 => (rng.gen_range(cx + 12..MAP_W - 8), rng.gen_range(cy + 6..MAP_H - 8)),
            2 => (rng.gen_range(bridge_rx + 4..cx - 8), rng.gen_range(cy + 6..MAP_H - 8)),
            _ => (rng.gen_range(bridge_rx + 4..cx - 8), rng.gen_range(3..cy - 6)),
        };
        let lr = rng.gen_range(3i32..6);
        place_lake(&mut map, rng, lx, ly, lr, MAP_W, MAP_H);
    }

    // ── central town ─────────────────────────────────────────────────────────
    place_walled_area(&mut map, cx, cy, 7);

    // ── roads ────────────────────────────────────────────────────────────────
    // N/S roads from town edges to map border
    for y in 1..cy - 7 {
        if map.get(cx, y) != TileKind::Water { map.set(cx, y, TileKind::Road); }
    }
    for y in cy + 8..MAP_H - 1 {
        map.set(cx, y, TileKind::Road);
    }
    // W road: town gate → bridge
    for x in bridge_rx + 3..cx - 7 {
        if map.get(x, cy) != TileKind::Water { map.set(x, cy, TileKind::Road); }
    }
    // E road: town → east border
    for x in cx + 8..MAP_W - 1 {
        map.set(x, cy, TileKind::Road);
    }

    // ── bridge over river ─────────────────────────────────────────────────────
    for bx in bridge_rx - 1..=bridge_rx + 3 {
        if bx > 0 && bx < MAP_W - 1 {
            map.set(bx, cy, TileKind::Road);
        }
    }

    // ── random outposts ───────────────────────────────────────────────────────
    let outpost_target = rng.gen_range(2usize..5);
    let mut placed = 0usize;
    let mut attempts = 0usize;
    while placed < outpost_target && attempts < 60 {
        attempts += 1;
        let ox = rng.gen_range(5..MAP_W - 10);
        let oy = rng.gen_range(5..MAP_H - 10);
        let r  = rng.gen_range(2i32..4);
        // don't overlap town or river
        if (ox - cx).abs() < 14 && (oy - cy).abs() < 14 { continue; }
        if (ox - bridge_rx).abs() < 4 { continue; }
        // small path from outpost toward nearest road
        place_walled_area(&mut map, ox, oy, r);
        // connect via dirt path to main road
        let (tx, ty) = if (ox - cx).abs() < (oy - cy).abs() {
            (cx, oy) // go horizontal to main N-S road
        } else {
            (ox, cy) // go vertical to main E-W road
        };
        draw_path(&mut map, ox, oy, tx, ty);
        placed += 1;
    }

    // ── forests ───────────────────────────────────────────────────────────────
    let forest_count = rng.gen_range(12usize..20);
    for _ in 0..forest_count {
        let bx = rng.gen_range(2..MAP_W - 2);
        let by = rng.gen_range(2..MAP_H - 2);
        // keep away from town center
        if (bx - cx).abs() < 10 && (by - cy).abs() < 10 { continue; }
        let density = rng.gen_range(4usize..16);
        for _ in 0..density {
            let tx = bx + rng.gen_range(-6i32..7);
            let ty = by + rng.gen_range(-6i32..7);
            if tx > 0 && ty > 0 && tx < MAP_W - 1 && ty < MAP_H - 1 {
                if map.get(tx, ty) == TileKind::Grass {
                    map.set(tx, ty, TileKind::Tree);
                }
            }
        }
    }

    // ── sand patches ─────────────────────────────────────────────────────────
    for _ in 0..8 {
        let sx = rng.gen_range(2..MAP_W - 2);
        let sy = rng.gen_range(2..MAP_H - 2);
        let r  = rng.gen_range(1..4i32);
        for dy in -r..=r { for dx in -r..=r {
            let x = sx + dx; let y = sy + dy;
            if x > 0 && y > 0 && x < MAP_W - 1 && y < MAP_H - 1 {
                if map.get(x, y) == TileKind::Grass { map.set(x, y, TileKind::Sand); }
            }
        }}
    }

    // ── ice patches (frozen ponds) ────────────────────────────────────────────
    for _ in 0..rng.gen_range(2usize..5) {
        let ix = rng.gen_range(3..MAP_W - 3);
        let iy = rng.gen_range(3..MAP_H - 3);
        let ir = rng.gen_range(1i32..3);
        for dy in -ir..=ir { for dx in -ir..=ir {
            let x = ix+dx; let y = iy+dy;
            if x > 0 && y > 0 && x < MAP_W-1 && y < MAP_H-1
                && dx*dx + dy*dy <= ir*ir
                && map.get(x, y) == TileKind::Grass {
                map.set(x, y, TileKind::Ice);
            }
        }}
    }

    // ── ruins clusters ────────────────────────────────────────────────────────
    for _ in 0..rng.gen_range(3usize..7) {
        let rx2 = rng.gen_range(4..MAP_W - 4);
        let ry2 = rng.gen_range(4..MAP_H - 4);
        if (rx2 - cx).abs() < 12 && (ry2 - cy).abs() < 12 { continue; }
        for dy in -2i32..=2 { for dx in -2i32..=2 {
            let x = rx2+dx; let y = ry2+dy;
            if x > 0 && y > 0 && x < MAP_W-1 && y < MAP_H-1
                && map.get(x, y) == TileKind::Grass && rng.gen_bool(0.5) {
                map.set(x, y, TileKind::Ruins);
            }
        }}
        // Broken walls around ruins
        for dy in -3i32..=3 { for dx in -3i32..=3 {
            let x = rx2+dx; let y = ry2+dy;
            if x > 0 && y > 0 && x < MAP_W-1 && y < MAP_H-1
                && (dx.abs() == 3 || dy.abs() == 3)
                && map.get(x, y) == TileKind::Grass && rng.gen_bool(0.45) {
                map.set(x, y, TileKind::Wall);
            }
        }}
    }

    // ── bramble patches ───────────────────────────────────────────────────────
    for _ in 0..rng.gen_range(6usize..12) {
        let bx = rng.gen_range(2..MAP_W - 2);
        let by = rng.gen_range(2..MAP_H - 2);
        for dy in -2i32..=2 { for dx in -2i32..=2 {
            let x = bx+dx; let y = by+dy;
            if x > 0 && y > 0 && x < MAP_W-1 && y < MAP_H-1
                && map.get(x, y) == TileKind::Grass && rng.gen_bool(0.5) {
                map.set(x, y, TileKind::Bramble);
            }
        }}
    }

    map.name = "Voidlands".into();
    map
}

/// Generate a large 256×128 singleplayer map with multiple rivers, lakes,
/// towns, dense forests, varied biomes, and outposts connected by roads.
pub fn generate_large(rng: &mut impl Rng) -> GameMap {
    let w = SP_MAP_W;
    let h = SP_MAP_H;
    let mut map = GameMap::new(w, h);
    let cx = w / 2;
    let cy = h / 2;

    // ── fill with grass ───────────────────────────────────────────────────────
    for y in 0..h {
        for x in 0..w { map.set(x, y, TileKind::Grass); }
    }

    // ── border walls ──────────────────────────────────────────────────────────
    for x in 0..w {
        map.set(x, 0,     TileKind::Wall);
        map.set(x, h - 1, TileKind::Wall);
    }
    for y in 0..h {
        map.set(0,     y, TileKind::Wall);
        map.set(w - 1, y, TileKind::Wall);
    }

    // ── multiple winding rivers (2-3) ─────────────────────────────────────────
    let num_rivers = rng.gen_range(2usize..4);
    let mut river_start_xs = Vec::new();
    for i in 0..num_rivers {
        let seg = w / (num_rivers as i32 + 1);
        let base_x = seg * (i as i32 + 1);
        let sx = (base_x + rng.gen_range(-10i32..10)).clamp(8, w - 8);
        river_start_xs.push(sx);
        let mut rx = sx;
        for y in 1..h - 1 {
            map.set(rx,     y, TileKind::Water);
            map.set(rx + 1, y, TileKind::Water);
            // sandy banks
            if map.get(rx - 1, y) == TileKind::Grass { map.set(rx - 1, y, TileKind::Sand); }
            if map.get(rx + 2, y) == TileKind::Grass { map.set(rx + 2, y, TileKind::Sand); }
            if rng.gen_bool(0.30) {
                rx = (rx + rng.gen_range(-2i32..=2)).clamp(4, w - 6);
            }
        }
    }

    // ── multiple lakes (3-5) ──────────────────────────────────────────────────
    let num_lakes = rng.gen_range(3usize..6);
    for _ in 0..num_lakes {
        let lx = rng.gen_range(10..w - 10);
        let ly = rng.gen_range(8..h - 8);
        let lr = rng.gen_range(3i32..9);
        place_lake(&mut map, rng, lx, ly, lr, w, h);
    }

    // ── 2-3 walled towns/villages of varying sizes ────────────────────────────
    // Central town
    place_walled_area(&mut map, cx, cy, 9);

    // Additional towns
    let num_extra_towns = rng.gen_range(1usize..3);
    let mut town_positions = vec![(cx, cy)];
    for _ in 0..num_extra_towns {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 80 { break; }
            let tx = rng.gen_range(15..w - 15);
            let ty = rng.gen_range(10..h - 10);
            // don't place too close to existing towns
            if town_positions.iter().any(|&(px, py)| {
                let dx = (tx - px).abs();
                let dy = (ty - py).abs();
                dx < 30 && dy < 20
            }) { continue; }
            let tr = rng.gen_range(4i32..8);
            place_walled_area(&mut map, tx, ty, tr);
            town_positions.push((tx, ty));
            break;
        }
    }

    // ── roads connecting towns ────────────────────────────────────────────────
    // Roads from central town outward
    for y in 1..cy - 9 {
        if map.get(cx, y) != TileKind::Water { map.set(cx, y, TileKind::Road); }
    }
    for y in cy + 10..h - 1 {
        if map.get(cx, y) != TileKind::Water { map.set(cx, y, TileKind::Road); }
    }
    for x in 1..cx - 9 {
        if map.get(x, cy) != TileKind::Water { map.set(x, cy, TileKind::Road); }
    }
    for x in cx + 10..w - 1 {
        if map.get(x, cy) != TileKind::Water { map.set(x, cy, TileKind::Road); }
    }

    // Roads connecting extra towns to center
    for &(tx, ty) in &town_positions[1..] {
        draw_path(&mut map, tx, ty, cx, cy);
    }

    // ── more outposts connected by roads (5-8) ────────────────────────────────
    let outpost_count = rng.gen_range(5usize..9);
    let mut placed = 0;
    let mut attempts = 0;
    while placed < outpost_count && attempts < 150 {
        attempts += 1;
        let ox = rng.gen_range(6..w - 10);
        let oy = rng.gen_range(6..h - 10);
        let r  = rng.gen_range(2i32..5);
        // don't overlap towns
        if town_positions.iter().any(|&(tx, ty)| {
            (ox - tx).abs() < 18 && (oy - ty).abs() < 14
        }) { continue; }
        place_walled_area(&mut map, ox, oy, r);
        // connect to nearest town via road
        let nearest = town_positions.iter()
            .min_by_key(|&&(tx, ty)| {
                let d = (ox - tx).abs() + (oy - ty).abs();
                d
            })
            .copied()
            .unwrap_or((cx, cy));
        draw_path(&mut map, ox, oy, nearest.0, nearest.1);
        placed += 1;
    }

    // ── dense forests ─────────────────────────────────────────────────────────
    let forest_count = rng.gen_range(30usize..50);
    for _ in 0..forest_count {
        let bx = rng.gen_range(3..w - 3);
        let by = rng.gen_range(3..h - 3);
        // keep away from central town
        if (bx - cx).abs() < 12 && (by - cy).abs() < 12 { continue; }
        let density = rng.gen_range(8usize..30);
        let spread = rng.gen_range(5i32..15);
        for _ in 0..density {
            let tx = bx + rng.gen_range(-spread..=spread);
            let ty = by + rng.gen_range(-spread..=spread);
            if tx > 1 && ty > 1 && tx < w - 2 && ty < h - 2 {
                if map.get(tx, ty) == TileKind::Grass {
                    map.set(tx, ty, TileKind::Tree);
                }
            }
        }
    }

    // ── lots of sand patches ──────────────────────────────────────────────────
    for _ in 0..25 {
        let sx = rng.gen_range(3..w - 3);
        let sy = rng.gen_range(3..h - 3);
        let r  = rng.gen_range(1..6i32);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let x = sx + dx;
                    let y = sy + dy;
                    if x > 1 && y > 1 && x < w - 2 && y < h - 2 {
                        if map.get(x, y) == TileKind::Grass {
                            map.set(x, y, TileKind::Sand);
                        }
                    }
                }
            }
        }
    }

    map.name = "Greater Voidlands".into();
    map
}

/// Returns all passable positions outside the town safe zone.
pub fn monster_spawn_positions(map: &GameMap) -> Vec<(i32, i32)> {
    let cx = map.width / 2;
    let cy = map.height / 2;
    let mut out = Vec::new();
    for y in 1..map.height - 1 {
        for x in 1..map.width - 1 {
            let dx = (x - cx).abs();
            let dy = (y - cy).abs();
            if dx > 12 || dy > 12 {
                if map.passable(x, y) { out.push((x, y)); }
            }
        }
    }
    out
}
