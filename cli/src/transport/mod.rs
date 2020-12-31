use crate::inventory::{Group, Inventory, Target};
use zap_parser::plan::ExecutableTask;

pub mod ssh;

/**
 * The Transport trait allows for multiple transports to be implemented for
 * connecting to targets
 */
pub trait Transport {
    fn run_group(&self, cmd: &ExecutableTask, group: &Group, inv: &Inventory, dry_run: bool)
        -> i32;
    fn run(&self, command: &ExecutableTask, target: &Target, dry_run: bool) -> i32;
}
