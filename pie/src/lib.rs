// Using Eq/PartialEq/Hash as trait objects: https://users.rust-lang.org/t/workaround-for-hash-trait-not-being-object-safe/53332/8 and https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=3a6d8b0a2e45ee2392b68f36c79d6173 and https://github.com/dtolnay/dyn-clone

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

use anymap::AnyMap;
use dyn_clone::DynClone;

// Trait object helpers

pub trait DynEq {
  fn dyn_eq(&self, other: &dyn Any) -> bool;
}

impl<T: Eq + Any> DynEq for T {
  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    if let Some(other) = other.downcast_ref::<Self>() {
      self == other
    } else {
      false
    }
  }
}

pub trait DynHash {
  fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<H: Hash + ?Sized> DynHash for H {
  #[inline]
  fn dyn_hash(&self, mut state: &mut dyn Hasher) {
    self.hash(&mut state);
  }
}


// Require task

pub trait RequireTask {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>>;
}


// Context

pub trait Context {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>>;
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind>;
  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind>;
}

pub struct ContextImpl<'rt, RT> {
  current_task: Box<dyn DynTask>,
  store: Rc<RefCell<Store>>,
  require_task: &'rt mut RT,
}

impl<'rt, RT> ContextImpl<'rt, RT> {
  fn new(current_task: Box<dyn DynTask>, store: Rc<RefCell<Store>>, require_task: &'rt mut RT) -> Self { Self { current_task, store, require_task } }
}

impl<'rt, RT: RequireTask> Context for ContextImpl<'rt, RT> {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    let result = self.require_task.require_task(task);
    // TODO: create require dependency from current task to task.
    todo!()
  }
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    // TODO: create require dependency from current task to file.
    todo!()
  }
  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    // TODO: create provide dependency from current task to file.
    todo!()
  }
}


// Task + implementations

pub trait Task: DynTask + Eq + Hash + Clone + 'static {
  type Output: Eq + Clone + 'static;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}

pub trait DynTask: DynEq + DynHash + DynClone + 'static {}

impl<T: Task> DynTask for T {}

// Noop task

#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NoopTask {}

impl Task for NoopTask {
  type Output = ();
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output { () }
}

// Read file to string task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ReadFileToString {
  path: PathBuf,
}

impl Task for ReadFileToString {
  // Use ErrorKind instead of Error which impls Eq and Clone.
  type Output = Result<String, std::io::ErrorKind>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = context.require_file(&self.path)?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|e| e.kind())?;
    Ok(string)
  }
}


// Dependency + implementations

pub trait Dependency<C: Context> {
  fn is_consistent(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>>;
}

// Task dependency

#[derive(Clone)]
pub struct TaskDependency<T> {
  task: T,
}

impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T) -> Self { Self { task } }
}

impl<T: Task, C: Context> Dependency<C> for TaskDependency<T> {
  #[inline]
  fn is_consistent(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
    if let Some(previous_output) = store.get_task_output::<T>(&self.task) {
      let output = context.require_task::<T>(&self.task)?;
      return Ok(output == *previous_output);
    }
    Ok(false) // Has not been executed before
  }
}

// File dependency

#[derive(Clone)]
pub struct FileDependency {
  path: PathBuf,
}

impl FileDependency {
  #[inline]
  pub fn new(path: PathBuf) -> Self { Self { path } }
  #[inline]
  fn open(&self) -> std::io::Result<File> { File::open(&self.path) }
}

impl<C: Context> Dependency<C> for FileDependency {
  #[inline]
  fn is_consistent(&self, _context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
    let consistent = if let Some(previous_modified) = store.file_modification_dates.get(&self.path) {
      let modified = self.open()?.metadata()?.modified()?;
      modified == *previous_modified
    } else {
      false
    };
    Ok(consistent)
  }
}


// Store

pub struct Store {
  task_outputs: AnyMap,
  task_dependencies: AnyMap,
  file_modification_dates: HashMap<PathBuf, SystemTime>,
}

impl Store {
  #[inline]
  fn new() -> Self {
    Self {
      task_outputs: AnyMap::new(),
      task_dependencies: AnyMap::new(),
      file_modification_dates: HashMap::default()
    }
  }

  #[inline]
  fn get_task_dependencies_map_mut<T: Task, C: Context>(&mut self) -> &mut HashMap<T, Vec<Box<dyn Dependency<C>>>> {
    self.task_dependencies.entry::<HashMap<T, Vec<Box<dyn Dependency<C>>>>>().or_insert_with(|| HashMap::default())
  }

  #[inline]
  fn get_task_output_map<T: Task>(&self) -> Option<&HashMap<T, T::Output>> {
    self.task_outputs.get::<HashMap<T, T::Output>>()
  }
  #[inline]
  fn get_task_output_map_mut<T: Task>(&mut self) -> &mut HashMap<T, T::Output> {
    self.task_outputs.entry::<HashMap<T, T::Output>>().or_insert_with(|| HashMap::default())
  }
  #[inline]
  fn get_task_output<T: Task>(&self, task: &T) -> Option<&T::Output> {
    self.get_task_output_map::<T>().map_or(None, |map| map.get(task))
  }
  #[inline]
  fn set_task_output<T: Task>(&mut self, task: T, output: T::Output) {
    self.get_task_output_map_mut::<T>().insert(task, output);
  }
}


// Runners


// Naive runner

pub struct NaiveRunner {}

impl RequireTask for NaiveRunner {
  #[inline]
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    Ok(task.execute(self))
  }
}

impl Context for NaiveRunner {
  #[inline]
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    Ok(task.execute(self))
  }
  #[inline]
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    File::open(path).map_err(|e| e.kind())
  }
  #[inline]
  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    File::open(path).map_err(|e| e.kind())
  }
}


// Top-down incremental runner

pub struct TopDownRunner {
  store: Rc<RefCell<Store>>,
}

impl RequireTask for TopDownRunner {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    let mut context = ContextImpl::new(Box::new(task.clone()), self.store.clone(), self);
    if self.should_execute_task(task, &mut context)? {
      TaskExecutor::execute(task, &mut context, &mut self.store)
    } else {
      // Assume: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.store.borrow_mut().get_task_output::<T>(task).unwrap().clone();
      Ok(output)
    }
  }
}

impl TopDownRunner {
  fn should_execute_task<'rt, T: Task>(&'rt mut self, task: &T, context: &mut ContextImpl<'rt, Self>) -> Result<bool, Box<dyn Error>> {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.store.borrow_mut().get_task_dependencies_map_mut::<T, ContextImpl<'rt, Self>>().remove(task);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        if !task_dependency.is_consistent(context, &mut self.store.borrow_mut())? {
          return Ok(true);
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.store.borrow_mut().get_task_dependencies_map_mut::<T, ContextImpl<'rt, Self>>().insert(task.clone(), task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      Ok(false)
    } else {
      // Task has not been executed before, therefore we need to execute it.
      Ok(true)
    }
  }
}

// Task executor

pub struct TaskExecutor {}

impl TaskExecutor {
  fn execute<T: Task, C: Context>(task: &T, context: &mut C, store: &RefCell<Store>) -> Result<T::Output, Box<dyn Error>> {
    let output = task.execute(context);
    store.borrow_mut().set_task_output(task.clone(), output.clone());
    // TODO: store dependencies that the task made!
    Ok(output)
  }
}
