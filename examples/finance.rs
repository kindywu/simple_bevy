// ============================================================
// 量化交易系统 - 单文件完整实现
// 依赖: bevy, sled, bincode, serde, axum, tokio
//
// Cargo.toml 配置:
// [dependencies]
// bevy = { version = "0.14", default-features = false, features = ["bevy_app", "bevy_ecs"] }
// sled = "0.34"
// bincode = "1.3"
// serde = { version = "1", features = ["derive"] }
// axum = "0.7"
// tokio = { version = "1", features = ["full"] }
// rand = "0.8"
// ============================================================

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tokio::sync::mpsc;

// ============================================================
// 1. 数据结构 (Components & Plain structs)
// ============================================================

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
struct Order {
    order_id: u64,
    symbol: String,
    price: f64,
    quantity: f64,
    side: OrderSide,
    status: OrderStatus,
    account_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "买"),
            OrderSide::Sell => write!(f, "卖"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum OrderStatus {
    Pending,
    Filled,
    Rejected,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
struct Account {
    account_id: u64,
    cash_balance: f64,
    positions: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Position {
    symbol: String,
    quantity: f64,
}

#[derive(Component, Debug)]
struct MarketData {
    symbol: String,
    last_price: f64,
    timestamp: Instant,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
struct Trade {
    trade_id: u64,
    buy_order_id: u64,
    sell_order_id: u64,
    symbol: String,
    price: f64,
    quantity: f64,
    timestamp_ms: u64,
}

// ============================================================
// 2. 持久化层 (sled 嵌入式数据库)
// ============================================================

#[derive(Clone)]
struct Database {
    orders: sled::Tree,
    accounts: sled::Tree,
    trades: sled::Tree,
    market: sled::Tree,
}

impl Database {
    /// 从已打开的 sled::Db 实例创建 Database（避免重复打开文件）
    fn new(db: &sled::Db) -> Self {
        Self {
            orders: db.open_tree("orders").expect("无法打开 orders tree"),
            accounts: db.open_tree("accounts").expect("无法打开 accounts tree"),
            trades: db.open_tree("trades").expect("无法打开 trades tree"),
            market: db.open_tree("market").expect("无法打开 market tree"),
        }
    }

    // --- Order ---
    fn save_order(&self, order: &Order) {
        let key = order.order_id.to_be_bytes();
        let val = bincode::serialize(order).unwrap();
        self.orders.insert(key, val).unwrap();
    }

    fn load_all_orders(&self) -> Vec<Order> {
        self.orders
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| bincode::deserialize::<Order>(&v).ok())
            .collect()
    }

    // --- Account ---
    fn save_account(&self, account: &Account) {
        let key = account.account_id.to_be_bytes();
        let val = bincode::serialize(account).unwrap();
        self.accounts.insert(key, val).unwrap();
    }

    fn load_all_accounts(&self) -> Vec<Account> {
        self.accounts
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| bincode::deserialize::<Account>(&v).ok())
            .collect()
    }

    // --- Trade ---
    fn save_trade(&self, trade: &Trade) {
        let key = trade.trade_id.to_be_bytes();
        let val = bincode::serialize(trade).unwrap();
        self.trades.insert(key, val).unwrap();
    }

    fn load_all_trades(&self) -> Vec<Trade> {
        self.trades
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| bincode::deserialize::<Trade>(&v).ok())
            .collect()
    }

    // --- Market price snapshot ---
    fn save_last_price(&self, symbol: &str, price: f64) {
        let val = bincode::serialize(&price).unwrap();
        self.market.insert(symbol.as_bytes(), val).unwrap();
    }

    fn load_last_price(&self, symbol: &str) -> Option<f64> {
        self.market
            .get(symbol.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| bincode::deserialize::<f64>(&v).ok())
    }
}

// 让 Database 能作为 Bevy Resource
impl Resource for Database {}

// ============================================================
// 3. Web API 层 (Axum)
// ============================================================

// 请求 / 响应结构
#[derive(Deserialize)]
struct CreateOrderRequest {
    symbol: String,
    price: f64,
    quantity: f64,
    side: String, // "buy" | "sell"
    account_id: u64,
}

#[derive(Serialize)]
struct CreateOrderResponse {
    order_id: u64,
    status: String,
}

// Axum 共享状态（跨线程克隆）
#[derive(Clone)]
struct AppState {
    order_sender: mpsc::Sender<Order>,
    db: Arc<Database>, // 使用 Arc 包装，便于跨线程共享
    order_id_counter: Arc<AtomicU64>,
}

// POST /orders — 创建新订单
async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> Json<CreateOrderResponse> {
    let order_id = state.order_id_counter.fetch_add(1, Ordering::SeqCst);

    let side = if req.side.to_lowercase() == "buy" {
        OrderSide::Buy
    } else {
        OrderSide::Sell
    };

    let order = Order {
        order_id,
        symbol: req.symbol,
        price: req.price,
        quantity: req.quantity,
        side,
        status: OrderStatus::Pending,
        account_id: req.account_id,
    };

    // 先持久化，再发给 Bevy（即使进程崩溃订单也不会丢失）
    state.db.save_order(&order);

    if let Err(e) = state.order_sender.send(order).await {
        eprintln!("❌ 发送订单到 Bevy 失败: {}", e);
    }

    Json(CreateOrderResponse {
        order_id,
        status: "pending".to_string(),
    })
}

// GET /orders — 查询所有订单（直接读数据库，不经过 Bevy）
async fn list_orders(State(state): State<AppState>) -> Json<Vec<Order>> {
    Json(state.db.load_all_orders())
}

// GET /trades — 查询所有成交记录
async fn list_trades(State(state): State<AppState>) -> Json<Vec<Trade>> {
    Json(state.db.load_all_trades())
}

// GET /accounts — 查询所有账户
async fn list_accounts(State(state): State<AppState>) -> Json<Vec<Account>> {
    Json(state.db.load_all_accounts())
}

// ============================================================
// 4. Bevy ECS 层
// ============================================================

// Resource：持有从 Axum 接收订单的 channel 接收端
#[derive(Resource)]
struct OrderReceiver(mpsc::Receiver<Order>);

// --- System 1：接收 API 订单，spawn 为 ECS Entity ---
fn receive_api_orders_system(mut commands: Commands, mut receiver: ResMut<OrderReceiver>) {
    while let Ok(order) = receiver.0.try_recv() {
        println!(
            "📨 API 下单: #{} {} {}@{} (账户 {})",
            order.order_id, order.side, order.quantity, order.price, order.account_id
        );
        commands.spawn(order);
    }
}

// --- System 2：模拟行情（随机游走）---
fn simulate_market_data_system(mut market_data: Query<&mut MarketData>, db: Res<Database>) {
    for mut market in market_data.iter_mut() {
        let change = (rand::random::<f64>() - 0.5) * 0.01 * market.last_price;
        market.last_price += change;
        market.timestamp = Instant::now();

        // 每帧持久化最新价格快照
        db.save_last_price(&market.symbol, market.last_price);

        // println!("📈 行情: {} = {:.2}", market.symbol, market.last_price);
    }
}

// --- System 3：风控检查 ---
fn risk_control_system(
    mut orders: Query<&mut Order>,
    accounts: Query<&Account>,
    db: Res<Database>,
) {
    for mut order in orders.iter_mut() {
        if order.status != OrderStatus::Pending {
            continue;
        }

        if let Some(account) = accounts.iter().find(|a| a.account_id == order.account_id) {
            let rejected = match order.side {
                OrderSide::Buy => {
                    let required = order.price * order.quantity;
                    if account.cash_balance < required {
                        println!(
                            "❌ 订单 #{} 拒绝: 资金不足 (需要 {:.2}，余额 {:.2})",
                            order.order_id, required, account.cash_balance
                        );
                        true
                    } else {
                        false
                    }
                }
                OrderSide::Sell => {
                    match account.positions.iter().find(|p| p.symbol == order.symbol) {
                        Some(pos) if pos.quantity >= order.quantity => false,
                        Some(_) => {
                            println!("❌ 订单 #{} 拒绝: 持仓不足", order.order_id);
                            true
                        }
                        None => {
                            println!("❌ 订单 #{} 拒绝: 无持仓", order.order_id);
                            true
                        }
                    }
                }
            };

            if rejected {
                order.status = OrderStatus::Rejected;
                db.save_order(&order);
            }
        }
    }
}

// --- System 4：订单撮合 ---
fn order_matching_system(
    mut commands: Commands,
    mut orders: Query<&mut Order>,
    market_data: Query<&MarketData>,
    mut trade_id_counter: Local<u64>,
    db: Res<Database>,
) {
    if let Some(market) = market_data.iter().next() {
        for mut order in orders.iter_mut() {
            if order.status != OrderStatus::Pending || order.symbol != market.symbol {
                continue;
            }

            let can_match = match order.side {
                OrderSide::Buy => order.price >= market.last_price,
                OrderSide::Sell => order.price <= market.last_price,
            };

            if can_match {
                *trade_id_counter += 1;

                let trade = Trade {
                    trade_id: *trade_id_counter,
                    buy_order_id: if order.side == OrderSide::Buy {
                        order.order_id
                    } else {
                        0
                    },
                    sell_order_id: if order.side == OrderSide::Sell {
                        order.order_id
                    } else {
                        0
                    },
                    symbol: order.symbol.clone(),
                    price: market.last_price,
                    quantity: order.quantity,
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };

                println!(
                    "✅ 撮合成功: 订单 #{} 以 {:.2} 成交 {} 手",
                    order.order_id, market.last_price, order.quantity
                );

                db.save_trade(&trade);
                order.status = OrderStatus::Filled;
                db.save_order(&order);

                commands.spawn(trade);
            }
        }
    }
}

// --- System 5：清算结算 ---
fn settlement_system(
    trades: Query<&Trade, Added<Trade>>,
    mut accounts: Query<&mut Account>,
    orders: Query<&Order>,
    db: Res<Database>,
) {
    for trade in trades.iter() {
        // 处理买方
        if trade.buy_order_id != 0 {
            if let Some(buy_order) = orders.iter().find(|o| o.order_id == trade.buy_order_id) {
                if let Some(mut account) = accounts
                    .iter_mut()
                    .find(|a| a.account_id == buy_order.account_id)
                {
                    let cost = trade.price * trade.quantity;
                    account.cash_balance -= cost;

                    if let Some(pos) = account
                        .positions
                        .iter_mut()
                        .find(|p| p.symbol == trade.symbol)
                    {
                        pos.quantity += trade.quantity;
                    } else {
                        account.positions.push(Position {
                            symbol: trade.symbol.clone(),
                            quantity: trade.quantity,
                        });
                    }

                    println!(
                        "💼 买方账户 {}: 扣款 {:.2}，持仓 +{} {}",
                        account.account_id, cost, trade.quantity, trade.symbol
                    );

                    db.save_account(&account);
                }
            }
        }

        // 处理卖方
        if trade.sell_order_id != 0 {
            if let Some(sell_order) = orders.iter().find(|o| o.order_id == trade.sell_order_id) {
                if let Some(mut account) = accounts
                    .iter_mut()
                    .find(|a| a.account_id == sell_order.account_id)
                {
                    let revenue = trade.price * trade.quantity;
                    account.cash_balance += revenue;

                    if let Some(pos) = account
                        .positions
                        .iter_mut()
                        .find(|p| p.symbol == trade.symbol)
                    {
                        pos.quantity -= trade.quantity;
                    }

                    println!(
                        "💼 卖方账户 {}: 入账 {:.2}，持仓 -{} {}",
                        account.account_id, revenue, trade.quantity, trade.symbol
                    );

                    db.save_account(&account);
                }
            }
        }
    }
}

// --- Startup System：初始化或从数据库恢复 ---
fn setup(mut commands: Commands, db: Res<Database>) {
    let existing_accounts = db.load_all_accounts();

    if existing_accounts.is_empty() {
        // 首次启动，创建默认账户
        let account = Account {
            account_id: 1001,
            cash_balance: 100_000.0,
            positions: vec![],
        };
        db.save_account(&account);
        commands.spawn(account);
        println!("🆕 首次启动，创建账户 1001（余额 100,000）");
    } else {
        // 从数据库恢复
        for account in existing_accounts {
            println!(
                "♻️ 恢复账户 {}（余额 {:.2}，持仓 {} 种）",
                account.account_id,
                account.cash_balance,
                account.positions.len()
            );
            commands.spawn(account);
        }
    }

    // 恢复 Pending 状态的历史订单
    let pending_orders: Vec<Order> = db
        .load_all_orders()
        .into_iter()
        .filter(|o| o.status == OrderStatus::Pending)
        .collect();

    if !pending_orders.is_empty() {
        println!("♻️ 恢复 {} 条 Pending 订单", pending_orders.len());
        for order in pending_orders {
            commands.spawn(order);
        }
    }

    // 恢复行情（首次使用默认价格）
    let last_price = db.load_last_price("BTC/USDT").unwrap_or(50_000.0);
    commands.spawn(MarketData {
        symbol: "BTC/USDT".to_string(),
        last_price,
        timestamp: Instant::now(),
    });

    println!("🚀 Bevy ECS 启动完成，行情初始价格: {:.2}", last_price);
}

// ============================================================
// 5. 主函数：同时启动 Axum + Bevy
// ============================================================

fn main() {
    // 1. 打开数据库（只开一次）
    let sled_db = sled::open("./trading.db").expect("无法打开数据库");
    let db = Database::new(&sled_db);

    // 2. channel 缓冲 100 条订单（Axum → Bevy）
    let (tx, rx) = mpsc::channel::<Order>(100);

    // 3. order_id 计数器
    let order_id_counter = Arc::new(AtomicU64::new(10000));

    // 4. Axum 状态（需要 Arc<Database> 以跨线程）
    let app_state = AppState {
        order_sender: tx,
        db: Arc::new(db.clone()), // 克隆 Database（共享底层 Tree）
        order_id_counter,
    };

    // 5. 在独立线程启动 Axum
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = Router::new()
                .route("/orders", post(create_order))
                .route("/orders", get(list_orders))
                .route("/trades", get(list_trades))
                .route("/accounts", get(list_accounts))
                .with_state(app_state);

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

            println!("🌐 API 服务启动: http://localhost:3000");
            println!("   POST /orders   — 下单");
            println!("   GET  /orders   — 查询订单");
            println!("   GET  /trades   — 查询成交");
            println!("   GET  /accounts — 查询账户");

            axum::serve(listener, router).await.unwrap();
        });
    });

    // 6. Bevy 运行在主线程（阻塞）
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(db) // 直接插入 Database 实例（已实现 Resource）
        .insert_resource(OrderReceiver(rx))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                receive_api_orders_system,
                simulate_market_data_system,
                risk_control_system,
                order_matching_system,
                settlement_system,
            )
                .chain(),
        )
        .run();
}
