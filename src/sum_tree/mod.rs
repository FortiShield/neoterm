use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt;

/// A trait for types that can be used as values in a `SumTree`.
/// These values must be able to be summed and subtracted.
pub trait SumTreeValue:
    Sized + Copy + Default + Add<Output = Self> + AddAssign + Sub<Output = Self> + SubAssign + PartialEq + fmt::Debug
{
    // No additional methods needed, just trait bounds.
}

// Implement for common numeric types
impl SumTreeValue for usize {}
impl SumTreeValue for u32 {}
impl SumTreeValue for i32 {}
impl SumTreeValue for f32 {}
impl SumTreeValue for f64 {}

/// A node in the `SumTree`.
#[derive(Debug, Clone, Copy)]
struct Node<T: SumTreeValue> {
    value: T,
    left: Option<usize>,
    right: Option<usize>,
    parent: Option<usize>,
}

/// A `SumTree` is a data structure that allows efficient querying of prefix sums
/// and updating of individual elements. It's often used in scenarios like
/// text buffers where you need to quickly find a character by its index
/// or a line by its byte offset, and also efficiently insert/delete text.
///
/// Each leaf node represents an element, and internal nodes store the sum
/// of their children's values.
pub struct SumTree<T: SumTreeValue> {
    nodes: Vec<Node<T>>,
    root: Option<usize>,
    next_node_idx: usize,
}

impl<T: SumTreeValue> SumTree<T> {
    /// Creates a new empty `SumTree`.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            next_node_idx: 0,
        }
    }

    /// Inserts a new value at a specific index.
    /// The index refers to the logical position in the sequence represented by the leaves.
    pub fn insert(&mut self, index: usize, value: T) {
        let new_node_idx = self.allocate_node(Node {
            value,
            left: None,
            right: None,
            parent: None,
        });

        if self.root.is_none() {
            self.root = Some(new_node_idx);
            return;
        }

        // Find the insertion point (simplified for a basic implementation)
        // A full implementation would traverse the tree to find the correct leaf
        // and then rebalance/update sums up to the root.
        // For this stub, we'll just add it as a new root if empty, or panic.
        // This method is primarily illustrative of the concept.
        if self.nodes.len() == 1 && self.root == Some(0) {
            // If there's only one node, we need to create a new parent
            let old_root_idx = self.root.unwrap();
            let new_parent_idx = self.allocate_node(Node {
                value: self.nodes[old_root_idx].value + value,
                left: Some(old_root_idx),
                right: Some(new_node_idx),
                parent: None,
            });
            self.nodes[old_root_idx].parent = Some(new_parent_idx);
            self.nodes[new_node_idx].parent = Some(new_parent_idx);
            self.root = Some(new_parent_idx);
        } else {
            // This simplified insert only handles empty tree or adding to a single-node tree.
            // A real SumTree would involve more complex tree manipulation.
            panic!("Simplified SumTree insert only supports empty tree or adding to a single-node tree.");
        }
    }

    /// Updates the value of an element at a specific index.
    /// This involves traversing to the leaf node and updating sums up to the root.
    pub fn update(&mut self, index: usize, new_value: T) -> Result<(), String> {
        // Find the leaf node corresponding to the index (simplified)
        // In a
