#!/usr/bin/env python3
"""Build original, fictional offline KNXBench demos, never ETS project output.

Product XML is bounded synthetic input for KNXBench's existing catalogue
adapter. These devices have no firmware, signatures or hardware behaviour.
The native project writer uses the real typed core/store in the Rust example.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

VERSION = "1.0.0"
MANUFACTURER = "M-7FF1"  # Synthetic fixture identity, not an allocated vendor claim.
NS = "http://knx.org/xml/project/11"
DPTS = {
    "1.001": ("DPT_Switch", "Switch", 1),
    "1.008": ("DPT_UpDown", "Up/down", 1),
    "1.018": ("DPT_Occupancy", "Occupancy", 1),
    "5.001": ("DPT_Scaling", "Percentage", 8),
    "9.001": ("DPT_Value_Temp", "Temperature in degrees Celsius", 16),
    "17.001": ("DPT_SceneNumber", "Scene number (recall only)", 8),
}


def signal(name, dpt, direction):
    return {"name": name, "dpt": dpt, "direction": direction}


# One source for both catalogue definitions and native communication objects.
PRODUCTS = {
    "SW4": ("Switch actuator 4-channel", 4, [
        signal("Switch command", "1.001", "Receive"),
        signal("Switch status", "1.001", "Send"),
        signal("Scene recall command", "17.001", "Receive"),
        signal("Occupancy status", "1.018", "Receive"),
    ]),
    "DIM4": ("Dimming actuator 4-channel", 4, [
        signal("Level command", "5.001", "Receive"),
        signal("Level status", "5.001", "Send"),
        signal("Scene recall command", "17.001", "Receive"),
        signal("Occupancy status", "1.018", "Receive"),
    ]),
    "BL4": ("Blind actuator 4-channel", 4, [
        signal("Blind move command", "1.008", "Receive"),
        signal("Blind position command", "5.001", "Receive"),
        signal("Blind position status", "5.001", "Send"),
        signal("Scene recall command", "17.001", "Receive"),
    ]),
    "HEAT4": ("Heating controller 4-zone", 4, [
        signal("Actual temperature status", "9.001", "Receive"),
        signal("Temperature setpoint command", "9.001", "Receive"),
        signal("Heating valve status", "5.001", "Send"),
    ]),
    "WALL": ("Room wall controller", 1, [
        signal("Switch command", "1.001", "Send"),
        signal("Switch status", "1.001", "Receive"),
        signal("Level command", "5.001", "Send"),
        signal("Level status", "5.001", "Receive"),
        signal("Blind move command", "1.008", "Send"),
        signal("Blind position command", "5.001", "Send"),
        signal("Blind position status", "5.001", "Receive"),
        signal("Actual temperature status", "9.001", "Send"),
        signal("Temperature setpoint command", "9.001", "Send"),
        signal("Heating valve status", "5.001", "Receive"),
        signal("Scene recall command", "17.001", "Send"),
    ]),
    "PRES": ("Presence detector", 1, [
        signal("Occupancy status", "1.018", "Send"),
    ]),
    "KEY": ("Central and scene keypad", 1, [
        signal("Central off command", "1.001", "Send"),
        signal("Scene recall command", "17.001", "Send"),
    ]),
    "LC": ("TP line coupler (illustrative)", 1, []),
    "IP": ("KNX IP interface (illustrative)", 1, []),
}


def ids(key):
    number = list(PRODUCTS).index(key) + 1
    app = f"{MANUFACTURER}_A-{number:04X}-10-0001"
    hardware = f"{MANUFACTURER}_H-DEMO-{key}-1"
    return app, f"{hardware}_P-DEMO-{key}", f"{hardware}_HP-{number:04X}-10-0001"


def element(parent, tag, **attrs):
    return ET.SubElement(parent, tag, {k: str(v) for k, v in attrs.items()})


def xml_document():
    return ET.Element("KNX", {"xmlns": NS, "CreatedBy": "KNXBench fictional demo builder", "ToolVersion": VERSION})


def xml_bytes(root):
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def objects(key):
    _, channels, signals = PRODUCTS[key]
    return [dict(s, number=channel * len(signals) + i, channel=channel + 1,
                 text=f"Channel {channel + 1}: {s['name']}")
            for channel in range(channels) for i, s in enumerate(signals)]


def catalog_members():
    master = xml_document()
    data = element(master, "MasterData", Version="0")
    types = element(data, "DatapointTypes")
    mains = {}
    for dpt, (name, text, size) in DPTS.items():
        main, sub = (int(x) for x in dpt.split("."))
        if main not in mains:
            mains[main] = element(types, "DatapointType", Id=f"DPT-{main}", Number=main,
                                  Name=f"DPT_{main}", Text=f"{main}.xxx")
            mains[main] = element(mains[main], "DatapointSubtypes")
        element(mains[main], "DatapointSubtype", Id=f"DPST-{main}-{sub}", Number=sub, Name=name, Text=text)
    element(element(data, "Manufacturers"), "Manufacturer", Id=MANUFACTURER,
            Name="KNXBench Demo Devices (fictional)")
    members = {"knx_master.xml": xml_bytes(master)}
    hardware = xml_document()
    hman = element(element(hardware, "ManufacturerData"), "Manufacturer", RefId=MANUFACTURER)
    hlist = element(hman, "Hardware")
    catalog = xml_document()
    cman = element(element(catalog, "ManufacturerData"), "Manufacturer", RefId=MANUFACTURER)
    section = element(element(cman, "Catalog"), "CatalogSection", Id=f"{MANUFACTURER}_CS-1",
                      Name="Fictional offline demo devices", Number=1, DefaultLanguage="en-US")
    for key, (name, channels, _) in PRODUCTS.items():
        app, product, h2p = ids(key)
        h = element(hlist, "Hardware", Id=f"{MANUFACTURER}_H-DEMO-{key}-1", Name=f"DEMO-{key}",
                    SerialNumber=f"DEMO-{key}", VersionNumber=1, HasIndividualAddress=1, HasApplicationProgram=1)
        element(element(h, "Products"), "Product", Id=product, Text=f"{name} (fictional)",
                OrderNumber=f"DEMO-{key}", DefaultLanguage="en-US")
        hp = element(element(h, "Hardware2Programs"), "Hardware2Program", Id=h2p, MediumTypes="MT-0")
        element(hp, "ApplicationProgramRef", RefId=app)
        element(section, "CatalogItem", Id=f"{MANUFACTURER}_CI-{key}", Name=f"{name} (fictional)",
                Number=list(PRODUCTS).index(key) + 1, ProductRefId=product, Hardware2ProgramRefId=h2p,
                VisibleDescription="Offline demonstration only. No firmware, certification or bus behaviour.",
                DefaultLanguage="en-US")
        program = xml_document()
        pman = element(element(program, "ManufacturerData"), "Manufacturer", RefId=MANUFACTURER)
        p = element(element(pman, "ApplicationPrograms"), "ApplicationProgram", Id=app,
                    Name=f"{name} (fictional)", ApplicationNumber=list(PRODUCTS).index(key) + 1,
                    ApplicationVersion=16, ProgramType="ApplicationProgram", DefaultLanguage="en-US")
        static = element(p, "Static")
        pt = element(element(static, "ParameterTypes"), "ParameterType", Id=f"{app}_PT-Delay", Name="delay")
        element(pt, "TypeNumber", SizeInBit=8, Type="unsignedInt", minInclusive=0, maxInclusive=240)
        element(element(static, "Parameters"), "Parameter", Id=f"{app}_P-1", Name="StartupDelay",
                Text="Delay after bus voltage recovery (s)", ParameterType=f"{app}_PT-Delay", Access="ReadWrite", Value=2)
        element(element(static, "ParameterRefs"), "ParameterRef", Id=f"{app}_P-1_R-1", RefId=f"{app}_P-1")
        table = element(static, "ComObjectTable")
        refs = element(static, "ComObjectRefs")
        dynamic = element(p, "Dynamic")
        block = element(element(dynamic, "ChannelIndependentBlock"), "ParameterBlock",
                        Id=f"{app}_PB-1", Name="General", Text="General (fictional demo)")
        element(block, "ParameterRefRef", RefId=f"{app}_P-1_R-1")
        for channel in range(1, channels + 1):
            c = element(dynamic, "Channel", Id=f"{app}_CH-{channel}", Name=f"Channel{channel}",
                        Text=f"Channel {channel}", Number=channel)
            cb = element(c, "ParameterBlock", Id=f"{app}_CH-{channel}_PB-1", Name=f"Channel{channel}", Text=f"Channel {channel}")
            for obj in (o for o in objects(key) if o["channel"] == channel):
                oid = f"{app}_O-{obj['number']}"
                main, sub = (int(x) for x in obj["dpt"].split("."))
                size = DPTS[obj["dpt"]][2]
                size_text = "1 Bit" if size == 1 else f"{size // 8} Byte{'s' if size > 8 else ''}"
                send = obj["direction"] == "Send"
                element(table, "ComObject", Id=oid, Name=obj["name"], Text=obj["text"], Number=obj["number"],
                        FunctionText=obj["name"], ObjectSize=size_text, DatapointType=f"DPST-{main}-{sub}",
                        CommunicationFlag="Enabled", ReadFlag="Enabled" if send else "Disabled",
                        WriteFlag="Disabled" if send else "Enabled", TransmitFlag="Enabled" if send else "Disabled",
                        UpdateFlag="Disabled" if send else "Enabled", ReadOnInitFlag="Disabled")
                element(refs, "ComObjectRef", Id=f"{oid}_R-1", RefId=oid)
                element(cb, "ComObjectRefRef", RefId=f"{oid}_R-1")
        members[f"{MANUFACTURER}/{app}.xml"] = xml_bytes(program)
    members[f"{MANUFACTURER}/Hardware.xml"] = xml_bytes(hardware)
    members[f"{MANUFACTURER}/Catalog.xml"] = xml_bytes(catalog)
    return members


def write_catalog(path):
    # Exclusive creation: no demo command may overwrite a user's input.
    with zipfile.ZipFile(path, "x", zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(catalog_members().items()):
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 8, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, data)


def write_specs(output):
    from community_demo_layouts import build_spec, layouts
    output.mkdir(parents=True, exist_ok=False)
    products = [dict(key=key, app=ids(key)[0], product=ids(key)[1], program=ids(key)[2],
                     objects=objects(key)) for key in PRODUCTS]
    data = dict(version=VERSION, products=products,
                projects=[build_spec(layout, PRODUCTS, ids) for layout in layouts()])
    (output / "projects.json").write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    write_catalog(output / "fictional-demo-devices.knxprod")


def exercise_targets(project):
    for device in project["devices"]:
        if device["product"] != "SW4":
            continue
        linked = {b["number"] for b in device["links"]}
        for obj in objects("SW4"):
            if obj["name"] == "Switch command" and obj["number"] not in linked:
                return device, obj
    raise ValueError("Demo has no reserve switch channel for the guided exercise")


def project_guide(project):
    device, obj = exercise_targets(project)
    address = ".".join(str(n) for n in device["address"])
    command = next(g for g in project["groups"] if g["name"].endswith("/ Switch command"))
    return f"""# {project['name']}

