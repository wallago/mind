//! MCP server exposing a Syncthing-shared notes folder: cheatsheets, things worth knowing, advice, and interesting finds.

/// Main application.
pub mod app;
/// Command-line arguments parser.
pub mod args;
/// Config file.
pub mod config;
/// Error handler implementation.
pub mod error;
/// Helper functions.
pub mod help;
/// Common types that can be glob-imported for convenience.
pub mod prelude;

/// Web layer.
pub mod server;

/// MCP server layer.
pub mod mcp;

/// The notes folder.
mod vault;

use prelude::*;

use crate::config::Config;

/// Runs `mind`.
///
/// # Errors
///
/// Returns an [`Error`] if the config can't be loaded or a component fails
/// to start.
pub async fn run(args: &Args) -> Result<()> {
    better_panic::install();
    let config = Config::load(args.config.as_deref())?;
    tracing::debug!("Loaded {config:?}");
    let vault = vault::Vault::open(&config.vault.root)?;

    // Runs until Ctrl-C or until any component stops.
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            tracing::info!("Interrupted: {:?}", result);
        },
        result = server::start(&config, vault) => {
            tracing::info!("Server stopped: {:?}", result);
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn example_config_loads() -> Result<()> {
        // Only written when an option adds settings.
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.toml");
        if path.exists() {
            Config::from_file(&path)?;
        }
        Ok(())
    }
}
