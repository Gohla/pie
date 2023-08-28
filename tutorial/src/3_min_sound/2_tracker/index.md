# Tracking Build Events

So far we have had no convenient way to inspect what our build system is doing, apart from `println!` debugging or attaching a debugger to the program.
In this section, we will change that by tracking build events for debugging and integration testing purposes.

We will:
1) Create a `Tracker` trait that receives build events through method calls. The `Tracker` trait can be implemented in different ways to handle build events in different ways.
2) Implement a `NoopTracker` that does nothing, removing the tracking overhead.
3) Make the build system generic over `Tracker`, such that `Context` implementations call methods on the tracker to create build events.
4) Implement a `WritingTracker` that writes build events to standard output or standard error, for debugging purposes.
5) Implement an `EventTracker` that stores build events for later inspection, for integration testing purposes.
6) Implement a `CompositeTracker` that forwards build events to 2 other trackers, so we can use multiple trackers at the same time.

## `Tracker` trait

Add the `tracker` module to `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/a_lib_module.rs.diff:4:}}
```

Then create the `pie/src/tracker` directory, create the `pie/src/tracker/mod.rs` file, and add the following content:

```rust,
{{#include b_tracker.rs}}
```

The `Tracker` trait is generic over `Task`.

```admonish
Here, we chose to put the `Task` constraint on the trait itself.
This will not lead to cascading constraints, as the `Tracker` trait will only be used as a constraint in `impl`s, not in structs or other traits.
```

`Tracker` has methods corresponding to events that happen during a build, such as requiring a file, requiring a task, and executing a task.
These methods accept `&mut self` so that tracker implementations can perform mutation, such as storing a build event.
We provide default methods that do nothing so that implementors of `Tracker` only have to override the methods for events they are interested in.
We use `#[allow(unused_variables)]` on the trait to not give warnings for unused variables, as all variables are unused due to the empty default implementations.

```admonish info title="Default methods" collapsible=true
Adding a method to `Tracker` with a default implementation ensures that implementations of `Tracker` do not have to be changed to work with the new method.
This is both good and bad.
Good because we can add methods without breaking compatibility.
Bad because we can forget to handle a new method, which can lead to problems with for example a composite tracker that forwards events to 2 trackers.
In this tutorial we chose the convenient option, but be sure to think about these kind of tradeoffs yourself!
```

Check that the code compiles with `cargo test`.

## No-op tracker

Add a no-op tracker, which is a tracker that does nothing, by adding the following code to `pie/src/tracker/mod.rs`:

```rust,
{{#include c_noop.rs:2:}}
```

Due to the default methods that do nothing on `Tracker`, this implementation is extremely simple. 

```admonish info title="Removing tracker overhead" collapsible=true
We will use generics to select which tracker implementation to use.
Therefore, all calls to trackers are statically dispatched, and could be inlined.
Because `NoopTracker` only has empty methods, and those empty methods can be inlined, using `NoopTracker` will effectively remove all tracking code from your binary, thus removing the overhead of tracking if you don't want it.

In this tutorial, we do not annotate methods with [`#[inline]`](https://nnethercote.github.io/perf-book/inlining.html), meaning that the Rust compiler (and the LLVM backend) will make its own decisions on what to make inlineable and what not.
If you care about performance here, be sure to annotate those default empty methods with `#[inline]`.
```

## Using the `Tracker` trait

Now we will make the build system generic over `Tracker`, and insert `Tracker` calls in context implementations.

Make `Pie` and `Session` generic over `Tracker` by modifying `src/lib.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/d_lib_tracker.rs.diff:4:}}
```

TODO: explain
TODO: default type for generic

Make `TopDownContext` generic over `Tracker` and insert method calls in `src/context/top_down.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/e_top_down_tracker.rs.diff:4:}}
```

TODO: explain

Check that the code compiles with `cargo test`.

We won't modify `NonIncrementalContext` to use a tracker, as `NonIncrementalContext` has no state, so we cannot pass a tracker to it.

## Implement writing tracker

implement `WritingTracker`
test build

change example to use it
run example, see build events

## Implement event tracker

## Implement composite tracker
