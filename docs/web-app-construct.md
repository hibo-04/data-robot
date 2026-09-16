# Web-Applikation: Produktkonstrukt (B2B SaaS)

Status: Diskussionsnotizen, noch keine Implementierung.
Stand: 2026-09-15.

**Arbeitstitel: Octa** (Logo und Branding: [`docs/branding/`](branding/)).

Beschreibt das **grundsätzliche Konstrukt** der späteren Web-Applikation als weltweit nutzbares B2B-SaaS. Nicht die heutige lokale PoC-UI, nicht Landingpage/Marketing, nicht das Analytics-Core-Innenleben.

Verwandte Dokumente:

| Dokument | Zuständig für |
| --- | --- |
| Dieses File | Mandanten, Sicherheit, Identität, Locale-Stack, Datengrenzen, Betrieb, Bequemlichkeit auf Produktebene |
| `docs/branding/` | Arbeitstitel, Logo, vorläufige Bildsprache |
| `docs/ui-ux-notes.md` | Chrome, Layout, Canvas, Mobile, Empty States, visuelles Verhalten |
| `docs/model-review.md` | Accept/Reject/Edit, Evidenz, Overlay als Quelle der Wahrheit |
| `.cursor/rules/analytics-core-scope.mdc` | Core bleibt quellenagnostisch, ohne Auth/Billing/Cloud im V1-PoC |

**Neubau des Produkts, kein Weiterbauen der lokalen Review-Seite.** Der PoC in `web/` darf weiter lokal und ohne Login bleiben. Sobald wir eine echte App bauen, gilt dieses Konstrukt — nicht erst „später, wenn es weh tut“. Entscheidungen hier sind so gewählt, dass wir Sicherheit, Sprachen und Zeitzonen nicht nachrüsten müssen.

Der Analytics-Core (`analytics`) bleibt frei von HTTP, HTML, Auth und Mandantenfiltern. Tenancy, Locale, Sessions und Verschlüsselung sitzen in der **Applikationsschicht** (Web-Adapter, Control Plane). Der Core bekommt weiterhin nur eine `DataSource` plus Overlay.

---

## Absicht

Wir verkaufen keine Einzelplatz-Datei, sondern ein **B2B-SaaS**: Organisationen zahlen, Menschen darin arbeiten, Kundendaten bleiben in der Organisation. Die App ist von jedem unterstützten Standort erreichbar. Ein User in Tokio und ein User in Berlin sehen dieselben Fakten, aber Uhrzeiten, Formate und Sprache passen zu **ihnen** — nicht zu unserem Server.

Drei Dinge sind nicht verhandelbar:

1. **Sicherheit und Isolation.** Ein Mandant darf die Daten eines anderen Mandanten nicht sehen, nicht erraten, nicht über Caches, Logs, Support-Tools, URLs oder Drittanbieter erreichen. Das ist der Produktvertrag.
2. **Weltweite Nutzung von Tag eins.** Sprache, Locale, Zeitzone, Kalender und verwandte Präferenzen sind Teil der Architektur, kein Add-on.
3. **Bequemlichkeit ohne Nachfragen.** Wer die App benutzt, soll sich nicht mit UTC, Message-IDs oder „welchen Tenant meine Query meint“ beschäftigen. Defaults sind richtig, Abweichungen sind selten und klar.

Zielbild: eine Organisation öffnet die App, sieht nur die eigenen Quellen und Modelle, arbeitet im Chrome aus `docs/ui-ux-notes.md`, bestätigt Vorschläge nach `docs/model-review.md`. Die Plattform darunter ist langweilig-sicher: fail-closed, nachvollziehbar, mehrsprachig, zeitzonenfest.

---

## Was wir festlegen / was wir hier weglassen

Festlegen (Konstrukt):

- Mandantenmodell (Organisation als Grenze)
- Identitäten, Sessions, Berechtigungen
- Isolations- und Krypto-Regeln für Kundendaten
- Locale-Stack (Sprache, Formate, Zeitzone, Kalender, Residenz, …)
- Prozess: jede sichtbare Änderung in **allen** aktiven Sprachen
- Betrieb, Compliance-Pfad, Bequemlichkeit auf Produktebene

Nicht hier (liegt woanders oder bewusst später):

- Pixel, Rails, Canvas, Empty-State-Illustrationen → `docs/ui-ux-notes.md`
- Welche Relationship-Bandes es gibt, Overlay-JSON → `docs/model-review.md`
- Preise, Pakete, konkrete Seat-Zahlen
- Zusätzliche DB-Connectoren, K8s-Topologie, Vendor-Auswahl
- Implementierung im aktuellen PoC (kein Auth, kein Billing, kein Multi-Tenant im Core-V1)

---

## Produktform

B2B-SaaS, nicht Consumer-App, nicht On-Prem-First (ein späteres Self-Host-Angebot darf dieselben Grenzen einhalten, ist aber nicht V1).

### Organisation ist die Grenze

| Begriff | Rolle |
| --- | --- |
| **Organisation (Mandant)** | Rechtliche und technische Isolation. Billing, Verträge, Datenresidenz, Quellen, Overlays, Audit. |
| **Mitglied** | Mensch mit Account, der zu einer oder mehreren Organisationen gehört. |
| **Rolle** | Was das Mitglied *in dieser* Organisation darf. |
| **User-Präferenzen** | Sprache, Locale, Anzeige-Zeitzone — hängen am Menschen, reisen mit in jede Org. |
| **Org-Defaults** | Reporting-Zeitzone, Standardsprache für neue Mitglieder, Datenregion, Sitzungsregeln. |

