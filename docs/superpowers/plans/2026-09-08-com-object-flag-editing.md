# Communication-Object Flag Editing (T7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user edit a communication object's five flags (Read/Write/Transmit/Update/Communication) from the Inspector, instead of only viewing them read-only.

**Architecture:** Adds one command pair to the existing `knx-core` command layer (`Command::SetComObjectFlag`/`RestoreComObjectFlag`), generic over which of the five flags it targets via a new `ComFlagKind` enum plus `ResolvedFlags::get`/`get_mut` accessors — same "set as `Layer::UserEdit`, undo restores the exact prior `Override`" shape `SetComObjectDpt`/`SetComObjectDescription` already use. `apps/knx-server` gets one new route; `apps/knx-web` gets one new checkbox row on the existing comm-object list, wired through `api.ts`. No new domain error variant (`CommandError::ComObjectNotFound` already covers the only failure mode), no projection change (`ComObjectNode` already carries all five flags as plain `bool`s).

**Tech Stack:** Rust (knx-core, knx-server/axum), TypeScript/React (knx-web), vitest, cargo test.

**Spec:** [docs/GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md) — Tier 2, **T7**, closes **B6**.

## Global Constraints

- Priority is deliberately out of scope this cycle — `ResolvedFlags`/`ComFlags` (`crates/knx-core/src/flags.rs`) model no priority field today, and T7's own doc text only asks for it "if modelled". Do not add a priority field as part of this plan.
- A checkbox has exactly two states — unlike `SetComObjectDpt`/`SetComObjectDescription` (which accept `Option<T>` so an empty string clears the override to `Override::Empty`), `SetComObjectFlag`'s `value` is a bare `bool`. There is no "clear override" gesture for flags in this plan.
- Every `Command` that changes a `Resolved<T>` sets its layer to `Layer::UserEdit` (command.rs's own module doc comment) — follow it here too.
- No Tauri `invoke` calls anywhere — this project's frontend is fetch()-based against `apps/knx-server` (api.ts's own header comment).

---

### Task 1: `knx-core` — `ComFlagKind` and the `Command` pair

**Files:**
- Modify: `crates/knx-core/src/flags.rs`
- Modify: `crates/knx-core/src/command.rs`
- Modify: `crates/knx-core/src/lib.rs:33`
- Test: inline `#[cfg(test)]` modules in both `flags.rs` and `command.rs`

**Interfaces:**
- Consumes: `crate::provenance::{Layer, Override, Resolved}` (already imported in `command.rs`), `ComObjectInstance`/`project.devices.com_object_mut` (already used by the `SetComObjectDpt` arm).
- Produces: `knx_core::ComFlagKind` (`Read`/`Write`/`Transmit`/`Update`/`Communication`), `ResolvedFlags::get(&self, ComFlagKind) -> &Override<bool>`, `ResolvedFlags::get_mut(&mut self, ComFlagKind) -> &mut Override<bool>`, `Command::SetComObjectFlag { com_object: ComObjectInstanceId, flag: ComFlagKind, value: bool }`, `Command::RestoreComObjectFlag { com_object: ComObjectInstanceId, flag: ComFlagKind, value: Override<bool> }` — Task 2's `apps/knx-server::domain` consumes all of these by name.

- [ ] **Step 1: Add `ComFlagKind` and the `ResolvedFlags` accessors**

In `crates/knx-core/src/flags.rs`, after the `ResolvedFlags` impl block (after the closing `}` of `impl ResolvedFlags { pub fn none() -> Self { ... } }`), add:

