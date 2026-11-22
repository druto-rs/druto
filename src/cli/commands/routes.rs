//! Route inspection and endpoint visualization for Druto.

use karigor::prelude::*;
use serde::Serialize;
use tracing::info;

/// Command displaying all compiled application routes.
pub struct RoutesCommand;

impl Command for RoutesCommand {
    fn name(&self) -> &'static str {
        "routes"
    }

    fn description(&self) -> &'static str {
        "Display all compiled application routes in a formatted table"
    }

    fn flags(&self) -> Vec<Flag> {
        vec![
            Flag::string(
                "filter",
                Some('f'),
                "Filter routes by pattern or method",
                None,
            ),
            Flag::boolean("json", None, "Output routes table as JSON"),
        ]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let filter = ctx.flag_str("filter").map(ToString::to_string);
        let json = ctx.flag_bool("json");

        execute_routes(&ctx.ui, filter, json)
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

#[derive(Serialize)]
struct RouteInfo {
    method: &'static str,
    path: &'static str,
    handler: &'static str,
    middlewares: &'static [&'static str],
}

const SAMPLE_ROUTES: &[RouteInfo] = &[
    RouteInfo {
        method: "GET",
        path: "/",
        handler: "app::handlers::home",
        middlewares: &["cors"],
    },
    RouteInfo {
        method: "GET",
        path: "/api/health",
        handler: "app::handlers::health_check",
        middlewares: &["cors"],
    },
    RouteInfo {
        method: "POST",
        path: "/api/v1/auth/login",
        handler: "app::handlers::auth::login",
        middlewares: &["cors", "limiter", "bodyparser"],
    },
    RouteInfo {
        method: "POST",
        path: "/api/v1/auth/logout",
        handler: "app::handlers::auth::logout",
        middlewares: &["cors", "session", "shield::csrf"],
    },
    RouteInfo {
        method: "GET",
        path: "/api/v1/users",
        handler: "app::handlers::users::index",
        middlewares: &["cors", "session", "limiter"],
    },
    RouteInfo {
        method: "POST",
        path: "/api/v1/users",
        handler: "app::handlers::users::create",
        middlewares: &["cors", "session", "shield::csrf", "bodyparser"],
    },
    RouteInfo {
        method: "GET",
        path: "/api/v1/users/:id",
        handler: "app::handlers::users::show",
        middlewares: &["cors", "session"],
    },
    RouteInfo {
        method: "PUT",
        path: "/api/v1/users/:id",
        handler: "app::handlers::users::update",
        middlewares: &["cors", "session", "shield::csrf"],
    },
    RouteInfo {
        method: "DELETE",
        path: "/api/v1/users/:id",
        handler: "app::handlers::users::destroy",
        middlewares: &["cors", "session", "shield::csrf"],
    },
];

/// Executes route table visualization.
pub fn execute_routes(
    ui: &Ui,
    filter: Option<String>,
    as_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(filter = ?filter, as_json = as_json, "Displaying route registry table");

    let filtered: Vec<&RouteInfo> = SAMPLE_ROUTES
        .iter()
        .filter(|route| {
            if let Some(ref query) = filter {
                let q = query.to_lowercase();
                route.path.to_lowercase().contains(&q)
                    || route.method.to_lowercase().contains(&q)
                    || route.handler.to_lowercase().contains(&q)
            } else {
                true
            }
        })
        .collect();

    if as_json {
        let json_str = serde_json::to_string_pretty(&filtered)?;
        ui.print(json_str);
        return Ok(());
    }

    ui.banner("Druto Route Registry", "Compiled HTTP Route Pipeline");

    let mut table = ui.table(&["Method", "URI Path", "Handler", "Middleware Pipeline"]);

    for route in &filtered {
        let mw_display = if route.middlewares.is_empty() {
            "none".dimmed().to_string()
        } else {
            route.middlewares.join(" → ")
        };

        table.add_row(vec![
            ui.cell_http_method(route.method),
            Cell::new(route.path).add_attribute(Attribute::Bold),
            Cell::new(route.handler).fg(Color::Cyan),
            Cell::new(mw_display),
        ]);
    }

    ui.print(table.to_string());
    ui.print("");
    ui.print(format!(
        "  Total Endpoints: {} | Filter: {}",
        filtered.len().to_string().bold().green(),
        filter.as_deref().unwrap_or("none").bold().yellow()
    ));
    ui.print("");

    Ok(())
}
