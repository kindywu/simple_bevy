# AGENTS.md

本文件为 AI 编程助手提供项目背景、构建命令、代码风格与架构速查。阅读本文件前，默认你对本项目一无所知。

## 项目概述

这是一个基于 **Bevy 0.18** + **bevy_replicon** 的多人联机对战游戏，采用 **服务端权威架构**（Server-Authoritative）。玩家控制三角形移动和射击，通过尖端碰撞或子弹击杀其他玩家得分，死亡后 3 秒自动重生。

项目使用 **Cargo Workspace** 管理 5 个 crate，所有 crate 均使用 **Rust Edition 2024**。

## 相关文档

项目根目录下还有两份面向不同读者的文档，AI 助手在深入代码前可根据目的选择阅读：

| 文件 | 定位 | 内容特点 |
|------|------|----------|
| `DESIGN.md` | **系统设计文档** | 聚焦架构全貌：工作区结构图、认证流程时序图、组件/消息/资源对照表、服务端系统链说明、网络数据流、战斗与渲染机制。适合快速掌握模块划分和接口契约。 |
| `LEARN.md` | **初学者入门教程** | 从零开始的学习路线：先跑通游戏，再逐文件逐函数讲解代码，穿插知识点提示（ECS、Bevy 渲染、网络同步、Rust 语法等）。适合第一次接触本项目或 Bevy 的开发者。 |

> 若需要理解**某个模块为什么这样设计**或**各模块如何协作**，优先读 `DESIGN.md`；若需要**理解每一行代码在做什么**或**学习涉及的 Rust/Bevy 知识点**，优先读 `LEARN.md`。

## 技术栈

| 技术 | 版本/说明 | 用途 |
|------|-----------|------|
| `bevy` | 0.18.1 | 游戏引擎（ECS、渲染、UI） |
| `bevy_replicon` | 0.39.4 | 网络复制框架（组件同步） |
| `bevy_renet` | 4.0.1 | renet 传输层 |
| `bevy_replicon_renet` | 0.15.0 | 两者适配层 |
| `axum` / `axum-server` | 0.8.9 / 0.7 | Platform HTTPS REST API |
| `tokio` | 1.x | Platform 异步运行时 |
| `ureq` + `rustls` | 2.x / 0.23 | 服务端 → Platform 的 HTTPS 客户端 |
| `serde` / `serde_json` | workspace | JSON / 二进制序列化 |
| `rand` | 0.10.1 | 随机数（重生点等） |
| `sha2` + `hex` | — | Platform 密码哈希（SHA-256） |
| `sled` | 0.34.7 | `lab/examples/finance.rs` 持久化 |

## 工作区结构

```
.
├── platform/   # 独立 HTTPS 认证服务（Axum + TLS，端口 3001）
├── shared/     # 共享库：ECS 组件、网络消息、资源、常量
├── server/     # 游戏服务端（Bevy App，UDP 端口 5000）
├── client/     # 游戏客户端（Bevy App，登录 UI + 渲染）
└── lab/        # 示例与演示（不依赖其他 crate）
```

依赖关系：`shared` 被 `server` 和 `client` 路径依赖；`platform` 完全独立；`lab` 完全独立。

### 各 Crate 说明

- **`shared`**：只包含数据定义（`src/lib.rs`，84 行）。所有需要网络同步的 Component、Message、Resource 和常量必须定义在这里。如果修改了复制类型，**务必同步修改** `server/src/main.rs` 和 `client/src/main.rs` 中的 `app.replicate::<T>()` / `app.add_client_message::<T>()` 注册。
- **`server`**：游戏逻辑核心。子模块包括：
  - `auth.rs` — 调用 Platform HTTPS API 验证 API Key 与玩家凭据（自定义 rustls TLS Agent，信任 mkcert CA + 系统根证书）。
  - `bullet.rs` — 射击冷却、子弹发射、移动、碰撞、生命周期。
  - `combat.rs` — 三角形尖端碰撞检测（重心坐标法）、安全重生点计算。
  - `render.rs` — 相机、网格创建、Transform 同步。
  - `scoreboard.rs` — 居中排行榜 UI。
