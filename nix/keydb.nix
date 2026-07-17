# KeyDB is not packaged in nixpkgs and has no clean macOS build, so the native
# demo needs it provided out-of-band. The CDN (marshal/broker) uses KeyDB's
# EXPIREMEMBER command for broker heartbeats; plain redis/valkey do not implement
# it, so a real KeyDB is required.
#
# Rather than build from source (KeyDB vendors jemalloc/lua and its macOS build
# is fragile) this repackages prebuilt binaries:
#   - Linux: the official keydb-tools .deb, relocated onto nixpkgs libs via
#     autoPatchelfHook (the binary is a dynamically-linked PIE).
#   - macOS (aarch64): the Homebrew bottle, whose keydb-server only links
#     openssl@3 outside the system libs; repoint it at nixpkgs openssl and
#     re-sign (install_name_tool invalidates the ad-hoc signature).
{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, dpkg
, openssl
, snappy
, zstd
, curl
, util-linux
, darwin
}:

let
  version = "6.3.4";
  debBase = "https://download.keydb.dev/pkg/open_source/deb/ubuntu22.04_jammy";

  # Linux: official keydb-tools debs (contain keydb-server).
  linuxSrcs = {
    x86_64-linux = fetchurl {
      url = "${debBase}/amd64/keydb-latest/keydb-tools_${version}-1~jammy1_amd64.deb";
      sha256 = "8653a2631858d6e58106e8607de61d4031b2d0c7bba0a8040859df65ea98724b";
    };
    aarch64-linux = fetchurl {
      url = "${debBase}/arm64/keydb-latest/keydb-tools_${version}-1~jammy1_arm64.deb";
      sha256 = "5496d9d116cf2449b7be1b12da866f6d882ccb531ace45c8a3d679fcd0ae98af";
    };
  };

  # macOS aarch64: Homebrew bottle (oldest arm64 build for broad OS compat).
  # The sha256 is the ghcr blob digest, which is also the file content hash.
  darwinBottleSha = "eefed6df2c14cfbab28ac8ce65f888d011bed8a1edec7095b891ba2b418ea733";
  darwinSrc = fetchurl {
    url = "https://ghcr.io/v2/homebrew/core/keydb/blobs/sha256:${darwinBottleSha}";
    sha256 = darwinBottleSha;
    curlOptsList = [ "-H" "Authorization: Bearer QQ==" ];
    name = "keydb-bottle-${version}.tar.gz";
  };

  src =
    if stdenv.isDarwin then darwinSrc
    else linuxSrcs.${stdenv.hostPlatform.system}
      or (throw "keydb.nix: unsupported system ${stdenv.hostPlatform.system}");
in
stdenv.mkDerivation {
  pname = "keydb";
  inherit version src;

  nativeBuildInputs =
    lib.optionals stdenv.isLinux [ autoPatchelfHook dpkg ]
    ++ lib.optionals stdenv.isDarwin [ darwin.autoSignDarwinBinariesHook ];

  buildInputs =
    lib.optionals stdenv.isLinux [
      (lib.getLib stdenv.cc.cc)
      openssl
      snappy
      zstd
      curl
      (lib.getLib util-linux)
    ]
    ++ lib.optionals stdenv.isDarwin [ openssl ];

  unpackPhase =
    if stdenv.isLinux
    then "dpkg-deb -x $src ."
    else "tar xzf $src";

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
  '' + (if stdenv.isLinux then ''
    cp usr/bin/keydb-server $out/bin/
  '' else ''
    cp keydb/${version}/bin/keydb-server $out/bin/
    install_name_tool \
      -change @@HOMEBREW_PREFIX@@/opt/openssl@3/lib/libcrypto.3.dylib ${lib.getLib openssl}/lib/libcrypto.3.dylib \
      -change @@HOMEBREW_PREFIX@@/opt/openssl@3/lib/libssl.3.dylib ${lib.getLib openssl}/lib/libssl.3.dylib \
      $out/bin/keydb-server
  '') + ''
    runHook postInstall
  '';

  meta = {
    description = "KeyDB (Redis fork) server, prebuilt, for the native demo CDN discovery store";
    homepage = "https://keydb.dev";
    platforms = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
    mainProgram = "keydb-server";
  };
}
