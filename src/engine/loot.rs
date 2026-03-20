use crate::entity::Item;
use super::LocalGame;

impl LocalGame {
    pub(crate) fn make_item(&mut self, template_id: &str) -> Option<Item> {
        let tmpl = self.cfg.items.get(template_id)?.clone();
        let name = tmpl.display_name(&self.cfg.locale.lang).to_string();
        self.next_id += 1;
        Some(Item {
            id:          self.next_id,
            template_id: tmpl.id,
            name,
            symbol:      tmpl.symbol,
            color:       tmpl.color,
            kind:        tmpl.kind,
        })
    }
}
