//! 宠物管理器模块

use crate::game::pet::data::{Pet, PetData, PetDatabase};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// 宠物错误类型
#[derive(Debug, Error, Clone)]
pub enum PetError {
    #[error("Pet not found: {0}")]
    PetNotFound(u32),

    #[error("Player not found: {0}")]
    PlayerNotFound(Uuid),

    #[error("Player already has a summoned pet")]
    AlreadyHasPet,

    #[error("Not your pet")]
    NotYourPet,

    #[error("Pet already summoned")]
    PetAlreadySummoned,

    #[error("No pet summoned")]
    NoSummonedPet,

    #[error("Invalid mob id: {0}")]
    InvalidMobId(u16),

    #[error("Capture failed: low capture rate")]
    CaptureFailed,

    #[error("Item is not pet food")]
    NotPetFood,

    #[error("Pet data not found for mob: {0}")]
    PetDataNotFound(u16),

    #[error("Cannot rename pet")]
    CannotRename,
}

/// 宠物管理器
pub struct PetManager {
    /// 所有宠物实例 (pet_id -> Pet)
    pets: RwLock<HashMap<u32, Pet>>,
    /// 当前召唤的宠物 (player_id -> Pet)
    summoned_pets: RwLock<HashMap<Uuid, Pet>>,
    /// 宠物数据库
    pet_database: PetDatabase,
    /// 下一个可用的宠物ID
    next_pet_id: RwLock<u32>,
}

impl PetManager {
    pub fn new() -> Self {
        Self {
            pets: RwLock::new(HashMap::new()),
            summoned_pets: RwLock::new(HashMap::new()),
            pet_database: PetDatabase::new(),
            next_pet_id: RwLock::new(1),
        }
    }

    /// 捕获宠物
    ///
    /// 玩家使用"捕捉用道具"对怪物使用时调用
    pub fn capture_pet(
        &self,
        player_id: u32,
        player_uuid: Uuid,
        mob_id: u16,
    ) -> Result<Pet, PetError> {
        // 获取怪物对应的宠物数据
        let pet_data = self
            .pet_database
            .get_by_mob_id(mob_id)
            .ok_or(PetError::PetDataNotFound(mob_id))?;

        // 计算捕获成功率
        // 基础成功率 + 玩家等级加成（待实现）
        let base_rate = pet_data.capture_rate as u32;
        let capture_roll = rand::random::<u32>() % 100;

        if capture_roll >= base_rate {
            return Err(PetError::CaptureFailed);
        }

        // 生成新宠物
        let mut next_id = self.next_pet_id.write();
        let pet_id = *next_id;
        *next_id += 1;

        let pet = Pet::new(
            pet_id,
            player_id,
            player_uuid,
            mob_id,
            pet_data.name.clone(),
            pet_data.egg_id,
            pet_data.equip_id,
        );

        // 保存宠物
        self.pets.write().insert(pet_id, pet.clone());

        Ok(pet)
    }

    /// 召唤宠物
    pub fn summon_pet(&self, player_id: Uuid, pet_id: u32) -> Result<(), PetError> {
        // 检查玩家是否已有召唤宠物
        if self.summoned_pets.read().contains_key(&player_id) {
            return Err(PetError::AlreadyHasPet);
        }

        // 获取宠物
        let pet = self
            .pets
            .read()
            .get(&pet_id)
            .ok_or(PetError::PetNotFound(pet_id))?
            .clone();

        // 检查归属权（使用owner_uuid）
        if pet.owner_uuid != player_id {
            return Err(PetError::NotYourPet);
        }

        // 检查宠物是否背叛
        if pet.will_abandon() {
            return Err(PetError::PetNotFound(pet_id));
        }

        // 召唤宠物
        self.summoned_pets.write().insert(player_id, pet);

        Ok(())
    }

    /// 解散宠物
    pub fn dismiss_pet(&self, player_id: Uuid) {
        self.summoned_pets.write().remove(&player_id);
    }

    /// 获取玩家召唤的宠物
    pub fn get_summoned_pet(&self, player_id: Uuid) -> Option<Pet> {
        self.summoned_pets.read().get(&player_id).cloned()
    }

    /// 喂食宠物
    pub fn feed_pet(&self, player_id: Uuid, item_id: u16) -> Result<(), PetError> {
        let mut summoned = self.summoned_pets.write();

        let pet = summoned
            .get_mut(&player_id)
            .ok_or(PetError::NoSummonedPet)?;

        // 检查是否是宠物食物（通过 monster_id 查找宠物数据）
        let pet_data = self
            .pet_database
            .get_by_mob_id(pet.monster_id)
            .ok_or(PetError::PetDataNotFound(pet.monster_id))?;

        if !pet_data.food.contains(&item_id) {
            return Err(PetError::NotPetFood);
        }

        // 根据食物类型恢复饥饿值
        let hunger_restore = match item_id {
            530..=534 => 30, // Jellopies: 30
            505..=509 => 40, // Potions: 40
            551..=555 => 50, // Meats: 50
            _ => 20,         // Default: 20
        };

        pet.feed(hunger_restore);

        // 食物也会增加少量亲密度
        pet.increase_intimacy(50);

        Ok(())
    }

    /// 重命名宠物
    pub fn rename_pet(&self, player_id: Uuid, new_name: &str) -> Result<(), PetError> {
        let mut summoned = self.summoned_pets.write();

        let pet = summoned
            .get_mut(&player_id)
            .ok_or(PetError::NoSummonedPet)?;

        // 检查宠物是否已被命名过
        if pet.renamed {
            return Err(PetError::CannotRename);
        }

        // 重命名
        pet.rename(new_name);

        Ok(())
    }

