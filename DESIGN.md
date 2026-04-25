# Bevy 多人游戏设计文档

## 项目概述

基于 Bevy 引擎 + bevy_replicon 的多人联机游戏原型，支持服务端/客户端同一份代码运行。玩家通过键盘方向键或 WASD 控制方块移动，每个玩家拥有随机颜色。

## 技术栈

| 组件 | 说明 |
|------|------|
| Bevy | 游戏引擎 |
| bevy_replicon | 网络复制框架 |
| bevy_replicon_renet | renet 传输层 |
| serde | 序列化 |

## 模块结构

```mermaid
graph TD
    subgraph main["main.rs - 入口 & 插件注册"]
        A[App] --> B[DefaultPlugins]
        A --> C[RepliconPlugins]
        A --> D[RepliconRenetPlugins]
        A --> E[渲染系统 spawn_render]
        A --> F[渲染系统 apply_position]
        A --> G[setup_camera]
        A --> H{命令行参数}
        H -->|server| I[服务端系统]
        H -->|client| J[客户端系统]
    end

    subgraph shared["shared.rs - 共享定义"]
        K[Position]
        L[PlayerId]
        M[PlayerColor]
        N[MoveInput]
        O[PlayerCount]
        P[ConnectTimer]
        Q[ConnectionState]
        R[hsv_to_rgb]
        S[spawn_render]
        T[apply_position]
    end

    subgraph server["server.rs - 服务端"]
        U[start_server]
        V[server_on_connect]
        W[server_handle_input]
    end

    subgraph client["client.rs - 客户端"]
        X[start_client]
        Y[client_send_input]
        Z[check_connection]
    end

    main --> shared
    main --> server
    main --> client
```

## 网络架构

```mermaid
sequenceDiagram
    participant C as 客户端
    participant S as 服务端

    Note over S: start_server<br/>监听 UDP:5000
    C->>S: start_client<br/>连接请求
    S-->>C: 连接确认
    Note over C: check_connection<br/>打印"已连接"

    S->>S: server_on_connect<br/>生成玩家实体 (Replicated)
    Note over S: Position, PlayerId, PlayerColor<br/>自动复制到客户端

    loop 每帧
        C->>C: client_send_input<br/>读取键盘 WASD/方向键
        C->>S: MoveInput {dx, dy}
        S->>S: server_handle_input<br/>更新 Position
        S-->>C: 同步 Position
        C->>C: apply_position<br/>更新 Transform
    end
```

## 组件定义

| 组件 | 属性 | 说明 |
|------|------|------|
| Position | x, y: f32 | 玩家位置，服务端权威 |
| PlayerId | u64 | 玩家唯一标识 |
| PlayerColor | r, g, b: f32 | 玩家颜色 |
| MoveInput | dx, dy: f32 | 移动输入向量（归一化）|
| LocalSprite | - | 标记已生成渲染精灵 |

## 资源定义

| 资源 | 说明 |
|------|------|
| PlayerCount | 已连接玩家数，用于生成颜色 |
| ConnectTimer | 客户端连接超时计时器（5秒）|
| ConnectionState | 连接状态标记 |

## 系统调度

### 服务端

```mermaid
graph LR
    Startup --> start_server
    Update --> server_handle_input
    Observer --> server_on_connect
```

### 客户端

```mermaid
graph LR
    Startup --> start_client
    Startup --> setup_camera
    Update --> client_send_input
    Update --> check_connection
```

### 通用渲染

```mermaid
graph LR
    Update --> spawn_render
    Update --> apply_position
```

## 数据流

```mermaid
graph LR
    subgraph Input["输入层"]
        K[键盘 WASD/方向键]
    end

    subgraph ClientSys["客户端系统"]
        A[client_send_input]
        B[check_connection]
        C[spawn_render]
        D[apply_position]
    end

    subgraph Network["网络层 (bevy_replicon + renet)"]
        E[MoveInput ↑]
        F[Position/PlayerColor ↓]
    end

    subgraph ServerSys["服务端系统"]
        G[server_handle_input]
        H[server_on_connect]
    end

    K --> A
    A --> E
    E --> G
    G --> F
    F --> D
    F --> C
    C --> I[Sprite 渲染]
    D --> J[Transform 更新]
```

## 启动方式

```bash
# 启动服务端
cargo run -- server

# 启动客户端（可多开）
cargo run -- client
```
