//! Persistence for `DeviceInstance` and `ComObjectInstance`
//! (DATA_MODEL §4) — the two entities `knx_core::devices::Devices` owns.
//! This file grows across three plan tasks: device + binary data here,
//! `com_object_instance` + the `Override<T>` codec next, `group_link`
//! last (it needs `group_address` rows to exist first).

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

use knx_core::address::IndividualAddress;
use knx_core::commissioning::CommissioningState;
use knx_core::device::{BinaryDataRef, ComObjectInstance, DeviceInstance};
use knx_core::dpt::DptRef;
use knx_core::flags::{Direction, GroupLink, ObjectSize, ResolvedFlags};
use knx_core::ids::{
    ComObjectInstanceId, DeviceId, GroupAddressId, InstallationId, ModuleInstanceId, SourceRef,
};
use knx_core::provenance::{Layer, Override, Resolved};
use knx_core::string_table::{LocalizedString, Text, TranslationKey};

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;

pub fn upsert_device(
    conn: &Connection,
    installation_id: InstallationId,
    topology_position: i64,
    device: &DeviceInstance,
) -> Result<(), StoreError> {
    let c = &device.commissioning;
    conn.execute(
        "INSERT INTO device
             (id, installation_id, line_id, topology_position, source_path, source_ets_id,
              name, description, address, product_ref, program_ref, completion,
              individual_address_loaded, application_program_loaded, parameters_loaded,
              communication_part_loaded, medium_config_loaded, last_modified, last_download,
              broken, visibility_calculated)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                 ?17, ?18, ?19, ?20)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             description = excluded.description,
             address = excluded.address,
             product_ref = excluded.product_ref,
             program_ref = excluded.program_ref,
             completion = excluded.completion,
             individual_address_loaded = excluded.individual_address_loaded,
             application_program_loaded = excluded.application_program_loaded,
             parameters_loaded = excluded.parameters_loaded,
             communication_part_loaded = excluded.communication_part_loaded,
             medium_config_loaded = excluded.medium_config_loaded,
             last_modified = excluded.last_modified,
             last_download = excluded.last_download,
             broken = excluded.broken,
             visibility_calculated = excluded.visibility_calculated",
        params![
            device.id.0,
            installation_id.0,
            topology_position,
            device.source.path,
            device.source.ets_id,
            device.name,
            device.description,
            device.address.map(|a| a.raw()),
            device.product_ref,
            device.program_ref,
            completion_to_str(c.completion),
            c.individual_address_loaded,
            c.application_program_loaded,
            c.parameters_loaded,
            c.communication_part_loaded,
            c.medium_config_loaded,
            c.last_modified.map(|d| d.to_rfc3339()),
            c.last_download.map(|d| d.to_rfc3339()),
            c.broken,
            device.visibility_calculated,
        ],
    )?;
    // NOTE: the INSERT above always sets line_id = NULL — placement into a
    // line is Task 4/11's job (topology::upsert_line owns the device's
    // *membership*, not this row's own line_id column directly, since a
    // device can be reassigned between lines independent of its own
    // fields). A later task revisits this: see Task 11's orchestration
    // note on device placement, which sets `device.line_id` with a
    // separate targeted UPDATE once the owning line is known. Until then,
    // `topology::load_topology`'s "unassigned" query would misclassify
    // every device as unassigned — Task 11 must not skip that step.
    //
    // Note the ON CONFLICT clause deliberately omits both `line_id` and
    // `topology_position` from its SET list (unlike `topology::upsert_area`/
    // `upsert_line`, which do update their own `position` on conflict —
    // those columns are not co-owned by a second function the way a
    // device's placement is). Re-running upsert_device on an already-placed
    // device therefore leaves its existing line_id/topology_position
    // untouched; placement is set exclusively through `set_device_line`
    // below, which updates both columns together as one unit.
    conn.execute(
        "DELETE FROM binary_data_ref WHERE device_id = ?1",
        params![device.id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO binary_data_ref (device_id, position, blob_id, name) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, b) in device.binary_data.iter().enumerate() {
        stmt.execute(params![device.id.0, i as i64, b.id, b.name])?;
    }
    Ok(())
}

/// Sets a device's line placement directly — the one column `upsert_device`
/// deliberately leaves untouched. Called by Task 11's orchestration once
/// the owning `Line`'s id is known (or with `line_id = None` for an
/// unassigned device). Command persistence uses the transactional whole-project
/// save, not a separate incremental caller of this low-level helper.
pub fn set_device_line(
    conn: &Connection,
    device_id: DeviceId,
    line_id: Option<knx_core::ids::LineId>,
    topology_position: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE device SET line_id = ?1, topology_position = ?2 WHERE id = ?3",
        params![line_id.map(|l| l.0), topology_position, device_id.0],
    )?;
    Ok(())
}

