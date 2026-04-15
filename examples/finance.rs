use axum::extract::Query as AxumQuery;
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
    #[serde(default)] // 兼容旧数据（若无此字段则填0.0）
    filled_quantity: f64,
    side: OrderSide,
    status: OrderStatus,
    account_id: u64,
}

impl Order {
    fn remaining(&self) -> f64 {
        (self.quantity - self.filled_quantity).max(0.0)
    }

    fn is_fully_filled(&self) -> bool {
        self.remaining() <= 1e-9
    }
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
#[serde(rename_all = "snake_case")]
enum OrderStatus {
    Pending,
    PartialFilled,
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
// 新增：订单簿（真实对手盘撮合专用，按价格排序）
// ============================================================
#[derive(Resource)]
struct OrderBook {
    buy_orders: Vec<Entity>,  // 价格从高到低
    sell_orders: Vec<Entity>, // 价格从低到高
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
        }
    }
}

impl OrderBook {
    fn add_order(&mut self, entity: Entity, order: &Order) {
        let vec = match order.side {
            OrderSide::Buy => &mut self.buy_orders,
            OrderSide::Sell => &mut self.sell_orders,
        };

        vec.push(entity);
    }
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

impl Resource for Database {}

// ============================================================
// 3. Web API 层 (Axum)
// ============================================================

#[derive(Deserialize)]
struct CreateOrderRequest {
    symbol: String,
    price: f64,
    quantity: f64,
    side: String,
    account_id: u64,
}

#[derive(Serialize)]
struct CreateOrderResponse {
    order_id: u64,
    status: String,
}

#[derive(Clone)]
struct AppState {
    order_sender: mpsc::Sender<Order>,
    db: Arc<Database>,
    order_id_counter: Arc<AtomicU64>,
}

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
        filled_quantity: 0.0,
        side,
        status: OrderStatus::Pending,
        account_id: req.account_id,
    };

    state.db.save_order(&order);

    if let Err(e) = state.order_sender.send(order).await {
        eprintln!("❌ 发送订单到 Bevy 失败: {}", e);
    }

    Json(CreateOrderResponse {
        order_id,
        status: "pending".to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct OrderQueryParams {
    status: Option<OrderStatus>,
}

async fn list_orders(
    State(state): State<AppState>,
    AxumQuery(params): AxumQuery<OrderQueryParams>,
) -> Json<Vec<Order>> {
    let mut orders = state.db.load_all_orders();

    if let Some(status) = params.status {
        orders.retain(|order| order.status == status);
    }

    Json(orders)
}

async fn list_trades(State(state): State<AppState>) -> Json<Vec<Trade>> {
    Json(state.db.load_all_trades())
}

async fn list_accounts(State(state): State<AppState>) -> Json<Vec<Account>> {
    Json(state.db.load_all_accounts())
}

// ============================================================
// 4. Bevy ECS 层
// ============================================================

#[derive(Resource)]
struct OrderReceiver(mpsc::Receiver<Order>);

// --- 接收订单 + 加入订单簿 ---
fn receive_api_orders_system(
    mut commands: Commands,
    mut receiver: ResMut<OrderReceiver>,
    mut order_book: ResMut<OrderBook>,
) {
    while let Ok(order) = receiver.0.try_recv() {
        println!(
            "📨 API 下单: #{} {} {}@{} (账户 {})",
            order.order_id, order.side, order.quantity, order.price, order.account_id
        );
        let entity = commands.spawn(order.clone()).id();
        order_book.add_order(entity, &order);
    }
}

// --- 维护订单簿排序 ---
fn sort_order_book_system(mut order_book: ResMut<OrderBook>, orders: Query<(Entity, &Order)>) {
    // 重新排序买入订单（价格降序）
    order_book.buy_orders.sort_by(|&a, &b| {
        let price_a = orders.get(a).map(|o| o.1.price).unwrap_or(0.0);
        let price_b = orders.get(b).map(|o| o.1.price).unwrap_or(0.0);
        price_b
            .partial_cmp(&price_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // 重新排序卖出订单（价格升序）
    order_book.sell_orders.sort_by(|&a, &b| {
        let price_a = orders.get(a).map(|o| o.1.price).unwrap_or(f64::MAX);
        let price_b = orders.get(b).map(|o| o.1.price).unwrap_or(f64::MAX);
        price_a
            .partial_cmp(&price_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

// --- 模拟行情 ---
fn simulate_market_data_system(mut market_data: Query<&mut MarketData>, db: Res<Database>) {
    for mut market in market_data.iter_mut() {
        let change = (rand::random::<f64>() - 0.5) * 0.001 * market.last_price;
        market.last_price += change;
        market.timestamp = Instant::now();
        db.save_last_price(&market.symbol, market.last_price);
    }
}

// --- 风控（检查剩余数量）---
fn risk_control_system(
    mut orders: Query<&mut Order>,
    accounts: Query<&Account>,
    db: Res<Database>,
) {
    for mut order in orders.iter_mut() {
        // 只检查未完全成交且未拒绝的订单
        if order.status == OrderStatus::Filled || order.status == OrderStatus::Rejected {
            continue;
        }
        let remaining = order.remaining();

        if let Some(account) = accounts.iter().find(|a| a.account_id == order.account_id) {
            let rejected = match order.side {
                OrderSide::Buy => {
                    let required = order.price * remaining;
                    account.cash_balance < required
                }
                OrderSide::Sell => {
                    let has_pos = account
                        .positions
                        .iter()
                        .any(|p| p.symbol == order.symbol && p.quantity >= remaining);
                    !has_pos
                }
            };

            if rejected {
                order.status = OrderStatus::Rejected;
                db.save_order(&order);
                println!("⚠️ 订单 #{} 被风控拒绝", order.order_id);
            }
        }
    }
}

// ============================================================
// 核心：真实对手盘撮合（支持部分成交）
// ============================================================
fn order_matching_system(
    mut commands: Commands,
    mut orders: Query<(Entity, &mut Order)>,
    mut trade_id_counter: Local<u64>,
    mut order_book: ResMut<OrderBook>,
    db: Res<Database>,
) {
    // 注意：假设 order_book 已经由 sort_order_book_system 排好序
    let mut i = 0;
    while i < order_book.buy_orders.len() {
        let buy_entity = order_book.buy_orders[i];
        let mut j = 0;
        while j < order_book.sell_orders.len() {
            let sell_entity = order_book.sell_orders[j];

            let (buy_ok, sell_ok) = match (
                orders.get(buy_entity).ok().map(|(_, o)| o),
                orders.get(sell_entity).ok().map(|(_, o)| o),
            ) {
                (Some(b), Some(s))
                    if b.symbol == s.symbol
                        && b.status != OrderStatus::Filled
                        && b.status != OrderStatus::Rejected
                        && s.status != OrderStatus::Filled
                        && s.status != OrderStatus::Rejected
                        && b.price >= s.price =>
                {
                    (b, s)
                }
                _ => {
                    j += 1;
                    continue;
                }
            };

            let trade_qty = buy_ok.remaining().min(sell_ok.remaining());
            if trade_qty <= 0.0 {
                j += 1;
                continue;
            }

            // 生成成交
            *trade_id_counter += 1;
            let trade = Trade {
                trade_id: *trade_id_counter,
                buy_order_id: buy_ok.order_id,
                sell_order_id: sell_ok.order_id,
                symbol: buy_ok.symbol.clone(),
                price: sell_ok.price, // 以卖单价格成交
                quantity: trade_qty,
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };
            db.save_trade(&trade);
            commands.spawn(trade);

            println!(
                "✅ 撮合成交: 买 #{} ↔ 卖 #{} | 价格 {:.2} 数量 {:.2}",
                buy_ok.order_id, sell_ok.order_id, sell_ok.price, trade_qty
            );

            // 更新买单
            if let Ok((_, mut bo)) = orders.get_mut(buy_entity) {
                bo.filled_quantity += trade_qty;
                if bo.is_fully_filled() {
                    bo.status = OrderStatus::Filled;
                } else {
                    bo.status = OrderStatus::PartialFilled;
                }
                db.save_order(&bo);
            }

            // 更新卖单
            if let Ok((_, mut so)) = orders.get_mut(sell_entity) {
                so.filled_quantity += trade_qty;
                if so.is_fully_filled() {
                    so.status = OrderStatus::Filled;
                } else {
                    so.status = OrderStatus::PartialFilled;
                }
                db.save_order(&so);
            }

            // 如果买单完全成交，则跳出内层循环，外层将处理下一个买单
            if let Ok((_, bo)) = orders.get(buy_entity) {
                if bo.status == OrderStatus::Filled {
                    break;
                }
            }
            j += 1;
        }
        i += 1;
    }

    // 清理订单簿中已完全成交的订单
    order_book.buy_orders.retain(|&e| {
        orders
            .get(e)
            .map(|(_, o)| o.status != OrderStatus::Filled)
            .unwrap_or(false)
    });
    order_book.sell_orders.retain(|&e| {
        orders
            .get(e)
            .map(|(_, o)| o.status != OrderStatus::Filled)
            .unwrap_or(false)
    });
}

// --- 清算结算 ---
fn settlement_system(
    trades: Query<&Trade, Added<Trade>>,
    mut accounts: Query<&mut Account>,
    orders: Query<&Order>,
    db: Res<Database>,
) {
    for trade in trades.iter() {
        // 买方结算
        if let Some(buy_order) = orders.iter().find(|o| o.order_id == trade.buy_order_id) {
            if let Some(mut acc) = accounts
                .iter_mut()
                .find(|a| a.account_id == buy_order.account_id)
            {
                acc.cash_balance -= trade.price * trade.quantity;
                if let Some(pos) = acc.positions.iter_mut().find(|p| p.symbol == trade.symbol) {
                    pos.quantity += trade.quantity;
                } else {
                    acc.positions.push(Position {
                        symbol: trade.symbol.clone(),
                        quantity: trade.quantity,
                    });
                }
                db.save_account(&acc);
            }
        }

        // 卖方结算
        if let Some(sell_order) = orders.iter().find(|o| o.order_id == trade.sell_order_id) {
            if let Some(mut acc) = accounts
                .iter_mut()
                .find(|a| a.account_id == sell_order.account_id)
            {
                acc.cash_balance += trade.price * trade.quantity;
                if let Some(pos) = acc.positions.iter_mut().find(|p| p.symbol == trade.symbol) {
                    pos.quantity -= trade.quantity;
                }
                db.save_account(&acc);
            }
        }
    }
}

// --- 初始化 ---
fn setup(mut commands: Commands, db: Res<Database>, mut order_book: ResMut<OrderBook>) {
    // 初始化账户
    let existing_accounts = db.load_all_accounts();
    if !existing_accounts.iter().any(|a| a.account_id == 1001) {
        let account = Account {
            account_id: 1001,
            cash_balance: 100_000.0,
            positions: vec![],
        };
        db.save_account(&account);
        commands.spawn(account);
        println!("🆕 创建账户 1001（余额 100,000）");
    }

    if !existing_accounts.iter().any(|a| a.account_id == 1002) {
        let account = Account {
            account_id: 1002,
            cash_balance: 50_000.0,
            positions: vec![Position {
                symbol: "BTC/USDT".to_string(),
                quantity: 1.0,
            }],
        };
        db.save_account(&account);
        commands.spawn(account);
        println!("🆕 创建账户 1002（余额 50,000，持仓 1 BTC）");
    }

    for account in existing_accounts {
        println!(
            "♻️ 恢复账户 {}（余额 {:.2}，持仓 {} 种）",
            account.account_id,
            account.cash_balance,
            account.positions.len()
        );
        commands.spawn(account);
    }

    // 恢复未完成订单（Pending 或 PartialFilled）
    let active_orders: Vec<_> = db
        .load_all_orders()
        .into_iter()
        .filter(|o| o.status == OrderStatus::Pending || o.status == OrderStatus::PartialFilled)
        .collect();
    for order in active_orders {
        println!(
            "♻️ 恢复订单 #{} {} {:.2}@{:.2} (已成交 {:.2})",
            order.order_id, order.side, order.quantity, order.price, order.filled_quantity
        );
        let entity = commands.spawn(order.clone()).id();
        order_book.add_order(entity, &order);
    }

    // 初始化行情
    let price = db.load_last_price("BTC/USDT").unwrap_or(50000.0);
    commands.spawn(MarketData {
        symbol: "BTC/USDT".into(),
        last_price: price,
        timestamp: Instant::now(),
    });

    commands.insert_resource(OrderBook::default());
    println!("🚀 系统启动完成，初始行情：{}", price);
}

// ============================================================
// 5. 主函数
// ============================================================
fn main() {
    // 数据库
    let sled_db = sled::open("./trading.db").expect("数据库打开失败");
    let db = Database::new(&sled_db);

    // 通道
    let (tx, rx) = mpsc::channel(100);
    let order_id_counter = Arc::new(AtomicU64::new(10000));

    // Axum 状态
    let app_state = AppState {
        order_sender: tx,
        db: Arc::new(db.clone()),
        order_id_counter,
    };

    // 启动 API 服务
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let app = Router::new()
                .route("/orders", post(create_order).get(list_orders))
                .route("/trades", get(list_trades))
                .route("/accounts", get(list_accounts))
                .with_state(app_state);

            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            println!("🌐 API 服务已启动：http://localhost:3000");
            axum::serve(listener, app).await.unwrap();
        });
    });

    // Bevy 引擎
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(db)
        .insert_resource(OrderReceiver(rx))
        .insert_resource(OrderBook::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                receive_api_orders_system,
                sort_order_book_system, // 新增：排序订单簿
                simulate_market_data_system,
                risk_control_system,
                order_matching_system,
                settlement_system,
            )
                .chain(),
        )
        .run();
}
