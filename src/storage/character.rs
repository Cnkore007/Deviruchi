use crate::error::Result;
use crate::game::status::effect::StatusEffect;
use crate::game::status::effect::StatusSource;
use crate::game::status::types::StatusChange;
use crate::game::storage::data::Storage;
use crate::protocol::map_packets::CharInfo;
use crate::storage::Database;
use crate::storage::chrono_now;
use rusqlite::params;

/// 角色状态效果数据（用于数据库存储）
#[derive(Debug, Clone)]
pub struct CharacterStatusData {
    pub status_type: u32,
    pub val1: i32,
    pub val2: i32,
    pub val3: i32,
    pub stack: u8,
    pub duration_ms: u64,
    pub started_at_ms: u64,
    pub source_type: u8,
    pub source_id: u16,
}

/// 角色物品栏数据（用于数据库存储）
#[derive(Debug, Clone)]
pub struct CharacterInventoryData {
    pub index: u8,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

/// 角色快捷键数据（用于数据库存储）
#[derive(Debug, Clone)]
pub struct CharacterHotkeyData {
    pub hotkey_id: u8,
    pub type_: u8,
    pub item_or_skill_id: u32,
}

impl Database {
    pub fn create_character(
        &self,
        account_id: u32,
        slot: u8,
        name: &str,
        str: u8,
        agi: u8,
        vit: u8,
        int: u8,
        dex: u8,
        luk: u8,
        hair: u16,
        hair_color: u16,
    ) -> Result<u32> {
        let now = chrono_now();
        self.execute_with_params(
            "INSERT INTO characters
             (account_id, char_num, name, str, agi, vit, int, dex, luk,
              hair, hair_color, base_level, job_level, hp, max_hp, sp, max_sp,
              last_map, last_x, last_y, save_map, save_x, save_y,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                     1, 1, 40, 40, 11, 11,
                     'new_1-1.gat', 53, 111, 'new_1-1.gat', 53, 111,
                     ?12, ?12)",
            params![
                account_id, slot, name, str, agi, vit, int, dex, luk, hair, hair_color, now
            ],
        )?;
        Ok(self.last_insert_rowid()? as u32)
    }

    pub fn get_characters_by_account(&self, account_id: u32) -> Result<Vec<Character>> {
        self.query(
            "SELECT char_id, char_num, name, class, base_level, job_level,
                    base_exp, job_exp, zeny, str, agi, vit, int, dex, luk,
                    hp, max_hp, sp, max_sp,
                    hair, hair_color, clothes_color,
                    weapon, shield, head_top, head_mid, head_bottom,
                    last_map, last_x, last_y, save_map, save_x, save_y,
                    delete_timer, created_at, updated_at
             FROM characters WHERE account_id = ?1
             ORDER BY char_num",
            params![account_id],
            |row| {
                Ok(Character {
                    char_id: row.get(0)?,
                    char_num: row.get(1)?,
                    name: row.get(2)?,
                    class: row.get(3)?,
                    base_level: row.get(4)?,
                    job_level: row.get(5)?,
                    base_exp: row.get(6)?,
                    job_exp: row.get(7)?,
                    zeny: row.get(8)?,
                    str: row.get(9)?,
                    agi: row.get(10)?,
                    vit: row.get(11)?,
                    int: row.get(12)?,
                    dex: row.get(13)?,
                    luk: row.get(14)?,
                    hp: row.get(15)?,
                    max_hp: row.get(16)?,
                    sp: row.get(17)?,
                    max_sp: row.get(18)?,
                    hair: row.get(19)?,
                    hair_color: row.get(20)?,
                    clothes_color: row.get(21)?,
                    weapon: row.get(22)?,
                    shield: row.get(23)?,
                    head_top: row.get(24)?,
                    head_mid: row.get(25)?,
                    head_bottom: row.get(26)?,
                    last_map: row.get(27)?,
                    last_x: row.get(28)?,
                    last_y: row.get(29)?,
                    save_map: row.get(30)?,
                    save_x: row.get(31)?,
                    save_y: row.get(32)?,
                    delete_timer: row.get(33)?,
                    created_at: row.get(34)?,
                    updated_at: row.get(35)?,
                })
            },
        )
    }

