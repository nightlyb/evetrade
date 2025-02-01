use core::f32;
use log::{debug, info};
use std::collections::{BinaryHeap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use std::u32;

use crate::kdtree::Node3D;
use crate::pathfinder::Pathfinder;
use crate::route::Route;
use crate::settings::Settings;
use crate::types::{Order, OrderGroup, System, TradePair, TradePath, TradeState, Type};

#[derive(Debug)]
struct PreprocessStats {
    initial_types: usize,
    removed_empty: usize,
    removed_volume: usize,
    removed_unprofitable: usize,
    final_types: usize,
}

pub struct OrderProcessor<'a> {
    orders: &'a mut HashMap<u32, OrderGroup>, // type id -> order group
    systems: &'a mut HashMap<u32, System>,    // system id -> system
    types: &'a HashMap<u32, Type>,
    cargo_volume: f32,
    initial_capital: f32,
    percentage_threshold: f32,
    max_jumps: u32,
}

impl<'a> OrderProcessor<'a> {
    pub fn new(
        orders: &'a mut HashMap<u32, OrderGroup>,
        systems: &'a mut HashMap<u32, System>,
        types: &'a HashMap<u32, Type>,
        //mean_jump_distance: f64,
    ) -> Self {
        let initial_capital = Settings::get_initial_capital();
        let cargo_volume = Settings::get_ship_cargo_volume();
        let percentage_threshold = Settings::get_percentage_threshold();
        let max_jumps = Settings::get_max_jumps();

        OrderProcessor {
            orders,
            systems,
            types,
            cargo_volume,
            initial_capital,
            percentage_threshold,
            max_jumps,
        }
    }

    pub fn compute(&mut self) -> Vec<Route> {
        info!("Preprocessing orders...");
        let mut start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let stats = self.preprocess_orders();
        let mut end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Processing orders took {:?}ms", (end - start).as_millis());
        debug!("Preprocessing results: {:?}", stats);

        let mut count = 0;
        for group in self.orders.values() {
            count += group.sell.len() + group.buy.len();
        }
        println!("Total orders: {}", count);

        info!("Processing routes...");
        start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let routes = self.process_routes();
        end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Processing routes took {:?} ms.", (end - start).as_millis());

        routes
    }

    fn preprocess_orders(&mut self) -> PreprocessStats {
        let mut stats = PreprocessStats {
            initial_types: self.orders.len(),
            removed_empty: 0,
            removed_volume: 0,
            removed_unprofitable: 0,
            final_types: 0,
        };

        // First pass: Remove obviously invalid orders and sort them
        let type_ids: Vec<_> = self.orders.keys().cloned().collect();
        for type_id in type_ids {
            if let Some(order_group) = self.orders.get_mut(&type_id) {
                if order_group.buy.is_empty() || order_group.sell.is_empty() {
                    self.orders.remove(&type_id);
                    stats.removed_empty += 1;
                    continue;
                }

                if let Some(item_type) = self.types.get(&type_id) {
                    if item_type.volume > self.cargo_volume {
                        self.orders.remove(&type_id);
                        stats.removed_volume += 1;
                        continue;
                    }
                } else {
                    self.orders.remove(&type_id);
                    continue;
                }

                order_group
                    .buy
                    .retain(|order| self.systems.get(&order.system_id).is_some());
                order_group
                    .sell
                    .retain(|order| self.systems.get(&order.system_id).is_some());

                // Descending for buy, ascending for sell
                order_group
                    .buy
                    .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
                order_group
                    .sell
                    .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            }

            if let Some(order_group) = self.orders.get_mut(&type_id) {
                let buy_price = order_group.buy[0].price;
                let sell_price = order_group.sell[0].price;
                if (buy_price - sell_price) / sell_price < self.percentage_threshold {
                    self.orders.remove(&type_id);
                    stats.removed_unprofitable += 1;
                    continue;
                }

                OrderProcessor::truncate_orders(order_group, self.percentage_threshold);

                if order_group.buy.is_empty() || order_group.sell.is_empty() {
                    self.orders.remove(&type_id);
                    stats.removed_unprofitable += 1;
                }
            }
        }

        self.orders
            .retain(|_, group| !group.buy.is_empty() && !group.sell.is_empty());
        stats.final_types = self.orders.len();

        stats
    }

