#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

REPO="$TMP_ROOT/repo"
BIN="$TMP_ROOT/bin"
mkdir -p "$REPO/.claude/rules" "$BIN" "$TMP_ROOT/toolkit/scripts" "$TMP_ROOT/toolkit/skills"
for crate in core daemon client cli tui gui; do
    mkdir -p "$REPO/crates/$crate/src"
done
cp "$ROOT/.claude/audit.sh" "$REPO/.claude/audit.sh"
printf '# Audit Log\n\n---\n' > "$REPO/.claude/audit-log.md"
printf '## Decyzje aktywne\n\n## Decyzje wycofane\n' > "$REPO/.claude/decisions-register.md"
printf 'fn main() { validate_policy_file(); }\n' > "$REPO/crates/daemon/src/main.rs"

# Fixture-only platform/toolkit controls; no real compiler or toolkit is invoked.
printf '2026.11.07\n' > "$TMP_ROOT/toolkit/VERSION"
cat > "$TMP_ROOT/toolkit/scripts/toolkit-sync.sh" <<'EOF'
#!/usr/bin/env bash
[ "${1:-}" = check ] || exit 64
exit "${FORCE_TOOLKIT_FAILURE:-0}"
EOF
cat > "$BIN/uname" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "${AUDIT_TEST_OS:-Linux}"
EOF
chmod +x "$BIN/uname"

cat > "$BIN/rustc" <<'EOF'
#!/usr/bin/env bash
printf 'rustc fixture\n'
EOF

cat > "$BIN/cargo-udeps" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

cat > "$BIN/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = "--version" ]; then
    printf 'cargo fixture\n'
    exit 0
fi

case "$*" in
    "build -p bootcontrold"|"fmt --all -- --check")
        exit 0
        ;;
    "+nightly udeps --workspace --all-features --all-targets")
        if [ "${FORCE_UDEPS_ERROR:-0}" = "1" ]; then
            printf 'forced cargo-udeps execution error\n' >&2
            exit 2
        fi
        if [ "${FORCE_UDEPS_FINDINGS:-0}" = "1" ]; then
            printf '%s\n' 'unused dependencies:' \
                '`crate-a v0.1.0 (/fixture/a)`' \
                '└─── dependencies' \
                '     ├─── "unused-one"' \
                '     └─── "unused-two"'
            exit 1
        fi
        exit 0
        ;;
    "clippy --workspace --all-targets --all-features -- -D warnings")
        if [ "${FORCE_CLIPPY_FAILURE:-0}" = "1" ]; then
            for n in 1 2 3 4 5 6; do
                printf 'error: forced clippy error %s\n' "$n" >&2
            done
            exit 1
        fi
        exit 0
        ;;
esac

if [ "${1:-}" != "test" ]; then
    printf 'unexpected cargo invocation: %s\n' "$*" >&2
    exit 64
fi

package=""
ignored=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        -p)
            package="$2"
            shift 2
            ;;
        --ignored)
            ignored=1
            shift
            ;;
        *)
            shift
            ;;
    esac
done

if [ "${FORCE_TEST_PACKAGE_FAILURE:-}" = "$package" ]; then
    printf 'forced test enumeration failure for %s\n' "$package" >&2
    exit 1
fi

case "$package" in
    bootcontrol-core) total=239; docs=47; skipped=0 ;;
    bootcontrold) total=235; docs=37; skipped=0 ;;
    bootcontrol-client) total=28; docs=7; skipped=0 ;;
    bootcontrol-cli) total=5; docs=0; skipped=0 ;;
    bootcontrol-tui) total=87; docs=15; skipped=0 ;;
    bootcontrol-gui) total=3; docs=0; skipped=3 ;;
    *) exit 65 ;;
esac

if [ "$ignored" -eq 1 ]; then
    for ((n = 1; n <= skipped; n++)); do
        printf 'ignored_%s: test\n' "$n"
    done
    exit 0
fi

unit=$((total - docs))
for ((n = 1; n <= unit; n++)); do
    printf 'unit_%s: test\n' "$n"
done
for ((n = 1; n <= docs; n++)); do
    printf 'crates/fixture/src/lib.rs - fixture_%s (line %s): test\n' "$n" "$n"
done
EOF
chmod +x "$BIN/cargo" "$BIN/cargo-udeps" "$BIN/rustc"

run_audit() {
    local output="$1"
    shift
    env PATH="$BIN:$PATH" CLAUDE_TOOLKIT="$TMP_ROOT/toolkit" "$@" \
        bash "$REPO/.claude/audit.sh" > "$output" 2>&1
}

healthy="$TMP_ROOT/healthy.out"
if ! run_audit "$healthy"; then
    cat "$healthy" >&2
    echo 'expected healthy audit fixture to pass' >&2
    exit 1
