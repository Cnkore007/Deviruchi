use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Mercenary {
    pub mercenary_id: u32,
    pub owner_id: u32,
    pub mercenary_class: u16,
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub atk: u32,
    pub loyalty: u32,
    pub contract_end: Option<DateTime<Utc>>,
    pub alive: bool,
}

#[derive(Debug, Clone)]
pub struct MercenaryData {
    pub class_id: u16,
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub atk: u32,
    pub contract_cost: u32,
}
