#!/usr/bin/env nix-shell
#! nix-shell -i bash -p curl jq nix-prefetch-github prefetch-npm-deps
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SOLHINT_DIR="nix/solhint"
OWNER="protofire"
REPO="solhint"

echo "Fetching latest release from GitHub..."
LATEST_VERSION=$(curl -s "https://api.github.com/repos/$OWNER/$REPO/releases/latest" | jq -r '.tag_name' | sed 's/^v//')
echo "Latest version: $LATEST_VERSION"

echo "Fetching source hash..."
nix-prefetch-github --rev "v$LATEST_VERSION" "$OWNER" "$REPO" > /tmp/solhint-prefetch.json
SRC_HASH=$(jq -r '.hash' /tmp/solhint-prefetch.json)
echo "Source hash: $SRC_HASH"

echo "Fetching source and computing npmDepsHash..."
SRC_DIR=$(mktemp -d)
trap "rm -rf $SRC_DIR" EXIT

curl -sL "https://github.com/$OWNER/$REPO/archive/refs/tags/v$LATEST_VERSION.tar.gz" | tar xz -C "$SRC_DIR" --strip-components=1

NPM_DEPS_HASH=$(prefetch-npm-deps "$SRC_DIR/package-lock.json")
echo "npmDepsHash: $NPM_DEPS_HASH"

echo "Writing source.json..."
cat > "$SOLHINT_DIR/source.json" <<EOF
{
  "version": "$LATEST_VERSION",
  "hash": "$SRC_HASH",
  "npmDepsHash": "$NPM_DEPS_HASH"
}
EOF

echo "Update complete!"
echo "Updated $SOLHINT_DIR/source.json:"
echo "  version: $LATEST_VERSION"
echo "  hash: $SRC_HASH"
echo "  npmDepsHash: $NPM_DEPS_HASH"
