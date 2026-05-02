use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpShareMode {
    Equal,
    LevelBased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemShareMode {
    LeaderPick,
    FreeForAll,
}

#[derive(Debug, Clone)]
pub struct PartyMember {
    pub player_id: Uuid,
    pub name: String,
    pub map_name: String,
    pub hp: u32,
    pub max_hp: u32,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct Party {
    pub id: Uuid,
    pub name: String,
    pub leader_id: Uuid,
    pub members: Vec<PartyMember>,
    pub exp_share: ExpShareMode,
    pub item_share: ItemShareMode,
}
