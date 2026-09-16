use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use monolith::{
    auth::{AuthMode, Role},
    backend::BackendState,
    backend_config::BackendConfig,
    db::Database,
    inventory::scan_root,
};
use std::{io::Read, path::PathBuf};

#[derive(Parser)]
#[command(name = "monolith-admin", about = "Administration du backend Monolith")]
struct Cli {
    #[arg(
        long,
        default_value = "config/backend.toml",
        env = "MONOLITH_BACKEND_CONFIG"
    )]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    Init {
        #[arg(long)]
        force: bool,
    },
    Show,
    Check,
}

#[derive(Subcommand)]
enum AuthCommand {
    Set {
        #[arg(value_enum)]
        mode: AuthMode,
        #[arg(long)]
        issuer: Option<String>,
        #[arg(long)]
        audience: Option<String>,
        #[arg(long)]
        jwks_url: Option<String>,
        #[arg(long)]
        auto_provision: bool,
    },
}

#[derive(Subcommand)]
enum LibraryCommand {
    Scan,
    Status {
        #[arg(long)]
        system_id: Option<i64>,
    },
}

#[derive(Subcommand)]
enum UserCommand {
    Add {
        username: String,
        #[arg(long, value_enum, default_value = "standard")]
        role: Role,
        #[arg(long, help = "Lire le mot de passe depuis l'entrée standard")]
        password_stdin: bool,
    },
    Enable {
        user_id: i64,
    },
    Disable {
        user_id: i64,
    },
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Config { command } => run_config(command, &cli.config),
        Command::Auth { command } => run_auth(command, &cli.config),
        Command::User { command } => run_user(command, &cli.config),
        Command::Library { command } => run_library(command, &cli.config),
    }
}

fn run_config(command: ConfigCommand, path: &std::path::Path) -> Result<()> {
    match command {
        ConfigCommand::Init { force } => {
            if path.exists() && !force {
                bail!("{} existe déjà (utilisez --force)", path.display());
            }
            BackendConfig::default().save(path)?;
            println!("Configuration créée : {}", path.display());
        }
        ConfigCommand::Show => println!("{}", toml::to_string_pretty(&BackendConfig::load(path)?)?),
        ConfigCommand::Check => {
            BackendConfig::load(path)?;
            println!("Configuration valide");
        }
    }
    Ok(())
}

fn run_auth(command: AuthCommand, path: &std::path::Path) -> Result<()> {
    let mut config = BackendConfig::load(path)?;
    match command {
        AuthCommand::Set {
            mode,
            issuer,
            audience,
            jwks_url,
            auto_provision,
        } => {
            config.auth.mode = mode;
            if issuer.is_some() {
                config.auth.oidc.issuer = issuer;
            }
            if audience.is_some() {
                config.auth.oidc.audience = audience;
            }
            if jwks_url.is_some() {
                config.auth.oidc.jwks_url = jwks_url;
            }
            config.auth.oidc.auto_provision = auto_provision;
            config.save(path)?;
            println!("Mode d'authentification : {:?}", mode);
        }
    }
    Ok(())
}

fn run_library(command: LibraryCommand, config_path: &std::path::Path) -> Result<()> {
    let config = BackendConfig::load(config_path)?;
    let directory = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let state = BackendState::new(config, directory)?;
    let database = Database::open(&state.database_path)?;

    match command {
        LibraryCommand::Scan => {
            let mut visited = 0;
            let mut accepted = 0;
            let mut ignored = 0;
            let mut missing = 0;
            let mut issues = 0;

            for root in &state.config.library.roots {
                let report = scan_root(root)?;
                visited += report.visited;
                accepted += report.accepted;
                ignored += report.ignored;
                issues += report.issues.len();
                if report.issues.is_empty() {
                    missing += database.sync_rom_inventory(root, &report.observations)?;
                }
            }
            println!(
                "Scan terminé : visités={visited} acceptés={accepted} ignorés={ignored} absents={missing} erreurs={issues}"
            );
        }
        LibraryCommand::Status { system_id } => {
            println!("SYSTEM_ID\tDISPONIBLES\tABSENTS");
            let mut counts = std::collections::BTreeMap::<i64, (usize, usize)>::new();
            for location in database.rom_locations()? {
                if system_id.is_some_and(|id| id != location.system_id) {
                    continue;
                }
                let count = counts.entry(location.system_id).or_default();
                match location.availability {
                    monolith::models::RomAvailability::Available => count.0 += 1,
                    monolith::models::RomAvailability::Missing => count.1 += 1,
                }
            }
            for (id, (available, missing)) in counts {
                println!("{id}\t{available}\t{missing}");
            }
        }
    }
    Ok(())
}

fn run_user(command: UserCommand, config_path: &std::path::Path) -> Result<()> {
    let config = BackendConfig::load(config_path)?;
    let directory = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let state = BackendState::new(config, directory)?;
    let database = Database::open(&state.database_path)?;
    match command {
        UserCommand::Add {
            username,
            role,
            password_stdin,
        } => {
            if !password_stdin {
                bail!(
                    "--password-stdin est requis pour éviter les secrets dans l'historique shell"
                );
            }
            let mut password = String::new();
            std::io::stdin()
                .read_to_string(&mut password)
                .context("lecture du mot de passe")?;
            let user_id = database.create_local_user(&username, password.trim_end(), role)?;
            println!("Utilisateur créé : {user_id} ({username})");
        }
        UserCommand::Enable { user_id } => {
            database.set_user_enabled(user_id, true)?;
            println!("Utilisateur {user_id} activé");
        }
        UserCommand::Disable { user_id } => {
            database.set_user_enabled(user_id, false)?;
            println!("Utilisateur {user_id} désactivé et sessions révoquées");
        }
        UserCommand::List => {
            println!("ID\tACTIF\tSOURCE\tRÔLE\tUTILISATEUR");
            for user in database.list_users()? {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    user.user_id,
                    user.enabled,
                    if user.sso { "sso" } else { "local" },
                    user.role,
                    user.username
                );
            }
        }
    }
    Ok(())
}
