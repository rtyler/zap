use crate::inventory::{Group, Inventory, Target};
use std::path::Path;
use zap_model::plan::ExecutableTask;

pub mod ssh;

/**
 * The Transport trait allows for multiple transports to be implemented for
 * connecting to targets
 */
pub trait Transport {
    fn connect(&mut self, target: &Target) -> bool;
    fn run_group(
        &mut self,
        cmd: &ExecutableTask,
        group: &Group,
        inv: &Inventory,
        dry_run: bool,
    ) -> i32;
    fn run(&mut self, command: &ExecutableTask, target: &Target, dry_run: bool) -> i32;
    fn send_bytes(&self, remote_path: &Path, bytes: &Vec<u8>, mode: i32) -> bool;
}
