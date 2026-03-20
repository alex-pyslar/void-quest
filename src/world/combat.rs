use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEvent {
    pub attacker: String,
    pub target:   String,
    pub damage:   i32,
    pub is_crit:  bool,
    pub killed:   bool,
}
