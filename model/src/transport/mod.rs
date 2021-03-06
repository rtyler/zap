use crate::inventory::{Group, Inventory, Target};
use crate::ExecutableTask;

use std::path::Path;

pub mod ssh;

pub enum TransportError {
    GeneralError(String),
}

/**
 * The Transport trait allows for multiple transports to be implemented for
 * connecting to targets
 */
pub trait Transport {
    fn connect(&mut self, target: &Target) -> bool;
    fn disconnect(&mut self);
    fn file_exists(&self, path: &Path) -> Result<bool, TransportError>;
    fn run(&mut self, command: &ExecutableTask, target: &Target, dry_run: bool) -> i32;
    fn run_script(&mut self, script: &str) -> i32;
    fn run_group(
        &mut self,
        cmd: &ExecutableTask,
        group: &Group,
        inv: &Inventory,
        dry_run: bool,
    ) -> i32;
    fn send_bytes(&self, remote_path: &Path, bytes: &Vec<u8>, mode: i32) -> bool;
}
