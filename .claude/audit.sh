#!/usr/bin/env bash
# audit.sh — cotygodniowy audyt jakości (warstwa statyczna).
# Liczy metryki, dopisuje sekcję na górę .claude/audit-log.md. BEZ LLM.
# Pełna procedura: .claude/rules/audit.md
#
# Uruchom w głównym working tree (nie worktree):
#   bash .claude/audit.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

LOG="$REPO_ROOT/.claude/audit-log.md"
# Timestamp z minutą — żeby kilka runów tego samego dnia nie kolidowało
# (każdy wpis ma unikalny nagłówek, log zachowuje chronologię).
DATE="$(date '+%Y-%m-%d %H:%M')"
section=""
add() { section+="$1"$'\n'; }

CRATES="core daemon client cli tui gui"

add "## Audyt $DATE"
add ""
add "_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._"
add ""

# === 1. Toolchain & basic build sanity =========================================
add "### Toolchain"
add "- rustc: \`$(rustc --version 2>/dev/null || echo 'BRAK')\`"
add "- cargo: \`$(cargo --version 2>/dev/null || echo 'BRAK')\`"
add ""

# === 2. Format ================================================================
add "### Formatowanie"
if command -v cargo >/dev/null 2>&1; then
    if cargo fmt --all -- --check >/dev/null 2>&1; then
        add "- cargo fmt: ✅ czysto"
    else
        add "- cargo fmt: ❌ wymaga \`cargo fmt --all\`"
    fi
else
    add "- cargo fmt: ⚠️  cargo niedostępne"
fi
add ""

# === 3. Clippy ================================================================
add "### Clippy (workspace, -D warnings)"
if command -v cargo >/dev/null 2>&1; then
    CLIPPY_TMP="$(mktemp)"
    if cargo clippy --workspace --all-targets --all-features -- -D warnings >"$CLIPPY_TMP" 2>&1; then
        add "- clippy: ✅ 0 findings"
    else
        CLIPPY_WARN=$(grep -cE "^warning:" "$CLIPPY_TMP" || echo 0)
        CLIPPY_ERR=$(grep -cE "^error:" "$CLIPPY_TMP" || echo 0)
        add "- clippy: ❌ warnings=$CLIPPY_WARN errors=$CLIPPY_ERR"
        add "  - top 5 findings:"
        grep -E "^(warning|error):" "$CLIPPY_TMP" | head -5 | sed 's/^/    /' | while IFS= read -r line; do section+="$line"$'\n'; done
    fi
    rm -f "$CLIPPY_TMP"
else
    add "- clippy: ⚠️  cargo niedostępne"
fi
add ""

# === 4. unwrap/expect/panic budgets per crate =================================
# Zlicza wystąpienia TYLKO w kodzie produkcyjnym: pomija linie po pierwszym
# `^#[cfg(test)]` / `^mod tests` w pliku (inline mod tests) i pomija linie
# zaczynające się `^/// ` (doctesty — wykonywane przez `cargo test --doc`,
# więc to też test code per decyzji "Doctests are integration tests").
add "### \`unwrap\` / \`expect\` / \`panic!\` w production (poza mod tests i doctestach)"
add ""
add "| Crate | unwrap | expect | panic! | Budżet |"
add "|-------|--------|--------|--------|--------|"
count_in_production() {
    # $1 = ERE pattern, $2 = src_dir
    # Używa `grep -E | wc -l` zamiast `grep -cE`, bo grep -c exit 1 na zero
    # match w połączeniu z `set -o pipefail` daje multi-line garbage w $().
    local pattern="$1"; local src_dir="$2"; local total=0; local n
    for f in $(find "$src_dir" -name "*.rs" 2>/dev/null); do
        local boundary
        boundary=$(grep -nE "^#\[cfg\(test\)\]|^mod tests\b" "$f" 2>/dev/null | head -1 | cut -d: -f1)
        if [ -z "$boundary" ]; then
            boundary=$(($(wc -l < "$f") + 1))
        fi
        # Linie < boundary (production scope) z pominięciem doctest comments
        n=$(awk -v b="$boundary" 'NR < b && $0 !~ /^[[:space:]]*\/\/\//' "$f" 2>/dev/null \
            | grep -E "$pattern" 2>/dev/null | wc -l | tr -d ' ')
        total=$((total + ${n:-0}))
    done
    echo "$total"
}
for c in $CRATES; do
    SRC_DIR="crates/$c/src"
    [ -d "$SRC_DIR" ] || continue
    UNWRAP=$(count_in_production "\.unwrap\(\)" "$SRC_DIR")
    EXPECT=$(count_in_production "\.expect\(" "$SRC_DIR")
    PANIC=$(count_in_production "panic!\(" "$SRC_DIR")
    # Budżety per decisions.md (2026-05-03 unwrap banned). Core/daemon strict,
    # frontend mniej rygorystyczne. Doctesty i mod tests już odfiltrowane.
    case "$c" in
        core|daemon) BUDGET="0/0/0 (strict)" ;;
        client) BUDGET="≤2/≤2/0" ;;
        cli|tui|gui) BUDGET="≤5/≤5/≤1" ;;
        *) BUDGET="?" ;;
    esac
    add "| $c | $UNWRAP | $EXPECT | $PANIC | $BUDGET |"
