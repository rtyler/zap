use crate::inventory::{Group, Inventory, Target};

pub mod ssh;

pub trait Transport {
    fn run_group(&self, cmd: &str, group: &Group, inv: &Inventory) -> i32;
    fn run(&self, command: &str, target: &Target) -> i32;
}
