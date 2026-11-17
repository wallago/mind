//! Streamable HTTP transport, mounted by the server at `/mcp`.

use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use super::Server;
use crate::{config::mcp::McpConfig, vault::Vault};

/// MCP over Streamable HTTP, ready for `Router::nest_service`.
///
/// Requests whose `Host` header is not in `[mcp] allowed_hosts` are refused,
/// against DNS rebinding. Behind a public URL (e.g. a claude.ai custom
/// connector), add that hostname to the list.
pub(crate) fn service(
    config: &McpConfig,
    vault: Vault,
) -> StreamableHttpService<Server, LocalSessionManager> {
    let http = StreamableHttpServerConfig::default().with_allowed_hosts(&config.allowed_hosts);
    StreamableHttpService::new(move || Ok(Server::new(vault.clone())), Arc::default(), http)
}
