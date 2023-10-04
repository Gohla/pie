use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, Task};
use crate::dependency::MakeConsistent;
use crate::fs::open_if_file;
use crate::stamp::{FileStamper, OutputStamper};

pub struct NonIncrementalContext;

impl<T: Task> Context<T> for NonIncrementalContext {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, _stamper: FileStamper) -> Result<Option<File>, io::Error> {
    open_if_file(&path)
  }

  fn require_task_with_stamper(&mut self, task: &T, _stamper: OutputStamper) -> T::Output {
    task.execute(self)
  }
}

impl<'p, 's, T: Task> MakeConsistent<T> for NonIncrementalContext {
  fn make_task_consistent(&mut self, task: &T) -> T::Output {
    task.execute(self)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_require_task_direct() {
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct ReturnHelloWorld;

    impl Task for ReturnHelloWorld {
      type Output = String;
      fn execute<C: Context<Self>>(&self, _context: &mut C) -> Self::Output {
        "Hello World!".to_string()
      }
    }

    let mut context = NonIncrementalContext;
    assert_eq!("Hello World!", context.require_task(&ReturnHelloWorld));
  }

  #[test]
  fn test_require_task() {
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum Test {
      ReturnHelloWorld,
      ToLowerCase,
    }

    impl Task for Test {
      type Output = String;
      fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
        match self {
          Self::ReturnHelloWorld => "Hello World!".to_string(),
          Self::ToLowerCase => context.require_task(&Self::ReturnHelloWorld).to_lowercase(),
        }
      }
    }

    let mut context = NonIncrementalContext;
    assert_eq!("Hello World!", context.require_task(&Test::ReturnHelloWorld));
    assert_eq!("hello world!", context.require_task(&Test::ToLowerCase));
  }
}
