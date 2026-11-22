//! Code generator commands for Druto with MiniJinja template rendering and smart module injection.

use std::{fs, path::Path};

use karigor::prelude::*;
use serde_json::json;
use tracing::info;

use crate::cli::{injector, stubs};

/// Command generating a modular route and optional controller.
pub struct MakeRouteCommand;

impl Command for MakeRouteCommand {
    fn name(&self) -> &'static str {
        "make:route"
    }

    fn description(&self) -> &'static str {
        "Generate a modular route handler and optional controller"
    }

    fn arguments(&self) -> Vec<Argument> {
        vec![Argument::required("name", "Route module name")]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![
            Flag::boolean("controller", Some('c'), "Generate paired controller"),
            Flag::boolean(
                "dry-run",
                None,
                "Preview generated route without writing to disk",
            ),
        ]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let name = ctx.arg("name").unwrap();
        let controller = ctx.flag_bool("controller");
        let dry_run = ctx.flag_bool("dry-run");

        execute_make_route(&ctx.ui, name, controller, dry_run)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Command generating a controller module.
pub struct MakeControllerCommand;

impl Command for MakeControllerCommand {
    fn name(&self) -> &'static str {
        "make:controller"
    }

    fn description(&self) -> &'static str {
        "Generate a paired controller module"
    }

    fn arguments(&self) -> Vec<Argument> {
        vec![Argument::required("name", "Controller name")]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![
            Flag::boolean("resource", Some('r'), "Generate RESTful CRUD actions"),
            Flag::boolean(
                "dry-run",
                None,
                "Preview generated controller without writing to disk",
            ),
        ]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let name = ctx.arg("name").unwrap();
        let resource = ctx.flag_bool("resource");
        let dry_run = ctx.flag_bool("dry-run");

        execute_make_controller(&ctx.ui, name, resource, dry_run)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Command generating custom onion middleware.
pub struct MakeMiddlewareCommand;

impl Command for MakeMiddlewareCommand {
    fn name(&self) -> &'static str {
        "make:middleware"
    }

    fn description(&self) -> &'static str {
        "Generate a custom onion middleware component"
    }

    fn arguments(&self) -> Vec<Argument> {
        vec![Argument::required("name", "Middleware name")]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![Flag::boolean(
            "dry-run",
            None,
            "Preview generated middleware without writing to disk",
        )]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let name = ctx.arg("name").unwrap();
        let dry_run = ctx.flag_bool("dry-run");

        execute_make_middleware(&ctx.ui, name, dry_run)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Command generating a console command or background task.
pub struct MakeCommandCommand;

impl Command for MakeCommandCommand {
    fn name(&self) -> &'static str {
        "make:command"
    }

    fn description(&self) -> &'static str {
        "Generate a custom console command or task"
    }

    fn arguments(&self) -> Vec<Argument> {
        vec![Argument::required("name", "Command name")]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![Flag::boolean(
            "dry-run",
            None,
            "Preview generated command without writing to disk",
        )]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let name = ctx.arg("name").unwrap();
        let dry_run = ctx.flag_bool("dry-run");

        execute_make_command(&ctx.ui, name, dry_run)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Command ejecting embedded stubs into `./stubs/`.
pub struct StubPublishCommand;

impl Command for StubPublishCommand {
    fn name(&self) -> &'static str {
        "stub:publish"
    }

    fn description(&self) -> &'static str {
        "Eject default template stubs into ./stubs for customization"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["stubs:publish"]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![Flag::boolean(
            "force",
            Some('f'),
            "Overwrite existing stubs in ./stubs",
        )]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let force = ctx.flag_bool("force");
        execute_stub_publish(&ctx.ui, force).map_err(|err| Error::Execution(err.to_string().into()))
    }
}

