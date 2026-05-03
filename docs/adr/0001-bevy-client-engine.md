# 选择 Bevy 作为 Devi 客户端引擎

最初考虑 Macroquad（轻量 Rust 2D 框架），但调查发现 Macroquad 不支持 CJK 输入法（IME）：macOS 完全缺失 IME 实现，Windows 仅有未发布的 PR，Linux 同样不支持。中文聊天是硬需求，且目标平台包含 macOS。Bevy 基于 winit，提供跨平台完整 IME 支持（Windows/macOS/Linux），同样使用 Rust + wgpu 渲染，与服务端技术栈一致。

**Considered Options:**
- Macroquad — 轻量但无 IME 支持
- Bevy — ECS 架构，完整 IME，wgpu 渲染
- C++ + Sokol + SDL — 极致性能但语言不一致
- Godot 4 — 开发效率高但 2D 渲染性能有瓶颈
- Unity — C# 友好但有 GC 开销和授权风险

**Consequences:** Bevy 的 ECS 架构学习曲线比 Macroquad 陡，编译时间较长，但 IME 支持是不可妥协的需求。
