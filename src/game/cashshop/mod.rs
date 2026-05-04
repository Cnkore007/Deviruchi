//! Cash Shop Module
//!
//! 现金商城模块，提供以下功能：
//! - 商城物品数据库
//! - 物品购买和赠送
//! - 现金点数管理
//! - Kafra 传送和仓库服务

pub mod data;
pub mod kafra;
pub mod manager;

// Re-exports
pub use data::{CashShopCategory, CashShopDatabase, CashShopItem};
pub use kafra::{
    KafraService, KafraServiceType, StorageResult, TeleportDestination, TeleportResult,
};
pub use manager::{CashPoints, CashShopManager, GiftResult, PurchaseResult, PurchaseType};
