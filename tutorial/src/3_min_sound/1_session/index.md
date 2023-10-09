# Minimality with Sessions

A task is consistent if its dependencies are consistent, and consistency of file dependencies is based on the filesystem.
However, the filesystem can change during a build, meaning that a task can be affected by multiple different changes in one build.
For example, after executing a task, it could immediately be affected by a change in a source file again without the build system knowing about it, and that would not be minimal nor sound.

Therefore, we will introduce the concept of a *session*.
Builds are only performed in a session, and at most one session may exist at any given time.
In one session, each task is checked or executed *at most once*, meaning that changes made to source files during a session are *not guaranteed to be detected*.

The result is that if a task is deemed inconsistent at the time it is checked, it will be executed, and will not be checked nor executed any more that session.
If a task is deemed consistent at the time it is checked, it will not be checked any more that session.
This simplifies minimality and soundness, as we do not need to worry about checking tasks multiple times.
Furthermore, it is also an optimisation, as requiring the same task many times only results in one check.

We will continue as follows:
1) Create the `Session` type to hold all session data, and the `Pie` type as an entry point into the build system that manages a session.
2) Update `TopDownContext` to work with `Session`.
3) Update the incrementality example to work with `Session` and `Pie`.
4) Ensure minimality by keeping track whether a task has been required this session.

## PIE and Session

Change the imports in `pie/src/lib.rs`: 

```diff2html fromfile linebyline
../../gen/3_min_sound/1_session/a_lib_import.rs.diff
```

Now add the `Pie` and `Session` types to `pie/src/lib.rs`:

```rust,
{{#include b_lib_pie_session.rs:2:}}
```

We set up the types such that `Pie` owns the store, and `Session` owns all data for a build session that `TopDownContext` previously owned.
We put the store in `Pie` because we want to keep the dependency graph and task outputs between build sessions, otherwise we cannot be incremental.

A `Session` is created with `Pie::new_session`, which borrows `Pie` mutibly, ensuring that there can only be one `Session` instance (per `Pie` instance).
`run_in_session` is a convenience method that runs given function inside a new session.

`Session::require` should require the task with the top-down context and return its up-to-date output, which we will implement once we've changed `TopDownContext`.
The dependency check errors can be accessed with `Session::dependency_check_errors`.

Note that `Session` also has access to `Store`, because `TopDownContext` needs access to the store.
The store is mutibly borrowed from `Pie`.
Therefore, the `Session` struct is generic over the `'p` lifetime, where the `p` stands for `Pie`.
We can leave out this lifetime in `Pie::new_session`, because the compiler infers it from us, but we must be explicit in structs and most impls.

Check that the code compiles (but gives warnings) with `cargo check`.

Now we need to modify `TopDownContext` to work with `Session`.

## Update TopDownContext

Change `TopDownContext` to only contain a mutable reference to `Session` in `pie/src/context/top_down.rs`:

```diff2html fromfile
../../gen/3_min_sound/1_session/c_top_down_new.rs.diff
```

Here, we use lifetime `'s` to denote the lifetime of a session, and make `TopDownContext` generic over it.
`new` now just accepts a mutable reference to the session.
The `get_dependency_check_errors` method can be removed.
We add a `require_initial` convenience method for `Session`.

In the rest of the file, we need to update the `impl` lines to include the lifetimes, and we need to replace most instances of `self` with `self.session`.
You could do this with the following find-replace regex: `self\.([\w\d_]+)\.` -> `self.session.$1.`

Change `pie/src/context/top_down.rs`:

```diff2html fromfile
../../gen/3_min_sound/1_session/d_top_down_fix.rs.diff
```

Now we change `Session` to use `TopDownContext`. 

## Update Session

Change `pie/src/lib.rs`:

```diff2html fromfile
../../gen/3_min_sound/1_session/e_lib_require.rs.diff
```

We reset the `current_executing_task` to `None`, to be sure that we start a build without an executing task.
Then, we just create a `TopDownContext` and call `require_initial`.

Finally, we can now make the `context` module private, as users of the library run builds using `Session`, instead of having to create a context implementation.
Change `pie/src/lib.rs`:

```diff2html fromfile
../../gen/3_min_sound/1_session/f_lib_private_module.rs.diff
```

Check that the code compiles with `cargo check --lib`.
This only checks if the library builds, but not any examples.
We need to update the incrementality example to work with these changes.

## Update incremental example

Change `pie/examples/incremental.rs` to use sessions:

```diff2html fromfile
../../gen/3_min_sound/1_session/g_example.rs.diff
```

When we only require one task, we replace `context.require_task` with `pie.new_session().require`.
When we want to require multiple tasks, we use `new_session` and call `session.require` multiple times.

It is very important to create a new session each time in this example, because a task is only checked/executed once each session.
If we use a single session, our changes are never seen, and we just execute each task once, which is not what we want.
Therefore, every time we make changes to source files, or expect that changes have been made to source files, we must create a new session.

```admonish question title="Multiple Sessions?"
In changes D and E, Rust is smart enough to allow creating a new session even though the previous `session` variable is still active, because it knows that we don't use that previous session anymore. 
```

Check that the example works with `cargo run --example incremental`, and check that the rest of the code works by running `cargo test`.

## Minimality

Now we can ensure minimality by keeping track whether a task has been required this session.
Change `pie/lib.rs`:

```diff2html fromfile linebyline
../../gen/3_min_sound/1_session/h_lib_consistent.rs.diff
```

We add the `consistent` field to `Session` which is a hash set over task nodes.
We create a new one each session, because we only want to keep track of which tasks are consistent on a per-session basis.

Now change the top-down context in `pie/context/top_down.rs` to use this:

```diff2html fromfile
../../gen/3_min_sound/1_session/i_context_consistent.rs.diff
```

At the start of requiring a task, we check whether the task is already deemed consistent this session, using the `consistent` hash set in `Session`.
If the task is consistent, we skip execution by using `!already_consistent &&` in the if check.
Because `&&` is [short-circuiting (also called lazy)](https://doc.rust-lang.org/reference/expressions/operator-expr.html#lazy-boolean-operators), we even skip the entire `should_execute` call that checks whether we should execute a task, when the task is already consistent.
This increases performance when a lot of consistent tasks are required.

Finally, at the end of `require`, we insert the task node into the `consistent` hash set, to denote that the task is now consistent this session.
That's it! This was a simple change due to the work we did before to get the `Session` API in place.

With this new API in place, minimality of task checking and execution in place, and all code adjusted to work with it, we can continue with tracking build events.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/1_session/source.zip).
```
