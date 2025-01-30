use core::f32;
use log::{debug, error, info, trace};
use rand::Rng;
use std::collections::{BinaryHeap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};
use std::u32;

use crate::kdtree::Node3D;
use crate::pathfinder::Pathfinder;
use crate::route::Route;
use crate::settings::Settings;
use crate::types::{Order, OrderGroup, System, SystemPath, TradePair, TradeState, Type, Vector3};

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
                    .retain(|order| !self.systems.get(&order.system_id).is_none());
                order_group
                    .sell
                    .retain(|order| !self.systems.get(&order.system_id).is_none());

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
        trade_state: &Option<TradeState>,
    ) -> u32 {
        // TODO: Modify this to take into account trade state phase and inventory.
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
                .min((state.available_isk / sell_order.price) as u32)
                .min((state.available_volume / type_volume) as u32);
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
        state.available_volume > self.cargo_volume * 0.2
            && state.available_isk > profit_goal
            && state.jumps < (self.max_jumps as f32 * 0.2) as u32 // TODO: Add these thresholds to settings? These values are arbitrary
    }

    fn should_accept_pair(
        &self,
        pair: &TradePair,
        state: &TradeState,
        pathfinder: &mut Pathfinder,
    ) -> bool {
        if pair.investment > state.available_isk {
            return false;
        }

        if pair.cargo_volume > state.available_volume {
            return false;
        }

        let jumps_to_sell_order = pathfinder
            .compute_path(state.current_system, pair.sell_order_system.id)
            .unwrap()
            .len() as u32;

        let jumps_in_pair = pathfinder
            .compute_path(pair.sell_order_system.id, pair.buy_order_system.id)
            .unwrap()
            .len() as u32;

        let jumps_from_buy_order = pathfinder
            .compute_path(pair.buy_order_system.id, state.destination)
            .unwrap()
            .len() as u32;

        if jumps_to_sell_order + jumps_from_buy_order + jumps_in_pair > self.max_jumps {
            return false;
        }

        return true;
    }

    fn construct_pairs(&self, nodes_map: &mut HashMap<u32, Vector3>) -> Vec<TradePair> {
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

                    nodes_map.insert(buy_order_system.id, buy_order_system.position.clone());
                    nodes_map.insert(sell_order_system.id, sell_order_system.position.clone());

                    buy_order.volume = units;
                    sell_order.volume = units;

                    pairs_for_type.push(TradePair {
                        buy_order,
                        sell_order,
                        buy_order_system,
                        sell_order_system,
                        cargo_volume,
                        units,
                        profit,
                        investment,
                    });
                }
            }

            pairs.extend(pairs_for_type);
        }

        pairs.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
        assert!(pairs.iter().nth(0).unwrap().profit > pairs.iter().nth(1).unwrap().profit);

        info!("Pairs are constructed and sorted.");
        pairs
    }

    fn find_initial_state(
        &self,
        pairs: &mut Vec<TradePair>,
        pathfinder: &mut Pathfinder,
    ) -> TradeState {
        let mut initial_state: Option<TradeState> = Option::None;
        let mut initial_pair: Option<TradePair> = Option::None;

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

            let profit_per_jump = pair.profit / jumps as f32;
            initial_state = Some(TradeState {
                available_isk: self.initial_capital - pair.investment,
                available_volume: self.cargo_volume - pair.cargo_volume,
                jumps,
                inventory: HashMap::new(),
                visited_systems: std::collections::HashSet::new(),
                current_system: sell_system.id,
                destination: buy_system.id,
                path: SystemPath::new(Vec::new()),
                orders: Vec::new(),
                profit_per_jump,
            });

            initial_pair = Some(pairs.remove(i));

            if let Some(state) = &mut initial_state {
                if let Some(pair) = &initial_pair {
                    state.inventory.insert(pair.buy_order.type_id, pair.units);
                    state.orders.push(pair.sell_order.clone());
                    state.orders.push(pair.buy_order.clone());
                    state.path = SystemPath::new(
                        pathfinder
                            .compute_path(state.current_system, state.destination)
                            .unwrap(),
                    );
                }
            }

            break;
        }

        if initial_state.is_none() {
            panic!("Initial state wasn't found. Exiting...");
            // TODO: Implement proper error handling here.
        }

        initial_state.unwrap()
    }

    fn process_routes(&mut self) -> Vec<Route> {
        info!("Starting processing routes...");

        let mut routes: Vec<Route> = Vec::new();
        let mut nodes_map: HashMap<u32, Vector3> = std::collections::HashMap::new();
        let mut pairs = self.construct_pairs(&mut nodes_map);

        let mut nodes = nodes_map
            .iter()
            .map(|(id, position)| Node3D::new(position, *id))
            .collect::<Vec<_>>();
        let root = Node3D::construct_tree(&mut nodes, 0).unwrap();

        info!("3-D k-d tree is constructed.");

        let mut pathfinder = Pathfinder::new(self.systems, &root);
        let mut states: BinaryHeap<TradeState> = BinaryHeap::new();
        let initial_state: TradeState = self.find_initial_state(&mut pairs, &mut pathfinder);

        let sell_order_position = &self.systems[&initial_state.orders[0].system_id].position;
        let buy_order_position =
            &self.systems[&initial_state.orders[initial_state.orders.len() - 1].system_id].position;

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

        let mut system_pairs: HashMap<u32, Vec<TradePair>> = HashMap::new();
        for pair in pairs {
            // Here we precompute the paths for all pairs and cache them in the pathfinder.
            let _ = pathfinder.compute_path(pair.sell_order_system.id, pair.buy_order_system.id);

            // We also cache the pairs for each system.
            system_pairs
                .entry(pair.sell_order_system.id)
                .or_insert(Vec::new())
                .push(pair);
        }

        /*
            итерируюясь по соседям, смотрим ближайшие точки с трейдами. в зависимости от типа купленного продукта,
            итерируемся по вектору с ордерами на покупку, ищем ближайшую точку. нашли - добавляем в путь. повторяем процесс пока continue_search.
            запоминаем оба стейта и итерируемся по обоим.
        */

        /*
            нужно каким то образом создать вектор стейтов со всеми возможными пермутациями пар. как?

            какой вариант лучше, оптимизировать пути или постепенно их улучшать?

            лучше работать с парами и максимально загружать карго. таким образом, мы будем сто процентно вынуждены лететь после этого в точку продажи.
            если будем полу пустые, то придётся искать очень сложные пермутации путей (A1-A2-B1-B2, вместо A1-B1-B2-A2)
            // TODO: добавить фильтровку пар по загруженности карго?

            нет, полный бред. забиваться полностью нет смысла. модули могут весить очень мало, но могут приносить очень много профита. надо подумать над другим вариантом.

            если работать с парами:
                берём стейт, смотрим ближайшие к current_system пары. добавляем их, модифицируя путь (от точки, где продали, до конечной точки). добавляем системы этих пар в visited_systems.
                инкрементируем current_system в зависимости от пути.
                сохраняем все эти стейты, в том числе и неизменённые.
        */
        let mut output_states: Vec<TradeState> = Vec::new();
        while let Some(state) = states.pop() {
            if !self.continue_search(&state) {
                output_states.push(state);
                continue;
            }

            let current_system = self.systems.get(&state.current_system).unwrap();
            let mut exclude_nodes: Vec<u32> = state.visited_systems.iter().cloned().collect();

            while let Some(nearest_node) = Node3D::nearest_neighbor(
                &Some(root.clone()), // TODO: Fix this. Would it cause a chain of clones? Reference is ideal.
                &current_system.position,
                &exclude_nodes,
            ) {
                let nearest_system_id = nearest_node.system_id;
                let pairs = match system_pairs.get(&nearest_system_id) {
                    Some(group) => group,
                    None => {
                        exclude_nodes.push(nearest_system_id);
                        continue;
                    }
                };
            }

            let mut new_state: Option<TradeState> = Option::None;

            states.push(state);
        }

        drop(system_pairs);

        debug!("Output states: {:?}", output_states);

        routes
    }
}
