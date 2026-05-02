//! Deviruchi - High-performance MMORPG game server

pub mod cli;
pub mod core;
pub mod game;
pub mod network;
pub mod protocol;
pub mod storage;
pub mod error;

pub use error::{Error, Result};
