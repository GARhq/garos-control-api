{
  description = "garos-backend — production HTTP API for kryonix-os-control-center";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rustfmt" "clippy" "rust-src" ];
        };
        buildDeps = with pkgs; [
          pkg-config
          openssl
          sqlite
        ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "garos-backend";
          version = "0.1.0";
          src = ./.;
          cargoLock = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = buildDeps;
          doCheck = true;
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/garos-backend";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            sqlite
            sqlx-cli
            cargo-watch
            mold
          ];
          shellHook = ''
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
            export SQLX_OFFLINE=true
            echo "garos-backend dev shell ready"
          '';
        };

        devShells.ci = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-audit
            cargo-deny
            cargo-tarpaulin
          ];
        };
      }) // {
      nixosModules.default = { config, lib, pkgs, ... }:
        with lib; {
          options.services.garos-backend = {
            enable = mkEnableOption "garos-backend HTTP API server";
            package = mkOption {
              type = types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
            };
            openFirewall = mkOption {
              type = types.bool;
              default = false;
            };
            settings = mkOption {
              type = types.attrs;
              default = {};
              description = "Override any GAROS setting (TOML-shaped).";
            };
            user = mkOption {
              type = types.str;
              default = "garos";
            };
            group = mkOption {
              type = types.str;
              default = "garos";
            };
            dataDir = mkOption {
              type = types.path;
              default = "/var/lib/garos";
            };
            configFile = mkOption {
              type = types.nullOr types.path;
              default = null;
            };
          };

          config = mkIf config.services.garos-backend.enable {
            users.users = mkIf (config.services.garos-backend.user == "garos") {
              garos = {
                isSystemUser = true;
                home = config.services.garos-backend.dataDir;
                group = config.services.garos-backend.group;
                description = "garos-backend service account";
              };
            };
            users.groups = mkIf (config.services.garos-backend.group == "garos") {
              garos = { };
            };

            environment.etc."garos/config.toml" =
              mkIf (config.services.garos-backend.configFile != null)
                { source = config.services.garos-backend.configFile; };

            systemd.tmpfiles.rules = [
              "d ${config.services.garos-backend.dataDir} 0750 ${config.services.garos-backend.user} ${config.services.garos-backend.group} -"
            ];

            systemd.services.garos-backend = {
              description = "garos-backend HTTP API";
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              serviceConfig = {
                Type = "simple";
                User = config.services.garos-backend.user;
                Group = config.services.garos-backend.group;
                ExecStart = "${config.services.garos-backend.package}/bin/garos-backend serve --env production";
                Restart = "on-failure";
                RestartSec = "5s";
                WorkingDirectory = config.services.garos-backend.dataDir;
                StateDirectory = baseNameOf config.services.garos-backend.dataDir;
                ConfigurationDirectory = "garos";
                LogsDirectory = "garos";
                LimitNOFILE = 65536;
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                PrivateTmp = true;
                ReadWritePaths = [ config.services.garos-backend.dataDir ];
                Environment = "RUST_LOG=info,garos_backend=info";
              };
            };

            networking.firewall.allowedTCPPorts = mkIf config.services.garos-backend.openFirewall [ 8080 ];
          };
        };
    };
}
