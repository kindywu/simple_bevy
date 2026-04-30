# simple

基于 Bevy + bevy_replicon 的多人联机对战游戏。工作区包含 5 个 crate。

## 功能

- **服务端权威架构**：服务端处理所有输入、碰撞、战斗、计分，并同步状态到客户端
- **多客户端联机**：支持最多 8 人同时连接，每位玩家通过黄金角度算法分配独立颜色
- **平台认证**：独立的 platform 服务，验证玩家用户名/密码和服务端 API Key
- **登录界面**：客户端两步式登录 UI（用户名 → 密码），支持中英文输入
- **战斗系统**：三角形尖端碰撞检测（近战击杀）+ 子弹射击系统，含冷却、伤害和击杀得分
- **生命值 & 重生**：每位玩家 3 点 HP，死亡后 3 秒自动安全重生
- **排行榜**：实时显示所有玩家分数排名

## 运行

```bash
# 1. 启动平台认证服务（先启动）
cargo run -p platform

# 2. 启动服务端（读取 ../.env 中的 PLATFORM_API_KEY）
cargo run -p server

# 3. 启动客户端（可多开，通过登录界面输入用户名/密码）
cargo run -p client

# 构建安全客户端（不包含服务端代码）
cargo build -p client --release

# 运行示例
cargo run -p lab
cargo run -p lab --example finance
cargo run -p lab --example single -- server
cargo run -p lab --example single -- client
```

默认用户：`kindy`、`ananda`、`martin`、`amy`（密码与用户名相同）。

## 操作

| 按键 | 动作 |
|------|------|
| WASD / 方向键 | 移动（三角形尖端朝向移动方向）|
| 空格 | 射击（从三角形尖端发射子弹）|

## 工作区结构

| Crate | 说明 |
|-------|------|
| `platform` | Axum HTTP 服务 (127.0.0.1:3001)，管理玩家凭据和 API Key 验证 |
| `shared` | 共享库：所有 ECS 组件、消息、资源和常量定义 |
| `server` | 游戏服务端：移动处理、战斗检测、子弹系统、排行榜 |
| `client` | 游戏客户端：登录 UI、键盘输入、渲染、排行榜 |
| `lab` | 示例和演示：finance 交易引擎、single 单文件版等 |

## 技术栈

- [Bevy](https://bevyengine.org/) 0.18 — 游戏引擎
- [bevy_replicon](https://github.com/projectharmonia/bevy_replicon) — 网络复制框架
- [bevy_replicon_renet](https://github.com/projectharmonia/bevy_replicon_renet) — renet 传输层
- [Axum](https://github.com/tokio-rs/axum) + Tokio — 平台 HTTP API
- ureq — 服务端→平台 HTTP 客户端
- serde + bincode / serde_json — 序列化
- rand — 安全重生点随机生成

## 架构

详见 [DESIGN.md](DESIGN.md)，详细逐行分析见 [LEARN.md](LEARN.md)。
