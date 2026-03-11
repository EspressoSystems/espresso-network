#!/usr/bin/env bash

if [ -z "$1" ]; then
    echo "Usage: $0 <claude-binary>"
    exit 1
fi

CLAUDE_BIN="$1"
PROJECT_DIR="$HOME/espresso/espresso-sequencer"

bwrap \
    --ro-bind /nix /nix \
    --ro-bind /etc/machine-id /etc/machine-id \
    --ro-bind /etc/resolv.conf /etc/resolv.conf \
    --ro-bind /etc/passwd /etc/passwd \
    --ro-bind-try /etc/ssl/certs /etc/ssl/certs \
    --ro-bind-try /etc/static /etc/static \
    --bind "$PROJECT_DIR" "$PROJECT_DIR" \
    --proc /proc \
    --dev /dev \
    --tmpfs /tmp \
    --bind "$HOME/.claude" "$HOME/.claude" \
    --bind "$HOME/.claude.json" "$HOME/.claude.json" \
    --bind "$HOME/.config/claude" "$HOME/.config/claude" \
    --unshare-all \
    --share-net \
    --die-with-parent \
    --chdir "$PROJECT_DIR" \
    --setenv ANTHROPIC_API_KEY "$ANTHROPIC_API_KEY" \
    --setenv HOME "$HOME" \
    --setenv TERM "$TERM" \
    --setenv PATH "$PATH" \
    --setenv SHELL $(which bash) \
    --setenv TMPDIR /tmp \
    --ro-bind /dev/null "$PROJECT_DIR/.env" \
    "$CLAUDE_BIN"
