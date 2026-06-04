#!/usr/bin/env bash
#
# SessionStart hook: install the `pup` Datadog CLI in Claude Code on the web.
#
# Cloud-only. Local dev shells already provide `pup` via the nix flake
# (see flake.nix / nix/pup), so this hook no-ops outside remote sessions.
#
# The version and SHA-256 checksums are pinned to match nix/pup/default.nix
# so the binary installed here is byte-for-byte identical to the one local
# developers get. If you bump pup there, bump it here too.
set -euo pipefail

# Cloud-only: skip entirely in local sessions.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

PUP_VERSION="0.51.0"
# sha256 of the GitHub release tarballs (hex of the SRI hashes in nix/pup/default.nix).
SHA256_X86_64="9d68183786cb40ddd3eebdb4cb0dfd5431eef07b5d8efd54f0b45e4d2e91619c"
SHA256_AARCH64="6ea2b32d2b231c668da2cf4fd8e8904ad9bf015c686785f62c8d7e9074d4b21f"

# Prefer /usr/local/bin (already on PATH). Fall back to ~/.local/bin and export
# it via $CLAUDE_ENV_FILE so later Bash commands in the session can find pup.
if [ -w /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      export PATH="${INSTALL_DIR}:${PATH}"
      if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
        echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >>"$CLAUDE_ENV_FILE"
      fi
      ;;
  esac
fi
PUP_BIN="${INSTALL_DIR}/pup"

# Idempotent: nothing to do if the pinned version is already installed.
if [ -x "$PUP_BIN" ] && "$PUP_BIN" --version 2>/dev/null | grep -q "$PUP_VERSION"; then
  echo "pup ${PUP_VERSION} already installed at ${PUP_BIN}"
  exit 0
fi

# Map architecture to the matching release asset + checksum.
case "$(uname -m)" in
  x86_64 | amd64) ASSET="pup_${PUP_VERSION}_Linux_x86_64.tar.gz"; EXPECTED="$SHA256_X86_64" ;;
  aarch64 | arm64) ASSET="pup_${PUP_VERSION}_Linux_arm64.tar.gz"; EXPECTED="$SHA256_AARCH64" ;;
  *) echo "pup: unsupported architecture '$(uname -m)'; skipping install" >&2; exit 0 ;;
esac

URL="https://github.com/datadog-labs/pup/releases/download/v${PUP_VERSION}/${ASSET}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Installing pup ${PUP_VERSION} from ${URL} ..."
if ! curl -fsSL --retry 3 --retry-delay 2 -o "${tmp}/pup.tar.gz" "$URL"; then
  echo "pup: download failed. Network access must allow GitHub release assets" >&2
  echo "     (covered by the default 'Trusted' level). Skipping install." >&2
  exit 0
fi

GOT="$(sha256sum "${tmp}/pup.tar.gz" | cut -d' ' -f1)"
if [ "$GOT" != "$EXPECTED" ]; then
  echo "pup: checksum mismatch for ${ASSET}" >&2
  echo "     expected ${EXPECTED}" >&2
  echo "     got      ${GOT}" >&2
  echo "     Refusing to install a binary that does not match the pin in nix/pup." >&2
  exit 1
fi

tar -xzf "${tmp}/pup.tar.gz" -C "$tmp"
install -Dm755 "${tmp}/pup" "$PUP_BIN"
echo "Installed $("$PUP_BIN" --version) to ${PUP_BIN}"

# Authentication is supplied at runtime (no secrets are baked into this hook).
# pup reads DD_* env vars natively; otherwise authenticate interactively with
# read-only scopes. See nix/pup/README.md.
if [ -z "${DD_API_KEY:-}" ] && [ -z "${DD_ACCESS_TOKEN:-}" ]; then
  echo "pup: no Datadog credentials detected. To authenticate, either:"
  echo "  - set DD_API_KEY + DD_APP_KEY (+ DD_SITE) in the environment's variables, or"
  echo "  - run: pup auth login --scopes metrics_read,logs_read_data,monitors_read,dashboards_read"
fi

exit 0
