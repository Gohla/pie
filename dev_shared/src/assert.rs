use std::panic;

pub fn catch_unwind_silent<F: FnOnce() -> R, R>(f: F) -> std::thread::Result<R> {
  // Source: https://stackoverflow.com/a/59211519
  let prev_hook = panic::take_hook();
  panic::set_hook(Box::new(|_| {}));
  let result = panic::catch_unwind(panic::AssertUnwindSafe(f)); // Note: forcing unwind safety for ease of use in tests.
  panic::set_hook(prev_hook);
  result
}

#[macro_export]
macro_rules! assert_panics {
  ($e:expr) => { assert!($crate::assert::catch_unwind_silent(||$e).is_err(), "expected panic"); }
}
