#!/usr/bin/env bash
# Local release artifact builder. Mirrors what a GitHub Actions release
# workflow would do, but runs on the developer's machine because this repo
# is local-CI-only (see scripts/install-hooks.sh).
#
# Outputs go to dist/ (gitignored) and never touch /usr or any system
# paths. The script never tags, never pushes, never uploads — you stay in
# full control of what reaches users.
#
# Usage:
#   ./scripts/release.sh                      # build for current Cargo version
#   ./scripts/release.sh --skip-deb           # opt-out of .deb step
#   ./scripts/release.sh --skip-rpm           # opt-out of .rpm step
#
# Optional environment variables:
#   PREV_TAG=v0.0.9  → override the "since" point for release notes
#                       (default: most recent annotated tag, or first commit
#                       if none).
#
# After it finishes:
#   dist/
#     ├── bootcontrol-<version>.tar.gz          (source tarball, always)
#     ├── bootcontrol-<version>-RELEASE.md      (notes, always)
#     ├── bootcontrol_<version>_<arch>.deb      (if dpkg-buildpackage exists)
#     ├── bootcontrold_<version>_<arch>.deb     (if dpkg-buildpackage exists)
#     └── bootcontrol-<version>-1.<arch>.rpm    (if rpmbuild exists)
#
# To publish, hand-upload from dist/ to wherever (gh release create, copy
# to a hosting box, hand to a downstream packager).

set -euo pipefail

cd "$(dirname "$0")/.."

SKIP_DEB=0
SKIP_RPM=0
while [ "${1:-}" != "" ]; do
    case "$1" in
        --skip-deb) SKIP_DEB=1 ;;
        --skip-rpm) SKIP_RPM=1 ;;
        -h|--help)
            sed -n '2,30p' "$0"  # echo the comment header
            exit 0
            ;;
        *)
            echo "release.sh: unknown argument: $1" >&2
            exit 2
            ;;
    esac
    shift
done

