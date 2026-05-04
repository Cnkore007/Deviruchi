//! 仓库同步协议定义
//! 用于仓库服务器间通信

use super::data::StorageSlot;

/// 仓库请求类型
#[derive(Debug, Clone)]
pub enum StorageRequest {
    /// 请求加载仓库
    Load { char_id: u32 },
    /// 请求保存仓库
    Save {
        char_id: u32,
        slots: Vec<StorageSlot>,
    },
    /// 请求仓库大小调整
    Resize { char_id: u32, new_size: u16 },
    /// 请求仓库解锁
    Unlock { char_id: u32 },
    /// 请求同步状态
    SyncStatus { char_id: u32 },
}

impl StorageRequest {
    pub fn char_id(&self) -> u32 {
        match self {
            StorageRequest::Load { char_id } => *char_id,
            StorageRequest::Save { char_id, .. } => *char_id,
            StorageRequest::Resize { char_id, .. } => *char_id,
            StorageRequest::Unlock { char_id } => *char_id,
            StorageRequest::SyncStatus { char_id } => *char_id,
        }
    }
}

/// 仓库响应类型
#[derive(Debug, Clone)]
pub enum StorageResponse {
    /// 仓库数据
    Data {
        char_id: u32,
        slots: Vec<StorageSlot>,
    },
    /// 保存成功
    Saved { char_id: u32 },
    /// 错误
    Error { char_id: u32, message: String },
    /// 同步状态
    SyncStatus {
        char_id: u32,
        is_dirty: bool,
        version: u64,
    },
}

impl StorageResponse {
    pub fn success(char_id: u32) -> Self {
        StorageResponse::Saved { char_id }
    }

    pub fn error(char_id: u32, message: impl Into<String>) -> Self {
        StorageResponse::Error {
            char_id,
            message: message.into(),
        }
    }
}