fi
grep -Fq '| core | 239 | 0 | 47 | 239 ✅ / 47 ✅ |' "$healthy"
grep -Fq '| cli | 5 | 0 | n/a | 5 ✅ / n/a |' "$healthy"
grep -Fq '| gui | 3 | 3 | 0 | 3 ✅ / 0 ✅ |' "$healthy"

clippy="$TMP_ROOT/clippy.out"
if run_audit "$clippy" FORCE_CLIPPY_FAILURE=1; then
    echo 'expected clippy failure to make audit.sh fail' >&2
    exit 1
fi
grep -Fq 'clippy: ❌ warnings=0 errors=6' "$clippy"
grep -Fq 'forced clippy error 1' "$clippy"
grep -Fq 'forced clippy error 5' "$clippy"
if grep -Fq 'forced clippy error 6' "$clippy"; then
    echo 'audit.sh emitted more than the promised top five clippy findings' >&2
    exit 1
fi

runner="$TMP_ROOT/runner.out"
if run_audit "$runner" FORCE_TEST_PACKAGE_FAILURE=bootcontrold; then
    echo 'expected runner enumeration failure to make audit.sh fail' >&2
    exit 1
fi
grep -Fq '| daemon | BLOCKED | BLOCKED | BLOCKED | ❌ runner failed |' "$runner"

printf 'fn forced_violation() { None::<u8>.expect("forced"); }\n' > "$REPO/crates/core/src/lib.rs"
budget="$TMP_ROOT/budget.out"
if run_audit "$budget"; then
    echo 'expected a strict panic-budget violation to make audit.sh fail' >&2
    exit 1
fi
grep -Fq '| core | 0 | 1 | 0 | 0/0/0 (strict) ❌ |' "$budget"
grep -Fq '**⚠ PANIC BUDGET BREACH**: core ' "$budget"
printf 'fn healthy_again() {}\n' > "$REPO/crates/core/src/lib.rs"

udeps_findings="$TMP_ROOT/udeps-findings.out"
if ! run_audit "$udeps_findings" FORCE_UDEPS_FINDINGS=1; then
    cat "$udeps_findings" >&2
    echo 'expected cargo-udeps findings to be reported without an exec error' >&2
    exit 1
fi
grep -Fq 'cargo-udeps: 2 unused dependencies' "$udeps_findings"

udeps_error="$TMP_ROOT/udeps-error.out"
if run_audit "$udeps_error" FORCE_UDEPS_ERROR=1; then
    echo 'expected cargo-udeps execution error to make audit.sh fail' >&2
    exit 1
fi
grep -Fq 'cargo-udeps: ❌ exec error (exit 2)' "$udeps_error"

# Missing/incomplete adoption blocks the overall result without cancelling
# independent checks or converting their results into a fictitious PASS.
preflight="$TMP_ROOT/preflight.out"
if run_audit "$preflight" FORCE_TOOLKIT_FAILURE=1; then
    echo 'expected incomplete toolkit adoption to block the overall audit' >&2
    exit 1
fi
grep -Fq 'TOOLKIT_STATE: BLOCKED (toolkit-sync check exit 1;' "$preflight"
grep -Fq 'clippy: ✅ 0 findings' "$preflight"
grep -Fq '| core | 239 | 0 | 47 | 239 ✅ / 47 ✅ |' "$preflight"

missing="$TMP_ROOT/missing-toolkit.out"
if run_audit "$missing" CLAUDE_TOOLKIT="$TMP_ROOT/missing-toolkit"; then
    echo 'expected an unavailable toolkit preflight to block the overall audit' >&2
    exit 1
fi
grep -Fq 'TOOLKIT_STATE: BLOCKED (trusted toolkit not located)' "$missing"
grep -Fq '| core | 239 | 0 | 47 | 239 ✅ / 47 ✅ |' "$missing"

nonlinux="$TMP_ROOT/nonlinux.out"
if run_audit "$nonlinux" AUDIT_TEST_OS=Darwin; then
    echo 'expected unavailable native Linux evidence to block the overall audit' >&2
    exit 1
fi
grep -Fq 'host `Darwin`, nie Linux' "$nonlinux"
grep -Fq '| core | 239 | 0 | 47 | 239 ✅ / 47 ✅ |' "$nonlinux"

skipbuild="$TMP_ROOT/skipbuild.out"
if run_audit "$skipbuild" SKIP_BUILD_GATE=1; then
    echo 'expected a skipped daemon build to block the overall audit' >&2
    exit 1
fi
grep -Fq 'build gate daemona POMINIĘTY' "$skipbuild"
grep -Fq '| core | 239 | 0 | 47 | 239 ✅ / 47 ✅ |' "$skipbuild"

echo 'audit evidence meta-tests: 10/10 passed'
