use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 拍卖物品条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionItem {
    pub item_id: u32,
    pub name: String,
    pub amount: u32,
    pub refined: u8,
    pub cards: [u16; 4],
}

/// 竞价记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionBid {
    pub bidder_id: Uuid,
    pub bidder_name: String,
    pub amount: u64,
    pub time: u64,
}

/// 拍卖条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionEntry {
    pub auction_id: Uuid,
    pub seller_id: Uuid,
    pub seller_name: String,
    pub item: AuctionItem,
    pub starting_price: u64,
    pub buyout_price: Option<u64>,
    pub current_bid: u64,
    pub bids: Vec<AuctionBid>,
    pub created_time: u64,
    pub end_time: u64,
    pub closed: bool,
}

impl AuctionEntry {
    pub fn new(
        seller_id: Uuid,
        seller_name: String,
        item: AuctionItem,
        starting_price: u64,
        buyout_price: Option<u64>,
        duration_hours: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            auction_id: Uuid::new_v4(),
            seller_id,
            seller_name,
            item,
            starting_price,
            buyout_price,
            current_bid: starting_price,
            bids: Vec::new(),
            created_time: now,
            end_time: now + duration_hours * 3600,
            closed: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.end_time
    }

    pub fn time_remaining_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.end_time.saturating_sub(now)
    }

    pub fn highest_bidder(&self) -> Option<&AuctionBid> {
        self.bids.last()
    }
}

/// 拍卖搜索结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionSearchEntry {
    pub auction_id: Uuid,
    pub seller_name: String,
    pub item: AuctionItem,
    pub current_bid: u64,
    pub buyout_price: Option<u64>,
    pub bid_count: usize,
    pub time_remaining_secs: u64,
}

/// 拍卖操作错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionError {
    NotFound,
    AlreadyClosed,
    Expired,
    BidTooLow,
    CannotBidOwnAuction,
    InsufficientFunds,
    InventoryFull,
    Success,
}