Version {VERSION}. Original fictional offline exploration project. All device/program data are synthetic; none is firmware, certified, ETS-validated or suitable for a real device download.

## Start here

1. Unzip to a working folder. Keep a second, untouched copy for reset.
2. In KNXBench, open **Product catalog**, choose its import action and install `fictional-demo-devices.knxprod` from this download. This adds the fictional products; do not replace your product database. The same package serves all three demos and a byte-identical retry is deduplicated.
3. Open `{project['key']}.knxdb` using **File → Open (.knxdb)…**. In the browser version, upload/select it through the file picker; the desktop offers a native picker for the same file. Choose English in Settings if desired; project content itself is English. Acceptance here uses the real web UI, not a new native-shell verification.
4. Explore **Buildings**, **Topology**, **Group addresses** and a device's **Communication objects**, **Parameters** and **Product data** tabs.

This project has {len(project['devices'])} devices, {len(project['groups'])} group addresses and {len(project['parts'])} building-space nodes (including floors, rooms and distribution boards). Topology contains {len(project['lines'])} lines, including the main line. Apparent device counts include illustrative couplers/interface; power supplies, cable lengths, bus-current budgets and real filter tables are NOT planned here.

## Short tour

- Buildings show rooms and the distribution boards holding their actuators. Open an actuator's description for exact channel-to-room assignments.
- Topology keeps one area and an explicit main TP line. The illustrative IP interface is a project participant, NOT a configured gateway. A child-line coupler is shown at its `1.line.0` address. This is an instructional relationship, not tested routing or isolation.
- Main group 0 = switched circuits; 1 = separately dimmed circuits; 2 = blinds; 3 = heating; 4 = occupancy; 5 = scene recall; 6 = central commands. Middle groups name building zones. Names distinguish each room's Command and Status signals.
- Wall controllers originate local commands. Actuators receive commands and originate status. Presence detector objects feed the configured lighting occupancy inputs. Heating controllers receive measured temperature/setpoint and expose valve status. These are declarations and links, NOT running control logic.
- Scene addresses use DPT 17.001 for recall only (no scene learning/storage); central off uses DPT 1.001 with value 0. No scene tables or actual output states are simulated.

