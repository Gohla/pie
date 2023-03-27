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
}
