//! Interactive and automated project scaffolding for Druto web applications.

use std::{fs, io::IsTerminal, path::Path};

use karigor::prelude::*;
use tracing::info;

/// Command generating a new Druto web application.
pub struct NewCommand;

impl Command for NewCommand {
    fn name(&self) -> &'static str {
        "new"
    }

    fn description(&self) -> &'static str {
        "Generate a new Druto web application"
    }

    fn arguments(&self) -> Vec<Argument> {
        vec![Argument::optional("name", "Project name", None)]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let initial_name = ctx.arg("name").map(ToString::to_string);
        execute_new(&ctx.ui, initial_name).map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Executes project scaffolding.
pub fn execute_new(
    ui: &Ui,
    initial_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    ui.banner(
        "Druto Scaffolding Wizard",
        "Generate a new Druto web application",
    );

    let is_terminal = std::io::stdin().is_terminal();

    let (project_name, init_git) = if is_terminal && initial_name.is_none() {
        let name = ui.input("Project name", Some("my-druto-app"))?;

        let presets = [
            "REST API (Batteries-Included: Router, CORS, Limiter, Shield, Bodyparser)",
            "REST API (Minimal: Lightweight Router & JSON parser)",
            "Full-Stack Web (Router, Session, Redis, Shield)",
        ];

        let preset_index = ui.select("Choose architecture preset", &presets, 0)?;

        let init_git = ui.confirm("Initialize a Git repository?", true)?;

        info!(
            project = %name,
            preset = %presets[preset_index],
            git = init_git,
            "Configured scaffolding parameters"
        );

        (name, init_git)
    } else {
        let name = initial_name.unwrap_or_else(|| "my-druto-app".to_string());
        (name, true)
    };

    ui.section(&format!(
        "Scaffolding Project: {}",
        project_name.bold().cyan()
    ));

    let project_dir = Path::new(&project_name);
    let package_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&project_name);

    if project_dir.exists() {
        return Err(format!("Directory `{}` already exists", project_name).into());
    }

    let spinner = ui.spinner("Creating project directory structure...");

    fs::create_dir_all(project_dir.join("src").join("routes"))?;
    fs::create_dir_all(project_dir.join("src").join("controllers"))?;

    spinner.set_message("Writing Cargo.toml manifest with Druto framework dependencies...");
    let cargo_toml = format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
druto = {{ git = "https://github.com/druto-rs/druto.git", branch = "main" }}
tokio = {{ version = "1.54.1", features = ["full"] }}
tracing = "0.1.44"
tracing-subscriber = {{ version = "0.3.23", features = ["env-filter", "fmt"] }}
serde = {{ version = "1.0.229", features = ["derive"] }}
serde_json = "1.0.154"
"#
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    spinner.set_message("Generating main server entrypoint and route dispatcher...");
    let main_rs = r#"use druto::ignitor;

mod controllers;
mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ignitor::start(routes::routes()).await?;
    Ok(())
}
"#;
    fs::write(project_dir.join("src").join("main.rs"), main_rs)?;

    let routes_mod_rs = r#"use druto::prelude::*;

pub fn routes() -> Router {
    let mut router = Router::new();

    // Default welcome route
    router.get(
        "/",
        from_handler(|ctx| {
            Box::pin(async move {
                ctx.response.text("Welcome to Druto Framework!\n");
                Ok(())
            })
        }),
    );

    router
}
"#;
    fs::write(
        project_dir.join("src").join("routes").join("mod.rs"),
        routes_mod_rs,
    )?;

    let controllers_mod_rs = "// Controller modules registered by cargo druto make:controller\n";
    fs::write(
        project_dir.join("src").join("controllers").join("mod.rs"),
        controllers_mod_rs,
    )?;

    spinner.set_message("Generating environment files (.env, .env.example)...");
    let env_content = "HOST=127.0.0.1\nPORT=5555\nAPP_ENV=development\n";
    fs::write(project_dir.join(".env"), env_content)?;
    fs::write(project_dir.join(".env.example"), env_content)?;

    let gitignore_content = "/target\nCargo.lock\n.env\n*.local\n";
    fs::write(project_dir.join(".gitignore"), gitignore_content)?;

    let readme_content = format!(
        r#"# {project_name}

A modern web application built on the [Druto](https://github.com/druto-rs/druto) framework.

## Getting Started

```bash
# Start development server
cargo druto dev

# Or directly with Cargo
cargo run
```

## Scaffolding Components

```bash
# Generate a new modular route with paired controller
cargo druto make:route user --controller

# Generate custom onion middleware
cargo druto make:middleware auth

# Inspect all registered routes
cargo druto routes
```
"#
    );
    fs::write(project_dir.join("README.md"), readme_content)?;

    if init_git {
        spinner.set_message("Initializing git repository...");
        let git_init = std::process::Command::new("git")
            .arg("init")
            .arg(project_dir)
            .output();
        drop(git_init);
    }

    spinner.finish_and_clear();

    ui.success(&format!(
        "Created Druto application in {}",
        project_dir.display().to_string().bold().cyan()
    ));
    ui.success("Wrote Cargo.toml with Druto framework dependencies");
    ui.success("Generated HTTP server entrypoint in src/main.rs");
    ui.success("Configured initial routing table in src/routes/mod.rs");
    if init_git {
        ui.success("Initialized Git repository");
    }

    let mut steps = ui.steps();
    steps.step(
        "Navigate to project directory",
        format!("cd {}", project_name.bold().yellow()),
    );
    steps.step("Start HTTP development server", "cargo druto dev");
    steps.step(
        "Generate modular route and controller",
        "cargo druto make:route user --controller",
    );
    ui.render_steps(&steps);

    Ok(())
}
