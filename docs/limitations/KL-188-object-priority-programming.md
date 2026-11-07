# A changed object priority is refused, not programmed

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-188)

<!-- BEGIN retained catalogue record -->

## Programming guard for project priorities (7 November 2026)

The device editor lets the user set an object's project priority (`Low`,
`High`, `Alert`) under the [priority-edit contract](../contracts/object-priority-edit.md).
The image builder writes the product's priority only (absent or `Low` as low,
`Alert` as urgent by the documented inference, `High` refused). It has no
evidence for encoding a project token, so it does not try:

- **Refused, not written.** An active object whose stated priority (a user
  edit, else a captured Known declaration still bound to the object) is not
  exactly the product's selected literal refuses the whole image
  (`ImageError::PriorityDiffers`). An absent product literal is not read as
  `Low` for this comparison. Equal values change no octet.
- **Unreadable declarations.** A captured Empty or Unknown declaration without
  a user edit refuses the request (`ProjectRequestError::UnreadablePriority`).
- **Not checked.** Captured priority is unavailable for native files saved
  before schema 17, other source schemas, module objects and unbound facts.
  Those objects state nothing, and the product priority is written as before.
  This does not prove that the device or the original project used it.
- **Inactive objects.** A statement for an object the parameters leave
  inactive has no octet and is ignored, like a flag override.

Lifting needs evidence for writing project tokens (or a decision to encode the
existing inference for them), or capture of the missing source schemas.
