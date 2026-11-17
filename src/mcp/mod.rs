//! MCP server, served over stdio.

use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerConfig},
    schemars::{self, JsonSchema},
    serde::Deserialize,
    tool, tool_handler, tool_router,
};

/// Streamable HTTP transport, mounted by the server.
pub(crate) mod http;

/// Arguments of the `echo` tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(crate = "rmcp::serde")]
pub(crate) struct EchoArgs {
    /// Text to send back.
    text: String,
}

/// MCP server: each `#[tool]` method below is a tool clients can call.
#[derive(Debug, Clone)]
pub(crate) struct Server;

#[tool_router]
impl Server {
    /// Sends `text` back unchanged.
    #[expect(
        clippy::unused_self,
        reason = "tools are methods; real ones read `self`"
    )]
    #[tool(description = "Echo the given text back.")]
    fn echo(&self, Parameters(args): Parameters<EchoArgs>) -> String {
        args.text
    }
}

#[allow(clippy::unused_async_trait_impl)]
#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(env!("CARGO_PKG_DESCRIPTION"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_sends_the_text_back() {
        let args = EchoArgs {
            text: String::from("hello"),
        };
        assert_eq!(Server.echo(Parameters(args)), "hello");
    }

    #[test]
    fn info_advertises_tools() {
        let info = Server.get_info();
        assert!(info.capabilities.tools.is_some());
        assert_eq!(info.server_info.name, env!("CARGO_PKG_NAME"));
    }
}
