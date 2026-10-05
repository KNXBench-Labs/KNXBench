//! Timed baseline for open, projection and search over one synthetic large
//! project, plus import over the reference ETS file when the corpus is present.

// Every number below is a pure function of `DEVICE_COUNT`, `GROUP_ADDRESS_COUNT`,
// `BUILDING_DEPTH` and `SEED` — the same four constants always produce the same
// project, byte for byte, so a run today and a run next year measure the same
// thing. No `OriginalData/` corpus is touched or required: this is the one test
// in the workspace that would rather build its own mountain than borrow someone
// else's.
//
// Deliberately `#[ignore]`: a benchmark that runs on every `cargo test` is a
// benchmark nobody trusts, because its numbers are drowned in noise from
// whatever else was running on the machine at the time. Reproduce it with:
//
//   cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture
//
// `--release`: debug-mode XML parsing and SQLite inserts are not what a real
// user's machine will do, and this file's job is to describe that machine, not
// a slower one nobody ships. Every stage below prints one `PERF ` line; see
// `docs/PERFORMANCE.md` for the numbers actually measured this way, with the
// machine and toolchain that produced them.
//
// Coverage, in the order data actually flows through it here: the freshly
// generated project is saved once (untimed setup) so `open`
// (`knx_store::load_project`) has a real `.knxdb` to read back; `projection`
// runs `knx_projection::build_project_tree` on what came back; `search` runs a
// batch of substring queries against it. There is no production Rust "search"
// API yet — that logic lives in the desktop UI's TypeScript today (see
// `apps/knx-web/src/searchMatch.ts`) — so this file's `search_project` is a
// direct, honest stand-in over the domain model: same shape of operation
// (case-insensitive substring over names), not a claim that it is the same
// code path a user's keystroke runs today.
//
// `import` is measured separately, against the ETS4 reference project in the
// gitignored `OriginalData/` corpus, and is skipped when that corpus is
// absent. Until 2026-09-20 it ran on a synthetic `.knxproj` this file wrote
// itself with `knx-etsproj`'s exporter; ADR-0028 withdrew the exporter, and
// there is no honest way to synthesize a 5 000-device `.knxproj` without
// one. A real ETS file is a smaller but truthful input — see
// `docs/PERFORMANCE.md` on why the two numbers must not be compared.

use std::fs;
use std::time::Instant;

use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    DeviceLoadStates, Devices, Direction, DptRef, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupAddressStyle, GroupLink, GroupRange, GroupRangeId, IdAllocators, IndividualAddress,
    Installation, InstallationId, Language, Layer, Line, LineId, Override, Project, ProjectInfo,
    Resolved, ResolvedFlags, SourceRef, StringTable, Text, Topology, CURRENT_SCHEMA_VERSION,
};
use knx_etsproj::report::Severity;
use knx_store::{load_project, open_and_migrate, save_project};

/// How many devices the synthetic project carries. Each gets
/// [`COM_OBJECTS_PER_DEVICE`] communication objects.
const DEVICE_COUNT: usize = 5_000;
/// How many group addresses the synthetic project carries, packed as
/// `ThreeLevel` (32 main ranges × 8 middle ranges × up to 256 addresses each
/// — the same bit widths `GroupAddressStyle::ThreeLevel` renders).
const GROUP_ADDRESS_COUNT: usize = 20_000;
/// Levels in the building hierarchy below the root `Building` node —
/// `Building → Floor → Room → Corridor → DistributionBoard → BuildingPart`,
/// one of `knx_core::BuildingPartType`'s six variants per level.
const BUILDING_DEPTH: usize = 5;
/// Children per building-hierarchy node. `BUILDING_BRANCH.pow(BUILDING_DEPTH
/// as u32)` leaves share `DEVICE_COUNT` devices round-robin.
const BUILDING_BRANCH: usize = 3;
const COM_OBJECTS_PER_DEVICE: usize = 4;
const MAIN_RANGES: u16 = 32;
const MIDDLE_PER_MAIN: u16 = 8;
const DEVICES_PER_LINE: usize = 255;
const LINES_PER_AREA: usize = 15;
/// Seeds every pseudo-random choice this file makes — currently just which
/// device/group-address names `bench_search` samples as queries. Today's
/// date at the time this file was written, so the number that timestamps it
/// also seeds it.
const SEED: u64 = 20_260_914;

