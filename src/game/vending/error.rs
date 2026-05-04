//! Vending system errors

use thiserror::Error;

/// 摆摊系统错误类型
#[derive(Debug, Clone, Error)]
pub enum VendingError {
    /// 玩家已有商店
    #[error("Player already has a shop")]
    AlreadyHasShop,

    /// 玩家没有商店
    #[error("Player does not have a shop")]
    NoShop,

    /// 商店物品已满
    #[error("Shop is full")]
    ShopFull,

    /// 无效的物品栏位
    #[error("Invalid inventory slot")]
    InvalidSlot,

    /// 物品数量不足
    #[error("Not enough items to sell")]
    NotEnoughItems,

    /// 玩家Zeny不足
    #[error("Buyer does not have enough zeny")]
    NotEnoughZeny,

    /// 不能购买自己的商店
    #[error("Cannot buy from your own shop")]
    CannotBuyOwnShop,

    /// 商店已关闭
    #[error("Shop is closed")]
    ShopClosed,

    /// 不在同一地图
    #[error("Must be on the same map as the shop")]
    MapMismatch,

    /// 物品不在商店中
    #[error("Item not found in shop")]
    ItemNotFound,

    /// 超过最大购买数量
    #[error("Exceeded maximum purchase amount")]
    ExceededMaxAmount,

    /// 商店未开启
    #[error("Shop is not open")]
    ShopNotOpen,

    /// 玩家不在线
    #[error("Player is not online")]
    PlayerNotOnline,

    /// 背包已满
    #[error("Inventory is full")]
    InventoryFull,

    /// 交易失败
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// 重量超限
    #[error("Weight limit exceeded")]
    WeightLimitExceeded,
}
