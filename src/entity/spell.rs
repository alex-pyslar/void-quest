use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    pub id:      String,
    pub name:    String,
    pub name_ru: Option<String>,
    pub icon:    char,
    pub color:   String,
    pub mp_cost: i32,
    pub damage:  i32,
    pub heal:    i32,
    pub effect:  String,
    pub classes: Vec<String>,
}

impl Spell {
    pub fn display_name<'a>(&'a self, lang: &str) -> &'a str {
        if lang == "ru" {
            if let Some(ref n) = self.name_ru { return n.as_str(); }
        }
        self.name.as_str()
    }
}