done
add ""

# === 5. TODO / FIXME / HACK / XXX =============================================
add "### TODO / FIXME / HACK / XXX w kodzie"
TODO_COUNT=$(grep -rEo "(TODO|FIXME|HACK|XXX)" --include="*.rs" --include="*.slint" --include="*.toml" crates/ 2>/dev/null | wc -l | tr -d ' ')
TODO_FILES=$(grep -rElE "(TODO|FIXME|HACK|XXX)" --include="*.rs" --include="*.slint" crates/ 2>/dev/null | wc -l | tr -d ' ')
add "- łącznie wystąpień: **$TODO_COUNT** (w $TODO_FILES plikach)"
add ""

# === 6. unsafe blocks =========================================================
add "### \`unsafe\` blocks"
add ""
add "| Crate | unsafe blocks |"
add "|-------|---------------|"
for c in $CRATES; do
    SRC_DIR="crates/$c/src"
    [ -d "$SRC_DIR" ] || continue
    UNSAFE=$(grep -rEo "\bunsafe\b *(\{|fn|impl)" "$SRC_DIR" 2>/dev/null | wc -l | tr -d ' ')
    add "| $c | $UNSAFE |"
done
add ""
add "_Każdy unsafe wymaga SAFETY: komentarza tuż obok ([rules/audit.md](rules/audit.md) §Bezpieczeństwo)._"
add ""

# === 7. Testy ==================================================================
# Doctest ratchet — minima ustawione w 2026-05-23 follow-up audit po
# dorobieniu doctestów w client crate. Każde minimum to **podłoga**: nigdy
# nie wolno zejść poniżej tej wartości bez świadomej decyzji właściciela
# (AGENT.md §II wymaga `# Examples` na publicznym API). Gdy stan rośnie,
# zaktualizuj te liczby w **górę** — raz osiągnięty poziom jest podłogą,
# nie sufitem (audit.md "Krok 4 Ratchet").
# Floors set to current observed values after the 2026-05-23 follow-up
# (indent-aware doctest counter — the previous floor numbers undercounted
# anything inside `impl`/`mod` blocks because the regex used `^/// ...`
# instead of `^\s*/// ...`).
DOCTEST_MIN_core=93
DOCTEST_MIN_daemon=85
DOCTEST_MIN_client=14
DOCTEST_MIN_cli=4
DOCTEST_MIN_tui=30
DOCTEST_MIN_gui=0  # gui to Slint UI; doctesty na .slint nie istnieją, na .rs sensowne tylko dla logic
add "### Testy"
add ""
add "| Crate | #[test] | tests/ | doctest | min ratchet |"
add "|-------|---------|--------|---------|-------------|"
RATCHET_BREACH=""
for c in $CRATES; do
    SRC_DIR="crates/$c/src"
    [ -d "$SRC_DIR" ] || continue
    TESTS_INLINE=$(grep -rE "^\s*#\[test\]|^\s*#\[tokio::test\]" "$SRC_DIR" 2>/dev/null | wc -l | tr -d ' ')
    TESTS_DIR="crates/$c/tests"
    if [ -d "$TESTS_DIR" ]; then
        TESTS_FILES=$(find "$TESTS_DIR" -name "*.rs" 2>/dev/null | wc -l | tr -d ' ')
    else
        TESTS_FILES=0
    fi
    # Doctest = '''rust' lub '''no_run' lub '''ignore' w docs.
    # Indent-aware: doctesty wewnątrz `impl Foo {` / `mod tests {` mają
    # wcięcie (4 spacje), więc anchor musi tolerować leading whitespace.
    DOCTESTS=$(grep -rE "^\s*/// \`\`\`($|rust|no_run|ignore|compile_fail)" "$SRC_DIR" 2>/dev/null | wc -l | tr -d ' ')
    MIN_VAR="DOCTEST_MIN_$c"
    MIN_VAL="${!MIN_VAR:-0}"
    if [ "$DOCTESTS" -lt "$MIN_VAL" ]; then
        STATUS="❌ <$MIN_VAL"
        RATCHET_BREACH+="$c "
    else
        STATUS="$MIN_VAL ✅"
    fi
    add "| $c | $TESTS_INLINE | $TESTS_FILES | $DOCTESTS | $STATUS |"
