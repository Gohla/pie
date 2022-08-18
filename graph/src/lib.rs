#![forbid(unsafe_code, missing_docs, missing_debug_implementations)]

//! The purpose of this crate is to maintain an topological order in the face
//! of single updates, like adding new nodes, adding new depedencies, deleting
//! dependencies, and deleting nodes.
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
//! dag.add_dependency(&lion, &human).unwrap();
//! dag.add_dependency(&lion, &gazelle).unwrap();
//!
//! dag.add_dependency(&human, &dog).unwrap();
//! dag.add_dependency(&human, &cat).unwrap();
//!
//! dag.add_dependency(&dog, &cat).unwrap();
//! dag.add_dependency(&cat, &mouse).unwrap();
//!
//! dag.add_dependency(&gazelle, &grass).unwrap();
//!
//! dag.add_dependency(&mouse, &grass).unwrap();
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
//! assert!(dag.contains_transitive_dependency(&lion, &grass));
//! assert!(!dag.contains_transitive_dependency(&human, &gazelle));
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

use fnv::FnvHashSet;
use thunderdome::{Arena, Index};

/// Data structure for maintaining a topological ordering over a collection
/// of elements, in an incremental fashion.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: index.html
#[derive(Default, Debug, Clone)]
pub struct DAG<N> {
  node_data: Arena<NodeRepr<N>>,
  last_topo_order: TopoOrder,
}


/// An identifier of a node in the [`IncrementalTopo`] object.
///
/// This identifier contains metadata so that a node which has been passed to
/// [`IncrementalTopo::delete_node`] will not be confused with a node created
/// later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Node(Index);

impl From<Index> for Node {
  #[inline]
  fn from(src: Index) -> Self { Self(src) }
}

/// An identifier of a node that is lacking additional safety metadata that
/// prevents ABA issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct UnsafeIndex(u32);

impl From<Node> for UnsafeIndex {
  #[inline]
  fn from(src: Node) -> Self { Self(src.0.slot()) }
}

impl From<&Node> for UnsafeIndex {
  #[inline]
  fn from(src: &Node) -> Self { Self(src.0.slot()) }
}


/// The representation of a node, with all information about it ordering, which
/// nodes it points to, and which nodes point to it.
#[derive(Debug, Clone)]
struct NodeRepr<N> {
  topo_order: TopoOrder,
  data: N,
  parents: FnvHashSet<UnsafeIndex>,
  children: FnvHashSet<UnsafeIndex>,
}

impl<N> NodeRepr<N> {
  /// Create a new node entry with the specified topological order.
  fn new(topo_order: TopoOrder, data: N) -> Self {
    NodeRepr {
      topo_order,
      data,
      parents: FnvHashSet::default(),
      children: FnvHashSet::default(),
    }
  }
}

type TopoOrder = u32;

impl<N> PartialEq for NodeRepr<N> {
  fn eq(&self, other: &Self) -> bool {
    self.topo_order == other.topo_order
  }
}

impl<N> Eq for NodeRepr<N> {}

impl<N> PartialOrd for NodeRepr<N> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.topo_order.cmp(&other.topo_order))
  }
}

impl<N> Ord for NodeRepr<N> {
  fn cmp(&self, other: &Self) -> Ordering {
    self.partial_cmp(other).unwrap()
  }
}


/// Different types of failures that can occur while updating or querying
/// the graph.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
  /// The given node was not found in the topological order.
  ///
  /// This usually means that the node was deleted, but a reference was
  /// kept around after which is now invalid.
  NodeMissing,
  /// Cycles of nodes may not be formed in the graph.
  CycleDetected,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::NodeMissing => {
        write!(f, "The given node was not found in the topological order")
      },
      Error::CycleDetected => write!(f, "Cycles of nodes may not be formed in the graph"),
    }
  }
}

impl std::error::Error for Error {}


impl<N> DAG<N> {
  /// Create a new DAG.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let dag = DAG::<()>::new();
  ///
  /// assert!(dag.is_empty());
  /// ```
  #[inline]
  pub fn new() -> Self { Self { last_topo_order: 0, node_data: Arena::new(), } }

