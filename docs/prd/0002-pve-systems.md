# PvE Systems: Monster Drop Tables, Experience Distribution, and Death & Respawn

## Problem Statement

Deviruchi currently has scattered drop, death, and experience logic without proper integration. MobAI does not trigger drop generation or experience distribution when a mob dies. Player death events are not published to the ChannelBus, so other players don't see death notifications. The existing `DropManager` lacks integration with mob drop tables. The `ExpDistributor` exists but is not wired into the death flow.

## Solution

Implement three integrated PvE systems:

1. **Mob Drop Table System** — YAML-based drop tables with MVP (most valuable player) loot assignment, treasure chest drops, and item drop broadcast via ChannelBus
2. **Experience Distribution System** — Connect `ExpDistributor` to MobAI death flow with party sharing, level-based splitting, and MVP bonus
3. **Death & Respawn System** — Player death with 1% EXP penalty, Zeny drop, respawn options (normal respawn / instant call), death/respawn event broadcast

## User Stories

### Mob Drop Tables

1. As a player, when I kill a mob, I want it to drop items based on a configured drop table so that different mobs have different loot
2. As a player, when multiple players participate in killing a mob, I want the MVP to receive bonus drops so that contribution is rewarded
3. As a player, when a mob drops an item, other players within vision range see the item appear on the ground so that loot competition is visible
4. As a player, when a mob drops Zeny, it is distributed among participating players so that Zeny is shared fairly
5. As a player, when I am within pickup range of a dropped item, I want to be able to pick it up so that I can collect my loot
6. As an admin, I want to configure drop tables via YAML so that I can adjust loot rates without recompiling
7. As an admin, I want monster level to affect drop rate and Zeny amount so that higher-level mobs are more rewarding
8. As a player, when a dropped item expires (5 minutes), it disappears so that the world doesn't fill with abandoned loot

### Experience Distribution

9. As a solo player, when I kill a mob, I want to receive 100% of the experience so that solo grinding is viable
10. As a party member, when we kill a mob, I want experience split equally among nearby party members so that party play is rewarding
11. As a party member, when we kill a mob, I want higher-level members to receive more experience in level-based mode so that helping lower-level players is encouraged
12. As a player, when I kill a mob far above my level, I want reduced experience so that farming high-level mobs for easy EXP is discouraged
13. As a player, when I kill a mob far below my level, I want reduced experience so that power-leveling trivial mobs is discouraged
14. As a party leader, when we kill a mob, I want MVP bonus experience so that the player who dealt the most damage is rewarded
15. As a player, when I gain enough experience, I want my level to increase so that progression feels meaningful

### Death & Respawn

16. As a player, when my HP reaches 0, I want to enter a dead state so that I cannot act until revived
17. As a player, when I die, I want to lose 1% of my current base and job experience so that death has consequence
18. As a player, when I die, I want nearby players to see my death notification so that they know what happened
19. As a player, when I die, I want to choose between normal respawn and instant call so that I have control over my respawn
20. As a player, when I use normal respawn, I want to respawn at a SavePoint so that I am returned to a safe location
21. As a player, when I use instant call, I want to respawn at the nearest navigation point so that I can return to combat quickly
22. As a player, when I respawn, I want my HP and SP to be fully restored so that I am ready to continue
23. As a player, when I die in a party, I want to respawn independently without affecting other party members so that one death doesn't disrupt the party
24. As a developer, I want death and respawn events published to ChannelBus so that other players see the notification
25. As a developer, I want respawn position logic centralized in a RespawnService so that respawn rules can be configured per map

## Implementation Decisions

### Modules to Build/Modify

#### 1. New Module: `src/game/mob/droptable.rs`

Drop table loader and resolver.

- `DropTableEntry`: item_id, min_amount, max_amount, chance (basis points), is_zeny, is_mvp_bonus
- `MobDropTable`: HashMap<MobTypeId, Vec<DropTableEntry>>
- `DropTableLoader::load_from_yaml(path)` → MobDropTable
- `DropResolver::resolve(droptable, rng, mob_level) → Vec<DropItem>` — rolls against each entry, returns actual drops
- `MVPResolver::pick_mvp(damage_dealt) → PlayerId` — selects highest damage dealer from damage log

**Interface:**
```rust
pub struct DropTableEntry {
    pub item_id: u32,
    pub min_amount: u16,
    pub max_amount: u16,
    pub chance: u32, // basis points (0-10000)
    pub is_zeny: bool,
    pub is_mvp_bonus: bool,
}

pub struct DropResolver;
impl DropResolver {
    pub fn resolve(&self, table: &[DropTableEntry], rng: &dyn GameRng, mob_level: u16) -> Vec<DropItem>;
}
```