    fn truncate_orders(order_group: &mut OrderGroup, percentage_threshold: f32) {
        let mut truncate_buy_at = order_group.buy.len();
        let mut truncate_sell_at = order_group.sell.len();

        // For each buy order, find if it has ANY profitable pairs
        // P.S. Should I refactor this? Looks too complex, but we should normally break early
        for (i, buy_order) in order_group.buy.iter().enumerate() {
            let mut has_profitable_trade = false;
            for sell_order in order_group.sell.iter() {
                let profit_ratio = (buy_order.price - sell_order.price) / sell_order.price;
                if profit_ratio >= percentage_threshold {
                    has_profitable_trade = true;
                    break;
                }
            }
            if has_profitable_trade {
                truncate_buy_at = i + 1; // Keep this buy order
            } else {
                break; // No need to check further buy orders (they'll be worse)
            }
        }

        // Doing the same thingy for sell orders
        for (k, sell_order) in order_group.sell.iter().enumerate() {
            let mut has_profitable_trade = false;
            for buy_order in order_group.buy.iter() {
                let profit_ratio = (buy_order.price - sell_order.price) / sell_order.price;
                if profit_ratio >= percentage_threshold {
                    has_profitable_trade = true;
                    break;
                }
            }
            if has_profitable_trade {
                truncate_sell_at = k + 1;
            } else {
                break;
            }
        }

        order_group.buy.truncate(truncate_buy_at);
        order_group.sell.truncate(truncate_sell_at);
    }

    fn calculate_max_units(
        &self,
        type_id: u32,
        buy_order: &Order,
        sell_order: &Order,
        trade_state: &Option<&TradeState>,
    ) -> u32 {
        let type_volume = self
            .types
            .get(&type_id)
            .map(|t| t.volume)
            .unwrap_or(f32::MAX);
        let volume_limit: u32 = (self.cargo_volume / type_volume).floor() as u32;
        let capital_limit: u32 = (self.initial_capital / sell_order.price).floor() as u32;
        let mut state_limit = u32::MAX;

        if let Some(state) = trade_state {
            state_limit = state_limit
                .min((state.get_available_isk() / sell_order.price) as u32)
                .min((state.get_available_volume() / type_volume) as u32);
        }

        volume_limit
            .min(capital_limit)
            .min(buy_order.volume)
            .min(sell_order.volume)
            .min(volume_limit)
            .min(state_limit)
    }

    fn continue_search(&self, state: &TradeState) -> bool {
        let profit_goal = 20_000_000.0; // TODO: Add configurable goal.
        if state.path.current_system() == state.path.destination() {
            return false;
        }

        state.get_profit() > profit_goal && state.get_jumps() < (self.max_jumps as f32 * 0.9) as u32
        // TODO: Add these thresholds to settings? These values are arbitrary
    }

    fn should_accept_pair(
        &self,
        pair: &TradePair,
        state: &TradeState,
        pathfinder: &mut Pathfinder,
    ) -> bool {
        let jumps_to_sell_order = pathfinder
            .compute_path(state.path.current_system(), pair.sell_order_system.id)
            .unwrap()
            .len() as u32;

        let jumps_in_pair = pathfinder
            .compute_path(pair.sell_order_system.id, pair.buy_order_system.id)
            .unwrap()
            .len() as u32;

        // let jumps_from_buy_order = pathfinder
        //     .compute_path(pair.buy_order_system.id, state.destination)
        //     .unwrap()
        //     .len() as u32;

        if jumps_to_sell_order + jumps_in_pair // + jumps_from_buy_order
            > (self.max_jumps as f32 * 1.2) as u32
        // TODO: Add this threshold too in the settings too? Remove it?
        {
            return false;
        }

        true
    }

