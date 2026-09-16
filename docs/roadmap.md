# Roadmap: von Account bis fertige Analyse

Status: Plan. Umgesetzt: **P0.1** (App-Crate `app/`, Paket `octa-app`), **P0.2** (Locale-Stack, Kataloge DE/EN, Glossar). Alles Weitere offen.
Stand: 2026-09-15.

**Arbeitstitel: Octa.** Logo: verspielter Oktopus — siehe [`docs/branding/`](branding/).

Baut das **B2B-SaaS-Produkt** Stück für Stück. Nicht die heutige lokale PoC-UI erweitern.

Verwandte Dokumente:

| Dokument | Zuständig für |
| --- | --- |
| Dieses File | Reihenfolge der Arbeit, Pakete, Abhängigkeiten, was „fertig“ heißt |
| `docs/branding/` | Arbeitstitel, Logo, vorläufige Bildsprache |
| `docs/glossary.md` | Produktwörter: übersetzt oder unübersetzt, Zustands-Labels |
| `docs/web-app-construct.md` | Mandanten, Sicherheit, Identität, Locale-Stack, Betrieb |
| `docs/ui-ux-notes.md` | Chrome, Canvas, Mobile, Empty States, erste leichte Handlung |
| `docs/model-review.md` | Accept / Reject / Edit, Evidenz, Overlay als Wahrheit |
| `.cursor/rules/analytics-core-scope.mdc` | Core bleibt ohne Auth, Billing, HTTP, HTML, Mandantenfilter |

---

## Ausgangslage

Der **Analytics-Core** (`analytics`) existiert und ist der wertvolle Teil. CLI und Tests decken die Pipeline ab:

```text
Schema → Profil → Relationships → Identities → Semantik → KPIs → Query-Plan → Reports
```

Quellen heute: Fixtures (CSV) und optional PostgreSQL. Dictionary-Packs sind optional. Der Core arbeitet ohne LLM. Overlays (Accept/Reject) liegen in der lokalen UI unter `out/overlays/`.

Die lokale Review-Seite `web/` (`analytics-web`) ist ein **Wegwerf-Stand**. Layout, CSS und Seitenstruktur werden nicht weitergebaut. Der PoC darf lokal und ohne Login bleiben, bis die neue App denselben Review-Vertrag erfüllt.

Was **nicht** existiert und hier geplant wird: Accounts, Organisationen, Sessions, i18n, App-Chrome, Job-System, persistentes Overlay in einer Mandanten-DB, echte Connect-Sicherheit, Reports/Explore als Produktfläche.

Leitlinie aus dem Konstrukt: nichts im PoC vortäuschen — aber nichts bauen, das dem späteren Produkt widerspricht. Auth, Tenancy und Locale sitzen in der **Applikationsschicht**. Der Core bekommt weiterhin nur `DataSource` plus Overlay.

---

## Leitplanken

1. **Neue App, nicht `web/` aufblasen.** Neues Crate / Control Plane. `analytics` bleibt mandantenfrei.
2. **Organisation ist die Grenze.** Jedes Objekt trägt `org_id`. Isolation in DB (RLS), Cache, Jobs, Storage — fail-closed.
3. **DE und EN von Paket 1 an.** Jede sichtbare Copy in allen aktiven Locales im selben Change. Engine liefert Reason-Codes, UI übersetzt.
4. **Erste Sitzung ist schmal.** Beispielquelle öffnen → etwas Fertiges sehen → eine leichte Entscheidung. Eigene Datenbank und unsichere Kanten kommen danach (`docs/ui-ux-notes.md`).
5. **Kernarbeit im Canvas.** Verbinden, Review, Reports, Explore nicht in Dialogen.
6. **Overlay ist die Wahrheit.** Re-Analyse löscht keine User-Entscheidungen. Queries und Reports nutzen das *published* Model.
7. **Sicherheit vor Feature-Komfort.** Keine öffentlichen Share-Links, kein Third-Party-JS auf App-Seiten, Secrets nie im Overlay.
8. **Fixtures bleiben klein.** Engine-Qualität über Kataloge und kompakte Schemas, nicht über Massendaten.

Schätzungen: **S** wenige Tage, **M** ein bis zwei Wochen, **L** mehrere Wochen — bei einer Person, ohne parallele Tracks. Sie dienen der Reihenfolge, nicht dem Kalender.

---

## Zielbild der ersten nutzbaren App (vertikaler Schnitt)

Bevor jede Fläche „vollständig“ ist, gibt es **einen** durchgängigen Weg:

```text
Signup / Login
    → Organisation (gründen oder Einladung annehmen)
    → App-Chrome (leer, aber stabil)
    → Beispielquelle öffnen
    → Analyse läuft (Fortschritt sichtbar)
    → Canvas zeigt KPI-Karten / einen vorgefertigten Bericht
    → eine hochkonfidente Relationship akzeptieren
    → Overlay gespeichert, Akzentleiste: 1 accepted
```

