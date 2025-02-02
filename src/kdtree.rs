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

    pub fn construct_tree(nodes: &mut [Node3D<'a>], depth: usize) -> Option<Box<Node3D<'a>>> {
        if nodes.is_empty() {
            return None;
        }

        // Determine the axis with the largest spread.
        let axis = {
            let (lowest_point, highest_point) = nodes.iter().fold(
                (
                    Vector3::new(f64::MAX, f64::MAX, f64::MAX),
                    Vector3::new(f64::MIN, f64::MIN, f64::MIN),
                ),
                |(mut low, mut high), node| {
                    low.x = low.x.min(node.position.x);
                    low.y = low.y.min(node.position.y);
                    low.z = low.z.min(node.position.z);
                    high.x = high.x.max(node.position.x);
                    high.y = high.y.max(node.position.y);
                    high.z = high.z.max(node.position.z);
                    (low, high)
                },
            );
            (highest_point - lowest_point).max_dimension()
        };

        // Sort nodes along the chosen axis.
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

        // Find the median index.
        let mid = nodes.len() / 2;

        // Split the nodes into left and right subtrees.
        let mut root = nodes[mid].clone();
        let (left, right) = nodes.split_at_mut(mid);
        let right = &mut right[1..]; // Exclude the median node.

        // Recursively construct the left and right subtrees.
        root.left = Node3D::construct_tree(left, depth + 1);
        root.right = Node3D::construct_tree(right, depth + 1);

        Some(Box::new(root))
    }

    pub fn nearest_neighbor(
        &self,
        target: &Vector3,
        exclude_nodes: &std::collections::HashSet<u32>,
    ) -> Option<Node3D<'a>> {
        let mut best_distance_sq = f64::INFINITY;
        let mut best_node: Option<&Node3D<'a>> = None;
        let mut stack: Vec<(&Node3D<'a>, usize)> = Vec::new();

        stack.push((self, 0));

        while let Some((current_node, depth)) = stack.pop() {
            if exclude_nodes.contains(&current_node.system_id) {
                continue;
            }

            let dist_sq = target.distance_squared(current_node.position);

            // Update the best node if this node is closer.
            if dist_sq < best_distance_sq {
                best_distance_sq = dist_sq;
                best_node = Some(current_node);
            }

            let axis = depth % 3;
            let diff = match axis {
                0 => target.x - current_node.position.x,
                1 => target.y - current_node.position.y,
                _ => target.z - current_node.position.z,
            };

            // Determine the order of traversal based on the splitting plane.
            let (near, far) = if diff < 0.0 {
                (&current_node.left, &current_node.right)
            } else {
                (&current_node.right, &current_node.left)
            };

            // Push the near branch first.
            if let Some(near_child) = near {
                stack.push((near_child, depth + 1));
            }

            // Push the far branch only if it might contain a closer point.
            if diff.powi(2) < best_distance_sq {
                if let Some(far_child) = far {
                    stack.push((far_child, depth + 1));
                }
            }
        }

        // Clone the best node only at the end, if necessary.
        best_node.cloned()
    }
}
