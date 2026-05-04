//! 战场数据定义

use std::time::Instant;

/// 战场类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BattlegroundType {
    /// 经典 PvP 模式
    /// Tierra Outpost
    TOC,
    /// Tierra Gorge
    Tierra,
    /// Freeman Gorge
    Freeman,
    /// NvsF Gorge
    Gorge,
    /// Flavius Gorge
    Flavius,
    /// 队伍对战
    TvT,
    /// 夺旗模式
    CTF,
    /// 抢夺皇冠
    SCM,
}

impl BattlegroundType {
    /// 获取战场类型的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            BattlegroundType::TOC => "Tierra Outpost",
            BattlegroundType::Tierra => "Tierra Gorge",
            BattlegroundType::Freeman => "Freeman Gorge",
            BattlegroundType::Gorge => "NvsF Gorge",
            BattlegroundType::Flavius => "Flavius Gorge",
            BattlegroundType::TvT => "Team vs Team",
            BattlegroundType::CTF => "Capture the Flag",
            BattlegroundType::SCM => "Steal the Crown Jewel",
        }
    }

    /// 获取队伍数量
    pub fn team_count(&self) -> u8 {
        match self {
            BattlegroundType::TOC => 2,
            BattlegroundType::Tierra => 2,
            BattlegroundType::Freeman => 2,
            BattlegroundType::Gorge => 2,
            BattlegroundType::Flavius => 2,
            BattlegroundType::TvT => 2,
            BattlegroundType::CTF => 2,
            BattlegroundType::SCM => 2,
        }
    }

    /// 获取默认最大玩家数
    pub fn default_max_players(&self) -> u16 {
        match self {
            BattlegroundType::TOC => 15,
            BattlegroundType::Tierra => 24,
            BattlegroundType::Freeman => 24,
            BattlegroundType::Gorge => 24,
            BattlegroundType::Flavius => 24,
            BattlegroundType::TvT => 30,
            BattlegroundType::CTF => 20,
            BattlegroundType::SCM => 20,
        }
    }

    /// 获取默认最小玩家数
    pub fn default_min_players(&self) -> u16 {
        match self {
            BattlegroundType::TOC => 3,
            BattlegroundType::Tierra => 5,
            BattlegroundType::Freeman => 5,
            BattlegroundType::Gorge => 5,
            BattlegroundType::Flavius => 5,
            BattlegroundType::TvT => 5,
            BattlegroundType::CTF => 5,
            BattlegroundType::SCM => 5,
        }
    }

    /// 获取默认时间限制（秒）
    pub fn default_time_limit(&self) -> u32 {
        match self {
            BattlegroundType::TOC => 900,     // 15分钟
            BattlegroundType::Tierra => 1800, // 30分钟
            BattlegroundType::Freeman => 1800,
            BattlegroundType::Gorge => 1800,
            BattlegroundType::Flavius => 1800,
            BattlegroundType::TvT => 1800,
            BattlegroundType::CTF => 1200, // 20分钟
            BattlegroundType::SCM => 1200,
        }
    }

    /// 获取默认分数限制
    pub fn default_score_limit(&self) -> u16 {
        match self {
            BattlegroundType::TOC => 100,
            BattlegroundType::Tierra => 300,
            BattlegroundType::Freeman => 300,
            BattlegroundType::Gorge => 300,
            BattlegroundType::Flavius => 300,
            BattlegroundType::TvT => 500,
            BattlegroundType::CTF => 3, // 夺3旗
            BattlegroundType::SCM => 1, // 抢1次皇冠
        }
    }
}

/// 战场状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlegroundState {
    /// 等待玩家加入
    Waiting,
    /// 准备开始（最低玩家数已满足）
    Ready,
    /// 战斗进行中
    Active,
    /// 战斗结束
    Ended,
}

/// 队伍颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamColor {
    Red,
    Blue,
    Green,
    Yellow,
}

impl TeamColor {
    /// 获取队伍颜色的名称
    pub fn name(&self) -> &'static str {
        match self {
            TeamColor::Red => "Red",
            TeamColor::Blue => "Blue",
            TeamColor::Green => "Green",
            TeamColor::Yellow => "Yellow",
        }
    }

    /// 根据队伍索引获取颜色
    pub fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => TeamColor::Red,
            1 => TeamColor::Blue,
            2 => TeamColor::Green,
            3 => TeamColor::Yellow,
            _ => TeamColor::Red,
        }
    }
}

/// 重生类型
#[derive(Debug, Clone, Copy)]
pub enum RespawnType {
    /// 在旗帜附近重生
    NearFlag,
    /// 在基地附近重生
    NearBase,
    /// 随机位置重生
    Random,
    /// 固定位置重生
    FixedPosition(u16, u16),
}

