use crate::protocol::packet_builder::PacketBuilder;

/// 客户端请求打开仓库 (0x0213)
pub struct CZReqStorageOpen;

impl CZReqStorageOpen {
    pub fn from_packet(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求关闭仓库 (0x0214)
pub struct CZReqStorageClose;

impl CZReqStorageClose {
    pub fn from_packet(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求移动物品（存/取）(0x0215)
pub struct CZReqStorageMoveItem {
    pub from_index: u16,     // 源位置（背包或仓库索引）
    pub to_index: u16,       // 目标位置（仓库或背包索引）
    pub amount: u16,         // 数量
    pub is_to_storage: bool, // true = 存入仓库, false = 取出到背包
}

impl CZReqStorageMoveItem {
    pub fn from_packet(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        Some(Self {
            from_index: u16::from_le_bytes([data[0], data[1]]),
            to_index: u16::from_le_bytes([data[2], data[3]]),
            amount: u16::from_le_bytes([data[4], data[5]]),
            is_to_storage: data[6] != 0,
        })
    }
}

/// 服务器通知仓库打开 (0x01F3)
pub struct ZCStorageOpen {
    pub result: u8, // 0 = 成功, 1 = 失败
}

impl ZCStorageOpen {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F3).put_u8(self.result).build()
    }
}

/// 服务器通知仓库关闭 (0x01F4)
pub struct ZCStorageClose;

impl ZCStorageClose {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F4).build()
    }
}

/// 仓库物品
#[derive(Debug, Clone)]
pub struct StorageItem {
    pub index: u16,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
}

/// 服务器发送仓库物品列表 (0x01F5)
pub struct ZCStorageItems {
    pub count: u16,
    pub items: Vec<StorageItem>,
}

impl ZCStorageItems {
    pub fn to_packet(&self) -> Vec<u8> {
        let mut builder = PacketBuilder::new(0x01F5);
        builder = builder.put_u16(self.count);

        for item in &self.items {
            builder = builder.put_u16(item.index);
            builder = builder.put_u16(item.item_id);
            builder = builder.put_u16(item.amount);
            builder = builder.put_u8(if item.identified { 1 } else { 0 });
        }

        builder.build()
    }
}

/// 服务器通知添加物品到仓库 (0x01F6)
pub struct ZCStorageItemAdd {
    pub index: u16,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
}

impl ZCStorageItemAdd {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F6)
            .put_u16(self.index)
            .put_u16(self.item_id)
            .put_u16(self.amount)
            .put_u8(if self.identified { 1 } else { 0 })
            .build()
    }
}

/// 服务器通知从仓库移除物品 (0x01F7)
pub struct ZCStorageItemRemove {
    pub index: u16,
    pub amount: u16,
}

impl ZCStorageItemRemove {
    pub fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F7)
            .put_u16(self.index)
            .put_u16(self.amount)
            .build()
    }
}