/// Executes modular route generator.
pub fn execute_make_route(
    ui: &Ui,
    name: &str,
    controller: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name_snake = name.to_snake_case().into_owned();
    let name_pascal = name.to_pascal_case().into_owned();
    let plural_str = name.to_plural();
    let name_plural = plural_str.to_snake_case().into_owned();
    let singular_str = name.to_singular();
    let name_singular = singular_str.to_snake_case().into_owned();

    info!(
        name = name,
        controller = controller,
        dry_run = dry_run,
        "Generating modular route component"
    );

    ui.section(&format!("Scaffolding Route Module: {}", name.bold().cyan()));

    let context = json!({
        "name": name,
        "name_snake": name_snake,
        "name_pascal": name_pascal,
        "name_singular": name_singular,
        "name_plural": name_plural,
        "has_controller": controller,
    });

    let route_template = stubs::get_stub("route.stub");
    let rendered_route = stubs::render_stub(&route_template, &context)?;

    let route_path = format!("src/routes/{name_snake}.rs");
    let routes_mod_path = "src/routes/mod.rs";

    if !dry_run {
        let route_file = Path::new(&route_path);
        if let Some(parent) = route_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(route_file, &rendered_route)?;
        format_file(route_file);

        let outcome = injector::inject_route_module(Path::new(routes_mod_path), &name_snake)?;

        ui.success(&format!("Generated {}", route_path.bold().white()));
        if outcome.module_injected {
            ui.info(&format!(
                "Injected {} into {}",
                format!("pub mod {name_snake};").yellow(),
                routes_mod_path.cyan()
            ));
        } else {
            ui.info(&format!(
                "Module {} already present in {}",
                format!("pub mod {name_snake};").yellow(),
                routes_mod_path.cyan()
            ));
        }

        if outcome.route_mounted {
            ui.info(&format!(
                "Mounted {} in {}",
                format!("router.nest(\"/{name_snake}\", {name_snake}::routes());").yellow(),
                routes_mod_path.cyan()
            ));
        } else {
            ui.info(&format!(
                "Route {} already mounted in {}",
                format!("\"/{name_snake}\"").yellow(),
                routes_mod_path.cyan()
            ));
        }
        format_file(Path::new(routes_mod_path));
    } else {
        ui.info(&format!(
            "[dry-run] Would generate {}",
            route_path.bold().white()
        ));
        ui.info(&format!(
            "[dry-run] Would inject into {}",
            routes_mod_path.cyan()
        ));
    }

    if controller {
        let controller_context = json!({
            "name": name,
            "name_snake": name_snake,
            "name_pascal": name_pascal,
            "name_singular": name_singular,
            "name_plural": name_plural,
            "api": false,
        });

        let controller_template = stubs::get_stub("controller.stub");
        let rendered_controller = stubs::render_stub(&controller_template, &controller_context)?;

        let controller_path = format!("src/controllers/{name_snake}.rs");
        let controllers_mod_path = "src/controllers/mod.rs";

        if !dry_run {
            let controller_file = Path::new(&controller_path);
            if let Some(parent) = controller_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(controller_file, &rendered_controller)?;
            format_file(controller_file);

            let injected =
                injector::inject_controller_module(Path::new(controllers_mod_path), &name_snake)?;

            ui.success(&format!(
                "Generated paired controller {}",
                controller_path.bold().white()
            ));
            if injected {
                ui.info(&format!(
                    "Injected {} into {}",
                    format!("pub mod {name_snake};").yellow(),
                    controllers_mod_path.cyan()
                ));
            } else {
                ui.info(&format!(
                    "Controller module {} already present in {}",
                    format!("pub mod {name_snake};").yellow(),
                    controllers_mod_path.cyan()
                ));
            }
            format_file(Path::new(controllers_mod_path));
        } else {
            ui.info(&format!(
                "[dry-run] Would generate {}",
                controller_path.bold().white()
            ));
            ui.info(&format!(
                "[dry-run] Would inject into {}",
                controllers_mod_path.cyan()
            ));
        }
    }

    ui.print("");
    ui.print("  Route Definition Preview:");
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    for line in rendered_route.lines() {
        ui.print(format!("  {line}"));
    }
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    ui.print("");

    Ok(())
}

/// Executes controller generator.
pub fn execute_make_controller(
    ui: &Ui,
    name: &str,
    resource: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name_snake = name.to_snake_case().into_owned();
    let name_pascal = name.to_pascal_case().into_owned();
    let plural_str = name.to_plural();
    let name_plural = plural_str.to_snake_case().into_owned();
    let singular_str = name.to_singular();
    let name_singular = singular_str.to_snake_case().into_owned();

    info!(
        name = name,
        resource = resource,
        dry_run = dry_run,
        "Generating controller component"
    );

    ui.section(&format!("Scaffolding Controller: {}", name.bold().cyan()));

    let context = json!({
        "name": name,
        "name_snake": name_snake,
        "name_pascal": name_pascal,
        "name_singular": name_singular,
        "name_plural": name_plural,
        "api": !resource,
    });

    let template = stubs::get_stub("controller.stub");
    let rendered = stubs::render_stub(&template, &context)?;

    let path_str = format!("src/controllers/{name_snake}.rs");
    let controllers_mod_path = "src/controllers/mod.rs";

    if !dry_run {
        let file_path = Path::new(&path_str);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &rendered)?;
        format_file(file_path);

        let injected =
            injector::inject_controller_module(Path::new(controllers_mod_path), &name_snake)?;

        ui.success(&format!("Generated {}", path_str.bold().white()));
        if injected {
            ui.info(&format!(
                "Injected {} into {}",
                format!("pub mod {name_snake};").yellow(),
                controllers_mod_path.cyan()
            ));
        } else {
            ui.info(&format!(
                "Controller module {} already present in {}",
                format!("pub mod {name_snake};").yellow(),
                controllers_mod_path.cyan()
            ));
        }
        format_file(Path::new(controllers_mod_path));
    } else {
        ui.info(&format!(
            "[dry-run] Would generate {}",
            path_str.bold().white()
        ));
        ui.info(&format!(
            "[dry-run] Would inject into {}",
            controllers_mod_path.cyan()
        ));
    }

    ui.print("");
    ui.print("  Controller Code Preview:");
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    for line in rendered.lines() {
        ui.print(format!("  {line}"));
    }
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    ui.print("");

    Ok(())
}

