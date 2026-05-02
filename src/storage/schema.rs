use anyhow::Result;
use crate::storage::Database;

pub fn init_schema(db: &Database) -> Result<()> {
    // 账户表
    db.execute(
        "CREATE TABLE IF NOT EXISTS accounts (
            account_id INTEGER PRIMARY KEY,
            user_id TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            sex INTEGER NOT NULL,
            email TEXT,
            group_id INTEGER DEFAULT 0,
            state INTEGER DEFAULT 0,
            unban_time INTEGER DEFAULT 0,
            expiration_time INTEGER DEFAULT 0,
            logcount INTEGER DEFAULT 0,
            last_login INTEGER,
            created_at INTEGER NOT NULL
        )",
    )?;

    // 角色表
    db.execute(
        "CREATE TABLE IF NOT EXISTS characters (
            char_id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL,
            char_num INTEGER NOT NULL,
            name TEXT NOT NULL,
            class INTEGER DEFAULT 0,
            base_level INTEGER DEFAULT 1,
            job_level INTEGER DEFAULT 1,
            base_exp INTEGER DEFAULT 0,
            job_exp INTEGER DEFAULT 0,
            zeny INTEGER DEFAULT 0,
            str INTEGER DEFAULT 1,
            agi INTEGER DEFAULT 1,
            vit INTEGER DEFAULT 1,
            int INTEGER DEFAULT 1,
            dex INTEGER DEFAULT 1,
            luk INTEGER DEFAULT 1,
            hair INTEGER DEFAULT 1,
            hair_color INTEGER DEFAULT 0,
            clothes_color INTEGER DEFAULT 0,
            body INTEGER DEFAULT 0,
            weapon INTEGER DEFAULT 0,
            shield INTEGER DEFAULT 0,
            head_top INTEGER DEFAULT 0,
            head_mid INTEGER DEFAULT 0,
            head_bottom INTEGER DEFAULT 0,
            last_map TEXT,
            last_x INTEGER,
            last_y INTEGER,
            save_map TEXT,
            save_x INTEGER,
            save_y INTEGER,
            hp INTEGER DEFAULT 1,
            max_hp INTEGER DEFAULT 1,
            sp INTEGER DEFAULT 1,
            max_sp INTEGER DEFAULT 1,
            option INTEGER DEFAULT 0,
            manner INTEGER DEFAULT 0,
            status_point INTEGER DEFAULT 0,
            skill_point INTEGER DEFAULT 0,
            delete_timer INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (account_id) REFERENCES accounts(account_id)
        )",
    )?;

    // 背包物品表
    db.execute(
        "CREATE TABLE IF NOT EXISTS inventory (
            id INTEGER PRIMARY KEY,
            char_id INTEGER NOT NULL,
            nameid INTEGER NOT NULL,
            amount INTEGER NOT NULL,
            equipped INTEGER DEFAULT 0,
            identify INTEGER DEFAULT 1,
            refine INTEGER DEFAULT 0,
            attribute INTEGER DEFAULT 0,
            card0 INTEGER DEFAULT 0,
            card1 INTEGER DEFAULT 0,
            card2 INTEGER DEFAULT 0,
            card3 INTEGER DEFAULT 0,
            FOREIGN KEY (char_id) REFERENCES characters(char_id)
        )",
    )?;

    // 技能表
    db.execute(
        "CREATE TABLE IF NOT EXISTS skills (
            id INTEGER PRIMARY KEY,
            char_id INTEGER NOT NULL,
            skill_id INTEGER NOT NULL,
            lv INTEGER NOT NULL,
            flag INTEGER DEFAULT 0,
            FOREIGN KEY (char_id) REFERENCES characters(char_id)
        )",
    )?;

    // 公会表
    db.execute(
        "CREATE TABLE IF NOT EXISTS guilds (
            guild_id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            master INTEGER NOT NULL,
            guild_lv INTEGER DEFAULT 1,
            exp INTEGER DEFAULT 0,
            emblem_data BLOB,
            created_at INTEGER NOT NULL
        )",
    )?;

    // 公会成员表
    db.execute(
        "CREATE TABLE IF NOT EXISTS guild_members (
            guild_id INTEGER NOT NULL,
            char_id INTEGER NOT NULL,
            position INTEGER DEFAULT 0,
            PRIMARY KEY (guild_id, char_id),
            FOREIGN KEY (guild_id) REFERENCES guilds(guild_id),
            FOREIGN KEY (char_id) REFERENCES characters(char_id)
        )",
    )?;

    tracing::info!("数据库 Schema 初始化完成");
    Ok(())
}
