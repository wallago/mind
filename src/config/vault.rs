use std::path::PathBuf;

use serde::Deserialize;

/// Vault config: where the notes live.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct VaultConfig {
    /// Notes folder, e.g. the folder Syncthing shares. Must exist.
    pub(crate) root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserializes_toml() -> Result<(), toml::de::Error> {
        let config = toml::from_str::<VaultConfig>(r#"root = "/home/me/notes""#)?;
        assert_eq!(config.root, PathBuf::from("/home/me/notes"));
        assert!(toml::from_str::<VaultConfig>("root = [\"/home/me/notes\"]").is_err());
        Ok(())
    }

    #[test]
    fn test_defaults_to_empty() -> Result<(), toml::de::Error> {
        let config = toml::from_str::<VaultConfig>("")?;
        assert_eq!(config.root, PathBuf::new());
        Ok(())
    }
}
