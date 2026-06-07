pub mod data;

use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::debug;
use uuid::Uuid;

pub use data::*;

/// 最大拍卖条目数
const MAX_AUCTION_ITEMS: usize = 500;
/// 拍卖最小增量百分比（5%）
const BID_INCREMENT_PERCENT: u64 = 5;

#[allow(dead_code)]
pub struct AuctionHouse {
    /// 活跃的拍卖
    active_auctions: RwLock<Vec<AuctionEntry>>,
    /// 已结束的拍卖（等待领取）
    pending_claim: RwLock<HashMap<Uuid, Vec<AuctionEntry>>>,
    /// 卖家已领取完成的拍卖
    claimed: RwLock<HashMap<Uuid, Vec<AuctionEntry>>>,
}

impl AuctionHouse {
    pub fn new() -> Self {
        Self {
            active_auctions: RwLock::new(Vec::new()),
            pending_claim: RwLock::new(HashMap::new()),
            claimed: RwLock::new(HashMap::new()),
        }
    }

    /// 上架拍卖物品
    pub fn list_item(
        &self,
        seller_id: Uuid,
        seller_name: &str,
        item: AuctionItem,
        starting_price: u64,
        buyout_price: Option<u64>,
        duration_hours: u64,
    ) -> Result<Uuid, AuctionError> {
        let mut auctions = self.active_auctions.write();
        if auctions.len() >= MAX_AUCTION_ITEMS {
            return Err(AuctionError::InventoryFull);
        }

        let entry = AuctionEntry::new(
            seller_id,
            seller_name.to_string(),
            item,
            starting_price,
            buyout_price,
            duration_hours,
        );
        let id = entry.auction_id;
        auctions.push(entry);
        debug!("Item listed for auction: {}", id);
        Ok(id)
    }

