# ETS6 revision exports and project-archive restore (8 November 2026)

Clean-room observation under the [evidence routes](../RESEARCH.md#evidence-routes):
the project made a fictional project in an ETS6 6.4.1 installation (Demo licence)
and exported it at three revision states for the
[authorized export harness](../AUTHORIZED_RESTORE_EXPORTS.md#run-record-8-november-2026).
The exports, manifest and logs stay private. Varied: one added group address with
one object link, one changed device parameter, and one archive restore. Not varied:
ETS version, schema, product, protection.

## Findings

- **[V]** The exports are schema 23 (`http://knx.org/xml/project/23`). ETS asked
  for no password or encryption, and the files are unprotected.
- **[V]** A device left entirely at its defaults has no `ParameterInstanceRef`.
  Changing one checkbox parameter produced exactly one `ParameterInstanceRef`.
  That parameter made ETS list one more communication object (20 → 21). No
  instance reference appears for the new object until it is linked.
- **[V]** The added group address carried a `Puid` attribute, as the first one
  did. The new object link (`ComObjectInstanceRef`) carried a `ChannelId`.
- **[V]** ETS's project archive ("create backup in archive", then "restore")
  is a whole-project operation. Restoring warned that local changes would be
  reset, closed the project tab and re-imported the archived project. The
  export made afterwards had a project `0.xml` byte-identical to the export made
  before the backup.
- **[V]** After the backup, `project.xml` carries an `ArchivedVersion`
  attribute (UTC timestamp) on `ProjectInformation`. Both the restored export
  and a later export of further edits carried it. KNXBench retains it as an
  opaque attribute, so the restored export has one more retained entry than the
  original.
- **[V]** The archive folder held the archived `.knxproj` plus status, project
  summary and trace files. Those three were not interpreted or copied.

## Limits

This is one ETS version, schema, product and archive restore, all unprotected.
Nothing here describes ETS's internal restore-point storage format, which KNXBench
does not read. It is also not evidence about device behaviour.
