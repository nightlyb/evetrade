use log::error;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::types::System;

// TODO. Modify pathfinder, add different heuristics. Security, profit/volume, distance, etc.
// P.S. Do we need this at the end of the day?

pub struct Pathfinder<'a> {
    pub systems: &'a HashMap<u32, System>,
    path_cache: HashMap<(u32, u32), Vec<u32>>,
}

impl<'a> Pathfinder<'a> {
    pub fn new(systems: &'a HashMap<u32, System>) -> Self {
        Pathfinder {
            systems,
            path_cache: HashMap::new(),
        }
    }

    fn heuristic(&self, origin: u32, destination: u32) -> f64 {
        if let (Some(current_system), Some(end_system)) =
            (self.systems.get(&origin), self.systems.get(&destination))
        {
            current_system
                .position
                .distance_squared(&end_system.position)

            // TODO: Should we use distance() instead? Does it affect the result?
            // TODO: Modify the heuristic to include security
        } else {
            error!("Invalid system ID");
            f64::INFINITY
        }
    }

    fn a_star(&self, origin: u32, destination: u32, _security_threshold: f32) -> Option<Vec<u32>> {
        if !self.systems.contains_key(&origin) || !self.systems.contains_key(&destination) {
            return None;
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<u32, u32> = HashMap::new();
        let mut g_scores: HashMap<u32, f64> = HashMap::new();
        let mut closed_set: HashSet<u32> = HashSet::new();

        g_scores.insert(origin, 0.0);
        open_set.push(Node {
            id: origin,
            f_score: self.heuristic(origin, destination),
        });

        while let Some(current) = open_set.pop() {
            let current_id = current.id;

            if current_id == destination {
                return self.reconstruct_path(came_from, current_id);
            }

            if !closed_set.insert(current_id) {
                continue;
            }

            if let Some(system) = self.systems.get(&current_id) {
                for stargate in &system.stargates {
                    if stargate.origin != current_id {
                        continue;
                    }

                    let neighbor_id = stargate.destination;
                    if closed_set.contains(&neighbor_id) {
                        continue;
                    }

                    // Only use Euclidean distance for path cost
                    let g_heuristic = self.heuristic(current_id, neighbor_id);
                    // let tentative_g_score = *g_scores.get(&current_id).unwrap()
                    //     + g_heuristic
                    //     + g_heuristic * stargate.weight;
                    let tentative_g_score = *g_scores.get(&current_id).unwrap() + g_heuristic;

                    if tentative_g_score < *g_scores.get(&neighbor_id).unwrap_or(&f64::INFINITY) {
                        came_from.insert(neighbor_id, current_id);
                        g_scores.insert(neighbor_id, tentative_g_score);

                        open_set.push(Node {
                            id: neighbor_id,
                            f_score: tentative_g_score + self.heuristic(neighbor_id, destination),
                        });
                    }
                }
            }
        }

        None
    }

    fn reconstruct_path(&self, came_from: HashMap<u32, u32>, mut current: u32) -> Option<Vec<u32>> {
        let mut path = vec![current];
        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }
        path.reverse();
        Some(path)
    }

    pub fn compute_path(&mut self, start_id: u32, end_id: u32) -> Option<Vec<u32>> {
        let cache_key = (start_id, end_id);

        if let Some(cached_path) = self.path_cache.get(&cache_key) {
            return Some(cached_path.clone());
        }

        let path = self.a_star(start_id, end_id, -1.0);

        if let Some(path) = path {
            self.path_cache.insert(cache_key, path.clone());
            Some(path)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone)]
struct Node {
    id: u32,
    f_score: f64,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.f_score.total_cmp(&other.f_score) == Ordering::Equal
    }
}

impl Eq for Node {}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (smaller f_score = better)
        other.f_score.total_cmp(&self.f_score)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
