{
  description = "Espresso Decentralized Sequencer";

  nixConfig = {
    extra-substituters = [
      "https://espresso-systems-private.cachix.org"
      "https://nixpkgs-cross-overlay.cachix.org"
    ];
    extra-trusted-public-keys = [
      "espresso-systems-private.cachix.org-1:LHYk03zKQCeZ4dvg3NctyCq88e44oBZVug5LpYKjPRI="
      "nixpkgs-cross-overlay.cachix.org-1:TjKExGN4ys960TlsGqNOI/NBdoz2Jdr2ow1VybWV5JM="
    ];
  };

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.nixpkgs-cross-overlay.url = "github:alekseysidorov/nixpkgs-cross-overlay";
  inputs.nixpkgs-cross-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.solc-bin.url = "github:EspressoSystems/nix-solc-bin";
  inputs.solc-bin.inputs.nixpkgs.follows = "nixpkgs";

  inputs.dregs.url = "github:EspressoSystems/dregs";
  inputs.dregs.inputs.nixpkgs.follows = "nixpkgs";

  inputs.flake-compat.url = "github:edolstra/flake-compat";
  inputs.flake-compat.flake = false;

  # Pinned echidna version - current nixpkgs version fails to build
  # See https://hydra.nixos.org/job/nixos/trunk-combined/nixpkgs.echidna.x86_64-linux for build status
  inputs.echidna-nixpkgs.url = "github:NixOS/nixpkgs/08dacfca559e1d7da38f3cf05f1f45ee9bfd213c";

  outputs =
    { self
    , nixpkgs
    , rust-overlay
    , nixpkgs-cross-overlay
    , flake-utils
    , solc-bin
    , echidna-nixpkgs
    , dregs
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      # node=error: disable noisy anvil output
      RUST_LOG = "info,libp2p=off,isahc=error,surf=error,node=error";
      RUST_BACKTRACE = 1;
      rustEnvVars = { inherit RUST_LOG RUST_BACKTRACE; };

      rustShellHook = ''
        # `bin/pwd -P` for macOS case-insensitive filesystems
        my_pwd=$(/bin/pwd -P 2> /dev/null || pwd)

        # Distinct from non-nix `target/` so they don't fight
        export CARGO_TARGET_DIR="$my_pwd/target/nix"
        export PATH="$CARGO_TARGET_DIR/debug:$PATH"
      '';

      overlays = [
        (import rust-overlay)
        solc-bin.overlays.default
      ];
      pkgs = import nixpkgs { inherit system overlays; };
      inherit (pkgs) lib stdenv;

      solhint = pkgs.callPackage ./nix/solhint { };
      pup = pkgs.callPackage ./nix/pup { };
      golangci-lint = pkgs.golangci-lint.overrideAttrs (old: rec {
        version = "1.64.8";
        src = pkgs.fetchFromGitHub {
          owner = "golangci";
          repo = "golangci-lint";
          rev = "v${version}";
          sha256 = "sha256-ODnNBwtfILD0Uy2AKDR/e76ZrdyaOGlCktVUcf9ujy8";
        };
        vendorHash = "sha256-/iq7Ju7c2gS7gZn3n+y0kLtPn2Nn8HY/YdqSDYjtEkI=";
      });
      prek-as-pre-commit = pkgs.writeShellScriptBin "pre-commit" ''
        exec ${pkgs.prek}/bin/prek "$@"
      '';
      myShell = pkgs.mkShellNoCC.override (lib.optionalAttrs stdenv.isLinux {
        # mold linker, ~50% faster on Linux
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
      });
      crossShell = { config }:
        let
          localSystem = system;
          crossSystem = {
            inherit config;
            useLLVM = true;
            isStatic = true;
          };
          pkgs = import "${nixpkgs-cross-overlay}/utils/nixpkgs.nix" {
            inherit overlays localSystem crossSystem;
          };
        in
        import ./cross-shell.nix
          {
            inherit pkgs rustShellHook;
            envVars = rustEnvVars;
          };
    in
    {
      devShells.default =
        let
          stableToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          # Bump the date when a newer rust-analyzer/rustfmt is wanted.
          nightlyToolchain = pkgs.rust-bin.nightly."2026-04-16".minimal.override {
            extensions = [ "rust-analyzer" "rustfmt" ];
          };
        in
        myShell (rustEnvVars // {
          packages = with pkgs; [
            pkg-config
            openssl
            curl
            protobuf # libp2p-autonat
            stableToolchain
            jq
            cargo-audit
            cargo-edit
            cargo-hack
            cargo-nextest
            cargo-sort
            typos
            just
            nightlyToolchain.passthru.availableComponents.rust-analyzer
            nightlyToolchain.passthru.availableComponents.rustfmt
            nixpkgs-fmt
            prek
            prek-as-pre-commit # `pre-commit` alias for muscle memory
            process-compose
            entr
            pup # html parser for Datadog log queries
            lazydocker # used by `just demo-docker`
            bc # used by scripts/verify-pos-deployment.sh
            # foundry kept for `anvil`; forge/cast/chisel/solc are in `.#contracts`
            foundry
            nodePackages.prettier
            solhint
            libusb1 # link-time dep of `libusb1-sys` crate
          ] ++ lib.optionals stdenv.isDarwin [ pkgs.darwin.libresolv ];
          shellHook = ''
            ${rustShellHook}

            export PATH="$my_pwd/scripts:$PATH"

            # Avoid clash with rustup's ~/.cargo
            export CARGO_HOME=$HOME/.cargo-nix

            # Install pre-commit hook. Rewritten on every shell entry so updates propagate.
            if [ -d .git ] && [ -f .pre-commit-config.yaml ]; then
              mkdir -p .git/hooks
              cat > .git/hooks/pre-commit <<'HOOK'
            #!/usr/bin/env bash
            # espresso-network-precommit-hook — managed by flake.nix
            set -e
            if command -v prek >/dev/null 2>&1 && command -v rustfmt >/dev/null 2>&1; then
              exec prek run --hook-stage=pre-commit "$@"
            fi
            # Re-enter nix shell for commits made outside it (IDE etc.).
            # Force experimental features in case the user's nix.conf doesn't enable them.
            export NIX_CONFIG="''${NIX_CONFIG:-}"$'\nexperimental-features = nix-command flakes'
            exec nix develop --quiet --accept-flake-config -c prek run --hook-stage=pre-commit "$@"
            HOOK
              chmod +x .git/hooks/pre-commit
            fi
          '';
          RUST_SRC_PATH = "${stableToolchain}/lib/rustlib/src/rust/library";
        });
      devShells.docs = pkgs.mkShellNoCC {
        packages = with pkgs; [ graphviz plantuml mdbook ];
      };

      devShells.go = pkgs.mkShellNoCC {
        packages = [ pkgs.go golangci-lint ];
      };

      devShells.contracts =
        let
          solc = pkgs.solc-bin."0.8.28";
        in
        pkgs.mkShellNoCC {
          inputsFrom = [ self.devShells.${system}.default ];
          packages = [
            solc
            pkgs.go-ethereum
          ];
          FOUNDRY_SOLC = "${solc}/bin/solc";
        };

      devShells.mutation = pkgs.mkShellNoCC {
        inputsFrom = [ self.devShells.${system}.contracts ];
        packages = [ dregs.packages.${system}.unwrapped ];
      };

      devShells.python = pkgs.mkShellNoCC {
        packages = with pkgs; [ python3 ruff ty ];
      };

      devShells.dockerShell = pkgs.mkShell {
        inputsFrom = [ self.devShells.${system}.default ];
        packages = [ pkgs.docker ];
        shellHook = ''
          ${self.devShells.${system}.default.shellHook}

          # Required for demo-native to run with docker-rootless
          export DOCKER_HOST=unix://$XDG_RUNTIME_DIR/docker.sock
        '';
      };
      devShells.crossShell =
        crossShell { config = "x86_64-unknown-linux-musl"; };
      devShells.armCrossShell =
        crossShell { config = "aarch64-unknown-linux-musl"; };
      devShells.nightly =
        let
          toolchain = pkgs.rust-bin.nightly.latest.minimal.override {
            extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
          };
        in
        myShell (rustEnvVars // {
          packages = with pkgs; [
            pkg-config
            openssl
            curl
            protobuf # libp2p-autonat
            toolchain
          ];
          shellHook = rustShellHook;
        });
      devShells.coverage =
        let
          toolchain = pkgs.rust-bin.nightly.latest.minimal;
        in
        myShell (rustEnvVars // {
          packages = with pkgs; [
            pkg-config
            openssl
            curl
            protobuf # libp2p-autonat
            toolchain
            grcov
          ];
          CARGO_INCREMENTAL = "0";
          shellHook = ''
            ${rustShellHook}
            RUSTFLAGS="$RUSTFLAGS -Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort -Zpanic_abort_tests -Cdebuginfo=2"
          '';
          RUSTDOCFLAGS = "-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort -Zpanic_abort_tests";
        });

      devShells.rustShell =
        let
          stableToolchain = pkgs.rust-bin.stable.latest.minimal.override {
            extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
          };
        in
        myShell (rustEnvVars // {
          packages = with pkgs; [
            pkg-config
            openssl
            curl
            protobuf # libp2p-autonat
            stableToolchain
          ];
          shellHook = rustShellHook;
        });

      # Separated due to ghc dep size.
      devShells.echidna =
        let
          solc = pkgs.solc-bin."0.8.28";
          echidna-pkgs = import echidna-nixpkgs { inherit system; };
        in
        myShell {
          packages = [
            pkgs.foundry
            solc
            echidna-pkgs.slither-analyzer
            echidna-pkgs.echidna
            echidna-pkgs.python3.pkgs.crytic-compile
          ];
          FOUNDRY_SOLC = "${solc}/bin/solc";
        };
    });
}