done
add ""
if [ -n "$RATCHET_BREACH" ]; then
    add "**⚠ RATCHET BREACH (doctest)**: $RATCHET_BREACH — patrz \`rules/audit.md\` Krok 4."
    add ""
fi

# === 8. Swallowed errors heurystyki ==========================================
add "### Swallowed errors (heurystyka — wymagają weryfikacji greppem)"
SWALLOW_PATTERN='let _ = |Err\(_\) =>|if let Err\(_\)'
SWALLOW=$(grep -rEo "let _ =|Err\(_\) =>|if let Err\(_\)" --include="*.rs" crates/ 2>/dev/null | wc -l | tr -d ' ')
add "- heurystyczna liczba: $SWALLOW. Każdy wpis → przejrzeć ręcznie (część bywa legalna: \`let _ = drop(...)\`)."
add ""

# === 9. Dependency / dead code (cargo-udeps jeśli jest) =======================
add "### Dead code / nieużywane deps"
if command -v cargo-udeps >/dev/null 2>&1; then
    UDEPS_TMP="$(mktemp)"
    if cargo +nightly udeps --workspace --all-features >"$UDEPS_TMP" 2>&1; then
        UDEPS_UNUSED=$(grep -c "unused" "$UDEPS_TMP" || echo 0)
        add "- cargo-udeps: $UDEPS_UNUSED unused (output w \`$UDEPS_TMP\` — przejrzyj)."
    else
        add "- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)"
    fi
    rm -f "$UDEPS_TMP"
else
    add "- cargo-udeps: ⚠️  niezainstalowane (\`cargo install cargo-udeps --locked\` żeby aktywować)."
fi
add ""

# === 10. decisions.md sanity =================================================
add "### Rejestr decyzji (\`.claude/rules/decisions.md\`)"
if [ -f .claude/rules/decisions.md ]; then
    ACTIVE_COUNT=$(awk '/^## Decyzje aktywne/,/^## Decyzje wycofane/' .claude/rules/decisions.md | grep -cE "^### [0-9]{4}-[0-9]{2}-[0-9]{2}")
    WYCOFANE=$(awk '/^## Decyzje wycofane/,0' .claude/rules/decisions.md | grep -cE "^### [0-9]{4}-[0-9]{2}-[0-9]{2}")
    add "- decyzje aktywne: $ACTIVE_COUNT"
    add "- decyzje wycofane: $WYCOFANE"
else
    add "- decisions.md: **BRAK** — zaadoptuj wg \`claude-toolkit/ADOPT.md\`"
fi
add ""

# === 11. Frontend nie omija client (decyzja architektury) =====================
add "### Inwariant: frontendy używają \`bootcontrol-client\`, nie \`bootcontrol-daemon\`"
VIOLATIONS=""
for f in cli tui gui; do
    if [ -f "crates/$f/Cargo.toml" ]; then
        if grep -qE "^bootcontrol-daemon\b|^daemon\b.*path.*daemon" "crates/$f/Cargo.toml" 2>/dev/null; then
            VIOLATIONS+="crates/$f/Cargo.toml "
        fi
    fi
done
if [ -z "$VIOLATIONS" ]; then
    add "- ✅ żaden frontend nie importuje daemon"
else
    add "- ❌ VIOLATION: $VIOLATIONS (patrz \`rules/decisions.md\` 2026-05-03 \"Frontendy nie omijają client\")"
fi
add ""

