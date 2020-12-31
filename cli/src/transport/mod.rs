use crate::inventory::{Group, Inventory, Target};
use std::collections::HashMap;


pub type EnvVars = HashMap<String, String>;

pub mod ssh;

/**
 * The Transport trait allows for multiple transports to be implemented for
 * connecting to targets
 */
pub trait Transport {
    fn run_group(&self, cmd: &str, group: &Group, inv: &Inventory, env: Option<EnvVars>) -> i32;
    fn run(&self, command: &str, target: &Target, env: Option<&EnvVars>) -> i32;
}
