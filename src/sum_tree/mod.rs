use anyhow::{Result, anyhow};

// This module implements a Sum Tree (also known as a Fenwick tree or Binary Indexed Tree),
// a data structure that can efficiently update elements and calculate prefix sums.
// It's useful for applications like prioritized experience replay in RL, or
// efficient range queries in text editors.

pub struct SumTree {
    tree: Vec<f64>, // Stores the sums
    data: Vec<f64>, // Stores the actual values
    capacity: usize,
}

impl SumTree {
    pub fn new(capacity: usize) -> Self {
        let tree_size = capacity * 2 - 1; // For a complete binary tree
        Self {
            tree: vec![0.0; tree_size],
            data: vec![0.0; capacity],
            capacity,
        }
    }

    pub fn init(&self) {
        log::info!("Sum tree manager initialized.");
    }

    /// Creates a new SumTree from a vector of values.
    pub fn create_tree(&self, values: Vec<f64>) -> SumTree {
        let capacity = values.len();
        let mut tree = SumTree::new(capacity);
        for (i, &val) in values.iter().enumerate() {
            tree.update(i, val);
        }
        tree
    }

    /// Updates the value at a given index and propagates the change up the tree.
    pub fn update(&mut self, mut idx: usize, val: f64) {
        if idx >= self.capacity {
            log::warn!("Index {} out of bounds for SumTree with capacity {}", idx, self.capacity);
            return;
        }
        let change = val - self.data[idx];
        self.data[idx] = val;

        // Adjust index to be 0-based for the data array, then map to tree index
        idx += self.capacity - 1; // Leaf node index in the tree array

        while idx >= 0 && idx < self.tree.len() {
            self.tree[idx] += change;
            if idx == 0 { break; } // Root node
            idx = (idx - 1) / 2; // Move to parent
        }
    }

    /// Queries the sum of values up to a given index (prefix sum).
    pub fn query_prefix_sum(&self, mut idx: usize) -> f64 {
        if idx >= self.capacity {
            log::warn!("Query index {} out of bounds for SumTree with capacity {}", idx, self.capacity);
            idx = self.capacity - 1; // Clamp to max index
        }
        let mut sum = 0.0;
        idx += self.capacity - 1; // Leaf node index

        while idx >= 0 && idx < self.tree.len() {
            sum += self.tree[idx];
            if idx == 0 { break; } // Root node
            idx = (idx - 1) / 2; // Move to parent
        }
        sum
    }

    /// Retrieves the value at a specific index.
    pub fn get_value(&self, idx: usize) -> Option<f64> {
        self.data.get(idx).cloned()
    }

    /// Finds the index of the first element whose prefix sum is greater than or equal to `sum`.
    pub fn query_index_by_sum(&self, mut sum: f64) -> Option<usize> {
        let mut idx = 0; // Start at root

        while idx < self.capacity - 1 { // While not a leaf node
            let left_child_idx = 2 * idx + 1;
            let right_child_idx = 2 * idx + 2;

            if left_child_idx < self.tree.len() && sum <= self.tree[left_child_idx] {
                idx = left_child_idx;
            } else if right_child_idx < self.tree.len() {
                if left_child_idx < self.tree.len() {
                    sum -= self.tree[left_child_idx];
                }
                idx = right_child_idx;
            } else {
                // Should not happen in a well-formed tree if sum is within total
                return None;
            }
        }
        Some(idx - (self.capacity - 1)) // Convert back to data index
    }

    /// Returns the total sum of all elements in the tree.
    pub fn total_sum(&self) -> f64 {
        if self.tree.is_empty() { 0.0 } else { self.tree[0] } // Root contains total sum
    }
}
