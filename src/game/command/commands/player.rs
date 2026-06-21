use crate::game::command::atcommand::CommandResult;
use crate::game::job::JobType;
use crate::game::map::{MapState, Player};

/// @heal - 恢复 HP/SP
pub fn cmd_heal(player: &mut Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    let max_hp = player.max_hp();
    let max_sp = player.max_sp();

    player.combat_mut().hp = max_hp;
    player.combat_mut().sp = max_sp;

    CommandResult::Success("HP 和 SP 已回满".to_string())
}

/// @revive - 复活
pub fn cmd_revive(player: &mut Player, _args: &[String], _map_state: &MapState) -> CommandResult {
    if player.is_alive() {
        return CommandResult::Failure("你并没有死亡".to_string());
    }

    player.respawn(player.pos_x(), player.pos_y());
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

    player.level_stats_mut().base_level = level;
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

    let max_hp = player.max_hp();
    player.combat_mut().hp = amount.min(max_hp);
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

    let max_sp = player.max_sp();
    player.combat_mut().sp = amount.min(max_sp);
    CommandResult::Success(format!("SP 设置为 {}", amount.min(max_sp)))
}

/// @jobchange <job_id> - 转职（GM 命令）
///
/// GM 命令跳过所有转职条件检查，直接变更职业。
/// 转职后会重置 Job 等级和经验，重算 HP/SP。
///
/// 用法: @jobchange <job_id>
/// 示例: @jobchange 7  （转职为骑士）
pub fn cmd_jobchange(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure(
            "用法: @jobchange <job_id>\n\
             常用职业 ID: 0=初心者, 1=剑士, 2=法师, 3=弓箭手, 4=服事, 5=商人, 6=盗贼\n\
             7=骑士, 8=祭司, 9=巫师, 10=铁匠, 11=猎人, 12=刺客\n\
             13=领主骑士, 14=大主教, 15=超魔导师, 16=神工匠, 17=神射手, 18=十字刺客"
                .to_string(),
        );
    }

    let target_job_id: u16 = match args[0].parse() {
        Ok(id) => id,
        Err(_) => return CommandResult::Failure("无效的职业 ID，必须是数字".to_string()),
    };

    let target_job = match JobType::from_u16(target_job_id) {
        Some(j) => j,
        None => return CommandResult::Failure(format!("无效的职业 ID: {}", target_job_id)),
    };

    let old_job = player.job();

    // 更新职业
    player.set_job(target_job_id);

    // 重置 Job 等级和经验
    {
        let mut lvl = player.level_stats_mut();
        lvl.job_level = 1;
        lvl.job_exp = 0;
    }

    // 重算最大 HP/SP
    let base_hp = target_job.base_hp();
    let base_sp = target_job.base_sp();
    let base_level = player.base_level();

    let hp_per_level = match target_job {
        JobType::Swordman | JobType::Knight | JobType::LordKnight | JobType::Paladin => 30,
        JobType::Mage | JobType::Wizard | JobType::HighWizard => 15,
        JobType::Archer | JobType::Hunter | JobType::Sniper => 20,
        JobType::Acolyte | JobType::Priest | JobType::HighPriest => 25,
        JobType::Merchant | JobType::Blacksmith | JobType::Whitesmith => 25,
        JobType::Thief | JobType::Assassin | JobType::AssassinCross => 25,
        JobType::Novice => 10,
    };

    let sp_per_level = match target_job {
        JobType::Mage | JobType::Wizard | JobType::HighWizard => 8,
        JobType::Acolyte | JobType::Priest | JobType::HighPriest => 5,
        JobType::Archer | JobType::Hunter | JobType::Sniper => 3,
        JobType::Merchant | JobType::Blacksmith | JobType::Whitesmith => 3,
        _ => 2,
    };

    let new_max_hp = base_hp + (base_level.saturating_sub(1) as u32) * hp_per_level;
    let new_max_sp = base_sp + (base_level.saturating_sub(1) as u32) * sp_per_level;

    {
        let mut combat = player.combat_mut();
        combat.max_hp = new_max_hp;
        combat.max_sp = new_max_sp;
        combat.hp = new_max_hp;
        combat.sp = new_max_sp;
    }

    // 更新最大负重
    player.update_max_weight();

    let old_name = JobType::from_u16(old_job)
        .map(|j| j.name())
        .unwrap_or("未知");
    CommandResult::Success(format!(
        "转职成功: {} -> {} (ID: {})",
        old_name,
        target_job.name(),
        target_job_id
    ))
}
