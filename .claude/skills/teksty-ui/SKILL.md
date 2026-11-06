---
name: teksty-ui
description: Przegląd tekstów interfejsu i danych, które w nim udają prawdziwe — usuwa marketingową nowomowę, prostuje etykiety przycisków, wyłapuje liczby, opinie i logotypy bez źródła, a na koniec wypisuje meldunek wszystkiego, co jest zmyślone albo zaślepką. Użyj po zbudowaniu lub zmianie widoku, przed oddaniem UI do review, albo gdy właściciel mówi że w interfejsie jest pełno AI-izmów. EN — reviews interface copy and data that pretends to be real; strips marketing filler, fixes button labels, catches numbers, testimonials and logos without a source, and ends with a report listing everything fabricated or placeholder.
compatibility: Wymaga bash i git oraz skryptu ui-copy-lint.sh z toolkitu. Projekt bez katalogu i pliku zwolnień dostaje katalog wbudowany — działa, ale pierwsze trafienie w termin dziedzinowy trzeba wtedy zwolnić ręcznie. Zaprojektowane dla agentów czytających SKILL.md.
metadata:
  author: claude-toolkit
  version: "2026.11.07"
---

# Teksty w interfejsie

Model proszony o widok produkuje dwie rzeczy naraz: prozę, która nic nie
znaczy, i **dane, które wyglądają jak prawdziwe**. Pierwsze widać. Drugie
przechodzi przez review, bo „99,9% uptime" i cytat klienta z imieniem
i stanowiskiem wyglądają jak coś, co ktoś kiedyś ustalił.

Ten skill przechodzi widok pod oba kąty i kończy **meldunkiem** — listą
wszystkiego, czego nie da się wskazać w źródle. Bez meldunku fikcja wchodzi do
produkcji, bo nikt nie wie, czego szukać.

Reguły żyją w `ui-copy.md`. Ten skill ich nie powtarza — wykonuje.

## Krok 1 — Bramka

```sh
bash <toolkit>/templates/ui-copy-lint.sh .
```

| Sygnał | Co z tym robisz |
|---|---|
| `UI_COPY_BLOCKING` | waga 3 — usuwasz w Kroku 2, bez pytania |
| `UI_COPY_REVIEW` | waga 2 i 1 — czytasz zdanie, decydujesz |
| `UI_COPY_NUMBERS` | liczba bez źródła — Krok 3 |
| `UI_COPY_ADDRESSES` | adres spoza pul zarezerwowanych |
| `UI_COPY_PUNCT` | wykrzykniki i emoji |
| `UI_COPY_EXEMPT` | trafienia zwolnione dziedzinowo — sprawdź, czy nie rośnie |

Gdy skrypt melduje brak pliku zwolnień, **powiedz o tym właścicielowi od razu**.
Bramka bez zwolnień w projekcie z własnym słownictwem zgłosi kilkadziesiąt
fałszywych trafień i zostanie wyłączona przy pierwszym przeglądzie — a wyłączona
bramka nie chroni przed niczym.

## Krok 2 — Teksty

Waga 3 znika bezwarunkowo. Waga 2 zostaje tylko wtedy, gdy w **tym samym
zdaniu** stoi konkret, który ją uzasadnia.

Przy każdej poprawce zadaj trzy pytania:

1. **Czy to zdanie zniknęłoby bez straty informacji?** Jeśli tak, usuń je
   zamiast przepisywać. Nowomowa przepisana na inną nowomowę to ta sama praca
   drugi raz.
2. **Czy przycisk mówi, co się stanie po kliknięciu?** Etykieta to czasownik
   plus obiekt. „Utwórz konto", nie „Rozpocznij".
3. **Czy nazwa czynności obiecuje mechanizm, którego nie ma?** To najdroższy
   błąd w tej klasie — patrz Krok 4.

Nagłówek mówi, **co to jest**, nie **jakie to wspaniałe**. Pusty stan mówi, co
zrobić. Błąd mówi, co się stało i co zrobić.

Skracanie długiej treści robisz **klamrą wizualną**, nigdy cięciem łańcucha —
ucięty tekst znika z kopiowania i z wyszukiwania w stronie.

## Krok 3 — Dane, które udają prawdziwe

Dla każdej liczby, cytatu, logotypu, certyfikatu, ceny i terminu w widoku
odpowiedz na jedno pytanie: **gdzie to jest w repozytorium?**

