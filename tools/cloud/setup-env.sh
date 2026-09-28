#!/bin/bash
# KNXBench — setup script for the Claude Code *cloud environment*.
#
# This file is the versioned source of truth. It does not run by itself:
# paste its full content into the "Setup script" field of the KNXBench
# environment at claude.ai/code (see docs/CLOUD_SESSIONS.md §2).
#
# Constraints from the cloud-environment documentation (checked 2026-09-28):
#   * runs as root on Ubuntu 24.04, after the repository clone;
#   * must exit 0, otherwise the session does not start;
#   * should finish in about five minutes, otherwise the filesystem
#     snapshot is not cached and every session pays the full setup again;
#   * "Trusted" network allows the Ubuntu archive and static.rust-lang.org,
#     but lists only `rustup.rs` itself — not `sh.rustup.rs` — so rustup-init
#     is downloaded from static.rust-lang.org directly.
#
# What it installs:
#   1. The Tauri/WebKit development packages CI installs, because the
#      workspace contains `apps/knx-desktop/src-tauri`, and
#      `cargo clippy --workspace` / `cargo test --workspace` build it.
#   2. rustup plus the exact toolchain pinned in rust-toolchain.toml.
#
# Everything else (npm packages, git identity) is per-session work done by
# tools/cloud/session-start.sh, because the npm tree belongs to the checkout.

set -u
export DEBIAN_FRONTEND=noninteractive
log() { echo "[knxbench-setup] $*"; }

RUST_TOOLCHAIN="1.98.0" # keep equal to rust-toolchain.toml

# Evidence for tools/cloud/session-start.sh, which otherwise cannot tell
# "the setup script never ran" from "it ran and failed". /var/tmp survives
# into the cached environment snapshot.
STATE_DIR="/var/tmp/knxbench-cloud"
mkdir -p "$STATE_DIR" 2>/dev/null || true
APT_LOG="$STATE_DIR/setup-apt.log"
STATUS_FILE="$STATE_DIR/setup.status"
RUN_BY="${1:-environment-setup-script}"

# Observed on a real cloud VM (CT-2 session, 2026-09-28): the image arrived with
# an interrupted dpkg run ("dpkg was interrupted, you must manually run
# 'dpkg --configure -a'"), so every install failed. Finishing that run is
# idempotent and harmless on a clean system. The same VM's proxy answers 403
# for some third-party PPAs; `apt-get update` then exits non-zero even though
# the Ubuntu archives were fetched, so its status must not gate the install.
# The install's own exit status is the verdict.
install_system_packages() {
  {
    echo "--- dpkg --configure -a"
    dpkg --configure -a || echo "[knxbench-setup] dpkg --configure -a failed; the install below will say whether it matters"
    echo "--- apt-get update"
    apt-get -o DPkg::Lock::Timeout=120 update ||
      echo "[knxbench-setup] apt-get update reported errors (e.g. a blocked PPA); installing anyway"
    echo "--- apt-get install"
    apt-get -o DPkg::Lock::Timeout=120 install -y --no-install-recommends \
      pkg-config \
      libwebkit2gtk-4.1-dev \
      libgtk-3-dev \
      libayatana-appindicator3-dev \
      librsvg2-dev
  } >"$APT_LOG" 2>&1
}

install_rust() {
  local rustup_bin
  rustup_bin="$(command -v rustup || true)"
  if [ -z "$rustup_bin" ] && [ -x "$HOME/.cargo/bin/rustup" ]; then
    rustup_bin="$HOME/.cargo/bin/rustup"
  fi
  if [ -z "$rustup_bin" ]; then
    # rustup-init dispatches on its own file name (argv[0]); a mktemp name
    # such as `tmp.AbC` fails with "unknown proxy name".
    local dir
    dir="$(mktemp -d)"
    curl --proto '=https' --tlsv1.2 -fsSL \
      https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init \
      -o "$dir/rustup-init" || return 1
    chmod +x "$dir/rustup-init"
    "$dir/rustup-init" -y --no-modify-path --profile minimal --default-toolchain none || return 1
    rm -rf "$dir"
    rustup_bin="$HOME/.cargo/bin/rustup"
  fi
  "$rustup_bin" toolchain install "$RUST_TOOLCHAIN" --profile minimal \
    --component rustfmt --component clippy &&
    "$rustup_bin" default "$RUST_TOOLCHAIN"
}

install_system_packages &
apt_pid=$!
install_rust &
rust_pid=$!

if wait "$apt_pid"; then
  apt_status="ok"
  log "system packages ok"
else
  apt_status="failed (see $APT_LOG)"
  log "WARNING: apt install failed; knx-desktop (Tauri) will not build"
  tail -n 20 "$APT_LOG" 2>/dev/null
fi
if wait "$rust_pid"; then
  rust_status="ok"
  log "rust $RUST_TOOLCHAIN ok"
else
  rust_status="failed"
  log "WARNING: rust toolchain install failed; session-start will report it"
fi

{
  echo "ran_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "run_by=$RUN_BY"
  echo "uid=$(id -u)"
  echo "apt=$apt_status"
  echo "rust=$rust_status"
} >"$STATUS_FILE" 2>/dev/null || log "WARNING: could not write $STATUS_FILE"

# Never block the session: the session-start hook reports what is missing.
exit 0
