

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
  #[should_panic]
  fn test_task_mapping_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&StringConstant::new("Hello"));
    let store: Store<StringConstant, String> = Store::default();
    store.get_task(&fake_node);
  }
