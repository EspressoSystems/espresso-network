# Copyright (c) 2024 Espresso Systems (espressosys.com)
# This file is part of the HotShot Builder library.
#
# This program is free software: you can redistribute it and/or modify it under the terms of the GNU
# General Public License as published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
# This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
# even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
# General Public License for more details.
# You should have received a copy of the GNU General Public License along with this program. If not,
# see <https://www.gnu.org/licenses/>.

{
  description = "Generic builder API for HotShot applications";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.flake-compat.url = "github:edolstra/flake-compat";
  inputs.flake-compat.flake = false;

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  inputs.pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  inputs.pre-commit-hooks.inputs.flake-utils.follows = "flake-utils";
  inputs.pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, flake-utils, rust-overlay, pre-commit-hooks, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
        };
        rustDeps = with pkgs;
          [
            pkg-config
            openssl
            bash

            curl
            docker

            cargo-audit
            cargo-edit
            cargo-udeps
            cargo-sort
            cmake

            # `postgresql` defaults to an older version (15), so we select the latest version (16)
            # explicitly.
            postgresql_16
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.SystemConfiguration

            # https://github.com/NixOS/nixpkgs/issues/126182
            libiconv
          ] ++ lib.optionals (!stdenv.isDarwin) [
            cargo-watch # broken: https://github.com/NixOS/nixpkgs/issues/146349
          ];
        cargo-llvm-cov = pkgs.rustPlatform.buildRustPackage rec {
          pname = "cargo-llvm-cov";
          version = "0.5.0";

          doCheck = false;

          buildInputs = [ pkgs.libllvm ];

          src = builtins.fetchTarball {
            url =
              "https://crates.io/api/v1/crates/${pname}/${version}/download";
            sha256 =
              "sha256:1a0grmpcjnqrz5c9jjbk07705py4573pmq0jcgr9m7l5xf4g1yc9";
          };

          cargoSha256 = "sha256-11xNgiOw0qysTWpoKAXQ5gx1uJSAsp+aDDir0zpkpeQ=";
          meta = with pkgs.lib; {
            description = "Cargo llvm cov generates code coverage via llvm.";
            homepage = "https://github.com/taiki-e/cargo-llvm-cov";

            license = with licenses; [ mit asl20 ];
          };
        };
        shellHook  = ''
          # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts with rustup
          # installations.
          export CARGO_HOME=$HOME/.cargo-nix
          export PATH="$PWD/$CARGO_TARGET_DIR/release:$PATH"
        '';

        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        RUST_BACKTRACE = 1;
        RUST_LOG = "info";
      in {
      	checks = {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              cargo-fmt = {
                enable = true;
                description = "Enforce rustfmt";
                entry = "cargo fmt --all -- --check";
                pass_filenames = false;
              };
              cargo-sort = {
                enable = true;
                description = "Ensure Cargo.toml are sorted";
                entry = "cargo sort -g -w";
                pass_filenames = false;
              };
              cargo-clippy = {
                enable = true;
                description = "Run clippy";
                entry = "cargo clippy --workspace --all-features --all-targets -- -D clippy::dbg-macro";
                pass_filenames = false;
              };
            };
          };
        };
        devShell = pkgs.mkShell {
          shellHook = shellHook
          	# install pre-commit hooks
            + self.checks.${system}.pre-commit-check.shellHook;
          buildInputs = with pkgs;
            [
              rust-bin.nightly.latest.rust-analyzer
              nixpkgs-fmt
              git
              mdbook # make-doc, documentation generation
              protobuf
              rustToolchain
            ] ++ rustDeps;

          inherit RUST_SRC_PATH RUST_BACKTRACE RUST_LOG;
        };
        devShells = {
          perfShell = pkgs.mkShell {
            shellHook = shellHook;
            buildInputs = with pkgs;
              [ cargo-llvm-cov rustToolchain protobuf ] ++ rustDeps;

            inherit RUST_SRC_PATH RUST_BACKTRACE RUST_LOG;
          };
        };
      });
}
