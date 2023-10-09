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

```admonish info title="Preventing Task Authoring Mistakes" collapsible=true
It is of course possible to make mistakes when authoring tasks, for example by creating a dependency to the wrong file, or by forgetting to create a file dependency.
Unfortunately, there is no easy way to solve this.

We will be writing a build event tracking system later, for which we will make an implementation that writes the entire build log to standard output.
This build log can help debug mistakes by precisely showing what the build system is doing.

A technique to catch file dependency mistakes is by sandboxing the filesystem to only have access to files that have been required.
For example, Bazel can perform [sandboxing](https://bazel.build/docs/sandboxing), but it is not fully cross-platform, and still allows reading files from absolute paths.
If a cross-platform and bulletproof sandboxing library exists, it could help catch file dependency mistakes in programmatic incremental build systems.

Finally, the ultimate technique to catch file dependency mistakes is by automatically creating these dependencies using filesystem tracing, instead of having the task author make them.
For example, the [Rattle](https://github.com/ndmitchell/rattle) build system uses [fsatrace](https://github.com/jacereda/fsatrace) to automatically create file dependencies, freeing task authors from having to think about file dependencies
However, filesystem tracing is also not fully cross-platform and bulletproof, so it cannot always be used.
Again, if a cross-platform and bulletproof filesystem tracing library exists, it would be extremely useful for programmatic incremental build systems.
```

In this chapter, we will show minimality and soundness by testing.
However, before testing, we make minimality and soundness more precise by changing the API to work with *sessions*, and implement build event tracking that is needed for testing.
We will continue as follows:

1) Introduce sessions and change the API to work with sessions: `Session` type for performing builds in a session, and the `Pie` type as the entry point that manages sessions.
2) Create infrastructure to track build events for testing and debugging purposes. Create the `Tracker` trait, and implement a `WritingTracker` for debugging and `EventTracker` for testing.
3) Create integration tests that test incrementality and soundness.
4) Find a soundness hole where multiple tasks write to the same file. Fix it by tracking file write dependencies separately from read dependencies, and catch these mistakes with dynamic verification.
5) Find a soundness hole where a task reads from a file before another task writes to it. Fix it by catching these mistakes with dynamic verification.
6) Find a soundness hole where cyclic task execution can still occur. Fix it by changing how task dependencies are stored.

```admonish question title="Proving Minimality and Soundness?" collapsible=true
While proving minimality and soundness would be a very interesting exercise, I am not at all an expert in formal proofs in proof assistants such as [Coq](https://coq.inria.fr/), [Agda](https://wiki.portal.chalmers.se/agda/pmwiki.php), etc.
If that is something that interests you, do pursue it and get in touch!
```
