self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.mind;
  settingsFormat = pkgs.formats.toml { };
  # systemd copies the token here (LoadCredential), readable by the service only.
  credentials = "/run/credentials/mind.service";
  # mind's own defaults are empty: listen on localhost:3000 unless told otherwise.
  settings =
    lib.recursiveUpdate
      {
        server = {
          address = "127.0.0.1";
          port = 3000;
        };
      }
      (
        lib.recursiveUpdate cfg.settings {
          server = lib.optionalAttrs (cfg.tokenFile != null) { token_file = "${credentials}/token"; };
        }
      );
  configFile = settingsFormat.generate "mind.toml" settings;
  vFlag = n: lib.optionalString (n > 0) "-${lib.concatStrings (lib.replicate n "v")}";
in
{
  options.services.mind = {
    enable = lib.mkEnableOption "mind, the notes MCP server";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "Package with the mind binary.";
    };

    verbose = lib.mkOption {
      type = lib.types.ints.between 0 3;
      default = 1;
      description = "0 = warn, 1 = info, 2 = debug, 3 = trace.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "mind";
      description = "User the server runs as. Needs write access to the vault.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "mind";
    };

    settings = lib.mkOption {
      inherit (settingsFormat) type;
      default = { };
      example = {
        server.port = 3000;
        # Public HTTPS URL: turns on OAuth for claude.ai connectors (needs tokenFile).
        server.public_url = "https://mind.example.com";
        vault.root = "/srv/notes";
        # Host names MCP clients use for /mcp.
        mcp.allowed_hosts = [
          "localhost"
          "127.0.0.1"
          "::1"
          "mind.example.com"
        ];
      };
      description = ''
        Written to config.toml, with the same keys as `config.example.toml`.
        `vault.root` must be absolute: a relative path would resolve inside
        the Nix store. `server` defaults to 127.0.0.1:3000.
      '';
    };

    tokenFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "/run/agenix/mind-token";
      description = ''
        File holding the bearer token for /mcp, e.g. an agenix or sops-nix
        secret. It can stay root-only. A string, not a Nix path, so the token
        stays out of the Nix store. null: no auth.
      '';
    };

    firewallInterfaces = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "wg0" ];
      description = "Open the server port on these interfaces only.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = lib.hasPrefix "/" (cfg.settings.vault.root or "");
        message = "services.mind.settings.vault.root must be an absolute path.";
      }
      {
        assertion = (cfg.settings.server.public_url or null) == null || cfg.tokenFile != null;
        message = "services.mind.settings.server.public_url turns on OAuth, which needs services.mind.tokenFile.";
      }
    ];

    users.users = lib.mkIf (cfg.user == "mind") {
      mind = {
        isSystemUser = true;
        inherit (cfg) group;
      };
    };
    users.groups = lib.mkIf (cfg.group == "mind") { mind = { }; };

    systemd.services.mind = {
      description = "mind notes MCP server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} --config ${configFile} ${vFlag cfg.verbose}";
        User = cfg.user;
        Group = cfg.group;
        # Notes it writes stay group-writable for the vault's other users (Syncthing).
        UMask = "0007";
        ReadWritePaths = lib.optional (settings ? vault.root) settings.vault.root;
        LoadCredential = lib.optional (cfg.tokenFile != null) "token:${cfg.tokenFile}";
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;
        Restart = "on-failure";
      };
    };

    networking.firewall.interfaces = lib.genAttrs cfg.firewallInterfaces (_: {
      allowedTCPPorts = [ settings.server.port ];
    });
  };
}
