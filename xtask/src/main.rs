//! Development helpers, run as `cargo xtask <task>`.
//!
//! `cargo xtask dev` asks before each step: it pulls the common crate, builds the auth, direct message
//! and chat server services and the client, starts the services in order, then runs the client. Declining
//! a step stops it, and the services already started are stopped. Their logs are in `target/dev-logs/`.

use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

const AUTH_PORT: u16 = 7900;
const DM_PORT: u16 = 7910;
const SERVER_PORT: u16 = 7878;

fn main() {
    let task = std::env::args().nth(1).unwrap_or_default();
    let result = match task.as_str() {
        "dev" => dev(),
        _ => Err("usage: cargo xtask dev".to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// The repository root: this crate's folder is one level below it.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is inside the repository")
        .to_path_buf()
}

/// Services that are running. They are stopped when this is dropped, however the task ends.
struct Services {
    children: Vec<(String, Child)>,
    logs: PathBuf,
}

impl Drop for Services {
    fn drop(&mut self) {
        for (_, child) in self.children.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Services {
    /// Starts one service from `target/debug`, logs to its own file, and waits until it accepts connections.
    fn start(&mut self, name: &str, port: u16) -> Result<(), String> {
        let exe = if cfg!(windows) { ".exe" } else { "" };
        let binary = root().join("target/debug").join(format!("{name}{exe}"));
        let log_path = self.logs.join(format!("{name}.log"));
        let log = File::create(&log_path).map_err(|e| e.to_string())?;
        let log_err = log.try_clone().map_err(|e| e.to_string())?;
        let child = Command::new(&binary)
            .current_dir(root())
            .stdout(log)
            .stderr(log_err)
            .spawn()
            .map_err(|e| format!("could not start {name}: {e}"))?;
        self.children.push((name.to_string(), child));
        self.wait_for(name, port)
    }

    /// Waits for the port to answer. Stops early if the service has already exited.
    fn wait_for(&mut self, name: &str, port: u16) -> Result<(), String> {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if let Some((_, child)) = self.children.last_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!(
                        "{name} stopped ({status}). Its log: {}",
                        self.logs.join(format!("{name}.log")).display()
                    ));
                }
            }
            if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                println!("{name} is listening on port {port}.");
                return Ok(());
            }
            sleep(Duration::from_millis(100));
        }
        Err(format!(
            "{name} did not start on port {port}. Its log: {}",
            self.logs.join(format!("{name}.log")).display()
        ))
    }
}

/// Asks before a step. Anything but yes stops the task.
fn confirm(question: &str) -> Result<(), String> {
    print!("{question} [y/N] ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).map_err(|e| e.to_string())?;
    match answer.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err("Stopped. Nothing else was started.".to_string()),
    }
}

/// Runs a program in a folder and fails if it does not succeed.
fn run(dir: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status})", args.join(" ")))
    }
}

fn dev() -> Result<(), String> {
    let root = root();
    let logs = root.join("target/dev-logs");
    fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    let mut services = Services { children: Vec::new(), logs: logs.clone() };

    confirm("Update common (git pull in common/)?")?;
    run(&root.join("common"), "git", &["pull", "--ff-only", "origin", "main"])?;

    confirm("Build auth, dm and server?")?;
    run(&root, "cargo", &["build", "-p", "auth", "-p", "dm", "-p", "server"])?;

    confirm("Build the client?")?;
    run(&root.join("client"), "cargo", &["build"])?;

    confirm("Start auth on 127.0.0.1:7900?")?;
    services.start("auth", AUTH_PORT)?;

    confirm("Start the direct message service on 127.0.0.1:7910?")?;
    services.start("dm", DM_PORT)?;

    confirm("Start the chat server on 127.0.0.1:7878?")?;
    services.start("server", SERVER_PORT)?;

    confirm("Start the client?")?;
    println!("Starting the client. Add the server 127.0.0.1:7878 to connect.");
    run(&root.join("client"), "cargo", &["run"])
}