Ein User kann in mehreren Organisationen sein (Agentur, Konzern mit Töchtern). Nach Login: letzte Organisation merken, Wechsel explizit und sichtbar. Daten, Suche, Clipboard-Flows und Tabs gehören immer zu **genau einer** aktiven Organisation. Kein stilles Mischen.

Workspace/Projekt unterhalb der Org ist optional später. Die Sicherheitsgrenze bleibt die Organisation, nicht ein „Sheet-Link“.

### Was die App tut (im Konstrukt)

Die App ist der Adapter um die Engine: Quellen verbinden, analysieren, Modell reviewen, Reports/Explore. Kundenseitige **Quelldaten** (Tabelleninhalte) sind das Kronjuwel. Metadaten, Overlay, Query-Ergebnisse und Exporte erben dieselbe Mandantengrenze.

Wir sind **Auftragsverarbeiter**, der Kunde ist Verantwortlicher. Keine Nutzung von Kundendaten für das Training fremder Modelle, keine Weitergabe an Analyse-/Support-SaaS ohne Vertrag und Notwendigkeit.

---

## Sicherheit (nicht verhandelbar)

Sicherheit ist das wichtigste Konstruktmerkmal. „Bombensicher“ heißt hier nicht Marketing, sondern: **Defense in Depth, fail-closed, Least Privilege, Zero Standing Access, nachweisbar.** Die OWASP Top 10:2025 sind Awareness, nicht das Programm. Verbindliche Messlatte für die App: **OWASP ASVS 5.x, Ziel Level 2** (sensible Geschäftsdaten). Level 3 nur dort, wo ein Enterprise-Vertrag es verlangt (eigene Keys, höhere Isolation).

Ergänzend als Nordstern, nicht als Checklisten-Theater: NIST SSDF, später ISO 27001 und SOC 2 Type II. DSGVO gilt von Tag eins (wir bauen aus der EU).

### Leitprinzipien

1. **Fail closed.** Fehlt Tenant-Kontext, Session, Key oder Berechtigung → keine Zeile, kein Objekt, kein „vielleicht leer“.
2. **Isolation überlebt App-Bugs.** Mandantenfilter leben mindestens in der Datenbank (Row-Level Security), nicht nur in der Hoffnung, dass jede Query `WHERE org_id` enthält.
3. **Kundendaten verlassen den Mandanten nicht.** Nicht zu anderen Kunden, nicht zu unbeauftragten Dritten, nicht in gemeinsame Caches, nicht in Support-Postfächer, nicht in LLM-APIs, nicht in Browser-Third-Party-Scripts.
4. **Least Privilege überall.** App-DB-Role kann RLS nicht umgehen. Connectoren zur Kundendatenbank nur lesend, nur benötigte Schemas. Mitarbeiter haben keinen Dauzugriff.
5. **Annahme: irgendetwas bricht.** Dann begrenzt Krypto, Netz, Audit und Incident-Prozess den Schaden. Wir designen für den Bruch, nicht nur für den Happy Path.
6. **Secure by default.** Öffentliche Share-Links, weite CORS-Origins, lange Sessions, Debug-Endpoints und „für Support kurz aus“ existieren in Produktion nicht.
7. **Keine Geheimnisse im Client.** Overlay-Logik und UI dürfen Begründungen zeigen; Credentials, Keys, Session-Secrets, interne Hosts nie.

### Bedrohungsmodell (kurz, produktspezifisch)

Was uns umbringen würde, wenn es klappt:

| Bedrohung | Warum sie uns trifft | Gegenwehr (Richtung) |
| --- | --- | --- |
| Cross-Tenant-Read (IDOR, vergessener Filter, Cache-Key ohne Org) | Analytics-Objekte sind leicht über IDs zu raten, wenn sie sequentiell sind | UUID, AuthZ an **jedem** Objekt, RLS, tenant-prefix an Cache/Storage, Tests die Isolation mutwillig brechen wollen |
| Geklaute Session / Token | B2B, oft geteilte Büros, lange Arbeitstage | Kurze Access-Tokens, Rotation, Binding, Idle-Timeout, Org-Policies (SSO-only, IP-Allowlist) |
| SSRF über „Quelle verbinden“ | User gibt Hosts an; die App baut Verbindungen nach außen | Allowlist/Block private ranges je nach Produktmodus, kein Follow in Link-Local, Timeouts, keine Cloud-Metadata-IPs |
| Injection (SQL zur *Kunden*-DB und zur *App*-DB) | Engine generiert SQL; User-Labels landen in UI | Parameterized queries, strikte Query-Plan-Schicht, Output-Encoding, CSP |
| Supply-Chain (Dependency, Build, Registry) | Höchstes CVE-Impact in OWASP 2025 A03 | Lockfiles, Signaturen, `cargo audit`/OSV, minimale Deps, reproduzierbare Builds |
| Insider / Support | „Schau mal kurz in Tenant X“ | Kein Standing Access, JIT + Vier-Augen + Audit, Kundensichtbarkeit |
| Exfil über Logs/Traces/Fehlerseiten | Profiling-Samples und Query-Results sind Inhalt | Redaction, keine Samples in Prod-Logs, generische Fehler nach außen |
| Quell-Credentials geleakt | Postgres-URLs in Tickets, Screenshots, Overlay-JSON | Secret-Store, Envelope-Encryption, nie in Overlay, nie in URLs |
| Öffentliches Objekt im Bucket | Export-CSV, Report-PNG | Private Buckets, kurzlebige signed URLs, Tenant-Prefix, keine ACL public-read |