/// splitmix64 — a small, dependency-free PRNG. Good enough for picking
/// benchmark sample indices deterministically; not used for anything
/// cryptographic or even domain-shaped.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn ceil_div(a: usize, b: usize) -> usize {
    a.div_ceil(b)
}

fn source(tag: String) -> SourceRef {
    SourceRef {
        path: "synthetic".to_string(),
        ets_id: tag,
    }
}

/// Sequential, project-unique id allocation while generating — mirrors
/// `knx_core::IdAllocators`'s own job, kept separate because the generator
/// needs the running counts before a `Project` exists to hand them to.
#[derive(Default)]
struct IdGen {
    device: u32,
    area: u32,
    line: u32,
    com_object: u32,
    group_range: u32,
    group_address: u32,
    building_part: u32,
}

impl IdGen {
    fn device(&mut self) -> DeviceId {
        self.device += 1;
        DeviceId(self.device)
    }
    fn area(&mut self) -> AreaId {
        self.area += 1;
        AreaId(self.area)
    }
    fn line(&mut self) -> LineId {
        self.line += 1;
        LineId(self.line)
    }
    fn com_object(&mut self) -> ComObjectInstanceId {
        self.com_object += 1;
        ComObjectInstanceId(self.com_object)
    }
    fn group_range(&mut self) -> GroupRangeId {
        self.group_range += 1;
        GroupRangeId(self.group_range)
    }
    fn group_address(&mut self) -> GroupAddressId {
        self.group_address += 1;
        GroupAddressId(self.group_address)
    }
    fn building_part(&mut self) -> BuildingPartId {
        self.building_part += 1;
        BuildingPartId(self.building_part)
    }
}

struct GroupSpace {
    ranges: Vec<GroupRange>,
    addresses: Vec<GroupAddressEntry>,
    /// Every generated address id, in generation order — the pool
    /// communication objects link against.
    pool: Vec<GroupAddressId>,
}

/// Builds `GROUP_ADDRESS_COUNT` addresses under a two-level `ThreeLevel`
/// range tree (main → middle, exactly the depth `knx_core::group`'s own doc
/// comment says the reference project uses). Main range `m` spans raw
/// `m << 11 ..= (m << 11) | 0x7FF`; middle range `m/k` spans the 256-address
/// slice at `(m << 11) | (k << 8)` — the same bit widths
/// `GroupAddressStyle::ThreeLevel` renders, so every address in this
/// project would print the same way a real ETS `ThreeLevel` export does.
fn build_group_addresses(ids: &mut IdGen) -> GroupSpace {
    assert!(
        GROUP_ADDRESS_COUNT <= (MAIN_RANGES as usize) * (MIDDLE_PER_MAIN as usize) * 256,
        "GROUP_ADDRESS_COUNT exceeds ThreeLevel's address space"
    );

    let mut ranges = Vec::new();
    let mut middle_ids = Vec::with_capacity((MAIN_RANGES * MIDDLE_PER_MAIN) as usize);
    let mut middle_starts = Vec::with_capacity((MAIN_RANGES * MIDDLE_PER_MAIN) as usize);

    for main_idx in 0..MAIN_RANGES {
        let main_id = ids.group_range();
        let main_start_raw = main_idx << 11;
        let mut children = Vec::with_capacity(MIDDLE_PER_MAIN as usize);
        for middle_local in 0..MIDDLE_PER_MAIN {
            let middle_id = ids.group_range();
            let middle_start_raw = main_start_raw | (middle_local << 8);
            ranges.push(GroupRange {
                id: middle_id,
                source: source(format!("GR-{middle_id}")),
                name: format!("Middle {main_idx}/{middle_local}"),
                start: GroupAddress::from_raw(middle_start_raw),
                end: GroupAddress::from_raw(middle_start_raw | 0x00FF),
                parent: Some(main_id),
                children: Vec::new(),
            });
            children.push(middle_id);
            middle_ids.push(middle_id);
            middle_starts.push(middle_start_raw);
        }
        ranges.push(GroupRange {
            id: main_id,
            source: source(format!("GR-{main_id}")),
            name: format!("Main {main_idx}"),
            start: GroupAddress::from_raw(main_start_raw),
            end: GroupAddress::from_raw(main_start_raw | 0x07FF),
            parent: None,
            children,
        });
    }

    let mut addresses = Vec::with_capacity(GROUP_ADDRESS_COUNT);
    let mut pool = Vec::with_capacity(GROUP_ADDRESS_COUNT);
    for ga_index in 0..GROUP_ADDRESS_COUNT {
        let middle_pos = ga_index % middle_ids.len();
        let sub_idx = (ga_index / middle_ids.len()) as u16;
        let middle_id = middle_ids[middle_pos];
        let raw = middle_starts[middle_pos] | sub_idx;
        let id = ids.group_address();
        addresses.push(GroupAddressEntry {
            id,
            source: source(format!("GA-{id}")),
            name: format!("GA {ga_index:05}"),
            address: GroupAddress::from_raw(raw),
            central: false,
            unfiltered: false,
            range: Some(middle_id),
            declared_dpt: Default::default(),
        });
        pool.push(id);
    }

    GroupSpace {
        ranges,
        addresses,
        pool,
    }
}

