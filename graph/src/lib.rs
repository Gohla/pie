#![forbid(unsafe_code, missing_docs)]

//! The purpose of this crate is to maintain an topological order in the face
//! of single updates, like adding new nodes, adding new edges, deleting
//! edges, and deleting nodes.
//!
//! Adding nodes, deleting nodes, and deleting dependencies require a trivial
//! amount of work to perform an update, because those operations do not change
//! the topological ordering. Adding new dependencies can change the topological
//! ordering.
//!
//! ## What is a Topological Order
//!
//! To define a topological order requires at least a simple definition of a
//! graph, and specifically a directed acyclic graph (DAG). A graph can be
//! described as a pair of sets, `(V, E)` where `V` is the set of all nodes in
//! the graph, and `E` is the set of edges. An edge is defined as a pair, `(m,
//! n)` where `m` and `n` are nodes. A directed graph means that edges only
//! imply a single direction of relationship between two nodes, as opposed to a
//! undirected graph which implies the relationship goes both ways. An example
//! of undirected vs. directed in social networks would be Facebook vs.
//! Twitter. Facebook friendship is a two way relationship, while following
//! someone on Twitter does not imply that they follow you back.
//!
//! A topological ordering, `ord_D` of a directed acyclic graph, `D = (V, E)`
//! where `x, y ∈ V`, is a mapping of nodes to priority values such that
//! `ord_D(x) < ord_D(y)` holds for all edges `(x, y) ∈ E`. This yields a total
//! ordering of the nodes in `D`.
//!
//! ## Examples
//!
//! ```
//! use pie_graph::DAG;
//! use std::{cmp::Ordering::*, collections::HashSet};
//!
//! let mut dag = DAG::new();
//!
//! let dog = dag.add_node(());
//! let cat = dag.add_node(());
//! let mouse = dag.add_node(());
//! let lion = dag.add_node(());
//! let human = dag.add_node(());
//! let gazelle = dag.add_node(());
//! let grass = dag.add_node(());
//!
//! assert_eq!(dag.len(), 7);
//!
//! dag.add_edge(&lion, &human, ()).unwrap();
//! dag.add_edge(&lion, &gazelle, ()).unwrap();
//!
//! dag.add_edge(&human, &dog, ()).unwrap();
//! dag.add_edge(&human, &cat, ()).unwrap();
//!
//! dag.add_edge(&dog, &cat, ()).unwrap();
//! dag.add_edge(&cat, &mouse, ()).unwrap();
//!
//! dag.add_edge(&gazelle, &grass, ()).unwrap();
//!
//! dag.add_edge(&mouse, &grass, ()).unwrap();
//!
//! let pairs = dag
//!     .descendants_unsorted(&human)
//!     .unwrap()
//!     .collect::<HashSet<_>>();
//! let expected_pairs = [(4, cat), (3, dog), (5, mouse), (7, grass)]
//!     .iter()
//!     .cloned()
//!     .collect::<HashSet<_>>();
//!
//! assert_eq!(pairs, expected_pairs);
//!
//! assert!(dag.contains_transitive_edge(&lion, &grass));
//! assert!(!dag.contains_transitive_edge(&human, &gazelle));
//!
//! assert_eq!(dag.topo_cmp(&cat, &dog), Greater);
//! assert_eq!(dag.topo_cmp(&lion, &human), Less);
//! ```
//!
//! ## Sources
//!
//! The [paper by D. J. Pearce and P. H. J. Kelly] contains descriptions of
//! three different algorithms for incremental topological ordering, along with
//! analysis of runtime bounds for each.
//!
//! [paper by D. J. Pearce and P. H. J. Kelly]: http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf

use std::{
  borrow::Borrow,
  cmp::{Ordering, Reverse},
  collections::BinaryHeap,
  fmt,
  iter::Iterator,
};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use hashlink::LinkedHashSet;
use slotmap::{DefaultKey, SlotMap};

type TopoOrder = u32;


/// Data structure for maintaining a directed-acyclic graph (DAG) with topological ordering, maintained in an 
/// incremental fashion.
///
/// When iterating over edges or their associated data, the order is the insertion order.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: index.html
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "N: serde::Serialize, E: serde::Serialize, H: BuildHasher + Default")))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "N: serde::Deserialize<'de>, E: serde::Deserialize<'de>, H: BuildHasher + Default")))]
pub struct DAG<N, E, H = RandomState> {
  node_info: SlotMap<DefaultKey, NodeInfo<N, H>>,
  edge_data: HashMap<(Node, Node), E, H>,
  last_topo_order: TopoOrder,

  #[cfg_attr(feature = "serde", serde(skip))]
  stack_visited_scratch_space: Cell<StackVisitedScratchSpace<Node, H>>,
}


/// A node (identifier) in the [`DAG`].
///
/// This identifier contains metadata so that a node which has been passed to [`DAG::delete_node`] 
/// will not be confused with a node created later.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Node(DefaultKey);


