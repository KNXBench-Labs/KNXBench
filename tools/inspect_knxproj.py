"""Reproduce the raw numbers quoted in docs/RESEARCH.md from a .knxproj file.

Read-only inspection of the ZIP container and the project XML. Deliberately
uses only the stdlib so the research can be re-verified without xknxproject
(which is GPL-2.0-only and must stay a test-only dependency, see RESEARCH.md
section 10).

    python3 tools/inspect_knxproj.py "Unser Zuhause ets4 - 2025-12-15.knxproj"
"""

from __future__ import annotations

import collections
import sys
import xml.etree.ElementTree as ET
import zipfile

OVERRIDE_ATTRS = (
    "DatapointType",
    "Description",
    "Text",
    "ReadFlag",
    "WriteFlag",
    "TransmitFlag",
    "UpdateFlag",
    "CommunicationFlag",
)


def find_project_id(archive: zipfile.ZipFile) -> str:
    for name in archive.namelist():
        if name.startswith("P-") and name.endswith(".signature"):
            return name.removesuffix(".signature")
    raise SystemExit("no P-*.signature found - not a .knxproj?")


def schema_version(namespace: str) -> int:
    return int(namespace.rsplit("/", 1)[-1])


def inventory(root: ET.Element) -> None:
    """Element counts, parents and the attribute set actually used."""
    counts: collections.Counter[str] = collections.Counter()
    attrs: dict[str, set[str]] = collections.defaultdict(set)
    parents: dict[str, set[str]] = collections.defaultdict(set)

    def walk(element: ET.Element) -> None:
        tag = element.tag.rpartition("}")[2]
        counts[tag] += 1
        attrs[tag].update(element.attrib)
        for child in element:
            parents[child.tag.rpartition("}")[2]].add(tag)
            walk(child)

    walk(root)
    print("\n== element inventory ==")
    for tag, n in counts.most_common():
        print(f"{tag:28} n={n:<6} parents={sorted(parents[tag])}")
        print(f"{'':28} attrs={sorted(attrs[tag])}")


def statistics(root: ET.Element, ns: dict[str, str]) -> None:
    devices = root.findall(".//k:DeviceInstance", ns)
    unassigned = root.findall(".//k:UnassignedDevices/k:DeviceInstance", ns)
    group_addresses = root.findall(".//k:GroupAddress", ns)

    print("\n== statistics ==")
    print(f"devices                {len(devices)} ({len(unassigned)} unassigned)")
    print(f"group addresses        {len(group_addresses)}")
    print(f"  Central=1            {sum(1 for g in group_addresses if g.get('Central') == '1')}")
    print(f"  Unfiltered=1         {sum(1 for g in group_addresses if g.get('Unfiltered') == '1')}")
    print(f"parameter instances    {len(root.findall('.//k:ParameterInstanceRef', ns))}")
    print(f"send connectors        {len(root.findall('.//k:Send', ns))}")
    print(f"receive connectors     {len(root.findall('.//k:Receive', ns))}")

    print("\ndevice completion status")
    for status, n in collections.Counter(d.get("CompletionStatus") for d in devices).most_common():
        print(f"  {str(status):20} {n}")

    print("\nbuilding part types")
    building_parts = root.findall(".//k:BuildingPart", ns)
    for kind, n in collections.Counter(b.get("Type") for b in building_parts).most_common():
        print(f"  {str(kind):20} {n}")

    com_object_instances = root.findall(".//k:ComObjectInstanceRef", ns)
    overrides: collections.Counter[str] = collections.Counter()
    for instance in com_object_instances:
        overrides.update(a for a in OVERRIDE_ATTRS if a in instance.attrib)
    print(f"\ncom object instances   {len(com_object_instances)}")
    print("instance-level overrides (RESEARCH.md section 3.2)")
    for attribute, n in overrides.most_common():
        print(f"  {attribute:20} {n}")


def main(path: str) -> None:
    with zipfile.ZipFile(path) as archive:
        project_id = find_project_id(archive)
        entries = archive.infolist()
        print(f"project id             {project_id}")
        print(f"archive entries        {len(entries)}")
        print(f"uncompressed bytes     {sum(e.file_size for e in entries)}")
        print("\nmanufacturers referenced")
        for name in sorted(e.filename for e in entries):
            if name.startswith("M-") and name.endswith(".signature"):
                print(f"  {name.removesuffix('.signature')}")
        with archive.open(f"{project_id}/0.xml") as handle:
            root = ET.parse(handle).getroot()

    namespace = root.tag[1:].partition("}")[0]
    print(f"\nnamespace              {namespace}")
    print(f"schema version         {schema_version(namespace)}")
    print(f"created by             {root.get('CreatedBy')} / {root.get('ToolVersion')}")

    statistics(root, {"k": namespace})
    inventory(root)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    main(sys.argv[1])
