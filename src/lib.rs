// 允许非 snake_case 命名：rAthena 兼容字段名使用驼峰命名
#![allow(non_snake_case)]

//! Deviruchi - High-performance MMORPG game server

pub mod cli;
pub mod core;
pub mod error;
pub mod game;
pub mod network;
pub mod protocol;
pub mod storage;

pub use error::{Error, Result};
