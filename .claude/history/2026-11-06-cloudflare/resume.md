# Wznowienie Cloudflare security-audit — BootControl

Ten checkpoint ma status **INCOMPLETE**, nie potwierdza bezpieczeństwa projektu.
`findings.json = []` oznacza brak zakończonego huntingu i niezależnej walidacji.
Oceniany kod: `bd28f2d0ce55b5a869c097e60429a41bf4240e63`, clean snapshot.
Profil standard, brak ścisłego budżetu całego runu; poprzednie limity pojedynczego
wywołania wynosiły 4 USD/240s. Zachowaj niezależność ról i rezerwę walidacji.

## Dokładny stan faz

| Faza | Stan |
|---|---|
| 1a produkt/stos | zakończona: recon/recon-a/facts.json |
| 1b tożsamość/uprawnienia | pierwsza próba BLOCKED; zakończone powtórzenie recon/recon-b-r2/facts.json |
| 1c wejścia/transformacje/sinki | zakończona: recon/recon-c/facts.json |
| 1d wykonanie i widoczność deploymentu | NIEURUCHOMIONA |
| 2 hunting | NIEURUCHOMIONY: 23 jednostki deferred, 8 planowanych grup; żaden hunter ani krytyk nie wystartował |
| 3 walidacja kandydatów | NIEURUCHOMIONA |
| 4 wynik maszynowy | puste findings i ledger poprawne strukturalnie; brak pipeline'u decyzji o kandydatach |
| 5 świeża weryfikacja rekordów | NIEURUCHOMIONA |
| 6 raport | wyłącznie ten jawny nieukończony checkpoint |

## Uruchomienie zainstalowanego skilla

Najpierw potrzebna jest rzeczywista zgoda właściciela na przetwarzanie tego
publicznego snapshotu przez Claude/Anthropic (`claude-fable-5-1`). Publiczność
repo udokumentowano osobno, lecz nie zastępuje odpowiedzi na oczekujące pytanie.
Nie zmieniaj transportu ani dostawcy, aby obejść odmowę. Bez zgody można czytać
lokalne źródła i ten checkpoint; nie uruchamiać kolejnych wywołań zewnętrznych.

Po zgodzie w zaufanej sesji agenta otwórz zainstalowany
`.claude/skills/security-audit/SKILL.md` (lub `skills/security-audit/SKILL.md`
w sprawdzonym klonie toolkitu), a następnie przekaż brief:

```text
Użyj security-audit w pełnym trybie, profil standard. Target to clean BootControl
bd28f2d0ce55b5a869c097e60429a41bf4240e63. Przeczytaj publiczny checkpoint:
run-metadata.json, architecture.md, coverage-ledger.json, findings.json oraz fakty
recon. To NIE jest zakończony audyt ani źródło potwierdzonych luk.
Najpierw wykonaj brakujące recon-d. Zaktualizuj architekturę i plan; przy tym
samym kodzie odroczone, nigdy nieprzydzielone jednostki wracają do planned.
Rozpocznij od 23 jednostek i dodaj nowe jednostki wynikające z recon/krytyków.
Użyj dokładnych bloków metody, natychmiastowego krytyka po każdej fali
i osobnego końcowego krytyka. Każdy kandydat otrzymuje świeżą
walidację, a każdy zachowany confirmed/needs_validation osobną świeżą weryfikację.
Odróżniaj source facts, potwierdzone lokalne wyniki i potrzeby walidacji.
Parent jest jedynym autorem wspólnych artefaktów. Role badawcze mają technicznie
wyłącznie odczyt. Nie wykonuj kodu celu na hoście ani nie kontaktuj produkcji.
Dokończ wszystkie raporty i oba walidatory albo jawnie zakończ jako incomplete.
```

Instrukcje, schemat i walidatory są w pełnym katalogu zainstalowanego skilla;
`method-manifest.json` identyfikuje metodę tego checkpointu. Przeczytaj też
RECONNAISSANCE, HUNTING, VALIDATION-AND-REPORTING oraz wybrane companion files.
Nie przenoś w ciemno starego statusu pokrycia na zmieniony kod. Jeśli kod się
zmienił, utwórz bieżące jednostki zgodnie z regułami prior-run.

Wybierz nowy katalog runu poza targetem. Zastąp `target: .` w lokalnych
metadanych rzeczywistym Linux checkoutem, bez publikowania ścieżek domowych.
Tymczasowe prywatne launchery, prompty i surowe transkrypty nie są częścią
checkpointu. Backend delegacji musi technicznie wymuszać Read/Glob/Grep;
sama nazwa roli lub instrukcja read-only nie jest granicą uprawnień.

Próby kodu celu wymagają kompletnego OS sandboxa: brak zewnętrznej sieci,
puste środowisko z allowlistą, target/toolchain read-only, zapisy tylko do
własnego scratch i limity CPU/RAM/PID/pliku/dysku/czasu. Rust/Linux, rzeczywisty
Polkit, uprawnienia instalacji i firmware nie zostały zweryfikowane. Wynik 10/10
w execution-scope.json dotyczy tylko metatestów wrappera z overlayami.
Nie zastępuj natywnej reprodukcji imitacją i nie nadaj severity needs_validation.

Na końcu sprawdź z katalogu skilla:

```bash
node validate-findings.cjs CHECKPOINT_DIR/findings.json
node validate-coverage-ledger.cjs CHECKPOINT_DIR/coverage-ledger.json
```

Podaj własny katalog wyników zamiast CHECKPOINT_DIR. Zielony walidator oznacza
poprawny kontrakt danych, nie bezpieczną aplikację ani zaliczone A4.