```rust
/// Which of a communication object's five flags a `Command::SetComObjectFlag`
/// targets. Mirrors `ResolvedFlags`' own field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComFlagKind {
    Read,
    Write,
    Transmit,
    Update,
    Communication,
}

impl ResolvedFlags {
    /// Borrows the one field `kind` names — lets `Command::SetComObjectFlag`
    /// stay generic over which of the five flags it edits instead of five
    /// near-identical match arms living in `command.rs`.
    pub fn get(&self, kind: ComFlagKind) -> &Override<bool> {
        match kind {
            ComFlagKind::Read => &self.read,
            ComFlagKind::Write => &self.write,
            ComFlagKind::Transmit => &self.transmit,
            ComFlagKind::Update => &self.update,
            ComFlagKind::Communication => &self.communication,
        }
    }

    /// The mutable counterpart of `get`.
    pub fn get_mut(&mut self, kind: ComFlagKind) -> &mut Override<bool> {
        match kind {
            ComFlagKind::Read => &mut self.read,
            ComFlagKind::Write => &mut self.write,
            ComFlagKind::Transmit => &mut self.transmit,
            ComFlagKind::Update => &mut self.update,
            ComFlagKind::Communication => &mut self.communication,
        }
    }
}
```

Then in the existing `#[cfg(test)] mod tests` at the bottom of the same file, add:

```rust
    #[test]
    fn get_and_get_mut_address_the_matching_field() {
        let mut flags = ResolvedFlags::none();
        *flags.get_mut(ComFlagKind::Communication) = Override::Value(Resolved {
            value: true,
            layer: Layer::UserEdit,
        });
        assert_eq!(
            flags.get(ComFlagKind::Communication).value().map(|r| r.value),
            Some(true)
        );
        // Untouched fields stay absent — `get_mut` must not alias another
        // field.
        assert!(!flags.get(ComFlagKind::Read).is_present());
    }
```

- [ ] **Step 2: Run the `knx-core` flags tests**

Run: `cargo test -p knx-core flags::tests -- --nocapture`
Expected: PASS, including the new `get_and_get_mut_address_the_matching_field` test.

- [ ] **Step 3: Add the `Command` variants**

In `crates/knx-core/src/command.rs:10`, change the import to also bring in `ComFlagKind`:

```rust
use crate::flags::{ComFlagKind, Direction, GroupLink};
```

Then, in the `Command` enum, immediately after the `RestoreComObjectDescription` variant (the block ending just before the `/// \`area.id\` is pre-allocated...` doc comment on `CreateGroupAddress`), add:

```rust
    /// Sets one of a communication object instance's five flags as a user
    /// edit. Always resolves to `Layer::UserEdit` — use
    /// `RestoreComObjectFlag` to put back an exact prior `Override<bool>`
    /// (what undo does). Unlike `SetComObjectDpt`/`SetComObjectDescription`,
    /// a flag checkbox has only two states, so `value` is a bare `bool`,
    /// never `Option<bool>` — there is no "clear the override" gesture here.
    SetComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: bool,
    },
    /// The undo/redo form of `SetComObjectFlag` — see `RestoreComObjectDpt`
    /// for why this cannot share the bare-`bool` shape.
    RestoreComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: Override<bool>,
    },
```

- [ ] **Step 4: Add the `apply` arms**

In the same file's `impl Command { pub fn apply(...) }`, immediately after the `Command::RestoreComObjectDescription { .. } => { ... }` arm and before the `Command::CreateGroupAddress { entry } => { ... }` arm, add:

```rust
            Command::SetComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = Override::Value(Resolved {
                    value: *value,
                    layer: Layer::UserEdit,
                });
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
            Command::RestoreComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = value.clone();
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
```

- [ ] **Step 5: Write the failing tests**

In `command.rs`'s `#[cfg(test)] mod tests`, immediately after `setting_com_object_dpt_to_none_writes_empty_not_absent`, add:

```rust
    #[test]
    fn set_com_object_flag_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                ..ResolvedFlags::none()
            },
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Read,
                    value: false,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.flags.read.value().unwrap().value, false);
        assert_eq!(updated.flags.read.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(restored.flags.read.value().unwrap().value, true);
        assert_eq!(restored.flags.read.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_a_never_stated_com_object_flag_writes_value_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Communication,
                    value: true,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.flags.communication.value().unwrap().value, true);
        assert_eq!(
            updated.flags.communication.value().unwrap().layer,
            Layer::UserEdit
        );
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!restored.flags.communication.is_present());
    }
```

- [ ] **Step 6: Run to verify it fails, then re-run after Steps 3-4**

