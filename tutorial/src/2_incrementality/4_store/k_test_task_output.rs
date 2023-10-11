

  #[test]
  fn test_task_outputs() {
    let mut store = Store::default();
    let output_a = "Hello".to_string();
    let task_a = StringConstant::new(&output_a);
    let node_a = store.get_or_create_task_node(&task_a);

    let output_b = "World".to_string();
    let task_b = StringConstant::new(&output_b);
    let node_b = store.get_or_create_task_node(&task_b);

    // Assert that tasks have no output by default.
    assert!(!store.task_has_output(&node_a));
    assert!(!store.task_has_output(&node_b));

    // Set output for task A, assert that A has that output but B is unchanged.
    store.set_task_output(&node_a, output_a.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(!store.task_has_output(&node_b));

    // Set output for task B, assert that B has that output but A is unchanged.
    store.set_task_output(&node_b, output_b.clone());
    assert!(store.task_has_output(&node_a));
    assert_eq!(store.get_task_output(&node_a), &output_a);
    assert!(store.task_has_output(&node_b));
    assert_eq!(store.get_task_output(&node_b), &output_b);
  }

  #[test]
  #[should_panic]
  fn test_task_has_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    store.task_has_output(&fake_node);
  }

  #[test]
  #[should_panic]
  fn test_get_task_output_panics() {
    let mut store = Store::default();
    let node = store.get_or_create_task_node(&StringConstant::new("Hello"));
    store.get_task_output(&node);
  }

  #[test]
  #[should_panic]
  fn test_set_task_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let mut store: Store<StringConstant, String> = Store::default();
    store.set_task_output(&fake_node, "Hello".to_string());
  }
