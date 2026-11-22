//! Development server supervisor and runner for Druto applications.

use std::path::Path;

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

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let host = ctx.flag_str("host").map(ToString::to_string);
        let port = ctx.flag_str("port").and_then(|p| p.parse::<u16>().ok());
        let dry_run = ctx.flag_bool("dry-run");

        execute_dev(&ctx.ui, host, port, dry_run)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Executes the development server supervisor.
pub fn execute_dev(
    ui: &Ui,
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

    ui.render_sticker("Druto Development Server v0.1.0", |card| {
        card.line(format!(
            "{}  http://{}:{}",
            "➜  Local:  ".green().bold(),
            bind_host,
            bind_port
        ));
        card.line(format!(
            "{}  {} (unoptimized + debuginfo)",
            "➜  Mode:   ".dimmed(),
            "Development".bold().yellow()
        ));
        card.line("");
        card.line("   [Ctrl+C] to stop server");
    });

    if dry_run {
        ui.info("[dry-run] Server supervisor configured successfully.");
        return Ok(());
    }

    if Path::new("Cargo.toml").exists() {
        ui.info(&format!(
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
        ui.warn("No Cargo.toml found in current directory. Run this inside a Druto project.");
    }

    Ok(())
}
