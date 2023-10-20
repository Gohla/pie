# Prevent Hidden Dependencies

There is one more file-based inconsistency in our incremental build system that we need to prevent: hidden dependencies.
A hidden dependency occurs when a task R requires file F that is provided by another task P, without task R requiring task P.

Hidden dependencies are problematic for the same reason as overlapping provided files: we can require tasks in a specific order that causes an inconsistency.
For example, we could first require task R, which reads file F, and then we could require task P, which writes to and changes file F in such a way that R's dependency to it becomes inconsistent.
This is incorrect, because we made task R consistent while its file dependency to F is inconsistent, so R should be inconsistent!

To prevent this problem, task R needs to require task P.
Then, when task R is required, task P will always first be made consistent, first writing its changes to file F, before task R reads file F.

This is all a bit abstract so let's do the same as the previous section: write tests to show the problem.
In this section, we will:
 
1) Create tests to showcase the hidden dependency problem.
2) Prevent hidden dependencies by checking for them at runtime, fixing the issue.
3) Improve and add additional tests.

## Test to showcase the issue

Add the following test to `pie/tests/top_down.rs`:

```rust,
{{#include a_1_test.rs:3:}}
```

In this test, task `read` reads from `file`, and task `write` writes to `file`.
Task `write` gets the string to write through `read_for_write` which reads it from `input_file`.
There is a hidden dependency here, because `read` reads `file`, which is provided by `write`, without `read` actually requiring `write`.
We can say that the dependency from `read` to `write` is hidden by `file`.

We first require `write`, assert that it is executed, and assert that `file` now contains `"Hi there"` which is what `write` wrote into `file`.
Then we require `read` and assert that it is executed and returns `"Hi there"`.
Even though there is a hidden dependency, we have not observed an inconsistency yet, because we've required the tasks in the correct order.

Now extend this test in `pie/tests/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/a_2_test.rs.diff}}
```

We change `input_file` such that `read_for_write` becomes inconsistent, making `write` inconsistent.
Basically, `write` now wants to write `"Hello There!"` to `file`.

We then require the tasks in the opposite order, first `read` and then `write`, but the result is incorrect.
Requiring `read` still returns `"Hi there"`, even though `write` is inconsistent and needs to first write `"Hello There!"` to `file` before `read` reads it!
Requiring `read` should really return `"Hello There!"`.

Similarly to overlapping provided files, this inconsistent behaviour is caused by the ability to require individual tasks, and our build system (incrementally and correctly) making only the required task (and its dependencies) consistent.
This inconsistent behaviour is undesirable, and should be prevented.

Before continuing, confirm the test succeeds with `cargo test`.
We will modify this test to assert the desired behaviour later.

## Prevent hidden dependencies

There are two ways in which a hidden dependency can be manifested:

1) When a task R requires file F: if F is provided by task P, and R does not require P, there is a hidden dependency.
2) When a task P provides file F: if F is required by tasks R*, and one or more tasks from R* does not require P, there is a hidden dependency.

We already saw an example of the first case in the test.
The second case occurs when a task first requires a file that is not yet provided by a task, but then later on a task provides it.
In both cases, the hidden dependency can result in tasks reading from a file that will later be written to (provided) by another task, which leaves those reading tasks in an inconsistent state.

We will need to check for both cases.
The first case can be checked in the `require_file_with_stamper` method, and the second one in `provide_file_with_stamper`.

Both checks need some way to query whether a task depends on another task.
We could query whether task R depends on P directly, and that would work fine.
However, sometimes task R will not require P directly, but require it through some other task(s) that require P eventually.
This is still correct, because P _will_ be made consistent _before_ R.
Therefore, we need to add a method to `Store` to query whether a task directly or indirectly (also called transitively) depends on another.

Furthermore, in the second check we need to get all tasks that require a file, for which we will also need a `Store` method.

### Add `Store` methods

Let's add those methods to `Store`.
Modify `pie/src/store.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/b_1_store.rs.diff}}
```