/// Information about a node: its ordering, which nodes it points to, and which nodes point to it.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "N: serde::Serialize, H: BuildHasher + Default")))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "N: serde::Deserialize<'de>, H: BuildHasher + Default")))]
struct NodeInfo<N, H> {
  topo_order: TopoOrder,
  data: N,
  parents: LinkedHashSet<Node, H>,
  children: LinkedHashSet<Node, H>,
}

impl<N, H: BuildHasher + Default> NodeInfo<N, H> {
  /// Create a new node entry with the specified topological order.
  fn new(topo_order: TopoOrder, data: N) -> Self {
    NodeInfo {
      topo_order,
      data,
      parents: Default::default(),
      children: Default::default(),
    }
  }
}


/// Different types of failures that can occur while updating or querying the graph.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
  /// The given node was not found in the topological order. This usually means that the node was deleted, but a 
  /// reference was kept around after which is now invalid.
  NodeMissing,
  /// Cycles of nodes may not be formed in the graph.
  CycleDetected,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::NodeMissing => {
        write!(f, "The given node was not found in the topological order")
      }
      Error::CycleDetected => write!(f, "Cycles of nodes may not be formed in the graph"),
    }
  }
}

impl std::error::Error for Error {}


impl<N, E, H: BuildHasher + Default> Default for DAG<N, E, H> {
  #[inline]
  fn default() -> Self {
    Self {
      last_topo_order: 0,
      node_info: SlotMap::default(),
      edge_data: Default::default(),
      stack_visited_scratch_space: Cell::default(),
    }
  }
}


impl<N, E> DAG<N, E> {
  /// Create a new DAG.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let dag = DAG::<(), ()>::new();
  ///
  /// assert!(dag.is_empty());
  /// ```
  #[inline]
  pub fn new() -> Self { Self::default() }
}

impl<N, E, H: BuildHasher + Default> DAG<N, E, H> {
  /// Add a new node with `data` to the graph and return a unique [`Node`] which identifies it.
  ///
  /// Initially this node will not have any order relative to the nodes that are already in the graph. Only when 
  /// relations are added with [`add_dependency`] will the order begin to matter.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::<(), ()>::new();
  ///
  /// let cat = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert_ne!(cat, dog);
  /// assert_ne!(cat, mouse);
  /// assert_ne!(cat, human);
  /// assert_ne!(dog, mouse);
  /// assert_ne!(dog, human);
  /// assert_ne!(mouse, human);
  ///
  /// assert!(dag.contains_node(&cat));
  /// assert!(dag.contains_node(&dog));
  /// assert!(dag.contains_node(&mouse));
  /// assert!(dag.contains_node(&human));
  /// ```
  ///
  /// [`add_dependency`]: struct.IncrementalTopo.html#method.add_dependency
  #[inline]
  pub fn add_node(&mut self, data: N) -> Node {
    let next_topo_order = self.last_topo_order + 1;
    self.last_topo_order = next_topo_order;
    let node_info = NodeInfo::new(next_topo_order, data);
    Node(self.node_info.insert(node_info))
  }

  /// Returns true if the graph contains the specified `node.`
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::<(), ()>::new();
  ///
  /// let cat = dag.add_node(());
  /// let dog = dag.add_node(());
  ///
  /// assert!(dag.contains_node(cat));
  /// assert!(dag.contains_node(dog));
  /// ```
  #[inline]
  pub fn contains_node(&self, node: impl Borrow<Node>) -> bool {
    let node = node.borrow();
    self.node_info.contains_key(node.0)
  }

  /// Gets data for given `node`.
  #[inline]
  pub fn get_node_data(&self, node: impl Borrow<Node>) -> Option<&N> {
    let node = node.borrow();
    self.node_info.get(node.0).map(|d| &d.data)
  }

  /// Gets mutable data for given `node`.
  #[inline]
  pub fn get_node_data_mut(&mut self, node: impl Borrow<Node>) -> Option<&mut N> {
    let node = node.borrow();
    self.node_info.get_mut(node.0).map(|d| &mut d.data)
  }

  /// Attempt to remove `node` from graph, returning true if the node was contained and removed.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::<(), ()>::new();
  ///
  /// let cat = dag.add_node(());
  /// let dog = dag.add_node(());
  ///
  /// assert!(dag.remove_node(cat));
  /// assert!(dag.remove_node(dog));
  ///
  /// assert!(!dag.remove_node(cat));
  /// assert!(!dag.remove_node(dog));
  /// ```
  pub fn remove_node(&mut self, node: Node) -> bool {
    if !self.node_info.contains_key(node.0) {
      return false;
    }

    // Remove node
    let node_info = self.node_info.remove(node.0).unwrap();
    // Remove forward edges
    for child in &node_info.children {
      if let Some(child_node) = self.node_info.get_mut(child.0) {
        child_node.parents.remove(&node.into());
      }
      self.edge_data.remove(&(node, *child));
    }
    // Remove backward edges
    for parent in &node_info.parents {
      if let Some(parent_node) = self.node_info.get_mut(parent.0) {
        parent_node.children.remove(&node.into());
      }
      self.edge_data.remove(&(*parent, node));
    }
    // OPTO: inefficient compaction step
    for other_node in self.node_info.values_mut() {
      if other_node.topo_order > node_info.topo_order {
        other_node.topo_order -= 1;
      }
    }
    // Decrement last topo order to account for shifted topo values
    self.last_topo_order -= 1;

    true
  }