- **`client`**：渲染与输入。子模块包括：
  - `login.rs` — 两步登录 UI（用户名 → 密码），通过 `user_data[256]` 携带凭据发起 renet 连接。
  - `render.rs` — 本地玩家标记 `LocalPlayer`、网格创建、Transform 同步、死亡可见性控制。
  - `scoreboard.rs` — 右上角排行榜。
- **`platform`**：单文件 `src/main.rs`。提供 `POST /api/auth/verify-key`、`POST /api/auth/login`、`GET /api/health`。玩家凭据存于 `players.json`（SHA-256 哈希），API Key 存于 `api_keys.json`。
- **`lab`**：示例集合，通过 `cargo run -p lab --example <name>` 运行：
  - `finance` — ECS 撮合引擎 + sled 持久化 + axum REST API。
  - `single` — 单文件版多人游戏（无模块拆分）。
  - `simple_finance` — 无外部依赖的简化 ECS 金融演示。
  - `server` / `client` — 最小 UDP Socket 测试。

## 构建与运行

### 前置条件（首次）

Platform 使用 TLS，需要 mkcert 生成本地证书：

```bash
# Windows
winget install FiloSottile.mkcert
mkcert -install
cd platform/certs && mkcert localhost 127.0.0.1 ::1 && cd ../..
```

### 常用命令

```bash
# 构建整个工作区
cargo build --workspace

# 1. 启动认证服务（必须先运行，HTTPS://127.0.0.1:3001）
cargo run -p platform

# 2. 启动游戏服务端（读取 ../.env 中的 PLATFORM_API_KEY）
cargo run -p server

# 3. 启动客户端（可多开，通过登录界面输入用户名/密码）
cargo run -p client

# 构建安全客户端（不包含服务端代码）
cargo build -p client --release

# 运行 lab 示例
cargo run -p lab
cargo run -p lab --example finance
cargo run -p lab --example single -- server
cargo run -p lab --example single -- client

# 快速启动测试（PowerShell）
./start-test.ps1
```

**启动顺序不可颠倒**：Platform → Server → Client。

### 默认测试账号

`kindy`、`ananda`、`martin`、`amy`（密码与用户名相同）。
默认 API Key 定义在 `shared/src/lib.rs` 的 `PLATFORM_API_KEY`，可被项目根目录 `.env` 文件中的同名变量覆盖。

## 代码风格与约定

### 语言与注释

- 文档和代码注释以 **中文** 为主（`DESIGN.md`、`LEARN.md`、log 输出、UI 文本）。
- 变量/函数命名使用英文，遵循 Rust snake_case。

### ECS 约定

- **服务端系统链**：`Update` schedule 中的系统使用 `.chain()` 强制按顺序执行，避免帧内竞态。顺序如下：
  ```
  spawn_render → tick_cooldowns → server_handle_input → server_handle_shoot
    → spawn_bullet_render → move_bullets → clamp_positions
    → bullet_player_collision → combat_detection → bullet_lifetime
    → respawn_dead_players → apply_position → apply_bullet_position
    → update_visibility → update_scoreboard
  ```
- **客户端状态门控**：使用 `GameState`（`Login` / `InGame`）配合 `run_if(in_state(...))`，确保登录 UI 系统仅在登录态运行，游戏系统仅在游戏态运行。
- **Observer**：服务端使用 `app.add_observer(server_on_connect)` 监听 `On<Add, ConnectedClient>`，在客户端连入时触发认证与玩家生成。
- **标记组件**：广泛使用零大小标记组件，例如 `Dead`、`SpriteReady`、`LocalSprite`、`LocalPlayer`、`LoginRoot`。存在即代表状态。

### 模块可见性

- 需要在 crate 内部跨模块共享时，使用 `pub(crate)`（如 `pub(crate) mod combat;`）。
- `shared` 中的类型一律 `pub`，因为被外部 crate 依赖。

