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
