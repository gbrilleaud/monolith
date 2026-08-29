use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use monolith::{
    auth::{AuthMode, Role},
    backend::BackendState,
    backend_config::BackendConfig,
    db::Database,
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