# === 11b. Regression guards — zamknięte audyty z 2026-05-23 ==================
# Każda pozycja: greppem wykrywa dokładny pattern który był naprawiony.
# Wpadka = regresja → audyt P0/P1 odbity.
add "### Regression guards (zamknięte audyty)"
GUARD_FAILS=""

# P0.1 — żaden authorize_with_polkit nie może być wywołany bez `actions::`.
NAKED_POLKIT=$(grep -rEn "authorize_with_polkit\(caller_uid\)\b[^,]" crates/daemon/src 2>/dev/null \
    | grep -v "#\[" | wc -l | tr -d ' ')
if [ "$NAKED_POLKIT" -eq 0 ]; then
    add "- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument"
else
    add "- P0.1 per-intent Polkit: ❌ $NAKED_POLKIT naked \`authorize_with_polkit(uid)\` — regresja!"
    GUARD_FAILS+="P0.1 "
fi

# P0.2 — rpm_ostree::kargs_append musi wywołać validate_kernel_param przed kargs_read.
if [ -f crates/daemon/src/rpm_ostree.rs ]; then
    if awk '/pub fn kargs_append/,/^}/' crates/daemon/src/rpm_ostree.rs \
            | grep -q "validate_kernel_param"; then
        add "- P0.2 sanitize rpm-ostree: ✅ \`kargs_append\` waliduje param"
    else
        add "- P0.2 sanitize rpm-ostree: ❌ \`kargs_append\` bez \`validate_kernel_param\` — regresja!"
        GUARD_FAILS+="P0.2 "
    fi
fi

# P1.1 — single blacklist: definicja KERNEL_CMDLINE_BLACKLIST tylko w core::security.
BLACKLIST_DEFS=$(grep -rEln "(const|let)\s+(BLACKLISTED_PATTERNS|BLACKLISTED_PARAMS|KERNEL_CMDLINE_BLACKLIST)\s*:\s*&\[" crates/ 2>/dev/null \
    | wc -l | tr -d ' ')
if [ "$BLACKLIST_DEFS" -le 1 ]; then
    add "- P1.1 single blacklist: ✅ $BLACKLIST_DEFS definicja (\`KERNEL_CMDLINE_BLACKLIST\` w \`core::security\`)"
else
    add "- P1.1 single blacklist: ❌ $BLACKLIST_DEFS definicji blacklisty — regresja, konsolidacja zniknęła!"
    GUARD_FAILS+="P1.1 "
fi

# P1.2 — policy_check zaserwowany w main.rs przed serve_at.
if grep -q "validate_policy_file" crates/daemon/src/main.rs 2>/dev/null; then
    add "- P1.2 startup policy validation: ✅ \`validate_policy_file\` w main.rs"
else
    add "- P1.2 startup policy validation: ❌ brak \`validate_policy_file\` w main.rs — regresja!"
    GUARD_FAILS+="P1.2 "
fi

# P2.1 — audit.sh `count_in_production` filtruje mod tests/doctesty.
if grep -q "count_in_production" "$REPO_ROOT/.claude/audit.sh" 2>/dev/null; then
    add "- P2.1 audit.sh filter: ✅ \`count_in_production\` aktywne"
else
    add "- P2.1 audit.sh filter: ❌ brak helper'a filtrującego — regresja!"
    GUARD_FAILS+="P2.1 "
fi

# P2.2 — fake \"org.bootcontrol.test\" nie wraca jako literal poza policy/actions.
FAKE_TEST_ACTION=$(grep -rEn '"org\.bootcontrol\.test"' crates/ 2>/dev/null | wc -l | tr -d ' ')
if [ "$FAKE_TEST_ACTION" -eq 0 ]; then
    add "- P2.2 no fake polkit action ID: ✅ \`\"org.bootcontrol.test\"\` nie istnieje"
else
    add "- P2.2 no fake polkit action ID: ❌ $FAKE_TEST_ACTION wystąpień — regresja!"
    GUARD_FAILS+="P2.2 "
fi

if [ -n "$GUARD_FAILS" ]; then
    add ""
    add "**⚠ REGRESJA**: $GUARD_FAILS — patrz \`audit-log.md\` sekcje zamkniętych audytów; nie ignoruj."
fi
add ""

