//! Bounded boundary parser for `Baggages.xml`.
//!
//! PDB-10 owns the typed baggage inventory.  This parser intentionally does
//! only enough to count index declarations at their schema location; it does
//! not interpret attributes or claim that an indexed payload was resolved.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::xml::local_name;
use crate::ProductDbError;

const MAX_DEPTH: usize = 128;
const MAX_DECLARATIONS: u64 = 100_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BaggageIndexIngest {
    pub declarations: u64,
}

pub(crate) fn parse_baggage_index(
    source_path: &str,
    bytes: &[u8],
) -> Result<BaggageIndexIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut parents = Vec::<String>::new();
    let mut declarations = 0u64;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| ProductDbError::Xml {
                source_path: source_path.into(),
                cause: error.to_string(),
            })?;
        let is_start = matches!(&event, Event::Start(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = local_name(&element);
                if name == "Baggage"
                    && parents == ["KNX", "ManufacturerData", "Manufacturer", "Baggages"]
                {
                    declarations =
                        declarations
                            .checked_add(1)
                            .ok_or_else(|| ProductDbError::Xml {
                                source_path: source_path.into(),
                                cause: "baggage declaration counter overflow".into(),
                            })?;
                    if declarations > MAX_DECLARATIONS {
                        return Err(ProductDbError::Xml {
                            source_path: source_path.into(),
                            cause: "baggage declaration limit exceeded".into(),
                        });
                    }
                }
                if is_start {
                    parents.push(name);
                    if parents.len() > MAX_DEPTH {
                        return Err(ProductDbError::Xml {
                            source_path: source_path.into(),
                            cause: "baggage index nesting limit exceeded".into(),
                        });
                    }
                }
            }
            Event::End(_) => {
                parents.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(BaggageIndexIngest { declarations })
}
