use anyhow::{Context, Result};
use clap::Parser;
use monolith::{backend, backend::BackendState, backend_config::BackendConfig};
use std::path::PathBuf;
use tower_http::trace::TraceLayer;

#[derive(Parser)]
#[command(name = "monolith-backend", about = "API backend de Monolith")]
struct Arguments {
    #[arg(
        long,
        default_value = "config/backend.toml",
        env = "MONOLITH_BACKEND_CONFIG"
    )]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let config = BackendConfig::load(&arguments.config)?;
    let directory = arguments
        .config
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let state = BackendState::new(config.clone(), directory)?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("écoute impossible sur {}", config.listen))?;
    println!("Monolith backend écoute sur http://{}", config.listen);
    axum::serve(
        listener,
        backend::router(state).layer(TraceLayer::new_for_http()),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
