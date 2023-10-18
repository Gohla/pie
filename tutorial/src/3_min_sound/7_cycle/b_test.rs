

// Cycle tests

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_panics() {
  let mut pie = test_pie();
  pie.require(&RequireSelf).unwrap();
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_a_panics() {
  let mut pie = test_pie();
  pie.require(&RequireA).unwrap();
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_b_panics() {
  let mut pie = test_pie();
  pie.require(&RequireB).unwrap();
}
