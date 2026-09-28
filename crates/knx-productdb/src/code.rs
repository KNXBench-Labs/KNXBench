//! Download data of an application program, read on demand from its source blob.
//!
//! This implements ADR-0044.
//!
//! The ingest keeps each program's XML verbatim in `source_file` but models
//! none of what a download needs (KNOWN_LIMITATIONS §7): the absolute code
//! segments with their base images, the placement of the three group
//! tables, the load procedures and the program's `Options`. This module
//! reads exactly those, for one program, from `ApplicationProgram/Static`.
//!
//! Load-procedure steps this module does not model are not dropped. A step
//! with an unknown name, an unknown attribute, or any child element becomes
//! [`LoadStep::Unmodelled`], carrying its name and attributes. Anything that
//! writes to hardware must refuse a procedure that contains one.
//!
//! `AbsoluteSegment/Mask` is returned as bytes and not interpreted: no KNX
//! PDF defines it (ADR-0044).

use std::collections::BTreeMap;
use std::fmt;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{Connection, OptionalExtension};

use crate::xml::{attrs, local_name, Attrs};
use crate::ProductDbError;

/// One `Static/Code/AbsoluteSegment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteSegment {
    /// `@Id`.
    pub id: String,
    /// `@Address`, the segment's first memory address.
    pub address: u32,
    /// `@Size` in octets.
    pub size: u32,
    /// `Data`, decoded. Its length equals `size`.
    pub data: Option<Vec<u8>>,
    /// `Mask`, decoded and uninterpreted. Its length equals `size`.
    pub mask: Option<Vec<u8>>,
    /// Every other attribute (`MemoryType`, `UserMemory`, …), verbatim.
    pub other: BTreeMap<String, String>,
}

/// Where one of the group tables lives: `Static/{AddressTable,
/// AssociationTable,ComObjectTable}`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TablePlacement {
    /// `@CodeSegment`, a segment id.
    pub code_segment: Option<String>,
    /// `@Offset` into that segment.
    pub offset: Option<u32>,
    /// `@MaxEntries`.
    pub max_entries: Option<u32>,
}

/// One step of a load procedure, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStep {
    /// `LdCtrlConnect`.
    Connect,
    /// `LdCtrlDisconnect`.
    Disconnect,
    /// `LdCtrlRestart`.
    Restart,
    /// `LdCtrlCompareProp`: compare a property with inline data.
    CompareProp {
        /// `@ObjIdx`.
        object_index: u8,
        /// `@PropId`.
        property_id: u8,
        /// `@InlineData`, decoded from hex.
        data: Vec<u8>,
    },
    /// `LdCtrlUnload`.
    Unload {
        /// `@LsmIdx`.
        lsm: u8,
    },
    /// `LdCtrlLoad`.
    Load {
        /// `@LsmIdx`.
        lsm: u8,
    },
    /// `LdCtrlLoadCompleted`.
    LoadCompleted {
        /// `@LsmIdx`.
        lsm: u8,
    },
    /// `LdCtrlAbsSegment`: allocate an absolute segment.
    AbsSegment {
        /// `@LsmIdx`.
        lsm: u8,
        /// `@SegType`.
        segment_type: u8,
        /// `@Address`.
        address: u16,
        /// `@Size`.
        size: u16,
        /// `@Access`.
        access: u8,
        /// `@MemType`.
        memory_type: u8,
        /// `@SegFlags`.
        flags: u8,
    },
    /// `LdCtrlTaskSegment`.
    TaskSegment {
        /// `@LsmIdx`.
        lsm: u8,
        /// `@Address`.
        address: u16,
    },
    /// Anything else, kept by name so that it can be refused by name.
    Unmodelled {
        /// The element's local name.
        name: String,
        /// Its attributes, verbatim.
        attributes: BTreeMap<String, String>,
        /// Whether it had child elements, which are not modelled either.
        has_children: bool,
    },
}

/// One `LoadProcedure`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadProcedure {
    /// `@MergeId`, present on procedures merged into a mask procedure.
    pub merge_id: Option<String>,
    /// The steps, in document order.
    pub steps: Vec<LoadStep>,
}

