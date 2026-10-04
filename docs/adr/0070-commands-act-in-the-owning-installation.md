# ADR 0070: Commands act in the installation that owns their target

Date: 2026-10-04
Status: Accepted (core and server); web UI follows under the Web lock
Session: goal-ui owner, Alpha package UA4 (`MODEL-01`)

## Context

A project can hold several installations; ADR-0038 treats separate KNX
infrastructures as separate installations. Until now almost every
`knx_core::Command` read or wrote `project.installations[0]`. A second
installation brought in by import was preserved and saved but could not be
edited, and new group links and parameter rows always landed in the first
installation. Ids are allocated project-wide, so an entity id normally names
exactly one installation; malformed imports can repeat an id.

## Decision

- A command addressed by an entity id (area, line, building part, group
  range, group address, device, parameter row) acts in the **one**
  installation that holds that entity. If the id occurs in several
  installations the command is refused as ambiguous; the first is never
  picked.
- `Delete*` inverses record the installation, so undo restores into the same
  installation and position.
- Root-level creates (`CreateArea`, main `CreateGroupRange`, root
  `CreateBuildingPart`, range-less `CreateGroupAddress`) take
  `installation: Option<InstallationId>`; `None` keeps the previous default,
  the first installation. A create with a parent goes to the parent's
  installation; a different explicit target is refused.
- **Nothing connects two installations.** Moving a device to a line or
  building part of another installation, linking a communication object to a
  group address of another installation, and reparenting lines, ranges or
  parts across installations are refused with
  `CommandError::CrossInstallation`.
- A parameter edit changes the existing row where it lives; a new row goes to
  the device's installation (the first one if the device is placed nowhere).
- `Command::RenameInstallation` renames an installation; the server exposes
  it as `PATCH /api/installations/{id}` and accepts `installationId` on the
  root create routes.

## Consequences

- Imported multi-installation projects become editable without guessing.
- A device cannot be moved between installations; that would need explicit
  rules for its links and parameters and is not offered.
- CSV group-address import/export targets the first installation unless the
  server request names `installationId` (added later the same day, IMPORT_EXPORT
  §11); the CLI still uses the first installation.
- The web UI must still offer installation rename and a target choice for
  root creates before MODEL-01 is complete for users.
