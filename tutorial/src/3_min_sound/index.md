# Minimality and Soundness

So far, the definitions we've used for minimality (incrementality) and soundness (correctness) have been a bit vague.
Let's define this more concretely and precisely.

An incremental and sound build system executes a task, if and only if, it is new or affected by a change.
A task is new if it has not been executed before.
When it is not new, a task is affected by a change when any of its dependencies are inconsistent.
A file dependency is inconsistent if its file stamp changes.
A task dependency is inconsistent if, after recursively checking the task, its output stamp changes.
The recursive nature of checking task dependencies ensures that indirect changes can affect tasks and cause them to be correctly executed.

By defining minimality and soundness in terms of dependencies, a task author forgetting to create a dependency or not choosing the correct stamper, does not change whether our build system is minimal and sound.
PIE works under the assumption that task authors correctly list all dependencies that mark their task as affected by a change when it actually is. 

```admonish info title="Preventing task authoring mistakes" collapsible=true
TODO: OS hook to detect all file changes
TODO: sandboxing like Bazel
```

In this chapter, we will show minimality and soundness by testing.
Before testing however, we will make our definition of minimality a bit more precise, as this change has a large impact on the API of our build system, and migrating all testing code later is not fun.

The issue is that tasks are affected by changes in the filesystem, and the filesystem can change during the build.
Therefore, a task can be affected by multiple different changes in one build.
For example, after executing a task, it could immediately be affected by a change in a source file again without the build system knowing about it, and that would not be minimal nor sound.

Therefore, we will introduce the concept of a *session*.
Builds are only performed in a session, and at most one session may exist at any given time.
In one session, each task is *executed at most once*, and changes made to source files during a session are *not guaranteed to be detected*.
Therefore, if a file dependency is inconsistent at the time it is checked, the corresponding task is executed once, and will not be executed any more that session.
This simplifies minimality and soundness, as we do not need to worry about checking tasks multiple times.

```admonish info title="Proving minimality and soundness?" collapsible=true
While proving minimality and soundness would be a very interesting exercise, I am not at all an expert in formal proofs in proof assistants such as [Coq](https://coq.inria.fr/), [Agda](https://wiki.portal.chalmers.se/agda/pmwiki.php), etc.
If that is something that interests you, do pursue it and get in touch!
```

We will continue as follows:

1) Introduce sessions and change the API to work with sessions: `Session` type for performing builds in a session, and the `PIE` type as the entry point that manages sessions.
2) Create infrastructure to track build events for testing and debugging purposes. Create the `Tracker` trait, and implement a `WritingTracker` for debugging and `EventTracker` for testing.
3) Find a soundness hole where multiple tasks write to the same file. Fix it by tracking file write dependencies separately from read dependencies, and catch these mistakes with dynamic verification.
4) Find a soundness hole where a task reads from a file before another task writes to it. Fix it by catching these mistakes with dynamic verification.
5) Find a soundness hole where cyclic task execution can still occur. Fix it by changing how task dependencies are stored.