We add the `get_tasks_requiring_file` method that does what it says on the tin.
It is almost identical to `get_task_providing_file`, but returns an `Iterator` because multiple tasks can require a single file.
We also have to make the lifetimes more explicit, to explain to Rust that the lifetimes on `self` and `dst` are not related to the implicit `'_` lifetime of the iterator.
This works because we are not borrowing anything in the iterator, because our `filter_map` copies nodes with `TaskNode(*n)`.

The `contains_transitive_task_dependency` method also does what it says.
Luckily, the graph library takes care of this query.

Per usual, add some tests for these methods in `pie/src/store.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/b_2_store.rs.diff}}
```

We assert that the new methods return what is expected in `test_dependencies`, and add tests confirming panics when used on non-existent nodes.

### Add checks to `TopDownContext`

Now we can add hidden dependency checks to `TopDownContext`.
Add the checks to `pie/src/context/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/c_top_down.rs.diff}}
```

We add the first check to `require_file_with_stamper`, where the current executing task is requiring a file.
We check whether a task provides the file being required, and (inside the `if let`) check that the current executing task requires the providing task.
If not, we panic with a hidden dependency error.

Similarly, in `provide_file_with_stamper`, we perform the second check.
Because multiple task can require a file, we perform the check for every requiring task with a `for` loop.
If any requiring task fails the check, there is a hidden dependency, and panic with an error.

That's it!
Test your changes with `cargo test`, which should make the `test_hidden_dependency` test fail as expected!

```admonish failure title="Expected Test Failure"
Test `test_hidden_dependency` will fail as expected, which we will now fix!
```

## Fixing and improving the tests

Like with the overlapping provided file test, we'll heavily simplify our test to only test that it panics.
Modify `pie/tests/top_down.rs`:

```diff2html
{{#include ../../gen/3_min_sound/6_hidden_dep/d_1_test.rs.diff}}
```

We check for a `"Hidden dependency"` panic, rename the test, wrap it in a nested `run` function to support `Result`, and simplify it.
The second call to `require_then_assert_one_execute` will panic due to a hidden dependency: `read` requires `file` without a task dependency to `write`.

Now add a test for the second case to `pie/tests/top_down.rs`:

```rust,
{{#include d_2_test.rs:2:}}
```

Here, the second call to `require_then_assert_one_execute` will panic due to a hidden dependency: `write` provides `file` which is required by `read` which does not have a task dependency to `write`.

Confirm both tests succeed with `cargo test`.
All tests are succeeding again 🎉.

```admonish success title="Fixed Tests"
Test `test_require_hidden_dependency_panics` (was: `test_hidden_dependency`) should now succeed.
```

We should also write some tests that show that non-hidden (visible?) dependencies do actually work.
However, our `ReadFile` task is not capable of making task dependencies at all, so we will need to fix that first (and refactor all uses of `ReadFile` unfortunately).

Modify `pie/tests/common/mod.rs`:

```diff2html
{{#include ../../gen/3_min_sound/6_hidden_dep/e_1_read_origin.rs.diff}}
```

We add an optional task argument to `ReadFile`, which we require when the read task is executed.
We call this optional task argument an `origin`, a shorthand for "originating task".
This is a pattern that appears in programmatic build systems, where a task requires certain files, but those files could be provided by another task.
Instead of `Option<Box<TestTask>>`, we can also use `Vec<TestTask>` if multiple originating tasks are required.

```admonish info title="Explicit Dependencies"
Due to disallowing hidden dependencies, we need to make these originating tasks explicit, which unfortunately requires some additional work when authoring tasks.
However, the reward is that the build system will incrementalize running our tasks for free, and also ensure that the incremental build is correct.
Also, I think it is not such a bad idea to be explicit about these dependencies, because these really are dependencies that exist in the build!
```

```admonish tip title="Returning Paths" collapsible=true
A slightly cleaner approach would be to make `WriteFile` return the path it wrote to, and change `ReadFile` to accept a task in place of its `PathBuf` argument.
Then we could pass a `WriteFile` task as the path argument for `ReadFile`.
We already hinted to this approach in the "Reduce Programming Errors by Returning Paths" block from the previous section.

However, that change would require a bigger refactoring, so we'll go with the simpler (but also more flexible) approach in this tutorial.
```

