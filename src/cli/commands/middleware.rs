//! Middleware onion pipeline inspection for Druto.

use karigor::prelude::*;
use tracing::info;

/// Command displaying the configured onion middleware pipeline.
pub struct MiddlewareCommand;

impl Command for MiddlewareCommand {
    fn name(&self) -> &'static str {
        "middleware"
    }

    fn description(&self) -> &'static str {
        "Inspect the active onion middleware pipeline and execution layer order"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["list:middleware", "middleware:list"]
    }

    fn run<'a>(&'a self, ctx: &'a mut Context) -> CommandFuture<'a> {
        Box::pin(async move {
            execute_middleware(&ctx.ui)
                .map_err(|err| KarigorError::Execution(err.to_string().into()))
        })
    }
}

struct MiddlewareInfo {
    layer: usize,
    name: &'static str,
    crate_name: &'static str,
    description: &'static str,
}

const SAMPLE_MIDDLEWARES: &[MiddlewareInfo] = &[
    MiddlewareInfo {
        layer: 1,
        name: "CorsMiddleware",
        crate_name: "druto-cors",
        description: "Cross-Origin preflight short-circuiting and header negotiation",
    },
    MiddlewareInfo {
        layer: 2,
        name: "LimiterMiddleware",
        crate_name: "druto-limiter",
        description: "Sliding window rate limiting and RFC 6585 429 throttling",
    },
    MiddlewareInfo {
        layer: 3,
        name: "ShieldMiddleware",
        crate_name: "druto-shield",
        description: "Security headers, CSP with per-request nonces, and CSRF token defense",
    },
    MiddlewareInfo {
        layer: 4,
        name: "SessionMiddleware",
        crate_name: "druto-session",
        description: "Encrypted cookie extraction and session state hydration",
    },
    MiddlewareInfo {
        layer: 5,
        name: "BodyparserMiddleware",
        crate_name: "druto-bodyparser",
        description: "JSON, form-urlencoded, and multipart payload decoding",
    },
];

/// Displays the active middleware onion pipeline.
pub fn execute_middleware(ui: &Ui) -> Result<(), Box<dyn std::error::Error>> {
    info!("Inspecting active onion middleware pipeline");

    ui.banner(
        "Onion Middleware Pipeline",
        "Execution order from outermost layer to innermost handler",
    );

    let mut table = ui.table(&[
        "Layer",
        "Middleware Component",
        "Crate Source",
        "Responsibility",
    ]);

    for middleware in SAMPLE_MIDDLEWARES {
        let layer_cell = format!(" #{} ", middleware.layer).cyan().bold().to_string();
        let name_cell = middleware.name.bold().white().to_string();
        let crate_cell = middleware.crate_name.yellow().to_string();
        let desc_cell = middleware.description.dimmed().to_string();

        table.add_row(vec![
            Cell::new(layer_cell),
            Cell::new(name_cell),
            Cell::new(crate_cell),
            Cell::new(desc_cell),
        ]);
    }

    ui.print(table.to_string());
    ui.print(format!(
        "\n  {} Requests enter at Layer #1 and traverse sequentially inward to route handlers.\n",
        "◆".cyan()
    ));

    Ok(())
}