#### 2. Modify: `src/game/mob/ai.rs`

Wire death flow into drop + exp distribution.

- Add `damage_log: HashMap<Uuid, u64>` to track damage per player during combat
- `MobAI::update_dead()`: resolve drops → add to DropManager → publish `ItemDrop` events → call `ExpDistributor::distribute_mob_exp()` → publish `MobDeath`
- Integrate with `ChannelBus` for drop event broadcast
- Integrate with `DropManager` for ground item creation

**Interface change:**
- `MobAI::update` accepts `drop_resolver: &DropResolver`, `exp_distributor: &ExpDistributor`, `drop_manager: &DropManager`, `channel_bus: &ChannelBus`

#### 3. Modify: `src/game/battle/exp.rs`

Enhance `ExpDistributor` with MVP support.

- Add `mvp_id: Option<Uuid>` parameter to `distribute_mob_exp`
- MVP gets bonus experience share (configurable, default 1.1x)
- Add `DistributeZeny(mob_level, participants)` method

**Interface change:**
```rust
pub fn distribute_mob_exp(
    map_state: &MapState,
    party_manager: &PartyManager,
    killer_id: Uuid,
    mvp_id: Option<Uuid>,
    mob_level: u16,
    mob_base_exp: u64,
    mob_job_exp: u64,
);
```

#### 4. Modify: `src/game/map/mod.rs` — Add RespawnService

Centralize respawn position logic.

- `RespawnService::new(save_point_db)` — loads save points from DB
- `RespawnService::get_respawn_position(player, respawn_type) → (x, y, map_name)`
- `RespawnType::Normal` → SavePoint lookup
- `RespawnType::InstantCall` → nearest navigation point (or spawn point)
- Map-specific respawn rules via config

**Interface:**
```rust
pub enum RespawnType {
    Normal,
    InstantCall,
}

pub struct RespawnService;
impl RespawnService {
    pub fn get_respawn_position(&self, player: &Player, respawn_type: RespawnType) -> (u16, u16, String);
}
```

#### 5. Modify: `src/game/map/player.rs`

Enhance death/respawn with Zeny drop.

- `Player::die()` already sets state + applies 1% EXP penalty — keep this
- Add `Player::drop_zeny_on_death(amount: u32) -> u32` — drops 50% of Zeny (configurable), returns actual dropped amount
- Add `Player::get_respawn_type() -> RespawnType` — player can select, defaults to Normal
- `Player::respawn_at(position)` already exists — use `RespawnService` to get position

#### 6. Modify: `src/game/map/channel.rs`

Ensure death/respawn events exist and are broadcast-ready.

- `GameEvent::PlayerDeath { player_id }` — already exists
- `GameEvent::PlayerRevive { player_id, x, y }` — already exists
- `GameEvent::ItemDrop { item_id, x, y, amount }` — already exists
- `GameEvent::ItemPickup { player_id, item_id, amount }` — already exists
- Ensure `MobDeath` includes killer_id and mob position

#### 7. Modify: `src/game/map/drop_item.rs`

Connect DropManager to ChannelBus.

- Add `DropManager::add_with_broadcast(item_id, amount, x, y, map_name, channel_bus)` → adds + publishes ItemDrop
- Drop pickup broadcasts `ItemPickup` event

#### 8. Modify: `src/game/game_loop.rs`

Wire everything together.

- GameLoop tick calls `MobAI::update` with all dependencies (drop_resolver, exp_distributor, drop_manager, channel_bus)
- GameLoop tick calls `DropManager::cleanup_expired` and publishes removal events
- GameLoop tick calls `MobSpawnManager::check_respawn` for dead mob respawn timing

### API Contracts

- `MobAI::update_dead()` is called when mob HP reaches 0 — triggers full death sequence (drops + exp + events)
- `Player::die()` is called when player HP reaches 0 — triggers death state, EXP penalty, Zeny drop
- `Player::respawn()` is called by player input or auto-respawn timer — restores HP/SP at resolved position
- `DropTableLoader` reads YAML at startup, panics on parse error (fail-fast, no silent defaults for drop tables)
- `RespawnService::get_respawn_position()` returns a valid position for all players (fallback to map spawn point if no save point)

### Configuration (via `config/game.yaml`)