    /// 获取宠物数据
    pub fn get_pet_data(&self, mob_id: u16) -> Option<PetData> {
        self.pet_database.get_by_mob_id(mob_id).cloned()
    }

    /// 获取宠物数据库
    pub fn get_pet_database(&self) -> &PetDatabase {
        &self.pet_database
    }

    /// 更新宠物饥饿度（游戏循环调用）
    pub fn update_pet_hunger(&self, player_id: Uuid) {
        let mut summoned = self.summoned_pets.write();

        if let Some(pet) = summoned.get_mut(&player_id) {
            let pet_data = self.pet_database.get_by_mob_id(pet.monster_id);

            if let Some(data) = pet_data {
                // 消耗饥饿
                pet.decrease_hunger(data.hunger_decrease);

                // 如果饥饿度过低，降低亲密度
                if pet.hunger < data.full_ratio {
                    pet.decrease_intimacy(data.intimacy_decrease);
                }
            }
        }
    }

    /// 获取玩家的所有宠物
    pub fn get_player_pets(&self, player_id: u32) -> Vec<Pet> {
        self.pets
            .read()
            .values()
            .filter(|p| p.owner_id == player_id)
            .cloned()
            .collect()
    }

    /// 删除宠物
    pub fn delete_pet(&self, pet_id: u32) -> bool {
        self.pets.write().remove(&pet_id).is_some()
    }

    /// 保存宠物信息到数据库（用于持久化）
    pub fn save_pet(&self, pet: &Pet) {
        self.pets.write().insert(pet.pet_id, pet.clone());
    }

    /// 从数据库加载宠物
    pub fn load_pet(&self, pet: Pet) {
        let pet_id = pet.pet_id;
        self.pets.write().insert(pet_id, pet);
        // 更新next_pet_id
        let mut next_id = self.next_pet_id.write();
        if pet_id >= *next_id {
            *next_id = pet_id + 1;
        }
    }
}

impl Default for PetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pet_capture() {
        let manager = PetManager::new();

        // 尝试捕获Poring（可能成功或失败）
        let player_uuid = Uuid::new_v4();
        let result = manager.capture_pet(100, player_uuid, 1001);
        // 捕获可能成功或失败（随机），但不应该返回InvalidMobId
        assert!(result.is_ok() || matches!(result, Err(PetError::CaptureFailed)));
    }

    #[test]
    fn test_pet_capture_invalid_mob() {
        let manager = PetManager::new();

        let result = manager.capture_pet(100, Uuid::new_v4(), 9999);
        assert!(matches!(result, Err(PetError::PetDataNotFound(9999))));
    }

    #[test]
    fn test_summon_dismiss_pet() {
        let manager = PetManager::new();

        let player_uuid = Uuid::new_v4();

        // 尝试捕获直到成功
        let pet = loop {
            match manager.capture_pet(100, player_uuid, 1001) {
                Ok(pet) => break pet,
                Err(PetError::CaptureFailed) => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        };

        // 召唤宠物
        let result = manager.summon_pet(player_uuid, pet.pet_id);
        assert!(result.is_ok());

        // 检查召唤的宠物
        let summoned = manager.get_summoned_pet(player_uuid);
        assert!(summoned.is_some());
        assert_eq!(summoned.unwrap().pet_id, pet.pet_id);

        // 解散宠物
        manager.dismiss_pet(player_uuid);

        // 检查宠物是否已解散
        let summoned = manager.get_summoned_pet(player_uuid);
        assert!(summoned.is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = PetManager::new();

        let player_uuid = Uuid::new_v4();
        // 尝试捕获直到成功
        let pet = loop {
            match manager.capture_pet(100, player_uuid, 1001) {
                Ok(pet) => break pet,
                Err(PetError::CaptureFailed) => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        };

        // 第一次召唤
        manager.summon_pet(player_uuid, pet.pet_id).unwrap();

        // 第二次召唤应该失败
        let result = manager.summon_pet(player_uuid, pet.pet_id);
        assert!(matches!(result, Err(PetError::AlreadyHasPet)));
    }

    #[test]
    fn test_feed_pet() {
        let manager = PetManager::new();

        let player_uuid = Uuid::new_v4();
        // 尝试捕获直到成功
        let pet = loop {
            match manager.capture_pet(100, player_uuid, 1001) {
                Ok(pet) => break pet,
                Err(PetError::CaptureFailed) => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        };

        manager.summon_pet(player_uuid, pet.pet_id).unwrap();

        // 喂食Jellopy (530)
        let result = manager.feed_pet(player_uuid, 530);
        assert!(result.is_ok());

        // 喂食无效物品应该失败
        let result = manager.feed_pet(player_uuid, 1201); // Dagger
        assert!(matches!(result, Err(PetError::NotPetFood)));
    }

    #[test]
    fn test_rename_pet() {
        let manager = PetManager::new();

        let player_uuid = Uuid::new_v4();
        // 尝试捕获直到成功
        let pet = loop {
            match manager.capture_pet(100, player_uuid, 1001) {
                Ok(pet) => break pet,
                Err(PetError::CaptureFailed) => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        };

        manager.summon_pet(player_uuid, pet.pet_id).unwrap();

        // 第一次重命名应该成功
        let result = manager.rename_pet(player_uuid, "My Pet");
        assert!(result.is_ok());

        let pet = manager.get_summoned_pet(player_uuid).unwrap();
        assert_eq!(pet.name, "My Pet");
        assert!(pet.renamed);

        // 第二次重命名应该失败
        let result = manager.rename_pet(player_uuid, "Another Name");
        assert!(matches!(result, Err(PetError::CannotRename)));
    }
}
