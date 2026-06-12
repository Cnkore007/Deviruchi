pub mod account;
pub mod backend;
pub mod character;
pub mod guild;
pub mod migration;
#[cfg(feature = "mysql-backend")]
pub mod mysql_backend;
pub mod password;
pub mod schema;
pub mod sqlite;
pub mod sqlite_backend;

pub use account::Account;
pub use backend::{Backend, IntoValue, Row, TransactionOps, Value};
pub use character::Character;
pub use guild::GuildStorage;
pub use migration::{Migration, MigrationManager};
#[cfg(feature = "mysql-backend")]
pub use mysql_backend::{MySqlBackend, MySqlConfig};
pub use schema::init_schema;
pub use sqlite::Database;
pub use sqlite::chrono_now;
pub use sqlite_backend::{SqliteBackend, SqliteConfig};

pub(crate) fn safe_u32(val: i32) -> u32 {
    val.max(0) as u32
}

pub(crate) fn safe_u16(val: i32) -> u16 {
    val.max(0).min(u16::MAX as i32) as u16
}

pub(crate) fn safe_u8(val: i32) -> u8 {
    val.max(0).min(u8::MAX as i32) as u8
}

pub(crate) fn safe_u32_from_i64(val: i64) -> u32 {
    val.max(0).min(u32::MAX as i64) as u32
}

pub(crate) fn safe_u16_from_i64(val: i64) -> u16 {
    val.max(0).min(u16::MAX as i64) as u16
}

pub(crate) fn safe_u8_from_i64(val: i64) -> u8 {
    val.max(0).min(u8::MAX as i64) as u8
}

pub(crate) fn safe_u64_from_i64(val: i64) -> u64 {
    val.max(0) as u64
}
