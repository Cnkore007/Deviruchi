use parking_lot::RwLock;
use std::collections::HashMap;

use super::data::{Homunculus, HomunculusType};

pub struct HomunculusManager {
    homunculi: RwLock<HashMap<u32, Homunculus>>,
    summoned: RwLock<HashMap<u32, u32>>, // char_id -> homun_id
}

impl Default for HomunculusManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HomunculusManager {
    pub fn new() -> Self {
        Self {
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
        }
    }

    pub fn create(
        &self,
        owner_id: u32,
        htype: HomunculusType,
        name: &str,
    ) -> Result<Homunculus, &'static str> {
        let homun = Homunculus {
            homun_id: rand::random(),
            owner_id,
            homunculus_type: htype,
            name: name.to_string(),
            level: 1,
            exp: 0,
            hunger: 100,
            intimacy: 100,
            hp: 500,
            max_hp: 500,
            alive: true,
        };

        let id = homun.homun_id;
        self.homunculi.write().insert(id, homun.clone());
        Ok(homun)
    }

    pub fn summon(&self, char_id: u32) -> Result<(), &'static str> {
        let homunculi = self.homunculi.read();
        if homunculi.values().find(|h| h.owner_id == char_id).is_none() {
            return Err("No homunculus found for this character");
        }
        drop(homunculi);

        self.summoned.write().insert(char_id, char_id);
        Ok(())
    }

    pub fn dismiss(&self, char_id: u32) {
        self.summoned.write().remove(&char_id);
    }

    pub fn feed(&self, homun_id: u32, _item_id: u16) -> Result<(), &'static str> {
        let mut homunculi = self.homunculi.write();
        let homun = homunculi.get_mut(&homun_id).ok_or("Homunculus not found")?;
        homun.hunger = (homun.hunger + 20).min(100);
        Ok(())
    }
}
