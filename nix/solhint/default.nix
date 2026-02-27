{ buildNpmPackage, fetchFromGitHub, lib }:

let
  source = lib.importJSON ./source.json;
in
buildNpmPackage {
  pname = "solhint";
  inherit (source) version npmDepsHash;

  src = fetchFromGitHub {
    owner = "protofire";
    repo = "solhint";
    rev = "refs/tags/v${source.version}";
    inherit (source) hash;
  };

  dontNpmBuild = true;
}