### 资源与启动

- **服务端启动**：`start_server` 函数签名使用 `&mut World`（而非 `Commands`），因为在 Schedule 运行前必须直接往 World 插入 `RenetServer` 和 `NetcodeServerTransport` 资源。
- **全局单例**：`server/src/auth.rs` 中的 TLS Agent 使用 `std::sync::OnceLock` 实现延迟初始化，全局复用。

### 渲染相关

- Bevy 0.18 要求：spawn 2D 网格实体时必须显式插入 `GlobalTransform::default()`，否则运行时会 panic。
- 项目使用 `Triangle2d` 网格而非 Sprite。玩家三角形高 40px、底宽 30px；子弹三角形高 12px、底宽 8px。
- 方向角公式：`angle = dy.atan2(dx) - FRAC_PI_2`，使三角形尖端指向移动方向。
- `AssetPlugin` 设置 `UnapprovedPathMode::Allow`，允许加载工作区外的字体文件（如 `client/assets/fonts/msyh.ttc`）。

## 测试策略

**本项目目前没有任何自动化测试（unit test / integration test）。**

验证修改的唯一方式是手动运行：
1. 启动 Platform、Server、Client。
2. 使用默认账号登录。
3. 检查移动、射击、碰撞、重生、排行榜、断线重连是否正常。

`start-test.ps1` 脚本可一次性启动 Platform + Server + 2 个 Client，方便回归测试。

## 安全注意事项

### 认证与凭据

- Platform 使用 **SHA-256** 对密码进行哈希（非加盐），存储在 `platform/players.json`。
- 服务端与 Platform 之间通过 **HTTPS + Bearer Token** 通信，自定义 rustls 配置信任 mkcert 本地 CA 和系统根证书。
- 玩家凭据通过 netcode 的 `user_data[256]` 字段从客户端传递到服务端，再转发到 Platform 验证。
- **游戏网络层（renet）使用 `Unsecure` 认证模式**，仅适合本地开发，不具备生产环境安全性。

### 敏感文件

- `.env`：包含 `PLATFORM_API_KEY`，已加入 `.gitignore`。
- `platform/certs/`：TLS 私钥和证书，已加入 `.gitignore`。
- `platform/players.json` / `platform/api_keys.json`：运行时会自动初始化默认值，实际部署时应妥善保管。

## 网络架构速查

```
Client          Server           Platform
  |               |                 |
  |--(UDP+renet)->|                 |
  |  user_data    |--(HTTPS/TLS)--->|
  |  (credentials)|  /api/auth/login|
  |               |<--{username}----|
  |<--Replicated--|                 |
  |  Position     |                 |
  |  Direction    |                 |
  |  Score        |                 |
  |  Health       |                 |
  |  Dead         |                 |
  |  Bullet       |                 |
```

- **Server → Client**：通过 `Replicated` 组件自动同步（`Position`, `Direction`, `PlayerId`, `PlayerColor`, `Score`, `Dead`, `PlayerName`, `Health`, `Bullet`）。
- **Client → Server**：通过 `add_client_message` 发送 `MoveInput` 和 `ShootInput`（`Channel::Ordered`）。

## 修改代码时的关键检查点

- **修改 `shared/src/lib.rs` 中的 Component/Message 后**：必须同时在 `server/src/main.rs` 和 `client/src/main.rs` 中检查 `app.replicate::<T>()` 和 `app.add_client_message::<T>()` 是否已注册对应类型，否则运行时会出现序列化错误。
- **修改常量后**：`shared` 中的常量（如 `PORT`, `MAX_HP`）对两端生效；`server/src/main.rs` 中的服务端专属常量（如 `MOVE_SPEED`, `KILL_SCORE`）仅影响服务端逻辑。
- **新增系统后**：服务端注意在 `.chain()` 中的插入位置；客户端注意是否需要加上 `run_if(in_state(GameState::InGame))`。
- **Platform API 变更后**：同步更新 `server/src/auth.rs` 中的请求路径与响应结构体。
