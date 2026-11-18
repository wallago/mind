//! Error handler implementation.

use thiserror::Error as ThisError;

/// Errors that can occur while running [`crate::run`].
#[derive(Debug, ThisError)]
pub enum Error {
    /// Error that may occur during I/O operations.
    #[error("IO error: `{0}`")]
    Io(#[from] std::io::Error),
    /// Error that may occur while loading the application config file.
    #[error("Config file error: `{0}`")]
    Config(String),
    /// The vault folder can't be opened.
    #[error("Vault error: `{0}`")]
    Vault(String),
    /// A path from a client is not a Markdown note inside the vault.
    #[error(
        "`{0}` is not a note in the vault: give a relative path to a .md file, like `rust/cheatsheet.md`"
    )]
    BadNote(String),
    /// The note doesn't exist.
    #[error("no note at `{0}`: see `list_notes`")]
    NoNote(String),
    /// A note already exists where a new one should go.
    #[error("`{0}` already exists: read it, then use `update_note`")]
    NoteExists(String),
    /// The text to replace is not in the note.
    #[error("the text to replace is not in `{0}`: read the note again and copy it exactly")]
    TextNotFound(String),
    /// The text to replace is in the note more than once.
    #[error(
        "the text to replace is in `{note}` {count} times: add surrounding lines so it is unique"
    )]
    TextNotUnique {
        /// The note.
        note: String,
        /// How many times the text is there.
        count: usize,
    },
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn io_error_converts_and_displays() {
        let message = "your computer is on fire!";
        let error = Error::from(IoError::other(message));
        assert!(matches!(error, Error::Io(_)));
        assert_eq!(error.to_string(), format!("IO error: `{message}`"));
    }

    #[test]
    fn config_error_displays() {
        let error = Error::Config("config.toml: bad".to_string());
        assert_eq!(error.to_string(), "Config file error: `config.toml: bad`");
    }
}
