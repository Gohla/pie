# Incremental Top-Down Context

In this chapter, we will implement an *incremental* build context.
An incremental context selectively executes tasks — only those that are affected by a change.
In other words, an incremental context executes the *minimum number of tasks* required to make all tasks up-to-date.

However, due to dynamic dependencies, this is not trivial.
We cannot first gather all tasks into a dependency tree and then topologically sort that, as dependencies are added and removed *while tasks are executing*.
To do incremental builds in the presence of dynamic dependencies, we need to check and execute affected tasks *one at a time*, updating the dependency graph, while tasks are executing.
To achieve this, we will employ a technique called *top-down incremental building*, where we start checking if a top (root) task needs to be executed, and recursively check whether dependent tasks should be executed until we reach the bottom (leaf) task(s), akin to a depth-first search.

Furthermore, build systems almost always interact with the file system in some way. 
For example, tasks read configuration and source files, or write intermediate and binary files.
Thus, a change in a file can affect a task that reads it, and executing a task can result in writing to new or existing files.
Therefore, we will also keep track of *file dependencies*.
Like task dependencies, file dependencies are also tracked dynamically while tasks are executing.

There are several ways to check if a file dependency is consistent (i.e., has not changed), such as checking the last modification date, or comparing a hash.
To make this configurable on a per-dependency basis, we will implement *stamps*.
A file stamp is just a value that is produced from a file, such as the modification date or hash, that is stored with the file dependency.
To check if a file dependency is consistent, we just stamp the file again and compare it with the stored stamp.

Similarly, we can employ stamps for task dependencies as well by stamping the output of a task.

First, we will start by adding file dependencies.

## Adding File Dependencies

To support file dependencies, add a method to the `Context` trait in `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/0_require_file/a_context.rs.diff:4:}}
```

`require_file` is similar to requiring a task, but instead takes a `path` to a file or directory on the filesystem as input.
We use `AsRef<Path>` as the type for the path, so that we can pass anything in that can dereference to a path.
For example, `str` has an `AsRef<Path>` implementation, so we can just use `"test.txt"` as a path.

As an output, we return `Result<Option<File>, io::Error>`, with `File` being a handle to an open file.
The reason for this complicated type is:

* An incremental context will want to read the metadata (such as the last modified date) of the file, or create a hash over the file, to be able to detect changes. Because getting metadata or reading the file can fail, and we want to propagate this error, we return a `Result` with `io::Error` as the error type.
* Tasks can create a dependency to a file that does not exist, and the existence of that file affects the task. For example, a task that prints true or false based on if a file exists. If the file does not exist (or it is a directory), we cannot open it, so we cannot return a `File`, hence we use `Option<File>` to return `None`.
* Otherwise, we return `Ok(Some(file))` so that the task can read the opened file.

```admonish info title="Rust Help" collapsible=true
[Recoverable error handling](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html) in Rust is done with the `Result<T, E>` type, which can either be `Ok(t)` or `Err(e)`.
In contrast to many languages which use exceptions, throwing, and exception handling; Rust treats recoverable errors just as regular values.

Similarly, [optional values](https://doc.rust-lang.org/std/option/) in Rust are defined using the `Option<T>` type, which can either be `Some(t)` or `None`.

Rust has many traits for converting values or references into others, which provides a lot of convenience in what would otherwise require a lot of explicit conversions.
[`AsRef<T>`](https://doc.rust-lang.org/std/convert/trait.AsRef.html) is such a conversion trait, that can convert itself into `&T`. 
Here, we use `AsRef<Path>` as a generic with a trait bound to support many different kinds of values to the `path` argument in `require_file`.
For example, we can call `context.require_file("test.txt")` because `str`, which is the type of string constants, [implements `AsRef<Path>`](https://doc.rust-lang.org/src/std/path.rs.html#3136-3141).
You can also see this as a kind of method overloading, without having to provide concrete overloads for all supported types.
```

Now we need to implement this method for `NonIncrementalContext`.
However, because we will be performing similar file system operations in the incremental context as well, we will create some utility functions for this first.

