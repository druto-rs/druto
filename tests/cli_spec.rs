use std::fs;

use druto::cli::create_kernel;
use tempfile::tempdir;

#[test]
fn test_cli_kernel_metadata_and_help() {
    let kernel = create_kernel();
    let result = kernel.handle(vec!["druto".to_string(), "--help".to_string()]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_kernel_version() {
    let kernel = create_kernel();
    let result = kernel.handle(vec!["druto".to_string(), "--version".to_string()]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_make_route_dry_run() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "make:route".to_string(),
        "users".to_string(),
        "--controller".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_make_subcommand_colon_normalization() {
    let kernel = create_kernel();
    // Invocations with space: `druto make route users --dry-run`
    let result = kernel.handle(vec![
        "druto".to_string(),
        "make".to_string(),
        "route".to_string(),
        "products".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_make_controller_dry_run() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "make:controller".to_string(),
        "orders".to_string(),
        "--resource".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_make_middleware_dry_run() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "make:middleware".to_string(),
        "auth".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_make_command_dry_run() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "make:command".to_string(),
        "cleanup".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_stub_publish() {
    let temp = tempdir().expect("tempdir");
    let target_dir = temp.path().join("stubs");
    let result = druto::cli::stubs::publish_stubs(&target_dir, false);
    assert!(result.is_ok());

    assert!(target_dir.join("route.stub").exists());
    assert!(target_dir.join("controller.stub").exists());
    assert!(target_dir.join("middleware.stub").exists());
    assert!(target_dir.join("command.stub").exists());
}

#[test]
fn test_cli_routes_and_filter() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "routes".to_string(),
        "--filter".to_string(),
        "api".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_routes_json_output() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "routes".to_string(),
        "--json".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_middleware_pipeline_inspection() {
    let kernel = create_kernel();
    let result = kernel.handle(vec!["druto".to_string(), "middleware".to_string()]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_doctor() {
    let kernel = create_kernel();
    let result = kernel.handle(vec!["druto".to_string(), "doctor".to_string()]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_dev_dry_run() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "dev".to_string(),
        "--dry-run".to_string(),
        "--port".to_string(),
        "8080".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_cargo_druto_invocation_normalization() {
    let kernel = create_kernel();
    // Invocations via `cargo druto doctor`
    let result = kernel.handle(vec![
        "cargo-druto".to_string(),
        "druto".to_string(),
        "doctor".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_scaffold_new_project_non_interactive() {
    let temp = tempdir().expect("tempdir");
    let project_dir = temp.path().join("demo_app");
    let project_path_str = project_dir.to_str().expect("valid path").to_string();

    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "new".to_string(),
        project_path_str,
    ]);
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

#[test]
fn test_cli_execute_exit_codes() {
    let kernel = create_kernel();
    let exit_ok = kernel.execute(vec!["druto".to_string(), "--help".to_string()]);
    assert_eq!(exit_ok, 0);

    let exit_unknown = kernel.execute(vec!["druto".to_string(), "nonexistent".to_string()]);
    assert_ne!(exit_unknown, 0);
}

#[test]
fn test_cli_key_generate_show() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "key:generate".to_string(),
        "--show".to_string(),
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_key_generate_write_to_env() {
    let temp = tempdir().expect("tempdir");
    let env_path = temp.path().join(".env");
    let env_str = env_path.to_str().expect("valid path").to_string();

    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "key:generate".to_string(),
        "--env".to_string(),
        env_str,
    ]);
    assert!(result.is_ok());

    let content = fs::read_to_string(&env_path).expect("read env");
    assert!(content.contains("APP_KEY=base64:"));
    let key_line = content
        .lines()
        .find(|l| l.starts_with("APP_KEY="))
        .expect("APP_KEY line");
    assert!(key_line.len() > 15);
}

#[test]
fn test_cli_key_generate_overwrite_with_force() {
    let temp = tempdir().expect("tempdir");
    let env_path = temp.path().join(".env");
    fs::write(&env_path, "PORT=5555\nAPP_KEY=base64:old_dummy_key\n").expect("write dummy");

    let env_str = env_path.to_str().expect("valid path").to_string();
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "key:generate".to_string(),
        "--env".to_string(),
        env_str,
        "--force".to_string(),
    ]);
    assert!(result.is_ok());

    let content = fs::read_to_string(&env_path).expect("read env");
    assert!(content.contains("PORT=5555"));
    assert!(!content.contains("old_dummy_key"));
    assert!(content.contains("APP_KEY=base64:"));
}

#[test]
fn test_cli_key_generate_alias_resolution() {
    let kernel = create_kernel();
    let result = kernel.handle(vec![
        "druto".to_string(),
        "generate:key".to_string(),
        "--show".to_string(),
    ]);
    assert!(result.is_ok());
}

static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_cli_binary_execution() {
    let output_druto = std::process::Command::new(env!("CARGO_BIN_EXE_druto"))
        .arg("--version")
        .output()
        .expect("execute druto binary");
    assert!(output_druto.status.success());
    let version_str = String::from_utf8_lossy(&output_druto.stdout);
    assert!(version_str.contains("druto 0.1.0"));

    let output_cargo_druto = std::process::Command::new(env!("CARGO_BIN_EXE_cargo-druto"))
        .args(["druto", "--version"])
        .output()
        .expect("execute cargo-druto binary");
    assert!(output_cargo_druto.status.success());
    let cargo_version_str = String::from_utf8_lossy(&output_cargo_druto.stdout);
    assert!(cargo_version_str.contains("druto 0.1.0"));
}

#[test]
fn test_injector_route_module_comprehensive() {
    let temp = tempdir().expect("tempdir");

    // 1. Injects into non-existent routes/mod.rs (creates file and parent dir)
    let routes_mod = temp.path().join("src").join("routes").join("mod.rs");
    let outcome = druto::cli::injector::inject_route_module(&routes_mod, "users").unwrap();
    assert!(outcome.module_injected);
    assert!(outcome.route_mounted);

    let content = fs::read_to_string(&routes_mod).unwrap();
    assert!(content.contains("pub mod users;"));
    assert!(content.contains("router.nest(\"/users\", users::routes());"));

    // 2. Idempotent call
    let second_outcome = druto::cli::injector::inject_route_module(&routes_mod, "users").unwrap();
    assert!(!second_outcome.module_injected);
    assert!(!second_outcome.route_mounted);

    // 3. Inserts after existing module declarations
    let auth_outcome = druto::cli::injector::inject_route_module(&routes_mod, "auth").unwrap();
    assert!(auth_outcome.module_injected);
    assert!(auth_outcome.route_mounted);
    let auth_content = fs::read_to_string(&routes_mod).unwrap();
    assert!(auth_content.contains("pub mod auth;"));
    assert!(auth_content.contains("router.nest(\"/auth\", auth::routes());"));

    // 4. Inserts after use statement when no module statements exist
    let routes_use_only = temp.path().join("src").join("routes").join("use_only.rs");
    fs::write(
        &routes_use_only,
        "use druto::prelude::*;\n\npub fn routes() -> Router {\n    let mut router = Router::new();\n    router\n}\n",
    )
    .unwrap();
    let use_outcome =
        druto::cli::injector::inject_route_module(&routes_use_only, "products").unwrap();
    assert!(use_outcome.module_injected);
    assert!(use_outcome.route_mounted);
    let use_content = fs::read_to_string(&routes_use_only).unwrap();
    assert!(use_content.contains("pub mod products;"));

    // 5. Inserts at top when no use and no mod statements exist
    let routes_bare = temp.path().join("src").join("routes").join("bare.rs");
    fs::write(
        &routes_bare,
        "pub fn routes() -> Router {\n    let mut router = Router::new();\n    router\n}\n",
    )
    .unwrap();
    let bare_outcome = druto::cli::injector::inject_route_module(&routes_bare, "billing").unwrap();
    assert!(bare_outcome.module_injected);
    assert!(bare_outcome.route_mounted);

    // 6. Mounts before closing brace fallback when function has no return router line
    let routes_closing_brace = temp.path().join("src").join("routes").join("brace.rs");
    fs::write(
        &routes_closing_brace,
        "pub fn routes() -> Router {\n    Router::new()\n}\n",
    )
    .unwrap();
    let brace_outcome =
        druto::cli::injector::inject_route_module(&routes_closing_brace, "invoices").unwrap();
    assert!(brace_outcome.module_injected);
    assert!(brace_outcome.route_mounted);
}

#[test]
fn test_injector_controller_module_comprehensive() {
    let temp = tempdir().expect("tempdir");

    // 1. Injects into non-existent controllers/mod.rs
    let controllers_mod = temp.path().join("src").join("controllers").join("mod.rs");
    let injected =
        druto::cli::injector::inject_controller_module(&controllers_mod, "users").unwrap();
    assert!(injected);
    let content = fs::read_to_string(&controllers_mod).unwrap();
    assert_eq!(content, "pub mod users;\n");

    // 2. Idempotent check
    let second_injected =
        druto::cli::injector::inject_controller_module(&controllers_mod, "users").unwrap();
    assert!(!second_injected);

    // 3. Inserts after existing module declarations
    let post_injected =
        druto::cli::injector::inject_controller_module(&controllers_mod, "posts").unwrap();
    assert!(post_injected);
    let post_content = fs::read_to_string(&controllers_mod).unwrap();
    assert!(post_content.contains("pub mod users;\npub mod posts;\n"));

    // 4. Appends when file has no module declarations
    let controllers_comments = temp
        .path()
        .join("src")
        .join("controllers")
        .join("comment_mod.rs");
    fs::write(&controllers_comments, "// Controller index\n").unwrap();
    let comment_injected =
        druto::cli::injector::inject_controller_module(&controllers_comments, "comments").unwrap();
    assert!(comment_injected);
    let comment_content = fs::read_to_string(&controllers_comments).unwrap();
    assert!(comment_content.contains("pub mod comments;"));
}

#[test]
fn test_cli_make_write_execution_workflow() {
    let guard = CWD_LOCK.lock().unwrap();
    let temp = tempdir().expect("tempdir");
    let ui = karigor::ui::Ui::new();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();

    // 1. make:route without controller
    let route_res = druto::cli::commands::make::execute_make_route(&ui, "customers", false, false);
    assert!(route_res.is_ok());
    assert!(temp.path().join("src/routes/customers.rs").exists());
    assert!(temp.path().join("src/routes/mod.rs").exists());

    // 2. make:route with controller
    let paired_res = druto::cli::commands::make::execute_make_route(&ui, "orders", true, false);
    assert!(paired_res.is_ok());
    assert!(temp.path().join("src/routes/orders.rs").exists());
    assert!(temp.path().join("src/controllers/orders.rs").exists());

    // Duplicate route execution to test already-present logging branch
    let duplicate_route_res =
        druto::cli::commands::make::execute_make_route(&ui, "orders", true, false);
    assert!(duplicate_route_res.is_ok());

    // 3. make:controller (resource)
    let controller_resource_res =
        druto::cli::commands::make::execute_make_controller(&ui, "invoices", true, false);
    assert!(controller_resource_res.is_ok());
    let invoice_ctrl_content =
        fs::read_to_string(temp.path().join("src/controllers/invoices.rs")).unwrap();
    assert!(invoice_ctrl_content.contains("pub fn index"));
    assert!(invoice_ctrl_content.contains("pub fn destroy"));

    // 4. make:controller (API)
    let controller_api_res =
        druto::cli::commands::make::execute_make_controller(&ui, "reports", false, false);
    assert!(controller_api_res.is_ok());

    // Duplicate controller execution
    let duplicate_ctrl_res =
        druto::cli::commands::make::execute_make_controller(&ui, "reports", false, false);
    assert!(duplicate_ctrl_res.is_ok());

    // 5. make:middleware
    let mw_res = druto::cli::commands::make::execute_make_middleware(&ui, "audit_logger", false);
    assert!(mw_res.is_ok());
    assert!(temp.path().join("src/middleware/audit_logger.rs").exists());

    // 6. make:command
    let cmd_res = druto::cli::commands::make::execute_make_command(&ui, "backup_database", false);
    assert!(cmd_res.is_ok());
    assert!(temp.path().join("src/commands/backup_database.rs").exists());

    // 7. stub:publish
    let stub_publish_res = druto::cli::commands::make::execute_stub_publish(&ui, false);
    assert!(stub_publish_res.is_ok());
    assert!(temp.path().join("stubs/route.stub").exists());

    // Re-publishing without force should skip existing stubs
    let skip_stubs_res = druto::cli::commands::make::execute_stub_publish(&ui, false);
    assert!(skip_stubs_res.is_ok());

    // Re-publishing with force should overwrite stubs
    let force_stubs_res = druto::cli::commands::make::execute_stub_publish(&ui, true);
    assert!(force_stubs_res.is_ok());

    std::env::set_current_dir(original_cwd).unwrap();
    drop(guard);
}

#[test]
fn test_cli_stubs_custom_override_and_render_error() {
    let guard = CWD_LOCK.lock().unwrap();
    let temp = tempdir().expect("tempdir");
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();

    let stubs_dir = temp.path().join("stubs");
    fs::create_dir_all(&stubs_dir).unwrap();
    fs::write(stubs_dir.join("route.stub"), "// Custom stub override\n").unwrap();

    let stub = druto::cli::stubs::get_stub("route.stub");
    assert_eq!(stub, "// Custom stub override\n");

    let fallback_stub = druto::cli::stubs::get_stub("middleware.stub");
    assert!(fallback_stub.contains("Middleware"));

    let err_render =
        druto::cli::stubs::render_stub("Hello {{ invalid_syntax", &serde_json::json!({}));
    assert!(err_render.is_err());

    std::env::set_current_dir(original_cwd).unwrap();
    drop(guard);
}

#[test]
fn test_cli_key_generate_base64_vectors_and_edge_cases() {
    use druto::cli::commands::key::encode_base64;

    assert_eq!(encode_base64(b""), "");
    assert_eq!(encode_base64(b"f"), "Zg==");
    assert_eq!(encode_base64(b"fo"), "Zm8=");
    assert_eq!(encode_base64(b"foo"), "Zm9v");
    assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
    assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
    assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");

    let guard = CWD_LOCK.lock().unwrap();
    let temp = tempdir().expect("tempdir");
    let ui = karigor::ui::Ui::new();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();

    // 1. When .env is absent but .env.example exists in current directory
    fs::write(".env.example", "APP_NAME=DrutoApp\nAPP_KEY=\n").unwrap();
    let res = druto::cli::commands::key::execute_key_generate(&ui, false, false, ".env");
    assert!(res.is_ok());
    let env_content = fs::read_to_string(".env").unwrap();
    assert!(env_content.contains("APP_NAME=DrutoApp"));
    assert!(env_content.contains("APP_KEY=base64:"));

    // 2. Overwrite error when APP_KEY already exists and force is false
    karigor::ui::set_non_interactive(true);
    let err_res = druto::cli::commands::key::execute_key_generate(&ui, false, false, ".env");
    assert!(err_res.is_err());
    karigor::ui::set_non_interactive(false);

    // 3. Overwrite success when force is true
    let force_res = druto::cli::commands::key::execute_key_generate(&ui, false, true, ".env");
    assert!(force_res.is_ok());

    // 4. Append APP_KEY to existing .env that lacks APP_KEY
    fs::write(
        ".env.custom",
        "DATABASE_URL=postgres://127.0.0.1:5432/druto\n",
    )
    .unwrap();
    let append_res =
        druto::cli::commands::key::execute_key_generate(&ui, false, false, ".env.custom");
    assert!(append_res.is_ok());
    let appended_content = fs::read_to_string(".env.custom").unwrap();
    assert!(appended_content.contains("DATABASE_URL="));
    assert!(appended_content.contains("APP_KEY=base64:"));

    // 5. Create new file when both .env and .env.example are absent
    let fresh_res =
        druto::cli::commands::key::execute_key_generate(&ui, false, false, ".env.fresh");
    assert!(fresh_res.is_ok());
    let fresh_content = fs::read_to_string(".env.fresh").unwrap();
    assert!(fresh_content.contains("APP_KEY=base64:"));

    std::env::set_current_dir(original_cwd).unwrap();
    drop(guard);
}

#[test]
fn test_cli_dev_and_new_branches() {
    let kernel = create_kernel();

    // Test dev alias serve
    let serve_res = kernel.handle(vec![
        "druto".to_string(),
        "serve".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(serve_res.is_ok());

    // Test dev dry-run with defaults (no host/port)
    let dev_default_res = kernel.handle(vec![
        "druto".to_string(),
        "dev".to_string(),
        "--dry-run".to_string(),
    ]);
    assert!(dev_default_res.is_ok());

    // Test new command when directory already exists
    let temp = tempdir().expect("tempdir");
    let existing_path = temp.path().to_str().unwrap().to_string();
    let new_err_res = kernel.handle(vec!["druto".to_string(), "new".to_string(), existing_path]);
    assert!(new_err_res.is_err());

    // Test dev non-dry-run warning when no Cargo.toml exists
    let guard = CWD_LOCK.lock().unwrap();
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    let ui = karigor::ui::Ui::new();
    let dev_no_cargo_res = druto::cli::commands::dev::execute_dev(&ui, None, None, false);
    assert!(dev_no_cargo_res.is_ok());

    // Test new default project scaffolding in empty workspace
    karigor::ui::set_non_interactive(true);
    let new_default_res = druto::cli::commands::new::execute_new(&ui, None);
    assert!(new_default_res.is_ok());
    assert!(temp.path().join("my-druto-app").exists());
    assert!(temp.path().join("my-druto-app/Cargo.toml").exists());
    karigor::ui::set_non_interactive(false);

    std::env::set_current_dir(original_cwd).unwrap();
    drop(guard);
}
