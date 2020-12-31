use crate::inventory::{Group, Inventory, Target};

pub mod ssh;

/**
 * The Transport trait allows for multiple transports to be implemented for
 * connecting to targets
 */
pub trait Transport {
    fn run_group(&self, cmd: &str, group: &Group, inv: &Inventory) -> i32;
    fn run(&self, command: &str, target: &Target) -> i32;
}