  /// Add a directed edge from `src` to `dst` with edge `data`.
  ///
  /// This edge indicates an ordering constraint on the two nodes, now `src` must always come before `dst` in the 
  /// ordering.
  ///
  /// Returns `Ok(true)` if the graph did not previously contain this edge. Returns `Ok(false)` if the graph did 
  /// have a previous edge between these two nodes.
  ///
  /// # Errors
  ///
  /// Returns an `Err` if the edge introduces a cycle or if either of the given nodes is not found in the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, dog, ()).unwrap());
  /// assert!(dag.add_edge(&human, cat, ()).unwrap());
  /// assert!(dag.add_edge(&cat, mouse, ()).unwrap());
  /// ```
  ///
  /// Here is an example which returns [`Error::CycleDetected`] when introducing a cycle:
  ///
  /// ```
  /// # use pie_graph::{DAG, Error};
  /// let mut dag = DAG::new();
  ///
  /// let n0 = dag.add_node(());
  /// assert_eq!(dag.add_edge(&n0, &n0, ()), Err(Error::CycleDetected));
  ///
  /// let n1 = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&n0, &n1, ()).unwrap());
  /// assert_eq!(dag.add_edge(&n1, &n0, ()), Err(Error::CycleDetected));
  ///
  /// let n2 = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&n1, &n2, ()).unwrap());
  /// assert_eq!(dag.add_edge(&n2, &n0, ()), Err(Error::CycleDetected));
  /// ```
  pub fn add_edge(
    &mut self,
    src: impl Borrow<Node>,
    dst: impl Borrow<Node>,
    data: E,
  ) -> Result<bool, Error> {
    let src = src.borrow();
    let dst = dst.borrow();

    if src == dst { // No loops to self
      return Err(Error::CycleDetected);
    }
    if !self.node_info.contains_key(src.0) || !self.node_info.contains_key(dst.0) {
      return Err(Error::NodeMissing);
    }

    // Insert forward edge
    let mut no_prev_edge = self.node_info[src.0].children.insert(*dst);
    let upper_bound = self.node_info[src.0].topo_order;
    // Insert backward edge
    no_prev_edge = no_prev_edge && self.node_info[dst.0].parents.insert(*src);
    let lower_bound = self.node_info[dst.0].topo_order;
    if !no_prev_edge { // If edge already exists short circuit
      return Ok(false);
    }
    self.edge_data.insert((*src, *dst), data);

    // If the affected region of the graph has non-zero size (i.e. the upper and
    // lower bound are equal) then perform an update to the topological ordering of
    // the graph
    if lower_bound < upper_bound {
      let mut visited = HashSet::<_, H>::default(); // OPTO: reuse allocation.
      // Walk changes forward from the succ, checking for any cycles that would be introduced
      let change_forward = match self.dfs_forward(*dst, &mut visited, upper_bound) {
        Ok(change_set) => change_set,
        Err(err) => { // Need to remove parent + child + edge info that was previously added
          self.node_info[src.0].children.remove(dst);
          self.node_info[dst.0].parents.remove(src);
          self.edge_data.remove(&(*src, *dst));
          return Err(err);
        }
      };
      // Walk backwards from the pred
      let change_backward = self.dfs_backward(*src, &mut visited, lower_bound);
      self.reorder_nodes(change_forward, change_backward);
    }

    Ok(true)
  }

  /// Returns true if the graph contains an edge from `src` to `dst`.
  ///
  /// Returns false if either node is not found, or if there is no dependency.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let human = dag.add_node(());
  /// let horse = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// assert!(dag.contains_edge(&cat, &mouse));
  /// assert!(!dag.contains_edge(&human, &mouse));
  /// assert!(!dag.contains_edge(&cat, &horse));
  /// ```
  #[inline]
  pub fn contains_edge(&self, src: impl Borrow<Node>, dst: impl Borrow<Node>) -> bool {
    let src = src.borrow();
    let dst = dst.borrow();

    if !self.node_info.contains_key(src.0) || !self.node_info.contains_key(dst.0) {
      return false;
    }
    self.edge_data.contains_key(&(*src, *dst))
  }

