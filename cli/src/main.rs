use gumdrop::Options;
use serde::Deserialize;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::Session;

#[derive(Clone, Debug, Deserialize)]
struct Inventory {
    groups: Vec<Group>,
    targets: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize)]
struct Target {
    name: String,
    uri: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Group {
    name: String,
    targets: Vec<String>,
}

fn main() {
    let opts = MyOptions::parse_args_default_or_exit();

    if opts.command.is_none() {
        println!("Must specify a subcommand!");
        std::process::exit(1);
    }

    let file = std::fs::File::open("inventory.yml").expect("Failed to load the inventory.ymll file");
    let reader = BufReader::new(file);
    let inventory: Inventory = serde_yaml::from_reader(reader).expect("Failed to read intenvory");

    match opts.command.unwrap() {
        Command::Cmd(runopts) => {
            if let Some(group) = inventory.groups.iter().find(|g| g.name == runopts.targets) {
                std::process::exit(run_group(&runopts.command, &group, &inventory));
            }

            if let Some(target) = inventory.targets.iter().find(|t| t.name == runopts.targets) {
                println!("run a command: {:?}", runopts);
                std::process::exit(run(&runopts.command, &target));
            }

            println!("Couldn't find a target named `{}`", runopts.targets);
        },
        _ => {},
    }
}

fn run_group(command: &str, group: &Group, inventory: &Inventory) -> i32 {
    let mut status = 1;
    for target_name in group.targets.iter() {
        // XXX: This is inefficient
        for target in inventory.targets.iter() {
            if &target.name == target_name {
                println!("Running on `{}`", target.name);
                status = run(command, &target);
            }
        }
    }
    status
}

fn run(command: &str, target: &Target) -> i32 {
    // Connect to the local SSH server
    let tcp = TcpStream::connect(format!("{}:22", target.uri)).unwrap();
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_agent(&std::env::var("USER").unwrap()).unwrap();

    let mut channel = sess.channel_session().unwrap();
    channel.exec(command).unwrap();
    let mut s = String::new();
    channel.read_to_string(&mut s).unwrap();
    print!("{}", s);
    channel.wait_close().expect("Failed to close the channel");
    return channel.exit_status().unwrap();
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
    #[options(help="Run a single command on a target(s)")]
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
    #[options(free, help="Command to execute on the target(s)")]
    command: String,
    #[options(help = "Name of a target or group")]
    targets: String,
}
