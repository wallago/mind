use std::path::PathBuf;

use serde::Deserialize;

/// Server Config.
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ServerConfig {
    /// Address to listen.
    pub(crate) address: String,
    /// Port to listen.
    pub(crate) port: u16,
    /// File holding the bearer token for `/mcp`. Unset: no auth.
    pub(crate) token_file: Option<PathBuf>,
    /// Public HTTPS URL, e.g. `https://mind.example.com`. Set with
    /// `token_file`, it turns on OAuth so claude.ai can add `/mcp` as a
    /// custom connector.
    pub(crate) public_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserializes_toml() -> Result<(), toml::de::Error> {
        let config = toml::from_str::<ServerConfig>(
            r#"
            address = "0.0.0.0"
            port = 3000
            "#,
        )?;
        assert_eq!(config.address, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.token_file, None);
        assert_eq!(config.public_url, None);
        assert!(
            toml::from_str::<ServerConfig>("address = [\"0.0.0.0\"]\nport = [\"3000\"]").is_err()
        );
        Ok(())
    }
}