/// Executes middleware generator.
pub fn execute_make_middleware(
    ui: &Ui,
    name: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name_snake = name.to_snake_case().into_owned();
    let name_pascal = name.to_pascal_case().into_owned();

    info!(
        name = name,
        dry_run = dry_run,
        "Generating onion middleware component"
    );

    ui.section(&format!(
        "Scaffolding Onion Middleware: {}",
        name.bold().cyan()
    ));

    let context = json!({
        "name": name,
        "name_snake": name_snake,
        "name_pascal": name_pascal,
    });

    let template = stubs::get_stub("middleware.stub");
    let rendered = stubs::render_stub(&template, &context)?;

    let path_str = format!("src/middleware/{name_snake}.rs");

    if !dry_run {
        let file_path = Path::new(&path_str);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &rendered)?;
        format_file(file_path);

        ui.success(&format!("Generated {}", path_str.bold().white()));
        ui.info(&format!(
            "Remember to register in pipeline: {}",
            format!("router.middleware({name_pascal});").yellow()
        ));
    } else {
        ui.info(&format!(
            "[dry-run] Would generate {}",
            path_str.bold().white()
        ));
    }

    ui.print("");
    ui.print("  Middleware Code Preview:");
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    for line in rendered.lines() {
        ui.print(format!("  {line}"));
    }
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    ui.print("");

    Ok(())
}

/// Executes custom console command generator.
pub fn execute_make_command(
    ui: &Ui,
    name: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name_snake = name.to_snake_case().into_owned();
    let name_pascal = name.to_pascal_case().into_owned();

    info!(
        name = name,
        dry_run = dry_run,
        "Generating console command component"
    );

    ui.section(&format!(
        "Scaffolding Console Command: {}",
        name.bold().cyan()
    ));

    let context = json!({
        "name": name,
        "name_snake": name_snake,
        "name_pascal": name_pascal,
    });

    let template = stubs::get_stub("command.stub");
    let rendered = stubs::render_stub(&template, &context)?;

    let path_str = format!("src/commands/{name_snake}.rs");

    if !dry_run {
        let file_path = Path::new(&path_str);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &rendered)?;
        format_file(file_path);

        ui.success(&format!("Generated {}", path_str.bold().white()));
    } else {
        ui.info(&format!(
            "[dry-run] Would generate {}",
            path_str.bold().white()
        ));
    }

    ui.print("");
    ui.print("  Command Code Preview:");
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    for line in rendered.lines() {
        ui.print(format!("  {line}"));
    }
    ui.print(format!(
        "  {}",
        "───────────────────────────────────────────────────────".dimmed()
    ));
    ui.print("");

    Ok(())
}

/// Publishes default stubs into `./stubs/` directory.
pub fn execute_stub_publish(ui: &Ui, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    ui.section("Publishing Default Template Stubs");

    let target_dir = Path::new("stubs");
    let results = stubs::publish_stubs(target_dir, force)?;

    for (filename, written) in results {
        if written {
            ui.success(&format!("Published stubs/{}", filename.bold().white()));
        } else {
            ui.warn(&format!(
                "Skipped stubs/{filename} (already exists, use --force to overwrite)"
            ));
        }
    }

    ui.print("");
    ui.info("You can now customize these stubs to match your project conventions.");
    ui.print("");

    Ok(())
}

fn format_file(path: &Path) {
    if let Some(path_str) = path.to_str() {
        let result = std::process::Command::new("cargo")
            .args(["fmt", "--", path_str])
            .output();
        drop(result);
    }
}