Das ist der erste Meilenstein für Menschen außerhalb des Repos. Alles danach vertieft denselben Kreis: mehr Quellen, volles Review, Explore, Team, Betrieb.

---

## Phasenüberblick

```text
0  Fundament          Locale, Reason-Codes, Org-Modell, Isolation, App-Crate
1  Identität          Signup, Login, Session, Org, Einladung, Rollen
2  Chrome             Rails, Suche, Akzent, Canvas-Gerüst, Mobile, Empty States
3  Erster Erfolg      Demo-Quelle, Analyse-Job, Übersicht, erster Bericht
4  Modell             Overlay in der App, Relationships / KPIs / Identities
5  Analyse vertiefen  Explore, eigene Postgres-Quelle, Export, Packs
6  Team & Vertrauen   Audit, MFA/Passkeys, Suche voll, Konflikte am Overlay
7  Betrieb            SSO, Billing, DSGVO-Prozesse, Residenz, Hardening
```

Phase 0–3 ergeben den vertikalen Schnitt. Phase 4 macht das Produkt zum Review-Werkzeug. Phase 5 schließt die Datenanalyse. Phase 6–7 sind B2B-Reife, nicht Blocker für interne Nutzung.

---

## Phase 0 — Fundament

Ohne diese Pakete wird jedes spätere Feature nachgerüstet und falsch. Noch keine Login-Seite mit echtem Traffic, aber die Schienen müssen liegen.

### P0.1 App-Schicht vom Core trennen — S — erledigt 2026-09-15

**Ziel.** Einen Ort schaffen, an dem Tenancy leben darf, ohne `analytics` anzufassen.

- Neues Workspace-Crate (Name später, hier: *App*). `web/` bleibt PoC.
- App darf HTTP, Sessions, DB der Control Plane. Core importiert weiterhin kein Postgres-App-Schema, kein Axum, kein HTML.
- Öffentliche Core-API bleibt `analyze` / `analyze_with_packs` / `query_from_analysis` plus Overlay-Anwendung (siehe P0.3).
- README/Docs: zwei Welten klar benennen (PoC lokal vs. Produkt-App).

**Fertig wenn:** `cargo test` für Core unverändert grün; App startet als leere Hülle.

**Umgesetzt als:** `app/` → Paket `octa-app` (`cargo run -p octa-app`, Loopback `127.0.0.1:4000`). Routen: nur `GET /healthz` → `{"status":"ok"}`; alles andere leere 404. Baseline-Header auf jeder Antwort (`nosniff`, `frame-ancestors 'none'`, `no-referrer`, `no-store`). `app/tests/boundary.rs` prüft, dass Core-Manifest und Core-Quellen frei von Axum/HTML/App-Crates bleiben und die App nicht auf `analytics-web` aufsetzt. Bewusst ohne UI-Copy und ohne `analytics`-Abhängigkeit — beides kommt mit P0.2 bzw. Phase 3, damit keine Sprache vor den Katalogen entsteht.

### P0.2 Locale-Stack — M — erledigt 2026-09-16

**Ziel.** Sprache, Formate und Zeitzonen sind Architektur, kein Add-on.

- Message-Kataloge DE + EN, ICU MessageFormat.
- CI bricht bei fehlendem Key in einer aktiven Locale.
- Felder: UI-Sprache und Anzeige-TZ am User; Reporting-TZ, Einladungs-Default, Fiscal später an der Org.
- Speicher UTC; Anzeige in User-TZ. Keine Server-Localzeit.
- `html lang` aus wirksamer UI-Sprache.
- Glossar der Produktwörter (Relationship, Overlay, Pack, …) festlegen — übersetzt oder bewusst unübersetzt, nicht gemischt.
- Fallback-UI-Sprache dokumentieren — **entschieden: `de`**.

**Fertig wenn:** Ein Hello-Screen in DE und EN umschaltbar ist; ein absichtlich fehlender Key fällt in CI auf.

**Umgesetzt als:**

