"""Production specs stay safe for case-level parallelism (ADR-0155)."""
from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from tools import production_lanes as lanes
from tools.run_web_gate import PRODUCTION_CONFIGS

APP = Path(__file__).resolve().parents[2] / 'apps/knx-web'


class Hazards(unittest.TestCase):
    def test_each_hazard_kind_is_named_with_its_line(self) -> None:
        source = (
            'import { project } from "./server-project-fixture";\n'
            'let count = 0;\n'
            'const seen = [];\n'
            'const state = { n: 0 };\n'
            'test.describe.serial("s", () => {});\n'
            'test.describe.configure({ mode: "serial" });\n'
            'test.beforeAll(async () => {});\n'
            'test("a", async () => { seen.push(1); state.n = 2; project.devices[0].name = "x"; });\n'
            'test("b", async () => { Object.assign(state, {}); });\n'
        )
        found = lanes.hazards(source)
        self.assertEqual([line for line, _ in found], [2, 5, 6, 7, 8, 8, 8, 9])
        self.assertTrue(all(reason for _, reason in found))

    def test_local_state_inside_a_test_is_not_a_hazard(self) -> None:
        source = (
            'import { project } from "./server-project-fixture";\n'
            'const NAMES = ["a", "b"];\n'
            'test("a", async ({ page }) => {\n'
            '  const seen = [];\n'
            '  let count = 0;\n'
            '  seen.push(NAMES.length);\n'
            '  count += 1;\n'
            '  const copy = structuredClone(project);\n'
            '  copy.devices = [];\n'
            '});\n'
        )
        self.assertEqual(lanes.hazards(source), [])

    def test_comments_strings_and_indentation_do_not_change_module_scope(self) -> None:
        source = (
            '// let harmless = 1; test.beforeAll(() => {});\n'
            'const TEXT = "let example = 2; test.describe.serial(\";\n'
            '  let leaked = 0;\n'
            '  const seen = [];\n'
            'test("a", () => { seen.push(1); });\n'
        )
        self.assertEqual([line for line, _ in lanes.hazards(source)], [3, 5])

    def test_recursive_helpers_aliases_and_cycles_are_checked(self) -> None:
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            e2e = app / 'e2e'
            (e2e / 'nested').mkdir(parents=True)
            (app / 'playwright.demo.config.ts').write_text(
                "export default { testMatch: ['demo.e2e.ts'] };\n")
            (e2e / 'demo.e2e.ts').write_text(
                "import { state as shared } from './nested/barrel';\n"
                "import './side';\n"
                "test('a', () => { shared.n += 1; });\n"
                "test('b', async () => { await import('./dynamic'); });\n")
            (e2e / 'nested/barrel.ts').write_text("export * from '../helper';\n")
            (e2e / 'helper.ts').write_text(
                "import './nested/barrel';\nexport let count = 0;\n")
            (e2e / 'side.ts').write_text('let side = 0;\n')
            (e2e / 'dynamic.ts').write_text('let dynamic = 0;\n')
            problems = lanes.check(app, ('demo',))
            self.assertEqual(len(problems), 4, problems)
            for name in ('demo.e2e.ts:3', 'helper.ts:2', 'side.ts:1', 'dynamic.ts:1'):
                self.assertTrue(any(name in problem for problem in problems), name)

    def test_mixed_explicit_and_unresolved_config_selection_is_refused(self) -> None:
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            config = app / 'playwright.demo.config.ts'
            for selection in ('["demo.e2e.ts", "**/*.other.ts"]',
                              '[/demo/, "demo.e2e.ts"]', 'SPECS'):
                with self.subTest(selection=selection):
                    config.write_text(f'export default {{ testMatch: {selection} }};\n')
                    with self.assertRaisesRegex(ValueError, 'explicit spec'):
                        lanes.production_specs(app, ('demo',))

    def test_production_spec_files_come_from_every_config(self) -> None:
        files = lanes.production_specs(APP, PRODUCTION_CONFIGS)
        self.assertEqual(sorted(files), sorted(set(files)))
        self.assertIn('table-column-widths.e2e.ts', files)
        self.assertIn('notes-workspace.e2e.ts', files)
        self.assertEqual(len(files), 35)
        for name in files:
            self.assertTrue((APP / 'e2e' / name).is_file(), name)

    def test_real_production_specs_and_their_helpers_have_no_hazard(self) -> None:
        problems = lanes.check(APP, PRODUCTION_CONFIGS)
        self.assertEqual(problems, [], '\n'.join(problems))


if __name__ == '__main__':
    unittest.main()
