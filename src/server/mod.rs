//! Server core.

use axum::{Router, http::StatusCode, middleware, routing::get, serve};

use crate::{
    config::{Config, mcp::McpConfig},
    prelude::*,
    vault::Vault,
};

/// Bearer token.
mod auth;
/// OAuth login for claude.ai connectors.
mod oauth;

/// Liveness probe: answers as long as the server is up.
async fn health() -> StatusCode {
    StatusCode::OK
}

/// Global server routes. With `tokens`, `/mcp` needs the server token or an
/// OAuth access token, and with OAuth on its login routes are served too.
fn routes(mcp_config: &McpConfig, vault: Vault, tokens: Option<auth::Tokens>) -> Router {
    let mut protected =
        Router::new().nest_service("/mcp", crate::mcp::http::service(mcp_config, vault));
    let mut router = Router::new().route("/health", get(health));
    if let Some(tokens) = tokens {
        if let Some(issuer) = tokens.oauth.clone() {
            router = router.merge(oauth::routes(issuer));
        }
        protected = protected.layer(middleware::from_fn_with_state(tokens, auth::require));
    }
    router.merge(protected)
}

/// The tokens `config` asks for: none without a `token_file`, and OAuth on
/// when `public_url` is set too.
///
/// # Errors
///
/// [`Error::Config`] if the token file can't be read, or `public_url` is set
/// without a `token_file`.
fn tokens(config: &Config) -> Result<Option<auth::Tokens>> {
    let server = &config.server;
    match (server.token_file.as_deref(), server.public_url.as_deref()) {
        (Some(path), public_url) => {
            let main = auth::Token::read(path)?;
            let oauth = public_url.map(|url| oauth::Issuer::new(url, main.clone()));
            Ok(Some(auth::Tokens { main, oauth }))
        }
        (None, Some(_)) => Err(Error::Config(
            "`public_url` turns on OAuth, which needs a `token_file`".to_owned(),
        )),
        (None, None) => {
            tracing::warn!("no `token_file`: /mcp is open to anyone who can reach it");
            Ok(None)
        }
    }
}

/// Start web services.
pub(super) async fn start(config: &Config, vault: Vault) -> Result<()> {
    let tokens = tokens(config)?;
    let (address, port) = (&config.server.address, config.server.port);
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
    tracing::info!("Server running on {}", listener.local_addr()?);
    Ok(serve(listener, routes(&config.mcp, vault, tokens)).await?)
}

/// A router over an empty temp vault unique to `test`, with these tokens.
#[cfg(test)]
fn test_router(test: &str, tokens: Option<auth::Tokens>) -> Router {
    let root = std::env::temp_dir().join(format!(
        "{}-{}-server-{test}",
        env!("CARGO_PKG_NAME"),
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temp dir");
    let vault = Vault::open(&root).expect("open vault");
    routes(&McpConfig::default(), vault, tokens)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_is_ok() {
        let request = Request::get("/health")
            .body(Body::empty())
            .expect("valid request");
        let response = test_router("health", None)
            .oneshot(request)
            .await
            .expect("infallible");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn oauth_needs_a_token_file() {
        let mut config = Config::default();
        assert!(matches!(tokens(&config), Ok(None)));
        config.server.public_url = Some("https://mind.example.com".to_owned());
        assert!(matches!(tokens(&config), Err(Error::Config(_))));
        config.server.token_file = Some(PathBuf::from("/nonexistent/token"));
        assert!(matches!(tokens(&config), Err(Error::Config(_))));
    }
}
