

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Dependency<T, O> {
  RequireFile(FileDependency),
  RequireTask(TaskDependency<T, O>),
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Inconsistency<O> {
  File(FileStamp),
  Task(OutputStamp<O>),
}

impl<T: Task> Dependency<T, T::Output> {
  /// Checks whether this dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if the dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if the dependency is consistent,
  /// - `Err(e)` if there was an error checking the dependency for consistency.
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Result<Option<Inconsistency<T::Output>>, io::Error> {
    let option = match self {
      Dependency::RequireFile(d) => d.is_inconsistent()?
        .map(|s| Inconsistency::File(s)),
      Dependency::RequireTask(d) => d.is_inconsistent(context)
        .map(|s| Inconsistency::Task(s)),
    };
    Ok(option)
  }
}