    fn construct_pairs(&self) -> Vec<TradePair> {
        let mut pairs: Vec<TradePair> = Vec::new();

        for (type_id, order_group) in self.orders.iter() {
            let type_id = *type_id;
            let type_orders = order_group.buy.len() * order_group.sell.len();
            let mut pairs_for_type = Vec::with_capacity(type_orders);

            for old_buy_order in &order_group.buy {
                for old_sell_order in &order_group.sell {
                    let mut buy_order = old_buy_order.clone();
                    let mut sell_order = old_sell_order.clone();

                    let buy_order_system = self.systems.get(&buy_order.system_id).unwrap();
                    let sell_order_system = self.systems.get(&sell_order.system_id).unwrap();
                    let units = self.calculate_max_units(type_id, &buy_order, &sell_order, &None);
                    let profit = (buy_order.price - sell_order.price) * units as f32;
                    let cargo_volume = self.types.get(&type_id).unwrap().volume * units as f32;
                    let investment = sell_order.price * units as f32;

                    if profit < 5_000_000.0 {
                        // TODO: Add this threshold to settings
                        continue;
                    }

                    buy_order.volume = units;
                    sell_order.volume = units;

                    pairs_for_type.push(TradePair {
                        buy_order,
                        sell_order,
                        buy_order_system,
                        sell_order_system,
                        cargo_volume,
                        profit,
                        investment,
                    });
                }
            }

            pairs.extend(pairs_for_type);
        }

        pairs.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
        assert!(pairs.first().unwrap().profit > pairs.get(1).unwrap().profit);

        info!("Pairs are constructed and sorted.");
        pairs
    }

    fn find_initial_state(
        &self,
        pairs: &mut Vec<TradePair>,
        pathfinder: &mut Pathfinder,
    ) -> TradeState {
        let mut initial_state: Option<TradeState> = Option::None;

        for (i, pair) in pairs.iter().enumerate() {
            if pair.investment > self.initial_capital {
                continue;
            }

            let sell_system = pair.sell_order_system;
            let buy_system = pair.buy_order_system;
            let jumps = pathfinder
                .compute_path(sell_system.id, buy_system.id)
                .unwrap()
                .len() as u32;
            if jumps > self.max_jumps {
                continue; // this block is only needed for debug (checking how many times we will go into the block below)
            }
            if jumps > self.max_jumps - (self.max_jumps as f32 * 0.8) as u32 {
                // TODO: Add threshold to settings
                debug!("Skipping initial pair candidate. Not enough jumps remanining.");
                continue;
            }

            // TODO:
            // Ask user if they want to use this pair. We need to add ability for the user to change the initial pair in case they don't like the results.
            // Ask user on startup for origin system_id and then rate initial pair candidates by profit/distance to origin system?

            initial_state = Some(TradeState {
                initial_capital: self.initial_capital,
                initial_volume: self.cargo_volume,
                visited_systems: std::collections::HashSet::new(),
                path: TradePath::new(),
            });

            if let Some(state) = &mut initial_state {
                let pair = pairs.remove(i);
                state
                    .path
                    .insert_order(pair.sell_order.clone(), 0, pathfinder);
                state
                    .path
                    .insert_order(pair.buy_order.clone(), 1, pathfinder);
            }

            break;
        }

        if initial_state.is_none() {
            panic!("Initial state wasn't found. Exiting...");
            // TODO: Implement proper error handling here.
        }

        initial_state.unwrap()
    }

    fn fit_pair(&self, pair: &mut TradePair, state: &TradeState) {
        if pair.investment > state.get_available_isk()
            || pair.cargo_volume > state.get_available_volume()
        {
            let units = self.calculate_max_units(
                pair.buy_order.type_id,
                &pair.buy_order,
                &pair.sell_order,
                &Some(state),
            );

            pair.buy_order.volume = units;
            pair.sell_order.volume = units;
            pair.cargo_volume =
                self.types.get(&pair.buy_order.type_id).unwrap().volume * units as f32;
            pair.investment = pair.sell_order.price * units as f32;
        }
    }

    fn preprocess_pairs(&self, pairs: &mut Vec<TradePair>, initial_state: &TradeState) {
        let sell_order_position = &self.systems[&initial_state
            .path
            .get_all_orders()
            .first()
            .unwrap()
            .system_id]
            .position;
        let buy_order_position = &self.systems[&initial_state
            .path
            .get_all_orders()
            .last()
            .unwrap()
            .system_id]
            .position;

        let vector = buy_order_position - sell_order_position;
        let middle_point = sell_order_position + &(&vector * 0.5);

        debug!("Number of unprocessed pairs: {}.", pairs.len());
        pairs.retain(|pair| {
            // TODO: Add this threshold to settings? Modify the value? It is also arbitrary.
            // Can this be dynamic, based on the max-jumps?
            let similarity = middle_point.similarity(&pair.buy_order_system.position, None, None);
            similarity >= 0.9
        });
        info!("Number of pairs left: {}.", pairs.len());

        assert_ne!(pairs.len(), 0); // TODO: Proper error handling here.
    }