Run: `cargo test -p knx-core set_com_object_flag`
Before Steps 3-4 exist this fails to compile (`Command::SetComObjectFlag` unknown); after them:
Expected: PASS.

- [ ] **Step 7: Re-export `ComFlagKind`**

In `crates/knx-core/src/lib.rs:33`, change:

```rust
pub use flags::{ComFlags, Direction, GroupLink, ObjectSize, ResolvedFlags};
```

to:

```rust
pub use flags::{ComFlagKind, ComFlags, Direction, GroupLink, ObjectSize, ResolvedFlags};
```

- [ ] **Step 8: Run the full `knx-core` test suite**

Run: `cargo test -p knx-core`
Expected: PASS, no regressions.

- [ ] **Step 9: Commit**

```bash
git add crates/knx-core/src/flags.rs crates/knx-core/src/command.rs crates/knx-core/src/lib.rs
git commit -m "feat(knx-core): Command::SetComObjectFlag — edit one of the five comm-object flags"
```

---

### Task 2: `apps/knx-server` — route and domain wiring

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Test: `apps/knx-server/tests/command_dispatch.rs`

**Interfaces:**
- Consumes: `knx_core::Command::SetComObjectFlag`, `knx_core::ComFlagKind` (Task 1); the existing private `apply(state, command)` helper already used by every other `*_impl` function in `domain.rs`; the existing `AppState`/`SharedState`/`ApiError` types already used by every other route.
- Produces: `pub fn set_com_object_flag_impl(state: &AppState, com_object_id: u32, flag: String, value: bool) -> Result<knx_projection::ProjectTree, String>`, `POST /api/com-object-flag` accepting `{ comObjectId: number, flag: string, value: boolean }` — Task 3's `api.ts` calls this route by exact path and body shape.

- [ ] **Step 1: Add the domain function**

In `apps/knx-server/src/domain.rs`, immediately after `set_com_object_description_impl` (the function ending just before the `/// Allocates a fresh \`GroupAddressId\`...` doc comment on `create_group_address_impl` or equivalent), add:

```rust
/// Parses the wire-format flag name (`"Read"`, `"Write"`, `"Transmit"`,
/// `"Update"`, `"Communication"` — `ComFlagKind`'s own `Debug` form) the
/// same way `parse_direction` parses `"Send"`/`"Receive"` for group links.
fn parse_com_flag_kind(flag: &str) -> Result<knx_core::ComFlagKind, String> {
    match flag {
        "Read" => Ok(knx_core::ComFlagKind::Read),
        "Write" => Ok(knx_core::ComFlagKind::Write),
        "Transmit" => Ok(knx_core::ComFlagKind::Transmit),
        "Update" => Ok(knx_core::ComFlagKind::Update),
        "Communication" => Ok(knx_core::ComFlagKind::Communication),
        other => Err(format!(
            "unknown com-object flag '{other}', expected one of Read/Write/Transmit/Update/Communication"
        )),
    }
}

pub fn set_com_object_flag_impl(
    state: &AppState,
    com_object_id: u32,
    flag: String,
    value: bool,
) -> Result<knx_projection::ProjectTree, String> {
    let flag = parse_com_flag_kind(&flag)?;
    apply(
        state,
        knx_core::Command::SetComObjectFlag {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            flag,
            value,
        },
    )
}
```

- [ ] **Step 2: Add the route**

In `apps/knx-server/src/routes.rs:23-26`, change:

```rust
        .route(
            "/api/com-object-description",
            post(set_com_object_description),
        )
```

to also register the new route right after it:

```rust
        .route(
            "/api/com-object-description",
            post(set_com_object_description),
        )
        .route("/api/com-object-flag", post(set_com_object_flag))
```

Then, immediately after the `set_com_object_description` handler (after its closing `}` around line 163), add:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectFlagBody {
    com_object_id: u32,
    flag: String,
    value: bool,
}

