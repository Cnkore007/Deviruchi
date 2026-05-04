//! 摆摊系统模块

pub mod error;
pub mod manager;
pub mod search;
pub mod shop;

pub use error::VendingError;
pub use manager::VendingManager;
pub use search::{ShopSearch, ShopSearchEngine, ShopSearchResult};
pub use shop::{MAX_SHOP_ITEMS, ShopItem, VendingShop};
