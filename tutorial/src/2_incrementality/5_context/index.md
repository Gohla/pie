# Top-down Context

We've implemented all the prerequisites for incremental top-down building.
Now we will create the `TopDownContext` type which implements the `Context` trait in an incremental way. 

## Top-down context basics

Add the `top_down` module to `pie/src/context/mod.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/5_context/a_module.rs.diff
```

Create the `pie/src/context/top_down.rs` file and add the following to get started:

```rust,
{{#include b_basic.rs}}
```

The `TopDownContext` type is generic over tasks `T` and their outputs `O`, owns a `Store`, and can be created using `new`.

`TopDownContext` implements `Context`, and the main challenge will be implementing the `require_file_with_stamper` and `require_task_with_stamper` methods *incrementally* and *correctly*.

## Requiring files

Tasks such as `ReadStringFromFile` which we've used in tests before call `context.require_file` to declare that they depend on a file in the filesystem.
For incrementality, we need to add this dependency to the dependency graph.
This dependency will go from the *current executing task* to the file.
Therefore, we will need to keep track of the current executing task.
 
Change `pie/src/context/mod.rs` to add a field for tracking the current executing task, and use it in `require_file_with_stamper`:

```diff2html fromfile
../../gen/2_incrementality/5_context/c_current.rs.diff
```

We're not setting `current_executing_task` yet, as that is the responsibility of `require_task_with_stamper` which we will implement later.
In `require_file_with_stamper` we're now getting the current executing task.
If there is no current executing task, which only happens if a user directly calls `require_file` on a context, we don't make a dependency and just open the file.

Now we need to add the file dependency, change `pie/src/context/mod.rs` to do this: 

```diff2html fromfile
../../gen/2_incrementality/5_context/d_file.rs.diff
```

We simply create or get an existing file node, create a file dependency, and add the file require dependency to the graph via `store`.
Errors are propagated to the caller, so they can react accordingly to filesystem operation failures.

## Requiring tasks

To implement `require_task_with_stamper`, we need to check whether we should execute a task.
A task should be executed either if it's new (it does not have an output stored yet), or if at least one of its dependencies is inconsistent.
If we don't execute it, then it must have an output value and all its dependencies are consistent, so we just return its output value.

Change `pie/src/context/mod.rs` to implement this logic:

```diff2html fromfile
../../gen/2_incrementality/5_context/e_task.rs.diff
```

We first create or get an existing file node.
Then, we check whether the task should be executed with `should_execute_task` which we still need to implement.

If that returns true, we reset the task, set the current executing task, actually execute the task, restore the previous executing task, and set the task output.
Otherwise, we get the output of the task from the store, which cannot panic because `should_execute_task` ensures that the task has an output if it returns false.
Finally, we return the output.

We still need to create a task dependency. Change `pie/src/context/mod.rs` to add the dependency:

```diff2html fromfile linebyline
../../gen/2_incrementality/5_context/f_task_dep.rs.diff
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

```admonish question title="How to Recover from Panics?" collapsible=true
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

## Checking tasks

The final piece to our puzzle is the `should_execute_task` implementation.

Add the following code to `pie/src/context/top_down.rs`:

```diff2html fromfile linebyline
../../gen/2_incrementality/5_context/g_check.rs.diff
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

```admonish tip title="Rust Help: Reference Counting to Avoid Clones" collapsible=true
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

```admonish question title="How to Avoid Heap Allocations from String?" collapsible=true
A technique for reducing allocations on strings (and string-like types such as `PathBuf`) is to apply [small string optimization](https://fasterthanli.me/articles/small-strings-in-rust), where small strings are stored inline instead of requiring a heap allocation.
This only works if the strings are usually small enough to fit inline on the stack (for example, 32 bytes).

Another technique for strings is string interning, where equal strings are stored in a central place and then re-used everywhere.
This technique is great when we use the same string a lot of times.
That may be a good strategy for a build system, where we work with the same file paths over and over.

There are several crates implementing these techniques, but I have not used one myself yet, so I cannot recommend one.
```

```admonish question title="How to Avoid Heap Allocations from Collecting into Vec?" collapsible=true
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

```diff2html fromfile linebyline
../../gen/2_incrementality/5_context/h_error_field.rs.diff
```

And then change `pie/src/context/mod.rs` to store these errors:

```diff2html fromfile linebyline
../../gen/2_incrementality/5_context/i_error_store.rs.diff
```

It took us a while, but now we've implemented an incremental build system with dynamic dependencies 🎉.
Let's set up a simple example to see the fruits of our labour.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/2_incrementality/5_example/source.zip).
```
