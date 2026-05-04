use parking_lot::RwLock;
use std::collections::HashMap;

use super::data::Mercenary;

pub struct MercenaryManager {
    mercenaries: RwLock<HashMap<u32, Mercenary>>,
    summoned: RwLock<HashMap<u32, u32>>, // char_id -> mercenary_id
}

impl Default for MercenaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MercenaryManager {
    pub fn new() -> Self {
        Self {
            mercenaries: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
        }
    }

    pub fn create(&self, owner_id: u32, class_id: u16) -> Result<Mercenary, &'static str> {
        let mercenary = Mercenary {
            mercenary_id: rand::random(),
            owner_id,
            mercenary_class: class_id,
            name: format!("Mercenary-{}", owner_id),
            level: 1,
            hp: 1000,
            max_hp: 1000,
            sp: 100,
            max_sp: 100,
            atk: 50,
            loyalty: 100,
            contract_end: None,
            alive: true,
        };

        let id = mercenary.mercenary_id;
        self.mercenaries.write().insert(id, mercenary.clone());
        Ok(mercenary)
    }

    pub fn summon(&self, char_id: u32, mercenary_id: u32) -> Result<(), &'static str> {
        let mercenaries = self.mercenaries.read();
        if !mercenaries.contains_key(&mercenary_id) {
            return Err("Mercenary not found");
        }
        drop(mercenaries);

        self.summoned.write().insert(char_id, mercenary_id);
        Ok(())
    }

    pub fn dismiss(&self, char_id: u32) {
        self.summoned.write().remove(&char_id);
    }

    pub fn get(&self, mercenary_id: u32) -> Option<Mercenary> {
        self.mercenaries.read().get(&mercenary_id).cloned()
    }

    pub fn get_summoned(&self, char_id: u32) -> Option<u32> {
        self.summoned.read().get(&char_id).copied()
    }
}
