use std::fs;

use druto::cli::create_kernel;
use tempfile::tempdir;

#[tokio::test]
async fn test_cli_kernel_metadata_and_help() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec!["druto".to_string(), "--help".to_string()])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_kernel_version() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec!["druto".to_string(), "--version".to_string()])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_make_route_dry_run() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "make:route".to_string(),
            "users".to_string(),
            "--controller".to_string(),
            "--dry-run".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_make_subcommand_colon_normalization() {
    let kernel = create_kernel();
    // Invocations with space: `druto make route users --dry-run`
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "make".to_string(),
            "route".to_string(),
            "products".to_string(),
            "--dry-run".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_make_controller_dry_run() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "make:controller".to_string(),
            "orders".to_string(),
            "--resource".to_string(),
            "--dry-run".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_make_middleware_dry_run() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "make:middleware".to_string(),
            "auth".to_string(),
            "--dry-run".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_make_command_dry_run() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "make:command".to_string(),
            "cleanup".to_string(),
            "--dry-run".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_stub_publish() {
    let temp = tempdir().expect("tempdir");
    let target_dir = temp.path().join("stubs");
    let result = druto::cli::stubs::publish_stubs(&target_dir, false);
    assert!(result.is_ok());

    assert!(target_dir.join("route.stub").exists());
    assert!(target_dir.join("controller.stub").exists());
    assert!(target_dir.join("middleware.stub").exists());
    assert!(target_dir.join("command.stub").exists());
}

#[tokio::test]
async fn test_cli_routes_and_filter() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "routes".to_string(),
            "--filter".to_string(),
            "api".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_routes_json_output() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "routes".to_string(),
            "--json".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_middleware_pipeline_inspection() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec!["druto".to_string(), "middleware".to_string()])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_doctor() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec!["druto".to_string(), "doctor".to_string()])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_dev_dry_run() {
    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "dev".to_string(),
            "--dry-run".to_string(),
            "--port".to_string(),
            "8080".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_cargo_druto_invocation_normalization() {
    let kernel = create_kernel();
    // Invocations via `cargo druto doctor`
    let result = kernel
        .handle(vec![
            "cargo-druto".to_string(),
            "druto".to_string(),
            "doctor".to_string(),
        ])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_scaffold_new_project_non_interactive() {
    let temp = tempdir().expect("tempdir");
    let project_dir = temp.path().join("demo_app");
    let project_path_str = project_dir.to_str().expect("valid path").to_string();

    let kernel = create_kernel();
    let result = kernel
        .handle(vec![
            "druto".to_string(),
            "new".to_string(),
            project_path_str,
        ])
        .await;
    assert!(result.is_ok());

    // Verify generated project structure
    assert!(project_dir.join("Cargo.toml").exists());
    assert!(project_dir.join("src/main.rs").exists());
    assert!(project_dir.join("src/routes/mod.rs").exists());
    assert!(project_dir.join("src/controllers/mod.rs").exists());
    assert!(project_dir.join(".env").exists());
    assert!(project_dir.join(".gitignore").exists());
    assert!(project_dir.join("README.md").exists());

    // Verify main.rs contains Ignitor bootstrap
    let main_rs = fs::read_to_string(project_dir.join("src/main.rs")).expect("read main.rs");
    assert!(main_rs.contains("ignitor::start"));
}
