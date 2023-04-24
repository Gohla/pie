#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Dependency<T, O> {
  RequireTask(TaskDependency<T, O>),
  RequireFile(FileDependency),
}

impl<T: Task> Dependency<T, T::Output> {
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Result<bool, io::Error> {
    match self {
      Dependency::RequireTask(d) => Ok(d.is_inconsistent(context)),
      Dependency::RequireFile(d) => d.is_inconsistent(),
    }
  }
}