    fn construct_states(
        &self,
        state: &TradeState,
        pair: &TradePair,
        pathfinder: &mut Pathfinder,
    ) -> Vec<TradeState> {
        let mut new_states: Vec<TradeState> = Vec::new();
        let mut new_state = state.clone();

        new_state.path.insert_order(
            pair.sell_order.clone(),
            new_state.path.current_point_index() + 1,
            pathfinder,
        );

        new_state.path.next_system_id();

        new_state.path.insert_order(
            pair.buy_order.clone(),
            new_state.path.current_point_index() + 1,
            pathfinder,
        );

        new_states.push(new_state);

        // for i in base_path.current_point_index()..base_path.len() {
        //     let mut new_path = base_path.clone();
        //     new_path.insert_order(pair.buy_order.clone(), i, pathfinder);

        //     // Create a new TradeState with the updated path
        //     let new_state = TradeState {
        //         path: new_path,
        //         available_isk: state.available_isk - pair.investment,
        //         available_volume: state.available_volume - pair.cargo_volume,
        //         jumps: state.jumps + 1,
        //         visited_systems: state.visited_systems.clone(),
        //         profit_per_jump: (state.profit_per_jump * state.jumps as f32 + pair.profit)
        //             / (state.jumps + 1) as f32,
        //     };

        //     new_states.push(new_state);
        // }

        new_states
    }

    fn process_routes(&mut self) -> Vec<Route> {
        info!("Starting processing routes...");

        let mut routes: Vec<Route> = Vec::new();
        let mut output_states: Vec<TradeState> = Vec::new();
        let mut pathfinder = Pathfinder::new(self.systems);

        let mut pairs = self.construct_pairs();

        let initial_state: TradeState = self.find_initial_state(&mut pairs, &mut pathfinder);
        self.preprocess_pairs(&mut pairs, &initial_state);

        let mut states: BinaryHeap<TradeState> = BinaryHeap::new();
        states.push(initial_state);

        let mut system_pairs: HashMap<u32, Vec<&mut TradePair>> = HashMap::new();
        let mut nodes_set: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut nodes: Vec<Node3D> = Vec::new();
        for pair in &mut pairs {
            // Here we precompute the paths for all pairs and cache them in the pathfinder.
            let _ = pathfinder.compute_path(pair.sell_order_system.id, pair.buy_order_system.id);

            // And collect the nodes for the K-D tree.
            if nodes_set.get(&pair.sell_order_system.id).is_none() {
                nodes.push(Node3D::new(
                    &pair.sell_order_system.position,
                    pair.sell_order_system.id,
                ));

                nodes_set.insert(pair.sell_order_system.id);
            }

            // We also cache the pairs for each system.
            system_pairs
                .entry(pair.sell_order_system.id)
                .or_default()
                .push(pair);
        }

        let root = Node3D::construct_tree(&mut nodes, 0).unwrap();
        info!("3-D k-d tree is constructed.");

        info!("Searching for routes...");
        while let Some(mut state) = states.pop() {
            if !self.continue_search(&state) {
                output_states.push(state);
                break;
            }

            let current_system = self.systems.get(&state.path.current_system()).unwrap();

            if let Some(nearest_node) = Node3D::nearest_neighbor(
                // TODO: here we only check one nearest node.
                &Some(root.clone()),
                &current_system.position,
                &state.visited_systems,
            ) {
                let nearest_system_id = nearest_node.system_id;
                state.visited_systems.insert(nearest_system_id);

                if let Some(closest_pairs) = system_pairs.get_mut(&nearest_system_id) {
                    for pair in closest_pairs.iter_mut() {
                        self.fit_pair(pair, &state);

                        if self.should_accept_pair(pair, &state, &mut pathfinder) {
                            states.extend(self.construct_states(&state, &pair, &mut pathfinder));
                        }
                    }
                }
            }

            state.path.next_system_id();
            states.push(state);
        }

        for state in output_states {
            routes.push(state.to_route(&mut pathfinder));
        }

        routes
    }
}
