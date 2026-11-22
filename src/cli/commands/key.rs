//! Cryptographic key generation command for Druto applications.

use std::{fs, io::IsTerminal, path::Path};

use karigor::prelude::*;
use tracing::info;

/// Command generating a 256-bit cryptographically secure application secret key.
pub struct KeyGenerateCommand;

impl Command for KeyGenerateCommand {
    fn name(&self) -> &'static str {
        "key:generate"
    }

    fn description(&self) -> &'static str {
        "Generate a 256-bit cryptographically secure application secret key"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["generate:key"]
    }

    fn flags(&self) -> Vec<Flag> {
        vec![
            Flag::boolean(
                "show",
                Some('s'),
                "Display the generated key in console without writing to .env",
            ),
            Flag::boolean(
                "force",
                Some('f'),
                "Overwrite existing APP_KEY in .env without confirmation",
            ),
            Flag::string(
                "env",
                None,
                "Path to the environment file to update",
                Some(".env".to_string()),
            ),
        ]
    }

    fn run(&self, ctx: &mut Context) -> Result<(), Error> {
        let show = ctx.flag_bool("show");
        let force = ctx.flag_bool("force");
        let env_path = ctx.flag_str("env").unwrap_or(".env");

        execute_key_generate(&ctx.ui, show, force, env_path)
            .map(|_| ())
            .map_err(|err| Error::Execution(err.to_string().into()))
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes raw byte slice to standard RFC 4648 Base64 string.
#[must_use]
pub fn encode_base64(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        result.push(BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        result.push(BASE64_ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(BASE64_ALPHABET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(BASE64_ALPHABET[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(feature = "crypto")]
fn generate_random_256_key() -> Result<String, Box<dyn std::error::Error>> {
    let key = gopon::Key::generate()?;
    Ok(key.to_prefixed_base64())
}

#[cfg(not(feature = "crypto"))]
fn generate_random_256_key() -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut buffer = [0u8; 32];
    let mut file = fs::File::open("/dev/urandom")?;
    file.read_exact(&mut buffer)?;
    Ok(format!("base64:{}", encode_base64(&buffer)))
}

/// Executes cryptographic key generation and environment variable writing.
pub fn execute_key_generate(
    ui: &Ui,
    show: bool,
    force: bool,
    env_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    info!(
        show = show,
        force = force,
        env = env_path,
        "Generating application secret key"
    );

    let key_str = generate_random_256_key()?;

    if show {
        ui.info(&format!("Generated Key: {}", key_str.bold().green()));
        return Ok(key_str);
    }

    let env_file = Path::new(env_path);

    if !env_file.exists() {
        let example_file = Path::new(".env.example");
        if example_file.exists() {
            fs::copy(example_file, env_file)?;
            ui.info(&format!(
                "Created {} from .env.example template.",
                env_path.cyan()
            ));
        } else {
            fs::write(env_file, "")?;
        }
    }

    let content = fs::read_to_string(env_file)?;
    let existing_value = content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with("APP_KEY=") {
            let (_, value) = trimmed.split_once('=')?;
            let clean = value.trim();
            if clean.is_empty() { None } else { Some(clean) }
        } else {
            None
        }
    });

    if existing_value.is_some() && !force {
        if !std::io::stdin().is_terminal() || karigor::ui::is_non_interactive() {
            return Err(
                "APP_KEY is already set in environment file. Use --force to overwrite it.".into(),
            );
        }

        let overwrite = ui.confirm(
            "An APP_KEY is already set. Do you want to overwrite it?",
            false,
        )?;
        if !overwrite {
            ui.info("Key generation aborted. Existing key preserved.");
            return Ok(key_str);
        }
    }

    let mut updated_lines: Vec<String> = Vec::new();
    let mut replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("APP_KEY=") || trimmed == "APP_KEY" {
            updated_lines.push(format!("APP_KEY={key_str}"));
            replaced = true;
        } else {
            updated_lines.push(line.to_string());
        }
    }

    if !replaced {
        if !updated_lines.is_empty() && updated_lines.last().is_some_and(|line| !line.is_empty()) {
            updated_lines.push(String::new());
        }
        updated_lines.push(format!("APP_KEY={key_str}"));
    }

    let mut new_content = updated_lines.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(env_file, new_content)?;

    ui.success(&format!(
        "Application key [{}] set successfully in {}.",
        key_str.bold().green(),
        env_path.cyan()
    ));

    Ok(key_str)
}
