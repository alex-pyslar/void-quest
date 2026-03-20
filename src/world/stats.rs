use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub hp:     i32,
    pub max_hp: i32,
    pub mp:     i32,
    pub max_mp: i32,
    pub str:    i32,
    pub dex:    i32,
    pub int:    i32,
    pub vit:    i32,
}

impl Stats {
    pub fn is_alive(&self) -> bool { self.hp > 0 }
    pub fn hp_pct(&self)   -> f64  { self.hp as f64 / self.max_hp.max(1) as f64 }
    pub fn mp_pct(&self)   -> f64  { self.mp as f64 / self.max_mp.max(1) as f64 }
}