- Kataloge `app/locales/de.json`, `en.json` (flache Keys, ICU MessageFormat), in die Binary kompiliert. Keys in Code nur über das Enum `Msg` (`app/src/i18n/mod.rs`).
- Eigener ICU-Parser für die genutzte Teilmenge (`app/src/i18n/message.rs`): Argumente, `number`, `plural` mit `#`, `offset:`, `=n`, `select`, Apostroph-Quoting. `other` ist Pflicht, sonst Ladefehler. `date`/`time`/`selectordinal` kommen, wenn ein Katalog sie braucht.
- CI-Gate = `cargo test --workspace`: Tests brechen bei fehlendem Key, verwaistem Key, unparsbarer Message, abweichenden Argumenten zwischen den Sprachen und Markup im Katalog. Es gibt noch keinen Workflow-File; `.cargo/config.toml` enthält einen maschinenspezifischen Linker-Pfad, der vor gehosteter CI raus muss.
- `app/src/locale.rs`: `Language` (aktiv `de`, `en`; `FALLBACK = De`), `Locale` (Formate `de-DE`, `en-US`: Tausender, Dezimal, numerisches Datum/Zeit), `UserLocalePrefs` (UI-Sprache, Locale, Anzeige-TZ auto/fest), `OrgLocaleDefaults` (Reporting-TZ, Einladungs-Sprache; Fiscal später), IANA-Zonen über `chrono-tz`, `Instant = DateTime<Utc>`.
- Auflösung: User → Org-Default → `?lang=`-Share-Hint → `Accept-Language` → `de`. Anzeige-TZ: fest → Gerät (bei auto) → Reporting-TZ der Org → UTC. Server-Localzeit kommt nirgends vor.
- Hello-Screen `GET /`: `html lang`, `Content-Language`, `Vary: Accept-Language`, Sprachwechsel über Links mit `hreflang`/`lang`/`aria-current`. Kein Cookie, keine Session — das übernimmt die gespeicherte Präferenz in P1.5. Die Seite ist Nachweis, nicht Chrome; Phase 2 ersetzt sie.
- Glossar: `docs/glossary.md` (en-US-Schreibweise; Flächen-/Konstruktnamen unübersetzt; Overlay-Zustände als Codes mit übersetzten Labels). Dort offen: Anrede Sie/du vor P1.1, Rollen-Labels vor P1.4.
- Nicht gebaut: Pseudo-Locale, Locale-Collation, Wochenstart/Fiscal, Währungsformat — jeweils beim ersten echten Bedarf (Suche P6.1, Reports P3.4/P5.5, Org-Einstellungen P1.5).

### P0.3 Reason-Codes in der Engine — M

**Ziel.** Der Core spricht keine UI-Sprache.

Heute sind `reason`-Felder englische Sätze (`src/relationships.rs`, `kpi.rs`, `identities.rs`, `reports.rs`, `semantic.rs`). Die App kann die nicht zuverlässig übersetzen.

- Stabile Codes + Parameter (z. B. `rel.coverage {from, to, ratio}`).
- Menschlicher englischer String darf intern/Tests bleiben, ist aber nicht die UI-Quelle.
- Katalog der Codes in der App übersetzen (DE/EN).
- Bestehende Tests auf Codes umstellen, wo sie auf Fließtext matchen.

**Fertig wenn:** Keine nutzersichtige Engine-Ausgabe hängt an einem Locale-String im Core.

### P0.4 Overlay ins Core-API, published Model — M

**Ziel.** Query-Planer und Reports nutzen dasselbe bestätigte Modell, unabhängig von HTML.

Heute lebt Overlay-Anwendung in `web/src/overlay.rs`. Das gehört nicht in die Wegwerf-UI.

- Overlay-Typen (Status `accepted | rejected | edited | pending`) als Teil der öffentlichen Core-API oder eines kleinen, UI-freien Moduls.
- `publish(analysis, overlay) → published semantic model + queryable KPIs`.
- Re-Analyse merget: alte Entscheidungen bleiben, Neue erscheinen als `pending`.
- CLI kann Overlay lesen/schreiben, ohne Axum.
- Semantische Rollen bleiben übersteuerbar (Vertrag in `docs/model-review.md`).

**Fertig wenn:** Ein Test ohne Web-Crate akzeptiert eine Kante, lehnt eine KPI ab, und `query_from_analysis` sieht nur das Published-Set.

### P0.5 Datenmodell: Org, User, Rolle, Objekte — M

**Ziel.** Das Blatt aus dem Konstrukt in ein Schema gießen, bevor Login gebaut wird.

Objekte mit `org_id` (UUID, nie sequential in URLs):

| Objekt | Phase, in der es lebendig wird |
| --- | --- |
| Organisation, Mitgliedschaft, Rolle | 1 |
| Session, Credential, Einladung | 1 |
| Quelle (Config, Secret-Metadaten) | 3 / 5 |
| Analyse-Lauf, Artefakte | 3 |
| Overlay, Overlay-Version | 4 |
| Report-Snapshot, Export-Job | 5 |
| Audit-Event | 1 (schreiben), 6 (lesen in der UI) |

Rollen V1 (Namen dürfen schärfer werden): Owner, Admin, Analyst, Viewer, Billing.

**Fertig wenn:** Schema + Migrationen existieren; noch keine öffentliche Signup-Route nötig.

### P0.6 Isolation fail-closed — L

**Ziel.** Mandantenfilter überleben App-Bugs.

- Postgres RLS `USING` + `WITH CHECK`, `FORCE ROW LEVEL SECURITY`, App-Role ohne `BYPASSRLS`.
- Pro Transaktion `SET LOCAL app.org_id`, nie Connection-weit im Pool.
- Cache-/Job-/Storage-Keys beginnen mit `org_id`.
- Autorisierung am Objekt (`can(org, actor, action, resource)`), nicht „ist eingeloggt“.
- Tests, die den App-Filter absichtlich weglassen: Antwort leer/deny, kein Leak.
- Threat Model skizzieren für: Login, Invite, Connect, Overlay-Write, Export.

