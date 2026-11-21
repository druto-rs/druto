//! Cargo subcommand proxy binary entry point (`cargo druto`).

#[tokio::main]
async fn main() {
    let kernel = druto::cli::create_kernel();
    if let Err(err) = kernel.handle_env().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
