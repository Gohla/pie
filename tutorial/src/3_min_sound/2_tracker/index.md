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

`Tracker` has methods corresponding to events that happen during a build, such as a build starting or ending, requiring a file, requiring a task, and executing a task.
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

Make `Pie` and `Session` generic over `Tracker` by modifying `pie/src/lib.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/d_lib_tracker.rs.diff:4:}}
```

We use `A` as the generic argument for tracker types in the source code.
The `Pie` struct owns the tracker, similarly to how it owns the store.

```admonish info title="Default type" collapsible=true
We assign `NoopTracker` as the default type for trackers in `Pie`, so that no tracking is performed when we use the `Pie` type without an explicit tracker type.
The `Default` implementation only works with `NoopTracker`, because we `impl Default for Pie<T, T::Output>`, which is equivalent to `impl Default for Pie<T, T::Output, NoopTracker>` due to the default type.
```

We make `Session` generic over trackers, and mutibly borrow the tracker from `Pie`, again like we do with the store.

Now we make `TopDownContext` generic over `Tracker`, and insert calls to tracker methods.
Modify `pie/src/context/top_down.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/e_top_down_tracker.rs.diff:4:}}
```

We make `TopDownContext` generic over trackers, and call methods on the tracker:

- `build_start`/`build_end` in `require_initial` to track build start and ends,
- `required_file` in `require_file_with_stamper` to track file dependencies,
- `require_task`/`required_task` in `require_file_with_stamper` to track task dependencies,
- `execute`/`executed` in `require_task_with_stamper` to track task execution start and ends.

In `require_file_with_stamper`, we also extract `should_execute` into a variable, and pull `dependency` out of the `if`, so that we can pass the required data to `tracker.required_task`.

Check that the code compiles with `cargo test`.
Existing code should keep working due to the `NoopTracker` default type in `Pie`.

We won't modify `NonIncrementalContext` to use a tracker, as `NonIncrementalContext` has no state, so we cannot pass a tracker to it.

## Implement writing tracker

Now we can implement some interesting trackers.
We start with a simple `WritingTracker` that writes build events to some writer.

Add the `writing` module to `pie/src/tracker/mod.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/f_mod_writing.rs.diff:4:}}
```

Then create the `pie/src/tracker/writing.rs` file and add:

```rust,
{{#include g_writing.rs}}
```

The `WritingTracker` is generic over a writer `W` that must implement `Write`, which is a standard trait for writing bytes to something.
`with_stdout` and `with_stderr` can be used to create buffered writers to standard output and standard error.
`new` can be used to create a writer to anything that implements `Write`, such as a `File`.

Add the `Tracker` implementation to `pie/src/tracker/writing.rs`:

```rust,
{{#include h_writing_impl.rs:2:}}
```

We implement 3 tracker methods that write when:
- ✓: a task is required but was not executed (i.e., consistent),
- →: a task starts to execute,
- ←: when the task is done executing.

The text to write is formatted with `format_args!`, which is passed into `writeln` using `std::fmt::Arguments` for flexibility.
We `flush` the writer after every event to ensure that bytes are written out.
When a task starts to execute, we increase indentation to signify the recursive checking/execution.
When a task is done executing, we decrease the indentation again.

```admonish info title="Saturating arithmetic" collapsible=true
We use `saturating_add` and `saturating_sub` for safety, which are saturating arithmetic operations that saturate at the numeric bounds instead of overflowing.
For example, `0u32.saturating_sub(1)` will result in `0` instead of overflowing into `4294967295`.

These saturating operations are not really needed when calls to `indent` and `unindent` are balanced.
However, if we make a mistake, it is better to write no indentation than to write 4294967295 spaces of indentation.

Alternatively, we could use standard arithmetic operations, which panic on overflow in debug/development mode, but silently overflow in release mode.
```

```admonish info title="Failing writes" collapsible=true
Writes can fail, but we silently ignore them in this tutorial (with `let _ = ...`) for simplicity.
You could panic when writing fails, but panicking when writing to standard output fails is probably going a bit too far.
You could store the latest write error and give access to it, which at least allows users of `WritingTracker` check for some errors.

In general, tracking events can fail, but the current `Tracker` API does not allow for propagating these errors with `Result`.
This in turn because `TopDownContext` does not return `Result` for `require_task` due to the trade-offs discussed in the section on `TopDownContext`.
```

If you want, you can capture more build events and write them, and/or provide more configuration as to what build events should be written.
But in this tutorial, we will keep it simple like this.

Check that the code compiles with `cargo test`.

Let's try out our writing tracker in the incrementality example by modifying `pie/examples/incremental.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/i_writing_example.rs.diff:4:}}
```

We remove the `println!` statements from tasks and create `Pie` with `WritingTracker`.
Now run the example with `cargo run --example incremental`, and you should see the writing tracker print consistent tasks and task executions to standard output.

## Implement event tracker

The writing tracker is great for debugging purposes, but we cannot use it to check whether our build system is incremental and sound.
To check incrementality and soundness, we need to be able to check whether a task has executed or not, and check the order of build events.
Therefore, we will implement the `EventTracker` that stores build events for later inspection.

Add the `event` module to `pie/src/tracker/mod.rs`:

```rust,customdiff
{{#include ../../gen/3_min_sound/2_tracker/j_mod_event.rs.diff:4:}}
```

Then create the `pie/src/tracker/event.rs` file and add:

```rust,
{{#include k_event.rs}}
```

The `EventTracker` stores build events in a `Vec`.
The `Event` enumeration mimics the `Tracker` methods, but has all arguments in owned form (for example `task: T` instead of `task: &T`) as we want to store these events.

Add the tracker implementation to `pie/src/tracker/event.rs`:

```rust,
{{#include l_event_tracker.rs:2:}}
```

We implement the relevant methods from `Tracker` and store the build events as `Event` instances in `self.events`.
When a new build starts, we clear the events.

TODO: add methods to access events

## Implement composite tracker

Currently, we cannot use both `EventTracker` and `WritingTracker` at the same time.
We want this so that we can check incrementality and soundness, but also look at standard output for debugging, at the same time.
Therefore, we will implement a `CompositeTracker` that forwards build events to 2 trackers.

TODO: implement composite tracker
