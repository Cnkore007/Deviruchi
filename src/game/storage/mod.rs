//! 仓库模块
//! 负责管理角色仓库的存储、同步和协议处理

pub mod data;
pub mod manager;
pub mod manager_sync;
pub mod protocol;
pub mod repository;
pub mod scheduler;
pub mod sync;

pub use data::{Storage, StorageSlot};
pub use manager::StorageManager;
pub use manager_sync::{DirtyStats, StorageSyncManager};
pub use protocol::{StorageRequest, StorageResponse};
pub use repository::StorageRepository;
pub use scheduler::{StorageSyncScheduler, SyncTask};
pub use sync::{SyncRecord, SyncState};