## Five exercises (use a working copy)

1. **Trace an existing connection.** Find `{command['address']}` — `{command['name']}`. Inspect its sender/receiver and their communication flags. Confirm DPT 1.001 on both ends. Do not connect to a bus.
2. **Edit the device description.** Find `{address}` — `{device['name']}`. In Properties append `Practice note: reviewed offline.` to Description and leave the field (Tab) to commit. Device names are not editable in the current UI; this exercise uses the supported description command.
3. **Edit a supported parameter.** On that device's Parameters tab change `Delay after bus voltage recovery (s)` from 2 to 5. Leave the field (Tab) to commit; verify the stored value is 5. This edits project intent only, never a physical device.
4. **Add a group and a reserve-channel link.** In the explorer's Group Addresses branch add `0/0/200`, Name `Practice light / Switch command`, selecting the Switching / {project['zones'][0]['name']} middle range (`0/0/0–0/0/255`). Open the practice actuator's Communication objects tab, expand its reserve channel and object {obj['number']} (`{obj['text']}`); choose the new group, choose Receive, then Link. The group initially has no declared DPT; after linking it **inherits DPT 1.001** from the object. That object is an unused reserve in the baseline. This practice group deliberately has no sender yet; it is an unfinished authoring exercise, not a working installation. Check for no DPT mismatch.
5. **Save and reopen.** Use Save as to create `{project['key']}-practice.knxdb`. Reopen it and check the description note, parameter 5, new group and receiver link. Reset by reopening an untouched extracted copy; native projects auto-save some edits once they have a store path, so retain a separate original. Do not overwrite your own projects.

