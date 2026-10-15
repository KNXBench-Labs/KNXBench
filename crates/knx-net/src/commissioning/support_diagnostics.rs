//! Fixed read-only diagnostic recipes, never a download or arbitrary memory reader.
use super::{ManagementSession, SessionTiming};
use crate::management::ManagementTransport;
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::properties::ObjectIndex;
use knx_core::IndividualAddress;

/// A built-in query; no client-provided addresses, bytes or script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Property {
        field: &'static str,
        object: u8,
        property: u8,
        length: u8,
    },
    ProgrammingMode,
}
impl Query {
    pub fn description(self) -> String {
        match self {
        Self::Property{field,object,property,length}=>format!("{field}: PropertyRead object {object}, property {property}, index 1, elements 1, expected {length} octets"),
        Self::ProgrammingMode=>"programmingMode: MemoryRead 0x0060, 1 octet; export bit 0 only".into(),
    }
    }
}
/// Resources v01.10.01 §4.3.28 (PID 78 decimal, 6 bytes), §4.23.2 property LSM;
/// retained MV-0025 master maps machines 1..3 to objects 1..3/PID5.
/// Programming mode address60h/bit0 reuses Core's cited resource constant.
/// Unknown families get descriptor type0 only. These recipes do not certify a board.
pub fn profile(mask: u16) -> Vec<Query> {
    match mask {
        0x0025 => vec![
            Query::Property {
                field: "manufacturer",
                object: 0,
                property: 12,
                length: 2,
            },
            Query::Property {
                field: "hardwareType",
                object: 0,
                property: 78,
                length: 6,
            },
            Query::Property {
                field: "addressTableState",
                object: 1,
                property: 5,
                length: 1,
            },
            Query::Property {
                field: "associationTableState",
                object: 2,
                property: 5,
                length: 1,
            },
            Query::Property {
                field: "applicationState",
                object: 3,
                property: 5,
                length: 1,
            },
        ],
        0x0012 => vec![Query::ProgrammingMode],
        _ => vec![],
    }
}
pub fn profile_id(mask: u16) -> &'static str {
    match mask {
        0x0025 => "system2-status-v1",
        0x0012 => "programming-mode-v1",
        _ => "descriptor-only-v1",
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub field: &'static str,
    pub value: Option<String>,
    pub status: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    pub mask: u16,
    pub profile: &'static str,
    pub facts: Vec<Observation>,
}

/// Only reads. Free access, no key/VerifyMode/authorisation request. At most six
/// logical reads plus disconnect; server wraps this in an overall deadline.
pub async fn collect<T: ManagementTransport>(
    transport: &T,
    target: IndividualAddress,
    timing: SessionTiming,
    expected_mask: Option<u16>,
) -> Result<Diagnostics, &'static str> {
    let mut s = ManagementSession::read_only(transport, target, AuthorisationPlan::Skip, timing)
        .map_err(|_| "target-refused")?;
    let result = async {
        s.connect().await.map_err(|_| "connection-failed")?;
        let mask = s
            .read_mask_version()
            .await
            .map_err(|_| "descriptor-unavailable")?
            .0;
        if expected_mask.is_some_and(|expected| mask != expected) {
            return Err("identity-mismatch");
        }
        let mut facts = Vec::new();
        for q in expected_mask.map(profile).unwrap_or_default() {
            let (field, result) = match q {
                Query::Property {
                    field,
                    object,
                    property,
                    length,
                } => {
                    let v = s
                        .read_property(ObjectIndex::new(object), property, 1, 1)
                        .await;
                    let value = match v {
                        Ok(v) if v.len() == length as usize => {
                            if property == 5 && v[0] > 5 {
                                None
                            } else {
                                Some(v.iter().map(|b| format!("{b:02X}")).collect::<String>())
                            }
                        }
                        _ => None,
                    };
                    (field, value)
                }
                Query::ProgrammingMode => {
                    use knx_core::commissioning::programming_mode::CURR_PROG_MODE_ADDRESS;
                    let v = s.read_memory(CURR_PROG_MODE_ADDRESS.into(), 1).await;
                    (
                        "programmingMode",
                        v.ok()
                            .filter(|v| v.len() == 1)
                            .map(|v| ((v[0] & 1) != 0).to_string()),
                    )
                }
            };
            facts.push(Observation {
                field,
                status: if result.is_some() {
                    "observed"
                } else {
                    "unavailable-or-refused"
                },
                value: result,
            });
        }
        Ok(Diagnostics {
            mask,
            profile: if expected_mask.is_some() {
                profile_id(mask)
            } else {
                "descriptor-only-v1"
            },
            facts,
        })
    }
    .await;
    s.disconnect().await;
    result
}
