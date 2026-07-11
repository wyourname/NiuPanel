use crate::modules::user::service::UserService;
use crate::startup::bootstrap::{connect_database, run_migrations};
use anyhow::{Context, Result};
use migration::{Migrator, MigratorTrait};
use niupanel_common::auth::validate_password_strength;
use niupanel_common::config::Config;
use niupanel_core::settings::SettingsManager;
use niupanel_entity::users;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::env;
use std::io::{self, Write};

pub async fn handle_commands() -> Result<bool> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Ok(false);
    }

    match args[1].as_str() {
        "reset" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "account" => {
                        let username = args.get(3).cloned();
                        let password = args.get(4).cloned();
                        handle_reset_account(username, password).await?;
                        return Ok(true);
                    }
                    "2fa" => {
                        handle_reset_2fa().await?;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        "user" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "list" => {
                        handle_user_list().await?;
                        return Ok(true);
                    }
                    "create" => {
                        let is_admin = args.contains(&"--admin".to_string());
                        // filter out --admin from args
                        let mut filtered_args = args.clone();
                        if let Some(pos) = filtered_args.iter().position(|x| x == "--admin") {
                            filtered_args.remove(pos);
                        }
                        let username = filtered_args.get(3).cloned();
                        let password = filtered_args.get(4).cloned();
                        handle_user_create(username, password, is_admin).await?;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        "maintenance" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "cleanup" => {
                        let days = args
                            .get(3)
                            .and_then(|d| d.parse::<i64>().ok())
                            .unwrap_or(30);
                        handle_maintenance_cleanup(days).await?;
                        return Ok(true);
                    }
                    "db-backup" => {
                        let path = args.get(3).cloned();
                        handle_maintenance_db_backup(path).await?;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        "migration" | "migrate" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "status" => {
                        handle_migration_status().await?;
                        return Ok(true);
                    }
                    "up" => {
                        let steps = parse_optional_steps(args.get(3))?;
                        handle_migration_up(steps).await?;
                        return Ok(true);
                    }
                    "down" => {
                        let steps = parse_optional_steps(args.get(3))?.or(Some(1));
                        handle_migration_down(steps).await?;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        "config" => {
            if args.len() > 2 && args[2] == "set" {
                let key = args.get(3).cloned();
                let value = args.get(4).cloned();
                handle_config_set(key, value).await?;
                return Ok(true);
            }
        }
        "plugin" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "keygen" => {
                        let private_key_path = args
                            .get(3)
                            .cloned()
                            .unwrap_or_else(|| "plugin-ed25519.pem".to_string());
                        let public_key_path = args
                            .get(4)
                            .cloned()
                            .unwrap_or_else(|| "plugin-ed25519.pub".to_string());
                        handle_plugin_keygen(private_key_path, public_key_path)?;
                        return Ok(true);
                    }
                    "pack" => {
                        handle_plugin_pack(&args[3..])?;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        "status" => {
            handle_status().await?;
            return Ok(true);
        }
        "help" | "--help" | "-h" => {
            print_help();
            return Ok(true);
        }
        _ => {}
    }

    Ok(false)
}

async fn handle_reset_account(username: Option<String>, password: Option<String>) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;
    run_migrations(&db).await?;

    let username = match username {
        Some(u) => u,
        None => {
            let mut u = String::new();
            print!("Enter username to reset: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut u)?;
            u.trim().to_string()
        }
    };

    if username.is_empty() {
        println!("Error: Username cannot be empty");
        return Ok(());
    }

    let user = users::Entity::find()
        .filter(users::Column::Username.eq(&username))
        .one(&db)
        .await?;

    if let Some(user) = user {
        let password = match password {
            Some(p) => p,
            None => {
                let mut p = String::new();
                print!("Enter new password for {}: ", username);
                io::stdout().flush()?;
                io::stdin().read_line(&mut p)?;
                p.trim().to_string()
            }
        };

        if let Err(e) = validate_password_strength(&password) {
            println!("Error: Invalid password - {}", e);
            return Ok(());
        }

        UserService::update_password(&db, user.id, &password)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update password: {}", e))?;
        println!("Successfully updated password for user: {}", username);
    } else {
        println!("Error: User '{}' not found", username);
    }

    Ok(())
}

async fn handle_reset_2fa() -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;

    let config_str = SettingsManager::get(&db, "plugin.telegram.config")
        .await
        .unwrap_or_default();

    if config_str.is_empty() {
        println!("Telegram plugin config not found. 2FA is likely not enabled via Telegram.");
        return Ok(());
    }

    let mut json_val: serde_json::Value = serde_json::from_str(&config_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse telegram config: {}", e))?;

    if let Some(obj) = json_val.as_object_mut() {
        if let Some(login_2fa) = obj.get_mut("login_2fa") {
            if login_2fa.as_bool() == Some(true) {
                *login_2fa = serde_json::Value::Bool(false);
                let new_config_str = serde_json::to_string(&json_val)?;
                SettingsManager::set(
                    &db,
                    "plugin.telegram.config",
                    &new_config_str,
                    Some("Notification"),
                )
                .await?;
                println!("Successfully disabled Telegram 2FA.");
            } else {
                println!("Telegram 2FA is already disabled.");
            }
        } else {
            println!("login_2fa field not found in telegram config.");
        }
    }

    Ok(())
}

async fn handle_user_list() -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;

    let users = users::Entity::find().all(&db).await?;
    println!(
        "{:<5} | {:<15} | {:<10} | {:<25}",
        "ID", "Username", "Role", "Email"
    );
    println!("{:-<5}-|-{:-<15}-|-{:-<10}-|-{:-<25}", "", "", "", "");
    for user in users {
        println!(
            "{:<5} | {:<15} | {:<10} | {:<25}",
            user.id,
            user.username,
            user.role,
            user.email.unwrap_or_default()
        );
    }
    Ok(())
}

async fn handle_user_create(
    username: Option<String>,
    password: Option<String>,
    is_admin: bool,
) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;
    run_migrations(&db).await?;

    let username = match username {
        Some(u) => u,
        None => {
            let mut u = String::new();
            print!("Enter username to create: ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut u)?;
            u.trim().to_string()
        }
    };

    if username.is_empty() {
        println!("Error: Username cannot be empty");
        return Ok(());
    }

    // Check if user already exists
    let existing = users::Entity::find()
        .filter(users::Column::Username.eq(&username))
        .one(&db)
        .await?;

    if existing.is_some() {
        println!("Error: User '{}' already exists", username);
        return Ok(());
    }

    let password = match password {
        Some(p) => p,
        None => {
            let mut p = String::new();
            print!("Enter password for {}: ", username);
            io::stdout().flush()?;
            io::stdin().read_line(&mut p)?;
            p.trim().to_string()
        }
    };

    if let Err(e) = validate_password_strength(&password) {
        println!("Error: Invalid password - {}", e);
        return Ok(());
    }

    // Use UserService logic to hash password
    use argon2::{
        Argon2, PasswordHasher,
        password_hash::{SaltString, rand_core::OsRng},
    };
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
        .to_string();

    let new_user = users::ActiveModel {
        username: Set(username.clone()),
        password_hash: Set(password_hash),
        role: Set(if is_admin {
            "admin".to_string()
        } else {
            "user".to_string()
        }),
        email_verified: Set(true),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    new_user.insert(&db).await?;
    println!(
        "Successfully created user: {} (Role: {})",
        username,
        if is_admin { "admin" } else { "user" }
    );

    Ok(())
}

async fn handle_maintenance_cleanup(days: i64) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;

    println!("Starting maintenance cleanup (older than {} days)...", days);

    // Cleanup logs
    match crate::modules::settings::service::cleanup_logs(&db, days).await {
        Ok(count) => println!("Cleaned up {} log files and database records.", count),
        Err(e) => println!("Warning: Failed to cleanup logs: {}", e),
    }

    Ok(())
}

async fn handle_maintenance_db_backup(path: Option<String>) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;

    let db_path_str = config.database_url.trim_start_matches("sqlite://");
    let db_path_clean = db_path_str.split('?').next().unwrap_or(db_path_str);
    let source_path = std::path::PathBuf::from(db_path_clean);

    if !source_path.exists() {
        println!("Error: Database file not found at {:?}", source_path);
        return Ok(());
    }

    let dest_path = match path {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let mut p = config.backup_dir.clone();
            p.push(format!("niupanel_db_{}.sqlite", timestamp));
            p
        }
    };

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(&source_path, &dest_path)?;
    println!("Database backup created at: {:?}", dest_path);

    Ok(())
}

fn parse_optional_steps(value: Option<&String>) -> Result<Option<u32>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let steps = value
        .parse::<u32>()
        .with_context(|| format!("Invalid migration step count: {value}"))?;
    if steps == 0 {
        return Ok(None);
    }
    Ok(Some(steps))
}

