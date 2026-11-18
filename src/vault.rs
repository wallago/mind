//! The notes folder: plain file access, no MCP.
//!
//! Nothing is cached: every call reads the disk, so a change Syncthing made a
//! second ago is always seen. Every path from a client goes through
//! [`Vault::resolve`], which keeps it inside the vault.

use std::{
    fs,
    io::{self, Write as _},
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

    /// Creates the note at `note` with `content`, and its folders if needed.
    ///
    /// # Errors
    ///
    /// [`Error::BadNote`] if `note` is not a note path in the vault,
    /// [`Error::NoteExists`] if something is already there, [`Error::Io`] if
    /// it can't be written.
    pub(crate) fn create(&self, note: &str, content: &str) -> Result<()> {
        let path = self.resolve(note)?;
        if path.exists() {
            return Err(Error::NoteExists(note.to_owned()));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // `create_new`: never overwrite a note that appeared in the meantime.
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(Error::NoteExists(note.to_owned()));
            }
            Err(error) => return Err(error.into()),
        };
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Replaces `old`, which must be in the note exactly once, with `new`.
    ///
    /// # Errors
    ///
    /// [`Error::BadNote`] or [`Error::NoNote`] as for [`Vault::read`],
    /// [`Error::TextNotFound`] or [`Error::TextNotUnique`] unless `old` is in
    /// the note exactly once, [`Error::Io`] if it can't be written.
    pub(crate) fn edit(&self, note: &str, old: &str, new: &str) -> Result<()> {
        let content = self.read(note)?;
        let count = if old.is_empty() {
            0
        } else {
            content.matches(old).count()
        };
        match count {
            0 => Err(Error::TextNotFound(note.to_owned())),
            1 => Ok(fs::write(
                self.resolve(note)?,
                content.replacen(old, new, 1),
            )?),
            count => Err(Error::TextNotUnique {
                note: note.to_owned(),
                count,
            }),
        }
    }

    /// Deletes the note at `note`. Its folder stays, even if empty.
    ///
    /// # Errors
    ///
    /// [`Error::BadNote`] or [`Error::NoNote`] as for [`Vault::read`],
    /// [`Error::Io`] if it can't be removed.
    pub(crate) fn delete(&self, note: &str) -> Result<()> {
        let path = self.resolve(note)?;
        if !path.is_file() {
            return Err(Error::NoNote(note.to_owned()));
        }
        Ok(fs::remove_file(path)?)
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

    #[test]
    fn create_writes_a_new_note_and_its_folders() -> Result<()> {
        let vault = vault("create", &[("advice.md", "old")])?;
        vault.create("rust/async/tokio.md", "# Tokio\n")?;
        assert_eq!(vault.read("rust/async/tokio.md")?, "# Tokio\n");
        let result = vault.create("advice.md", "new");
        assert!(matches!(&result, Err(Error::NoteExists(_))), "{result:?}");
        assert_eq!(vault.read("advice.md")?, "old");
        let result = vault.create("../escape.md", "");
        assert!(matches!(&result, Err(Error::BadNote(_))), "{result:?}");
        Ok(())
    }

    #[test]
    fn edit_replaces_text_found_once() -> Result<()> {
        let vault = vault("edit", &[("note.md", "# Title\n- one\n- two\n- two\n")])?;
        vault.edit("note.md", "- one", "- first")?;
        assert_eq!(vault.read("note.md")?, "# Title\n- first\n- two\n- two\n");

        let result = vault.edit("note.md", "- two", "- second");
        assert!(
            matches!(&result, Err(Error::TextNotUnique { count: 2, .. })),
            "{result:?}"
        );
        for old in ["- three", ""] {
            let result = vault.edit("note.md", old, "x");
            assert!(matches!(&result, Err(Error::TextNotFound(_))), "{result:?}");
        }
        let result = vault.edit("missing.md", "a", "b");
        assert!(matches!(&result, Err(Error::NoNote(_))), "{result:?}");
        // Nothing changed after the failures.
        assert_eq!(vault.read("note.md")?, "# Title\n- first\n- two\n- two\n");
        Ok(())
    }

    #[test]
    fn delete_removes_only_notes() -> Result<()> {
        let vault = vault("delete", &[("rust/a.md", ""), ("rust/b.md", "")])?;
        vault.delete("rust/a.md")?;
        assert_eq!(vault.list()?, ["rust/b.md"]);
        let result = vault.delete("rust/a.md");
        assert!(matches!(&result, Err(Error::NoNote(_))), "{result:?}");
        let result = vault.delete("rust");
        assert!(matches!(&result, Err(Error::BadNote(_))), "{result:?}");
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
        let result = vault.create("link/new.md", "");
        assert!(matches!(&result, Err(Error::BadNote(_))), "{result:?}");
        let result = vault.delete("link/secret.md");
        assert!(matches!(&result, Err(Error::BadNote(_))), "{result:?}");
        assert_eq!(outside.read("secret.md")?, "secret");
        Ok(())
    }
}
