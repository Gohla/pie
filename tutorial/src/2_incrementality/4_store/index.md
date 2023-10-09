# Dependency Graph Store

To do incremental building, we need to keep track of all files, tasks, their dependencies, and task outputs, in a dependency graph.
This will be the responsibility of the `Store` data structure.
Context implementations will use methods on `Store` to query and mutate the dependency graph.
In other words, `Store` encapsulates the dependency graph.

However, writing a dependency graph data structure is outside of the scope of this tutorial, so we will be using the `pie_graph` library which we prepared exactly for this use case.
The graph from this library is a directed acyclic graph (DAG), meaning that edges are directed and there may be no cycles in edges, as that would prohibit topological orderings.

```admonish tip title="Graph Library" collapsible=true
The `pie_graph` library is a modified version of the great [`incremental-topo`](https://github.com/declanvk/incremental-topo/) library which implements incremental topological ordering: it keeps the topological ordering up-to-date incrementally while nodes and edges are added and removed.
That is exactly what we need, as dynamic dependencies prevents us from calculating the topological ordering in one go, and calculating the topological ordering after every task execution is prohibitively expensive.
The implementation in the `incremental-topo` library is based on a [paper by D. J. Pearce and P. H. J. Kelly](http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf) that describes several dynamic topological sort algorithms for directed acyclic graphs.
```

Add the `pie_graph` dependency to `pie/Cargo.toml`:

```diff2html fromfile linebyline
../../gen/2_incrementality/4_store/a_Cargo.toml.diff
```

## Store basics

Add the `store` module to `pie/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/4_store/b_module.rs.diff
```

This module is private, as users of the library should not interact with the store.
Only `Context` implementations will use the store.

Create the `pie/src/store.rs` file and add the following to get started:

```rust,
{{#include c_basic.rs}}
```

The `Store` is generic over tasks `T` and their outputs `O`, like we have done before with `Dependency`.

The `DAG` type from `pie_graph` represents a DAG with nodes and edges, and data attached to those nodes and edges.
The nodes in our graph are either files or tasks, and the edges are dependencies.

The first generic argument to `DAG` is the type of data to attach to nodes, which is `NodeData<T, O>` in our case.
Because nodes can be files or tasks, `NodeData<T, O>` enumerates these, storing the path for files, and the task along with its output for tasks.
We store file paths as `PathBuf`, which is the owned version of `Path` (similar to `String` being the owned version of `str`).
The task output is stored as `Option<O>` because we can add a task to the graph without having executed it, so we don't have its output yet.

The second argument is the type of data to attach to edges, which is `Dependency<T, O>`, using the `Dependency` enum we defined earlier.

We implement `Default` for the store to initialize it.

```admonish question title="Why not Derive Default?" collapsible=true
We cannot derive this `Default` implementation even though it seems we should be able to, because the derived implementation will require `T` and `O` to be `Default`, and this is not always the case.
This is because the `Default` derive macro is conservative and adds a `: Default` bound to *every* generic argument in the `Default` trait implementation, and there is no way to disable this behaviour.
Therefore, we implement `Default` ourselves.

There are several crates that have more configurable derive macros for these things, but adding an extra dependency to generate a few lines of code is not worth the extra compilation time, so we just implement it manually here.
```

## Graph nodes

