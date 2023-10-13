# Prevent Hidden Dependencies

```admonish warning title="Under Construction"
This page is under construction
```

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
3) Improve and add additional tests

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

This hints to a check in the `require_file_with_stamper` and `provide_file_with_stamper` methods.

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

