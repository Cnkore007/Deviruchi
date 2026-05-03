# 怪物寻路系统 (A* Pathfinding) 设计

## 目标

为怪物追击行为实现 A* 寻路算法，支持八方向移动和路径缓存，替代现有的简单 `approach()` 直线追击。

## 架构

### 数据结构

**`MobPathManager`** — 管理单个怪物的路径缓存
```rust
pub struct MobPathManager {
    pub is_chasing: bool,                  // 是否正在追击
    pub target_pos: Option<(u16, u16)>,    // 追击目标坐标
    pub cached_path: Vec<(u16, u16)>,      // 缓存的路径点（不含起点）
    pub current_step: usize,                // 当前路径索引
    pub path_invalid: bool,                 // 路径是否失效
}
```

**Mob 结构体新增字段**
```rust
pub struct Mob {
    // ... 现有字段 ...
    pub path_manager: RwLock<MobPathManager>,
}
```

### 寻路算法

- **库**: `pathfinding` crate
- **算法**: A* (使用 `astar()` 函数)
- **启发函数**: 欧几里得距离 (适配八方向)
- **搜索范围**: 以 Mob 当前位置为中心，`chase_range` 为半径的矩形区域
- **方向**: 八方向 (cardinal + diagonal)

### 斜角移动规则 (严格模式)

当移动方向为斜角时，必须同时满足：
1. 目标格 `is_walkable`
2. 相邻的两个 cardinal 格都 `is_walkable`

**示例**: 向东南 `(x+1, y+1)` 移动时，必须检查:
- `(x+1, y+1)` — 目标格
- `(x+1, y)` — 东侧
- `(x, y+1)` — 南侧

三个格都 walkable 才能允许斜角移动。

### 路径缓存策略

1. **开始追击** → 以 Mob 当前位置为起点，目标玩家位置为终点，计算 A* 路径
2. **每 tick 移动** → 沿缓存路径走一步
3. **路径有效** → 移动到下个格，`current_step++`
4. **路径失效** (下个格不可通行) → 停止移动，`path_invalid = true`
5. **下次 tick** → 如果 `path_invalid = true`，重算路径

### 路径失效兜底

寻路失败或目标不可达时：
- 设置 `is_chasing = false`，怪物回到 Idle 状态
- 不使用 `approach()` 直线追

## 集成点

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 `pathfinding` 依赖 |
| `src/game/mob/data.rs` | 添加 `MobPathManager` struct，`Mob` 添加 `path_manager` 字段 |
| `src/game/mob/ai.rs` | 重写 `update_chase()` 使用 `MobPathManager` |

## 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 搜索范围 | chase_range 内搜索 | 怪物追击有范围限制，超出范围会放弃 |
| 斜角规则 | 严格模式 | 防止穿墙角 bug |
| 失效处理 | 等待重算 | 地图障碍变化是小概率事件 |
| 路径节点 | 含起点不含终点 | 移动逻辑更清晰 |

## 测试计划

- 单元测试: A* 寻路正确性（直线、L形、U形障碍）
- 单元测试: 斜角规则正确性
- 单元测试: 路径缓存和失效逻辑
- 集成测试: MobAI chase 状态切换
