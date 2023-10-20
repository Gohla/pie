# Prevent Cycles

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

To support reserved task dependencies, we will first add a `ReservedRequireTask` variant to `Dependency`.
Modify `pie/src/dependency.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/7_cycle/c_1_dependency.rs.diff}}
```

The `ReservedRequireTask` variant has no data, as this variant needs to be creatable before we have the output of the task.

A reserved task dependency cannot be consistency checked, so we panic if this occurs, but this will never occur if our implementation is correct.
A reserved task dependency is added from the current executing task that is being made consistent, and we never check a task that is already consistent this session.
As long as the reserved task dependency is updated to a real `RequireTask` dependency within this session, we will never check a reserved task dependency.

```admonish note title="Properties of the Build System"
Again, it is great that we have defined these kind of properties/invariants of the build system, so we can informally reason about whether certain situations occur or not.
```

This change breaks the `WritingTracker`, which we will update in `pie/src/tracker/writing.rs`:

```diff2html linebyline
{{#include ../../gen/3_min_sound/7_cycle/c_2_writing_tracker.rs.diff}}
```

Since reserved task dependencies are never checked, we can just ignore them in the `check_dependency_end` method.

Now we update the `Store` with a method to reserve a task dependency, and a method to update a reserved task dependency to a real one.
Modify `pie/src/store.rs`:

```diff2html
{{#include ../../gen/3_min_sound/7_cycle/c_3_store.rs.diff}}
```

We rename `add_task_require_dependency` to `reserve_task_require_dependency`, change it to add `Dependency::ReservedRequireTask` as edge dependency data, and remove the `dependency` parameter since we don't need that anymore.
Note that this method still creates the dependency edge, and can still create cycles which need to be handled.
This is good, because this allows us to catch cycles before we start checking and potentially executing a task.
For example, this will catch the self-cycle created by `TestTask::RequireSelf` because `graph.add_edge` returns a cycle error on a self-cycle.

We add the `update_task_require_dependency` method to update a reserved task dependency to a real one.

As per usual, we will update the tests.
Modify `pie/src/store.rs`:

```diff2html
{{#include ../../gen/3_min_sound/7_cycle/c_4_store_test.rs.diff}}
```

We update `test_dependencies` to reserve and update task dependencies, and rename `test_add_task_require_dependency_panics`.
We add 2 tests for `update_task_require_dependency`.

The store now handles reserved task dependencies.
Now we need to use them in `TopDownContext`.
Task dependencies are made in `require_file_with_stamper`, so we just need to update that method.

Modify `pie/src/context/top_down.rs`:

```diff2html
{{#include ../../gen/3_min_sound/7_cycle/c_5_top_down.rs.diff}}
```

Before we call `make_task_consistent` and potentially execute a task, we first reserve the task dependency (if a task is currently executing).
Since `reserve_task_require_dependency` now can make cycles, we move the cycle check to the start.
As mentioned before, this will catch self cycles.

Additionally, this will add a dependency edge to the graph that is needed to catch future cycles, such as the cycle between `TestTask::RequireA` and `TestTask::RequireB`.
For example, `TestTask::RequireA` executes and requires `TestTask::RequireB`, thus we reserve an edge from A to B.
Then, we require and execute B, which requires A, thus we reserve an edge from B to A, and the cycle is detected!
If the edge from A to B was not in the graph before executing B, we would not catch this cycle.

After the call to `make_task_consistent` we have the consistent output of the task, and we update the reserved dependency to a real one with `update_task_require_dependency`.

This will correctly detect all cyclic tasks.
Confirm your changes work and all tests now succeed with `cargo test`.

```admonish success title="Fixed Tests"
Tests `require_self_panics`, `require_cycle_a_panics`, and `require_cycle_b_panics` should now succeed.
```

We don't need to write additional tests, as these 3 tests capture the kind of cycles we wanted to fix.
Additional positive tests are not really needed, as the other tests cover the fact that cycles are only detected when there actually is one.

This is the last correctness issue that needed to be solved.
Our programmatic incremental build system is now truly incremental (minimal) and correct (sound)!
There are of course certain caveats, such as non-canonical paths and symbolic links which need to be solved for additional correctness.
We will not do that in this tutorial, but feel free to solve those issues (and write tests for them!).

```admonish example title="Download source code" collapsible=true
You can [download the source files up to this point](../../gen/3_min_sound/7_cycle/source.zip).
```

This is currently the end of the guided programming tutorial.

## Future work

I'd still like to write a tutorial going over an example where we use this build system for incremental batch builds, but at the same time also reuse the same build for an interactive environment.
This example will probably be something like interactively developing a parser with live feedback.
 
I'd also like to go over all kinds of extensions to the build system, as there are a lot of interesting ones.
Unfortunately, those will not be guided like the rest of this programming tutorial, due to lack of time.

## PIE implementations

In this tutorial, you implemented a large part of the PIE, the programmatic incremental build system that I developed during my PhD.
There are currently two versions of PIE:

