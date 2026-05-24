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
        # on mac os `bin/pwd -P` returns the canonical path on case insensitive file-systems
        my_pwd=$(/bin/pwd -P 2> /dev/null || pwd)

        # Use a distinct target dir for builds from within nix shells.
        export CARGO_TARGET_DIR="$my_pwd/target/nix"

        # Add rust binaries to PATH
        export PATH="$CARGO_TARGET_DIR/debug:$PATH"
      '';

      overlays = [
        (import rust-overlay)
        solc-bin.overlays.default
      ];
      pkgs = import nixpkgs { inherit system overlays; };
      inherit (pkgs) lib stdenv;

      # Local custom packages — kept out of `overlays` so they don't add
      # an extra layer on top of every `pkgs.*` access. Referenced directly
      # from the shells that need them.
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
        # The mold linker is around 50% faster on Linux than the default linker.
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
          # Pinned (was `selectLatestNightlyWith`, which iterates the entire
          # rust-overlay nightly attrset). Bump as needed when a newer
          # rust-analyzer/rustfmt is wanted.
          nightlyToolchain = pkgs.rust-bin.nightly."2026-04-16".minimal.override {
            extensions = [ "rust-analyzer" "rustfmt" ];
          };
        in
        myShell (rustEnvVars // {
          packages = with pkgs; [
            # Rust dependencies
            pkg-config
            openssl
            curl
            protobuf # to compile libp2p-autonat
            stableToolchain
            jq

            # Rust tools
            cargo-audit
            cargo-edit
            cargo-hack
            cargo-nextest
            cargo-sort
            typos
            just
            nightlyToolchain.passthru.availableComponents.rust-analyzer
            nightlyToolchain.passthru.availableComponents.rustfmt

            # Tools
            nixpkgs-fmt
            prek
            prek-as-pre-commit # compat to allow running pre-commit
            process-compose
            entr # file-watcher
            pup # html parser, used for Datadog log queries — see nix/pup/README.md
            lazydocker # docker compose TUI, used by `just demo-docker`
            bc # used by scripts/verify-pos-deployment.sh

            # Ethereum contracts, solidity, ...
            # foundry is here because `anvil` (bundled inside it) is used
            # as an EVM test node by Rust tests. Other foundry tools
            # (forge / cast / chisel) and `solc` live in `.#contracts`.
            foundry
            nodePackages.prettier
            solhint
            libusb1 # link-time dep of `libusb1-sys` Rust crate
          ] ++ lib.optionals stdenv.isDarwin [ pkgs.darwin.libresolv ];
          shellHook = ''
            ${rustShellHook}

            # Add the local scripts to the PATH
            export PATH="$my_pwd/scripts:$PATH"

            # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts
            # with rustup installations.
            export CARGO_HOME=$HOME/.cargo-nix

            # Install a nix-aware pre-commit hook that runs prek with
            # the committed .pre-commit-config.yaml. If the user is
            # already inside this dev shell (or any shell with the hook
            # tools on PATH), the hook calls prek directly. If they're
            # outside (e.g. committing from an IDE), it re-enters
            # `nix develop` first so rustfmt/forge/solhint/etc. resolve.
            # Rewritten unconditionally on every shell entry so updates
            # propagate. If you want a customised hook, put it under a
            # different name (git only runs `.git/hooks/pre-commit`).
            if [ -d .git ] && [ -f .pre-commit-config.yaml ]; then
              mkdir -p .git/hooks
              cat > .git/hooks/pre-commit <<'HOOK'
            #!/usr/bin/env bash
            # espresso-network-precommit-hook — managed by flake.nix.
            # Runs prek against the committed .pre-commit-config.yaml,
            # entering the nix dev shell first if necessary so the hook
            # tools resolve.
            set -e
            if command -v prek >/dev/null 2>&1 && command -v rustfmt >/dev/null 2>&1; then
              exec prek run --hook-stage=pre-commit "$@"
            fi
            # Force experimental features on for users whose nix.conf
            # doesn't enable nix-command/flakes by default.
            export NIX_CONFIG="''${NIX_CONFIG:-}"$'\nexperimental-features = nix-command flakes'
            exec nix develop --quiet --accept-flake-config -c prek run --hook-stage=pre-commit "$@"
            HOOK
              chmod +x .git/hooks/pre-commit
            fi
          '';
          RUST_SRC_PATH = "${stableToolchain}/lib/rustlib/src/rust/library";
        });
      # Opt-in shell for rebuilding architecture diagrams and the mdbook
      # site (`make doc`). Pulled out of default to trim ~80K derivation
      # constructions during eval.
      devShells.docs = pkgs.mkShellNoCC {
        packages = with pkgs; [ graphviz plantuml mdbook ];
      };

      # Opt-in shell for working on the Go SDK under `sdks/go/`. The full
      # toolchain is heavy at eval time (~200K thunks); CI doesn't run it
      # against this repo, and most contributors don't touch Go daily.
      devShells.go = pkgs.mkShellNoCC {
        packages = [ pkgs.go golangci-lint ];
      };

      # Add-on shell for smart-contract work: default + `solc` +
      # `go-ethereum` (abigen) + `FOUNDRY_SOLC`. Rust tests that touch
      # contracts often need both `forge` / `anvil` (in default) and
      # `solc` (here), so contracts is layered on top of default rather
      # than being a separate minimal shell.
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

      # Add-on shell for mutation testing via `dregs`. Layered on
      # contracts (which itself is layered on default) so rust + forge
      # + solc + abigen are all available alongside dregs. Isolated
      # because dregs's flake graph is by far the heaviest eval cost of
      # any single tool in this repo (~3.4M values) — no reason to pay
      # for it during daily contract work.
      devShells.mutation = pkgs.mkShellNoCC {
        inputsFrom = [ self.devShells.${system}.contracts ];
        packages = [ dregs.packages.${system}.unwrapped ];
      };

      # Opt-in shell for the Python helper scripts under `scripts/` —
      # `just py-fmt` / `just py-check` etc. CI calls these scripts with
      # the GitHub-Actions-provided python3, not via nix, so the default
      # shell doesn't need them.
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
            # Rust dependencies
            pkg-config
            openssl
            curl
            protobuf # to compile libp2p-autonat
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
            # Rust dependencies
            pkg-config
            openssl
            curl
            protobuf # to compile libp2p-autonat
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
            # Rust dependencies
            pkg-config
            openssl
            curl
            protobuf # to compile libp2p-autonat
            stableToolchain
          ];
          shellHook = rustShellHook;
        });

      # A separate dev-shell due to large size of dependencies (incl. ghc)
      devShells.echidna =
        let
          solc = pkgs.solc-bin."0.8.28";
          echidna-pkgs = import echidna-nixpkgs { inherit system; };
        in
        myShell {
          packages = [
            # Foundry tools
            pkgs.foundry
            solc

            # Security analysis tools
            echidna-pkgs.slither-analyzer
            echidna-pkgs.echidna
            echidna-pkgs.python3.pkgs.crytic-compile
          ];
          FOUNDRY_SOLC = "${solc}/bin/solc";
        };
    });
}
