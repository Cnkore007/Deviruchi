use crate::error::Result;
use crate::game::status::effect::StatusEffect;
use crate::game::status::effect::StatusSource;
use crate::game::status::types::StatusChange;
use crate::game::storage::data::Storage;
use crate::protocol::map_packets::CharInfo;
use crate::storage::Database;
use crate::storage::backend::IntoValue;
use crate::storage::chrono_now;

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

/// 辅助函数：构建 Character 结构体的参数数组
fn character_row_params() -> Vec<&'static dyn IntoValue> {
    vec![]
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
        self.execute_params(
            "INSERT INTO characters
             (account_id, char_num, name, str, agi, vit, int, dex, luk,
              hair, hair_color, base_level, job_level, hp, max_hp, sp, max_sp,
              last_map, last_x, last_y, save_map, save_x, save_y,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                     1, 1, 40, 40, 11, 11,
                     'new_1-1.gat', 53, 111, 'new_1-1.gat', 53, 111,
                     ?12, ?12)",
            &[
                &(account_id as i32) as &dyn IntoValue,
                &(slot as i32) as &dyn IntoValue,
                &name as &dyn IntoValue,
                &(str as i32) as &dyn IntoValue,
                &(agi as i32) as &dyn IntoValue,
                &(vit as i32) as &dyn IntoValue,
                &(int as i32) as &dyn IntoValue,
                &(dex as i32) as &dyn IntoValue,
                &(luk as i32) as &dyn IntoValue,
                &(hair as i32) as &dyn IntoValue,
                &(hair_color as i32) as &dyn IntoValue,
                &now as &dyn IntoValue,
            ],
        )?;
        Ok(self.last_insert_rowid() as u32)
    }

    /// 从 Row 构建 Character 结构体（减少重复代码）
    fn character_from_row(row: &crate::storage::backend::Row) -> Result<Character> {
        Ok(Character {
            char_id: row.get_i32(0)? as u32,
            char_num: row.get_i32(1)? as u8,
            name: row.get_string(2)?,
            class: row.get_i32(3)? as u16,
            base_level: row.get_i32(4)? as u16,
            job_level: row.get_i32(5)? as u16,
            base_exp: row.get_i32(6)? as u32,
            job_exp: row.get_i32(7)? as u32,
            zeny: row.get_i32(8)? as u32,
            str: row.get_i32(9)? as u16,
            agi: row.get_i32(10)? as u16,
            vit: row.get_i32(11)? as u16,
            int: row.get_i32(12)? as u16,
            dex: row.get_i32(13)? as u16,
            luk: row.get_i32(14)? as u16,
            hp: row.get_i32(15)? as u32,
            max_hp: row.get_i32(16)? as u32,
            sp: row.get_i32(17)? as u32,
            max_sp: row.get_i32(18)? as u32,
            hair: row.get_i32(19)? as u16,
            hair_color: row.get_i32(20)? as u16,
            clothes_color: row.get_i32(21)? as u16,
            weapon: row.get_i32(22)? as u16,
            shield: row.get_i32(23)? as u16,
            head_top: row.get_i32(24)? as u16,
            head_mid: row.get_i32(25)? as u16,
            head_bottom: row.get_i32(26)? as u16,
            last_map: row.get_string(27)?,
            last_x: row.get_i32(28)?,
            last_y: row.get_i32(29)?,
            save_map: row.get_string(30)?,
            save_x: row.get_i32(31)?,
            save_y: row.get_i32(32)?,
            delete_timer: row.get_i32(33)? as u32,
            status_point: row.get_i32(34)? as u16,
            skill_point: row.get_i32(35)? as u16,
            created_at: row.get_i64(36)?,
            updated_at: row.get_i64(37)?,
        })
    }

    /// 角色列表查询 SQL（复用）
    const CHARACTER_SELECT_SQL: &'static str =
        "SELECT char_id, char_num, name, class, base_level, job_level,
                base_exp, job_exp, zeny, str, agi, vit, int, dex, luk,
                hp, max_hp, sp, max_sp,
                hair, hair_color, clothes_color,
                weapon, shield, head_top, head_mid, head_bottom,
                last_map, last_x, last_y, save_map, save_x, save_y,
                delete_timer, status_point, skill_point, created_at, updated_at
         FROM characters";

    pub fn get_characters_by_account(&self, account_id: u32) -> Result<Vec<Character>> {
        let sql = format!("{} WHERE account_id = ?1 ORDER BY char_num", Self::CHARACTER_SELECT_SQL);
        let rows = self.query_rows(
            &sql,
            &[&(account_id as i32) as &dyn IntoValue],
        )?;
        let mut characters = Vec::new();
        for row in &rows {
            characters.push(Self::character_from_row(row)?);
        }
        Ok(characters)
    }

    pub fn get_character_by_id(&self, char_id: u32) -> Result<Option<Character>> {
        let sql = format!("{} WHERE char_id = ?1", Self::CHARACTER_SELECT_SQL);
        self.query_row_optional(
            &sql,
            &[&(char_id as i32) as &dyn IntoValue],
            |row| Self::character_from_row(row),
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
        let rows = self.query_rows(
            "SELECT slot_index, item_id, amount, identified, refine,
                    card0, card1, card2, card3
             FROM storage WHERE char_id = ?1 ORDER BY slot_index",
            &[&(char_id as i64) as &dyn IntoValue],
        )?;

        let mut items = Vec::new();
        for row in &rows {
            items.push((
                row.get_i64(0)? as u16,
                row.get_i64(1)? as u16,
                row.get_i64(2)? as u16,
                row.get_i64(3)? != 0,
                row.get_i64(4)? as u8,
                [
                    row.get_i64(5)? as u16,
                    row.get_i64(6)? as u16,
                    row.get_i64(7)? as u16,
                    row.get_i64(8)? as u16,
                ],
            ));
        }

        Ok(Storage::from_db_format(char_id, max_size, items))
    }

    /// 保存角色仓库（事务包裹）
    pub fn save_storage(&self, storage: &Storage) -> Result<()> {
        let char_id = storage.char_id();
        let slots: Vec<_> = storage.slots().iter().filter(|s| !s.is_empty()).cloned().collect();

        self.with_transaction(|tx| {
            tx.execute_params(
                "DELETE FROM storage WHERE char_id = ?1",
                &[&(char_id as i64) as &dyn IntoValue],
            )?;

            for slot in &slots {
                tx.execute_params(
                    "INSERT INTO storage (char_id, slot_index, item_id, amount, identified, refine,
                                         card0, card1, card2, card3)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    &[
                        &(char_id as i64) as &dyn IntoValue,
                        &(slot.index as i64) as &dyn IntoValue,
                        &(slot.item_id as i64) as &dyn IntoValue,
                        &(slot.amount as i64) as &dyn IntoValue,
                        &(slot.identified as i64) as &dyn IntoValue,
                        &(slot.refine as i64) as &dyn IntoValue,
                        &(slot.cards[0] as i64) as &dyn IntoValue,
                        &(slot.cards[1] as i64) as &dyn IntoValue,
                        &(slot.cards[2] as i64) as &dyn IntoValue,
                        &(slot.cards[3] as i64) as &dyn IntoValue,
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
        class: u16,
        zeny: u32,
        str: u16,
        agi: u16,
        vit: u16,
        int: u16,
        dex: u16,
        luk: u16,
        status_point: u16,
        skill_point: u16,
        status_effects: &[StatusEffect],
        inventory: &[CharacterInventoryData],
        hotkeys: &[CharacterHotkeyData],
    ) -> Result<()> {
        let now = chrono_now();

        self.with_transaction(|tx| {
            // 1. 更新 characters 表基础数据（包含 class/职业 字段）
            tx.execute_params(
                "UPDATE characters SET
                    last_map = ?1, last_x = ?2, last_y = ?3,
                    hp = ?4, max_hp = ?5, sp = ?6, max_sp = ?7,
                    base_exp = ?8, job_exp = ?9,
                    base_level = ?10, job_level = ?11,
                    class = ?12,
                    zeny = ?13,
                    str = ?14, agi = ?15, vit = ?16, int = ?17, dex = ?18, luk = ?19,
                    status_point = ?20, skill_point = ?21,
                    updated_at = ?22
                 WHERE char_id = ?23",
                &[
                    &last_map as &dyn IntoValue,
                    &last_x as &dyn IntoValue,
                    &last_y as &dyn IntoValue,
                    &(hp as i32) as &dyn IntoValue,
                    &(max_hp as i32) as &dyn IntoValue,
                    &(sp as i32) as &dyn IntoValue,
                    &(max_sp as i32) as &dyn IntoValue,
                    &(base_exp as i64) as &dyn IntoValue,
                    &(job_exp as i64) as &dyn IntoValue,
                    &(base_level as i32) as &dyn IntoValue,
                    &(job_level as i32) as &dyn IntoValue,
                    &(class as i32) as &dyn IntoValue,
                    &(zeny as i32) as &dyn IntoValue,
                    &(str as i32) as &dyn IntoValue,
                    &(agi as i32) as &dyn IntoValue,
                    &(vit as i32) as &dyn IntoValue,
                    &(int as i32) as &dyn IntoValue,
                    &(dex as i32) as &dyn IntoValue,
                    &(luk as i32) as &dyn IntoValue,
                    &(status_point as i32) as &dyn IntoValue,
                    &(skill_point as i32) as &dyn IntoValue,
                    &now as &dyn IntoValue,
                    &(char_id as i32) as &dyn IntoValue,
                ],
            )?;

            // 2. 保存状态效果
            tx.execute_params(
                "DELETE FROM character_status WHERE char_id = ?1",
                &[&(char_id as i32) as &dyn IntoValue],
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
                tx.execute_params(
                    "INSERT INTO character_status
                        (char_id, status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    &[
                        &(char_id as i32) as &dyn IntoValue,
                        &(effect.id as u32) as &dyn IntoValue,
                        &effect.val1 as &dyn IntoValue,
                        &effect.val2 as &dyn IntoValue,
                        &effect.val3 as &dyn IntoValue,
                        &(effect.stack as i32) as &dyn IntoValue,
                        &(effect.duration_ms as i64) as &dyn IntoValue,
                        &(effect.started_at.elapsed().as_millis() as i64) as &dyn IntoValue,
                        &(source_type as i32) as &dyn IntoValue,
                        &(source_id as i32) as &dyn IntoValue,
                    ],
                )?;
            }

            // 3. 保存物品栏
            tx.execute_params(
                "DELETE FROM character_inventory WHERE char_id = ?1",
                &[&(char_id as i32) as &dyn IntoValue],
            )?;
            for item in inventory {
                tx.execute_params(
                    "INSERT INTO character_inventory
                        (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    &[
                        &(char_id as i32) as &dyn IntoValue,
                        &(item.index as i32) as &dyn IntoValue,
                        &(item.item_id as i32) as &dyn IntoValue,
                        &(item.amount as i32) as &dyn IntoValue,
                        &(item.identified as i32) as &dyn IntoValue,
                        &(item.refine as i32) as &dyn IntoValue,
                        &(item.cards[0] as i32) as &dyn IntoValue,
                        &(item.cards[1] as i32) as &dyn IntoValue,
                        &(item.cards[2] as i32) as &dyn IntoValue,
                        &(item.cards[3] as i32) as &dyn IntoValue,
                    ],
                )?;
            }

            // 4. 保存快捷键
            tx.execute_params(
                "DELETE FROM character_hotkeys WHERE char_id = ?1",
                &[&(char_id as i32) as &dyn IntoValue],
            )?;
            for hotkey in hotkeys {
                tx.execute_params(
                    "INSERT INTO character_hotkeys (char_id, hotkey_id, type, item_or_skill_id)
                     VALUES (?1, ?2, ?3, ?4)",
                    &[
                        &(char_id as i32) as &dyn IntoValue,
                        &(hotkey.hotkey_id as i32) as &dyn IntoValue,
                        &(hotkey.type_ as i32) as &dyn IntoValue,
                        &(hotkey.item_or_skill_id as i32) as &dyn IntoValue,
                    ],
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
        self.execute_params(
            "DELETE FROM character_status WHERE char_id = ?1",
            &[&(char_id as i32) as &dyn IntoValue],
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

            self.execute_params(
                "INSERT INTO character_status
                    (char_id, status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                &[
                    &(char_id as i32) as &dyn IntoValue,
                    &(effect.id as u32) as &dyn IntoValue,
                    &effect.val1 as &dyn IntoValue,
                    &effect.val2 as &dyn IntoValue,
                    &effect.val3 as &dyn IntoValue,
                    &(effect.stack as i32) as &dyn IntoValue,
                    &(effect.duration_ms as i64) as &dyn IntoValue,
                    &(effect.started_at.elapsed().as_millis() as i64) as &dyn IntoValue,
                    &(source_type as i32) as &dyn IntoValue,
                    &(source_id as i32) as &dyn IntoValue,
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

        let rows = self.query_rows(
            "SELECT status_type, val1, val2, val3, stack, duration_ms, started_at, source_type, source_id
             FROM character_status WHERE char_id = ?1",
            &[&(char_id as i32) as &dyn IntoValue],
        )?;

        let mut results = Vec::new();
        for row in &rows {
            let status_type: u32 = row.get_i32(0)? as u32;
            let val1: i32 = row.get_i32(1)?;
            let val2: i32 = row.get_i32(2)?;
            let val3: i32 = row.get_i32(3)?;
            let stack: u8 = row.get_i32(4)? as u8;
            let duration_ms: u64 = row.get_i64(5)? as u64;
            let started_at_ms: u64 = row.get_i64(6)? as u64;
            let source_type: u8 = row.get_i32(7)? as u8;
            let source_id: u16 = row.get_i32(8)? as u16;

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
                results.push(StatusEffect {
                    id: StatusChange::from_u32(status_type),
                    duration_ms: 0,
                    started_at: std::time::Instant::now(),
                    source,
                    val1,
                    val2,
                    val3,
                    stack,
                });
                continue;
            }

            // 使用剩余时间作为新的 duration，started_at 为当前时间
            results.push(StatusEffect {
                id: StatusChange::from_u32(status_type),
                duration_ms: remaining_ms,
                started_at: std::time::Instant::now(),
                source,
                val1,
                val2,
                val3,
                stack,
            });
        }

        Ok(results)
    }

    /// 保存角色物品栏
    pub fn save_character_inventory(
        &self,
        char_id: u32,
        items: &[CharacterInventoryData],
    ) -> Result<()> {
        // 先删除该角色的所有物品栏数据
        self.execute_params(
            "DELETE FROM character_inventory WHERE char_id = ?1",
            &[&(char_id as i32) as &dyn IntoValue],
        )?;

        // 插入当前物品
        for item in items {
            self.execute_params(
                "INSERT INTO character_inventory
                    (char_id, slot_index, item_id, amount, identified, refine, card0, card1, card2, card3)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                &[
                    &(char_id as i32) as &dyn IntoValue,
                    &(item.index as i32) as &dyn IntoValue,
                    &(item.item_id as i32) as &dyn IntoValue,
                    &(item.amount as i32) as &dyn IntoValue,
                    &(item.identified as i32) as &dyn IntoValue,
                    &(item.refine as i32) as &dyn IntoValue,
                    &(item.cards[0] as i32) as &dyn IntoValue,
                    &(item.cards[1] as i32) as &dyn IntoValue,
                    &(item.cards[2] as i32) as &dyn IntoValue,
                    &(item.cards[3] as i32) as &dyn IntoValue,
                ],
            )?;
        }

        Ok(())
    }

    /// 加载角色物品栏
    pub fn load_character_inventory(&self, char_id: u32) -> Result<Vec<CharacterInventoryData>> {
        let rows = self.query_rows(
            "SELECT slot_index, item_id, amount, identified, refine, card0, card1, card2, card3
             FROM character_inventory WHERE char_id = ?1 ORDER BY slot_index",
            &[&(char_id as i32) as &dyn IntoValue],
        )?;

        let mut items = Vec::new();
        for row in &rows {
            items.push(CharacterInventoryData {
                index: row.get_i32(0)? as u8,
                item_id: row.get_i32(1)? as u16,
                amount: row.get_i32(2)? as u16,
                identified: row.get_i32(3)? != 0,
                refine: row.get_i32(4)? as u8,
                cards: [
                    row.get_i32(5)? as u16,
                    row.get_i32(6)? as u16,
                    row.get_i32(7)? as u16,
                    row.get_i32(8)? as u16,
                ],
            });
        }

        Ok(items)
    }

    /// 保存角色快捷键
    pub fn save_character_hotkeys(
        &self,
        char_id: u32,
        hotkeys: &[CharacterHotkeyData],
    ) -> Result<()> {
        // 先删除该角色的所有快捷键
        self.execute_params(
            "DELETE FROM character_hotkeys WHERE char_id = ?1",
            &[&(char_id as i32) as &dyn IntoValue],
        )?;

        // 插入当前快捷键
        for hotkey in hotkeys {
            self.execute_params(
                "INSERT INTO character_hotkeys (char_id, hotkey_id, type, item_or_skill_id)
                 VALUES (?1, ?2, ?3, ?4)",
                &[
                    &(char_id as i32) as &dyn IntoValue,
                    &(hotkey.hotkey_id as i32) as &dyn IntoValue,
                    &(hotkey.type_ as i32) as &dyn IntoValue,
                    &(hotkey.item_or_skill_id as i32) as &dyn IntoValue,
                ],
            )?;
        }

        Ok(())
    }

    /// 清理已过期的删除定时器角色（delete_timer > 0 且已过期）
    pub fn cleanup_deleted_characters(&self) -> Result<usize> {
        let now = chrono_now() as u32;
        let deleted = self.execute_params(
            "DELETE FROM characters WHERE delete_timer > 0 AND delete_timer <= ?1",
            &[&(now as i32) as &dyn IntoValue],
        )?;
        if deleted > 0 {
            tracing::info!("Cleaned up {} expired deleted characters", deleted);
        }
        Ok(deleted)
    }

    /// 设置角色删除定时器（标记删除）
    /// delete_after_secs: 从现在起多少秒后删除（rAthena 默认 86400 = 24小时）
    pub fn mark_character_for_deletion(
        &self,
        char_id: u32,
        account_id: u32,
        delete_after_secs: i64,
    ) -> Result<bool> {
        let now = chrono_now();
        let delete_timer = (now + delete_after_secs) as u32;

        // 验证角色属于该账户
        let affected = self.execute_params(
            "UPDATE characters SET delete_timer = ?1
             WHERE char_id = ?2 AND account_id = ?3 AND (delete_timer = 0 OR delete_timer IS NULL)",
            &[
                &(delete_timer as i32) as &dyn IntoValue,
                &(char_id as i32) as &dyn IntoValue,
                &(account_id as i32) as &dyn IntoValue,
            ],
        )?;

        if affected > 0 {
            tracing::info!(
                "Character {} marked for deletion at {} (in {} seconds)",
                char_id, delete_timer, delete_after_secs
            );
            Ok(true)
        } else {
            tracing::warn!(
                "Failed to mark character {} for deletion (not found or already marked)",
                char_id
            );
            Ok(false)
        }
    }

    /// 取消角色删除标记
    pub fn cancel_character_deletion(
        &self,
        char_id: u32,
        account_id: u32,
    ) -> Result<bool> {
        let affected = self.execute_params(
            "UPDATE characters SET delete_timer = 0
             WHERE char_id = ?1 AND account_id = ?2 AND delete_timer > 0",
            &[
                &(char_id as i32) as &dyn IntoValue,
                &(account_id as i32) as &dyn IntoValue,
            ],
        )?;

        if affected > 0 {
            tracing::info!("Character {} deletion cancelled", char_id);
            Ok(true)
        } else {
            tracing::warn!(
                "Failed to cancel deletion for character {} (not found or not marked)",
                char_id
            );
            Ok(false)
        }
    }

    /// 按名称查找角色（用于名称重复检查）
    pub fn get_character_by_name(&self, name: &str) -> Result<Option<Character>> {
        let sql = format!("{} WHERE name = ?1", Self::CHARACTER_SELECT_SQL);
        self.query_row_optional(
            &sql,
            &[&name as &dyn IntoValue],
            |row| Self::character_from_row(row),
        )
    }

    /// 加载角色快捷键
    pub fn load_character_hotkeys(&self, char_id: u32) -> Result<Vec<CharacterHotkeyData>> {
        let rows = self.query_rows(
            "SELECT hotkey_id, type, item_or_skill_id
             FROM character_hotkeys WHERE char_id = ?1 ORDER BY hotkey_id",
            &[&(char_id as i32) as &dyn IntoValue],
        )?;

        let mut hotkeys = Vec::new();
        for row in &rows {
            hotkeys.push(CharacterHotkeyData {
                hotkey_id: row.get_i32(0)? as u8,
                type_: row.get_i32(1)? as u8,
                item_or_skill_id: row.get_i32(2)? as u32,
            });
        }

        Ok(hotkeys)
    }

    /// ==================== 技能管理 ====================

    /// 获取角色的指定技能等级（未学习返回 0）
    pub fn get_skill_level(&self, char_id: u32, skill_id: u16) -> Result<u8> {
        self.query_row_optional(
            "SELECT lv FROM skills WHERE char_id = ?1 AND skill_id = ?2",
            &[
                &(char_id as i32) as &dyn IntoValue,
                &(skill_id as i32) as &dyn IntoValue,
            ],
            |row| Ok(row.get_i32(0)? as u8),
        )
        .map(|opt| opt.unwrap_or(0))
    }

    /// 设置角色技能等级（不存在则插入，已存在则更新）
    pub fn set_skill_level(&self, char_id: u32, skill_id: u16, level: u8) -> Result<()> {
        self.execute_params(
            "INSERT INTO skills (char_id, skill_id, lv, flag) VALUES (?1, ?2, ?3, 0)
             ON CONFLICT(char_id, skill_id) DO UPDATE SET lv = ?3",
            &[
                &(char_id as i32) as &dyn IntoValue,
                &(skill_id as i32) as &dyn IntoValue,
                &(level as i32) as &dyn IntoValue,
            ],
        )?;
        Ok(())
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
    /// 可分配的状态点数（升级获得）
    pub status_point: u16,
    /// 可分配的技能点数（升级获得）
    pub skill_point: u16,
    pub created_at: i64,
    pub updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_account(db: &crate::storage::Database) -> u32 {
        // 创建测试账户
        db.execute_params(
            "INSERT INTO accounts (account_id, user_id, password_hash, sex, created_at)
             VALUES (1, 'test_user', 'hash', 0, 1000)",
            &[],
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
            7, // class: Knight
            100000,
            50,
            50,
            50,
            50,
            50,
            50,
            48,
            7,
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
        assert_eq!(char.class, 7); // Knight
        assert_eq!(char.zeny, 100000);
    }

    #[test]
    fn test_cleanup_deleted_characters() {
        let db =
            crate::storage::Database::open_memory().expect("Failed to create in-memory database");
        crate::storage::schema::init_schema(&db).expect("Failed to init schema");

        let account_id = setup_test_account(&db);

        // 创建一个正常角色
        let _char1_id = db
            .create_character(account_id, 0, "Normal", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");

        // 创建一个已标记删除且定时器已过期的角色
        let char2_id = db
            .create_character(account_id, 1, "Expired", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");
        let past_time = (chrono_now() - 3600) as u32; // 1 小时前
        db.execute_params(
            "UPDATE characters SET delete_timer = ?1 WHERE char_id = ?2",
            &[
                &(past_time as i32) as &dyn IntoValue,
                &(char2_id as i32) as &dyn IntoValue,
            ],
        )
        .expect("Failed to set delete_timer");

        // 创建一个已标记删除但定时器未过期的角色
        let _char3_id = db
            .create_character(account_id, 2, "Pending", 10, 10, 10, 10, 10, 10, 1, 0)
            .expect("Failed to create character");
        let future_time = (chrono_now() + 3600) as u32; // 1 小时后
        db.execute_params(
            "UPDATE characters SET delete_timer = ?1 WHERE char_id = ?2",
            &[
                &(future_time as i32) as &dyn IntoValue,
                &(_char3_id as i32) as &dyn IntoValue,
            ],
        )
        .expect("Failed to set delete_timer");

        // 清理已过期的角色
        let deleted = db
            .cleanup_deleted_characters()
            .expect("Failed to cleanup");
        assert_eq!(deleted, 1, "Should have deleted 1 expired character");

        // 验证：正常角色仍在
        assert!(db
            .get_character_by_id(_char1_id)
            .expect("query failed")
            .is_some());
        // 验证：已过期角色被删除
        assert!(db
            .get_character_by_id(char2_id)
            .expect("query failed")
            .is_none());
        // 验证：待删除但未过期角色仍在
        assert!(db
            .get_character_by_id(_char3_id)
            .expect("query failed")
            .is_some());
    }

    #[test]
    fn test_mark_and_cancel_character_deletion() {
        let db = crate::storage::Database::open_memory().unwrap();
        crate::storage::schema::init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "DeleteMe", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 标记删除（24小时后）
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(result, "标记删除应成功");

        // 验证 delete_timer 已设置
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert!(char.delete_timer > 0, "delete_timer 应大于 0");

        // 取消删除
        let result = db.cancel_character_deletion(char_id, account_id).unwrap();
        assert!(result, "取消删除应成功");

        // 验证 delete_timer 已重置
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert_eq!(char.delete_timer, 0, "delete_timer 应重置为 0");
    }

    #[test]
    fn test_mark_deletion_wrong_account() {
        let db = crate::storage::Database::open_memory().unwrap();
        crate::storage::schema::init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "MyChar", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 用错误的 account_id 尝试标记删除
        let result = db.mark_character_for_deletion(char_id, 9999, 86400).unwrap();
        assert!(!result, "错误账户标记删除应失败");

        // 验证角色未被标记
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert_eq!(char.delete_timer, 0, "角色不应被错误账户标记删除");
    }

    #[test]
    fn test_get_character_by_name() {
        let db = crate::storage::Database::open_memory().unwrap();
        crate::storage::schema::init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        db.create_character(account_id, 0, "UniqueName", 5, 5, 5, 5, 5, 5, 1, 0).unwrap();

        // 查找存在的角色
        let result = db.get_character_by_name("UniqueName").unwrap();
        assert!(result.is_some(), "应能找到已创建的角色");
        assert_eq!(result.unwrap().name, "UniqueName");

        // 查找不存在的角色
        let result = db.get_character_by_name("NonExistent").unwrap();
        assert!(result.is_none(), "不存在的角色应返回 None");
    }

    #[test]
    fn test_double_mark_deletion_fails() {
        let db = crate::storage::Database::open_memory().unwrap();
        crate::storage::schema::init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "DoubleDel", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 第一次标记成功
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(result);

        // 第二次标记应失败（已标记）
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(!result, "重复标记删除应返回 false");
    }
}
