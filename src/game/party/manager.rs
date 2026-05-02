use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;
use super::data::{Party, PartyMember, ExpShareMode, ItemShareMode};

pub struct PartyManager {
    parties: RwLock<HashMap<Uuid, Party>>,
    player_party: RwLock<HashMap<Uuid, Uuid>>,
}

impl PartyManager {
    pub fn new() -> Self {
        Self {
            parties: RwLock::new(HashMap::new()),
            player_party: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new party with the given player as leader
    pub fn create_party(&self, name: &str, leader_id: Uuid, leader_name: String) -> Party {
        let party = Party {
            id: Uuid::new_v4(),
            name: name.to_string(),
            leader_id,
            members: vec![PartyMember {
                player_id: leader_id,
                name: leader_name,
                map_name: String::new(),
                hp: 0,
                max_hp: 0,
                online: true,
            }],
            exp_share: ExpShareMode::Equal,
            item_share: ItemShareMode::FreeForAll,
        };
        let party_id = party.id;
        self.parties.write().insert(party_id, party.clone());
        self.player_party.write().insert(leader_id, party_id);
        party
    }

    /// Join an existing party
    pub fn join_party(&self, party_id: &Uuid, player_id: Uuid, name: String) -> Option<()> {
        let mut parties = self.parties.write();
        let party = parties.get_mut(party_id)?;
        party.members.push(PartyMember {
            player_id,
            name,
            map_name: String::new(),
            hp: 0,
            max_hp: 0,
            online: true,
        });
        drop(parties);
        self.player_party.write().insert(player_id, *party_id);
        Some(())
    }

    /// Leave a party
    pub fn leave_party(&self, player_id: &Uuid) {
        let party_id = match self.player_party.write().remove(player_id) {
            Some(id) => id,
            None => return,
        };

        let mut parties = self.parties.write();
        if let Some(party) = parties.get_mut(&party_id) {
            party.members.retain(|m| m.player_id != *player_id);

            // Transfer leader if needed
            if party.leader_id == *player_id {
                if let Some(new_leader) = party.members.first() {
                    party.leader_id = new_leader.player_id;
                }
            }

            // Remove empty party
            if party.members.is_empty() {
                parties.remove(&party_id);
            }
        }
    }

    /// Get party by ID
    pub fn get_party(&self, party_id: &Uuid) -> Option<Party> {
        self.parties.read().get(party_id).cloned()
    }

    /// Get party for a player
    pub fn get_player_party(&self, player_id: &Uuid) -> Option<Party> {
        let party_id = self.player_party.read().get(player_id).copied()?;
        self.parties.read().get(&party_id).cloned()
    }

    /// Check if a player is the leader of a party
    pub fn is_leader(&self, party_id: &Uuid, player_id: &Uuid) -> bool {
        self.parties.read().get(party_id).map(|p| p.leader_id == *player_id).unwrap_or(false)
    }

    /// Kick a member from party (only leader can do this)
    pub fn kick_member(&self, party_id: &Uuid, leader_id: &Uuid, target_id: &Uuid) -> bool {
        if !self.is_leader(party_id, leader_id) {
            return false;
        }

        let mut parties = self.parties.write();
        if let Some(party) = parties.get_mut(party_id) {
            party.members.retain(|m| m.player_id != *target_id);
            self.player_party.write().remove(target_id);
            true
        } else {
            false
        }
    }

    /// Update member position/status
    pub fn update_member(&self, player_id: &Uuid, map_name: &str, hp: u32, max_hp: u32) {
        let party_id = match self.player_party.read().get(player_id).copied() {
            Some(id) => id,
            None => return,
        };

        let mut parties = self.parties.write();
        if let Some(party) = parties.get_mut(&party_id) {
            if let Some(member) = party.members.iter_mut().find(|m| m.player_id == *player_id) {
                member.map_name = map_name.to_string();
                member.hp = hp;
                member.max_hp = max_hp;
            }
        }
    }
}

impl Default for PartyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_create_party_creates_party_with_leader() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());

        assert_eq!(party.name, "Test Party");
        assert_eq!(party.leader_id, leader_id);
        assert_eq!(party.members.len(), 1);
        assert_eq!(party.members[0].player_id, leader_id);
        assert_eq!(party.members[0].name, "Leader");
    }

    #[test]
    fn test_join_party_adds_member() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();
        let member_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        manager.join_party(&party_id, member_id, "Member".to_string()).unwrap();

        let updated_party = manager.get_party(&party_id).unwrap();
        assert_eq!(updated_party.members.len(), 2);
        assert!(updated_party.members.iter().any(|m| m.player_id == member_id));
    }

    #[test]
    fn test_leave_party_removes_member_and_transfers_leadership() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();
        let member_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        manager.join_party(&party_id, member_id, "Member".to_string()).unwrap();

        // Leader leaves
        manager.leave_party(&leader_id);

        let updated_party = manager.get_party(&party_id).unwrap();
        assert_eq!(updated_party.members.len(), 1);
        assert_eq!(updated_party.leader_id, member_id);
        assert!(!updated_party.members.iter().any(|m| m.player_id == leader_id));
    }

    #[test]
    fn test_get_party_returns_party_by_id() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        let retrieved = manager.get_party(&party_id).unwrap();
        assert_eq!(retrieved.id, party_id);
        assert_eq!(retrieved.name, "Test Party");
    }

    #[test]
    fn test_get_player_party_returns_party_for_player() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());

        let retrieved = manager.get_player_party(&leader_id).unwrap();
        assert_eq!(retrieved.id, party.id);
    }

    #[test]
    fn test_is_leader_correctly_identifies_leader() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();
        let member_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        manager.join_party(&party_id, member_id, "Member".to_string()).unwrap();

        assert!(manager.is_leader(&party_id, &leader_id));
        assert!(!manager.is_leader(&party_id, &member_id));
    }

    #[test]
    fn test_kick_member_removes_member_when_called_by_leader() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();
        let member_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        manager.join_party(&party_id, member_id, "Member".to_string()).unwrap();

        let result = manager.kick_member(&party_id, &leader_id, &member_id);
        assert!(result);

        let updated_party = manager.get_party(&party_id).unwrap();
        assert_eq!(updated_party.members.len(), 1);
        assert!(!updated_party.members.iter().any(|m| m.player_id == member_id));
    }

    #[test]
    fn test_kick_member_returns_false_when_called_by_non_leader() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();
        let member1_id = create_test_uuid();
        let member2_id = create_test_uuid();

        let party = manager.create_party("Test Party", leader_id, "Leader".to_string());
        let party_id = party.id;

        manager.join_party(&party_id, member1_id, "Member1".to_string()).unwrap();
        manager.join_party(&party_id, member2_id, "Member2".to_string()).unwrap();

        // Try to kick member2 by member1 (not leader)
        let result = manager.kick_member(&party_id, &member1_id, &member2_id);
        assert!(!result);

        // Party should be unchanged
        let updated_party = manager.get_party(&party_id).unwrap();
        assert_eq!(updated_party.members.len(), 3);
    }

    #[test]
    fn test_update_member_updates_status() {
        let manager = PartyManager::new();
        let leader_id = create_test_uuid();

        manager.create_party("Test Party", leader_id, "Leader".to_string());

        manager.update_member(&leader_id, "Map1", 100, 200);

        let party = manager.get_player_party(&leader_id).unwrap();
        let member = party.members.iter().find(|m| m.player_id == leader_id).unwrap();
        assert_eq!(member.map_name, "Map1");
        assert_eq!(member.hp, 100);
        assert_eq!(member.max_hp, 200);
    }
}
