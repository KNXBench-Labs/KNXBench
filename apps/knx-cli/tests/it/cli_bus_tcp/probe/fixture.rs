//! Save and independently admit synthetic MV-0701 inputs through real read-only planners.

use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use knx_productdb::byte_order_probe::{ProbePlan, SkipReason};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(super) const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0002">
<ApplicationPrograms><ApplicationProgram Id="M-0002_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="PAR" Address="17408" Size="32" /></Code>
<ParameterTypes>
<ParameterType Id="U16" Name="U16"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType>
<ParameterType Id="U8" Name="U8"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
<ParameterType Id="TXT" Name="Text"><TypeText SizeInBit="16" /></ParameterType>
</ParameterTypes><Parameters>
<Parameter Id="W" Name="Word" ParameterType="U16" Value="300"><Memory CodeSegment="PAR" Offset="0" BitOffset="0" /></Parameter>
<Parameter Id="V" Name="OtherWord" ParameterType="U16" Value="258"><Memory CodeSegment="PAR" Offset="2" BitOffset="0" /></Parameter>
<Parameter Id="B" Name="Octet" ParameterType="U8" Value="7"><Memory CodeSegment="PAR" Offset="8" BitOffset="0" /></Parameter>
<Parameter Id="Z" Name="Either" ParameterType="U16" Value="0"><Memory CodeSegment="PAR" Offset="10" BitOffset="0" /></Parameter>
<Parameter Id="T" Name="UnsupportedText" ParameterType="TXT" Value="ab"><Memory CodeSegment="PAR" Offset="16" BitOffset="0" /></Parameter>
</Parameters><ParameterRefs>
<ParameterRef Id="W_R" RefId="W" /><ParameterRef Id="V_R" RefId="V" /><ParameterRef Id="B_R" RefId="B" />
<ParameterRef Id="Z_R" RefId="Z" /><ParameterRef Id="T_R" RefId="T" />
</ParameterRefs></Static><Dynamic><Channel Id="CH" Text="Synthetic"><ParameterBlock Id="PB" Text="Block">
<ParameterRefRef RefId="W_R" /><ParameterRefRef RefId="V_R" /><ParameterRefRef RefId="B_R" />
<ParameterRefRef RefId="Z_R" /><ParameterRefRef RefId="T_R" />
</ParameterBlock></Channel></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
pub(super) const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0002">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0002_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0002_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

pub(in super::super) struct Fixture {
    pub dir: tempfile::TempDir,
    pub project: PathBuf,
    pub products: PathBuf,
    pub plan: ProbePlan,
    before: Vec<(String, Vec<u8>)>,
}