**Fertig wenn:** Ein Cross-Tenant-Test in CI ist Pflicht und würde bei vergessenem `org_id` rot.

---

## Phase 1 — Identität: Signup, Login, Organisation

Erstes nutzersichtiges Produkt. Noch keine Analyse. Passwort-only ist laut Konstrukt nicht das B2B-Zielbild — für den ersten Schnitt aber der realistische Einstieg, sofern MFA und SSO nicht verbaut werden.

### P1.1 Signup — M

**Ziel.** Eine Person kann ein Konto anlegen.

- E-Mail + Passwort (Argon2id), oder Magic-Link nur als Einladungs-Ausnahme.
- Verifikation der E-Mail.
- Sprache/TZ-Vorschlag aus Browser, speichern nach Bestätigung.
- Keine Org-Daten in der Mail; keine Magic-Admin-URLs.
- Rate Limit gegen Credential-Stuffing / Account-Enumeration (generische Fehler).
- Copy und Fehler in DE/EN.

**Fertig wenn:** Unverifizierte Accounts kommen nicht in die App; verifizierte landen in P1.2.

### P1.2 Login, Session, Logout — M

**Ziel.** Sitzung ist langweilig-sicher.

- Login E-Mail/Passwort; Session-Cookie `HttpOnly`, `Secure`, `SameSite`.
- Kurzes Access, rotierendes Refresh, server-seitig widerrufbar. Keine Tokens in LocalStorage.
- Idle-Timeout und absolute Lifetime (Defaults konservativ, später org-konfigurierbar).
- CSRF wo SameSite nicht reicht.
- „Überall abmelden“, Geräteliste kann in P6 nachziehen.
- Tiefe Links: nach Login zurück zum ursprünglichen Ort derselben Org.
- Audit: Login success/fail (ohne Passwörter, ohne Payloads).

**Fertig wenn:** Abgelaufene Session fail-closed; kein Request ohne Tenant-Kontext liefert Daten.

### P1.3 Organisation gründen und wechseln — M

**Ziel.** Nach dem ersten Login gibt es genau eine aktive Organisation, sichtbar.

- Flow „Org anlegen“ (Name, Reporting-TZ als Pflicht oder dokumentierter Default).
- Aktive Org in der Session, nicht aus User-Input in der URL als einzige Wahrheit.
- Org-Wechsel explizit; Tabs/Clipboard/Suche gehören zu genau einer Org.
- Datenresidenz-Default EU (Feld existiert, Umschalten später).
- Owner-Rolle für die gründende Person.

**Fertig wenn:** Zwei User in zwei Orgs können einander keine Objekte über IDs erraten (Tests aus P0.6 gegen echte Routen).

### P1.4 Einladung und RBAC — M

**Ziel.** Ein zweiter Mensch kommt in dieselbe Org, mit klarer Rolle.

- Einladung zeitlich begrenzt, an E-Mail gebunden, Rolle sichtbar.
- Annahme: bestehender Account oder Signup, dann Mitgliedschaft.
- Analyst darf später Review; Viewer liest, kein Accept/Reject, keine neuen Quellen.
- Owner/Admin: Mitglieder und Rollen.
- Kein impliziter Superuser in der Kunden-UI.

**Fertig wenn:** Viewer sieht Canvas-Daten (sobald Phase 3 existiert) ohne Mutations-Routen; Invite-Replay nach Ablauf ist tot.

### P1.5 Account-Einstellungen — S

**Ziel.** Locale hängt am Menschen und reist in jede Org.

- UI-Sprache, Locale-Formate, Anzeige-Zeitzone (auto vs. fest).
- Org-Defaults: Reporting-TZ, Standardsprache für Einladungen.
- Ein Einstellungsort, kein verstecktes `?lang=` als Wahrheit.

**Fertig wenn:** Derselbe User sieht in Org A und Org B dieselben Wörter/Formate, aber Reporting-Grenzen folgen der jeweiligen Org.

Nicht in Phase 1: SSO, SAML, SCIM, Passkeys, IP-Allowlist, Billing-UI, Support-JIT.

---

## Phase 2 — App-Chrome (noch ohne Engine-Fläche)

Leere, aber echte App. Wer Navigation und Suche einmal gelernt hat, findet sie später immer dort. Referenz: `docs/ui-ux-notes.md`.

### P2.1 Desktop-Gerüst — M

