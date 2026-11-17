{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { self, ... }@inputs:
    {
      nixosModules.default = import ./nix/nixos-module.nix self;
    }
    # ── Then merge the per-system outputs onto it ──
    //
      inputs.flake-utils.lib.eachSystem
        [
          "x86_64-linux"
          "aarch64-linux"
        ]
        (
          system:
          let
            overlays = [
              (import inputs.rust-overlay)
            ];
            pkgs = import inputs.nixpkgs {
              inherit system overlays;
              config.allowUnfree = true;
            };

            # ── Toolchain ─────────────────────────────────────────────
            rust = pkgs.rust-bin.nightly.latest.default.override {
              extensions = [
                "llvm-tools-preview"
                "rust-src"
              ];
            };

            naersk' = pkgs.callPackage inputs.naersk {
              cargo = rust;
              rustc = rust;
            };

            # ── Build helper ──────────────────────────────────────────
            buildApp =
              release:
              pkgs.callPackage ./nix/package.nix {
                inherit naersk' release;
                src = ./.;
              };

            # ── Tooling shared ────────────────────────────────────────
            ciTools = with pkgs; [
              just
              statix
              deadnix
              rust
              taplo
              cargo-machete
              cargo-deny
              cargo-audit
              cargo-llvm-cov
              editorconfig-checker
              typos
              lychee
              committed
              actionlint
            ];
            devTools = with pkgs; [
              rust-analyzer
              nodejs
              git-cliff
            ];
          in
          {
            # ── Packages (nix build) ─────────────────────────────────
            packages = {
              default = buildApp true;
              debug = buildApp false;
            };

            # ── Checks (nix flake check) ─────────────────────────────
            checks.default = self.packages.${system}.debug;

            # ── Dev Shell (nix develop) ──────────────────────────────
            devShells.default =
              let
                banner = pkgs.writeShellApplication {
                  name = "project-banner";
                  runtimeInputs = with pkgs; [
                    gum
                    jq
                    git
                    coreutils
                    findutils
                  ];
                  text = ''
                    export CLICOLOR_FORCE=1
                    cd "$(git rev-parse --show-toplevel)"

                    created=$(git log --reverse --format=%as | sed -n 1p)
                    updated=$(git log -1 --format=%cr)
                    commits=$(git rev-list --count HEAD)
                    nixpkgs=$(date -d @"$(jq -r .nodes.nixpkgs.locked.lastModified flake.lock)" +%F)

                    title=$(gum style --foreground 111 --bold 'mind')
                    desc=$(gum style --foreground 244 --italic "MCP server exposing a Syncthing-shared notes folder: cheatsheets, things worth knowing, advice, and interesting finds.")
                    keys=$(gum style --foreground 80 --align right --padding "0 2 0 0" \
                      created updated commits nixpkgs)
                    vals=$(gum style --foreground 255 \
                      "$created" "$updated" "$commits" "$nixpkgs")

                    header=$(gum join --vertical --align center "$title" "" "$desc")

                    body=$(gum join --vertical --align center \
                      "$header" "" "$(gum join --horizontal "$keys" "$vals")")

                    gum style --border double --border-foreground 111 \
                      --margin "1 2" --padding "1 4" "$body"
                  '';
                };
              in
              pkgs.mkShell {
                PROJECT_BANNER = pkgs.lib.getExe banner;
                buildInputs = ciTools ++ devTools;
              };

            # ── CI Shell (nix develop .#ci) ──────────────────────────
            # Lean: just the toolchain + checks, no editor/claude/shellHook.
            devShells.ci = pkgs.mkShell {
              buildInputs = ciTools;
            };
          }
        );
}