  /// Returns true if the graph contains a transitive edge from `src` to `dst`.
  ///
  /// In this context a transitive dependency means that `succ` exists as a descendant of `pred`, with some chain of 
  /// other nodes in between.
  ///
  /// Returns false if either node is not found in the graph, or there is no transitive dependency.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// assert!(dag.contains_transitive_edge(&human, &mouse));
  /// assert!(!dag.contains_transitive_edge(&dog, &mouse));
  /// ```
  pub fn contains_transitive_edge(
    &self,
    src: impl Borrow<Node>,
    dst: impl Borrow<Node>,
  ) -> bool {
    let src = src.borrow();
    let dst = dst.borrow();

    // If either node is missing, return quick
    if !self.node_info.contains_key(src.0) || !self.node_info.contains_key(dst.0) {
      return false;
    }

    // A node cannot depend on itself
    if src.0 == dst.0 {
      return false;
    }

    // Else we have to search the graph. Using dfs in this case because it avoids
    // the overhead of the binary heap, and this task doesn't really need ordered
    // descendants.
    let mut scratch = self.stack_visited_scratch_space.take();
    scratch.clear();
    scratch.stack.push(*src);

    // For each node key popped off the stack, check that we haven't seen it
    // before, then check if its children contain the node we're searching for.
    // If they don't, continue the search by extending the stack with the children.
    while let Some(key) = scratch.stack.pop() {
      if scratch.visited.contains(&key) {
        continue;
      } else {
        scratch.visited.insert(key);
      }

      let children = &self.node_info.get(key.0).unwrap().children;
      if children.contains(dst) {
        self.stack_visited_scratch_space.set(scratch);
        return true;
      } else {
        scratch.stack.extend(children.iter());
        continue;
      }
    }

    self.stack_visited_scratch_space.set(scratch);

    // If we exhaust the stack, then there is no transitive dependency.
    false
  }


  /// Gets data for the edge from `src` to `dst`.
  #[inline]
  pub fn get_edge_data(&self, src: impl Borrow<Node>, dst: impl Borrow<Node>) -> Option<&E> {
    let src = src.borrow();
    let dst = dst.borrow();
    self.edge_data.get(&(*src, *dst))
  }

  /// Gets mutable data for the edge from `src` to `dst`.
  #[inline]
  pub fn get_edge_data_mut(&mut self, src: impl Borrow<Node>, dst: impl Borrow<Node>) -> Option<&mut E> {
    let src = src.borrow();
    let dst = dst.borrow();
    self.edge_data.get_mut(&(*src, *dst))
  }


