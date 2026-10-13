# Access-key assignment during download

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-112)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 112. A download plan that needs `A_Key_Write` is refused outright, not carried out

**Limitation.** CP §3.5.2 Nr. 11, p. 44, and CP §3.5.3 AP2 Nr. 13, p. 47,
both read *"Set access keys as required"* — a step this project's download
sequencer now models honestly with `AccessKeyDeclaration`
(`crates/knx-core/src/commissioning/authorisation.rs`): a plan declares
either `NoneRequired` or `Required(Vec<AccessKeyAssignment>)`. A plan
declaring `NoneRequired` completes and the step is reported empty, which is
correct. A plan declaring `Required` is refused with
`DownloadError::AccessKeysNotSupported` at the step, because
`A_Key_Write` has no encoder: `crates/knx-net/src/cemi.rs`'s
`key_write_has_an_apci_but_no_encoder` test documents exactly this —
the APCI constant exists, the frame variant does not.

**Cause.** `A_Key_Write` was out of scope for commissioning phase 2 (design
spec §10.7), and encoding it needs its own frame-format decision (the
payload shape, and how a "delete" key — `FFFFFFFFh` — is represented,
since `AccessKey::new` deliberately rejects that value as the free-access
sentinel rather than a key). Building that encoder was not this task's job;
this task's job was to stop a plan that needs it from silently completing
without it, which is what the refusal now does.

**Impact.** Every download this project can currently run must declare
`AccessKeyDeclaration::NoneRequired`, i.e. must leave every access level at
its existing key. A commissioning workflow that also wants to set or
change a device's key as part of the same download cannot do so through
this sequencer today; the operator must do that separately, by whatever
means already exists outside this application, or wait for `A_Key_Write`
to be built. No device is left on a key the operator believes was changed:
the whole download is refused rather than partially honoured.

**Lifted when.** `A_Key_Write` gets an encoder in `cemi.rs` and
`ManagementSession` gains a way to send it — at which point
`modify_access_keys` (`crates/knx-net/src/commissioning/download.rs`) can
carry out a `Required` declaration instead of refusing it.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
A plan that needs `A_Key_Write` keeps being refused by name; no access key
is ever guessed. A project that needs keys (together with KNX Secure, §8)
reopens this.
<!-- END retained catalogue record -->