## Honest boundaries

Unassigned actuator channels and unused wall-controller functions are deliberate reserves. Product-program information and the parameter panel need the shared fictional catalogue; retained native values alone are not a complete catalogue experience. All download/load flags remain false. No bus traffic, live values, firmware, memory images, certified design, Secure, access control or fire alarm planning. Separate apartment groups/lines do not establish security boundaries. Never download these fictional programmes to real hardware.

Read `SHA256SUMS` to verify the extracted files. Native schema/build provenance is in the combined download's manifest; compatibility applies only to the actually recorded tested build, not every previous alpha.
"""


def write_archive(path, members):
    members = dict(members)
    members["SHA256SUMS"] = "".join(
        f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in sorted(members.items())
    ).encode()
    with zipfile.ZipFile(path, "x", zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(members.items()):
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 8, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, data)


def build(output):
    if output.exists():
        raise FileExistsError(f"Refusing to overwrite existing output: {output}")
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="community-demo-") as tmp:
        stage = Path(tmp) / "bundle"
        write_specs(stage)
        subprocess.run(["cargo", "run", "--quiet", "-p", "knx-app", "--example", "community_demos", "--", str(stage)],
                       cwd=root, check=True)
        data = json.loads((stage / "projects.json").read_text(encoding="utf-8"))
        report = json.loads((stage / "generation-report.json").read_text(encoding="utf-8"))
        source = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        tool_paths = ["tools/community_demos.py", "tools/community_demo_layouts.py", "crates/knx-app/examples/community_demos.rs"]
        manifest = dict(version=VERSION, source_base=source, generation=report,
                        generator_sha256={p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in tool_paths},
                        projects=[dict(key=p["key"], name=p["name"], devices=len(p["devices"]),
                                       group_addresses=len(p["groups"]), building_parts=len(p["parts"]), lines=len(p["lines"]))
                                  for p in data["projects"]])
        manifest_bytes = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
        (stage / "manifest.json").write_bytes(manifest_bytes)
        source_paths = tool_paths + ["tools/verify_community_demos.js", "tools/tests/test_community_demos.py"]
        shared = {"fictional-demo-devices.knxprod": (stage / "fictional-demo-devices.knxprod").read_bytes(),
                  "manifest.json": manifest_bytes, "LICENSE": (root / "LICENSE").read_bytes(),
                  "source/projects.json": (stage / "projects.json").read_bytes(),
                  **{f"source/{path}": (root / path).read_bytes() for path in source_paths}}
        shared["SOURCES.md"] = (
            "# Sources and reproduction\n\n"
            "These original fictional demo definitions and tooling keep KNXBench's existing AGPL-3.0-or-later terms; see LICENSE. No third-party manufacturer content is included.\n\n"
            "The source/ directory includes the original descriptors and generator sources. The Rust writer depends on the KNXBench workspace, not external-format-shaped SQL.\n\n"
            "To rebuild, use the repository source_base in manifest.json, overlay the included source files, and run `python3 tools/community_demos.py build NEW_OUTPUT_DIRECTORY` with the repository's Cargo.lock/toolchain. The complete KNXBench source is in KNXBench-Labs/KNXBench; no application binary is included here.\n\n"
            "Generator digests bind this uncommitted local review candidate; source_base is its application base, not a claim that the generator was already published in that commit.\n"
        ).encode()
        combined = dict(shared)
        for project in data["projects"]:
            guide = project_guide(project).encode()
            native_name = f"{project['key']}.knxdb"
            (stage / f"{project['key']}.md").write_bytes(guide)
            members = dict(shared, **{native_name: (stage / native_name).read_bytes(), "README.md": guide})
            write_archive(stage / f"{project['key']}-{VERSION}.zip", members)
            combined[native_name] = members[native_name]
            combined[f"guides/{project['key']}.md"] = guide
        overview = ("# KNXBench fictional offline community demos\n\n"
                    f"Version {VERSION}. Local review candidate, not a published or ETS-compatible release.\n\n"
                    "Install the included fictional catalogue once, then open any native project. See guides/ for each tour and five exercises.\n\n"
                    + "\n".join(f"- {p['name']}: {len(p['devices'])} devices; {len(p['groups'])} group addresses." for p in data["projects"])
                    + "\n\nNo bus operation or real-device download. Keep untouched copies to reset. Version/source/schema and generator digests: manifest.json.\n")
        (stage / "README.md").write_text(overview, encoding="utf-8")
        combined["README.md"] = overview.encode()
        write_archive(stage / f"knxbench-community-demos-{VERSION}.zip", combined)
        sums = "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
                       for p in sorted(stage.iterdir()) if p.is_file())
        (stage / "SHA256SUMS").write_text(sums, encoding="utf-8")
        # copytree's exclusive destination creation preserves existing files/dirs,
        # including a destination that appeared while native generation was running.
        shutil.copytree(stage, output)


def write_ui_script(candidate, evidence, output):
    data = json.loads((candidate / "projects.json").read_text(encoding="utf-8"))
    config = dict(candidate=str(candidate.resolve()), evidence=str(evidence.resolve()),
                  origin="http://127.0.0.1:4826", targets=[])
    for project in data["projects"]:
        device, obj = exercise_targets(project)
        group = next(g for g in project["groups"] if g["name"].endswith("/ Switch command"))
        config["targets"].append(dict(key=project["key"], name=project["name"], devices=len(project["devices"]),
            groups=len(project["groups"]), deviceName=device["name"],
            deviceAddress=".".join(str(n) for n in device["address"]), objectNumber=obj["number"],
            objectText=obj["text"], traceAddress=group["address"], traceName=group["name"]))
    template = Path(__file__).with_name("verify_community_demos.js").read_text(encoding="utf-8")
    with output.open("x", encoding="utf-8") as handle:
        handle.write(template.replace("/* DEMO_CONFIG */ null", json.dumps(config)))
    evidence.mkdir(parents=True, exist_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    catalog = commands.add_parser("catalog", help="Write the fictional standalone product package")
    catalog.add_argument("output", type=Path)
    specs = commands.add_parser("specs", help="Write authoring specifications and shared fictional catalogue")
    specs.add_argument("output", type=Path)
    package = commands.add_parser("build", help="Build verified native files and reproducible offline ZIP packages")
    package.add_argument("output", type=Path)
    ui = commands.add_parser("ui-script", help="Bind a real-browser verifier to a candidate; use only with an isolated loopback server")
    ui.add_argument("candidate", type=Path)
    ui.add_argument("evidence", type=Path)
    ui.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.command == "catalog":
        write_catalog(args.output)
    elif args.command == "specs":
        write_specs(args.output)
    elif args.command == "build":
        build(args.output)
    elif args.command == "ui-script":
        write_ui_script(args.candidate, args.evidence, args.output)


if __name__ == "__main__":
    main()
