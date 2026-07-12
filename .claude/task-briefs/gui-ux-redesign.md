# GUI — audyt UX na zrzutach + handoff redesignu

**Task-Id:** gui-ux-redesign
**Status:** zapisane — gotowe do startu (uprawnienie Screen Recording nadane 2026-07-12)
**Zadanie źródłowe:** sesja 2026-07-12 — właściciel zgłosił „katastrofę UX" GUI; wcześniejsza próba (Opus) nie przyniosła rezultatu.
**Powiązanie:** To zadanie wynikło z przeglądu projektu 2026-07-12, ponieważ statyczna analiza `.slint` wykazała, że problem UX ma techniczną przyczynę (niedokończona implementacja v2), a ocena wizualna była zablokowana brakiem uprawnienia Screen Recording — już nadanego.

---

## Diagnoza z sesji 2026-07-12 (ustalona, nie powtarzać analizy)

**Rdzeń problemu to nie estetyka, tylko niedokończone v2:**

1. [`crates/gui/ui/pages/bootloader.slint`](../../crates/gui/ui/pages/bootloader.slint) (39 linii) — **jawny placeholder**: karta „Coming in PR 7", odsyła użytkownika na inną stronę. Komentarz w pliku: realne kontrolki wymagają **typed-getter D-Bus methods po stronie daemona, które nie istnieją** — implementacja §3.3 ma zależność daemonową.
2. [`crates/gui/ui/pages/boot_entries.slint`](../../crates/gui/ui/pages/boot_entries.slint) (73 linie) — **UX v1**: płaska tabela key=value, per-row „Save", sztywne `width: 180px`/`height: 60px`. Komentarz: „PR 3 is a faithful port"; Inspector/reorder/staged-changes z v2 §3.2 „lands in PR 6" — nigdy nie wylądowały.
3. Komponenty v2 **istnieją** (`crates/gui/ui/components/`: action footer, confirmation sheet, diff preview, preflight card, onboarding…), ale główne strony ich nie używają. Granite (Phase C/D) dał tylko nową farbę na IA v1.
4. ROADMAP Phase 3.5 „PR 1–4, 6, 7, 7b ✅ Done" (mega-commit `64e1001`) = **naddeklaracja** — sprostowanie ROADMAP to element tego zadania (kontynuacja doc-honesty; wzór: commity `00a9a9c`, `9a371ce`).
5. Znane zepsucie (osobny backlog P1, poza zakresem tego zadania): panel Secure Boot woła `sign_and_enroll_uki("")`/`backup_nvram("")` — przyciski zawsze failują.

## Pipeline zrzutów ekranu (sprawdzony 2026-07-12 do momentu uprawnienia)

```bash
cargo build -p bootcontrol-gui                      # buduje się czysto
BOOTCONTROL_DEMO=1 ./target/debug/bootcontrol-gui & # okno 1100×812, działa na macOS
# wariant high-contrast:
BOOTCONTROL_HIGH_CONTRAST=1 BOOTCONTROL_DEMO=1 ./target/debug/bootcontrol-gui &
```

Window ID przez CoreGraphics (Swift, bez dodatkowych narzędzi) — zapisz jako
`findwin.swift` w scratchpadzie i uruchom `swift findwin.swift`:

```swift
import CoreGraphics
import Foundation
let opts: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
if let list = CGWindowListCopyWindowInfo(opts, kCGNullWindowID) as? [[String: Any]] {
  for w in list {
    let owner = (w[kCGWindowOwnerName as String] as? String ?? "").lowercased()
    if owner.contains("bootcontrol") {
      print(w[kCGWindowNumber as String] as? Int ?? 0)
    }
  }
}
```

Zrzut: `screencapture -x -l<ID> out.png`. **Pułapki:**
- Screen Recording dla VSCode: **nadane 2026-07-12** — jeśli `could not create image`, VSCode nie został zrestartowany po nadaniu.
- Nawigacja między stronami (sidebar: Overview / Boot Entries / Bootloader / Secure Boot / Snapshots / Logs + Settings): klik/klawisze przez `osascript` (System Events) wymagają **osobnego uprawnienia Accessibility**. Jeśli brak — nie walcz: poproś właściciela, by przeklikał strony ręcznie, a ty rób zrzut po każdej zmianie.
- Confirmation Sheet: otwiera się z „⟳ Rebuild GRUB" na Boot Entries (wg CLAUDE_DESIGN_BRIEF §2).
- Zrzutów **nie commitować do repo** (binaria) — trzymać w scratchpadzie; do handoffu wizualnego pchać przez DesignSync.

