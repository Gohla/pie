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
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use slotmap::{DefaultKey, SlotMap};

type TopoOrder = u32;


/// Data structure for maintaining a directed-acyclic graph (DAG) with topological ordering, maintained in an 
/// incremental fashion.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: index.html
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DAG<N, E, H = RandomState> {
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "N: serde::Serialize, H: BuildHasher + Default, SlotMap<DefaultKey, Node<N, H>>: serde::Serialize",
  deserialize = "N: serde::Deserialize<'de>, SlotMap<DefaultKey, Node<N, H>>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  nodes: SlotMap<DefaultKey, Node<N, H>>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "E: serde::Serialize, H: BuildHasher + Default, HashMap<(NodeId, NodeId), E, H>: serde::Serialize",
  deserialize = "E: serde::Deserialize<'de>, HashMap<(NodeId, NodeId), E, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  edges: HashMap<(NodeId, NodeId), E, H>,
  last_topo_order: TopoOrder,
}


/// An identifier of a node in the [`DAG`].
///
/// This identifier contains metadata so that a node which has been passed to [`DAG::delete_node`] 
/// will not be confused with a node created later.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NodeId(DefaultKey);

impl From<DefaultKey> for NodeId {
  #[inline]
  fn from(src: DefaultKey) -> Self { Self(src) }
}


/// A node with all information about it ordering, which nodes it points to, and which nodes point to it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Node<N, H> {
  topo_order: TopoOrder,
  data: N,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, HashSet<NodeId, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashSet<NodeId, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  parents: HashSet<NodeId, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, HashSet<NodeId, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashSet<NodeId, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  children: HashSet<NodeId, H>,
}