  /// Add a new node to the graph and return a unique [`Node`] which
  /// identifies it.
  ///
  /// Initially this node will not have any order relative to the values
  /// that are already in the graph. Only when relations are added
  /// with [`add_dependency`] will the order begin to matter.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
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
    let node_data = NodeRepr::new(next_topo_order, data);
    Node(self.node_data.insert(node_data))
  }

  /// Returns true if the graph contains the specified node.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
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
    self.node_data.contains(node.0)
  }

  /// Gets data for given `node`.
  #[inline]
  pub fn get_node_data(&self, node: impl Borrow<Node>) -> Option<&N> {
    let node = node.borrow();
    self.node_data.get(node.0).map(|d| &d.data)
  }

  /// Gets mutable data for given `node`.
  #[inline]
  pub fn get_node_data_mut(&mut self, node: impl Borrow<Node>) -> Option<&mut N> {
    let node = node.borrow();
    self.node_data.get_mut(node.0).map(|d| &mut d.data)
  }

  /// Attempt to remove node from graph, returning true if the node was
  /// contained and removed.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
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
    if !self.node_data.contains(node.0) {
      return false;
    }

    // Remove associated data
    let data = self.node_data.remove(node.0).unwrap();
    // Delete forward edges
    for child in data.children {
      if let Some((_, child_data)) = self.node_data.get_by_slot_mut(child.0) {
        child_data.parents.remove(&node.into());
      }
    }
    // Delete backward edges
    for parent in data.parents {
      if let Some((_, parent_data)) = self.node_data.get_by_slot_mut(parent.0) {
        parent_data.children.remove(&node.into());
      }
    }
    // OPTO: inefficient compaction step
    for (_, other_node) in self.node_data.iter_mut() {
      if other_node.topo_order > data.topo_order {
        other_node.topo_order -= 1;
      }
    }
    // Decrement last topo order to account for shifted topo values
    self.last_topo_order -= 1;

    true
  }

  /// Add a directed link between two nodes already present in the graph.
  ///
  /// This link indicates an ordering constraint on the two nodes, now
  /// `pred` must always come before `succ` in the ordering.
  ///
  /// Returns `Ok(true)` if the graph did not previously contain this
  /// dependency. Returns `Ok(false)` if the graph did have a previous
  /// dependency between these two nodes.
  ///
  /// # Errors
  /// This function will return an `Err` if the dependency introduces a
  /// cycle into the graph or if either of the nodes passed is not
  /// found in the graph.
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
  /// assert!(dag.add_dependency(&human, dog).unwrap());
  /// assert!(dag.add_dependency(&human, cat).unwrap());
  /// assert!(dag.add_dependency(&cat, mouse).unwrap());
  /// ```
  ///
  /// Here is an example which returns [`Error::CycleDetected`] when
  /// introducing a cycle:
  ///
  /// ```
  /// # use pie_graph::{DAG, Error};
  /// let mut dag = DAG::new();
  ///
  /// let n0 = dag.add_node(());
  /// assert_eq!(dag.add_dependency(&n0, &n0), Err(Error::CycleDetected));
  ///
  /// let n1 = dag.add_node(());
  ///
  /// assert!(dag.add_dependency(&n0, &n1).unwrap());
  /// assert_eq!(dag.add_dependency(&n1, &n0), Err(Error::CycleDetected));
  ///
  /// let n2 = dag.add_node(());
  ///
  /// assert!(dag.add_dependency(&n1, &n2).unwrap());
  /// assert_eq!(dag.add_dependency(&n2, &n0), Err(Error::CycleDetected));
  /// ```
  pub fn add_dependency(
    &mut self,
    pred: impl Borrow<Node>,
    succ: impl Borrow<Node>,
  ) -> Result<bool, Error> {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if pred == succ { // No loops to self
      return Err(Error::CycleDetected);
    }

    let succ_index = UnsafeIndex::from(succ);
    let pred_index = UnsafeIndex::from(pred);

    // Insert forward edge
    let mut no_prev_edge = self.node_data[pred.0].children.insert(succ_index);
    let upper_bound = self.node_data[pred.0].topo_order;
    // Insert backward edge
    no_prev_edge = no_prev_edge && self.node_data[succ.0].parents.insert(pred_index);
    let lower_bound = self.node_data[succ.0].topo_order;
    // If edge already exists short circuit
    if !no_prev_edge {
      return Ok(false);
    }
    // If the affected region of the graph has non-zero size (i.e. the upper and
    // lower bound are equal) then perform an update to the topological ordering of
    // the graph
    if lower_bound < upper_bound {
      let mut visited = FnvHashSet::default(); // OPTO: reuse allocation.
      // Walk changes forward from the succ, checking for any cycles that would be introduced
      let change_forward = match self.dfs_forward(succ_index, &mut visited, upper_bound) {
        Ok(change_set) => change_set,
        Err(err) => { // Need to remove parent + child info that was previously added
          self.node_data[pred.0].children.remove(&succ_index);
          self.node_data[succ.0].parents.remove(&pred_index);
          return Err(err);
        },
      };
      // Walk backwards from the pred
      let change_backward = self.dfs_backward(pred_index, &mut visited, lower_bound);
      self.reorder_nodes(change_forward, change_backward);
    }

    Ok(true)
  }

  /// Returns true if the graph contains a dependency from `pred` to
  /// `succ`.
  ///
  /// Returns false if either node is not found, or if there is no
  /// dependency.
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
  ///
  /// assert!(dag.contains_dependency(&cat, &mouse));
  /// assert!(!dag.contains_dependency(&human, &mouse));
  /// assert!(!dag.contains_dependency(&cat, &horse));
  /// ```
  #[inline]
  pub fn contains_dependency(&self, pred: impl Borrow<Node>, succ: impl Borrow<Node>) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if !self.node_data.contains(pred.0) || !self.node_data.contains(succ.0) {
      return false;
    }
    self.node_data[pred.0].children.contains(&succ.into())
  }

  /// Returns true if the graph contains a transitive dependency from
  /// `pred` to `succ`.
  ///
  /// In this context a transitive dependency means that `succ` exists as
  /// a descendant of `pred`, with some chain of other nodes in
  /// between.
  ///
  /// Returns false if either node is not found in the graph, or there is
  /// no transitive dependency.
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
  ///
  /// assert!(dag.contains_transitive_dependency(&human, &mouse));
  /// assert!(!dag.contains_transitive_dependency(&dog, &mouse));
  /// ```
  pub fn contains_transitive_dependency(
    &self,
    pred: impl Borrow<Node>,
    succ: impl Borrow<Node>,
  ) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    // If either node is missing, return quick
    if !self.node_data.contains(pred.0) || !self.node_data.contains(succ.0) {
      return false;
    }

    // A node cannot depend on itself
    if pred.0 == succ.0 {
      return false;
    }

    // Else we have to search the graph. Using dfs in this case because it avoids
    // the overhead of the binary heap, and this task doesn't really need ordered
    // descendants.
    let mut stack = Vec::new(); // OPTO: reuse allocation
    let mut visited = FnvHashSet::default(); // OPTO: reuse allocation

    stack.push(UnsafeIndex::from(pred));

    // For each node key popped off the stack, check that we haven't seen it
    // before, then check if its children contain the node we're searching for.
    // If they don't, continue the search by extending the stack with the children.
    while let Some(key) = stack.pop() {
      if visited.contains(&key) {
        continue;
      } else {
        visited.insert(key);
      }

      let children = &self.node_data.get_by_slot(key.0).unwrap().1.children;
      if children.contains(&succ.into()) {
        return true;
      } else {
        stack.extend(children);
        continue;
      }
    }

    // If we exhaust the stack, then there is no transitive dependency.
    false
  }


  /// Gets the outgoing dependency nodes of given `node`.
  #[inline]
  pub fn get_outgoing_dependency_nodes(&self, node: impl Borrow<Node>) -> impl Iterator<Item=Node> + '_ {
    let node = node.borrow();
    self.get_outgoing_dependencies(node.0).map(|(i, _)| Node(i))
  }

  /// Gets the outgoing dependency data of given `node`.
  #[inline]
  pub fn get_outgoing_dependency_data(&self, node: impl Borrow<Node>) -> impl Iterator<Item=&N> + '_ {
    let node = node.borrow();
    self.get_outgoing_dependencies(node.0).map(|(_, d)| &d.data)
  }

  /// Gets the outgoing dependency nodes and data of given `node`.
  #[inline]
  pub fn get_outgoing_dependency_nodes_and_data(&self, node: impl Borrow<Node>) -> impl Iterator<Item=(Node, &N)> + '_ {
    let node = node.borrow();
    self.get_outgoing_dependencies(node.0).map(|(i, d)| (Node(i), &d.data))
  }

  #[inline]
  fn get_outgoing_dependencies(&self, index: Index) -> impl Iterator<Item=(Index, &NodeRepr<N>)> + '_ {
    self.node_data.get(index)
      .into_iter()
      .flat_map(|d| d.children.iter().flat_map(|c| self.node_data.get_by_slot(c.0).into_iter()))
  }


  /// Gets the incoming dependency nodes of given `node`.
  #[inline]
  pub fn get_incoming_dependency_nodes(&self, node: impl Borrow<Node>) -> impl Iterator<Item=Node> + '_ {
    let node = node.borrow();
    self.get_incoming_dependencies(node.0).map(|(i, _)| Node(i))
  }

  /// Gets the incoming dependency data of given `node`.
  #[inline]
  pub fn get_incoming_dependency_data(&self, node: impl Borrow<Node>) -> impl Iterator<Item=&N> + '_ {
    let node = node.borrow();
    self.get_incoming_dependencies(node.0).map(|(_, d)| &d.data)
  }

  /// Gets the incoming dependency nodes and data of given `node`.
  #[inline]
  pub fn get_incoming_dependency_nodes_and_data(&self, node: impl Borrow<Node>) -> impl Iterator<Item=(Node, &N)> + '_ {
    let node = node.borrow();
    self.get_incoming_dependencies(node.0).map(|(i, d)| (Node(i), &d.data))
  }

  #[inline]
  fn get_incoming_dependencies(&self, index: Index) -> impl Iterator<Item=(Index, &NodeRepr<N>)> + '_ {
    self.node_data.get(index)
      .into_iter()
      .flat_map(|d| d.parents.iter().flat_map(|c| self.node_data.get_by_slot(c.0).into_iter()))
  }


  /// Attempt to remove a dependency from the graph, returning true if the
  /// dependency was removed.
  ///
  /// Returns false is either node is not found in the graph.
  ///
  /// Removing a dependency from the graph is an extremely simple
  /// operation, which requires no recalculation of the
  /// topological order. The ordering before and after a removal
  /// is exactly the same.
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
  ///
  /// assert!(dag.remove_dependency(&cat, mouse));
  /// assert!(dag.remove_dependency(&human, dog));
  /// assert!(!dag.remove_dependency(&human, mouse));
  /// ```
  pub fn remove_dependency(&mut self, pred: impl Borrow<Node>, succ: impl Borrow<Node>) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if !self.node_data.contains(pred.0) || !self.node_data.contains(succ.0) {
      return false;
    }
    let pred_children = &mut self.node_data[pred.0].children;
    if !pred_children.contains(&succ.into()) {
      return false;
    }
    pred_children.remove(&succ.into());
    self.node_data[succ.0].parents.remove(&pred.into());

    true
  }

  /// Return the number of nodes within the graph.
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
  /// assert_eq!(dag.len(), 4);
  /// ```
  #[inline]
  pub fn len(&self) -> usize {
    self.node_data.len()
  }

  /// Return true if there are no nodes in the graph.
  ///
  /// # Examples
  /// ```
  /// use pie_graph::DAG;
  /// let mut dag = DAG::new();
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
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
    self.node_data
      .iter()
      .map(|(index, node)| (node.topo_order, index.into()))
  }

  /// Return an iterator over the descendants of a node in the graph, in
  /// an unsorted order.
  ///
  /// Accessing the nodes in an unsorted order allows for faster access
  /// using a iterative DFS search. This is opposed to the order
  /// descendants iterator which requires the use of a binary heap
  /// to order the values.
  ///
  /// # Errors
  ///
  /// This function will return an error if the given node is not present in
  /// the graph.
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&dog, &cat).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
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
  ) -> Result<DescendantsUnsorted<N>, Error> {
    let node = node.borrow();
    if !self.node_data.contains(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut stack = Vec::new(); // OPTO: reuse allocation
    // Add all children of selected node
    stack.extend(&self.node_data[node.0].children);
    let visited = FnvHashSet::default(); // OPTO: reuse allocation

    Ok(DescendantsUnsorted {
      dag: self,
      stack,
      visited,
    })
  }

  /// Return an iterator over descendants of a node in the graph, in a
  /// topologically sorted order.
  ///
  /// Accessing the nodes in a sorted order requires the use of a
  /// BinaryHeap, so some performance penalty is paid there. If
  /// all is required is access to the descendants of a node, use
  /// [`IncrementalTopo::descendants_unsorted`].
  ///
  /// # Errors
  ///
  /// This function will return an error if the given node is not present in
  /// the graph.
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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&dog, &cat).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
  ///
  /// let ordered_nodes = dag.descendants(human).unwrap().collect::<Vec<_>>();
  ///
  /// assert_eq!(ordered_nodes, vec![dog, cat, mouse]);
  /// ```
  pub fn descendants(&self, node: impl Borrow<Node>) -> Result<Descendants<N>, Error> {
    let node = node.borrow();
    if !self.node_data.contains(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut queue = BinaryHeap::new(); // OPTO: reuse allocation
    // Add all children of selected node
    queue.extend(
      self.node_data[node.0]
        .children
        .iter()
        .cloned()
        .map(|child_key| {
          let child_order = self.get_node_repr(child_key).topo_order;
          (Reverse(child_order), child_key)
        }),
    );
    let visited = FnvHashSet::default(); // OPTO: reuse allocation

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
  /// assert!(dag.add_dependency(&human, &cat).unwrap());
  /// assert!(dag.add_dependency(&human, &dog).unwrap());
  /// assert!(dag.add_dependency(&dog, &cat).unwrap());
  /// assert!(dag.add_dependency(&cat, &mouse).unwrap());
  ///
  /// assert_eq!(dag.topo_cmp(&human, &mouse), Less);
  /// assert_eq!(dag.topo_cmp(&cat, &dog), Greater);
  /// assert_eq!(dag.topo_cmp(&cat, &horse), Less);
  /// ```
  #[inline]
  pub fn topo_cmp(&self, node_a: impl Borrow<Node>, node_b: impl Borrow<Node>) -> Ordering {
    let node_a = node_a.borrow();
    let node_b = node_b.borrow();
    self.node_data[node_a.0]
      .topo_order
      .cmp(&self.node_data[node_b.0].topo_order)
  }


  fn dfs_forward(
    &self,
    start_key: UnsafeIndex,
    visited: &mut FnvHashSet<UnsafeIndex>,
    upper_bound: TopoOrder,
  ) -> Result<FnvHashSet<UnsafeIndex>, Error> {
    let mut stack = Vec::new(); // OPTO: reuse allocation
    let mut result = FnvHashSet::default(); // OPTO: reuse allocation

    stack.push(start_key);

    while let Some(next_key) = stack.pop() {
      visited.insert(next_key);
      result.insert(next_key);

      for child_key in &self.get_node_repr(next_key).children {
        let child_topo_order = self.get_node_repr(*child_key).topo_order;

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
    start_key: UnsafeIndex,
    visited: &mut FnvHashSet<UnsafeIndex>,
    lower_bound: TopoOrder,
  ) -> FnvHashSet<UnsafeIndex> {
    let mut stack = Vec::new(); // OPTO: reuse allocation
    let mut result = FnvHashSet::default(); // OPTO: reuse allocation

    stack.push(start_key);

    while let Some(next_key) = stack.pop() {
      visited.insert(next_key);
      result.insert(next_key);

      for parent_key in &self.get_node_repr(next_key).parents {
        let parent_topo_order = self.get_node_repr(*parent_key).topo_order;

        if !visited.contains(parent_key) && lower_bound < parent_topo_order {
          stack.push(*parent_key);
        }
      }
    }

    result
  }

  fn reorder_nodes(
    &mut self,
    change_forward: FnvHashSet<UnsafeIndex>,
    change_backward: FnvHashSet<UnsafeIndex>,
  ) {
    let mut change_forward: Vec<_> = change_forward
      .into_iter()
      .map(|key| (key, self.get_node_repr(key).topo_order))
      .collect(); // OPTO: reuse allocation
    change_forward.sort_unstable_by_key(|pair| pair.1);

    let mut change_backward: Vec<_> = change_backward
      .into_iter()
      .map(|key| (key, self.get_node_repr(key).topo_order))
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
      self.node_data
        .get_by_slot_mut(key.0)
        .unwrap()
        .1
        .topo_order = topo_order;
    }
  }

  fn get_node_repr(&self, idx: UnsafeIndex) -> &NodeRepr<N> {
    self.node_data.get_by_slot(idx.0).unwrap().1
  }
}

/// An iterator over the descendants of a node in the graph, which outputs the
/// nodes in an unsorted order with their topological ranking.
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
/// assert!(dag.add_dependency(&human, &cat).unwrap());
/// assert!(dag.add_dependency(&human, &dog).unwrap());
/// assert!(dag.add_dependency(&dog, &cat).unwrap());
/// assert!(dag.add_dependency(&cat, &mouse).unwrap());
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
#[derive(Debug)]
pub struct DescendantsUnsorted<'a, N> {
  dag: &'a DAG<N>,
  stack: Vec<UnsafeIndex>,
  visited: FnvHashSet<UnsafeIndex>,
}

impl<'a, N> Iterator for DescendantsUnsorted<'a, N> {
  type Item = (TopoOrder, Node);
  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    while let Some(key) = self.stack.pop() {
      if self.visited.contains(&key) {
        continue;
      } else {
        self.visited.insert(key);
      }
      let (index, node_data) = self.dag.node_data.get_by_slot(key.0).unwrap();
      let order = node_data.topo_order;
      self.stack.extend(&node_data.children);
      return Some((order, index.into()));
    }

    None
  }
}

