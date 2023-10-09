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

```diff2html fromfile linebyline
../../gen/3_min_sound/2_tracker/a_lib_module.rs.diff
```

Then create the `pie/src/tracker` directory, create the `pie/src/tracker/mod.rs` file, and add the following content:

```rust,
{{#include b_tracker.rs}}
```

The `Tracker` trait is generic over `Task`.

```admonish note title="Trait Bound"
Here, we chose to put the `Task` trait bound on the trait itself.
This will not lead to cascading trait bounds, as the `Tracker` trait will only be used as a bound in `impl`s, not in structs or other traits.
```

`Tracker` has methods corresponding to events that happen during a build, such as a build starting, requiring a file, requiring a task, checking a dependency, and executing a task.
All but the `require_file` event have start and end variants to give trackers control over nesting these kind of events. 
Then end variants usually have more parameters as more info is available when something is has finished.

Tracker methods accept `&mut self` so that tracker implementations can perform mutation, such as storing a build event.
We provide default methods that do nothing so that implementors of `Tracker` only have to override the methods for events they are interested in.
We use `#[allow(unused_variables)]` on the trait to not give warnings for unused variables, as all variables are unused due to the empty default implementations.

```admonish tip title="Rust Help: References in Result and Option" collapsible=true
The `check_dependency_end` method accepts the inconsistency as `Result<Option<&Inconsistency<T::Output>>, &io::Error>`.
The reason we accept it like this is that many methods in `Result` and `Option` take `self`, not `&self`, and therefore cannot be called on `&Result<T, E>` and `&Option<T>`.

We can turn `&Result<T, E>` into `Result<&T, &E>` with [`as_ref`](https://doc.rust-lang.org/std/result/enum.Result.html#method.as_ref) (same for `Option`).
Since trackers always want to work with `Result<&T, &E>`, it makes more sense for the caller of the tracker method to call `as_ref` to turn their result into `Result<&T, &E>`.

The final reason to accept `Result<&T, &E>` is that if you have a `&T` or `&E`, you can easily construct a `Result<&T, &E>` with `Ok(&t)` and `Err(&e)`.
However, you _cannot_ construct a `&Result<T, E>` from `&T` or `&E`, so `Result<&T, &E>` is a more flexible type.
```

```admonish question title="Are these Default Methods Useful?" collapsible=true
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

```admonish tip title="Rust Help: Removing Tracker Overhead" collapsible=true
We will use generics to select which tracker implementation to use.
Therefore, all calls to trackers are statically dispatched, and could be inlined.
Because `NoopTracker` only has empty methods, and those empty methods can be inlined, using `NoopTracker` will effectively remove all tracking code from your binary, thus removing the overhead of tracking if you don't want it.

