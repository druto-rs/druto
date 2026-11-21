//! Development server supervisor and runner for Druto applications.

use std::path::Path;

use colored::Colorize;
use karigor::prelude::*;
use tracing::info;

/// Command supervising the development server process.
pub struct DevCommand;

impl Command for DevCommand {
    fn name(&self) -> &'static str {
        "dev"
    }

    fn description(&self) -> &'static str {
        "Start the development server with live reload"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["serve"]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![
            Flag::string("host", Some('H'), "Host address to bind", None),
            Flag::string("port", Some('p'), "Port to listen on", None),
            Flag::boolean(
                "dry-run",
                None,
                "Preview configuration without spawning child process",
            ),
        ]
    }

    fn run<'a>(&'a self, ctx: &'a mut Context) -> CommandFuture<'a> {
        Box::pin(async move {
            let host = ctx.flag_str("host").map(ToString::to_string);
            let port = ctx.flag_str("port").and_then(|p| p.parse::<u16>().ok());
            let dry_run = ctx.flag_bool("dry-run");

            execute_dev(host, port, dry_run)
                .await
                .map_err(|err| KarigorError::Execution(err.to_string().into()))
        })
    }
}

/// Executes the development server supervisor.
pub async fn execute_dev(
    host: Option<String>,
    port: Option<u16>,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind_host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let bind_port = port.unwrap_or(5555);

    info!(
        host = %bind_host,
        port = bind_port,
        dry_run = dry_run,
        "Initializing development server supervisor"
    );

    println!();
    println!(
        "{}",
        " ╭──────────────────────────────────────────────────────────╮".cyan()
    );
    println!(
        " {}                                                          {}",
        "│".cyan(),
        "│".cyan()
    );
    println!(
        " {}   {}                   {}",
        "│".cyan(),
        "⚡ Druto Development Server v0.1.0".bold().white(),
        "│".cyan()
    );
    println!(
        " {}                                                          {}",
        "│".cyan(),
        "│".cyan()
    );
    println!(
        " {}   {}  http://{}:{}                     {}",
        "│".cyan(),
        "➜  Local:  ".green().bold(),
        bind_host,
        bind_port,
        "│".cyan()
    );
    println!(
        " {}   {}  {} (unoptimized + debuginfo)         {}",
        "│".cyan(),
        "➜  Mode:   ".dimmed(),
        "Development".bold().yellow(),
        "│".cyan()
    );
    println!(
        " {}                                                          {}",
        "│".cyan(),
        "│".cyan()
    );
    println!(
        " {}   [Ctrl+C] to stop server                                {}",
        "│".cyan(),
        "│".cyan()
    );
    println!(
        " {}                                                          {}",
        "│".cyan(),
        "│".cyan()
    );
    println!(
        "{}",
        " ╰──────────────────────────────────────────────────────────╯".cyan()
    );
    println!();

    if dry_run {
        karigor::ui::print_info("[dry-run] Server supervisor configured successfully.");
        return Ok(());
    }

    if Path::new("Cargo.toml").exists() {
        karigor::ui::print_info(&format!(
            "Starting application process on {bind_host}:{bind_port}..."
        ));

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("run");
        cmd.env("HOST", &bind_host);
        cmd.env("PORT", bind_port.to_string());

        let mut child = cmd.spawn()?;
        let status = child.wait()?;

        if !status.success() {
            return Err(format!("Process exited with status: {status}").into());
        }
    } else {
        karigor::ui::print_warn(
            "No Cargo.toml found in current directory. Run this inside a Druto project.",
        );
    }

    Ok(())
}