# Read workspace version from the root Cargo.toml. Greps the unambiguous
# `version = "x.y.z"` line under `[workspace.package]` — the only place
# this template appears in our manifest.
VERSION=$(grep -E '^version\s*=\s*"' Cargo.toml | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$VERSION" ]; then
    echo "release.sh: failed to parse workspace version from Cargo.toml" >&2
    exit 1
fi

ARCH=$(uname -m)
DIST="dist"
mkdir -p "$DIST"

echo "==> version $VERSION ($ARCH)"

# ── Source tarball ─────────────────────────────────────────────────────────
# Use `git archive` so the tarball matches exactly what's committed —
# untracked files (target/, target/, IDE noise) never leak in.
echo "==> building source tarball"
TARBALL="$DIST/bootcontrol-$VERSION.tar.gz"
git archive --format=tar.gz \
    --prefix="bootcontrol-$VERSION/" \
    -o "$TARBALL" \
    HEAD
echo "    $TARBALL  ($(du -h "$TARBALL" | cut -f1))"

# ── Release notes ─────────────────────────────────────────────────────────
# Default "since" point: the most recent annotated tag. Fall back to the
# repo's root commit if there are no tags yet (very first release).
if [ -z "${PREV_TAG:-}" ]; then
    PREV_TAG=$(git describe --tags --abbrev=0 2>/dev/null || true)
fi
if [ -z "${PREV_TAG:-}" ]; then
    PREV_TAG=$(git rev-list --max-parents=0 HEAD | head -n1)
    SINCE_LABEL="repository start"
else
    SINCE_LABEL="$PREV_TAG"
fi

NOTES="$DIST/bootcontrol-$VERSION-RELEASE.md"
{
    echo "# BootControl $VERSION"
    echo
    echo "Released: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "## Changes since $SINCE_LABEL"
    echo
    # One markdown bullet per commit. Conventional Commit prefixes
    # (feat/fix/docs/…) stay visible so the audience can scan by category.
    git log --no-merges --pretty=format:'- %s (%h)' "$PREV_TAG..HEAD"
    echo
    echo
    echo "## SHA-256 checksums"
    echo
    echo '```'
    (cd "$DIST" && sha256sum *.tar.gz 2>/dev/null || true)
    echo '```'
} > "$NOTES"
echo "==> release notes: $NOTES  ($(wc -l < "$NOTES") lines)"

# ── .deb (Debian / Ubuntu) ────────────────────────────────────────────────
if [ "$SKIP_DEB" -eq 0 ]; then
    if command -v dpkg-buildpackage > /dev/null; then
        echo "==> building .deb (dpkg-buildpackage -us -uc -b)"
        # -us -uc = unsigned source / unsigned changes; we sign at upload
        # time (or not at all, if you're handing the artifact to a
        # downstream packager). -b = binary-only build.
        if dpkg-buildpackage -us -uc -b > "$DIST/dpkg-buildpackage.log" 2>&1; then
            # debian/rules drops .deb in the parent directory by default.
            # Move them into dist/ so everything ships from one place.
            mv ../bootcontrol_*${VERSION}*.deb "$DIST/" 2>/dev/null || true
            mv ../bootcontrold_*${VERSION}*.deb "$DIST/" 2>/dev/null || true
            # Some intermediate files debian build leaves alongside; they
            # are not artifacts we ship.
            rm -f ../bootcontrol_*.{buildinfo,changes,deb} \
                  ../bootcontrold_*.{buildinfo,changes,deb} 2>/dev/null || true
            echo "    .deb artifacts moved to $DIST/"
        else
            echo "    !! dpkg-buildpackage failed; see $DIST/dpkg-buildpackage.log"
            SKIP_DEB=2
        fi
    else
        echo "==> skipping .deb (dpkg-buildpackage not installed)"
        SKIP_DEB=2
    fi
fi

# ── .rpm (Fedora / RHEL / openSUSE) ───────────────────────────────────────
if [ "$SKIP_RPM" -eq 0 ]; then
    if command -v rpmbuild > /dev/null; then
        echo "==> building .rpm (rpmbuild -bb packaging/rpm/bootcontrol.spec)"
        # rpmbuild requires a specific tree shape under ~/rpmbuild/. We
        # construct a private one rooted at $PWD/dist/rpmbuild so the
        # build does not contaminate the user's home.
        RPM_TOP="$(pwd)/$DIST/rpmbuild"
        mkdir -p "$RPM_TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
        cp "$TARBALL" "$RPM_TOP/SOURCES/"
        cp packaging/rpm/bootcontrol.spec "$RPM_TOP/SPECS/"
        if rpmbuild --define "_topdir $RPM_TOP" \
                    -bb "$RPM_TOP/SPECS/bootcontrol.spec" \
                    > "$DIST/rpmbuild.log" 2>&1; then
            # Move every generated .rpm out of the RPMS/<arch> subdir.
            find "$RPM_TOP/RPMS" -name '*.rpm' -exec mv {} "$DIST/" \;
            echo "    .rpm artifacts moved to $DIST/"
        else
            echo "    !! rpmbuild failed; see $DIST/rpmbuild.log"
            SKIP_RPM=2
        fi
    else
        echo "==> skipping .rpm (rpmbuild not installed)"
        SKIP_RPM=2
    fi
fi

# ── Final inventory + checksum addendum ───────────────────────────────────
echo
echo "==> artifacts in $DIST/"
ls -la "$DIST" | grep -vE '\.log$|^total|^d|rpmbuild' || true

# Refresh the SHA-256 block in release notes now that all artifacts exist.
# Cheaper than computing it inline because .deb/.rpm timing varies a lot.
SHA_TMP=$(mktemp)
(cd "$DIST" && sha256sum *.tar.gz *.deb *.rpm 2>/dev/null | sort) > "$SHA_TMP"
if [ -s "$SHA_TMP" ]; then
    # Replace the SHA-256 block (the one between '```' fences after the
    # '## SHA-256 checksums' heading).
    awk -v shafile="$SHA_TMP" '
        /^## SHA-256 checksums/ { inblock = 1; print; next }
        inblock == 1 && /^```$/ {
            print "```"
            while ((getline line < shafile) > 0) print line
            print "```"
            inblock = 2
            next
        }
        inblock == 2 && /^```$/ { inblock = 0; next }
        inblock != 2 { print }
    ' "$NOTES" > "$NOTES.new" && mv "$NOTES.new" "$NOTES"
fi
rm -f "$SHA_TMP"

echo
echo "==> done. Hand-publish from $DIST/ when ready."
[ "$SKIP_DEB" -eq 2 ] && echo "    note: .deb was skipped"
[ "$SKIP_RPM" -eq 2 ] && echo "    note: .rpm was skipped"
exit 0
