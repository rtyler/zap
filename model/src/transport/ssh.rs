use crate::inventory::{Group, Inventory, Target};
use crate::transport::Transport;
use crate::{ExecutableTask, TransportError};

use colored::*;

use log::*;
use ssh2::Session;
use std::convert::TryInto;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;
use std::path::Path;

const REMOTE_SCRIPT: &str = "._zap_command";

#[derive(Clone)]
pub struct Ssh {
    session: Session,
    connected: bool,
}

impl Default for Ssh {
    fn default() -> Self {
        Self {
            session: Session::new().unwrap(),
            connected: false,
        }
    }
}

impl Transport for Ssh {
    fn run_group(
        &mut self,
        command: &ExecutableTask,
        group: &Group,
        inventory: &Inventory,
        dry_run: bool,
    ) -> i32 {
        let mut status = 1;
        for target_name in group.targets.iter() {
            // XXX: This is inefficient
            for target in inventory.targets.iter() {
                if &target.name == target_name {
                    println!("Running on `{}` {}", target.name, target.uri);
                    status = self.run(command, &target, dry_run);
                    self.disconnect();
                }
            }
        }
        status
    }

    fn disconnect(&mut self) {
        debug!("Disconnecting");
        if self.connected {
            self.session.disconnect(None, "Zappidy doo-da", None);
            // There doesn't seem to be any cleaner way to close other than
            //.just dropping the session
            self.session = Session::new().unwrap();
        }
        self.connected = false;
    }

    fn connect(&mut self, target: &Target) -> bool {
        if self.connected {
            return self.connected;
        }
        debug!("Connecting to {}", target.uri);
        let tcp = TcpStream::connect(format!("{}:22", target.uri)).unwrap();
        self.session.set_tcp_stream(tcp);
        self.session.handshake().unwrap();

        let mut authenticated = false;

        if let Some(config) = &target.config {
            if let Some(sshconfig) = &config.ssh {
                // requires PasswordAuthentication yes
                if sshconfig.password.is_some() {
                    self.session
                        .userauth_password(&sshconfig.user, sshconfig.password.as_ref().unwrap())
                        .unwrap();
                    authenticated = true;
                } else if sshconfig.privatekey_path.is_some() {
                    let privatekey_path = sshconfig.privatekey_path.as_ref().unwrap();
                    let privatekey_path = Path::new(&privatekey_path);
                    self.session
                        .userauth_pubkey_file(&sshconfig.user, None, privatekey_path, None)
                        .unwrap();
                    authenticated = true;
                } else {
                    panic!("one of sshconfig.password or sshconfig.privatekey_path is required");
                }
            }
        }
        if !authenticated {
            self.session
                .userauth_agent(&std::env::var("USER").unwrap())
                .unwrap();
        }

        self.connected = true;
        true
    }

    fn file_exists(&self, path: &Path) -> Result<bool, TransportError> {
        if let Err(error) = self.session.scp_recv(path) {
            if error.code() == ssh2::ErrorCode::Session(-28) {
                debug!("The file ({}) does not exist", path.display());
            } else {
                error!(
                    "A failure occurred while trying to check a file exists: {:?}",
                    error
                );
                return Err(TransportError::GeneralError(
                    "Failed to check that file exists".into(),
                ));
            }
        } else {
            // If we successfully fetched the provided file, then we should
            // return 0 and skip the function
            trace!("The file exists: {}", path.display());
            return Ok(true);
        }
        return Ok(false);
    }

    /**
     * run_script will copy the given string over and execute it
     */
    fn run_script(&mut self, script: &str) -> i32 {
        if self.send_bytes(Path::new(REMOTE_SCRIPT), &script.as_bytes().to_vec(), 0o700) {
            let mut channel = self.session.channel_session().unwrap();
            channel.exec(&format!("./{}", REMOTE_SCRIPT));

            let mut s = String::new();
            channel.read_to_string(&mut s).unwrap();
            print!("{}", s);
            channel.wait_close().expect("Failed to close the channel");
            return channel.exit_status().unwrap();
        }
        return -10;
    }

    fn run(&mut self, command: &ExecutableTask, target: &Target, dry_run: bool) -> i32 {
        if !self.connect(target) {
            error!("Failed to connect to {:?}", target);
            return -1;
        }

        if let Some(provides) = &command.parameters.get("provides") {
            debug!(
                "A `provides` parameter was given, checking to see if {} exists on the remote",
                provides
            );

            if let Ok(found) = self.file_exists(Path::new(provides)) {
                if found {
                    debug!("File {} exists, skipping task", provides);
                    return 0;
                }
            } else {
                return -1;
            }
        }

        if let Some(unless) = &command.parameters.get("unless") {
            debug!("An `unless` parameter was given, running {}", unless);
            if 0 == self.run_script(unless) {
                debug!("`unless` script returned 0, skipping the task");
                return 0;
            }
        }

        if let Some(script) = command.task.script.as_bytes(Some(&command.parameters)) {
            if dry_run {
                println!("{}", "Dry-run\n----".yellow());
                let mut out = std::io::stdout();
                out.write(&script)
                    .expect("Somehow failed to write to stdout");
                println!("{}", "\n----".yellow());
                return 0;
            }

            if !self.send_bytes(Path::new(REMOTE_SCRIPT), &script, 0o700) {
                error!("Failed to upload script file for execution");
                return -1;
            }

            let mut channel = self.session.channel_session().unwrap();
            let stderr = channel.stderr();
            let args_file = "._zap_args.json";

            if command.task.script.has_file() {
                let args = serde_json::to_string(&command.parameters)
                    .expect("Failed to serialize parameters for task");
                if self.send_bytes(Path::new(args_file), &args.into_bytes(), 0o400) {
                    channel
                        .exec(&format!("./{} {}", REMOTE_SCRIPT, args_file))
                        .unwrap();
                } else {
                    error!("Failed to upload the arguments file");
                    return -1;
                }
            } else {
                channel.exec(&format!("./{}", REMOTE_SCRIPT)).unwrap();
            }

            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                println!("err: {}", line.unwrap());
            }

            let mut s = String::new();
            channel.read_to_string(&mut s).unwrap();
            print!("{}", s);
            channel.wait_close().expect("Failed to close the channel");
            let exit = channel.exit_status().unwrap();

            /*
             * This seems a little dumb and hacky, but we need to clean up the file
             * somehow and I'm not seeing anything that would allow me to just reach
             * out and remove a file
             */
            let mut channel = self.session.channel_session().unwrap();
            channel
                .exec(&format!("rm -f {} {}", REMOTE_SCRIPT, args_file))
                .unwrap();
            return exit;
        } else {
            error!("No script available to run for task!");
            return -1;
        }
    }

    fn send_bytes(&self, remote_path: &Path, bytes: &Vec<u8>, mode: i32) -> bool {
        let mut remote_file = self
            .session
            .scp_send(
                remote_path,
                mode,
                bytes
                    .len()
                    .try_into()
                    .expect("Failed converting the size of the file to send, yikes!"),
                None,
            )
            .unwrap();
        remote_file.write(bytes).unwrap();
        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();
        true
    }
}
