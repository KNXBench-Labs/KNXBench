"""Synthetic checks for converter-exception observations, namespaces and refusal."""
import importlib.util
import pathlib
import unittest

TOOL = pathlib.Path(__file__).resolve().parents[1] / "analyze_cvexc.py"


def analyzer():
    spec = importlib.util.spec_from_file_location("analyze_cvexc", TOOL)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def document(body, version="14", extra=""):
    return f'''<ConverterExceptions xmlns="http://knx.org/xml/cvexc/{version}" Version="201" Signature="synthetic" {extra}>
    <ManufacturerData><Manufacturer xmlns="http://knx.org/xml/project/{version}" RefId="M-0001">
    <ApplicationPrograms>{body}</ApplicationPrograms></Manufacturer></ManufacturerData></ConverterExceptions>'''.encode()


class ConverterExceptions(unittest.TestCase):
    def test_observes_relative_ranges_without_enabling_device_behavior(self):
        result = analyzer().analyze(document('''<ApplicationProgram Id="A-1" Name="Synthetic" ApplicationVersion="1"><Static>
        <DeviceCompare><ExcludeMemory CodeSegment="AS-1" Offset="007" Size="3"/></DeviceCompare>
        <Options Comparable="true"/></Static></ApplicationProgram>'''))
        self.assertEqual(result["applicationPrograms"], 1)
        self.assertEqual(result["exclusionCount"], 1)
        self.assertEqual(result["records"][0]["exclusions"][0], {"codeSegment": "AS-1", "offset": "007", "size": "3", "offsetNumeric": 7, "sizeNumeric": 3, "endExclusiveOffset": 10, "status": "observed"})
        self.assertEqual(result["records"][0]["options"]["Comparable"], {"state": "value", "raw": "true"})
        self.assertEqual(result["records"][0]["options"]["Reconstructable"], {"state": "absent", "raw": None})
        self.assertFalse(result["executable"])
        self.assertEqual(result["signatureVerification"], "not-performed")

    def test_unknown_attributes_and_foreign_names_are_not_typed_by_local_name(self):
        result = analyzer().analyze(document('''<ApplicationProgram Id="A-1" xmlns:x="urn:foreign"><Static>
        <DeviceCompare><x:ExcludeMemory CodeSegment="AS-1" Offset="1" Size="2"/>
        <ExcludeMemory CodeSegment="AS-1" Offset="1" Size="2" x:Offset="999"/></DeviceCompare>
        <Options x:Comparable="false"/></Static></ApplicationProgram>'''))
        self.assertEqual(result["exclusionCount"], 1)
        self.assertEqual(result["records"][0]["exclusions"][0]["offsetNumeric"], 1)
        self.assertEqual(result["records"][0]["options"]["Comparable"]["state"], "absent")
        self.assertEqual(len(result["unknown"]), 3)
        self.assertTrue(all("path" in entry for entry in result["unknown"]))

    def test_malformed_lexemes_are_reported_and_retained_not_coerced(self):
        result = analyzer().analyze(document('''<ApplicationProgram Id="A-1"><Static><DeviceCompare>
        <ExcludeMemory CodeSegment="AS-1" Offset="-1" Size=" 4 "/></DeviceCompare><Options Comparable="maybe"/></Static></ApplicationProgram>'''))
        exclusion = result["records"][0]["exclusions"][0]
        self.assertEqual(exclusion["offset"], "-1")
        self.assertEqual(exclusion["size"], " 4 ")
        self.assertIsNone(exclusion["endExclusiveOffset"])
        self.assertEqual(exclusion["status"], "malformed")
        self.assertEqual(result["records"][0]["options"]["Comparable"], {"state": "malformed", "raw": "maybe"})

    def test_duplicate_declarations_are_observed_and_flagged_not_deduplicated(self):
        result = analyzer().analyze(document('<ApplicationProgram Id="A-1"><Static/></ApplicationProgram>' * 2))
        self.assertEqual(result["applicationPrograms"], 2)
        self.assertEqual(len(result["records"]), 2)
        self.assertIn("duplicate-application-id", [entry["code"] for entry in result["findings"]])

    def test_lookalike_root_dtd_unsafe_depth_and_bad_xml_refuse(self):
        module = analyzer()
        for data in [document("", version="014"), b'<!DOCTYPE x [<!ENTITY a "bad">]><x/>', b'<broken>', b'<x>' * 140 + b'</x>' * 140]:
            with self.subTest(size=len(data)), self.assertRaises(ValueError):
                module.analyze(data)

    def test_known_wrappers_do_not_silently_drop_character_content(self):
        result = analyzer().analyze(document('<ApplicationProgram Id="A-1"><Static><Options Comparable="true">extra &amp; text</Options></Static></ApplicationProgram>'))
        self.assertEqual(len(result["unknown"]), 1)
        self.assertEqual(result["unknown"][0]["kind"], "text")
        self.assertEqual(result["unknown"][0]["raw"], "extra & text")

    def test_misplaced_known_leaf_is_unknown_not_an_exclusion(self):
        result = analyzer().analyze(document('<ApplicationProgram Id="A-1"><Static><ExcludeMemory CodeSegment="AS-1" Offset="1" Size="2"/></Static></ApplicationProgram>'))
        self.assertEqual(result["exclusionCount"], 0)
        self.assertEqual(len(result["unknown"]), 1)


if __name__ == "__main__":
    unittest.main()