impl LoadProcedure {
    /// The steps nothing can execute yet.
    pub fn unmodelled(&self) -> impl Iterator<Item = &LoadStep> {
        self.steps
            .iter()
            .filter(|step| matches!(step, LoadStep::Unmodelled { .. }))
    }
}

/// Everything a download of one program needs from its product file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProgramCode {
    /// `ApplicationProgram/@Id`.
    pub program_id: String,
    /// `@LoadProcedureStyle`.
    pub load_procedure_style: Option<String>,
    /// `@MaskVersion`.
    pub mask_version: Option<String>,
    /// The absolute segments, in document order.
    pub segments: Vec<AbsoluteSegment>,
    /// `Static/AddressTable`.
    pub address_table: Option<TablePlacement>,
    /// `Static/AssociationTable`.
    pub association_table: Option<TablePlacement>,
    /// `Static/ComObjectTable`.
    pub com_object_table: Option<TablePlacement>,
    /// The load procedures, in document order.
    pub load_procedures: Vec<LoadProcedure>,
    /// `Static/Options` attributes, verbatim.
    pub options: BTreeMap<String, String>,
}

impl ProgramCode {
    /// The segment with this id.
    pub fn segment(&self, id: &str) -> Option<&AbsoluteSegment> {
        self.segments.iter().find(|segment| segment.id == id)
    }
}

/// Why a program's code could not be read.
#[derive(Debug)]
pub enum CodeError {
    /// The database failed.
    Database(ProductDbError),
    /// The program row names a source file the database does not hold.
    MissingSource {
        /// The program.
        program_id: String,
        /// The missing blob's hash.
        sha256: String,
    },
    /// The source holds something this module will not guess about.
    Malformed {
        /// The program.
        program_id: String,
        /// What is wrong, naming the element.
        cause: String,
    },
}

impl std::error::Error for CodeError {}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeError::Database(error) => write!(f, "{error}"),
            CodeError::MissingSource { program_id, sha256 } => write!(
                f,
                "{program_id}: its source file {sha256} is not in the product database"
            ),
            CodeError::Malformed { program_id, cause } => write!(f, "{program_id}: {cause}"),
        }
    }
}

impl From<ProductDbError> for CodeError {
    fn from(error: ProductDbError) -> Self {
        CodeError::Database(error)
    }
}

impl From<rusqlite::Error> for CodeError {
    fn from(error: rusqlite::Error) -> Self {
        CodeError::Database(ProductDbError::Sqlite(error))
    }
}

/// Reads `program_id`'s code from the database's stored source file.
/// `Ok(None)` means the database does not know the program.
pub fn load_program_code(
    conn: &Connection,
    program_id: &str,
) -> Result<Option<ProgramCode>, CodeError> {
    let sha256: Option<String> = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id = ?1",
            [program_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(sha256) = sha256 else {
        return Ok(None);
    };
    let Some(bytes) = crate::load_source_file(conn, &sha256)? else {
        return Err(CodeError::MissingSource {
            program_id: program_id.to_string(),
            sha256,
        });
    };
    parse_program_code(&sha256, &bytes, program_id)
}

/// Reads `program_id`'s code from one application-program XML file.
/// `Ok(None)` means the file has no such program.
pub fn parse_program_code(
    source_path: &str,
    bytes: &[u8],
    program_id: &str,
) -> Result<Option<ProgramCode>, CodeError> {
    let mut parser = Parser {
        reader: Reader::from_reader(bytes),
        source_path,
        program_id,
    };
    parser.find_program()
}

struct Parser<'a> {
    reader: Reader<&'a [u8]>,
    source_path: &'a str,
    program_id: &'a str,
}

/// What an element event carries, owned, so the reader can move on.
struct Element {
    name: String,
    attributes: Attrs,
    empty: bool,
}

impl<'a> Parser<'a> {
    fn malformed(&self, cause: impl Into<String>) -> CodeError {
        CodeError::Malformed {
            program_id: self.program_id.to_string(),
            cause: cause.into(),
        }
    }

