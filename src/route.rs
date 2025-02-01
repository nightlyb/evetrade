use log::error;
use num_traits::{Signed, ToPrimitive};
use std::fmt::Write;

use crate::{
    settings::Settings,
    types::{Order, System, Waypoint},
};

#[derive(Clone)]
pub struct Route {
    path: Vec<Waypoint>,
    profit: f32,
    is_dirty: bool,
    representation: String,
    jumps: usize,
    profit_per_jump: f32,
}

impl Route {
    pub fn new() -> Self {
        Route {
            path: Vec::new(),
            profit: 0.0,
            is_dirty: true,
            representation: String::new(),
            jumps: 0,
            profit_per_jump: 0.0,
        }
    }

    pub fn from_path(path: Vec<Waypoint>) -> Self {
        let mut route = Route {
            path,
            profit: 0.0,
            is_dirty: true,
            representation: String::new(),
            jumps: 0,
            profit_per_jump: 0.0,
        };

        route.jumps = route
            .path
            .iter()
            .filter(|point| {
                if let Waypoint::System(_) = point {
                    true
                } else {
                    false
                }
            })
            .count();

        route
    }

    pub fn add_systems(&mut self, systems: Vec<System>) {
        for system in systems {
            self.path.push(Waypoint::System(system));
            self.jumps += 1;
        }
        self.is_dirty = true;
    }

    pub fn add_order(&mut self, order: Order) {
        self.path.push(Waypoint::Order(order));
        self.is_dirty = true;
    }

    pub fn get_jumps(&self) -> usize {
        self.jumps
    }

    pub fn get_path(&self) -> &Vec<Waypoint> {
        &self.path
    }

    pub fn calculate_profit(&mut self) {
        let mut buy_total: f32 = 0.0;
        let mut sell_total: f32 = 0.0;

        for point in &self.path {
            if let Waypoint::Order(order) = point {
                if order.is_buy_order {
                    sell_total += order.price * order.volume as f32; // If order is a buy order, we are selling
                } else {
                    buy_total += order.price * order.volume as f32;
                }
            }
        }

        if buy_total == 0.0 && sell_total == 0.0 {
            error!("No orders found.");
        } else if buy_total == 0.0 || sell_total == 0.0 {
            error!("Missing buy or sell order.");
        }

        self.profit = sell_total - buy_total;
        self.profit_per_jump = if self.jumps > 0 {
            (self.profit / self.jumps as f32).round()
        } else {
            0.0
        };
    }

    pub fn get_profit(&mut self) -> f32 {
        if self.is_dirty {
            self.calculate_profit();
        }
        self.profit
    }

    pub fn get_profit_per_jump(&mut self) -> f32 {
        if self.is_dirty {
            self.calculate_profit();
        }
        self.profit_per_jump
    }

    pub fn sort_routes(routes: &mut Vec<Route>) {
        for route in routes.iter_mut() {
            route.get_profit_per_jump();
        }
        routes.sort_by(|a, b| {
            b.profit
                .partial_cmp(&a.profit)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn format_number<T>(val: T) -> String
    where
        T: Signed + std::fmt::Display + ToPrimitive + std::cmp::PartialOrd,
    {
        // Convert the value to an integer (if it's a float, it will be truncated)
        let int_val = val.to_i64().expect("Failed to convert value to i64");
        let abs_val = int_val.abs();
        let abs_str = abs_val.to_string();
        let len = abs_str.len();

        // Split the string into chunks of 3 digits from the end
        let mut chunks = Vec::new();
        let mut start = len;
        while start > 0 {
            let end = start;
            start = if start >= 3 { start - 3 } else { 0 };
            chunks.push(&abs_str[start..end]);
        }

        // Reverse the chunks to get the correct order
        chunks.reverse();

        let num = chunks.join(",");

        if int_val < 0 {
            format!("-{}", num)
        } else {
            num
        }
    }

    pub fn represent(&mut self) -> String {
        if !self.is_dirty {
            return self.representation.clone();
        }

        if self.path.is_empty() {
            return String::new();
        }

        let mut representation = String::new();
        let mut systems = Vec::new();
        let mut jumps = 1;

        writeln!(representation, "----------------------------------------\n").unwrap();

        for point in &self.path {
            match point {
                Waypoint::System(system) => {
                    writeln!(
                        representation,
                        "\t{:0>#3}. {:<20} ({:>4.1}) ->",
                        jumps, system.name, system.security_status
                    )
                    .unwrap();
                    systems.push(system.name.clone());
                    jumps += 1;
                }
                Waypoint::Order(order) => {
                    jumps = 0;
                    let order_type = if order.is_buy_order { "Buy" } else { "Sell" };
                    writeln!(
                        representation,
                        "\n\n\t{} order for {} of {} ({} ISK; {} units; {}% cargo).",
                        order_type,
                        order.volume,
                        order.order_type.name,
                        Route::format_number(order.volume as f32 * order.price),
                        order.volume,
                        ((Settings::get_ship_cargo_volume() / order.volume as f32) * 100.0 * 10.0)
                            .round()
                            / 10.0
                    )
                    .unwrap();
                    writeln!(
                        representation,
                        "\tEve Market Browser: {}\n\n",
                        crate::urls::get_market_browser_url(order.order_type.type_id)
                    )
                    .unwrap();
                }
            }
        }

        writeln!(
            representation,
            "\n\nEve Gatecamp Check: {}\n",
            crate::urls::get_gatecamp_url(systems, "secure")
        )
        .unwrap();
        writeln!(representation, "Total jumps: {}\n", self.jumps).unwrap();
        writeln!(
            representation,
            "Total profit: {}\n",
            Route::format_number(self.get_profit())
        )
        .unwrap();
        writeln!(
            representation,
            "Profit per jump: {}\n",
            Route::format_number(self.get_profit_per_jump())
        )
        .unwrap();

        writeln!(representation, "----------------------------------------\n").unwrap();

        writeln!(representation, "\n\n\n").unwrap();

        self.representation = representation.clone();
        self.is_dirty = false;

        representation
    }
}
