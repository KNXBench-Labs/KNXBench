# ADR 0092: LCARS is a built-in palette with a controlled presentation

Date: 2026-10-08

Status: Accepted — user approved the interactive offline study and requested
production integration. Extends ADR-0022 with an explicit built-in presentation
boundary; declarative v1 theme-pack admission (ADR-0060) and the single dropdown
(ADR-0079) remain unchanged.

## Context

The approved LCARS design needs recognizable elbows, segmented framing and
rounded navigation while keeping engineering tables and properties quiet.
A colour-only pack cannot express that geometry. Importing executable CSS,
layouts or animations would weaken the existing v1 format/security boundary.
Motion and density already have independent settings and must not become
hidden theme preferences.

## Decision

- Ship `lcars` as a complete CSS palette in the existing built-in registry,
  named **LCARS**, with fixed warm orange/lavender accents and dark surfaces.
  Selection uses the existing acknowledged conditional Theme dropdown write.
  No installed pack, new setting, schema migration or dependency is needed.
- `useThemeId` sets `data-presentation="lcars"` only for this successfully
  resolved built-in choice, not for any pack. It removes the marker on other
  choices, fallback and disposal. The pre-paint bootstrap mirrors the built-in
  marker and palette choice; runtime corrects cached settings as before.
- Palette blocks retain ADR-0022's complete-token, contrast and selector rules.
  Application-owned component geometry uses the separate presentation marker
  and the existing colour/type tokens. This is a named extension, not a
  relaxation of theme selectors or v1 validation. Imported metadata/tokens
  cannot opt into this presentation; an import still requires `user-` identity.
- Preserve the current toolbar, explorer, workspace, inspector, diagnostics,
  native tables, selection, keyboard handlers and pane controls. Pseudo-element
  bands are pointer-inert. No decorative columns, fake counters or new workflows.
  At narrow sizes the frame shrinks/disappears before the existing stacked
  workspace loses room. Compact/comfortable density remains independent.
- Standard motion adds one finite underline reveal on the actual current
  navigation item; Subtle uses short colour/border transitions without that
  reveal. Duration and easing come from the existing motion settings. The
  motion-free baseline removes animation names and transition properties;
  activation exists only under OS `no-preference` and admitted motion levels,
  so Off or OS reduction also cancels already-running effects. No idle loops,
  sound, artificial delay, success animation or protocol behavior is added.
- Save/errors/import warnings and bus state retain their existing meaning.
  Framing never represents successful persistence or a connected bus.

## Alternatives

- Palette-only v1 file: insufficient for the visually approved LCARS framing.
- Imported arbitrary CSS or a v2 executable pack: unnecessary and unsafe scope.
- A second Theme/Layout picker or user-owned presentation setting: unnecessary
  for this one optional theme and contrary to the agreed single-picker UX.
- Core/API changes or rearranged workspaces: irrelevant to a presentation task.

## Consequences and evidence

The palette applies application-wide; the distinctive geometry targets the
existing workbench shell, while dialogs and secondary windows retain their
functional layouts. LCARS is not exportable as a falsely lossless v1 pack and
is not removable through imported-pack controls. Other themes are unchanged.

Tests cover registry/selection/persistence/cleanup, pre-paint choice, complete
palette and role contrast, geometry isolation, density/motion ownership and
finite guarded feedback. A CLI verifier exercises the actual production build
with explicitly intercepted synthetic requests, including native selection,
filters, shortcuts, density, live motion cancellation, refused Save As, theme
switches/reload, real application UI scale 1.5, and small viewports.

This is Chromium/self-review evidence, not native WebKitGTK/Orca, Firefox,
complete accessibility certification or an official licensed series product.
Light mode, sounds, arbitrary CSS import and workflow redesign are out of scope.
See [LCARS guide](../DESIGN_LCARS.md) for operation and verification boundaries.