Now we need to refactor the tests to provide `None` as the origin task for every `ReadFile` task we create.
Modify `pie/tests/top_down.rs`:

```diff2html
{{#include ../../gen/3_min_sound/6_hidden_dep/e_2_read_refactor.rs.diff}}
```

Confirm your changes are correct with `cargo test`.

Now add the following test to `pie/tests/top_down.rs`:

```rust,
{{#include f_1_test.rs:2:}}
```

This is similar to earlier tests, but now we create an explicit dependency from `read` to `write` by passing in the `write` task as the last argument to `ReadFile`.
When we require `read`, it will first require its origin task `write` to make `file` up-to-date, and then require `file` and read from it.
This is not a hidden dependency: `file` is provided by `write`, but `read` has a dependency to `write`!

Now let's test what happens if we remove `file`.
Modify the test in `pie/tests/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/f_2_test.rs.diff}}
```

When we remove `file` and require `read`, `read` will check its task dependency to `write`.
`write` is inconsistent due to the file being removed (modified stamp becomes `None`), so it will re-execute and re-generate the provided file!

```admonish info title="Benefits of Precise Dynamic Dependencies"
This is another great benefit of the precise dynamic dependencies in programmatic builds: removing an intermediate or output file does not break the build.
Instead, the file is just re-generated as needed, and the build is brought into a consistent state again.
Similarly, modifying `file` would result in the same behaviour: the provided file is re-generated and does not break the build.
```

```admonish info title="File Contents Hash Stamper"
Unfortunately, `read` is re-executed because its `file` dependency is inconsistent due to the changed modified date of `file`.
If we implement a file contents hash stamper and use that as the stamper for `file`, we can prevent this re-execution because the file contents is still the same.
This of course is not free, as hashing file contents has an I/O and processing (CPU) overhead.

In this case, `read` is so simple that the overhead from a hash stamper would be larger than the gains of not executing `read`.
But for expensive tasks with lots of I/O operations and/or processing, a file contents hash stamper makes a lot of sense.
```

As the last test, we will modify `input_file` and confirm that changes to that file propagate to `read`.
Modify the test in `pie/tests/top_down.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/6_hidden_dep/f_3_test.rs.diff}}
```

Confirm the test succeeds with `cargo test`.

We've now fixed the last file dependency inconsistency in the build system.
Absence of overlapping provided files ensures that provided files always end up in a consistent state, even in a programmatic incremental setting.
Absence of hidden dependencies ensures that when a task requires a file, that file is always in a consistent (up-to-date) state.

We needed to prevent these issues in order to make the builds correct under incrementality.
However, we can also use these properties in alternative context implementations to reason about whether certain states can occur.
For example, the actual PIE library contains a bottom-up incremental build context that instead performs incremental builds from the bottom up.
There we can use the absence of overlapping provided files and hidden dependencies to reason that a bottom-up build can correctly skip checking tasks in certain situations, increasing incrementality.
We do not (currently) cover bottom-up builds in this tutorial, but I found it important to highlight that these are fundamental properties.

```admonish question title="Can we Infer Hidden Dependencies?" collapsible=true
Currently, when we encounter a hidden dependency, we panic to stop the build.
Can we instead infer the hidden dependency and continue building?
Unfortunately, not really.

We could infer the first hidden dependency case: task R requires file F, provided by task P, without R requiring P.
In that case, we could require P before creating a dependency to F, and that could work rather well.

Unfortunately, we cannot infer the second hidden dependency case: task P provides file F, required by tasks R*, with one or more tasks from R* not requiring P.
At this point, it can already be too late to end up in a consistent state: a task from R* could have already been required/executed and have already read inconsistent file F.
We could infer the dependency but the task has already read inconsistent state, which is not correct.

We could choose to only infer the first hidden dependency case, but this can be very error-prone and inconsistent.
Without these explicit dependencies, we would rely on the build system to infer these for us.
But it could still occur that a task first requires file F before a task provides it, which would still panic due to the second case.
Whether this happens or not relies on which tasks are required by the user, which tasks are executed by the build system, and the order in which that happens.
Therefore, it is unfortunately a bad idea to infer hidden dependencies in a programmatic incremental build system.
```