  /// Gets the destination nodes and edge data for all outgoing edges of `src`.
  #[inline]
  pub fn get_outgoing_edges(&self, src: impl Borrow<Node>) -> impl Iterator<Item=(&Node, &E)> + '_ {
    let src = *src.borrow();
    self.node_info.get(src.0)
      .into_iter()
      .flat_map(|node_info| node_info.children.iter())
      .map(move |child_node| (child_node, self.edge_data.get(&(src, *child_node)).unwrap()))
  }

  /// Gets the destination nodes of all outgoing edges of `src`.
  #[inline]
  pub fn get_outgoing_edge_nodes(&self, src: impl Borrow<Node>) -> impl Iterator<Item=&Node> {
    let src = src.borrow();
    self.node_info.get(src.0)
      .into_iter()
      .flat_map(|node_info| node_info.children.iter())
  }

  /// Gets the edge data for all outgoing edges of `src`.
  #[inline]
  pub fn get_outgoing_edge_data(&self, src: impl Borrow<Node>) -> impl Iterator<Item=&E> + '_ {
    let src = *src.borrow();
    self.node_info.get(src.0)
      .into_iter()
      .flat_map(|node_info| node_info.children.iter())
      .map(move |child_node| self.edge_data.get(&(src, *child_node)).unwrap())
  }

  /// Gets the destination node data for all outgoing edges of `src`.
  #[inline]
  pub fn get_outgoing_edge_node_data(&self, src: impl Borrow<Node>) -> impl Iterator<Item=&N> + '_ {
    let src = src.borrow();
    self.node_info.get(src.0)
      .into_iter()
      .flat_map(|node_info| node_info.children.iter())
      .flat_map(|child_node| self.node_info.get(child_node.0).into_iter())
      .map(|node_info| &node_info.data)
  }


  /// Gets the source node and edge data for all incoming edges of `dst`.
  #[inline]
  pub fn get_incoming_edges(&self, dst: impl Borrow<Node>) -> impl Iterator<Item=(&Node, &E)> + '_ {
    let dst = *dst.borrow();
    self.node_info.get(dst.0)
      .into_iter()
      .flat_map(|node_info| node_info.parents.iter())
      .map(move |parent_node| (parent_node, self.edge_data.get(&(*parent_node, dst)).unwrap()))
  }

  /// Gets the source nodes for all incoming edges of `dst`.
  #[inline]
  pub fn get_incoming_edge_nodes(&self, dst: impl Borrow<Node>) -> impl Iterator<Item=&Node> + '_ {
    let dst = *dst.borrow();
    self.node_info.get(dst.0)
      .into_iter()
      .flat_map(|node_info| node_info.parents.iter())
  }

  /// Gets the edge data for all incoming edges of `dst`.
  #[inline]
  pub fn get_incoming_edge_data(&self, dst: impl Borrow<Node>) -> impl Iterator<Item=&E> + '_ {
    let dst = *dst.borrow();
    self.node_info.get(dst.0)
      .into_iter()
      .flat_map(|node_info| node_info.parents.iter())
      .map(move |parent_node| self.edge_data.get(&(*parent_node, dst)).unwrap())
  }

  /// Gets the source node data for all incoming edges of `dst`.
  #[inline]
  pub fn get_incoming_edge_node_data(&self, dst: impl Borrow<Node>) -> impl Iterator<Item=&N> + '_ {
    let dst = dst.borrow();
    self.node_info.get(dst.0)
      .into_iter()
      .flat_map(|node_info| node_info.parents.iter())
      .flat_map(|parent_node| self.node_info.get(parent_node.0).into_iter())
      .map(|node_info| &node_info.data)
  }


  /// Attempt to remove the edge from `src` to `dst` from the graph, returning `Some(edge_data)` if the edge was 
  /// removed, or `None` otherwise.
  ///
  /// Returns `false` is either node is not found in the graph.
  ///
  /// Removing an edge from the graph is an extremely simple operation, which requires no recalculation of the
  /// topological order. The ordering before and after a removal is exactly the same.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// assert!(dag.remove_edge(&cat, mouse).is_some());
  /// assert!(dag.remove_edge(&human, dog).is_some());
  /// assert!(dag.remove_edge(&human, mouse).is_none());
  /// ```
  pub fn remove_edge(&mut self, src: impl Borrow<Node>, dst: impl Borrow<Node>) -> Option<E> {
    let src = src.borrow();
    let dst = dst.borrow();

    if !self.node_info.contains_key(src.0) || !self.node_info.contains_key(dst.0) {
      return None;
    }
    let src_children = &mut self.node_info[src.0].children;
    if !src_children.contains(&dst) {
      return None;
    }
    src_children.remove(&dst);
    self.node_info[dst.0].parents.remove(&src);
    self.edge_data.remove(&(*src, *dst))
  }

  /// Attempt to remove all outgoing edges of `src` from the graph, returning `Some(edge_data)` if any edges were 
  /// removed, or `None` if the node does not exist or does not have any edges.
  pub fn remove_edges_of_node(&mut self, src: impl Borrow<Node>) -> Option<Vec<(Node, E)>> { // TODO: test!
    let pred_id = src.borrow();
    if !self.node_info.contains_key(pred_id.0) {
      return None;
    }

    let children: Vec<_> = self.node_info[pred_id.0].children.drain().collect(); // OPTO: reuse allocation
    if children.is_empty() {
      return None;
    }

    let mut edge_data = Vec::new();
    for succ_id in children {
      if let Some(succ) = self.node_info.get_mut(succ_id.0) {
        succ.parents.remove(&pred_id);
      }
      if let Some(data) = self.edge_data.remove(&(*pred_id, succ_id)) {
        edge_data.push((succ_id, data));
      }
    }
    Some(edge_data)
  }

  /// Return the number of nodes within the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::<(), ()>::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert_eq!(dag.len(), 4);
  /// ```
  #[inline]
  pub fn len(&self) -> usize {
    self.node_info.len()
  }

  /// Return `true` if there are no nodes in the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::<(), ()>::new();
  ///
  /// assert!(dag.is_empty());
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(!dag.is_empty());
  /// ```
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Return an iterator over all the nodes of the graph in an unsorted order.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// use std::collections::HashSet;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// let pairs = dag.iter_unsorted().collect::<HashSet<_>>();
  ///
  /// let mut expected_pairs = HashSet::new();
  /// expected_pairs.extend(vec![(1, human), (2, cat), (4, mouse), (3, dog)]);
  ///
  /// assert_eq!(pairs, expected_pairs);
  /// ```
  #[inline]
  pub fn iter_unsorted(&self) -> impl Iterator<Item=(TopoOrder, Node)> + '_ {
    self.node_info
      .iter()
      .map(|(key, node)| (node.topo_order, Node(key)))
  }

  /// Return an iterator over the descendants of a node in the graph, in an unsorted order.
  ///
  /// Accessing the nodes in an unsorted order allows for faster access using a iterative DFS search. This is opposed to
  /// the order descendants iterator which requires the use of a binary heap to order the values.
  ///
  /// # Errors
  ///
  /// This function will return an error if the given node is not present in the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// use std::collections::HashSet;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&dog, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// let pairs = dag
  ///     .descendants_unsorted(human)
  ///     .unwrap()
  ///     .collect::<HashSet<_>>();
  ///
  /// let mut expected_pairs = HashSet::new();
  /// expected_pairs.extend(vec![(2, dog), (3, cat), (4, mouse)]);
  ///
  /// assert_eq!(pairs, expected_pairs);
  /// ```
  pub fn descendants_unsorted(
    &self,
    node: impl Borrow<Node>,
  ) -> Result<DescendantsUnsorted<N, E, H>, Error> {
    let node = node.borrow();
    if !self.node_info.contains_key(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut stack = Vec::new(); // OPTO: reuse allocation
    // Add all children of selected node
    stack.extend(self.node_info[node.0].children.iter());
    let visited = HashSet::<_, H>::default(); // OPTO: reuse allocation

    Ok(DescendantsUnsorted {
      dag: self,
      stack,
      visited,
    })
  }

  /// Return an iterator over descendants of a node in the graph, in a topologically sorted order.
  ///
  /// Accessing the nodes in a sorted order requires the use of a BinaryHeap, so some performance penalty is paid there.
  /// If all is required is access to the descendants of a node, use [`DAG::descendants_unsorted`].
  ///
  /// # Errors
  ///
  /// This function will return an error if the given node is not present in the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&dog, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// let ordered_nodes = dag.descendants(human).unwrap().collect::<Vec<_>>();
  ///
  /// assert_eq!(ordered_nodes, vec![dog, cat, mouse]);
  /// ```
  pub fn descendants(&self, node: impl Borrow<Node>) -> Result<Descendants<N, E, H>, Error> {
    let node = node.borrow();
    if !self.node_info.contains_key(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut queue = BinaryHeap::new(); // OPTO: reuse allocation
    // Add all children of selected node
    queue.extend(
      self.node_info[node.0]
        .children
        .iter()
        .cloned()
        .map(|child_node| {
          let child_order = self.get_node(child_node).topo_order;
          (Reverse(child_order), child_node)
        }),
    );
    let visited = HashSet::<_, H>::default(); // OPTO: reuse allocation

    Ok(Descendants {
      dag: self,
      queue,
      visited,
    })
  }

  /// Compare two nodes present in the graph, topographically.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// use std::cmp::Ordering::*;
  ///
  /// let mut dag = DAG::new();
  ///
  /// let cat = dag.add_node(());
  /// let mouse = dag.add_node(());
  /// let dog = dag.add_node(());
  /// let human = dag.add_node(());
  /// let horse = dag.add_node(());
  ///
  /// assert!(dag.add_edge(&human, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&human, &dog, ()).unwrap());
  /// assert!(dag.add_edge(&dog, &cat, ()).unwrap());
  /// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
  ///
  /// assert_eq!(dag.topo_cmp(&human, &mouse), Less);
  /// assert_eq!(dag.topo_cmp(&cat, &dog), Greater);
  /// assert_eq!(dag.topo_cmp(&cat, &horse), Less);
  /// ```
  #[inline]
  pub fn topo_cmp(&self, node_a: impl Borrow<Node>, node_b: impl Borrow<Node>) -> Ordering {
    let node_a = node_a.borrow();
    let node_b = node_b.borrow();
    self.node_info[node_a.0]
      .topo_order
      .cmp(&self.node_info[node_b.0].topo_order)
  }


  fn dfs_forward(
    &self,
    start_key: Node,
    visited: &mut HashSet<Node, H>,
    upper_bound: TopoOrder,
  ) -> Result<HashSet<Node, H>, Error> {
    let mut stack = Vec::new(); // OPTO: reuse allocation
    let mut result = HashSet::<_, H>::default(); // OPTO: reuse allocation

    stack.push(start_key);

    while let Some(next_key) = stack.pop() {
      visited.insert(next_key);
      result.insert(next_key);

      for child_key in self.get_node(next_key).children.iter() {
        let child_topo_order = self.get_node(*child_key).topo_order;

        if child_topo_order == upper_bound {
          return Err(Error::CycleDetected);
        }

        if !visited.contains(child_key) && child_topo_order < upper_bound {
          stack.push(*child_key);
        }
      }
    }

    Ok(result)
  }

  fn dfs_backward(
    &self,
    start_key: Node,
    visited: &mut HashSet<Node, H>,
    lower_bound: TopoOrder,
  ) -> HashSet<Node, H> {
    let mut stack = Vec::new(); // OPTO: reuse allocation
    let mut result = HashSet::<_, H>::default(); // OPTO: reuse allocation

    stack.push(start_key);

    while let Some(next_key) = stack.pop() {
      visited.insert(next_key);
      result.insert(next_key);

      for parent_key in self.get_node(next_key).parents.iter() {
        let parent_topo_order = self.get_node(*parent_key).topo_order;

        if !visited.contains(parent_key) && lower_bound < parent_topo_order {
          stack.push(*parent_key);
        }
      }
    }

    result
  }

  fn reorder_nodes(
    &mut self,
    change_forward: HashSet<Node, H>,
    change_backward: HashSet<Node, H>,
  ) {
    let mut change_forward: Vec<_> = change_forward
      .into_iter()
      .map(|key| (key, self.get_node(key).topo_order))
      .collect(); // OPTO: reuse allocation
    change_forward.sort_unstable_by_key(|pair| pair.1);

    let mut change_backward: Vec<_> = change_backward
      .into_iter()
      .map(|key| (key, self.get_node(key).topo_order))
      .collect(); // OPTO: reuse allocation
    change_backward.sort_unstable_by_key(|pair| pair.1);

    let mut all_keys = Vec::new(); // OPTO: reuse allocation
    let mut all_topo_orders = Vec::new(); // OPTO: reuse allocation

    for (key, topo_order) in change_backward {
      all_keys.push(key);
      all_topo_orders.push(topo_order);
    }

    for (key, topo_order) in change_forward {
      all_keys.push(key);
      all_topo_orders.push(topo_order);
    }

    all_topo_orders.sort_unstable();

    for (key, topo_order) in all_keys.into_iter().zip(all_topo_orders.into_iter()) {
      self.node_info
        .get_mut(key.0)
        .unwrap()
        .topo_order = topo_order;
    }
  }

  fn get_node(&self, idx: Node) -> &NodeInfo<N, H> {
    self.node_info.get(idx.0).unwrap()
  }
}

/// An iterator over the descendants of a node in the graph, which outputs the nodes in an unsorted order with their 
/// topological ranking.
///
/// # Examples
/// ```
/// use pie_graph::DAG;
/// use std::collections::HashSet;
/// let mut dag = DAG::new();
///
/// let cat = dag.add_node(());
/// let mouse = dag.add_node(());
/// let dog = dag.add_node(());
/// let human = dag.add_node(());
///
/// assert!(dag.add_edge(&human, &cat, ()).unwrap());
/// assert!(dag.add_edge(&human, &dog, ()).unwrap());
/// assert!(dag.add_edge(&dog, &cat, ()).unwrap());
/// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
///
/// let pairs = dag
///     .descendants_unsorted(human)
///     .unwrap()
///     .collect::<HashSet<_>>();
///
/// let mut expected_pairs = HashSet::new();
/// expected_pairs.extend(vec![(2, dog), (3, cat), (4, mouse)]);
///
/// assert_eq!(pairs, expected_pairs);
/// ```
pub struct DescendantsUnsorted<'a, N, E, H> {
  dag: &'a DAG<N, E, H>,
  stack: Vec<Node>,
  visited: HashSet<Node, H>,
}