async fn handle_migration_status() -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;
    let migrations = Migrator::get_migration_with_status(&db).await?;

    println!("{:<48} | Status", "Migration");
    println!("{:-<48}-|-{:-<8}", "", "");
    for migration in migrations {
        println!("{:<48} | {}", migration.name(), migration.status());
    }

    Ok(())
}

async fn handle_migration_up(steps: Option<u32>) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;
    Migrator::up(&db, steps).await?;
    println!(
        "Migrations applied{}.",
        steps
            .map(|steps| format!(" ({steps} step{})", if steps == 1 { "" } else { "s" }))
            .unwrap_or_else(|| " (all pending)".to_string())
    );
    Ok(())
}

async fn handle_migration_down(steps: Option<u32>) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;
    Migrator::down(&db, steps).await?;
    println!(
        "Migrations rolled back{}.",
        steps
            .map(|steps| format!(" ({steps} step{})", if steps == 1 { "" } else { "s" }))
            .unwrap_or_else(|| " (all applied)".to_string())
    );
    Ok(())
}

async fn handle_config_set(key: Option<String>, value: Option<String>) -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    let db = connect_database(&config.database_url).await?;

    let key = match key {
        Some(k) => k,
        None => {
            println!("Error: Configuration key is required.");
            return Ok(());
        }
    };

    let value = match value {
        Some(v) => v,
        None => {
            println!("Error: Configuration value is required.");
            return Ok(());
        }
    };

    SettingsManager::set(&db, &key, &value, Some("CLI Override")).await?;
    println!("Successfully set configuration: {} = {}", key, value);

    Ok(())
}