/// Loads the `device` row and its `binary_data_ref` rows. `com_objects` is
/// left empty — it is filled in by Task 11's orchestration, which loads
/// `com_object_instance` rows (Task 7) for this device and assigns them
/// into the returned value's `com_objects` field.
pub fn load_device(conn: &Connection, id: DeviceId) -> Result<DeviceInstance, StoreError> {
    let (
        source_path,
        source_ets_id,
        name,
        description,
        address,
        product_ref,
        program_ref,
        completion,
        individual_address_loaded,
        application_program_loaded,
        parameters_loaded,
        communication_part_loaded,
        medium_config_loaded,
        last_modified,
        last_download,
        broken,
        visibility_calculated,
    ) = conn.query_row(
        "SELECT source_path, source_ets_id, name, description, address, product_ref,
                program_ref, completion, individual_address_loaded, application_program_loaded,
                parameters_loaded, communication_part_loaded, medium_config_loaded,
                last_modified, last_download, broken, visibility_calculated
         FROM device WHERE id = ?1",
        params![id.0],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<u16>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, bool>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, bool>(10)?,
                row.get::<_, bool>(11)?,
                row.get::<_, bool>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, bool>(15)?,
                row.get::<_, bool>(16)?,
            ))
        },
    )?;

    let mut bd_stmt = conn.prepare(
        "SELECT blob_id, name FROM binary_data_ref WHERE device_id = ?1 ORDER BY position",
    )?;
    let binary_data = bd_stmt
        .query_map(params![id.0], |row| {
            Ok(BinaryDataRef {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DeviceInstance {
        id,
        source: SourceRef {
            path: source_path,
            ets_id: source_ets_id,
        },
        name,
        description,
        address: address.map(IndividualAddress::from_raw),
        product_ref,
        program_ref,
        commissioning: CommissioningState {
            completion: completion_from_str(&completion)?,
            individual_address_loaded,
            application_program_loaded,
            parameters_loaded,
            communication_part_loaded,
            medium_config_loaded,
            last_modified: last_modified.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            last_download: last_download.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            broken,
            // Loaded from the project store, which stores intent only.
            // The device side of §11.2 is a fact nobody has yet checked.
            device_reported: Default::default(),
        },
        visibility_calculated,
        com_objects: vec![],
        binary_data,
    })
}

pub fn load_all_device_ids(conn: &Connection) -> Result<Vec<DeviceId>, StoreError> {
    let mut stmt = conn.prepare("SELECT id FROM device ORDER BY id")?;
    let ids = stmt
        .query_map([], |row| Ok(DeviceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Text,
    Description,
    Dpt,
    Read,
    Write,
    Transmit,
    Update,
    Communication,
    /// Schema 7 (§117). Absent from every pre-v7 project file, which is
    /// exactly right: no row means `Override::Absent`, never
    /// `Override::Value(false)`.
    ReadOnInit,
}

pub fn attr_to_str(a: Attr) -> &'static str {
    match a {
        Attr::Text => "text",
        Attr::Description => "description",
        Attr::Dpt => "dpt",
        Attr::Read => "read",
        Attr::Write => "write",
        Attr::Transmit => "transmit",
        Attr::Update => "update",
        Attr::Communication => "communication",
        Attr::ReadOnInit => "read_on_init",
    }
}

fn layer_to_str(l: Layer) -> &'static str {
    match l {
        Layer::Program => "Program",
        Layer::ProgramRef => "ProgramRef",
        Layer::Instance => "Instance",
        Layer::Inferred => "Inferred",
        Layer::UserEdit => "UserEdit",
    }
}

fn layer_from_str(s: &str) -> Layer {
    match s {
        "ProgramRef" => Layer::ProgramRef,
        "Instance" => Layer::Instance,
        "Inferred" => Layer::Inferred,
        "UserEdit" => Layer::UserEdit,
        _ => Layer::Program,
    }
}

/// One row of `com_object_override`, already string-encoded. Building this
/// is what turns each `Override<T>` field into the four columns the table
/// stores — the encoding side of the codec.
pub(crate) struct OverrideRow {
    pub(crate) state: &'static str,
    pub(crate) value: Option<String>,
    text_kind: Option<&'static str>,
    pub(crate) layer: Option<&'static str>,
}

fn encode_text(t: &Override<Text>) -> OverrideRow {
    match t {
        Override::Absent => OverrideRow {
            state: "absent",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Empty => OverrideRow {
            state: "empty",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => {
            let (kind, text) = match value {
                Text::Literal(s) => ("literal", s.clone()),
                Text::Localized(LocalizedString(TranslationKey(k))) => ("localized", k.clone()),
            };
            OverrideRow {
                state: "value",
                value: Some(text),
                text_kind: Some(kind),
                layer: Some(layer_to_str(*layer)),
            }
        }
    }
}

fn decode_text(
    state: &str,
    value: Option<String>,
    text_kind: Option<String>,
    layer: Option<String>,
) -> Override<Text> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => {
            let text = match text_kind.as_deref() {
                Some("localized") => Text::Localized(LocalizedString(TranslationKey(
                    value.expect("value state always carries a value"),
                ))),
                _ => Text::Literal(value.expect("value state always carries a value")),
            };
            Override::Value(Resolved {
                value: text,
                layer: layer_from_str(&layer.expect("value state always carries a layer")),
            })
        }
        _ => Override::Absent,
    }
}

pub(crate) fn encode_dpt(d: &Override<DptRef>) -> OverrideRow {
    match d {
        Override::Absent => OverrideRow {
            state: "absent",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Empty => OverrideRow {
            state: "empty",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => OverrideRow {
            state: "value",
            value: Some(value.to_string()),
            text_kind: None,
            layer: Some(layer_to_str(*layer)),
        },
    }
}

pub(crate) fn decode_dpt(
    state: &str,
    value: Option<String>,
    layer: Option<String>,
) -> Override<DptRef> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => Override::Value(Resolved {
            value: DptRef::parse(&value.expect("value state always carries a value"))
                .expect("stored DptRef text is always valid"),
            layer: layer_from_str(&layer.expect("value state always carries a layer")),
        }),
        _ => Override::Absent,
    }
}

fn encode_bool(b: &Override<bool>) -> OverrideRow {
    match b {
        Override::Absent => OverrideRow {
            state: "absent",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Empty => OverrideRow {
            state: "empty",
            value: None,
            text_kind: None,
            layer: None,
        },
        Override::Malformed(raw) => OverrideRow {
            state: "malformed",
            value: Some(raw.clone()),
            text_kind: None,
            layer: None,
        },
        Override::Value(Resolved { value, layer }) => OverrideRow {
            state: "value",
            value: Some(if *value { "1".into() } else { "0".into() }),
            text_kind: None,
            layer: Some(layer_to_str(*layer)),
        },
    }
}

fn decode_bool(state: &str, value: Option<String>, layer: Option<String>) -> Override<bool> {
    match state {
        "empty" => Override::Empty,
        "malformed" => Override::Malformed(value.expect("malformed state always carries a value")),
        "value" => Override::Value(Resolved {
            value: value.expect("value state always carries a value") == "1",
            layer: layer_from_str(&layer.expect("value state always carries a layer")),
        }),
        _ => Override::Absent,
    }
}

fn write_override_row(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    attr: Attr,
    row: OverrideRow,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO com_object_override
             (com_object_instance_id, attr, state, value, text_kind, layer)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(com_object_instance_id, attr) DO UPDATE SET
             state = excluded.state,
             value = excluded.value,
             text_kind = excluded.text_kind,
             layer = excluded.layer",
        params![
            com_object_instance_id.0,
            attr_to_str(attr),
            row.state,
            row.value,
            row.text_kind,
            row.layer,
        ],
    )?;
    Ok(())
}

/// Writes exactly one `com_object_override` row for the `dpt` field. This
/// low-level helper is not the whole-project command-persistence entry point.
pub fn upsert_com_object_dpt_override(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    dpt: &Override<DptRef>,
) -> Result<(), StoreError> {
    write_override_row(conn, com_object_instance_id, Attr::Dpt, encode_dpt(dpt))
}

/// Writes exactly one `com_object_override` row for the `description`
/// field. This low-level helper is not the whole-project command-persistence
/// entry point. Mirrors
/// `upsert_com_object_dpt_override`.
pub fn upsert_com_object_description_override(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    description: &Override<Text>,
) -> Result<(), StoreError> {
    write_override_row(
        conn,
        com_object_instance_id,
        Attr::Description,
        encode_text(description),
    )
}

fn size_columns(
    size: &Option<Resolved<ObjectSize>>,
) -> (Option<&'static str>, Option<i64>, Option<&'static str>) {
    match size {
        None => (None, None, None),
        Some(Resolved { value, layer }) => {
            let (kind, v) = match value {
                ObjectSize::Bit(n) => ("bit", *n as i64),
                ObjectSize::Byte(n) => ("byte", *n as i64),
            };
            (Some(kind), Some(v), Some(layer_to_str(*layer)))
        }
    }
}

fn decode_size(
    kind: Option<String>,
    value: Option<i64>,
    layer: Option<String>,
) -> Option<Resolved<ObjectSize>> {
    let kind = kind?;
    let value = value.expect("size_value present whenever size_kind is");
    let layer = layer.expect("size_layer present whenever size_kind is");
    let size = match kind.as_str() {
        "byte" => ObjectSize::Byte(value as u16),
        _ => ObjectSize::Bit(value as u8),
    };
    Some(Resolved {
        value: size,
        layer: layer_from_str(&layer),
    })
}

pub fn upsert_com_object_instance(
    conn: &Connection,
    device_id: DeviceId,
    position: i64,
    com: &ComObjectInstance,
) -> Result<(), StoreError> {
    let (size_kind, size_value, size_layer) = size_columns(&com.size);
    conn.execute(
        "INSERT INTO com_object_instance
             (id, device_id, position, source_path, source_ets_id, number, size_kind,
              size_value, size_layer, is_active, module_instance_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
             device_id = excluded.device_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             number = excluded.number,
             size_kind = excluded.size_kind,
             size_value = excluded.size_value,
             size_layer = excluded.size_layer,
             is_active = excluded.is_active,
             module_instance_id = excluded.module_instance_id",
        params![
            com.id.0,
            device_id.0,
            position,
            com.source.path,
            com.source.ets_id,
            com.number,
            size_kind,
            size_value,
            size_layer,
            com.is_active,
            com.module_instance.map(|m| m.0),
        ],
    )?;

    write_override_row(conn, com.id, Attr::Text, encode_text(&com.text))?;
    write_override_row(
        conn,
        com.id,
        Attr::Description,
        encode_text(&com.description),
    )?;
    write_override_row(conn, com.id, Attr::Dpt, encode_dpt(&com.dpt))?;
    write_override_row(conn, com.id, Attr::Read, encode_bool(&com.flags.read))?;
    write_override_row(conn, com.id, Attr::Write, encode_bool(&com.flags.write))?;
    write_override_row(
        conn,
        com.id,
        Attr::Transmit,
        encode_bool(&com.flags.transmit),
    )?;
    write_override_row(conn, com.id, Attr::Update, encode_bool(&com.flags.update))?;
    write_override_row(
        conn,
        com.id,
        Attr::Communication,
        encode_bool(&com.flags.communication),
    )?;
    write_override_row(
        conn,
        com.id,
        Attr::ReadOnInit,
        encode_bool(&com.flags.read_on_init),
    )?;
    Ok(())
}

fn apply_override(
    com: &mut ComObjectInstance,
    attr: &str,
    state: &str,
    value: Option<String>,
    text_kind: Option<String>,
    layer: Option<String>,
) -> Result<(), StoreError> {
    match attr {
        "text" => com.text = decode_text(state, value, text_kind, layer),
        "description" => com.description = decode_text(state, value, text_kind, layer),
        "dpt" => com.dpt = decode_dpt(state, value, layer),
        "read" => com.flags.read = decode_bool(state, value, layer),
        "write" => com.flags.write = decode_bool(state, value, layer),
        "transmit" => com.flags.transmit = decode_bool(state, value, layer),
        "update" => com.flags.update = decode_bool(state, value, layer),
        "communication" => com.flags.communication = decode_bool(state, value, layer),
        "read_on_init" => com.flags.read_on_init = decode_bool(state, value, layer),
        other => return Err(StoreError::UnknownOverrideAttr(other.to_string())),
    }
    Ok(())
}

pub fn load_com_object_instance(
    conn: &Connection,
    id: ComObjectInstanceId,
) -> Result<ComObjectInstance, StoreError> {
    let (
        source_path,
        source_ets_id,
        device_id,
        number,
        size_kind,
        size_value,
        size_layer,
        is_active,
        module_instance_id,
    ) = conn.query_row(
        "SELECT source_path, source_ets_id, device_id, number, size_kind, size_value,
                size_layer, is_active, module_instance_id
         FROM com_object_instance WHERE id = ?1",
        params![id.0],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u16>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, bool>(7)?,
                row.get::<_, Option<u32>>(8)?,
            ))
        },
    )?;
    let mut com = ComObjectInstance {
        id,
        source: SourceRef {
            path: source_path,
            ets_id: source_ets_id,
        },
        device: DeviceId(device_id),
        number,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: decode_size(size_kind, size_value, size_layer),
        is_active,
        links: vec![],
        module_instance: module_instance_id.map(ModuleInstanceId),
    };

    let mut stmt = conn.prepare(
        "SELECT attr, state, value, text_kind, layer FROM com_object_override
         WHERE com_object_instance_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![id.0], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (attr, state, value, text_kind, layer) in rows {
        apply_override(&mut com, &attr, &state, value, text_kind, layer)?;
    }
    Ok(com)
}

pub(crate) fn load_all_com_objects(
    conn: &Connection,
) -> Result<BTreeMap<DeviceId, Vec<ComObjectInstance>>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT com.id, com.device_id, com.source_path, com.source_ets_id, com.number,
                com.size_kind, com.size_value, com.size_layer, com.is_active,
                com.module_instance_id
         FROM com_object_instance AS com
         INNER JOIN device ON device.id = com.device_id
         ORDER BY com.device_id, com.position",
    )?;
    let mut objects = Vec::new();
    let mut indices = BTreeMap::new();
    let rows = stmt.query_map([], |row| {
        Ok(ComObjectInstance {
            id: ComObjectInstanceId(row.get(0)?),
            source: SourceRef {
                path: row.get(2)?,
                ets_id: row.get(3)?,
            },
            device: DeviceId(row.get(1)?),
            number: row.get(4)?,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: decode_size(row.get(5)?, row.get(6)?, row.get(7)?),
            is_active: row.get(8)?,
            links: Vec::new(),
            module_instance: row.get::<_, Option<u32>>(9)?.map(ModuleInstanceId),
        })
    })?;
    for row in rows {
        let com = row?;
        indices.insert(com.id, objects.len());
        objects.push(com);
    }

    let mut stmt = conn.prepare(
        "SELECT override.com_object_instance_id, override.attr, override.state,
                override.value, override.text_kind, override.layer
         FROM com_object_override AS override
         INNER JOIN com_object_instance AS com ON com.id = override.com_object_instance_id
         INNER JOIN device ON device.id = com.device_id
         ORDER BY override.com_object_instance_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            ComObjectInstanceId(row.get(0)?),
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    for row in rows {
        let (id, attr, state, value, text_kind, layer) = row?;
        let index = indices
            .get(&id)
            .copied()
            .ok_or_else(|| StoreError::from(rusqlite::Error::QueryReturnedNoRows))?;
        apply_override(&mut objects[index], &attr, &state, value, text_kind, layer)?;
    }

    let mut stmt = conn.prepare(
        "SELECT link.com_object_instance_id, link.group_address_id, link.direction
         FROM group_link AS link
         INNER JOIN com_object_instance AS com ON com.id = link.com_object_instance_id
         INNER JOIN device ON device.id = com.device_id
         ORDER BY link.com_object_instance_id, link.position",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            ComObjectInstanceId(row.get(0)?),
            GroupLink {
                ga: GroupAddressId(row.get(1)?),
                direction: direction_from_str(&row.get::<_, String>(2)?),
            },
        ))
    })?;
    for row in rows {
        let (id, link) = row?;
        let index = indices
            .get(&id)
            .copied()
            .ok_or_else(|| StoreError::from(rusqlite::Error::QueryReturnedNoRows))?;
        objects[index].links.push(link);
    }

    let mut by_device = BTreeMap::new();
    for com in objects {
        by_device
            .entry(com.device)
            .or_insert_with(Vec::new)
            .push(com);
    }
    Ok(by_device)
}

