# KNX ETS Alternative – Claude Code Development Strategy

## Ziel

Entwicklung einer modernen, Linux-first KNX Engineering Software als Alternative zu ETS.

Prioritäten:

1. Datenintegrität
2. KNX-/ETS-Kompatibilität
3. Saubere Architektur
4. Wartbarkeit
5. UX
6. Performance

---

## Modellstrategie

| Phase | Modell | Effort |
|---|---|---|
| Research & Machbarkeit | **Opus** | **Max/High** |
| Architektur | **Opus** | **Max/High** |
| KNX Core | **Sonnet** | **High** |
| ETS-Projekt-Importer | **Opus** | **High/Max** |
| Herstellerdatenbanken | **Opus** | **High/Max** |
| UI/UX | **Sonnet** | **High** |
| KNXnet/IP | **Sonnet** | **High** |
| Integration & Tests | **Sonnet** | **High** |
| Routine-Features/Bugfixes | **Sonnet** | **Medium** |

### Grundregel

**Starkes Reasoning für Architektur, Formate und komplexe Parser.  
Sonnet für die anschließende modulare Implementierung.**

---

# Session-Struktur

## Session 0 – Technical Research

**Opus + Max/High**

Noch keinen Anwendungscode schreiben.

Untersuchen:

- KNX/ETS Projektformate
- Hersteller-Produktdatenbanken
- Applikationsprogramme
- KNX-Datenmodell
- Gruppenadressen
- physikalische Adressen
- Topologie
- Kommunikationsobjekte
- DPTs
- KNXnet/IP
- verfügbare Open-Source-Libraries
- technische und rechtliche Grenzen
- mögliche Informationsverluste

### Ergebnis

```text
docs/
└── RESEARCH.md
```

Ziel: belastbare technische Entscheidungsgrundlage.

---

## Session 1 – Architektur

**Opus + Max/High**

Auf Basis von `RESEARCH.md`:

- Gesamtarchitektur
- internes KNX-Datenmodell
- Datenbankmodell
- Import-/Exportarchitektur
- Plugin-/Adapter-System
- UI-Architektur
- KNXnet/IP-Abstraktion
- Migration/Versionierung
- Teststrategie

### Ergebnis

```text
docs/
├── ARCHITECTURE.md
├── DATA_MODEL.md
├── IMPORT_EXPORT.md
├── COMPATIBILITY.md
├── ROADMAP.md
└── adr/
```

Noch keine umfangreiche UI- oder Feature-Implementierung.

---

## Session 2 – KNX Core

**Sonnet + High**

Nur Domain/Core implementieren.

```text
KNX Project
├── Building
├── Area
├── Line
├── Device
├── Physical Address
├── Group Address
├── Communication Object
├── Datapoint Type
└── Connection
```

Fokus:

- korrektes Datenmodell
- Validierung
- Serialisierung
- Migration
- Unit Tests

**Noch keine komplexe UI.**

---

## Session 3 – ETS Project Import

**Opus + High/Max**

Technisch besonders kritischer Bereich.

Vorgehen:

```text
ETS File
   ↓
Format Detection
   ↓
Parser
   ↓
Validation
   ↓
Normalization
   ↓
KNX Core Model
```

Implementieren:

- Format-Erkennung
- Parser
- Mapping
- Kompatibilitätsprüfung
- Unsupported-Data Handling
- Fehler-/Warnungsbericht
- Roundtrip Tests

### Grundprinzip

**Keine stillschweigende Datenvernichtung.**

Nicht unterstützte Informationen müssen erkannt und möglichst erhalten bzw. explizit dokumentiert werden.

---

## Session 4 – Manufacturer Database

**Opus + High/Max**

Eigene Device Database Layer entwickeln.

```text
Manufacturer
    ↓
Product
    ↓
Application Program
    ↓
Parameters
    ↓
Communication Objects
    ↓
DPTs
```

Unterstützen:

- Hersteller
- Produkte
- Produktversionen
- Applikationsprogramme
- Parameter
- Kommunikationsobjekte
- DPTs
- Versionierung
- Duplikaterkennung
- Kompatibilitätsprüfung

---

## Session 5 – UI/UX

**Sonnet + High**

Erst beginnen, wenn das Core-Modell stabil ist.

Zentrale UI:

- Project Explorer
- Building View
- Topology View
- Group Address View
- Device Editor
- Properties Inspector
- globale Suche
- Command Palette
- Drag & Drop
- Inline Editing
- Tabs
- Kontextmenüs
- Dark/Light Mode
- Keyboard Shortcuts

Ziel:

**Moderne Engineering-Software statt klassischer ETS-UI.**

---

## Session 6 – KNXnet/IP

**Sonnet + High**

Kommunikation strikt vom UI trennen.

```text
UI
 ↓
Application
 ↓
KNX Core
 ↓
KNX Service Layer
 ↓
KNXnet/IP
 ↓
KNX Interface/Router
 ↓
KNX Bus
```

Unterstützen, soweit sinnvoll:

- Tunneling
- Routing
- Discovery
- Bus-Kommunikation
- Diagnose
- Verbindungstest

---

## Session 7 – Integration & Hardening

**Sonnet + High**

Alles zusammenführen.

Testen:

- Import
- Core
- UI
- Datenbank
- KNXnet/IP
- Export
- Roundtrip
- große Projekte
- Performance
- Fehlerfälle
- Crash Recovery
- Autosave
- Migration
- Regression

---

# Repository als gemeinsames Gedächtnis

Sessions dürfen **nicht vom Chat-Kontext abhängig sein**.

Alle wichtigen Erkenntnisse müssen im Repository stehen.

```text
docs/
├── RESEARCH.md
├── ARCHITECTURE.md
├── DATA_MODEL.md
├── IMPORT_EXPORT.md
├── COMPATIBILITY.md
├── ROADMAP.md
├── IMPLEMENTATION_STATUS.md
├── KNOWN_LIMITATIONS.md
└── adr/
```

Zusätzlich:

```text
CLAUDE.md
```

mit den verbindlichen Projektregeln.

### Regel für Claude Code

> Never rely on conversation context for project knowledge.  
> All architectural decisions, discoveries, constraints, implementation status and known limitations must be persisted in the repository.

---

# Wichtigste Architekturregel

Das interne Datenmodell darf **nicht von ETS-Dateiformaten abhängig sein**.

```text
ETS / Manufacturer Files
          ↓
      Importer
          ↓
   Normalized KNX Model
          ↓
      Application
          ↓
      Exporter
```

Dadurch bleiben Importer, UI, KNXnet/IP und zukünftige Formate unabhängig voneinander.

---

# Entwicklungsprinzipien

- Erst verstehen, dann implementieren.
- Keine proprietären Details erfinden.
- Keine unnötigen Abhängigkeiten.
- Modular entwickeln.
- Jede Phase muss buildbar bleiben.
- Tests parallel zur Implementierung.
- Datenverlust beim Import vermeiden.
- Unsupported Data explizit behandeln.
- Architekturentscheidungen als ADR dokumentieren.
- Große Features in eigenständige Sessions aufteilen.
- Git als verbindliche Versionsbasis verwenden.

## Priorität

**Korrektheit → Datenintegrität → Kompatibilität → Wartbarkeit → UX → Performance**