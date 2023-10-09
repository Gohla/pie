# Dynamic Dependencies

Now that we've implemented stamps, we can implement dynamic dependencies and their consistency checking.
A dependency is inconsistent if after stamping, the new stamp is different from the old stamp.
Therefore, dependencies need to keep track of their stamper and their previous stamp.
To that end, we will implement the `FileDependency` and `TaskDependency` types with methods for consistency checking.
We will also implement a `Dependency` type that abstracts over `FileDependency` and `TaskDependency`, which we will need for the dependency graph implementation in the next chapter.

Add the `dependency` module to `pie/src/lib.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/3_dependency/a_module.rs.diff
```

Users of the library will not construct dependencies.
They will create dependencies (and choose stampers) via `Context` methods.
However, dependencies will be used in the public API for debug logging later, so we make the module public.

## File dependencies

Create the `pie/src/dependency.rs` file and add:

```rust,
{{#include b_file.rs}}
```

A `FileDependency` stores the `path` the dependency is about, the `stamper` used to create a stamp for this dependency, and the `stamp` that was created at the time the file dependency was made.
The `FileDependency::new_with_file` function also returns the opened file if it exists, so that users of this function can read from the file without having to open it again.
We add getter methods to get parts of the file dependency without allowing mutation.
Since we will use those getter methods later, we annotate them with `#[allow(dead_code)]` to disable unused warnings.

A file dependency is inconsistent when the stored stamp is not equal to a stamp that we create at the time of checking, implemented in `FileDependency::is_inconsistent`.
For example, if we created a file dependency (with modified stamper) for a file that was modified yesterday, then modify the file, and then call `is_inconsistent` on the file dependency, it would return `Some(new_stamp)` indicating that the dependency is inconsistent.

We implement an `is_inconsistent` method here instead of an `is_consistent` method, so that we can return the changed stamp when the dependency is inconsistent, which we will use for debug logging purposes later.

Creating and checking a file dependency can fail due to file operations failing (for example, cannot access the file), so we propagate those errors.

## Task dependencies

Task dependencies are implemented in a similar way.
Add to `pie/src/dependency.rs`:

```rust,
{{#include c_task.rs:3:}}
```

A `TaskDependency` stores the `task` the dependency is about, along with its `stamper` and `stamp` that is created when the dependency is created.
Task dependencies are generic over the type of tasks `T`, and their type of outputs `O`.
We also add immutable getters here.

```admonish question title="Why not a Trait Bound on TaskDependency?" collapsible=true
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
Again, there is more mutual recursion here.

This recursive consistency checking is one of the core ideas that make programmatic incremental build systems possible.
But why is this so important? Why do we need recursive checking?
Well, we want our build system to be *sound*, meaning that we must execute *all* tasks that are affected by a change.
When we *do not execute* a task that *is affected by a change*, we are *unsound*, and introduce an *incrementality bug*! 

Because of dynamic dependencies, a change in a leaf in the dependency tree may affect a task at the root.
For example, a compilation task depends on a task that reads a configuration file, which depends on the configuration file.
A change to a configuration file (leaf) affects a task that reads the configuration file, which in turn affects the compilation task (root).
Therefore, we need to recursively check the dependency tree in order to execute all tasks affected by changes.

A different way to think about this, is to think about the invariant of the dependency consistency checking.
The invariant is that a dependency is consistent if and only if the subtree of that dependency is consistent, and the dependency itself is consistent.
The easiest way to adhere to this invariant, is recursive checking.

A final note about recursive checking is that tasks can be executed during it, and executing task can lead to new dynamic dependencies.
However, recursive checking handles this without problems because these dependencies are created through the `Context`, which in turn will call `is_inconsistent` when needed.

## Dependency enum

Finally, we create a `Dependency` enum that abstracts over these two kinds of dependencies.
Add to `pie/src/dependency.rs`:

```rust,
{{#include d_dependency.rs:3:}}
```

`Dependency` just merges the two kinds of dependencies and provides an `is_inconsistent` method that calls the corresponding method.
We return the changed stamp here as well for debug logging later.
We wrap the changed stamp in an `Inconsistency` enum, and map to the correct variant if there is an inconsistency.

Because `Dependency` can store a `TaskDependency`, we need to propagate the `T` and `O` generics.
Likewise, `Inconsistency` propagates the `O` generic for `OutputStamp`.

```admonish question title="User-Defined Dependencies?" collapsible=true
Like with stampers, `Dependency` could also be a trait to allow users of the library to define their own dependencies.
However, there are two requirements that make it hard to define such a trait:

1) We can implement different `Context`s which treat some dependencies differently. 
For example, in the actual PIE library, we have a bottom-up context that schedules tasks from the bottom-up.
This bottom-up context treats file and task dependencies in a completely different way compared to the top-down context.
2) Dynamic dependencies also require validation to ensure correctness, which we will do later on in the tutorial.

It is currently unclear to me how to create a `Dependency` trait with these requirements in mind. 
```

## Tests

As usual, we write some tests to confirm the behaviour.
Add tests to `pie/src/dependency.rs`:

```rust,
{{#include e_test.rs:3:}}
```

We test a file dependency by asserting that `is_inconsistent` returns `Some` after changing the file.

Testing task dependencies requires a bit more work.
We create task `ReadStringFromFile` that reads a string from a file, and then returns that string as output.
We require the task to get its output (`"test1"`), and create a task dependency with it.
Then, we change the file and check consistency of the task dependency.
That recursively requires the task, the context will execute the task, and the task now returns (`"test2"`).
Since we use the `Equals` output stamper, and `"test1"` does not equal `"test2"`, the dependency is inconsistent and returns a stamp containing `"test2"`.

Note that we are both testing the specific dependencies (`FileDependency` and `TaskDependency`), and the general `Dependency`.

```admonish note
Normally, a task such as `ReadStringFromFile` shound return a `Result<String, io::Error>`, but for testing purposes we are just using panics with `expect`.

In the file dependency case, using `Dependency` requires an explicit type annotation because there is no task to infer the type from.
We just use `Dependency<ReadStringFromFile, String>` as the type, and this is fine even though we don't use `ReadStringFromFile` in that test, because the `Dependency::RequireFile` variant does not use those types. 
```

Run `cargo test` to confirm everything still works.
You will get some warnings about unused things, but that is ok as we will use them in the next section.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/3_dependency/source.zip).
```
