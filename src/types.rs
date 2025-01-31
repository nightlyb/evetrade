use crate::pathfinder::Pathfinder;
use crate::route::Route;
use log::debug;
use serde;
use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Stargate {
    pub origin: u32,
    pub destination: u32,
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct System {
    pub id: u32,
    pub name: String,
    pub security_status: f32,
    pub stargates: Vec<Stargate>,
    pub position: Vector3,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Order {
    pub is_buy_order: bool,
    pub order_type: Type,
    pub price: f32,
    pub station_id: u32,
    pub system_id: u32,
    pub region_id: u32,
    pub volume: u32,
    pub type_id: u32,
}

impl Iterator for Order {
    type Item = Order;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.clone())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct OrderGroup {
    pub buy: Vec<Order>,
    pub sell: Vec<Order>,
}

impl OrderGroup {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_order(&mut self, order: Order) {
        if order.is_buy_order {
            self.buy.push(order);
        } else {
            self.sell.push(order);
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Type {
    pub type_id: u32,
    pub group_id: u32,
    pub name: String,
    pub volume: f32,
}

#[derive(Clone)]
pub enum Waypoint {
    System(System),
    Order(Order),
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector3 { x, y, z }
    }

    pub fn zero() -> Self {
        Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn one() -> Self {
        Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    pub fn from_array(arr: [f64; 3]) -> Self {
        Vector3 {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }

    pub fn max_dimension(&self) -> usize {
        if self.x > self.y {
            if self.x > self.z {
                0
            } else {
                2
            }
        } else if self.y > self.z {
            1
        } else {
            2
        }
    }

    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn similarity(&self, other: &Vector3, w1: Option<f64>, w2: Option<f64>) -> f64 {
        // TODO: Change the weights? Configurable? Dynamic?
        let w1 = w1.unwrap_or(0.5);
        let w2 = w2.unwrap_or(0.5);

        let mag1 = self.magnitude();
        let mag2 = other.magnitude();

        if mag1 == 0.0 || mag2 == 0.0 {
            return 0.0;
        }

        let cos_theta = self.dot(other) / (mag1 * mag2);
        let direction_similarity = (cos_theta + 1.0) / 2.0;

        let magnitude_similarity = mag1.min(mag2) / mag1.max(mag2);

        w1 * direction_similarity + w2 * magnitude_similarity
    }

    pub fn distance(&self, other: &Vector3) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_squared(&self, other: &Vector3) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)
    }
}

impl Sub for Vector3 {
    type Output = Vector3;

    fn sub(self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Sub for &Vector3 {
    type Output = Vector3;

    fn sub(self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul for Vector3 {
    type Output = Vector3;

    fn mul(self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<f64> for Vector3 {
    type Output = Vector3;

    fn mul(self, scalar: f64) -> Vector3 {
        Vector3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul<f64> for &Vector3 {
    type Output = Vector3;

    fn mul(self, scalar: f64) -> Vector3 {
        Vector3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Add for Vector3 {
    type Output = Vector3;

    fn add(self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Add for &Vector3 {
    type Output = Vector3;

    fn add(self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

// pub struct TradeOpportunity {
//     system_id: u32,
//     type_id: u32,
//     price: f64,
//     volume: u32,
//     operation_type: OperationType,
// }

// pub enum OperationType {
//     Buy,
//     Sell,
//     Transit,
// }

#[derive(Debug, Clone)]
pub struct TradePoint {
    system_id: u32,
    order: Option<Order>,
}

#[derive(Debug, Clone)]
pub struct TradePath {
    points: Vec<TradePoint>,
    current_point: usize,
}

impl TradePath {
    pub fn new() -> Self {
        TradePath {
            points: Vec::new(),
            current_point: 0,
        }
    }

    pub fn current_point_index(&self) -> usize {
        self.current_point
    }

    pub fn insert_order(&mut self, order: Order, index: usize, pathfinder: &mut Pathfinder) {
        self.points.insert(
            index,
            TradePoint {
                system_id: order.system_id,
                order: Some(order),
            },
        );

        let mut i: usize = 0;
        while i < self.points.len() - 1 {
            let order1 = &self.points[i];
            if let Some(order2) = self.points.get(i + 1) {
                if order1.system_id == order2.system_id {
                    i += 1;
                    continue;
                }

                let path = pathfinder
                    .compute_path(order1.system_id, order2.system_id)
                    .unwrap();

                for &id in &path[1..path.len() - 1] {
                    self.points.push(TradePoint {
                        system_id: id,
                        order: None,
                    });
                }

                i += 2;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn destination(&self) -> u32 {
        self.points.get(self.points.len() - 1).unwrap().system_id
    }

    pub fn current_system(&self) -> u32 {
        self.points.get(self.current_point).unwrap().system_id
    }

    // Move to the next system and return its ID
    pub fn next_system_id(&mut self) -> Option<u32> {
        if self.current_point + 1 < self.points.len() {
            self.current_point += 1;
            Some(self.current_system())
        } else {
            None
        }
    }

    pub fn get_all_orders(&self) -> Vec<Order> {
        self.points
            .iter()
            .filter_map(|point| point.order.clone())
            .collect()
    }
}

pub struct TradePair<'a> {
    pub buy_order: Order,
    pub sell_order: Order,

    pub buy_order_system: &'a System,
    pub sell_order_system: &'a System,

    pub units: u32,
    pub cargo_volume: f32,
    pub profit: f32,
    pub investment: f32,
}

#[derive(Debug, Clone)]
pub struct TradeState {
    pub path: TradePath,
    pub available_isk: f32,
    pub available_volume: f32,
    pub visited_systems: std::collections::HashSet<u32>,
}

impl TradeState {
    pub fn get_profit_per_jump(&self) -> f32 {
        self.path
            .points
            .iter()
            .filter_map(|point| point.order.clone())
            .map(|order| order.price)
            .sum::<f32>()
            / self.path.points.len() as f32
    }

    pub fn get_jumps(&self) -> u32 {
        self.path.len() as u32
    }

    pub fn to_route(&self, pathfinder: &mut Pathfinder) -> Route {
        // TODO: do we need this at all? review the code later, too tired rn
        let mut path: Vec<Waypoint> = Vec::new();
        let orders = self.path.get_all_orders();
        assert_eq!(orders.len() % 2, 0); // Ensure there is always a pair of buy/sell orders

        let mut i: usize = 0;
        while i < orders.len() {
            let sell_order = orders[i].clone();
            let buy_order = orders[i + 1].clone();
            path.push(Waypoint::Order(sell_order.clone()));
            path.extend(
                pathfinder
                    .compute_path(sell_order.system_id, buy_order.system_id)
                    .unwrap()
                    .iter()
                    .map(|&id| Waypoint::System(pathfinder.systems.get(&id).unwrap().clone())),
            );
            path.push(Waypoint::Order(buy_order.clone()));
            i += 2;
        }

        Route::from_path(path)
    }
}

impl PartialEq for TradeState {
    fn eq(&self, other: &Self) -> bool {
        self.get_profit_per_jump() == other.get_profit_per_jump()
    }
}

impl PartialOrd for TradeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TradeState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_profit_per_jump()
            .partial_cmp(&other.get_profit_per_jump())
            .unwrap_or(Ordering::Equal)
            .reverse()
    }
}

impl Eq for TradeState {}