- Left Rail (Icons, ausklappbar): Quellen, aktive Quelle, Übersicht, Modell, Reports, Explore; unten Packs/Overlay unter „Mehr“, bis sie Inhalt haben.
- Top-Bar: Name, **Suche oben rechts** (Feld, Shortcut `/` oder `Ctrl+K` — Treffer dürfen erst Orte sein).
- Akzentleiste: eine Statuszeile (noch oft leer).
- Seitenleiste: Titel + eine Primäraktion.
- Canvas: Empty State der aktuellen Route.
- Right Rail: Icon-Leiste, Panel später.
- Org-Wechsel und Account im Chrome, ohne den Canvas zu klauen.
- Theme: professionell, kühl; noch keine finale Palette nötig, aber keine PoC-Serife.

**Fertig wenn:** Alle Hauptorte routen in dasselbe Gerüst; Dialoge existieren nur als Muster für destruktive Bestätigung.

### P2.2 Mobile-Gerüst — M

- Bis ~767px: keine Seiten-Rails, Canvas kantenbündig, Bottom-Nav (max. fünf Orte, Rest hinter Mehr).
- Suche als Icon oben rechts, vollbreites Overlay.
- Inspektor als Bottom-Sheet.
- Touch-Ziele, `safe-area-inset-*`, kein Desktop-`max-width`.

**Fertig wenn:** Dieselben Routen ohne horizontales `body`-Scrollen nutzbar sind.

### P2.3 Empty States und Ton — S

Pro leerer Fläche: Illustration-Platzhalter, Satz, nächste leichte Handlung. Drei Einstiege wo sinnvoll (Video, Vorlage, vorgefertigter Bericht) — Videos dürfen Links/Stubs sein.

Erste Meilenstein-Momente vorbereiten (Toast-System, Accept-Zustand), noch ohne echte Daten.

**Fertig wenn:** Quellen-leer, Reports-leer, Modell-leer jeweils eine klare Primäraktion zeigen — DE und EN.

---

## Phase 3 — Erster analytischer Erfolg

Hier trifft die App auf den bestehenden Core. Noch keine eigene Kunden-Postgres. Der schnelle Gewinn ist absichtlich eine **Beispielquelle**.

### P3.1 Home / Quellenliste — S

- Canvas: vorhandene Quellen der Org, „Beispiel öffnen“.
- Beispielvorlagen = bestehende Fixtures (`ecommerce_*`, `crm`, …), org-kopiert, nicht global geteilt.
- Kein Modal-Wizard.

**Fertig wenn:** Analyst sieht Karten/Zeilen; Viewer sieht dieselben, ohne „neue Verbindung“.

### P3.2 Analyse als Job — M

**Ziel.** Die Pipeline läuft nicht im Request-Thread bis Timeout.

- Job mit Mandantenkontext des Auslösers (kein globaler Worker ohne RLS).
- Fortschritt in der Akzentleiste; Abbrechen; Tab darf zu, Job läuft weiter.
- Fehler handlungsfähig, Reason-Codes übersetzt, keine Stacktraces, keine Samples in Logs.
- Artefakte (Schema, Profil, Vorschläge) org-isoliert speichern.
- Quotas/Timeouts pro Org (Fairness, noch kein Billing).

**Fertig wenn:** Fixture-Analyse in der App dasselbe fachliche Ergebnis liefert wie `cargo run -- inspect --fixture …`.

### P3.3 Übersicht — M

Canvas: Schema, Spaltenrollen, Profil-Kennzahlen, Laufzeiten. Seitenleiste: Ansicht umschalten, Re-Analyse.

Evidenz und Zahlen formatieren nach User-Locale; Rohwerte bleiben Daten.

**Fertig wenn:** Nach dem Job ist die Übersicht nicht leer; Re-Analyse überschreibt Vorschläge, nicht ein späteres Overlay (Overlay kommt in Phase 4 — bis dahin gilt Analyse direkt, wie der Core-V1-Vertrag).

### P3.4 Reports als ersten Erfolg — M

Absicht der UI-Notizen: nicht zuerst unsichere Kanten, sondern etwas Fertiges sehen.

- KPI-Karten und 1–3 vorgeschlagene Reports aus `suggest_reports`.
- Eine Visualisierung (Karte, Balken oder Linie) reicht für V1 dieses Pakets.
- Empty State „Reports“: vorgefertigten Bericht auf der Beispielquelle öffnen.
- Toast / ruhige Illustration beim ersten sichtbaren Bericht.
- Queries gegen die Analyse (noch ohne Overlay), Ausführung über `DataSource` des Jobs.

**Fertig wenn:** Ein neuer User kann ohne eigene DB in unter wenigen Minuten einen Bericht sehen.

Damit ist der **vertikale Schnitt** geschlossen: Account → Org → Chrome → Quelle → Analyse → sichtbares Ergebnis.

---

## Phase 4 — Modell reviewen (Produktvertrag)

Ohne dieses Phase bleiben Reports Schätzungen der Engine. Danach sind sie Entscheidungen der Org. Vertrag: `docs/model-review.md`.

### P4.1 Overlay in der Org-DB — M

