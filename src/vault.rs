//! The notes folder: plain file access, no MCP.
//!
//! Nothing is cached: every call reads the disk, so a change Syncthing made a
//! second ago is always seen. Every path from a client goes through
//! [`Vault::resolve`], which keeps it inside the vault.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::prelude::*;

/// A folder of Markdown notes.
#[derive(Clone, Debug)]
pub(crate) struct Vault {
    /// Canonical vault folder.
    root: PathBuf,
}

impl Vault {
    /// Opens the vault at `root`.
    ///
    /// # Errors
    ///
    /// [`Error::Vault`] if `root` is not set or is not an existing folder.
    pub(crate) fn open(root: &Path) -> Result<Self> {
        if root.as_os_str().is_empty() {
            return Err(Error::Vault(String::from(
                "no notes folder: set `root` in the `[vault]` table of the config",
            )));
        }
        let root = root
            .canonicalize()
            .map_err(|error| Error::Vault(format!("{}: {error}", root.display())))?;
        if !root.is_dir() {
            return Err(Error::Vault(format!("{}: not a folder", root.display())));
        }
        Ok(Self { root })
    }

    /// Every note, as sorted paths relative to the vault like `rust/cheatsheet.md`.
    /// Hidden files and folders, like `.obsidian` or `.stfolder`, are skipped.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if a folder can't be read.
    pub(crate) fn list(&self) -> Result<Vec<String>> {
        let mut notes = Vec::new();
        let mut dirs = vec![self.root.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    dirs.push(path);
                } else if kind.is_file()
                    && is_markdown(&path)
                    && let Ok(relative) = path.strip_prefix(&self.root)
                {
                    notes.push(slashed(relative));
                }
            }
        }
        notes.sort();
        Ok(notes)
    }

    /// The content of the note at `note`.
    ///
    /// # Errors
    ///
    /// [`Error::BadNote`] if `note` is not a note path in the vault,
    /// [`Error::NoNote`] if it doesn't exist, [`Error::Io`] if it can't be read.
    pub(crate) fn read(&self, note: &str) -> Result<String> {
        let path = self.resolve(note)?;
        if !path.is_file() {
            return Err(Error::NoNote(note.to_owned()));
        }
        Ok(fs::read_to_string(path)?)
    }

    /// Turns a client's relative note path into a path inside the vault.
    ///
    /// # Errors
    ///
    /// [`Error::BadNote`] if `note` is absolute, has `..`, isn't a `.md`
    /// file, or leads out of the vault through a symlink.
    fn resolve(&self, note: &str) -> Result<PathBuf> {
        let bad = || Error::BadNote(note.to_owned());
        let relative = Path::new(note);
        let only_names = relative
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
        if note.is_empty() || !only_names || !is_markdown(relative) {
            return Err(bad());
        }
        let path = self.root.join(relative);
        // A symlink inside the vault could still point out of it: check the
        // deepest part that exists, once resolved.
        let existing = path
            .ancestors()
            .find(|ancestor| ancestor.exists())
            .ok_or_else(bad)?;
        let real = existing.canonicalize()?;
        if !real.starts_with(&self.root) {
            return Err(bad());
        }
        Ok(path)
    }
}

/// `true` if `path` ends in `.md`.
fn is_markdown(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// `path` with `/` between its parts, on every OS.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// A fresh vault folder unique to `test`, with `files` written in it.
    fn vault(test: &str, files: &[(&str, &str)]) -> Result<Vault> {
        let root = std::env::temp_dir().join(format!(
            "{}-{}-vault-{test}",
            env!("CARGO_PKG_NAME"),
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        for (path, content) in files {
            let path = root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
        }
        fs::create_dir_all(&root)?;
        Vault::open(&root)
    }

    #[test]
    fn open_needs_a_root() {
        let result = Vault::open(Path::new(""));
        assert!(
            matches!(&result, Err(Error::Vault(msg)) if msg.contains("[vault]")),
            "{result:?}"
        );
    }

    #[test]
    fn open_needs_an_existing_folder() {
        let result = Vault::open(Path::new("/nonexistent/notes"));
        assert!(matches!(&result, Err(Error::Vault(_))), "{result:?}");
    }

    #[test]
    fn list_finds_notes_sorted_and_skips_the_rest() -> Result<()> {
        let vault = vault(
            "list",
            &[
                ("rust/cheatsheet.md", ""),
                ("advice.md", ""),
                ("image.png", ""),
                (".obsidian/app.md", ""),
                (".hidden.md", ""),
            ],
        )?;
        assert_eq!(vault.list()?, ["advice.md", "rust/cheatsheet.md"]);
        Ok(())
    }

    #[test]
    fn read_gives_the_content() -> Result<()> {
        let vault = vault("read", &[("rust/cheatsheet.md", "# Rust\n")])?;
        assert_eq!(vault.read("rust/cheatsheet.md")?, "# Rust\n");
        assert_eq!(vault.read("./rust/cheatsheet.md")?, "# Rust\n");
        Ok(())
    }

    #[test]
    fn read_a_missing_note_is_an_error() -> Result<()> {
        let vault = vault("missing", &[])?;
        let result = vault.read("nope.md");
        assert!(matches!(&result, Err(Error::NoNote(_))), "{result:?}");
        Ok(())
    }

    #[test]
    fn paths_out_of_the_vault_are_refused() -> Result<()> {
        let vault = vault("escape", &[("note.md", "")])?;
        for note in [
            "../note.md",
            "/etc/passwd.md",
            "a/../../note.md",
            "",
            "note.txt",
        ] {
            let result = vault.read(note);
            assert!(
                matches!(&result, Err(Error::BadNote(_))),
                "{note}: {result:?}"
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_out_of_the_vault_are_refused() -> Result<()> {
        let outside = vault("outside", &[("secret.md", "secret")])?;
        let vault = vault("symlink", &[])?;
        std::os::unix::fs::symlink(&outside.root, vault.root.join("link"))?;
        let result = vault.read("link/secret.md");
        assert!(matches!(&result, Err(Error::BadNote(_))), "{result:?}");
        Ok(())
    }
}
