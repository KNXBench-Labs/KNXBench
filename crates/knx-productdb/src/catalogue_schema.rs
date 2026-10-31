//! Bounded private admission of the exact fresh-chain catalogue schema.
//!
//! This is not a full witness, reservation or installation capability. A shared
//! transaction borrow does not prevent mutation by another same-connection call.

use crate::operation::{PublicationGate, Stop};
use rusqlite::types::{Value, ValueRef};
use rusqlite::Transaction;

mod schema28;
mod witness;

/// The admitted `user_version`. The pinned fresh-chain shape (`schema28`) is
/// the one v28 introduced; v29 (WP-08), v30 (WP-03) and v31 (master BOM ranges) are
/// data-only migrations and change no DDL. Any later version must re-establish its
/// shape before it is admitted.
const ADMITTED_VERSION: i64 = 31;
const _: () = assert!(ADMITTED_VERSION == crate::migration::CURRENT_PRODUCTDB_VERSION);

pub(crate) use witness::{
    copy_content_witness, witness_current, CatalogueWitness, CopyContentWitness, WitnessError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Limit {
    Objects,
    Tables,
    Columns,
    Identifier,
    Sql,
    Metadata,
}

#[derive(Debug)]
pub(crate) enum AdmissionError {
    UnsupportedVersion(i64),
    UnsupportedSchemaShape,
    WitnessSchemaAdmissionLimit(Limit),
    Observation(rusqlite::Error),
    Stop(Stop),
}

impl From<rusqlite::Error> for AdmissionError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Observation(error)
    }
}

#[derive(Default)]
struct Budget {
    used: u64,
}