/// An iterator over the descendants of a node in the graph, which outputs the
/// nodes in a sorted order by their topological ranking.
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
/// assert!(dag.add_dependency(&human, &cat).unwrap());
/// assert!(dag.add_dependency(&human, &dog).unwrap());
/// assert!(dag.add_dependency(&dog, &cat).unwrap());
/// assert!(dag.add_dependency(&cat, &mouse).unwrap());
///
/// let ordered_nodes = dag.descendants(human).unwrap().collect::<Vec<_>>();
///
/// assert_eq!(ordered_nodes, vec![dog, cat, mouse]);
/// ```
#[derive(Debug)]
pub struct Descendants<'a, N> {
  dag: &'a DAG<N>,
  queue: BinaryHeap<(Reverse<TopoOrder>, UnsafeIndex)>,
  visited: FnvHashSet<UnsafeIndex>,
}

impl<'a, N> Iterator for Descendants<'a, N> {
  type Item = Node;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return if let Some((_, key)) = self.queue.pop() {
        if self.visited.contains(&key) {
          continue;
        } else {
          self.visited.insert(key);
        }

        let (index, node_data) = self.dag.node_data.get_by_slot(key.0).unwrap();
        for child in &node_data.children {
          let order = self.dag.get_node_repr(*child).topo_order;
          self.queue.push((Reverse(order), *child))
        }

        Some(index.into())
      } else {
        None
      }
    }
  }
}


