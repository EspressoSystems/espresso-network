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

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

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
      RUST_LOG = "info,node=error";
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
      myShell = pkgs.mkShellNoCC.override (lib.optionalAttrs stdenv.hostPlatform.isLinux {
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
      stableToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      rustDevEnv = rustEnvVars // {
        RUST_SRC_PATH = "${stableToolchain}/lib/rustlib/src/rust/library";
      };
      solc = pkgs.solc-bin."0.8.35";
      solcEnv = { FOUNDRY_SOLC = "${solc}/bin/solc"; };
      # inputsFrom merges packages and shellHooks, not stdenv or env vars
      addOnShell = { inputsFrom ? [ self.devShells.${system}.default ], ... }@attrs:
        myShell (rustDevEnv // attrs // { inherit inputsFrom; });
    in
    {
      devShells.default =
        let
          # Bump the date when a newer rust-analyzer/rustfmt is wanted.
          nightlyToolchain = pkgs.rust-bin.nightly."2026-04-16".minimal.override {
            extensions = [ "rust-analyzer" "rustfmt" ];
          };
        in
        myShell (rustDevEnv // {
          packages = with pkgs; [
            pkg-config
            openssl
            curl
            protobuf # protoc, for the espresso-api v2 codegen
            buf # just proto-check
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
            postgresql_18 # TmpDb in rust tests, scripts/run-postgres for demo-native
            datadog-pup
            lazydocker # used by `just demo`
            foundry # need `anvil` in rust tests
            prettier
            solhint
            libusb1 # link-time dep of `libusb1-sys` crate
            coreutils # GNU timeout, realpath -m on macOS
            bc # scripts/verify-pos-deployment.sh
          ] ++ lib.optionals stdenv.hostPlatform.isDarwin [ pkgs.darwin.libresolv ];
          shellHook = ''
            ${rustShellHook}

            export PATH="$my_pwd/scripts:$PATH"

            # Avoid clash with rustup's ~/.cargo
            export CARGO_HOME=$HOME/.cargo-nix

            # Skipped for with-tools fallbacks: they may run inside a git hook.
            if [ -f .pre-commit-config.yaml ] && [ -z "''${WITH_TOOLS:-}" ] && git rev-parse --git-dir >/dev/null 2>&1; then
              prek install -q --overwrite
            fi
          '';
        });
      devShells.docs = addOnShell {
        packages = with pkgs; [ graphviz plantuml mdbook ];
      };

      devShells.go = addOnShell {
        packages = [ pkgs.go_1_27 golangci-lint ];
      };

      devShells.contracts = addOnShell (solcEnv // {
        packages = [ solc pkgs.go-ethereum pkgs.python3 ]; # python3: forge ffi storage-layout tests
      });

      devShells.mutation = addOnShell (solcEnv // {
        inputsFrom = [ self.devShells.${system}.contracts ];
        packages = [ dregs.packages.${system}.unwrapped ];
      });

      devShells.python = addOnShell {
        packages = with pkgs; [ python3 uv ruff ty ];
        # Let uv-installed binary wheels (matplotlib/numpy/...) find libstdc++ on NixOS.
        shellHook = ''
          export LD_LIBRARY_PATH="${lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib pkgs.zlib ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        '';
      };

      devShells.aws = addOnShell {
        inputsFrom = [ self.devShells.${system}.python ];
        packages = with pkgs; [ opentofu awscli2 ]; # scripts/network-bench/aws
      };

      devShells.dockerShell = addOnShell {
        packages = [ pkgs.docker ];
        # Required for demo-native to run with docker-rootless
        shellHook = ''
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
            protobuf # protoc, for the espresso-api v2 codegen
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
            protobuf # protoc, for the espresso-api v2 codegen
            toolchain
            grcov
          ];
          CARGO_INCREMENTAL = "0";
          shellHook = ''
            ${rustShellHook}
            # These replace `build.rustflags` from `.cargo/config.toml` rather than adding to it,
            # so `--cfg tokio_unstable` (needed for the blocking-pool metrics) is repeated here.
            export RUSTFLAGS="$RUSTFLAGS -Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort -Zpanic_abort_tests -Cdebuginfo=2 --cfg tokio_unstable"
          '';
          RUSTDOCFLAGS = "-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort -Zpanic_abort_tests --cfg tokio_unstable";
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
            protobuf # protoc, for the espresso-api v2 codegen
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