A node in `DAG` is represented by a `Node`, which is a transparent identifier (sometimes called a [handle](https://en.wikipedia.org/wiki/Handle_(computing))) that points to the node and its data.
We can create nodes in the graph, and then query attached data (`NodeData`) given a node.
So `DAG` allows us to go from `Node` to a `PathBuf` and task `T` through attached `NodeData`.

However, we want each unique file and task to be represented by a single unique node in the graph.
We need this for incrementality so that if the build system encounters the same task twice, we can find the corresponding task node in the graph the second time, check if it is consistent, and return its output if it is.

To ensure unique nodes, we need to maintain the reverse mapping from `PathBuf` and `T` to `Node` ourselves, which we will do with `HashMap`s.
This is also the reason for the `Eq` and `Hash` trait bounds on the `Task` trait, so we can use them as keys in `HashMap`s.

Change `pie/src/store.rs` to add hash maps to map between these things:

```diff2html fromfile linebyline
../../gen/2_incrementality/4_store/d1_mapping_diff.rs.diff
```

To prevent accidentally using a file node as a task node, and vice versa, change `pie/src/store.rs` to add specific types of nodes:

```diff2html fromfile
../../gen/2_incrementality/4_store/d2_mapping_diff.rs.diff
```

The `FileNode` and `TaskNode` types are [newtypes](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) that wrap a `Node` into a specific type of node.
The `Borrow` implementations will make subsequent code a bit more concise by automatically converting `&FileNode` and `&TaskNode`s to `&Node`s.

```admonish question title="Do these Newtypes Improve Type-Safety?" collapsible=true
Because the `Node`s inside the newtypes are not public, it is not possible to construct a `FileNode` or `TaskNode` outside of this module.
Therefore, if we only accept and create `FileNode` and `TaskNode` in the `Store` API, it is not possible to use the wrong kind of node, increasing type-safety.

The `Borrow` implementation does leak outside of this module, but not outside of this crate (library).
This is because the visibility of a trait implementation is the intersection of the visibilities of the trait and type it is implemented on.
`Borrow` is public, but `FileNode` and `TaskNode` are only public within this crate.
Thefore, modules of this crate can extract the `Node` out of `FileNode` and `TaskNode`.
However, that `Node` cannot be used to construct a `FileNode` or `TaskNode`, so it is not a problem.
```

Now we will add methods create nodes and to query their attached data.
Add the following code to `pie/src/store.rs`:

```rust,
{{#include e_mapping.rs:2:}}
```

The `get_or_create_file_node` method creates file nodes.
When we want to go from a file path (using `impl AsRef<Path>`) to a `FileNode`, either we have already added this file path to the graph and want to get the `FileNode` for it, or we have not yet added it to the graph yet and should add it.
The former is handled by the if branch in `get_or_create_file_node`, where we just retrieve the `FileNode` from the `file_to_node` hash map.
The latter is handled by the else branch where we add the node to the graph with `graph.add_node` which attaches the `NodeData::File` data to the node, and then returns a `FileNode` which we insert into the `file_to_node` map.

The `get_file_path` method does the inverse.
We get the attached data given a node, and extract the file path from it.

Note that we are using `panic!` here to indicate that invalid usage of this method is an *unrecoverable programming error* that should not occur.
Returning an `Option<&PathBuf>` makes no sense here, as the caller of this method has no way to recover from this.
Because this is not an end-user-facing API (`store` module is private), we control all the calls to this method, and thus we are responsible for using these methods in a valid way. 
Therefore, when we call these methods, we should document why it is valid (if this is not immediately obvious), and we need to test whether we really use it in a valid way.

We're also documenting the panics in a `# Panics` section in the documentation comment, as is common practice in Rust.

```admonish question title="How to Trigger these Panics?" collapsible=true
Because only `Store` can create `FileNode`s and `TaskNode`s, and all methods only take these values as inputs, these panics will not happen under normal usage.
The only way to trigger these panics (in safe Rust) would be to create two stores, and use the nodes from one store in another.
However, since this is a private module, we just need to make sure that we don't do that.

There are some tricks to prevent even this kind of invalid usage.
For example, the [generativity](https://docs.rs/generativity/latest/generativity/) crate generates unique identifiers based on lifetimes.
However, that is a bit overkill, especially for an internal API, so we won't be using that.
```

We implement similar methods for task nodes in `get_or_create_task_node` and `get_task`.

## Task outputs

When we do not need to execute a task because it is consistent, we still need to return its output.
Therefore, we store the task output in `NodeData::Task` and add methods to query and manipulate task outputs.
Add the following code to `pie/src/store.rs`:

```rust,
{{#include f_output.rs:2:}}
```

The `task_has_output`, `get_task_output`, and `set_task_output` methods manipulate task outputs in `NodeData::Task`.

Again, we are using panics here to indicate unrecoverable programming errors.

## Dependencies

Now we need methods to query and manipulate dependencies.
The edges in the graph are dependencies between tasks and files.
Tasks can depend on other tasks and files, but there are no dependencies between files.
An edge does not have its own dedicated representation, and is simply represented by two nodes: the source node and the destination node of the edge.

Add the following code to `pie/src/store.rs`:

```rust,
{{#include g_dependency.rs:2:}}
```

The `get_dependencies_of_task` method gets the dependencies (edge data of outgoing edges) of a task, and returns it as an iterator (which is empty if task has no dependencies).
This method needs explicit lifetime annotations due to the signature of `get_outgoing_edge_data` and the way we return an iterator using `impl Iterator<...`.
We're using `debug_assert!` here to trigger a panic indicating an unrecoverable programming error only in development mode, because this check is too expensive to run in release (optimized) mode.

The `add_file_require_dependency` method adds a file dependency.
Adding an edge to the graph can result in cycles, which are not allowed in a directed *acyclic* graph (DAG).
Therefore, `graph.add_edge` can return an `Err` indicating that there is a cycle.
In case of files, this cannot happen because files do not have outgoing dependencies, and the API enforces this by never taking a `FileNode` as a source (`src`) of an edge.

Tasks can depend on other tasks, so they can create cycles.
In `add_task_require_dependency`, we propagate the cycle detected error (by returning `Err(())`) to the caller because the caller has more information to create an error message for the user that made a cyclic task dependency.

[//]: # (We want to catch those cycles not just because they are not allowed in a DAG, but because cycles between tasks could result in cyclic task execution, causing builds to infinitely execute.)

[//]: # ()
[//]: # (When a task requires another task, we need to add an edge to the dependency graph to check if this edge creates a cycle, but also to have this edge in the dependency graph for future cycle detection.)

[//]: # (However, at the moment we require the task, we do not yet have an output for the task, as we only get an output after we've executed the task.)

[//]: # (Therefore, we first *reserve* the task dependency, and then update it with an output.)

[//]: # (This manifests itself as the attached edge data being `Option<Dependency<T, O>>`, where `None` indicates that the dependency has been reserved.)

[//]: # ()
[//]: # (The `reserve_task_require_dependency` and `update_reserved_task_require_dependency` methods implement this behavior.)

[//]: # (We propagate cycle errors so that the caller can report an error message.)

## Resetting tasks

Finally, when we determine that a task is inconsistent and needs to be executed, we first need to remove its output and remove its outgoing dependencies, as those will interfere with incrementality when not removed.
We do NOT want to remove incoming dependencies, as that would remove dependencies from other tasks to this task, which breaks incrementality, so we can't just remove and re-add the task to the graph.
Add the `reset_task` method that does this to `pie/src/store.rs`:

```rust,
{{#include h_reset.rs:2:}}
```

This will reset the task output back to `None`, and remove all outgoing edges (dependencies).

## Tests

Now we've implemented everything we need for implementing the top-down context, but first we will write some tests.

### Testing file mapping

Add the following code to `pie/src/store.rs` for testing the file mapping:

```rust,
{{#include i_test_file_mapping.rs:3:}}
```

We create a simple task `StringConstant` because we need a `Task` implementation to test `Store`, as `Store` is generic over a `Task` type.
We will never execute it because `Store` does not execute tasks.

Test `test_file_mapping` checks whether the file node mapping works as expected:
- `get_or_create_file_node` calls with the same path should produce the same `FileNode`.
- `get_or_create_file_node` calls with different paths should produce different `FileNode`s.

This works because `"hello.txt"` and `"world.txt"` are different paths, thus their `Eq` and `Hash` implementations ensure they get separate spots in the `file_to_node` hash map.

Test `test_file_mapping_panics` triggers the panic in `get_file_path` by creating a `FileNode` with a "fake store", and then using that rogue file node in another store.
While it is unlikely that we will make this mistake when using `Store`, it is good to confirm that this panics.

```admonish tip title="Rust Help: Testing Panics" collapsible=true
The `#[should_panic]` attribute makes the test succeed if it panics, and fail if it does not panic.
```

### Testing task mapping

Test the task mapping by inserting the following code into the `test` module (before the last `}`):

```rust,
{{#include j_test_task_mapping.rs:3:}}
```

We test this in the same way as the file mapping.
Again, this works because `StringConstant("Hello")` and `StringConstant("World")` are different due to their derived `Eq` and `Hash` implementations, which make them different due to the strings being different. 
Likewise, `StringConstant::new("Hello")` and `StringConstant::new("Hello")` are equal even if they are created with 2 separate invocations of `new`.

These (in)equalities might seem quite obvious, but it is important to keep in mind because incrementality can only work if we can identify equal tasks at a later time, so that we can check their dependencies and return their cached output when those dependencies are consistent.
Later on we will also see that this is important for soundness of the incremental build system.

### Testing task outputs

Test task outputs by inserting the following code into the `test` module:

```rust,
{{#include k_test_task_output.rs:3:}}
```

Test `test_task_outputs` ensures that:
- `task_has_output` only returns true if given task has an output, 
- and that `get_task_output` returns the output set by `set_task_output` for given task.

Test `test_get_task_output_panics` triggers a panic when we call `get_task_output` for a task that has no output, which is an invalid usage of `Store` that is more likely to happen than the other panics. 

### Testing dependencies

Test dependencies by inserting the following code into the `test` module:

```rust,
{{#include l_test_dependencies.rs:3:}}
```

The `test_dependencies` test is a bit more involved because it ensures that:
- `get_dependencies_of_task` returns the dependencies of given task. If the task has no dependencies, the iterator is empty. We test if an iterator is empty by getting the first element of the iterator with `.next()` and assert that it is `None`.
- `get_dependencies_of_task` returns the dependencies of given task in the order in which they were added, which will be important for soundness later. The graph library returns dependencies in insertion order.
- `add_task_require_dependency` adds a dependency to the correct task.
- creating a cycle with `add_task_require_dependency` results in it returning `Err(())`.

Note that the `StringConstant` task does not actually create file or task dependencies, but since `Store` never executes a task, we can pretend that it does in tests. 

### Testing task reset

Finally, test task reset by inserting the following code into the `test` module:

```rust,
{{#include m_test_reset.rs:3:}}
```

Here, we ensure that a task with an output and dependencies, does not have an output and dependencies after a reset, while leaving another task untouched.

Confirm that the store implementation works with `cargo test`.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/4_store/source.zip).
```