```yaml
drop:
  mvp_bonus_multiplier: 1.1  # MVP gets 10% bonus EXP
  zeny_drop_rate: 5000       # basis points for Zeny drop (50% chance)
  zeny_drop_percent: 50      # percent of Zeny dropped on death
  pickup_range: 2            # tiles

exp:
  level_penalty:
    diff_10: 1.0
    diff_15: 0.75
    diff_20: 0.5
    diff_25: 0.25
    diff_above: 0.1

respawn:
  normal_respawn_delay_ms: 5000
  instant_call_delay_ms: 1000
  default_map: "prontera"
  default_x: 157
  default_y: 183
```

## Testing Decisions

### What Makes a Good Test

- Test external behavior: mob dies → items appear in DropManager, experience added to correct players
- Do not test implementation details (e.g., do not assert internal damage_log state)
- Use deterministic RNG for repeatable results

### Modules to Test

#### 1. DropResolver (pure logic)

- Single entry with 100% chance → always drops
- Single entry with 0% chance → never drops
- Range amount (min 1, max 5) → amount within range
- Multiple entries → each rolls independently
- MVP bonus entry → only included if mvp_id provided

**Prior art**: `DropManager::add_creates_drop_item_and_returns_id` in drop_item.rs

#### 2. ExpDistributor (integration)

- Solo kill → 100% to killer
- Party equal split → divided by party size (nearby members only)
- Party level-based → weighted by level
- MVP bonus → MVP gets bonus multiplier
- Level penalty → diff > 10 applies correct coefficient

**Prior art**: `test_solo_exp_distribution`, `test_level_penalty_reduces_exp` in battle/exp.rs

#### 3. RespawnService (pure logic)

- Player with save point → returns save point position
- Player without save point → returns map default
- Instant call → returns nearest navigation point
- Invalid map → falls back to global default

#### 4. Player death/respawn (unit)

- `die()` → state = Dead, HP = 0, EXP reduced by 1%
- `respawn()` → state = Alive, HP = max_hp, SP = max_sp
- `drop_zeny_on_death()` → correct percentage dropped
- Small EXP → saturating_sub, no negative

**Prior art**: `test_player_die_sets_state_dead`, `test_player_respawn_restores_hp_and_state` in player.rs

#### 5. Integration: Mob death → drops + exp (integration test)

- Mob dies → DropManager contains expected items
- Mob dies → correct players receive EXP
- Drop pickup → item removed from DropManager, player inventory updated

### Testing Approach

- **Unit tests**: Pure logic modules (DropResolver, ExpDistributor math, RespawnService)
- **Integration tests**: Full death flow (MobAI death → drops + exp + events)
- **Mock strategy**: Use `thread_rng` in integration tests, deterministic `SmallRng` for unit tests

## Out of Scope

- Item pickup to player inventory (inventory system is separate)
- Equipment durability loss on death
- Abnormal status effects on death (curses, etc.)
- PvP death mechanics (separate system)
- Monster card drop mechanics
- Enchanted treasure chest (item-grade system)
- Auto-loot / loot window UI
- Resurrection magic / item resurrection
- Resurrection penalty stacking (multiple deaths)
- Map-specific respawn restrictions (e.g., boss room)
- GM force-respawn commands

## Further Notes

### Integration Points

| Flow | Trigger | Actions |
|------|---------|---------|
| MobDeath | MobAI HP → 0 | Roll drops → DropManager → ItemDrop event → ExpDistributor → MobDeath event → MobSpawnManager start respawn timer |
| PlayerDeath | Player HP → 0 | Player.die() → EXP penalty → Zeny drop → PlayerDeath event → enable respawn UI |
| PlayerRespawn | Player input / auto-timer | RespawnService.resolve_position → Player.respawn() → PlayerRevive event |
| ItemPickup | Player within range + input | DropManager.pickup → ItemPickup event → inventory update |

### Phase Order

1. **Phase 1: Drop Table System** — Build DropTableLoader, DropResolver, wire into MobAI::update_dead
2. **Phase 2: Experience Distribution** — Wire ExpDistributor into MobAI death, add MVP support, add Zeny distribution
3. **Phase 3: Death & Respawn** — Enhance Player death, build RespawnService, wire death/respawn events to ChannelBus

### Prior Art in Codebase

- `DropManager` and `DropItem` already exist in `drop_item.rs` with tests
- `Player::die()`, `Player::respawn()`, `PlayerState` already exist in `player.rs` with tests
- `ExpDistributor` with tests exists in `battle/exp.rs`
- `ChannelBus::publish` with vision filtering exists in `channel.rs`
- `GameEvent` variants already cover all needed events
