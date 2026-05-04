# 技能效果应用系统设计

## 概述

完善技能施放流程，实现冷却时间管理、距离检查和效果应用。

## 当前状态

- `data.rs` - 技能数据结构完整（4 个默认技能）
- `effect.rs` - 效果计算逻辑完整
- `handler.rs` - 施放流程骨架，但冷却检查未实现

## 缺失功能

1. **冷却时间管理** - 玩家技能冷却状态追踪
2. **施法距离检查** - 验证目标是否在技能范围内
3. **吟唱时间处理** - 延迟执行
4. **效果应用** - 技能结果应用到目标

## 设计方案

### 1. 冷却时间管理

```rust
// 每个玩家的技能冷却状态
pub struct PlayerCooldown {
    player_id: Uuid,
    cooldowns: RwLock<HashMap<u16, Instant>>,  // skill_id -> ready_time
}

impl PlayerCooldown {
    pub fn is_ready(&self, skill_id: u16) -> bool;
    pub fn set_cooldown(&self, skill_id: u16, duration_ms: u32);
    pub fn remaining_ms(&self, skill_id: u16) -> u64;
}

// SkillHandler 扩展
pub struct SkillHandler {
    db: Arc<Database>,
    cooldowns: RwLock<HashMap<Uuid, PlayerCooldown>>,  // player_id -> cooldowns
}
```

### 2. can_use_skill 检查流程

```rust
pub fn can_use_skill(&self, player: &Player, skill: &Skill, target_id: Option<Uuid>) -> Result<(), SkillError> {
    // 1. 检查 SP 消耗
    if *player.sp.read() < skill.sp_cost {
        return Err(SkillError::InsufficientSp);
    }
    
    // 2. 检查 HP 消耗
    if *player.hp.read() < skill.hp_cost {
        return Err(SkillError::InsufficientHp);
    }
    
    // 3. 检查冷却
    if let Some(cooldown) = self.cooldowns.read().get(&player.id) {
        if !cooldown.is_ready(skill.id) {
            return Err(SkillError::Cooldown(cooldown.remaining_ms(skill.id)));
        }
    }
    
    // 4. 距离检查（对敌技能需要目标）
    if skill.target == SkillTarget::Enemy || skill.target == SkillTarget::Ally {
        if let Some(target_id) = target_id {
            if !self.is_in_range(player, target_id, skill.range) {
                return Err(SkillError::OutOfRange);
            }
        }
    }
    
    Ok(())
}
```

### 3. 吟唱时间处理

```rust
pub fn cast_skill(&self, player: &Player, skill: &Skill, target_id: Option<Uuid>) -> Result<(), SkillError> {
    // 检查能否施法
    self.can_use_skill(player, skill, target_id)?;
    
    if skill.cast_time > 0 {
        // 异步延迟执行
        let handler = self.clone();
        let skill_clone = skill.clone();
        let player_id = player.id;
        let target = target_id;
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(skill_clone.cast_time)).await;
            handler.execute_skill(&player_id, &skill_clone, target);
        });
    } else {
        // 立即执行
        self.execute_skill(player, skill, target_id);
    }
}
```

### 4. 效果应用

```rust
fn execute_skill(&self, player: &Player, skill: &Skill, target_id: Option<Uuid>) {
    // 消耗 SP/HP
    *player.sp.write() -= skill.sp_cost;
    
    // 设置冷却
    if skill.cooldown > 0 {
        self.set_cooldown(player.id, skill.id, skill.cooldown);
    }
    
    // 根据技能类型应用效果
    match skill.type_ {
        SkillType::Attack => {
            if let Some(target) = self.find_target(target_id) {
                let result = SkillEffect::apply_attack(skill, &target);
                self.apply_damage(player, &target, result.damage);
            }
        }
        SkillType::Healing => {
            let result = SkillEffect::apply_healing(skill, player);
            let heal_amount = result.heal_amount;
            *player.hp.write() = (*player.hp.read() + heal_amount).min(*player.max_hp.read());
        }
        _ => {}
    }
}
```

## 文件修改

```
src/game/skill/
  ├── mod.rs       # 导出 PlayerCooldown, SkillHandler 扩展
  ├── data.rs      # Skill 结构体（已有）
  ├── handler.rs   # 添加冷却管理方法
  └── effect.rs    # SkillEffect（已有）
```

## 实现步骤

1. 添加 PlayerCooldown 结构体
2. SkillHandler 添加 cooldowns 字段
3. 实现冷却检查和设置方法
4. 完善 can_use_skill 检查
5. 实现 cast_skill 吟唱处理
6. 实现 execute_skill 效果应用
7. 添加测试
