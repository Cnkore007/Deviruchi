use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::game::guild::{Guild, GuildMember, GuildPosition};
use crate::storage::Database;
use crate::storage::chrono_now;

/// 公会数据库持久化
pub struct GuildStorage {
    db: Arc<Database>,
}

impl GuildStorage {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 保存公会（插入或更新）
    pub fn save_guild(&self, guild: &Guild) -> Result<()> {
        let existing_id: Option<i64> = self.db.query_row_optional(
            "SELECT guild_id FROM guilds WHERE guild_uuid = ?1",
            [&guild.id.to_string()],
            |row| row.get(0),
        )?;

        let guild_db_id: i64 = if let Some(id) = existing_id {
            self.db.execute_with_params(
                "UPDATE guilds SET name=?1, master_name=?2, guild_lv=?3, exp=?4, max_exp=?5,
                 member_count=?6, max_members=?7, average_level=?8, notice=?9, emblem_id=?10
                 WHERE guild_id=?11",
                rusqlite::params![
                    guild.name,
                    guild.master_name,
                    guild.level,
                    guild.exp,
                    guild.max_exp,
                    guild.member_count,
                    guild.max_members,
                    guild.average_level,
                    guild.notice,
                    guild.emblem_id,
                    id,
                ],
            )?;
            id
        } else {
            self.db.execute_with_params(
                "INSERT INTO guilds (guild_uuid, name, master_name, guild_lv, exp, max_exp,
                 member_count, max_members, average_level, notice, emblem_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    guild.id.to_string(),
                    guild.name,
                    guild.master_name,
                    guild.level,
                    guild.exp,
                    guild.max_exp,
                    guild.member_count,
                    guild.max_members,
                    guild.average_level,
                    guild.notice,
                    guild.emblem_id,
                    chrono_now(),
                ],
            )?;
            self.db.last_insert_rowid()? as i64
        };

        // 删除旧数据并重新插入职位和成员
        self.db.execute_with_params(
            "DELETE FROM guild_positions WHERE guild_id=?1",
            [guild_db_id],
        )?;
        self.db
            .execute_with_params("DELETE FROM guild_members WHERE guild_id=?1", [guild_db_id])?;

        // 保存职位
        for pos in &guild.positions {
            self.db.execute_with_params(
                "INSERT INTO guild_positions (guild_id, position_id, name, can_invite, can_expel, can_use_storage, can_use_skill)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    guild_db_id,
                    pos.id,
                    pos.name,
                    pos.can_invite as i32,
                    pos.can_expel as i32,
                    pos.can_use_storage as i32,
                    pos.can_use_skill as i32,
                ],
            )?;
        }

        // 保存成员
        for member in guild.members.values() {
            self.db.execute_with_params(
                "INSERT INTO guild_members (guild_id, player_uuid, char_id, name, position, level, job, contribution, online, map_name, joined_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    guild_db_id,
                    member.player_id.to_string(),
                    member.char_id,
                    member.name,
                    member.position_id,
                    member.level,
                    member.job,
                    member.contribution,
                    member.online as i32,
                    member.map_name,
                    chrono_now(),
                ],
            )?;
        }

        Ok(())
    }

    /// 加载单个公会
    pub fn load_guild(&self, guild_uuid: &Uuid) -> Result<Option<Guild>> {
        let row = self.db.query_row_optional(
            "SELECT guild_id, name, master_name, guild_lv, exp, max_exp, member_count,
                        max_members, average_level, notice, emblem_id
                 FROM guilds WHERE guild_uuid = ?1",
            [guild_uuid.to_string()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? as u8,
                    row.get::<_, i64>(4)? as u64,
                    row.get::<_, i64>(5)? as u64,
                    row.get::<_, i64>(6)? as u32,
                    row.get::<_, i64>(7)? as u32,
                    row.get::<_, i64>(8)? as u16,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)? as u32,
                ))
            },
        )?;

        let Some((
            guild_db_id,
            name,
            master_name,
            level,
            exp,
            max_exp,
            member_count,
            max_members,
            average_level,
            notice,
            emblem_id,
        )) = row
        else {
            return Ok(None);
        };

        // 加载职位
        let positions = self.db.query(
            "SELECT position_id, name, can_invite, can_expel, can_use_storage, can_use_skill
             FROM guild_positions WHERE guild_id = ?1 ORDER BY position_id",
            [guild_db_id],
            |row| {
                Ok(GuildPosition {
                    id: row.get::<_, i64>(0)? as u8,
                    name: row.get::<_, String>(1)?,
                    can_invite: row.get::<_, i64>(2)? != 0,
                    can_expel: row.get::<_, i64>(3)? != 0,
                    can_use_storage: row.get::<_, i64>(4)? != 0,
                    can_use_skill: row.get::<_, i64>(5)? != 0,
                })
            },
        )?;

        // 加载成员
        let members: Vec<GuildMember> = self.db.query(
            "SELECT player_uuid, char_id, name, position, level, job, contribution, online, map_name
             FROM guild_members WHERE guild_id = ?1",
            [guild_db_id],
            |row| {
                Ok(GuildMember {
                    player_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    char_id: row.get::<_, i64>(1)? as u32,
                    name: row.get::<_, String>(2)?,
                    position_id: row.get::<_, i64>(3)? as u8,
                    level: row.get::<_, i64>(4)? as u16,
                    job: row.get::<_, i64>(5)? as u16,
                    contribution: row.get::<_, i64>(6)? as u32,
                    online: row.get::<_, i64>(7)? != 0,
                    map_name: row.get::<_, String>(8)?,
                })
            },
        )?;

        let mut member_map = HashMap::new();
        for m in members {
            member_map.insert(m.player_id, m);
        }

        Ok(Some(Guild {
            id: *guild_uuid,
            name,
            master_name,
            level,
            exp,
            max_exp,
            member_count,
            max_members,
            average_level,
            notice,
            emblem_id,
            positions,
            members: member_map,
        }))
    }

    /// 加载所有公会
    pub fn load_all_guilds(&self) -> Result<Vec<Guild>> {
        let guild_uuids: Vec<String> =
            self.db.query("SELECT guild_uuid FROM guilds", [], |row| {
                row.get::<_, String>(0)
            })?;

        let mut guilds = Vec::new();
        for uuid_str in guild_uuids {
            if let Ok(uuid) = Uuid::parse_str(&uuid_str)
                && let Some(guild) = self.load_guild(&uuid)?
            {
                guilds.push(guild);
            }
        }
        Ok(guilds)
    }

    /// 删除公会
    pub fn delete_guild(&self, guild_uuid: &Uuid) -> Result<bool> {
        let affected = self.db.execute_with_params(
            "DELETE FROM guilds WHERE guild_uuid = ?1",
            [guild_uuid.to_string()],
        )?;
        Ok(affected > 0)
    }
}
