{
  description = "Velib MCP server - Nix build with proper layering";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type:
            craneLib.filterCargoSources path type;
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = with pkgs; [
            openssl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "velib-mcp-deps";
        });

        velib-mcp = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "velib-mcp";

          CARGO_PROFILE_RELEASE_LTO = "true";
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
          CARGO_PROFILE_RELEASE_PANIC = "abort";
          CARGO_PROFILE_RELEASE_STRIP = "true";
          CARGO_PROFILE_RELEASE_OPT_LEVEL = "s";
        });

        container = pkgs.dockerTools.buildLayeredImage {
          name = "velib-mcp";
          tag = "latest";

          contents = with pkgs; [
            cacert
          ];

          config = {
            Cmd = [ "${velib-mcp}/bin/velib-mcp" ];
            Env = [
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              "PORT=8080"
              "PATH=${velib-mcp}/bin"
            ];
            ExposedPorts = {
              "8080/tcp" = {};
            };
          };
        };

      in
      {
        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            rustToolchain
            rust-analyzer
            pkg-config
            openssl
            podman
            skopeo
            jq
            nixpkgs-fmt
            nil
          ];
        };

        packages = {
          inherit velib-mcp container;
          default = container;
        };

        checks = {
          velib-mcp-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          velib-mcp-fmt = craneLib.cargoFmt {
            inherit (commonArgs) src;
          };

          velib-mcp-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "--all-features --workspace";
          });
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