async fn set_com_object_flag(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectFlagBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_flag_impl(&state, body.com_object_id, body.flag, body.value)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 3: Write the failing test**

In `apps/knx-server/tests/command_dispatch.rs`, immediately after `setting_com_object_dpt_marks_it_user_edit_and_undo_restores_the_program_layer`, add:

```rust
#[test]
fn setting_com_object_flag_marks_it_user_edit_and_undo_restores_absence() {
    let state = state_with_two_devices();
    knx_server::set_com_object_flag_impl(&state, 1, "Communication".into(), true).unwrap();

    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert!(com.communication);
    drop(project);

    knx_server::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert!(!com.communication); // fixture's com object had no flags stated at all
}

#[test]
fn setting_an_unknown_com_object_flag_name_is_rejected() {
    let state = state_with_two_devices();
    let err = knx_server::set_com_object_flag_impl(&state, 1, "Priority".into(), true)
        .expect_err("unknown flag name must be rejected");
    assert!(err.contains("Priority"));
}
```

- [ ] **Step 4: Run to verify it fails, then implement Steps 1-2, then re-run**

Run: `cargo test -p knx-server setting_com_object_flag setting_an_unknown_com_object_flag`
Expected: FAIL to compile before Steps 1-2 (`set_com_object_flag_impl` unknown); PASS after.

- [ ] **Step 5: Run the full `knx-server` test suite**

Run: `cargo test -p knx-server`
Expected: PASS, no regressions.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-server/src/domain.rs apps/knx-server/src/routes.rs apps/knx-server/tests/command_dispatch.rs
git commit -m "feat(knx-server): POST /api/com-object-flag"
```

---

### Task 3: `apps/knx-web` — Inspector checkboxes

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/api.test.ts`
- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/styles.css`

**Interfaces:**
- Consumes: `POST /api/com-object-flag` (Task 2); `ComObjectNode` (`apps/knx-web/src/bindings/ComObjectNode.ts`, already generated, already has `read`/`write`/`transmit`/`update`/`communication: boolean` fields — no `ts-rs` regeneration needed).
- Produces: `export type ComFlagName = "Read" | "Write" | "Transmit" | "Update" | "Communication"`, `export function setComObjectFlag(comObjectId: number, flag: ComFlagName, value: boolean): Promise<ProjectTree>` in `api.ts`; a `ComObjectFlagsRow` component in `Inspector.tsx`, rendered once per comm object in `DeviceInspector`.

- [ ] **Step 1: Write the failing `api.ts` test**

In `apps/knx-web/src/api.test.ts`, immediately after the `setComObjectDescription` test, add:

```ts
  it("setComObjectFlag posts to /api/com-object-flag with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setComObjectFlag(3, "Communication", true);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/com-object-flag");
    expect(JSON.parse(init.body as string)).toEqual({
      comObjectId: 3,
      flag: "Communication",
      value: true,
    });
  });
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd apps/knx-web && npx vitest run api.test.ts`
Expected: FAIL — `api.setComObjectFlag is not a function`.

- [ ] **Step 3: Implement `setComObjectFlag` in `api.ts`**

In `apps/knx-web/src/api.ts`, immediately after `setComObjectDescription` (after its closing `}`), add:

```ts
export type ComFlagName = "Read" | "Write" | "Transmit" | "Update" | "Communication";

export function setComObjectFlag(
  comObjectId: number,
  flag: ComFlagName,
  value: boolean,
): Promise<ProjectTree> {
  return request("/api/com-object-flag", {
    method: "POST",
    body: JSON.stringify({ comObjectId, flag, value }),
  });
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd apps/knx-web && npx vitest run api.test.ts`
Expected: PASS.

- [ ] **Step 5: Add the `ComObjectFlagsRow` component**

In `apps/knx-web/src/Inspector.tsx`, immediately after `DptField`'s closing `}` (right before the `// One existing \`GroupLink\`...` comment on `GroupLinkRow`), add:

```tsx
// Five checkboxes, one per `ComFlagName` — each toggle applies immediately
// (no blur/Enter gesture, unlike `DptField`/`ComObjectDescriptionField`,
// since a checkbox's `onChange` already fires exactly once per intended
// edit). No "clear to inherited" affordance exists here, matching
// `Command::SetComObjectFlag`'s own bare-`bool` shape (see command.rs).
function ComObjectFlagsRow(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const [error, setError] = useState<string | null>(null);

  async function toggle(flag: api.ComFlagName, value: boolean) {
    setError(null);
    try {
      const tree = await api.setComObjectFlag(com.id, flag, value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  const flags: { label: string; name: api.ComFlagName; value: boolean }[] = [
    { label: "R", name: "Read", value: com.read },
    { label: "W", name: "Write", value: com.write },
    { label: "T", name: "Transmit", value: com.transmit },
    { label: "U", name: "Update", value: com.update },
    { label: "C", name: "Communication", value: com.communication },
  ];

  return (
    <div className="com-object-flags">
      {flags.map((f) => (
        <label key={f.name} title={f.name}>
          <input
            type="checkbox"
            checked={f.value}
            onChange={(e) => toggle(f.name, e.target.checked)}
          />
          {f.label}
        </label>
      ))}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
```

- [ ] **Step 6: Render it in `DeviceInspector`**

In the same file, in `DeviceInspector`'s JSX (the `<ul className="com-object-list">` block), immediately after `<ComObjectDescriptionField com={com} onApplied={onApplied} />` and its following `description_layer` badge, add:

```tsx
            <ComObjectFlagsRow com={com} onApplied={onApplied} />
```

so the block reads (unchanged lines omitted for brevity — only the new line is added, right before `<ul className="group-link-list">`):

```tsx
            <ComObjectDescriptionField com={com} onApplied={onApplied} />
            {com.description_layer && (
              <span className="provenance-badge">{com.description_layer}</span>
            )}
            <ComObjectFlagsRow com={com} onApplied={onApplied} />
            <ul className="group-link-list">
```

- [ ] **Step 7: Add the CSS**

In `apps/knx-web/src/styles.css`, immediately after the `.com-object-label { ... }` block, add:

```css
.com-object-flags {
  display: flex;
  gap: 0.6rem;
  margin: 0.35rem 0;
  font-family: var(--knx-font-mono);
  font-size: 0.85em;
}

.com-object-flags label {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  cursor: pointer;
}
```

- [ ] **Step 8: Run the full `knx-web` test suite and build**

Run: `cd apps/knx-web && npx vitest run && npx tsc --noEmit`
Expected: PASS, no regressions, no type errors (`api.ComFlagName` must resolve — `api` is already imported as `import * as api from "./api"` at the top of `Inspector.tsx`; confirm this import exists, it is used by every other field component in the file already).

- [ ] **Step 9: Commit**

```bash
git add apps/knx-web/src/api.ts apps/knx-web/src/api.test.ts apps/knx-web/src/Inspector.tsx apps/knx-web/src/styles.css
git commit -m "feat(knx-web): comm-object flag checkboxes on DeviceInspector"
```

---

### Task 4: Documentation and handover

**Files:**
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**
- Consumes: nothing — prose only.
- Produces: nothing further tasks depend on; this is the last task.

- [ ] **Step 1: Close T7 in `GAP_ANALYSIS_ETS.md`**

In `docs/GAP_ANALYSIS_ETS.md`, in the "Tier 2" section, replace:

```markdown
- **T7. Communication-object flag editing.** `Command::SetComObjectFlags`
  (read/write/transmit/update/communication, and priority if modelled),
  wired to the existing Inspector fields that already display them
  read-only. Closes **B6**.
```

with:

```markdown
- **T7. Communication-object flag editing. Done (2026-09-08).**
  `Command::SetComObjectFlag`/`RestoreComObjectFlag` land, one flag at a
  time (`ComFlagKind`) rather than all five at once — a checkbox has only
  two states, so there is no "clear to inherited" gesture the way
  `SetComObjectDpt`/`SetComObjectDescription`'s empty-string convention
  gives text fields. Priority stays out of scope: `ResolvedFlags`/
  `ComFlags` model no priority field today, so there was nothing to wire.
  `apps/knx-web` gains a `ComObjectFlagsRow` of five checkboxes on the
  comm-object Inspector row — the flags were exported to `ComObjectNode`
  read-only in an earlier cycle but, contrary to this task's original
  text, were never actually rendered anywhere in the UI until now. Closes
  **B6**.
```