    /// The next event that matters: an element opening, an element closing
    /// (`None` inside `Some`), or the end of the file (`None`). Text,
    /// comments and processing instructions between elements are skipped.
    fn next(&mut self) -> Result<Option<Option<Element>>, CodeError> {
        loop {
            let event = self
                .reader
                .read_event()
                .map_err(|e| self.malformed(e.to_string()))?;
            let (start, empty) = match event {
                Event::Start(start) => (start, false),
                Event::Empty(start) => (start, true),
                Event::End(_) => return Ok(Some(None)),
                Event::Eof => return Ok(None),
                _ => continue,
            };
            let name = local_name(&start);
            let attributes = attrs(&start, self.source_path)?;
            return Ok(Some(Some(Element {
                name,
                attributes,
                empty,
            })));
        }
    }

    /// Like [`Self::next`], but the end of the file is an error: it is only
    /// called inside an element that has not closed yet.
    fn next_inside(&mut self, parent: &str) -> Result<Option<Element>, CodeError> {
        self.next()?
            .ok_or_else(|| self.malformed(format!("the file ends inside <{parent}>")))
    }

    fn skip(&mut self, element: &Element) -> Result<(), CodeError> {
        if element.empty {
            return Ok(());
        }
        // `skip_subtree` matches the raw name; every element this parser
        // skips is read by local name, so skip by depth instead.
        let mut depth = 1usize;
        while depth > 0 {
            match self.next_inside(&element.name)? {
                Some(child) if !child.empty => depth += 1,
                Some(_) => {}
                None => depth -= 1,
            }
        }
        Ok(())
    }

    fn find_program(&mut self) -> Result<Option<ProgramCode>, CodeError> {
        // Walks every element: the program is nested several levels down,
        // and a program with another id holds nothing this search reacts to.
        while let Some(event) = self.next()? {
            let Some(element) = event else { continue };
            if element.name == "ApplicationProgram"
                && element.attributes.get("Id") == Some(self.program_id)
            {
                return self.read_program(&element).map(Some);
            }
        }
        Ok(None)
    }

    fn read_program(&mut self, program: &Element) -> Result<ProgramCode, CodeError> {
        let mut code = ProgramCode {
            program_id: self.program_id.to_string(),
            load_procedure_style: program
                .attributes
                .get("LoadProcedureStyle")
                .map(str::to_string),
            mask_version: program.attributes.get("MaskVersion").map(str::to_string),
            ..ProgramCode::default()
        };
        if program.empty {
            return Ok(code);
        }
        while let Some(child) = self.next_inside("ApplicationProgram")? {
            if child.name == "Static" && !child.empty {
                self.read_static(&mut code)?;
            } else {
                self.skip(&child)?;
            }
        }
        Ok(code)
    }

    fn read_static(&mut self, code: &mut ProgramCode) -> Result<(), CodeError> {
        while let Some(child) = self.next_inside("Static")? {
            match child.name.as_str() {
                "Code" if !child.empty => self.read_code(code)?,
                "AddressTable" => {
                    code.address_table = Some(self.placement(&child)?);
                    self.skip(&child)?;
                }
                "AssociationTable" => {
                    code.association_table = Some(self.placement(&child)?);
                    self.skip(&child)?;
                }
                "ComObjectTable" => {
                    code.com_object_table = Some(self.placement(&child)?);
                    self.skip(&child)?;
                }
                "Options" => {
                    code.options = verbatim(&child.attributes);
                    self.skip(&child)?;
                }
                "LoadProcedures" if !child.empty => self.read_procedures(code)?,
                _ => self.skip(&child)?,
            }
        }
        Ok(())
    }

    fn read_code(&mut self, code: &mut ProgramCode) -> Result<(), CodeError> {
        while let Some(child) = self.next_inside("Code")? {
            if child.name == "AbsoluteSegment" {
                let segment = self.read_segment(&child)?;
                code.segments.push(segment);
            } else {
                self.skip(&child)?;
            }
        }
        Ok(())
    }

