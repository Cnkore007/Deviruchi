# Deviruchi

Rust MMORPG 服务端 + 客户端项目，基于 rAthena 协议重构，目标是构建**高性能、稳定性与可扩展性**的 Ragnarok Online 游戏体验。

## Project Goals

- **零配置运行** — 双击运行，无需安装数据库、导入 SQL、配置连接参数
- **语言级安全** — 编译期杜绝缓冲区溢出和 SQL 注入，封锁外挂攻击面
- **微秒级延迟** — 内存 channel 直连通信，公会战、多人 boss 战依然流畅
- **无缝升级** — 自动数据库迁移，SQLite 到 MySQL 一行配置，rAthena 更新后随之更新
- **配套客户端** — 独立的 Devi 客户端（Bevy 引擎），支持 Windows + macOS

## Language

### 服务端

- **Deviruchi** — Rust 实现的 MMORPG 服务端，兼容 rAthena 协议。_Avoid: 服务端/Server_
- **Session** — 客户端连接状态，包含认证阶段（Login → Char → Map）。_Avoid: 连接/Connection_
- **ChannelBus** — 基于视野半径的事件广播系统，14 格视野范围。_Avoid: 消息总线/EventBus_
- **TokenStore** — Char → Map 阶段的一次性认证令牌，30 秒过期。_Avoid: 令牌/Token_
- **Legacy Protocol** — 原有 rAthena TCP 二进制协议，端口 6900/6000/6121。_Avoid: 旧协议/Old Protocol_
- **Modern Protocol** — 新增 WebSocket 二进制协议，端口 16900/16000/16121。_Avoid: 新协议/New Protocol_

### 客户端

- **Devi** — 基于 Bevy 的跨平台客户端（Windows + macOS）。_Avoid: 客户端/Client_
- **Devi Launch** — 独立启动器，负责版本检查、资源更新、启动客户端。_Avoid: 启动器/Launcher_
- **Pixel Enhance** — Integer Scaling + xBRZ 组合像素增强方案，用户可开关。_Avoid: 像素优化/Pixel Optimization_
- **Predicted Entity** — 自己角色的客户端预测移动，服务端校正。_Avoid: 预测实体_
- **Interpolated Entity** — 其他角色/Mob 的服务端位置插值渲染。_Avoid: 插值实体_

### 资源

- **Manifest** — 资源版本清单（manifest.json），记录文件哈希用于增量更新。_Avoid: 版本文件_
- **Sprite Atlas** — 精灵动画图集（PNG + JSON），由 Python 工具从 .spr/.act 转换。_Avoid: 精灵表/Sprite Sheet_
- **Map Tileset** — 地图瓦片贴图（PNG），由 Python 工具从 GRF 导出。_Avoid: 地图贴图_

## Relationships

- **Deviruchi** serves **Legacy Protocol** on ports 6900/6000/6121
- **Deviruchi** serves **Modern Protocol** on ports 16900/16000/16121
- **Devi** connects via **Modern Protocol** to **Deviruchi**
- **Devi Launch** checks **Manifest** and updates resources before starting **Devi**
- **Devi** renders **Predicted Entity** for own character, **Interpolated Entity** for others
- **Python tools** convert GRF → **Sprite Atlas** + **Map Tileset**
- **Deviruchi** validates all game logic server-side (anti-cheat)

## Example Dialogue

> **Dev:** Devi 收到 Map Server 的位置推送后怎么渲染？
>
> **Domain Expert:** 看是谁的角色。自己的角色走 Predicted Entity，客户端先预测移动，服务端回包校正。其他玩家和 Mob 走 Interpolated Entity，直接插值到服务端推送的位置。
>
> **Dev:** 那如果预测和校正偏差很大呢？
>
> **Domain Expert:** 平滑滑回正确位置，不要瞬移。20 tick/s 的频率下偏差通常很小。

## Flagged Ambiguities

- **"客户端"** — 统一指 Devi，不要和 rAthena 原版客户端混淆。如果需要指原版客户端，明确说"rAthena 客户端"。
- **"协议"** — 必须明确是 Legacy Protocol 还是 Modern Protocol，不要笼统说"协议"。
