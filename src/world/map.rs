use serde::{Deserialize, Serialize};
use super::tile::TileKind;

pub const MAP_W: i32 = 80;
pub const MAP_H: i32 = 50;

/// Large singleplayer map dimensions
pub const SP_MAP_W: i32 = 256;
pub const SP_MAP_H: i32 = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMap {
    pub width:  i32,
    pub height: i32,
    pub tiles:  Vec<u8>,   // TileKind as u8, row-major (y * width + x)
    pub name:   String,
}

impl GameMap {
    pub fn new(w: i32, h: i32) -> Self {
        Self { width: w, height: h, tiles: vec![0u8; (w * h) as usize], name: "World".into() }
    }

    pub fn get(&self, x: i32, y: i32) -> TileKind {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return TileKind::Wall;
        }
        TileKind::from_u8(self.tiles[(y * self.width + x) as usize])
    }

    pub fn set(&mut self, x: i32, y: i32, t: TileKind) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.tiles[(y * self.width + x) as usize] = t as u8;
        }
    }

    pub fn passable(&self, x: i32, y: i32) -> bool {
        self.get(x, y).passable()
    }
}
