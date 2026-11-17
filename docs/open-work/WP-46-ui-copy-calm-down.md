# WP-46 — Calmer UI copy: "Default" priority label, explanations behind ?, one-time hints

Origin: owner UI feedback, 17 November 2026 (WebUI session). Not tied to a
GitHub issue.

## Requested outcome

1. **Priority rename:** "Nicht von dir festgelegt" / "Not set by you" in the
   own-priority select becomes "Standard" / "Default".
2. **Long explanations behind ?:** verbose explanatory texts move out of the
   always-visible surface into the existing help-tip mechanism (`?`), or are
   removed where redundant.
3. **One-time hints:** manufacturer-field hints (e.g. access captions) appear
   once at the top of the section, not repeated per value.
4. Remove noise sentences that make the UI restless.

## Current basis (source anchors on `origin/main` `9af5c698a`)

- Rename: `inspector.priorityEdit.none` in `messages/de.ts:2089`
  ("Nicht von dir festgelegt") and `messages/en.ts:2096` ("Not set by you");
  consumer `EffectivePriority.tsx:121`; test pin `PriorityEdit.test.tsx:91`.
- Existing `?` mechanism: `apps/knx-web/src/HelpTip.tsx` (ADR-0024 short-form
  help tip, focusable button with `role="tooltip"`), already used in
  `Inspector.tsx`, `BusMonitorPanel.tsx`, `GroupAddressTable.tsx` and others.
- Candidate texts to move behind `?` or drop:
  - `inspector.savedPriority.hint` / `inspector.effectivePriority.hint`
    (`SavedInstancePriority.tsx:70`, `EffectivePriority.tsx:86`, plus the
    long `unavailable.*` variants, `messages/de.ts:2049-2075`);
  - `recordedDevice.title/explanation/dates/privacy`
    (`RecordedDeviceStatePanel.tsx:24`, `messages/de.ts:747-748+`) — the long
    explanation block quoted by the owner;
  - `parameters.objectFunctionsEmpty` (`messages/de.ts:582`);
  - `comTable.lastDownload.links.absent` and the object-flags equivalent
    (`messages/de.ts:1055+`);
  - `comTable.projectFolderRefused.*` (`messages/de.ts:991-995`);
  - device-assumption paragraph ("Was das Projekt über dieses Gerät annimmt …").
- Per-value repetition: access captions in `ParameterPanel.tsx:347`
  (`parameters.accessNoneCaption` "Vom Hersteller verborgen (Access None) …",
  `parameters.access.None` "Kein Zugriff", `messages/de.ts:2191/:2301`) and
  the untranslated badge `untranslated.label` "Unübersetzt ({source})"
  (`languageFallback.tsx:27-28`) — show once per view/section header, keep a
  compact marker per value.

## Bounded packages

| Package | Outcome | Status |
| --- | --- | --- |
| 46.1 | Rename priority option to "Standard"/"Default" in de+en, update tests | Not started |
| 46.2 | Move the listed long explanations behind HelpTip (`?`) or delete redundant ones; keep one-line essentials visible | Not started |
| 46.3 | Manufacturer-field hints (access None/hidden, untranslated) once per section, compact per-value marker | Not started |

## Boundaries

- Message-key changes must keep de/en in sync and update pinned tests
  (`PriorityEdit.test.tsx`, `SavedInstancePriority.test.tsx`,
  `EffectivePriority.test.tsx`, `ParameterPanel.test.tsx`,
  `parameterMetadata.test.tsx`).
- No meaning loss: anything a user needs for a decision stays visible; only
  explanatory redundancy moves behind `?`.
- Pure web UI/i18n change; no domain, server or storage impact.
