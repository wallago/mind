# mind

MCP server exposing a Syncthing-shared notes folder: cheatsheets, things worth knowing, advice, and interesting finds.

## Development environment

Every tool comes from the Nix flake. Enter the dev shell with:

```sh
nix develop
```

`.envrc` is `use flake`, so with direnv, `direnv allow` once and the shell
loads on `cd`.

## Tasks

`just` is the entry point:

```sh
just          # list every recipe
just ci       # every check CI runs; run it before pushing
```

## Usage

```sh
cargo run -- --help
```

Settings are read from a TOML file; `config.example.toml` lists them.

### Bearer token

Set `[server] token_file` to a file holding a token (agenix, sops...).
Then `/mcp` answers 401 unless the request sends `Authorization: Bearer <token>`.
Without `token_file` there is no auth: keep the server on localhost.

```sh
claude mcp add --transport http mind https://mind.example.com/mcp \
  --header "Authorization: Bearer $(cat /path/to/token)"
```

### claude.ai connector (OAuth)

claude.ai custom connectors log in with OAuth, not a fixed header.
Set `[server] public_url` next to `token_file` to turn on a minimal built-in OAuth server:

```toml
[server]
token_file = "/run/secrets/mind-token"
public_url = "https://mind.example.com"

[mcp]
allowed_hosts = ["localhost", "127.0.0.1", "::1", "mind.example.com"]
```

Then in claude.ai: Settings > Connectors > Add custom connector, with the URL `https://mind.example.com/mcp`.
claude.ai opens a login page on the server: paste the server token once and click Allow.

- `public_url` must be the public HTTPS URL claude.ai reaches, and its host must be in `[mcp] allowed_hosts`.
- Nothing is stored: codes and tokens are signed with a key derived from the server token.
  A restart keeps everyone logged in; changing the token logs every OAuth client out.
- Access tokens last 1 hour, refresh tokens 90 days.
- The static `Authorization: Bearer <token>` keeps working for Claude Code and scripts.
- Only Claude's redirect URIs are accepted: the claude.ai callback and loopback URLs.

## NixOS

The flake builds the server (`nix build`) and ships a NixOS module:

```nix
{
  inputs.mind.url = "github:wallago/mind";

  outputs = { nixpkgs, mind, ... }: {
    nixosConfigurations.host = nixpkgs.lib.nixosSystem {
      modules = [
        mind.nixosModules.default
        {
          services.mind = {
            enable = true;
            tokenFile = "/run/agenix/mind-token";
            settings = {
              vault.root = "/srv/notes";
              server.public_url = "https://mind.example.com";
              mcp.allowed_hosts = [ "localhost" "127.0.0.1" "::1" "mind.example.com" ];
            };
          };
        }
      ];
    };
  };
}
```

`settings` is written to the config file as is, with the keys of `config.example.toml`;
`server` defaults to `127.0.0.1:3000`. The service runs as the `mind` user, which needs
write access to `vault.root` (add it to the Syncthing group). `tokenFile` stays out of the
Nix store: systemd hands it to the service as a credential. `firewallInterfaces` opens the
port on the interfaces listed, such as `wg0`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