Add the `fs` module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/0_require_file/b_fs_module.rs.diff:4:}}
```

Create file `pie/src/fs.rs` with:

```rust,
{{#include 0_require_file/c_fs.rs}}
```

The `metadata` function gets the filesystem metadata given a path, and `open_if_file` opens the file for given path.
The reason for these functions is that the standard library function `std::fs::metadata` treats non-existent files as an error, whereas we don't want to treat it as an error and just return `None`.
Furthermore, `open_if_file` works around an issue where opening a directory on Windows (and possibly other operating systems) is an error, where we want to treat it as `None` again.
The documentation comments explain the exact behaviour.

```admonish info title="Rust Help" collapsible=true
The `?` operator makes it easy to [propgate errors](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator).
Because errors are just values in Rust, to propgate an error, you'd normally have to match each result and manually propagate the error.
The `r?` operator applied to a `Result` `r` does this for you, it basically desugars to something like `match r { Err(e) => return Err(e), _ => {} }`.

Comments with three forward slashes `///` are [documentation comments](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments) that document the function/struct/enum/trait/etc. they are applied to.
```

We will write some tests to confirm the behaviour, but for that we need utilities to create temporary files and directories.
Furthermore, we will be writing more unit tests, integration tests, and even benchmarks in this tutorial, so we will set up these utilities in such a way that they are reachable by all these use cases.
The only way to do that in Rust right now, is to create a separate crate and have the `pie` crate depend on it.

And yes, we went from adding file dependencies, to creating file system utilities, to testing those file system utilities, to creating testing utilities, and now to making a crate for those testing utilities.
We will start unwinding this stack soon!

Next to the `pie` directory, create a directory named `dev_shared`.
Create the `dev_shared/Cargo.toml` file with the following contents:

```toml,
{{#include 0_require_file/d_dev_shared_Cargo.toml}}
```

We've added the `tempfile` dependency here already, which is a crate that creates and automatically cleans up temporary files and directories.

```admonish info title="Rust Help" collapsible=true
We use other libraries (crates) by [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).
Because basically every Rust library adheres to [semantic versioning](https://semver.org/), we can use `"3"` as a version requirement which indicates that we will use the most up-to-date `3.x.x` version.
```

Create the main library file `dev_shared/src/lib.rs`, with functions for creating temporary files and directories:

```rust,
{{#include 0_require_file/e_dev_shared_lib.rs}}
```

Your directory structure should now look like this:

```
{{#include ../../gen/2_top_down/0_require_file/e_dir.txt:2:}}
```

To access these utility functions in the `pie` crate, add a dependency to `dev_shared` in `pie/Cargo.toml`:

```toml,customdiff,
{{#include ../../gen/2_top_down/0_require_file/f_Cargo.toml.diff:4:}}
```

Note that this is dependency is added under `dev-dependencies`, indicating that this dependency is only available when running tests, benchmarks, and examples.
Therefore, users of our library will not depend on this library, which is good, because temporary file management is not necessary to users of our library.

Back to testing our filesystem utilities.
Add the following tests to `pie/src/fs.rs`:

```rust,
{{#include 0_require_file/g_fs_test.rs}}
```

We test whether the functions conform to the specified behaviour.
Unfortunately, we can't easily test when `metadata` and `open_if_file` should return an error, because we cannot disable read permissions on files via the Rust standard library.

We use our `create_temp_file` and `create_temp_dir` utility functions to create temporary files and directories.
The `tempfile` library takes care of deleting temporary files when they go out of scope (at the end of the test).

Now we are done unwinding our stack and have filesystem and testing utilities.
Make the non-incremental context compatible by changing `pie/src/context/non_incremental.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/0_require_file/h_non_incremental_context.rs.diff:4:}}
```

Since the non-incremental context does not track anything, we simply try to open the file and return it.
This implements the description we made earlier:

* If opening the file results in an error, the `?` operator returns `Err(...)` immediately.
* If the file does not exist or is a directory, `open_if_file` returns `None` and `file` is `None`.
* Otherwise, `file` is `Some(file)`.

Confirm everything works with `cargo test`.

## Implementing the Incremental Context

Now we get to the fun part, incrementality!

To check whether we need to execute a task, we need to check the dependencies of that task to see if any of them are not consistent.
A dependency is consistent if after stamping, the new stamp equals the stored stamp.
If all dependencies of the task are consistent, we return the cached output of the task.
If not, we execute the task.

To implement this, we will need several components:
- `FileStamper` and `FileStamp` types for stamping files.
- `OutputStamper` and `OutputStamp` types for stamping task outputs.
- Extension to `Context` to support passing `FileStamper` and `OutputStamper` when requiring files and tasks.
- A `FileDependency` type that holds a `FileStamper` and `FileStamp` to check whether a file is consistent.
- A `TaskDependency` type that holds an `OutputStamper` and `OutputStamp` to check whether a task is consistent.
- A `Dependency` type that merges `FileDependency` and `TaskDependency` so we can check whether a dependency is consistent without having to know what kind of dependency it is.
- A `Store` type which holds the dependency graph with methods for mutating and querying the graph, using `Dependency` to represent dependencies.
- A `TopDownContext` type that implements `Context` in an incremental way, using `Store`.

We will start with implementing stamps and dependencies, as those can be implemented as a stand-alone part.

### Stamp implementation

Add the `stamp` module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/1_stamp/a_module.rs.diff:4:}}
```

This module is public as users of the library should be able to construct stampers.

#### File stamps

Create the `pie/src/stamp.rs` file and add:

```rust,
{{#include 1_stamp/b_file.rs}}
```

We're implementing `FileStamper` as an enum for simplicity.

A `FileStamper` has a single method `stamp` which takes something that can be dereferenced to a path, and produces a `FileStamp` or an error if creating the stamp failed.
For now, we implement only two kinds of file stampers: `Exists` and `Modified`.
The `Exists` stamper just returns a boolean indicating whether a file exists.
It can be used to create a file dependency where a task behaves differently based on whether a file exists or not.
The `Modified` stamper returns the last modification date if the file exists, or `None` if the file does not exist.

We derive `Eq` for stamps so that we can compare them.
Equal stamps indicate a consistent dependency, unequal indicates inconsistent.
We also derive `Eq` for stampers, because the stamper of a dependency could change, making the dependency inconsistent.

#### Task output stamps

We implement task output stampers in a similar way.
Add to `pie/src/stamp.rs`:

```rust,
{{#include 1_stamp/c_output.rs}}
```

The `Inconsequential` stamper simply ignores the output and always returns the same stamp (thus is always equal).
It can be used to create a task dependency where we are interested in some side effect of a task, but don't care about its output.
The `Equals` stamper simply wraps the output of a task, so the stamp is equal when the output is equal.

Output stamps are generic over the task output type `O`.

```admonish info title="Trait bounds and derive macros" collapsible=true
Because `O` is used in the enum, the `derive` attributes on `OutputStamp` create bounds over `O`.
Thus, `OutputStamp` is only `Clone` when `O` is `Clone`, `OutputStamp` is only `Clone` when `O` is `Clone`, and so forth.
Because we declared `Task::Output` with bound `Clone + Eq + Debug`, we can be sure that `OutputStamp` is always `Clone`, `Eq`, and `Debug`.
```

```admonish info title="User-defined stamps" collapsible=true
`FileStamper` and `OutputStamper` could also be a trait which would allow users of the library to implement their own stampers.
For simplicity, we do not explore that option in this tutorial.
If you feel adventurous, you could try to implement this after you've finished the tutorial.
Do note that this introduces a lot of extra generics and trait bounds everywhere, which can be a bit cumbersome.
```

#### Tests

Finally, we write some tests.
Add to `pie/src/stamp.rs`:

```rust,
{{#include 1_stamp/d_test.rs}}
```

We test file stamps by creating a stamp, changing the file, creating a new stamp, and then compare the stamps.
We test task output stamps by just passing a different output value to the `stamp` function, and then compare the stamps.

Run `cargo test` to confirm the stamp implementation.

### Stamps in Context

We now have a module dedicated to stamps.
However, stampers are constructed by users of the library that author tasks, and they need to pass in these stampers when creating dependencies.
Therefore, we need to update the `Context` trait to allow passing in these stampers.

Change `Context` in `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/2_stamp_context/a1_context.rs.diff:4:}}
```

We add the `require_file_with_stamper` method which allow passing in a stamper.
We add a default implementation for `require_file` that passes in a default stamper.
The default is provided by `default_require_file_stamper` which can be overridden by context implementations.

Now apply the same to tasks, changing `Context` again in `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/2_stamp_context/a2_context.rs.diff:4:}}
```

Update `NonIncrementalContext` in `src/context/non_incremental.rs` to implement the new methods:

```rust,customdiff
{{#include ../../gen/2_top_down/2_stamp_context/b_non_incremental_context.rs.diff:4:}}
```

We just ignore the stampers in `NonIncrementalContext`, as they are only needed for incrementality.

Run `cargo test` to confirm everything still works.

### Dependency implementation

Add the `dependency` module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/3_dependency/a_module.rs.diff:4:}}
```

This module is private, as users of the library should not construct dependencies.
They should only create stampers, which are passed to dependencies via the `Context`.

#### File dependencies

Create the `pie/src/dependency.rs` file and add:

```rust,
{{#include 3_dependency/b_file.rs}}
```

A `FileDependency` stores the `path` the dependency is about, the `stamper` used to create a stamp for this dependency, and the `stamp` that was created at the time the file dependency was made.
The `FileDependency::new_with_file` function also returns the opened file if it exists, so that users of this function can read from the file without having to open it again.

A file dependency is inconsistent when the stored stamp is not equal to a stamp that we create at the time of checking, implemented in `FileDependency::is_inconsistent`.
For example, if we created a file dependency (with modified stamper) for a file that was modified yesterday, then modify the file, and then call `is_inconsistent` on the file dependency, it would return `Some(new_stamp)` indicating that the dependency is inconsistent.

We implement an `is_inconsistent` method here instead of an `is_consistent` method, so that we can return the changed stamp when the dependency is inconsistent, which we will use for debug logging purposes later.

Creating and checking a file dependency can fail due to file operations failing (for example, cannot access the file), so we propagate those errors.

#### Task dependencies

Task dependencies are implemented in a similar way.
Add to `pie/src/dependency.rs`:

```rust,
{{#include 3_dependency/c_task.rs}}
```

A `TaskDependency` stores the `task` the dependency is about, along with its `stamper` and `stamp` that is created when the dependency is created.
Task dependencies are generic over the type of tasks `T`, and their type of outputs `O`.

```admonish info title="Trait bounds on structs" collapsible=true
We chose not to put a `Task` trait bound on `TaskDependency`, and instead put the bound on the impl.
There are several up and downsides that should be considered when making such a decision.

The main upside for putting the `Task` bound on the `TaskDependency` struct, is that we can leave out `O` and use `OutputStamp<T::Output>` as the type of the `stamp` field.
This cuts down a generic parameter, which reduces boilerplate.
The downside is that we need to then put the `Task` bound on every struct that uses `TaskDependency`, which increases boilerplate.

In this case, we chose not to put the trait bound on the struct to prevent that trait bound from bubbling up into other structs that use `TaskDependency`, as it would need to appear in almost every struct in the library.
```

A task dependency is inconsistent if, after recursively checking it, its stamp has changed, implemented in `TaskDependency::is_inconsistent`.
Usually, this will be using the `Equals` task output stamper, so a task dependency is usually inconsistent when the output of the task changes.

Because we need to recursively check the task, `TaskDependency::is_inconsistent` requires a context to be passed in.

#### Dependency enum

Finally, we create a `Dependency` enum that abstracts over these two kinds of dependencies.
Add to `pie/src/dependency.rs`:

```rust,
{{#include 3_dependency/d_dependency.rs}}
```

`Dependency` just merges the two kinds of dependencies and provides an `is_inconsistent` method that calls the corresponding method.
This will make it easier to write a dependency graph implementation later.

We return the changed stamp here as well for debug logging later.
We wrap the changed stamp in an `InconsistentDependency` enum, and map to the correct variant if there is an inconsistency.

Because `Dependency` can store a `TaskDependency`, we need to propagate the `T` and `O` generics.
Likewise, `InconsistentDependency` propagates the `O` generic for `OutputStamp`.

```admonish info title="User-defined dependencies" collapsible=true
Like with stampers, `Dependency` could also be a trait to allow users of the library to define their own dependencies.
However, as we will see later, these dynamic dependencies also require validation, and I am unsure how such a `Dependency` trait should look.
Therefore, we don't have an appendix on how to implement this.
But, if you have an idea on how to this nicely (after you've completed this tutorial), please get in touch! 
```

#### Tests

As usual, we write some tests to confirm the behaviour.
Add tests to `pie/src/dependency.rs`:

```rust,
{{#include 3_dependency/e_test.rs}}
```

We test a file dependency by asserting that `is_inconsistent` returns `Some` after changing the file.

Testing task dependencies requires a bit more work.
We create task `ReadStringFromFile` that reads a string from a file, and then returns that string as output.
We require the task to get its output (`"test1"`), and create a task dependency with it.
Then, we change the file and check consistency of the task dependency.
That recursively requires the task, the context will execute the task, and the task now returns (`"test2"`).
Since we use the `Equals` output stamper, and `"test1"` does not equal `"test2"`, the dependency is inconsistent and returns a stamp containing `"test2"`.

Note that we are both testing the specific dependencies (`FileDependency` and `TaskDependency`), and the general `Dependency`.

```admonish
Normally, a task such as `ReadStringFromFile` shound return a `Result<String, io::Error>`, but for testing purposes we are just using panics with `expect`.

In the file dependency case, using `Dependency` requires an explicit type annotation because there is no task to infer the type from.
We just use `Dependency<ReadStringFromFile, String>` as the type, and this is fine even though we don't use `ReadStringFromFile` in that test, because the `Dependency::RequireFile` variant does not use those types. 
```

Run `cargo test` to confirm everything still works.
You will get some warnings about unused things, but that is ok as we will use them in the next section.

### Store implementation

To do incremental building, we need to keep track of all files, tasks, their dependencies, and task outputs.
This will be the responsibility of the `Store` data structure.
The `TopDownContext` and future context implementations will use methods on `Store` to request and update this data.
In other words, `Store` encapsulates this data.

Basically, `Store` will be a dependency graph.
However, writing a dependency graph data structure is outside of the scope of this tutorial, so we will be using the `pie_graph` library which we prepared exactly for this use case.
The graph from this library is a directed acyclic graph (DAG), meaning that edges are directed and there may be no cycles in edges, as that would prohibit topological orderings.

```admonish info title="Graph library" collapsible=true
The `pie_graph` library is a modified version of the great [`incremental-topo`](https://github.com/declanvk/incremental-topo/) library which implements incremental topological ordering: it keeps the topological ordering up-to-date incrementally while nodes and edges are added and removed.
That is exactly what we need, as dynamic dependencies prevents us from calculating the topological ordering in one go, and calculating the topological ordering after every task execution is prohibitively expensive.
The implementation in the `incremental-topo` library is based on a [paper by D. J. Pearce and P. H. J. Kelly](http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf) that describes several dynamic topological sort algorithms for directed acyclic graphs.
```

Add the `pie_graph` dependency to `pie/Cargo.toml`:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/a_Cargo.toml.diff:4:}}
```

#### Store basics

Add the `store` module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/b_module.rs.diff:4:}}
```

This module is private, as users of the library should not interact with the store.
Only `Context` implementations will use the store.

Create the `pie/src/store.rs` file and add the following to get started:

```rust,
{{#include 4_store/c_basic.rs}}
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
We also add a `new` function to initialize the store, which right now is the same as `default`, but will get a more specific meaning later.

```admonish info title="Deriving default" collapsible=true
We cannot derive this `Default` implementation even though it seems we should be able to, because the derived implementation will require `T` and `O` to be `Default`, and this is not always the case.
This is because the `Default` derive macro is conservative and adds a `: Default` bound to *every* generic argument in the `Default` trait implementation, and there is no way to disable this behaviour.
Therefore, we implement `Default` ourselves.

There are several crates that have more configurable derive macros for these things, but adding an extra dependency to generate a few lines of code is not worth the extra compilation time, so we just implement it manually here.
```

#### Graph nodes

A node in `DAG` is represented by a `Node`, which is a transparent identifier (sometimes called a [handle](https://en.wikipedia.org/wiki/Handle_(computing))) that points to the node and its data.
We can create nodes in the graph, and then query attached data (`NodeData`) given a node.
So `DAG` allows us to go from `Node` to a `PathBuf` and task `T` through attached `NodeData`.

However, we want each unique file and task to be represented by a single unique node in the graph.
We need this for incrementality so that if the build system encounters the same task twice, we can find the corresponding task node in the graph the second time, check if it is consistent, and return its output if it is.

To ensure unique nodes, we need to maintain the reverse mapping from `PathBuf` and `T` to `Node` ourselves, which we will do with `HashMap`s.
This is also the reason for the `Eq` and `Hash` trait bounds on the `Task` trait, so we can use them as keys in `HashMap`s.

Change `pie/src/store.rs` to add hash maps to map between these things:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/d1_mapping_diff.rs.diff:4:}}
```

To prevent accidentally using a file node as a task node, and vice versa, change `pie/src/store.rs` to add specific types of nodes:

```rust,customdiff
{{#include ../../gen/2_top_down/4_store/d2_mapping_diff.rs.diff:4:}}
```

The `FileNode` and `TaskNode` types are [newtypes](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html) that wrap a `Node` into a specific type of node.
The `Borrow` implementations will make subsequent code a bit more concise by automatically converting `&FileNode` and `&TaskNode`s to `&Node`s.

```admonish info title="Newtypes" collapsible=true
Because the `Node`s inside the newtypes are not public, it is not possible to construct a `FileNode` or `TaskNode` outside of this module.
Therefore, if we only accept and create `FileNode` and `TaskNode` in the `Store` API, it is not possible to use the wrong kind of node.

The `Borrow` implementation does leak outside of this module, but not outside of this crate (library).
This is because the visibility of a trait implementation is the intersection of the visibilities of the trait and type it is implemented on.
`Borrow` is public, but `FileNode` and `TaskNode` are only public within this crate.
Thefore, modules of this crate can extract the `Node` out of `FileNode` and `TaskNode`.
However, that `Node` cannot be used to construct a `FileNode` or `TaskNode`, so it is not a problem.
```

Now we will add methods create nodes and to query their attached data.
Add the following code to `pie/src/store.rs`:

```rust,
{{#include 4_store/e_mapping.rs}}
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

```admonish info title="Triggering these panics" collapsible=true
Because only `Store` can create `FileNode`s and `TaskNode`s, and all methods only take these values as inputs, these panics will not happen under normal usage.
The only way to trigger these panics (in safe Rust) would be to create two stores, and use the nodes from one store in another.
However, since this is a private module, we just need to make sure that we don't do that.

There are some tricks to prevent even this kind of invalid usage.
For example, the [generativity](https://docs.rs/generativity/latest/generativity/) crate generates unique identifiers based on lifetimes.
However, that is a bit overkill, especially for an internal API, so we won't be using that.
```

We implement similar methods for task nodes in `get_or_create_task_node` and `get_task`.

#### Task outputs

When we do not need to execute a task because it is consistent, we still need to return its output.
Therefore, we store the task output in `NodeData::Task` and add methods to query and manipulate task outputs.
Add the following code to `pie/src/store.rs`:

```rust,
{{#include 4_store/f_output.rs}}
```

The `task_has_output`, `get_task_output`, and `set_task_output` methods manipulate task outputs in `NodeData::Task`.

Again, we are using panics here to indicate unrecoverable programming errors.

#### Dependencies

Now we need methods to query and manipulate dependencies.
The edges in the graph are dependencies between tasks and files.
Tasks can depend on other tasks and files, but there are no dependencies between files.
An edge does not have its own dedicated representation, and is simply represented by two nodes: the source node and the destination node of the edge.

Add the following code to `pie/src/store.rs`:

```rust,
{{#include 4_store/g_dependency.rs}}
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

#### Resetting tasks

Finally, when we determine that a task is inconsistent and needs to be executed, we first need to remove its output and remove its outgoing dependencies, as those will interfere with incrementality when not removed.
We do NOT want to remove incoming dependencies, as that would remove dependencies from other tasks to this task, which breaks incrementality, so we can't just remove and re-add the task to the graph.
Add the `reset_task` method that does this to `pie/src/store.rs`:

```rust,
{{#include 4_store/h_reset.rs}}
```

This will reset the task output back to `None`, and remove all outgoing edges (dependencies).

#### Tests

Now we've implemented everything we need for implementing the top-down context, but first we will write some tests.

##### Testing file mapping

Add the following code to `pie/src/store.rs` for testing the file mapping:

```rust,
{{#include 4_store/i_test_file_mapping.rs}}
```

We create a simple task `StringConstant` because we need a `Task` implementation to test `Store`, as `Store` is generic over a `Task` type.
We will never execute it because `Store` does not execute tasks.

Test `test_file_mapping` checks whether the file node mapping works as expected:
- `get_or_create_file_node` calls with the same path should produce the same `FileNode`.
- `get_or_create_file_node` calls with different paths should produce different `FileNode`s.

This works because `"hello.txt"` and `"world.txt"` are different paths, thus their `Eq` and `Hash` implementations ensure they get separate spots in the `file_to_node` hash map.

Test `test_file_mapping_panics` triggers the panic in `get_file_path` by creating a `FileNode` with a "fake store", and then using that rogue file node in another store.
While it is unlikely that we will make this mistake when using `Store`, it is good to confirm that this panics.

```admonish info title="Rust help" collapsible=true
The `#[should_panic]` attribute makes the test succeed if it panics, and fail if it does not panic.
```

##### Testing task mapping

Test the task mapping by inserting the following code into the `test` module (before the last `}`):

```rust,
{{#include 4_store/j_test_task_mapping.rs}}
```

We test this in the same way as the file mapping.
Again, this works because `StringConstant("Hello")` and `StringConstant("World")` are different due to their derived `Eq` and `Hash` implementations, which make them different due to the strings being different. 
Likewise, `StringConstant::new("Hello")` and `StringConstant::new("Hello")` are equal even if they are created with 2 separate invocations of `new`.

These (in)equalities might seem quite obvious, but it is important to keep in mind because incrementality can only work if we can identify equal tasks at a later time, so that we can check their dependencies and return their cached output when those dependencies are consistent.
Later on we will also see that this is important for soundness of the incremental build system.

##### Testing task outputs

Test task outputs by inserting the following code into the `test` module:

```rust,
{{#include 4_store/k_test_task_output.rs}}
```

Test `test_task_outputs` ensures that:
- `task_has_output` only returns true if given task has an output, 
- and that `get_task_output` returns the output set by `set_task_output` for given task.

Test `test_get_task_output_panics` triggers a panic when we call `get_task_output` for a task that has no output, which is an invalid usage of `Store` that is more likely to happen than the other panics. 

##### Testing dependencies

Test dependencies by inserting the following code into the `test` module:

```rust,
{{#include 4_store/l_test_dependencies.rs}}
```

The `test_dependencies` test is a bit more involved because it ensures that:
- `get_dependencies_of_task` returns the dependencies of given task. If the task has no dependencies, the iterator is empty. We test if an iterator is empty by getting the first element of the iterator with `.next()` and assert that it is `None`.
- `get_dependencies_of_task` returns the dependencies of given task in the order in which they were added, which will be important for soundness later. The graph library returns dependencies in insertion order.
- `add_task_require_dependency` adds a dependency to the correct task.
- creating a cycle with `add_task_require_dependency` results in it returning `Err(())`.

Note that the `StringConstant` task does not actually create file or task dependencies, but since `Store` never executes a task, we can pretend that it does in tests. 

##### Testing task reset

Finally, test task reset by inserting the following code into the `test` module:

```rust,
{{#include 4_store/m_test_reset.rs}}
```

Here, we ensure that a task with an output and dependencies, does not have an output and dependencies after a reset, while leaving another task untouched.

### Top-down context implementation

#### Top-down context basics

Add the `top_down` module to `pie/src/context/mod.rs`:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/a_module.rs.diff:4:}}
```

Create the `pie/src/context/top_down.rs` file and add the following to get started:

```rust,
{{#include 5_context/b_basic.rs}}
```

The `TopDownContext` type is generic over tasks `T` and their outputs `O`, owns a `Store`, and can be created using `default` or `new`.

`TopDownContext` implements `Context`, and the main challenge will be implementing the `require_file_with_stamper` and `require_task_with_stamper` methods *incrementally* and *correctly*.

#### Requiring files

Tasks such as `ReadStringFromFile` which we've used in tests before call `context.require_file` to declare that they depend on a file in the filesystem.
For incrementality, we need to add this dependency to the dependency graph.
This dependency will go from the *current executing task* to the file.
Therefore, we will need to keep track of the current executing task.
 
Change `pie/src/context/mod.rs` to add a field for tracking the current executing task, and use it in `require_file_with_stamper`:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/c_current.rs.diff:4:}}
```

We're not setting `current_executing_task` yet, as that is the responsibility of `require_task_with_stamper` which we will implement later.
In `require_file_with_stamper` we're now getting the current executing task.
If there is no current executing task, which only happens if a user directly calls `require_file` on a context, we don't make a dependency and just open the file.

Now we need to add the file dependency, change `pie/src/context/mod.rs` to do this: 

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/d_file.rs.diff:4:}}
```

We simply create or get an existing file node, create a file dependency, and add the file require dependency to the graph via `store`.
Errors are propagated to the caller, so they can react accordingly to filesystem operation failures.

Due to all the prerequisite work we've done, this was quite simple to implement!

#### Requiring tasks

To implement `require_task_with_stamper`, we need to check whether we should execute a task.
A task should be executed either if it's new (it does not have an output stored yet), or if at least one of its dependencies is inconsistent.
If we don't execute it, then it must have an output value and all its dependencies are consistent, so we just return its output value.

Change `pie/src/context/mod.rs` to implement this logic:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/e_task.rs.diff:4:}}
```

We first create or get an existing file node.
Then, we check whether the task should be executed with `should_execute_task` which we still need to implement.

If that returns true, we reset the task, set the current executing task, actually execute the task, restore the previous executing task, and set the task output.
Otherwise, we get the output of the task from the store, which cannot panic because `should_execute_task` ensures that the task has an output if it returns false.
Finally, we return the output.

We still need to create a task dependency. Change `pie/src/context/mod.rs` to add the dependency:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/f_task_dep.rs.diff:4:}}
```

If there is no current executing task, which occurs when a user requires the initial task, we skip creating a dependency.
Otherwise, we create a dependency and add it to the store.
However, creating a task dependency can create cycles, and we need to handle that error.

At this point, we need to make a hard decision about the API of our library.
`require_task_with_stamper` returns the task output, with no opportunity to return an error.
If we want to propagate this error, we'd need to change the `Context::require_task` API to return `Result<T::Output, CycleError>`.
However, because tasks call these methods on `Context`, we'd also need to change `Task::execute` to return `Result<T::Output, CycleError>`.
That would require all tasks to propagate these cycle errors every time they require another task.

Furthermore, some tasks want to return their own kinds of errors, where `T::Output` will be `Result<AnOutput, AnError>`.
In that case, the concrete return type would be `Result<Result<AnOutput, AnError>, CycleError>`, which is annoying to deal with.

On the other hand, we can panic when a cycle is found, which requires no changes to the API.
We do end up in a mostly unrecoverable state, so a panic is a valid option.
However, this is not ideal, because it means the build system can panic due to invalid task dependencies created by the user of the system.
Panics will (most of the time) stop the program, which can be annoying to deal with.

This is a hard trade-off to make.
Either we propagate errors which will not end the program but will introduce a lot of boilerplate and annoyance in task implementations.
Or we panic which will end the program but introduces no boilerplate.

In this tutorial, we will go with panics on cycles, because it results in a much simpler system.

```admonish info title="Recovering from panics" collapsible=true
Panics either abort the program (when panics are set to abort in `Cargo.toml`), or unwind the call stack and then end the program.

When panics abort, there is nothing we can do about it. 
A panic will immediately abort the program.
When panics unwind, the call stack is unwound, which still runs all destructors ([`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)), and this unwinding can be caught.

We can catch unwinding panics with [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html), which is a way to recover from panics.
This does require that the types used in the closure passed to `catch_unwind` are [unwind safe](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html).
This is because panics exit a function early, which can mess up some invariants of your code.
For example, a call to set a task output can be skipped when a panic occurs, breaking a code invariant.
Therefore, types such as `&mut T` are not unwind safe by default, because these invariants can break under panics.

Note that unwind safety is something different than the general safety guarantees provided by Rust: type-safe, memory-safe, thread-safe.
An unwind unsafe type is still type-safe, memory-safe, and thread-safe.

Unwind safety can be more easily achieved by using owned types which run destructors when the function call ends, which work under normal circumstances, but also when unwinding panics.

In the context of the PIE build system, if we panic on unrecoverable errors, but want to allow catching these panics, we need to think about unwind safety.
At any point we panic, we need to think about keeping the system in a valid state.

Another way to recover from panics is to run the panicking code on a different thread.
If the code panics, it will only end that thread, effectively allowing panic recovery.
However, this does require some form of thread-safety, beause you are moving a computation to a different thread.
Furthermore, some platforms do not have access to threads, such as WASM, where this approach would not work.

A final note is that care must be taken when [unwiding panics across foreign function interfaces (FFI)](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding).
```

#### Checking tasks

The final piece to our puzzle is the `should_execute_task` implementation.

Add the following code to `pie/src/context/mod.rs`:

```rust,customdiff,
{{#include ../../gen/2_top_down/5_context/g_check.rs.diff:4:}}
```

The premise of `should_execute_task` is simple: go over the dependencies of a task until `dependency.is_inconsistent` is true, at which we return true.
If all dependencies are consistent, then return true only if the task has no output.
Otherwise, return false.

However, there are some complications due to borrowing.
Checking if a task dependency is inconsistent requires recursive checking: `TaskDependency::is_inconsistent` requires a `&mut Context` to call `Context::require_task`, which in turn can require this method again. 
To that end, we pass `self` to `is_inconsistent`, because `self` is an instance of `TopDownContext` which implements `Context`.

In this method, `self` is `&mut self`, a mutable borrow.
Therefore, we cannot have *any other borrows* active while `is_inconsistent` is being called, because that would violate one of the safety mechanisms of Rust where mutable borrows are *exclusive*.
Getting the task's dependencies from the store requires a borrow, so we cannot hold onto that borrow.
We get around that here by cloning the dependencies and collecting them into a `Vec`.

We also document this fact in a comment to explain to readers (us in the future) why we do this cloning, preventing refactorings only to hit that same borrowing issue again. 

Cloning and collecting does have a performance overhead as we need to clone the dependencies and heap allocate a `Vec` to store them.
For this tutorial, that is fine, but in a real-world application we should minimize cloning if possible and look into reducing heap allocations.

```admonish info title="Reference counting" collapsible=true
Cloning a `Dependency` results in heap allocations, because cloning `FileDependency` clones a `PathBuf` which is a heap allocated string (basically a `Vec<u8>`), and cloning a `TaskDependency` clones the `Task`, which may require allocations as well.

One way to avoid heap allocations in both kinds of dependencies is to store the `PathBuf` and `Task` in a [reference-counting pointer `Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html).
Then, there will only be one heap allocated `PathBuf` and `Task`, and cloning just increments the reference count.
The upside is that this approach is easy to implement and reduces allocations.
The downside is that clones require incrementing the reference count, which is a write operation that does have a tiny bit of overhead.
In many cases, this overhead is smaller than cloning data when the data is large enough or requires heap allocations.
In our case, it would probably be worth doing this, but benchmarking is required to confirm this.

Note that instead of always wrapping tasks in a `Rc`, task authors could implement `Task` on `Rc<TheirTask>` instead.
Since `Rc` implements `Clone`, any time we `task.clone()`, we would just increase the reference count instead.

When working in a multi-threaded situation, you would use the thread-safe [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) instead. 
```

```admonish info title="String optimizations" collapsible=true
A technique for reducing allocations on strings (and string-like types such as `PathBuf`) is to apply [small string optimization](https://fasterthanli.me/articles/small-strings-in-rust), where small strings are stored inline instead of requiring a heap allocation.
This only works if the strings are usually small enough to fit inline on the stack (for example, 32 bytes).

Another technique for strings is string interning, where equal strings are stored in a central place and then re-used everywhere.
This technique is great when we use the same string a lot of times.
That may be a good strategy for a build system, where we work with the same file paths over and over.

There are several crates implementing these techniques, but I have not used one myself yet, so I cannot recommend one.
```

```admonish info title="Avoiding heap allocations from collecting into Vecs" collapsible=true
Collecting the elements of an iterator into a `Vec` requires heap allocations as `Vec` is allocated on the heap.
We can avoid or at least reduce the number of heap allocations by re-using the same `Vec` instead of creating a new one.
Instead of collecting, you would store the `Vec` in the struct, clear it, and then `extend` it with the iterator.

When you `clear` a `Vec`, it removes all the elements, but keeps the heap allocated space.
Only if you would add more elements than it has space for, another heap allocation would be required, which will happen less and less frequently when you keep reusing the same `Vec`.
The downside is that you are keeping this heap allocated space for as long as you keep reusing the same `Vec`, which could waste some memory, but usually this is not a big problem.
You could of course call `vec.shrink_to_fit()` after not using it for a while to free up this space.

However, we cannot apply this technique here, because if we store the `Vec` in `TopDownContext`, we would run into the same borrowing problem again.
This technique also requires that you have mutable access to the `Vec` in order to mutate it.

Both of these limitations can be overcome by using a [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html).
`Cell` allows mutation to its inner value in an immutable context.
The catch is that you *cannot get a reference to its inner value*, you can only `take` the value out, mutate it, and then `set` it back.
Unfortunately, even this technique cannot be fully applied to `should_execute_task`, because it is called recursively and therefore the `Cell` will be empty when we try to `take` the `Vec` out.

If we want to avoid heap allocations from collecting new `Vec`s in `should_execute_task`, we would need to come up with a creative solution.
But this is outside of the scope of even this extra information block, so we'll just leave it at that.
```

Finally, we need to do something with dependency checking failures.
We've ignored the case where `dependency.is_inconsistent` returns `Err`.
When dependency checking result in an error, we should store the error for the user to investigate, and assume the dependency is inconsistent.

Change `pie/src/context/mod.rs` to store dependency check errors and give users access to it:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/h_error_field.rs.diff:4:}}
```

And then change `pie/src/context/mod.rs` to store these errors:

```rust,customdiff
{{#include ../../gen/2_top_down/5_context/i_error_store.rs.diff:4:}}
```

It took us a while, but now we've implemented an incremental build system with dynamic dependencies!
Instead of throwing a party, we should first write tests to see if it is indeed incremental, but also to check that it is correct.

#### Tests
