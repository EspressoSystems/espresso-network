#!/usr/bin/env bash
# Refresh crates/espresso/telemetry/proto/{remote,types}.proto from prometheus/prometheus
# at a given tag, strip the Go-only `gogoproto` annotations, and update the
# pinned-tag references in proto/README.md and the proto file headers.
#
# Usage:
#   scripts/update-prometheus-protos.sh                  # re-fetch current pin from README
#   scripts/update-prometheus-protos.sh --tag v2.55.1
#
# Idempotent: re-running with the existing pin produces no diff. Writes are
# atomic (temp dir + rename); a failed curl leaves the existing protos
# untouched.

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PROTO_DIR="$ROOT/crates/espresso/telemetry/proto"
README="$PROTO_DIR/README.md"

usage() {
  cat >&2 <<'EOF'
usage: update-prometheus-protos.sh [--tag <git-tag>]

  --tag <git-tag>  Prometheus tag to fetch (e.g. v2.55.1).
                   Defaults to the current pin in crates/espresso/telemetry/proto/README.md.
EOF
  exit 2
}

TAG=
while [ $# -gt 0 ]; do
  case "$1" in
    --tag) [ $# -ge 2 ] || usage; TAG=$2; shift 2 ;;
    -h|--help) usage ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

if [ -z "$TAG" ]; then
  TAG=$(grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+' "$README" | head -1 || true)
  if [ -z "$TAG" ]; then
    echo "could not infer tag from $README; pass --tag explicitly" >&2
    exit 1
  fi
  echo "using current pin: $TAG"
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

BASE="https://raw.githubusercontent.com/prometheus/prometheus/$TAG/prompb"

for name in types remote; do
  echo "fetching $name.proto from $TAG..."
  curl --fail -sSL -o "$TMP/$name.proto.raw" "$BASE/$name.proto"
done

# Strip gogoproto bits — Go codegen hints, no effect on the wire format.
# Removes the import line *and* a single trailing blank line so the result is
# byte-identical to a file vendored by hand (idempotent re-runs).
strip_gogoproto() {
  awk '
    /^import "gogoproto\/gogo\.proto";$/ { skip_blank = 1; next }
    skip_blank && /^$/                   { skip_blank = 0; next }
                                         { skip_blank = 0 }
    { gsub(/ \[\(gogoproto\.nullable\) = false\]/, ""); print }
  ' "$1" > "$2"
}

# Insert the "Vendored from ..." note right after the Apache license block,
# matching the format used when the protos were first vendored.
inject_vendored_note() {
  local file=$1 name=$2
  local url="https://github.com/prometheus/prometheus/blob/$TAG/prompb/$name.proto"
  awk -v url="$url" -v tag="$TAG" '
    /^\/\/ limitations under the License\.$/ && !done {
      print
      print ""
      print "// Vendored from prometheus/prometheus " tag ":"
      print "//   " url
      print "// gogoproto import + `[(gogoproto.nullable) = false]` annotations removed"
      print "// (Go-codegen hints, zero wire-format effect). See ./README.md."
      done = 1
      next
    }
    { print }
  ' "$file" > "$file.noted" && mv "$file.noted" "$file"
}

for name in types remote; do
  strip_gogoproto "$TMP/$name.proto.raw" "$TMP/$name.proto.stripped"
  inject_vendored_note "$TMP/$name.proto.stripped" "$name"
done

# Atomic swap into the proto directory.
mv "$TMP/types.proto.stripped" "$PROTO_DIR/types.proto"
mv "$TMP/remote.proto.stripped" "$PROTO_DIR/remote.proto"

# Bump every `v<MAJOR>.<MINOR>.<PATCH>` in the README to the new tag. The
# Apache "Version 2.0" string lacks the leading `v` and only has two numeric
# components, so it's safe.
sed -i -E "s|v[0-9]+\.[0-9]+\.[0-9]+|$TAG|g" "$README"

echo "running cargo check..."
if ! ( cd "$ROOT" && cargo check -p espresso-telemetry --quiet ); then
  echo "" >&2
  echo "cargo check failed — vendored protos may be incompatible with the build." >&2
  echo "If the failure mentions 'Could not find protoc', re-run inside 'nix develop'." >&2
  exit 1
fi

echo "done — pin is $TAG"