SSRF ist in OWASP 2025 unter **A01 Broken Access Control** einsortiert. Bei uns ist es Alltagsrisiko, weil Connecting-to-Source Kernfeature ist.

### Mandantenisolation (harte Grenze)

Jeder mandantenspezifische Datensatz trägt eine nicht-nulle, indizierte `org_id` (UUID). Das gilt für Quellen-Konfiguration, Overlays, Analyseartefakte, Reports, Exporte, Audit-Events, Feature-Flags, Uploads.

Schichten, die **alle** halten müssen:

| Schicht | Mechanismus | Wenn sie fehlt |
| --- | --- | --- |
| Edge | AuthN + aktive Org aus Session, nicht aus User-Input | Request ohne Mandant |
| Applikation | `can(org, actor, action, resource)` — Objekt, nicht nur Route | User A sieht Objekt von User B **in derselben** Org; fremde Org sowieso nie |
| Transaktion | `SET LOCAL app.org_id` (oder Äquivalent), nie Connection-weit im Pool | Context blutet in den nächsten Checkout |
| Datenbank | RLS `USING` + `WITH CHECK`, `FORCE ROW LEVEL SECURITY`, App-Role ohne `BYPASSRLS` | Eine vergessene WHERE-Klausel wird zum Datenleck |
| Cache / Suche / Queue / Object Storage | Key beginnt mit `org_id`, keine globalen IDs als Key | Redis/OpenSearch liefert fremde Treffer |
| Krypto (später / für Secrets und sensible Blobs) | Envelope-Encryption, Tenant-KEK, Context `org_id` | Gestohlener Ciphertext eines anderen Mandanten bleibt unlesbar |

IDs sind **UUIDs** (oder vergleichbar unvorhersagbar), keine fortlaufenden Zahlen in URLs.

Autorisierung prüft **Objektzugehörigkeit**, nicht „User ist eingeloggt“. Klassiker verbieten: `/api/overlays/123` ohne Org-Check, Suche über den ganzen Cluster, Webhooks ohne Tenant, Export-Jobs die „alle CSVs“ packen.

Cross-Tenant-Tests sind Pflicht, sobald Tenancy existiert: absichtliches Weglassen des Filters muss leer/deny liefern, nicht 500 mit Leak. Pen-Tests und automatisierte IDOR-Suiten gegen die öffentliche API.

### Kryptographie

- **Transit:** TLS 1.3, HSTS, moderne Ciphers, keine gemischten Inhalte.
- **Ruhe:** Speicher- und DB-Verschlüsselung (Plattform-KMS). Secrets und Kunden-Verbindungsdaten zusätzlich **anwendungsverschlüsselt** (Envelope: DEK pro Secret, KEK pro Org oder pro Secret-Typ).
- **Keys:** in KMS/HSM, Rotation, kein Key in Git, .env nur lokal im PoC.
- **BYOK / Customer-managed Keys:** Enterprise-Pfad, Architektur nicht verbauen (Key-Context ist die Org).
- **Passwörter:** wenn es sie gibt, nur tropische Hashes (Argon2id o. ä.), nie selbst gebautes Hashing. Zielbild ist eher SSO + Passkeys als Passwort-Berge.
- **Keine eigenen Crypto-Protokolle.** Keine ECB, keine selbst rollenden JWT-Verschlüsselungen, keine Tokens in LocalStorage wenn Cookies möglich sind.

ASVS und OWASP A04: kryptographische Fehler sind oft Datenlecks. Wir behandeln Query-Results, Schema-Samples und Overlay-Notizen als schützenswert, nicht nur „das Passwort“.

### Authentifizierung (B2B)

Passwort-only ist für ernstes B2B nicht das Zielbild.

Reihenfolge, in der wir denken (nicht alles am ersten Tag live):

1. **SSO (OIDC/SAML)** pro Organisation — Enterprise-Kunden verlangen das.
2. **Passkeys / WebAuthn** für Personen ohne SSO oder als Zweitfaktor.
3. **MFA** Pflicht, sobald Passwörter existieren; Org darf MFA erzwingen.
4. E-Mail-Magic-Link nur als schmale Ausnahme (Einladung), keine Dauer-Sessions daraus ohne Bindung.

Session:

- Kurzes Access-Token, rotierendes Refresh, Server-seitig widerrufbar.
- Cookies: `HttpOnly`, `Secure`, `SameSite=Lax` oder `Strict` je nach Flow; CSRF-Schutz wo `Lax` nicht reicht.
- Idle-Timeout und absolute Lifetime org-konfigurierbar (Default konservativ).
- Geräteliste, „überall abmelden“, Step-up für gefährliche Aktionen (neue Quelle, Credential rotieren, Org löschen, Massenexport, SSO-Config).
- Einladung: zeitlich begrenzt, an E-Mail gebunden, Rolle klar, kein „Link kennt die Org für immer“.

SCIM später für Provisioning/Deprovisioning. Wegfall eines IdP-Users = sofort kein Zugang, nicht beim nächsten Login-Versuch in drei Wochen.

Org-Policies (Enterprise): SSO-enforced (kein lokales Passwort), IP-Allowlist, Session-Dauer, Export-Verbote.

### Zugriffskontrolle in der Organisation

RBAC von Anfang an, auch wenn es erst wenige Rollen gibt. Keine impliziten Superuser in der App außer einem break-glass Betriebskanal außerhalb der Kunden-UI.