    pub fn get_character_by_id(&self, char_id: u32) -> Result<Option<Character>> {
        self.query_row_optional(
            "SELECT char_id, char_num, name, class, base_level, job_level,
                    base_exp, job_exp, zeny, str, agi, vit, int, dex, luk,
                    hp, max_hp, sp, max_sp,
                    hair, hair_color, clothes_color,
                    weapon, shield, head_top, head_mid, head_bottom,
                    last_map, last_x, last_y, save_map, save_x, save_y,
                    delete_timer, created_at, updated_at
             FROM characters WHERE char_id = ?1",
            params![char_id],
            |row| {
                Ok(Character {
                    char_id: row.get(0)?,
                    char_num: row.get(1)?,
                    name: row.get(2)?,
                    class: row.get(3)?,
                    base_level: row.get(4)?,
                    job_level: row.get(5)?,
                    base_exp: row.get(6)?,
                    job_exp: row.get(7)?,
                    zeny: row.get(8)?,
                    str: row.get(9)?,
                    agi: row.get(10)?,
                    vit: row.get(11)?,
                    int: row.get(12)?,
                    dex: row.get(13)?,
                    luk: row.get(14)?,
                    hp: row.get(15)?,
                    max_hp: row.get(16)?,
                    sp: row.get(17)?,
                    max_sp: row.get(18)?,
                    hair: row.get(19)?,
                    hair_color: row.get(20)?,
                    clothes_color: row.get(21)?,
                    weapon: row.get(22)?,
                    shield: row.get(23)?,
                    head_top: row.get(24)?,
                    head_mid: row.get(25)?,
                    head_bottom: row.get(26)?,
                    last_map: row.get(27)?,
                    last_x: row.get(28)?,
                    last_y: row.get(29)?,
                    save_map: row.get(30)?,
                    save_x: row.get(31)?,
                    save_y: row.get(32)?,
                    delete_timer: row.get(33)?,
                    created_at: row.get(34)?,
                    updated_at: row.get(35)?,
                })
            },
        )
    }

    pub fn character_to_char_info(&self, char: &Character) -> CharInfo {
        CharInfo {
            char_id: char.char_id,
            exp: char.base_exp,
            gold: char.zeny,
            job_exp: char.job_exp,
            job_level: char.job_level,
            body_state: 0,
            health_state: 0,
            effect_state: 0,
            virtue: 0,
            honor: 0,
            job: char.class,
            hair: char.hair,
            hair_color: char.hair_color,
            clothes_color: char.clothes_color,
            body: 0,
            weapon: char.weapon,
            head_bottom: char.head_bottom,
            shield: char.shield,
            head_top: char.head_top,
            head_mid: char.head_mid,
            hair_color2: 0,
            clothes_color2: 0,
            name: char.name.clone(),
            base_level: char.base_level,
            str: char.str,
            agi: char.agi,
            vit: char.vit,
            int: char.int,
            dex: char.dex,
            luk: char.luk,
            slot: char.char_num,
            delete_timer: char.delete_timer,
            rename: 0,
            map_name: char.last_map.clone(),
        }
    }

    /// 加载角色仓库
    pub fn load_storage(&self, char_id: u32, max_size: u16) -> Result<Storage> {
        let items: Vec<(u16, u16, u16, bool, u8, [u16; 4])> = self.query(
            "SELECT slot_index, item_id, amount, identified, refine,
                    card0, card1, card2, card3
             FROM storage WHERE char_id = ?1 ORDER BY slot_index",
            params![char_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as u16,
                    row.get::<_, i64>(1)? as u16,
                    row.get::<_, i64>(2)? as u16,
                    row.get::<_, i64>(3)? != 0,
                    row.get::<_, i64>(4)? as u8,
                    [
                        row.get::<_, i64>(5)? as u16,
                        row.get::<_, i64>(6)? as u16,
                        row.get::<_, i64>(7)? as u16,
                        row.get::<_, i64>(8)? as u16,
                    ],
                ))
            },
        )?;