- Versioniertes Overlay pro Quelle: Relationships, KPIs, Identities, Packs, später Rollen-Overrides.
- Jede Mutation hat Akteur + Zeit (Sicherheit und Nachvollziehbarkeit).
- Publish-Schritt oder Auto-Publish nach Accept — festlegen und bei einer Variante bleiben.
- Konflikt: nicht still überschreiben, wenn zwei Analysten parallel schreiben (Version / Hinweis).
- Re-Analyse: pending für Neues, accepted/rejected bleiben.

**Fertig wenn:** Overlay überlebt Server-Neustart und zweiten Analysten derselben Org; fremde Org sieht nichts.

### P4.2 Relationships im Canvas — L

- Liste/Graph der Kanten: Band (declared / inferred / uncertain), Konfidenz, Evidence, Reason (übersetzt).
- Declared FKs vor-akzeptiert, trotzdem deaktivierbar.
- Accept / Reject / Edit an der Zeile und als Seitenleisten-Aktion für Auswahl.
- Beziehung hinzufügen (Canvas, nicht Dialog).
- Filter nach Band in der Seitenleiste; Evidenz-Panel rechts bzw. Sheet mobil.
- Rejected bleibt sichtbar.

**Fertig wenn:** Eine rejected Kante steckt nicht mehr im published Model und nicht in Folge-Queries.

### P4.3 KPIs im Canvas — M

- Strukturname, optionales Dictionary-Label, eigene Bezeichnung.
- Aggregation, Quelle, Konfidenz, Pack-Nachweis oder `none`.
- Übernehmen / verwerfen; Label speichern.
- Dropped KPIs nicht in Reports/Explore.

**Fertig wenn:** Umbenannte KPI erscheint unter dem User-Label in Reports.

### P4.4 Numeric Identities im Canvas — M

- Expression, Template, Match-Ratio, MAE, Sample-Größe, `k`, Scope.
- Wertebasiert erklären (Codes, keine Namensmagie).
- Accept / Reject / Formel editieren.
- Akzeptierte Identities: abgeleitete Measures, Double-Count-Warnung (Core tut das schon teilweise via `drop_double_counted_kpis`).

**Fertig wenn:** Rejected Identity fließt nicht ins published Model.

### P4.5 Semantische Rollen übersteuern — S

Measure / Dimension / Time / Identifier / FK am Canvas, gleicher Overlay-Mechanismus.

**Fertig wenn:** Eine fälschlich klassifizierte Spalte lässt sich korrigieren, ohne die Quell-DB zu ändern.

### P4.6 Leichte erste Entscheidung verdrahten — S

Nach P3.4: auf der Modell-Fläche eine hochkonfidente Kante als *eine* Tap-Aktion, sichtbare Zustandsfarbe, Akzentleiste `n pending · m accepted`.

Fortgeschrittenes (SQL, Overlay-Datei, Template-Details) hinter Overflow / Right-Rail.

**Fertig wenn:** Der Onboarding-Pfad aus den UI-Notizen ohne Wizard durchläuft.

---

## Phase 5 — Datenanalyse vervollständigen

Jetzt darf die Org **eigene** Daten anschließen und ad hoc fragen. Das ist der Kern des verkauften Produkts.

### P5.1 Explore — M

- Canvas: Measure, Dimension, optionale Zeitgranularität gegen das **published** Model.
- Ausführen in der Seitenleiste; Result-Tabelle; Copy-SQL in der Right-Rail.
- Leere Fläche: Beispiel-Query, kein leeres Formular als Erstes.
- Kein Chart-Builder, kein Drag-and-Drop-Ersatz für das Modell (`docs/model-review.md`).

**Fertig wenn:** Explore ignoriert rejected KPIs/Kanten; SQL entsteht nur über die Query-Schicht (parametrisiert / Allowlist aus dem eigenen Schema-Katalog).

### P5.2 Eigene PostgreSQL-Quelle — L

Höchstes Alltagsrisiko (SSRF, Secret-Leak, Injection in die Kunden-DB). Threat Model aus P0.6 hier abarbeiten, bevor die Route live geht.

- Verbindung im Canvas: Host, Port, DB, User, Secret, optionales Schema-Allowlist. Testen, Schema sehen.
- Credentials nur Secret-Store / Envelope-Encryption; nach dem Speichern nie Klartext in der UI.
- Bevorzugt Hinweis auf read-only Rolle.
- Egress: private Ranges / Cloud-Metadata je nach Modus blocken, Timeouts, kein offenes Redirect-Follow.
- Samples nur für den Mandanten; nicht in Logs, nicht in Error-Tracking-Bodies.
- Step-up (Re-Auth) für neue Quelle und Credential-Rotation.
- Admin legt an; Analyst nutzt; Viewer weder Secret noch Connect.

**Fertig wenn:** Isolationstests + SSRF-Negativtests grün; Fixture-Pfad bleibt für Demos und CI.

### P5.3 Packs in der App — S