pub(in super::super) fn tree(path: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), out);
            } else {
                out.push((
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .into(),
                    std::fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    visit(path, path, &mut out);
    out.sort();
    out
}

impl Fixture {
    pub fn new() -> Self {
        Self::with_program(PROGRAM)
    }

    /// The same saved numeric inputs, with an explicit read-comparison plan.
    pub fn comparison() -> Self {
        Self::with_program(&Self::comparison_program())
    }

    /// A complete load-state sequence, not the comparison-only allocation recipe.
    pub fn download() -> Self {
        let program = Self::comparison_program()
            .replace(
                "<LdCtrlAbsSegment",
                "<LdCtrlUnload LsmIdx=\"3\"/><LdCtrlLoad LsmIdx=\"3\"/><LdCtrlAbsSegment",
            )
            .replace(
                "<LdCtrlDisconnect/>",
                "<LdCtrlLoadCompleted LsmIdx=\"3\"/><LdCtrlDisconnect/>",
            );
        assert!(program.contains("<LdCtrlUnload LsmIdx=\"3\"/><LdCtrlLoad LsmIdx=\"3\"/>"));
        assert!(program.contains("<LdCtrlLoadCompleted LsmIdx=\"3\"/>"));
        Self::with_program(&program)
    }

    fn comparison_program() -> String {
        let program = PROGRAM.replace("ApplicationVersion=\"1\"", "ApplicationVersion=\"1\" PeiType=\"1\"").replace("</Static>", r#"<LoadProcedures><LoadProcedure>
<LdCtrlConnect/><LdCtrlCompareProp ObjIdx="3" PropId="13" InlineData="0002000101" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="32" Access="255" MemType="3" SegFlags="128" />
<LdCtrlDisconnect/></LoadProcedure></LoadProcedures></Static>"#);
        let program = program.replace(
            r#"<AbsoluteSegment Id="PAR" Address="17408" Size="32" />"#,
            r#"<AbsoluteSegment Id="PAR" Address="17408" Size="32"><Data>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</Data></AbsoluteSegment>"#,
        );
        assert!(program.contains(r#"PeiType="1""#));
        assert!(program
            .contains(r#"<LdCtrlCompareProp ObjIdx="3" PropId="13" InlineData="0002000101" />"#));
        assert!(program.contains("<Data>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</Data>"));
        program
    }

    fn with_program(program: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let inputs = dir.path().join("inputs");
        std::fs::create_dir(&inputs).unwrap();
        let products = inputs.join("products.sqlite");
        let project_path = inputs.join("project.knxdb");
        let db = knx_productdb::open_and_migrate(&products).unwrap();
        knx_productdb::ingest_file(&db, "Synthetic.xml", program.as_bytes()).unwrap();
        knx_productdb::ingest_file(&db, "Hardware.xml", HARDWARE).unwrap();
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "Synthetic".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(1)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: "DEV-1".into(),
            },
            name: "Synthetic".into(),
            description: None,
            address: Some("1.1.67".parse().unwrap()),
            product_ref: "M-0002_H-1_P-1".into(),
            program_ref: "H_HP".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        let conn = knx_store::open_and_migrate(&project_path).unwrap();
        knx_store::save_project(&conn, &project).unwrap();
        drop(conn);
        drop(db);
        // Admission MUST precede any gateway: read the actually saved inputs, not hand-made fields.
        let conn = knx_store::open_existing_read_only(&project_path)
            .unwrap()
            .conn;
        let saved = knx_store::load_project(&conn).unwrap();
        let db = knx_productdb::open_read_only(&products).unwrap();
        let plan = knx_app::device_download::prepare_byte_order_probe(
            &db.conn,
            &saved,
            "1.1.67".parse().unwrap(),
        )
        .unwrap();
        assert_eq!(plan.mask.0, 0x0701);
        assert_eq!(plan.manufacturer, 2);
        assert_eq!(
            plan.fields
                .iter()
                .map(|f| (
                    f.parameter_ref.as_str(),
                    f.address,
                    f.value,
                    f.field.size_in_bit
                ))
                .collect::<Vec<_>>(),
            [
                ("W_R", 0x4400, 300, 16),
                ("V_R", 0x4402, 258, 16),
                ("B_R", 0x4408, 7, 8)
            ]
        );
        assert_eq!(plan.runs(), [(0x4400, 4), (0x4408, 1)]);
        assert_eq!(
            plan.skipped
                .iter()
                .map(|s| (s.parameter_ref.as_str(), &s.reason))
                .collect::<Vec<_>>(),
            [
                ("Z_R", &SkipReason::EitherOrder),
                ("T_R", &SkipReason::Text)
            ]
        );
        drop(db);
        drop(conn);
        let before = tree(&inputs);
        Self {
            dir,
            project: project_path,
            products,
            plan,
            before,
        }
    }

    pub fn command(&self, gateway: &str, transport: Option<&str>) -> Command {
        let mut cmd = self.bare_command();
        cmd.args(["device", "byte-order-probe", "1.1.67", "--project"])
            .arg(&self.project)
            .arg("--product-db")
            .arg(&self.products)
            .args(["--gateway", gateway]);
        if let Some(transport) = transport {
            cmd.args(["--tunnel-transport", transport]);
        }
        cmd
    }

    pub fn bare_command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_knx"));
        for (key, leaf) in [
            ("HOME", "home"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_RUNTIME_DIR", "runtime"),
        ] {
            cmd.env(key, self.dir.path().join(leaf));
        }
        cmd
    }

    pub fn intact(&self) {
        assert_eq!(
            tree(&self.dir.path().join("inputs")),
            self.before,
            "read-only inputs/sidecars changed"
        );
        for leaf in ["home", "data", "config", "runtime"] {
            let path = self.dir.path().join(leaf);
            assert!(
                !path.exists() || tree(&path).is_empty(),
                "probe created per-user history/state: {leaf}"
            );
        }
    }
}
