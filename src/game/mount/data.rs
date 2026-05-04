//! 坐骑数据模块

use serde::{Deserialize, Serialize};

/// 坐骑类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MountType {
    Peco,     // Knight mount (骑士的Peco Peco)
    Gryphon,  // High wizard mount (高魔的狮鹫)
    Peco2,    // Rune knight mount (符文骑士的改进Peco)
    Gryphon2, // Warlock mount (魔导的改进狮鹫)
    Warg,     // Ranger mount (游侠的战狼)
    #[default]
    Other, // 其他类型
}

impl MountType {
    /// 从数值ID获取坐骑类型
    pub fn from_id(id: u8) -> Self {
        match id {
            1 => Self::Peco,
            2 => Self::Gryphon,
            3 => Self::Peco2,
            4 => Self::Gryphon2,
            5 => Self::Warg,
            _ => Self::Other,
        }
    }

    /// 转换为数值ID
    pub fn to_id(&self) -> u8 {
        match self {
            Self::Peco => 1,
            Self::Gryphon => 2,
            Self::Peco2 => 3,
            Self::Gryphon2 => 4,
            Self::Warg => 5,
            Self::Other => 0,
        }
    }

    /// 获取类型名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Peco => "Peco Peco",
            Self::Gryphon => "Gryphon",
            Self::Peco2 => "Peco Peco 2",
            Self::Gryphon2 => "Gryphon 2",
            Self::Warg => "Warg",
            Self::Other => "Mount",
        }
    }
}

/// 坐骑实例数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub mount_id: u16,
    pub name: String,
    pub speed_modifier: i32, // 速度百分比 (100 = 正常, 150 = 50% faster)
    pub required_level: u16,
    pub mount_type: MountType,
    pub item_id: u16,  // 召唤坐骑需要的物品ID
    pub model_id: u16, // 显示的模型ID
}

impl Mount {
    pub fn new(
        mount_id: u16,
        name: &str,
        speed_modifier: i32,
        required_level: u16,
        mount_type: MountType,
        item_id: u16,
        model_id: u16,
    ) -> Self {
        Self {
            mount_id,
            name: name.to_string(),
            speed_modifier,
            required_level,
            mount_type,
            item_id,
            model_id,
        }
    }

    /// 计算实际速度（基于基础速度）
    pub fn calculate_speed(&self, base_speed: u16) -> u16 {
        ((base_speed as i32 * self.speed_modifier) / 100) as u16
    }
}

/// 坐骑数据库
pub struct MountDatabase {
    mounts: std::collections::HashMap<u16, Mount>,
}

impl MountDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            mounts: std::collections::HashMap::new(),
        };
        db.init_default_mounts();
        db
    }

    fn init_default_mounts(&mut self) {
        // Peco Peco (骑士)
        self.mounts.insert(
            1,
            Mount::new(
                1,
                "Peco Peco",
                150, // 50% faster
                50,  // 骑士转职等级
                MountType::Peco,
                2260, // Peco Peco Feather
                1288,
            ),
        );

        // Gryphon (高魔)
        self.mounts.insert(
            2,
            Mount::new(
                2,
                "Gryphon",
                175, // 75% faster
                60,  // 高魔转职等级
                MountType::Gryphon,
                2282, // Gryphon Card 或物品
                1289,
            ),
        );

        // Peco Peco 2 (符文骑士)
        self.mounts.insert(
            3,
            Mount::new(
                3,
                "Peco Peco 2",
                175, // 75% faster
                70,  // 进阶骑士等级
                MountType::Peco2,
                1322, // Improved Peco Feather
                1290,
            ),
        );

        // Gryphon 2 (魔导)
        self.mounts.insert(
            4,
            Mount::new(
                4,
                "Gryphon 2",
                200, // 100% faster
                80,  // 进阶高魔等级
                MountType::Gryphon2,
                1323, // Improved Gryphon Item
                1291,
            ),
        );

        // Warg (游侠)
        self.mounts.insert(
            5,
            Mount::new(
                5,
                "Warg",
                160, // 60% faster
                65,  // 游侠等级
                MountType::Warg,
                1324, // Warg Whistle
                1292,
            ),
        );
    }

    pub fn get(&self, mount_id: u16) -> Option<&Mount> {
        self.mounts.get(&mount_id)
    }

    pub fn get_by_item_id(&self, item_id: u16) -> Option<&Mount> {
        self.mounts.values().find(|m| m.item_id == item_id)
    }

    pub fn get_by_type(&self, mount_type: MountType) -> Vec<&Mount> {
        self.mounts
            .values()
            .filter(|m| m.mount_type == mount_type)
            .collect()
    }

    pub fn all(&self) -> Vec<&Mount> {
        self.mounts.values().collect()
    }
}

impl Default for MountDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_type_conversion() {
        assert_eq!(MountType::Peco.to_id(), 1);
        assert_eq!(MountType::Gryphon.to_id(), 2);
        assert_eq!(MountType::Warg.to_id(), 5);

        assert_eq!(MountType::from_id(1), MountType::Peco);
        assert_eq!(MountType::from_id(5), MountType::Warg);
        assert_eq!(MountType::from_id(99), MountType::Other);
    }

    #[test]
    fn test_mount_type_names() {
        assert_eq!(MountType::Peco.name(), "Peco Peco");
        assert_eq!(MountType::Gryphon.name(), "Gryphon");
        assert_eq!(MountType::Other.name(), "Mount");
    }

    #[test]
    fn test_mount_creation() {
        let mount = Mount::new(1, "Test Mount", 150, 50, MountType::Peco, 1000, 2000);
        assert_eq!(mount.mount_id, 1);
        assert_eq!(mount.name, "Test Mount");
        assert_eq!(mount.speed_modifier, 150);
        assert_eq!(mount.required_level, 50);
        assert_eq!(mount.item_id, 1000);
        assert_eq!(mount.model_id, 2000);
    }

    #[test]
    fn test_mount_speed_calculation() {
        let mount = Mount::new(1, "Test", 150, 50, MountType::Peco, 0, 0);

        // 基础速度150，150% = 225
        assert_eq!(mount.calculate_speed(150), 225);

        // 基础速度200，150% = 300
        assert_eq!(mount.calculate_speed(200), 300);

        // 200%速度
        let fast_mount = Mount::new(2, "Fast", 200, 50, MountType::Other, 0, 0);
        assert_eq!(fast_mount.calculate_speed(150), 300);
    }

    #[test]
    fn test_mount_database() {
        let db = MountDatabase::new();

        // 通过ID获取
        let peco = db.get(1);
        assert!(peco.is_some());
        assert_eq!(peco.unwrap().name, "Peco Peco");

        // 通过物品ID获取
        let gryphon = db.get_by_item_id(2282);
        assert!(gryphon.is_some());
        assert_eq!(gryphon.unwrap().mount_id, 2);

        // 通过类型获取
        let peco_type = db.get_by_type(MountType::Peco);
        assert!(!peco_type.is_empty());

        // 获取所有
        let all = db.all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_mount_database_not_found() {
        let db = MountDatabase::new();

        assert!(db.get(999).is_none());
        assert!(db.get_by_item_id(9999).is_none());
    }
}
