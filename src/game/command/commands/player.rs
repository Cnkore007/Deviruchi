use crate::game::command::atcommand::CommandResult;
use crate::game::map::{MapState, Player};

/// @heal - 恢复 HP/SP
pub fn cmd_heal(player: &mut Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    let max_hp = *player.max_hp.read();
    let max_sp = *player.max_sp.read();

    *player.hp.write() = max_hp;
    *player.sp.write() = max_sp;

    CommandResult::Success("HP 和 SP 已回满".to_string())
}

/// @revive - 复活
pub fn cmd_revive(player: &mut Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    if player.is_alive() {
        return CommandResult::Failure("你并没有死亡".to_string());
    }

    player.respawn(*player.pos_x.read(), *player.pos_y.read());
    CommandResult::Success("已复活".to_string())
}

/// @level <level> - 设置等级
pub fn cmd_level(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @level <等级>".to_string());
    }

    let level: u16 = match args[0].parse() {
        Ok(l) => l,
        Err(_) => return CommandResult::Failure("无效的等级".to_string()),
    };

    if level > 99 {
        return CommandResult::Failure("等级不能超过 99".to_string());
    }

    *player.base_level.write() = level;
    CommandResult::Success(format!("等级设置为 {}", level))
}

/// @zeny <amount> - 增加 Zeny
pub fn cmd_zeny(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @zeny <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    player.add_zeny(amount as u64);
    CommandResult::Success(format!("获得 {} Zeny", amount))
}

/// @hp <amount> - 设置 HP
pub fn cmd_hp(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @hp <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    let max_hp = *player.max_hp.read();
    *player.hp.write() = amount.min(max_hp);
    CommandResult::Success(format!("HP 设置为 {}", amount.min(max_hp)))
}

/// @sp <amount> - 设置 SP
pub fn cmd_sp(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @sp <数量>".to_string());
    }

    let amount: u32 = match args[0].parse() {
        Ok(a) => a,
        Err(_) => return CommandResult::Failure("无效的数量".to_string()),
    };

    let max_sp = *player.max_sp.read();
    *player.sp.write() = amount.min(max_sp);
    CommandResult::Success(format!("SP 设置为 {}", amount.min(max_sp)))
}