# === 11c. Faza A signal — informational =====================================
# Faza A jest poza ROADMAP (patrz backlog.md P2). Każdy nowy commit z
# "Faza A PR #N" w treści to sygnał że ROADMAP wymaga back-fill'a.
add "### Faza A stream signal (informational)"
if command -v git >/dev/null 2>&1 && [ -d .git ]; then
    FAZAA_COUNT=$(git log --all --format='%s' 2>/dev/null | grep -cE "Faza A PR" || true)
    FAZAA_LATEST=$(git log --all --format='%h %ad %s' --date=short 2>/dev/null \
        | grep -E "Faza A PR" | head -1 || true)
    add "- commitów z \"Faza A PR\": $FAZAA_COUNT"
    if [ -n "$FAZAA_LATEST" ]; then
        add "- najnowszy: \`$FAZAA_LATEST\`"
    fi
    add "- jeśli pojawi się PR powyżej tych zarejestrowanych w \`ROADMAP.md\` \"Out-of-roadmap streams\" → back-fill (backlog P2)."
else
    add "- (git niedostępny — pomiń)"
fi
add ""

# === 12. Pre-push hook =======================================================
add "### Hooki gitowe"
if [ -x .githooks/pre-push ]; then
    add "- pre-push: ✅ obecny i executable"
else
    add "- pre-push: ❌ brak / nie-executable (\`./scripts/install-hooks.sh\` żeby aktywować)"
fi
add ""

# === 13. Data ostatniego audytu =============================================
add "### Trend audytów"
if [ -f "$LOG" ]; then
    LAST_DATE=$(grep -m1 -E "^## Audyt [0-9]{4}-[0-9]{2}-[0-9]{2}" "$LOG" | sed 's/^## Audyt //' || echo "BRAK")
    TOTAL_AUDITS=$(grep -cE "^## Audyt [0-9]{4}-[0-9]{2}-[0-9]{2}" "$LOG")
    add "- poprzedni audyt: $LAST_DATE"
    add "- łącznie audytów: $TOTAL_AUDITS (włącznie z tym)"
else
    add "- audit-log.md: BRAK — to pierwszy audyt"
fi
add ""

# === 14. Skille / toolkit refresh check =====================================
add "### Skille (\`.claude/skills/\`)"
if [ -d .claude/skills ]; then
    SKILL_COUNT=$(find .claude/skills -name SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    add "- skills lokalnie: $SKILL_COUNT"
else
    add "- .claude/skills/: BRAK"
fi
TOOLKIT="$HOME/DevProjects/claude-toolkit"
if [ -d "$TOOLKIT" ]; then
    TOOLKIT_SKILLS=$(find "$TOOLKIT/skills" -name SKILL.md 2>/dev/null | wc -l | tr -d ' ')
    add "- skills w toolkit: $TOOLKIT_SKILLS — refresh: \`cd $TOOLKIT && git pull\`, potem skopiuj do \`.claude/skills/\`"
else
    add "- toolkit: niezlokalizowany (oczekiwany: \`$TOOLKIT\`)"
fi
add ""

# === FOOTER ==================================================================
add "**Do przeglądu agentem** (warstwa głęboka — patrz \`rules/audit.md\` Krok 2):"
add "bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista"
add "P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2."
add ""

# === Output i append ==========================================================
echo "$section"

# Append na górę audit-log.md (po headerze).
if [ ! -f "$LOG" ]; then
    printf '# Audit Log — BootControl\n\nHistoria cotygodniowych audytów. Najnowszy na górze.\nPełna procedura: [`.claude/rules/audit.md`](rules/audit.md).\n\n---\n\n' > "$LOG"
fi

tmp="$(mktemp)"
# Zachowaj header (do *pierwszej* `---`), potem nową sekcję, potem stare wpisy.
# Kluczowe: tylko **pierwsza** `^---$` to koniec headera; każda kolejna to
# wewnętrzny separator sekcji audytu — nie wolno go traktować jak header end.
sed -n '1,/^---$/p' "$LOG" > "$tmp"
echo "" >> "$tmp"
echo "$section" >> "$tmp"
# Wszystko PO pierwszej `---` w oryginalnym LOG (stare audyty).
awk 'BEGIN{after_first=0}
     after_first { print }
     !after_first && /^---$/ { after_first=1 }' "$LOG" >> "$tmp"
mv "$tmp" "$LOG"
echo ""
echo "Zapisano sekcję do: $LOG"
