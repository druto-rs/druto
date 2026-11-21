//! Smart syntax-aware code injection and module registration for Druto projects.

use std::{fs, path::Path};

/// Result of injecting a route into `src/routes/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteInjectionOutcome {
    /// Whether `pub mod <name>;` was injected (false if already present).
    pub module_injected: bool,
    /// Whether `router.nest("/<name>", <name>::routes());` was mounted (false if already present).
    pub route_mounted: bool,
}

/// Injects a route module and mounts it in `src/routes/mod.rs`.
pub fn inject_route_module(
    routes_mod_path: &Path,
    name_snake: &str,
) -> Result<RouteInjectionOutcome, std::io::Error> {
    if !routes_mod_path.exists() {
        if let Some(parent) = routes_mod_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = format!(
            "use druto::prelude::*;\n\npub mod {name_snake};\n\npub fn routes() -> Router {{\n    let mut router = Router::new();\n    router.nest(\"/{name_snake}\", {name_snake}::routes());\n    router\n}}\n"
        );
        fs::write(routes_mod_path, content)?;
        return Ok(RouteInjectionOutcome {
            module_injected: true,
            route_mounted: true,
        });
    }

    let original = fs::read_to_string(routes_mod_path)?;
    let mut lines: Vec<String> = original.lines().map(ToString::to_string).collect();

    let mut outcome = RouteInjectionOutcome {
        module_injected: false,
        route_mounted: false,
    };

    // 1. Check & Inject Module Declaration
    let mod_decl = format!("pub mod {name_snake};");
    let has_mod = lines
        .iter()
        .any(|line| line.trim() == mod_decl || line.trim() == format!("mod {name_snake};"));

    if !has_mod {
        let last_mod_idx = lines.iter().rposition(|line| {
            line.trim().starts_with("pub mod ") || line.trim().starts_with("mod ")
        });

        if let Some(idx) = last_mod_idx {
            lines.insert(idx + 1, mod_decl);
        } else {
            let last_use_idx = lines
                .iter()
                .rposition(|line| line.trim().starts_with("use "));
            if let Some(idx) = last_use_idx {
                lines.insert(idx + 1, String::new());
                lines.insert(idx + 2, mod_decl);
            } else {
                lines.insert(0, mod_decl);
            }
        }
        outcome.module_injected = true;
    }

    // 2. Check & Inject Route Mount
    let nest_call = format!("    router.nest(\"/{name_snake}\", {name_snake}::routes());");
    let has_mount = lines.iter().any(|line| {
        line.contains(&format!("router.nest(\"/{name_snake}\""))
            || line.contains(&format!(".nest(\"/{name_snake}\""))
    });

    if !has_mount {
        // Find return router statement in `pub fn routes`
        let routes_fn_idx = lines
            .iter()
            .position(|line| line.contains("fn routes") && line.contains("Router"));

        if let Some(start_idx) = routes_fn_idx {
            // Find return expression `router` or closing brace `}`
            let insert_idx = lines[start_idx..].iter().position(|line| {
                let trimmed = line.trim();
                trimmed == "router" || trimmed == "router;"
            });

            if let Some(offset) = insert_idx {
                lines.insert(start_idx + offset, nest_call);
                outcome.route_mounted = true;
            } else {
                // Fallback to inserting before last closing brace of function
                let closing_brace_idx = lines[start_idx..]
                    .iter()
                    .position(|line| line.trim() == "}");
                if let Some(offset) = closing_brace_idx {
                    lines.insert(start_idx + offset, nest_call);
                    outcome.route_mounted = true;
                }
            }
        }
    }

    if outcome.module_injected || outcome.route_mounted {
        let mut updated = lines.join("\n");
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        fs::write(routes_mod_path, updated)?;
    }

    Ok(outcome)
}

/// Injects a controller module declaration into `src/controllers/mod.rs`.
pub fn inject_controller_module(
    controllers_mod_path: &Path,
    name_snake: &str,
) -> Result<bool, std::io::Error> {
    if !controllers_mod_path.exists() {
        if let Some(parent) = controllers_mod_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = format!("pub mod {name_snake};\n");
        fs::write(controllers_mod_path, content)?;
        return Ok(true);
    }

    let original = fs::read_to_string(controllers_mod_path)?;
    let mut lines: Vec<String> = original.lines().map(ToString::to_string).collect();

    let mod_decl = format!("pub mod {name_snake};");
    let has_mod = lines
        .iter()
        .any(|line| line.trim() == mod_decl || line.trim() == format!("mod {name_snake};"));

    if has_mod {
        return Ok(false);
    }

    let last_mod_idx = lines
        .iter()
        .rposition(|line| line.trim().starts_with("pub mod ") || line.trim().starts_with("mod "));

    if let Some(idx) = last_mod_idx {
        lines.insert(idx + 1, mod_decl);
    } else {
        lines.push(mod_decl);
    }

    let mut updated = lines.join("\n");
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    fs::write(controllers_mod_path, updated)?;

    Ok(true)
}