    fn read_segment(&mut self, element: &Element) -> Result<AbsoluteSegment, CodeError> {
        let a = &element.attributes;
        let id = a
            .get("Id")
            .ok_or_else(|| self.malformed("an AbsoluteSegment has no Id"))?
            .to_string();
        let number = |name: &str| -> Result<u32, CodeError> {
            let text = a
                .get(name)
                .ok_or_else(|| self.malformed(format!("AbsoluteSegment {id} has no {name}")))?;
            text.parse()
                .map_err(|_| self.malformed(format!("AbsoluteSegment {id} has {name}={text:?}")))
        };
        let address = number("Address")?;
        let size = number("Size")?;
        let other = verbatim(a)
            .into_iter()
            .filter(|(name, _)| !matches!(name.as_str(), "Id" | "Address" | "Size"))
            .collect();
        let mut segment = AbsoluteSegment {
            id,
            address,
            size,
            data: None,
            mask: None,
            other,
        };
        if element.empty {
            return Ok(segment);
        }
        while let Some(child) = self.next_inside("AbsoluteSegment")? {
            let slot = match child.name.as_str() {
                "Data" => &mut segment.data,
                "Mask" => &mut segment.mask,
                _ => {
                    self.skip(&child)?;
                    continue;
                }
            };
            let text = if child.empty {
                String::new()
            } else {
                self.text_of(&child.name)?
            };
            let compact: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
            let decoded = BASE64.decode(compact.as_bytes()).map_err(|e| {
                self.malformed(format!(
                    "AbsoluteSegment {}: {} is not base64: {e}",
                    segment.id, child.name
                ))
            })?;
            if decoded.len() as u64 != u64::from(segment.size) {
                return Err(self.malformed(format!(
                    "AbsoluteSegment {}: {} decodes to {} octets, Size is {}",
                    segment.id,
                    child.name,
                    decoded.len(),
                    segment.size
                )));
            }
            *slot = Some(decoded);
        }
        Ok(segment)
    }

    /// The text content of a leaf element, up to its end tag. A child
    /// element or an entity reference is refused: base64 needs neither.
    fn text_of(&mut self, name: &str) -> Result<String, CodeError> {
        let mut text = String::new();
        loop {
            match self
                .reader
                .read_event()
                .map_err(|e| self.malformed(e.to_string()))?
            {
                Event::Text(chunk) => text.push_str(&chunk.xml10_content()),
                Event::CData(chunk) => text.push_str(&chunk.xml10_content()),
                Event::End(_) => return Ok(text),
                Event::Comment(_) | Event::PI(_) => {}
                Event::Eof => return Err(self.malformed(format!("the file ends inside <{name}>"))),
                other => {
                    return Err(
                        self.malformed(format!("<{name}> holds {other:?}, not only base64 text"))
                    )
                }
            }
        }
    }

    fn placement(&self, element: &Element) -> Result<TablePlacement, CodeError> {
        let a = &element.attributes;
        let number = |name: &str| -> Result<Option<u32>, CodeError> {
            a.get(name)
                .map(|text| {
                    text.parse().map_err(|_| {
                        self.malformed(format!("{} has {name}={text:?}", element.name))
                    })
                })
                .transpose()
        };
        Ok(TablePlacement {
            code_segment: a.get("CodeSegment").map(str::to_string),
            offset: number("Offset")?,
            max_entries: number("MaxEntries")?,
        })
    }

    fn read_procedures(&mut self, code: &mut ProgramCode) -> Result<(), CodeError> {
        while let Some(child) = self.next_inside("LoadProcedures")? {
            if child.name != "LoadProcedure" {
                self.skip(&child)?;
                continue;
            }
            let mut procedure = LoadProcedure {
                merge_id: child.attributes.get("MergeId").map(str::to_string),
                steps: Vec::new(),
            };
            if !child.empty {
                while let Some(step) = self.next_inside("LoadProcedure")? {
                    let has_children = !step.empty;
                    if has_children {
                        self.skip(&step)?;
                    }
                    procedure.steps.push(self.step(&step, has_children)?);
                }
            }
            code.load_procedures.push(procedure);
        }
        Ok(())
    }

