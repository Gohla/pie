

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

    // Add file dependency from task A to file C.
    let file_dependency_a2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&node_a, &node_c, file_dependency_a2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    assert_eq!(store.get_dependencies_of_task(&node_b).next(), None);

    // Add task dependency from task B to task A.
    let task_dependency_b2a = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_b, &node_a, task_dependency_b2a.clone());
    assert_eq!(result, Ok(()));
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), None);

    // Add file dependency from task B to file C.
    let file_dependency_b2c = FileDependency::new(&path_c, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&node_b, &node_c, file_dependency_b2c.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency_a2c.clone())));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireTask(task_dependency_b2a.clone())));
    assert_eq!(deps_of_b.get(1), Some(&Dependency::RequireFile(file_dependency_b2c.clone())));
    assert_eq!(deps_of_b.get(2), None);

    // Add task dependency from task A to task B, creating a cycle.
    let task_dependency_a2b = TaskDependency::new(task_a.clone(), OutputStamper::Equals, output_a.clone());
    let result = store.add_task_require_dependency(&node_a, &node_b, task_dependency_a2b);
    assert_eq!(result, Err(())); // Creates a cycle: error
  }

  #[test]
  #[should_panic]
  fn test_get_dependencies_of_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    let _ = store.get_dependencies_of_task(&fake_node);
  }

  #[test]
  #[should_panic]
  fn test_add_file_require_dependency_panics() {
    let mut fake_store = Store::default();
    let fake_file_node = fake_store.get_or_create_file_node("hello.txt");
    let fake_task_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    let dependency = FileDependency::new("hello.txt", FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&fake_task_node, &fake_file_node, dependency);
  }

  #[test]
  #[should_panic]
  fn test_add_task_require_dependency_panics() {
    let mut fake_store = Store::default();
    let output = "Hello".to_string();
    let task = StringConstant::new(&output);
    let fake_task_node = fake_store.get_or_create_task_node(&task);
    let mut store: Store<StringConstant, String> = Store::default();
    let dependency = TaskDependency::new(task, OutputStamper::Equals, output);
    let _ = store.add_task_require_dependency(&fake_task_node, &fake_task_node, dependency);
  }
