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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== StorageRequest 测试 ==========

    /// Load 请求的 char_id
    #[test]
    fn request_load_char_id() {
        let req = StorageRequest::Load { char_id: 42 };
        assert_eq!(req.char_id(), 42);
    }

    /// Save 请求的 char_id
    #[test]
    fn request_save_char_id() {
        let req = StorageRequest::Save {
            char_id: 100,
            slots: vec![],
        };
        assert_eq!(req.char_id(), 100);
    }

    /// Resize 请求的 char_id
    #[test]
    fn request_resize_char_id() {
        let req = StorageRequest::Resize {
            char_id: 200,
            new_size: 150,
        };
        assert_eq!(req.char_id(), 200);
    }

    /// Unlock 请求的 char_id
    #[test]
    fn request_unlock_char_id() {
        let req = StorageRequest::Unlock { char_id: 300 };
        assert_eq!(req.char_id(), 300);
    }

    /// SyncStatus 请求的 char_id
    #[test]
    fn request_sync_status_char_id() {
        let req = StorageRequest::SyncStatus { char_id: 400 };
        assert_eq!(req.char_id(), 400);
    }

    /// Request 可以 clone
    #[test]
    fn request_is_cloneable() {
        let req = StorageRequest::Load { char_id: 1 };
        let req2 = req.clone();
        assert_eq!(req2.char_id(), 1);
    }

    /// Request 可以 debug 输出
    #[test]
    fn request_is_debuggable() {
        let req = StorageRequest::Load { char_id: 1 };
        let debug_str = format!("{:?}", req);
        assert!(debug_str.contains("Load"));
        assert!(debug_str.contains("1"));
    }

    // ========== StorageResponse 测试 ==========

    /// success 构造 Saved 响应
    #[test]
    fn response_success() {
        let resp = StorageResponse::success(42);
        match resp {
            StorageResponse::Saved { char_id } => assert_eq!(char_id, 42),
            _ => panic!("期望 Saved"),
        }
    }

    /// error 构造 Error 响应（&str）
    #[test]
    fn response_error() {
        let resp = StorageResponse::error(42, "test error");
        match resp {
            StorageResponse::Error { char_id, message } => {
                assert_eq!(char_id, 42);
                assert_eq!(message, "test error");
            }
            _ => panic!("期望 Error"),
        }
    }

    /// error 构造 Error 响应（String）
    #[test]
    fn response_error_with_string() {
        let msg = String::from("owned string");
        let resp = StorageResponse::error(1, msg);
        match resp {
            StorageResponse::Error { message, .. } => {
                assert_eq!(message, "owned string");
            }
            _ => panic!("期望 Error"),
        }
    }

    /// Data 响应携带格子数据
    #[test]
    fn response_data_with_slots() {
        let slots = vec![
            StorageSlot {
                index: 0,
                item_id: 501,
                amount: 10,
                identified: true,
                refine: 0,
                cards: [0; 4],
            },
            StorageSlot {
                index: 1,
                item_id: 601,
                amount: 1,
                identified: true,
                refine: 7,
                cards: [4001, 0, 0, 0],
            },
        ];
        let resp = StorageResponse::Data {
            char_id: 42,
            slots: slots.clone(),
        };
        match resp {
            StorageResponse::Data {
                char_id,
                slots: loaded,
            } => {
                assert_eq!(char_id, 42);
                assert_eq!(loaded.len(), 2);
                assert_eq!(loaded[0].item_id, 501);
                assert_eq!(loaded[1].refine, 7);
            }
            _ => panic!("期望 Data"),
        }
    }

    /// SyncStatus 响应
    #[test]
    fn response_sync_status() {
        let resp = StorageResponse::SyncStatus {
            char_id: 42,
            is_dirty: true,
            version: 5,
        };
        match resp {
            StorageResponse::SyncStatus {
                char_id,
                is_dirty,
                version,
            } => {
                assert_eq!(char_id, 42);
                assert!(is_dirty);
                assert_eq!(version, 5);
            }
            _ => panic!("期望 SyncStatus"),
        }
    }

    /// Response 可以 clone
    #[test]
    fn response_is_cloneable() {
        let resp = StorageResponse::success(1);
        let resp2 = resp.clone();
        assert!(matches!(resp2, StorageResponse::Saved { char_id: 1 }));
    }

    /// Response 可以 debug 输出
    #[test]
    fn response_is_debuggable() {
        let resp = StorageResponse::success(1);
        let debug_str = format!("{:?}", resp);
        assert!(debug_str.contains("Saved"));
    }
}
