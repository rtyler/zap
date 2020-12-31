use colored::*;
use gumdrop::Options;
use log::*;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;

mod inventory;
mod transport;

use crate::inventory::*;
use crate::transport::ssh::Ssh;
use zap_parser::*;

fn main() {
    pretty_env_logger::init();
    let opts = MyOptions::parse_args_default_or_exit();

    if opts.command.is_none() {
        println!("Must specify a subcommand!");
        std::process::exit(1);
    }

    let file =
        std::fs::File::open("inventory.yml").expect("Failed to load the inventory.ymll file");
    let reader = BufReader::new(file);
    let inventory: Inventory = serde_yaml::from_reader(reader).expect("Failed to read intenvory");

    let runner = match &inventory.config.transport {
        crate::inventory::Transport::Ssh => Ssh::default(),
    };

    match opts.command.unwrap() {
        Command::Cmd(opts) => handle_cmd(opts, &runner, inventory),
        Command::Task(opts) => handle_task(opts, &runner, inventory),
        _ => {}
    }
}

/**
 * This function will handle a task
 */
fn handle_task(opts: TaskOpts, runner: &dyn crate::transport::Transport, inventory: Inventory) {
    println!("running task: {:?}", opts);

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

            if let Some(script) = task.get_script() {
                let command = render_command(&script, &parameters);

                // TODO: refactor with handle_cmd
                if let Some(group) = inventory.groups.iter().find(|g| g.name == opts.targets) {
                    std::process::exit(runner.run_group(&command, &group, &inventory));
                }

                if let Some(target) = inventory.targets.iter().find(|t| t.name == opts.targets) {
                    std::process::exit(runner.run(&command, &target));
                }
            }
        },
        Err(err) => {
            println!("Failed to load task: {:?}", err);
        },
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
fn handle_cmd(opts: CmdOpts, runner: &dyn crate::transport::Transport, inventory: Inventory) {
    if let Some(group) = inventory.groups.iter().find(|g| g.name == opts.targets) {
        std::process::exit(runner.run_group(&opts.command, &group, &inventory));
    }

    if let Some(target) = inventory.targets.iter().find(|t| t.name == opts.targets) {
        println!("{}", format!("run a command: {:?}", opts).green());
        std::process::exit(runner.run(&opts.command, &target));
    }

    println!(
        "{}",
        format!("Couldn't find a target named `{}`", opts.targets).red()
    );
}

/**
 * render_command will handle injecting the parameters for a given command
 * into the string where appropriate, using the Handlebars syntax.
 *
 * If the template fails to render, then this will just return the command it
 * was given
 */
fn render_command(cmd: &str, parameters: &HashMap<String, String>) -> String {
    use handlebars::Handlebars;

    let handlebars = Handlebars::new();
    match handlebars.render_template(cmd, parameters) {
        Ok(rendered) => {
            return rendered;
        }
        Err(err) => {
            error!("Failed to render command ({:?}): {}", err, cmd);
            return cmd.to_string();
        }
    }
}

#[derive(Debug, Options)]
struct MyOptions {
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
    #[options(free, help = "Task to execute, must exist in ZAP_PATH")]
    task: PathBuf,
    #[options(short = "p", help = "Parameter values")]
    parameter: Vec<String>,
    #[options(help = "Name of a target or group")]
    targets: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_command() {
        let cmd = "echo \"{{msg}}\"";
        let mut params = HashMap::new();
        params.insert("msg".to_string(), "hello".to_string());

        let output = render_command(&cmd, &params);
        assert_eq!(output, "echo \"hello\"");
    }

    #[test]
    fn test_render_command_bad_template() {
        let cmd = "echo \"{{msg\"";
        let mut params = HashMap::new();
        params.insert("msg".to_string(), "hello".to_string());

        let output = render_command(&cmd, &params);
        assert_eq!(output, "echo \"{{msg\"");
    }
}
