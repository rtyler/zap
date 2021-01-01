#[macro_use]
extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::collections::HashMap;

pub mod plan;
pub mod task;

/**
 * An ExecutableTask is a light container over a Task execpt with user-provided information and is
 * therefore ready for execution
 */
#[derive(Clone, Debug)]
pub struct ExecutableTask {
    pub task: task::Task,
    pub parameters: HashMap<String, String>,
}

impl ExecutableTask {
    pub fn new(task: task::Task, parameters: HashMap<String, String>) -> Self {
        Self { task, parameters }
    }
}