Erste Rollen (Namen später schärfen):

| Rolle | Darf grob |
| --- | --- |
| Owner | Org löschen, Billing, SSO, Residenz, Danger-Zone |
| Admin | Mitglieder, Rollen, Sicherheits-Policies, Quellen-Credentials |
| Analyst | Quellen nutzen, Modell reviewen, Queries, Reports |
| Viewer | Lesen, keine Accept/Reject, keine neuen Verbindungen |
| Billing | Rechnungen, Steuer-IDs — keine Quelldaten |

Custom Roles später, Datenmodell nicht auf drei hart kodierte Strings nageln.

Jede Mutation am Overlay ist einem Akteur zugeordnet (wer hat akzeptiert). Das ist Sicherheit *und* Nachvollziehbarkeit.

Öffentliche Links auf Reports: **Default aus.** Wenn es sie je gibt: zeitlich begrenzt, tokenisiert, widerrufbar, ohne Directory-Listing, niemals „wer den UUID kennt, sieht die Tabelle“.

### Kundendaten und Dritte

„Nicht über Dritte abrufbar“ heißt konkret:

- Kein Third-Party-JS in der authentifizierten App, das Kundendaten oder DOM mit Query-Results sehen kann (Analytics-, Chat-, Session-Replay-Snippets sind in der App **verboten**, solange sie Seiteninhalt lesen). Marketing-Site ≠ App.
- Keine LLM-/Support-Copilot-Calls mit Schema, Samples oder SQL des Kunden.
- Keine gemeinsamen Such-/CDN-Caches für HTML mit Mandanteninhalt. Statische Assets ja, personalisierte Antworten nein.
- Object Storage privat. Exports nur über kurzlebige, auth-geprüfte URLs.
- E-Mail enthält keine Tabelleninhalte, keine Connection-Strings, keine Magic-Admin-URLs.
- Subprozessoren (Hosting, E-Mail, Error-Tracking) nur mit AVV, Liste öffentlich, EU-Präferenz, Datenminimierung. Error-Tracking ohne Request-Bodies und ohne Samples.
- Mitarbeiterzugriff: Just-in-Time, begründet, zeitlich begrenzt, vollständig auditiert. Optional: Kunde sieht „wer hat wann auf die Org geschaut“. Default: niemand schaut.
- Staging/Dev kennen **keine** Produktionskundendaten. Fixtures und synthetische Daten, wie der PoC schon arbeitet.

Connector-Zugänge zur Kunden-DB:

- Bevorzugt **read-only** Rolle, Minimal-Schema.
- Credentials nur im Secret-Store, Anzeige nie im Klartext nach dem Speichern.
- Netzwerk: Egress kontrolliert; Kunden dürfen unsere Egress-IPs allowlisten.
- Timeouts, Row/Time-Budgets, kein unbegrenzt großes Materialisieren in *unsere* Disk ohne Tenant-Quota.
- Analyse-Samples in der UI sind für den Mandanten bestimmt; sie landen nicht in unseren Log-Aggregatoren.

Der Core arbeitet ohne LLM — das bleibt so für Kundendaten.

### Injection, XSS, CSRF, Integrität

- SQL nur über die Query-Schicht / Parameter; nie String-Bau mit User-Label, Tabellennamen aus der UI ohne Allowlist aus dem *eigenen* Schema-Katalog.
- HTML: escapen by default (oder ein Framework, das das erzwingt). Evidenz-Texte und Reason-Strings sind Daten, kein HTML.
- CSP streng (default-src self, keine `unsafe-inline` sobald praktikabel), Trusted Types anstreben.
- CSRF-Tokens oder SameSite+double-submit je nach Cookie-Modell.
- Upload/Export: Typ, Größe, Virenscan wo Dateien ankommen; Downloads `Content-Disposition` + korrekter MIME, kein HTML als CSV.
- Integrität: lockfiles, signierte Artefakte, keine ungeprüften Webhooks; Overlay-Writes authentisiert und autorisiert. OWASP A08 (Software/Data Integrity) und A03 (Supply Chain) getrennt behandeln: Build-Pipeline ≠ Laufzeit-Trust.

### Konfiguration, Fehler, Logging (OWASP A02, A09, A10)

A02 Misconfiguration ist in 2025 auf Platz 2 — bei uns heißen die Defaults:

- Debug aus, Stacktraces nie zum Client, Directory-Listing aus, Default-Passwörter unmöglich, Cloud-Buckets privat, Admin-Interfaces nicht im Internet ohne SSO+MFA.
- CORS: explizite Origins, kein `*`, keine Credentials an fremde Origins.
- Security-Header: HSTS, `X-Content-Type-Options`, `Referrer-Policy` streng, `Permissions-Policy` eng, Frame-Ancestors none (oder allowlist).

A10 (neu 2025): **Exceptional Conditions fail closed.** Timeout, Partial-Join, fehlender Tenant-Context, volle Disk, IdP down → Abbruch mit sicherem Zustand, keine halben Overlays, keine Requests „trotzdem durch“. Transaktionen atomar. Nach außen: stabile Fehlercodes; nach innen: genug zum Debuggen ohne Payload.

A09: Security-relevante Ereignisse werden geloggt **und** alarmiert. Logs ohne Alerting zählen nicht. Pflicht-Events (Beispiele): Login fail/success, MFA, Rolle geändert, Quelle angelegt, Credential gelesen/rotiert, Overlay published, Export, SSO-Config, JIT-Support-Zugriff, RLS/AuthZ-Deny in Menge, Egress zu neuem Host.

