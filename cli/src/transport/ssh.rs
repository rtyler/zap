use crate::inventory::{Group, Inventory, Target};
use crate::transport::Transport;

use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ssh {}

impl Default for Ssh {
    fn default() -> Self {
        Self {}
    }
}

impl Transport for Ssh {
    fn run_group(&self, command: &str, group: &Group, inventory: &Inventory) -> i32 {
        let mut status = 1;
        for target_name in group.targets.iter() {
            // XXX: This is inefficient
            for target in inventory.targets.iter() {
                if &target.name == target_name {
                    println!("Running on `{}`", target.name);
                    status = self.run(command, &target);
                }
            }
        }
        status
    }
    fn run(&self, command: &str, target: &Target) -> i32 {
        // Connect to the local SSH server
        let tcp = TcpStream::connect(format!("{}:22", target.uri)).unwrap();
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake().unwrap();
        sess.userauth_agent(&std::env::var("USER").unwrap())
            .unwrap();

        let mut channel = sess.channel_session().unwrap();
        channel.exec(command).unwrap();
        let mut s = String::new();
        channel.read_to_string(&mut s).unwrap();
        print!("{}", s);
        channel.wait_close().expect("Failed to close the channel");
        return channel.exit_status().unwrap();
    }
}
