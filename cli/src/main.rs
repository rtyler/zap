use colored::*;
use gumdrop::Options;
use std::io::BufReader;

mod inventory;
mod transport;

use crate::inventory::*;
use crate::transport::ssh::Ssh;
use crate::transport::Transport;

fn main() {
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
        Command::Cmd(runopts) => {
            if let Some(group) = inventory.groups.iter().find(|g| g.name == runopts.targets) {
                std::process::exit(runner.run_group(&runopts.command, &group, &inventory));
            }

            if let Some(target) = inventory.targets.iter().find(|t| t.name == runopts.targets) {
                println!("{}", format!("run a command: {:?}", runopts).green());
                std::process::exit(runner.run(&runopts.command, &target));
            }

            println!("{}", format!("Couldn't find a target named `{}`", runopts.targets).red());
        }
        _ => {}
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
    Cmd(RunOpts),
}

#[derive(Debug, Options)]
struct HelpOpts {
    #[options(free)]
    free: Vec<String>,
}

// Options accepted for the `make` command
#[derive(Debug, Options)]
struct RunOpts {
    #[options(free, help = "Command to execute on the target(s)")]
    command: String,
    #[options(help = "Name of a target or group")]
    targets: String,
}
