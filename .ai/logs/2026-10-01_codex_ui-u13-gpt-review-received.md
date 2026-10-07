# U13 independent GPT review received — acceptance pending

Provenance: read-only report returned by separate Hermes session `20261001_185328_b549c4` (GPT-6.1-Sol), message `87018`. Reviewed revision: `53aefe4ba0f71df9f30e853604f2b71b270d6a82`. Received 2026-10-01 19:52 CEST. This is the reviewer's report, not a fresh reproduction by the UI implementation session. Its execution totals are reported evidence and must be reverified during fixes/integration. The originally selected Claude review has not been replaced by user decision. No U13 closure or checkbox change follows this intake.

---

# U13-Reviewurteil: **Änderungen erforderlich**

Geprüfter Stand: `origin/main` **`53aefe4ba0f71df9f30e853604f2b71b270d6a82`**, zuletzt mit dem Remote abgeglichen am **1. Oktober 2026, 19:24 CEST**. Die Prüfung erfolgte im isolierten Review-Worktree; dessen versionierter Arbeitsbaum blieb unverändert. Keine Commits, Firewalländerungen oder produktiven KNX-Operationen.

Die vorhandenen Tests sind grün. **Drei zusätzliche Offline-Reproduktionen zeigen jedoch freigaberelevante Zustandsfehler**, die diese Tests nicht abdecken. Grün ist hier leider noch kein Freifahrtschein.

## 1. Befunde nach Schweregrad

### P1 — Nach Gerätewechsel können Änderungen das vorherige Gerät treffen

**Stellen:**
`apps/knx-web/src/App.tsx:488–518`
`apps/knx-web/src/Inspector.tsx:194–214`
`apps/knx-web/src/Inspector.tsx:958–973`

**Ursache:** Beim Wechsel von Gerät A zu B wird die Auswahl sofort geändert, `deviceDetail` aber nicht vor dem neuen GET geleert. Bis die Antwort für B eintrifft, bleiben die editierbaren Felder von A verfügbar. Deren Schreibaufrufe verwenden weiterhin die ID aus dem alten Detailobjekt.

**Auswirkung:** Der Benutzer kann B ausgewählt haben und dennoch eine lokale Projektänderung an A auslösen.

**Beleg:** Offline-Reproduktion mit dem tatsächlichen `App`-Code, React/happy-dom und vollständig gemockten APIs:

- Gerät A geladen.
- Gerät B ausgewählt; dessen Detailantwort bewusst offen gehalten.
- B war ausgewählt, der Inspector zeigte weiterhin A mit editierbaren Feldern.
- Eine Beschreibungsänderung erzeugte einen gemockten Schreibaufruf für **Gerät A, ID 1**.

Dies belegt eine falsche Projektänderungszuordnung, **keinen Hardwarewrite**.

**Erforderlich:** Beim Auswahlwechsel alte Details sofort invalidieren beziehungsweise Editoren sperren. Zusätzlich sollte die Darstellung die Übereinstimmung von Auswahl-ID und Detail-ID erzwingen. Regression: A laden → B auswählen → B-GET verzögern → kein Schreibaufruf für A möglich.

---

### P1 — Undo/Redo aktualisiert die angezeigten Parameterwerte nicht

**Stellen:**
`apps/knx-web/src/ParameterPanel.tsx:329–343`
`apps/knx-web/src/Inspector.tsx:909–917`
`apps/knx-web/src/App.tsx:568–580`

**Ursache:** Das Parameterpanel lädt ausschließlich bei Änderungen von `deviceId` oder Produktsprache. Nach Undo/Redo werden Projektsnapshot und Gerätedetails aktualisiert, das weiterhin gemountete Parameterpanel erhält aber kein Invalidierungssignal.

**Auswirkung:** Der Parametereditor kann eine andere Konfiguration anzeigen als das autoritative Projekt. Die korrekte Core-Undo-Funktion wird dadurch in der UI falsch dargestellt.

**Beleg:** Offline-Reproduktion am tatsächlichen Komponentenverbund:

| Zustand | Autoritativer Parameterwert | Angezeigter Wert | Parameter-GETs |
|---|---:|---:|---:|
| Vor Undo | 6 | 6 | 1 |
| Nach neuem Undo-Snapshot | 5 | **6** | **1** |

Ein Geräte- oder Sprachwechsel ist derzeit nötig, um diese Anzeige neu zu laden.

