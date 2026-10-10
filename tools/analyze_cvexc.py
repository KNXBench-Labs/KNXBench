"""Observe converter-exception declarations without activating device policy."""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import xml.etree.ElementTree as ET
from collections.abc import Iterable
from typing import cast

MAX_INPUT_BYTES = 8 * 1024 * 1024
MAX_NODES = 100_000
MAX_DEPTH = 128
CV_NAMESPACES = {f"http://knx.org/xml/cvexc/{v}": f"http://knx.org/xml/project/{v}" for v in (11, 14)}
PROGRAM_ATTRIBUTES = {"Id", "ApplicationNumber", "ApplicationVersion", "ProgramType", "MaskVersion", "Name", "LoadProcedureStyle", "PeiType", "HelpFile", "DefaultLanguage", "DynamicTableManagement", "Linkable", "PreEts4Style", "ConvertedFromPreEts4Data", "Hash"}
OPTION_NAMES = ("Comparable", "Reconstructable")


def _tree(data: bytes) -> ET.Element:
    if len(data) > MAX_INPUT_BYTES:
        raise ValueError("converter-exception input exceeds 8 MiB")
    try:
        text = data.decode("utf-8-sig")
    except UnicodeError as error:
        raise ValueError("only UTF-8 converter-exception input is observed") from error
    if re.search(r"<!\s*(DOCTYPE|ENTITY)\b", text, re.I):
        raise ValueError("DTD/entity declarations are refused")
    declaration = re.match(r"\s*<\?xml\b[^?]*\?>", text)
    if declaration:
        encoding = re.search(r"encoding\s*=\s*(['\"])(.*?)\1", declaration[0], re.I)
        if encoding and encoding[2].lower() not in ("utf-8", "utf8"):
            raise ValueError("XML encoding declaration is not UTF-8")
    parser = ET.XMLPullParser(events=("start", "end"))
    depth = nodes = 0
    root: ET.Element | None = None
    try:
        for offset in range(0, len(text), 16_384):
            parser.feed(text[offset:offset + 16_384])
            for event, element in cast(Iterable[tuple[str, ET.Element]], parser.read_events()):
                if event == "start":
                    depth += 1
                    nodes += 1
                    if depth > MAX_DEPTH or nodes > MAX_NODES:
                        raise ValueError("converter-exception XML work budget exceeded")
                    if root is None:
                        root = element
                else:
                    depth -= 1
        parser.close()
    except ET.ParseError as error:
        raise ValueError("malformed converter-exception XML") from error
    if root is None or depth:
        raise ValueError("empty or incomplete converter-exception XML")
    return root


def _decimal(raw: str | None) -> int | None:
    if raw is None or not re.fullmatch(r"[0-9]{1,20}", raw):
        return None
    value = int(raw)
    return value if value <= 2**64 - 1 else None