Anwendungslogs enthalten keine Passwörter, Tokens, Connection-Strings, keine Query-Result-Zeilen, keine E-Mail-Inhalte. `org_id` und `actor_id` ja, Samples nein.

Audit-Log (mandantensichtig für Admins): wer hat was wann an Modell, Mitgliedern, Quellen, Exports geändert. Append-only aus Sicht der App, eigene Retention, exportierbar für den Kunden.

### Rate Limits, Quotas, Missbrauch

Pro Organisation und pro Akteur: Login, API, Query-Ausführung, Analyse-Jobs, Exporte. Schutz vor Credential-Stuffing, teuren Full-Scans und einem Mandanten, der das Cluster auffrisst. Limits sind Sicherheits- und Fairness-Konstrukt, kein Billing-Feature (Billing darf dieselben Zähler später nutzen).

### Programm, nicht Hoffnung

- Threat Modeling für Connect, Review, Export, Invite, SSO — bevor die Features gebaut werden (A06 Insecure Design).
- Dependency- und Secret-Scanning in CI; Builds reproduzierbar; SBOM für Releases.
- Automatisierte Isolationstests + periodischer Pen-Test vor nennenswertem Kundenbetrieb.
- Incident: 72-Stunden-Pfad nach DSGVO Art. 33, Runbook, Statusseite ohne Kundendaten im Status-Text.
- Keine Security-durch-Obscurity: interne IDs, Feature-Flags und versteckte Routen sind keine Kontrolle.

---

## Locale-Stack (Sprache, Zeit, und alles Verwandte)

Sprache und Zeitzone sind zwei Achsen. Wer nur `i18n.t()` einbaut und UTC im UI anzeigt, hat das Problem nicht verstanden. Wir behandeln einen **Locale-Stack**: alles, was je nach Mensch, Organisation und Region anders ist, ohne die Fakten zu verdrehen.

### Sprachen (UI)

Von Anfang an **Deutsch und Englisch**. Weitere Sprachen kommen als zusätzliche Kataloge dazu, nicht als Sonderbau.

Regeln:

- Jeder nutzersichtbare Text liegt in Message-Katalogen (UI, Fehler, E-Mails, Toasts, Empty States, rechtliche Kurztexte in der App, System-Benachrichtigungen). Keine zusammengebauten Sätze aus Fragmenten (`"Hallo " + name` in einer Sprache, die die Reihenfolge dreht).
- **ICU MessageFormat** (Plural, Genus, Selektion). Zahlen und Daten nicht in den String hardcoden, sondern als Platzhalter formatieren.
- **Eine Änderung = alle aktiven Sprachen.** Wer Copy, Label, Fehlermeldung oder E-Mail anfasst, liefert in demselben Change Deutsch *und* Englisch (und jede später aktivierte Locale). Fehlende Keys sind ein CI-Fehler, kein stiller Fallback in Produktion ohne Alarm.
- In Entwicklung: fehlender Key schlägt hart fehl oder zeigt bewusst hässliche Markierung. In Produktion: nie Roh-Keys; Fallback-Sprache nur mit Telemetrie.
- Sprache hängt am **User**, nicht am Browser allein: Browser/Accept-Language nur für den ersten Vorschlag, dann gespeicherte Präferenz. Org-Default für Einladungen und systemische Mails, wenn die Person noch keine Präferenz hat.
- `html lang` und E-Mail-`Content-Language` folgen der wirksamen UI-Sprache.
- Produktbegriffe (Relationship, Overlay, Pack, …) gehören in ein **Glossar**: entweder bewusst unübersetzt in beiden Sprachen oder fest übersetzt — nicht gemischt. Das Glossar: `docs/glossary.md`.
- Engine-Ausgaben: der Core spricht keine UI-Sprache. Er liefert **stabile Reason-Codes + Parameter**; die Applikationsschicht übersetzt. Freitext aus Kunden-DBs (Spaltennamen, Werte) bleibt Original, mit korrektem Unicode und Richtung.
- Hilfevideos und Screenshots: mittelfristig sprachspezifisch; bis dahin Sprache der UI klar kennzeichnen, nicht so tun, als wäre ein deutsches Video englisch.
- Pseudo-Locale in Tests (`[!!! text !!!]`), damit fehlende Verdrahtung auffällt.
- Kein Verlass auf Browser-Auto-Translate der App.

Rechtstexte (AVV, AGB) sind Versionen mit Locale, nicht ein Google-Translate-Button.

### Zeitzonen

Speichern: **UTC** (oder Offset-aware Instant). Anzeigen: Zeitzone des Users (IANA, z. B. `Europe/Berlin`, `Asia/Tokyo`). Niemals Server-Localzeit als Wahrheit.

Zwei getrennte Uhren — das ist für Analytics essenziell:

| Uhr | Wofür | Wer setzt sie |
| --- | --- | --- |
| **Anzeige-Zeitzone** | „Overlay gespeichert um 15:04“, Login, Audit, Toasts mit Uhrzeit | User, Default aus Gerät/Browser beim ersten Setzen |
| **Reporting-Zeitzone** | „Umsatz heute“, Tages-/Monatsgrenzen, „letzte 7 Tage“, geplante Reports | Organisation (Admin). Nicht still aus dem letzten User ableiten |

DST und politische TZ-Updates (IANA-DB aktuell halten) gehören zum Betrieb. Recurring Schedules (falls später) speichern IANA-Zone + Lokalzeit, nicht „jeden Montag 09:00 UTC“.