/// 战场队伍
#[derive(Debug, Clone)]
pub struct BattlegroundTeam {
    /// 队伍ID
    pub team_id: u32,
    /// 公会ID（用于公会战场）
    pub guild_id: Option<u32>,
    /// 玩家ID列表
    pub players: Vec<u32>,
    /// 分数
    pub score: u16,
    /// 击杀数
    pub kills: u16,
    /// 死亡数
    pub deaths: u16,
    /// 队伍颜色
    pub color: TeamColor,
}

impl BattlegroundTeam {
    /// 创建新队伍
    pub fn new(team_id: u32, color: TeamColor) -> Self {
        Self {
            team_id,
            guild_id: None,
            players: Vec::new(),
            score: 0,
            kills: 0,
            deaths: 0,
            color,
        }
    }

    /// 添加玩家到队伍
    pub fn add_player(&mut self, char_id: u32) -> bool {
        if self.players.contains(&char_id) {
            return false;
        }
        self.players.push(char_id);
        true
    }

    /// 从队伍移除玩家
    pub fn remove_player(&mut self, char_id: u32) -> bool {
        if let Some(pos) = self.players.iter().position(|&id| id == char_id) {
            self.players.remove(pos);
            true
        } else {
            false
        }
    }

    /// 检查玩家是否在队伍中
    pub fn has_player(&self, char_id: u32) -> bool {
        self.players.contains(&char_id)
    }

    /// 获取玩家数量
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// 增加击杀数
    pub fn add_kill(&mut self) {
        self.kills += 1;
    }

    /// 增加死亡数
    pub fn add_death(&mut self) {
        self.deaths += 1;
    }

    /// 增加分数
    pub fn add_score(&mut self, score: u16) {
        self.score += score;
    }
}

/// 战场
#[derive(Debug, Clone)]
pub struct Battleground {
    /// 战场ID
    pub bg_id: u32,
    /// 战场类型
    pub bg_type: BattlegroundType,
    /// 战场名称
    pub name: String,
    /// 地图名称
    pub map_name: String,
    /// 战场状态
    pub state: BattlegroundState,
    /// 队伍列表
    pub teams: Vec<BattlegroundTeam>,
    /// 每队最大玩家数
    pub max_players_per_team: u16,
    /// 每队最小玩家数
    pub min_players_per_team: u16,
    /// 重生类型
    pub respawn_type: RespawnType,
    /// 分数限制
    pub score_limit: u16,
    /// 时间限制（秒）
    pub time_limit_secs: u32,
    /// 开始时间
    pub started_at: Option<Instant>,
}

impl Battleground {
    /// 创建新战场
    pub fn new(bg_id: u32, bg_type: BattlegroundType, name: String) -> Self {
        let team_count = bg_type.team_count() as usize;
        let mut teams = Vec::with_capacity(team_count);

        for i in 0..team_count {
            teams.push(BattlegroundTeam::new(i as u32, TeamColor::from_index(i)));
        }

        Self {
            bg_id,
            bg_type,
            name,
            map_name: format!(
                "bg_{}",
                bg_type.display_name().to_lowercase().replace(' ', "_")
            ),
            state: BattlegroundState::Waiting,
            teams,
            max_players_per_team: bg_type.default_max_players(),
            min_players_per_team: bg_type.default_min_players(),
            respawn_type: RespawnType::NearBase,
            score_limit: bg_type.default_score_limit(),
            time_limit_secs: bg_type.default_time_limit(),
            started_at: None,
        }
    }