    fn step(&self, element: &Element, has_children: bool) -> Result<LoadStep, CodeError> {
        let a = &element.attributes;
        let name = element.name.as_str();
        let known: &[&str] = match name {
            "LdCtrlConnect" | "LdCtrlDisconnect" | "LdCtrlRestart" => &[],
            "LdCtrlCompareProp" => &["ObjIdx", "PropId", "InlineData"],
            "LdCtrlUnload" | "LdCtrlLoad" | "LdCtrlLoadCompleted" => &["LsmIdx"],
            "LdCtrlAbsSegment" => &[
                "LsmIdx", "SegType", "Address", "Size", "Access", "MemType", "SegFlags",
            ],
            "LdCtrlTaskSegment" => &["LsmIdx", "Address"],
            _ => return Ok(unmodelled(element, has_children)),
        };
        let exact = a.names().all(|attribute| known.contains(&attribute))
            && known.iter().all(|attribute| a.get(attribute).is_some());
        if has_children || !exact {
            return Ok(unmodelled(element, has_children));
        }
        let octet = |attribute: &str| -> Result<u8, CodeError> { self.field(name, a, attribute) };
        let word = |attribute: &str| -> Result<u16, CodeError> { self.field(name, a, attribute) };
        Ok(match name {
            "LdCtrlConnect" => LoadStep::Connect,
            "LdCtrlDisconnect" => LoadStep::Disconnect,
            "LdCtrlRestart" => LoadStep::Restart,
            "LdCtrlCompareProp" => LoadStep::CompareProp {
                object_index: octet("ObjIdx")?,
                property_id: octet("PropId")?,
                data: self.hex(name, a.get("InlineData").unwrap_or_default())?,
            },
            "LdCtrlUnload" => LoadStep::Unload {
                lsm: octet("LsmIdx")?,
            },
            "LdCtrlLoad" => LoadStep::Load {
                lsm: octet("LsmIdx")?,
            },
            "LdCtrlLoadCompleted" => LoadStep::LoadCompleted {
                lsm: octet("LsmIdx")?,
            },
            "LdCtrlAbsSegment" => LoadStep::AbsSegment {
                lsm: octet("LsmIdx")?,
                segment_type: octet("SegType")?,
                address: word("Address")?,
                size: word("Size")?,
                access: octet("Access")?,
                memory_type: octet("MemType")?,
                flags: octet("SegFlags")?,
            },
            _ => LoadStep::TaskSegment {
                lsm: octet("LsmIdx")?,
                address: word("Address")?,
            },
        })
    }

    fn field<T: std::str::FromStr>(
        &self,
        step: &str,
        a: &Attrs,
        attribute: &str,
    ) -> Result<T, CodeError> {
        let text = a.get(attribute).unwrap_or_default();
        text.parse()
            .map_err(|_| self.malformed(format!("<{step}> has {attribute}={text:?}")))
    }

    fn hex(&self, step: &str, text: &str) -> Result<Vec<u8>, CodeError> {
        let bad = || self.malformed(format!("<{step}> has InlineData={text:?}, not hex octets"));
        // An odd length leaves a one-character tail, which `get(i..i + 2)`
        // refuses, so the pair check covers it.
        (0..text.len())
            .step_by(2)
            .map(|i| {
                text.get(i..i + 2)
                    .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                    .ok_or_else(bad)
            })
            .collect()
    }
}

