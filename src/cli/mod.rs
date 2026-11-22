//! Command-line interface and code generation toolkit for Druto.
//!
//! Powered by [`karigor`], Druto provides an ergonomic terminal interface
//! for scaffolding, application supervision, code generation, and diagnostics.

pub mod commands;
pub mod injector;
pub mod stubs;

pub use commands::*;
pub use karigor;

/// Builds and configures the default CLI application kernel with all standard commands.
#[must_use]
pub fn create_kernel() -> karigor::Kernel {
    karigor::Kernel::new("druto", "Developer-friendly command-line toolkit for Druto")
        .version(env!("CARGO_PKG_VERSION"))
        .group(
            "make",
            "Scaffold routes, controllers, middleware, and commands",
        )
        .register(commands::NewCommand)
        .register(commands::DevCommand)
        .register(commands::MakeRouteCommand)
        .register(commands::MakeControllerCommand)
        .register(commands::MakeMiddlewareCommand)
        .register(commands::MakeCommandCommand)
        .register(commands::StubPublishCommand)
        .register(commands::RoutesCommand)
        .register(commands::MiddlewareCommand)
        .register(commands::DoctorCommand)
        .register(commands::KeyGenerateCommand)
}