Dictionary-Packs (`generic`, `commerce`, …) pro Quelle wählbar, im Overlay gespeichert. Re-Analyse mit neuen Packs ergänzt Labels, löscht keine Accepts.

**Fertig wenn:** Pack-Wechsel ist eine Org-Entscheidung, sichtbar in der Akzentleiste.

### P5.4 Export — M

- CSV/Excel: Trennzeichen und Dezimal nach Locale; UTF-8 BOM wo Excel es braucht; optional später invariant CSV für Maschinen.
- Dateiname inkl. Datum in User-Locale.
- Private Storage, kurzlebige auth-geprüfte URLs, Tenant-Prefix, Quota.
- Viewer-Export: offene Konstrukt-Frage — Default konservativ (Analyst ja, Viewer nein), bis entschieden.
- Keine Tabelleninhalte in E-Mails.

**Fertig wenn:** Ein DE-User öffnet den Export in Excel ohne zerschossene Spalten.

### P5.5 Reports vertiefen — M

- Weitere Visualisierungen aus dem Core (`line`, `bar`, `pie`, `table`).
- Reports rechnen auf published Model (nach Phase 4).
- SQL zeigen als Inspektor, nicht als Voraussetzung.
- Keine öffentlichen Report-Links (Default aus, Produktentscheidung später).

**Fertig wenn:** Dashboard der Org zeigt nur akzeptierte bzw. nicht verworfene Measures.

---

## Phase 6 — Team, Suche, Vertrauen

Die Analyse-Schleife ist geschlossen. Jetzt wird aus einem Werkzeug ein B2B-Alltag.

### P6.1 Globale Suche voll — M

Quellen, Tabellen, Spalten, Relationships, KPIs, Identities, Reports, App-Orte. Enter springt in den Canvas und markiert den Treffer. Optional später Aktionen aus Treffern (offene UI-Frage). Locale-Collation.

### P6.2 Audit-UI für Admins — M

Wer hat Overlay published, Quelle angelegt, Rolle geändert, Export gezogen. Append-only aus App-Sicht. Keine Cell-Values in der Ansicht, die nicht sowieso Overlay sind.

### P6.3 MFA, Passkeys, Geräte — L

Sobald Passwörter existieren: MFA erzwingbar durch die Org. Passkeys/WebAuthn für Personen ohne SSO. Step-up für Gefahrzonen (Quelle, SSO später, Org löschen, Massenexport).

### P6.4 Overlay-Kollision und Präsenz — M

Sichtbares „zuletzt published von“. Optimistic concurrency. Live-Cursor ausdrücklich später / optional.

### P6.5 Benachrichtigungen — S

In der User-Sprache, Quiet Hours der Anzeige-TZ, Sprungziel, abschaltbar. Keine Mail pro Accept. Analyse-fertig und Einladung reichen für V1.

### P6.6 Hilfe an der Funktion — S

Kurze Videos pro Fläche (Quellen, Relationships, Reports, Explore), `?` in der Seitenleiste wenn die Fläche nicht mehr leer ist. WCAG 2.2 AA an den Kernflows nachziehen (Fokus, Kontrast, Screenreader-Texte aus denselben Katalogen).

---

## Phase 7 — Betrieb und Enterprise

Nicht nötig, um intern Daten zu analysieren. Nötig, bevor fremde Unternehmen echte Daten anvertrauen.

### P7.1 SSO (OIDC, später SAML) — L

Pro Org. SSO-enforced Policy (kein lokales Passwort). IdP down = fail-closed, nicht fail-open. SCIM separat danach.

### P7.2 Billing am Rand — M

Rechnung an die Org, EU-Pflichtfelder, Steuern. Nutzungsmessung ohne Query-Inhalte. UI darf schmal sein; Datenmodell der Org darf Billing tragen, ohne den Canvas zu vermüllen.

### P7.3 Datenlebenszyklus — M

Auskunft, maschinenlesbarer Org-Export, Löschen inkl. Secrets und Overlay, Soft-Delete mit harter Frist. Retention: Audit länger als Samples. Staging kennt keine Produktionskundendaten.

### P7.4 Observability und Incident — M

Metriken/Traces mit `org_id`, ohne Payloads und SQL-Werte. Security-Events geloggt **und** alarmiert. 72-Stunden-Pfad DSGVO Art. 33, Statusseite ohne Kundendaten. `cargo audit` / SBOM in CI (kann früher starten, hier wird es Release-Pflicht).

### P7.5 Residenz, AVV, Security-Seite — L

EU-Default durchhalten. Subprozessorliste, TOMs, Vulnerability-Meldeweg. BYOK nur vorbereiten (Key-Context = Org), nicht in V1 erzwingen. Pen-Test vor nennenswertem Kundenbetrieb.

---

## Was der Core parallel tun darf

Die App wartet nicht auf Engine-Perfektion. Der Core wächst weiter nach bestehenden Regeln: Kataloge statt Hardcoding, kleine Fixtures, Confidence + Evidence an jedem Vorschlag.