In this tutorial, we do not annotate methods with [`#[inline]`](https://nnethercote.github.io/perf-book/inlining.html), meaning that the Rust compiler (and the LLVM backend) will make its own decisions on what to make inlineable and what not.
If you care about performance here, be sure to annotate those default empty methods with `#[inline]`.
```

## Using the `Tracker` trait

Now we will make the build system generic over `Tracker`, and insert `Tracker` calls in context implementations.

Make `Pie` and `Session` generic over `Tracker` by modifying `pie/src/lib.rs`:

```diff2html fromfile
../../gen/3_min_sound/2_tracker/d_lib_tracker.rs.diff
```

We use `A` as the generic argument for tracker types in the source code.
The `Pie` struct owns the tracker, similarly to how it owns the store.
`Pie` can be created with a specific tracker with `with_tracker`, and provides access to the tracker with `tracker` and `tracker_mut`.

```admonish tip title="Rust Help: Default Type" collapsible=true
We assign `NoopTracker` as the [default type](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#default-generic-type-parameters-and-operator-overloading) for trackers in `Pie`, so that no tracking is performed when we use the `Pie` type without an explicit tracker type.
The `Default` implementation only works with `NoopTracker`, because we `impl Default for Pie<T, T::Output>`, which is equivalent to `impl Default for Pie<T, T::Output, NoopTracker>` due to the default type.
```

We make `Session` generic over trackers, and mutibly borrow the tracker from `Pie`, again like we do with the store.
For convenience, `Session` also provides access to the tracker with `tracker` and `tracker_mut`.

Now we make `TopDownContext` generic over `Tracker`, and insert calls to tracker methods.
Modify `pie/src/context/top_down.rs`:

```diff2html fromfile
../../gen/3_min_sound/2_tracker/e_top_down_tracker.rs.diff
```

We make `TopDownContext` generic over trackers, and call methods on the tracker:

- In `require_initial` we call `build_start`/`build_end` to track builds.
- In `require_file_with_stamper` we call `require_file_end` to track file dependencies.
- In `require_file_with_stamper` we call `require_task_start`/`require_task_end` to track task dependencies. 
  - We extract `should_execute` into a variable, and pull `dependency` out of the `if`, so that we can pass them to `tracker.required_task`.
  - We also call `execute_start`/`execute_end` to track execution.
- In `should_execute_task` we call `check_dependency_start`/`check_dependency_end` to track dependency checking.
  - We extract `inconsistency` into a variable, and convert it into the right type for `check_dependency_end`.

Check that the code compiles with `cargo test`.
Existing code should keep working due to the `NoopTracker` default type in `Pie`.

We won't modify `NonIncrementalContext` to use a tracker, as `NonIncrementalContext` has no state, so we cannot pass a tracker to it.

## Implement writing tracker

Now we can implement some interesting trackers.
We start with a simple `WritingTracker` that writes build events to some writer.

Add the `writing` module to `pie/src/tracker/mod.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/2_tracker/f_mod_writing.rs.diff
```

Then create the `pie/src/tracker/writing.rs` file and add:

```rust,
{{#include g_writing.rs}}
```

The `WritingTracker` is generic over a writer `W` that must implement `Write`, which is a standard trait for writing bytes to something.
`with_stdout` and `with_stderr` can be used to create buffered writers to standard output and standard error.
`new` can be used to create a writer to anything that implements `Write`, such as a `File`.

Add some utility functions for `WritingTracker` to `pie/src/tracker/writing.rs`: 

```rust,
{{#include h_1_writing_impl.rs:2:}}
```

`writeln` and `write` will mainly be used for writing text.
The text to write is passed into these methods using `std::fmt::Arguments` for flexibility, accepting the result of `format_args!`.
`WritingTracker` keeps track of `indentation` to show recursive dependency checking and execution, which is controlled with `indent` and `unindent`.
Since we are usually writing to buffers, we must `flush` them to observe the output.

```admonish note title="Failing Writes" collapsible=true
Writes can fail, but we silently ignore them in this tutorial (with `let _ = ...`) for simplicity.
You could panic when writing fails, but panicking when writing to standard output fails is probably going a bit too far.
You could store the latest write error and give access to it, which at least allows users of `WritingTracker` check for some errors.

In general, tracking events can fail, but the current `Tracker` API does not allow for propagating these errors with `Result`.
This in turn because `TopDownContext` does not return `Result` for `require_task` due to the trade-offs discussed in the section on `TopDownContext`.
```

```admonish tip title="Rust Help: Saturating Arithmetic" collapsible=true
We use [`saturating_add`](https://doc.rust-lang.org/stable/std/primitive.u32.html#method.saturating_add) and [`saturating_sub`](https://doc.rust-lang.org/stable/std/primitive.u32.html#method.saturating_sub) for safety, which are saturating arithmetic operations that saturate at the numeric bounds instead of overflowing.
For example, `0u32.saturating_sub(1)` will result in `0` instead of overflowing into `4294967295`.

These saturating operations are not really needed when calls to `indent` and `unindent` are balanced.
However, if we make a mistake, it is better to write no indentation than to write 4294967295 spaces of indentation.

Alternatively, we could use standard arithmetic operations, which panic on overflow in debug/development mode, but silently overflow in release mode.
```

Now we can implement the tracker using these utility methods.
Add the `Tracker` implementation to `pie/src/tracker/writing.rs`:

```rust,
{{#include h_2_writing_impl.rs:2:}}
```

We implement most tracker methods and write what is happening, using some unicode symbols to signify events:
- `🏁`: end of a build,
- `-`: created a file dependency,
- `→`: start requiring a task,
- `←`: end of requiring a task,
- `?`: start checking a task dependency,
- `✓`: end of dependency checking, when the dependency is consistent,
- `✗`: end of dependency checking, when the dependency is inconsistent,
- `▶`: start of task execution,
- `◀`: end of task execution.

We `flush` the writer after every event to ensure that bytes are written out.
When a task is required, checked, or executed, we increase indentation to signify the recursive checking/execution.
When a task is done being required, checked, or executed, we decrease the indentation again.
In `check_dependency_end` we write the old and new stamps if a dependency is inconsistent.

This tracker is very verbose.
You can add configuration booleans to control what should be written, but in this tutorial we will keep it simple like this.

Check that the code compiles with `cargo test`.

Let's try out our writing tracker in the incrementality example by modifying `pie/examples/incremental.rs`:

```diff2html fromfile
../../gen/3_min_sound/2_tracker/i_writing_example.rs.diff
```

We remove the `println!` statements from tasks and create `Pie` with `WritingTracker`.
Now run the example with `cargo run --example incremental`, and you should see the writing tracker print to standard output:

```
{{#include ../../gen/3_min_sound/2_tracker/i_writing_example.txt}}
```

## Implement event tracker

The writing tracker is great for debugging purposes, but we cannot use it to check whether our build system is incremental and sound.
To check incrementality and soundness, we need to be able to check whether a task has executed or not, and check the order of build events.
Therefore, we will implement the `EventTracker` that stores build events for later inspection.

Add the `event` module to `pie/src/tracker/mod.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/2_tracker/j_mod_event.rs.diff
```

Then create the `pie/src/tracker/event.rs` file and add:

```rust,
{{#include k_event.rs}}
```

The `EventTracker` stores build events in a `Vec`.
The `Event` enumeration mimics the relevant `Tracker` methods, but uses structs with all arguments in owned form (for example `task: T` instead of `task: &T`) as we want to store these events.
We also store the index of every event, so we can easily check whether an event happened before or after another.

Add the tracker implementation to `pie/src/tracker/event.rs`:

```rust,
{{#include l_event_tracker.rs:2:}}
```

We implement the relevant methods from `Tracker` and store the build events as `Event` instances in `self.events`.
When a new build starts, we clear the events.

Now we will add code to inspect the build events.
This is quite a bit of code that we will be using in integration testing to test incrementality and soundness.
We'll add in just two steps to keep the tutorial going, and we will use this code in the next section, but feel free to take some time to inspect the code.

First we add some methods to `Event` to make finding the right event and getting its data easier for the rest of the code.
Add the following code to `pie/src/tracker/event.rs`:

```rust,
{{#include m_1_event_inspection.rs:2:}}
```

These methods check if the current event is a specific kind of event, and return their specific data as `Some(data)`, or `None` if it is a different kind of event.

Finally, we add methods to `EventTracker` for inspecting events.
Add the following code to `pie/src/tracker/event.rs`:

```rust,
{{#include m_2_event_inspection.rs:2:}}
```

We add several general inspection methods:
- `slice` and `iter` provide raw access to all stored `Event`s,
- `any` and `one` are for checking predicates over all events,
- `find_map` for finding the first event given some function, returning the output of that function.

Then we add methods for specific kinds of events, following the general methods.
For example, `first_require_task` finds the first require task start and end events for a task, and return their event data as a tuple.
`first_require_task_range` finds the same events, but returns their indices as a `RangeInclusive<usize>`.

Check that the code compiles with `cargo test`.

## Implement composite tracker

Currently, we cannot use both `EventTracker` and `WritingTracker` at the same time.
We want this so that we can check incrementality and soundness, but also look at standard output for debugging, at the same time.
Therefore, we will implement a `CompositeTracker` that forwards build events to 2 trackers.

Add the following code to `pie/src/tracker/mod.rs`:

```rust,
{{#include n_composite.rs:2:}}
```

`CompositeTracker` is a tuple struct containing 2 trackers that implements all tracker methods and forwards them to the 2 contained trackers.
Its tuple fields are `pub` so it can be constructed with `CompositeTracker(t1, t2)` and the contained trackers can be accessed with `c.0` and `c.1`.

Check that the code compiles with `cargo test`.

Now that the build event tracking infrastructure is in place, we can start integration testing!

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/2_tracker/source.zip).
```