impl<'a, N, E, H: BuildHasher> Iterator for DescendantsUnsorted<'a, N, E, H> {
  type Item = (TopoOrder, Node);
  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    while let Some(node) = self.stack.pop() {
      if self.visited.contains(&node) {
        continue;
      } else {
        self.visited.insert(node);
      }
      let node_repr = self.dag.node_info.get(node.0).unwrap();
      let order = node_repr.topo_order;
      self.stack.extend(node_repr.children.iter());
      return Some((order, node));
    }

    None
  }
}

/// An iterator over the descendants of a node in the graph, which outputs the nodes in a sorted order by their 
/// topological ranking.
///
/// # Examples
/// ```
/// use pie_graph::DAG;
/// let mut dag = DAG::new();
///
/// let cat = dag.add_node(());
/// let mouse = dag.add_node(());
/// let dog = dag.add_node(());
/// let human = dag.add_node(());
///
/// assert!(dag.add_edge(&human, &cat, ()).unwrap());
/// assert!(dag.add_edge(&human, &dog, ()).unwrap());
/// assert!(dag.add_edge(&dog, &cat, ()).unwrap());
/// assert!(dag.add_edge(&cat, &mouse, ()).unwrap());
///
/// let ordered_nodes = dag.descendants(human).unwrap().collect::<Vec<_>>();
///
/// assert_eq!(ordered_nodes, vec![dog, cat, mouse]);
/// ```
pub struct Descendants<'a, N, E, H> {
  dag: &'a DAG<N, E, H>,
  queue: BinaryHeap<(Reverse<TopoOrder>, Node)>,
  visited: HashSet<Node, H>,
}