fn verbatim(attributes: &Attrs) -> BTreeMap<String, String> {
    attributes
        .names()
        .filter_map(|name| {
            attributes
                .evidence_value(name)
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect()
}

fn unmodelled(element: &Element, has_children: bool) -> LoadStep {
    LoadStep::Unmodelled {
        name: element.name.clone(),
        attributes: verbatim(&element.attributes),
        has_children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "M-0001_A-0001-01-0000";

    fn program(static_body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <ApplicationPrograms>
        <ApplicationProgram Id="{ID}" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure">
          <Static>{static_body}</Static>
          <Dynamic><Channel Id="CH"><LoadProcedures><LdCtrlConnect /></LoadProcedures></Channel></Dynamic>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#
        )
    }

    fn parse(static_body: &str) -> Result<Option<ProgramCode>, CodeError> {
        parse_program_code("t.xml", program(static_body).as_bytes(), ID)
    }

    fn parsed(static_body: &str) -> ProgramCode {
        parse(static_body)
            .expect("valid")
            .expect("the program exists")
    }

    fn malformed(static_body: &str) -> String {
        match parse(static_body) {
            Err(CodeError::Malformed { cause, .. }) => cause,
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn segments_decode_data_and_mask_and_keep_other_attributes() {
        let code = parsed(
            r#"<Code>
                <AbsoluteSegment Id="S1" Address="16384" Size="3" MemoryType="EEPROM">
                  <Data>AQID</Data>
                  <Mask>/wAA</Mask>
                </AbsoluteSegment>
                <AbsoluteSegment Id="S2" Address="1792" Size="152" />
              </Code>"#,
        );
        assert_eq!(code.program_id, ID);
        assert_eq!(code.mask_version.as_deref(), Some("MV-0701"));
        assert_eq!(
            code.load_procedure_style.as_deref(),
            Some("ProductProcedure")
        );
        assert_eq!(code.segments.len(), 2);
        let s1 = code.segment("S1").expect("S1");
        assert_eq!((s1.address, s1.size), (16384, 3));
        assert_eq!(s1.data.as_deref(), Some(&[1u8, 2, 3][..]));
        assert_eq!(s1.mask.as_deref(), Some(&[0xFFu8, 0, 0][..]));
        assert_eq!(
            s1.other.get("MemoryType").map(String::as_str),
            Some("EEPROM")
        );
        let s2 = code.segment("S2").expect("S2");
        assert_eq!((s2.address, s2.size, s2.data.is_none()), (1792, 152, true));
    }

    #[test]
    fn base64_may_be_wrapped_across_lines() {
        let code = parsed(
            "<Code><AbsoluteSegment Id=\"S\" Address=\"0\" Size=\"3\"><Data>\n  AQ\n  ID\n</Data></AbsoluteSegment></Code>",
        );
        assert_eq!(code.segments[0].data.as_deref(), Some(&[1u8, 2, 3][..]));
    }

    #[test]
    fn data_of_the_wrong_length_is_refused_not_padded() {
        let cause = malformed(
            r#"<Code><AbsoluteSegment Id="S" Address="0" Size="4"><Data>AQID</Data></AbsoluteSegment></Code>"#,
        );
        assert!(
            cause.contains("S") && cause.contains('3') && cause.contains('4'),
            "{cause}"
        );
    }

    #[test]
    fn a_mask_of_the_wrong_length_is_refused() {
        let cause = malformed(
            r#"<Code><AbsoluteSegment Id="S" Address="0" Size="3"><Mask>/w==</Mask></AbsoluteSegment></Code>"#,
        );
        assert!(cause.contains("Mask"), "{cause}");
    }

    #[test]
    fn bad_base64_is_refused() {
        let cause = malformed(
            r#"<Code><AbsoluteSegment Id="S" Address="0" Size="3"><Data>!!!!</Data></AbsoluteSegment></Code>"#,
        );
        assert!(cause.contains("Data"), "{cause}");
    }

    #[test]
    fn a_segment_without_size_is_refused() {
        let cause = malformed(r#"<Code><AbsoluteSegment Id="S" Address="0" /></Code>"#);
        assert!(cause.contains("Size"), "{cause}");
    }

    #[test]
    fn table_placements_and_options_are_read() {
        let code = parsed(
            r#"<Options LegacyAllowPartialDownloadIfAp2Mismatch="true" />
               <AddressTable CodeSegment="A" Offset="0" MaxEntries="255" />
               <AssociationTable CodeSegment="B" Offset="1" MaxEntries="254" />
               <ComObjectTable CodeSegment="C" Offset="2" />"#,
        );
        assert_eq!(
            code.address_table,
            Some(TablePlacement {
                code_segment: Some("A".into()),
                offset: Some(0),
                max_entries: Some(255)
            })
        );
        assert_eq!(
            code.association_table.as_ref().and_then(|t| t.max_entries),
            Some(254)
        );
        assert_eq!(
            code.com_object_table,
            Some(TablePlacement {
                code_segment: Some("C".into()),
                offset: Some(2),
                max_entries: None
            })
        );
        assert_eq!(
            code.options
                .get("LegacyAllowPartialDownloadIfAp2Mismatch")
                .map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn a_missing_table_stays_missing() {
        let code = parsed("");
        assert_eq!(code.address_table, None);
        assert_eq!(code.com_object_table, None);
        assert!(code.load_procedures.is_empty());
    }

    #[test]
    fn modelled_steps_are_typed_in_document_order() {
        let code = parsed(
            r#"<LoadProcedures><LoadProcedure>
                 <LdCtrlConnect />
                 <LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="0000000001270000" />
                 <LdCtrlUnload LsmIdx="1" />
                 <LdCtrlLoad LsmIdx="1" />
                 <LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="513" Access="255" MemType="3" SegFlags="128" />
                 <LdCtrlTaskSegment LsmIdx="1" Address="16384" />
                 <LdCtrlLoadCompleted LsmIdx="1" />
                 <LdCtrlRestart />
                 <LdCtrlDisconnect />
               </LoadProcedure></LoadProcedures>"#,
        );
        assert_eq!(code.load_procedures.len(), 1);
        assert_eq!(
            code.load_procedures[0].steps,
            vec![
                LoadStep::Connect,
                LoadStep::CompareProp {
                    object_index: 0,
                    property_id: 78,
                    data: vec![0, 0, 0, 0, 1, 0x27, 0, 0]
                },
                LoadStep::Unload { lsm: 1 },
                LoadStep::Load { lsm: 1 },
                LoadStep::AbsSegment {
                    lsm: 1,
                    segment_type: 0,
                    address: 16384,
                    size: 513,
                    access: 255,
                    memory_type: 3,
                    flags: 128
                },
                LoadStep::TaskSegment {
                    lsm: 1,
                    address: 16384
                },
                LoadStep::LoadCompleted { lsm: 1 },
                LoadStep::Restart,
                LoadStep::Disconnect,
            ]
        );
        assert_eq!(code.load_procedures[0].unmodelled().count(), 0);
    }

    #[test]
    fn an_unknown_step_is_kept_by_name_with_its_attributes() {
        let code = parsed(
            r#"<LoadProcedures><LoadProcedure MergeId="2">
                 <LdCtrlWriteProp ObjIdx="3" PropId="13" Verify="true" />
               </LoadProcedure></LoadProcedures>"#,
        );
        let procedure = &code.load_procedures[0];
        assert_eq!(procedure.merge_id.as_deref(), Some("2"));
        let LoadStep::Unmodelled {
            name,
            attributes,
            has_children,
        } = &procedure.steps[0]
        else {
            panic!("{:?}", procedure.steps[0]);
        };
        assert_eq!(name, "LdCtrlWriteProp");
        assert_eq!(attributes.get("PropId").map(String::as_str), Some("13"));
        assert!(!has_children);
    }

    #[test]
    fn a_known_step_with_an_unknown_attribute_is_unmodelled() {
        let code = parsed(
            r#"<LoadProcedures><LoadProcedure>
                 <LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00" ObjType="0" />
               </LoadProcedure></LoadProcedures>"#,
        );
        assert!(matches!(
            &code.load_procedures[0].steps[0],
            LoadStep::Unmodelled { name, attributes, .. }
                if name == "LdCtrlCompareProp" && attributes.contains_key("ObjType")
        ));
    }

    #[test]
    fn a_known_step_with_children_is_unmodelled_and_the_next_step_still_reads() {
        let code = parsed(
            r#"<LoadProcedures><LoadProcedure>
                 <LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00"><OnError Cause="x" /></LdCtrlCompareProp>
                 <LdCtrlRestart />
               </LoadProcedure></LoadProcedures>"#,
        );
        let steps = &code.load_procedures[0].steps;
        assert!(matches!(
            &steps[0],
            LoadStep::Unmodelled { name, has_children: true, .. } if name == "LdCtrlCompareProp"
        ));
        assert_eq!(steps[1], LoadStep::Restart);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn a_choose_inside_a_procedure_is_unmodelled() {
        let code = parsed(
            r#"<LoadProcedures><LoadProcedure>
                 <choose ParamRefId="P"><when test="1"><LdCtrlRestart /></when></choose>
                 <LdCtrlDisconnect />
               </LoadProcedure></LoadProcedures>"#,
        );
        let steps = &code.load_procedures[0].steps;
        assert!(
            matches!(&steps[0], LoadStep::Unmodelled { name, has_children: true, .. } if name == "choose")
        );
        assert_eq!(steps[1], LoadStep::Disconnect);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn a_step_value_out_of_range_is_refused() {
        let cause = malformed(
            r#"<LoadProcedures><LoadProcedure><LdCtrlLoad LsmIdx="256" /></LoadProcedure></LoadProcedures>"#,
        );
        assert!(
            cause.contains("LdCtrlLoad") && cause.contains("LsmIdx"),
            "{cause}"
        );
    }

    #[test]
    fn odd_inline_hex_is_refused() {
        let cause = malformed(
            r#"<LoadProcedures><LoadProcedure><LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="012" /></LoadProcedure></LoadProcedures>"#,
        );
        assert!(cause.contains("InlineData"), "{cause}");
    }

    #[test]
    fn another_program_in_the_same_file_is_not_read() {
        let xml = program(r#"<Code><AbsoluteSegment Id="S" Address="0" Size="1" /></Code>"#);
        assert_eq!(
            parse_program_code("t.xml", xml.as_bytes(), "M-0001_A-9999").expect("valid"),
            None
        );
    }

    #[test]
    fn an_earlier_program_with_another_id_is_skipped_whole() {
        let earlier = r#"<ApplicationProgram Id="M-0001_A-OTHER"><Static><Code><AbsoluteSegment Id="X" Address="9" Size="1" /></Code></Static></ApplicationProgram>"#;
        let xml = program(r#"<Code><AbsoluteSegment Id="S" Address="0" Size="1" /></Code>"#)
            .replace(
                "<ApplicationPrograms>",
                &format!("<ApplicationPrograms>{earlier}"),
            );
        let code = parse_program_code("t.xml", xml.as_bytes(), ID)
            .expect("valid")
            .expect("found");
        let ids: Vec<&str> = code.segments.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["S"]);
    }

    #[test]
    fn code_outside_static_is_not_read() {
        let xml = program(r#"<Code><AbsoluteSegment Id="S" Address="0" Size="1" /></Code>"#).replace(
            "<Dynamic>",
            r#"<Elsewhere><Code><AbsoluteSegment Id="X" Address="9" Size="1" /></Code></Elsewhere><Dynamic>"#,
        );
        let code = parse_program_code("t.xml", xml.as_bytes(), ID)
            .expect("valid")
            .expect("found");
        assert_eq!(code.segments.len(), 1);
    }

    #[test]
    fn a_module_defs_static_is_not_mistaken_for_the_programs() {
        let code = parsed(
            r#"<Code><AbsoluteSegment Id="S" Address="0" Size="1" /></Code>
               <ModuleDefs><ModuleDef Id="MD"><Static><Code><AbsoluteSegment Id="X" Address="9" Size="1" /></Code></Static></ModuleDef></ModuleDefs>"#,
        );
        assert_eq!(code.segments.len(), 1);
        assert_eq!(code.segments[0].id, "S");
    }

    #[test]
    fn truncated_xml_is_an_error() {
        let xml = program("<Code>");
        let cut = &xml.as_bytes()[..xml.find("<Code>").expect("present") + 6];
        assert!(matches!(
            parse_program_code("t.xml", cut, ID),
            Err(CodeError::Malformed { .. })
        ));
    }
}
