use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Self { Self { x, y } }

    pub fn dist_sq(self, o: Pos) -> i32 {
        (self.x - o.x).pow(2) + (self.y - o.y).pow(2)
    }

    pub fn step_toward(self, target: Pos) -> Pos {
        let dx = (target.x - self.x).signum();
        let dy = (target.y - self.y).signum();
        // prefer cardinal movement
        if dx.abs() >= dy.abs() { Pos::new(self.x + dx, self.y) }
        else                    { Pos::new(self.x, self.y + dy) }
    }

    pub fn adjacent(self, o: Pos) -> bool {
        (self.x - o.x).abs() <= 1 && (self.y - o.y).abs() <= 1
    }
}