impl<N, H: BuildHasher + Default> Node<N, H> {
  /// Create a new node entry with the specified topological order.
  fn new(topo_order: TopoOrder, data: N) -> Self {
    Node {
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
  pub fn new() -> Self { Self::with_default_hasher() }
}

impl<N, E, H: BuildHasher + Default> DAG<N, E, H> {
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
  pub fn with_default_hasher() -> Self { Self { last_topo_order: 0, nodes: SlotMap::new(), edges: Default::default() } }

  /// Add a new node with `data` to the graph and return a unique [`NodeId`] which identifies it.
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
  pub fn add_node(&mut self, data: N) -> NodeId {
    let next_topo_order = self.last_topo_order + 1;
    self.last_topo_order = next_topo_order;
    let node_data = Node::new(next_topo_order, data);
    NodeId(self.nodes.insert(node_data))
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
  pub fn contains_node(&self, node: impl Borrow<NodeId>) -> bool {
    let node = node.borrow();
    self.nodes.contains_key(node.0)
  }

  /// Gets data for given `node`.
  #[inline]
  pub fn get_node_data(&self, node: impl Borrow<NodeId>) -> Option<&N> {
    let node = node.borrow();
    self.nodes.get(node.0).map(|d| &d.data)
  }

  /// Gets mutable data for given `node`.
  #[inline]
  pub fn get_node_data_mut(&mut self, node: impl Borrow<NodeId>) -> Option<&mut N> {
    let node = node.borrow();
    self.nodes.get_mut(node.0).map(|d| &mut d.data)
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
  pub fn remove_node(&mut self, node_id: NodeId) -> bool {
    if !self.nodes.contains_key(node_id.0) {
      return false;
    }

    // Remove node
    let node = self.nodes.remove(node_id.0).unwrap();
    // Delete forward edges
    for child in &node.children {
      if let Some(child_node) = self.nodes.get_mut(child.0) {
        child_node.parents.remove(&node_id.into());
      }
      self.edges.remove(&(node_id, *child));
    }
    // Delete backward edges
    for parent in &node.parents {
      if let Some(parent_node) = self.nodes.get_mut(parent.0) {
        parent_node.children.remove(&node_id.into());
      }
      self.edges.remove(&(*parent, node_id));
    }
    // OPTO: inefficient compaction step
    for (_, other_node) in self.nodes.iter_mut() {
      if other_node.topo_order > node.topo_order {
        other_node.topo_order -= 1;
      }
    }
    // Decrement last topo order to account for shifted topo values
    self.last_topo_order -= 1;

    true
  }

  /// Add a directed edge from `pred` to `succ`, with `parent_data` being attached to the incoming dependencies of 
  /// `succ`, and `child_data` being attached to the outgoing dependencies of `pred`.
  ///
  /// This edge indicates an ordering constraint on the two nodes, now `pred` must always come before `succ` in the 
  /// ordering.
  ///
  /// Returns `Ok(true)` if the graph did not previously contain this dependency. Returns `Ok(false)` if the graph did 
  /// have a previous dependency between these two nodes.
  ///
  /// # Errors
  /// This function will return an `Err` if the dependency introduces a cycle into the graph or if either of the nodes
  /// passed is not found in the graph.
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
  /// Here is an example which returns [`Error::CycleDetected`] when
  /// introducing a cycle:
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
    pred: impl Borrow<NodeId>,
    succ: impl Borrow<NodeId>,
    data: E,
  ) -> Result<bool, Error> {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if pred == succ { // No loops to self
      return Err(Error::CycleDetected);
    }

    // Insert forward edge
    let mut no_prev_edge = self.nodes[pred.0].children.insert(*succ);
    let upper_bound = self.nodes[pred.0].topo_order;
    // Insert backward edge
    no_prev_edge = no_prev_edge && self.nodes[succ.0].parents.insert(*pred);
    let lower_bound = self.nodes[succ.0].topo_order;
    if !no_prev_edge { // If edge already exists short circuit
      return Ok(false);
    }
    self.edges.insert((*pred, *succ), data);

    // If the affected region of the graph has non-zero size (i.e. the upper and
    // lower bound are equal) then perform an update to the topological ordering of
    // the graph
    if lower_bound < upper_bound {
      let mut visited = HashSet::<_, H>::default(); // OPTO: reuse allocation.
      // Walk changes forward from the succ, checking for any cycles that would be introduced
      let change_forward = match self.dfs_forward(*succ, &mut visited, upper_bound) {
        Ok(change_set) => change_set,
        Err(err) => { // Need to remove parent + child + edge info that was previously added
          self.nodes[pred.0].children.remove(succ);
          self.nodes[succ.0].parents.remove(pred);
          self.edges.remove(&(*pred, *succ));
          return Err(err);
        }
      };
      // Walk backwards from the pred
      let change_backward = self.dfs_backward(*pred, &mut visited, lower_bound);
      self.reorder_nodes(change_forward, change_backward);
    }

    Ok(true)
  }

  /// Checks whether adding a directed edge from `pred` to `succ` would induce a cycle.
  pub fn would_edge_induce_cycle(
    &mut self,
    pred: impl Borrow<NodeId>,
    succ: impl Borrow<NodeId>,
  ) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if pred == succ { // Introduces cycle to self.
      return true;
    }

    // Insert forward edge
    let mut no_prev_edge = self.nodes[pred.0].children.insert(*succ);
    let upper_bound = self.nodes[pred.0].topo_order;
    // Insert backward edge TODO: not needed for cycle checking?
    no_prev_edge = no_prev_edge && self.nodes[succ.0].parents.insert(*pred);
    let lower_bound = self.nodes[succ.0].topo_order;
    if !no_prev_edge { // If edge already exists, then it cannot introduce a cycle.
      return false;
    }

    let cycle = if lower_bound < upper_bound {
      let mut visited = HashSet::<_, H>::default(); // OPTO: reuse allocation.
      self.dfs_forward_cycle_check(*succ, &mut visited, upper_bound)
    } else {
      false
    };

    // Need to remove parent + child + edge info that was previously added
    self.nodes[pred.0].children.remove(succ);
    self.nodes[succ.0].parents.remove(pred); // TODO: not needed for cycle checking?

    cycle
  }

  /// Returns true if the graph contains a direct dependency from `pred` to `succ`.
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
  /// assert!(dag.contains_dependency(&cat, &mouse));
  /// assert!(!dag.contains_dependency(&human, &mouse));
  /// assert!(!dag.contains_dependency(&cat, &horse));
  /// ```
  #[inline]
  pub fn contains_dependency(&self, pred: impl Borrow<NodeId>, succ: impl Borrow<NodeId>) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if !self.nodes.contains_key(pred.0) || !self.nodes.contains_key(succ.0) {
      return false;
    }
    self.edges.contains_key(&(*pred, *succ))
  }

  /// Returns true if the graph contains a transitive dependency from `pred` to `succ`.
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
  /// assert!(dag.contains_transitive_dependency(&human, &mouse));
  /// assert!(!dag.contains_transitive_dependency(&dog, &mouse));
  /// ```
  pub fn contains_transitive_dependency(
    &self,
    pred: impl Borrow<NodeId>,
    succ: impl Borrow<NodeId>,
  ) -> bool {
    let pred = pred.borrow();
    let succ = succ.borrow();

    // If either node is missing, return quick
    if !self.nodes.contains_key(pred.0) || !self.nodes.contains_key(succ.0) {
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
    let mut visited = HashSet::<_, H>::default(); // OPTO: reuse allocation

    stack.push(pred);

    // For each node key popped off the stack, check that we haven't seen it
    // before, then check if its children contain the node we're searching for.
    // If they don't, continue the search by extending the stack with the children.
    while let Some(key) = stack.pop() {
      if visited.contains(&key) {
        continue;
      } else {
        visited.insert(key);
      }

      let children = &self.nodes.get(key.0).unwrap().children;
      if children.contains(&succ) {
        return true;
      } else {
        stack.extend(children.iter());
        continue;
      }
    }

    // If we exhaust the stack, then there is no transitive dependency.
    false
  }


  /// Gets the outgoing dependencies of given `node`.
  #[inline]
  pub fn get_outgoing_dependencies(&self, node_id: impl Borrow<NodeId>) -> impl Iterator<Item=(&NodeId, &E)> + '_ {
    let node_id = *node_id.borrow();
    self.nodes.get(node_id.0)
      .into_iter()
      .flat_map(|node| node.children.iter())
      .map(move |child_node_id| (child_node_id, self.edges.get(&(node_id, *child_node_id)).unwrap()))
  }

  /// Gets the outgoing dependency nodes of given `node`.
  #[inline]
  pub fn get_outgoing_dependency_nodes(&self, node: impl Borrow<NodeId>) -> impl Iterator<Item=&NodeId> + '_ {
    let node = node.borrow();
    self.nodes.get(node.0)
      .into_iter()
      .flat_map(|d| d.children.iter())
  }

