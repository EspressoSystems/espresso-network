# Prebuilt KeyDB (not in nixpkgs): Linux keydb-tools .deb, macOS Homebrew bottle.
{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, dpkg
, openssl
, snappy
, zstd
, bzip2
, lz4
, zlib
, systemd
, curl
, util-linux
, darwin
}:

let
  version = "6.3.4";
  debBase = "https://download.keydb.dev/pkg/open_source/deb/ubuntu22.04_jammy";

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

  # sha256 == ghcr blob digest.
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
      bzip2
      lz4
      zlib
      (lib.getLib systemd)
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
    description = "KeyDB server, prebuilt, for the native demo CDN discovery store";
    homepage = "https://keydb.dev";
    platforms = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
    mainProgram = "keydb-server";
  };
}
