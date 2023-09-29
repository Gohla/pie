

#[cfg(test)]
mod test {
  use crate::Context;
  use crate::stamp::{FileStamper, OutputStamper};

  use super::*;

  /// Task that returns its owned string. Never executed, just used for testing the store.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct StringConstant(String);

  impl StringConstant {
    pub fn new(string: impl Into<String>) -> Self { Self(string.into()) }
  }

  impl Task for StringConstant {
    type Output = String;
    fn execute<C: Context<Self>>(&self, _context: &mut C) -> Self::Output {
      self.0.clone()
    }
  }

  #[test]
  fn test_file_mapping() {
    let mut store: Store<StringConstant, String> = Store::default();

    let path_a = PathBuf::from("hello.txt");
    let node_a = store.get_or_create_file_node(&path_a);
    assert_eq!(node_a, store.get_or_create_file_node(&path_a)); // Same node
    assert_eq!(&path_a, store.get_file_path(&node_a)); // Same file path

    let path_b = PathBuf::from("world.txt");
    let node_b = store.get_or_create_file_node(&path_b);
    assert_eq!(node_b, store.get_or_create_file_node(&path_b));
    assert_eq!(&path_b, store.get_file_path(&node_b));

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  #[should_panic]
  fn test_file_mapping_panics() {
    let mut fake_store: Store<StringConstant, String> = Store::default();
    let fake_node = fake_store.get_or_create_file_node("hello.txt");
    let store: Store<StringConstant, String> = Store::default();
    store.get_file_path(&fake_node);
  }
}