    /// 获取战场状态描述
    pub fn state_description(&self) -> &'static str {
        match self.state {
            BattlegroundState::Waiting => "Waiting for players",
            BattlegroundState::Ready => "Ready to start",
            BattlegroundState::Active => "Battle in progress",
            BattlegroundState::Ended => "Battle finished",
        }
    }

    /// 检查是否可以开始
    pub fn can_start(&self) -> bool {
        self.state == BattlegroundState::Waiting && self.all_teams_have_min_players()
    }

    /// 检查所有队伍是否都有最低玩家数
    pub fn all_teams_have_min_players(&self) -> bool {
        let min = self.min_players_per_team as usize;
        self.teams.iter().all(|t| t.player_count() >= min)
    }

    /// 获取总玩家数
    pub fn total_players(&self) -> usize {
        self.teams.iter().map(|t| t.player_count()).sum()
    }

    /// 获取玩家所在队伍
    pub fn get_player_team(&self, char_id: u32) -> Option<&BattlegroundTeam> {
        self.teams.iter().find(|t| t.has_player(char_id))
    }

    /// 获取玩家所在队伍（可变）
    pub fn get_player_team_mut(&mut self, char_id: u32) -> Option<&mut BattlegroundTeam> {
        self.teams.iter_mut().find(|t| t.has_player(char_id))
    }

    /// 添加玩家到指定队伍
    pub fn add_player_to_team(&mut self, char_id: u32, team_id: u32) -> Result<(), BGError> {
        // 检查玩家是否已在其他队伍
        if self.get_player_team(char_id).is_some() {
            return Err(BGError::PlayerAlreadyInBattle);
        }

        // 找到目标队伍
        let team = self
            .teams
            .iter_mut()
            .find(|t| t.team_id == team_id)
            .ok_or(BGError::TeamNotFound)?;

        // 检查队伍是否已满
        if team.player_count() >= self.max_players_per_team as usize {
            return Err(BGError::TeamFull);
        }

        team.add_player(char_id);
        Ok(())
    }

    /// 从战场移除玩家
    pub fn remove_player(&mut self, char_id: u32) -> bool {
        for team in &mut self.teams {
            if team.remove_player(char_id) {
                return true;
            }
        }
        false
    }

    /// 添加分数到队伍
    pub fn add_score_to_team(&mut self, team_id: u32, score: u16) -> bool {
        if let Some(team) = self.teams.iter_mut().find(|t| t.team_id == team_id) {
            team.add_score(score);
            return true;
        }
        false
    }

    /// 检查是否达到分数限制
    pub fn is_score_limit_reached(&self) -> bool {
        self.teams.iter().any(|t| t.score >= self.score_limit)
    }

    /// 获取领先的队伍
    pub fn get_leading_team(&self) -> Option<&BattlegroundTeam> {
        self.teams.iter().max_by_key(|t| t.score)
    }

    /// 获取领先的队伍ID
    pub fn get_leading_team_id(&self) -> Option<u32> {
        self.get_leading_team().map(|t| t.team_id)
    }

    /// 开始战场
    pub fn start(&mut self) {
        self.state = BattlegroundState::Active;
        self.started_at = Some(Instant::now());
    }

    /// 结束战场
    pub fn end(&mut self) {
        self.state = BattlegroundState::Ended;
    }

    /// 检查时间是否耗尽
    pub fn is_time_expired(&self) -> bool {
        if let Some(started) = self.started_at {
            let elapsed = started.elapsed().as_secs();
            elapsed >= self.time_limit_secs as u64
        } else {
            false
        }
    }

    /// 获取剩余时间（秒）
    pub fn remaining_time(&self) -> u32 {
        if let Some(started) = self.started_at {
            let elapsed = started.elapsed().as_secs() as u32;
            self.time_limit_secs.saturating_sub(elapsed)
        } else {
            self.time_limit_secs
        }
    }
}

/// 战场错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BGError {
    /// 战场不存在
    BattleNotFound,
    /// 玩家已在战场中
    PlayerAlreadyInBattle,
    /// 玩家不在战场中
    PlayerNotInBattle,
    /// 队伍已满
    TeamFull,
    /// 队伍不存在
    TeamNotFound,
    /// 玩家已在队列中
    AlreadyInQueue,
    /// 玩家不在队列中
    NotInQueue,
    /// 战场已开始
    BattleAlreadyStarted,
    /// 战场已结束
    BattleEnded,
    /// 最小玩家数不足
    NotEnoughPlayers,
    /// 战场类型不支持
    UnsupportedBattlegroundType,
    /// 战场已存在
    BattleAlreadyExists,
}

impl std::fmt::Display for BGError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BGError::BattleNotFound => write!(f, "Battle not found"),
            BGError::PlayerAlreadyInBattle => write!(f, "Player already in battle"),
            BGError::PlayerNotInBattle => write!(f, "Player not in battle"),
            BGError::TeamFull => write!(f, "Team is full"),
            BGError::TeamNotFound => write!(f, "Team not found"),
            BGError::AlreadyInQueue => write!(f, "Already in queue"),
            BGError::NotInQueue => write!(f, "Not in queue"),
            BGError::BattleAlreadyStarted => write!(f, "Battle already started"),
            BGError::BattleEnded => write!(f, "Battle ended"),
            BGError::NotEnoughPlayers => write!(f, "Not enough players"),
            BGError::UnsupportedBattlegroundType => write!(f, "Unsupported battleground type"),
            BGError::BattleAlreadyExists => write!(f, "Battle already exists"),
        }
    }
}

impl std::error::Error for BGError {}