| Odpowiedź | Co robisz |
|---|---|
| plik, endpoint, konfiguracja | zostaje |
| „właściciel podał w rozmowie" | zostaje, ale zapisz źródło w komentarzu |
| nie ma nigdzie | **znacznik `[TODO: …]` albo zmienna**, nigdy liczba |

Dane demonstracyjne wyłącznie z pul zarezerwowanych — `example.com`,
`800-555-01xx`, `192.0.2.0/24`, `Jan Przykładowy`. I **widocznie oznaczone**
w interfejsie: baner „Dane demonstracyjne", seria wykresu nazwana „Przykładowe
dane", nigdy „Przychód Q3".

Sekcja, która wymaga dowodu, a dowodu nie ma — opinie, „zaufali nam",
statystyki — **nie powstaje**. Zostaw komentarz w kodzie zamiast wygenerowanej
atrapy: atrapa z awatarem i nazwiskiem to nie zaślepka, to fałszerstwo.

## Krok 4 — Dwa błędy, które wyglądają na kosmetykę

**Napis o postępie jest twierdzeniem o stanie systemu.** „Zapisuję…", „Uczę…",
„Trwa analiza" mówią, że coś się dzieje. Jeśli przy błędzie albo przekroczeniu
czasu nie ma ścieżki, która ten napis zdejmuje, komunikat kłamie tak samo jak
wymyślona liczba — i zostawia widok, który nie reaguje. Sprawdź, czy stan
„w trakcie" gaśnie w każdej ścieżce wyjścia, nie tylko w szczęśliwej.

**Nazwa czynności obiecuje mechanizm.** Etykieta „Ucz" przy operacji, która
tylko dopisuje regułę do pola, obiecuje uczenie maszynowe. Sprawdzian: *czy
użytkownik, czytając tę etykietę, oczekuje czegoś, czego system nie robi?*
Jeśli tak, zmiana idzie przez wszystkie warstwy naraz — napis, tytuł, opis
strony, nazwa modułu, nazwa funkcji — inaczej za miesiąc kod i interfejs mówią
co innego o tej samej rzeczy.

## Krok 5 — Meldunek

Obowiązkowy, także gdy jest pusty.

```text
UI_COPY_BLOCKING: <przed> → <po>
UI_COPY_REVIEW: <n> (<ile zostawiono i dlaczego>)
UI_FABRICATED: <n>
- <widok>: <co> → [TODO] | placeholder | dane demonstracyjne oznaczone
UI_LANG_MIX: <wstawki w obcym języku | brak>
EXEMPTIONS_ADDED: <słowo → ścieżka, powód | brak>
```

`UI_FABRICATED: 0` jest wynikiem i też się je wypisuje. Milczenie w tym miejscu
czyta się jako „sprawdzone", a zwykle znaczy „nie sprawdzałem".

## Egzekwowanie

**Blokują:**

- słowo z wagą 3 w tekście widocznym dla użytkownika, poza zwolnieniami;
- liczba, cytat, logotyp albo certyfikat, którego nie da się wskazać w źródle;
- dane demonstracyjne spoza pul zarezerwowanych;
- brak meldunku z Kroku 5.

**Raportują:**

- waga 2 i 1;
- wykrzykniki i emoji poza systemem designu;
- etykieta przycisku bez czasownika;
- rosnąca liczba zwolnień dziedzinowych — katalog jest wtedy źle dobrany do
  tego projektu i trzeba go poprawić, a nie dosypywać wyjątków.

## Sygnały maszynowe

| Sygnał | Znaczenie |
|---|---|
| `UI_COPY_BLOCKING` | niezerowe po przebiegu = skill nie skończył pracy |
| `UI_FABRICATED` | brak pola = widok nieprzejrzany, nie widok czysty |
| `UI_COPY_EXEMPT` | szybki wzrost = katalog nie pasuje do dziedziny projektu |
| `EXEMPTIONS_ADDED` | zwolnienie bez powodu jest nieodróżnialne od kapitulacji |

Powiązane: `ui-copy.md` (reguły), `naming-conventions.md` (leksykon
i jedno słowo na jedną rzecz), `design-system-foundations.md`,
`forms-and-validation.md`, `rules-as-gates.md`.