Naive Timestamps aus Kundendatenbanken: nicht raten, ohne es zu sagen. Entweder deklarierte Quell-TZ (Overlay/Org-Default) oder Anzeige als „ohne Zone“, nie still auf UTC oder auf Berlin mappen.

Relative Zeit („vor 3 Stunden“) plus exaktes Datum/Zeit on hover oder in Details. APIs: ISO-8601 mit Offset oder `Z`.

### Locale (nicht dasselbe wie Sprache)

`de` ≠ `de-DE` ≠ `de-AT` ≠ `en-US` ≠ `en-GB`. Locale steuert Formate; Sprache steuert Wörter.

| Achse | Default-Richtung |
| --- | --- |
| Zahlen | `1.234,56` vs `1,234.56` — aus Locale, nicht aus UI-Sprache raten wenn beides gesetzt |
| Prozent, Einheiten | Locale; Dateigrößen konsistent (KiB vs KB festlegen und bleiben) |
| Datum | Numeric vs medium; nie `09/11/2026` ohne Locale (US vs Rest) |
| Uhrzeit | 24h in `de-*`, 12h in typischen `en-US`; User-Override erlaubt |
| Wochenstart | DE: Montag; US oft Sonntag — Reports „diese Woche“ folgen Org-Kalender, nicht dem Entwicklerlaptop |
| Kalenderwochen | ISO-8601 als Default (EU), Org-Override |
| Währung **Anzeige** | Symbol/Position aus Locale |
| Währung **Daten** | Aus Quelle/KPI, nie still in EUR umrechnen |
| Sortierung | Locale-Collation (`ä`, `ß`); DB-Collation nicht hard auf C lassen wo User sortieren |
| Namen | Display-Name als Feld, nicht `first + " " + last` als einziges Modell |
| Adresse / Telefon | Nur wo wir sie erheben (Billing); E.164 speichern, national formatieren |
| CSV/Excel-Export | Trennzeichen und Dezimal an Locale (DE: `;` und `,`) — sonst öffnet Excel Müll. Zusätzlich UTF-8 BOM wo Excel es braucht |
| Papier | PDF A4 Default (EU), Letter als Locale-/Org-Option |
| Fiscal Calendar | B2B: Geschäftsjahr ≠ Kalenderjahr; Org-Setting für Reports „dieses Jahr“ |

Quelle und UI nicht verwechseln: Spaltenwerte bleiben Daten. Formatiert wird am Rand (UI, Export, E-Mail).

### Residenz, Recht, Kalender der Organisation

Ähnlich „weltweit“ wie Sprache, aber auf der Org:

- **Datenregion:** Default EU. Ein Mandant klebt an einer Region; kein stilles Replizieren in die USA „für Latenz“.
- **Vertrag/Recht:** AGB, AVV, DPA, TOMs; Steuer (USt-IdNr., Reverse Charge) ist EU-B2B-Alltag, kein Nachtrag.
- **Geschäftskalender / Feiertage:** nur wenn „Arbeitstage“ in Reports vorkommen; dann Org-Kalender, nicht bayerische Feiertage hardcoden.
- **Arbeitszeiten / Ruhe** für Notifications: nicht um 03:00 Org-Lokalzeit pushen, wenn Quiet Hours gesetzt sind.

### Schrift, Unicode, Richtung

UTF-8 Ende-zu-Ende. Keine Latin-1-Annahme in CSV, Logs, Postgres-Client-Encoding.

RTL (AR, HE) ist nicht V1, aber: keine UI-Logik die `margin-left` als „Start“ meint, sobald wir Layout-Tokens setzen; `lang` und später `dir` nicht verbauen. Produkt-UI V1 bleibt LTR (DE/EN).

### Wie der Stack am Menschen hängt

Wirksame Werte, in dieser Reihenfolge wo nicht anders gesagt:

1. Explizite User-Präferenz
2. Org-Default (vor allem Reporting-TZ, Einladungsmails, Fiscal)
3. Sinnvoller Vorschlag aus Browser/OS beim **ersten** Besuch
4. Produkt-Fallback: UI **`de`** (entschieden 2026-09-16; Code: `Language::FALLBACK` in `app/src/locale.rs`), Reporting-TZ der Org, Speicher UTC

User, die reisen: Anzeige-TZ darf dem Gerät folgen, **wenn** sie „automatisch“ gewählt haben; Reporting-TZ der Org bleibt stehen. Sonst rutscht „Umsatz heute“ mit dem Flieger mit.

Einstellungen liegen an einem Ort (Account + Org-Sicherheit/Daten). Kein verstecktes `?lang=` als einzige Wahrheit; URL-Locale höchstens als Share-Hilfe, Session gewinnt.

---

## Bequemlichkeit (Konstrukt, nicht Chrome)

Pixel und Rails stehen in `docs/ui-ux-notes.md`. Hier nur, was das **System** tun muss, damit niemand an Kleinigkeiten trägt:

- **Richtige Defaults, wenig Fragen.** Sprache und Anzeige-TZ einmal vorschlagen, bestätigen, fertig. Reporting-TZ setzt ein Admin einmal.
- **Eine aktive Organisation**, sichtbar, wechselbar, ohne Datenleck zwischen Tabs.
- **Tiefe Links** in dieselbe Org und denselben Canvas-Ort (Review-Zeile, Report). Abgelaufene Session: nach Login zurück, nicht auf eine leere Home ohne Kontext.
- **Idempotente, umkehrbare Arbeit** wo das Overlay es hergibt: Accept/Reject/Edit bleibt die Wahrheit; versehentliches Reject ist kein Datenverlust in der Quelle.
- **Gefährliches ist schwer, Häufiges ist leicht.** Quelle löschen, SSO umstellen, Org löschen: Step-up und klare Sprache. Accept einer hochkonfidenten Kante: ein Schritt (siehe UI-Notizen).
- **Fehler sind handlungsfähig** und in der UI-Sprache: was passierte, was man tun kann, keine Generic-500-Philosophie nach außen, keine Stacktraces.
- **Warten ist erklärt.** Analyse und Queries brauchen Zeit; Fortschritt, Abbrechen, „Tab darf zu“ wenn Job serverseitig weiterläuft. Kein doppelter Submit.
- **Benachrichtigungen** in der User-Sprache, zur erlaubten Uhrzeit, mit Sprungziel, abschaltbar. Keine Mail für jedes Accept.
- **Suche und Kürzel** respektieren Locale (Collation) und Desktop-Konventionen; sie überschreiben keine assistiven Shortcuts.
- **Exporte öffnen** auf dem Rechner des Users (Excel DE/US). Downloads haben sprechende Namen inklusive Datum in der User-Locale.
- **Mehrere Mitglieder** an einem Modell: wer zuletzt published hat, ist sichtbar; Konflikt am Overlay nicht still überschreiben (Versionierung aus `docs/model-review.md`).
- **Wiederkehrende Geräte:** Session merken nach Org-Policy, nicht ewig.
- **Barrierefreiheit ist Bequemlichkeit.** Tastatur, Fokus, Screenreader-Texte aus denselben Katalogen, Kontrast, `prefers-reduced-motion` (UI-Notizen). Ziel **WCAG 2.2 AA**. In DE/EU zusätzlich den BFSG-/EAA-Pfad im Blick behalten — nicht darauf wetten, dass „nur B2B“ uns dauerhaft ausnimmt.
- **Offline/Flaky Netz:** ehrlicher Zustand, keine Geister-Saves. Die App ist Online-SaaS, kein versteckter Local-only-Modus der Tenancy bricht.
- **Hilfe in der Sprache der UI**, am Ort der Funktion (Empty States schon in den UI-Notizen).

Wir sollen uns nicht gegenseitig an Locale, Sicherheitshinweisen oder „wo ist meine Org“ erinnern müssen. Wenn etwas in DE existiert, existiert es in EN. Wenn eine Uhrzeit sichtbar ist, ist die Zone definiert. Wenn ein Datensatz geladen wird, ist die Org geprüft.

---

## Weitere essenzielle Konstrukte

### Identitäten der Personen vs. der Engine

Accounts sind Menschen (oder später Service Accounts). Sie sind nicht die `DataSource`. API-Tokens (falls Kunden-API) sind org-gebunden, scoped, rotierbar, im Klartext nur einmal sichtbar, jederzeit widerrufbar. Dieselben AuthZ-Regeln wie die UI — keine Hintertür-API.

### Control Plane vs. Data Plane

- **Control Plane:** Login, Org, Billing, Mitglieder, Locale, Policies, Secret-Metadaten.
- **Data Plane:** Profiling, Queries, Samples, Reports — strikt org-isoliert, eigene Quotas, eigene Verschlüsselung.

Jobs (Analyse, Export) laufen mit dem Mandantenkontext des Auslösers, nicht mit einem globalen Worker ohne RLS.

### Zuverlässigkeit und Weltweite Erreichbarkeit

- HTTPS überall, IPv4/IPv6, vernünftige Timeouts nah am User (Edge für statische Assets; APIs in der Datenregion der Org).
- Statusseite, Wartungsfenster in Org-Zeitzonen kommunizieren.
- Backups verschlüsselt, Restore **getestet**, Restore kennt Tenancy (kein Backup-File „alles in einem Klecks“ ohne Org-Restore).
- Abhängigkeit vom IdP: klarer Fehler, kein Fail-open.

### Observability ohne Leak

Metriken und Traces mit `org_id`, Latenz, Fehlerklasse. Payloads und SQL mit Werten nicht in SaaS-APM. Sampling von Traces darf keine Cell-Values enthalten.

### Collaboration innerhalb der Grenze

Mehrere Analysten einer Org sind das Normalbild. Presence/Live-Cursor ist optional später. Teilen **außerhalb** der Org ist ein Sicherheitsfeature mit Default-off, nicht ein virales Consumer-Pattern (Smartsheet-Sharing bewusst nicht übernehmen, siehe UI-Notizen).

### Billing (nur als Rand, damit das Modell nicht kollidiert)

Rechnung an die Organisation, EU-Rechnungspflichtfelder, Steuern. Nutzungsmessung darf keine Query-Inhalte speichern. Kündigen/Löschen: siehe Datenlebenszyklus. UI-Chrome für Billing ist nicht Teil der lokalen PoC-Notizen; das Datenmodell der Org darf Billing später tragen.

### Datenlebenszyklus (DSGVO)

- Verzeichnis der Verarbeitungstätigkeiten, Zweckbindung, Minimierung.
- Auskunft, Export (maschinenlesbar, org-scoped), Löschen inkl. Backups-innerhalb-der-Policy, Overlay und Secrets.
- Retention: Audit länger als Samples; Samples so kurz wie für Review nötig.
- Soft-Delete nur mit harter Frist und ohne, dass gelöschte Orgs über Support-Tools weiterleben.
- Sterbedaten/Accounts: Deprovisioning über Admin/SCIM.

