"""Inventory KNX product-package corpora without extracting archives.

The scanner uses only the Python standard library, reads ZIP members in memory
with explicit size limits, and emits machine-readable JSON for reproducible
corpus research. Nested ZIP/download bundles are inspected recursively.

    python3 tools/inspect_product_corpus.py \
        OriginalData/ProductDatabases/Gira \
        OriginalData/ProductDatabases/MDT \
        --output /tmp/product-corpus.json
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import io
import json
import pathlib
import re
import xml.etree.ElementTree as ET
import zipfile

MAX_INPUT_BYTES = 256 * 1024 * 1024
MAX_MEMBER_BYTES = 128 * 1024 * 1024
MAX_NESTING = 3
INTERESTING_ELEMENTS = {
    "AbsoluteSegment",
    "ApplicationProgram",
    "ApplicationProgramRef",
    "Argument",
    "Assign",
    "Baggage",
    "CatalogItem",
    "CatalogSection",
    "Channel",
    "ChannelIndependentBlock",
    "Choose",
    "CodeSegment",
    "ComObject",
    "ComObjectRef",
    "ComObjectTable",
    "DatapointSubtype",
    "DatapointType",
    "Dynamic",
    "FunctionPoint",
    "FunctionType",
    "Hardware",
    "Hardware2Program",
    "KNX",
    "Language",
    "LoadProcedure",
    "Manufacturer",
    "Memory",
    "Module",
    "ModuleDef",
    "NumericArg",
    "Options",
    "Parameter",
    "ParameterBlock",
    "ParameterRef",
    "ParameterType",
    "Product",
    "Property",
    "RelativeSegment",
    "Repeat",
    "SpaceUsage",
    "Translation",
    "TranslationElement",
    "TranslationUnit",
    "TextArg",
    "TypeFloat",
    "TypeNumber",
    "TypeRestriction",
    "TypeText",
    "choose",
    "when",
}
PROGRAM_ATTRIBUTES = (
    "Id",
    "Name",
    "ApplicationNumber",
    "ApplicationVersion",
    "ProgramType",
    "MaskVersion",
    "PeiType",
    "LoadProcedureStyle",
    "DefaultLanguage",
    "Linkable",
    "IsSecureEnabled",
    "Hash",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def split_tag(tag: str) -> tuple[str, str]:
    if tag.startswith("{"):
        namespace, _, local = tag[1:].partition("}")
        return namespace, local
    return "", tag


def xml_inventory(data: bytes, source: str) -> dict[str, object]:
    counts: collections.Counter[str] = collections.Counter()
    attributes: dict[str, set[str]] = collections.defaultdict(set)
    attribute_values: dict[str, dict[str, collections.Counter[str]]] = collections.defaultdict(
        lambda: collections.defaultdict(collections.Counter)
    )
    namespaces: set[str] = set()
    root = ""
    programs: list[dict[str, str]] = []
    manufacturers: set[str] = set()
    products: list[dict[str, str]] = []
    languages: set[str] = set()
    parent_child: collections.Counter[str] = collections.Counter()
    parameter_type_children: collections.Counter[str] = collections.Counter()
    dynamic_elements: collections.Counter[str] = collections.Counter()
    stack: list[str] = []
    try:
        for event, element in ET.iterparse(io.BytesIO(data), events=("start", "end")):
            namespace, local = split_tag(element.tag)
            if event == "end":
                stack.pop()
                element.clear()
                continue
            if not root:
                root = local
            if stack:
                parent_child[f"{stack[-1]}/{local}"] += 1
                if stack[-1] == "ParameterType":
                    parameter_type_children[local] += 1
            if "Dynamic" in stack:
                dynamic_elements[local] += 1
            stack.append(local)
            if namespace:
                namespaces.add(namespace)
            counts[local] += 1
            attributes[local].update(element.attrib)
            if local in INTERESTING_ELEMENTS:
                for name, value in element.attrib.items():
                    attribute_values[local][name][value] += 1
            if local == "ApplicationProgram":
                programs.append({name: element.attrib[name] for name in PROGRAM_ATTRIBUTES if name in element.attrib})
            elif local == "Manufacturer":
                value = element.attrib.get("Id") or element.attrib.get("RefId")
                if value:
                    manufacturers.add(value)
            elif local == "Product":
                products.append(
                    {
                        name: element.attrib[name]
                        for name in ("Id", "Text", "OrderNumber", "DefaultLanguage")
                        if name in element.attrib
                    }
                )
            elif local == "Language" and element.attrib.get("Identifier"):
                languages.add(element.attrib["Identifier"])
    except (ET.ParseError, UnicodeError, ValueError) as error:
        return {"source": source, "error": str(error), "size": len(data)}

    values: dict[str, dict[str, dict[str, int]]] = {}
    for element, attrs in attribute_values.items():
        values[element] = {
            name: dict(sorted(counter.items())) for name, counter in sorted(attrs.items())
        }
    return {
        "source": source,
        "size": len(data),
        "root": root,
        "namespaces": sorted(namespaces),
        "counts": dict(sorted(counts.items())),
        "attributes": {name: sorted(values_) for name, values_ in sorted(attributes.items())},
        "interesting_values": values,
        "programs": programs,
        "manufacturers": sorted(manufacturers),
        "products": products,
        "languages": sorted(languages),
        "parent_child": dict(sorted(parent_child.items())),
        "parameter_type_children": dict(sorted(parameter_type_children.items())),
        "dynamic_elements": dict(sorted(dynamic_elements.items())),
    }


def namespace_scheme(namespace: str) -> int | None:
    match = re.fullmatch(r"http://knx\.org/xml/project/(\d+)", namespace)
    return int(match.group(1)) if match else None


def archive_inventory(data: bytes, source: str, depth: int) -> dict[str, object]:
    result: dict[str, object] = {
        "source": source,
        "size": len(data),
        "sha256": sha256(data),
        "kind": "zip",
    }
    try:
        archive = zipfile.ZipFile(io.BytesIO(data))
    except (zipfile.BadZipFile, OSError, ValueError) as error:
        result.update(kind="invalid-zip", error=str(error))
        return result

    members = archive.infolist()
    names = [member.filename for member in members]
    result.update(
        member_count=len(members),
        duplicate_members=sorted(name for name, count in collections.Counter(names).items() if count > 1),
        encrypted_members=sorted(member.filename for member in members if member.flag_bits & 1),
        compressed_bytes=sum(member.compress_size for member in members),
        uncompressed_bytes=sum(member.file_size for member in members),
        member_extensions=dict(
            sorted(collections.Counter(pathlib.PurePosixPath(name).suffix.lower() or "<none>" for name in names).items())
        ),
    )
    xml_files: list[dict[str, object]] = []
    nested: list[dict[str, object]] = []
    oversized: list[str] = []
    read_errors: list[dict[str, str]] = []
    for member in members:
        if member.is_dir():
            continue
        if member.file_size > MAX_MEMBER_BYTES:
            oversized.append(member.filename)
            continue
        try:
            with archive.open(member) as member_file:
                payload = member_file.read(MAX_MEMBER_BYTES + 1)
        except (RuntimeError, zipfile.BadZipFile, OSError, ValueError) as error:
            read_errors.append({"member": member.filename, "error": str(error)})
            continue
        if len(payload) > MAX_MEMBER_BYTES:
            oversized.append(member.filename)
            continue
        logical_source = f"{source}!{member.filename}"
        if member.filename.lower().endswith(".xml"):
            xml_files.append(xml_inventory(payload, logical_source))
        if depth < MAX_NESTING and payload.startswith(b"PK\x03\x04"):
            nested.append(archive_inventory(payload, logical_source, depth + 1))
    result["oversized_members"] = oversized
    result["read_errors"] = read_errors
    result["xml_files"] = xml_files
    result["nested_archives"] = nested

    master = next(
        (item for item in xml_files if item["source"].rsplit("!", 1)[-1].lower() == "knx_master.xml"),
        None,
    )
    if master and not master.get("error"):
        namespaces = master.get("namespaces", [])
        result["master_namespaces"] = namespaces
        result["schemes"] = sorted(
            scheme for namespace in namespaces if (scheme := namespace_scheme(namespace)) is not None
        )
    else:
        result["master_namespaces"] = []
        result["schemes"] = []
    return result


def file_inventory(path: pathlib.Path) -> dict[str, object]:
    if path.stat().st_size > MAX_INPUT_BYTES:
        return {
            "source": str(path),
            "size": path.stat().st_size,
            "kind": "oversized-input",
        }
    data = path.read_bytes()
    if data.startswith(b"PK\x03\x04"):
        return archive_inventory(data, str(path), 0)
    return {
        "source": str(path),
        "size": len(data),
        "sha256": sha256(data),
        "kind": "binary",
        "extension": path.suffix.lower(),
        "magic_hex": data[:32].hex(),
        "magic_ascii": "".join(chr(byte) if 32 <= byte < 127 else "." for byte in data[:64]),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    args = parser.parse_args()

    roots: list[dict[str, object]] = []
    for root in args.roots:
        files = sorted(path for path in root.rglob("*") if path.is_file())
        roots.append(
            {
                "root": str(root),
                "file_count": len(files),
                "files": [file_inventory(path) for path in files],
            }
        )
    output = json.dumps({"format": 1, "roots": roots}, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    else:
        print(output, end="")


if __name__ == "__main__":
    main()