/// 战场类型配置
#[derive(Debug, Clone)]
pub struct BattlegroundConfig {
    /// 地图名称
    pub map_name: String,
    /// 每队最大玩家数
    pub max_players: u16,
    /// 每队最小玩家数
    pub min_players: u16,
    /// 分数限制
    pub score_limit: u16,
    /// 时间限制（秒）
    pub time_limit_secs: u32,
    /// 重生位置
    pub respawn_positions: Vec<(TeamColor, u16, u16)>,
}

impl Default for BattlegroundConfig {
    fn default() -> Self {
        Self {
            map_name: String::new(),
            max_players: 15,
            min_players: 5,
            score_limit: 100,
            time_limit_secs: 900,
            respawn_positions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battleground_type_display_name() {
        assert_eq!(BattlegroundType::TOC.display_name(), "Tierra Outpost");
        assert_eq!(BattlegroundType::CTF.display_name(), "Capture the Flag");
    }

    #[test]
    fn test_battleground_team_operations() {
        let mut team = BattlegroundTeam::new(0, TeamColor::Red);
        assert_eq!(team.player_count(), 0);

        assert!(team.add_player(1));
        assert!(team.add_player(2));
        assert!(!team.add_player(1)); // 重复添加
        assert_eq!(team.player_count(), 2);

        assert!(team.has_player(1));
        assert!(team.remove_player(1));
        assert!(!team.has_player(1));
        assert_eq!(team.player_count(), 1);
    }

    #[test]
    fn test_battleground_creation() {
        let bg = Battleground::new(1, BattlegroundType::TOC, "Test BG".to_string());
        assert_eq!(bg.bg_id, 1);
        assert_eq!(bg.bg_type, BattlegroundType::TOC);
        assert_eq!(bg.name, "Test BG");
        assert_eq!(bg.teams.len(), 2);
        assert_eq!(bg.state, BattlegroundState::Waiting);
    }

    #[test]
    fn test_battleground_player_operations() {
        let mut bg = Battleground::new(1, BattlegroundType::TOC, "Test BG".to_string());

        // 添加玩家到队伍0
        assert!(bg.add_player_to_team(1, 0).is_ok());
        assert!(bg.add_player_to_team(2, 0).is_ok());

        // 尝试添加到同一个玩家
        assert_eq!(
            bg.add_player_to_team(1, 1).unwrap_err(),
            BGError::PlayerAlreadyInBattle
        );

        // 添加玩家到队伍1
        assert!(bg.add_player_to_team(3, 1).is_ok());

        assert_eq!(bg.total_players(), 3);
        assert_eq!(bg.get_player_team(1).unwrap().team_id, 0);
        assert_eq!(bg.get_player_team(3).unwrap().team_id, 1);
    }

    #[test]
    fn test_battleground_team_full() {
        let mut bg = Battleground::new(1, BattlegroundType::TOC, "Test BG".to_string());
        bg.max_players_per_team = 2;

        assert!(bg.add_player_to_team(1, 0).is_ok());
        assert!(bg.add_player_to_team(2, 0).is_ok());
        assert_eq!(bg.add_player_to_team(3, 0).unwrap_err(), BGError::TeamFull);
    }

    #[test]
    fn test_battleground_score_tracking() {
        let mut bg = Battleground::new(1, BattlegroundType::TOC, "Test BG".to_string());
        bg.score_limit = 100;

        assert!(bg.add_score_to_team(0, 10));
        assert!(bg.add_score_to_team(1, 5));
        assert!(!bg.add_score_to_team(99, 10)); // 无效队伍

        assert_eq!(bg.get_leading_team_id(), Some(0));
        assert!(!bg.is_score_limit_reached());

        assert!(bg.add_score_to_team(0, 90));
        assert!(bg.is_score_limit_reached());
    }

    #[test]
    fn test_battleground_remove_player() {
        let mut bg = Battleground::new(1, BattlegroundType::TOC, "Test BG".to_string());

        bg.add_player_to_team(1, 0).unwrap();
        assert_eq!(bg.total_players(), 1);

        assert!(bg.remove_player(1));
        assert_eq!(bg.total_players(), 0);
        assert!(!bg.remove_player(99)); // 不存在的玩家
    }

    #[test]
    fn test_team_color_from_index() {
        assert_eq!(TeamColor::from_index(0), TeamColor::Red);
        assert_eq!(TeamColor::from_index(1), TeamColor::Blue);
        assert_eq!(TeamColor::from_index(2), TeamColor::Green);
        assert_eq!(TeamColor::from_index(3), TeamColor::Yellow);
        assert_eq!(TeamColor::from_index(4), TeamColor::Red); // 循环
    }
}