def analyze(data: bytes) -> dict:
    root = _tree(data)
    namespace = root.tag[1:].split("}", 1)[0] if root.tag.startswith("{") else ""
    project_namespace = CV_NAMESPACES.get(namespace)
    if project_namespace is None or root.tag != f"{{{namespace}}}ConverterExceptions":
        raise ValueError("unsupported converter-exception root namespace")
    cv = lambda name: f"{{{namespace}}}{name}"
    pr = lambda name: f"{{{project_namespace}}}{name}"
    result = {"formatVersion": 1, "rootNamespace": namespace, "projectNamespace": project_namespace,
              "sourceVersion": root.get("Version"), "signaturePresent": "Signature" in root.attrib,
              "signatureVerification": "not-performed", "executable": False, "applicationPrograms": 0,
              "exclusionCount": 0, "records": [], "unknown": [], "findings": [],
              "limitations": ["Observed declarations only; no schema validation or signature verification.",
                              "Offset/Size describe candidate spans relative to the stated code-segment reference, not executable device addresses.",
                              "Absent options are not false. Defaults, precedence and runtime application are unverified.",
                              "Unknown subtrees are serialized observations, not byte-exact XML; the input is never modified."]}
    seen = set()

    def attributes(element: ET.Element, known: set[str], path: str):
        character_content = (element.text or "") + "".join(child.tail or "" for child in element)
        if character_content.strip():
            result["unknown"].append({"path": path, "kind": "text", "raw": character_content})
        for key, value in element.attrib.items():
            if key not in known:
                result["unknown"].append({"path": path, "kind": "attribute", "name": key, "raw": value})

    def unknown(element: ET.Element, path: str):
        result["unknown"].append({"path": path, "kind": "element", "name": element.tag,
                                  "xml": ET.tostring(element, encoding="unicode")})

    def find(code, path):
        result["findings"].append({"code": code, "path": path})

    def program(element: ET.Element, manufacturer: str | None, path: str):
        attributes(element, PROGRAM_ATTRIBUTES, path)
        record = {"manufacturerRef": manufacturer, "attributes": dict(element.attrib), "exclusions": [],
                  "options": {name: {"state": "absent", "raw": None} for name in OPTION_NAMES}}
        key = (manufacturer, element.get("Id"))
        if key[1] is None:
            find("missing-application-id", path)
        elif key in seen:
            find("duplicate-application-id", path)
        seen.add(key)
        options_seen = False
        for static in element:
            static_path = path + "/" + static.tag
            if static.tag != pr("Static"):
                unknown(static, static_path)
                continue
            attributes(static, set(), static_path)
            for child in static:
                child_path = static_path + "/" + child.tag
                if child.tag == pr("Options"):
                    attributes(child, set(OPTION_NAMES), child_path)
                    if options_seen:
                        unknown(child, child_path)
                        find("duplicate-options", child_path)
                        continue
                    options_seen = True
                    for name in OPTION_NAMES:
                        raw = child.get(name)
                        state = "absent" if raw is None else "value" if raw in ("true", "false", "1", "0") else "malformed"
                        record["options"][name] = {"state": state, "raw": raw}
                        if state == "malformed":
                            find("malformed-option", child_path)
                    for nested in child:
                        unknown(nested, child_path + "/" + nested.tag)
                elif child.tag == pr("DeviceCompare"):
                    attributes(child, set(), child_path)
                    for exclusion in child:
                        exclusion_path = child_path + "/" + exclusion.tag
                        if exclusion.tag != pr("ExcludeMemory"):
                            unknown(exclusion, exclusion_path)
                            continue
                        attributes(exclusion, {"CodeSegment", "Offset", "Size"}, exclusion_path)
                        offset, size = exclusion.get("Offset"), exclusion.get("Size")
                        numeric_offset, numeric_size = _decimal(offset), _decimal(size)
                        end = None if numeric_offset is None or numeric_size is None else numeric_offset + numeric_size
                        valid = bool(exclusion.get("CodeSegment")) and end is not None and end <= 2**64 - 1
                        record["exclusions"].append({"codeSegment": exclusion.get("CodeSegment"), "offset": offset, "size": size,
                            "offsetNumeric": numeric_offset, "sizeNumeric": numeric_size,
                            "endExclusiveOffset": end if valid else None, "status": "observed" if valid else "malformed"})
                        result["exclusionCount"] += 1
                        if not valid:
                            find("malformed-exclusion", exclusion_path)
                        for nested in exclusion:
                            unknown(nested, exclusion_path + "/" + nested.tag)
                else:
                    unknown(child, child_path)
        result["records"].append(record)
        result["applicationPrograms"] += 1

    root_path = "/" + root.tag
    attributes(root, {"Version", "Signature"}, root_path)
    for data_element in root:
        data_path = root_path + "/" + data_element.tag
        if data_element.tag != cv("ManufacturerData"):
            unknown(data_element, data_path)
            continue
        attributes(data_element, set(), data_path)
        for manufacturer in data_element:
            manufacturer_path = data_path + "/" + manufacturer.tag
            if manufacturer.tag != pr("Manufacturer"):
                unknown(manufacturer, manufacturer_path)
                continue
            attributes(manufacturer, {"RefId"}, manufacturer_path)
            if manufacturer.get("RefId") is None:
                find("missing-manufacturer-reference", manufacturer_path)
            for programs in manufacturer:
                programs_path = manufacturer_path + "/" + programs.tag
                if programs.tag != pr("ApplicationPrograms"):
                    unknown(programs, programs_path)
                    continue
                attributes(programs, set(), programs_path)
                for element in programs:
                    path = programs_path + "/" + element.tag
                    if element.tag == pr("ApplicationProgram"):
                        program(element, manufacturer.get("RefId"), path)
                    else:
                        unknown(element, path)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path, help="Private JSON report; never overwrites an existing file")
    args = parser.parse_args()
    try:
        with args.source.open("rb") as file:
            data = file.read(MAX_INPUT_BYTES + 1)
        report = analyze(data)
        payload = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
        with args.output.open("x", encoding="utf-8") as file:
            file.write(payload)
        print(f"Observed {report['applicationPrograms']} program declarations and {report['exclusionCount']} exclusions; no device policy activated.")
    except (OSError, ValueError) as error:
        parser.exit(1, f"converter-exception analysis refused: {error}\n")


if __name__ == "__main__":
    main()
