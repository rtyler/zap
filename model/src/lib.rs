#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::collections::HashMap;
use std::path::PathBuf;

pub mod inventory;
pub mod plan;
pub mod task;
pub mod tasks;
pub mod transport;

pub use crate::plan::Plan;
pub use crate::task::Task;
pub use crate::transport::{Transport, TransportError};

/**
 * An ExecutableTask is a light container over a Task execpt with user-provided information and is
 * therefore ready for execution
 */
#[derive(Clone, Debug)]
pub struct ExecutableTask {
    pub task: Task,
    pub parameters: HashMap<String, String>,
}

impl ExecutableTask {
    pub fn new(task: Task, parameters: HashMap<String, String>) -> Self {
        Self { task, parameters }
    }

    /**
     * Provides will return the files that the ExecutableTask provides
     *
     * If these files exist, the task should not be executed
     */
    pub fn provides() -> Vec<PathBuf> {
        vec![]
    }
}

#[cfg(test)]
mod tests {}
