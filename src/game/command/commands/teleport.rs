use crate::game::map::{Player, MapState};
use crate::game::command::atcommand::CommandResult;

/// @warp <map> [x] [y] - 传送到地图
pub fn cmd_warp(player: &mut Player, args: &[String], _map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @warp <地图> [x] [y]".to_string());
    }

    let map_name = &args[0];
    let x: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let y: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);

    player.map_name = map_name.clone();
    player.move_to(x, y);

    CommandResult::Success(format!("传送到 {} ({}, {})", map_name, x, y))
}

/// @goto <player> - 传送到玩家位置
pub fn cmd_goto(player: &mut Player, args: &[String], map_state: &MapState) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Failure("用法: @goto <玩家名>".to_string());
    }

    let target_name = &args[0];
    let target = map_state.find_player_by_name(target_name);

    match target {
        Some(target) => {
            player.map_name = target.map_name.clone();
            let (x, y) = target.get_position();
            player.move_to(x, y);
            CommandResult::Success(format!("传送到 {} 的位置 ({}, {})", target_name, x, y))
        }
        None => CommandResult::Failure(format!("找不到玩家: {}", target_name)),
    }
}