impl<'a, N, E, H: BuildHasher + Default> Iterator for Descendants<'a, N, E, H> {
  type Item = Node;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return if let Some((_, node)) = self.queue.pop() {
        if self.visited.contains(&node) {
          continue;
        } else {
          self.visited.insert(node);
        }

        let node_repr = self.dag.node_info.get(node.0).unwrap();
        for child in node_repr.children.iter() {
          let order = self.dag.get_node(*child).topo_order;
          self.queue.push((Reverse(order), *child))
        }

        Some(node)
      } else {
        None
      };
    }
  }
}


#[derive(Debug)]
struct StackVisitedScratchSpace<T, H> {
  stack: Vec<T>,
  visited: HashSet<T, H>,
}

impl<T, H: Default> Default for StackVisitedScratchSpace<T, H> {
  fn default() -> Self {
    Self { stack: Vec::new(), visited: HashSet::<_, H>::default() }
  }
}

impl<T, H> StackVisitedScratchSpace<T, H> {
  fn clear(&mut self) {
    self.stack.clear();
    self.visited.clear();
  }
}


#[cfg(test)]
mod tests {
  extern crate pretty_env_logger;

  use super::*;

  fn get_basic_dag() -> Result<([Node; 7], DAG<(), ()>), Error> {
    let mut dag = DAG::new();

    let dog = dag.add_node(());
    let cat = dag.add_node(());
    let mouse = dag.add_node(());
    let lion = dag.add_node(());
    let human = dag.add_node(());
    let gazelle = dag.add_node(());
    let grass = dag.add_node(());

    assert_eq!(dag.len(), 7);

    dag.add_edge(lion, human, ())?;
    dag.add_edge(lion, gazelle, ())?;

    dag.add_edge(human, dog, ())?;
    dag.add_edge(human, cat, ())?;

    dag.add_edge(dog, cat, ())?;
    dag.add_edge(cat, mouse, ())?;

    dag.add_edge(gazelle, grass, ())?;

    dag.add_edge(mouse, grass, ())?;

    Ok(([dog, cat, mouse, lion, human, gazelle, grass], dag))
  }