pub fn load_com_object_ids_for_device(
    conn: &Connection,
    device_id: DeviceId,
) -> Result<Vec<ComObjectInstanceId>, StoreError> {
    let mut stmt =
        conn.prepare("SELECT id FROM com_object_instance WHERE device_id = ?1 ORDER BY position")?;
    let ids = stmt
        .query_map(params![device_id.0], |row| {
            Ok(ComObjectInstanceId(row.get(0)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn direction_to_str(d: Direction) -> &'static str {
    match d {
        Direction::Send => "send",
        Direction::Receive => "receive",
    }
}

fn direction_from_str(s: &str) -> Direction {
    match s {
        "receive" => Direction::Receive,
        _ => Direction::Send,
    }
}

pub fn upsert_group_links(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    links: &[GroupLink],
) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM group_link WHERE com_object_instance_id = ?1",
        params![com_object_instance_id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO group_link (com_object_instance_id, group_address_id, direction, position)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, link) in links.iter().enumerate() {
        stmt.execute(params![
            com_object_instance_id.0,
            link.ga.0,
            direction_to_str(link.direction),
            i as i64,
        ])?;
    }
    Ok(())
}

pub fn load_group_links(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
) -> Result<Vec<GroupLink>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT group_address_id, direction FROM group_link
         WHERE com_object_instance_id = ?1 ORDER BY position",
    )?;
    let links = stmt
        .query_map(params![com_object_instance_id.0], |row| {
            let ga: u32 = row.get(0)?;
            let direction: String = row.get(1)?;
            Ok((ga, direction))
        })?
        .map(|r| {
            let (ga, direction) = r?;
            Ok(GroupLink {
                ga: GroupAddressId(ga),
                direction: direction_from_str(&direction),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(links)
}

/// Writes one `com_object_program_default` row. Reuses `Attr` and its
/// `attr_to_str` — same three attribute names (`text`/`description`/`dpt`)
/// as `com_object_override`, since this is the program layer's own say on
/// the same three fields, not a fourth kind of attribute.
fn write_program_default_row(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    attr: Attr,
    value: &str,
    text_kind: Option<&str>,
    layer: Layer,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO com_object_program_default
             (com_object_instance_id, attr, value, text_kind, layer)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(com_object_instance_id, attr) DO UPDATE SET
             value = excluded.value,
             text_kind = excluded.text_kind,
             layer = excluded.layer",
        params![
            com_object_instance_id.0,
            attr_to_str(attr),
            value,
            text_kind,
            layer_to_str(layer),
        ],
    )?;
    Ok(())
}

/// Replaces every `com_object_program_default` row for `id` with what
/// `defaults` now says — deletes first, same "rewrite the whole set" shape
/// `upsert_com_object_instance` uses for `com_object_override`, since a
/// field that goes from `Some` back to `None` (the program database
/// changed, or was dropped) must not leave a stale row behind.
pub fn upsert_com_object_program_defaults(
    conn: &Connection,
    com_object_instance_id: ComObjectInstanceId,
    defaults: &knx_core::device::ProgramDefaults,
) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM com_object_program_default WHERE com_object_instance_id = ?1",
        params![com_object_instance_id.0],
    )?;
    if let Some(Resolved { value, layer }) = &defaults.text {
        let (kind, text) = match value {
            Text::Literal(s) => ("literal", s.clone()),
            Text::Localized(LocalizedString(TranslationKey(k))) => ("localized", k.clone()),
        };
        write_program_default_row(
            conn,
            com_object_instance_id,
            Attr::Text,
            &text,
            Some(kind),
            *layer,
        )?;
    }
    if let Some(Resolved { value, layer }) = &defaults.description {
        let (kind, text) = match value {
            Text::Literal(s) => ("literal", s.clone()),
            Text::Localized(LocalizedString(TranslationKey(k))) => ("localized", k.clone()),
        };
        write_program_default_row(
            conn,
            com_object_instance_id,
            Attr::Description,
            &text,
            Some(kind),
            *layer,
        )?;
    }
    if let Some(Resolved { value, layer }) = &defaults.dpt {
        write_program_default_row(
            conn,
            com_object_instance_id,
            Attr::Dpt,
            &value.to_string(),
            None,
            *layer,
        )?;
    }
    Ok(())
}

/// Every com object's program defaults, in one pass — `load_project`'s
/// counterpart to `load_all_com_objects`, same "one query beats N" reason.
pub(crate) fn load_all_program_defaults(
    conn: &Connection,
) -> Result<BTreeMap<ComObjectInstanceId, knx_core::device::ProgramDefaults>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT com_object_instance_id, attr, value, text_kind, layer
         FROM com_object_program_default
         ORDER BY com_object_instance_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                ComObjectInstanceId(row.get(0)?),
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut by_id: BTreeMap<ComObjectInstanceId, knx_core::device::ProgramDefaults> =
        BTreeMap::new();
    for (id, attr, value, text_kind, layer) in rows {
        let entry = by_id.entry(id).or_default();
        let layer = layer_from_str(&layer);
        match attr.as_str() {
            "text" => {
                let text = match text_kind.as_deref() {
                    Some("localized") => Text::Localized(LocalizedString(TranslationKey(value))),
                    _ => Text::Literal(value),
                };
                entry.text = Some(Resolved { value: text, layer });
            }
            "description" => {
                let text = match text_kind.as_deref() {
                    Some("localized") => Text::Localized(LocalizedString(TranslationKey(value))),
                    _ => Text::Literal(value),
                };
                entry.description = Some(Resolved { value: text, layer });
            }
            "dpt" => {
                let dpt = DptRef::parse(&value).expect("stored DptRef text is always valid");
                entry.dpt = Some(Resolved { value: dpt, layer });
            }
            other => return Err(StoreError::UnknownOverrideAttr(other.to_string())),
        }
    }
    Ok(by_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::CompletionStatus;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn device() -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "0.xml".into(),
                ets_id: "M-1".into(),
            },
            name: "Aktor".into(),
            description: Some("Schaltaktor".into()),
            address: Some(IndividualAddress::new(1, 1, 5).unwrap()),
            product_ref: "P-1".into(),
            program_ref: "H-1".into(),
            commissioning: CommissioningState {
                completion: CompletionStatus::FinishedDesign,
                individual_address_loaded: true,
                application_program_loaded: true,
                parameters_loaded: false,
                communication_part_loaded: true,
                medium_config_loaded: false,
                last_modified: None,
                last_download: None,
                broken: false,
                device_reported: Default::default(),
            },
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![
                BinaryDataRef {
                    id: "guid-1".into(),
                    name: "cert.dat".into(),
                },
                BinaryDataRef {
                    id: "guid-2".into(),
                    name: "key.dat".into(),
                },
            ],
        }
    }

    #[test]
    fn a_device_with_binary_data_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name);
        assert_eq!(loaded.binary_data, d.binary_data);
        assert_eq!(loaded.commissioning, d.commissioning);
        assert_eq!(loaded.address, d.address);
    }

    #[test]
    fn a_device_without_an_address_or_description_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let mut d = device();
        d.address = None;
        d.description = None;
        d.binary_data = vec![];
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.address, None);
        assert_eq!(loaded.description, None);
        assert_eq!(loaded.binary_data, vec![]);
    }

    #[test]
    fn set_device_line_updates_placement_without_touching_other_fields() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        // A line row must exist to satisfy the foreign key.
        conn.execute(
            "INSERT INTO area (id, installation_id, position, source_path, source_ets_id, name, address, completion)
             VALUES (1, 0, 0, 't', 't', 'A', 1, 'FinishedDesign')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO line (id, area_id, position, source_path, source_ets_id, name, address, medium_ref, completion)
             VALUES (1, 1, 0, 't', 't', 'L', 1, 'TP', 'FinishedDesign')",
            [],
        )
        .unwrap();
        set_device_line(&conn, d.id, Some(knx_core::ids::LineId(1)), 0).unwrap();
        let line_id: Option<i64> = conn
            .query_row("SELECT line_id FROM device WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(line_id, Some(1));
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name); // untouched
    }

    #[test]
    fn re_upserting_a_placed_device_does_not_reset_its_placement() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        // A line row must exist to satisfy the foreign key.
        conn.execute(
            "INSERT INTO area (id, installation_id, position, source_path, source_ets_id, name, address, completion)
             VALUES (1, 0, 0, 't', 't', 'A', 1, 'FinishedDesign')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO line (id, area_id, position, source_path, source_ets_id, name, address, medium_ref, completion)
             VALUES (1, 1, 0, 't', 't', 'L', 1, 'TP', 'FinishedDesign')",
            [],
        )
        .unwrap();
        set_device_line(&conn, d.id, Some(knx_core::ids::LineId(1)), 3).unwrap();

        // Re-upsert the same device (e.g. an unrelated field changed) — this
        // must NOT reset line_id/topology_position back to NULL/0, the exact
        // bug this task's ON CONFLICT fix closes.
        let mut d2 = d.clone();
        d2.name = "Renamed".into();
        upsert_device(&conn, InstallationId(0), 0, &d2).unwrap();

        let (line_id, position): (Option<i64>, i64) = conn
            .query_row(
                "SELECT line_id, topology_position FROM device WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(line_id, Some(1));
        assert_eq!(position, 3);
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, "Renamed");
    }

    fn com_object_fixture() -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: SourceRef {
                path: "0.xml".into(),
                ets_id: "M-1_O-0_R-1".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("An/Aus".into()),
                layer: Layer::Instance,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::UserEdit,
            }),
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Instance,
                }),
                write: Override::Absent,
                transmit: Override::Empty,
                update: Override::Malformed("???".into()),
                communication: Override::Value(Resolved {
                    value: false,
                    layer: Layer::Program,
                }),
                read_on_init: Override::Value(Resolved {
                    value: true,
                    layer: Layer::ProgramRef,
                }),
            },
            size: Some(Resolved {
                value: ObjectSize::Bit(1),
                layer: Layer::Program,
            }),
            is_active: true,
            links: vec![],
            module_instance: None,
        }
    }

    #[test]
    fn a_com_object_instance_round_trips_every_override_state() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.text, com.text);
        assert_eq!(loaded.description, com.description);
        assert_eq!(loaded.dpt, com.dpt);
        assert_eq!(loaded.flags, com.flags);
        assert_eq!(loaded.size, com.size);
        assert_eq!(loaded.is_active, com.is_active);
    }

    #[test]
    fn program_defaults_round_trip_and_an_empty_replacement_clears_the_row() {
        use knx_core::device::ProgramDefaults;

        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();

        let defaults = ProgramDefaults {
            text: Some(Resolved {
                value: Text::Literal("An/Aus (Programm)".into()),
                layer: Layer::Program,
            }),
            description: Some(Resolved {
                value: Text::Localized(LocalizedString(TranslationKey("k9".into()))),
                layer: Layer::ProgramRef,
            }),
            dpt: Some(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(2),
                },
                layer: Layer::Program,
            }),
        };
        upsert_com_object_program_defaults(&conn, com.id, &defaults).unwrap();

        let loaded = load_all_program_defaults(&conn).unwrap();
        assert_eq!(loaded.get(&com.id), Some(&defaults));

        // Replacing with an empty ProgramDefaults deletes the rows rather
        // than leaving three stale ones behind.
        upsert_com_object_program_defaults(&conn, com.id, &ProgramDefaults::default()).unwrap();
        let loaded = load_all_program_defaults(&conn).unwrap();
        assert_eq!(loaded.get(&com.id), None);
    }

    #[test]
    fn a_localized_text_override_round_trips_distinct_from_a_literal_one() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let mut com = com_object_fixture();
        com.text = Override::Value(Resolved {
            value: Text::Localized(LocalizedString(TranslationKey("k1".into()))),
            layer: Layer::Program,
        });
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.text, com.text);
        assert_ne!(
            loaded.text,
            Override::Value(Resolved {
                value: Text::Literal("k1".into()),
                layer: Layer::Program
            })
        );
    }

    #[test]
    fn upsert_com_object_dpt_override_touches_only_the_dpt_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();

        let new_dpt = Override::Value(Resolved {
            value: DptRef {
                main: 5,
                sub: Some(1),
            },
            layer: Layer::UserEdit,
        });
        upsert_com_object_dpt_override(&conn, com.id, &new_dpt).unwrap();

        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.dpt, new_dpt);
        assert_eq!(loaded.text, com.text); // untouched by the targeted upsert
        assert_eq!(loaded.description, com.description); // untouched
        assert_eq!(loaded.flags, com.flags); // untouched
        assert_eq!(loaded.size, com.size); // untouched
        assert_eq!(loaded.is_active, com.is_active); // untouched
    }

    #[test]
    fn upsert_com_object_description_override_touches_only_the_description_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();

        let new_description = Override::Value(Resolved {
            value: Text::Literal("Aktoreingang 1".into()),
            layer: Layer::UserEdit,
        });
        upsert_com_object_description_override(&conn, com.id, &new_description).unwrap();

        let loaded = load_com_object_instance(&conn, com.id).unwrap();
        assert_eq!(loaded.description, new_description);
        assert_eq!(loaded.text, com.text); // untouched by the targeted upsert
        assert_eq!(loaded.dpt, com.dpt); // untouched
        assert_eq!(loaded.flags, com.flags); // untouched
        assert_eq!(loaded.size, com.size); // untouched
        assert_eq!(loaded.is_active, com.is_active); // untouched
    }

    /// A `com_object_override.attr` this build does not know reaches
    /// `load_com_object_instance` from any database this code did not write
    /// — hand-edited, corrupted, or written by a newer/third-party tool. It
    /// is an error to report, not a process to abort.
    #[test]
    fn an_unknown_override_attribute_is_an_error_not_a_panic() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        conn.execute(
            "INSERT INTO com_object_override
                 (com_object_instance_id, attr, state, value, text_kind, layer)
             VALUES (?1, 'priority', 'value', 'low', NULL, 'Instance')",
            params![com.id.0],
        )
        .unwrap();

        match load_com_object_instance(&conn, com.id) {
            Err(StoreError::UnknownOverrideAttr(attr)) => assert_eq!(attr, "priority"),
            other => panic!("expected UnknownOverrideAttr, got {other:?}"),
        }
    }

    #[test]
    fn group_links_round_trip_in_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let com = com_object_fixture();
        upsert_com_object_instance(&conn, d.id, 0, &com).unwrap();
        for id in [10, 20] {
            conn.execute(
                &format!(
                    "INSERT INTO group_address (id, installation_id, range_id, position,
                        source_path, source_ets_id, name, address, central, unfiltered)
                     VALUES ({id}, 0, NULL, 0, 't', 't', 'GA', {id}, 0, 0)"
                ),
                [],
            )
            .unwrap();
        }
        let links = vec![
            GroupLink {
                ga: GroupAddressId(20),
                direction: Direction::Send,
            },
            GroupLink {
                ga: GroupAddressId(10),
                direction: Direction::Receive,
            },
        ];
        upsert_group_links(&conn, com.id, &links).unwrap();
        assert_eq!(load_group_links(&conn, com.id).unwrap(), links);
    }

    #[test]
    fn bulk_loader_preserves_device_ownership_positions_and_links() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();

        let d1 = device();
        let mut d2 = device();
        d2.id = DeviceId(2);
        upsert_device(&conn, InstallationId(0), 0, &d1).unwrap();
        upsert_device(&conn, InstallationId(0), 1, &d2).unwrap();

        conn.execute(
            "INSERT INTO module_instance
             (id, device_id, position, source_path, source_ets_id, repeat_index, instance_ets_id)
             VALUES (7, 1, 0, '0.xml', 'MI-7', '', 'MI-7')",
            [],
        )
        .unwrap();
        for id in [10, 20] {
            conn.execute(
                "INSERT INTO group_address
                 (id, installation_id, range_id, position, source_path, source_ets_id,
                  name, address, central, unfiltered)
                 VALUES (?1, 0, NULL, ?2, '0.xml', ?3, 'GA', ?4, 0, 0)",
                params![id, id, format!("GA-{id}"), id],
            )
            .unwrap();
        }

        let mut first = com_object_fixture();
        first.id = ComObjectInstanceId(9);
        first.device = d1.id;
        first.module_instance = Some(ModuleInstanceId(7));
        let mut second = com_object_fixture();
        second.id = ComObjectInstanceId(3);
        second.device = d1.id;
        second.number = 1;
        let mut third = com_object_fixture();
        third.id = ComObjectInstanceId(4);
        third.device = d2.id;

        upsert_com_object_instance(&conn, d1.id, 0, &first).unwrap();
        upsert_com_object_instance(&conn, d1.id, 1, &second).unwrap();
        upsert_com_object_instance(&conn, d2.id, 0, &third).unwrap();
        let expected_links = vec![
            GroupLink {
                ga: GroupAddressId(20),
                direction: Direction::Receive,
            },
            GroupLink {
                ga: GroupAddressId(10),
                direction: Direction::Send,
            },
        ];
        upsert_group_links(&conn, first.id, &expected_links).unwrap();

        let loaded = load_all_com_objects(&conn).unwrap();
        assert_eq!(
            loaded[&DeviceId(1)]
                .iter()
                .map(|com| com.id)
                .collect::<Vec<_>>(),
            vec![ComObjectInstanceId(9), ComObjectInstanceId(3)]
        );
        assert_eq!(loaded[&DeviceId(1)][0].links, expected_links);
        assert_eq!(
            loaded[&DeviceId(1)][0].module_instance,
            Some(ModuleInstanceId(7))
        );
        assert_eq!(loaded[&DeviceId(1)][0].text, first.text);
        assert_eq!(loaded[&DeviceId(1)][0].description, first.description);
        assert_eq!(loaded[&DeviceId(1)][0].dpt, first.dpt);
        assert_eq!(loaded[&DeviceId(1)][0].flags, first.flags);
        assert_eq!(loaded[&DeviceId(1)][0].size, first.size);
        assert_eq!(loaded[&DeviceId(1)][0].is_active, first.is_active);
        assert_eq!(
            loaded[&DeviceId(2)]
                .iter()
                .map(|com| com.id)
                .collect::<Vec<_>>(),
            vec![ComObjectInstanceId(4)]
        );
    }
}
