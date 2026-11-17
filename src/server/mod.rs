//! Server core.

use axum::{Router, http::StatusCode, routing::get, serve};

use crate::{config::Config, prelude::*};

/// Liveness probe: answers as long as the server is up.
async fn health() -> StatusCode {
    StatusCode::OK
}

/// Global server routes.
fn routes() -> Router {
    let router = Router::new();
    router.route("/health", get(health))
}

/// Start web services.
pub(super) async fn start(config: &Config) -> Result<()> {
    let (address, port) = (&config.server.address, config.server.port);
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
    tracing::info!("Server running");
    let routes = routes();
    let routes = routes.nest_service("/mcp", crate::mcp::http::service(&config.mcp));
    Ok(serve(listener, routes).await?)
}
