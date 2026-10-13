//! Persists project Functions, their ordered members and both allocator
//! high-water marks (schema 15, ADR-0124).

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;
use knx_core::function::{validate_functions, Function, FunctionDetails, FunctionMember};
use knx_core::{BuildingPartId, FunctionId, FunctionMemberId, GroupAddressId, Project, SourceRef};
use rusqlite::{params, Connection};

fn invalid(reason: impl Into<String>) -> StoreError {
    StoreError::FunctionIntegrity(reason.into())
}

/// The ownership/reference rules plus allocators that cover every ID.
pub(crate) fn check(project: &Project) -> Result<(), StoreError> {
    validate_functions(project).map_err(|e| invalid(e.to_string()))?;
    if project
        .functions
        .iter()
        .any(|f| f.id.0 > project.ids.peek_function())
    {
        return Err(invalid(
            "function allocator is below an existing function id",
        ));
    }
    if project
        .functions
        .iter()
        .flat_map(|f| &f.members)
        .any(|m| m.id.0 > project.ids.peek_function_member())
    {
        return Err(invalid("member allocator is below an existing member id"));
    }
    Ok(())
}

pub(crate) fn write(conn: &Connection, project: &Project) -> Result<(), StoreError> {
    check(project)?;
    conn.execute(
        "INSERT INTO project_function_state(id,last_function_id,last_member_id) VALUES(0,?1,?2)",
        params![
            project.ids.peek_function(),
            project.ids.peek_function_member()
        ],
    )?;
    let mut function_row = conn.prepare(
        "INSERT INTO project_function(id,position,location,source_path,source_ets_id,name,\
         function_type,number,comment,description,completion) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )?;
    let mut member_row = conn.prepare(
        "INSERT INTO project_function_member(id,function_id,position,group_address,\
         source_path,source_ets_id,name,role) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
    )?;
    for (position, function) in project.functions.iter().enumerate() {
        let d = &function.details;
        function_row.execute(params![
            function.id.0,
            position as i64,
            function.location.0,
            function.source.path,
            function.source.ets_id,
            d.name,
            d.function_type,
            d.number,
            d.comment,
            d.description,
            completion_to_str(d.completion),
        ])?;
        for (position, member) in function.members.iter().enumerate() {
            member_row.execute(params![
                member.id.0,
                function.id.0,
                position as i64,
                member.group_address.0,
                member.source.path,
                member.source.ets_id,
                member.name,
                member.role,
            ])?;
        }
    }
    Ok(())
}

fn id_u32(value: i64, what: &str) -> Result<u32, StoreError> {
    u32::try_from(value).map_err(|_| invalid(format!("{what} is outside u32")))
}

pub(crate) fn load(conn: &Connection, project: &mut Project) -> Result<(), StoreError> {
    let state_rows: i64 =
        conn.query_row("SELECT count(*) FROM project_function_state", [], |r| {
            r.get(0)
        })?;
    if state_rows != 1 {
        return Err(invalid(
            "function allocator singleton is missing or duplicated",
        ));
    }
    let (last_function, last_member): (i64, i64) = conn.query_row(
        "SELECT last_function_id,last_member_id FROM project_function_state WHERE id=0",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let last_function = id_u32(last_function, "function allocator")?;
    let last_member = id_u32(last_member, "member allocator")?;

    let mut functions: Vec<Function> = Vec::new();
    let mut statement = conn.prepare(
        "SELECT id,position,location,source_path,source_ets_id,name,function_type,number,\
         comment,description,completion FROM project_function ORDER BY position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, i64>(1)? != functions.len() as i64 {
            return Err(invalid("function positions are not contiguous"));
        }
        functions.push(Function {
            id: FunctionId(id_u32(row.get(0)?, "function id")?),
            location: BuildingPartId(id_u32(row.get(2)?, "function location")?),
            source: SourceRef {
                path: row.get(3)?,
                ets_id: row.get(4)?,
            },
            details: FunctionDetails {
                name: row.get(5)?,
                function_type: row.get(6)?,
                number: row.get(7)?,
                comment: row.get(8)?,
                description: row.get(9)?,
                completion: completion_from_str(&row.get::<_, String>(10)?)?,
            },
            members: Vec::new(),
        });
    }

    let mut statement = conn.prepare(
        "SELECT id,function_id,position,group_address,source_path,source_ets_id,name,role \
         FROM project_function_member ORDER BY function_id,position",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let function = FunctionId(id_u32(row.get(1)?, "member function id")?);
        let owner = functions
            .iter_mut()
            .find(|f| f.id == function)
            .ok_or_else(|| invalid("a member refers to a missing function"))?;
        if row.get::<_, i64>(2)? != owner.members.len() as i64 {
            return Err(invalid("member positions are not contiguous"));
        }
        owner.members.push(FunctionMember {
            id: FunctionMemberId(id_u32(row.get(0)?, "member id")?),
            group_address: GroupAddressId(id_u32(row.get(3)?, "member group address")?),
            source: SourceRef {
                path: row.get(4)?,
                ets_id: row.get(5)?,
            },
            name: row.get(6)?,
            role: row.get(7)?,
        });
    }

    project.functions = functions;
    validate_functions(project).map_err(|e| invalid(e.to_string()))?;
    // A stale counter is not refused here: `load_project_reporting` raises
    // it through the explicit allocator-repair report, as for notes.
    project.ids = project
        .ids
        .clone()
        .with_function_counts(last_function, last_member);
    Ok(())
}
