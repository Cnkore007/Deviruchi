//! 包处理器
//!
//! 提供协议包的回调注册和分发机制。
//! 上层模块可以通过注册回调函数来处理特定类型的协议包，
//! 当收到包时调用 `dispatch` 方法即可自动路由到对应的处理函数。

use crate::protocol::Packet;
use crate::protocol::login::LoginResponse;
use crate::protocol::char_mod::{CharListResponse, CharCreateResponse, CharDeleteResponse, CharEnterResponse};

/// 登录响应回调类型
type LoginResponseCallback = Box<dyn Fn(&LoginResponse) + Send + Sync>;
/// 角色列表响应回调类型
type CharListCallback = Box<dyn Fn(&CharListResponse) + Send + Sync>;
/// 创建角色响应回调类型
type CharCreateCallback = Box<dyn Fn(&CharCreateResponse) + Send + Sync>;
/// 删除角色响应回调类型
type CharDeleteCallback = Box<dyn Fn(&CharDeleteResponse) + Send + Sync>;
/// 进入角色响应回调类型
type CharEnterCallback = Box<dyn Fn(&CharEnterResponse) + Send + Sync>;

/// 协议包处理器
///
/// 管理各类协议包的回调函数，通过 `dispatch` 方法将收到的包
/// 自动路由到对应的已注册回调。未注册回调的包类型会记录警告日志。
pub struct PacketHandler {
    /// 登录响应包回调
    login_response_cb: Option<LoginResponseCallback>,
    /// 角色列表响应包回调
    char_list_cb: Option<CharListCallback>,
    /// 创建角色响应包回调
    char_create_cb: Option<CharCreateCallback>,
    /// 删除角色响应包回调
    char_delete_cb: Option<CharDeleteCallback>,
    /// 进入角色响应包回调
    char_enter_cb: Option<CharEnterCallback>,
}

impl PacketHandler {
    /// 创建一个空的包处理器
    pub fn new() -> Self {
        Self {
            login_response_cb: None,
            char_list_cb: None,
            char_create_cb: None,
            char_delete_cb: None,
            char_enter_cb: None,
        }
    }

    /// 注册登录响应包的回调函数
    pub fn on_login_response<F: Fn(&LoginResponse) + Send + Sync + 'static>(&mut self, cb: F) {
        self.login_response_cb = Some(Box::new(cb));
    }

    /// 注册角色列表响应包的回调函数
    pub fn on_char_list<F: Fn(&CharListResponse) + Send + Sync + 'static>(&mut self, cb: F) {
        self.char_list_cb = Some(Box::new(cb));
    }

    /// 注册创建角色响应包的回调函数
    pub fn on_char_create<F: Fn(&CharCreateResponse) + Send + Sync + 'static>(&mut self, cb: F) {
        self.char_create_cb = Some(Box::new(cb));
    }

    /// 注册删除角色响应包的回调函数
    pub fn on_char_delete<F: Fn(&CharDeleteResponse) + Send + Sync + 'static>(&mut self, cb: F) {
        self.char_delete_cb = Some(Box::new(cb));
    }

    /// 注册进入角色响应包的回调函数
    pub fn on_char_enter<F: Fn(&CharEnterResponse) + Send + Sync + 'static>(&mut self, cb: F) {
        self.char_enter_cb = Some(Box::new(cb));
    }

    /// 分发协议包到对应的回调函数
    ///
    /// 根据包的类型查找已注册的回调并调用。
    /// 如果对应类型的回调未注册，则记录警告日志。
    pub fn dispatch(&self, packet: &Packet) {
        match packet {
            Packet::LoginResponse(resp) => {
                if let Some(cb) = &self.login_response_cb {
                    cb(resp);
                }
            }
            Packet::CharListResponse(resp) => {
                if let Some(cb) = &self.char_list_cb {
                    cb(resp);
                }
            }
            Packet::CharCreateResponse(resp) => {
                if let Some(cb) = &self.char_create_cb {
                    cb(resp);
                }
            }
            Packet::CharDeleteResponse(resp) => {
                if let Some(cb) = &self.char_delete_cb {
                    cb(resp);
                }
            }
            Packet::CharEnterResponse(resp) => {
                if let Some(cb) = &self.char_enter_cb {
                    cb(resp);
                }
            }
            _ => {
                tracing::warn!("未处理的包类型: ID=0x{:04X}", packet.packet_id());
            }
        }
    }
}
