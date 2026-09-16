# UI/UX-Notizen (erste Gedanken)

Status: Diskussionsnotizen, noch keine Implementierung.
Stand: 2026-09-15.

**Arbeitstitel: Octa.** Maskottchen/Logo: [`docs/branding/octa-logo.png`](branding/octa-logo.png). In der Top-Bar später Produktname + Oktopus; Illustrationen dürfen dieselbe Figur-Familie nutzen (Empty States, Meilensteine), nicht das Chrome überladen.

Bezieht sich auf die **künftige Produkt-App** (Chrome nach diesem File). Die heutige lokale PoC-UI (`web/` / `analytics-web`) ist Wegwerf und nicht die Branding-Basis; Landingpage/Marketing bleiben außen vor.

Referenz: Smartsheet-App-Screenshots (u. a. [SaaSUI Smartsheet-Galerie](https://www.saasui.design/application/smartsheet)). Gemeint ist das **Design der abgebildeten App**, nicht das Design der Galerie-Website selbst.

**Neubau, kein Weiterbauen.** Die heutige Review-UI (eine lange Seite, Sprungmarken, Papier/Serife, schmale `main`-Spalte, Karten untereinander) ist ein Wegwerf-Stand. Nichts davon — Layout, CSS, Seitenstruktur, Komponenten — dient als Basis. Neu gedacht wird das Chrome nach den Smartsheet-Bildern. Der Produktvertrag in `docs/model-review.md` bleibt (Accept/Reject/Edit, Evidenz, Overlay); die Oberfläche dafür wird neu entworfen.

---

## Absicht

Smartsheet ist Chrome-Vorbild, keine Produkt-Kopie. Festgehalten:

- globale Navigation links, ausklappbar
- globale Suche oben rechts in der hellen Top-Bar (gleiche Position wie bei Smartsheet)
- dünne horizontale Akzentleiste
- darunter eine **Seitenleiste** mit Buttons und Aktionen der geöffneten Seite
- rechte Leiste mit weiteren, seitenspezifischen Werkzeugen
- die eigentliche Arbeit (Quellen verbinden, Relationships festlegen, Modell reviewen) passiert **im Canvas**, nicht in Dialogen
- Dialoge nur für Nebensächliches
- neue User zuerst **leichte Handlungen mit sichtbarem Erfolg**, Komplexität erst danach; die Fläche nicht mit Funktionen überladen
- leere Arbeitsflächen erklären die jeweilige Funktion: Hilfevideo, Beispiel-Vorlage, vorgefertigter Bericht
- **nüchtern und professionell**, aber die App darf Spaß machen: positives UI-Feedback, Illustrationen an den richtigen Stellen

Zielbild: fester Rahmen, wechselnder Inhalt. Wer die App einmal gelernt hat, findet Suche, Nav und Seitenaktionen immer am selben Ort. Wer neu ist, sieht zuerst eine klare nächste Handlung und ein Ergebnis — nicht die ganze Mächtigkeit der Engine. Die Arbeit fühlt sich ernst an, der Erfolg darf sich gut anfühlen.

Desktop ist die Referenz für das Gerüst. Mobil gilt dasselbe System, nur **zusammengeklappt**: kein zweites Produkt, keine abgespeckte Mini-Seite. Auf dem Telefon zählt jede horizontale Pixelzeile für den Canvas — seitliches Chrome und große Ränder fallen weg.

---

## Was wir übernehmen / was wir weglassen

Übernehmen (Struktur und Verhalten):

- Mehrzonen-Layout: Left Rail, Top-Bar inkl. Suche, Akzentleiste, Seitenleiste, Canvas, Right Rail
- Icon-Rail, die zu einer beschrifteten Navigation aufklappt
- Globale Suche oben rechts, immer sichtbar
- Seitenspezifische Buttons in der Leiste zwischen Akzent und Canvas
- Rechte Leiste für Inspektor-/Nebenwerkzeuge der offenen Seite
- Klare Primäraktion, visuell getrennt vom Chrome

Nicht übernehmen:

- Auth, Sharing, Collaborators, Workspaces, Billing, Trial-Banner
- „Upgrade“, „Invite your team“
- Smartsheet-Branding und deren Solution-Center-Marketingkarten (fremde Illustrationen, Template-Theater)
- Wizards und Chart-Builder als Ersatz für die Canvas-Arbeit (siehe `docs/model-review.md`)
- Dialoge als Ort, an dem man Datenquellen aufsetzt oder Relationships/KPIs/Identities entscheidet

Visuelle Sprache: **professionell, nicht steril.** Chrome und Tabellen bleiben kühl und klar (Ausgangspunkt Smartsheet-Gerüst, nicht das aktuelle Papier-Theme). Spaß entsteht über Feedback und eigene Illustrationen — nicht über Spielzeug-Chrome oder ständig sichtbare Deko auf der Arbeitsliste.

Palette, Typo und Illustrationsstil später bewusst setzen; sie sollen zusammengehören.

---

## Chrome: Zonen

```text
┌────────┬──────────────────────────────────────────┬──────┐
│        │  Top-Bar     [Suche oben rechts]         │      │
│  Left  ├──────────────────────────────────────────┤ Right│
│  Rail  │  Akzentleiste (Status / Hinweis)         │ Rail │
│        ├──────────────────────────────────────────┤      │
│        │  Seitenleiste (Buttons der offenen Seite)│      │
│        ├──────────────────────────────────────────┤      │
│        │                                          │      │
│        │  Canvas — hier passiert die Arbeit       │      │
│        │                                          │      │
└────────┴──────────────────────────────────────────┴──────┘
              Dialog nur für Nebensächliches
```

Die Left/Right-Rails laufen über die volle Höhe. Top-Bar, Akzent, Seitenleiste und Canvas stapeln sich in der Mitte.

Das Diagramm gilt für **Desktop**. Mobil siehe [Mobile Ansicht](#mobile-ansicht): Rails nicht dauerhaft an den Seiten, Canvas kantenbündig.

### 1. Linke Leiste (global, ausklappbar)

Dunkle, schmale Icon-Leiste über die volle Höhe. Immer da, unabhängig von der Seite.

Beobachtung bei Smartsheet:

- oben: App-weite Einstiege (Home, Recents, Favorites, Browse)
- Mitte: primäre Erzeug-Aktion (`+`)
- unten: sekundäre Orte und Wechsel zwischen „Welten“
- ausgeklappt: zweite Spalte mit Labels, lokaler Suche, Kategorien

Für uns **App-Navigation**, nicht Arbeitsinhalt:

| Icon / Eintrag | Rolle |
| --- | --- |
| Quellen / Home | verbundene Quellen, neue Verbindung |
| Aktive Quelle | gerade geöffnete Quelle |
| Übersicht | Schema, Profil, Laufzeiten |
| Modell | Relationships, KPIs, Identities |
| Reports | vorgeschlagene Reports, Dashboard |
| Explore | Ad-hoc-Query gegen das published Model |
| (unten) Packs / Overlay | Dictionary-Packs, Overlay-Datei |

Ausgeklappt: Labels plus Badges (`12 pending`). Eingeklappt nur Icons.

Die Left-Rail listet keine einzelnen Relationships oder KPI-Zeilen. Das gehört in den Canvas.

### 2. Top-Bar und globale Suche

Helle Leiste ganz oben. Links Marke / Produktname, optional der Name der offenen Quelle. **Rechts die Suche**, wie bei Smartsheet: fest oben rechts, immer erreichbar, nicht in der Left-Rail versteckt und nicht als Extra-Seite.

Das Suchfeld ist ein zentrales Chrome-Stück, kein Nice-to-have. Position und Gewicht analog zum Smartsheet-Header (Magnifier + Eingabe, nicht nur ein Icon ohne Feld).

Was die Suche finden soll (erste Idee):

- Quellen und Verbindungen
- Tabellen und Spalten der offenen Quelle
- Relationships, KPIs, Identities (Sprung in den Canvas, Treffer markieren)
- Reports und Explore-Einstiege
- App-Orte (Übersicht, Modell, Reports)

Tastatur: Fokus per Shortcut (z. B. `/` oder `Ctrl+K`). Ergebnisse als kompakte Liste unter dem Feld, Enter springt hin. Keine Vollseite „Search“.

Zusätzlich denkbar, später: eine *lokale* Suche in der ausgeklappten Left-Rail (wie Smartsheets Solution-Center-Feld), gefiltert auf den Rail-Inhalt. Das ersetzt die globale Suche oben rechts nicht.

### 3. Horizontale Akzentleiste

Bei Smartsheet: kräftige Akzentfarbe unter dem weißen Header. Inhalt dort ist Marketing (Trial). Das Muster zählt, der Inhalt nicht.

Bei uns ein **dünner Statusstreifen** in der Akzentfarbe:

- geladene Quelle
- letzter Analyse-Lauf
- aktive Packs
- Review-Stand, z. B. `8 pending · 21 accepted · 2 rejected`
- kurze Hinweise (Re-Analyse nötig, Overlay gespeichert)

Keine Navigation, keine Formulare, keine Seiten-Buttons. Eine Zeile. Klickbar nur, wenn der Hinweis selbst ein Sprung ist (z. B. zu pending Reviews).

### 4. Seitenleiste (zwischen Akzent und Canvas)

Eigene horizontale Leiste, fest über dem Arbeitsbereich. Entspricht bei Smartsheet der Objekt-Toolbar (`File`, `Automation`, `Form`, Titel in der Mitte, Primäraktion rechts).

Hier liegen die **Aktionen der geöffneten Seite** als Buttons, Toggles, einfache Selects — nicht in der Left-Rail, nicht als Ersatz für den Canvas, nicht in einem Dialog versteckt.

Erste Aufteilung:

| Seite | Seitenleiste (Beispiele) |
| --- | --- |
| Quellen / Verbindung | Neue Verbindung, Verbindung testen, Trennen |
| Übersicht | Ansicht (Schema / Profil), Analyse neu laufen lassen |
| Relationships | Beziehung hinzufügen, ausgewählte akzeptieren / ablehnen, Band-Filter |
| KPIs | Label speichern, ausgewählte übernehmen / verwerfen |
| Identities | ausgewählte akzeptieren / ablehnen, Template-Filter |
| Reports | Report öffnen, Visualisierung wählen |
| Explore | Query ausführen, letzte Queries |

Mitte der Leiste: Seitentitel plus knappe Meta (`Relationships · 14`). Rechts: die eine Primäraktion der Seite, falls es sie gibt.

Filter, die die Canvas-Liste nur eingrenzen, dürfen hier als Chips/Buttons stehen. Tiefere Inspektor-Funktionen eher rechts.

### 5. Rechte Leiste (kontextuell)

Schmale helle Icon-Leiste. Icons ändern sich mit der Seite. Globale Nav bleibt links, Seiten-Buttons bleiben in der Seitenleiste, rechts ist **Inspektor und Nebenwerkzeug**.

| Seite | Rechte Leiste |
| --- | --- |
| Übersicht | Schema-Info, Overlay-Datei, Packs-Details |
| Relationships | Evidenz-Panel ein/aus, Sortierung, Notizen |
| KPIs | Label-Quelle, nur queryable |
| Identities | Scope, Match-Details |
| Reports | SQL zeigen, published vs. raw |
| Explore | Measure/Dimension-Hilfe, Copy-SQL |

4–8 Icons. Klick öffnet ein seitliches Panel (Canvas wird schmaler) oder — nur wenn es wirklich eine Nebenaktion ist — einen Dialog. Keine zweite globale Navigation. Keine Kernarbeit der Modellarbeit hier verstecken.

### 6. Canvas (der Arbeitsbereich)

Hier passiert das Aufsetzen und Entscheiden. Volle Restbreite zwischen den Rails, volle Resthöhe unter der Seitenleiste.

**Nicht in Dialogen, sondern im Canvas:**

- Datenverbindung anlegen und prüfen (Quelle wählen, Packs, Test, Schema sehen)
- Relationships sichten, festlegen, akzeptieren, ablehnen, ergänzen
- KPIs benennen, übernehmen, verwerfen
- Identities prüfen (Expression, Match, Sample) und entscheiden
- Reports und Explore-Ergebnisse lesen und steuern

Die Fläche ist eine Arbeitsfläche wie ein Sheet: Liste, Tabelle oder Graph der aktuellen Aufgabe, Evidenz *neben oder unter* dem Gegenstand, Aktionen an dem Gegenstand selbst oder in der Seitenleiste. Man bleibt im Kontext, scrollt, vergleicht, entscheidet.

Empty States leben ebenfalls im Canvas: was fehlt, was der nächste Schritt auf *dieser* Fläche ist.

### 7. Dialogfenster (nur Nebensächliches)

Das Smartsheet-Modal (zentriert, abgedunkelter Hintergrund, Titel, Cancel / Primäraktion) bleibt als *Muster* für seltene, abgeschlossene Mini-Aufgaben. Es ist **kein** Ort für den Kernflow.

Nicht im Dialog:

- Verbindung aufsetzen
- Relationships definieren oder reviewen
- KPI- und Identity-Entscheidungen
- Evidenz lesen, die man für Accept/Reject braucht
- Query bauen und ausführen

Denkbar im Dialog:

- destruktive Bestätigung (Overlay löschen, Verbindung entfernen)
- kurze Hilfe / „was bedeutet dieses Band“
- seltenes Export-Ziel wählen
- Toasts sind kein Dialog; kurzes Feedback nach Save bleibt Toast

Regeln, falls doch ein Dialog:

- eine kleine Aufgabe, kein Wizard
- kein Nested-Modal
- Abbrechen verwirft
- die Arbeit darunter bleibt sichtbar und ist der eigentliche Ort

---

## Weitere Komponenten

### Home / Quellen

Ausgeklappte Rail oder Secondary Nav: vorhandene Quellen, zuletzt geöffnet. Canvas: Karten oder Zeilen pro Quelle, plus eine Fläche „neue Verbindung“ — wieder im Canvas, nicht als Modal-Wizard. Liste und Verbindung sind nüchtern. Illustrationen gehören auf die **leere** Quellen-Fläche und auf den Erfolg nach dem ersten Öffnen, nicht als Marketing-Raster auf jede Quellenkarte.

### Tabellen

Dicht, klebrige Header, numerisch tabellarisch. Schema, Explore-Results, Review-Listen dürfen sich wie Arbeitslisten anfühlen, nicht wie Artikel. Keine Illustrationen in Tabellenzeilen — hier zählt Dichte.

### Positives Feedback (Toasts, Mikro, Erfolg)

Die App soll sich nach einer richtigen Handlung **belohnen**, ohne laut zu werden.

- **Toasts:** kurz nach Speichern, Accept, Query, erstem Bericht. Erfolgston (Farbe, Häkchen, eine Zeile Text), dann weg. Die Akzentleiste bleibt Status, kein Toast.
- **Am Gegenstand:** Accept färbt die Zeile/Karte klar, Reject bleibt sichtbar aber ruhig, Pending wirkt „noch offen“. Der Unterschied muss sich lohnen — das ist das Erfolgserlebnis nach der leichten Handlung.
- **Mikro:** Button-Press, Fokus, Einblenden eines ersten Charts — kurze, präzise Bewegung, kein Bouncy-Chaos. Reduced-motion respektieren.
- **Erste Meilensteine** dürfen etwas mehr: eigene Illustration plus Satz, wenn die erste Quelle steht, der erste Bericht sichtbar ist, oder das Modell die ersten Accepts hat. Danach wieder nüchtern arbeiten.
- Fehlerfeedback ebenso klar, aber ohne Drama: was schiefging, was man tun kann.

Kein Dauerfeuer. Feedback folgt der Handlung, ersetzt sie nicht, und überdeckt nie Evidenz.

### Illustrationen

Erwünscht — als **eigene Bildsprache**, nicht als Smartsheet-Kopie. Sie machen leere Flächen und Erfolge menschlich; sie dekorieren nicht die ganze App.

Wo ja:

- Empty States (Hilfe zur Funktion)
- Erfolg nach der ersten leichten Handlung / Meilenstein
- Hilfe-Einstiege (Video, Vorlage, vorgefertigter Bericht) als kleine Szene, nicht als Icon-Wüste
- optional: ruhiges Spot-Motiv auf der leeren Startfläche

Wo nein:

- Left/Right-Rail, Akzentleiste, Suchfeld
- Tabellen, Relationship-Listen, SQL, Evidenzblöcke
- jede Quellenkarte im vollen Home-Raster (kein Template-Store-Look)
- Illustration statt Inhalt, sobald Daten da sind

Stil-Richtung (festzuhalten, noch nicht final): klar, freundlich, zum professionellen Chrome passend. Eine Familie (Strich, Fläche, Farbe), wiedererkennbare Figuren oder Metaphern (Quelle, Beziehung, Bericht) — kein Clipart-Mix, kein Meme, kein Konfetti auf jeder Aktion.

---

## Informationsarchitektur (neu, nicht aus der alten UI abgeleitet)

Nur Richtung, noch keine Routes in Stein:

| Ort | Canvas | Seitenleiste | Rechts |
| --- | --- | --- | --- |
| Quellen | verbinden, vorhandene Quellen | Neue Verbindung, Testen | — |
| Übersicht | Schema, Stats, Timings | Ansicht, Re-Analyse | Info, Overlay, Packs |
| Modell / Relationships | Kanten + Evidenz, Entscheiden | Hinzufügen, Accept/Reject, Filter | Evidenz-Panel, Sort |
| Modell / KPIs | Kandidaten + Labels, Entscheiden | Übernehmen, verwerfen | Label-Quelle |
| Modell / Identities | Formeln + Güte, Entscheiden | Accept/Reject, Templates | Scope |
| Reports | Karten, Charts, Vorschläge | Öffnen, Viz | SQL |
| Explore | Query + Result | Ausführen | Copy-SQL, Hilfe |

Suche oben rechts springt in diese Orte bzw. markiert Treffer im Canvas.

Review-Inhalt bleibt gebunden an `docs/model-review.md`: Konfidenz, Reason, Evidence, Rejected sichtbar. Neu ist nur, *wo* das im Chrome sitzt — nämlich dauerhaft im Canvas.

---

## Visuelle Beobachtungen (Smartsheet-App)

Zum späteren Theming, nicht als Pflicht-Palette:

- Left Rail: sehr dunkles Navy, weiße Icons, aktiver Zustand etwas heller
- Suche: helles Feld oben rechts, dezentes Icon, volle Header-Höhe
- Akzent: satte Farbe als dünnes Band
- Seitenleiste: hell, flache Text/Icon-Buttons, Primäraktion rechts eigenfarbig (dort oft Grün)
- Canvas: Weiß / helles Grau
- Rechte Rail: hell, icon-only
- Modal (falls): weiß, weicher Schatten, klarer Footer — bei uns selten
- Typo: neutrale UI-Sans
- Abstände in den Rails gleichmäßig

Unsere Illustrationen und das positive Feedback sind **kein** Smartsheet-Import. Das Gerüst bleibt; Ton und Bildsprache sind eigen.

---

## UX-Prinzipien

1. **Chrome stabil, Inhalt wechselt.**
2. **Suche immer oben rechts.** Global, gleiche Ecke wie die Referenz (Desktop: Feld, Mobil: Icon plus vollbreites Overlay).
3. **Seitenaktionen in der Seitenleiste**, zwischen Akzent und Canvas.
4. **Kernarbeit im Canvas.** Verbindungen, Relationships, KPIs, Identities werden dort aufgesetzt und entschieden, nicht in Modals.
5. **Global links, Inspektor rechts** — am Desktop. Mobil: globale Orte in der Bottom-Nav, Inspektor als Sheet.
6. **Erklärung am Gegenstand.** Evidenz liegt im Arbeitsbereich, nicht hinter einem Dialog.
7. **Eine Primäraktion** rechts in der Seitenleiste, wenn die Seite eine hat.
8. **Kein SaaS-Ballast.** Keine Share-/Account-Flächen.
9. **Overlay bleibt Quelle der Wahrheit.** `accepted / rejected / edited / pending`, nichts still löschen.
10. **Keine Altlasten.** Alte UI nicht „migrieren“, sondern ersetzen.
11. **Mobil: Breite dem Canvas.** Keine dauerhaften Seiten-Rails, keine Desktop-Ränder. Chrome stapelt oder liegt als Overlay, die Arbeit nutzt die volle Viewport-Breite.
12. **Confidence vor Mächtigkeit.** Leichte erste Handlungen, sichtbarer Erfolg, Komplexität erst wenn die Basis steht. Oberfläche nicht mit Funktionen überladen.
13. **Leere Fläche = Hilfe zur Funktion.** Video, Beispiel-Vorlage oder vorgefertigter Bericht — im Canvas der jeweiligen Seite, kein globaler Wizard.
14. **Professionell, mit Freude.** Chrome und Daten nüchtern; Erfolg und leere Flächen mit klarem Feedback und Illustration. Spaß ohne Spielzeug-UI.

---

## Confidence, Komplexität, leere Flächen

Neue User sollen schnell Sicherheit aufbauen: **kleine Handlung, großes Erfolgserlebnis.** Die Engine kann viel (Profil, Relationships, Identities, KPIs, Queries, Reports). Davon darf die erste Sitzung nur den Teil zeigen, der sofort ein Ergebnis liefert. Fortgeschrittene Werkzeuge bleiben erreichbar, aber nicht alle gleichzeitig sichtbar.

Das ist kein Onboarding-Wizard und kein Ersatz für die Canvas-Arbeit. Es ist die Reihenfolge, in der die Oberfläche Fähigkeiten anbietet.

### Leicht zuerst, komplex später

Erste Sitzung (wenig Klicks, klares Ergebnis):

1. Eine **Beispiel-Quelle** öffnen oder eine vorbereitete Verbindung wählen — nicht sofort eigene Datenbanken verdrahten.
2. Im Canvas etwas **Fertiges sehen**: Übersicht, ein paar KPI-Karten, ein erster Bericht. Das ist das Erfolgserlebnis — sichtbar, mit positivem Feedback, gern mit Illustration am Meilenstein.
3. Eine **leichte Entscheidung**: z. B. eine hochkonfidente Relationship akzeptieren, einen vorgeschlagenen Bericht öffnen. Ein Tap, Modell oder Report reagiert sichtbar.

Danach, wenn eine Quelle und ein erstes Ergebnis da sind:

- unsichere Relationships mit Evidenz prüfen
- KPIs umbenennen, Identities annehmen oder verwerfen
- eigene Verbindung aufsetzen, Packs wählen, Explore-Queries

Noch später / hinter Overflow oder Right-Rail:

- SQL, Overlay-Datei, feine Filter, Template-Details, Re-Analyse-Optionen

Die Seitenleiste zeigt auf einer leeren oder „noch neu“-Fläche **eine** Primäraktion plus Hilfe. Zusätzliche Buttons erst, wenn sie auf vorhandenen Inhalt wirken.

### Nicht überladen

- Pro Seite eine offensichtliche nächste Handlung, nicht acht gleichwertige.
- Fortgeschrittenes hinter Overflow (`⋯`), Right-Rail / Sheet, oder erst nach dem ersten Erfolg.
- Chrome (Suche, Nav, Akzent) bleibt; die *Funktionsdichte im Canvas* startet schmal.
- Keine permanente Feature-Tour, keine Tooltips auf jedem Icon. Hilfe sitzt dort, wo die Fläche leer ist, und kann später weg.

### Empty States — Hilfe an der Funktion

Eine leere Arbeitsumgebung ist kein totes „Noch nichts vorhanden“. Jede Funktion, die noch keinen Inhalt hat, erklärt sich **auf dieser Fläche** (Canvas, volle Breite). Drei gleichwertige Einstiege, soweit sie zur Seite passen:

| Einstieg | Wofür | Beispiel |
| --- | --- | --- |
| **Hilfevideo** | kurz verstehen, was diese Seite tut | „Relationships in 60s“, „Ersten Bericht lesen“ |
| **Beispiel-Vorlage** | mit Daten üben, ohne eigene Quelle | vorhandene Fixtures / Beispielmodelle als Vorlage wählen |
| **Vorgefertigter Bericht** | sofort ein Ergebnis sehen | Engine-Vorschlag oder Katalogbericht auf der Beispielquelle öffnen |

Nicht alle drei auf jeder Seite erzwingen:

- **Quellen leer:** Video „Quelle verbinden“ + Beispiel-Vorlage (Fixture wählen). Kein Bericht, solange keine Quelle da ist.
- **Modell / Relationships leer:** Video + Vorlage, die schon Kanten enthält; Erfolg = Liste mit Accept.
- **Reports leer:** Video + **vorgefertigte Berichte** auswählen (und optional Vorlage mit Daten). Das ist der schnellste sichtbare Gewinn nach einer Quelle.
- **Explore leer:** Video + eine Beispiel-Query ausführen, nicht ein leeres Formular.

Regeln für diese Empty States:

- Sie leben im Canvas der jeweiligen Funktion, nicht als App-weites Modal.
- Eine Illustration zur Funktion (nicht generisches Clipart) plus kurzer Satz: was die Fläche ist, was der nächste leichte Schritt ist, was man danach sieht.
- Video startet nicht automatisch; Klick/Tap.
- Vorlage und vorgefertigter Bericht sind echte Handlungen mit Ergebnis, keine Demos hinter Glas.
- Sobald eigener Inhalt da ist, verschwindet der Empty State samt großer Illustration. Hilfe bleibt dezent auffindbar (z. B. `?` in der Seitenleiste), ohne die Fläche wieder zu füllen.
- Mobil dieselben drei Einstiege, gestapelt, volle Breite — Illustration oben, Aktionen darunter, ohne seitliche Luft.

Das widerspricht `docs/model-review.md` (keine Wizards als Ersatz für Review) nicht: Vorlagen und Berichte **füllen den Canvas**, sie ziehen die Arbeit nicht in einen Dialog.

---

## Mobile Ansicht

Gleiche Orte, gleiche Aufgaben, gleiches Overlay. Mobil ändert sich nur, **wo das Chrome liegt** — nicht, dass Kernarbeit im Canvas stattfindet.

Ziel auf dem Telefon: **keinen Platz an den Seiten verschenken.** Der Arbeitsbereich geht von Kante zu Kante. Persistent Left-Rail + Right-Rail wie am Desktop würde genau die Breite stehlen, die Tabellen, Relationship-Zeilen und Charts brauchen.

### Breakpoints (Richtung)

Übliche Stufen, später an echten Layouts schärfen:

| Stufe | Breite (Richtwert) | Chrome |
| --- | --- | --- |
| Telefon | bis ~767px | kein seitliches Rail; Bottom-Nav; Suche als Icon; Inspektor als Sheet |
| Tablet | ~768–1023px | optional schmale Icon-Rail links, wenn sie den Canvas nicht einengt; rechts weiterhin Sheet statt zweiter Spalte |
| Desktop | ab ~1024px | volles Gerüst wie oben |

Nicht die Desktop-Seite skalieren. Komponenten umbrechen nach Best Practice: Nav an den Daumen, Overlays vollbreit, Touch-Ziele statt Hover.

### Chrome-Übersetzung

```text
┌─────────────────────────────────────┐
│  Top-Bar   [Titel]        [Suche]   │
├─────────────────────────────────────┤
│  Akzent (eine kompakte Zeile)       │
├─────────────────────────────────────┤
│  Seitenleiste (eine Zeile, Overflow)│
├─────────────────────────────────────┤
│                                     │
│  Canvas — volle Breite              │
│                                     │
├─────────────────────────────────────┤
│  Bottom-Nav (globale Orte)          │
└─────────────────────────────────────┘
     Sheet / Vollfläche von unten
```

| Desktop | Mobil |
| --- | --- |
| Left-Rail, dauerhaft | **Bottom-Navigation** für die Hauptorte (Quellen, Übersicht, Modell, Reports, Explore). Rest unter „Mehr“. Kein Hamburger als einzige Nav — der versteckt Orte und lässt seitlich nichts frei, das der Canvas bräuchte. |
| Top-Bar + sichtbares Suchfeld | Top-Bar bleibt oben. **Suche bleibt oben rechts** als Icon; Tap öffnet ein **vollbreites** Such-Overlay (Feld + Trefferliste über die ganze Breite). Kein schmales Feld mit Rändern links/rechts. |
| Akzentleiste | Bleibt, aber eine Zeile, Text kürzt mit Ellipse. Details per Tap auf die Leiste, nicht als zweite Statusspalte. |
| Seitenleiste | Bleibt direkt über dem Canvas. Eine Zeile, Primäraktion sichtbar, weitere Aktionen hinter Overflow (`⋯`). Chips horizontal scrollen statt umzubrechen (kein dreizeiliger Toolbar-Block). |
| Right-Rail | Nicht als Spalte. Icon in der Seitenleiste oder Top-Bar öffnet ein **Bottom-Sheet** bzw. eine Vollfläche (Evidenz, SQL, Packs). Canvas darunter bleibt der Entscheidungsort; das Sheet ist Inspektor, kein Modal-Wizard für Relationships. |
| Zentriertes Dialog | Seltene Nebenaktion: **Bottom-Sheet** oder Vollfläche, kantenbündig. Kein kleines zentriertes Fenster mit großen abgedunkelten Seitenrändern. |
| Canvas zwischen zwei Rails | Canvas **100% der Viewport-Breite** unter dem Chrome, über der Bottom-Nav. |

Die Bottom-Nav nimmt vertikale Höhe, nicht horizontale. Das ist der Tausch: unten ein Streifen, links und rechts null.

### Kein Platz an den Seiten

Konkret:

- Kein zentrierter Desktop-`max-width` (keine 960px-Spalte mit Grau daneben).
- Keine großen horizontalen Gutters. Text und Formulare: etwa 16px zum Rand, angeglichen an `safe-area-inset-left/right`. **Daten** (Tabellen, Review-Listen, Charts) dürfen bündig an den Viewport — interne Zellen-Padding statt Außenrand.
- Keine dauerhafte linke oder rechte Icon-Spalte auf dem Telefon.
- Overlays (Suche, Inspektor, Bestätigung) ebenfalls vollbreit, nicht als schmale Karte in der Mitte.
- Querformat: weiterhin kantenbündig; Bottom-Nav darf schmaler werden, Rails nicht „halb Desktop“ einschalten, nur weil die Breite knapp über einem Breakpoint liegt.

Tablet: eine schmale Left-Rail ist erlaubt, wenn der Canvas danach immer noch die Fläche füllt. Eine zweite rechte Spalte nur, wenn genug Breite für Tabelle *und* Inspektor bleibt; sonst Sheet.

### Canvas und Inhalte

Kernarbeit bleibt im Canvas (Verbindungen, Relationships, KPIs, Identities) — auch mobil kein Ausweichen in Dialog-Wizards.

Listen und Karten: eine Spalte, volle Breite. Evidenz unter dem Gegenstand stapeln, nicht in einer zweiten Spalte daneben.

Tabellen: horizontales Scrollen *innerhalb* der Tabelle, klebrige erste Spalte wenn sinnvoll. Die Seite selbst scrollt nicht seitlich (kein `body` mit Overflow-X).

Primäraktionen am Listeneintrag: große Touch-Flächen, nicht nur winzige Textlinks. Accept/Reject in der Zeile oder in der Seitenleiste für die Auswahl — nicht hinter einem Modal.

Empty States: volle Canvas-Breite, wie unter [Confidence, Komplexität, leere Flächen](#confidence-komplexität-leere-flächen) — Video, Vorlage, vorgefertigter Bericht gestapelt, ohne seitliche Luft.

### Suche, Tastatur, Touch

- Such-Icon oben rechts (gleiche Ecke wie Desktop). Overlay: Feld oben kantenbündig, Treffer darunter, nicht als eigene App-Route.
- Inputs mindestens 16px Schrift (kein iOS-Zoom beim Fokus).
- Touch-Ziele mindestens ~44×44px.
- Keine Hover-only-Zustände: Filter, Evidenz, Overflow müssen tap-fähig sein.
- Viewport: `width=device-width`, `viewport-fit=cover`. Home-Indicator und Notch über `safe-area-inset-*` — Bottom-Nav und Top-Bar in den Safe Areas, Canvas dazwischen.
- Tastatur-Shortcuts (`/`, `Ctrl+K`) bleiben Desktop; mobil öffnet die Suche über das Icon.

### Best Practices, an denen wir uns halten

Nicht jedes Pattern erfinden. Orientierung:

1. **Content first, Chrome second.** Was die Aufgabe ist, bekommt die Breite; Nav und Inspektor sind ein- und ausblendbar.
2. **Daumenreach.** Globale Orte unten, Suche und Overflow oben, Primäraktion der Seite in der Seitenleiste oder am unteren Canvas-Rand über der Nav — nicht nur oben links.
3. **Ein Navigationsmuster.** Bottom-Nav *oder* (für „Mehr“) eine Liste in einem Sheet. Nicht gleichzeitig Hamburger, Tabs und eine zweite Icon-Leiste.
4. **Progressive Disclosure.** Filter und Evidenz aufklappen, nicht alle Desktop-Tools gleichzeitig in die erste View pressen.
5. **Sheets statt seitlicher Panels.** Material/iOS-üblich für sekundären Kontext; Wischen zum Schließen.
6. **Konsistente Breakpoints.** Dieselben Stufen in Layout und Typo; kein willkürliches Umbrechen einzelner Buttons.
7. **Gleiche Information, andere Dichte.** Konfidenz, Reason, Evidence bleiben sichtbar — gestapelt, nicht weggelassen.

### Was wir mobil nicht tun

- Desktop-Layout mit `transform: scale` oder winzigen Rails links und rechts
- Große seitliche Margins „für Luft“
- Kernflow (Verbindung, Relationships) in Vollbild-Wizards, die den Canvas ersetzen
- Suche als eigene Seite ohne Bezug zum aktuellen Ort
- Bottom-Nav mit mehr als etwa fünf sichtbaren Orten (Rest hinter Mehr)

---

## Offene Fragen

- Relationships, KPIs, Identities: eine Modell-Seite mit Umschalter in der Seitenleiste, oder drei Orte in der Left-Rail?
- Rechte Leiste immer nur Icons, oder darf sie zu einem festen Evidenz-Panel aufklappen (Canvas bleibt trotzdem der Entscheidungsort)?
- Suche: nur Sprungmarken, oder auch Aktionen („accept this relationship“) aus den Treffern?
- Visuelle Linie: näher an Smartsheet (navy/blau/sans) oder eigene Palette auf demselben Gerüst?
- Quellen verbinden: eine eigene Canvas-Seite mit Schritten *auf der Fläche*, oder eine Quelle-Detailansicht, die Verbindung und Schema zusammen zeigt?
- Bottom-Nav: welche fünf Orte sichtbar, was hinter „Mehr“ (Packs/Overlay)?
- Tablet: ab wann eine Left-Icon-Rail wieder dauerhaft sein darf, ohne den Canvas zu erdrücken?
- Welches Erfolgserlebnis ist das erste der App (Beispielquelle + KPI-Karten vs. erster vorgefertigter Bericht)?
- Hilfevideos: wo liegen sie (lokal, Links), wie kurz, eine Datei pro Funktion?
- Wann gilt eine Fläche als „nicht mehr leer“ — schon bei Engine-Vorschlägen, oder erst nach der ersten User-Entscheidung?
- Illustrationsstil: eine Figur/Metapher-Familie, wie farblich an den Akzent gebunden, wie „laut“ der erste Meilenstein sein darf?

---

## Nächster Schritt (wenn wir weiterreden)

Noch nicht implementieren. Als Nächstes lohnt:

1. IA festziehen (Left-Rail-Orte, was in die Seitenleiste vs. nach rechts vs. in den Canvas)
2. Wireframe: Top-Bar inkl. Suche, Akzent, Seitenleiste, Canvas-Arbeit an Relationships (ohne Modal)
3. Wireframe: Quelle verbinden als Canvas-Flow
4. Theme gegen das Gerüst halten
5. Mobil-Wireframe derselben Flows: volle Breite, Bottom-Nav, Suche-Overlay, Evidenz als Sheet
6. Empty-State-Wireframes: Quellen, Relationships, Reports — Illustration plus Video / Vorlage / vorgefertigter Bericht, eine leichte Primäraktion
7. Feedback-Momente skizzieren: Accept-Zustand, erster Bericht, Toast — nüchtern vs. Meilenstein