        Ok(Storage::from_db_format(char_id, max_size, items))
    }

    /// 保存角色仓库（事务包裹）
    pub fn save_storage(&self, storage: &Storage) -> Result<()> {
        let char_id = storage.char_id();
        let slots: Vec<_> = storage.slots().iter().filter(|s| !s.is_empty()).cloned().collect();

        self.with_transaction(|conn| {
            conn.execute(
                "DELETE FROM storage WHERE char_id = ?1",
                params![char_id],
            )?;

            for slot in &slots {
                conn.execute(
                    "INSERT INTO storage (char_id, slot_index, item_id, amount, identified, refine,
                                         card0, card1, card2, card3)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        char_id as i64,
                        slot.index as i64,
                        slot.item_id as i64,
                        slot.amount as i64,
                        slot.identified as i64,
                        slot.refine as i64,
                        slot.cards[0] as i64,
                        slot.cards[1] as i64,
                        slot.cards[2] as i64,
                        slot.cards[3] as i64,
                    ],
                )?;
            }

            Ok(())
        })
    }

    /// ==================== 玩家数据持久化 ====================

    /// 保存角色完整数据到数据库
    pub fn save_character(
        &self,
        char_id: u32,
        last_map: &str,
        last_x: i32,
        last_y: i32,
        hp: u32,
        max_hp: u32,
        sp: u32,
        max_sp: u32,
        base_exp: u64,
        job_exp: u64,
        base_level: u16,
        job_level: u16,
        zeny: u32,
        str: u16,
        agi: u16,
        vit: u16,
        int: u16,
        dex: u16,
        luk: u16,
        status_effects: &[StatusEffect],
        inventory: &[CharacterInventoryData],
        hotkeys: &[CharacterHotkeyData],
    ) -> Result<()> {
        let now = chrono_now();

        self.with_transaction(|conn| {
            // 1. 更新 characters 表基础数据
            conn.execute(
                "UPDATE characters SET
                    last_map = ?1, last_x = ?2, last_y = ?3,
                    hp = ?4, max_hp = ?5, sp = ?6, max_sp = ?7,
                    base_exp = ?8, job_exp = ?9,
                    base_level = ?10, job_level = ?11,
                    zeny = ?12,
                    str = ?13, agi = ?14, vit = ?15, int = ?16, dex = ?17, luk = ?18,
                    updated_at = ?19
                 WHERE char_id = ?20",
                params![
                    last_map, last_x, last_y, hp, max_hp, sp, max_sp,
                    base_exp as i64, job_exp as i64, base_level, job_level, zeny,
                    str, agi, vit, int, dex, luk, now, char_id
                ],
            )?;

            // 2. 保存状态效果
            conn.execute(
                "DELETE FROM character_status WHERE char_id = ?1",
                params![char_id],
            )?;
            for effect in status_effects {
                let source_type = match effect.source {
                    StatusSource::Skill(_) => 0u8,
                    StatusSource::Item(_) => 1,
                    StatusSource::Passive => 2,
                    StatusSource::Quest => 3,
                    StatusSource::Environment => 4,
                };
                let source_id = match effect.source {
                    StatusSource::Skill(id) => id,
                    StatusSource::Item(id) => id,
                    _ => 0,
                };
                conn.execute(
                    "INSERT INTO character_status
                        (char_id, status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        char_id, effect.id as u32, effect.val1, effect.val2, effect.val3,
                        effect.stack, effect.duration_ms as i64,
                        effect.started_at.elapsed().as_millis() as i64,
                        source_type, source_id
                    ],
                )?;
            }

            // 3. 保存物品栏
            conn.execute(
                "DELETE FROM character_inventory WHERE char_id = ?1",
                params![char_id],
            )?;
            for item in inventory {
                conn.execute(
                    "INSERT INTO character_inventory
                        (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        char_id, item.index, item.item_id, item.amount,
                        item.identified as i32, item.refine,
                        item.cards[0], item.cards[1], item.cards[2], item.cards[3]
                    ],
                )?;
            }

            // 4. 保存快捷键
            conn.execute(
                "DELETE FROM character_hotkeys WHERE char_id = ?1",
                params![char_id],
            )?;
            for hotkey in hotkeys {
                conn.execute(
                    "INSERT INTO character_hotkeys (char_id, hotkey_id, type, item_or_skill_id)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![char_id, hotkey.hotkey_id, hotkey.type_, hotkey.item_or_skill_id],
                )?;
            }

            Ok(())
        })?;

        tracing::debug!("Saved character {} data successfully", char_id);
        Ok(())
    }

    /// 保存角色状态效果
    pub fn save_character_status(&self, char_id: u32, statuses: &[StatusEffect]) -> Result<()> {
        // 先删除该角色的所有状态效果
        self.execute_with_params(
            "DELETE FROM character_status WHERE char_id = ?1",
            params![char_id],
        )?;

        // 插入当前状态效果
        for effect in statuses {
            let source_type = match effect.source {
                StatusSource::Skill(_) => 0u8,
                StatusSource::Item(_) => 1,
                StatusSource::Passive => 2,
                StatusSource::Quest => 3,
                StatusSource::Environment => 4,
            };

            let source_id = match effect.source {
                StatusSource::Skill(id) => id,
                StatusSource::Item(id) => id,
                _ => 0,
            };

            self.execute_with_params(
                "INSERT INTO character_status
                    (char_id, status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    char_id,
                    effect.id as u32,
                    effect.val1,
                    effect.val2,
                    effect.val3,
                    effect.stack,
                    effect.duration_ms as i64,
                    effect.started_at.elapsed().as_millis() as i64,
                    source_type,
                    source_id
                ],
            )?;
        }

        Ok(())
    }

    /// 加载角色状态效果
    pub fn load_character_status(&self, char_id: u32) -> Result<Vec<StatusEffect>> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        self.query(
            "SELECT status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id
             FROM character_status WHERE char_id = ?1",
            params![char_id],
            |row| {
                let status_type: u32 = row.get(0)?;
                let val1: i32 = row.get(1)?;
                let val2: i32 = row.get(2)?;
                let val3: i32 = row.get(3)?;
                let stack: u8 = row.get(4)?;
                let duration_ms: u64 = row.get::<_, i64>(5)? as u64;
                let started_at_ms: u64 = row.get::<_, i64>(6)? as u64;
                let source_type: u8 = row.get(7)?;
                let source_id: u16 = row.get(8)?;

                let source = match source_type {
                    0 => StatusSource::Skill(source_id),
                    1 => StatusSource::Item(source_id),
                    2 => StatusSource::Passive,
                    3 => StatusSource::Quest,
                    4 => StatusSource::Environment,
                    _ => StatusSource::Environment,
                };

                // 计算剩余时间：started_at_ms + duration_ms - now_ms
                let end_time_ms = started_at_ms.saturating_add(duration_ms);
                let remaining_ms = end_time_ms.saturating_sub(now_ms);

                // 如果状态已过期则跳过
                if remaining_ms == 0 {
                    return Ok(StatusEffect {
                        id: StatusChange::from_u32(status_type),
                        duration_ms: 0,
                        started_at: std::time::Instant::now(),
                        source,
                        val1,
                        val2,
                        val3,
                        stack,
                    });
                }

                // 使用剩余时间作为新的 duration，started_at 为当前时间
                Ok(StatusEffect {
                    id: StatusChange::from_u32(status_type),
                    duration_ms: remaining_ms,
                    started_at: std::time::Instant::now(),
                    source,
                    val1,
                    val2,
                    val3,
                    stack,
                })
            },
        )
    }

    /// 保存角色物品栏
    pub fn save_character_inventory(
        &self,
        char_id: u32,
        items: &[CharacterInventoryData],
    ) -> Result<()> {
        // 先删除该角色的所有物品栏数据
        self.execute_with_params(
            "DELETE FROM character_inventory WHERE char_id = ?1",
            params![char_id],
        )?;

        // 插入当前物品
        for item in items {
            self.execute_with_params(
                "INSERT INTO character_inventory
                    (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    char_id,
                    item.index,
                    item.item_id,
                    item.amount,
                    item.identified as i32,
                    item.refine,
                    item.cards[0],
                    item.cards[1],
                    item.cards[2],
                    item.cards[3]
                ],
            )?;
        }

        Ok(())
    }

    /// 加载角色物品栏
    pub fn load_character_inventory(&self, char_id: u32) -> Result<Vec<CharacterInventoryData>> {
        self.query(
            "SELECT slot_index, item_id, amount, identified, refine, card0, card1, card2, card3
             FROM character_inventory WHERE char_id = ?1 ORDER BY slot_index",
            params![char_id],
            |row| {
                Ok(CharacterInventoryData {
                    index: row.get(0)?,
                    item_id: row.get(1)?,
                    amount: row.get(2)?,
                    identified: row.get::<_, i32>(3)? != 0,
                    refine: row.get(4)?,
                    cards: [row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?],
                })
            },
        )
    }

    /// 保存角色快捷键
    pub fn save_character_hotkeys(
        &self,
        char_id: u32,
        hotkeys: &[CharacterHotkeyData],
    ) -> Result<()> {
        // 先删除该角色的所有快捷键
        self.execute_with_params(
            "DELETE FROM character_hotkeys WHERE char_id = ?1",
            params![char_id],
        )?;

        // 插入当前快捷键
        for hotkey in hotkeys {
            self.execute_with_params(
                "INSERT INTO character_hotkeys (char_id, hotkey_id, type, item_or_skill_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    char_id,
                    hotkey.hotkey_id,
                    hotkey.type_,
                    hotkey.item_or_skill_id
                ],
            )?;
        }

        Ok(())
    }

    /// 清理已过期的删除定时器角色（delete_timer > 0 且已过期）
    pub fn cleanup_deleted_characters(&self) -> Result<usize> {
        let now = chrono_now() as u32;
        let deleted = self.execute_with_params(
            "DELETE FROM characters WHERE delete_timer > 0 AND delete_timer <= ?1",
            params![now],
        )?;
        if deleted > 0 {
            tracing::info!("Cleaned up {} expired deleted characters", deleted);
        }
        Ok(deleted)
    }

    /// 加载角色快捷键
    pub fn load_character_hotkeys(&self, char_id: u32) -> Result<Vec<CharacterHotkeyData>> {
        self.query(
            "SELECT hotkey_id, type, item_or_skill_id
             FROM character_hotkeys WHERE char_id = ?1 ORDER BY hotkey_id",
            params![char_id],
            |row| {
                Ok(CharacterHotkeyData {
                    hotkey_id: row.get(0)?,
                    type_: row.get(1)?,
                    item_or_skill_id: row.get(2)?,
                })
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct Character {
    pub char_id: u32,
    pub char_num: u8,
    pub name: String,
    pub class: u16,
    pub base_level: u16,
    pub job_level: u16,
    pub base_exp: u32,
    pub job_exp: u32,
    pub zeny: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub hair: u16,
    pub hair_color: u16,
    pub clothes_color: u16,
    pub weapon: u16,
    pub shield: u16,
    pub head_top: u16,
    pub head_mid: u16,
    pub head_bottom: u16,
    pub last_map: String,
    pub last_x: i32,
    pub last_y: i32,
    pub save_map: String,
    pub save_x: i32,
    pub save_y: i32,
    pub delete_timer: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_account(db: &crate::storage::Database) -> u32 {
        // 创建测试账户
        db.execute_with_params(
            "INSERT INTO accounts (account_id, user_id, password_hash, sex, created_at)
             VALUES (1, 'test_user', 'hash', 0, 1000)",
            [],
        )
        .expect("Failed to create test account");
        1
    }

    #[test]
    fn test_save_and_load_character_status() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");

        // 初始化 schema
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        // 创建测试账户
        let account_id = setup_test_account(&db);

        // 创建测试角色
        let char_id = db
            .create_character(account_id, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 创建测试状态效果
        let status_effects = vec![
            StatusEffect::with_values(
                StatusChange::Blessing,
                60000,
                StatusSource::Skill(9),
                10,
                0,
                0,
            ),
            StatusEffect::with_values(
                StatusChange::Haste,
                30000,
                StatusSource::Skill(29),
                30,
                0,
                0,
            ),
        ];

        // 保存状态效果
        db.save_character_status(char_id, &status_effects)
            .expect("Failed to save status");

        // 加载状态效果
        let loaded = db
            .load_character_status(char_id)
            .expect("Failed to load status");

        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().any(|s| s.id == StatusChange::Blessing));
        assert!(loaded.iter().any(|s| s.id == StatusChange::Haste));
    }

    #[test]
    fn test_save_and_load_character_inventory() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");

        // 初始化 schema
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        // 创建测试账户
        let account_id = setup_test_account(&db);

        // 创建测试角色
        let char_id = db
            .create_character(account_id, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 创建测试物品
        let inventory = vec![
            CharacterInventoryData {
                index: 0,
                item_id: 501, // Red Potion
                amount: 10,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            CharacterInventoryData {
                index: 1,
                item_id: 502, // Blue Potion
                amount: 5,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
        ];

        // 保存物品栏
        db.save_character_inventory(char_id, &inventory)
            .expect("Failed to save inventory");

        // 加载物品栏
        let loaded = db
            .load_character_inventory(char_id)
            .expect("Failed to load inventory");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].item_id, 501);
        assert_eq!(loaded[0].amount, 10);
        assert_eq!(loaded[1].item_id, 502);
    }

    #[test]
    fn test_save_and_load_character_hotkeys() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");

        // 初始化 schema
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        // 创建测试账户
        let account_id = setup_test_account(&db);

        // 创建测试角色
        let char_id = db
            .create_character(account_id, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 创建测试快捷键
        let hotkeys = vec![
            CharacterHotkeyData {
                hotkey_id: 0,
                type_: 0,              // Item
                item_or_skill_id: 501, // Red Potion
            },
            CharacterHotkeyData {
                hotkey_id: 1,
                type_: 1,             // Skill
                item_or_skill_id: 28, // Heal
            },
        ];

        // 保存快捷键
        db.save_character_hotkeys(char_id, &hotkeys)
            .expect("Failed to save hotkeys");

        // 加载快捷键
        let loaded = db
            .load_character_hotkeys(char_id)
            .expect("Failed to load hotkeys");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].type_, 0);
        assert_eq!(loaded[1].type_, 1);
    }

    #[test]
    fn test_save_character_updates_all_fields() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");

        // 初始化 schema
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        // 创建测试账户
        let account_id = setup_test_account(&db);

        // 创建测试角色
        let char_id = db
            .create_character(account_id, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 准备测试数据
        let status_effects = vec![StatusEffect::new(
            StatusChange::Blessing,
            60000,
            StatusSource::Skill(9),
        )];
        let inventory = vec![CharacterInventoryData {
            index: 0,
            item_id: 501,
            amount: 10,
            identified: true,
            refine: 0,
            cards: [0; 4],
        }];
        let hotkeys = vec![CharacterHotkeyData {
            hotkey_id: 0,
            type_: 0,
            item_or_skill_id: 501,
        }];

        // 保存完整角色数据
        db.save_character(
            char_id,
            "prontera.gat",
            100,
            200,
            500,
            1000,
            100,
            200,
            10000,
            5000,
            50,
            30,
            100000,
            50,
            50,
            50,
            50,
            50,
            50,
            &status_effects,
            &inventory,
            &hotkeys,
        )
        .expect("Failed to save character");

        // 验证角色数据已更新
        let char = db
            .get_character_by_id(char_id)
            .expect("Failed to get character")
            .expect("Character not found");

        assert_eq!(char.last_map, "prontera.gat");
        assert_eq!(char.last_x, 100);
        assert_eq!(char.last_y, 200);
        assert_eq!(char.hp, 500);
        assert_eq!(char.max_hp, 1000);
        assert_eq!(char.zeny, 100000);
    }

    #[test]
    fn test_cleanup_deleted_characters() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        let account_id = setup_test_account(&db);

        // 创建一个正常角色
        let char1_id = db
            .create_character(account_id, 0, "Normal", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 创建一个已标记删除且定时器已过期的角色
        let char2_id = db
            .create_character(account_id, 1, "Expired", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");
        let past_time = (chrono_now() - 3600) as u32; // 1 小时前
        db.execute_with_params(
            "UPDATE characters SET delete_timer = ?1 WHERE char_id = ?2",
            params![past_time, char2_id],
        )
        .expect("Failed to set delete_timer");

        // 创建一个已标记删除但定时器未过期的角色
        let char3_id = db
            .create_character(account_id, 2, "Pending", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");
        let future_time = (chrono_now() + 3600) as u32; // 1 小时后
        db.execute_with_params(
            "UPDATE characters SET delete_timer = ?1 WHERE char_id = ?2",
            params![future_time, char3_id],
        )
        .expect("Failed to set delete_timer");

        // 清理已过期的角色
        let deleted = db
            .cleanup_deleted_characters()
            .expect("Failed to cleanup");
        assert_eq!(deleted, 1, "Should have deleted 1 expired character");

        // 验证：正常角色仍在
        assert!(db
            .get_character_by_id(char1_id)
            .expect("query failed")
            .is_some());
        // 验证：已过期角色被删除
        assert!(db
            .get_character_by_id(char2_id)
            .expect("query failed")
            .is_none());
        // 验证：待删除但未过期角色仍在
        assert!(db
            .get_character_by_id(char3_id)
            .expect("query failed")
            .is_some());
    }
}