fn handle_plugin_keygen(private_key_path: String, public_key_path: String) -> Result<()> {
    use openssl::base64;
    use openssl::pkey::PKey;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::Path;

    let key = PKey::generate_ed25519().context("Failed to generate Ed25519 key")?;
    let private_pem = key
        .private_key_to_pem_pkcs8()
        .context("Failed to encode private key")?;
    let public_key = key
        .raw_public_key()
        .context("Failed to export raw public key")?;

    write_file_with_parent(Path::new(&private_key_path), &private_pem)?;
    write_file_with_parent(
        Path::new(&public_key_path),
        base64::encode_block(&public_key).as_bytes(),
    )?;

    println!("Plugin signing key generated");
    println!("Private key: {}", private_key_path);
    println!("Public key:  {}", public_key_path);
    println!(
        "Trusted key: sha256:{}",
        hex::encode(Sha256::digest(&public_key))
    );
    println!("Set TRUSTED_PLUGIN_PUBLIC_KEYS to the trusted key value above.");

    // Restrict private key permissions on Unix. Ignore failures on non-Unix filesystems.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut permissions) = fs::metadata(&private_key_path).map(|meta| meta.permissions())
        {
            permissions.set_mode(0o600);
            let _ = fs::set_permissions(&private_key_path, permissions);
        }
    }

    Ok(())
}

