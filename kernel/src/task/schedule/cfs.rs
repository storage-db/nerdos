//! A slice-based Red-Black tree
//!
//! Let's assume you want to create a tree holding up to 100 pairs of `u8 <-> f64`:
//! ```
//! use slice_rbtree::tree::{tree_size, RBTree, TreeParams};
//! use std::mem::size_of;
//!
//! // RBTree requires input slice to have a proper size
//! let size = tree_size(
//!     TreeParams {
//!         k_size: size_of::<u8>(),
//!         v_size: size_of::<f64>(),
//!     },
//!     100,
//! );
//!
//! let mut buffer = vec![0; size];
//!
//! let mut tree: RBTree<u8, f64, 1, 8> = RBTree::init_slice(&mut buffer).unwrap();
//!
//! // Insert some values
//! tree.insert(15, 1.245).unwrap();
//! tree.insert(17, 5.5).unwrap();
//! tree.insert(19, 6.5).unwrap();
//!
//! // This type does not implement `Drop` trait - all the data is immediately serialized in the
//! // slice, so this is actually a no-op
//! drop(tree);
//!
//! let mut new_tree: RBTree<u8, f64, 1, 8> = unsafe { RBTree::from_slice(&mut buffer).unwrap() };
//! assert_eq!(new_tree.get(&15), Some(1.245));
//!
//! // Create iterator of key-value pairs and collect in a `Vec`
//! let pairs:Vec<_> = new_tree.pairs().collect();
//! assert_eq!(pairs[0], (15, 1.245));
//! assert_eq!(pairs[1], (17, 5.5));
//! assert_eq!(pairs[2], (19, 6.5));
//!
//! // There are 3 ways to remove a value from the tree:
//! // 1. remove() will deserialize the value to be removed
//! assert_eq!(new_tree.remove(&17), Some(5.5));
//! // 2. remove_entry() will deserialize both the key and the value
//! assert_eq!(new_tree.remove_entry(&15), Some((15,1.245)));
//! // 2. delete() will not deserialize anything, will return `true` if the value was present
//! assert_eq!(new_tree.delete(&19), true);
//! ```
//!
//! # Internal structure
//! Internally, [`RBTree`] is just a wrapper around [`RBForest`](super::forest::RBForest) with `max_roots`
//! equal to `1`. See [`RBForest`](super::forest::RBForest) docs for description of the internals.
use borsh::{BorshDeserialize, BorshSerialize};
use core::borrow::Borrow;
use core::cmp::Ord;
use core::fmt;

pub use super::forest::iterators::{KeysIterator, PairsIterator, ValuesIterator};
use super::forest::{forest_size, init_forest, ForestParams, RBForest};
use super::Error;

/// Parameters required to calculate [`RBTree`] size
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TreeParams {
    ///  key buffer size
    pub k_size: usize,
    ///  value buffer size
    pub v_size: usize,
}
/// Returns the required size of the slice
#[must_use]
#[inline]
pub const fn tree_size(params: TreeParams, max_nodes: usize) -> usize {
    forest_size(
        ForestParams {
            k_size: params.k_size,
            v_size: params.v_size,
            max_roots: 1,
        },
        max_nodes,
    )
}

/// Initializes [`RBTree`] in the given slice without returning it
///
/// This function can be used than you don't know buffer sizes at compile time.
#[inline]
pub fn init_tree(params: TreeParams, slice: &mut [u8]) -> Result<(), Error> {
    init_forest(
        ForestParams {
            k_size: params.k_size,
            v_size: params.v_size,
            max_roots: 1,
        },
        slice,
    )
}

/// A slice-based Red-Black tree
///
/// See [module](super::tree) level documentation for more info.
pub struct RBTree<'a, K, V, const KSIZE: usize, const VSIZE: usize>(
    RBForest<'a, K, V, KSIZE, VSIZE>,
)
where
    K: Ord + BorshDeserialize + BorshSerialize,
    V: BorshDeserialize + BorshSerialize;

