//! Chat Module - Private messaging and chat command parsing
//!
//! This module provides whisper/private messaging functionality for
//! player-to-player direct communication.
//!
//! # Features
//! - Send whispers to online players
//! - Queue messages for offline players
//! - Deliver offline messages on login
//! - Rate limiting to prevent spam
//! - Reply (/r) functionality
//! - Command parsing for /w, /whisper, /t, /r

pub mod manager;
pub mod parser;
pub mod rate_limit;
pub mod whisper;

pub use manager::{ChatManager, ChatResult};
pub use parser::{ChatCommand, parse_chat};
pub use rate_limit::WhisperRateLimiter;
pub use whisper::{OfflineMessage, WhisperManager, WhisperResult};
