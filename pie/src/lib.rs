// Using Eq/PartialEq/Hash as trait objects: https://users.rust-lang.org/t/workaround-for-hash-trait-not-being-object-safe/53332/8 and https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=3a6d8b0a2e45ee2392b68f36c79d6173 and https://github.com/dtolnay/dyn-clone

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::time::SystemTime;

use anymap::AnyMap;
use dyn_clone::{clone_box, DynClone};

// Trait object helpers

pub trait AsAny {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
  #[inline]
  fn as_any(&self) -> &dyn Any { self }
}

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
  fn dyn_hash(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
}


// Context

pub trait Context where
// Note: 'static bound because it is used as a type parameter in Dependency, which in turn is stored in an AnyMap, which
// requires it to be 'static. Cannot move the type parameter to the method, because generic methods are not supported in 
// trait objects, and Dependency is used as a trait object.
  Self: 'static
{
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>>;
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind>;
  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind>;
}


// Task + implementations

pub trait Task: DynTask + Eq + Hash + Clone + 'static {
  type Output: Eq + Clone + 'static;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}

pub trait DynTask: DynEq + DynHash + DynClone + AsAny + 'static {}

impl<T: Task> DynTask for T {}

impl PartialEq for dyn DynTask {
  fn eq(&self, other: &dyn DynTask) -> bool {
    DynEq::dyn_eq(self, other.as_any())
  }
}

impl Eq for dyn DynTask {}

impl Hash for dyn DynTask {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.dyn_hash(state);
  }
}

dyn_clone::clone_trait_object!(DynTask);

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

pub trait Dependency<C: Context + GetStore> {
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>>;
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

impl<T: Task, C: Context + GetStore> Dependency<C> for TaskDependency<T> {
  #[inline]
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    if let Some(previous_output) = context.get_store_mut().get_task_output::<T>(&self.task).cloned() { // OPTO: remove clone
      let output = context.require_task::<T>(&self.task)?;
      return Ok(output == previous_output);
    }
    Ok(false) // Has not been executed before
  }
}

// File dependency

#[derive(Clone)]
pub struct RequireFileDependency {
  path: PathBuf,
}

impl RequireFileDependency {
  #[inline]
  pub fn new(path: PathBuf) -> Self { Self { path } }
  #[inline]
  fn open(&self) -> Result<File, std::io::ErrorKind> { File::open(&self.path).map_err(|e| e.kind()) }
}

impl<C: Context + GetStore> Dependency<C> for RequireFileDependency {
  #[inline]
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    let consistent = if let Some(previous_modified) = context.get_store().required_file_modification_dates.get(&self.path) {
      let modified = self.open().map_err(|ek| std::io::Error::from(ek))?.metadata()?.modified()?;
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
  required_file_modification_dates: HashMap<PathBuf, SystemTime>,
  provided_file_modification_dates: HashMap<PathBuf, SystemTime>,
}

impl Store {
  #[inline]
  fn new() -> Self {
    Self {
      task_outputs: AnyMap::new(),
      task_dependencies: AnyMap::new(),
      required_file_modification_dates: HashMap::default(),
      provided_file_modification_dates: HashMap::default(),
    }
  }

  #[inline]
  fn get_task_dependencies_map_mut<C: Context>(&mut self) -> &mut HashMap<Box<dyn DynTask>, Vec<Box<dyn Dependency<C>>>> {
    self.task_dependencies.entry::<HashMap<Box<dyn DynTask>, Vec<Box<dyn Dependency<C>>>>>().or_insert_with(|| HashMap::default())
  }
  #[inline]
  fn remove_task_dependencies<C: Context>(&mut self, task: &dyn DynTask) -> Option<Vec<Box<dyn Dependency<C>>>> {
    self.get_task_dependencies_map_mut::<C>().remove(task)
  }
  #[inline]
  fn set_task_dependencies<C: Context>(&mut self, task: Box<dyn DynTask>, dependencies: Vec<Box<dyn Dependency<C>>>) {
    self.get_task_dependencies_map_mut::<C>().insert(task, dependencies);
  }
  #[inline]
  fn add_to_task_dependencies<C: Context>(&mut self, task: Box<dyn DynTask>, dependency: Box<dyn Dependency<C>>) {
    let dependencies = self.get_task_dependencies_map_mut::<C>().entry(task).or_insert_with(|| Vec::new());
    dependencies.push(dependency);
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

  #[inline]
  fn get_required_file_modification_date(&self, file: &PathBuf) -> Option<&SystemTime> {
    self.required_file_modification_dates.get(file)
  }
  #[inline]
  fn set_required_file_modification_date(&mut self, file: &PathBuf, modification_date: SystemTime) {
    self.required_file_modification_dates.insert(file.clone(), modification_date);
  }
}

pub trait GetStore {
  fn get_store(&self) -> &Store;
  fn get_store_mut(&mut self) -> &mut Store;
}


// Runners


// Naive runner

pub struct NaiveRunner {}

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
  store: Store,
  task_execution_stack: Vec<Box<dyn DynTask>>,
}

impl Context for TopDownRunner {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    if self.should_execute_task(task)? {
      if let Some(current_task) = self.task_execution_stack.last() {
        self.store.add_to_task_dependencies::<Self>(current_task.clone(), Box::new(TaskDependency::new(task.clone())));
      }
      self.task_execution_stack.push(Box::new(task.clone())); // TODO: check for cycles with LinkedHashSet!
      let output = task.execute(self);
      self.store.set_task_output(task.clone(), output.clone());
      Ok(output)
    } else {
      // Assume: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.store.get_task_output::<T>(task).unwrap().clone();
      Ok(output)
    }
  }

  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    let dependency = RequireFileDependency::new(path.clone());
    if let Some(current_task) = self.task_execution_stack.last() {
      self.store.add_to_task_dependencies::<Self>(current_task.clone(), Box::new(dependency.clone()));
      // TODO: store task -> required stamp now.
    }
    dependency.open()
  }

  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> {
    let dependency = RequireFileDependency::new(path.clone()); // TODO: provided file dependency
    if let Some(current_task) = self.task_execution_stack.last() {
      self.store.add_to_task_dependencies::<Self>(current_task.clone(), Box::new(dependency.clone()));
      // TODO: store task -> provided stamp now.
    }
    dependency.open()
  }
}

impl GetStore for TopDownRunner {
  fn get_store(&self) -> &Store {
    &self.store
  }

  fn get_store_mut(&mut self) -> &mut Store {
    &mut self.store
  }
}

impl TopDownRunner {
  fn should_execute_task(&mut self, task: &dyn DynTask) -> Result<bool, Box<dyn Error>> {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.store.remove_task_dependencies(task);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        if !task_dependency.is_consistent(self)? {
          return Ok(true);
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.store.set_task_dependencies(clone_box(task), task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      Ok(false)
    } else {
      // Task has not been executed before, therefore we need to execute it.
      Ok(true)
    }
  }
}