fn handle_plugin_pack(args: &[String]) -> Result<()> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use openssl::base64;
    use openssl::pkey::PKey;
    use openssl::sign::Signer;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use tar::Builder;

    if args.len() < 2 {
        println!("Error: plugin pack requires <source_dir> <output.tgz>");
        println!("Usage: niupanel plugin pack <source_dir> <output.tgz> [--key private.pem]");
        return Ok(());
    }

    let source_dir = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);
    let private_key_path = parse_flag_value(args, "--key").map(PathBuf::from);

    if !source_dir.is_dir() {
        println!(
            "Error: plugin source directory not found: {}",
            source_dir.display()
        );
        return Ok(());
    }
    if !source_dir.join("plugin.json").is_file() && !source_dir.join("agent.json").is_file() {
        println!("Error: plugin source directory must contain plugin.json or agent.json");
        return Ok(());
    }
    let canonical_source = source_dir.canonicalize().with_context(|| {
        format!(
            "Failed to resolve source directory {}",
            source_dir.display()
        )
    })?;
    let absolute_output = if output_path.is_absolute() {
        output_path.clone()
    } else {
        std::env::current_dir()?.join(&output_path)
    };
    if normalize_path_lexically(&absolute_output).starts_with(&canonical_source) {
        println!("Error: output package path must not be inside the plugin source directory");
        return Ok(());
    }
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create package {}", output_path.display()))?;
    let encoder = GzEncoder::new(output_file, Compression::default());
    let mut tar = Builder::new(encoder);
    append_plugin_package_dir(&mut tar, &source_dir, Path::new(""))?;
    tar.finish()?;
    let encoder = tar.into_inner()?;
    encoder.finish()?;

    let package_bytes = fs::read(&output_path)
        .with_context(|| format!("Failed to read package {}", output_path.display()))?;
    let checksum = hex::encode(Sha256::digest(&package_bytes));

    println!("Plugin package created");
    println!("Package:         {}", output_path.display());
    println!("checksum_sha256: {}", checksum);

    if let Some(private_key_path) = private_key_path {
        let private_key_pem = fs::read(&private_key_path).with_context(|| {
            format!("Failed to read private key {}", private_key_path.display())
        })?;
        let key = PKey::private_key_from_pem(&private_key_pem)
            .with_context(|| format!("Invalid private key {}", private_key_path.display()))?;
        let public_key = key
            .raw_public_key()
            .context("Failed to export raw public key")?;
        let mut signer = Signer::new_without_digest(&key).context("Failed to create signer")?;
        let signature = signer
            .sign_oneshot_to_vec(&package_bytes)
            .context("Failed to sign plugin package")?;

        println!("signature_ed25519: {}", base64::encode_block(&signature));
        println!("public_key_ed25519: {}", base64::encode_block(&public_key));
        println!(
            "trusted_key: sha256:{}",
            hex::encode(Sha256::digest(&public_key))
        );
    }

    Ok(())
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

fn append_plugin_package_dir(
    tar: &mut tar::Builder<flate2::write::GzEncoder<std::fs::File>>,
    source_dir: &std::path::Path,
    relative_dir: &std::path::Path,
) -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let current = source_dir.join(relative_dir);
    let mut entries = fs::read_dir(&current)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(anyhow::anyhow!(
                "Plugin packages cannot contain symlinks: {}",
                entry.path().display()
            ));
        }
        let relative_path: PathBuf = relative_dir.join(entry.file_name());
        if file_type.is_dir() {
            tar.append_dir(&relative_path, entry.path())?;
            append_plugin_package_dir(tar, source_dir, &relative_path)?;
        } else if file_type.is_file() {
            tar.append_path_with_name(entry.path(), &relative_path)?;
        }
    }

    Ok(())
}

fn write_file_with_parent(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn normalize_path_lexically(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::{Component, PathBuf};

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

async fn handle_status() -> Result<()> {
    let config = Config::from_env().context("Failed to load configuration")?;
    println!("NiuPanel System Status");
    println!("----------------------");
    println!("Version:      {}", env!("CARGO_PKG_VERSION"));
    println!("Database URL: {}", config.database_url);
    println!("Server Addr:  {}", config.server_addr);
    println!("Scripts Dir:  {}", config.scripts_dir.display());
    println!("Logs Dir:     {}", config.logs_dir.display());
    println!("Backup Dir:   {}", config.backup_dir.display());

    let db = connect_database(&config.database_url).await;
    match db {
        Ok(_) => println!("Database:     Connected"),
        Err(e) => println!("Database:     Error ({})", e),
    }

    Ok(())
}

fn print_help() {
    println!("NiuPanel 命令行管理工具");
    println!("");
    println!("用法:");
    println!("  niupanel <command> [arguments]");
    println!("");
    println!("命令:");
    println!("  reset account [用户名] [密码]          重置账号密码");
    println!("  reset 2fa                              关闭全局 Telegram 二次验证");
    println!("  user list                              列出所有用户");
    println!("  user create [--admin] [用户] [密码]    创建用户账号");
    println!("  maintenance cleanup [天数]             清理旧日志，默认 30 天");
    println!("  maintenance db-backup [路径]           导出数据库原始备份");
    println!("  migration status                       查看数据库迁移状态");
    println!("  migration up [步数]                    执行待应用的迁移，默认全部");
    println!("  migration down [步数]                  回退已应用的迁移，默认 1 步");
    println!("  config set <键> <值>                   强制更新系统配置");
    println!("  plugin keygen [私钥路径] [公钥路径]    生成 Ed25519 插件签名密钥");
    println!("  plugin pack <目录> <包.tgz> [--key k]  打包插件，可选签名");
    println!("  status                                 查看系统状态");
    println!("  help                                   显示本帮助");
}