/// Builds `DEVICE_COUNT` devices, each with [`COM_OBJECTS_PER_DEVICE`]
/// communication objects linked round-robin over `ga_pool`, wired into a
/// topology sized to hold them (255 devices/line, 15 lines/area). Returns
/// the topology plus every device id in creation order (the order buildings
/// distribute them into leaves).
fn build_devices_and_topology(
    ga_pool: &[GroupAddressId],
    ids: &mut IdGen,
    devices: &mut Devices,
) -> (Topology, Vec<DeviceId>) {
    let lines_needed = ceil_div(DEVICE_COUNT, DEVICES_PER_LINE).max(1);
    let areas_needed = ceil_div(lines_needed, LINES_PER_AREA).max(1);

    let mut lines: Vec<Line> = Vec::with_capacity(lines_needed);
    let mut areas: Vec<Area> = Vec::with_capacity(areas_needed);
    let mut line_idx_global = 0usize;
    for area_idx in 0..areas_needed {
        let area_id = ids.area();
        let mut area_lines = Vec::new();
        while line_idx_global < lines_needed && area_lines.len() < LINES_PER_AREA {
            let line_id = ids.line();
            let line_address = (area_lines.len() + 1) as u8;
            lines.push(Line {
                id: line_id,
                source: source(format!("L-{line_id}")),
                name: format!("Line {area_idx}.{line_address}"),
                address: line_address,
                medium_ref: "MT-0".to_string(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: Vec::new(),
            });
            area_lines.push(line_id);
            line_idx_global += 1;
        }
        areas.push(Area {
            id: area_id,
            source: source(format!("A-{area_id}")),
            name: format!("Area {area_idx}"),
            address: (area_idx + 1) as u8,
            completion: CompletionStatus::FinishedDesign,
            lines: area_lines,
        });
    }

    let mut order = Vec::with_capacity(DEVICE_COUNT);
    let mut global_com_index = 0usize;
    for device_idx in 0..DEVICE_COUNT {
        let line_idx = device_idx / DEVICES_PER_LINE;
        let device_num = (device_idx % DEVICES_PER_LINE + 1) as u8;
        let area_idx = line_idx / LINES_PER_AREA;
        let area_address = (area_idx + 1) as u8;
        let line_address = (line_idx % LINES_PER_AREA + 1) as u8;
        let address = IndividualAddress::new(area_address, line_address, device_num)
            .expect("area/line stay within IndividualAddress's 4-bit fields by construction");

        let device_id = ids.device();
        let mut com_ids = Vec::with_capacity(COM_OBJECTS_PER_DEVICE);
        for local in 0..COM_OBJECTS_PER_DEVICE {
            let com_id = ids.com_object();
            let ga_id = ga_pool[global_com_index % ga_pool.len()];
            // ~1 in 5 receive, the rest send — the reference project's own
            // ratio is far more send-heavy still (569:27), this just keeps
            // both directions exercised.
            let direction = if global_com_index.is_multiple_of(5) {
                Direction::Receive
            } else {
                Direction::Send
            };
            global_com_index += 1;
            devices.insert_com_object(ComObjectInstance {
                id: com_id,
                // Real ETS `RefId`s of this exact shape are shared across
                // every device instance running the same application
                // program — reusing it here for every device is the
                // realistic case, not an oversight.
                source: source(format!("M-BENCH_A-1_O-{local}_R-{local}")),
                device: device_id,
                number: local as u16,
                text: Override::Value(Resolved {
                    value: Text::Literal(format!("CO {device_idx:05}-{local}")),
                    layer: Layer::Instance,
                }),
                description: Override::Absent,
                dpt: Override::Value(Resolved {
                    value: DptRef {
                        main: 1,
                        sub: Some(1),
                    },
                    layer: Layer::Instance,
                }),
                flags: ResolvedFlags {
                    read: Override::Value(Resolved {
                        value: true,
                        layer: Layer::Instance,
                    }),
                    write: Override::Absent,
                    transmit: Override::Value(Resolved {
                        value: true,
                        layer: Layer::Instance,
                    }),
                    update: Override::Absent,
                    communication: Override::Value(Resolved {
                        value: true,
                        layer: Layer::Instance,
                    }),
                    read_on_init: Override::Absent,
                },
                size: None,
                is_active: true,
                links: vec![GroupLink {
                    ga: ga_id,
                    direction,
                }],
                module_instance: None,
            });
            com_ids.push(com_id);
        }

        devices.insert(DeviceInstance {
            id: device_id,
            source: source(format!("DI-{device_id}")),
            name: format!("Device {device_idx:05}"),
            description: None,
            address: Some(address),
            product_ref: "M-BENCH_H-1_P-1".to_string(),
            program_ref: "M-BENCH_H-1_HP-1".to_string(),
            commissioning: CommissioningState {
                completion: CompletionStatus::FinishedDesign,
                individual_address_loaded: true,
                application_program_loaded: true,
                parameters_loaded: true,
                communication_part_loaded: true,
                medium_config_loaded: true,
                last_modified: None,
                last_download: None,
                broken: false,
                // No device has ever answered this synthetic project, and
                // the benchmark is not about to invent an answer: project
                // intent above, device fact left empty.
                device_reported: DeviceLoadStates::default(),
            },
            visibility_calculated: true,
            com_objects: com_ids,
            binary_data: Vec::new(),
        });
        lines[line_idx].devices.push(device_id);
        order.push(device_id);
    }

    (
        Topology {
            areas,
            lines,
            unassigned: Vec::new(),
        },
        order,
    )
}

/// The six `BuildingPartType` variants, one per level of
/// [`BUILDING_DEPTH`] — a level deeper than the list is long just repeats
/// the last (generic `BuildingPart`), which never happens at the depth this
/// file actually uses.
const BUILDING_KINDS: [BuildingPartType; 6] = [
    BuildingPartType::Building,
    BuildingPartType::Floor,
    BuildingPartType::Room,
    BuildingPartType::Corridor,
    BuildingPartType::DistributionBoard,
    BuildingPartType::BuildingPart,
];

/// Recursively builds the building hierarchy: `BUILDING_BRANCH` children
/// per node down to `BUILDING_DEPTH` levels below the root, appending every
/// node to `parts` and every leaf's id to `leaves`. Devices are not
/// assigned here — the caller distributes them across `leaves` once the
/// whole tree (and the device list) exists.
fn build_building_tree(
    ids: &mut IdGen,
    level: usize,
    path: &str,
    parent: Option<BuildingPartId>,
    parts: &mut Vec<BuildingPart>,
    leaves: &mut Vec<BuildingPartId>,
) -> BuildingPartId {
    let id = ids.building_part();
    let kind = BUILDING_KINDS[level.min(BUILDING_KINDS.len() - 1)];
    let mut children = Vec::new();
    if level < BUILDING_DEPTH {
        for branch in 0..BUILDING_BRANCH {
            let child_path = format!("{path}-{branch}");
            children.push(build_building_tree(
                ids,
                level + 1,
                &child_path,
                Some(id),
                parts,
                leaves,
            ));
        }
    } else {
        leaves.push(id);
    }
    parts.push(BuildingPart {
        id,
        source: source(format!("BP-{id}")),
        name: format!("{kind:?} {path}"),
        number: None,
        kind,
        default_line: None,
        completion: CompletionStatus::FinishedDesign,
        children,
        devices: Vec::new(),
        parent,
    });
    id
}

/// Builds the whole building hierarchy and distributes `device_order`
/// round-robin across its leaves.
fn build_buildings(ids: &mut IdGen, device_order: &[DeviceId]) -> Vec<BuildingPart> {
    let mut parts = Vec::new();
    let mut leaves = Vec::new();
    build_building_tree(ids, 0, "0", None, &mut parts, &mut leaves);

    let mut position_by_id = std::collections::HashMap::with_capacity(parts.len());
    for (position, part) in parts.iter().enumerate() {
        position_by_id.insert(part.id, position);
    }
    for (device_idx, &device_id) in device_order.iter().enumerate() {
        let leaf_id = leaves[device_idx % leaves.len()];
        let position = position_by_id[&leaf_id];
        parts[position].devices.push(device_id);
    }
    parts
}

/// Assembles the whole synthetic project: [`DEVICE_COUNT`] devices,
/// [`GROUP_ADDRESS_COUNT`] group addresses, a building hierarchy
/// [`BUILDING_DEPTH`] levels deep — deterministically, from those three
/// numbers alone.
fn build_synthetic_project() -> Project {
    let mut ids = IdGen::default();
    let mut devices = Devices::new();

    let group_space = build_group_addresses(&mut ids);
    let (topology, device_order) =
        build_devices_and_topology(&group_space.pool, &mut ids, &mut devices);
    let buildings = build_buildings(&mut ids, &device_order);

    let default_line = topology.lines.first().map(|l| l.id);
    let installation = Installation {
        id: InstallationId(0),
        name: "KNXBench Synthetic Large Installation".to_string(),
        default_line,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology,
        buildings,
        group_ranges: group_space.ranges,
        group_addresses: group_space.addresses,
        parameters: Vec::new(),
    };

    Project {
        schema_version: CURRENT_SCHEMA_VERSION,
        strings: StringTable::new(Language("en".to_string())),
        info: ProjectInfo {
            project_id: "P-BENCH1".to_string(),
            name: "KNXBench Synthetic Large Project".to_string(),
            project_number: None,
            group_address_style: GroupAddressStyle::ThreeLevel,
            completion: CompletionStatus::FinishedDesign,
            last_modified: None,
            project_start: None,
            ets_schema_version: 11,
            unlifted_group_address_dpt_declarations: 0,
        },
        installations: vec![installation],
        devices,
        ids: IdAllocators::from_counts(
            ids.device,
            ids.area,
            ids.line,
            ids.com_object,
            ids.group_range,
            ids.group_address,
            ids.building_part,
            0,
            0,
        ),
    }
}

/// A stand-in for a project-wide substring search (see this file's own
/// header comment for why this is not the production search path): every
/// device and group-address name, case-insensitive `contains`.
fn search_project(project: &Project, needle_lower: &str) -> usize {
    let device_hits = project
        .devices
        .iter()
        .filter(|d| d.name.to_lowercase().contains(needle_lower))
        .count();
    let ga_hits: usize = project
        .installations
        .iter()
        .map(|inst| {
            inst.group_addresses
                .iter()
                .filter(|g| g.name.to_lowercase().contains(needle_lower))
                .count()
        })
        .sum();
    device_hits + ga_hits
}

#[test]
#[ignore]
fn perf_baseline_large_project() {
    let project = build_synthetic_project();

    // --- import (the real production path, over a real ETS file) ---
    let store_dir = tempfile::tempdir().expect("temp dir for the benchmark's own throwaway files");
    let import_elapsed = if knx_testsupport::corpus_available() {
        let reference = knx_testsupport::reference_ets4_path();
        let source_bytes = fs::metadata(&reference)
            .expect("the corpus check above said this file exists")
            .len();
        let import_conn = open_and_migrate(&store_dir.path().join("import.sqlite"))
            .expect("migrate a fresh store for import");
        let import_started = Instant::now();
        let imported = knx_app::import_ets_project(&reference, &import_conn)
            .expect("the ETS4 reference project imports cleanly");
        let elapsed = import_started.elapsed();
        assert!(
            imported
                .report
                .errors
                .iter()
                .all(|e| e.severity != Severity::Error),
            "the ETS4 reference project imports without errors; see tests/import_service.rs"
        );
        println!("PERF import_source_bytes={source_bytes}");
        Some(elapsed)
    } else {
        eprintln!(
            "skip: OriginalData/ corpus not present (gitignored, local-only) — no import timing"
        );
        None
    };

    // --- open (knx_store::load_project against a real .knxdb) ---
    let open_conn = open_and_migrate(&store_dir.path().join("open.sqlite"))
        .expect("migrate a fresh store for the open benchmark");
    save_project(&open_conn, &project).expect("save before open is untimed setup");
    let open_started = Instant::now();
    let loaded = load_project(&open_conn).expect("load back what was just saved");
    let open_elapsed = open_started.elapsed();

    assert_eq!(loaded.devices.iter().count(), DEVICE_COUNT);
    assert_eq!(
        loaded.installations[0].group_addresses.len(),
        GROUP_ADDRESS_COUNT
    );

    // --- projection (knx_projection::build_project_tree) ---
    let projection_started = Instant::now();
    let tree = knx_projection::build_project_tree(&loaded);
    let projection_elapsed = projection_started.elapsed();
    assert_eq!(tree.installations.len(), 1);

    // --- search (this file's own stand-in, see header comment) ---
    let mut state = SEED;
    let mut queries = Vec::with_capacity(41);
    for _ in 0..20 {
        let idx = (splitmix64(&mut state) as usize) % DEVICE_COUNT;
        queries.push(format!("device {idx:05}"));
    }
    for _ in 0..20 {
        let idx = (splitmix64(&mut state) as usize) % GROUP_ADDRESS_COUNT;
        queries.push(format!("ga {idx:05}"));
    }
    queries.push("no-such-name-anywhere-in-this-project".to_string());

    let search_started = Instant::now();
    let mut total_hits = 0usize;
    for query in &queries {
        total_hits += search_project(&loaded, query);
    }
    let search_elapsed = search_started.elapsed();
    assert_eq!(
        total_hits, 40,
        "20 device queries + 20 GA queries each hit exactly one name"
    );

    let query_count = queries.len();
    println!("PERF device_count={DEVICE_COUNT} group_address_count={GROUP_ADDRESS_COUNT} building_depth={BUILDING_DEPTH} com_objects={}", DEVICE_COUNT * COM_OBJECTS_PER_DEVICE);
    if let Some(import_elapsed) = import_elapsed {
        println!(
            "PERF import_ms={:.3}",
            import_elapsed.as_secs_f64() * 1000.0
        );
    }
    println!("PERF open_ms={:.3}", open_elapsed.as_secs_f64() * 1000.0);
    println!(
        "PERF projection_ms={:.3}",
        projection_elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "PERF search_total_ms={:.3} queries={query_count} avg_us_per_query={:.1}",
        search_elapsed.as_secs_f64() * 1000.0,
        search_elapsed.as_secs_f64() * 1_000_000.0 / query_count as f64
    );
}