- [PIE in Java](https://github.com/metaborg/pie). The motivation for using Java was so that we could use PIE to correctly incrementalize the [Spoofax Language Workbench](https://spoofax.dev/), a set of tools and interactive development environment (IDE) for developing programming languages. In Spoofax, you develop a programming language by _defining_ the aspects of your language in _domain-specific meta-languages_, such as SDF3 for syntax definition, and Statix for type system and name binding definitions. 
 
  Applying PIE to Spoofax culminated in [Spoofax 3](https://spoofax.dev/spoofax-pie/develop/) (sometimes also called Spoofax-PIE), a new version of Spoofax that uses PIE for all tasks such as generating parsers, running parsers on files to create ASTs, running highlighters on those ASTs to provide syntax highlighting for code editors, etc. Because all tasks are PIE tasks, we can do correct and incremental batch builds of language definitions, but also live development of those language definitions in an IDE, using PIE to get feedback such as inline errors and syntax highlighting as fast as possible.
- [PIE in Rust](https://github.com/Gohla/pie), a superset of what you have been developing in this tutorial. I plan to make this a full-fledged and usable library for incremental batch builds and interactive systems. You are of course free to continue developing the library you made in this tutorial, but I would appreciate users and/or contributions to the PIE library!

  The motivation for developing a PIE library in Rust was to test whether the idea of a programmatic incremental build system really is programming-language agnostic, as a target for developing this tutorial, and to get a higher-performance implementation compared to the Java implementation of PIE. 

  In my opinion, implementing PIE in Rust as part of this tutorial is a much nicer experience than implementing it in Java, due to the more powerful type system and great tooling provided by Cargo. However, supporting multiple task types, which we didn't do in this tutorial, is a bit of a pain due to requiring trait objects, which can be really complicated to work with in certain cases. In Java, everything is a like a trait object, and you get many of these things for free, at the cost of garbage colletion and performance of course.

## Publications about Programmatic Incremental Build System and PIE

```admonish warning title="Under construction"
This subsection is under construction.
```

### My publications

I wrote two papers about programmatic incremental build systems, and PIE, which you can [find the most updated versions of in my dissertation](https://gkonat.github.io/assets/dissertation/konat_dissertation.pdf).
I wrote these papers during my time as a PhD candidate at the [Programming Languages group](https://pl.ewi.tudelft.nl) at [Delft University of Technology](https://www.tudelft.nl/).

The two papers are found in my dissertation at:
- Chapter 7, page 83: PIE: A Domain-Specific Language for Interactive Software Development Pipelines. 
 
  This describes a domain-specific language (DSL) for programmatic incremental build systems, and introduces the PIE library in Kotlin. This implementation was later changed to a pure Java library to reduce the number of dependencies. 
- Chapter 8, page 109: Scalable Incremental Building with Dynamic Task Dependencies.

  This describes a hybrid incremental build algorithm that builds from the bottom-up, only switching to top-down building when necessary. Bottom-up builds are more efficient with changes that have a small effect (i.e., most changes), due to only _checking the part of the dependency graph affected by changes_. Therefore, this algorithm _scales down to small changes while scaling up to large dependency graphs_. 

  Unfortunately, we did not implement (hybrid) bottom-up building in this tutorial due to a lack of time. However, the [PIE in Rust](https://github.com/Gohla/pie) library has a [bottom-up context implementation](https://github.com/Gohla/pie/blob/master/pie/src/context/bottom_up.rs) which you can check out. Due to similarities between the top-down and bottom-up context, some common functionality was [extracted into an extension trait](https://github.com/Gohla/pie/blob/master/pie/src/context/mod.rs).

### Supervised publications

TODO: observability

TODO: improved PIE DSL

### Pluto

PIE is based on [Pluto, a programmatic incremental build system](https://www.pl.informatik.uni-mainz.de/files/2019/04/pluto-incremental-build.pdf) developed by Sebastian Erdweg et al.
This is not a coincidence, as Sebastian Erdweg frequently contributed improvements to our software (Spoofax 2 uses Pluto), he was my PhD copromotor, and coauthored the "Scalable Incremental Building with Dynamic Task Dependencies" paper.

This paper provides a more formal proof of incrementality and correctness for the top-down build algorithm, which provides confidence that this algorithm works correctly, but also explains the intricate details of the algorithm very well.
Note that Pluto uses "builder" instead of "task".
In fact, a Pluto builder is more like an incremental function that _does not carry its input_, whereas a PIE task is more like an incremental closure that includes its input.
 
PIE uses almost the same top-down build algorithm as Pluto, but there are some technical changes that make PIE more convenient to use.
In Pluto, tasks are responsible for storing their output and dependencies, called "build units", which are typically stored in files.
In PIE, the library handles this for you, which is convenient, but does require a mapping from a `Task` (using its `Eq` and `Hash` impls) to its dependencies and output, which is what the `Store` does.

### Other related work