Sinnvolle Core-Pakete **neben** der App (kein Auth im Core):

| Paket | Warum die App es braucht |
| --- | --- |
| Reason-Codes (P0.3) | Übersetzung |
| Overlay-Publish-API (P0.4) | Queries = Userwille |
| Job-freundliche Analyse (Abbruch, Teilfortschritt) | P3.2 |
| Naive Timestamps nicht still mappen | Reporting-TZ ehrlich |
| Weitere Identity-Templates / Dictionary-Einträge wenn Tests Lücken zeigen | Qualität, nicht Roadmap-Gate |

PostgreSQL-Connector existiert bereits im Core. Die App liefert Isolation, Secrets und SSRF; der Core bleibt Adapter hinter `DataSource`.

---

## Reihenfolge der ersten zehn Pakete

Wenn nur eine Spur gearbeitet wird:

1. P0.1 App-Crate  
2. P0.2 Locale-Stack  
3. P0.5 + P0.6 Schema und RLS (ohne UI)  
4. P0.3 Reason-Codes  
5. P0.4 Overlay-API im Core  
6. P1.1–P1.3 Signup, Login, Org  
7. P2.1–P2.2 Chrome Desktop + Mobil  
8. P3.1–P3.2 Quellen + Analyse-Job  
9. P3.4 Erster Bericht  
10. P1.4 Einladung (sobald zwei Menschen dasselbe Modell brauchen), dann P4.x Review  

P1.4 kann vor dem ersten Bericht kommen, wenn Team-Test wichtiger ist als Solo-Demo. Fachlich hängt Review nicht an Einladungen.

---

## Definition of Done je Phase

| Phase | Menschen können … |
| --- | --- |
| 0 | nichts in der UI, aber Isolation und Locale sind testbar |
| 1 | Konto anlegen, einloggen, eine Org besitzen, Sprache setzen |
| 2 | sich in der App bewegen, auch auf dem Telefon |
| 3 | eine Beispielquelle analysieren und einen Bericht sehen |
| 4 | das Modell korrigieren; Reports folgen dem Overlay |
| 5 | eigene Postgres anschließen, Explore, Export |
| 6 | zu zweit arbeiten, Suche, Audit, MFA |
| 7 | Enterprise-Login, Rechnung, Löschauskunft, Betrieb |

---

## Bewusst nicht auf dieser Roadmap

- Landingpage, Marketing, Trial-Banner, „Invite your team“ als Wachstumsfläche  
- Auth/Billing/Tenancy **im** Crate `analytics`  
- Zusätzliche Connectoren (Oracle, SQL Server, SAP, …)  
- Chart-Builder, Drag-and-Drop-Wizards als Ersatz für Canvas-Review  
- Öffentliches Report-Sharing, Consumer-Sheet-Links  
- LLM auf Kundendaten  
- K8s-Feinschliff, Multi-Region aktiv, Self-Host als V1  
- Massendaten-Performance-Tests und Fixture-Aufblähung  

---

## Offene Fragen, die Pakete blockieren können

Aus Konstrukt und UI-Notizen; vor dem genannten Paket entscheiden:

| Frage | Spätestens vor |
| --- | --- |
| ~~Fallback-UI `de` oder `en`?~~ entschieden 2026-09-16: **`de`** | P0.2 |
| Reporting-TZ Pflicht vs. Default `Europe/Berlin`? | P1.3 |
| Passkeys-first vs. Passwort+MFA für Design-Partner? | P1.1 / P6.3 |
| Session: ein Account-Cookie plus Org-Switch, oder Token pro Org? | P1.2 |
| Rollen fest vs. Permission-Sets für Custom Roles? | P0.5 |
| Modell: eine Seite mit Umschalter oder drei Rail-Orte? | P2.1 / P4.2 |
| Erstes Erfolgserlebnis: KPI-Karten vs. vorgefertigter Bericht? | P3.4 |
| Dürfen Viewer exportieren? | P5.4 |
| Naive DB-Timestamps: Org-Default, pro Quelle, oder „unbekannt“? | P5.2 |
| Residenz V1 nur EU? | P7.5 |

---

## Nächster konkreter Schritt

Nicht die Login-Seite als Erstes pixeln. Als Nächstes implementieren, sobald dieser Plan gilt:

1. ~~App-Crate anlegen (P0.1)~~ erledigt: `app/` (`octa-app`)  
2. ~~Message-Kataloge + CI-Check (P0.2)~~ erledigt: `app/locales/`, `app/src/i18n/`, `app/src/locale.rs`, `docs/glossary.md`  
3. Org/User-Schema mit RLS und einem Isolationstest (P0.5–P0.6)  
4. Reason-Codes + Overlay-Publish im Core (P0.3–P0.4)  

Danach erst Signup/Login (Phase 1). So landet kein Account auf einer Architektur, die Tenancy oder Sprachen nachrüsten muss.