#[cfg(test)]
mod tests {
  extern crate pretty_env_logger;

  use super::*;

  fn get_basic_dag() -> Result<([Node; 7], DAG<()>), Error> {
    let mut dag = DAG::new();

    let dog = dag.add_node(());
    let cat = dag.add_node(());
    let mouse = dag.add_node(());
    let lion = dag.add_node(());
    let human = dag.add_node(());
    let gazelle = dag.add_node(());
    let grass = dag.add_node(());

    assert_eq!(dag.len(), 7);

    dag.add_dependency(lion, human)?;
    dag.add_dependency(lion, gazelle)?;

    dag.add_dependency(human, dog)?;
    dag.add_dependency(human, cat)?;

    dag.add_dependency(dog, cat)?;
    dag.add_dependency(cat, mouse)?;

    dag.add_dependency(gazelle, grass)?;

    dag.add_dependency(mouse, grass)?;

    Ok(([dog, cat, mouse, lion, human, gazelle, grass], dag))
  }

  #[test]
  fn add_nodes_basic() {
    let mut dag = DAG::new();

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
    let mut dag = DAG::new();

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

    assert!(dag.add_dependency(&n1, &n2).is_ok());
    assert!(dag.add_dependency(&n2, &n3).is_ok());

    assert!(dag.add_dependency(&n3, &n1).is_err());
    assert!(dag.add_dependency(&n1, &n1).is_err());
  }

