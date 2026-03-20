use rand::Rng;
use crate::world::{GameMap, TileKind};

pub(super) fn place_walled_area(map: &mut GameMap, cx: i32, cy: i32, r: i32) {
    // fill floor
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) { map.set(x, y, TileKind::Floor); }
    }
    // walls on perimeter
    for y in (cy - r)..=(cy + r) {
        map.set(cx - r, y, TileKind::Wall);
        map.set(cx + r, y, TileKind::Wall);
    }
    for x in (cx - r)..=(cx + r) {
        map.set(x, cy - r, TileKind::Wall);
        map.set(x, cy + r, TileKind::Wall);
    }
    // gates N/S/E/W
    map.set(cx,     cy - r, TileKind::Road);
    map.set(cx,     cy + r, TileKind::Road);
    map.set(cx - r, cy,     TileKind::Road);
    map.set(cx + r, cy,     TileKind::Road);
    // cross roads inside
    for x in (cx - r + 1)..cx + r { map.set(x, cy, TileKind::Road); }
    for y in (cy - r + 1)..cy + r { map.set(cx, y, TileKind::Road); }
}

pub(super) fn place_lake(map: &mut GameMap, rng: &mut impl Rng, cx: i32, cy: i32, r: i32, map_w: i32, map_h: i32) {
    for dy in -r..=r {
        for dx in -r..=r {
            let noise = rng.gen_range(-1i32..=1);
            if dx * dx + dy * dy <= (r + noise) * (r + noise) {
                let x = cx + dx;
                let y = cy + dy;
                if x > 0 && y > 0 && x < map_w - 1 && y < map_h - 1 {
                    map.set(x, y, TileKind::Water);
                }
            }
        }
    }
    // sandy shores
    for dy in -(r + 1)..=(r + 1) {
        for dx in -(r + 1)..=(r + 1) {
            let x = cx + dx;
            let y = cy + dy;
            if x > 0 && y > 0 && x < map_w - 1 && y < map_h - 1 {
                if map.get(x, y) == TileKind::Grass { map.set(x, y, TileKind::Sand); }
            }
        }
    }
}

/// Draw a simple L-shaped dirt path from (x0,y0) to (tx,ty).
pub(super) fn draw_path(map: &mut GameMap, x0: i32, y0: i32, tx: i32, ty: i32) {
    let mut x = x0;
    while x != tx {
        let step = (tx - x).signum();
        x += step;
        if map.get(x, y0) == TileKind::Grass { map.set(x, y0, TileKind::Road); }
    }
    let mut y = y0;
    while y != ty {
        let step = (ty - y).signum();
        y += step;
        if map.get(tx, y) == TileKind::Grass { map.set(tx, y, TileKind::Road); }
    }
}
