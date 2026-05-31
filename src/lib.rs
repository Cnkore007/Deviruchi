#![allow(dead_code, unused_variables, non_snake_case)]

//! Deviruchi - High-performance MMORPG game server

pub mod cli;
pub mod core;
pub mod error;
pub mod game;
pub mod network;
pub mod protocol;
pub mod storage;

pub use error::{Error, Result};