  #[test]
  fn get_children_unordered() {
    let ([dog, cat, mouse, _, human, _, grass], dag) = get_basic_dag().unwrap();

    let children: FnvHashSet<_> = dag
      .descendants_unsorted(&human)
      .unwrap()
      .map(|(_, v)| v)
      .collect();

    let mut expected_children = FnvHashSet::default();
    expected_children.extend(vec![dog, cat, mouse, grass]);

    assert_eq!(children, expected_children);

    let ordered_children: Vec<_> = dag.descendants(human).unwrap().collect();
    assert_eq!(ordered_children, vec![dog, cat, mouse, grass])
  }

  #[test]
  fn topo_order_values_no_gaps() {
    let ([.., lion, _, _, _], dag) = get_basic_dag().unwrap();

    let topo_orders: FnvHashSet<_> = dag
      .descendants_unsorted(lion)
      .unwrap()
      .map(|p| p.0)
      .collect();

    assert_eq!(topo_orders, (2..=7).collect::<FnvHashSet<_>>())
  }

  #[test]
  fn readme_example() {
    let mut dag = DAG::new();

    let cat = dag.add_node(());
    let dog = dag.add_node(());
    let human = dag.add_node(());

    assert_eq!(dag.len(), 3);

    dag.add_dependency(&human, &dog).unwrap();
    dag.add_dependency(&human, &cat).unwrap();
    dag.add_dependency(&dog, &cat).unwrap();

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

    assert!(dag.add_dependency(&human, &cat).unwrap());
    assert!(dag.add_dependency(&human, &dog).unwrap());
    assert!(dag.add_dependency(&dog, &cat).unwrap());
    assert!(dag.add_dependency(&cat, &mouse).unwrap());

    let pairs = dag
      .descendants_unsorted(&human)
      .unwrap()
      .collect::<FnvHashSet<_>>();

    let mut expected_pairs = FnvHashSet::default();
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

    assert!(dag.add_dependency(&human, &cat).unwrap());
    assert!(dag.add_dependency(&human, &dog).unwrap());
    assert!(dag.add_dependency(&dog, &cat).unwrap());
    assert!(dag.add_dependency(&cat, &mouse).unwrap());

    assert_eq!(dag.topo_cmp(&human, &mouse), Less);
    assert_eq!(dag.topo_cmp(&cat, &dog), Greater);
    assert_eq!(dag.topo_cmp(&cat, &horse), Less);
  }
}
