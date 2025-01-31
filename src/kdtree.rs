// K-D tree implementation (3 dimensions in this case)

use crate::types::Vector3;

#[derive(Debug, Clone)]
pub struct Node3D<'a> {
    position: &'a Vector3,
    pub system_id: u32,
    left: Option<Box<Node3D<'a>>>,
    right: Option<Box<Node3D<'a>>>,
}

impl<'a> Node3D<'a> {
    pub fn new(position: &'a Vector3, system_id: u32) -> Self {
        Node3D {
            position,
            system_id,
            left: None,
            right: None,
        }
    }

    pub fn construct_tree(nodes: &mut Vec<Node3D<'a>>, depth: usize) -> Option<Box<Node3D<'a>>> {
        if nodes.is_empty() {
            return None;
        }

        //let axis = depth % 3;

        let mut lowest_point = Vector3::new(f64::MAX, f64::MAX, f64::MAX);
        let mut highest_point = Vector3::new(f64::MIN, f64::MIN, f64::MIN);

        // TODO: Search for a better approach? How do we construct the tree with the highest quality possible?
        nodes.iter().for_each(|node| {
            lowest_point.x = lowest_point.x.min(node.position.x);
            lowest_point.y = lowest_point.y.min(node.position.y);
            lowest_point.z = lowest_point.z.min(node.position.z);

            highest_point.x = highest_point.x.max(node.position.x);
            highest_point.y = highest_point.y.max(node.position.y);
            highest_point.z = highest_point.z.max(node.position.z);
        });

        let axis = (highest_point - lowest_point).max_dimension();

        nodes.sort_by(|a, b| match axis {
            0 => a
                .position
                .x
                .partial_cmp(&b.position.x)
                .unwrap_or(std::cmp::Ordering::Equal),
            1 => a
                .position
                .y
                .partial_cmp(&b.position.y)
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => a
                .position
                .z
                .partial_cmp(&b.position.z)
                .unwrap_or(std::cmp::Ordering::Equal),
        });

        let mid = nodes.len() / 2;

        let mut root = nodes.remove(mid);

        root.left = Node3D::construct_tree(&mut nodes[..mid].to_vec(), depth + 1);
        root.right = Node3D::construct_tree(&mut nodes[mid..].to_vec(), depth + 1);

        Some(Box::new(root))
    }

    pub fn nearest_neighbor(
        root: &Option<Box<Node3D<'a>>>,
        target: &Vector3,
        exclude_nodes: &std::collections::HashSet<u32>,
    ) -> Option<Node3D<'a>> {
        if root.is_none() {
            return None;
        }

        let mut best_distance: f64 = 0.0;
        let mut best_node: Option<Node3D<'a>> = None;
        let mut stack: Vec<(&Box<Node3D>, usize)> = Vec::new();

        // Start by pushing the root node onto the stack.
        if let Some(node) = root {
            stack.push((node, 0));
        }

        while let Some((current_node, depth)) = stack.pop() {
            let axis = depth % 3;

            if exclude_nodes.contains(&current_node.system_id) {
                continue;
            }

            // Compute the distance to the target point.
            let dist = target.distance_squared(current_node.position);

            // Update the best point if necessary.
            if best_distance == 0.0 || dist < best_distance {
                best_distance = dist;
                best_node = Some(*current_node.clone());
            }

            // Determine which branch to search first based on the target point.
            let diff = match axis {
                0 => target.x - current_node.position.x,
                1 => target.y - current_node.position.y,
                _ => target.z - current_node.position.z,
            };

            let (first, second) = if diff < 0.0 {
                (&current_node.left, &current_node.right)
            } else {
                (&current_node.right, &current_node.left)
            };

            // Push the first branch to search.
            if let Some(first_child) = first {
                stack.push((first_child, depth + 1));
            }

            // Push the second branch if it might contain a closer point.
            if diff.powi(2) < best_distance {
                if let Some(second_child) = second {
                    stack.push((second_child, depth + 1));
                }
            }
        }

        best_node
    }
}
