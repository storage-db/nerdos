Skip to content
Search or jump to…
Pull requests
Issues
Codespaces
Marketplace
Explore
 
@cargo-youth 
solcery
/
slice-rbtree
Public
Fork your own copy of solcery/slice-rbtree
Code
Issues
Pull requests
Actions
Projects
Security
Insights
slice-rbtree/src/forest.rs /
@totikom
totikom Fixed clippy warns
Latest commit cbeac75 on Nov 27, 2022
 History
 1 contributor
1257 lines (1122 sloc)  46.3 KB

//! A slice-based forest of Red-Black trees
//!
//! It sometimes happens, that you have to use a set of similar trees of unknown size. In that
//! case you could allocate such trees in different slices, but it will be very ineffective: you
//! have to think about capacity of each tree beforehand and it is still possible, that some trees
//! will be full, while others are (almost) empty.
//!
//! [`RBForest`] solves this issue, by using a common node pool for a set of trees.
//! the API of [`RBForest`] mimics [`RBTree`](super::tree::RBTree) but with one additional argument: index of the tree.
//!
//!```
//! use slice_rbtree::forest::{forest_size, RBForest, ForestParams};
//! let size = forest_size(ForestParams {k_size: 50, v_size: 50, max_roots: 2}, 10);
//! let mut buffer = vec![0; size];
//! // `String` type has variable length, but we have to chose some fixed maximum length (50 bytes for both key and value)
//! let mut reviews: RBForest<String, String, 50, 50> = RBForest::init_slice(&mut buffer, 2).unwrap();
//!
//! // Let tree 0 be the movie tree and tree 1 - the book tree
//!
//! // review some movies.
//! reviews.insert(0,"Office Space".to_string(),       "Deals with real issues in the workplace.".to_string());
//! reviews.insert(0,"Pulp Fiction".to_string(),       "Masterpiece.".to_string());
//! reviews.insert(0,"The Godfather".to_string(),      "Very enjoyable.".to_string());
//! reviews.insert(0,"The Blues Brothers".to_string(), "Eye lyked it a lot.".to_string());
//!
//! // review some books
//! reviews.insert(1,"Fight club".to_string(),       "Brad Pitt is cool!".to_string());
//! reviews.insert(1,"Alice in Wonderland".to_string(),       "Deep than you think.".to_string());
//! reviews.insert(1,"1984".to_string(),      "A scary dystopia.".to_string());
//! reviews.insert(1,"The Lord of the Rings".to_string(), "Poor Gollum.".to_string());
//!
//! // check for a specific one.
//! if !reviews.contains_key(0,"Les Misérables") {
//!     println!("We've got {} movie reviews, but Les Misérables ain't one.",
//!              reviews.len(0).unwrap());
//! }
//! if reviews.contains_key(1,"1984") {
//!     println!("We've got {} book reviews and 1984 among them: {}.",
//!              reviews.len(0).unwrap(), reviews.get(1, "1984").unwrap());
//! }
//!
//! // oops, this review has a lot of spelling mistakes, let's delete it.
//! reviews.remove(0, "The Blues Brothers");
//!
//! // look up the values associated with some keys.
//! let to_find = ["Up!".to_string(), "Office Space".to_string()];
//! for movie in &to_find {
//!     match reviews.get(0, movie) {
//!        Some(review) => println!("{movie}: {review}"),
//!        None => println!("{movie} is unreviewed.")
//!     }
//! }
//!
//! // iterate over movies.
//! for (movie, review) in reviews.pairs(0).expect("No such tree") {
//!     println!("{movie}: \"{review}\"");
//! }
//!
//! // Too many reviews, delete them all!
//! reviews.clear();
//! assert!(reviews.is_empty(0));
//! assert!(reviews.is_empty(1));
//! ```
//!
//! # Internal structure
//! To read about internal data layout, compile docs with `--document-private-items` and see
//! [`internals`] module.

// Added this, because this lint ignores, that len() method also has different signature
#![allow(clippy::len_without_is_empty)]

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{cast_mut, cast_slice_mut};
use core::borrow::Borrow;
use core::cmp::Ord;
use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;
use core::mem;

mod internals;
pub mod iterators;

pub(crate) use internals::{Header, Node};

use iterators::{KeysIterator, PairsIterator, ValuesIterator};

use super::Error;

/// Parameters required to calculate [`RBForest`] size
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ForestParams {
    ///  key buffer size
    pub k_size: usize,
    ///  value buffer size
    pub v_size: usize,
    ///  maximum number of trees in the forest
    pub max_roots: usize,
}

/// Returns the required size of the slice
#[must_use]
#[inline]
pub const fn forest_size(params: ForestParams, max_nodes: usize) -> usize {
    mem::size_of::<Header>()
        + (mem::size_of::<Node<0, 0>>() + params.k_size + params.v_size) * max_nodes
        + 4 * params.max_roots
}

