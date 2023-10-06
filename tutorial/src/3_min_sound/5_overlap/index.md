# Prevent Overlapping File Writes

```admonish warning title="Under construction"
This section is under construction.
```

So far we have only considered reading files in the build system.
However, there are many tasks that also write files.
For example, a C compiler reads a C source file (and header files) and writes an object file with the compiled result, which is typically an intermediate file that gets passed to a linker.
Another example is a file copy task that reads a file and copies it to another file.

We can handle file writes in tasks with `context.require_file`.
However, what should happen when two tasks write to the same file?
In a non-incremental setting, the last writer wins by overwriting (or appending to) the file.
Does this behaviour also occur in our incremental build system?

Unfortunately, this is not always the case in our incremental build system, because we can `require` individual tasks.
This is a bit tricky to explain without an example, so we will first add some testing tasks and write a test that showcases the problem.
In this section, we will continue with:
 
1) Add the `WriteFile` and `Sequence` tasks to the testing tasks.
2) Create a `test_overlapping_file_write` test to showcase a soundness hole.
3) Introduce a new kind of dependency: a _provide file dependency_ for writing to (and creating) files.
4) Prevent overlapping file writes by checking for them at runtime, fixing the soundness hole.

[//]: # (We did ensure that the `Store` returns dependencies in the order in which they were added, meaning that dependencies are checked and executed in the order they were created.)
[//]: # (Also, the `execute` methods of `Task`s are regular parts of the program that are executed from top to bottom, so that is also ordered.)
[//]: # (So if a `Sequence` task requires two file writing tasks `write_1` and `write_2` in that order, that write to the same `output_file`, then `write_2` will win over `write_1` _when the `Sequence` is required_.)
[//]: # (So, what's the problem then?)
[//]: # ()
[//]: # (Well, in our build system we can `require` individual tasks.)
[//]: # (If after requiring the `Sequence` task, we require `write_1`, it will be executed because its fi)
[//]: # (However, in our build system we cannot fully rely on the order in which distinct tasks are executed, because we can `require` individual tasks.)
[//]: # (For example, in the previous section, we could have required the `read` task instead of the `lower` or `upper` task if we were only)
[//]: # (unless your `execute` is somehow non-deterministic, but that is another can of worms)

## Add `WriteFile` and `Sequence` tasks
