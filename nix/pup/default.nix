{ lib
, fetchurl
, stdenvNoCC
, autoPatchelfHook
,
}:
let
  version = "0.51.0";

  artifacts = {
    x86_64-linux = {
      name = "pup_${version}_Linux_x86_64.tar.gz";
      hash = "sha256-nWgYN4bLQN3T7r20yw39VDHu8Htdjv1U8LReTS6RYZw=";
    };
    aarch64-linux = {
      name = "pup_${version}_Linux_arm64.tar.gz";
      hash = "sha256-bqKzLSsjHGaNos9P2OiQStm/AVxoZ4X2LI1+kHTUsh8=";
    };
    x86_64-darwin = {
      name = "pup_${version}_Darwin_x86_64.tar.gz";
      hash = "sha256-Xty8PiyeQ4igmGK/e2m/B+z0HlUNKqxegkinV03ZBNA=";
    };
    aarch64-darwin = {
      name = "pup_${version}_Darwin_arm64.tar.gz";
      hash = "sha256-8h545Y/8pIbQUWhsOjYijNTy+bDEiNCul84YS59P0lU=";
    };
  };

  system = stdenvNoCC.hostPlatform.system;
  artifact =
    artifacts.${system}
      or (throw "Unsupported system for pup: ${system}");
in
stdenvNoCC.mkDerivation {
  pname = "pup";
  inherit version;

  src = fetchurl {
    url = "https://github.com/datadog-labs/pup/releases/download/v${version}/${artifact.name}";
    inherit (artifact) hash;
  };

  nativeBuildInputs = lib.optionals stdenvNoCC.hostPlatform.isLinux [ autoPatchelfHook ];

  dontConfigure = true;
  dontBuild = true;
  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    install -Dm755 pup $out/bin/pup
    runHook postInstall
  '';

  meta = {
    description = "CLI companion for Datadog workflows";
    homepage = "https://github.com/datadog-labs/pup";
    license = lib.licenses.asl20;
    mainProgram = "pup";
    platforms = builtins.attrNames artifacts;
  };
}
