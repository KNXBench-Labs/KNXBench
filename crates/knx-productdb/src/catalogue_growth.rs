//! Total-main extent ceiling on the actual controlled SQL allocator.
//!
//! Copy extent admission is separate; this setting does not bound backup,
//! journals, native temporary storage or filesystem allocation.

use rusqlite::Connection;

pub(crate) const MAIN_BYTE_CEILING: u64 = 68_719_476_736;

#[derive(Debug)]
pub(crate) enum GrowthError {
    Synchronous(i64),
    Attached,
    ApplicationTemp,
    JournalMode(String),
    LockingMode(String),
    PageFacts,
    AboveCap,
    Readback { expected: i64, observed: i64 },
    Sqlite(rusqlite::Error),
}
impl From<rusqlite::Error> for GrowthError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

fn page_cap(size: i64, count: i64, bytes: u64) -> Result<i64, GrowthError> {
    if !(512..=65_536).contains(&size) || size & (size - 1) != 0 || count <= 0 {
        return Err(GrowthError::PageFacts);
    }
    let size = u64::try_from(size).map_err(|_| GrowthError::PageFacts)?;
    let count = u64::try_from(count).map_err(|_| GrowthError::PageFacts)?;
    let extent = count.checked_mul(size).ok_or(GrowthError::PageFacts)?;
    if extent > bytes {
        return Err(GrowthError::AboveCap);
    }
    i64::try_from(bytes / size).map_err(|_| GrowthError::PageFacts)
}

pub(crate) fn require_synchronous(conn: &Connection) -> Result<(), GrowthError> {
    let value = conn.query_row("PRAGMA main.synchronous", [], |r| r.get(0))?;
    if !matches!(value, 2 | 3) {
        return Err(GrowthError::Synchronous(value));
    }
    Ok(())
}

pub(crate) struct WriterGrowth {
    cap: i64,
    prior: i64,
}
impl WriterGrowth {
    pub(crate) fn admit(conn: &Connection) -> Result<Self, GrowthError> {
        let cap = admitted_cap(conn, ceiling(1))?;
        let prior = conn.query_row("PRAGMA main.max_page_count", [], |r| r.get(0))?;
        Ok(Self { cap, prior })
    }
    pub(crate) fn prior(&self) -> i64 {
        self.prior
    }
    pub(crate) fn enforce(self, conn: &Connection) -> Result<(), GrowthError> {
        set_exact(conn, self.cap)
    }
}

/// Explicit, uncancellable configuration cleanup after transaction settlement.
/// A clamped setter is not an exact restoration, even after a witnessed commit.
pub(crate) fn restore(conn: &Connection, cap: i64) -> Result<(), GrowthError> {
    set_exact(conn, cap)
}

/// Observe, never convert, the supported main-only DELETE/normal lane.
fn admit_lane(conn: &Connection) -> Result<(), GrowthError> {
    let temporary: i64 =
        conn.query_row("SELECT count(*) FROM temp.sqlite_schema", [], |r| r.get(0))?;
    if temporary != 0 {
        return Err(GrowthError::ApplicationTemp);
    }
    let mut databases = conn.prepare("PRAGMA database_list")?;
    let mut rows = databases.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if !matches!(name.as_str(), "main" | "temp") {
            return Err(GrowthError::Attached);
        }
    }
    let journal: String = conn.query_row("PRAGMA main.journal_mode", [], |r| r.get(0))?;
    if journal != "delete" {
        return Err(GrowthError::JournalMode(journal));
    }
    let locking: String = conn.query_row("PRAGMA main.locking_mode", [], |r| r.get(0))?;
    if locking != "normal" {
        return Err(GrowthError::LockingMode(locking));
    }
    Ok(())
}

pub(crate) fn enforce(conn: &Connection) -> Result<(), GrowthError> {
    set_exact(conn, admitted_cap(conn, ceiling(0))?)
}

fn ceiling(_role: usize) -> u64 {
    #[cfg(test)]
    if let Some(bytes) = TEST_CEILINGS.with(|caps| caps.get()[_role]) {
        return bytes;
    }
    MAIN_BYTE_CEILING
}