**Erforderlich:** Parameterdaten nach relevanten autoritativen Projektänderungen invalidieren und erneut laden; dabei bestehende Stale-Response-Guards erhalten. Regressionen müssen Undo **und** Redo bei unverändert ausgewähltem Gerät abdecken.

---

### P1 — Ein laufender Autosave kann sich nach dem Abschalten wieder aktivieren

**Stellen:**
`apps/knx-web/src/useAutosave.ts:141–145`
`apps/knx-web/src/useAutosave.ts:187–209`
Testlücke: `apps/knx-web/src/useAutosave.test.tsx:250–266`

**Ursache:** Das Abschalten beziehungsweise Effect-Cleanup entfernt vorhandene Timer. Ein bereits laufendes `runSave()` ruft nach seinem `await` jedoch im `finally` bedingungslos die alte `scheduleNextCycle()`-Closure auf. Diese prüft weder die Gültigkeit der Effect-Generation noch den aktuellen Aktivierungszustand.

**Auswirkung:** Autosave kann nach dem Abschalten erneut Countdowns starten und bei einem weiterhin oder erneut geänderten Projekt weitere Saves ausführen. Das widerspricht der ausdrücklich dokumentierten Bedeutung von `enabled: false`.

**Beleg:** Offline-Reproduktion mit dem tatsächlichen Hook und injizierten Timern:

1. Erster Save läuft: keine Timer mehr.
2. Autosave deaktiviert: weiterhin keine Timer.
3. Erster Save abgeschlossen: **ein neuer Timer entsteht**.
4. Bei weiterhin geändertem Projekt erfolgt **ein zweiter Save trotz deaktiviertem Autosave**.

Der vorhandene Abschalttest prüft nur einen laufenden **Countdown**, nicht einen bereits laufenden **Save**.

**Erforderlich:** Nach asynchronem Abschluss nur aus einer noch gültigen, aktivierten Effect-Generation neu planen. Regressionen für Disable, Unmount und Intervallwechsel während eines offenen Save-Promises ergänzen.

## 2. Bewertung der beiden ISSUE-12-Abnahmefelder

### Feld 1: „Implement the narrow fix at the owning network/packaging layer …“

**Bewertung: Für den dokumentierten Host sachlich erfüllt — als Umgebungsbehebung, nicht als KNXBench-Protokollpatch.**

Belege:

- `docs/RESEARCH.md:6224–6250`: Am **29. September 2026** wurde die Gateway-Antwort im UFW-Droplog nachgewiesen; direkte Unicast-Proben antworteten.
- `docs/RESEARCH.md:6252–6272`: Nach der **vom Benutzer am 30. September 2026** vorgenommenen Firewallregel fanden unveränderte CLI- und HTTP-Discovery das Gateway.
- `apps/knx-web/src/gatewayEndpoint.ts:37–54`: Der manuelle IPv4-Endpunkt bleibt ausdrücklich unterstützt und validiert.

Damit ist ein spekulativer Packaging-Patch, Retry oder zusätzlicher Sleep **nicht gerechtfertigt**. Die UI-Session kann dieses Feld mit einem präzisen Hinweis auf die externe, hostbezogene Behebung abnehmen.

**Grenzen:** Dies ist die Bewertung der eingecheckten Evidenz, keine neue Live-Messung. Ein Wire-Capture wurde nicht durchgeführt. Der native WebKitGTK-Klick auf **Search** wurde nach der Behebung ebenfalls nicht beobachtet.

### Feld 2: „Add loopback tests where possible and document the boundary …“

**Bewertung: Teilweise erfüllt; mit der vorliegenden Evidenz noch nicht vollständig abnahmefähig.**

- `apps/knx-server/tests/http_bus_discover.rs:1–9` ersetzt Discovery vollständig durch einen FakeConnector. Die fünf Tests prüfen HTTP-Verträge, Ergebnisse und Fehler — **ohne Socket oder Datagramm**.
- `crates/knx-net/src/client.rs:1239` prüft die lokale Discovery-HPAI-Auflösung und den tatsächlichen Socket-Port. Dieser Test kann ohne Multicast-Route überspringen. Er belegt **keinen Search-Request/Response-Roundtrip**.
- Die reale Netzwerkgrenze ist dokumentiert; die vorhandenen Tests sind dennoch kein vollständiger Multicast-Loopbacktest.

