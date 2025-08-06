# A simplest nix shell file with the project dependencies and
# a cross-compilation support.
{ pkgs, envVars, rustShellHook }:
pkgs.mkShell (envVars // {
  # Native project dependencies like build utilities and additional routines
  # like container building, linters, etc.
  nativeBuildInputs = with pkgs.pkgsBuildHost; [
    # Rust
    (rust-bin.stable.latest.minimal.override {
      extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
    })

    # Will add some dependencies like libiconv
    rustBuildHostDependencies

    # Crate dependencies
    cargoDeps.openssl-sys
    protobuf # required by libp2p

    openssh
  ];
  # Libraries essential to build the service binaries
  buildInputs = with pkgs; [
    # Enable Rust cross-compilation support
    rustCrossHook
  ];

  shellHook = rustShellHook;
})