impl Budget {
    fn record(&mut self, values: &[ValueRef<'_>]) -> Result<(), AdmissionError> {
        let limit = || AdmissionError::WitnessSchemaAdmissionLimit(Limit::Metadata);
        let mut charge = 8_u64;
        for value in values {
            let payload = match value {
                ValueRef::Null => 0,
                ValueRef::Integer(_) | ValueRef::Real(_) => 8,
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => 8_u64
                    .checked_add(u64::try_from(bytes.len()).map_err(|_| limit())?)
                    .ok_or_else(limit)?,
            };
            charge = charge
                .checked_add(1)
                .and_then(|n| n.checked_add(payload))
                .ok_or_else(limit)?;
        }
        let next = self.used.checked_add(charge).ok_or_else(limit)?;
        if next > 2_097_152 {
            return Err(limit());
        }
        self.used = next;
        Ok(())
    }
}

// Each tuple retains the actual SQLite storage class. Fixed widths and the
// named containers separate the distinct PRAGMA contracts, without JSON.
#[derive(Debug, PartialEq)]
struct Record<const N: usize>([Value; N]);

#[derive(Debug, PartialEq)]
struct Index {
    list: Record<5>,       // seq, name, unique, origin, partial
    xinfo: Vec<Record<6>>, // seqno, cid, name, desc, coll, key
}

#[derive(Debug, PartialEq)]
struct Relation {
    table: Record<6>,        // schema, name, type, ncol, wr, strict
    columns: Vec<Record<7>>, // cid, name, type, notnull, default, pk, hidden
    indexes: Vec<Index>,
    foreign_keys: Vec<Record<8>>, // id, seq, table, from, to, on_update, on_delete, match
}

struct Admitted<'a, 'c> {
    _transaction: &'a Transaction<'c>,
    objects: Vec<Record<6>>, // rowid, type, name, tbl_name, rootpage, nullable sql
    relations: Vec<Relation>,
    metadata_bytes: u64,
}

#[derive(Clone, Copy)]
enum Field {
    Integer,
    Identifier,
    Sql,
}

fn bound(found: i64, ceiling: i64, kind: Limit) -> Result<(), AdmissionError> {
    if found < 0 || found > ceiling {
        Err(AdmissionError::WitnessSchemaAdmissionLimit(kind))
    } else {
        Ok(())
    }
}

// Inspect all borrowed fields and the complete charge before the first owned
// scalar copy. Every repeated occurrence is charged; NULL SQL stays NULL.
fn retain<const N: usize>(
    values: [ValueRef<'_>; N],
    fields: [Field; N],
    budget: &mut Budget,
) -> Result<Record<N>, AdmissionError> {
    for (value, field) in values.iter().zip(fields) {
        match (value, field) {
            (ValueRef::Integer(_), Field::Integer)
            | (ValueRef::Null, Field::Sql | Field::Identifier) => {}
            (ValueRef::Text(bytes), Field::Identifier | Field::Sql) => {
                let (max, kind) = match field {
                    Field::Identifier => (256, Limit::Identifier),
                    _ => (262_144, Limit::Sql),
                };
                bound(
                    i64::try_from(bytes.len())
                        .map_err(|_| AdmissionError::WitnessSchemaAdmissionLimit(kind))?,
                    max,
                    kind,
                )?;
            }
            _ => return Err(AdmissionError::UnsupportedSchemaShape),
        }
    }
    budget.record(&values)?;
    let mut owned = [const { Value::Null }; N];
    for (i, value) in values.into_iter().enumerate() {
        #[cfg(test)]
        tests::owned_scalar_copy();
        owned[i] = Value::try_from(value).map_err(|cause| {
            AdmissionError::Observation(rusqlite::Error::FromSqlConversionFailure(
                i,
                value.data_type(),
                Box::new(cause),
            ))
        })?;
    }
    Ok(Record(owned))
}

fn matches<const N: usize>(actual: &Record<N>, expected: &[ValueRef<'_>; N]) -> bool {
    actual
        .0
        .iter()
        .zip(expected)
        .all(|(a, b)| ValueRef::from(a) == *b)
}

fn exact<const N: usize>(
    actual: &[Record<N>],
    expected: &[[ValueRef<'_>; N]],
) -> Result<(), AdmissionError> {
    if actual.len() != expected.len() || !actual.iter().zip(expected).all(|(a, b)| matches(a, b)) {
        return Err(AdmissionError::UnsupportedSchemaShape);
    }
    Ok(())
}

struct Inspection<'a, 'c, 'g> {
    tx: &'a Transaction<'c>,
    gate: &'g PublicationGate,
    budget: Budget,
}

impl Inspection<'_, '_, '_> {
    fn checkpoint(&self) -> Result<(), AdmissionError> {
        self.gate.checkpoint().map_err(AdmissionError::Stop)
    }

    // SQL checkpoints bracket prepare, query/step and scalar reads, including
    // unsuccessful SQL calls. Native internal work is not preempted or bounded.
    fn sql<T>(&self, work: impl FnOnce() -> rusqlite::Result<T>) -> Result<T, AdmissionError> {
        self.checkpoint()?;
        let result = work();
        self.checkpoint()?;
        Ok(result?)
    }

    fn scope(&self) -> Result<(), AdmissionError> {
        let version: i64 = self.sql(|| {
            self.tx
                .query_row("PRAGMA main.user_version", [], |r| r.get(0))
        })?;
        if version != ADMITTED_VERSION {
            return Err(AdmissionError::UnsupportedVersion(version));
        }
        let utf8 = self.sql(|| {
            self.tx.query_row("PRAGMA main.encoding", [], |r| {
                Ok(r.get_ref(0)? == ValueRef::Text(b"UTF-8"))
            })
        })?;
        if !utf8 {
            return Err(AdmissionError::UnsupportedSchemaShape);
        }
        let mut statement = self.sql(|| self.tx.prepare("PRAGMA database_list"))?;
        let mut rows = self.sql(|| statement.query([]))?;
        while let Some(row) = self.sql(|| rows.next())? {
            // Never fetch or retain the filesystem-path column.
            let name = row.get_ref(1)?;
            if name != ValueRef::Text(b"main") && name != ValueRef::Text(b"temp") {
                return Err(AdmissionError::UnsupportedSchemaShape);
            }
            self.checkpoint()?;
        }
        let temp: i64 = self.sql(|| {
            self.tx
                .query_row("SELECT count(*) FROM temp.sqlite_schema", [], |r| r.get(0))
        })?;
        if temp != 0 {
            return Err(AdmissionError::UnsupportedSchemaShape);
        }
        Ok(())
    }

    fn preflight(&self) -> Result<(), AdmissionError> {
        let sizes: [i64; 4] = self.sql(|| self.tx.query_row("SELECT count(*), coalesce(max(length(CAST(name AS BLOB))),0), coalesce(max(length(CAST(tbl_name AS BLOB))),0), coalesce(max(length(CAST(sql AS BLOB))),0) FROM main.sqlite_schema", [], |r| Ok([r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?])))?;
        for (found, (ceiling, kind)) in sizes.into_iter().zip([
            (256, Limit::Objects),
            (256, Limit::Identifier),
            (256, Limit::Identifier),
            (262_144, Limit::Sql),
        ]) {
            bound(found, ceiling, kind)?;
        }
        // A table-valued PRAGMA name can resolve to an unadmitted table/view.
        // Direct PRAGMA cannot be shadowed; inspect borrowed scalars only here.
        let mut statement = self.sql(|| self.tx.prepare("PRAGMA main.table_list"))?;
        let mut rows = self.sql(|| statement.query([]))?;
        let (mut tables, mut columns) = (0_i64, 0_i64);
        while let Some(row) = self.sql(|| rows.next())? {
            let values = self.sql(|| Ok((row.get_ref(0)?, row.get_ref(1)?, row.get_ref(3)?)))?;
            let (ValueRef::Text(schema), ValueRef::Text(name), ValueRef::Integer(ncol)) = values
            else {
                return Err(AdmissionError::UnsupportedSchemaShape);
            };
            if schema == b"main" && name != b"sqlite_schema" {
                tables = tables
                    .checked_add(1)
                    .ok_or(AdmissionError::WitnessSchemaAdmissionLimit(Limit::Tables))?;
                columns = columns.max(ncol);
            }
            self.checkpoint()?;
        }
        bound(tables, 64, Limit::Tables)?;
        bound(columns, 256, Limit::Columns)?;
        Ok(())
    }

    // Only fixed internal queries reach this helper. The parameter is a fixed
    // expected identifier, admitted by the object scan (or sqlite_schema itself).
    // Binding PRAGMA function arguments avoids SQL identifier interpolation.
    fn records<const N: usize>(
        &mut self,
        sql: &str,
        name: &str,
        fields: [Field; N],
        maximum: usize,
    ) -> Result<Vec<Record<N>>, AdmissionError> {
        let mut statement = self.sql(|| self.tx.prepare(sql))?;
        let mut rows = self.sql(|| statement.query([name]))?;
        let mut result = Vec::new();
        while let Some(row) = self.sql(|| rows.next())? {
            self.checkpoint()?;
            if result.len() == maximum {
                return Err(AdmissionError::UnsupportedSchemaShape);
            }
            let mut values = [ValueRef::Null; N];
            for (i, value) in values.iter_mut().enumerate() {
                *value = row.get_ref(i)?;
            }
            let record = retain(values, fields, &mut self.budget)?;
            result.push(record);
            #[cfg(test)]
            tests::record_boundary();
            self.checkpoint()?;
        }
        Ok(result)
    }

    fn objects(&mut self) -> Result<Vec<Record<6>>, AdmissionError> {
        use Field::{Identifier as Id, Integer as Int, Sql};
        let objects = self.records("SELECT rowid,type,name,tbl_name,rootpage,sql FROM main.sqlite_schema WHERE ?1='' ORDER BY name COLLATE BINARY", "", [Int,Id,Id,Id,Int,Sql],256)?;
        if objects.len() != schema28::OBJECTS.len() {
            return Err(AdmissionError::UnsupportedSchemaShape);
        }
        for (actual, expected) in objects.iter().zip(schema28::OBJECTS) {
            for (slot, value) in [1, 2, 3, 5].into_iter().zip(expected) {
                if ValueRef::from(&actual.0[slot]) != *value {
                    return Err(AdmissionError::UnsupportedSchemaShape);
                }
            }
        }
        Ok(objects)
    }

    fn relation(
        &mut self,
        expected: &schema28::ExpectedRelation,
    ) -> Result<Relation, AdmissionError> {
        use Field::{Identifier as Id, Integer as Int, Sql};
        let name = expected.table[1]
            .as_str()
            .map_err(|_| AdmissionError::UnsupportedSchemaShape)?;
        let mut table = self.records("SELECT schema,name,type,ncol,wr,strict FROM pragma_table_list WHERE schema='main' AND name=?1",name,[Id,Id,Id,Int,Int,Int],1)?;
        exact(&table, std::slice::from_ref(&expected.table))?;
        let table = table.pop().ok_or(AdmissionError::UnsupportedSchemaShape)?;
        let columns = self.records("SELECT cid,name,type,\"notnull\",dflt_value,pk,hidden FROM pragma_table_xinfo(?1,'main') ORDER BY cid",name,[Int,Id,Id,Int,Sql,Int,Int],256)?;
        exact(&columns, expected.columns)?;
        let foreign_keys = self.records("SELECT id,seq,\"table\",\"from\",\"to\",on_update,on_delete,\"match\" FROM pragma_foreign_key_list(?1,'main') ORDER BY id,seq",name,[Int,Int,Id,Id,Id,Id,Id,Id],expected.foreign_keys.len())?;
        exact(&foreign_keys, expected.foreign_keys)?;
        let lists = self.records("SELECT seq,name,\"unique\",origin,partial FROM pragma_index_list(?1,'main') ORDER BY name COLLATE BINARY",name,[Int,Id,Int,Id,Int],expected.indexes.len())?;
        if lists.len() != expected.indexes.len() {
            return Err(AdmissionError::UnsupportedSchemaShape);
        }
        let mut indexes = Vec::new();
        for (list, expected) in lists.into_iter().zip(expected.indexes) {
            if !list.0[1..]
                .iter()
                .zip(expected.list)
                .all(|(a, b)| ValueRef::from(a) == b)
            {
                return Err(AdmissionError::UnsupportedSchemaShape);
            }
            let index_name = expected.list[0]
                .as_str()
                .map_err(|_| AdmissionError::UnsupportedSchemaShape)?;
            let xinfo = self.records("SELECT seqno,cid,name,\"desc\",coll,\"key\" FROM pragma_index_xinfo(?1,'main') ORDER BY seqno",index_name,[Int,Int,Id,Int,Id,Int],expected.xinfo.len())?;
            exact(&xinfo, expected.xinfo)?;
            // Moving the already charged record is not a duplicate retention.
            indexes.push(Index { list, xinfo });
            self.checkpoint()?;
        }
        Ok(Relation {
            table,
            columns,
            indexes,
            foreign_keys,
        })
    }
}

fn admit<'a, 'c>(
    tx: &'a Transaction<'c>,
    gate: &PublicationGate,
) -> Result<Admitted<'a, 'c>, AdmissionError> {
    let mut inspection = Inspection {
        tx,
        gate,
        budget: Budget::default(),
    };
    inspection.scope()?;
    inspection.preflight()?;
    let objects = inspection.objects()?;
    let mut relations = Vec::new();
    for expected in schema28::RELATIONS {
        relations.push(inspection.relation(expected)?);
    }
    inspection.checkpoint()?;
    Ok(Admitted {
        _transaction: tx,
        objects,
        relations,
        metadata_bytes: inspection.budget.used,
    })
}

#[cfg(test)]
mod tests;
