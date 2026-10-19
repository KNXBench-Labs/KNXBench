"""Change-scoped gate planning: conservative selection with named reasons."""
from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from tools import gate_scope

ROOT = Path(__file__).resolve().parents[2]
MEMBERS = {
    'knx-core': {'dir': 'crates/knx-core', 'deps': set()},
    'knx-projection': {'dir': 'crates/knx-projection', 'deps': {'knx-core'}},
    'knx-store': {'dir': 'crates/knx-store', 'deps': {'knx-core'}},
    'knx-server': {'dir': 'apps/knx-server', 'deps': {'knx-store', 'knx-projection'}},
    'knx-cli': {'dir': 'apps/knx-cli', 'deps': {'knx-store'}},
    'xtask': {'dir': 'xtask', 'deps': set()},
}


class Fixture:
    """A tiny repository: a web app with two specs and two fixture pages."""

    def __init__(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        files = {
            'apps/knx-web/index.html': '<script type="module" src="/src/main.tsx"></script>',
            'apps/knx-web/src/main.tsx': 'import { App } from "./App";\nimport "./styles.css";\n',
            'apps/knx-web/src/App.tsx': 'import { Panel } from "./Panel";\nexport const App = Panel;\n',
            'apps/knx-web/src/Panel.tsx': 'export const Panel = 1;\n',
            'apps/knx-web/src/Lonely.tsx': 'export const Lonely = 1;\n',
            'apps/knx-web/src/styles.css': 'body {}\n',
            'apps/knx-web/e2e/lonely-fixture.html': '<script type="module" src="/e2e/lonely-fixture.tsx"></script>',
            'apps/knx-web/e2e/lonely-fixture.tsx': 'import { Lonely } from "../src/Lonely";\n',
            'apps/knx-web/e2e/helper.ts': 'export const open = (page) => page.goto("/");\n',
            'apps/knx-web/e2e/app.e2e.ts': 'import { open } from "./helper";\ntest("a", async () => {});\n',
            'apps/knx-web/e2e/lonely.e2e.ts': 'test("b", async ({ page }) => { await page.goto("/e2e/lonely-fixture.html"); });\n',
            'apps/knx-web/playwright.devices.config.ts': 'export default { testMatch: ["**/app.e2e.ts"] };\n',
            'crates/knx-core/src/lib.rs': '// core\n',
            'crates/knx-core/tests/doc_inventory.rs': 'const DOC: &str = "../../docs/spec/inventory.md";\n',
            'docs/spec/inventory.md': '# inventory\n',
        }
        for name, text in files.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)

    def plan(self, *changed: str) -> gate_scope.Plan:
        return gate_scope.plan(self.root, list(changed), MEMBERS)

    def close(self) -> None:
        self.directory.cleanup()


class GateScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = Fixture()
        self.addCleanup(self.fixture.close)

    def test_a_leaf_rust_change_selects_only_that_package(self) -> None:
        result = self.fixture.plan('apps/knx-cli/src/main.rs')
        self.assertFalse(result.full)
        self.assertEqual(result.rust_packages, ['knx-cli'])
        self.assertEqual(result.corpus_packages, ['knx-cli'])
        self.assertFalse(result.web)
        self.assertFalse(result.bindings)

    def test_a_core_change_selects_every_reverse_dependent_and_the_bindings(self) -> None:
        result = self.fixture.plan('crates/knx-core/src/lib.rs')
        self.assertEqual(result.rust_packages, ['knx-cli', 'knx-core', 'knx-projection', 'knx-server', 'knx-store'])
        self.assertTrue(result.bindings)

    def test_a_fixture_read_from_another_crate_reruns_the_reader(self) -> None:
        reader = self.fixture.root / 'xtask/tests/reads.rs'
        reader.parent.mkdir(parents=True)
        reader.write_text('const F: &str = "../crates/knx-core/tests/fixtures/sample.json";\n')
        sample = self.fixture.root / 'crates/knx-core/tests/fixtures/sample.json'
        sample.parent.mkdir(parents=True)
        sample.write_text('{}')
        result = self.fixture.plan('crates/knx-core/tests/fixtures/sample.json')
        self.assertIn('xtask', result.rust_packages)
        self.assertEqual(result.rust_packages, ['knx-core', 'xtask'])
        self.assertNotIn('xtask', self.fixture.plan('crates/knx-core/src/lib.rs').rust_packages)

    def test_workspace_manifests_select_every_package(self) -> None:
        self.assertEqual(self.fixture.plan('Cargo.lock').rust_packages, sorted(MEMBERS))

    def test_unknown_paths_select_the_complete_gate(self) -> None:
        result = self.fixture.plan('mystery/thing.bin', 'docs/README.md')
        self.assertTrue(result.full)
        self.assertIn('mystery/thing.bin', result.reasons['complete gate (no rule for this path)'])

    def test_docs_only_needs_no_runtime_tests_unless_a_test_reads_the_document(self) -> None:
        result = self.fixture.plan('docs/ARCHITECTURE.md', '.ai/CURRENT_STATE.md')
        self.assertFalse(result.full)
        self.assertEqual(result.rust_packages, [])
        self.assertFalse(result.web)
        # Only the reading crate's tests re-run: its library did not change.
        self.assertEqual(self.fixture.plan('docs/spec/inventory.md').rust_packages, ['knx-core'])

    def test_a_test_only_change_reruns_only_that_crate(self) -> None:
        self.assertEqual(self.fixture.plan('crates/knx-core/tests/doc_inventory.rs').rust_packages, ['knx-core'])
        self.assertEqual(self.fixture.plan('crates/knx-store/src/lib.rs').rust_packages,
                         ['knx-cli', 'knx-server', 'knx-store'])

    def test_a_web_source_change_selects_the_specs_that_reach_it(self) -> None:
        result = self.fixture.plan('apps/knx-web/src/Panel.tsx')
        self.assertTrue(result.web)
        self.assertEqual(result.browser_specs, ['app.e2e.ts'])
        self.assertEqual(result.production_configs, [])
        self.assertEqual(result.rust_packages, [])
        lonely = self.fixture.plan('apps/knx-web/src/Lonely.tsx')
        self.assertEqual(lonely.browser_specs, ['lonely.e2e.ts'])
        css = self.fixture.plan('apps/knx-web/src/styles.css')
        self.assertEqual(css.browser_specs, ['app.e2e.ts'])

    def test_a_spec_or_helper_change_selects_its_own_specs(self) -> None:
        self.assertEqual(self.fixture.plan('apps/knx-web/e2e/lonely.e2e.ts').browser_specs, ['lonely.e2e.ts'])
        self.assertEqual(self.fixture.plan('apps/knx-web/e2e/helper.ts').browser_specs, ['app.e2e.ts'])

    def test_runner_or_build_inputs_select_every_spec_and_production_suites(self) -> None:
        result = self.fixture.plan('apps/knx-web/vite.config.ts')
        self.assertIsNone(result.browser_specs)
        self.assertEqual(result.production_configs, list(gate_scope.PRODUCTION_CONFIGS))
        self.assertIsNone(self.fixture.plan('tools/browser_tmp.py').browser_specs)
        one = self.fixture.plan('apps/knx-web/playwright.devices.config.ts')
        self.assertEqual(one.production_configs, ['devices'])

    def test_a_computed_navigation_makes_the_spec_run_on_every_web_change(self) -> None:
        spec = self.fixture.root / 'apps/knx-web/e2e/computed.e2e.ts'
        spec.write_text('test("c", async ({ page }) => { await page.goto(url); });\n')
        self.assertEqual(self.fixture.plan('apps/knx-web/src/Lonely.tsx').browser_specs,
                         ['computed.e2e.ts', 'lonely.e2e.ts'])

    def test_an_unresolvable_import_makes_the_spec_run_on_every_web_change(self) -> None:
        (self.fixture.root / 'apps/knx-web/src/Panel.tsx').write_text('import "./missing";\n')
        self.assertEqual(self.fixture.plan('apps/knx-web/src/Lonely.tsx').browser_specs,
                         ['app.e2e.ts', 'lonely.e2e.ts'])

    def test_website_story_and_tools_have_their_own_stages(self) -> None:
        self.assertTrue(self.fixture.plan('website/index.html').website_tests)
        self.assertTrue(self.fixture.plan('story/edition.json').story_tests)
        self.assertEqual(self.fixture.plan('tools/run_corpus_tests.py').corpus_packages, ['*'])

    def test_the_repository_specs_are_statically_analysable(self) -> None:
        # A spec the graph cannot follow runs on every web change: correct,
        # but it should stay the exception, so name them here when it grows.
        app = ROOT / 'apps/knx-web'
        _, why = gate_scope.select_specs(app, [app / 'src/App.tsx'])
        opaque = sorted(s for s, cause in why.items() if cause.startswith('not statically'))
        self.assertLessEqual(len(opaque), 12, opaque)
        self.assertTrue(set(why) - set(opaque), 'App.tsx must reach some spec')


if __name__ == '__main__':
    unittest.main()