    /// 竞价
    pub fn place_bid(
        &self,
        auction_id: &Uuid,
        bidder_id: Uuid,
        bidder_name: &str,
        bid_amount: u64,
    ) -> Result<AuctionBid, AuctionError> {
        let mut auctions = self.active_auctions.write();
        let entry = auctions
            .iter_mut()
            .find(|a| &a.auction_id == auction_id)
            .ok_or(AuctionError::NotFound)?;

        if entry.closed {
            return Err(AuctionError::AlreadyClosed);
        }

        if entry.is_expired() {
            entry.closed = true;
            return Err(AuctionError::Expired);
        }

        if bidder_id == entry.seller_id {
            return Err(AuctionError::CannotBidOwnAuction);
        }

        // 检查是否达到一口价
        if let Some(buyout) = entry.buyout_price {
            if bid_amount >= buyout {
                let bid = AuctionBid {
                    bidder_id,
                    bidder_name: bidder_name.to_string(),
                    amount: buyout,
                    time: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                entry.current_bid = buyout;
                entry.bids.push(bid.clone());
                entry.closed = true;

                self.move_to_pending(entry);
                return Ok(bid);
            }
        }

        // 检查出价是否高于当前最高价 + 最小增量
        let min_bid = entry.current_bid + (entry.current_bid * BID_INCREMENT_PERCENT / 100);
        if bid_amount < min_bid {
            return Err(AuctionError::BidTooLow);
        }

        let bid = AuctionBid {
            bidder_id,
            bidder_name: bidder_name.to_string(),
            amount: bid_amount,
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        entry.current_bid = bid_amount;
        entry.bids.push(bid.clone());

        debug!(
            "Bid placed: {} by {} for {} zeny",
            auction_id, bidder_name, bid_amount
        );
        Ok(bid)
    }

    /// 一口价直接购买
    pub fn buyout(
        &self,
        auction_id: &Uuid,
        buyer_id: Uuid,
        buyer_name: &str,
    ) -> Result<AuctionBid, AuctionError> {
        let mut auctions = self.active_auctions.write();
        let entry = auctions
            .iter_mut()
            .find(|a| &a.auction_id == auction_id)
            .ok_or(AuctionError::NotFound)?;

        if entry.closed {
            return Err(AuctionError::AlreadyClosed);
        }

        if entry.is_expired() {
            entry.closed = true;
            return Err(AuctionError::Expired);
        }

        if buyer_id == entry.seller_id {
            return Err(AuctionError::CannotBidOwnAuction);
        }

        let buyout_price = entry.buyout_price.ok_or(AuctionError::BidTooLow)?;

        let bid = AuctionBid {
            bidder_id: buyer_id,
            bidder_name: buyer_name.to_string(),
            amount: buyout_price,
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        entry.current_bid = buyout_price;
        entry.bids.push(bid.clone());
        entry.closed = true;

        self.move_to_pending(entry);
        Ok(bid)
    }

    /// 将已结束的拍卖移到待领取列表
    fn move_to_pending(&self, entry: &AuctionEntry) {
        let mut pending = self.pending_claim.write();
        let buyer_id = entry
            .highest_bidder()
            .map(|b| b.bidder_id)
            .unwrap_or(entry.seller_id);

        pending.entry(buyer_id).or_default().push(entry.clone());
    }

    /// 搜索拍卖
    pub fn search(
        &self,
        name_filter: Option<&str>,
        min_price: Option<u64>,
        max_price: Option<u64>,
    ) -> Vec<AuctionSearchEntry> {
        let auctions = self.active_auctions.read();
        auctions
            .iter()
            .filter(|a| !a.closed && !a.is_expired())
            .filter(|a| {
                if let Some(name) = name_filter {
                    a.item.name.to_lowercase().contains(&name.to_lowercase())
                } else {
                    true
                }
            })
            .filter(|a| {
                if let Some(min) = min_price {
                    a.current_bid >= min
                } else {
                    true
                }
            })
            .filter(|a| {
                if let Some(max) = max_price {
                    a.current_bid <= max
                } else {
                    true
                }
            })
            .map(|a| AuctionSearchEntry {
                auction_id: a.auction_id,
                seller_name: a.seller_name.clone(),
                item: a.item.clone(),
                current_bid: a.current_bid,
                buyout_price: a.buyout_price,
                bid_count: a.bids.len(),
                time_remaining_secs: a.time_remaining_secs(),
            })
            .collect()
    }

    /// 获取玩家上架的拍卖
    pub fn get_seller_auctions(&self, seller_id: &Uuid) -> Vec<AuctionEntry> {
        self.active_auctions
            .read()
            .iter()
            .filter(|a| &a.seller_id == seller_id)
            .cloned()
            .collect()
    }

    /// 获取玩家竞价的拍卖
    pub fn get_bidder_auctions(&self, bidder_id: &Uuid) -> Vec<AuctionEntry> {
        self.active_auctions
            .read()
            .iter()
            .filter(|a| a.bids.iter().any(|b| &b.bidder_id == bidder_id))
            .cloned()
            .collect()
    }

    /// 领取拍卖物品（买家）
    pub fn claim_item(&self, player_id: &Uuid, auction_id: &Uuid) -> Result<AuctionEntry, AuctionError> {
        let mut pending = self.pending_claim.write();
        if let Some(entries) = pending.get_mut(player_id) {
            if let Some(pos) = entries.iter().position(|e| &e.auction_id == auction_id) {
                let entry = entries.remove(pos);
                debug!("Item claimed for auction {}", auction_id);
                return Ok(entry);
            }
        }
        Err(AuctionError::NotFound)
    }

    /// 领取拍卖所得（卖家）
    pub fn collect_proceeds(
        &self,
        seller_id: &Uuid,
        auction_id: &Uuid,
    ) -> Result<u64, AuctionError> {
        // 在活跃列表查找已关闭的自己的拍卖
        let auctions = self.active_auctions.read();
        let entry = auctions
            .iter()
            .find(|a| &a.seller_id == seller_id && &a.auction_id == auction_id && a.closed)
            .cloned()
            .ok_or(AuctionError::NotFound)?;

        let proceeds = entry.current_bid;
        debug!(
            "Seller {} collected {} zeny from auction {}",
            seller_id, proceeds, auction_id
        );
        Ok(proceeds)
    }

    /// 取消拍卖（仅无人竞价时）
    pub fn cancel_auction(&self, seller_id: &Uuid, auction_id: &Uuid) -> Result<(), AuctionError> {
        let mut auctions = self.active_auctions.write();
        let idx = auctions
            .iter()
            .position(|a| &a.seller_id == seller_id && &a.auction_id == auction_id)
            .ok_or(AuctionError::NotFound)?;

        if !auctions[idx].bids.is_empty() {
            return Err(AuctionError::BidTooLow); // 有人竞价不能取消
        }

        auctions.remove(idx);
        debug!("Auction {} cancelled", auction_id);
        Ok(())
    }

    /// 处理过期拍卖
    pub fn process_expired(&self) {
        let mut auctions = self.active_auctions.write();
        let (expired, active): (Vec<_>, Vec<_>) = auctions
            .drain(..)
            .partition(|a| a.is_expired() && !a.closed);

        for mut entry in expired {
            entry.closed = true;
            let mut pending = self.pending_claim.write();
            let buyer_id = entry
                .highest_bidder()
                .map(|b| b.bidder_id)
                .unwrap_or(entry.seller_id);
            pending.entry(buyer_id).or_default().push(entry);
        }

        *auctions = active;
    }

    /// 活跃拍卖数
    pub fn active_count(&self) -> usize {
        self.active_auctions.read().len()
    }
}

impl Default for AuctionHouse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(seed: u64) -> Uuid {
        Uuid::from_u64_pair(seed, seed)
    }

    fn test_item() -> AuctionItem {
        AuctionItem {
            item_id: 1201,
            name: "Sword".to_string(),
            amount: 1,
            refined: 0,
            cards: [0; 4],
        }
    }

    #[test]
    fn test_list_and_search() {
        let house = AuctionHouse::new();
        let seller = make_id(1);

        house
            .list_item(seller, "Seller", test_item(), 1000, Some(5000), 24)
            .unwrap();

        let results = house.search(None, None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].current_bid, 1000);
    }

    #[test]
    fn test_bidding() {
        let house = AuctionHouse::new();
        let seller = make_id(1);
        let bidder = make_id(2);

        let id = house
            .list_item(seller, "Seller", test_item(), 1000, None, 24)
            .unwrap();

        let bid = house.place_bid(&id, bidder, "Bidder", 2000).unwrap();
        assert_eq!(bid.amount, 2000);
    }

    #[test]
    fn test_cannot_bid_own_auction() {
        let house = AuctionHouse::new();
        let seller = make_id(1);

        let id = house
            .list_item(seller, "Seller", test_item(), 1000, None, 24)
            .unwrap();

        let result = house.place_bid(&id, seller, "Seller", 2000);
        assert!(matches!(result, Err(AuctionError::CannotBidOwnAuction)));
    }

    #[test]
    fn test_buyout() {
        let house = AuctionHouse::new();
        let seller = make_id(1);
        let buyer = make_id(2);

        let id = house
            .list_item(seller, "Seller", test_item(), 1000, Some(3000), 24)
            .unwrap();

        let bid = house.buyout(&id, buyer, "Buyer").unwrap();
        assert_eq!(bid.amount, 3000);
    }

    #[test]
    fn test_bid_too_low() {
        let house = AuctionHouse::new();
        let seller = make_id(1);
        let bidder = make_id(2);

        let id = house
            .list_item(seller, "Seller", test_item(), 1000, None, 24)
            .unwrap();

        // 低于最小增量（1000 + 5% = 1050）
        let result = house.place_bid(&id, bidder, "Bidder", 1020);
        assert!(matches!(result, Err(AuctionError::BidTooLow)));
    }

    #[test]
    fn test_cancel_no_bids() {
        let house = AuctionHouse::new();
        let seller = make_id(1);

        let id = house
            .list_item(seller, "Seller", test_item(), 1000, None, 24)
            .unwrap();

        assert!(house.cancel_auction(&seller, &id).is_ok());
        assert_eq!(house.active_count(), 0);
    }
}
