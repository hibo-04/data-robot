# Glossar der Produktwörter

Stand: 2026-09-16 (festgelegt mit P0.2). Gilt für die Kataloge `app/locales/*.json`, Hilfetexte, E-Mails und die Docs, sobald sie Nutzer-Copy zitieren.

Regel aus `docs/web-app-construct.md`: Ein Produktbegriff ist **entweder fest übersetzt oder bewusst unübersetzt** — nie mal so, mal so. Zwei Wörter für eine Sache gibt es nicht.

Konventionen:

- Englischer Katalog in **en-US-Schreibweise** (`organization`, `analyze`).
- Engine-Ausgaben (Status, Reason-Codes, Rollen-Codes) sind sprachneutrale **Codes**. Die UI zeigt das übersetzte Label aus dem Katalog, nie den Code.
- Chrome-Vokabular der Docs (Left Rail, Akzentleiste, Seitenleiste, Canvas, Inspektor) ist intern. Es taucht nicht als Wort in der UI auf.

## Unübersetzt (in beiden Sprachen gleich)

| Begriff | Warum |
| --- | --- |
| **Octa** | Arbeitstitel, Eigenname |
| **Overlay** | Kein deutsches Wort trägt „Nutzerentscheidungen über dem Analyseergebnis“ |
| **Pack** (Dictionary-Pack) | Produktkonzept mit Katalog-Namen (`generic`, `commerce`) |
| **KPI** | Fachwort in beiden Sprachen; „Kennzahl“ wird nicht zusätzlich benutzt |
| **Report / Reports** | Fläche *und* einzelnes Objekt; im Deutschen etabliertes Lehnwort, „Bericht“ wird nicht benutzt |
| **Explore** | Name der Fläche; als Verb im Fließtext trotzdem „erkunden“ / „explore“ |
| **Measure, Dimension** | Semantische Rollen, BI-Jargon in beiden Sprachen |

## Fest übersetzt

| Deutsch | English | Anmerkung |
| --- | --- | --- |
| Quelle | source | nicht „Datenquelle“ / „data source“ als zweites Wort |
| Beziehung | relationship | Docs sagen intern oft „Kante“; UI nie |
| numerische Identität | numeric identity | immer mit Adjektiv, damit keine Verwechslung mit Login-Identität |
| Modell | model | die Fläche; „Datenmodell“ nur wenn der Kontext es braucht |
| Übersicht | overview | die Fläche |
| Analyse, Analyse-Lauf | analysis, analysis run | |
| Vorschlag | suggestion | alles, was die Engine anbietet |
| Konfidenz | confidence | |
| Evidenz | evidence | |
| Begründung | reason | übersetzter Reason-Code |
| Beispielquelle | sample source | Fixtures in der App |
| Verbindung (herstellen) | connection (connect) | |
| Organisation | organization | Mandant; „Mandant“/„tenant“ nur in Docs |
| Mitglied | member | |
| Rolle | role | |
| Einladung | invitation | |
| Sprache | language | UI-Sprache |
| Zeitzone: Anzeige- / Reporting- | time zone: display / reporting | zwei Uhren, immer mit Attribut |
| Zeit | time | semantische Rolle |
| ID | identifier | semantische Rolle; UI-Kurzform „ID“ in beiden Sprachen |
| Fremdschlüssel | foreign key | semantische Rolle; Abkürzung „FK“ nur in Evidenz |
| Export | export | |
| Suche | search | |

## Zustände des Overlays

Code bleibt Code (`accepted`, `rejected`, `edited`, `pending`); die UI zeigt:

| Code | Deutsch | English |
| --- | --- | --- |
| `accepted` | akzeptiert | accepted |
| `rejected` | abgelehnt | rejected |
| `edited` | bearbeitet | edited |
| `pending` | offen | pending |

Akzentleiste-Beispiel: DE „3 offen · 1 akzeptiert“, EN „3 pending · 1 accepted“. Nicht „1 accepted“ in einer deutschen Oberfläche.

Aktionen: akzeptieren / accept, ablehnen / reject, bearbeiten / edit, veröffentlichen / publish (falls ein expliziter Publish-Schritt kommt, P4.1).

## Offen (blockiert das genannte Paket)

| Frage | Spätestens vor |
| --- | --- |
| **Anrede im Deutschen: Sie oder du?** Empfehlung: **Sie** (B2B, Sicherheitskontext, Design-Partner aus Unternehmen). Bisherige Copy vermeidet die Anrede. | P1.1 (Signup-Copy) |
| Rollen-Labels (`owner`, `admin`, `analyst`, `viewer`, `billing`): Codes stehen, deutsche Labels nicht („Eigentümer“ vs. „Owner“) | P1.4 |
| Genderform in Rollen und Mitgliederlisten | P1.4 |

## Pflege

Neuer Produktbegriff → Zeile hier, dann Katalog. Wer einen Begriff umbenennt, ändert Glossar und alle Kataloge im selben Change (`.cursor/rules/i18n-locales.mdc`).