## Cel

Pełny audyt UX na materiale wizualnym + decyzja właściciela o zakresie naprawy + (opcjonalnie) handoff wizualny do Claude Design.

## Zakres — kolejne kroki

1. **Zrzuty:** 7 stron + Confirmation Sheet + high-contrast (9 plików).
2. **Audyt heurystyczny:** zgodność ze spec v2 (`docs/GUI_V2_SPEC_v2.md` §3, §8, §10) + heurystyki Nielsena; findingi P0/P1/P2 z odwołaniem do zrzutu i pliku `.slint`. Raport → `.claude/history/2026-MM-DD-gui-ux-audit.md` (markdown, bez binariów).
3. **Sprostowanie ROADMAP Phase 3.5** (osobny mały commit doc-honesty).
4. **Przedstawienie właścicielowi dwóch torów** (rekomendacja + szacunek pracy):
   - **Tor A (inżynierski):** implementacja §3.2 (Boot Entries: icon-list + Inspector + reorder + staged changes/action footer) i §3.3 (Bootloader: typed settings) — uwaga na zależność: §3.3 wymaga typed getters w daemonie.
   - **Tor B (wizualny):** aktualizacja [`docs/CLAUDE_DESIGN_BRIEF.md`](../../docs/CLAUDE_DESIGN_BRIEF.md) — **zdjąć założenie „IA locked, tylko wygląd"** (nieaktualne po diagnozie) i dopisać ustalenia z tej sesji; następnie handoff przez narzędzie **DesignSync** (dostępne w Claude Code, spięte z claude.ai/design: `list_projects` → wybór/`create_project` → `finalize_plan` → `write_files` z podglądami/zrzutami). Tor B może iść równolegle z A.
5. **Implementacja dopiero po zielonym świetle właściciela** (zapis ≠ zgoda).

## Poza zakresem

- Naprawa przycisków Secure Boot (backlog P1, osobny node).
- Praca nad CLI (właściciel prowadzi ją oddzielnie).
- Bramki release-readiness G2–G7 (osobny brief).

## Acceptance criteria

- 9 zrzutów istnieje i został na nich oparty raport audytu w `history/`.
- ROADMAP Phase 3.5 sprostowany commitem.
- Właściciel dostał porównanie Tor A / Tor B z rekomendacją i podjął decyzję (zapisaną w tym briefie lub `decisions.md`).
- Jeśli wybrano Tor B: CLAUDE_DESIGN_BRIEF zaktualizowany i materiał wypchnięty przez DesignSync.

## Zależności i kolejność merge

- Czy można zacząć teraz: **tak** (uprawnienie nadane; kroki 1–4 są read-only poza dwoma małymi commitami docs).
- Branch base: `main`; proponowane gałęzie: `docs/roadmap-phase35-honesty`, później `feat/gui-v2-boot-entries` (Tor A) — jedna gałąź per rezultat.
- Kolejność: zrzuty → audyt → sprostowanie ROADMAP → decyzja właściciela → dopiero implementacja.

## Prompt rozpoczynający nową rozmowę

> Przeczytaj `CLAUDE.md`, potem `.claude/task-briefs/gui-ux-redesign.md` (handoff — diagnoza, pipeline zrzutów, pułapki uprawnień) oraz `docs/GUI_V2_SPEC_v2.md` §3.2/§3.3/§8 i `docs/CLAUDE_DESIGN_BRIEF.md`. Nie powtarzaj analizy przyczyn — diagnoza w briefie jest aktualna. Zacznij od kroku 1 (zrzuty wg pipeline'u z briefu), potem audyt (krok 2) i przedstaw mi tory A/B (krok 4). Niczego nie implementuj bez mojej zgody.

## Znormalizowana intencja do commitów

Intent: GUI UX audit and redesign handoff — visual evidence, heuristic
findings, ROADMAP Phase 3.5 correction, and the owner's A/B scope decision,
following the 2026-07-12 diagnosis (v2 pages never implemented).
Task-Ref: gui-ux-redesign
