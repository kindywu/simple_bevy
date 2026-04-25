# simple

基于 Bevy + bevy_replicon 的多人联机游戏原型。

## 功能

- 单一二进制文件，通过命令行参数切换服务端/客户端模式
- 服务端权威架构：服务端处理所有输入并同步状态
- 支持多客户端同时连接，每个玩家拥有独立颜色
- WASD 或方向键控制方块移动

## 运行

```bash
# 启动服务端
cargo run -- server

# 启动客户端（可多开）
cargo run -- client
```

## 技术栈

- [Bevy](https://bevyengine.org/) 0.18 — 游戏引擎
- [bevy_replicon](https://github.com/projectharmonia/bevy_replicon) — 网络复制框架
- [bevy_replicon_renet](https://github.com/projectharmonia/bevy_replicon_renet) — renet 传输层

## 架构

详见 [design.md](design.md)。
