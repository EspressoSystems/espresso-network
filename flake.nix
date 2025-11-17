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

  # Use ...foundry.nix/stable for latest stable release
  # On 1.4 foundry's formatting is a bit strange, so we pin 1.3.6 for now
  inputs.foundry-nix.url = "github:shazow/foundry.nix/e632b06dc759e381ef04f15ff9541f889eda6013";
  inputs.foundry-nix.inputs.nixpkgs.follows = "nixpkgs";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.nixpkgs-cross-overlay.url = "github:alekseysidorov/nixpkgs-cross-overlay";
  inputs.nixpkgs-cross-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.solc-bin.url = "github:EspressoSystems/nix-solc-bin";
  inputs.solc-bin.inputs.nixpkgs.follows = "nixpkgs";

  inputs.flake-compat.url = "github:edolstra/flake-compat";
  inputs.flake-compat.flake = false;

  inputs.git-hooks.url = "github:cachix/git-hooks.nix";
  inputs.git-hooks.inputs.nixpkgs.follows = "nixpkgs";

  outputs =
    { self
    , nixpkgs
    , foundry-nix
    , rust-overlay
    , nixpkgs-cross-overlay
    , flake-utils
    , git-hooks
    , solc-bin
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

      solhintPkg = { buildNpmPackage, fetchFromGitHub }:
        buildNpmPackage rec {
          pname = "solhint";
          version = "5.05"; # TODO: normally semver, tag screwed up
          src = fetchFromGitHub {
            owner = "protofire";
            repo = pname;
            rev = "refs/tags/v${version}";
            hash = "sha256-F8x3a9OKOQuhMRq6CHh5cVlOS72h+YGHTxnKKAh6c9A=";
          };
          npmDepsHash = "sha256-FKoh5D6sjZwhu1Kp+pedb8q6Bv0YYFBimdulugZ2RT0=";
          dontNpmBuild = true;
        };

      overlays = [
        (import rust-overlay)
        foundry-nix.overlay
        solc-bin.overlays.default
        (final: prev: {
          solhint =
            solhintPkg { inherit (prev) buildNpmPackage fetchFromGitHub; };
        })

        (final: prev: {
          golangci-lint = prev.golangci-lint.overrideAttrs (old: rec {
            version = "1.64.8";
            src = prev.fetchFromGitHub {
              owner = "golangci";
              repo = "golangci-lint";
              rev = "v${version}";
              sha256 = "sha256-ODnNBwtfILD0Uy2AKDR/e76ZrdyaOGlCktVUcf9ujy8";
            };
            vendorHash = "sha256-/iq7Ju7c2gS7gZn3n+y0kLtPn2Nn8HY/YdqSDYjtEkI=";
          });
        })

        (final: prev: {
          prek-as-pre-commit = final.runCommand "prek-as-pre-commit" { } ''
            mkdir -p $out/bin
            ln -s ${final.prek}/bin/prek $out/bin/pre-commit
          '';
        })
      ];
      pkgs = import nixpkgs { inherit system overlays; };
      myShell = pkgs.mkShellNoCC.override (pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
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
    with pkgs; {
      checks = {
        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          # Use the rust pre-commit implementation `prek`
          imports = [
            ({ lib, ... }: {
              config.package = lib.mkForce prek;
            })
          ];
          hooks = {
            doc = {
              enable = true;
              description = "Generate figures";
              entry = "make doc";
              types_or = [ "plantuml" ];
              pass_filenames = false;
            };
            cargo-fmt = {
              enable = true;
              description = "Enforce rustfmt";
              entry = "just fmt";
              types_or = [ "rust" "toml" ];
              pass_filenames = false;
            };
            cargo-sort = {
              enable = true;
              description = "Ensure Cargo.toml are sorted";
              entry = "cargo sort -g -w";
              types_or = [ "toml" ];
              pass_filenames = false;
            };
            cargo-lock = {
              enable = true;
              description = "Ensure Cargo.lock is compatible with Cargo.toml";
              entry = "cargo update --workspace --verbose";
              types_or = [ "toml" ];
              pass_filenames = false;
            };
            forge-fmt = {
              enable = true;
              description = "Enforce forge fmt";
              entry = "forge fmt";
              types_or = [ "solidity" ];
              pass_filenames = false;
            };
            solhint = {
              enable = true;
              description = "Solidity linter";
              entry = "solhint --fix --noPrompt 'contracts/{script,src,test}/**/*.sol'";
              types_or = [ "solidity" ];
              pass_filenames = true;
            };
            contract-bindings = {
              enable = true;
              description = "Generate contract bindings";
              entry = "just gen-bindings";
              types_or = [ "solidity" ];
              pass_filenames = false;
            };
            prettier-fmt = {
              enable = true;
              description = "Enforce markdown formatting";
              entry = "prettier -w";
              types_or = [ "markdown" "ts" ];
              pass_filenames = true;
            };
            spell-checking = {
              enable = true;
              description = "Spell checking";
              # --force-exclude to exclude excluded files if they are passed as arguments
              entry = "typos --force-exclude";
              pass_filenames = true;
              # Add excludes to the .typos.toml file instead
            };
            nixpkgs-fmt.enable = true;
          };
        };
      };
      devShells.default =
        let
          stableToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal.override {
            extensions = [ "rust-analyzer" "rustfmt" ];
          });
          solc = pkgs.solc-bin."0.8.28";
          pre-commit = self.checks.${system}.pre-commit-check;
        in
        myShell (rustEnvVars // {
          packages = [
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
            cargo-mutants
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
            entr
            process-compose
            lazydocker # a docker compose TUI
            # `postgresql` defaults to an older version (15), so we select the latest version (16)
            # explicitly.
            postgresql_16

            # Figures
            graphviz
            plantuml
            coreutils

            # Ethereum contracts, solidity, ...
            foundry-bin
            solc
            nodePackages.prettier
            solhint
            (python3.withPackages (ps: with ps; [ black ]))
            libusb1
            yarn

            go
            golangci-lint
            # provides abigen
            go-ethereum
          ] ++ lib.optionals (!stdenv.isDarwin) [ cargo-watch ] # broken on OSX
          ++ pre-commit.enabledPackages;
          shellHook = ''
            ${rustShellHook}

            # Add the local scripts to the PATH
            export PATH="$my_pwd/scripts:$PATH"

            # Add node binaries to PATH for development
            export PATH="$my_pwd/node_modules/.bin:$PATH"

            # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts
            # with rustup installations.
            export CARGO_HOME=$HOME/.cargo-nix

            ${pre-commit.shellHook}
          '';
          RUST_SRC_PATH = "${stableToolchain}/lib/rustlib/src/rust/library";
          FOUNDRY_SOLC = "${solc}/bin/solc";
        });
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
          packages = [
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
          packages = [
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
          packages = [
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
        in
        myShell {
          packages = [
            # Foundry tools
            foundry-bin
            solc

            # Security analysis tools
            slither-analyzer
            echidna
            python3.pkgs.crytic-compile
          ];
          FOUNDRY_SOLC = "${solc}/bin/solc";
        };
    });
}