#[cfg(test)]
thread_local! { static TEST_CEILINGS: std::cell::Cell<[Option<u64>; 2]> = const { std::cell::Cell::new([None; 2]) }; }

/// Thread-local, non-Send guard: finite fixtures cannot alter another test or
/// production construction. The two actual allocators are isolated explicitly.
#[cfg(test)]
pub(crate) struct TestCeilings([Option<u64>; 2], std::marker::PhantomData<std::rc::Rc<()>>);
#[cfg(test)]
impl TestCeilings {
    pub(crate) fn new(rehearsal: Option<u64>, writer: Option<u64>) -> Self {
        let next = [rehearsal, writer];
        assert!(next
            .iter()
            .flatten()
            .all(|&n| n > 0 && n <= MAIN_BYTE_CEILING));
        Self(
            TEST_CEILINGS.with(|caps| caps.replace(next)),
            std::marker::PhantomData,
        )
    }
}
#[cfg(test)]
impl Drop for TestCeilings {
    fn drop(&mut self) {
        TEST_CEILINGS.with(|caps| caps.set(self.0));
    }
}

fn admitted_cap(conn: &Connection, bytes: u64) -> Result<i64, GrowthError> {
    admit_lane(conn)?;
    let size = conn.query_row("PRAGMA main.page_size", [], |r| r.get(0))?;
    let count = conn.query_row("PRAGMA main.page_count", [], |r| r.get(0))?;
    page_cap(size, count, bytes)
}

fn set_exact(conn: &Connection, cap: i64) -> Result<(), GrowthError> {
    conn.execute_batch(&format!("PRAGMA main.max_page_count={cap}"))?;
    let observed = conn.query_row("PRAGMA main.max_page_count", [], |r| r.get(0))?;
    if observed != cap {
        return Err(GrowthError::Readback {
            expected: cap,
            observed,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_total_extent_policy_is_pinned_independently() {
        assert_eq!(MAIN_BYTE_CEILING, 68_719_476_736);
        for (size, expected) in [(512, 134217728), (4096, 16777216), (65536, 1048576)] {
            assert_eq!(page_cap(size, 1, MAIN_BYTE_CEILING).unwrap(), expected);
            assert_eq!(
                page_cap(size, expected, MAIN_BYTE_CEILING).unwrap(),
                expected
            );
            assert!(matches!(
                page_cap(size, expected + 1, MAIN_BYTE_CEILING),
                Err(GrowthError::AboveCap)
            ));
        }
    }
    #[test]
    fn page_facts_refuse_nonpositive_unsupported_overflow_and_existing_extent_above_cap() {
        for size in [-1, 0, 1, 511, 513, 65537, 131072] {
            assert!(matches!(
                page_cap(size, 1, MAIN_BYTE_CEILING),
                Err(GrowthError::PageFacts)
            ));
        }
        for count in [i64::MIN, -1, 0] {
            assert!(matches!(
                page_cap(4096, count, MAIN_BYTE_CEILING),
                Err(GrowthError::PageFacts)
            ));
        }
        assert!(matches!(
            page_cap(65536, i64::MAX, MAIN_BYTE_CEILING),
            Err(GrowthError::PageFacts)
        ));
        assert!(matches!(
            page_cap(4096, 3, 8192),
            Err(GrowthError::AboveCap)
        ));
        assert_eq!(page_cap(4096, 2, 12287).unwrap(), 2, "floor is intentional");
    }
    #[test]
    fn finite_override_is_thread_local_scoped_and_does_not_change_production_pin() {
        assert_eq!(ceiling(0), 68_719_476_736);
        assert_eq!(ceiling(1), 68_719_476_736);
        {
            let _caps = TestCeilings::new(Some(8192), Some(16384));
            assert_eq!(ceiling(0), 8192);
            assert_eq!(ceiling(1), 16384);
            std::thread::spawn(|| {
                assert_eq!(ceiling(0), 68_719_476_736);
                assert_eq!(ceiling(1), 68_719_476_736);
            })
            .join()
            .unwrap();
        }
        assert_eq!(ceiling(0), 68_719_476_736);
        assert_eq!(ceiling(1), 68_719_476_736);
    }
}
