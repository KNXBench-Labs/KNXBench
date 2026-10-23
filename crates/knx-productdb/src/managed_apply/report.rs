//! Read-only report facts from the actual controlled rehearsal, not a second import.
use crate::update::{InstalledVersion, ProgramUpdate, ProgramUpdateStatus};
use crate::{Connection, ProductDbError, ProductFileInstall};

#[derive(Debug)]
pub struct PackageProgram {
    pub package_key: usize,
    pub program: ProgramUpdate,
}
#[derive(Debug, Default)]
pub struct ControlledReport {
    pub programs: Vec<PackageProgram>,
    pub products_added: u64,
    pub hardware2programs_added: u64,
}

struct Row {
    id: String,
    manufacturer: String,
    name: Option<String>,
    number: Option<String>,
    version: Option<String>,
}
pub(crate) struct Before {
    programs: Vec<Row>,
    products: u64,
    links: u64,
}
fn count(conn: &Connection, sql: &str) -> Result<u64, ProductDbError> {
    let value: i64 = conn.query_row(sql, [], |r| r.get(0))?;
    u64::try_from(value).map_err(|_| ProductDbError::Xml {
        source_path: "controlled report".into(),
        cause: "negative count".into(),
    })
}
impl Before {
    pub(crate) fn read(conn: &Connection) -> Result<Self, ProductDbError> {
        let mut query = conn.prepare("SELECT id, manufacturer_id, name, application_number, application_version FROM application_program ORDER BY id")?;
        let programs = query
            .query_map([], |r| {
                Ok(Row {
                    id: r.get(0)?,
                    manufacturer: r.get(1)?,
                    name: r.get(2)?,
                    number: r.get(3)?,
                    version: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            programs,
            products: count(conn, "SELECT count(*) FROM product")?,
            links: count(conn, "SELECT count(*) FROM hardware2program")?,
        })
    }
    pub(crate) fn finish(
        self,
        conn: &Connection,
        install: &ProductFileInstall,
    ) -> Result<ControlledReport, ProductDbError> {
        let after = Self::read(conn)?;
        let invalid = || ProductDbError::Xml {
            source_path: "controlled report".into(),
            cause: "rehearsal count regressed".into(),
        };
        let mut report = ControlledReport {
            programs: Vec::new(),
            products_added: after
                .products
                .checked_sub(self.products)
                .ok_or_else(invalid)?,
            hardware2programs_added: after.links.checked_sub(self.links).ok_or_else(invalid)?,
        };
        let mut query = conn.prepare("SELECT DISTINCT ap.id FROM application_program ap JOIN package_member pm ON pm.source_sha256=ap.source_sha256 WHERE pm.package_sha256=?1 ORDER BY ap.id")?;
        for (package_key, package) in install.packages.iter().enumerate() {
            for id in query.query_map([&package.report.sha256], |r| r.get::<_, String>(0))? {
                let id = id?;
                let row = after
                    .programs
                    .iter()
                    .find(|r| r.id == id)
                    .ok_or_else(invalid)?;
                let installed_versions: Vec<_> = self
                    .programs
                    .iter()
                    .filter(|old| {
                        old.id != row.id
                            && old.manufacturer == row.manufacturer
                            && old.number.is_some()
                            && old.number == row.number
                    })
                    .map(|old| InstalledVersion {
                        id: old.id.clone(),
                        application_version: old.version.clone(),
                    })
                    .collect();
                let status = if self.programs.iter().any(|old| old.id == row.id) {
                    ProgramUpdateStatus::AlreadyInstalled
                } else if installed_versions.is_empty() {
                    ProgramUpdateStatus::New
                } else {
                    ProgramUpdateStatus::NewVersion
                };
                report.programs.push(PackageProgram {
                    package_key,
                    program: ProgramUpdate {
                        id: row.id.clone(),
                        manufacturer_id: row.manufacturer.clone(),
                        name: row.name.clone(),
                        application_number: row.number.clone(),
                        application_version: row.version.clone(),
                        status,
                        installed_versions,
                    },
                });
            }
        }
        Ok(report)
    }
}
