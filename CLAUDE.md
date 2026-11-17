# mind

MCP server exposing a Syncthing-shared notes folder: cheatsheets, things worth knowing, advice, and interesting finds.

## Layout

- `flake.nix`: dev shell (`nix develop`) and the lean CI shell (`nix develop .#ci`). Tools the checks run go in `ciTools`, editor-only ones in `devTools`.
- `src/main.rs`: parses `Args`, sets up tracing, calls `run`. Keep it thin.
- `src/lib.rs`: `run` builds every component from `Config`, then waits on all
  of them in one `tokio::select!` until Ctrl-C or the first one stops. A new
  component is a module, a field in `Config`, and an arm in that `select!`.
- `src/config/`: `Config`, read from TOML; one table per component, each
  listed in `config.example.toml` (a test loads it).
- `src/error.rs`: the crate's one `Error` enum; add a variant per new failure
  source, with `#[from]` where it wraps a library error.
- `tests/cli.rs`: runs the built binary the way a user would.
- `src/server/`: axum server and its `/health` probe; routes are added in `routes()`.
- `src/mcp/`: MCP server on rmcp; a tool is a `#[tool]` method on `Server`. It must not glob-import the prelude: rmcp's macros expand to a bare two-argument `Result`.
- `src/mcp/http.rs`: serves MCP over Streamable HTTP at `/mcp`, mounted by the server; `allowed_hosts` in the config guards the `Host` header.
- `.github/workflows/ci.yml`: one job per `just ci` recipe, each run in the flake's CI shell, plus the Nix flake check and the commit-format check.

<!-- TODO: what this project is for, and where a typical change lands. Claude
     reads this file before every task, so keep it true. -->

## Rules

Standalone rules live in `.claude/rules/` and are imported here. Only files
reachable from `CLAUDE.md` get loaded, so a new rule needs a line below.

@.claude/rules/nix.md

## Commands

`just` is the entry point, not the raw toolchain; `just --list` shows every
recipe.

- `just ci`: the gate. It runs every check CI runs, recipe by recipe, so a
  green `just ci` is a green CI. Run it before calling a change done.
- `just check`, `just test`, `just lint`, `just fmt`: the fast loop.

## Constraints

- Clippy runs with `-D warnings` and `Cargo.toml` warns on `unwrap_used`,
  `expect_used`, `panic`, `todo`, `print_stdout` and
  `missing_docs_in_private_items`: no `.unwrap()`, no `.expect()`, no bare
  `panic!`, and every item gets a doc comment.
- `cargo machete` fails on a dependency nothing uses; drop it in the change that orphans it.
- A new crate has to clear `cargo deny check`: licences, bans, sources and advisories.
- `cargo audit` fails on a dependency with a RustSec advisory.
- `editorconfig-checker` holds every file to `.editorconfig`: indentation, final newline, no trailing spaces.
- `typos` runs over the repo. Add a real term to `typos.toml` rather than rewording around it.
- Every link in Markdown is checked. A placeholder URL fails the gate.
- Commit messages are Conventional Commits (`type(scope): subject`); CI checks every commit of a PR with `committed`, and `git-cliff` builds the changelog from them.

## Verifying a change

`just ci` green is the bar. Say which recipes ran when reporting a change.

<!-- TODO: what the gate cannot catch here (hardware, a UI, a remote service),
     and how it gets checked instead. -->
