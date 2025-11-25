#!/usr/bin/env bash
set -euo pipefail

TEMP_FILES=()
cleanup() {
  for file in "${TEMP_FILES[@]}"; do
    rm -f "$file"
  done
}
# trap cleanup EXIT

usage() {
  cat <<EOF
Download and verify GitHub release assets with checksum validation.

Usage: $(basename "$0") [OPTIONS]

Required:
  --repo OWNER/REPO       GitHub repository (e.g., SUPERCILEX/fuc)
  --asset NAME            Asset name to download

Required (one of):
  --output PATH           Output path for downloaded file
  --extract-to DIR        Extract tarball to directory (auto-cleanup)

Optional:
  --tag TAG               Release tag (default: latest)
  --extract-file FILE     Extract specific file from tarball

Examples:
  # Download binary artifact
  $(basename "$0") --repo SUPERCILEX/fuc --asset x86_64-unknown-linux-gnu-rmz --output /tmp/rmz

  # Download tarball artifcat
  $(basename "$0") --repo F1bonacc1/process-compose --asset process-compose_linux_amd64.tar.gz --extract-to /tmp --extract-file process-compose 

  # Download and extract a single binary from a tarball
  $(basename "$0") --repo foundry-rs/foundry --tag nightly --asset foundry_nightly_linux_amd64.tar.gz --extract-to /tmp --extract-file anvil

EOF
  exit 1
}

repo=""
asset=""
output=""
tag="latest"
extract_to=""
extract_file=""

while [[ $# -gt 0 ]]; do
  case $1 in
    --repo) repo="$2"; shift 2 ;;
    --asset) asset="$2"; shift 2 ;;
    --output) output="$2"; shift 2 ;;
    --tag) tag="$2"; shift 2 ;;
    --extract-to) extract_to="$2"; shift 2 ;;
    --extract-file) extract_file="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

if [[ -z "$repo" || -z "$asset" ]]; then
  echo "Error: --repo and --asset are required"
  usage
fi

if [[ -z "$output" && -z "$extract_to" ]]; then
  echo "Error: Either --output or --extract-to must be specified"
  usage
fi

user_specified_output="$output"
if [[ -z "$output" ]]; then
  output=$(mktemp)
  TEMP_FILES+=("$output")
fi

echo "Fetching release info from GitHub API..."
if [[ "$tag" = "latest" ]]; then
  release_info=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest")
else
  release_info=$(curl -fsSL "https://api.github.com/repos/$repo/releases/tags/$tag")
fi

expected_checksum=$(echo "$release_info" | jq -r ".assets[] | select(.name == \"$asset\") | .digest" | cut -d: -f2)
if [[ -z "$expected_checksum" || "$expected_checksum" = "null" ]]; then
  echo "Error: Could not fetch checksum for $asset"
  echo "Available assets:"
  echo "$release_info" | jq -r '.assets[].name'
  exit 1
fi
echo "Expected SHA256: $expected_checksum"

echo "Downloading $asset..."
download_url=$(echo "$release_info" | jq -r ".assets[] | select(.name == \"$asset\") | .browser_download_url")

max_attempts=5
attempt=1
delay=1

# Our CI sometimes is rate limited by github: try a few times with exponential backoff
while [[ $attempt -le $max_attempts ]]; do
  if [[ $attempt -gt 1 ]]; then
    echo "Retry attempt $attempt/$max_attempts after ${delay}s delay..."
    sleep "$delay"
    delay=$((delay * 2))
  fi

  if curl -fsSL "$download_url" -o "$output"; then
    break
  fi

  if [[ $attempt -eq $max_attempts ]]; then
    echo "Error: Download failed after $max_attempts attempts"
    exit 1
  fi

  echo "Download failed, will retry..."
  attempt=$((attempt + 1))
done

echo "Verifying checksum..."
actual_checksum=$(sha256sum "$output" | cut -d' ' -f1)
if [[ "$actual_checksum" != "$expected_checksum" ]]; then
  echo "Error: Checksum mismatch!"
  echo "Expected: $expected_checksum"
  echo "Actual:   $actual_checksum"
  exit 1
fi
echo "Checksum verified successfully"

if [[ -n "$extract_to" ]]; then
  echo "Extracting to $extract_to..."
  if [[ -n "$extract_file" ]]; then
    tar -xzf "$output" -C "$extract_to" "$extract_file"
  else
    tar -xzf "$output" -C "$extract_to"
  fi
fi

echo ""
echo "Created files:"
if [[ -n "$user_specified_output" ]]; then
  echo "  $user_specified_output"
elif [[ -n "$extract_to" ]]; then
  if [[ -n "$extract_file" ]]; then
    echo "  $extract_to/$extract_file"
  else
    tar -tzf "$output" | while IFS= read -r file; do
      echo "  $extract_to/$file"
    done
  fi
fi
