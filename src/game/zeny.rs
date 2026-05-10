use crate::game::map::Player;

pub use super::constants::MAX_ZENY;

pub struct ZenyManager;

impl ZenyManager {
    /// 增加Zeny，返回实际增加数量
    pub fn add(player: &Player, amount: u32) -> u32 {
        let mut eco = player.economy_mut();
        let can_add = MAX_ZENY - eco.zeny;
        let actual_add = amount.min(can_add);
        eco.zeny += actual_add;
        actual_add
    }

    /// 扣除Zeny，返回是否成功
    pub fn sub(player: &Player, amount: u32) -> bool {
        let mut eco = player.economy_mut();
        if eco.zeny >= amount {
            eco.zeny -= amount;
            true
        } else {
            false
        }
    }

    /// 检查是否足够（注意：此检查不具有原子性保证，仅用于快速预检）
    pub fn can_spend(player: &Player, amount: u32) -> bool {
        player.zeny() >= amount
    }

    /// 获取当前Zeny
    pub fn get(player: &Player) -> u32 {
        player.zeny()
    }

    /// 设置Zeny（用于初始化）
    pub fn set(player: &Player, amount: u32) {
        player.economy_mut().zeny = amount.min(MAX_ZENY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Character;

    fn create_test_player() -> Player {
        let char = Character {
            char_id: 1,
            char_num: 0,
            name: "Test".to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 1000,
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            hair: 0,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 50,
            last_y: 50,
            save_map: "new_1-1.gat".to_string(),
            save_x: 50,
            save_y: 50,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        };
        Player::from_character(char)
    }

    #[test]
    fn test_add_zeny() {
        let player = create_test_player();
        let added = ZenyManager::add(&player, 500);
        assert_eq!(added, 500);
        assert_eq!(ZenyManager::get(&player), 1500);
    }

    #[test]
    fn test_add_zeny_capped() {
        let player = create_test_player();
        player.economy_mut().zeny = MAX_ZENY - 100;
        let added = ZenyManager::add(&player, 500);
        assert_eq!(added, 100);
        assert_eq!(ZenyManager::get(&player), MAX_ZENY);
    }

    #[test]
    fn test_sub_zeny() {
        let player = create_test_player();
        assert!(ZenyManager::sub(&player, 500));
        assert_eq!(ZenyManager::get(&player), 500);
    }

    #[test]
    fn test_sub_zeny_insufficient() {
        let player = create_test_player();
        assert!(!ZenyManager::sub(&player, 2000));
        assert_eq!(ZenyManager::get(&player), 1000); // unchanged
    }

    #[test]
    fn test_can_spend() {
        let player = create_test_player();
        assert!(ZenyManager::can_spend(&player, 500));
        assert!(!ZenyManager::can_spend(&player, 2000));
    }

    #[test]
    fn test_set_zeny() {
        let player = create_test_player();
        ZenyManager::set(&player, 5000);
        assert_eq!(ZenyManager::get(&player), 5000);
    }

    #[test]
    fn test_set_zeny_capped() {
        let player = create_test_player();
        ZenyManager::set(&player, MAX_ZENY + 100);
        assert_eq!(ZenyManager::get(&player), MAX_ZENY);
    }

    #[test]
    fn test_player_add_zeny_no_overflow() {
        let player = create_test_player();
        player.economy_mut().zeny = MAX_ZENY - 100;
        player.add_zeny(u64::MAX);
        assert_eq!(player.zeny(), MAX_ZENY);
    }
}