/// Initializes [`RBForest`] in the given slice without returning it
///
/// This function can be used then you don't know buffer sizes at compile time.
/// Computationally it is identical to [`init_slice`](RBForest::init_slice), but without const
/// generics
pub fn init_forest(params: ForestParams, slice: &mut [u8]) -> Result<(), Error> {
    if slice.len() <= mem::size_of::<Header>() {
        return Err(Error::TooSmall);
    }

    let (header, tail) = slice.split_at_mut(mem::size_of::<Header>());

    if tail.len() <= params.max_roots * 4 {
        return Err(Error::TooSmall);
    }

    let (nodes, roots) = tail.split_at_mut(tail.len() - params.max_roots * 4);

    if nodes.len() % (mem::size_of::<Node<0, 0>>() + params.k_size + params.v_size) != 0 {
        return Err(Error::WrongSliceSize);
    }

    if nodes.len() / (mem::size_of::<Node<0, 0>>() + params.k_size + params.v_size)
        > u32::MAX as usize
    {
        return Err(Error::TooBig);
    }

    let header: &mut [[u8; mem::size_of::<Header>()]] = cast_slice_mut(header);
    let header: &mut Header = cast_mut(&mut header[0]);
    let roots: &mut [[u8; 4]] = cast_slice_mut(roots);

    // Allocator initialization

    // Here comes the most fragile part of that function.
    // Our node allocator is just a singly-linked list of all free nodes.
    // parent field of Node<_,_> struct is used as a link field, because that sounded adequate.
    // Since size_of<Node<k,v>> depends on k and v, which is unknown at compile-time, we can not
    // cast from &[u8] to &[Node<_,_>]. However, Node memory layout is stabilized, so here we will
    // properly initialize nodes by offsetting to the needed fields.
    let mut nodes =
        nodes.chunks_exact_mut(mem::size_of::<Node<0, 0>>() + params.k_size + params.v_size);

    let nodes_len = nodes.len() as u32;

    // parent field occupy 4 bytes starting from (k_size + v_size + 4 + 4) in big-endian.
    let parent_offset = params.k_size + params.v_size + 4 + 4;
    // Bit flags occupy parent_offset + 4, is_parent_present is bit 3.
    let flags_offset = parent_offset + 4;
    if let Some(first_node) = nodes.next() {
        first_node[flags_offset] = 0b0000; // No flags set. All the values are set to None.
    }

    for (i, node) in nodes.enumerate() {
        node[parent_offset..flags_offset].copy_from_slice(&u32::to_be_bytes(i as u32));
        node[flags_offset] = 0b0100;
    }

    // Roots initialization
    for root in roots.iter_mut() {
        *root = u32::to_be_bytes(u32::MAX);
    }

    header.fill(
        params.k_size as u16,
        params.v_size as u16,
        nodes_len,
        params.max_roots as u32,
        Some(nodes_len - 1),
    );
    Ok(())
}

/// A slice-based forest of Red-Black trees
pub struct RBForest<'a, K, V, const KSIZE: usize, const VSIZE: usize>
where
    K: Ord + BorshDeserialize + BorshSerialize,
    V: BorshDeserialize + BorshSerialize,
{
    header: &'a mut Header,
    nodes: &'a mut [Node<KSIZE, VSIZE>],
    roots: &'a mut [[u8; 4]],
    _phantom_key: PhantomData<K>,
    _phantom_value: PhantomData<V>,
    // This field is used to check if new value fits the existing node
    // See put() method
    buffer: [u8; VSIZE],
}