impl<'a, K, V, const KSIZE: usize, const VSIZE: usize> RBTree<'a, K, V, KSIZE, VSIZE>
where
    K: Ord + BorshDeserialize + BorshSerialize,
    V: BorshDeserialize + BorshSerialize,
{
    /// Initializes [`RBTree`] in a given slice
    pub fn init_slice(slice: &'a mut [u8]) -> Result<Self, Error> {
        RBForest::<'a, K, V, KSIZE, VSIZE>::init_slice(slice, 1).map(|tree| Self(tree))
    }

    /// Returns [`RBTree`], contained in the given slice
    ///
    /// # Safety
    /// This function must be called only on slices, previously initialized as [`RBTree`] using
    /// [`init_tree`] or [`RBTree::init_slice`]
    pub unsafe fn from_slice(slice: &'a mut [u8]) -> Result<Self, Error> {
        unsafe { RBForest::<'a, K, V, KSIZE, VSIZE>::from_slice(slice).map(|tree| Self(tree)) }
    }

    /// Returns the number of occupied nodes
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len(0).unwrap()
    }

    /// Clears the tree
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns the number of free nodes
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    #[must_use]
    pub fn free_nodes_left(&self) -> usize {
        self.0.free_nodes_left()
    }

    /// Returns true if the map contains a value for the specified key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.contains_key(0, k)
    }

    /// Returns a key-value pair corresponding to the supplied key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn get_entry<Q>(&self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.get_entry(0, k)
    }

    /// Returns the value corresponding to the key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn get<Q>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.get(0, k)
    }

    /// Inserts a new key-value pair and returns the old value if it was present
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, Error> {
        self.0.insert(0, k, v)
    }

    /// Returns `true` if the tree contains no elements
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty(0)
    }

    /// Deletes entry and returns deserialized value
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.remove(0, key)
    }

    /// Deletes entry and returns deserialized key-value pair
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.remove_entry(0, key)
    }

    /// Deletes entry without deserializing the value
    ///
    /// Returns `true` if there was a value with the given key.
    pub fn delete<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        self.0.delete(0, key)
    }

    /// Returns the first key-value pair in the map
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn first_entry(&self) -> Option<(K, V)> {
        self.0.first_entry(0)
    }

    /// Returns the last key-value pair in the map
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn last_entry(&self) -> Option<(K, V)> {
        self.0.last_entry(0)
    }

    /// Creates an iterator over key-value pairs, in order by key
    #[must_use]
    pub fn pairs<'b>(&'b self) -> PairsIterator<'b, 'a, K, V, KSIZE, VSIZE> {
        self.0.pairs(0).unwrap()
    }

    /// Creates an iterator over keys, from smallest to biggest
    #[must_use]
    pub fn keys<'b>(&'b self) -> KeysIterator<'b, 'a, K, V, KSIZE, VSIZE> {
        self.0.keys(0).unwrap()
    }

    /// Creates an iterator over values, in order by key
    #[must_use]
    pub fn values<'b>(&'b self) -> ValuesIterator<'b, 'a, K, V, KSIZE, VSIZE> {
        self.0.values(0).unwrap()
    }
}

impl<'a, K, V, const KSIZE: usize, const VSIZE: usize> fmt::Debug for RBTree<'a, K, V, KSIZE, VSIZE>
where
    K: Ord + BorshDeserialize + BorshSerialize + fmt::Debug,
    V: BorshDeserialize + BorshSerialize + fmt::Debug,
{
    #[cfg(not(test))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_map().entries(self.pairs()).finish()
    }
    #[cfg(test)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl<'a, K, V, const KSIZE: usize, const VSIZE: usize> Extend<(K, V)>
    for RBTree<'a, K, V, KSIZE, VSIZE>
where
    K: Ord + BorshDeserialize + BorshSerialize,
    V: BorshDeserialize + BorshSerialize,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for elem in iter {
            self.insert(elem.0, elem.1).unwrap();
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(any(test, fuzzing))]
pub mod internal_checks;
