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

    def test_every_mutator_and_ordering_name_is_caught_with_line_and_reason(self) -> None:
        # Characterization: these branches already worked; the tests pin them (review1 survivors).
        for name in ('push', 'pop', 'shift', 'unshift', 'splice', 'sort', 'reverse', 'set',
                     'delete', 'clear', 'add', 'fill', 'copyWithin'):
            with self.subTest(mutator=name):
                found = lanes.hazards(f'const shared = new Map();\ntest("a", () => {{ shared.{name}(1); }});\n')
                self.assertEqual(found, [(2, 'mutates module binding shared')])
        for construct in ('beforeAll', 'afterAll'):
            with self.subTest(hook=construct):
                found = lanes.hazards(f'test.{construct}(async () => {{}});\n')
                self.assertEqual(found, [(1, f'ordered construct {construct}(')])

    def test_set_and_map_clear_are_mutations(self) -> None:
        self.assertEqual(lanes.hazards('const ids = new Set();\ntest("a", () => { ids.clear(); });\n'),
                         [(2, 'mutates module binding ids')])
        self.assertEqual(lanes.hazards('const m = new Map();\ntest("a", () => { m.clear(); });\n'),
                         [(2, 'mutates module binding m')])

    def test_after_all_is_an_ordered_construct(self) -> None:
        self.assertEqual(lanes.hazards('test.afterAll(async () => {});\n'),
                         [(1, 'ordered construct afterAll(')])

    def test_comma_declarators_are_inventoried(self) -> None:
        source = ('const first = [], second = [];\n'
                  'test("a", () => { second.push(1); });\n')
        self.assertEqual(lanes.hazards(source), [(2, 'mutates module binding second')])
        source = 'let a = 1,\n  b = 2;\nconst c = fn(1, d), e = {x: 1, y: 2};\n'
        self.assertEqual([line for line, _ in lanes.hazards(source)], [1, 2])
        source = ('const NAMES = ["a", "b"], N: Map<string, number> = new Map();\n'
                  'test("a", () => { const y = N.size; });\n')
        self.assertEqual(lanes.hazards(source), [])

    def test_whitespace_before_a_mutator_call_is_still_a_mutation(self) -> None:
        self.assertEqual(lanes.hazards('const seen = [];\ntest("a", () => { seen.push (1); });\n'),
                         [(2, 'mutates module binding seen')])

    def test_import_alias_split_over_lines_is_resolved(self) -> None:
        source = ('import { state as\n shared } from "./helper";\n'
                  'import { other\n as\n  second } from "./helper";\n'
                  'test("a", () => { shared.n = 1; second.n = 2; });\n')
        self.assertEqual(lanes.hazards(source), [(6, 'mutates module binding second'),
                                                 (6, 'mutates module binding shared')])

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

    def _selection(self, text: str) -> list[str]:
        with TemporaryDirectory() as tmp:
            app = Path(tmp)
            (app / 'playwright.demo.config.ts').write_text(text)
            return lanes.production_specs(app, ('demo',))

    def test_trailing_expressions_and_decoy_keys_are_refused(self) -> None:
        refused = (
            'export default { testMatch: ["demo.e2e.ts"].concat(["other.e2e.ts"]) };\n',
            'export default { testMatch: "demo.e2e.ts" + suffix };\n',
            'export default { testMatch: ["demo.e2e.ts"] + suffix };\n',
            'export default { testMatch: "demo.e2e.ts".toString() };\n',
            'export default { testMatch: cond ? "demo.e2e.ts" : "x.e2e.ts" };\n',
            'export default { testMatch: ["demo.e2e.ts"] ?? SPECS };\n',
            'export default { testMatch: ["demo.e2e.ts"][0] };\n',
            'export default { testMatch: `demo.e2e.ts` };\n',
            '// testMatch: ["demo.e2e.ts"]\nexport default { testMatch: SPECS };\n',
            '/* testMatch: "demo.e2e.ts" */ export default { testMatch: SPECS };\n',
            'const note = "testMatch: [\\"demo.e2e.ts\\"]";\nexport default {};\n',
            'export default { testMatch: ["demo.e2e.ts"], other: { testMatch: ["x.e2e.ts"] } };\n',
            'export default { testMatch: ["demo.e2e.ts"], testMatch: SPECS };\n',
            'export default { testMatch: ["demo.e2e.ts" "x.e2e.ts"] };\n',
            'export default { testMatch: ["demo.e2e.ts",, "x.e2e.ts"] };\n',
            'export default { testMatch: [] };\n',
            'export default {};\n',
        )
        for text in refused:
            with self.subTest(config=text):
                with self.assertRaisesRegex(ValueError, 'explicit spec'):
                    self._selection(text)

    def test_complete_literal_selections_with_comments_and_whitespace_are_read(self) -> None:
        accepted = (
            ('export default { testMatch: "demo.e2e.ts" };\n', ['demo.e2e.ts']),
            ('export default { testMatch : \n  /* a */ "demo.e2e.ts" // b\n , x: 1 };\n', ['demo.e2e.ts']),
            ("export default { testMatch: ['demo.e2e.ts', \"**/other.e2e.ts\",] };\n",
             ['demo.e2e.ts', 'other.e2e.ts']),
            ('export default { testMatch: [\n  // first\n  "demo.e2e.ts", /* second */ "b.e2e.ts"\n  ]\n};\n',
             ['b.e2e.ts', 'demo.e2e.ts']),
            ('// testMatch: SPECS\nexport default { testMatch: ["demo.e2e.ts"] };\n', ['demo.e2e.ts']),
            ('export default defineConfig({...base,testMatch:"demo.e2e.ts"});\n', ['demo.e2e.ts']),
        )
        for text, expected in accepted:
            with self.subTest(config=text):
                self.assertEqual(self._selection(text), expected)

    def test_duplicate_quoted_or_computed_selection_keys_are_refused(self) -> None:
        for key in ('"testMatch"', "'testMatch'", '["testMatch"]', "['testMatch']"):
            with self.subTest(key=key):
                with self.assertRaisesRegex(ValueError, 'explicit spec'):
                    self._selection('export default { testMatch: ["demo.e2e.ts"], '
                                    + key + ': ["demo.e2e.ts", "other.e2e.ts"] };')
        self.assertEqual(self._selection(
            '// "testMatch": OTHER\nexport default { testMatch: "demo.e2e.ts" };'),
            ['demo.e2e.ts'])

    def test_unrelated_template_or_regex_after_selection_is_not_tokenized(self) -> None:
        for tail in ("webServer: { command: `echo it's` }", "note: /it's/", 'note: `say "hello`'):
            with self.subTest(tail=tail):
                self.assertEqual(self._selection(
                    'export default { testMatch: "demo.e2e.ts", ' + tail + ' };'),
                    ['demo.e2e.ts'])

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