```admonish warning title="Symbolic Links: An Incremental Build System's Nightmare"
A file or directory can be a [symbolic link](https://en.wikipedia.org/wiki/Symbolic_link) to another file or directory. 
In this tutorial we do not deal with symbol links at all, and this is a threat to correctness.
For example, a task could circumvent a hidden dependency by creating a new symbolic link that links to the file it wants to read, where the linked-to file is provided by a task.
An overlapping provided file can be made in a similar way.

Therefore, we should resolve symbolic links, right... right?
Surely this should be easy.

One does not simply resolve a symbolic link in an incremental system.
The problem is that creating a dependency to a symbolic link, is actually creating two dependencies:
- a dependency to the symbolic link file/directory, with the stamper working on the _link_,
- a dependency to the linked-to file/directory, with the stamper working on the linked-to file/directory.

But wait, there's more.
A symbolic link can point to a file with another symbolic link!
Therefore, any file dependency could become many file dependencies.
We have to recursively traverse the symbolic link tree.

What if I told you that there can even be cycles in symbolic links?
In that case, creating a file dependency actually creates infinite file dependencies!

We chose not to deal with this in the tutorial for simplicity.
In fact, I would almost refuse to support symbolic links, as they are the root of all evil from an incremental build systems's perspective.
```

```admonish warning title="Performance Impact of Symbolic Links" collapsible=true
Symbolic links can be a performance problem, because resolving a symbolic link requires a system call, and we need to resolve every path.
We need to resolve every path because any path could point to a file or directory that again points to a different file or directory (and this can be recursive even!)
Therefore, the presence of symbolic links turn simple and cheap path operations into complex and expensive system calls.
```

```admonish warning title="Non-Canonical Paths"
There is an even simpler way than symbolic links to circumvent our checks: just create different paths that point to the same file.
For example, `in_out.txt` and `./in_out.txt` both point to the same file, but are different paths (i.e., comparing them with `Eq` will return `false`).

The issue is that we use non-canonical paths in the dependency graph, and thus also to check for overlapping provided files and hidden dependencies.
Instead, we should first canonicalize a path, converting relative paths to absolute ones, removing excess `..` `.` parts, and more.

We could use Rust's [`canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html) function, but on Windows this returns paths that many tools do not support.
The [dunce](https://docs.rs/dunce/latest/dunce/) library can resolve this issue by canonicalizing to more compatible paths on Windows.

However, canonicalizing a path also resolves symbolic links.
If we resolve symbolic links but do not create separate dependencies to link files and linked-to files, we are breaking incrementality and correctness there.

We have four options:
1) Write our own path canonicalization function that does not resolve symbolic links. Document that a dependency to a symbolic link only results in a dependency to the symbolic link file, which breaks incrementality and correctness when the linked-to file changes.
2) Write our own path canonicalization function that also correctly resolves symbolic links by creating dependencies to both link files and linked-to files, handle recursion, and handle cycles.
3) Canonicalize the path. Document that a dependency to a symbolic link only results in a dependency to the pointed-to file, which breaks incrementality and correctness when the link changes.
4) Don't care about any of this.

In this tutorial, we go for option 4 for simplicity.
Personally, I would choose for option 3 unless it is critical that symbolic links are handled in a correct way (then I'd have to choose option 2 and be grumpy).
```

```admonish warning title="Circumventing Checks"
There are many other ways to circumvent the hidden dependency check.
A simple one is to just not create a dependency!

We cannot fully waterproof our system, just like you can circumvent Rust's safety with `unsafe` or by sharing mutable state via files. 
That is fine.
We should at least try our best to catch accidents, such as accidentally using different non-canonical paths for the same file.
```

In the next section, we will fix the remaining correctness issue related to cyclic tasks.

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/6_hidden_dep/source.zip).
```
