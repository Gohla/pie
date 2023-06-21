#[cfg(test)]
mod test {
  use dev_shared::assert_panics;

  use crate::Context;
  use crate::stamp::{FileStamper, OutputStamper};

  use super::*;

  /// Task that returns its owned string. Never executed, just used for testing the store.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  pub struct StringConstant(String);

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
  fn test_task_mapping() {
    let mut store = Store::default();

    let task_a = StringConstant::new("Hello");
    let node_a = store.get_or_create_task_node(&task_a);
    assert_eq!(node_a, store.get_or_create_task_node(&task_a)); // Same node
    assert_eq!(&task_a, store.get_task(&node_a)); // Same task

    let task_b = StringConstant::new("World");
    let node_b = store.get_or_create_task_node(&task_b);
    assert_eq!(node_b, store.get_or_create_task_node(&task_b));
    assert_eq!(&task_b, store.get_task(&node_b));

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  fn test_task_outputs() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(&output_a);
    let node_a = store.get_or_create_task_node(&task_a);

    let output_b = "World".to_string();
    let task_b = StringConstant::new(&output_b);
    let node_b = store.get_or_create_task_node(&task_b);

    assert!(!store.task_has_output(&node_a));
    assert_panics!(store.get_task_output(&node_a));
    assert!(!store.task_has_output(&node_b));
    assert_panics!(store.get_task_output(&node_b));

    store.set_task_output(&node_a, output_a.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(!store.task_has_output(&node_b));
    assert_panics!(store.get_task_output(&node_b));

    store.set_task_output(&node_b, output_b.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(store.task_has_output(&node_b));
    assert_eq!(store.get_task_output(&node_b), &output_b);
  }

  #[test]
  fn test_dependencies() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(output_a.clone());
    let node_a = store.get_or_create_task_node(&task_a);
    let output_b = "World".to_string();
    let task_b = StringConstant::new(output_b.clone());
    let node_b = store.get_or_create_task_node(&task_b);
    let path_c = PathBuf::from("hello.txt");
    let node_c = store.get_or_create_file_node(&path_c);

    assert_eq!(store.get_dependencies_of_task(&node_a).next(), None);
    assert_eq!(store.get_dependencies_of_task(&node_b).next(), None);

    let file_dependency_a2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&node_a, &node_c, file_dependency_a2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    assert_eq!(store.get_dependencies_of_task(&node_b).next(), None);

    let task_dependency_b2a = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_b, &node_a, task_dependency_b2a.clone());
    assert_eq!(result, Ok(()));
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), None);

    let file_dependency_b2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&node_b, &node_c, file_dependency_b2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), Some(&Dependency::RequireFile(file_dependency_b2c.clone())));
    assert_eq!(deps_of_b.get(2), None);

    let task_dependency_a2b = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_a, &node_b, task_dependency_a2b);
    assert_eq!(result, Err(())); // Creates a cycle: error
  }

  #[test]
  fn test_reset() {
    let mut store = Store::default();
    let output = "Hello".to_string();
    let task = StringConstant::new(output.clone());
    let task_node = store.get_or_create_task_node(&task);
    let path = PathBuf::from("hello.txt");
    let file_node = store.get_or_create_file_node(&path);

    store.set_task_output(&task_node, output.clone());
    assert!(store.task_has_output(&task_node));
    assert_eq!(store.get_task_output(&task_node), &output);

    let file_dependency = FileDependency::new(&path, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&task_node, &file_node, file_dependency.clone());
    let deps: Vec<_> = store.get_dependencies_of_task(&task_node).cloned().collect();
    assert_eq!(deps.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps.get(1), None);

    store.reset_task(&task_node);
    assert!(!store.task_has_output(&task_node));
    assert_panics!(store.get_task_output(&task_node));
    assert_eq!(store.get_dependencies_of_task(&task_node).next(), None);
  }
}