  /// Gets the outgoing dependency data of given `node`.
  #[inline]
  pub fn get_outgoing_dependency_data(&self, node: impl Borrow<NodeId>) -> impl Iterator<Item=&N> + '_ {
    let node = node.borrow();
    self.nodes.get(node.0)
      .into_iter()
      .flat_map(|d| d.children.iter())
      .flat_map(|c| self.nodes.get(c.0).into_iter())
      .map(|nr| &nr.data)
  }


  /// Gets the incoming dependencies of given `node`.
  #[inline]
  pub fn get_incoming_dependencies(&self, node_id: impl Borrow<NodeId>) -> impl Iterator<Item=(&NodeId, &E)> + '_ {
    let node_id = *node_id.borrow();
    self.nodes.get(node_id.0)
      .into_iter()
      .flat_map(|node| node.parents.iter())
      .map(move |parent_node_id| (parent_node_id, self.edges.get(&(*parent_node_id, node_id)).unwrap()))
  }

  /// Gets the incoming dependency nodes of given `node`.
  #[inline]
  pub fn get_incoming_dependency_nodes(&self, node: impl Borrow<NodeId>) -> impl Iterator<Item=&NodeId> + '_ {
    let node = node.borrow();
    self.nodes.get(node.0)
      .into_iter()
      .flat_map(|d| d.parents.iter())
  }

  /// Gets the incoming dependency data of given `node`.
  #[inline]
  pub fn get_incoming_dependency_data(&self, node: impl Borrow<NodeId>) -> impl Iterator<Item=&N> + '_ {
    let node = node.borrow();
    self.nodes.get(node.0)
      .into_iter()
      .flat_map(|d| d.parents.iter())
      .flat_map(|c| self.nodes.get(c.0).into_iter())
      .map(|nr| &nr.data)
  }


  /// Attempt to remove the dependency from `pred` to `succ` from the graph, returning 
  /// `Some(edge_data)` if the dependency was removed, or `None` otherwise.
  ///
  /// Returns `false` is either node is not found in the graph.
  ///
  /// Removing a dependency from the graph is an extremely simple operation, which requires no recalculation of the
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
  /// assert!(dag.remove_dependency(&cat, mouse).is_some());
  /// assert!(dag.remove_dependency(&human, dog).is_some());
  /// assert!(dag.remove_dependency(&human, mouse).is_none());
  /// ```
  pub fn remove_dependency(&mut self, pred: impl Borrow<NodeId>, succ: impl Borrow<NodeId>) -> Option<E> {
    let pred = pred.borrow();
    let succ = succ.borrow();

    if !self.nodes.contains_key(pred.0) || !self.nodes.contains_key(succ.0) {
      return None;
    }
    let pred_children = &mut self.nodes[pred.0].children;
    if !pred_children.contains(&succ) {
      return None;
    }
    pred_children.remove(&succ);
    self.nodes[succ.0].parents.remove(&pred);
    self.edges.remove(&(*pred, *succ))
  }

  /// Attempt to remove all outgoing dependencies of `pred_id` from the graph, returning
  /// `Some(edge_data)` if any dependencies were removed, or `None` if the node does not exist or 
  /// does not have any dependencies.
  pub fn remove_dependencies_of_node(&mut self, pred_id: impl Borrow<NodeId>) -> Option<Vec<E>> { // TODO: test!
    let pred_id = pred_id.borrow();
    if !self.nodes.contains_key(pred_id.0) {
      return None;
    }

    let children: Vec<_> = self.nodes[pred_id.0].children.drain().collect(); // OPTO: reuse allocation
    if children.is_empty() {
      return None;
    }

    let mut edge_data = Vec::new();
    for succ_id in children {
      if let Some(succ) = self.nodes.get_mut(succ_id.0) {
        succ.parents.remove(&pred_id);
      }
      if let Some(data) = self.edges.remove(&(*pred_id, succ_id)) {
        edge_data.push(data);
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
    self.nodes.len()
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
  pub fn iter_unsorted(&self) -> impl Iterator<Item=(TopoOrder, NodeId)> + '_ {
    self.nodes
      .iter()
      .map(|(index, node)| (node.topo_order, index.into()))
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
    node: impl Borrow<NodeId>,
  ) -> Result<DescendantsUnsorted<N, E, H>, Error> {
    let node = node.borrow();
    if !self.nodes.contains_key(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut stack = Vec::new(); // OPTO: reuse allocation
    // Add all children of selected node
    stack.extend(self.nodes[node.0].children.iter());
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
  pub fn descendants(&self, node: impl Borrow<NodeId>) -> Result<Descendants<N, E, H>, Error> {
    let node = node.borrow();
    if !self.nodes.contains_key(node.0) {
      return Err(Error::NodeMissing);
    }

    let mut queue = BinaryHeap::new(); // OPTO: reuse allocation
    // Add all children of selected node
    queue.extend(
      self.nodes[node.0]
        .children
        .iter()
        .cloned()
        .map(|child_key| {
          let child_order = self.get_node(child_key).topo_order;
          (Reverse(child_order), child_key)
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
  pub fn topo_cmp(&self, node_a: impl Borrow<NodeId>, node_b: impl Borrow<NodeId>) -> Ordering {
    let node_a = node_a.borrow();
    let node_b = node_b.borrow();
    self.nodes[node_a.0]
      .topo_order
      .cmp(&self.nodes[node_b.0].topo_order)
  }


  fn dfs_forward(
    &self,
    start_key: NodeId,
    visited: &mut HashSet<NodeId, H>,
    upper_bound: TopoOrder,
  ) -> Result<HashSet<NodeId, H>, Error> {
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

  fn dfs_forward_cycle_check(
    &self,
    start_key: NodeId,
    visited: &mut HashSet<NodeId, H>,
    upper_bound: TopoOrder,
  ) -> bool {
    let mut stack = Vec::new(); // OPTO: reuse allocation
    stack.push(start_key);
    while let Some(next_key) = stack.pop() {
      visited.insert(next_key);
      for child_key in self.get_node(next_key).children.iter() {
        let child_topo_order = self.get_node(*child_key).topo_order;
        if child_topo_order == upper_bound {
          return true;
        }
        if !visited.contains(child_key) && child_topo_order < upper_bound {
          stack.push(*child_key);
        }
      }
    }
    false
  }

  fn dfs_backward(
    &self,
    start_key: NodeId,
    visited: &mut HashSet<NodeId, H>,
    lower_bound: TopoOrder,
  ) -> HashSet<NodeId, H> {
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
    change_forward: HashSet<NodeId, H>,
    change_backward: HashSet<NodeId, H>,
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
      self.nodes
        .get_mut(key.0)
        .unwrap()
        .topo_order = topo_order;
    }
  }

  fn get_node(&self, idx: NodeId) -> &Node<N, H> {
    self.nodes.get(idx.0).unwrap()
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
#[derive(Debug)]
pub struct DescendantsUnsorted<'a, N, E, H> {
  dag: &'a DAG<N, E, H>,
  stack: Vec<NodeId>,
  visited: HashSet<NodeId, H>,
}

impl<'a, N, E, H: BuildHasher> Iterator for DescendantsUnsorted<'a, N, E, H> {
  type Item = (TopoOrder, NodeId);
  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    while let Some(node) = self.stack.pop() {
      if self.visited.contains(&node) {
        continue;
      } else {
        self.visited.insert(node);
      }
      let node_repr = self.dag.nodes.get(node.0).unwrap();
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
#[derive(Debug)]
pub struct Descendants<'a, N, E, H> {
  dag: &'a DAG<N, E, H>,
  queue: BinaryHeap<(Reverse<TopoOrder>, NodeId)>,
  visited: HashSet<NodeId, H>,
}

impl<'a, N, E, H: BuildHasher + Default> Iterator for Descendants<'a, N, E, H> {
  type Item = NodeId;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      return if let Some((_, node)) = self.queue.pop() {
        if self.visited.contains(&node) {
          continue;
        } else {
          self.visited.insert(node);
        }

        let node_repr = self.dag.nodes.get(node.0).unwrap();
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


#[cfg(test)]
mod tests {
  extern crate pretty_env_logger;

  use super::*;

  fn get_basic_dag() -> Result<([NodeId; 7], DAG<(), ()>), Error> {
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