  #[test]
  fn add_nodes_basic() {
    let mut dag = DAG::<_, ()>::new();

    let dog = dag.add_node(());
    let cat = dag.add_node(());
    let mouse = dag.add_node(());
    let lion = dag.add_node(());
    let human = dag.add_node(());

    assert_eq!(dag.len(), 5);
    assert!(dag.contains_node(&dog));
    assert!(dag.contains_node(&cat));
    assert!(dag.contains_node(&mouse));
    assert!(dag.contains_node(&lion));
    assert!(dag.contains_node(&human));
  }

  #[test]
  fn delete_nodes() {
    let mut dag = DAG::<_, ()>::new();

    let dog = dag.add_node(());
    let cat = dag.add_node(());
    let human = dag.add_node(());

    assert_eq!(dag.len(), 3);

    assert!(dag.contains_node(&dog));
    assert!(dag.contains_node(&cat));
    assert!(dag.contains_node(&human));

    assert!(dag.remove_node(human));
    assert_eq!(dag.len(), 2);
    assert!(!dag.contains_node(&human));
  }

  #[test]
  fn reject_cycle() {
    let mut dag = DAG::new();

    let n1 = dag.add_node(());
    let n2 = dag.add_node(());
    let n3 = dag.add_node(());

    assert_eq!(dag.len(), 3);

    assert!(dag.add_edge(&n1, &n2, ()).is_ok());
    assert!(dag.add_edge(&n2, &n3, ()).is_ok());

    assert!(dag.add_edge(&n3, &n1, ()).is_err());
    assert!(dag.add_edge(&n1, &n1, ()).is_err());
  }

  #[test]
  fn get_children_unordered() {
    let ([dog, cat, mouse, _, human, _, grass], dag) = get_basic_dag().unwrap();

    let children: HashSet<_> = dag
      .descendants_unsorted(&human)
      .unwrap()
      .map(|(_, v)| v)
      .collect();

    let mut expected_children = HashSet::default();
    expected_children.extend(vec![dog, cat, mouse, grass]);

    assert_eq!(children, expected_children);

    let ordered_children: Vec<_> = dag.descendants(human).unwrap().collect();
    assert_eq!(ordered_children, vec![dog, cat, mouse, grass])
  }

  #[test]
  fn topo_order_values_no_gaps() {
    let ([.., lion, _, _, _], dag) = get_basic_dag().unwrap();

    let topo_orders: HashSet<_> = dag
      .descendants_unsorted(lion)
      .unwrap()
      .map(|p| p.0)
      .collect();

    assert_eq!(topo_orders, (2..=7).collect::<HashSet<_>>())
  }

  #[test]
  fn readme_example() {
    let mut dag = DAG::new();

    let cat = dag.add_node(());
    let dog = dag.add_node(());
    let human = dag.add_node(());

    assert_eq!(dag.len(), 3);

    dag.add_edge(&human, &dog, ()).unwrap();
    dag.add_edge(&human, &cat, ()).unwrap();
    dag.add_edge(&dog, &cat, ()).unwrap();

    let animal_order: Vec<_> = dag.descendants(&human).unwrap().collect();

    assert_eq!(animal_order, vec![dog, cat]);
  }

  #[test]
  fn unordered_iter() {
    let mut dag = DAG::new();

    let cat = dag.add_node(());
    let mouse = dag.add_node(());
    let dog = dag.add_node(());
    let human = dag.add_node(());

    assert!(dag.add_edge(&human, &cat, ()).unwrap());
    assert!(dag.add_edge(&human, &dog, ()).unwrap());
    assert!(dag.add_edge(&dog, &cat, ()).unwrap());
    assert!(dag.add_edge(&cat, &mouse, ()).unwrap());

    let pairs = dag
      .descendants_unsorted(&human)
      .unwrap()
      .collect::<HashSet<_>>();

    let mut expected_pairs = HashSet::default();
    expected_pairs.extend(vec![(2, dog), (3, cat), (4, mouse)]);

    assert_eq!(pairs, expected_pairs);
  }

  #[test]
  fn topo_cmp() {
    use std::cmp::Ordering::*;
    let mut dag = DAG::new();

    let cat = dag.add_node(());
    let mouse = dag.add_node(());
    let dog = dag.add_node(());
    let human = dag.add_node(());
    let horse = dag.add_node(());

    assert!(dag.add_edge(&human, &cat, ()).unwrap());
    assert!(dag.add_edge(&human, &dog, ()).unwrap());
    assert!(dag.add_edge(&dog, &cat, ()).unwrap());
    assert!(dag.add_edge(&cat, &mouse, ()).unwrap());

    assert_eq!(dag.topo_cmp(&human, &mouse), Less);
    assert_eq!(dag.topo_cmp(&cat, &dog), Greater);
    assert_eq!(dag.topo_cmp(&cat, &horse), Less);
  }
}
