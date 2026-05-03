# Deviruchi 服务端内建双协议支持

选择在 Deviruchi 服务端内建 Legacy Protocol（TCP）和 Modern Protocol（WebSocket）双协议支持，通过配置开关区分，而非独立 Gateway 进程。端口偏移 +10000 区分：Legacy 6900/6000/6121，Modern 16900/16000/16121。

**Considered Options:**
- 独立 Gateway 进程 — 解耦但多一个部署单元
- 服务端内建双协议 — 部署简单，共享 TokenStore 和 Session 逻辑
- 单端口 + 包路由 — 客户端简单但服务端复杂

**Consequences:** 服务端代码复杂度增加（两套网络 IO 路径），但共享认证/会话逻辑避免了 Gateway 的重复实现。未来如果 WebSocket 流量压力过大，可以拆出独立 Gateway，客户端无需改动。
