# Prevent Cycles

```admonish warning title="Under construction"
This page is under construction.
```

In this section, we will fix the remaining correctness issue with cyclic tasks.

Didn't we already catch dependency graph cycles in the Incremental Top-Down Context section?
Yes, you remembered right!
However, there is a corner case that we didn't handle.
The issue is that we add a task dependency to the dependency graph only _after the task has finished executing_.
We do this because we need the output from executing the task to create the dependency.

But what would happen if we made a task that just requires itself?
Let's figure that out in this section, in which we will:

1) Add cyclic tasks to the testing tasks.
2) Create tests to showcase the cyclic task execution problem.
3) Prevent cycles by _reserving_ a task dependency before executing the task.
4) Improve and add additional tests.

## Add cyclic testing tasks

We don't have any testing tasks to easily construct different kinds of cycles yet, so we will add those first.

Modify `pie/tests/common/mod.rs`:
       
```diff2html linebyline
{{#include ../../gen/3_min_sound/7_cycle/a_task.rs.diff}}
```

We add the `RequireSelf` task which directly requires itself.
We also add the `RequireA` and `RequireB` tasks which require each other in a cycle.
We want to prevent both of these kinds of cycles.

## Add cycle tests 

Now add tests that check whether requiring these tasks (correctly) panics due to cycles.

Modify `pie/tests/top_down.rs`:

```rust,
{{#include b_test.rs:3:}}
```

These test are simple: require the task and that's it.
Which of these tests will correctly result in a cyclic task dependency panic?

```admonish warning title="Infinite Recursion"
Running these tests will result in infinite recursion, but should quickly cause a stack overflow.
However, be sure to save everything in the event your computer becomes unresponsive.
```

```admonish failure title="Expected Test Failure"
Tests `require_self_panics`, `require_cycle_a_panics`, and `require_cycle_b_panics` will fail as expected, which we will fix in this section!
```

Run the tests with `cargo test`, or skip running them (and comment them out) if you don't want to waste battery life running infinite recursions.
These tests will infinitely recurse and thus fail.

The issue is that we only add a dependency to the dependency graph _after the task has executed_.
We do this because we need the output from the executing task to create the dependency.
Therefore, no dependencies are ever added to the dependency graph in these tests, because a task never finishes executing!
This in turn causes the cycle detection to never trigger, because there is no cycle in the dependency graph.

To fix this, we need to add task dependencies to the dependency graph _before we execute the task_.
But we cannot do this, because we need the output of the task to create the task dependency.
Therefore, we need to first _reserve_ a task dependency in the dependency graph, which creates an edge in the dependency graph without any attached data.

## Reserving task dependencies

