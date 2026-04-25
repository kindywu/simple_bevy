use bevy::prelude::*;
use std::time::Instant;

// ==========================================
// 1. 定义组件 (Components)
// ==========================================

#[derive(Component, Debug, Clone)]
struct Order {
    order_id: u64,
    symbol: String,
    price: f64,
    quantity: f64,
    side: OrderSide,
    status: OrderStatus,
    account_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq)]
enum OrderStatus {
    Pending,
    Filled,
    Rejected,
}

#[derive(Component, Debug)]
struct Account {
    account_id: u64,
    cash_balance: f64,
    positions: Vec<Position>,
}

#[derive(Debug, Clone)]
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

#[derive(Component, Debug)]
struct Trade {
    trade_id: u64,
    buy_order_id: u64,
    sell_order_id: u64,
    symbol: String,
    price: f64,
    quantity: f64,
    timestamp: Instant,
}

// ==========================================
// 2. 系统 (Systems)
// ==========================================

fn risk_control_system(mut orders: Query<&mut Order>, accounts: Query<&Account>) {
    for mut order in orders.iter_mut() {
        if order.status != OrderStatus::Pending {
            continue;
        }

        // 找到关联账户，注意返回的是 Option，不是 Result
        if let Some(account) = accounts.iter().find(|a| a.account_id == order.account_id) {
            match order.side {
                OrderSide::Buy => {
                    let required_cash = order.price * order.quantity;
                    if account.cash_balance < required_cash {
                        order.status = OrderStatus::Rejected;
                        println!("❌ 订单 {} 被拒: 资金不足", order.order_id);
                    }
                }
                OrderSide::Sell => {
                    if let Some(pos) = account.positions.iter().find(|p| p.symbol == order.symbol) {
                        if pos.quantity < order.quantity {
                            order.status = OrderStatus::Rejected;
                            println!("❌ 订单 {} 被拒: 持仓不足", order.order_id);
                        }
                    } else {
                        order.status = OrderStatus::Rejected;
                        println!("❌ 订单 {} 被拒: 无持仓", order.order_id);
                    }
                }
            }
        }
    }
}

fn order_matching_system(
    mut commands: Commands,
    mut orders: Query<&mut Order>,
    market_data: Query<&MarketData>,
    mut trade_id_counter: Local<u64>,
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
                commands.spawn(Trade {
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
                    timestamp: Instant::now(),
                });

                order.status = OrderStatus::Filled;
                println!(
                    "✅ 订单 {} 成交: 价格 {}, 数量 {}",
                    order.order_id, market.last_price, order.quantity
                );
            }
        }
    }
}

fn settlement_system(
    trades: Query<&Trade, Added<Trade>>,
    mut accounts: Query<&mut Account>,
    orders: Query<&Order>,
) {
    for trade in trades.iter() {
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
                        "💼 账户 {} 结算: 资金 -{}，持仓 +{}",
                        account.account_id, cost, trade.quantity
                    );
                    println!("✅ 成交 {} 时间 {:?}", trade.trade_id, trade.timestamp);
                }
            }
        }

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
                        "💼 账户 {} 结算: 资金 +{}，持仓 -{}",
                        account.account_id, revenue, trade.quantity
                    );
                }
            }
        }
    }
}

fn simulate_market_data_system(mut market_data: Query<&mut MarketData>) {
    for mut market in market_data.iter_mut() {
        let change = (rand::random::<f64>() - 0.5) * 0.01 * market.last_price;
        market.last_price += change;
        market.timestamp = Instant::now();
        println!(
            "📈 市场数据更新: {} 价格 {:.2}",
            market.symbol, market.last_price
        );
    }
}

// ==========================================
// 3. 主函数
// ==========================================

fn setup(mut commands: Commands) {
    commands.spawn(Account {
        account_id: 1001,
        cash_balance: 100_000.0,
        positions: vec![],
    });

    commands.spawn(MarketData {
        symbol: "BTC/USDT".to_string(),
        last_price: 50_000.0,
        timestamp: Instant::now(),
    });

    commands.spawn(Order {
        order_id: 1,
        symbol: "BTC/USDT".to_string(),
        price: 50_100.0,
        quantity: 0.5,
        side: OrderSide::Buy,
        status: OrderStatus::Pending,
        account_id: 1001,
    });

    println!("🚀 系统启动，初始状态已设置...");
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                simulate_market_data_system,
                risk_control_system,
                order_matching_system,
                settlement_system,
            )
                .chain(),
        )
        .run();
}
