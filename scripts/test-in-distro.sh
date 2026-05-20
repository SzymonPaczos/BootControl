#!/usr/bin/env bash
# Run the workspace tests inside a podman/docker container of the named
# distro. Catches distro-specific build / behaviour differences that the
# pre-push hook on a single Ubuntu host doesn't see:
#
#   * coreutils variant (GNU vs uutils — bit us on Ubuntu 26.04 with the
#     paranoia test stub script)
#   * default /bin/sh (dash vs bash — bit us with ${@: -1} array slice)
#   * package-manager-installed Rust vs rustup
#   * libc version skew
#
# Usage:
#   ./scripts/test-in-distro.sh ubuntu
#   ./scripts/test-in-distro.sh fedora
#   ./scripts/test-in-distro.sh arch
#   ./scripts/test-in-distro.sh --all          # run every supported distro
#   PODMAN_BIN=docker ./scripts/test-in-distro.sh ubuntu
#
# Requirements on the host:
#   * podman (preferred — rootless) OR docker
#   * Network access for the first run (pull base image)
#   * ~3 GiB of disk per distro (cargo build target dir is hefty)
#
# Mounts the repo read-only into the container as /workspace, copies it
# to a writable location inside (so cargo can build), then runs the full
# scripts/ci-local.sh pipeline — including the E2E step, which works
# because each Containerfile installs dbus + dbus-x11 (provides
# dbus-run-session). Target/ goes to /tmp/target via CARGO_TARGET_DIR so
# the host's build cache isn't touched.

set -euo pipefail

cd "$(dirname "$0")/.."

PODMAN_BIN="${PODMAN_BIN:-}"
if [ -z "$PODMAN_BIN" ]; then
    if command -v podman > /dev/null; then
        PODMAN_BIN=podman
    elif command -v docker > /dev/null; then
        PODMAN_BIN=docker
    else
        cat >&2 <<'EOF'
ERROR: neither `podman` nor `docker` is on $PATH.

Install one:
  Ubuntu / Debian:   sudo apt install podman
  Fedora:            sudo dnf install podman
  Arch:              sudo pacman -S podman

Or set PODMAN_BIN=<binary> if your container runtime has a different name.
EOF
        exit 1
    fi
fi

SUPPORTED=(ubuntu fedora arch)

run_one() {
    local distro="$1"
    local containerfile="containers/$distro/Containerfile"

    if [ ! -f "$containerfile" ]; then
        echo "ERROR: no Containerfile for '$distro' at $containerfile" >&2
        echo "Supported: ${SUPPORTED[*]}" >&2
        return 2
    fi

    echo
    echo "════════════════════════════════════════════════════════════════"
    echo " Running tests in $distro container"
    echo " (via $PODMAN_BIN, Containerfile at $containerfile)"
    echo "════════════════════════════════════════════════════════════════"

    local image="bootcontrol-test-$distro:latest"

    # Build (or rebuild) the image. Each Containerfile pins a base image
    # tag — see containers/<distro>/Containerfile for the rationale.
    "$PODMAN_BIN" build \
        -t "$image" \
        -f "$containerfile" \
        .

    # Run the canonical local-CI script inside. The container's
    # WORKDIR is /workspace and the repo is bind-mounted there. Tests
    # write their target/ tree inside the container — kept separate
    # from the host's target/ so they don't fight over file locks.
    "$PODMAN_BIN" run \
        --rm \
        --name "bootcontrol-test-$distro-$$" \
        -v "$(pwd):/workspace:ro" \
        -e CARGO_TARGET_DIR=/tmp/target \
        "$image" \
        /bin/bash -c "cp -r /workspace /tmp/src && cd /tmp/src && \
                      ./scripts/ci-local.sh"
}

if [ "${1:-}" = "--all" ]; then
    failed=()
    for d in "${SUPPORTED[@]}"; do
        if ! run_one "$d"; then
            failed+=("$d")
        fi
    done
    echo
    if [ ${#failed[@]} -eq 0 ]; then
        echo "==> all distros passed: ${SUPPORTED[*]}"
        exit 0
    else
        echo "==> failures: ${failed[*]}" >&2
        exit 1
    fi
elif [ -n "${1:-}" ]; then
    run_one "$1"
else
    cat >&2 <<EOF
Usage: $0 <distro>|--all
Supported distros: ${SUPPORTED[*]}
EOF
    exit 2
fi