- [ ] **Step 2: Append to `IMPLEMENTATION_STATUS.md`**

In `docs/IMPLEMENTATION_STATUS.md`, at the end of the file, add:

```markdown

**T7, communication-object flag editing (2026-09-08).** `crates/knx-core`
gains `Command::SetComObjectFlag`/`RestoreComObjectFlag`, generic over
which of the five flags it targets via a new `ComFlagKind` enum and
`ResolvedFlags::get`/`get_mut` accessors, rather than five near-identical
commands — same "resolves to `Layer::UserEdit`, undo restores the exact
prior `Override<bool>`" shape `SetComObjectDpt`/`SetComObjectDescription`
already use, except `value` is a bare `bool` (a checkbox has only two
states — no "clear the override" gesture exists here, unlike those two
text-field commands' empty-string convention). `apps/knx-server` gains
`POST /api/com-object-flag`, parsing the wire-format flag name the same
way `parse_direction` already parses group-link directions. `apps/knx-web`
gains `ComObjectFlagsRow`, five checkboxes rendered on every comm-object's
Inspector row — `ComObjectNode` already carried all five flags as plain
`bool`s from an earlier cycle, but nothing in the UI ever rendered them
before this cycle, contrary to what this task's own backlog text assumed.
No `knx-projection` change was needed. 3 new `cargo test` tests
(`knx-core` x2, `knx-server` x2 — one positive, one rejecting an unknown
flag name) and 1 new `vitest` test. Closes **T7**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), **B6**.
```

- [ ] **Step 3: Update `.ai/CURRENT_STATE.md`**

Overwrite `.ai/CURRENT_STATE.md`, replacing the `**Timestamp:**` line's
value with the actual wall-clock time this step runs (`date +"%Y-%m-%d %H:%M"`),
keeping every other line exactly as written below:

```markdown
- **Last Agent:** Claude
- **Timestamp:** 2026-09-08 00:00
- **Completed:** T7, communication-object flag editing. `knx-core` gains
  `Command::SetComObjectFlag`/`RestoreComObjectFlag` (generic over a new
  `ComFlagKind` enum via `ResolvedFlags::get`/`get_mut`, same
  set-as-`UserEdit`/undo-restores-exact-`Override` shape as
  `SetComObjectDpt`/`SetComObjectDescription`, but `value` is a bare
  `bool` — no "clear to inherited" gesture for a checkbox).
  `apps/knx-server` gains `POST /api/com-object-flag`. `apps/knx-web`
  gains `ComObjectFlagsRow`, five checkboxes on the comm-object Inspector
  row — the flags existed read-only on `ComObjectNode` from an earlier
  cycle but were never actually rendered in the UI until now. 3 new
  `cargo test` tests, 1 new `vitest` test, all green. Priority stays
  unmodelled/out of scope (`ResolvedFlags` has no priority field).
  `docs/GAP_ANALYSIS_ETS.md` and `docs/IMPLEMENTATION_STATUS.md` updated
  (T7 closed).
- **Pending/Next Steps:** Tier 2's remaining items: T8 (building-part
  CRUD commands, `CreateBuildingPart`/`DeleteBuildingPart`/
  `RenameBuildingPart`/`MoveDeviceToBuildingPart`, closes B4) and T9
  (bulk/multi-select operations — flagged in `GAP_ANALYSIS_ETS.md` as
  needing its own design spec for a `Command::Batch` wrapper before
  implementation, unlike every other item in this backlog so far).
- **Notes for Codex:** Work happened in a git worktree per this
  project's usual flow; merge/rebase onto `main` before continuing from
  here. No new `CommandError` variant was needed for T7 — every failure
  path (unknown com-object, unknown flag name) reuses
  `CommandError::ComObjectNotFound` or a plain `Err(String)` from the
  server's own `parse_com_flag_kind`, same as `parse_direction`'s
  existing convention.
```

- [ ] **Step 4: Commit**

```bash
git add docs/GAP_ANALYSIS_ETS.md docs/IMPLEMENTATION_STATUS.md .ai/CURRENT_STATE.md
git commit -m "docs: close T7 (comm-object flag editing)"
```