**Kleinste sinnvolle Ergänzung:** Ein begrenzter Offline-Discovery-Roundtrip über einen lokalen UDP-Responder oder injizierten Datagrammtransport: Request mit korrektem HPAI-Port → Unicast-Response → Gateway-Liste, einschließlich Timeout und ungültiger Antworten. Ohne produktiven Gatewaykontakt, LAN-Multicast oder Firewalländerung.

Ein vollständiger realer Multicasttest ist dafür nicht zwingend erforderlich. Das Feld darf aber nicht allein durch Umbenennung der vorhandenen Fake-/HPAI-Tests als erfüllt gelten.

## 3. Abdeckung des gesamten UI-Tracks

| Paket | Geprüfter Bereich und Ergebnis |
|---|---|
| U0 | Baseline, Isolation und dokumentierte Zuständigkeiten geprüft. |
| U1 | ADR-0038/Ground-Site-Semantik und Core-Verträge geprüft; daraus folgt keine vollständige ETS-Kompatibilität. |
| U2 | Discovery-Ursachenevidenz und Adresseditor-Regeln geprüft; spätere Firewallbehebung berücksichtigt. |
| U3 | File-Menü und Save-Flows geprüft. |
| U4 | Logsuche und Export geprüft; Begrenzung, Verlusthinweise und lokale Exportadapter bleiben explizit. |
| U5 | Fehlerdarstellung und Hilferouting geprüft; kein zusätzlicher Blocker im geprüften Umfang. |
| U6 | Zoom, persistierte Pane-Geometrie und Tastatur-Resize geprüft. |
| U7 | Monitor-Pause, Cursor-/Sessiongrenzen, Export, Decode-Diagnosen und begrenzte Statistiken geprüft. |
| U8 | Dialog-Resize, Fokusbehandlung und Formularbedienung geprüft; reale Screenreader-Abnahme bleibt offen. |
| U9 | Welcome- und Neuprojekt-Flows geprüft. |
| U10 | Host/Port-Vertrag korrekt begrenzt; Discovery-Abnahme wie oben. |
| U11 | Katalog-/Bulk-Erzeugung und Geräteeditor geprüft; Gerätewechsel-Befund blockiert. |
| U12 | Strukturänderungen, Gruppen, Device checks und Debug-/K6-Verträge geprüft; Parameter-/Undo-Befund blockiert. |

**Sicherheitsgrenzen:** Im geprüften Umfang wurde kein neuer Bypass festgestellt. K6 bleibt serverseitig vor dem Tunnel fail-closed; die UI-Verfügbarkeit ersetzt keine Schreibautorisierung. Service Control bleibt default-off, bestätigt und auf eine **property-only** Sicherung begrenzt. Simulatorergebnisse wurden nicht als Hardware- oder Vollbackupnachweis gewertet.

## 4. Tatsächlich ausgeführte Prüfungen

| Prüfung | Ergebnis |
|---|---|
| `npx tsc --noEmit` | Erfolgreich |
| `npx vitest run` | **82 Dateien, 1.303 Tests bestanden** |
| Core-Command-Tests | **152 bestanden** |
| HTTP Discovery / Monitor / Compare / Readiness / Service Control / Settings | **5 / 12 / 1 / 3 / 12 / 14 bestanden** |
| Explizite private-corpus-backed Katalogtests | **2 bestanden**, nicht nur Skip-Pfad |
| Neu hinzugekommene Download-Worker-Lifecycle-Tests auf finalem Stand | **8 bestanden** |
| `cargo fmt --all --check`, `git diff --check` | Erfolgreich |
| Drei zusätzliche In-Memory-Reproduktionen | Die oben beschriebenen Fehler bestätigt |

**Nicht als erledigt behauptet:** vollständige U13-Integrationsgates, native WebKitGTK-Abnahme, reale Screenreader-Prüfung, Wire-Capture oder vollständiger Multicast-Loopbacktest.

## Abschluss

**Nicht freigabefähig, bevor die drei P1-Befunde behoben und mit Regressionstests abgesichert sind.** Danach bleiben ISSUE-12-Feld 2 beziehungsweise dessen ausdrücklich akzeptierte Abgrenzung sowie die integrierten Abschlussgates Aufgabe der UI-Session.

U13, die Checkboxes und `.ai/CURRENT_STATE.md` wurden nicht geändert.

**Reviewer-Provenienz:** Dieser Bericht wurde von der aktuellen unabhängigen **GPT-6.1-Sol-Session** erstellt, nicht von Claude. Er darf daher nicht ohne ausdrückliche Nutzerentscheidung als Ersatz für die dokumentierte Wahl eines Claude-Reviews verbucht werden.