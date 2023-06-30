# Introduction

In this part, we will implement an *incremental* build context.
An incremental context selectively executes tasks — only those that are affected by a change.
In other words, an incremental context executes the *minimum number of tasks* required to make all tasks up-to-date.

However, due to dynamic dependencies, this is not trivial.
We cannot first gather all tasks into a dependency tree and then topologically sort that, as dependencies are added and removed *while tasks are executing*.
To do incremental builds in the presence of dynamic dependencies, we need to check and execute affected tasks *one at a time, updating the dependency graph, while tasks are executing*.
To achieve this, we will employ a technique called *top-down incremental building*, where we start checking if a top (root) task needs to be executed, and recursively check whether dependent tasks should be executed until we reach the bottom (leaf) task(s), akin to a depth-first search.

Furthermore, build systems almost always interact with the file system in some way. 
For example, tasks read configuration and source files, or write intermediate and binary files.
Thus, a change in a file can affect a task that reads it, and executing a task can result in writing to new or existing files.
Therefore, we will also keep track of *file dependencies*.
Like task dependencies, file dependencies are also tracked dynamically while tasks are executing.

There are several ways to check if a file dependency is consistent (i.e., has not changed), such as checking the last modification date, or comparing a hash.
To make this configurable on a per-dependency basis, we will implement *stamps*.
A file stamp is just a value that is produced from a file, such as the modification date or hash, that is stored with the file dependency.
To check if a file dependency is consistent, we just stamp the file again and compare it with the stored stamp.

Similarly, we can employ stamps for task dependencies as well by stamping the output of a task.

To achieve incrementality, we will continue with these steps in the following chapters:
1) Extend `Context` with a method to *require a file*, enabling tasks to specify dynamic dependencies to files.
2) Implement *file stamps* and *task output stamps*, and extend `Context` with methods to select *stampers*, enabling tasks to specify when a dependency is consistent.
3) Implement *dynamic dependencies* and their *consistency checking*.
4) Implement a *dependency graph store* with methods to query and mutate the dependency graph. 
5) Implement the *top-down incremental context* that incrementally executes tasks.

[//]: # (## Implementing the Incremental Context)

[//]: # ()
[//]: # (Now we get to the fun part, incrementality!)

[//]: # ()
[//]: # (To check whether we need to execute a task, we need to check the dependencies of that task to see if any of them are not consistent.)

[//]: # (A dependency is consistent if after stamping, the new stamp equals the stored stamp.)

[//]: # (If all dependencies of the task are consistent, we return the cached output of the task.)

[//]: # (If not, we execute the task.)

[//]: # ()
[//]: # (To implement this, we will need several components:)

[//]: # (- `FileStamper` and `FileStamp` types for stamping files.)

[//]: # (- `OutputStamper` and `OutputStamp` types for stamping task outputs.)

[//]: # (- Extension to `Context` to support passing `FileStamper` and `OutputStamper` when requiring files and tasks.)

[//]: # (- A `FileDependency` type that holds a `FileStamper` and `FileStamp` to check whether a file is consistent.)

[//]: # (- A `TaskDependency` type that holds an `OutputStamper` and `OutputStamp` to check whether a task is consistent.)

[//]: # (- A `Dependency` type that merges `FileDependency` and `TaskDependency` so we can check whether a dependency is consistent without having to know what kind of dependency it is.)

[//]: # (- A `Store` type which holds the dependency graph with methods for mutating and querying the graph, using `Dependency` to represent dependencies.)

[//]: # (- A `TopDownContext` type that implements `Context` in an incremental way, using `Store`.)

[//]: # ()
[//]: # (We will start with implementing stamps and dependencies, as those can be implemented as a stand-alone part.)