### Recht und Vertrauen nach außen

Sobald echte Kunden: AVV, TOMs, Subprozessorliste, Security-Seite (Maßnahmen in Klartext, kein „military grade“), Meldeweg für Schwachstellen. Zertifizierungen (ISO 27001, SOC 2) folgen der Architektur, sie ersetzen sie nicht.

### Browser und Clients

Unterstützte Browser-Liste (letzte zwei Evergreen-Versionen). Kein IE. App-Origin ist nicht die Marketing-Origin (Cookies, CSP). Mobile ist dieselbe App (UI-Notizen), nicht ein zweiter Stack mit zweiten AuthZ-Bugs.

### Feature Flags

Immer mit `org_id`. Flags dürfen Isolation nicht umgehen („Flag on = Admin für alle“). Kill-Switch für Connect/Query, falls Egress-Missbrauch.

### Dokumente und Agenten

Dieses File ist die Quelle für Plattform-Fragen. UI-Fragen: `docs/ui-ux-notes.md`. Künftige Copy-Änderungen ohne zweite Locale sind unvollständig — analog zu „Catalogs grow with tests“ bei der Engine.

---

## Prinzipien (kurz)

1. **Organisation ist die Sicherheits- und Datengrenze.**
2. **Fail closed, Least Privilege, kein Standing Access auf Kundendaten.**
3. **Isolation in der Datenbank und in jedem Nebensystem (Cache, Queue, Bucket, Suche).**
4. **Kundendaten nicht an Dritte, nicht an LLMs, nicht in Logs, nicht in Marketing-Scripts.**
5. **ASVS als Prüfmaß, Top 10:2025 als Mindest-Awareness.**
6. **Core bleibt mandantenfrei; die App-Schicht erzwingt Tenancy.**
7. **UTC speichern, zwei Uhren anzeigen: User vs. Reporting.**
8. **DE und EN von Tag eins; jede Copy-Änderung in allen aktiven Sprachen.**
9. **Locale ≠ Sprache ≠ Zeitzone ≠ Residenz.** Alle vier existieren bewusst.
10. **Engine spricht Codes, UI spricht die User-Sprache; Kundendaten bleiben Original.**
11. **Bequemlichkeit durch Defaults, tiefe Links, ehrliche Fehler, WCAG 2.2 AA.**
12. **Öffentliches Teilen und Third-Party-Beobachter sind opt-in oder verboten, nie Default.**
13. **EU-Residenz Default; Subprozessoren namentlich und vertraglich.**
14. **Nichts davon im PoC vortäuschen — aber nichts bauen, das dem widerspricht.**

---

## Offene Fragen

- ~~Fallback-UI-Sprache: `de` (Firma/EU) oder `en` (weite B2B-Norm), wenn nichts gesetzt ist?~~ **Entschieden 2026-09-16: `de`.**
- Reporting-Zeitzone Pflichtfeld beim Org-Setup, oder Default `Europe/Berlin` mit Warnung?
- Naive DB-Timestamps: Org-Default-TZ, pro Quelle, oder immer „unbekannt“ bis der Mensch es setzt?
- Residenz V1 nur `eu-central` oder von Anfang an US-Region mit rechtlichem Split?
- Rollen: vier feste, oder früh ein Permission-Set, das Custom Roles billig macht?
- Passkeys-first vs. SSO-first für die ersten Design-Partner?
- Dürfen Viewer Exports — oft ein Leak-Pfad?
- Reason-Codes: Katalog im Core oder nur im Adapter? (Core muss testbar ohne Locale bleiben.)
- ~~Glossar: Produktwörter auf Deutsch übersetzen (`Beziehung`) oder englische Fachwörter behalten?~~ **Festgelegt in `docs/glossary.md`** (2026-09-16): Flächen- und Konstruktnamen bleiben englisch, alles andere fest übersetzt. Offen dort: Anrede Sie/du, Rollen-Labels.
- Support-JIT: mit oder ohne sichtbares Kunden-Audit im UI V1?
- Session: ein globales Account-Cookie plus Org-Switch, oder Token pro Org?
- CSV: immer Locale-Delimiter, oder zusätzlich „invariant CSV“ für Maschinen?
- BFSG-Pflicht für unser konkretes Angebot — rechtlich klären, technisch trotzdem AA bauen.
- Self-Host später: dieselben Isolationsgarantien, wer patched die Supply Chain?

---

## Nächster Schritt (wenn wir weiterreden)

Noch nicht implementieren (kein Auth im PoC, keine Tenancy im Core). Als Nächstes lohnt:

1. Org-/User-/Rollen-Modell auf einem Blatt festziehen (inkl. welche Objekte `org_id` tragen)
2. Isolationstabelle: DB, Cache, Jobs, Storage, Logs — ein Satz Fail-closed-Regeln
3. Locale-Stack: welche Felder am User, welche an der Org, Reporting- vs. Anzeige-TZ
4. Message-Katalog-Konvention (ICU, CI-Fail bei fehlendem Locale, Glossar)
5. Threat Model „Quelle verbinden“ (SSRF, Secrets, read-only, Samples)
6. Reason-Code-Schema, damit die Engine übersetzbar wird ohne HTML/i18n-Deps
7. Abgleich mit `docs/ui-ux-notes.md`: wo Account/Org/Sprache im Chrome sitzen, ohne den Canvas zu klauen
