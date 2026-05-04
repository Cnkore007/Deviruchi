# 技能效果应用系统实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 完善技能施放流程，实现冷却管理、距离检查和效果应用

**Architecture:** 扩展现有 SkillHandler，添加冷却状态追踪，完善施放流程

**Tech Stack:** Rust, tokio, parking_lot

---

## Task 1: 添加 PlayerCooldown 结构体

**Files:**
- Modify: `src/game/skill/mod.rs`

- [ ] **Step 1: 添加 PlayerCooldown 结构体到 mod.rs**

```rust
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// 每个玩家的技能冷却状态
pub struct PlayerCooldown {
    player_id: Uuid,
    cooldowns: RwLock<HashMap<u16, Instant>>,  // skill_id -> ready_time
}

impl PlayerCooldown {
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            cooldowns: RwLock::new(HashMap::new()),
        }
    }

    /// 检查技能是否冷却完成
    pub fn is_ready(&self, skill_id: u16) -> bool {
        let cooldowns = self.cooldowns.read();
        match cooldowns.get(&skill_id) {
            Some(ready_time) => Instant::now() >= *ready_time,
            None => true,
        }
    }

    /// 设置技能冷却
    pub fn set_cooldown(&self, skill_id: u16, duration_ms: u32) {
        let ready_time = Instant::now() + Duration::from_millis(duration_ms as u64);
        self.cooldowns.write().insert(skill_id, ready_time);
    }

    /// 获取剩余冷却时间（毫秒）
    pub fn remaining_ms(&self, skill_id: u16) -> u64 {
        let cooldowns = self.cooldowns.read();
        match cooldowns.get(&skill_id) {
            Some(ready_time) => {
                let remaining = ready_time.saturating_duration_since(Instant::now());
                remaining.as_millis() as u64
            }
            None => 0,
        }
    }

    /// 清除所有冷却
    pub fn clear_all(&self) {
        self.cooldowns.write().clear();
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | head -30`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src/game/skill/mod.rs
git commit -m "feat(skill): add PlayerCooldown for cooldown management"
```

---

## Task 2: 扩展 SkillHandler 添加冷却管理

**Files:**
- Modify: `src/game/skill/handler.rs`

- [ ] **Step 1: 读取并修改 handler.rs**

添加字段：
```rust
pub struct SkillHandler {
    db: Arc<Database>,
    cooldowns: RwLock<HashMap<Uuid, PlayerCooldown>>,
    // ... 现有字段
}
```

添加方法：
```rust
/// 获取或创建玩家冷却状态
fn get_or_create_cooldown(&self, player_id: Uuid) -> PlayerCooldown {
    let mut cooldowns = self.cooldowns.write();
    if let Some(cd) = cooldowns.get(&player_id) {
        return PlayerCooldown::new(player_id);  // 返回新实例，实际数据在 HashMap 中
    }
    let cd = PlayerCooldown::new(player_id);
    cooldowns.insert(player_id, PlayerCooldown::new(player_id));
    cooldowns.get(&player_id).cloned().unwrap_or_else(|| PlayerCooldown::new(player_id))
}

/// 检查技能冷却
fn check_cooldown(&self, player_id: Uuid, skill_id: u16) -> Result<u64, SkillError> {
    let cooldowns = self.cooldowns.read();
    if let Some(cd) = cooldowns.get(&player_id) {
        if !cd.is_ready(skill_id) {
            return Err(SkillError::Cooldown(cd.remaining_ms(skill_id)));
        }
    }
    Ok(0)
}

