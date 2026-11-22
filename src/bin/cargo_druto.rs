//! Cargo subcommand proxy binary entry point (`cargo druto`).

fn main() -> std::process::ExitCode {
    let kernel = druto::cli::create_kernel();
    std::process::ExitCode::from(kernel.execute_env() as u8)
}
