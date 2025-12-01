{
  description = "Espresso Staking UI Server";

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

  inputs.foundry-nix.url = "github:shazow/foundry.nix/e632b06dc759e381ef04f15ff9541f889eda6013";
  inputs.foundry-nix.inputs.nixpkgs.follows = "nixpkgs";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.flake-compat.url = "github:edolstra/flake-compat";
  inputs.flake-compat.flake = false;

  inputs.git-hooks.url = "github:cachix/git-hooks.nix";
  inputs.git-hooks.inputs.nixpkgs.follows = "nixpkgs";

  outputs =
    { self
    , nixpkgs
    , foundry-nix
    , rust-overlay
    , flake-utils
    , git-hooks
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      RUST_LOG = "info,isahc=error,surf=error";
      RUST_BACKTRACE = 1;
      RUST_MIN_STACK = 10485760;
      rustEnvVars = { inherit RUST_LOG RUST_BACKTRACE RUST_MIN_STACK; };

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
        foundry-nix.overlay
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
          stableToolchain = pkgs.rust-bin.stable.latest.minimal.override {
            extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
          };
          nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal.override {
            extensions = [ "rust-analyzer" "rustfmt" ];
          });
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
            nodePackages.prettier
            foundry-bin
          ] ++ lib.optionals (!stdenv.isDarwin) [ cargo-watch ] # broken on OSX
          ++ pre-commit.enabledPackages;
          shellHook = ''
            ${rustShellHook}

            # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts
            # with rustup installations.
            export CARGO_HOME=$HOME/.cargo-nix

            alias nix='nix --extra-experimental-features nix-command --extra-experimental-features flakes'

            ${pre-commit.shellHook}
          '';
          RUST_SRC_PATH = "${stableToolchain}/lib/rustlib/src/rust/library";
        });
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
    });
}