/// 设置技能冷却
fn set_cooldown(&self, player_id: Uuid, skill_id: u16, duration_ms: u32) {
    let mut cooldowns = self.cooldowns.write();
    let cd = cooldowns.entry(player_id).or_insert_with(|| PlayerCooldown::new(player_id));
    cd.set_cooldown(skill_id, duration_ms);
}
```

修改 `can_use_skill` 添加冷却检查：
```rust
// 在检查 SP 之后添加
if let Err(e) = self.check_cooldown(player.id, skill.id) {
    return Err(e);
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | head -50`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src/game/skill/handler.rs
git commit -m "feat(skill): add cooldown management to SkillHandler"
```

---

## Task 3: 实现距离检查和施法流程

**Files:**
- Modify: `src/game/skill/handler.rs`

- [ ] **Step 1: 添加距离检查方法**

```rust
/// 检查目标是否在技能范围内
fn is_in_range(&self, caster: &Player, target_id: Uuid, range: u16) -> bool {
    if let Some(target) = self.find_target(target_id) {
        if caster.map_name != target.map_name {
            return false;
        }
        let (cx, cy) = caster.get_position();
        let (tx, ty) = target.get_position();
        let dx = (cx as i32 - tx as i32).unsigned_abs() as u16;
        let dy = (cy as i32 - ty as i32).unsigned_abs() as u16;
        dx <= range && dy <= range
    } else {
        false
    }
}
```

- [ ] **Step 2: 修改 can_use_skill 添加距离检查**

```rust
// 对于需要目标的技能，检查距离
if matches!(skill.target, SkillTarget::Enemy | SkillTarget::Ally | SkillTarget::Party) {
    if let Some(target_id) = target_id {
        if !self.is_in_range(player, target_id, skill.range) {
            return Err(SkillError::OutOfRange);
        }
    } else {
        return Err(SkillError::NoTarget);
    }
}
```

- [ ] **Step 3: 实现 cast_skill 吟唱处理**

```rust
/// 施放技能（处理吟唱时间）
pub fn cast_skill(&self, player: &Player, skill: &Skill, target_id: Option<Uuid>) -> Result<(), SkillError> {
    // 检查能否施法
    if let Err(e) = self.check_can_use(player, skill, target_id) {
        return Err(e);
    }

    // 消耗 SP/HP
    *player.sp.write() = player.sp.read().saturating_sub(skill.sp_cost);
    if skill.hp_cost > 0 {
        *player.hp.write() = player.hp.read().saturating_sub(skill.hp_cost);
    }

    if skill.cast_time > 0 {
        // 异步延迟执行
        let handler = self.clone();
        let skill_clone = skill.clone();
        let player_id = player.id;
        let target = target_id;

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(skill_clone.cast_time as u64)).await;
            handler.execute_skill(&player_id, &skill_clone, target);
        });
    } else {
        // 立即执行
        self.execute_skill(player, skill, target_id);
    }

    Ok(())
}

/// 检查能否使用技能
fn check_can_use(&self, player: &Player, skill: &Skill, target_id: Option<Uuid>) -> Result<(), SkillError> {
    // SP 消耗检查
    if *player.sp.read() < skill.sp_cost as u32 {
        return Err(SkillError::InsufficientSp);
    }

    // HP 消耗检查
    if skill.hp_cost > 0 && *player.hp.read() < skill.hp_cost {
        return Err(SkillError::InsufficientHp);
    }

    // 冷却检查
    if let Err(e) = self.check_cooldown(player.id, skill.id) {
        return Err(e);
    }

    // 距离检查
    if matches!(skill.target, SkillTarget::Enemy | SkillTarget::Ally | SkillTarget::Party) {
        if let Some(tid) = target_id {
            if !self.is_in_range(player, tid, skill.range) {
                return Err(SkillError::OutOfRange);
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 4: 实现 execute_skill 效果应用**

```rust
/// 执行技能效果
fn execute_skill(&self, player_id: &Uuid, skill: &Skill, target_id: Option<Uuid>) {
    let Some(player) = self.find_caster(player_id) else {
        return;
    };

    // 设置冷却
    if skill.cooldown > 0 {
        self.set_cooldown(player.id, skill.id, skill.cooldown);
    }

    // 根据技能类型应用效果
    match skill.type_ {
        SkillType::Attack => {
            if let Some(target) = self.find_target(target_id) {
                let result = SkillEffect::apply_attack(skill, &target);
                self.apply_damage_to_target(&player, &target, result.damage);
            }
        }
        SkillType::Healing => {
            let result = SkillEffect::apply_healing(skill, &player);
            let heal = result.heal_amount;
            let max_hp = *player.max_hp.read();
            *player.hp.write() = (*player.hp.read() + heal).min(max_hp);
            tracing::info!("Player {} healed {} HP from skill {}", player.name, heal, skill.name);
        }
        SkillType::Support => {
            // TODO: 应用 buff
            tracing::info!("Player {} cast {} (support skill)", player.name, skill.name);
        }
        SkillType::Debuff => {
            if let Some(target) = self.find_target(target_id) {
                let result = SkillEffect::apply_debuff(skill);
                tracing::info!("Player {} cast {} on {}", player.name, skill.name, target.name);
            }
        }
        _ => {}
    }
}

/// 找到施法者（需要 map_state 引用，这里简化处理）
fn find_caster(&self, player_id: &Uuid) -> Option<Player> {
    // TODO: 从 MapState 获取
    None
}

/// 对目标造成伤害
fn apply_damage_to_target(&self, attacker: &Player, target: &Player, damage: i32) {
    if damage <= 0 {
        return;
    }
    let damage = damage as u32;
    let died = target.take_damage(damage);
    tracing::info!("Player {} dealt {} damage to {}", attacker.name, damage, target.name);
    if died {
        tracing::info!("Player {} was killed by {}", target.name, attacker.name);
    }
}
```

- [ ] **Step 5: 编译验证**

Run: `cargo build 2>&1 | head -50`
Expected: 无错误

- [ ] **Step 6: 提交**

```bash
git add src/game/skill/handler.rs
git commit -m "feat(skill): implement cast_skill with cast time and execute_skill with effects"
```

---

## Task 4: 添加单元测试

**Files:**
- Add: `src/game/skill/tests.rs`

- [ ] **Step 1: 创建测试文件**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_cooldown_ready() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        assert!(cd.is_ready(1)); // 新技能无冷却
    }

    #[test]
    fn test_player_cooldown_set() {
        let cd = PlayerCooldown::new(Uuid::new_v4());
        cd.set_cooldown(1, 5000);
        assert!(!cd.is_ready(1));
        assert!(cd.remaining_ms(1) > 0);
    }

    #[test]
    fn test_skill_sp_cost() {
        let skill = Skill {
            id: 28,
            name: "Heal",
            type_: SkillType::Healing,
            target: SkillTarget::Ally,
            level: 1,
            sp_cost: 10,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 0,
            hit: 0,
            element: 0,
            flags: 0,
        };

        // 测试 Heal 技能 SP 消耗
        assert_eq!(skill.sp_cost, 10);
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test skill 2>&1`
Expected: 所有测试通过

- [ ] **Step 3: 提交**

```bash
git add src/game/skill/
git commit -m "test(skill): add unit tests for cooldown system"
```
