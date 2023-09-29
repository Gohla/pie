

  #[test]
  fn test_reset() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(output_a.clone());
    let task_a_node = store.get_or_create_task_node(&task_a);
    let output_b = "World".to_string();
    let task_b = StringConstant::new(output_b.clone());
    let task_b_node = store.get_or_create_task_node(&task_b);
    let path = PathBuf::from("hello.txt");
    let file_node = store.get_or_create_file_node(&path);

    // Set outputs for task A and B.
    store.set_task_output(&task_a_node, output_a.clone());
    assert!(store.task_has_output(&task_a_node));
    assert_eq!(store.get_task_output(&task_a_node), &output_a);
    store.set_task_output(&task_b_node, output_b.clone());
    assert!(store.task_has_output(&task_b_node));
    assert_eq!(store.get_task_output(&task_b_node), &output_b);

    // Add file dependency for task A and B.
    let file_dependency = FileDependency::new(&path, FileStamper::Exists).unwrap();
    store.add_file_require_dependency(&task_a_node, &file_node, file_dependency.clone());
    let deps_of_a: Vec<_> = store.get_dependencies_of_task(&task_a_node).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_a.get(1), None);
    store.add_file_require_dependency(&task_b_node, &file_node, file_dependency.clone());
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_b.get(1), None);

    // Reset only task A.
    store.reset_task(&task_a_node);
    // Assert that task A is reset.
    assert!(!store.task_has_output(&task_a_node));
    assert_eq!(store.get_dependencies_of_task(&task_a_node).next(), None);
    // Assert that task B is unchanged.
    assert!(store.task_has_output(&task_b_node));
    assert_eq!(store.get_task_output(&task_b_node), &output_b);
    let deps_of_b: Vec<_> = store.get_dependencies_of_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::RequireFile(file_dependency.clone())));
    assert_eq!(deps_of_b.get(1), None);
  }

  #[test]
  #[should_panic]
  fn test_reset_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    store.reset_task(&fake_node);
  }
