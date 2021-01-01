use colored::*;
use gumdrop::Options;
use log::*;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;

use zap_model::inventory::Inventory;
use zap_model::transport::ssh::Ssh;
use zap_model::{Plan, Task, Transport};
use zap_model::ExecutableTask;

fn main() {
    pretty_env_logger::init();
    let opts = ZapOptions::parse_args_default_or_exit();

    if opts.command.is_none() {
        println!("Must specify a subcommand!");
        std::process::exit(1);
    }

    let file =
        std::fs::File::open("inventory.yml").expect("Failed to load the inventory.ymll file");
    let reader = BufReader::new(file);
    let inventory: Inventory = serde_yaml::from_reader(reader).expect("Failed to read intenvory");

    let mut runner = match &inventory.config.transport {
        zap_model::inventory::Transport::Ssh => Ssh::default(),
    };

    match opts.command.unwrap() {
        Command::Cmd(opts) => handle_cmd(opts, &mut runner, inventory),
        Command::Task(opts) => handle_task(opts, &mut runner, inventory),
        Command::Plan(opts) => handle_plan(opts, &mut runner, inventory),
        Command::Check(opts) => handle_check(opts),
        _ => {}
    }
}

/**
 * This function simply attempts to parse the given ztask or zplan files in order
 * to validate that they're properly formatted
 */
fn handle_check(opts: CheckOpts) {
    for file in opts.files.iter() {
        if let Some(ext) = file.as_path().extension() {
            if ext == "ztask" {
                let task = Task::from_path(&file).expect("Failed to parse");
                println!("Parsed task {} properly", task.name);
            }
            if ext == "zplan" {
                let _plan = Plan::from_path(&file).expect("Failed to parse");
                println!("Parsed plan properly");
            }
        }
    }
}

/**
 * This function will parse and execute a plan
 */
fn handle_plan(opts: PlanOpts, runner: &mut dyn Transport, inventory: Inventory) {
    println!("{}", format!("Running plan with: {:?}", opts).green());
    let mut exit: i32 = -1;

    match Plan::from_path(&opts.plan) {
        Ok(plan) => {
            info!("Plan located, preparing to execute");
            for task in plan.tasks {
                info!("Running executable task: {:?}", task);
                exit = execute_task_on(
                    opts.targets.clone(),
                    &task,
                    runner,
                    &inventory,
                    opts.dry_run,
                );
            }
        }
        Err(err) => {
            println!("Failed to load plan: {:?}", err);
        }
    }
    std::process::exit(exit);
}

fn execute_task_on(
    targets: String,
    task: &ExecutableTask,
    runner: &mut dyn Transport,
    inventory: &Inventory,
    dry_run: bool,
) -> i32 {
    if let Some(group) = inventory.groups.iter().find(|g| g.name == targets) {
        return runner.run_group(task, &group, &inventory, dry_run);
    }

    if let Some(target) = inventory.targets.iter().find(|t| t.name == targets) {
        return runner.run(task, &target, dry_run);
    }
    error!("Failed to locate a script to execute for the task!");
    return -1;
}

/**
 * This function will handle a task
 */
fn handle_task(opts: TaskOpts, runner: &mut dyn Transport, inventory: Inventory) {
    println!("{}", format!("Running task with: {:?}", opts).green());

    match Task::from_path(&opts.task) {
        Ok(task) => {
            info!("Task located, preparing to execute");
            let mut parameters = HashMap::new();

            /*
             * XXX: This is very primitive way, there must be a better way to take
             * arbitrary command line parameters than this.
             */
            for parameter in opts.parameter.iter() {
                let parts: Vec<&str> = parameter.split("=").collect();
                if parts.len() == 2 {
                    parameters.insert(parts[0].to_string(), parts[1].to_string());
                }
            }

            let task = ExecutableTask::new(task, parameters);

            std::process::exit(execute_task_on(
                opts.targets,
                &task,
                runner,
                &inventory,
                opts.dry_run,
            ));
        }
        Err(err) => {
            println!("Failed to load task: {:?}", err);
        }
    }
}

/**
 * This function will handle executing a single specified command on the target(s)
 * identified in the `opts`.
 *
 * In the case of a single target, the status code from the executed command will
 * be propogated up.
 *
 * In the case of multiple targets, any non-zero status code will be used to exit
 * non-zero.
 */
fn handle_cmd(opts: CmdOpts, runner: &mut dyn Transport, inventory: Inventory) {
    let mut task = ExecutableTask::new(Task::new("Dynamic"), HashMap::new());
    task.task.script.inline = Some(opts.command);
    std::process::exit(execute_task_on(
        opts.targets,
        &task,
        runner,
        &inventory,
        false,
    ));
}

#[derive(Debug, Options)]
struct ZapOptions {
    // Options here can be accepted with any command (or none at all),
    // but they must come before the command name.
    #[options(help = "print help message")]
    help: bool,
    #[options(help = "be verbose")]
    verbose: bool,

    // The `command` option will delegate option parsing to the command type,
    // starting at the first free argument.
    #[options(command)]
    command: Option<Command>,
}

// The set of commands and the options each one accepts.
//
// Each variant of a command enum should be a unary tuple variant with only
// one field. This field must implement `Options` and is used to parse arguments
// that are given after the command name.
#[derive(Debug, Options)]
enum Command {
    // Command names are generated from variant names.
    // By default, a CamelCase name will be converted into a lowercase,
    // hyphen-separated name; e.g. `FooBar` becomes `foo-bar`.
    //
    // Names can be explicitly specified using `#[options(name = "...")]`
    #[options(help = "show help for a command")]
    Help(HelpOpts),
    #[options(help = "Run a single command on a target(s)")]
    Cmd(CmdOpts),
    #[options(help = "Execute a task on a target(s)")]
    Task(TaskOpts),
    #[options(help = "Execute a plan on a target(s)")]
    Plan(PlanOpts),
    #[options(help = "Check that the specified .ztask or .zplan file is valid")]
    Check(CheckOpts),
}

#[derive(Debug, Options)]
struct HelpOpts {
    #[options(free)]
    free: Vec<String>,
}
#[derive(Debug, Options)]
struct CmdOpts {
    #[options(free, help = "Command to execute on the target(s)")]
    command: String,
    #[options(help = "Name of a target or group")]
    targets: String,
}

#[derive(Debug, Options)]
struct TaskOpts {
    #[options(free, help = "Task to execute")]
    task: PathBuf,
    #[options(short = "p", help = "Parameter values")]
    parameter: Vec<String>,
    #[options(help = "Name of a target or group")]
    targets: String,
    #[options(help = "Run the task in dry-run mode")]
    dry_run: bool,
}

#[derive(Debug, Options)]
struct PlanOpts {
    #[options(free, help = "Plan to execute")]
    plan: PathBuf,
    #[options(help = "Name of a target or group")]
    targets: String,
    #[options(help = "Run the task in dry-run mode")]
    dry_run: bool,
}

#[derive(Debug, Options)]
struct CheckOpts {
    #[options(free, help = "Files to check")]
    files: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {}
