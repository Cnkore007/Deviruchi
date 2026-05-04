#[derive(Debug, Clone, Copy)]
pub enum HomunculusType {
    Amistad,
    Filir,
    Vanilmirth,
}

#[derive(Debug, Clone)]
pub struct Homunculus {
    pub homun_id: u32,
    pub owner_id: u32,
    pub homunculus_type: HomunculusType,
    pub name: String,
    pub level: u16,
    pub exp: u64,
    pub hunger: u32,
    pub intimacy: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub alive: bool,
}
