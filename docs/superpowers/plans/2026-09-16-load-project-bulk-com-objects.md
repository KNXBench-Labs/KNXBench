# Bulk Communication-Object Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Replace the measured 65,000 communication-object point queries in `knx_store::load_project` with three ordered scans while preserving the reconstructed project graph and every existing error.

**Architecture:** `crates/knx-store/src/devices.rs` continues to own SQLite decoding for devices and communication objects. A new crate-private bulk loader reads base communication-object rows, override rows, and group-link rows in stable ownership/order sequence, then returns objects bucketed by device. `project.rs` consumes that result while retaining its existing device insertion order and public `load_project` interface.

**Tech Stack:** Rust 2021, `rusqlite`, `knx-core`, existing `knx-store` fixtures and ignored release performance baseline.

**Spec:** `docs/PERFORMANCE.md` section “load_project investigation (2026-09-16)”.

## Global Constraints

- Preserve device ownership and per-device communication-object `position` order.
- Preserve override decoding, including `UnknownOverrideAttr` failure behavior.
- Preserve module-instance association and group-link `position` order.
- Keep the public store API and SQLite schema unchanged.
- Do not touch Claude-owned product-data, server, or web files.

---

### Task 1: Bulk communication-object graph loader

**Files:**
- Modify: `crates/knx-store/src/devices.rs`

**Interfaces:**
- Produces: `pub(crate) fn load_all_com_objects(conn: &Connection) -> Result<BTreeMap<DeviceId, Vec<ComObjectInstance>>, StoreError>`.
- Reuses: the existing override decoders and `direction_from_str`.

- [ ] **Step 1: Write a failing ownership/order regression test**

Add a `devices.rs` unit test that persists two devices and communication objects in ID order different from their per-device positions, including every override state, a module-instance id, and two group links. Call the missing `load_all_com_objects` API and assert exact device buckets and object/link order.

```rust
let loaded = load_all_com_objects(&conn).unwrap();
assert_eq!(loaded[&DeviceId(1)].iter().map(|c| c.id).collect::<Vec<_>>(),
           vec![ComObjectInstanceId(9), ComObjectInstanceId(3)]);
assert_eq!(loaded[&DeviceId(1)][0].links, expected_links);
assert_eq!(loaded[&DeviceId(1)][0].module_instance, Some(ModuleInstanceId(7)));
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test -p knx-store devices::tests::bulk_loader`

Expected: compilation fails because `load_all_com_objects` does not exist.

- [ ] **Step 3: Implement the three ordered scans**

Read base rows with `ORDER BY device_id, position`, overrides with `ORDER BY com_object_instance_id`, and links with `ORDER BY com_object_instance_id, position`. Build communication objects once, apply overrides through one shared helper also used by the point loader, append links in stored order, then bucket objects by owning device without sorting by object id.

```rust
pub(crate) fn load_all_com_objects(
    conn: &Connection,
) -> Result<BTreeMap<DeviceId, Vec<ComObjectInstance>>, StoreError> {
    // one base scan, one override scan, one link scan
}
```

- [ ] **Step 4: Run the focused test and full crate tests**

Run: `cargo test -p knx-store devices::tests::bulk_loader`

Run: `cargo test -p knx-store`

Expected: all pass.

### Task 2: Use the bulk graph in project loading and measure it

**Files:**
- Modify: `crates/knx-store/src/project.rs`
- Modify: `docs/PERFORMANCE.md`

**Interfaces:**
- Consumes: `load_all_com_objects` from Task 1.
- Preserves: `pub fn load_project(conn: &Connection) -> Result<Project, StoreError>`.

- [ ] **Step 1: Add a project-level regression that distinguishes stored position from ID order**

Extend the existing project round-trip fixture so one device references communication objects in non-ID order and assert the exact list after `load_project`.

```rust
assert_eq!(loaded_device.com_objects, vec![ComObjectInstanceId(9), ComObjectInstanceId(3)]);
```

- [ ] **Step 2: Verify the regression catches an ID-sorted implementation**

Run the focused project test with a temporary ID-sorted bulk result and confirm the order assertion fails; restore the production-order implementation immediately afterward.

- [ ] **Step 3: Switch `load_project` to the bulk loader**

Load the map once, remove each device's ordered object vector while iterating device IDs, copy its ids into `device.com_objects`, insert the objects into `Devices`, then insert the device. Missing buckets produce an empty list exactly as before.

- [ ] **Step 4: Run behavior-preservation gates**

Run: `cargo fmt --all --check`

Run: `cargo test -p knx-store`

Run: `cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture`

Run: `cargo clippy --workspace --all-targets -- -D warnings`

Run: `cargo test --workspace`

Run: `cargo run -p xtask -- check-layering`

- [ ] **Step 5: Record measured evidence**

Append the post-change release benchmark result to `docs/PERFORMANCE.md`, state the remaining device/binary N+1 work, and avoid claiming a stable speedup beyond the observed run and environment.

