use rusqlite::params;
use crate::storage::Database;
use crate::error::Result;
use crate::storage::chrono_now;
use crate::protocol::map_packets::CharInfo;

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
                account_id, slot, name, str, agi, vit, int, dex, luk,
                hair, hair_color, now
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
                    last_map, last_x, last_y,
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
                    delete_timer: row.get(30)?,
                    created_at: row.get(31)?,
                    updated_at: row.get(32)?,
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
                    last_map, last_x, last_y,
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
                    delete_timer: row.get(30)?,
                    created_at: row.get(31)?,
                    updated_at: row.get(32)?,
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
    pub delete_timer: u32,
    pub created_at: i64,
    pub updated_at: i64,
}