impl<'a, K, V, const KSIZE: usize, const VSIZE: usize> RBForest<'a, K, V, KSIZE, VSIZE>
where
    K: Ord + BorshDeserialize + BorshSerialize,
    V: BorshDeserialize + BorshSerialize,
{
    /// Initializes [`RBForest`] in a given slice
    ///
    /// This function runs in `O(n)` where `n` is a number of nodes, however it is very fast,
    /// because it have to write only 4 bytes per node
    pub fn init_slice(slice: &'a mut [u8], max_roots: usize) -> Result<Self, Error> {
        if slice.len() <= mem::size_of::<Header>() {
            return Err(Error::TooSmall);
        }

        let (header, tail) = slice.split_at_mut(mem::size_of::<Header>());

        if tail.len() <= max_roots * 4 {
            return Err(Error::TooSmall);
        }

        let (nodes, roots) = tail.split_at_mut(tail.len() - max_roots * 4);

        if nodes.len() % mem::size_of::<Node<KSIZE, VSIZE>>() != 0 {
            return Err(Error::WrongSliceSize);
        }

        let nodes: &mut [Node<KSIZE, VSIZE>] = cast_slice_mut(nodes);
        let header: &mut [[u8; mem::size_of::<Header>()]] = cast_slice_mut(header);
        let header: &mut Header = cast_mut(&mut header[0]);
        let roots: &mut [[u8; 4]] = cast_slice_mut(roots);

        if nodes.len() > u32::MAX as usize {
            return Err(Error::TooBig);
        }

        // Allocator initialization
        nodes[0].set_parent(None);

        for (i, node) in nodes.iter_mut().enumerate().skip(1) {
            node.set_parent(Some((i - 1) as u32));
        }

        // Roots initialization
        for root in roots.iter_mut() {
            *root = u32::to_be_bytes(u32::MAX);
        }

        header.fill(
            KSIZE as u16,
            VSIZE as u16,
            nodes.len() as u32,
            max_roots as u32,
            Some((nodes.len() - 1) as u32),
        );

        Ok(Self {
            header,
            nodes,
            roots,
            _phantom_key: PhantomData::<K>,
            _phantom_value: PhantomData::<V>,
            buffer: [0; VSIZE],
        })
    }

    /// Returns [`RBForest`], contained in the given slice
    ///
    /// # Safety
    /// This function must be called only on slices, previously initialized as [`RBForest`] using
    /// [`init_forest`] or [`RBForest::init_slice`]
    pub unsafe fn from_slice(slice: &'a mut [u8]) -> Result<Self, Error> {
        if slice.len() <= mem::size_of::<Header>() {
            return Err(Error::TooSmall);
        }

        let (header, tail) = slice.split_at_mut(mem::size_of::<Header>());

        let header: &mut [[u8; mem::size_of::<Header>()]] = cast_slice_mut(header);
        let header: &mut Header = cast_mut(&mut header[0]);

        if !header.check_magic() {
            return Err(Error::WrongMagic);
        }

        if tail.len() <= (header.max_roots() as usize) * 4 {
            return Err(Error::TooSmall);
        }

        let (nodes, roots) = tail.split_at_mut(tail.len() - (header.max_roots() as usize) * 4);
        let roots: &mut [[u8; 4]] = cast_slice_mut(roots);

        if nodes.len() % mem::size_of::<Node<KSIZE, VSIZE>>() != 0 {
            return Err(Error::WrongSliceSize);
        }

        let nodes: &mut [Node<KSIZE, VSIZE>] = cast_slice_mut(nodes);

        if nodes.len() > u32::MAX as usize {
            return Err(Error::TooBig);
        }

        if header.k_size() as usize != KSIZE {
            return Err(Error::WrongKeySize);
        }

        if header.v_size() as usize != VSIZE {
            return Err(Error::WrongValueSize);
        }

        if header.max_nodes() as usize != nodes.len() {
            return Err(Error::WrongNodePoolSize);
        }

        Ok(Self {
            header,
            nodes,
            roots,
            _phantom_key: PhantomData::<K>,
            _phantom_value: PhantomData::<V>,
            buffer: [0; VSIZE],
        })
    }

    /// Returns the number of occupied nodes
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    pub fn len(&self, tree_id: usize) -> Result<usize, Error> {
        if self.roots.len() > tree_id {
            Ok(self.size(self.root(tree_id)))
        } else {
            Err(Error::TooBigTreeId)
        }
    }

    /// Returns the maximum number of trees in the forest
    #[must_use]
    pub fn max_roots(&self) -> usize {
        self.header.max_roots() as usize
    }

    /// Returns the number of free nodes
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    #[must_use]
    pub fn free_nodes_left(&self) -> usize {
        let mut counter = 0;
        let mut maybe_id = self.header.head();
        while let Some(id) = maybe_id {
            counter += 1;
            maybe_id = self.nodes[id as usize].parent();
        }
        counter
    }

    /// Clears the forest
    ///
    /// This function runs in `O(n)`, where `n` - is the number of nodes
    pub fn clear(&mut self) {
        // Allocator reinitialization
        self.nodes[0].set_parent(None);

        for (i, node) in self.nodes.iter_mut().enumerate().skip(1) {
            node.set_parent(Some((i - 1) as u32));
        }

        for tree_id in 0..self.roots.len() {
            self.set_root(tree_id, None);
        }
        self.header.set_head(Some((self.nodes.len() - 1) as u32));
    }

    /// Returns true if the map contains a value for the specified key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn contains_key<Q>(&self, tree_id: usize, k: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, k).is_some()
        } else {
            false
        }
    }

    /// Returns a key-value pair corresponding to the supplied key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn get_entry<Q>(&self, tree_id: usize, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, k).map(|id| {
                let node = &self.nodes[id];
                let node_key = K::deserialize(&mut node.key.as_slice()).expect("Key corrupted");
                let node_value =
                    V::deserialize(&mut node.value.as_slice()).expect("Value corrupted");
                (node_key, node_value)
            })
        } else {
            None
        }
    }

    /// Returns the value corresponding to the key
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn get<Q>(&self, tree_id: usize, k: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, k).map(|id| {
                let node = &self.nodes[id];
                let node_value =
                    V::deserialize(&mut node.value.as_slice()).expect("Value corrupted");
                node_value
            })
        } else {
            None
        }
    }

    /// Inserts a new key-value pair and returns the old value if it was present
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn insert(&mut self, tree_id: usize, key: K, value: V) -> Result<Option<V>, Error> {
        if self.roots.len() > tree_id {
            let result = self.put(tree_id, self.root(tree_id), None, key, value);
            match result {
                Ok((id, old_val)) => {
                    self.set_root(tree_id, Some(id));
                    self.nodes[id as usize].set_is_red(false);
                    Ok(old_val)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(Error::TooBigTreeId)
        }
    }

    /// Returns `true` if the tree contains no elements
    #[must_use]
    pub fn is_empty(&self, tree_id: usize) -> bool {
        if self.roots.len() > tree_id {
            self.root(tree_id).is_none()
        } else {
            true
        }
    }

    /// Deletes entry and returns deserialized value
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn remove<Q>(&mut self, tree_id: usize, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, key).map(|id| {
                let deallocated_node_id = { self.delete_node(tree_id, id) };

                let value = V::deserialize(&mut self.nodes[deallocated_node_id].value.as_slice())
                    .expect("Value corrupted");
                value
            })
        } else {
            None
        }
    }

    /// Deletes entry and returns deserialized key-value pair
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    pub fn remove_entry<Q>(&mut self, tree_id: usize, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, key).map(|id| {
                let deallocated_node_id = { self.delete_node(tree_id, id) };

                let key = K::deserialize(&mut self.nodes[deallocated_node_id].key.as_slice())
                    .expect("Key corrupted");
                let value = V::deserialize(&mut self.nodes[deallocated_node_id].value.as_slice())
                    .expect("Value corrupted");
                (key, value)
            })
        } else {
            None
        }
    }

    /// Deletes entry without deserializing the value.
    ///
    /// Return `true` if there was a value with the given `key`.
    pub fn delete<Q>(&mut self, tree_id: usize, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.roots.len() > tree_id {
            self.get_key_index(tree_id, key)
                .map(|id| {
                    self.delete_node(tree_id, id);
                })
                .is_some()
        } else {
            false
        }
    }

    /// Creates an iterator over key-value pairs, in order by key
    pub fn pairs<'b>(
        &'b self,
        tree_id: usize,
    ) -> Result<PairsIterator<'b, 'a, K, V, KSIZE, VSIZE>, Error> {
        if self.roots.len() > tree_id {
            Ok(PairsIterator::from_raw_parts(
                self,
                self.root(tree_id).map(|root_id| self.min(root_id as usize)),
            ))
        } else {
            Err(Error::TooBigTreeId)
        }
    }

    /// Creates an iterator over keys, from smallest to biggest
    pub fn keys<'b>(
        &'b self,
        tree_id: usize,
    ) -> Result<KeysIterator<'b, 'a, K, V, KSIZE, VSIZE>, Error> {
        if self.roots.len() > tree_id {
            Ok(KeysIterator::from_raw_parts(
                self,
                self.root(tree_id).map(|root_id| self.min(root_id as usize)),
            ))
        } else {
            Err(Error::TooBigTreeId)
        }
    }

    /// Creates an iterator over values, in order by key
    pub fn values<'b>(
        &'b self,
        tree_id: usize,
    ) -> Result<ValuesIterator<'b, 'a, K, V, KSIZE, VSIZE>, Error> {
        if self.roots.len() > tree_id {
            Ok(ValuesIterator::from_raw_parts(
                self,
                self.root(tree_id).map(|root_id| self.min(root_id as usize)),
            ))
        } else {
            Err(Error::TooBigTreeId)
        }
    }

    /// Returns the first key-value pair in the map
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn first_entry(&self, tree_id: usize) -> Option<(K, V)> {
        if self.roots.len() > tree_id {
            self.root(tree_id).map(|root_id| {
                let node = &self.nodes[self.min(root_id as usize)];
                let key = K::deserialize(&mut node.key.as_slice()).expect("Key corrupted");
                let value = V::deserialize(&mut node.value.as_slice()).expect("Value corrupted");
                (key, value)
            })
        } else {
            None
        }
    }

    /// Returns the last key-value pair in the map
    ///
    /// This function runs in `O(log(n))`, where `n` - is the number of nodes
    #[must_use]
    pub fn last_entry(&self, tree_id: usize) -> Option<(K, V)> {
        if self.roots.len() > tree_id {
            self.root(tree_id).map(|root_id| {
                let node = &self.nodes[self.max(root_id as usize)];
                let key = K::deserialize(&mut node.key.as_slice()).expect("Key corrupted");
                let value = V::deserialize(&mut node.value.as_slice()).expect("Value corrupted");
                (key, value)
            })
        } else {
            None
        }
    }

    fn root(&self, id: usize) -> Option<u32> {
        let num = u32::from_be_bytes(self.roots[id]);
        if num == u32::MAX {
            None
        } else {
            Some(num)
        }
    }

    pub(super) fn set_root(&mut self, id: usize, root: Option<u32>) {
        match root {
            Some(idx) => {
                assert!(idx < u32::MAX);
                self.roots[id] = u32::to_be_bytes(idx);
            }
            None => {
                self.roots[id] = u32::to_be_bytes(u32::MAX);
            }
        }
    }

    #[must_use]
    fn size(&self, maybe_id: Option<u32>) -> usize {
        if let Some(id) = maybe_id {
            let node = self.nodes[id as usize];
            self.size(node.left()) + self.size(node.right()) + 1
        } else {
            0
        }
    }

    fn put(
        &mut self,
        tree_id: usize,
        maybe_id: Option<u32>,
        parent: Option<u32>,
        key: K,
        value: V,
    ) -> Result<(u32, Option<V>), Error> {
        if let Some(mut id) = maybe_id {
            let old_val;
            let node = &self.nodes[id as usize];
            let node_key = K::deserialize(&mut node.key.as_slice()).expect("Key corrupted");
            match key.cmp(node_key.borrow()) {
                Ordering::Less => {
                    let left_result = self.put(
                        tree_id,
                        self.nodes[id as usize].left(),
                        Some(id),
                        key,
                        value,
                    );
                    match left_result {
                        Ok((child_id, val)) => {
                            old_val = val;
                            {
                                self.nodes[id as usize].set_left(Some(child_id));
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ordering::Greater => {
                    let right_result = self.put(
                        tree_id,
                        self.nodes[id as usize].right(),
                        Some(id),
                        key,
                        value,
                    );
                    match right_result {
                        Ok((child_id, val)) => {
                            old_val = val;
                            {
                                self.nodes[id as usize].set_right(Some(child_id));
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ordering::Equal => {
                    old_val = V::deserialize(&mut self.nodes[id as usize].value.as_slice()).ok();
                    // This is needed to check if the value fits in the slice
                    // Otherwise we can invalidate data in the node
                    let serialization_container = &mut self.buffer;
                    let serialization_result =
                        value.serialize(&mut serialization_container.as_mut_slice());

                    match serialization_result {
                        Ok(()) => self.nodes[id as usize]
                            .value
                            .copy_from_slice(serialization_container.as_slice()),
                        Err(_) => return Err(Error::ValueSerializationError),
                    }
                }
            }
            let right_left_subnode = match self.nodes[id as usize].right() {
                Some(sub_id) => self.nodes[sub_id as usize].left(),
                None => None,
            };

            if self.is_red(self.nodes[id as usize].right()) && self.is_red(right_left_subnode) {
                self.rotate_right(tree_id, self.nodes[id as usize].right().unwrap());
            }

            if self.is_red(self.nodes[id as usize].right())
                && !self.is_red(self.nodes[id as usize].left())
            {
                id = self.rotate_left(tree_id, id);
            }

            let left_subnode = match self.nodes[id as usize].left() {
                Some(sub_id) => self.nodes[sub_id as usize].left(),
                None => None,
            };

            if self.is_red(self.nodes[id as usize].left()) && self.is_red(left_subnode) {
                id = self.rotate_right(tree_id, id);
            }

            if self.is_red(self.nodes[id as usize].right())
                && self.is_red(self.nodes[id as usize].left())
            {
                // If nodes are red, they are not Option::None, so unwrap will never fail
                let left_id = self.nodes[id as usize].left().unwrap() as usize;
                let right_id = self.nodes[id as usize].right().unwrap() as usize;

                // Color swap
                self.nodes[left_id].set_is_red(false);
                self.nodes[right_id].set_is_red(false);
                self.nodes[id as usize].set_is_red(true);
            }

            Ok((id, old_val))
        } else {
            let new_id = match self.allocate_node() {
                Some(id) => id,
                None => return Err(Error::NoNodesLeft),
            };
            let new_node = &mut self.nodes[new_id];

            new_node.init_node(parent);

            // Here it is ok to write directly to slice, because in case of error the node
            // will be deallocated anyway,
            if value.serialize(&mut new_node.value.as_mut_slice()).is_err() {
                // SAFETY: We are deleting previously allocated empty node, so no invariants
                // are changed.
                self.deallocate_node(new_id);
                return Err(Error::ValueSerializationError);
            }

            if key.serialize(&mut new_node.key.as_mut_slice()).is_err() {
                self.deallocate_node(new_id);
                return Err(Error::KeySerializationError);
            }

            Ok((new_id as u32, None))
        }
    }

    #[must_use]
    fn is_red(&self, maybe_id: Option<u32>) -> bool {
        match maybe_id {
            Some(id) => self.nodes[id as usize].is_red(),
            None => false,
        }
    }

    fn get_key_index<Q>(&self, tree_id: usize, k: &Q) -> Option<usize>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let mut maybe_id = self.root(tree_id);
        while let Some(id) = maybe_id {
            let node = &self.nodes[id as usize];
            let node_key = K::deserialize(&mut node.key.as_slice()).expect("Key corrupted");
            match k.cmp(node_key.borrow()) {
                Ordering::Equal => {
                    return Some(id as usize);
                }
                Ordering::Less => maybe_id = node.left(),
                Ordering::Greater => maybe_id = node.right(),
            }
        }
        None
    }

    fn rotate_left(&mut self, tree_id: usize, h: u32) -> u32 {
        let x = self.nodes[h as usize]
            .right()
            .expect("RBTree invariants corrupted: rotate_left on subtree without right child");

        self.nodes[h as usize].set_right(self.nodes[x as usize].left());
        self.nodes[x as usize].set_left(Some(h));
        self.nodes[x as usize].set_is_red(self.nodes[h as usize].is_red());
        self.nodes[h as usize].set_is_red(true);

        // fix parents
        if let Some(parent_id) = self.nodes[h as usize].parent() {
            let parent_node = &mut self.nodes[parent_id as usize];
            if parent_node.left() == Some(h) {
                parent_node.set_left(Some(x));
            } else {
                debug_assert_eq!(parent_node.right(), Some(h));

                parent_node.set_right(Some(x));
            }
        } else {
            self.set_root(tree_id, Some(x));
        }
        self.nodes[x as usize].set_parent(self.nodes[h as usize].parent());
        self.nodes[h as usize].set_parent(Some(x));
        if let Some(right) = self.nodes[h as usize].right() {
            self.nodes[right as usize].set_parent(Some(h));
        }

        x
    }

    fn rotate_right(&mut self, tree_id: usize, h: u32) -> u32 {
        let x = self.nodes[h as usize]
            .left()
            .expect("RBTree invariants corrupted: rotate_left on subtree without left child");

        self.nodes[h as usize].set_left(self.nodes[x as usize].right());
        self.nodes[x as usize].set_right(Some(h));
        self.nodes[x as usize].set_is_red(self.nodes[h as usize].is_red());
        self.nodes[h as usize].set_is_red(true);

        // fix parents
        if let Some(parent_id) = self.nodes[h as usize].parent() {
            let parent_node = &mut self.nodes[parent_id as usize];
            if parent_node.left() == Some(h) {
                parent_node.set_left(Some(x));
            } else {
                debug_assert_eq!(parent_node.right(), Some(h));

                parent_node.set_right(Some(x));
            }
        } else {
            self.set_root(tree_id, Some(x));
        }
        self.nodes[x as usize].set_parent(self.nodes[h as usize].parent());
        self.nodes[h as usize].set_parent(Some(x));
        if let Some(left) = self.nodes[h as usize].left() {
            self.nodes[left as usize].set_parent(Some(h));
        }

        x
    }

    fn delete_node<Q>(&mut self, tree_id: usize, mut id: usize) -> usize
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        if self.nodes[id].left().is_some() && self.nodes[id].right().is_some() {
            {
                id = self.swap_max_left(id);
            }
        }

        match (self.nodes[id].left(), self.nodes[id].right()) {
            (Some(_), Some(_)) => {
                unreachable!("swap_max_left() returned a node with two children");
            }
            (Some(left), None) => {
                let left_id = left as usize;
                // This node has to be black, its child has to be red
                debug_assert!(!self.nodes[id].is_red());
                debug_assert!(self.nodes[left_id].is_red());

                self.swap_nodes(id, left_id);

                self.nodes[id].set_left(None);
                self.deallocate_node(left_id);

                left_id
            }
            (None, Some(right)) => {
                let right_id = right as usize;
                // This node has to be black, its child has to be red
                debug_assert!(!self.nodes[id].is_red());
                debug_assert!(self.nodes[right_id].is_red());

                self.swap_nodes(id, right_id);

                self.nodes[id].set_right(None);

                self.deallocate_node(right_id);

                right_id
            }
            (None, None) => {
                if self.nodes[id].is_red() {
                    // Root node is always black, so if nodes[id] is red, it always has a parent
                    let parent_id = self.nodes[id].parent().unwrap();
                    let parent_node = &mut self.nodes[parent_id as usize];

                    if parent_node.left() == Some(id as u32) {
                        parent_node.set_left(None);
                    } else {
                        debug_assert_eq!(parent_node.right(), Some(id as u32));

                        parent_node.set_right(None);
                    }

                    self.deallocate_node(id);

                    id
                } else {
                    if let Some(parent_id) = self.nodes[id].parent() {
                        let parent_node = &mut self.nodes[parent_id as usize];
                        if parent_node.left() == Some(id as u32) {
                            parent_node.set_left(None);
                        } else {
                            debug_assert_eq!(parent_node.right(), Some(id as u32));

                            parent_node.set_right(None);
                        }

                        self.balance_subtree(tree_id, parent_id as usize);
                    } else {
                        self.set_root(tree_id, None);
                    }

                    self.deallocate_node(id);

                    id
                }
            }
        }
    }

    #[must_use]
    fn swap_max_left(&mut self, id: usize) -> usize {
        let mut max_id = self.nodes[id]
            .left()
            .expect("swap_max_left should only be called on nodes with two children")
            as usize;
        while let Some(maybe_max) = self.nodes[max_id].right() {
            max_id = maybe_max as usize;
        }

        debug_assert_ne!(id, max_id);
        self.swap_nodes(id, max_id);
        max_id
    }

    fn swap_nodes(&mut self, a: usize, b: usize) {
        let tmp_key = self.nodes[a].key;
        self.nodes[a].key = self.nodes[b].key;
        self.nodes[b].key = tmp_key;

        let tmp_value = self.nodes[a].value;
        self.nodes[a].value = self.nodes[b].value;
        self.nodes[b].value = tmp_value;
    }

    fn balance_subtree(&mut self, tree_id: usize, id: usize) {
        let left_child = self.nodes[id].left();
        let right_child = self.nodes[id].right();
        let left_depth = self.black_depth(left_child);
        let right_depth = self.black_depth(right_child);
        match left_depth.cmp(&right_depth) {
            Ordering::Greater => {
                // left_depth is greater than right_depth, so it is >= 1 and therefore left_child
                // always exists
                let left_id = left_child.unwrap() as usize;
                if self.nodes[id].is_red() {
                    debug_assert!(!self.nodes[left_id].is_red());
                    let left_grandchild = self.nodes[left_id].left();
                    let right_grandchild = self.nodes[left_id].right();
                    match (self.is_red(left_grandchild), self.is_red(right_grandchild)) {
                        (false, false) => {
                            self.nodes[id].set_is_red(false);
                            self.nodes[left_id].set_is_red(true);
                        }
                        (true, _) => {
                            self.rotate_right(tree_id, id as u32);

                            self.nodes[id].set_is_red(false);
                            self.nodes[left_id].set_is_red(true);
                            // left_grandchild is red, so it exists
                            self.nodes[left_grandchild.unwrap() as usize].set_is_red(false);
                        }
                        (false, true) => {
                            self.rotate_left(tree_id, left_id as u32);
                            self.rotate_right(tree_id, id as u32);
                            // right_grandchild is red, so it exists
                            self.nodes[right_grandchild.unwrap() as usize].set_is_red(false);
                        }
                    }
                } else if self.nodes[left_id].is_red() {
                    debug_assert!(!self.is_red(self.nodes[left_id].left()));
                    debug_assert!(!self.is_red(self.nodes[left_id].right()));
                    // left_depth is greater than right_depth, so it is >= 1
                    // left_child is red and does not affect black height
                    // therefore left and right grandchildren exists
                    let right_grandchild = self.nodes[left_id].right().unwrap() as usize;
                    let left_grandgrandchild = self.nodes[right_grandchild].left();
                    let right_grandgrandchild = self.nodes[right_grandchild].right();

                    match (
                        self.is_red(left_grandgrandchild),
                        self.is_red(right_grandgrandchild),
                    ) {
                        (false, false) => {
                            self.rotate_right(tree_id, id as u32);
                            self.nodes[id].set_is_red(false);
                            self.nodes[right_grandchild].set_is_red(true);
                        }
                        (true, _) => {
                            self.rotate_left(tree_id, left_id as u32);
                            self.rotate_right(tree_id, id as u32);
                            // left_grandgrandchild is red, so it always exists
                            self.nodes[left_grandgrandchild.unwrap() as usize].set_is_red(false);
                            self.nodes[right_grandchild].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                        (false, true) => {
                            self.rotate_left(tree_id, right_grandchild as u32);
                            self.rotate_left(tree_id, left_id as u32);
                            self.rotate_right(tree_id, id as u32);
                            // left_grandgrandchild is red, so it always exists
                            self.nodes[right_grandgrandchild.unwrap() as usize].set_is_red(false);
                            self.nodes[right_grandchild].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                    }
                } else {
                    let left_grandchild = self.nodes[left_id].left();
                    let right_grandchild = self.nodes[left_id].right();

                    match (self.is_red(left_grandchild), self.is_red(right_grandchild)) {
                        (false, false) => {
                            self.nodes[left_id].set_is_red(true);
                            if let Some(parent_id) = self.nodes[id].parent() {
                                self.balance_subtree(tree_id, parent_id as usize);
                            }
                        }
                        (_, true) => {
                            self.rotate_left(tree_id, left_id as u32);
                            self.rotate_right(tree_id, id as u32);
                            self.nodes[left_id].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                        (true, false) => {
                            self.nodes[left_grandchild.unwrap() as usize].set_is_red(false);
                            self.rotate_right(tree_id, id as u32);
                            self.nodes[id].set_is_red(false);
                        }
                    }
                }
            }
            Ordering::Less => {
                // right_depth is greater than left_depth, so it >= 1 and therefore right_child
                // always exists
                let right_id = right_child.unwrap() as usize;
                if self.nodes[id].is_red() {
                    debug_assert!(!self.nodes[right_id].is_red());
                    let right_grandchild = self.nodes[right_id].right();
                    let left_grandchild = self.nodes[right_id].left();
                    match (self.is_red(right_grandchild), self.is_red(left_grandchild)) {
                        (false, false) => {
                            self.nodes[id].set_is_red(false);
                            self.nodes[right_id].set_is_red(true);
                        }
                        (true, _) => {
                            self.rotate_left(tree_id, id as u32);

                            self.nodes[id].set_is_red(false);
                            self.nodes[right_id].set_is_red(true);
                            // right_grandchild is red, so it always exists
                            self.nodes[right_grandchild.unwrap() as usize].set_is_red(false);
                        }
                        (false, true) => {
                            self.rotate_right(tree_id, right_id as u32);
                            self.rotate_left(tree_id, id as u32);
                            // right_grandchild is red, so it always exists
                            self.nodes[left_grandchild.unwrap() as usize].set_is_red(false);
                        }
                    }
                } else if self.nodes[right_id].is_red() {
                    debug_assert!(!self.is_red(self.nodes[right_id].right()));
                    debug_assert!(!self.is_red(self.nodes[right_id].left()));
                    // right_depth is greater than left_depth, so it is >= 1
                    // right_child is red and does not affect black height
                    // therefore left and right grandchildren exists
                    let left_grandchild = self.nodes[right_id].left().unwrap() as usize;
                    let right_grandgrandchild = self.nodes[left_grandchild].right();
                    let left_grandgrandchild = self.nodes[left_grandchild].left();

                    match (
                        self.is_red(right_grandgrandchild),
                        self.is_red(left_grandgrandchild),
                    ) {
                        (false, false) => {
                            self.rotate_left(tree_id, id as u32);
                            self.nodes[id].set_is_red(false);
                            self.nodes[left_grandchild].set_is_red(true);
                        }
                        (true, _) => {
                            self.rotate_right(tree_id, right_id as u32);
                            self.rotate_left(tree_id, id as u32);
                            // right_grandgrandchild is red, so it always exists
                            self.nodes[right_grandgrandchild.unwrap() as usize].set_is_red(false);
                            self.nodes[left_grandchild].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                        (false, true) => {
                            self.rotate_right(tree_id, left_grandchild as u32);
                            self.rotate_right(tree_id, right_id as u32);
                            self.rotate_left(tree_id, id as u32);
                            // left_grandgrandchild is red, so it always exists
                            self.nodes[left_grandgrandchild.unwrap() as usize].set_is_red(false);
                            self.nodes[left_grandchild].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                    }
                } else {
                    let right_grandchild = self.nodes[right_id].right();
                    let left_grandchild = self.nodes[right_id].left();

                    match (self.is_red(right_grandchild), self.is_red(left_grandchild)) {
                        (false, false) => {
                            self.nodes[right_id].set_is_red(true);
                            if let Some(parent_id) = self.nodes[id].parent() {
                                self.balance_subtree(tree_id, parent_id as usize);
                            }
                        }
                        (_, true) => {
                            self.rotate_right(tree_id, right_id as u32);
                            self.rotate_left(tree_id, id as u32);
                            self.nodes[right_id].set_is_red(false);
                            self.nodes[id].set_is_red(false);
                        }
                        (true, false) => {
                            // right_grandchild is red, so it always exists
                            self.nodes[right_grandchild.unwrap() as usize].set_is_red(false);
                            self.rotate_left(tree_id, id as u32);
                            self.nodes[id].set_is_red(false);
                        }
                    }
                }
            }
            Ordering::Equal => {
                unreachable!("balance_subtree() should only be called on non ballanced trees. It could be a sign, that the tree was not previously balanced.");
            }
        }
    }

    #[must_use]
    fn black_depth(&self, mut maybe_id: Option<u32>) -> usize {
        let mut depth = 0;
        while let Some(id) = maybe_id {
            if !self.nodes[id as usize].is_red() {
                depth += 1;
            }
            maybe_id = self.nodes[id as usize].left();
        }
        depth
    }

    /// Deallocates a node
    ///
    /// # Safety
    ///
    /// This function does nothing but deallocation. It should be checked, that the node is
    /// completely unlinked from the tree.
    fn deallocate_node(&mut self, index: usize) {
        let allocator_head = self.header.head();
        let node_index = Some(index as u32);

        self.nodes[index].set_parent(allocator_head);
        self.header.set_head(node_index);
    }

    /// Allocates a node
    ///
    /// # Safety
    ///
    /// This function does nothing but allocation. The returned node (if present) is
    /// completely unlinked from the tree and is in the unknown state. The caller must fill the
    /// node with correct data.
    #[must_use]
    fn allocate_node(&mut self) -> Option<usize> {
        let allocator_head = self.header.head();
        match allocator_head {
            Some(index) => {
                let new_head = self.nodes[index as usize].parent();
                self.header.set_head(new_head);
                Some(index as usize)
            }
            None => None,
        }
    }

    fn min(&self, mut min_id: usize) -> usize {
        while let Some(id) = self.nodes[min_id].left() {
            min_id = id as usize;
        }
        min_id
    }

    fn max(&self, mut max_id: usize) -> usize {
        while let Some(id) = self.nodes[max_id].right() {
            max_id = id as usize;
        }
        max_id
    }
}

impl<'a, K, V, const KSIZE: usize, const VSIZE: usize> fmt::Debug
    for RBForest<'a, K, V, KSIZE, VSIZE>
where
    K: Ord + BorshDeserialize + BorshSerialize + fmt::Debug,
    V: BorshDeserialize + BorshSerialize + fmt::Debug,
{
    #[cfg(not(test))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let max_roots = self.max_roots();
        f.debug_map()
            .entries((0..max_roots).map(|i| (i, self.pairs(i))))
            .finish()
    }

    #[cfg(test)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("RBForest")
            .field("header", self.header)
            .field(
                "roots",
                &self
                    .roots
                    .iter()
                    .map(|x| {
                        let num = u32::from_be_bytes(*x);
                        if num == u32::MAX {
                            None
                        } else {
                            Some(num)
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .field("nodes", &self.nodes)
            .finish()
    }
}

#[cfg(test)]
pub(super) mod tests;

#[cfg(any(test, fuzzing))]
pub mod internal_checks;
Footer
© 2023 GitHub, Inc.
Footer navigation
Terms
Privacy
Security
Status
Docs
Contact GitHub
Pricing
API
Training
Blog
About
slice-rbtree/forest.rs at main · solcery/slice-rbtree