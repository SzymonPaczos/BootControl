#!/usr/bin/env bash
# One-shot installer for the local-CI git hooks.
#
# Why this file exists: `.git/hooks/` is not under version control, so the
# repo cannot ship a hook directly. Git's standard answer is the
# `core.hooksPath` setting, which redirects hook lookup to a directory we
# *can* track. This script wires it up.
#
# Usage:
#   ./scripts/install-hooks.sh
#
# Per-clone — every contributor runs it once after cloning. CI lives in
# `scripts/ci-local.sh` and the pre-push hook gates `git push` on it.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -d .githooks ]; then
    echo "ERROR: .githooks/ directory missing from this clone." >&2
    exit 1
fi

# Set core.hooksPath in *this* clone's config. Local — does not modify
# global ~/.gitconfig and does not affect other repositories on the host.
git config core.hooksPath .githooks

# Make sure every hook in .githooks/ is executable. The bit is preserved
# by git on Linux/macOS but a fresh clone on a filesystem that does not
# preserve it (e.g. SMB mount) would otherwise silently no-op.
chmod +x .githooks/*

echo "Installed git hooks from .githooks/"
echo
echo "Pushes from this clone now run scripts/ci-local.sh first."
echo "Bypass once with: git push --no-verify"
