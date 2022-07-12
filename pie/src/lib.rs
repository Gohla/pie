// Using Eq/PartialEq/Hash as trait objects: https://users.rust-lang.org/t/workaround-for-hash-trait-not-being-object-safe/53332/8 
// and https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=3a6d8b0a2e45ee2392b68f36c79d6173 
// and https://github.com/dtolnay/dyn-clone

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
use hashlink::LinkedHashSet;

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

pub trait Context {
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

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ReadFileToString {
  path: PathBuf,
}

impl ReadFileToString {
  pub fn new(path: PathBuf) -> Self { Self { path } }
}

impl Task for ReadFileToString {
  // Use ErrorKind instead of Error which impls Eq and Clone.
  type Output = Result<String, std::io::ErrorKind>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    println!("Executing {:?}", self);
    let mut file = context.require_file(&self.path)?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|e| e.kind())?;
    Ok(string)
  }
}


// Dependency + implementations

pub trait Dependency<C: Context> {
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>>;
}

// Task dependency

#[derive(Clone)]
pub struct TaskDependency<T: Task> {
  task: T,
  output: T::Output,
}

impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T, output: T::Output) -> Self { Self { task, output } }
}

impl<T: Task, C: Context> Dependency<C> for TaskDependency<T> {
  #[inline]
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    let output = context.require_task::<T>(&self.task)?;
    return Ok(output == self.output);
  }
}

// File dependency

#[derive(Clone)]
pub struct FileDependency {
  path: PathBuf,
  modification_date: SystemTime,
}

impl FileDependency {
  #[inline]
  fn new(path: PathBuf) -> Result<Self, std::io::Error> {
    let modification_date = File::open(&path)?.metadata()?.modified()?;
    Ok(Self { path, modification_date })
  }
  #[inline]
  fn open(&self) -> Result<File, std::io::ErrorKind> { File::open(&self.path).map_err(|e| e.kind()) }
}

impl<C: Context> Dependency<C> for FileDependency {
  #[inline]
  fn is_consistent(&self, _context: &mut C) -> Result<bool, Box<dyn Error>> {
    let modification_date = self.open().map_err(|ek| std::io::Error::from(ek))?.metadata()?.modified()?;
    Ok(modification_date == self.modification_date)
  }
}


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
  task_outputs: AnyMap,
  task_dependencies: HashMap<Box<dyn DynTask>, Vec<Box<dyn Dependency<Self>>>>,
  task_execution_stack: LinkedHashSet<Box<dyn DynTask>>,
}

impl Context for TopDownRunner {
  fn require_task<T: Task>(&mut self, task: &T) -> Result<T::Output, Box<dyn Error>> {
    if self.should_execute_task(task)? {
      if !self.task_execution_stack.insert(Box::new(task.clone())) {
        panic!("Cycle");
      }
      let output = task.execute(self);
      self.task_execution_stack.pop_back();
      if let Some(current_task) = self.task_execution_stack.back() {
        self.add_to_task_dependencies(current_task.clone(), Box::new(TaskDependency::new(task.clone(), output.clone())));
      }
      self.set_task_output(task.clone(), output.clone());
      Ok(output)
    } else {
      // Assume: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.get_task_output::<T>(task).unwrap().clone();
      Ok(output)
    }
  }

  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> { // TODO: hidden dependency detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_task) = self.task_execution_stack.back() {
      self.add_to_task_dependencies(current_task.clone(), Box::new(dependency));
    }
    opened
  }

  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::ErrorKind> { // TODO: hidden dependency detection
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_task) = self.task_execution_stack.back() {
      self.add_to_task_dependencies(current_task.clone(), Box::new(dependency));
    }
    opened
  }
}

impl TopDownRunner {
  pub fn new() -> Self {
    Self {
      task_outputs: AnyMap::new(),
      task_dependencies: HashMap::new(),
      task_execution_stack: LinkedHashSet::new(),
    }
  }

  fn should_execute_task(&mut self, task: &dyn DynTask) -> Result<bool, Box<dyn Error>> {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.remove_task_dependencies(task);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        if !task_dependency.is_consistent(self)? {
          return Ok(true);
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.set_task_dependencies(clone_box(task), task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      Ok(false)
    } else {
      // Task has not been executed before, therefore we need to execute it.
      Ok(true)
    }
  }

  #[inline]
  fn remove_task_dependencies(&mut self, task: &dyn DynTask) -> Option<Vec<Box<dyn Dependency<Self>>>> {
    self.task_dependencies.remove(task)
  }
  #[inline]
  fn set_task_dependencies(&mut self, task: Box<dyn DynTask>, dependencies: Vec<Box<dyn Dependency<Self>>>) {
    self.task_dependencies.insert(task, dependencies);
  }
  #[inline]
  fn add_to_task_dependencies(&mut self, task: Box<dyn DynTask>, dependency: Box<dyn Dependency<Self>>) {
    let dependencies = self.task_dependencies.entry(task).or_insert_with(|| Vec::new());
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
}

#[cfg(test)]
mod test {
  use std::fs;
  use std::path::PathBuf;

  use crate::{Context, ReadFileToString, Task, TopDownRunner};

  #[test]
  fn test() {
    let mut runner = TopDownRunner::new();
    let path = PathBuf::from("../target/test/test.txt");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "test").unwrap();
    let task = ReadFileToString::new(path);
    runner.require_task(&task).unwrap().unwrap();
    runner.require_task(&task).unwrap().unwrap();
  }

  #[test]
  #[should_panic(expected = "Cycle")]
  fn cycle_panics() {
    let mut runner = TopDownRunner::new();
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct RequireSelf;
    impl Task for RequireSelf {
      type Output = ();
      fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
        context.require_task(self).unwrap();
      }
    }
    runner.require_task(&RequireSelf).unwrap();
  }
}
